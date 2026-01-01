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

use std::env;

use nautilus_data::client::DataClient;
use nautilus_model::identifiers::ClientId;

/// Test binary for Longport adapter connectivity.
fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get credentials from environment or command line
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

    println!("Longport Adapter Test");
    println!("====================");
    println!("App Key: {}", app_key);
    println!("App Secret: ***");
    println!("Access Token: ***");
    println!();

    // Create data client
    let client_id = ClientId::from("LONGPORT");
    let config = nautilus_longport::config::LongportDataClientConfig {
        app_key: Some(app_key),
        app_secret: Some(app_secret),
        access_token: Some(access_token),
        markets: vec![nautilus_longport::common::enums::LongportMarket::HK],
        ..Default::default()
    };

    match nautilus_longport::data::LongportDataClient::new(client_id, config) {
        Ok(client) => {
            println!("✓ Data client created successfully");
            println!("  Client ID: {}", client.client_id());
            println!("  Venue: {:?}", client.venue());
        }
        Err(e) => {
            eprintln!("✗ Failed to create data client: {e}");
            std::process::exit(1);
        }
    }

    println!();
    println!("Test completed successfully!");

    Ok(())
}
