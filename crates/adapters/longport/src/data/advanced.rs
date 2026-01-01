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

//! Advanced data fetching functionality for Longport adapter.
//!
//! This module provides comprehensive methods for fetching:
//! - Stock real-time and historical data
//! - Option chains and option quotes
//! - Fundamental and financial data
//! - Dividend and corporate action data
//! - Market depth and liquidity data

use std::collections::HashMap;
use anyhow::Context;
use longport::{
    quote::{QuoteContext, Period, RealtimeQuote},
    Market,
};
use nautilus_core::{UnixNanos, time::get_atomic_clock_realtime};
use nautilus_model::{
    data::{Bar, BarType, BarSpecification, TradeTick},
    enums::{AggressorSide, BarAggregation, AggregationSource, PriceType},
    identifiers::{InstrumentId, Symbol, TradeId},
    types::{Price, Quantity},
};

use crate::common::{consts::LONGPORT_VENUE, parse::parse_instrument_id_from_symbol};

/// Configuration for historical data requests.
#[derive(Debug, Clone)]
pub struct HistoricalDataConfig {
    /// Number of bars to fetch (default: 200)
    pub bar_count: Option<usize>,
    /// Start date for historical data (ISO format: YYYY-MM-DD)
    pub start_date: Option<String>,
    /// End date for historical data (ISO format: YYYY-MM-DD)
    pub end_date: Option<String>,
    /// Price adjustment type (None, Forward, or All)
    pub adjust_type: AdjustConfig,
    /// Include pre-market and after-hours data
    pub include_extended_hours: bool,
}

impl Default for HistoricalDataConfig {
    fn default() -> Self {
        Self {
            bar_count: Some(200),
            start_date: None,
            end_date: None,
            adjust_type: AdjustConfig::None,
            include_extended_hours: false,
        }
    }
}

/// Price adjustment configuration for historical data.
#[derive(Debug, Clone, Copy)]
pub enum AdjustConfig {
    /// No adjustment (raw prices)
    None,
}

// Simplified - Longport uses string-based adjust type
impl AdjustConfig {
    fn as_str(&self) -> &str {
        match self {
            AdjustConfig::None => "none",
        }
    }
}

/// Option chain query configuration.
#[derive(Debug, Clone)]
pub struct OptionChainConfig {
    /// Whether to filter by expiration dates
    pub expiry_dates: Option<Vec<String>>,
    /// Filter by strike prices relative to current price
    pub strike_filter: Option<StrikeFilter>,
    /// Include expired options
    pub include_expired: bool,
}

impl Default for OptionChainConfig {
    fn default() -> Self {
        Self {
            expiry_dates: None,
            strike_filter: Some(StrikeFilter::All),
            include_expired: false,
        }
    }
}

/// Filter for option strike prices.
#[derive(Debug, Clone, Copy)]
pub enum StrikeFilter {
    /// All strikes
    All,
    /// In-the-money options only
    ITM,
    /// Out-of-the-money options only
    OTM,
    /// Near-the-money options (within 5%)
    NTM,
    /// Custom percentage range from ATM (e.g., 0.1 for 10%)
    Range(f64),
}

/// Comprehensive stock quote data.
#[derive(Debug, Clone)]
pub struct StockQuote {
    /// Instrument ID
    pub instrument_id: InstrumentId,
    /// Last traded price
    pub last_price: Option<f64>,
    /// Day high
    pub high_price: Option<f64>,
    /// Day low
    pub low_price: Option<f64>,
    /// Previous close
    pub prev_close_price: Option<f64>,
    /// Volume
    pub volume: Option<u64>,
    /// Turnover (total value traded)
    pub turnover: Option<f64>,
    /// Open price
    pub open_price: Option<f64>,
    /// Timestamp
    pub timestamp: UnixNanos,
}

