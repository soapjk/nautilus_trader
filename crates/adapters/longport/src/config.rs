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

//! Configuration structures for the Longport adapter.

use nautilus_model::identifiers::{AccountId, TraderId};

use crate::common::enums::LongportMarket;

// Default Longport API endpoints (from official SDK)
// https://github.com/longportapp/openapi
const LONGPORT_HTTP_URL_DEFAULT: &str = "https://openapi.longportapp.com";
const LONGPORT_QUOTE_WS_URL_DEFAULT: &str = "wss://openapi-quote.longportapp.com/v2";
const LONGPORT_TRADE_WS_URL_DEFAULT: &str = "wss://openapi-trade.longportapp.com/v2";

#[cfg(feature = "python")]
use pyo3::prelude::*;

/// Configuration for the Longport data client.
///
/// # Environment Variables
///
/// The Longport SDK supports the following environment variables:
/// - `LONGPORT_HTTP_URL`: HTTP endpoint URL (default: https://openapi.longportapp.com)
/// - `LONGPORT_QUOTE_WS_URL`: Quote WebSocket URL (default: wss://openapi-quote.longportapp.com/v2)
/// - `LONGPORT_TRADE_WS_URL`: Trade WebSocket URL (default: wss://openapi-trade.longportapp.com/v2)
/// - `LONGPORT_APP_KEY`: App key
/// - `LONGPORT_APP_SECRET`: App secret
/// - `LONGPORT_ACCESS_TOKEN`: Access token
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.adapters.longport")
)]
pub struct LongportDataClientConfig {
    /// Application key for authentication.
    pub app_key: Option<String>,
    /// Application secret for authentication.
    pub app_secret: Option<String>,
    /// Access token for authentication.
    pub access_token: Option<String>,
    /// Markets to subscribe to (HK, US, or both).
    pub markets: Vec<LongportMarket>,
    /// Optional list of instrument IDs to load (e.g., ["700.HK", "AAPL.US"]).
    /// When specified, only these instruments will be subscribed to,
    /// without fetching the full security list from the API.
    pub load_ids: Option<Vec<String>>,
    /// Optional HTTP timeout in seconds.
    pub http_timeout_secs: Option<u64>,
    /// Optional custom HTTP URL.
    pub http_url: Option<String>,
    /// Optional custom quote WebSocket URL (real-time market data).
    pub quote_ws_url: Option<String>,
    /// Optional custom trade WebSocket URL (order status updates).
    pub trade_ws_url: Option<String>,
}

impl Default for LongportDataClientConfig {
    fn default() -> Self {
        Self {
            app_key: None,
            app_secret: None,
            access_token: None,
            markets: vec![LongportMarket::HK],
            load_ids: None,
            http_timeout_secs: Some(30),
            http_url: None,
            quote_ws_url: None,
            trade_ws_url: None,
        }
    }
}

impl LongportDataClientConfig {
    /// Creates a new configuration with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            app_key: None,
            app_secret: None,
            access_token: None,
            markets: vec![LongportMarket::HK],
            load_ids: None,
            http_timeout_secs: Some(30),
            http_url: None,
            quote_ws_url: None,
            trade_ws_url: None,
        }
    }

    /// Returns `true` when all required credential fields are available (in config or env vars).
    #[must_use]
    pub fn has_credentials(&self) -> bool {
        let has_key = self.app_key.is_some() || std::env::var("LONGPORT_APP_KEY").is_ok();
        let has_secret = self.app_secret.is_some() || std::env::var("LONGPORT_APP_SECRET").is_ok();
        let has_token =
            self.access_token.is_some() || std::env::var("LONGPORT_ACCESS_TOKEN").is_ok();
        has_key && has_secret && has_token
    }

    /// Returns the app key, from config or environment variable.
    #[must_use]
    pub fn get_app_key(&self) -> Option<String> {
        self.app_key
            .clone()
            .or_else(|| std::env::var("LONGPORT_APP_KEY").ok())
    }

    /// Returns the app secret, from config or environment variable.
    #[must_use]
    pub fn get_app_secret(&self) -> Option<String> {
        self.app_secret
            .clone()
            .or_else(|| std::env::var("LONGPORT_APP_SECRET").ok())
    }

    /// Returns the access token, from config or environment variable.
    #[must_use]
    pub fn get_access_token(&self) -> Option<String> {
        self.access_token
            .clone()
            .or_else(|| std::env::var("LONGPORT_ACCESS_TOKEN").ok())
    }

    /// Returns the HTTP URL, from config or environment variable.
    #[must_use]
    pub fn get_http_url(&self) -> Option<String> {
        self.http_url
            .clone()
            .or_else(|| std::env::var("LONGPORT_HTTP_URL").ok())
            .or(Some(LONGPORT_HTTP_URL_DEFAULT.to_string()))
    }

    /// Returns the quote WebSocket URL, from config or environment variable.
    #[must_use]
    pub fn get_quote_ws_url(&self) -> Option<String> {
        self.quote_ws_url
            .clone()
            .or_else(|| std::env::var("LONGPORT_QUOTE_WS_URL").ok())
            .or(Some(LONGPORT_QUOTE_WS_URL_DEFAULT.to_string()))
    }

    /// Returns the trade WebSocket URL, from config or environment variable.
    #[must_use]
    pub fn get_trade_ws_url(&self) -> Option<String> {
        self.trade_ws_url
            .clone()
            .or_else(|| std::env::var("LONGPORT_TRADE_WS_URL").ok())
            .or(Some(LONGPORT_TRADE_WS_URL_DEFAULT.to_string()))
    }
}

