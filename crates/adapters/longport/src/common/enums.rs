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

//! Enumerations mapping Longport concepts onto idiomatic Nautilus variants.

use nautilus_model::enums::{
    AggressorSide, LiquiditySide, OrderSide, OrderStatus, OrderType, PositionSide,
};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString};

/// Represents the side of an order or trade (Buy/Sell).
#[derive(
    Copy,
    Clone,
    Debug,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(eq, eq_int, module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
pub enum LongportSide {
    /// Buy side of a trade or order.
    Buy,
    /// Sell side of a trade or order.
    Sell,
}

impl From<OrderSide> for LongportSide {
    fn from(value: OrderSide) -> Self {
        match value {
            OrderSide::Buy => Self::Buy,
            OrderSide::Sell => Self::Sell,
            _ => panic!("Invalid `OrderSide`"),
        }
    }
}

impl From<LongportSide> for AggressorSide {
    fn from(value: LongportSide) -> Self {
        match value {
            LongportSide::Buy => Self::Buyer,
            LongportSide::Sell => Self::Seller,
        }
    }
}

impl From<LongportSide> for OrderSide {
    fn from(side: LongportSide) -> Self {
        match side {
            LongportSide::Buy => Self::Buy,
            LongportSide::Sell => Self::Sell,
        }
    }
}

/// Represents the available order types on Longport.
#[derive(
    Copy,
    Clone,
    Debug,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(eq, eq_int, module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
pub enum LongportOrderType {
    /// Market order, executed immediately at current market price.
    Market,
    /// Limit order, executed only at specified price or better.
    Limit,
    /// Stop market order (stop loss).
    Stop,
    /// Stop limit order.
    StopLimit,
    /// Market if touched order.
    MarketIfTouched,
    /// Limit if touched order.
    LimitIfTouched,
    /// One-cancels-other order.
    Oco,
    /// Trail stop order.
    TrailStop,
    /// Unknown order type.
    Unknown,
}

impl From<OrderType> for LongportOrderType {
    fn from(value: OrderType) -> Self {
        match value {
            OrderType::Market => Self::Market,
            OrderType::Limit => Self::Limit,
            OrderType::StopMarket => Self::Stop,
            OrderType::StopLimit => Self::StopLimit,
            OrderType::MarketIfTouched => Self::MarketIfTouched,
            OrderType::LimitIfTouched => Self::LimitIfTouched,
            _ => panic!("Invalid `OrderType` for Longport: {value:?}"),
        }
    }
}

impl From<LongportOrderType> for OrderType {
    fn from(ord_type: LongportOrderType) -> Self {
        match ord_type {
            LongportOrderType::Market => Self::Market,
            LongportOrderType::Limit => Self::Limit,
            LongportOrderType::Stop => Self::StopMarket,
            LongportOrderType::StopLimit => Self::StopLimit,
            LongportOrderType::MarketIfTouched => Self::MarketIfTouched,
            LongportOrderType::LimitIfTouched => Self::LimitIfTouched,
            LongportOrderType::Oco | LongportOrderType::TrailStop | LongportOrderType::Unknown => {
                panic!("Invalid `LongportOrderType` for Nautilus: {ord_type:?}")
            }
        }
    }
}

/// Represents the possible states of an order throughout its lifecycle.
#[derive(
    Copy,
    Clone,
    Debug,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LongportOrderStatus {
    /// Order is being processed by the exchange.
    Pending,
    /// Order has been reported as new.
    Reported,
    /// Order has been partially filled.
    PartiallyFilled,
    /// Order has been completely filled.
    Filled,
    /// Order has been canceled.
    Canceled,
    /// Order has been rejected.
    Rejected,
    /// Order is being modified.
    Replacing,
    /// Order modification failed.
    ReplaceRejected,
    /// Unknown status.
    Unknown,
}

impl From<OrderStatus> for LongportOrderStatus {
    fn from(value: OrderStatus) -> Self {
        match value {
            OrderStatus::Submitted => Self::Pending,
            OrderStatus::Accepted => Self::Reported,
            OrderStatus::PartiallyFilled => Self::PartiallyFilled,
            OrderStatus::Filled => Self::Filled,
            OrderStatus::Canceled => Self::Canceled,
            OrderStatus::Rejected => Self::Rejected,
            _ => panic!("Invalid `OrderStatus`"),
        }
    }
}

impl From<LongportOrderStatus> for OrderStatus {
    fn from(status: LongportOrderStatus) -> Self {
        match status {
            LongportOrderStatus::Pending => Self::Submitted,
            LongportOrderStatus::Reported => Self::Accepted,
            LongportOrderStatus::PartiallyFilled => Self::PartiallyFilled,
            LongportOrderStatus::Filled => Self::Filled,
            LongportOrderStatus::Canceled => Self::Canceled,
            LongportOrderStatus::Rejected => Self::Rejected,
            LongportOrderStatus::Replacing | LongportOrderStatus::ReplaceRejected => {
                Self::Submitted
            }
            LongportOrderStatus::Unknown => Self::Accepted,
        }
    }
}

/// Represents the type of execution that generated a trade.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
pub enum LongportExecType {
    #[default]
    None,
    /// Trade was a taker (aggressive).
    Taker,
    /// Trade was a maker (passive).
    Maker,
}

impl From<LiquiditySide> for LongportExecType {
    fn from(value: LiquiditySide) -> Self {
        match value {
            LiquiditySide::NoLiquiditySide => Self::None,
            LiquiditySide::Taker => Self::Taker,
            LiquiditySide::Maker => Self::Maker,
        }
    }
}

impl From<LongportExecType> for LiquiditySide {
    fn from(exec: LongportExecType) -> Self {
        match exec {
            LongportExecType::None => Self::NoLiquiditySide,
            LongportExecType::Taker => Self::Taker,
            LongportExecType::Maker => Self::Maker,
        }
    }
}

/// Represents the market on Longport.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(eq, eq_int, module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
pub enum LongportMarket {
    /// Hong Kong market
    #[default]
    HK,
    /// United States market
    US,
    /// China A-share market
    CN,
}

impl From<String> for LongportMarket {
    fn from(s: String) -> Self {
        match s.to_uppercase().as_str() {
            "HK" => Self::HK,
            "US" => Self::US,
            "CN" => Self::CN,
            _ => Self::HK,
        }
    }
}

impl LongportMarket {
    /// Returns the market code as a string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::HK => "HK",
            Self::US => "US",
            Self::CN => "CN",
        }
    }
}

