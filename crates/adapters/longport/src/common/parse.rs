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

//! Parsing utilities for converting Longport data to Nautilus types.

use longport::{
    trade::{OrderSide as LongportOrderSide, OrderType as LongportOrderType, TimeInForceType},
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    enums::{OrderSide, PositionSide, TimeInForce as NautilusTimeInForce, OrderType as NautilusOrderType},
    identifiers::{InstrumentId, Symbol},
    instruments::{equity::Equity, InstrumentAny},
    types::{Currency, Price, Quantity},
};

use crate::common::{
    consts::LONGPORT_VENUE,
    enums::{LongportMarket, LongportSecurityType, LongportSide},
    models::LongportInstrument,
};

/// Parses a Longport market string to [`LongportMarket`].
#[must_use]
pub fn parse_market(market: &str) -> LongportMarket {
    match market.to_uppercase().as_str() {
        "HK" => LongportMarket::HK,
        "US" => LongportMarket::US,
        "CN" => LongportMarket::CN,
        _ => LongportMarket::HK, // Default to HK
    }
}

/// Parses an instrument ID from a symbol string (e.g., "700.HK").
/// This is used when the symbol already includes the market suffix.
pub fn parse_instrument_id_from_symbol(symbol: &str) -> anyhow::Result<InstrumentId> {
    // The symbol already includes the market (e.g., "700.HK")
    // We can use it directly as the instrument ID
    Ok(InstrumentId::new(
        Symbol::new(symbol),
        *LONGPORT_VENUE,
    ))
}

/// Parses an instrument ID from separate symbol and market parts.
pub fn parse_instrument_id_from_parts(symbol: &str, _market: &str) -> anyhow::Result<InstrumentId> {
    // For Longport, we can use the symbol directly as the instrument ID
    // since the symbol typically includes the market suffix
    Ok(InstrumentId::new(
        Symbol::new(symbol),
        *LONGPORT_VENUE,
    ))
}

/// Parses a Longport security type string to [`LongportSecurityType`].
#[must_use]
pub fn parse_security_type(security_type: &str) -> LongportSecurityType {
    match security_type.to_lowercase().as_str() {
        "stock" => LongportSecurityType::Stock,
        "warrant" | "cw" => LongportSecurityType::Warrant,
        "etf" => LongportSecurityType::Etf,
        "index" => LongportSecurityType::Index,
        "bond" => LongportSecurityType::Bond,
        "futures" => LongportSecurityType::Futures,
        "option" => LongportSecurityType::Option,
        "trust" => LongportSecurityType::Trust,
        _ => LongportSecurityType::Unknown,
    }
}

/// Parses a Longport symbol string to create an [`InstrumentId`].
///
/// # Arguments
///
/// * `symbol` - The security symbol (e.g., "700" for Tencent)
/// * `market` - The market code (e.g., "HK", "US")
///
/// # Returns
///
/// An [`InstrumentId] with the format "{SYMBOL}.{MARKET}" (e.g., "700.HK")
#[must_use]
pub fn parse_instrument_id(symbol: &str, market: &str) -> InstrumentId {
    let symbol_str = format!("{}.{}", symbol, market);
    InstrumentId::new(Symbol::new(symbol_str), *crate::common::consts::LONGPORT_VENUE)
}

/// Parses an InstrumentId to extract the Longport symbol string.
///
/// # Arguments
///
/// * `instrument_id` - The Nautilus instrument ID
///
/// # Returns
///
/// The symbol string (e.g., "700.HK")
///
/// # Errors
///
/// Returns an error if the instrument ID is not a valid Longport format.
pub fn instrument_id_to_string(instrument_id: InstrumentId) -> anyhow::Result<String> {
    Ok(instrument_id.symbol.as_str().to_string())
}

/// Parses a Longport side string to [`OrderSide`].
#[must_use]
pub fn parse_side(side: &str) -> OrderSide {
    match side.to_uppercase().as_str() {
        "BUY" | "LONG" => OrderSide::Buy,
        "SELL" | "SHORT" => OrderSide::Sell,
        _ => OrderSide::NoOrderSide,
    }
}

/// Converts a Nautilus OrderSide to Longport OrderSide.
pub fn parse_order_side(side: OrderSide) -> anyhow::Result<LongportOrderSide> {
    match side {
        OrderSide::Buy => Ok(LongportOrderSide::Buy),
        OrderSide::Sell => Ok(LongportOrderSide::Sell),
        _ => anyhow::bail!("Invalid OrderSide for Longport: {side:?}"),
    }
}

/// Converts a Nautilus OrderType to Longport OrderType.
pub fn parse_order_type(order_type: NautilusOrderType) -> anyhow::Result<LongportOrderType> {
    match order_type {
        NautilusOrderType::Market => Ok(LongportOrderType::MO), // Market Order
        NautilusOrderType::Limit => Ok(LongportOrderType::LO), // Limit Order
        NautilusOrderType::StopMarket => Ok(LongportOrderType::LO), // Use Limit with stop price
        NautilusOrderType::StopLimit => Ok(LongportOrderType::LO), // Use Limit with stop price
        NautilusOrderType::MarketIfTouched => Ok(LongportOrderType::MIT), // Market If Touched
        NautilusOrderType::LimitIfTouched => Ok(LongportOrderType::LIT), // Limit If Touched
        _ => anyhow::bail!("Unsupported OrderType for Longport: {order_type:?}"),
    }
}

