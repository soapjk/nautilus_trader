// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Live execution client implementation for the Longport adapter.

use std::{
    future::Future,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::Context;
use async_trait::async_trait;
// use chrono::{DateTime, Utc}; // Unused import
// use futures_util::StreamExt; // Unused import
use longport::{
    trade::{TradeContext, SubmitOrderOptions},
    Config, decimal,
};
use nautilus_common::{
    live::{runner::get_exec_event_sender, runtime::get_runtime},
    messages::{
        ExecutionEvent,
        execution::{
            BatchCancelOrders, CancelAllOrders, CancelOrder, GenerateFillReports,
            GenerateOrderStatusReport, GeneratePositionStatusReports, ModifyOrder, QueryAccount,
            QueryOrder, SubmitOrder, SubmitOrderList,
        },
    },
};
use nautilus_core::{MUTEX_POISONED, UUID4, UnixNanos, time::get_atomic_clock_realtime};
use nautilus_execution::client::{ExecutionClient, base::ExecutionClientCore};
use nautilus_model::{
    accounts::AccountAny,
    enums::{OmsType, OrderType as NautilusOrderType},
    events::{AccountState, OrderEventAny, OrderSubmitted, OrderAccepted, OrderRejected, OrderCanceled},
    identifiers::{AccountId, ClientId, Venue, VenueOrderId, ClientOrderId, TraderId},
    orders::Order,
    reports::{ExecutionMassStatus, FillReport, OrderStatusReport, PositionStatusReport},
    types::{AccountBalance, Currency, MarginBalance, Money},
};
use rust_decimal::prelude::ToPrimitive;
use tokio::task::JoinHandle;
use ustr::Ustr;

use crate::{
    common::consts::LONGPORT_VENUE,
    config::LongportExecClientConfig,
    common::parse::{instrument_id_to_string, parse_order_side, parse_order_type, parse_time_in_force},
};

/// LongPort execution client.
pub struct LongportExecutionClient {
    core: ExecutionClientCore,
    config: LongportExecClientConfig,
    trade_ctx: Arc<TradeContext>,
    exec_event_sender: Option<tokio::sync::mpsc::UnboundedSender<ExecutionEvent>>,
    started: bool,
    connected: AtomicBool,
    ws_stream_handle: Option<JoinHandle<()>>,
    pending_tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl std::fmt::Debug for LongportExecutionClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LongportExecutionClient")
            .field("core", &self.core)
            .field("config", &self.config)
            .field("started", &self.started)
            .field("connected", &self.connected)
            .finish_non_exhaustive()
    }
}

impl LongportExecutionClient {
    /// Creates a new [`LongportExecutionClient`].
    ///
    /// # Errors
    ///
    /// Returns an error if the client fails to initialize.
    pub fn new(core: ExecutionClientCore, config: LongportExecClientConfig) -> anyhow::Result<Self> {
        // Build Longport configuration from credentials
        let app_key = config
            .get_app_key()
            .ok_or_else(|| anyhow::anyhow!("LONGPORT_APP_KEY not set"))?;
        let app_secret = config
            .get_app_secret()
            .ok_or_else(|| anyhow::anyhow!("LONGPORT_APP_SECRET not set"))?;
        let access_token = config
            .get_access_token()
            .ok_or_else(|| anyhow::anyhow!("LONGPORT_ACCESS_TOKEN not set"))?;

        let longport_config = Config::new(app_key, app_secret, access_token);

        let runtime = get_runtime();
        let (trade_ctx, _receiver) = runtime
            .block_on(TradeContext::try_new(std::sync::Arc::new(longport_config)))
            .context("failed to create Longport trade context")?;

        Ok(Self {
            core,
            config,
            trade_ctx: Arc::new(trade_ctx),
            exec_event_sender: None,
            started: false,
            connected: AtomicBool::new(false),
            ws_stream_handle: None,
            pending_tasks: Mutex::new(Vec::new()),
        })
    }