impl std::fmt::LowerHex for LongportMarket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents the security type on Longport.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(eq, eq_int, module = "nautilus_trader.core.nautilus_pyo3.longport")
)]
pub enum LongportSecurityType {
    #[default]
    /// Stock/Share
    Stock,
    /// Warrant/CW
    Warrant,
    /// ETF
    Etf,
    /// Index
    Index,
    /// Bond
    Bond,
    /// Futures
    Futures,
    /// Option
    Option,
    /// Trust
    Trust,
    /// Unknown security type
    Unknown,
}

impl LongportSecurityType {
    /// Returns the security type code as a string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Stock => "stock",
            Self::Warrant => "warrant",
            Self::Etf => "etf",
            Self::Index => "index",
            Self::Bond => "bond",
            Self::Futures => "futures",
            Self::Option => "option",
            Self::Trust => "trust",
            Self::Unknown => "unknown",
        }
    }
}

/// Represents the position side on Longport.
#[derive(
    Copy,
    Clone,
    Debug,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LongportPositionSide {
    /// Long position
    Long,
    /// Short position
    Short,
    /// Flat position (no position)
    Flat,
}

impl From<PositionSide> for LongportPositionSide {
    fn from(value: PositionSide) -> Self {
        match value {
            PositionSide::Long => Self::Long,
            PositionSide::Short => Self::Short,
            PositionSide::Flat => Self::Flat,
            PositionSide::NoPositionSide => Self::Flat,
        }
    }
}

impl From<LongportPositionSide> for PositionSide {
    fn from(side: LongportPositionSide) -> Self {
        match side {
            LongportPositionSide::Long => Self::Long,
            LongportPositionSide::Short => Self::Short,
            LongportPositionSide::Flat => Self::Flat,
        }
    }
}

/// Represents time in force for Longport orders.
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Display,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LongportTimeInForce {
    #[default]
    /// Good Till Cancelled
    Day,
    /// Good Till Cancelled
    GoodTillCancel,
    /// Immediate or Cancel
    ImmediateOrCancel,
    /// All or None
    AllOrNone,
    /// Fill or Kill
    FillOrKill,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longport_side_from_order_side() {
        assert_eq!(LongportSide::from(OrderSide::Buy), LongportSide::Buy);
        assert_eq!(LongportSide::from(OrderSide::Sell), LongportSide::Sell);
    }

    #[test]
    fn test_order_side_from_longport_side() {
        assert_eq!(OrderSide::from(LongportSide::Buy), OrderSide::Buy);
        assert_eq!(OrderSide::from(LongportSide::Sell), OrderSide::Sell);
    }

    #[test]
    fn test_longport_market_as_str() {
        assert_eq!(LongportMarket::HK.as_str(), "HK");
        assert_eq!(LongportMarket::US.as_str(), "US");
        assert_eq!(LongportMarket::CN.as_str(), "CN");
    }

    #[test]
    fn test_longport_security_type_as_str() {
        assert_eq!(LongportSecurityType::Stock.as_str(), "stock");
        assert_eq!(LongportSecurityType::Warrant.as_str(), "warrant");
        assert_eq!(LongportSecurityType::Etf.as_str(), "etf");
    }
}
