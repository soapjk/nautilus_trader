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

//! Data conversion utilities for Longport to Nautilus types.

use longport::quote;
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{
        Bar, BarType, OrderBookDelta, OrderBookDeltas, QuoteTick, TradeTick,
        order::BookOrder,
    },
    enums::{BarAggregation, OrderSide, PriceType},
    identifiers::{InstrumentId, TradeId},
    instruments::{Instrument, InstrumentAny},
    types::{Price, Quantity},
};
use rust_decimal::prelude::ToPrimitive;

// Re-export the time type from longport's internal usage
type OffsetDateTime = time::OffsetDateTime;

/// Converts a Longport timestamp to Unix nanoseconds.
#[must_use]
pub fn timestamp_to_nanos(ts: OffsetDateTime) -> UnixNanos {
    (ts.unix_timestamp_nanos().max(0) as u64).into()
}

/// Converts a Longport `Decimal` to f64.
#[must_use]
pub fn decimal_to_f64(dec: &rust_decimal::Decimal) -> f64 {
    dec.to_f64().unwrap_or(0.0)
}

/// Creates a QuoteTick from the best bid/ask in depth data.
///
/// # Errors
///
/// Returns an error if the conversion fails.
pub fn depth_to_quote_tick(
    depth: &quote::SecurityDepth,
    instrument: &InstrumentAny,
    ts_event: UnixNanos,
) -> anyhow::Result<Option<QuoteTick>> {
    let instrument_id = instrument.id();

    // Get best bid (first element, highest price)
    let best_bid = depth.bids.first();
    // Get best ask (first element, lowest price)
    let best_ask = depth.asks.first();

    match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => {
            let bid_price = match bid.price {
                Some(p) if p > rust_decimal::Decimal::ZERO => {
                    Price::new(decimal_to_f64(&p), instrument.price_precision())
                }
                _ => return Ok(None),
            };

            let ask_price = match ask.price {
                Some(p) if p > rust_decimal::Decimal::ZERO => {
                    Price::new(decimal_to_f64(&p), instrument.price_precision())
                }
                _ => return Ok(None),
            };

            let bid_size = Quantity::new(bid.volume as f64, instrument.size_precision());
            let ask_size = Quantity::new(ask.volume as f64, instrument.size_precision());

            let quote_tick = QuoteTick::new_checked(
                instrument_id,
                bid_price,
                ask_price,
                bid_size,
                ask_size,
                ts_event,
                ts_event,
            )?;

            Ok(Some(quote_tick))
        }
        _ => Ok(None),
    }
}

/// Converts a Longport `Trade` to a Nautilus `TradeTick`.
///
/// # Errors
///
/// Returns an error if the conversion fails.
pub fn trade_to_trade_tick(
    trade: &quote::Trade,
    instrument: &InstrumentAny,
) -> anyhow::Result<TradeTick> {
    let instrument_id = instrument.id();
    let price = Price::new(decimal_to_f64(&trade.price), instrument.price_precision());
    let size = Quantity::new(trade.volume as f64, instrument.size_precision());
    let ts_event = timestamp_to_nanos(trade.timestamp);
    let ts_init = ts_event;

    // Create trade ID from price, size, and timestamp
    let trade_id = TradeId::new(format!(
        "{}-{}-{}",
        trade.price,
        trade.volume,
        ts_event
    ));

    // Determine aggressor side from trade type if available
    // Longport trade types: empty (normal), *, D, M, P, U, X, Y
    // The trade_type field doesn't directly indicate aggressor side (buyer/seller initiated)
    // so we use NoAggressor as the default
    use nautilus_model::enums::AggressorSide;
    let aggressor_side = AggressorSide::NoAggressor;

    Ok(TradeTick::new(
        instrument_id,
        price,
        size,
        aggressor_side,
        trade_id,
        ts_event,
        ts_init,
    ))
}

