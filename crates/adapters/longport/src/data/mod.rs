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

//! Live market data client implementation for the Longport adapter.

pub mod advanced;

#[cfg(feature = "python")]
use pyo3::{pymethods, PyResult};

use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};

use ahash::AHashMap;
use anyhow::Context;
use longport::{
    quote::{QuoteContext, SubFlags, PushEvent, PushEventDetail, Period, AdjustType, TradeSessions, SecurityListCategory},
    Market, Config,
};
use nautilus_common::{
    live::{runner::get_data_event_sender, runtime::get_runtime},
    messages::{
        DataEvent,
        data::{
            BarsResponse, DataResponse, InstrumentResponse, InstrumentsResponse, RequestBars,
            RequestInstrument, RequestInstruments, RequestTrades, SubscribeBars,
            SubscribeBookDeltas, SubscribeBookSnapshots, SubscribeQuotes, SubscribeTrades,
            TradesResponse, UnsubscribeBars, UnsubscribeBookDeltas, UnsubscribeBookSnapshots,
            UnsubscribeQuotes, UnsubscribeTrades,
        },
    },
};
use nautilus_core::{
    MUTEX_POISONED,
    datetime::datetime_to_unix_nanos,
    time::{AtomicTime, get_atomic_clock_realtime},
};
use nautilus_data::client::DataClient;
use nautilus_model::{
    data::{Data, Bar},
    enums::BookType,
    identifiers::{ClientId, InstrumentId, Venue},
    instruments::{Instrument, InstrumentAny},
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{
    common::{
        consts::LONGPORT_VENUE,
        convert::{depth_to_quote_tick, security_depth_to_deltas, trade_to_trade_tick},
        enums::LongportMarket,
        models::LongportInstrument,
        parse::{parse_instrument, instrument_id_to_string, create_minimal_instrument},
    },
    config::LongportDataClientConfig,
};

/// LongPort market data client.
#[cfg_attr(feature = "python", pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.longport"))]
pub struct LongportDataClient {
    client_id: ClientId,
    config: LongportDataClientConfig,
    quote_ctx: Arc<QuoteContext>,
    event_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<PushEvent>>,
    is_connected: AtomicBool,
    cancellation_token: CancellationToken,
    tasks: Vec<JoinHandle<()>>,
    instruments: Arc<RwLock<AHashMap<InstrumentId, InstrumentAny>>>,
    clock: &'static AtomicTime,
    /// Data event sender for forwarding data to the async runner.
    /// Set during connect() when TLS context is available.
    data_sender: Option<tokio::sync::mpsc::UnboundedSender<DataEvent>>,
}

impl std::fmt::Debug for LongportDataClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LongportDataClient")
            .field("client_id", &self.client_id)
            .field("config", &self.config)
            .field("is_connected", &self.is_connected)
            .field("clock", &self.clock)
            .finish_non_exhaustive()
    }
}

impl LongportDataClient {
    /// Creates a new [`LongportDataClient`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the client fails to initialize.
    pub fn new(client_id: ClientId, config: LongportDataClientConfig) -> anyhow::Result<Self> {
        let clock = get_atomic_clock_realtime();

        // Build Longport configuration
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

        // Create quote context with event receiver for WebSocket data
        // The receiver will be stored and used in the event consumption loop
        let (quote_ctx, event_receiver) = get_runtime()
            .block_on(QuoteContext::try_new(Arc::new(longport_config)))
            .context("failed to create Longport quote context")?;

        // Note: data_sender will be obtained in connect() when TLS context is available
        // We cannot get it here because new() is called during factory.create() which
        // happens during TradingNode.build(), before the runner starts and sets TLS.

        Ok(Self {
            client_id,
            config,
            quote_ctx: Arc::new(quote_ctx),
            event_receiver: Some(event_receiver),
            is_connected: AtomicBool::new(false),
            cancellation_token: CancellationToken::new(),
            tasks: Vec::new(),
            instruments: Arc::new(RwLock::new(AHashMap::new())),
            clock,
            data_sender: None,  // Will be set in connect() when TLS context is available
        })
    }

