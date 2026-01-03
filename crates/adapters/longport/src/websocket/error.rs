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

//! WebSocket client errors for the Longport adapter.

use thiserror::Error;

/// Represents errors that can occur during Longport WebSocket operations.
#[derive(Debug, Error)]
pub enum LongportWsError {
    /// WebSocket connection error.
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Authentication error.
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Message parsing error.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded")]
    RateLimited,

    /// Invalid message received.
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Subscription failed.
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),
}
