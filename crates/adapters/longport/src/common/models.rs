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

//! Common data models for the Longport adapter.

use crate::common::enums::LongportMarket;
use serde::{Deserialize, Serialize};

/// Represents a Longport instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongportInstrument {
    /// The instrument symbol (e.g., "700" for Tencent).
    pub symbol: String,
    /// The instrument name.
    pub name: String,
    /// The market where the instrument is traded.
    pub market: LongportMarket,
    /// The lot size (minimum order quantity).
    #[serde(default = "default_lot_size")]
    pub lot_size: i64,
    /// The price tick size.
    #[serde(default = "default_tick_size")]
    pub tick_size: f64,
    /// Whether short selling is allowed.
    #[serde(default)]
    pub shortable: bool,
    /// Whether margin trading is allowed.
    #[serde(default)]
    pub marginable: bool,
}

fn default_lot_size() -> i64 {
    100
}

fn default_tick_size() -> f64 {
    0.01
}

impl LongportInstrument {
    /// Creates a new [`LongportInstrument`].
    #[must_use]
    pub const fn new(
        symbol: String,
        name: String,
        market: LongportMarket,
    ) -> Self {
        Self {
            symbol,
            name,
            market,
            lot_size: 100,
            tick_size: 0.01,
            shortable: false,
            marginable: false,
        }
    }

    /// Creates a new [`LongportInstrument`] with the specified lot size.
    #[must_use]
    pub fn new_with_lot_size(
        symbol: String,
        name: String,
        market: LongportMarket,
        lot_size: i64,
    ) -> Self {
        Self {
            symbol,
            name,
            market,
            lot_size,
            tick_size: 0.01,
            shortable: false,
            marginable: false,
        }
    }

    /// Returns the full instrument ID symbol.
    #[must_use]
    pub fn id(&self) -> String {
        format!("{}.{}", self.symbol, self.market.as_str())
    }
}

/// Represents Longport account balance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongportAccountBalance {
    /// Total cash balance.
    pub total_cash: f64,
    /// Available cash for trading.
    pub available_cash: f64,
    /// Currency of the balance.
    pub currency: String,
    /// Buying power (for margin trading).
    pub buying_power: Option<f64>,
    /// Net asset value.
    pub net_asset_value: Option<f64>,
}

impl LongportAccountBalance {
    /// Creates a new [`LongportAccountBalance`].
    #[must_use]
    pub const fn new(
        total_cash: f64,
        available_cash: f64,
        currency: String,
        buying_power: Option<f64>,
        net_asset_value: Option<f64>,
    ) -> Self {
        Self {
            total_cash,
            available_cash,
            currency,
            buying_power,
            net_asset_value,
        }
    }
}

/// Represents Longport position information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongportPosition {
    /// The instrument symbol.
    pub symbol: String,
    /// The market.
    pub market: LongportMarket,
    /// The quantity of the position.
    pub quantity: i64,
    /// Whether the position is long (positive) or short (negative).
    pub side: String,
    /// The average entry price.
    pub avg_price: f64,
    /// The market value of the position.
    pub market_value: f64,
    /// The cost basis of the position.
    pub cost_basis: f64,
    /// The unrealized PnL.
    pub unrealized_pnl: f64,
}

impl LongportPosition {
    /// Creates a new [`LongportPosition`].
    #[must_use]
    pub const fn new(
        symbol: String,
        market: LongportMarket,
        quantity: i64,
        side: String,
        avg_price: f64,
        market_value: f64,
        cost_basis: f64,
        unrealized_pnl: f64,
    ) -> Self {
        Self {
            symbol,
            market,
            quantity,
            side,
            avg_price,
            market_value,
            cost_basis,
            unrealized_pnl,
        }
    }

    /// Returns the full instrument ID symbol.
    #[must_use]
    pub fn instrument_id(&self) -> String {
        format!("{}.{}", self.symbol, self.market.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longport_instrument_id() {
        let instrument = LongportInstrument::new(
            "700".to_string(),
            "Tencent".to_string(),
            LongportMarket::HK,
        );
        assert_eq!(instrument.id(), "700.HK");
    }

    #[test]
    fn test_longport_position_instrument_id() {
        let position = LongportPosition {
            symbol: "AAPL".to_string(),
            market: LongportMarket::US,
            quantity: 100,
            side: "Long".to_string(),
            avg_price: 150.0,
            market_value: 15000.0,
            cost_basis: 15000.0,
            unrealized_pnl: 0.0,
        };
        assert_eq!(position.instrument_id(), "AAPL.US");
    }
}
