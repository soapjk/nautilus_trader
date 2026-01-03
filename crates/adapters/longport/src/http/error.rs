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

//! HTTP client errors for the Longport adapter.

use thiserror::Error;

/// Represents errors that can occur during Longport HTTP operations.
#[derive(Debug, Error)]
pub enum LongportHttpError {
    /// Missing required credentials (app_key, app_secret, access_token).
    #[error("Missing required credentials: {0}")]
    MissingCredentials(String),

    /// Error returned by the Longport API.
    #[error("Longport API error (code: {code}, message: {message})")]
    LongportError { code: String, message: String },

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(String),

    /// Input validation error.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Network connectivity error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded")]
    RateLimited,

    /// Authentication/authorization error.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

impl From<serde_json::Error> for LongportHttpError {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonError(e.to_string())
    }
}