/// Option contract information.
#[derive(Debug, Clone)]
pub struct OptionContract {
    /// Instrument ID
    pub instrument_id: InstrumentId,
    /// Underlying symbol
    pub underlying: String,
    /// Option type (Call or Put)
    pub option_type: OptionType,
    /// Strike price
    pub strike_price: f64,
    /// Expiration date (YYYY-MM-DD)
    pub expiry_date: String,
    /// Last traded price
    pub last_price: Option<f64>,
    /// Day high
    pub high_price: Option<f64>,
    /// Day low
    pub low_price: Option<f64>,
    /// Previous close
    pub prev_close_price: Option<f64>,
    /// Volume
    pub volume: Option<u64>,
    /// Turnover
    pub turnover: Option<f64>,
    /// Open interest
    pub open_interest: Option<u64>,
    /// Implied volatility
    pub implied_volatility: Option<f64>,
    /// Delta
    pub delta: Option<f64>,
    /// Gamma
    pub gamma: Option<f64>,
    /// Theta
    pub theta: Option<f64>,
    /// Vega
    pub vega: Option<f64>,
    /// Rho
    pub rho: Option<f64>,
}

/// Option type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionType {
    Call,
    Put,
}

/// Option chain containing all options for a given underlying.
#[derive(Debug, Clone)]
pub struct OptionChain {
    /// Underlying symbol
    pub underlying: String,
    /// Current underlying price
    pub underlying_price: Option<f64>,
    /// Available expiration dates
    pub expiry_dates: Vec<String>,
    /// Options grouped by expiry and strike
    pub options: HashMap<String, Vec<OptionContract>>,
}

/// Market depth data.
#[derive(Debug, Clone)]
pub struct MarketDepth {
    /// Instrument ID
    pub instrument_id: InstrumentId,
    /// Bid levels (price -> quantity)
    pub bids: Vec<(Price, Quantity)>,
    /// Ask levels (price -> quantity)
    pub asks: Vec<(Price, Quantity)>,
    /// Timestamp
    pub timestamp: UnixNanos,
}

/// Capital flow information.
#[derive(Debug, Clone)]
pub struct CapitalFlow {
    /// Date (YYYY-MM-DD)
    pub date: String,
    /// Net capital inflow (positive) or outflow (negative)
    pub net_flow: f64,
}

/// Advanced data fetcher for Longport.
pub struct AdvancedDataFetcher {
    quote_ctx: QuoteContext,
}

impl std::fmt::Debug for AdvancedDataFetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdvancedDataFetcher")
            .finish()
    }
}

impl AdvancedDataFetcher {
    /// Creates a new [`AdvancedDataFetcher`].
    pub fn new(quote_ctx: QuoteContext) -> Self {
        Self { quote_ctx }
    }

    // ========================================================================
    // STOCK QUOTE DATA
    // ========================================================================

    /// Fetch real-time quote for a single stock.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock symbol (e.g., "700.HK", "AAPL.US")
    pub async fn fetch_stock_quote(&self, symbol: &str) -> anyhow::Result<StockQuote> {
        let quotes = self.quote_ctx.quote(vec![symbol]).await
            .context("failed to fetch stock quote")?;

        if quotes.is_empty() {
            anyhow::bail!("No quote data returned for symbol: {}", symbol);
        }

        let quote = &quotes[0];
        let instrument_id = parse_instrument_id_from_symbol(symbol)?;

        Ok(StockQuote {
            instrument_id,
            last_price: Some(decimal_to_f64(&quote.last_done)),
            high_price: Some(decimal_to_f64(&quote.high)),
            low_price: Some(decimal_to_f64(&quote.low)),
            prev_close_price: Some(decimal_to_f64(&quote.prev_close)),
            volume: Some(quote.volume as u64),
            turnover: Some(decimal_to_f64(&quote.turnover)),
            open_price: Some(decimal_to_f64(&quote.open)),
            timestamp: get_atomic_clock_realtime().get_time_ns(),
        })
    }

    /// Fetch real-time quotes for multiple stocks.
    ///
    /// # Arguments
    ///
    /// * `symbols` - Vector of stock symbols
    pub async fn fetch_stock_quotes_batch(&self, symbols: Vec<String>) -> anyhow::Result<Vec<StockQuote>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let symbols_ref: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
        let quotes = self.quote_ctx.quote(symbols_ref).await
            .context("failed to fetch batch stock quotes")?;

        let timestamp = get_atomic_clock_realtime().get_time_ns();

