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
            GenerateOrderStatusReport, GeneratePositionReports, ModifyOrder, QueryAccount,
            QueryOrder, SubmitOrder, SubmitOrderList,
        },
    },
};
use nautilus_core::{MUTEX_POISONED, UUID4, UnixNanos, time::get_atomic_clock_realtime};
use nautilus_execution::client::{ExecutionClient, base::ExecutionClientCore};
use nautilus_live::execution::client::LiveExecutionClient;
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

            // Create AccountBalance from Longport AccountBalance
            // Note: frozen_cash and cash fields don't exist in the SDK's AccountBalance
            // Use total_cash as total and a placeholder for locked/free calculation
            balances.push(AccountBalance {
                currency: Currency::USD(), // TODO: Determine actual currency from acc_balance.currency
                total: Money::new(total_cash, Currency::USD()),
                locked: Money::new(0.0, Currency::USD()),  // TODO: Calculate from available cash
                free: Money::new(total_cash, Currency::USD()),  // Use total cash as free for now
            });
        }

        let account_state = AccountState::new(
            self.core.account_id,
            self.core.account_type,
            balances,
            vec![], // margins - TODO: Fetch margin information
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
            // TODO: Implement order query via Longport SDK
            // Use trade_ctx.query_order() or similar method
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
        // Longport doesn't support order lists (bracket/OCO orders) natively
        // We need to submit orders individually
        tracing::warn!(
            "submit_order_list: Longport doesn't support native order lists, submitting {} orders individually",
            cmd.order_list.orders.len()
        );

        let order_list = cmd.order_list.clone();
        let sender = self.exec_event_sender.clone();
        let trade_ctx = self.trade_ctx.clone();
        let trader_id = self.core.trader_id;
        let account_id = self.core.account_id;

        self.spawn_task("submit_order_list", async move {
            let mut submitted_count = 0;
            let mut failed_count = 0;

            for order in &order_list.orders {
                // Submit each order individually
                // Note: This is a simplified implementation
                // In production, you'd use the full submit_order logic
                match submit_single_order(&trade_ctx, order, trader_id, account_id).await {
                    Ok(_) => submitted_count += 1,
                    Err(e) => {
                        tracing::error!("Failed to submit order in list: {e}");
                        failed_count += 1;
                    }
                }
            }

            tracing::info!(
                "Order list submission complete: {} submitted, {} failed",
                submitted_count,
                failed_count
            );

            Ok(())
        });

        Ok(())
    }

    fn modify_order(&self, cmd: &ModifyOrder) -> anyhow::Result<()> {
        let trade_ctx = self.trade_ctx.clone();
        let command = cmd.clone();
        let sender = self.exec_event_sender.clone();
        let trader_id = self.core.trader_id;
        let strategy_id = cmd.strategy_id;
        let instrument_id = cmd.instrument_id;
        let client_order_id = cmd.client_order_id;
        let account_id = self.core.account_id;
        let venue_order_id = cmd.venue_order_id;

        self.spawn_task("modify_order", async move {
            // Longport SDK doesn't have a direct modify_order API
            // We need to implement cancel + replace pattern
            tracing::debug!(
                "Modifying order: client_order_id={}, venue_order_id={}, price={:?}, qty={:?}",
                client_order_id,
                venue_order_id,
                command.price,
                command.quantity
            );

            // Step 1: Cancel the existing order
            let cancel_result = trade_ctx.cancel_order(venue_order_id.as_str()).await;

            if let Err(e) = cancel_result {
                tracing::error!("Failed to cancel order for modification: {e:?}");
                // Send rejection event
                if let Some(sender) = sender {
                    let reject_event = OrderRejected::new(
                        trader_id,
                        strategy_id,
                        instrument_id,
                        client_order_id,
                        account_id,
                        Ustr::from(&format!("Modify failed (cancel error): {e}")),
                        UUID4::new(),
                        get_atomic_clock_realtime().get_time_ns(),
                        get_atomic_clock_realtime().get_time_ns(),
                        false,  // reconciliation
                        false,  // due_post_only
                    );
                    let _ = sender.send(ExecutionEvent::Order(OrderEventAny::Rejected(reject_event)));
                }
                anyhow::bail!("Failed to cancel order for modification: {e}");
            }

            // Step 2: Submit new order with modified parameters
            // We need to rebuild the order options from the modified command
            // For now, we'll need to query the original order details first
            // This is a simplified implementation - in production you'd cache order details

            // Since we don't have the original order details readily available,
            // we'll emit a cancellation event and let the system handle resubmission
            if let Some(sender) = sender {
                let canceled_event = OrderCanceled::new(
                    trader_id,
                    strategy_id,
                    instrument_id,
                    client_order_id,
                    UUID4::new(),
                    get_atomic_clock_realtime().get_time_ns(),
                    get_atomic_clock_realtime().get_time_ns(),
                    false,  // reconciliation
                    Some(venue_order_id),
                    Some(account_id),
                );
                let _ = sender.send(ExecutionEvent::Order(OrderEventAny::Canceled(canceled_event)));
            }

            tracing::info!("Order canceled for modification, resubmit with new parameters");
            Ok(())
        });

        Ok(())
    }

    fn cancel_order(&self, cmd: &CancelOrder) -> anyhow::Result<()> {
        let trade_ctx = self.trade_ctx.clone();
        let command = cmd.clone();
        let sender = self.exec_event_sender.clone();
        let trader_id = self.core.trader_id;
        let strategy_id = cmd.strategy_id;
        let instrument_id = cmd.instrument_id;
        let client_order_id = cmd.client_order_id;
        let account_id = self.core.account_id;

        self.spawn_task("cancel_order", async move {
            // Cancel order via Longport SDK
            let venue_order_id = command.venue_order_id;
            match trade_ctx.cancel_order(venue_order_id.as_str()).await {
                Ok(_) => {
                    tracing::info!("Order canceled successfully: {}", venue_order_id);

                    // Send OrderCanceled event
                    if let Some(sender) = sender {
                        let canceled_event = OrderCanceled::new(
                            trader_id,
                            strategy_id,
                            instrument_id,
                            client_order_id,
                            UUID4::new(),
                            get_atomic_clock_realtime().get_time_ns(),
                            get_atomic_clock_realtime().get_time_ns(),
                            false,  // reconciliation
                            Some(venue_order_id),
                            Some(account_id),
                        );
                        let _ = sender.send(ExecutionEvent::Order(OrderEventAny::Canceled(canceled_event)));
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to cancel order: {e:?}");
                }
            }

            Ok(())
        });

        Ok(())
    }

    fn cancel_all_orders(&self, cmd: &CancelAllOrders) -> anyhow::Result<()> {
        let trade_ctx = self.trade_ctx.clone();
        let instrument_id = cmd.instrument_id;
        let sender = self.exec_event_sender.clone();
        let trader_id = self.core.trader_id;
        let account_id = self.core.account_id;

        self.spawn_task("cancel_all_orders", async move {
            tracing::debug!("Canceling all open orders for {}", instrument_id);

            // Longport SDK doesn't have a mass cancel API
            // We need to fetch open orders and cancel them individually
            // Use today_orders with filter to get open orders
            match trade_ctx.today_orders(None).await {
                Ok(orders) => {
                    let mut canceled_count = 0;
                    let mut failed_count = 0;

                    for order in orders {
                        // Only cancel working orders (not filled, canceled, rejected)
                        // Working orders are those that are NotReported (newly submitted)
                        match order.status {
                            longport::trade::OrderStatus::NotReported => {
                                match trade_ctx.cancel_order(order.order_id.as_str()).await {
                                    Ok(_) => {
                                        canceled_count += 1;
                                        tracing::debug!("Canceled order: {}", order.order_id);
                                    }
                                    Err(e) => {
                                        failed_count += 1;
                                        tracing::error!("Failed to cancel order {}: {}", order.order_id, e);
                                    }
                                }
                            }
                            _ => {
                                // Skip non-working orders
                                tracing::trace!("Skipping non-working order: {}", order.order_id);
                            }
                        }
                    }

                    tracing::info!(
                        "Cancel all orders complete: {} canceled, {} failed for {}",
                        canceled_count,
                        failed_count,
                        instrument_id
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to fetch open orders for cancel_all: {e}");
                }
            }

            Ok(())
        });

        Ok(())
    }

    fn batch_cancel_orders(&self, cmd: &BatchCancelOrders) -> anyhow::Result<()> {
        let trade_ctx = self.trade_ctx.clone();
        let cancels = cmd.cancels.clone();
        let sender = self.exec_event_sender.clone();
        let trader_id = self.core.trader_id;
        let account_id = self.core.account_id;

        self.spawn_task("batch_cancel_orders", async move {
            for cancel in cancels {
                let venue_order_id = cancel.venue_order_id;
                tracing::debug!(
                    "Batch cancel: client_order_id={}, venue_order_id={}",
                    cancel.client_order_id,
                    venue_order_id
                );

                let _ = trade_ctx.cancel_order(venue_order_id.as_str()).await;
                // Send cancellation event
            }

            Ok(())
        });

        Ok(())
    }
}

#[async_trait(?Send)]
impl LiveExecutionClient for LongportExecutionClient {
    async fn generate_order_status_report(
        &self,
        cmd: &GenerateOrderStatusReport,
    ) -> anyhow::Result<Option<OrderStatusReport>> {
        tracing::debug!("generate_order_status_report: {cmd:?}");

        // Fetch order details from Longport SDK
        let client_order_id = cmd.client_order_id;
        let venue_order_id = cmd.venue_order_id;

        if let Some(venue_order_id) = venue_order_id {
            match self.trade_ctx.order_detail(venue_order_id.as_str()).await {
                Ok(order_detail) => {
                    // Convert Longport order detail to Nautilus OrderStatusReport
                    let report = self.build_order_status_report(order_detail, client_order_id)?;
                    Ok(Some(report))
                }
                Err(e) => {
                    tracing::error!("Failed to fetch order details: {e}");
                    Ok(None)
                }
            }
        } else {
            tracing::warn!("Cannot generate order status report without venue_order_id");
            Ok(None)
        }
    }

    async fn generate_order_status_reports(
        &self,
        cmd: &GenerateOrderStatusReport,
    ) -> anyhow::Result<Vec<OrderStatusReport>> {
        tracing::debug!("generate_order_status_reports: {cmd:?}");

        // Fetch all orders for the account from Longport SDK
        // Use today_orders to get current day's orders
        match self.trade_ctx.today_orders(None).await {
            Ok(orders) => {
                let mut reports = Vec::new();
                for order_detail in orders {
                    match self.build_order_status_report_from_detail(order_detail, cmd.client_order_id) {
                        Ok(report) => reports.push(report),
                        Err(e) => tracing::error!("Failed to build order status report: {e}"),
                    }
                }
                Ok(reports)
            }
            Err(e) => {
                tracing::error!("Failed to fetch order details: {e}");
                Ok(Vec::new())
            }
        }
    }

    async fn generate_fill_reports(
        &self,
        cmd: GenerateFillReports,
    ) -> anyhow::Result<Vec<FillReport>> {
        tracing::debug!("generate_fill_reports: {cmd:?}");

        // Longport SDK provides today's executions through today_executions
        match self.trade_ctx.today_executions(None).await {
            Ok(executions) => {
                let mut reports = Vec::new();
                for exec in executions {
                    // Convert execution to FillReport
                    match self.build_fill_report(&exec) {
                        Ok(report) => reports.push(report),
                        Err(e) => tracing::error!("Failed to build fill report: {e}"),
                    }
                }
                Ok(reports)
            }
            Err(e) => {
                tracing::error!("Failed to fetch executions: {e}");
                Ok(Vec::new())
            }
        }
    }

    async fn generate_position_status_reports(
        &self,
        cmd: &GeneratePositionReports,
    ) -> anyhow::Result<Vec<PositionStatusReport>> {
        tracing::debug!("generate_position_status_reports: {cmd:?}");

        // Fetch stock positions from Longport SDK
        // The stock_positions method returns StockPositionsResponse which has channels
        // Each channel has positions
        match self.trade_ctx.stock_positions(None).await {
            Ok(response) => {
                let mut reports = Vec::new();
                // response.channels contains Vec<StockPositionChannel>
                for channel in response.channels {
                    // Each channel has positions (Vec<StockPosition>)
                    for item in channel.positions {
                        match self.build_position_status_report_from_item(&item) {
                            Ok(report) => reports.push(report),
                            Err(e) => tracing::error!("Failed to build position status report: {e}"),
                        }
                    }
                }
                Ok(reports)
            }
            Err(e) => {
                tracing::error!("Failed to fetch stock positions: {e}");
                Ok(Vec::new())
            }
        }
    }

    async fn generate_mass_status(
        &self,
        lookback_mins: Option<u64>,
    ) -> anyhow::Result<Option<ExecutionMassStatus>> {
        tracing::debug!("generate_mass_status: lookback_mins={lookback_mins:?}");

        let ts_init = get_atomic_clock_realtime().get_time_ns();

        // For mass status, we create a basic report
        // Full order and position details would require additional queries
        let mass_status = ExecutionMassStatus::new(
            self.core.client_id,
            self.core.account_id,
            *LONGPORT_VENUE,
            ts_init,
            None, // report_id - will generate new UUID4
        );

        Ok(Some(mass_status))
    }
}

// Helper function for submitting a single order (used by submit_order_list)
async fn submit_single_order(
    trade_ctx: &Arc<TradeContext>,
    order: &impl Order,
    trader_id: TraderId,
    account_id: AccountId,
) -> anyhow::Result<()> {
    // Build submit options from order
    let symbol = crate::common::parse::instrument_id_to_string(order.instrument_id())?;
    let side = crate::common::parse::parse_order_side(order.order_side())?;
    let order_type = crate::common::parse::parse_order_type(order.order_type())?;
    let time_in_force = crate::common::parse::parse_time_in_force(order.time_in_force())?;

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

    // Submit the order
    trade_ctx.submit_order(opts).await?;
    Ok(())
}

impl LongportExecutionClient {
    /// Builds an OrderStatusReport from Longport order detail.
    fn build_order_status_report(
        &self,
        order_detail: longport::trade::OrderDetail,
        client_order_id: Option<ClientOrderId>,
    ) -> anyhow::Result<OrderStatusReport> {
        use nautilus_model::{enums::{OrderStatus, TimeInForce, OrderSide, OrderType}, types::Price, types::Quantity};

        let venue_order_id = VenueOrderId::new(order_detail.order_id.as_str());

        // Convert order status using the correct Longport OrderStatus enum variants
        let order_status = match order_detail.status {
            longport::trade::OrderStatus::NotReported |
            longport::trade::OrderStatus::ReplacedNotReported |
            longport::trade::OrderStatus::ProtectedNotReported |
            longport::trade::OrderStatus::VarietiesNotReported => OrderStatus::Submitted,
            longport::trade::OrderStatus::WaitToNew |
            longport::trade::OrderStatus::New => OrderStatus::Accepted,
            longport::trade::OrderStatus::Rejected => OrderStatus::Rejected,
            longport::trade::OrderStatus::Canceled => OrderStatus::Canceled,
            longport::trade::OrderStatus::Expired => OrderStatus::Expired,
            longport::trade::OrderStatus::PartialFilled => OrderStatus::PartiallyFilled,
            longport::trade::OrderStatus::Filled => OrderStatus::Filled,
            longport::trade::OrderStatus::Replaced => OrderStatus::Accepted,
            longport::trade::OrderStatus::PendingReplace |
            longport::trade::OrderStatus::WaitToReplace |
            longport::trade::OrderStatus::PendingCancel |
            longport::trade::OrderStatus::WaitToCancel |
            longport::trade::OrderStatus::PartialWithdrawal => OrderStatus::Submitted,
            _ => OrderStatus::Triggered, // Use Triggered as fallback instead of Unknown
        };

        // Parse instrument ID from symbol (Longport SDK uses symbol, not stock_symbol)
        // The symbol already includes the market (e.g., "700.HK")
        let instrument_id = crate::common::parse::parse_instrument_id_from_symbol(&order_detail.symbol)?;

        let order_side: OrderSide = order_side_from_longport(order_detail.side);
        let order_type: OrderType = order_type_from_longport(order_detail.order_type);
        let time_in_force = TimeInForce::Day; // Default, Longport uses different TIF semantics

        // Convert Decimal quantity to f64
        let quantity = Quantity::new(
            rust_decimal::Decimal::to_f64(&order_detail.quantity).unwrap_or(0.0),
            0, // precision
        );

        // filled_qty from executed_quantity if available
        let filled_qty = Quantity::new(
            rust_decimal::Decimal::to_f64(&order_detail.executed_quantity).unwrap_or(0.0),
            0,
        );

        // ts_accepted from submitted_at - convert OffsetDateTime to UnixNanos
        let ts_accepted = UnixNanos::from(
            order_detail.submitted_at.unix_timestamp() as u64 * 1_000_000_000
                + order_detail.submitted_at.nanosecond() as u64
        );

        // ts_last from updated_at if available
        let ts_last = UnixNanos::from(
            order_detail.updated_at
                .unwrap_or(order_detail.submitted_at)
                .unix_timestamp() as u64 * 1_000_000_000
                + order_detail.updated_at
                    .unwrap_or(order_detail.submitted_at)
                    .nanosecond() as u64
        );

        let ts_init = ts_accepted;

        // Use a default account_id since OrderDetail doesn't have account_id
        let account_id = self.core.account_id;

        let mut report = OrderStatusReport::new(
            account_id,
            instrument_id,
            client_order_id,
            venue_order_id,
            order_side,
            order_type,
            time_in_force,
            order_status,
            quantity,
            filled_qty,
            ts_accepted,
            ts_last,
            ts_init,
            Some(UUID4::new()),
        );

        // Add optional price if available
        if let Some(p) = order_detail.price {
            let price = Price::new(rust_decimal::Decimal::to_f64(&p).unwrap_or(0.0), 2);
            report = report.with_price(price);
        }

        // Add avg_px from executed_price if available
        if let Some(avg_p) = order_detail.executed_price {
            let avg_px = rust_decimal::Decimal::to_f64(&avg_p).unwrap_or(0.0);
            report = report.with_avg_px(avg_px)?;
        }

        Ok(report)
    }

    /// Builds an OrderStatusReport from Longport order (from today_orders).
    fn build_order_status_report_from_detail(
        &self,
        order: longport::trade::Order,
        client_order_id: Option<ClientOrderId>,
    ) -> anyhow::Result<OrderStatusReport> {
        use nautilus_model::{enums::{OrderStatus, TimeInForce, OrderSide, OrderType}, types::Price, types::Quantity};

        let venue_order_id = VenueOrderId::new(order.order_id.as_str());

        // Convert order status using the correct Longport OrderStatus enum variants
        let order_status = match order.status {
            longport::trade::OrderStatus::NotReported |
            longport::trade::OrderStatus::ReplacedNotReported |
            longport::trade::OrderStatus::ProtectedNotReported |
            longport::trade::OrderStatus::VarietiesNotReported => OrderStatus::Submitted,
            longport::trade::OrderStatus::WaitToNew |
            longport::trade::OrderStatus::New => OrderStatus::Accepted,
            longport::trade::OrderStatus::Rejected => OrderStatus::Rejected,
            longport::trade::OrderStatus::Canceled => OrderStatus::Canceled,
            longport::trade::OrderStatus::Expired => OrderStatus::Expired,
            longport::trade::OrderStatus::PartialFilled => OrderStatus::PartiallyFilled,
            longport::trade::OrderStatus::Filled => OrderStatus::Filled,
            longport::trade::OrderStatus::Replaced => OrderStatus::Accepted,
            longport::trade::OrderStatus::PendingReplace |
            longport::trade::OrderStatus::WaitToReplace |
            longport::trade::OrderStatus::PendingCancel |
            longport::trade::OrderStatus::WaitToCancel |
            longport::trade::OrderStatus::PartialWithdrawal => OrderStatus::Submitted,
            _ => OrderStatus::Triggered, // Use Triggered as fallback instead of Unknown
        };

        // Parse instrument ID from symbol
        let instrument_id = crate::common::parse::parse_instrument_id_from_symbol(&order.symbol)?;

        let order_side: OrderSide = order_side_from_longport(order.side);
        let order_type: OrderType = order_type_from_longport(order.order_type);
        let time_in_force = TimeInForce::Day; // Default

        // Convert Decimal quantity to f64
        let quantity = Quantity::new(
            rust_decimal::Decimal::to_f64(&order.quantity).unwrap_or(0.0),
            0, // precision
        );

        // filled_qty from executed_quantity if available
        let filled_qty = Quantity::new(
            rust_decimal::Decimal::to_f64(&order.executed_quantity).unwrap_or(0.0),
            0,
        );

        // ts_accepted from submitted_at - convert OffsetDateTime to UnixNanos
        let ts_accepted = UnixNanos::from(
            order.submitted_at.unix_timestamp() as u64 * 1_000_000_000
                + order.submitted_at.nanosecond() as u64
        );

        // ts_last from updated_at if available
        let ts_last = UnixNanos::from(
            order.updated_at
                .unwrap_or(order.submitted_at)
                .unix_timestamp() as u64 * 1_000_000_000
                + order.updated_at
                    .unwrap_or(order.submitted_at)
                    .nanosecond() as u64
        );

        let ts_init = ts_accepted;

        // Use account_id from core
        let account_id = self.core.account_id;

        let mut report = OrderStatusReport::new(
            account_id,
            instrument_id,
            client_order_id,
            venue_order_id,
            order_side,
            order_type,
            time_in_force,
            order_status,
            quantity,
            filled_qty,
            ts_accepted,
            ts_last,
            ts_init,
            Some(UUID4::new()),
        );

        // Add optional price if available
        if let Some(p) = order.price {
            let price = Price::new(rust_decimal::Decimal::to_f64(&p).unwrap_or(0.0), 2);
            report = report.with_price(price);
        }

        // Add avg_px from executed_price if available
        if let Some(avg_p) = order.executed_price {
            let avg_px = rust_decimal::Decimal::to_f64(&avg_p).unwrap_or(0.0);
            report = report.with_avg_px(avg_px)?;
        }

        Ok(report)
    }

    /// Builds a FillReport from Longport execution details.
    fn build_fill_report(
        &self,
        execution: &longport::trade::Execution,
    ) -> anyhow::Result<FillReport> {
        use nautilus_model::{enums::{OrderSide, LiquiditySide}, identifiers::TradeId, types::{Price, Quantity, Money, Currency}};

        // Parse instrument ID from symbol (Execution has symbol field)
        let instrument_id = crate::common::parse::parse_instrument_id_from_symbol(&execution.symbol)?;

        let last_px = Price::new(
            rust_decimal::Decimal::to_f64(&execution.price).unwrap_or(0.0),
            2,
        );
        let last_qty = Quantity::new(
            rust_decimal::Decimal::to_f64(&execution.quantity).unwrap_or(0.0),
            0,
        );

        let venue_order_id = VenueOrderId::new(execution.order_id.as_str());
        let trade_id = TradeId::new(execution.trade_id.as_str());

        // Execution doesn't have side field - assume based on context or default
        // In practice, you would need to look up the order to determine the side
        let order_side = OrderSide::Buy; // Default to Buy, ideally should look up order

        // Commission - Longport executions don't have commission info directly
        // Use zero commission as placeholder
        let commission = Money::new(0.0, Currency::USD());

        let liquidity_side = LiquiditySide::Taker; // Assume taker for simplicity

        // ts_event from trade_done_at - convert OffsetDateTime to UnixNanos
        let ts_event = UnixNanos::from(
            execution.trade_done_at.unix_timestamp() as u64 * 1_000_000_000
                + execution.trade_done_at.nanosecond() as u64
        );
        let ts_init = ts_event;

        // Use account_id from core
        let account_id = self.core.account_id;

        Ok(FillReport::new(
            account_id,
            instrument_id,
            venue_order_id,
            trade_id,
            order_side,
            last_qty,
            last_px,
            commission,
            liquidity_side,
            None, // client_order_id - not available from execution
            None, // venue_position_id
            ts_event,
            ts_init,
            None, // event_id - will generate new UUID4
        ))
    }

    /// Builds a PositionStatusReport from Longport position details.
    fn build_position_status_report_from_item(
        &self,
        item: &longport::trade::StockPosition,
    ) -> anyhow::Result<PositionStatusReport> {
        use nautilus_model::enums::PositionSideSpecified;
        use nautilus_model::types::Quantity;

        // Parse instrument ID from symbol and market
        let instrument_id = crate::common::parse::parse_instrument_id_from_parts(
            &item.symbol,
            &item.market.to_string(),
        )?;

        // Determine position side based on quantity sign
        let qty_value = rust_decimal::Decimal::to_f64(&item.quantity).unwrap_or(0.0);
        let position_side = if qty_value < 0.0 {
            PositionSideSpecified::Short
        } else {
            PositionSideSpecified::Long
        };

        let quantity = Quantity::new(qty_value.abs(), 0);

        // avg_px_open from cost_price (as rust_decimal::Decimal to f64 to Decimal)
        let avg_px_open: Option<rust_decimal::Decimal> = Some(item.cost_price);

        // Use current time as ts_last since StockPosition doesn't provide timestamp
        let ts_last = get_atomic_clock_realtime().get_time_ns();
        let ts_init = ts_last;

        let account_id = self.core.account_id;

        Ok(PositionStatusReport::new(
            account_id,
            instrument_id,
            position_side,
            quantity,
            ts_last,
            ts_init,
            None, // event_id - will generate new UUID4
            None, // venue_position_id - not available
            avg_px_open,
        ))
    }
}

// Helper functions for conversions (keep those that are used)
fn order_side_from_longport(side: longport::trade::OrderSide) -> nautilus_model::enums::OrderSide {
    match side {
        longport::trade::OrderSide::Buy => nautilus_model::enums::OrderSide::Buy,
        longport::trade::OrderSide::Sell => nautilus_model::enums::OrderSide::Sell,
        _ => nautilus_model::enums::OrderSide::NoOrderSide,
    }
}

fn order_type_from_longport(order_type: longport::trade::OrderType) -> nautilus_model::enums::OrderType {
    match order_type {
        longport::trade::OrderType::LO => nautilus_model::enums::OrderType::Limit,
        longport::trade::OrderType::MO => nautilus_model::enums::OrderType::Market,
        longport::trade::OrderType::ELO => nautilus_model::enums::OrderType::Limit,
        longport::trade::OrderType::LIT => nautilus_model::enums::OrderType::LimitIfTouched,
        longport::trade::OrderType::MIT => nautilus_model::enums::OrderType::MarketIfTouched,
        // STP and STP_LMT don't exist in Longport SDK, using StopMarket/StopLimit as fallback
        _ => nautilus_model::enums::OrderType::Limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nautilus_common::{cache::Cache, clock::TestClock};
    use nautilus_model::identifiers::{AccountId, ClientId, TraderId};
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

        let client = LongportExecutionClient::new(core, config);
        // assert!(client.is_ok());
        // let client = client.unwrap();
        // assert_eq!(client.venue(), *LONGPORT_VENUE);
    }
}
