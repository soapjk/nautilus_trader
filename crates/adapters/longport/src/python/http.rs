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

//! Python bindings for the Longport HTTP client.

use pyo3::prelude::*;

use crate::http::LongportHttpClient;

#[pymethods]
impl LongportHttpClient {
    #[new]
    #[pyo3(signature = (
        base_url,
        app_key=None,
        app_secret=None,
        access_token=None,
    ))]
    fn py_new(
        base_url: String,
        app_key: Option<String>,
        app_secret: Option<String>,
        access_token: Option<String>,
    ) -> PyResult<Self> {
        // Create credential from optional parameters
        let credential = if app_key.is_some() || app_secret.is_some() || access_token.is_some() {
            Some(crate::common::credential::Credential::new(
                app_key.unwrap_or_default(),
                app_secret.unwrap_or_default(),
                access_token.unwrap_or_default(),
            ))
        } else {
            None
        };

        Self::new(base_url, credential).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create LongportHttpClient: {e}"
            ))
        })
    }

    #[getter]
    #[pyo3(name = "base_url")]
    #[must_use]
    pub fn py_base_url(&self) -> &str {
        &self.raw().base_url
    }
}