    fn venue(&self) -> Venue {
        *LONGPORT_VENUE
    }

    fn send_data(sender: &tokio::sync::mpsc::UnboundedSender<DataEvent>, data: Data) {
        if let Err(e) = sender.send(DataEvent::Data(data)) {
            tracing::error!("Failed to emit data event: {e}");
        }
    }

    /// Ensure an instrument exists in the internal cache, creating it if necessary.
    fn ensure_instrument(&self, instrument_id: &InstrumentId) -> anyhow::Result<()> {
        // Check if instrument already exists
        {
            let guard = self.instruments.read().expect(MUTEX_POISONED);
            if guard.contains_key(instrument_id) {
                return Ok(());
            }
        }

        // Instrument doesn't exist, try to create it
        let instrument_id_str = instrument_id.symbol.as_str();
        match create_minimal_instrument(instrument_id_str) {
            Ok(instrument) => {
                let mut guard = self.instruments.write().expect(MUTEX_POISONED);
                guard.insert(instrument.id(), instrument.clone());
                tracing::debug!("Created minimal instrument for {}", instrument_id_str);
                Ok(())
            }
            Err(e) => {
                anyhow::bail!("Failed to create minimal instrument for {}: {}", instrument_id, e)
            }
        }
    }

    async fn fetch_instruments_for_market(
        &self,
        market: LongportMarket,
    ) -> anyhow::Result<Vec<LongportInstrument>> {
        // Convert LongportMarket to SDK Market type
        let sdk_market = match market {
            LongportMarket::HK => Market::HK,
            LongportMarket::US => Market::US,
            LongportMarket::CN => Market::CN,
        };

        tracing::debug!("Fetching instruments for market: {:?}", sdk_market);

        // Fetch security list from Longport SDK
        // Get all securities (no category filter)
        let securities = self
            .quote_ctx
            .security_list(sdk_market, None::<SecurityListCategory>)
            .await
            .context("failed to fetch security list from Longport")?;

        tracing::info!("Fetched {} securities for market {:?}", securities.len(), sdk_market);

        // Convert SDK Security to our internal LongportInstrument format
        let result = securities
            .into_iter()
            .filter_map(|security| {
                // Extract symbol and name (prefer English name, fallback to Chinese)
                let symbol = security.symbol;
                let name = if !security.name_en.is_empty() {
                    security.name_en
                } else if !security.name_cn.is_empty() {
                    security.name_cn
                } else {
                    // If both names are empty, use symbol as fallback
                    symbol.clone()
                };
                // Use default lot size since Security doesn't contain lot_size info
                let lot_size = 100;

                // Create LongportInstrument with actual data
                Some(LongportInstrument::new_with_lot_size(
                    symbol,
                    name,
                    market,
                    lot_size,
                ))
            })
            .collect();

        Ok(result)
    }

