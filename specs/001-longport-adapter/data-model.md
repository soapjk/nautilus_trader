# Data Model: LongPort Adapter

**Feature**: LongPort Adapter API Support
**Date**: 2026-01-03
**Status**: Phase 1 Output

## Overview

This document defines the domain entities and data structures for the LongPort adapter, mapping between LongPort SDK types and NautilusTrader domain models.

## Core Entities

### 1. LongportInstrument

**Description**: Represents a tradable security on LongPort (stock, ETF, warrant, etc.)

**Attributes**:
- `id: InstrumentId` - Unique instrument identifier (symbol.venue, e.g., "700.HK.LONGPORT")
- `raw_symbol: Ustr` - Original symbol from LongPort (e.g., "700.HK")
- `name: String` - Security name (e.g., "Tencent Holdings Ltd")
- `security_type: LongportSecurityType` - Stock, ETF, Warrant, CBBC, etc.
- `market: LongportMarket` - HK (Hong Kong), US (United States), CN (A-shares)
- `lot_size: Quantity` - Minimum trading unit (e.g., 100 for HK stocks)
- `tick_size: Price` - Minimum price increment
- `price_precision: u8` - Decimal places for price
- `quantity_precision: u8` - Decimal places for quantity
- `base_currency: Currency` - Settlement currency (HKD, USD, CNY)
- `is_tradable: bool` - Whether instrument can be traded
- `listing_date: Option<UnixTimestamp>` - When instrument started trading

**Validation Rules**:
- lot_size > 0
- tick_size > 0
- price_precision <= 8
- quantity_precision <= 8
- symbol must contain valid market suffix (.HK, .US, .SH, .SZ)

**State Transitions**: N/A (immutable after creation)

### 2. LongportOrder

**Description**: Represents an order submitted to LongPort

**Attributes**:
- `client_order_id: ClientOrderId` - Nautilus client-side order ID
- `order_id: Option<OrderId>` - LongPort-assigned order ID (populated after submission)
- `venue_order_id: Option<VenueOrderId>` - LongPort's internal order ID
- `symbol: InstrumentId` - Instrument being traded
- `side: OrderSide` - Buy or Sell
- `order_type: LongportOrderType` - LO (Limit), MO (Market), ELO (Enhanced Limit)
- `time_in_force: TimeInForce` - GTC, IOC, FOK, DAY
- `quantity: Quantity` - Order size in lots
- `price: Option<Price>` - Limit price (None for market orders)
- `status: OrderStatus` - Initialized, Submitted, Accepted, Filled, Canceled, Rejected
- `filled_quantity: Quantity` - Total quantity filled so far
- `avg_px: Option<Price>` - Average fill price
- `submit_time: UnixTimestamp` - When order was created
- `update_time: UnixTimestamp` - Last update timestamp
- `error: Option<String>` - Error message if rejected

**Validation Rules**:
- quantity > 0
- price > 0 if order_type is LO or ELO
- client_order_id must be unique
- symbol must be valid instrument

**State Transitions**:
```
Initialized → Submitted → Accepted → [Filled | PartiallyFilled] → Filled
                ↓
              Rejected
                ↓
              Expired
                ↓
              Canceled
```

### 3. LongportQuote

**Description**: Real-time quote (bid/ask) from LongPort

**Attributes**:
- `instrument_id: InstrumentId` - Instrument symbol
- `bid_price: Price` - Best bid price
- `ask_price: Price` - Best ask price
- `bid_size: Quantity` - Size at best bid
- `ask_size: Quantity` - Size at best ask
- `bid_exchange_id: Option<String>` - Exchange providing bid
- `ask_exchange_id: Option<String>` - Exchange providing ask
- `ts_event: UnixTimestampNanos` - Quote generation time
- `ts_init: UnixTimestampNanos` - Quote receipt time

**Validation Rules**:
- bid_price > 0, ask_price > 0
- bid_size > 0, ask_size > 0
- ask_price >= bid_price (no inverted quotes)
- ts_event <= ts_init

