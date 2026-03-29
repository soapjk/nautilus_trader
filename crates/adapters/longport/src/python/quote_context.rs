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

//! Python bindings for Longport QuoteContext.

use std::sync::Arc;
use std::hash::{Hash, Hasher};
use pyo3::prelude::*;
use longport::{quote::QuoteContext, Config, quote::PushEvent, quote::PushEventDetail};
use nautilus_cryptography::providers::install_cryptographic_provider;
use nautilus_model::{
    data::Data,
    instruments::{InstrumentAny, Instrument},
};
use nautilus_model::python::data::data_to_pycapsule;
use crate::common::parse::create_minimal_instrument;
use crate::common::convert::{timestamp_to_nanos, decimal_to_f64};

/// Python wrapper for Longport QuoteContext.
#[pyclass(name = "QuoteContext", module = "nautilus_trader.core.nautilus_pyo3.longport")]
pub struct PyQuoteContext {
    inner: Option<Arc<QuoteContext>>,
    /// The Tokio runtime - kept alive for spawning async tasks.
    _runtime: Option<Box<tokio::runtime::Runtime>>,
}

impl std::fmt::Debug for PyQuoteContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyQuoteContext")
            .field("inner", &self.inner.is_some())
            .field("has_runtime", &self._runtime.is_some())
            .finish()
    }
}

impl PyQuoteContext {
    /// Create a new PyQuoteContext from a QuoteContext and runtime.
    pub fn from_quote_ctx(quote_ctx: QuoteContext, runtime: tokio::runtime::Runtime) -> Self {
        Self {
            inner: Some(Arc::new(quote_ctx)),
            _runtime: Some(Box::new(runtime)),
        }
    }

    /// Get the inner QuoteContext.
    pub fn inner(&self) -> Option<Arc<QuoteContext>> {
        self.inner.as_ref().map(Arc::clone)
    }

    /// Get the Tokio runtime handle.
    pub fn runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self._runtime.as_ref().map(|r| r.handle().clone())
    }

    /// Extract the inner Arc<QuoteContext>, consuming self.
    pub fn into_inner(self) -> Arc<QuoteContext> {
        self.inner.expect("PyQuoteContext not initialized")
    }
}

#[pymethods]
impl PyQuoteContext {
    /// Close the QuoteContext and release resources.
    pub fn close(&mut self) {
        self.inner = None;
    }

    /// Returns true if the QuoteContext is initialized.
    pub fn is_initialized(&self) -> bool {
        self.inner.is_some()
    }

    fn __repr__(&self) -> String {
        if self.inner.is_some() {
            "QuoteContext(initialized)".to_string()
        } else {
            "QuoteContext(not initialized)".to_string()
        }
    }
}

/// Python wrapper for receiving push events from Longport WebSocket.
///
/// This wrapper holds the actual event receiver from QuoteContext and provides
/// async methods to receive events.
#[pyclass(name = "PushEventReceiver", module = "nautilus_trader.core.nautilus_pyo3.longport")]
pub struct PyPushEventReceiver {
    receiver: Option<std::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<PushEvent>>>,
    instruments: Arc<dashmap::DashMap<String, InstrumentAny>>,
}

impl std::fmt::Debug for PyPushEventReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyPushEventReceiver")
            .field("has_receiver", &self.receiver.is_some())
            .field("instruments_count", &self.instruments.len())
            .finish()
    }
}

#[pymethods]
impl PyPushEventReceiver {
    /// Create a new PushEventReceiver.
    ///
    /// This should only be called from create_quote_context.
    #[new]
    pub fn py_new() -> Self {
        Self {
            receiver: None,
            instruments: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Check if the receiver has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.receiver.is_some()
    }

    /// Cache an instrument for data conversion.
    pub fn cache_instrument(&self, py: Python, instrument: Py<PyAny>) -> PyResult<()> {
        use nautilus_model::python::instruments::pyobject_to_instrument_any;
        let inst = pyobject_to_instrument_any(py, instrument)?;
        let key = format!("{}", inst.id());
        self.instruments.insert(key, inst);
        Ok(())
    }

    /// Receive a push event asynchronously.
    ///
    /// This method tries to receive an event without blocking, then converts it to
    /// Nautilus Data type and returns it as a PyCapsule.
    ///
    /// Returns
    /// -------
    /// PyCapsule | None
    ///     A PyCapsule containing Data pointer, or None if no event is available.
    pub fn recv_async(&self, py: Python) -> PyResult<Py<PyAny>> {
        let receiver_ref = self.receiver.as_ref()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "PushEventReceiver not initialized"
            ))?;

        let mut receiver = receiver_ref.lock().unwrap();