    async fn refresh_account_state(&self) -> anyhow::Result<AccountState> {
        // Fetch account balance from Longport SDK
        // The SDK returns a Vec<AccountBalance> and accepts an optional account type parameter
        let account_balances = self
            .trade_ctx
            .account_balance(None)  // None means use default account
            .await
            .context("failed to fetch account balance")?;

        // Convert Longport balance to Nautilus AccountBalance
        let mut balances = Vec::new();

        // Process each balance from Longport
        for acc_balance in account_balances {
            // Convert rust_decimal to f64
            use rust_decimal::Decimal;
            let total_cash = Decimal::to_f64(&acc_balance.total_cash).unwrap_or(0.0);

            // Determine currency - Longport primarily serves HK market, so default to HKD
            // In the future, we could enhance this by parsing currency from acc_balance if available
            let currency = Currency::HKD();

            // Create AccountBalance from Longport AccountBalance
            // Note: The SDK's AccountBalance structure may have limited currency information
            // Use total_cash as total and assume it's all free for now
            balances.push(AccountBalance {
                currency,
                total: Money::new(total_cash, currency),
                locked: Money::new(0.0, currency),
                free: Money::new(total_cash, currency),
            });
        }

        let account_state = AccountState::new(
            self.core.account_id,
            self.core.account_type,
            balances,
            vec![], // margins - Margin information would require additional SDK calls
            false,  // reported
            UUID4::new(),
            get_atomic_clock_realtime().get_time_ns(),
            get_atomic_clock_realtime().get_time_ns(),
            None, // base_currency
        );

        Ok(account_state)
    }

    fn update_account_state(&self) -> anyhow::Result<()> {
        let runtime = get_runtime();
        runtime.block_on(self.refresh_account_state())?;
        Ok(())
    }

    fn spawn_task<F>(&self, description: &'static str, fut: F)
    where
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let runtime = get_runtime();
        let handle = runtime.spawn(async move {
            if let Err(e) = fut.await {
                tracing::warn!("{description} failed: {e:?}");
            }
        });

        let mut tasks = self.pending_tasks.lock().expect(MUTEX_POISONED);
        tasks.retain(|handle| !handle.is_finished());
        tasks.push(handle);
    }

    fn abort_pending_tasks(&self) {
        let mut tasks = self.pending_tasks.lock().expect(MUTEX_POISONED);
        for handle in tasks.drain(..) {
            handle.abort();
        }
    }

    fn is_conditional_order(&self, order_type: NautilusOrderType) -> bool {
        matches!(
            order_type,
            NautilusOrderType::StopMarket
                | NautilusOrderType::StopLimit
                | NautilusOrderType::MarketIfTouched
                | NautilusOrderType::LimitIfTouched
        )
    }

    /// Convert Nautilus order to Longport SubmitOrderOptions
    fn build_submit_order_options(
        &self,
        order: &impl Order,
    ) -> anyhow::Result<SubmitOrderOptions> {
        let symbol = instrument_id_to_string(order.instrument_id())?;
        let side = parse_order_side(order.order_side())?;
        let order_type = parse_order_type(order.order_type())?;
        let time_in_force = parse_time_in_force(order.time_in_force())?;

        let quantity = decimal!(order.quantity().as_f64());

        let mut opts = SubmitOrderOptions::new(
            symbol,
            order_type,
            side,
            quantity,
            time_in_force,
        );

        // Set price for limit orders
        if let Some(price) = order.price() {
            let price_dec: rust_decimal::Decimal = rust_decimal::Decimal::from_f64_retain(price.as_f64())
                .unwrap_or(rust_decimal::Decimal::from(0));
            opts = opts.submitted_price(price_dec);
        }

        // Set stop price for stop orders
        if let Some(trigger_price) = order.trigger_price() {
            let trigger_price_dec: rust_decimal::Decimal = rust_decimal::Decimal::from_f64_retain(trigger_price.as_f64())
                .unwrap_or(rust_decimal::Decimal::from(0));
            opts = opts.trigger_price(trigger_price_dec);
        }

        // Add remark for tracking
        opts = opts.remark(format!("Nautilus: {}", order.client_order_id()));

        Ok(opts)
    }
}

#[async_trait(?Send)]
impl ExecutionClient for LongportExecutionClient {
    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    fn client_id(&self) -> ClientId {
        self.core.client_id
    }

    fn account_id(&self) -> AccountId {
        self.core.account_id
    }

    fn venue(&self) -> Venue {
        *LONGPORT_VENUE
    }

    fn oms_type(&self) -> OmsType {
        self.core.oms_type
    }

    fn get_account(&self) -> Option<AccountAny> {
        self.core.get_account()
    }