**State Transitions**: N/A (immutable snapshots)

### 4. LongportTradeTick

**Description**: Individual trade execution from LongPort

**Attributes**:
- `instrument_id: InstrumentId` - Instrument symbol
- `price: Price` - Trade price
- `size: Quantity` - Trade size
- `side: Option<AggressorSide>` - Aggressor side (Buy/Sell)
- `trade_id: TradeId` - Unique trade identifier
- `ts_event: UnixTimestampNanos` - Trade execution time
- `ts_init: UnixTimestampNanos` - Trade receipt time

**Validation Rules**:
- price > 0
- size > 0
- ts_event <= ts_init
- trade_id must be unique per instrument

**State Transitions**: N/A (immutable events)

### 5. LongportOrderBook

**Description**: Order book snapshot or delta

**Attributes**:
- `instrument_id: InstrumentId` - Instrument symbol
- `bids: Vec<(Price, Quantity)>` - Bid levels (price, size)
- `asks: Vec<(Price, Quantity)>` - Ask levels (price, size)
- `book_type: OrderBookType` - Snapshot or Delta
- `sequence: Option<u64>` - Book update sequence number
- `ts_event: UnixTimestampNanos` - Book generation time
- `ts_init: UnixTimestampNanos` - Book receipt time

**Validation Rules**:
- Bids sorted descending (best bid first)
- Asks sorted ascending (best ask first)
- All bids > 0, all asks > 0
- Best bid < best ask (no crossed book)
- ts_event <= ts_init

**State Transitions**:
- Snapshots replace entire book
- Deltas apply incrementally (add/update/remove levels)

## Enums

### LongportSecurityType

```rust
pub enum LongportSecurityType {
    Stock,          // 普通股
    ETF,            // 交易所交易基金
    Warrant,        // 窝轮
    CBBC,           // 牛熊证
    Bond,           // 债券
    Index,          // 指数
    Option,         // 期权
    Future,         // 期货
    Trust,          // 信托
    Right,          // 权证
}
```

### LongportMarket

```rust
pub enum LongportMarket {
    HK,   // Hong Kong (.HK suffix)
    US,   // United States (.US suffix)
    SH,   // Shanghai A-shares (.SH suffix)
    SZ,   // Shenzhen A-shares (.SZ suffix)
    HK_Futures,  // Hong Kong futures (.HSI suffix)
    US_Options,   // US options (.O suffix)
}
```

### LongportOrderType

```rust
pub enum LongportOrderType {
    LO,   // Limit Order (限价)
    MO,   // Market Order (市价)
    ELO,  // Enhanced Limit Order (增强限价)
    AO,   // At-Auction Order (竞价)
    OTO,  // On-Touch Order (触及)
    OCO,  // On-Touch or Kill (触及或取消)
}
```

### LongportSide

```rust
pub enum LongportSide {
    Buy,   // 买入
    Sell,  // 卖出
}
```

## Type Conversions

### Longport → Nautilus Mapping

| Longport Type | Nautilus Type | Conversion Logic |
|--------------|---------------|------------------|
| `LongportSecurityType::Stock` | `AssetClass::Equity` | Direct mapping |
| `LongportSecurityType::ETF` | `AssetClass::Equity` | ETF treated as equity |
| `LongportSecurityType::Option` | `InstrumentKind::Option` | With option-specific attrs |
| `LongportSide::Buy` | `OrderSide::BUY` | Direct mapping |
| `LongportSide::Sell` | `OrderSide::SELL` | Direct mapping |
| `LongportOrderType::LO` | `OrderType::LIMIT` | Direct mapping |
| `LongportOrderType::MO` | `OrderType::MARKET` | Direct mapping |
| `LongportOrderType::ELO` | `OrderType::LIMIT` | Enhanced treated as limit |
| Unix timestamp (ms) | `UnixTimestampNanos` | Multiply by 1_000_000 |
| Price string | `Price` | Parse with precision |
| Quantity string | `Quantity` | Parse with lot_size |

### Nautilus → Longport Mapping