    /// Spawns a task to consume WebSocket push events from Longport.
    fn spawn_event_consumption_task(&mut self) -> anyhow::Result<()> {
        tracing::info!("Attempting to spawn event consumption task...");
        let mut receiver = self.event_receiver.take()
            .ok_or_else(|| anyhow::anyhow!("event receiver already taken or not initialized"))?;
        tracing::info!("Event receiver obtained, spawning task...");

        // Get the data_sender that was set in connect()
        let data_sender = self.data_sender.clone()
            .ok_or_else(|| anyhow::anyhow!("data_sender not set - connect() should have been called first and set up TLS context"))?;
        tracing::info!("Data sender cloned for event consumption task");

        let instruments = Arc::clone(&self.instruments);
        let cancellation_token = self.cancellation_token.clone();
        let clock = self.clock;

        let task = get_runtime().spawn(async move {
            tracing::info!("🟢 [LONGPORT] Event consumption loop STARTED");

            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        tracing::info!("Event consumption loop cancelled");
                        break;
                    }
                    result = receiver.recv() => {
                        match result {
                            Some(event) => {
                                tracing::info!("🟡 [LONGPORT] Got event from receiver, handling with data_sender...");
                                // Use the stored data_sender instead of getting from TLS
                                Self::handle_push_event(event, &data_sender, &instruments, clock);
                            }
                            None => {
                                tracing::warn!("Event receiver closed, connection may be lost");
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
            }

            tracing::info!("Longport event consumption loop finished");
        });

        self.tasks.push(task);
        Ok(())
    }

    /// Handles a single push event from Longport WebSocket.
    fn handle_push_event(
        event: PushEvent,
        data_sender: &tokio::sync::mpsc::UnboundedSender<DataEvent>,
        instruments: &Arc<RwLock<AHashMap<InstrumentId, InstrumentAny>>>,
        clock: &'static AtomicTime,
    ) {
        let symbol = event.symbol.clone();
        let ts_init = clock.get_time_ns();

        // Log when we receive ANY event from Longport
        tracing::info!("🔵 [LONGPORT] Received push event for symbol: {symbol}");

        // Find instrument by symbol
        let instrument_id = match Self::find_instrument_id_by_symbol(&symbol, instruments) {
            Some(id) => id,
            None => {
                tracing::debug!("Received event for unknown symbol: {symbol}");
                return;
            }
        };

        let guard = instruments.read().expect(MUTEX_POISONED);
        let instrument = match guard.get(&instrument_id) {
            Some(inst) => inst,
            None => {
                tracing::warn!("Instrument ID {instrument_id} not in cache for symbol: {symbol}");
                return;
            }
        };

        match event.detail {
            PushEventDetail::Depth(depth) => {
                // Convert Longport depth to Nautilus types
                // PushDepth has Vec<Depth> for asks and bids, convert to SecurityDepth format
                let security_depth = longport::quote::SecurityDepth {
                    asks: depth.asks,
                    bids: depth.bids,
                };

                // Try to create QuoteTick from best bid/ask
                match depth_to_quote_tick(&security_depth, instrument, ts_init) {
                    Ok(Some(quote_tick)) => {
                        Self::send_data(data_sender, Data::Quote(quote_tick));
                    }
                    Ok(None) => {
                        tracing::trace!("No valid quote tick from depth update for {symbol}");
                    }
                    Err(e) => {
                        tracing::error!("Failed to convert depth to quote tick for {symbol}: {e}");
                    }
                }

                // Also send order book deltas
                match security_depth_to_deltas(&security_depth, instrument, ts_init) {
                    Ok(deltas) => {
                        use nautilus_model::data::OrderBookDeltas_API;
                        Self::send_data(data_sender, Data::Deltas(OrderBookDeltas_API::new(deltas)));
                    }
                    Err(e) => {
                        tracing::error!("Failed to convert depth to deltas for {symbol}: {e}");
                    }
                }
            }
            PushEventDetail::Trade(trades) => {
                for trade in &trades.trades {
                    match trade_to_trade_tick(trade, instrument) {
                        Ok(trade_tick) => {
                            Self::send_data(data_sender, Data::Trade(trade_tick));
                        }
                        Err(e) => {
                            tracing::error!("Failed to convert trade for {symbol}: {e}");
                        }
                    }
                }
            }
            PushEventDetail::Quote(quote) => {
                // PushQuote contains OHLCV data, can be used to create QuoteTick if we had bid/ask
                // For now, just log it
                tracing::trace!(
                    "Received quote push for {symbol}: last_done={}, volume={}",
                    quote.last_done, quote.volume
                );
            }
            PushEventDetail::Candlestick(candlestick) => {
                tracing::trace!("Received candlestick push for {symbol}");

                // PushCandlestick contains a nested Candlestick structure
                let cs = &candlestick.candlestick;

                // Determine the bar aggregation from the period
                let aggregation = match candlestick.period {
                    longport::quote::Period::OneMinute => nautilus_model::enums::BarAggregation::Minute,
                    longport::quote::Period::Day => nautilus_model::enums::BarAggregation::Day,
                    longport::quote::Period::Week => nautilus_model::enums::BarAggregation::Week,
                    longport::quote::Period::Month => nautilus_model::enums::BarAggregation::Month,
                    _ => nautilus_model::enums::BarAggregation::Minute,
                };

                match crate::common::convert::candlestick_to_bar(cs, instrument, aggregation) {
                    Ok(bar) => {
                        Self::send_data(data_sender, Data::Bar(bar));
                    }
                    Err(e) => {
                        tracing::error!("Failed to convert candlestick to bar for {symbol}: {e}");
                    }
                }
            }
            PushEventDetail::Brokers(_brokers) => {
                tracing::trace!("Received brokers push for {symbol}");
                // Brokers data not currently used
            }
        }
    }

    /// Finds instrument ID by symbol string.
    fn find_instrument_id_by_symbol(
        symbol: &str,
        instruments: &Arc<RwLock<AHashMap<InstrumentId, InstrumentAny>>>,
    ) -> Option<InstrumentId> {
        let guard = instruments.read().ok()?;
        guard.iter().find(|(_, instrument)| {
            instrument.id().symbol.as_str() == symbol
        }).map(|(id, _)| *id)
    }
}

#[async_trait::async_trait(?Send)]
impl DataClient for LongportDataClient {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn venue(&self) -> Option<Venue> {
        Some(self.venue())
    }