    async fn connect(&mut self) -> anyhow::Result<()> {
        if self.connected.load(Ordering::Acquire) {
            return Ok(());
        }

        // Initialize exec event sender (must be done in async context after runner is set up)
        if self.exec_event_sender.is_none() {
            self.exec_event_sender = Some(get_exec_event_sender());
        }

        let Some(sender) = self.exec_event_sender.as_ref() else {
            tracing::error!("Execution event sender not initialized");
            anyhow::bail!("Execution event sender not initialized");
        };

        // Fetch and emit initial account state
        let account_state = self
            .refresh_account_state()
            .await
            .context("failed to request Longport account state")?;

        if let Err(e) = sender.send(ExecutionEvent::Account(account_state)) {
            tracing::error!("Failed to send account state: {e}");
        }

        self.connected.store(true, Ordering::Release);
        tracing::info!(client_id = %self.core.client_id, "Connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if !self.connected.load(Ordering::Acquire) {
            return Ok(());
        }

        self.abort_pending_tasks();

        if let Some(handle) = self.ws_stream_handle.take() {
            handle.abort();
        }

        self.connected.store(false, Ordering::Release);
        tracing::info!(client_id = %self.core.client_id, "Disconnected");
        Ok(())
    }

    fn query_account(&self, _cmd: &QueryAccount) -> anyhow::Result<()> {
        self.update_account_state()
    }

    fn query_order(&self, cmd: &QueryOrder) -> anyhow::Result<()> {
        tracing::debug!(
            "query_order for client_order_id={}",
            cmd.client_order_id
        );

        let trade_ctx = self.trade_ctx.clone();
        let client_order_id = cmd.client_order_id;

        self.spawn_task("query_order", async move {
            // Order query implementation would require using the Longport SDK's order query methods
            // This is currently a placeholder that logs the query
            tracing::debug!("Querying order: {}", client_order_id);
            Ok(())
        });

        Ok(())
    }

    fn generate_account_state(
        &self,
        balances: Vec<AccountBalance>,
        margins: Vec<MarginBalance>,
        reported: bool,
        ts_event: UnixNanos,
    ) -> anyhow::Result<()> {
        self.core
            .generate_account_state(balances, margins, reported, ts_event)
    }

    fn start(&mut self) -> anyhow::Result<()> {
        if self.started {
            return Ok(());
        }

        self.started = true;

        tracing::info!(
            client_id = %self.core.client_id,
            account_id = %self.core.account_id,
            account_type = ?self.core.account_type,
            markets = ?self.config.markets,
            "Started"
        );
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        if !self.started {
            return Ok(());
        }

        self.started = false;
        self.connected.store(false, Ordering::Release);
        if let Some(handle) = self.ws_stream_handle.take() {
            handle.abort();
        }
        self.abort_pending_tasks();
        tracing::info!(client_id = %self.core.client_id, "Stopped");
        Ok(())
    }

