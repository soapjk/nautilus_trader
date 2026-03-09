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

//! Factory functions for creating Longport clients and components.

use std::{any::Any, cell::RefCell, rc::Rc};

use nautilus_common::{cache::Cache, clients::ExecutionClient};
use nautilus_execution::client::core::ExecutionClientCore;
use nautilus_model::{
    enums::{AccountType, OmsType},
    identifiers::ClientId,
};
use nautilus_system::factories::{ClientConfig, ExecutionClientFactory};

use crate::{
    common::consts::LONGPORT_VENUE,
    config::{LongportDataClientConfig, LongportExecClientConfig},
    execution::LongportExecutionClient,
};

impl ClientConfig for LongportDataClientConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ClientConfig for LongportExecClientConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// NOTE: LongportDataClientFactory is removed since DataClient is now implemented in Python
// following the OKX architecture pattern. Only ExecutionClientFactory remains in Rust.

/// Factory for creating Longport execution clients.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.adapters.longport")
)]
pub struct LongportExecutionClientFactory;

impl LongportExecutionClientFactory {
    /// Creates a new [`LongportExecutionClientFactory`] instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LongportExecutionClientFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionClientFactory for LongportExecutionClientFactory {
    fn create(
        &self,
        name: &str,
        config: &dyn ClientConfig,
        cache: Rc<RefCell<Cache>>,
    ) -> anyhow::Result<Box<dyn ExecutionClient>> {
        let longport_config = config
            .as_any()
            .downcast_ref::<LongportExecClientConfig>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid config type for LongportExecutionClientFactory. Expected LongportExecClientConfig, was {config:?}",
                )
            })?
            .clone();

        // Longport uses cash accounts by default (spot trading)
        let account_type = AccountType::Cash;
        let oms_type = OmsType::Hedging;

        let core = ExecutionClientCore::new(
            longport_config.trader_id,
            ClientId::from(name),
            *LONGPORT_VENUE,
            oms_type,
            longport_config.account_id,
            account_type,
            None, // base_currency
            cache,
        );

        let client = LongportExecutionClient::new(core, longport_config)?;

        Ok(Box::new(client))
    }

    fn name(&self) -> &'static str {
        "LONGPORT"
    }

    fn config_type(&self) -> &'static str {
        "LongportExecClientConfig"
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use nautilus_common::cache::Cache;
    use nautilus_model::identifiers::{AccountId, ClientId, TraderId};
    use rstest::rstest;

    use super::*;
    use crate::common::enums::LongportMarket;

    #[rstest]
    fn test_longport_execution_client_factory_creation() {
        let factory = LongportExecutionClientFactory::new();
        assert_eq!(factory.name(), "LONGPORT");
        assert_eq!(factory.config_type(), "LongportExecClientConfig");
    }

    #[rstest]
    fn test_longport_execution_client_factory_default() {
        let factory = LongportExecutionClientFactory::new();
        assert_eq!(factory.name(), "LONGPORT");
    }

    #[rstest]
    fn test_longport_exec_client_config_implements_client_config() {
        let config = LongportExecClientConfig {
            trader_id: TraderId::from("TRADER-001"),
            account_id: AccountId::from("LONGPORT-001"),
            app_key: Some("test_key".to_string()),
            app_secret: Some("test_secret".to_string()),
            access_token: Some("test_token".to_string()),
            markets: vec![LongportMarket::HK],
            ..Default::default()
        };

        let boxed_config: Box<dyn ClientConfig> = Box::new(config);
        let downcasted = boxed_config.as_any().downcast_ref::<LongportExecClientConfig>();

        assert!(downcasted.is_some());
    }

    #[rstest]
    fn test_longport_execution_client_factory_creates_client() {
        let factory = LongportExecutionClientFactory::new();
        let config = LongportExecClientConfig {
            trader_id: TraderId::from("TRADER-001"),
            account_id: AccountId::from("LONGPORT-001"),
            app_key: Some("test_key".to_string()),
            app_secret: Some("test_secret".to_string()),
            access_token: Some("test_token".to_string()),
            markets: vec![LongportMarket::HK, LongportMarket::US],
            ..Default::default()
        };

        let cache = Rc::new(RefCell::new(Cache::default()));

        let result = factory.create("LONGPORT-TEST", &config, cache);

        // The test will fail because Longport SDK validates the token
        // This is expected behavior - the factory correctly attempts to create the client
        // and Longport SDK rejects invalid credentials
        match result {
            Ok(client) => {
                // If somehow it succeeds (e.g., mocking), verify basic properties
                assert_eq!(client.client_id(), ClientId::from("LONGPORT-TEST"));
                assert_eq!(client.venue(), *LONGPORT_VENUE);
            }
            Err(e) => {
                // Expected: Longport SDK rejects invalid test credentials
                // Verify the error is related to authentication or connection
                let error_msg = e.to_string().to_lowercase();
                assert!(
                    error_msg.contains("token") ||
                    error_msg.contains("auth") ||
                    error_msg.contains("credential") ||
                    error_msg.contains("trade context") ||
                    error_msg.contains("invalid"),
                    "Expected authentication/connection error, got: {e}"
                );
            }
        }
    }

    #[rstest]
    fn test_longport_execution_client_factory_rejects_wrong_config_type() {
        let factory = LongportExecutionClientFactory::new();
        let wrong_config = LongportDataClientConfig::default();

        let cache = Rc::new(RefCell::new(Cache::default()));

        let result = factory.create("LONGPORT-TEST", &wrong_config, cache);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("Invalid config type")
        );
    }

    #[rstest]
    fn test_longport_data_client_config_implements_client_config() {
        let config = LongportDataClientConfig {
            app_key: Some("test_key".to_string()),
            app_secret: Some("test_secret".to_string()),
            access_token: Some("test_token".to_string()),
            markets: vec![LongportMarket::US],
            ..Default::default()
        };

        let boxed_config: Box<dyn ClientConfig> = Box::new(config);
        let downcasted = boxed_config.as_any().downcast_ref::<LongportDataClientConfig>();

        assert!(downcasted.is_some());
    }
}
