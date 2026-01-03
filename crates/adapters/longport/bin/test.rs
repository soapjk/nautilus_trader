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

//! Test binary for Longport adapter connectivity.
//!
//! This tests the Rust HTTP and WebSocket clients that wrap the Longport SDK.

use std::env;

/// Test the Longport adapter clients.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // Create Longport configuration
    let config = longport::Config::new(app_key, app_secret, access_token);

    // Test QuoteContext creation
    println!("Testing QuoteContext creation...");
    let (quote_ctx, _event_receiver) = longport::quote::QuoteContext::try_new(std::sync::Arc::new(config))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create QuoteContext: {e}"))?;

    println!("✓ QuoteContext created successfully");

    // Test creating HTTP client from context
    println!("Testing HTTP client creation...");
    let http_client = nautilus_longport::http::LongportHttpClient::from_context_internal(
        std::sync::Arc::new(quote_ctx.clone())
    );
    println!("✓ HTTP client created successfully");

    // Test creating WebSocket client from context
    println!("Testing WebSocket client creation...");
    let ws_client = nautilus_longport::websocket::LongportWebSocketClient::from_context_internal(
        std::sync::Arc::new(quote_ctx)
    );
    println!("✓ WebSocket client created successfully");

    println!();
    println!("Test completed successfully!");
    println!();
    println!("Note: DataClient is now implemented in Python following the OKX pattern.");
    println!("Use the Python LiveMarketDataClient for full functionality.");

    Ok(())
}