/// Configuration for the Longport execution client.
///
/// # Environment Variables
///
/// The Longport SDK supports the following environment variables:
/// - `LONGPORT_HTTP_URL`: HTTP endpoint URL (default: https://openapi.longportapp.com)
/// - `LONGPORT_QUOTE_WS_URL`: Quote WebSocket URL (default: wss://openapi-quote.longportapp.com/v2)
/// - `LONGPORT_TRADE_WS_URL`: Trade WebSocket URL (default: wss://openapi-trade.longportapp.com/v2)
/// - `LONGPORT_APP_KEY`: App key
/// - `LONGPORT_APP_SECRET`: App secret
/// - `LONGPORT_ACCESS_TOKEN`: Access token
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.adapters.longport")
)]
pub struct LongportExecClientConfig {
    /// The trader ID for the client.
    pub trader_id: TraderId,
    /// The account ID for the client.
    pub account_id: AccountId,
    /// Application key for authentication.
    pub app_key: Option<String>,
    /// Application secret for authentication.
    pub app_secret: Option<String>,
    /// Access token for authentication.
    pub access_token: Option<String>,
    /// Markets to trade on (HK, US, or both).
    pub markets: Vec<LongportMarket>,
    /// Optional HTTP timeout in seconds.
    pub http_timeout_secs: Option<u64>,
    /// Optional custom HTTP URL.
    pub http_url: Option<String>,
    /// Optional custom quote WebSocket URL.
    pub quote_ws_url: Option<String>,
    /// Optional custom trade WebSocket URL.
    pub trade_ws_url: Option<String>,
}

impl Default for LongportExecClientConfig {
    fn default() -> Self {
        Self {
            trader_id: TraderId::from("TRADER-001"),
            account_id: AccountId::from("LONGPORT-001"),
            app_key: None,
            app_secret: None,
            access_token: None,
            markets: vec![LongportMarket::HK],
            http_timeout_secs: Some(30),
            http_url: None,
            quote_ws_url: None,
            trade_ws_url: None,
        }
    }
}