    fn submit_order(&self, cmd: &SubmitOrder) -> anyhow::Result<()> {
        let order = &cmd.order;

        if order.is_closed() {
            let client_order_id = order.client_order_id();
            tracing::warn!("Cannot submit closed order {client_order_id}");
            return Ok(());
        }

        let event = OrderSubmitted::new(
            self.core.trader_id,
            order.strategy_id(),
            order.instrument_id(),
            order.client_order_id(),
            self.core.account_id,
            UUID4::new(),
            cmd.ts_init,
            get_atomic_clock_realtime().get_time_ns(),
        );
        if let Some(sender) = &self.exec_event_sender {
            tracing::debug!("OrderSubmitted client_order_id={}", order.client_order_id());
            if let Err(e) = sender.send(ExecutionEvent::Order(OrderEventAny::Submitted(event))) {
                tracing::warn!("Failed to send OrderSubmitted event: {e}");
            }
        } else {
            tracing::warn!("Cannot send OrderSubmitted: exec_event_sender not initialized");
        }

        // Submit order via Longport SDK
        let trade_ctx = self.trade_ctx.clone();
        let order_clone = order.clone();
        let sender = self.exec_event_sender.clone();
        let trader_id = self.core.trader_id;
        let strategy_id = order.strategy_id();
        let instrument_id = order.instrument_id();
        let client_order_id = order.client_order_id();
        let account_id = self.core.account_id;

        // Build order options
        let submit_opts = match self.build_submit_order_options(order) {
            Ok(opts) => opts,
            Err(e) => {
                tracing::error!("Failed to build submit order options: {e:?}");
                // Send rejection event
                if let Some(sender) = sender {
                    let reject_event = OrderRejected::new(
                        trader_id,
                        strategy_id,
                        instrument_id,
                        client_order_id,
                        account_id,
                        Ustr::from(&format!("Failed to build order: {e}")),
                        UUID4::new(),
                        get_atomic_clock_realtime().get_time_ns(),
                        get_atomic_clock_realtime().get_time_ns(),
                        false,  // reconciliation
                        false,  // due_post_only
                    );
                    let _ = sender.send(ExecutionEvent::Order(OrderEventAny::Rejected(reject_event)));
                }
                return Ok(());
            }
        };

        self.spawn_task("submit_order", async move {
            // Submit order via Longport SDK
            match trade_ctx.submit_order(submit_opts).await {
                Ok(response) => {
                    tracing::info!(
                        "Order submitted successfully: client_order_id={}, venue_order_id={}",
                        client_order_id,
                        response.order_id
                    );

                    // Send OrderAccepted event
                    if let Some(sender) = sender {
                        let accepted_event = OrderAccepted::new(
                            trader_id,
                            strategy_id,
                            instrument_id,
                            client_order_id,
                            VenueOrderId::new(response.order_id.as_str()),
                            account_id,
                            UUID4::new(),
                            get_atomic_clock_realtime().get_time_ns(),
                            get_atomic_clock_realtime().get_time_ns(),
                            false,  // reconciliation
                        );
                        let _ = sender.send(ExecutionEvent::Order(OrderEventAny::Accepted(accepted_event)));
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to submit order: {e:?}");

                    // Send rejection event
                    if let Some(sender) = sender {
                        let reject_event = OrderRejected::new(
                            trader_id,
                            strategy_id,
                            instrument_id,
                            client_order_id,
                            account_id,
                            Ustr::from(&format!("Submission failed: {e}")),
                            UUID4::new(),
                            get_atomic_clock_realtime().get_time_ns(),
                            get_atomic_clock_realtime().get_time_ns(),
                            false,  // reconciliation
                            false,  // due_post_only
                        );
                        let _ = sender.send(ExecutionEvent::Order(OrderEventAny::Rejected(reject_event)));
                    }
                }
            }

            Ok(())
        });

        Ok(())
    }

    fn submit_order_list(&self, cmd: &SubmitOrderList) -> anyhow::Result<()> {
        // TODO: Implement order list submission for User Story 3 (Execution)
        // Longport doesn't support order lists (bracket/OCO orders) natively
        // We need to submit orders individually
        tracing::warn!(
            "submit_order_list: Longport doesn't support native order lists - not yet implemented"
        );
        Ok(())
    }

    fn modify_order(&self, _cmd: &ModifyOrder) -> anyhow::Result<()> {
        // TODO: Implement modify order for User Story 3 (Execution)
        tracing::warn!("modify_order: not yet implemented");
        Ok(())
    }

    fn cancel_order(&self, _cmd: &CancelOrder) -> anyhow::Result<()> {
        // TODO: Implement cancel order for User Story 3 (Execution)
        tracing::warn!("cancel_order: not yet implemented");
        Ok(())
    }

    fn cancel_all_orders(&self, _cmd: &CancelAllOrders) -> anyhow::Result<()> {
        // TODO: Implement cancel all orders for User Story 3 (Execution)
        tracing::warn!("cancel_all_orders: not yet implemented");
        Ok(())
    }

    fn batch_cancel_orders(&self, _cmd: &BatchCancelOrders) -> anyhow::Result<()> {
        // TODO: Implement batch cancel for User Story 3 (Execution)
        tracing::warn!("batch_cancel_orders: not yet implemented");
        Ok(())
    }
}

// TODO: Implement report generation methods for User Story 3 (Execution)
// TODO: Implement LiveExecutionClient trait methods for report generation

#[cfg(test)]
mod tests {
    use super::*;
    use nautilus_common::{cache::Cache, clock::TestClock};
    use nautilus_model::{identifiers::{AccountId, ClientId, TraderId}, enums::{AccountType, OmsType}};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn test_longport_execution_client_venue() {
        let trader_id = TraderId::from("TRADER-001");
        let account_id = AccountId::from("LONGPORT-001");
        let client_id = ClientId::from("LONGPORT");
        let venue = *LONGPORT_VENUE;
        let oms_type = OmsType::Hedging;
        let account_type = AccountType::Cash;

        let clock = Rc::new(RefCell::new(TestClock::new()));
        let cache = Rc::new(RefCell::new(Cache::default()));

        let core = ExecutionClientCore::new(
            trader_id,
            client_id,
            venue,
            oms_type,
            account_id,
            account_type,
            None,
            clock,
            cache,
        );

        let config = LongportExecClientConfig {
            trader_id,
            account_id,
            app_key: Some("test_key".to_string()),
            app_secret: Some("test_secret".to_string()),
            access_token: Some("test_token".to_string()),
            ..Default::default()
        };

        let _client = LongportExecutionClient::new(core, config);
        // TODO: Add assertions when we implement actual client creation
    }
}
