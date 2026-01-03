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

//! HTTP client for the Longport adapter.
//!
//! This module provides an HTTP client that wraps the Longport SDK's QuoteContext
//! for HTTP-based market data operations.
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
use pyo3::Python;

/// HTTP client for Longport market data.
///
/// This client wraps the Longport SDK's QuoteContext to provide HTTP-based
/// market data operations. The QuoteContext is shared with the WebSocket client
/// via Arc.
#[derive(Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
pub struct LongportHttpClient {
    /// The shared QuoteContext from the Longport SDK.
    quote_ctx: Arc<QuoteContext>,
    /// Instruments cache for price/size precision.
    instruments: Arc<dashmap::DashMap<InstrumentId, nautilus_model::instruments::InstrumentAny>>,
}

impl std::fmt::Debug for LongportHttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LongportHttpClient")
            .field("quote_ctx", &"<QuoteContext>")
            .field("instruments_count", &self.instruments.len())
            .finish()
    }
}

#[cfg(feature = "python")]
use pyo3::pymethods;

#[cfg(feature = "python")]
#[pymethods]
impl LongportHttpClient {
    /// Creates a new LongportHttpClient from a QuoteContext.
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

    /// Caches an instrument for price/size precision.
    pub fn cache_instrument(&self, py: pyo3::Python, instrument: pyo3::Py<pyo3::PyAny>) -> pyo3::PyResult<()> {
        use nautilus_model::python::instruments::pyobject_to_instrument_any;
        let inst = pyobject_to_instrument_any(py, instrument)?;
        self.instruments.insert(inst.id(), inst);
        Ok(())
    }
}

impl LongportHttpClient {
    /// Creates a new [`LongportHttpClient`] from a QuoteContext (internal Rust).
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

    /// Caches an instrument for price/size precision.
    pub fn cache_instrument_internal(&self, instrument: &nautilus_model::instruments::InstrumentAny) {
        self.instruments.insert(instrument.id(), instrument.clone());
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