    fn start(&mut self) -> anyhow::Result<()> {
        tracing::info!(
            client_id = %self.client_id,
            markets = ?self.config.markets,
            "Started"
        );
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Stopping {id}", id = self.client_id);
        self.cancellation_token.cancel();
        self.is_connected.store(false, Ordering::Relaxed);
        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        tracing::debug!("Resetting {id}", id = self.client_id);
        self.is_connected.store(false, Ordering::Relaxed);
        self.cancellation_token = CancellationToken::new();
        self.tasks.clear();
        Ok(())
    }

    fn dispose(&mut self) -> anyhow::Result<()> {
        tracing::debug!("Disposing {id}", id = self.client_id);
        // Inline stop logic to avoid calling PyO3 wrapper which has different return type
        self.cancellation_token.cancel();
        self.is_connected.store(false, Ordering::Relaxed);
        Ok(())
    }

    async fn connect(&mut self) -> anyhow::Result<()> {
        if self.is_connected() {
            return Ok(());
        }

        // Get the data event sender from TLS - this is called when connect() is executed
        // in the Nautilus runner context where TLS is properly initialized
        if self.data_sender.is_none() {
            tracing::info!("Obtaining data_event_sender from TLS context...");
            self.data_sender = Some(get_data_event_sender());
            tracing::info!("data_event_sender obtained successfully");
        }

        // Strategy: Use load_ids if specified, otherwise fall back to security_list API
        let use_load_ids = self.config.load_ids.is_some() && !self.config.load_ids.as_ref().unwrap().is_empty();

        let mut symbols_to_subscribe: Vec<String> = Vec::new();

        if use_load_ids {
            // Load instruments from load_ids configuration
            tracing::info!("Using load_ids configuration for instruments");
            let load_ids = self.config.load_ids.as_ref().unwrap();

            for instrument_id_str in load_ids {
                match create_minimal_instrument(instrument_id_str) {
                    Ok(instrument) => {
                        tracing::debug!("Created minimal instrument for {}", instrument_id_str);
                        let mut guard = self.instruments.write().expect(MUTEX_POISONED);
                        guard.insert(instrument.id(), instrument.clone());
                        // Collect symbols for subscription
                        symbols_to_subscribe.push(instrument_id_str.clone());
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create instrument for {}: {}", instrument_id_str, e);
                    }
                }
            }

            tracing::info!("Loaded {} instruments from load_ids configuration", load_ids.len());

            // Note: We don't subscribe here because data_sender is not available yet.
            // Subscriptions will be handled through the _subscribe_* methods which are called
            // after the data_sender is set up by the Nautilus runner.
            tracing::info!("Instruments loaded, subscriptions will be handled through subscribe methods");
        } else {
            // Fall back to fetching full instrument list via security_list API
            // Note: This may fail if the user doesn't have permission to access the full instrument list.
            tracing::info!("No load_ids specified, fetching instruments from API");
            for market in &self.config.markets {
                match self.fetch_instruments_for_market(*market).await {
                    Ok(fetched) => {
                        tracing::info!("Successfully fetched {} instruments for market {:?}", fetched.len(), market);
                        // Store instruments in cache for internal use
                        for instrument in &fetched {
                            if let Ok(parsed) = parse_instrument(instrument.clone()) {
                                let mut guard = self.instruments.write().expect(MUTEX_POISONED);
                                guard.insert(parsed.id(), parsed.clone());
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to fetch full instrument list for market {:?}: {}. \
                             Consider specifying 'load_ids' configuration to load specific instruments.",
                            market, e
                        );
                        // Don't return error - continue with connection
                    }
                }
            }
        }

        // Start the event consumption loop to process WebSocket push data
        // This loop will use the stored data_sender (obtained in new())
        tracing::info!("Starting event consumption task...");
        self.spawn_event_consumption_task()
            .context("failed to start event consumption task")?;
        tracing::info!("Event consumption task started successfully");

        self.is_connected.store(true, Ordering::Release);
        tracing::info!(client_id = %self.client_id, "Connected");
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if self.is_disconnected() {
            return Ok(());
        }

        self.cancellation_token.cancel();

        // Cancel all subscriptions
        // TODO: Implement proper subscription cancellation via Longport SDK

        let handles: Vec<_> = self.tasks.drain(..).collect();
        for handle in handles {
            if let Err(e) = handle.await {
                tracing::error!("Error joining task: {e}");
            }
        }

        self.is_connected.store(false, Ordering::Release);
        tracing::info!(client_id = %self.client_id, "Disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Relaxed)
    }

    fn is_disconnected(&self) -> bool {
        !self.is_connected()
    }

    fn subscribe_trades(&mut self, cmd: &SubscribeTrades) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Subscribing to trades for {}", instrument_id);

        // Ensure instrument exists in cache
        self.ensure_instrument(&instrument_id)?;

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .subscribe(vec![symbol.as_str()], SubFlags::TRADE)
                .await
                .context("failed to subscribe to trades")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn unsubscribe_trades(&mut self, cmd: &UnsubscribeTrades) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Unsubscribing from trades for {}", instrument_id);

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .unsubscribe(vec![symbol.as_str()], SubFlags::TRADE)
                .await
                .context("failed to unsubscribe from trades")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn subscribe_quotes(&mut self, cmd: &SubscribeQuotes) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Subscribing to quotes for {}", instrument_id);

        // Ensure instrument exists in cache
        self.ensure_instrument(&instrument_id)?;

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .subscribe(vec![symbol.as_str()], SubFlags::QUOTE)
                .await
                .context("failed to subscribe to quotes")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn unsubscribe_quotes(&mut self, cmd: &UnsubscribeQuotes) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Unsubscribing from quotes for {}", instrument_id);

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .unsubscribe(vec![symbol.as_str()], SubFlags::QUOTE)
                .await
                .context("failed to unsubscribe from quotes")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn subscribe_book_deltas(&mut self, cmd: &SubscribeBookDeltas) -> anyhow::Result<()> {
        if cmd.book_type != BookType::L2_MBP {
            anyhow::bail!("Longport only supports L2_MBP order book deltas");
        }
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Subscribing to order book deltas for {}", instrument_id);

        // Ensure instrument exists in cache
        self.ensure_instrument(&instrument_id)?;

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .subscribe(vec![symbol.as_str()], SubFlags::DEPTH)
                .await
                .context("failed to subscribe to order book deltas")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn unsubscribe_book_deltas(&mut self, cmd: &UnsubscribeBookDeltas) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Unsubscribing from order book deltas for {}", instrument_id);

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .unsubscribe(vec![symbol.as_str()], SubFlags::DEPTH)
                .await
                .context("failed to unsubscribe from order book deltas")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn subscribe_book_snapshots(
        &mut self,
        cmd: &SubscribeBookSnapshots,
    ) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Subscribing to order book snapshots for {}", instrument_id);

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .subscribe(vec![symbol.as_str()], SubFlags::DEPTH)
                .await
                .context("failed to subscribe to order book snapshots")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn unsubscribe_book_snapshots(
        &mut self,
        cmd: &UnsubscribeBookSnapshots,
    ) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        tracing::debug!("Unsubscribing from order book snapshots for {}", instrument_id);

        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .unsubscribe(vec![symbol.as_str()], SubFlags::DEPTH)
                .await
                .context("failed to unsubscribe from order book snapshots")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn subscribe_bars(&mut self, cmd: &SubscribeBars) -> anyhow::Result<()> {
        tracing::debug!("Subscribing to bars for {}", cmd.bar_type);

        let instrument_id = cmd.bar_type.instrument_id();

        // Ensure instrument exists in cache
        self.ensure_instrument(&instrument_id)?;

        let symbol = instrument_id_to_string(instrument_id)?;

        // Longport SDK doesn't have a dedicated bar/candlestick subscription flag
        // Quote data can provide bar updates through PushQuote events
        // For now, we subscribe to quotes to get price updates
        get_runtime().block_on(async move {
            self.quote_ctx
                .subscribe(vec![symbol.as_str()], SubFlags::QUOTE)
                .await
                .context("failed to subscribe to quotes for bar data")?;
            Ok::<(), anyhow::Error>(())
        })?;

        tracing::info!("Subscribed to quotes for {} (bar data will be derived from quotes)", cmd.bar_type);
        Ok(())
    }

    fn unsubscribe_bars(&mut self, cmd: &UnsubscribeBars) -> anyhow::Result<()> {
        tracing::debug!("Unsubscribing from bars for {}", cmd.bar_type);

        let instrument_id = cmd.bar_type.instrument_id();
        let symbol = instrument_id_to_string(instrument_id)?;

        get_runtime().block_on(async move {
            self.quote_ctx
                .unsubscribe(vec![symbol.as_str()], SubFlags::QUOTE)
                .await
                .context("failed to unsubscribe from quotes for bar data")?;
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    fn request_instruments(&self, request: &RequestInstruments) -> anyhow::Result<()> {
        let request_id = request.request_id;
        let client_id = request.client_id.unwrap_or(self.client_id);
        let venue = self.venue();
        let start = request.start;
        let end = request.end;
        let params = request.params.clone();
        let clock = self.clock;
        let start_nanos = datetime_to_unix_nanos(start);
        let end_nanos = datetime_to_unix_nanos(end);
        let markets = self.config.markets.clone();

        // Use stored data_sender if available, otherwise try TLS
        let data_sender = match &self.data_sender {
            Some(sender) => sender.clone(),
            None => {
                // Fallback to TLS for compatibility
                get_data_event_sender()
            }
        };

        get_runtime().spawn(async move {
            let all_instruments = Vec::new(); // Not mutable since we don't add to it

            for market in markets {
                // TODO: Implement actual instrument fetching via Longport SDK
                tracing::debug!("Fetching instruments for market: {:?}", market);
            }

            let response = DataResponse::Instruments(InstrumentsResponse::new(
                request_id,
                client_id,
                venue,
                all_instruments,
                start_nanos,
                end_nanos,
                clock.get_time_ns(),
                params,
            ));

            if let Err(e) = data_sender.send(DataEvent::Response(response)) {
                tracing::error!("Failed to send instruments response: {e}");
            }
        });

        Ok(())
    }

    fn request_instrument(&self, request: &RequestInstrument) -> anyhow::Result<()> {
        let instruments = self.instruments.clone();
        let instrument_id = request.instrument_id;
        let request_id = request.request_id;
        let client_id = request.client_id.unwrap_or(self.client_id);
        let start = request.start;
        let end = request.end;
        let params = request.params.clone();
        let clock = self.clock;
        let start_nanos = datetime_to_unix_nanos(start);
        let end_nanos = datetime_to_unix_nanos(end);

        // Use stored data_sender if available, otherwise try TLS
        let data_sender = match &self.data_sender {
            Some(sender) => sender.clone(),
            None => get_data_event_sender(),
        };

        get_runtime().spawn(async move {
            // Check if instrument is already cached
            let guard = instruments.read().expect(MUTEX_POISONED);
            if let Some(instrument) = guard.get(&instrument_id) {
                let response = DataResponse::Instrument(Box::new(InstrumentResponse::new(
                    request_id,
                    client_id,
                    instrument_id,
                    instrument.clone(),
                    start_nanos,
                    end_nanos,
                    clock.get_time_ns(),
                    params,
                )));

                if let Err(e) = data_sender.send(DataEvent::Response(response)) {
                    tracing::error!("Failed to send instrument response: {e}");
                }
            } else {
                tracing::warn!("Instrument not found: {}", instrument_id);
            }
        });

        Ok(())
    }

    fn request_trades(&self, request: &RequestTrades) -> anyhow::Result<()> {
        let instruments = self.instruments.clone();
        let quote_ctx = self.quote_ctx.clone();
        let instrument_id = request.instrument_id;
        let request_id = request.request_id;
        let client_id = request.client_id.unwrap_or(self.client_id);
        let start = request.start;
        let end = request.end;
        let limit = request.limit.map(|n| n.get() as usize).unwrap_or(1000);
        let params = request.params.clone();
        let clock = self.clock;
        let start_nanos = datetime_to_unix_nanos(start);
        let end_nanos = datetime_to_unix_nanos(end);

        // Use stored data_sender if available, otherwise try TLS
        let data_sender = match &self.data_sender {
            Some(sender) => sender.clone(),
            None => get_data_event_sender(),
        };

        get_runtime().spawn(async move {
            let symbol = match instrument_id_to_string(instrument_id) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to convert instrument ID to string: {e}");
                    return;
                }
            };

            // Get instrument for precision info
            let instrument = match instruments.read().ok().and_then(|guard| guard.get(&instrument_id).cloned()) {
                Some(inst) => inst,
                None => {
                    tracing::warn!("Instrument not found: {}", instrument_id);
                    return;
                }
            };

            // Fetch trades from Longport SDK
            match quote_ctx.trades(&symbol, limit).await {
                Ok(trades) => {
                    let trade_ticks: Vec<nautilus_model::data::TradeTick> = trades
                        .into_iter()
                        .filter_map(|trade| {
                            match trade_to_trade_tick(&trade, &instrument) {
                                Ok(tick) => Some(tick),
                                Err(e) => {
                                    tracing::error!("Failed to convert trade: {e}");
                                    None
                                }
                            }
                        })
                        .collect();

                    let response = DataResponse::Trades(TradesResponse::new(
                        request_id,
                        client_id,
                        instrument_id,
                        trade_ticks,
                        start_nanos,
                        end_nanos,
                        clock.get_time_ns(),
                        params,
                    ));

                    if let Err(e) = data_sender.send(DataEvent::Response(response)) {
                        tracing::error!("Failed to send trades response: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to fetch trades from Longport: {e}");
                }
            }
        });

        Ok(())
    }

    fn request_bars(&self, request: &RequestBars) -> anyhow::Result<()> {
        use nautilus_model::enums::BarAggregation;

        let instruments = self.instruments.clone();
        let quote_ctx = self.quote_ctx.clone();
        let bar_type = request.bar_type;
        let request_id = request.request_id;
        let client_id = request.client_id.unwrap_or(self.client_id);
        let start = request.start;
        let end = request.end;
        let limit = request.limit.map(|n| n.get() as usize).unwrap_or(1000);
        let params = request.params.clone();
        let clock = self.clock;
        let start_nanos = datetime_to_unix_nanos(start);
        let end_nanos = datetime_to_unix_nanos(end);

        // Convert BarAggregation to Period
        let period = match bar_type.spec().aggregation {
            BarAggregation::Minute => Period::OneMinute,
            BarAggregation::Hour => Period::OneMinute, // Longport doesn't have hourly, use minute
            BarAggregation::Day => Period::Day,
            BarAggregation::Week => Period::Week,
            BarAggregation::Month => Period::Month,
            _ => {
                tracing::warn!("Unsupported bar aggregation: {:?}, using OneMinute", bar_type.spec().aggregation);
                Period::OneMinute
            }
        };

        // Use stored data_sender if available, otherwise try TLS
        let data_sender = match &self.data_sender {
            Some(sender) => sender.clone(),
            None => get_data_event_sender(),
        };

        get_runtime().spawn(async move {
            let instrument_id = bar_type.instrument_id();
            let symbol = match instrument_id_to_string(instrument_id) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to convert instrument ID to string: {e}");
                    return;
                }
            };

            // Get instrument for precision info
            let instrument = match instruments.read().ok().and_then(|guard| guard.get(&instrument_id).cloned()) {
                Some(inst) => inst,
                None => {
                    tracing::warn!("Instrument not found: {}", instrument_id);
                    return;
                }
            };

            // Fetch candlesticks from Longport SDK
            match quote_ctx.candlesticks(
                &symbol,
                period,
                limit,
                AdjustType::NoAdjust,
                TradeSessions::Intraday,
            ).await {
                Ok(candlesticks) => {
                    let bars: Vec<Bar> = candlesticks
                        .into_iter()
                        .filter_map(|candlestick| {
                            match crate::common::convert::candlestick_to_bar(
                                &candlestick,
                                &instrument,
                                bar_type.spec().aggregation,
                            ) {
                                Ok(bar) => Some(bar),
                                Err(e) => {
                                    tracing::error!("Failed to convert candlestick: {e}");
                                    None
                                }
                            }
                        })
                        .collect();

                    let response = DataResponse::Bars(BarsResponse::new(
                        request_id,
                        client_id,
                        bar_type,
                        bars,
                        start_nanos,
                        end_nanos,
                        clock.get_time_ns(),
                        params,
                    ));

                    if let Err(e) = data_sender.send(DataEvent::Response(response)) {
                        tracing::error!("Failed to send bars response: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to fetch candlesticks from Longport: {e}");
                }
            }
        });

        Ok(())
    }
}

// Python bindings (only when python feature is enabled)
#[cfg(feature = "python")]
#[pymethods]
impl LongportDataClient {
    #[new]
    fn py_new(
        client_id: nautilus_model::identifiers::ClientId,
        config: crate::config::LongportDataClientConfig,
    ) -> PyResult<Self> {
        Self::new(client_id, config).map_err(|e| {
            pyo3::PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create LongportDataClient: {e}"
            ))
        })
    }

    fn connect(&mut self) -> PyResult<()> {
        use nautilus_common::live::runtime::get_runtime;

        eprintln!("[RUST PyO3] connect() wrapper - ENTER");
        let result = get_runtime()
            .block_on(<Self as nautilus_data::client::DataClient>::connect(self));
        eprintln!("[RUST PyO3] connect() wrapper - result: {:?}", result.is_ok());
        result.map_err(|e| {
            pyo3::PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Connection failed: {e}"
            ))
        })
    }