        // Try to receive without blocking first
        match receiver.try_recv() {
            Ok(event) => {
                // Convert PushEvent to Nautilus Data
                match self.convert_push_event(event) {
                    Some(data) => {
                        // Convert Data to PyCapsule
                        Ok(data_to_pycapsule(py, data))
                    }
                    None => {
                        // Conversion failed, return None
                        Ok(py.None())
                    }
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                // No event available, return None
                // The Python side should poll in a loop
                Ok(py.None())
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                // Channel closed
                tracing::warn!("PushEvent receiver channel disconnected");
                Ok(py.None())
            }
        }
    }

    /// Receive a push event (blocking with timeout).
    ///
    /// Parameters
    /// ----------
    /// timeout_secs : float
    ///     Maximum time to wait in seconds.
    ///
    /// Returns
    /// -------
    /// PyCapsule | None
    ///     A PyCapsule containing Data pointer, or None if timeout/empty.
    pub fn recv(&self, py: Python, _timeout_secs: f64) -> PyResult<Py<PyAny>> {
        let receiver_ref = self.receiver.as_ref()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "PushEventReceiver not initialized"
            ))?;

        let mut receiver = receiver_ref.lock().unwrap();

        // Try non-blocking receive first
        match receiver.try_recv() {
            Ok(event) => {
                // Convert PushEvent to Nautilus Data
                match self.convert_push_event(event) {
                    Some(data) => {
                        // Convert Data to PyCapsule
                        Ok(data_to_pycapsule(py, data))
                    }
                    None => {
                        // Conversion failed, return None
                        Ok(py.None())
                    }
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                // TODO: Implement blocking wait with timeout
                // For now, just return None
                Ok(py.None())
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                Ok(py.None())
            }
        }
    }

    fn __repr__(&self) -> String {
        if self.receiver.is_some() {
            format!("PushEventReceiver(instruments={})", self.instruments.len())
        } else {
            "PushEventReceiver(not initialized)".to_string()
        }
    }
}

impl PyPushEventReceiver {
    /// Set the actual receiver from QuoteContext.
    pub fn set_receiver(&mut self, receiver: tokio::sync::mpsc::UnboundedReceiver<PushEvent>) {
        self.receiver = Some(std::sync::Mutex::new(receiver));
    }

    /// Get or create an instrument for the given symbol.
    fn get_or_create_instrument(&self, symbol: &str) -> Option<InstrumentAny> {
        let key = symbol.to_string();

        // Try to get from cache
        if let Some(instrument) = self.instruments.get(&key) {
            return Some(instrument.clone());
        }

        // Try to create minimal instrument
        match create_minimal_instrument(symbol) {
            Ok(instrument) => Some(instrument),
            Err(e) => {
                tracing::warn!("Failed to create instrument for {}: {}", symbol, e);
                None
            }
        }
    }

