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

//! Python bindings for the Longport adapter.

pub mod quote_context;

use nautilus_system::factories::{ClientConfig, ExecutionClientFactory};
use pyo3::prelude::*;

#[cfg(feature = "python")]
use nautilus_system::get_global_pyo3_registry;

use quote_context::{PyQuoteContext, PyPushEventReceiver, create_minimal_instrument_py, create_quote_context};

// NOTE: Longport DataClient is now implemented in Python (nautilus_trader/adapters/longport/data.py)
// following the OKX architecture pattern. Only HTTP/WebSocket clients and configs are exposed.

/// Extractor function for `LongportDataClientConfig`.
#[cfg(feature = "python")]
fn extract_longport_data_config(py: Python<'_>, config: Py<PyAny>) -> PyResult<Box<dyn ClientConfig>> {
    match config.extract::<crate::config::LongportDataClientConfig>(py) {
        Ok(concrete_config) => Ok(Box::new(concrete_config)),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to extract LongportDataClientConfig: {e}"
        ))),
    }
}

/// Extractor function for `LongportExecClientConfig`.
#[cfg(feature = "python")]
fn extract_longport_exec_config(py: Python<'_>, config: Py<PyAny>) -> PyResult<Box<dyn ClientConfig>> {
    match config.extract::<crate::config::LongportExecClientConfig>(py) {
        Ok(concrete_config) => Ok(Box::new(concrete_config)),
        Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to extract LongportExecClientConfig: {e}"
        ))),
    }
}

/// Longport adapter Python module.
///
/// Exposed under `nautilus_trader.core.nautilus_pyo3.longport`.
#[pymodule]
pub fn longport(_: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__package__", "nautilus_trader.core.nautilus_pyo3.longport")?;

    // Add enums
    m.add_class::<crate::common::enums::LongportMarket>()?;
    m.add_class::<crate::common::enums::LongportOrderType>()?;
    m.add_class::<crate::common::enums::LongportSecurityType>()?;
    m.add_class::<crate::common::enums::LongportSide>()?;

    // Add data client config (for use by Python DataClient)
    m.add_class::<crate::config::LongportDataClientConfig>()?;
    m.add_class::<crate::config::LongportExecClientConfig>()?;

    // Add QuoteContext and PushEventReceiver for Python DataClient
    m.add_class::<PyQuoteContext>()?;
    m.add_class::<PyPushEventReceiver>()?;
    m.add_function(wrap_pyfunction!(create_minimal_instrument_py, m)?)?;
    m.add_function(wrap_pyfunction!(create_quote_context, m)?)?;

    // Add HTTP client for Python to use
    m.add_class::<crate::http::LongportHttpClient>()?;

    // Add WebSocket client for Python to use
    m.add_class::<crate::websocket::LongportWebSocketClient>()?;

    // Add execution client factory
    m.add_class::<crate::factories::LongportExecutionClientFactory>()?;

    // Register extractors with the global registry (only when python feature is enabled)
    #[cfg(feature = "python")]
    {
        let registry = get_global_pyo3_registry();

        // Register data client config extractor
        if let Err(e) = registry.register_config_extractor(
            "LongportDataClientConfig".to_string(),
            extract_longport_data_config,
        ) {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to register longport data config extractor: {e}"
            )));
        }

        // Register execution client config extractor
        if let Err(e) = registry.register_config_extractor(
            "LongportExecClientConfig".to_string(),
            extract_longport_exec_config,
        ) {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to register longport exec config extractor: {e}"
            )));
        }
    }

    Ok(())
}
