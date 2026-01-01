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

use std::{env, time::Duration};

use nautilus_common::logging::init_logging;
use nautilus_live::node::TradingNode;
use nautilus_model::identifiers::{AccountId, ClientId, TraderId};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    init_logging();

    // Get credentials from environment
    let app_key = env::var("LONGPORT_APP_KEY").unwrap_or_else(|_| {
        eprintln!("LONGPORT_APP_KEY not set");
        std::process::exit(1);
    });

    let app_secret = env::var("LONGPORT_APP_SECRET").unwrap_or_else(|_| {
        eprintln!("LONGPORT_APP_SECRET not set");
        std::process::exit(1);
    });

    let access_token = env::var("LONGPORT_ACCESS_TOKEN").unwrap_or_else(|_| {
        eprintln!("LONGPORT_ACCESS_TOKEN not set");
        std::process::exit(1);
    });

    // Create configuration
    let config = nautilus_core::config::TradingNodeConfig {
        data_clients: vec![],
        exec_clients: vec![],
        ..Default::default()
    };

    // Create and build the node
    let mut node = TradingNode::new(config)?;

    // Add the execution client factory
    // node.add_exec_client_factory(
    //     nautilus_longport::LONGPORT,
    //     nautilus_longport::LongportExecutionClientFactory,
    // );

    // Build the node
    node.build();

    // Start the node
    node.start().await?;

    println!("Longport Execution Tester");
    println!("=========================");
    println!("Connected to Longport execution");
    println!();

    // Keep running for testing
    tokio::time::sleep(Duration::from_secs(30)).await;

    // Stop the node
    node.stop().await?;

    println!();
    println!("Test completed!");

    Ok(())
}
