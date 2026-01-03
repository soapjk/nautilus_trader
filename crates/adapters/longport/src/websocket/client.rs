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

//! WebSocket client for Longport market data.
//!
//! This module provides a WebSocket client that wraps the Longport SDK's QuoteContext
//! for WebSocket-based market data operations.
//!
//! # Architecture
//!
//! The QuoteContext is shared between HTTP and WebSocket clients via Arc:
//! ```text
//! Python creates QuoteContext
//!         │
//!         ├──> Arc::clone ──> LongportHttpClient (HTTP operations)
//!         │
//!         └──> Arc::clone ──> LongportWebSocketClient (WebSocket operations)
//! ```

use std::sync::Arc;

use longport::quote::QuoteContext;
use nautilus_model::{
    identifiers::InstrumentId,
    instruments::Instrument,
};

#[cfg(feature = "python")]
use pyo3::{Python, Bound, PyAny};

/// WebSocket client for Longport market data.
///
/// This client wraps the Longport SDK's QuoteContext to provide WebSocket-based
/// market data operations. The QuoteContext is shared with the HTTP client
/// via Arc.
#[derive(Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
pub struct LongportWebSocketClient {
    /// The shared QuoteContext from the Longport SDK.
    quote_ctx: Arc<QuoteContext>,
    /// Instruments cache for data conversion.
    instruments: Arc<dashmap::DashMap<InstrumentId, nautilus_model::instruments::InstrumentAny>>,
}

impl std::fmt::Debug for LongportWebSocketClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LongportWebSocketClient")
            .field("quote_ctx", &"<QuoteContext>")
            .field("instruments_count", &self.instruments.len())
            .finish()
    }
}

#[cfg(feature = "python")]
use pyo3::pymethods;

#[cfg(feature = "python")]
#[pymethods]
impl LongportWebSocketClient {
    /// Creates a new LongportWebSocketClient from a QuoteContext.
    ///
    /// This is intended to be called from Python with a QuoteContext
    /// that was created in Python.
    #[new]
    pub fn py_new_from_context(
        quote_ctx: pyo3::Py<crate::python::quote_context::PyQuoteContext>,
    ) -> pyo3::PyResult<Self> {
        Python::with_gil(|py| {
            let ctx = quote_ctx.borrow(py);
            let inner = ctx.inner().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err("QuoteContext not initialized")
            })?;
            Ok(Self {
                quote_ctx: inner,
                instruments: Arc::new(dashmap::DashMap::new()),
            })
        })
    }

    /// Caches an instrument for data conversion.
    pub fn cache_instrument(&self, py: pyo3::Python, instrument: pyo3::Py<pyo3::PyAny>) -> pyo3::PyResult<()> {
        use nautilus_model::python::instruments::pyobject_to_instrument_any;
        let inst = pyobject_to_instrument_any(py, instrument)?;
        self.instruments.insert(inst.id(), inst);
        Ok(())
    }

    /// Subscribes to quotes for a symbol.
    pub fn subscribe_quotes<'py>(&self, py: Python<'py>, symbol: &str) -> pyo3::PyResult<Bound<'py, PyAny>> {
        let ctx = Arc::clone(&self.quote_ctx);
        let symbol = symbol.to_string();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            if let Err(e) = ctx
                .subscribe(vec![&symbol], longport::quote::SubFlags::QUOTE)
                .await
            {
                tracing::error!("Failed to subscribe to quotes: {e}");
            }
            Ok(())
        })
    }

    /// Unsubscribes from quotes for a symbol.
    pub fn unsubscribe_quotes<'py>(&self, py: Python<'py>, symbol: &str) -> pyo3::PyResult<Bound<'py, PyAny>> {
        let ctx = Arc::clone(&self.quote_ctx);
        let symbol = symbol.to_string();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            if let Err(e) = ctx
                .unsubscribe(vec![&symbol], longport::quote::SubFlags::QUOTE)
                .await
            {
                tracing::error!("Failed to unsubscribe from quotes: {e}");
            }
            Ok(())
        })
    }

    /// Subscribes to trades for a symbol.
    pub fn subscribe_trades<'py>(&self, py: Python<'py>, symbol: &str) -> pyo3::PyResult<Bound<'py, PyAny>> {
        let ctx = Arc::clone(&self.quote_ctx);
        let symbol = symbol.to_string();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            if let Err(e) = ctx
                .subscribe(vec![&symbol], longport::quote::SubFlags::TRADE)
                .await
            {
                tracing::error!("Failed to subscribe to trades: {e}");
            }
            Ok(())
        })
    }

    /// Unsubscribes from trades for a symbol.
    pub fn unsubscribe_trades<'py>(&self, py: Python<'py>, symbol: &str) -> pyo3::PyResult<Bound<'py, PyAny>> {
        let ctx = Arc::clone(&self.quote_ctx);
        let symbol = symbol.to_string();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            if let Err(e) = ctx
                .unsubscribe(vec![&symbol], longport::quote::SubFlags::TRADE)
                .await
            {
                tracing::error!("Failed to unsubscribe from trades: {e}");
            }
            Ok(())
        })
    }

    /// Subscribes to order book depth for a symbol.
    pub fn subscribe_depth<'py>(&self, py: Python<'py>, symbol: &str) -> pyo3::PyResult<Bound<'py, PyAny>> {
        let ctx = Arc::clone(&self.quote_ctx);
        let symbol = symbol.to_string();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            if let Err(e) = ctx
                .subscribe(vec![&symbol], longport::quote::SubFlags::DEPTH)
                .await
            {
                tracing::error!("Failed to subscribe to depth: {e}");
            }
            Ok(())
        })
    }

    /// Unsubscribes from order book depth for a symbol.
    pub fn unsubscribe_depth<'py>(&self, py: Python<'py>, symbol: &str) -> pyo3::PyResult<Bound<'py, PyAny>> {
        let ctx = Arc::clone(&self.quote_ctx);
        let symbol = symbol.to_string();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            if let Err(e) = ctx
                .unsubscribe(vec![&symbol], longport::quote::SubFlags::DEPTH)
                .await
            {
                tracing::error!("Failed to unsubscribe from depth: {e}");
            }
            Ok(())
        })
    }
}

impl LongportWebSocketClient {
    /// Creates a new [`LongportWebSocketClient`] from a QuoteContext (internal Rust).
    ///
    /// # Arguments
    ///
    /// * `quote_ctx` - The shared QuoteContext from the Longport SDK.
    pub fn from_context_internal(quote_ctx: Arc<QuoteContext>) -> Self {
        Self {
            quote_ctx,
            instruments: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Returns a reference to the underlying QuoteContext.
    pub fn quote_context(&self) -> Arc<QuoteContext> {
        Arc::clone(&self.quote_ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // This test requires valid credentials
        // In a real test environment, set up mocks
    }
}