    /// Convert a PushEvent to Nautilus Data.
    fn convert_push_event(&self, event: PushEvent) -> Option<Data> {
        use nautilus_model::{
            data::{Bar, BarSpecification, BarType, OrderBookDelta, OrderBookDeltas, OrderBookDeltas_API, QuoteTick, TradeTick, order::BookOrder},
            enums::{AggregationSource, BarAggregation, BookAction, OrderSide, PriceType},
            identifiers::TradeId,
            types::{Price, Quantity},
        };

        // Get or create instrument for this symbol
        let instrument = self.get_or_create_instrument(&event.symbol)?;
        let instrument_id = instrument.id();

        match &event.detail {
            PushEventDetail::Quote(quote) => {
                // Convert PushQuote to QuoteTick
                // PushQuote contains OHLC data, but we can create a QuoteTick from last_done
                let ts_event = timestamp_to_nanos(quote.timestamp);
                let ts_init = ts_event;

                // For QuoteTick we need bid/ask, but PushQuote only has last_done
                // We'll create a synthetic QuoteTick with last_done as both bid and ask
                let price = Price::new(decimal_to_f64(&quote.last_done), instrument.price_precision());
                let volume = Quantity::new(quote.current_volume as f64, instrument.size_precision());

                let tick = QuoteTick::new(
                    instrument_id,
                    price, // bid_price
                    price, // ask_price
                    volume, // bid_size
                    volume, // ask_size
                    ts_event,
                    ts_init,
                );
                Some(Data::from(tick))
            }

            PushEventDetail::Depth(depth) => {
                // Convert PushDepth to OrderBookDeltas
                let ts_event = timestamp_to_nanos(
                    time::OffsetDateTime::from_unix_timestamp(0).unwrap()
                );
                let ts_init = ts_event;
                let sequence = 0;
                let flags = 0;
                let mut deltas = Vec::new();

                // Helper to create order ID hash
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                event.symbol.hash(&mut hasher);

                // Process bids (buy orders)
                for bid in &depth.bids {
                    if let Some(price) = bid.price {
                        if price > rust_decimal::Decimal::ZERO {
                            let price_val = Price::new(decimal_to_f64(&price), instrument.price_precision());
                            let size = Quantity::new(bid.volume as f64, instrument.size_precision());

                            // Create u64 order_id from hash
                            let mut id_hasher = std::collections::hash_map::DefaultHasher::new();
                            format!("bid-{}-{}-{}", event.symbol, bid.position, bid.volume).hash(&mut id_hasher);
                            let order_id = id_hasher.finish();

                            let order = BookOrder::new(
                                OrderSide::Buy,
                                price_val,
                                size,
                                order_id,
                            );

                            let delta = OrderBookDelta::new(
                                instrument_id,
                                BookAction::Add,
                                order,
                                flags,
                                sequence,
                                ts_event,
                                ts_init,
                            );
                            deltas.push(delta);
                        }
                    }
                }

                // Process asks (sell orders)
                for ask in &depth.asks {
                    if let Some(price) = ask.price {
                        if price > rust_decimal::Decimal::ZERO {
                            let price_val = Price::new(decimal_to_f64(&price), instrument.price_precision());
                            let size = Quantity::new(ask.volume as f64, instrument.size_precision());

                            // Create u64 order_id from hash
                            let mut id_hasher = std::collections::hash_map::DefaultHasher::new();
                            format!("ask-{}-{}-{}", event.symbol, ask.position, ask.volume).hash(&mut id_hasher);
                            let order_id = id_hasher.finish();

                            let order = BookOrder::new(
                                OrderSide::Sell,
                                price_val,
                                size,
                                order_id,
                            );

                            let delta = OrderBookDelta::new(
                                instrument_id,
                                BookAction::Add,
                                order,
                                flags,
                                sequence,
                                ts_event,
                                ts_init,
                            );
                            deltas.push(delta);
                        }
                    }
                }

                if !deltas.is_empty() {
                    let deltas_obj = OrderBookDeltas::new(instrument_id, deltas);
                    Some(Data::from(OrderBookDeltas_API::new(deltas_obj)))
                } else {
                    None
                }
            }

            PushEventDetail::Trade(trades) => {
                // Convert PushTrades to TradeTick(s)
                // Return the first trade tick (most recent)
                if let Some(trade) = trades.trades.first() {
                    let price = Price::new(decimal_to_f64(&trade.price), instrument.price_precision());
                    let size = Quantity::new(trade.volume as f64, instrument.size_precision());
                    let ts_event = timestamp_to_nanos(trade.timestamp);
                    let ts_init = ts_event;

                    let trade_id = TradeId::new(format!(
                        "{}-{}-{}",
                        trade.price,
                        trade.volume,
                        ts_event
                    ));

                    use nautilus_model::enums::AggressorSide;
                    let tick = TradeTick::new(
                        instrument_id,
                        price,
                        size,
                        AggressorSide::NoAggressor,
                        trade_id,
                        ts_event,
                        ts_init,
                    );
                    Some(Data::from(tick))
                } else {
                    None
                }
            }

            PushEventDetail::Candlestick(candlestick) => {
                // Convert PushCandlestick to Bar
                // Determine aggregation from period
                let aggregation = match candlestick.period {
                    longport::quote::Period::OneMinute => BarAggregation::Minute,
                    longport::quote::Period::TwoMinute => BarAggregation::Minute,
                    longport::quote::Period::ThreeMinute => BarAggregation::Minute,
                    longport::quote::Period::FiveMinute => BarAggregation::Minute,
                    longport::quote::Period::TenMinute => BarAggregation::Minute,
                    longport::quote::Period::FifteenMinute => BarAggregation::Minute,
                    longport::quote::Period::TwentyMinute => BarAggregation::Minute,
                    longport::quote::Period::ThirtyMinute => BarAggregation::Minute,
                    longport::quote::Period::FortyFiveMinute => BarAggregation::Minute,
                    longport::quote::Period::SixtyMinute => BarAggregation::Hour,
                    longport::quote::Period::TwoHour => BarAggregation::Hour,
                    longport::quote::Period::ThreeHour => BarAggregation::Hour,
                    longport::quote::Period::FourHour => BarAggregation::Hour,
                    longport::quote::Period::Day => BarAggregation::Day,
                    longport::quote::Period::Week => BarAggregation::Week,
                    longport::quote::Period::Month => BarAggregation::Month,
                    _ => {
                        tracing::warn!("Unsupported period: {:?}", candlestick.period);
                        return None;
                    }
                };

                let open = Price::new(decimal_to_f64(&candlestick.candlestick.open), instrument.price_precision());
                let high = Price::new(decimal_to_f64(&candlestick.candlestick.high), instrument.price_precision());
                let low = Price::new(decimal_to_f64(&candlestick.candlestick.low), instrument.price_precision());
                let close = Price::new(decimal_to_f64(&candlestick.candlestick.close), instrument.price_precision());
                let volume = Quantity::new(candlestick.candlestick.volume as f64, instrument.size_precision());
                let ts_event = timestamp_to_nanos(candlestick.candlestick.timestamp);
                let ts_init = ts_event;

                let spec = BarSpecification::new(1, aggregation, PriceType::Last);
                let bar_type = BarType::new(
                    instrument_id,
                    spec,
                    AggregationSource::External,
                );

                let bar = Bar::new(
                    bar_type,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    ts_event,
                    ts_init,
                );
                Some(Data::from(bar))
            }

            PushEventDetail::Brokers(_brokers) => {
                // Brokers data - not directly supported by Nautilus data model
                tracing::debug!("Received Brokers event for {}, skipping", event.symbol);
                None
            }
        }
    }
}