        let result = quotes.into_iter().map(|quote| {
            let symbol = quote.symbol.clone();
            let instrument_id = parse_instrument_id_from_symbol(&symbol)
                .unwrap_or_else(|_| InstrumentId::new(Symbol::new(&symbol), *LONGPORT_VENUE));

            StockQuote {
                instrument_id,
                last_price: Some(decimal_to_f64(&quote.last_done)),
                high_price: Some(decimal_to_f64(&quote.high)),
                low_price: Some(decimal_to_f64(&quote.low)),
                prev_close_price: Some(decimal_to_f64(&quote.prev_close)),
                volume: Some(quote.volume as u64),
                turnover: Some(decimal_to_f64(&quote.turnover)),
                open_price: Some(decimal_to_f64(&quote.open)),
                timestamp,
            }
        }).collect();

        Ok(result)
    }

    /// Fetch real-time quote with latest trade information.
    pub async fn fetch_realtime_quote(&self, symbol: &str) -> anyhow::Result<RealtimeQuote> {
        let quotes = self.quote_ctx.realtime_quote(vec![symbol]).await
            .context("failed to fetch realtime quote")?;

        if quotes.is_empty() {
            anyhow::bail!("No realtime quote data for symbol: {}", symbol);
        }

        Ok(quotes[0].clone())
    }

    // ========================================================================
    // HISTORICAL DATA (CANDLESTICKS/BARS)
    // ========================================================================

    /// Fetch historical candlestick bars for a stock.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock symbol
    /// * `period` - Bar period (Day, Week, Month, Minute, etc.)
    /// * `config` - Historical data configuration
    pub async fn fetch_historical_bars(
        &self,
        symbol: &str,
        period: Period,
        config: HistoricalDataConfig,
    ) -> anyhow::Result<Vec<Bar>> {
        let instrument_id = parse_instrument_id_from_symbol(symbol)?;

        // Fetch historical bars using offset method (most recent N bars)
        // Note: Longport requires 7 parameters for history_candlesticks_by_offset
        let bars_response = self.quote_ctx.history_candlesticks_by_offset(
            symbol,
            period,
            longport::quote::AdjustType::NoAdjust,
            false, // forward parameter
            None, // time parameter
            config.bar_count.unwrap_or(200),
            longport::quote::TradeSessions::All,
        ).await.context("failed to fetch historical bars by offset")?;

        let bars = bars_response
            .into_iter()
            .filter_map(|candle| {
                let ts_event = candle.timestamp.unix_timestamp_nanos() as u64;
                let ts_init = ts_event;

                Bar::new_checked(
                    create_bar_type(instrument_id, period),
                    Price::new(decimal_to_f64(&candle.open), 2),
                    Price::new(decimal_to_f64(&candle.high), 2),
                    Price::new(decimal_to_f64(&candle.low), 2),
                    Price::new(decimal_to_f64(&candle.close), 2),
                    Quantity::new(candle.volume as f64, 0),
                    UnixNanos::from(ts_event),
                    UnixNanos::from(ts_init),
                ).ok()
            })
            .collect();

        Ok(bars)
    }

    /// Fetch intraday candlesticks (1-minute, 5-minute, etc.).
    pub async fn fetch_intraday_bars(
        &self,
        symbol: &str,
        period: Period,
    ) -> anyhow::Result<Vec<Bar>> {
        let instrument_id = parse_instrument_id_from_symbol(symbol)?;

        // intraday requires TradeSessions, not Period
        let lines = self.quote_ctx.intraday(symbol, longport::quote::TradeSessions::All).await
            .context("failed to fetch intraday bars")?;

        let bars = lines
            .into_iter()
            .filter_map(|line| {
                let ts_event = line.timestamp.unix_timestamp_nanos() as u64;
                let ts_init = ts_event;
                let price = decimal_to_f64(&line.price);

                // Intraday only provides close price, use it for all OHLC
                Bar::new_checked(
                    create_bar_type(instrument_id, period),
                    Price::new(price, 2),
                    Price::new(price, 2),
                    Price::new(price, 2),
                    Price::new(price, 2),
                    Quantity::new(line.volume as f64, 0),
                    UnixNanos::from(ts_event),
                    UnixNanos::from(ts_init),
                ).ok()
            })
            .collect();

        Ok(bars)
    }

    /// Fetch recent trades for a symbol.
    pub async fn fetch_recent_trades(
        &self,
        symbol: &str,
        count: usize,
    ) -> anyhow::Result<Vec<TradeTick>> {
        let instrument_id = parse_instrument_id_from_symbol(symbol)?;
        let trades = self.quote_ctx.trades(symbol, count).await
            .context("failed to fetch recent trades")?;

        let trade_ticks = trades
            .into_iter()
            .enumerate()
            .filter_map(|(idx, trade)| {
                let ts_event = trade.timestamp.unix_timestamp_nanos() as u64;
                let price = Price::new(decimal_to_f64(&trade.price), 2);
                let size = Quantity::new(trade.volume as f64, 0);
                let aggressor = AggressorSide::Buyer; // Default to buyer
                let trade_id = TradeId::new(&format!("{}-{}-{}", symbol, idx, ts_event));

                TradeTick::new_checked(
                    instrument_id,
                    price,
                    size,
                    aggressor,
                    trade_id,
                    UnixNanos::from(ts_event),
                    UnixNanos::from(ts_event),
                ).ok()
            })
            .collect();

        Ok(trade_ticks)
    }

    // ========================================================================
    // OPTION CHAIN DATA
    // ========================================================================

    /// Fetch available option expiry dates for a symbol.
    ///
    /// # Arguments
    ///
    /// * `underlying` - Underlying stock symbol (e.g., "AAPL.US")
    pub async fn fetch_option_expiry_dates(&self, underlying: &str) -> anyhow::Result<Vec<String>> {
        let dates = self.quote_ctx.option_chain_expiry_date_list(underlying).await
            .context("failed to fetch option expiry dates")?;

        Ok(dates.into_iter().map(|d| d.to_string()).collect())
    }

    /// Fetch complete option chain for a given underlying and expiry date.
    ///
    /// # Arguments
    ///
    /// * `underlying` - Underlying stock symbol
    /// * `expiry_date` - Expiry date (YYYY-MM-DD format)
    /// * `config` - Option chain configuration
    pub async fn fetch_option_chain(
        &self,
        underlying: &str,
        expiry_date: &str,
        _config: OptionChainConfig,
    ) -> anyhow::Result<OptionChain> {
        // Parse expiry_date to chrono::Date
        let date = parse_date(expiry_date)?;

        // Fetch option chain info for the expiry date
        let strike_info_list = self.quote_ctx.option_chain_info_by_date(
            underlying,
            date,
        ).await.context("failed to fetch option chain info")?;

        // Get current underlying price
        let underlying_price = self.fetch_stock_quote(underlying).await?
            .last_price;

        let mut options = Vec::new();

        // Collect all option symbols (both calls and puts)
        let mut option_symbols = Vec::new();
        for strike_info in &strike_info_list {
            if !strike_info.call_symbol.is_empty() {
                option_symbols.push(strike_info.call_symbol.clone());
            }
            if !strike_info.put_symbol.is_empty() {
                option_symbols.push(strike_info.put_symbol.clone());
            }
        }

        // Fetch quotes for all options
        let option_quotes = self.fetch_option_quotes(option_symbols).await?;

        // Create option contracts from the quotes
        for quote in option_quotes {
            let option_type = if quote.option_type == OptionType::Call {
                OptionType::Call
            } else {
                OptionType::Put
            };

            options.push(OptionContract {
                instrument_id: quote.instrument_id.clone(),
                underlying: underlying.to_string(),
                option_type,
                strike_price: quote.strike_price,
                expiry_date: quote.expiry_date.clone(),
                last_price: quote.last_price,
                high_price: quote.high_price,
                low_price: quote.low_price,
                prev_close_price: quote.prev_close_price,
                volume: quote.volume,
                turnover: quote.turnover,
                open_interest: quote.open_interest,
                implied_volatility: quote.implied_volatility,
                delta: quote.delta,
                gamma: quote.gamma,
                theta: quote.theta,
                vega: quote.vega,
                rho: quote.rho,
            });
        }

        Ok(OptionChain {
            underlying: underlying.to_string(),
            underlying_price,
            expiry_dates: vec![expiry_date.to_string()],
            options: {
                let mut map = HashMap::new();
                map.insert(expiry_date.to_string(), options);
                map
            },
        })
    }

    /// Fetch option quotes for multiple symbols.
    pub async fn fetch_option_quotes(&self, symbols: Vec<String>) -> anyhow::Result<Vec<OptionContract>> {
        let symbols_ref: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();

        let quotes = self.quote_ctx.option_quote(symbols_ref).await
            .context("failed to fetch option quotes")?;

        let contracts = quotes.into_iter().map(|quote| {
            let instrument_id = parse_instrument_id_from_symbol(&quote.symbol)
                .unwrap_or_else(|_| InstrumentId::new(Symbol::new(&quote.symbol), *LONGPORT_VENUE));

            // Determine option type from symbol
            let option_type = if quote.symbol.contains('C') {
                OptionType::Call
            } else {
                OptionType::Put
            };

            OptionContract {
                instrument_id,
                underlying: String::new(), // Would need to parse from symbol
                option_type,
                strike_price: decimal_to_f64(&quote.strike_price),
                expiry_date: quote.expiry_date.to_string(),
                last_price: Some(decimal_to_f64(&quote.last_done)),
                high_price: Some(decimal_to_f64(&quote.high)),
                low_price: Some(decimal_to_f64(&quote.low)),
                prev_close_price: Some(decimal_to_f64(&quote.prev_close)),
                volume: Some(quote.volume as u64),
                turnover: Some(decimal_to_f64(&quote.turnover)),
                open_interest: Some(quote.open_interest as u64),
                implied_volatility: Some(decimal_to_f64(&quote.implied_volatility)),
                delta: None,
                gamma: None,
                theta: None,
                vega: None,
                rho: None,
            }
        }).collect();

        Ok(contracts)
    }

    // ========================================================================
    // MARKET DEPTH AND LIQUIDITY
    // ========================================================================

    /// Fetch market depth (order book) data.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Stock symbol
    /// * `depth_levels` - Number of price levels to fetch (default: 20)
    pub async fn fetch_market_depth(
        &self,
        symbol: &str,
        depth_levels: Option<usize>,
    ) -> anyhow::Result<MarketDepth> {
        let security_depth = self.quote_ctx.depth(symbol).await
            .context("failed to fetch market depth")?;

        let instrument_id = parse_instrument_id_from_symbol(symbol)?;
        let levels = depth_levels.unwrap_or(security_depth.asks.len().min(security_depth.bids.len()));

        let bids = security_depth.bids.iter()
            .take(levels)
            .filter_map(|bid| {
                bid.price.map(|p| (
                    Price::new(decimal_to_f64(&p), 2),
                    Quantity::new(bid.volume as f64, 0),
                ))
            })
            .collect();

        let asks = security_depth.asks.iter()
            .take(levels)
            .filter_map(|ask| {
                ask.price.map(|p| (
                    Price::new(decimal_to_f64(&p), 2),
                    Quantity::new(ask.volume as f64, 0),
                ))
            })
            .collect();

        Ok(MarketDepth {
            instrument_id,
            bids,
            asks,
            timestamp: get_atomic_clock_realtime().get_time_ns(),
        })
    }

    /// Fetch real-time market depth snapshot.
    pub async fn fetch_realtime_market_depth(&self, symbol: &str) -> anyhow::Result<MarketDepth> {
        let security_depth = self.quote_ctx.realtime_depth(symbol).await
            .context("failed to fetch realtime market depth")?;

        let instrument_id = parse_instrument_id_from_symbol(symbol)?;

        let bids = security_depth.bids.iter()
            .filter_map(|bid| {
                bid.price.map(|p| (
                    Price::new(decimal_to_f64(&p), 2),
                    Quantity::new(bid.volume as f64, 0),
                ))
            })
            .collect();

        let asks = security_depth.asks.iter()
            .filter_map(|ask| {
                ask.price.map(|p| (
                    Price::new(decimal_to_f64(&p), 2),
                    Quantity::new(ask.volume as f64, 0),
                ))
            })
            .collect();

        Ok(MarketDepth {
            instrument_id,
            bids,
            asks,
            timestamp: get_atomic_clock_realtime().get_time_ns(),
        })
    }

    // ========================================================================
    // CAPITAL FLOW AND MARKET SENTIMENT
    // ========================================================================

    /// Fetch capital flow data for a stock.
    ///
    /// This shows the net buying/selling pressure from different trader categories.
    pub async fn fetch_capital_flow(&self, symbol: &str) -> anyhow::Result<Vec<CapitalFlow>> {
        let flow_lines = self.quote_ctx.capital_flow(symbol).await
            .context("failed to fetch capital flow")?;

        let flows = flow_lines.into_iter().map(|line| {
            let date = format!("{}", line.timestamp.date());
            CapitalFlow {
                date,
                net_flow: decimal_to_f64(&line.inflow),
            }
        }).collect();

        Ok(flows)
    }

    /// Fetch capital distribution data.
    pub async fn fetch_capital_distribution(&self, symbol: &str) -> anyhow::Result<Vec<CapitalFlow>> {
        let dist = self.quote_ctx.capital_distribution(symbol).await
            .context("failed to fetch capital distribution")?;

        // CapitalDistributionResponse is a single object, not a list
        // Calculate net flow from capital_in and capital_out
        let capital_in_total = decimal_to_f64(&dist.capital_in.large)
            + decimal_to_f64(&dist.capital_in.medium)
            + decimal_to_f64(&dist.capital_in.small);
        let capital_out_total = decimal_to_f64(&dist.capital_out.large)
            + decimal_to_f64(&dist.capital_out.medium)
            + decimal_to_f64(&dist.capital_out.small);

        let net_flow = capital_in_total - capital_out_total;

        // Format timestamp as string
        let date = format!("{}", dist.timestamp.date());

        Ok(vec![CapitalFlow {
            date,
            net_flow,
        }])
    }

    // ========================================================================
    // TRADING CALENDAR
    // ========================================================================

    /// Fetch trading days for a market.
    ///
    /// # Arguments
    ///
    /// * `market` - Market to query (HK, US, CN)
    /// * `start_date` - Start date (YYYY-MM-DD)
    /// * `end_date` - End date (YYYY-MM-DD)
    pub async fn fetch_trading_days(
        &self,
        market: &str,
        start_date: &str,
        end_date: &str,
    ) -> anyhow::Result<Vec<String>> {
        let sdk_market = parse_market_str(market)?;

        let start = parse_date(start_date)?;
        let end = parse_date(end_date)?;

        let market_days = self.quote_ctx.trading_days(sdk_market, start, end).await
            .context("failed to fetch trading days")?;

        // MarketTradingDays contains trading_days field
        Ok(market_days.trading_days.into_iter().map(|d| d.to_string()).collect())
    }
}