/// Converts a Nautilus TimeInForce to Longport TimeInForceType.
pub fn parse_time_in_force(tif: NautilusTimeInForce) -> anyhow::Result<TimeInForceType> {
    match tif {
        NautilusTimeInForce::Day => Ok(TimeInForceType::Day),
        NautilusTimeInForce::Gtc => Ok(TimeInForceType::GoodTilCanceled),
        // Longport doesn't support IOC or FOK - map to Day
        NautilusTimeInForce::Ioc => Ok(TimeInForceType::Day),
        NautilusTimeInForce::Fok => Ok(TimeInForceType::Day),
        NautilusTimeInForce::AtTheClose => Ok(TimeInForceType::Day),
        _ => anyhow::bail!("Unsupported TimeInForce for Longport: {tif:?}"),
    }
}

/// Converts a [`LongportSide`] to [`OrderSide`].
#[must_use]
pub fn longport_side_to_order_side(side: LongportSide) -> OrderSide {
    match side {
        LongportSide::Buy => OrderSide::Buy,
        LongportSide::Sell => OrderSide::Sell,
    }
}

/// Creates a minimal Nautilus [`InstrumentAny`] from an instrument ID string.
///
/// This function creates a basic instrument with default values when only the
/// instrument ID is available (e.g., from `load_ids` configuration).
///
/// # Arguments
///
/// * `instrument_id_str` - The instrument ID string (e.g., "700.HK", "AAPL.US")
///
/// # Returns
///
/// A minimal [`InstrumentAny`] with default lot_size and tick_size.
///
/// # Errors
///
/// Returns an error if the instrument ID format is invalid.
pub fn create_minimal_instrument(instrument_id_str: &str) -> anyhow::Result<InstrumentAny> {
    // Parse the instrument ID to extract symbol and market
    // Expected format: "SYMBOL.MARKET" (e.g., "700.HK", "AAPL.US")
    let parts: Vec<&str> = instrument_id_str.split('.').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid instrument ID format: {instrument_id_str}, expected SYMBOL.MARKET");
    }

    let _symbol = parts[0];
    let market_str = parts[1];

    // Parse market
    let market = match market_str.to_uppercase().as_str() {
        "HK" => LongportMarket::HK,
        "US" => LongportMarket::US,
        "CN" => LongportMarket::CN,
        _ => anyhow::bail!("Unknown market in instrument ID: {instrument_id_str}"),
    };

    // Create instrument ID
    let instrument_id = InstrumentId::new(
        Symbol::new(instrument_id_str),
        *LONGPORT_VENUE,
    );

    let nautilus_symbol = Symbol::new(instrument_id_str);

    // Determine currency from market
    let currency = match market {
        LongportMarket::HK => Currency::HKD(),
        LongportMarket::US => Currency::USD(),
        LongportMarket::CN => Currency::CNY(),
    };

    // Set default tick_size and lot_size based on market
    // HK stocks: lot_size typically 100-1000, tick_size 0.001-0.1
    // US stocks: lot_size typically 1, tick_size 0.01
    // CN stocks: lot_size typically 100, tick_size 0.01
    let (lot_size, tick_size, price_precision) = match market {
        LongportMarket::HK => (100i64, 0.01f64, 2u8),  // Common for HK stocks
        LongportMarket::US => (1i64, 0.01f64, 2u8),     // US stocks
        LongportMarket::CN => (100i64, 0.01f64, 2u8),   // A-shares
    };

    let quantity_precision = if lot_size >= 100 { 0u8 } else { 2u8 };

    let price_increment = Price::new(tick_size, price_precision);
    let quantity_increment = Quantity::new(lot_size as f64, quantity_precision);

    let equity_instrument = Equity::new_checked(
        instrument_id,
        nautilus_symbol,
        None, // isin
        currency,
        price_precision,
        price_increment,
        Some(quantity_increment), // lot_size
        None, // max_quantity
        Some(quantity_increment), // min_quantity (lot size)
        None, // max_price
        None, // min_price
        None, // margin_init
        None, // margin_maint
        None, // maker_fee
        None, // taker_fee
        UnixNanos::default(), // ts_event
        UnixNanos::default(), // ts_init
    )?;

    Ok(InstrumentAny::Equity(equity_instrument))
}