/// Converts Longport depth data to OrderBookDeltas.
///
/// # Errors
///
/// Returns an error if the conversion fails.
pub fn security_depth_to_deltas(
    depth: &quote::SecurityDepth,
    instrument: &InstrumentAny,
    ts_event: UnixNanos,
) -> anyhow::Result<OrderBookDeltas> {
    use nautilus_model::enums::BookAction;
    let instrument_id = instrument.id();
    let sequence = 0; // Longport doesn't provide sequence numbers
    let flags = 0; // No special flags
    let mut deltas = Vec::new();

    // Process bids (buy orders)
    for bid in &depth.bids {
        if let Some(price) = bid.price {
            if price > rust_decimal::Decimal::ZERO {
                let price_val = Price::new(decimal_to_f64(&price), instrument.price_precision());
                let size = Quantity::new(bid.volume as f64, instrument.size_precision());
                let order = BookOrder::new(
                    OrderSide::Buy,
                    price_val,
                    size,
                    order_id_from_depth(bid, instrument_id),
                );

                deltas.push(OrderBookDelta::new_checked(
                    instrument_id,
                    BookAction::Add,
                    order,
                    flags,
                    sequence,
                    ts_event,
                    ts_event,
                )?);
            }
        }
    }

    // Process asks (sell orders)
    for ask in &depth.asks {
        if let Some(price) = ask.price {
            if price > rust_decimal::Decimal::ZERO {
                let price_val = Price::new(decimal_to_f64(&price), instrument.price_precision());
                let size = Quantity::new(ask.volume as f64, instrument.size_precision());
                let order = BookOrder::new(
                    OrderSide::Sell,
                    price_val,
                    size,
                    order_id_from_depth(ask, instrument_id),
                );

                deltas.push(OrderBookDelta::new_checked(
                    instrument_id,
                    BookAction::Add,
                    order,
                    flags,
                    sequence,
                    ts_event,
                    ts_event,
                )?);
            }
        }
    }

    Ok(OrderBookDeltas::new_checked(instrument_id, deltas)?)
}

/// Creates an order ID from depth data.
fn order_id_from_depth(depth: &quote::Depth, instrument_id: InstrumentId) -> u64 {
    // Create a unique ID from position, price, and volume
    let price_val = depth.price.map(|p| p.to_string()).unwrap_or_default();
    hash_string(&format!(
        "{}-{}-{}-{}",
        instrument_id,
        depth.position,
        price_val,
        depth.volume
    ))
}

/// Converts a Longport `Candlestick` to a Nautilus `Bar`.
///
/// # Errors
///
/// Returns an error if the conversion fails.
pub fn candlestick_to_bar(
    candlestick: &quote::Candlestick,
    instrument: &InstrumentAny,
    aggregation: BarAggregation,
) -> anyhow::Result<Bar> {
    use nautilus_model::data::BarSpecification;
    use nautilus_model::enums::AggregationSource;

    let instrument_id = instrument.id();
    let open = Price::new(decimal_to_f64(&candlestick.open), instrument.price_precision());
    let high = Price::new(decimal_to_f64(&candlestick.high), instrument.price_precision());
    let low = Price::new(decimal_to_f64(&candlestick.low), instrument.price_precision());
    let close = Price::new(decimal_to_f64(&candlestick.close), instrument.price_precision());
    let volume = Quantity::new(candlestick.volume as f64, instrument.size_precision());
    let ts_event = timestamp_to_nanos(candlestick.timestamp);
    let ts_init = ts_event;

    // Create BarSpecification
    // Use step=1 for single bars
    let spec = BarSpecification::new(1, aggregation, PriceType::Last);
    let bar_type = BarType::new(
        instrument_id,
        spec,
        AggregationSource::External,
    );

    Ok(Bar::new(
        bar_type,
        open,
        high,
        low,
        close,
        volume,
        ts_event,
        ts_init,
    ))
}

/// Helper function for hashing strings.
fn hash_string(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_to_nanos() {
        let dt = OffsetDateTime::from_unix_timestamp(0).unwrap();
        assert_eq!(timestamp_to_nanos(dt), 0);
    }

    #[test]
    fn test_decimal_to_f64() {
        let dec = rust_decimal::Decimal::try_from(123.45).unwrap();
        assert_eq!(decimal_to_f64(&dec), 123.45);
    }
}