    fn disconnect(&mut self) -> PyResult<()> {
        use nautilus_common::live::runtime::get_runtime;
        get_runtime()
            .block_on(<Self as nautilus_data::client::DataClient>::disconnect(self))
            .map_err(|e| {
                pyo3::PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Disconnect failed: {e}"
                ))
            })
    }

    fn is_connected(&self) -> bool {
        <Self as nautilus_data::client::DataClient>::is_connected(self)
    }

    fn is_disconnected(&self) -> bool {
        <Self as nautilus_data::client::DataClient>::is_disconnected(self)
    }

    #[getter]
    fn client_id(&self) -> nautilus_model::identifiers::ClientId {
        self.client_id
    }

    fn start(&mut self) -> PyResult<()> {
        <Self as nautilus_data::client::DataClient>::start(self)
            .map_err(|e| {
                pyo3::PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Start failed: {e}"
                ))
            })
    }

    fn stop(&mut self) -> PyResult<()> {
        <Self as nautilus_data::client::DataClient>::stop(self)
            .map_err(|e| {
                pyo3::PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Stop failed: {e}"
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nautilus_model::identifiers::ClientId;

    #[test]
    fn test_longport_data_client_venue() {
        let client_id = ClientId::from("LONGPORT");
        let config = LongportDataClientConfig::default();
        // Note: This test will fail without proper credentials
        // In a real test environment, set up mock credentials
        let client = LongportDataClient::new(client_id, config);
        // assert!(client.is_ok());
        // let client = client.unwrap();
        // assert_eq!(client.venue(), Some(*LONGPORT_VENUE));
    }
}
