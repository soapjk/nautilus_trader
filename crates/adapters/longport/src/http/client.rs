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
//! This module provides a two-layer HTTP client architecture:
//! - `LongportHttpClient`: High-level client with domain transformations
//! - Wraps LongPort SDK's QuoteContext for market data HTTP operations

use std::{collections::HashMap, sync::Arc};

use nautilus_network::http::HttpClient;
use ustr::Ustr;

use crate::common::credential::Credential;

/// Low-level HTTP client wrapping LongPort SDK operations.
///
/// This client provides raw access to LongPort's HTTP endpoints through the SDK,
/// handling authentication, request signing, and basic error handling.
#[derive(Clone, Debug)]
pub struct LongportRawHttpClient {
    /// The base URL for Longport HTTP API.
    pub base_url: String,
    /// HTTP client for making requests.
    client: HttpClient,
    /// Optional credentials for authenticated requests.
    credential: Option<Credential>,
}

impl LongportRawHttpClient {
    /// Creates a new [`LongportRawHttpClient`].
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for Longport HTTP API.
    /// * `credential` - Optional credentials for authenticated requests.
    pub fn new(base_url: String, credential: Option<Credential>) -> anyhow::Result<Self> {
        // Validate base_url
        if base_url.is_empty() {
            return Err(anyhow::anyhow!("Base URL cannot be empty"));
        }

        // Create HTTP client
        let client = HttpClient::new(
            HashMap::new(),  // default_headers
            vec![],          // middlewares
            vec![],          // rate_limiter_quotas
            None,            // rest_quota_quota
            Some(30),        // timeout_secs
            None,            // proxy_url
        )?;

        Ok(Self {
            base_url,
            client,
            credential,
        })
    }

    /// Returns a reference to the underlying HTTP client.
    pub fn client(&self) -> &HttpClient {
        &self.client
    }

    /// Returns a reference to the credentials.
    pub fn credential(&self) -> Option<&Credential> {
        self.credential.as_ref()
    }
}

/// High-level HTTP client for Longport with domain-level transformations.
///
/// This client wraps the raw client and provides methods that work with
/// Nautilus domain types, handling instrument loading and market data queries.
#[derive(Clone, Debug)]
pub struct LongportHttpClient {
    /// The raw HTTP client.
    raw: Arc<LongportRawHttpClient>,
}

impl LongportHttpClient {
    /// Creates a new [`LongportHttpClient`].
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for LongPort HTTP API.
    /// * `credential` - Optional credentials for authenticated requests.
    pub fn new(base_url: String, credential: Option<Credential>) -> anyhow::Result<Self> {
        let raw = Arc::new(LongportRawHttpClient::new(base_url, credential)?);
        Ok(Self { raw })
    }

    /// Returns a reference to the raw HTTP client.
    pub fn raw(&self) -> &LongportRawHttpClient {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_raw_client() {
        let client = LongportRawHttpClient::new(
            "https://open.longport.com".to_string(),
            None,
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_create_raw_client_empty_url() {
        let client = LongportRawHttpClient::new(String::new(), None);
        assert!(client.is_err());
    }

    #[test]
    fn test_create_high_level_client() {
        let client = LongportHttpClient::new(
            "https://open.longport.com".to_string(),
            None,
        );
        assert!(client.is_ok());
    }
}