| Nautilus Type | Longport Type | Conversion Logic |
|--------------|---------------|------------------|
| `OrderSide::BUY` | `LongportSide::Buy` | Direct mapping |
| `OrderSide::SELL` | `LongportSide::Sell` | Direct mapping |
| `OrderType::LIMIT` | `LongportOrderType::LO` | Direct mapping |
| `OrderType::MARKET` | `LongportOrderType::MO` | Direct mapping |
| `TimeInForce::GTC` | `Day` (LongPort) | GTC maps to Day session |
| `TimeInForce::IOC` | `AllOrNone` | IOC with immediate fill/kill |
| `UnixTimestampNanos` | Unix timestamp (ms) | Divide by 1_000_000 |

## Collections

### Instrument Cache

**Structure**: `Arc<DashMap<InstrumentId, InstrumentAny>>`

**Purpose**: Thread-safe cache of loaded instruments

**Operations**:
- `cache_instruments(instruments: Vec<InstrumentAny>)` - Bulk replace
- `cache_instrument(instrument: InstrumentAny)` - Upsert single
- `get_instrument(symbol: &str) -> Option<InstrumentAny>` - Lookup by symbol

**Lifecycle**:
- Load: `load_all_async()` via SDK
- Refresh: Every 60 minutes (configurable)
- Invalidation: On WebSocket disconnect/reconnect

### Subscription State

**Structure**: `Arc<SubscriptionState>` (from nautilus_network)

**Purpose**: Track WebSocket subscription status across client/handler

**Operations**:
- `mark_subscribe(topic: String)` - Mark as pending
- `confirm(topic: String)` - Mark as confirmed
- `mark_failure(topic: String)` - Mark as failed (retry)
- `mark_unsubscribe(topic: String)` - Mark for removal
- `subscription_count() -> usize` - Count confirmed subscriptions

**States**: Pending → Confirmed (or Failed for retry)

## Error Types

### LongportHttpError

```rust
pub enum LongportHttpError {
    MissingCredentials,
    LongportError { code: String, message: String },
    JsonError(String),
    ValidationError(String),
    NetworkError(String),
    RateLimited,
    Unauthorized(String),
}
```

### LongportWsError

```rust
pub enum LongportWsError {
    ConnectionError(String),
    AuthenticationError(String),
    ParseError(String),
    RateLimited,
    InvalidMessage(String),
    SubscriptionFailed(String),
}
```

## Data Flow

### Market Data Flow

```
LongPort SDK (WebSocket)
    → QuoteContext callback
    → LongportWsFeedHandler
    → parse_quote_tick() / parse_trade_tick()
    → NautilusWsMessage::Data(...)
    → out_tx channel
    → LongportWebSocketClient
    → publish to MessageBus
    → Strategy receives QuoteTick/TradeTick
```

### Order Execution Flow

```
Strategy
    → SubmitOrder command
    → LongportExecutionClient
    → convert to Longport order format
    → TradeContext.submit_order()
    → Longport SDK → LongPort API
    → OrderAccepted event (SDK callback)
    → LongportExecutionClient
    → publish to MessageBus
    → Strategy receives OrderAccepted
```

## Validation Summary

### Pre-Submission Validation (Orders)

1. **Instrument validation**: Symbol exists in cache, is_tradable = true
2. **Quantity validation**: quantity % lot_size == 0
3. **Price validation**: price % tick_size == 0 (for limit orders)
4. **Balance validation**: Sufficient funds/holdings (via account state)
5. **Market hours**: Instrument is in trading session

### Post-Execution Reconciliation

1. **Order status**: Confirm LongPort order status matches Nautilus state
2. **Fill reconciliation**: Match fills with LongPort execution reports
3. **Position reconciliation**: Confirm positions match LongPort
4. **Balance reconciliation**: Confirm account balances match

## Next Steps

With data model defined:
1. Implement parser functions in `common/parse.rs`
2. Define PyO3 bindings for types
3. Create Python config classes
4. Implement instrument provider