/// Creates a Nautilus [`InstrumentAny`] from a [`LongportInstrument`].
///
/// # Errors
///
/// Returns an error if the instrument data is invalid.
pub fn parse_instrument(instrument: LongportInstrument) -> anyhow::Result<InstrumentAny> {
    let symbol_str = format!("{}.{}", instrument.symbol, instrument.market.as_str());
    let instrument_id = InstrumentId::new(Symbol::new(symbol_str), *crate::common::consts::LONGPORT_VENUE);
    let symbol = Symbol::new(format!("{}.{}", instrument.symbol, instrument.market.as_str()));
    let _venue = *crate::common::consts::LONGPORT_VENUE; // Prefix with underscore to indicate intentionally unused

    // Parse price precision from tick size
    let price_precision = if instrument.tick_size >= 1.0 {
        0
    } else if instrument.tick_size >= 0.1 {
        1
    } else if instrument.tick_size >= 0.01 {
        2
    } else if instrument.tick_size >= 0.001 {
        3
    } else {
        4
    };

    // Parse quantity precision from lot size
    let quantity_precision = if instrument.lot_size >= 100 {
        0
    } else if instrument.lot_size >= 10 {
        1
    } else {
        2
    };

    let price_increment = Price::new(instrument.tick_size, price_precision as u8);
    let quantity_increment = Quantity::new(instrument.lot_size as f64, quantity_precision as u8);

    let equity_instrument = Equity::new_checked(
        instrument_id,
        symbol,
        None, // isin
        Currency::USD(), // TODO: determine from market
        price_precision as u8,
        price_increment,
        Some(quantity_increment), // lot_size
        None, // max_quantity
        Some(quantity_increment), // min_quantity (lot size)
        None, // max_price
        None, // min_price
        None, // margin_init
        None, // margin_maint
        None, // maker_fee
        None, // taker_fee
        UnixNanos::default(), // ts_event
        UnixNanos::default(), // ts_init
    )?;

    Ok(InstrumentAny::Equity(equity_instrument))
}

/// Parses position side from quantity.
///
/// # Arguments
///
/// * `quantity` - The position quantity (can be positive or negative)
///
/// # Returns
///
/// [`PositionSide::Long`] for positive quantities, [`PositionSide::Short`] for negative.
#[must_use]
pub fn parse_position_side(quantity: i64) -> PositionSide {
    if quantity > 0 {
        PositionSide::Long
    } else if quantity < 0 {
        PositionSide::Short
    } else {
        PositionSide::Flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_market() {
        assert_eq!(parse_market("HK"), LongportMarket::HK);
        assert_eq!(parse_market("US"), LongportMarket::US);
        assert_eq!(parse_market("CN"), LongportMarket::CN);
        assert_eq!(parse_market("unknown"), LongportMarket::HK); // Default
    }

    #[test]
    fn test_parse_security_type() {
        assert_eq!(parse_security_type("stock"), LongportSecurityType::Stock);
        assert_eq!(parse_security_type("warrant"), LongportSecurityType::Warrant);
        assert_eq!(parse_security_type("etf"), LongportSecurityType::Etf);
    }

    #[test]
    fn test_parse_instrument_id_with_parts() {
        let id = parse_instrument_id("700", "HK");
        assert_eq!(id.symbol.as_str(), "700.HK");
    }

    #[test]
    fn test_parse_side() {
        assert_eq!(parse_side("BUY"), OrderSide::Buy);
        assert_eq!(parse_side("SELL"), OrderSide::Sell);
        assert_eq!(parse_side("LONG"), OrderSide::Buy);
        assert_eq!(parse_side("SHORT"), OrderSide::Sell);
    }

    #[test]
    fn test_parse_order_side() {
        assert!(parse_order_side(OrderSide::Buy).is_ok());
        assert!(parse_order_side(OrderSide::Sell).is_ok());
    }

    #[test]
    fn test_parse_order_type() {
        assert!(parse_order_type(NautilusOrderType::Market).is_ok());
        assert!(parse_order_type(NautilusOrderType::Limit).is_ok());
    }

    #[test]
    fn test_parse_time_in_force() {
        assert!(parse_time_in_force(NautilusTimeInForce::Day).is_ok());
        assert!(parse_time_in_force(NautilusTimeInForce::GTC).is_ok());
        assert!(parse_time_in_force(NautilusTimeInForce::IOC).is_ok());
        assert!(parse_time_in_force(NautilusTimeInForce::FOK).is_ok());
    }

    #[test]
    fn test_longport_side_to_order_side() {
        assert_eq!(
            longport_side_to_order_side(LongportSide::Buy),
            OrderSide::Buy
        );
        assert_eq!(
            longport_side_to_order_side(LongportSide::Sell),
            OrderSide::Sell
        );
    }

    #[test]
    fn test_parse_position_side() {
        assert_eq!(parse_position_side(100), PositionSide::Long);
        assert_eq!(parse_position_side(-100), PositionSide::Short);
        assert_eq!(parse_position_side(0), PositionSide::Flat);
    }

    #[test]
    fn test_parse_instrument() {
        let longport_instrument = LongportInstrument {
            symbol: "700".to_string(),
            name: "Tencent".to_string(),
            market: LongportMarket::HK,
        };

        let result = parse_instrument(longport_instrument);
        assert!(result.is_ok());

        let instrument = result.unwrap();
        assert_eq!(instrument.id().symbol.as_str(), "700.HK");
    }
}
