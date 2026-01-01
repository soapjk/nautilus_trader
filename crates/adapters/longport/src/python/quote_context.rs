// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  you may not use this file except in compliance with the License.
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
use pyo3::prelude::*;
use longport::quote::QuoteContext;
use nautilus_model::instruments::InstrumentAny;
use nautilus_model::python::instruments::instrument_any_to_pyobject;
use crate::common::parse::create_minimal_instrument;

/// Python wrapper for Longport QuoteContext.
#[pyclass(name = "QuoteContext", module = "nautilus_trader.core.nautilus_pyo3.longport")]
pub struct PyQuoteContext {
    inner: Option<Arc<QuoteContext>>,
}

impl std::fmt::Debug for PyQuoteContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyQuoteContext")
            .field("inner", &self.inner.is_some())
            .finish()
    }
}

#[pymethods]
impl PyQuoteContext {
    /// Create a new QuoteContext with credentials.
    #[new]
    #[pyo3(signature = (app_key, app_secret, access_token))]
    pub fn py_new(
        app_key: String,
        app_secret: String,
        access_token: String,
    ) -> PyResult<Self> {
        // Note: We can't create QuoteContext here because it requires async
        // Instead, we'll store the config and create it in a separate method
        Ok(Self { inner: None })
    }

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
#[pyclass(name = "PushEventReceiver", module = "nautilus_trader.core.nautilus_pyo3.longport")]
#[derive(Debug)]
pub struct PyPushEventReceiver {
    // Placeholder for future implementation
    _private: Vec<u8>,
}

#[pymethods]
impl PyPushEventReceiver {
    /// Create a new PushEventReceiver.
    #[new]
    pub fn py_new() -> Self {
        Self { _private: Vec::new() }
    }

    /// Receive a push event (blocking with timeout).
    pub fn recv(&self, _py: Python, _timeout_secs: f64) -> PyResult<Py<PyAny>> {
        // Placeholder - returns None for now
        Ok(_py.None())
    }

    fn __repr__(&self) -> String {
        "PushEventReceiver()".to_string()
    }
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

    instrument_any_to_pyobject(py, instrument)
}