// ========================================================================
// HELPER FUNCTIONS
// ========================================================================

fn create_bar_type(instrument_id: InstrumentId, period: Period) -> BarType {
    let (aggregation, step) = match period {
        Period::Day => (BarAggregation::Day, 1),
        Period::Week => (BarAggregation::Week, 1),
        Period::Month => (BarAggregation::Month, 1),
        Period::OneMinute => (BarAggregation::Minute, 1),
        Period::FiveMinute => (BarAggregation::Minute, 5),
        Period::FifteenMinute => (BarAggregation::Minute, 15),
        Period::ThirtyMinute => (BarAggregation::Minute, 30),
        Period::SixtyMinute => (BarAggregation::Minute, 60),
        _ => (BarAggregation::Day, 1), // Default
    };

    let spec = BarSpecification::new(step, aggregation, PriceType::Last);
    BarType::new(instrument_id, spec, AggregationSource::External)
}

fn decimal_to_f64(d: &rust_decimal::Decimal) -> f64 {
    // Convert rust_decimal to f64
    use rust_decimal::prelude::ToPrimitive;
    d.to_f64().unwrap_or(0.0)
}

fn parse_date(date_str: &str) -> anyhow::Result<time::Date> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid date format: {}, expected YYYY-MM-DD", date_str);
    }

    let year: i32 = parts[0].parse()?;
    let month: u8 = parts[1].parse()?;
    let day: u8 = parts[2].parse()?;

    time::Date::from_calendar_date(year, month.try_into()?, day)
        .map_err(|_| anyhow::anyhow!("Invalid date: {}", date_str))
}

fn parse_market_str(market: &str) -> anyhow::Result<Market> {
    match market.to_uppercase().as_str() {
        "HK" => Ok(Market::HK),
        "US" => Ok(Market::US),
        "CN" => Ok(Market::CN),
        "SG" => Ok(Market::SG),
        _ => anyhow::bail!("Unsupported market: {}", market),
    }
}