impl LongportExecClientConfig {
    /// Creates a new configuration with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            trader_id: TraderId::from("TRADER-001"),
            account_id: AccountId::from("LONGPORT-001"),
            app_key: None,
            app_secret: None,
            access_token: None,
            markets: vec![LongportMarket::HK],
            http_timeout_secs: Some(30),
            http_url: None,
            quote_ws_url: None,
            trade_ws_url: None,
        }
    }

    /// Returns `true` when all required credential fields are available (in config or env vars).
    #[must_use]
    pub fn has_credentials(&self) -> bool {
        let has_key = self.app_key.is_some() || std::env::var("LONGPORT_APP_KEY").is_ok();
        let has_secret = self.app_secret.is_some() || std::env::var("LONGPORT_APP_SECRET").is_ok();
        let has_token =
            self.access_token.is_some() || std::env::var("LONGPORT_ACCESS_TOKEN").is_ok();
        has_key && has_secret && has_token
    }

    /// Returns the app key, from config or environment variable.
    #[must_use]
    pub fn get_app_key(&self) -> Option<String> {
        self.app_key
            .clone()
            .or_else(|| std::env::var("LONGPORT_APP_KEY").ok())
    }

    /// Returns the app secret, from config or environment variable.
    #[must_use]
    pub fn get_app_secret(&self) -> Option<String> {
        self.app_secret
            .clone()
            .or_else(|| std::env::var("LONGPORT_APP_SECRET").ok())
    }

    /// Returns the access token, from config or environment variable.
    #[must_use]
    pub fn get_access_token(&self) -> Option<String> {
        self.access_token
            .clone()
            .or_else(|| std::env::var("LONGPORT_ACCESS_TOKEN").ok())
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl LongportExecClientConfig {
    #[new]
    #[pyo3(signature = (
        trader_id,
        account_id,
        app_key=None,
        app_secret=None,
        access_token=None,
        markets=None,
        http_timeout_secs=None,
        http_url=None,
        quote_ws_url=None,
        trade_ws_url=None
    ))]
    pub fn py_new(
        trader_id: &str,
        account_id: &str,
        app_key: Option<String>,
        app_secret: Option<String>,
        access_token: Option<String>,
        markets: Option<Vec<LongportMarket>>,
        http_timeout_secs: Option<u64>,
        http_url: Option<String>,
        quote_ws_url: Option<String>,
        trade_ws_url: Option<String>,
    ) -> Self {
        Self {
            trader_id: TraderId::from(trader_id),
            account_id: AccountId::from(account_id),
            app_key,
            app_secret,
            access_token,
            markets: markets.unwrap_or_default(),
            http_timeout_secs,
            http_url,
            quote_ws_url,
            trade_ws_url,
        }
    }
}

#[cfg(feature = "python")]
#[pymethods]
impl LongportDataClientConfig {
    #[new]
    #[pyo3(signature = (
        app_key=None,
        app_secret=None,
        access_token=None,
        markets=None,
        load_ids=None,
        http_timeout_secs=None,
        http_url=None,
        quote_ws_url=None,
        trade_ws_url=None
    ))]
    pub fn py_new(
        app_key: Option<String>,
        app_secret: Option<String>,
        access_token: Option<String>,
        markets: Option<Vec<LongportMarket>>,
        load_ids: Option<Vec<String>>,
        http_timeout_secs: Option<u64>,
        http_url: Option<String>,
        quote_ws_url: Option<String>,
        trade_ws_url: Option<String>,
    ) -> Self {
        Self {
            app_key,
            app_secret,
            access_token,
            markets: markets.unwrap_or_else(|| vec![LongportMarket::HK]),
            load_ids,
            http_timeout_secs,
            http_url,
            quote_ws_url,
            trade_ws_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longport_data_client_config_default() {
        let config = LongportDataClientConfig::default();
        assert_eq!(config.markets.len(), 1);
        assert_eq!(config.markets[0], LongportMarket::HK);
        assert_eq!(config.http_timeout_secs, Some(30));
    }

    #[test]
    fn test_longport_exec_client_config_default() {
        let config = LongportExecClientConfig::default();
        assert_eq!(config.trader_id, TraderId::from("TRADER-001"));
        assert_eq!(config.account_id, AccountId::from("LONGPORT-001"));
        assert_eq!(config.markets.len(), 1);
        assert_eq!(config.markets[0], LongportMarket::HK);
    }

    #[test]
    fn test_longport_data_client_config_new() {
        let config = LongportDataClientConfig::new();
        assert_eq!(config.markets.len(), 1);
        assert_eq!(config.markets[0], LongportMarket::HK);
    }

    #[test]
    fn test_longport_data_client_config_get_urls() {
        let config = LongportDataClientConfig::default();
        assert_eq!(
            config.get_http_url(),
            Some(LONGPORT_HTTP_URL_DEFAULT.to_string())
        );
        assert_eq!(
            config.get_quote_ws_url(),
            Some(LONGPORT_QUOTE_WS_URL_DEFAULT.to_string())
        );
        assert_eq!(
            config.get_trade_ws_url(),
            Some(LONGPORT_TRADE_WS_URL_DEFAULT.to_string())
        );
    }
}