/// Create a Longport QuoteContext with the given credentials and optional URLs.
///
/// This is the main entry point for creating a QuoteContext from Python.
/// It handles the async creation and returns both the context and event receiver.
///
/// Parameters
/// ----------
/// app_key : str
///     The Longport app key.
/// app_secret : str
///     The Longport app secret.
/// access_token : str
///     The Longport access token.
/// http_url : str, optional
///     Custom HTTP URL (default: https://openapi.longportapp.com)
/// quote_ws_url : str, optional
///     Custom quote WebSocket URL (default: wss://openapi-quote.longportapp.com/v2)
/// trade_ws_url : str, optional
///     Custom trade WebSocket URL (default: wss://openapi-trade.longportapp.com/v2)
///
/// Returns
/// -------
/// tuple[QuoteContext, PushEventReceiver]
///     A tuple containing the QuoteContext and PushEventReceiver.
///
/// Raises
/// ------
/// RuntimeError
///     If the QuoteContext creation fails.
#[pyfunction]
#[pyo3(signature = (
    app_key,
    app_secret,
    access_token,
    http_url=None,
    quote_ws_url=None,
    trade_ws_url=None
))]
pub fn create_quote_context(
    py: Python<'_>,
    app_key: String,
    app_secret: String,
    access_token: String,
    http_url: Option<String>,
    quote_ws_url: Option<String>,
    trade_ws_url: Option<String>,
) -> PyResult<(PyQuoteContext, PyPushEventReceiver)> {
    install_cryptographic_provider();
    let _ = (http_url, quote_ws_url, trade_ws_url);  // Suppress unused warnings
    // Build the configuration
    let config = Config::new(app_key, app_secret, access_token);

    // Create the QuoteContext (blocking call using the runtime)
    let runtime = py.allow_threads(|| {
        tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create async runtime: {}", e)
            ))
    })?;

    let (quote_ctx, event_receiver) = runtime.block_on(async {
        QuoteContext::try_new(Arc::new(config))
            .await
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create QuoteContext: {}", e)
            ))
    })?;

    // Create PyQuoteContext with the runtime kept alive
    let py_quote_ctx = PyQuoteContext::from_quote_ctx(quote_ctx, runtime);

    // Create a proper receiver wrapper
    let mut py_event_receiver = PyPushEventReceiver::py_new();
    py_event_receiver.set_receiver(event_receiver);

    Ok((py_quote_ctx, py_event_receiver))
}

/// Create a minimal instrument for a given instrument ID string.
///
/// This is a Python wrapper around the Rust `create_minimal_instrument` function.
///
/// Parameters
/// ----------
/// instrument_id_str : str
///     The instrument ID string in format "SYMBOL.MARKET" (e.g., "700.HK", "AAPL.US").
///
/// Returns
/// -------
/// InstrumentAny
///     The created instrument.
///
/// Raises
/// ------
/// ValueError
///     If the instrument ID format is invalid or the market is unknown.
#[pyfunction]
#[pyo3(signature = (instrument_id_str))]
pub fn create_minimal_instrument_py(
    py: Python<'_>,
    instrument_id_str: String,
) -> PyResult<Py<PyAny>> {
    let instrument = create_minimal_instrument(&instrument_id_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;

    nautilus_model::python::instruments::instrument_any_to_pyobject(py, instrument)
}
