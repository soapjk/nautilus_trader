# Research: LongPort OpenSDK Integration

**Feature**: LongPort Adapter API Support
**Date**: 2026-01-03
**Status**: Complete

## Overview

This document consolidates research findings on LongPort OpenSDK, OKX adapter architecture, and best practices for implementing the LongPort adapter following NautilusTrader patterns.

## LongPort OpenSDK Analysis

### Decision: Use Official LongPort OpenSDK

**Rationale**:
- SDK is Rust-based with Python bindings (aligns with NautilusTrader architecture)
- Provides QuoteContext and TradeContext abstractions (simplifies integration)
- Handles authentication, rate limiting, reconnection internally
- Officially maintained by LongPort (reduces maintenance burden)
- Already battle-tested in production

**Alternatives Considered**:
1. **Direct HTTP/WebSocket**: Rejected - would require re-implementing auth, rate limiting, reconnection
2. **Third-party SDKs**: Rejected - lack official support, may have security/stability issues

### LongPort SDK Unique Characteristics

Unlike OKX/BitMEX (pure REST/WebSocket), LongPort SDK provides:

1. **High-Level Contexts**:
   - `QuoteContext`: Market data (quotes, trades, candlesticks, order books)
   - `TradeContext`: Order management (submit, cancel, query orders)
   - Config-based authentication (app_key, app_secret, access_token)

2. **Rust-Based with Python Bindings**:
   - Core SDK implemented in Rust for performance
   - Python bindings via PyO3 (matches NautilusTrader pattern)
   - No custom request signing needed (SDK handles internally)

3. **Authentication**:
   - Three tokens: app_key, app_secret, access_token
   - Loaded via Config object (from_env() or explicit parameters)
   - No HMAC signing (unlike OKX which requires custom signatures)

4. **WebSocket Architecture**:
   - QuoteContext manages WebSocket connections internally
   - Callback-based push data (on_quote event handlers)
   - Automatic reconnection and subscription restoration

### API Endpoints

**REST API** (via SDK HTTP layer):
- Market data: Historical bars, quotes, trades
- Instruments: Security information, tick size, lot size
- Account: Balances, positions, orders
- Trading: Submit/cancel/modify orders

**WebSocket** (via QuoteContext):
- Real-time quotes (bid/ask)
- Trade ticks
- Candlesticks (OHLCV)
- Order book snapshots and deltas

**Configuration URLs**:
- HTTP API: `https://open.longport.com` (production)
- WebSocket: `wss://open.longport.com` (production)
- Testnet: Different URLs (documented in SDK)

## OKX Adapter Architecture Analysis

### Two-Layer HTTP Client Pattern

**Raw Client** (`OKXRawHttpClient`):
- Handles low-level HTTP operations
- Request signing with HMAC-SHA256
- Retry logic and error handling
- Direct mapping to OKX API endpoints

**Domain Client** (`OKXHttpClient`):
- Wraps raw client with Arc
- High-level methods accepting Nautilus types
- Instrument caching
- Ergonomic API for Python bindings

**Adaptation for LongPort**:
- Replace custom signing with SDK Config
- Wrap SDK's HTTP methods instead of raw HTTP
- Maintain same two-layer structure for consistency

### Dual-Tier WebSocket Architecture

**Client Tier** (`OKXWebSocketClient`):
- Orchestrates connection lifecycle
- Manages subscription state
- Handles reconnection logic
- Emits commands to handler

**Handler Tier** (`OKXWsFeedHandler`):
- I/O boundary in dedicated task
- Processes WebSocket messages
- Transforms to Nautilus domain events
- Manages pending request state

**Adaptation for LongPort**:
- Client wraps QuoteContext's WebSocket management
- Handler transforms SDK callback data to Nautilus events
- Maintain dual-tier for performance and separation of concerns

### Parser Functions

**Location**: `src/common/parse.rs` (cross-cutting) and `http/parse.rs`/`websocket/parse.rs` (specific)

**Pattern**:
```rust
// Convert SDK types to Nautilus domain models
pub fn parse_quote_tick(sdk_quote: &LongportQuote) -> QuoteTick { }
pub fn parse_trade_tick(sdk_trade: &LongportTrade) -> TradeTick { }
pub fn parse_instrument_any(sdk_instrument: &LongportSecurity) -> InstrumentAny { }
```

**Key Considerations**:
- Empty string handling (LongPort uses "" for null)
- Timestamp conversion (SDK uses Unix milliseconds → Nautilus nanoseconds)
- Decimal precision (SDK strings → Nautilus Quantity/Price)
- Enum mapping (LongportSide → OrderSide, etc.)

## Configuration Architecture

### OKX Config Pattern

```python
class OKXDataClientConfig(LiveDataClientConfig, frozen=True):
    api_key: str | None = None  # Falls back to env var
    api_secret: str | None = None
    api_passphrase: str | None = None
    base_url_http: str | None = None
    base_url_ws: str | None = None
```

### LongPort Adaptation (Constitution Compliance)

**Critical Difference**: Per constitution modification, LongPort config MUST:
- NOT read environment variables
- Require explicit URL parameters (http_url, wss_quote_url, wss_trade_url)
- Require explicit app_key, app_secret, access_token

```python
class LongportDataClientConfig(NautilusConfig, frozen=True):
    app_key: str  # REQUIRED, no default
    app_secret: str  # REQUIRED, no default
    access_token: str  # REQUIRED, no default
    http_url: str  # REQUIRED, no default
    wss_quote_url: str  # REQUIRED, no default
    wss_trade_url: str  # REQUIRED, no default
    markets: list[str] = ["HK", "US", "CN"]
    # NO environment variable fallbacks
```

## Instrument Loading Patterns

### OKX Approach

1. **HTTP Fetch**: Load from OKX REST API
2. **Filtering**: Apply instrument_types, contract_types, families
3. **Caching**: Store in memory and WebSocket clients
4. **Subscription**: Subscribe to appropriate channels

### LongPort Adaptation

1. **SDK Fetch**: Use QuoteContext.get_security_info() or similar
2. **Market Filtering**: Filter by HK/US/CN markets
3. **Caching**: Same pattern (DashMap in Rust, cache in Python)
4. **Instrument Attributes**: Parse price_precision, tick_size, lot_size from SDK

## Type Safety & PyO3 Patterns

### Critical Constitution Requirement

From constitution modification:
> "be carefully about calling rust func. make sure throgh rust support type to rust, not pass a class what is totoally define in python"

**Implementation Pattern**:
```python
# WRONG - passes Python-defined class
class MyOrder:
    pass
rust_client.submit_order(MyOrder())  # ❌ Will crash

# CORRECT - convert to Rust type first
order = RustOrder.from_python(my_order)  # ✅ Safe conversion
rust_client.submit_order(order)
```

**Type Conversion Functions**:
- `instrument_any_to_pyobject()`: Rust → Python
- `pyobject_to_instrument_any()`: Python → Rust
- NEVER use `.into_py_any()` directly

## Performance Optimizations

### Lock-Free Data Structures

- **Arc<DashMap>**: Thread-safe cache for concurrent access
- **AHashMap**: Single-threaded hot path (handler inner loop)
- **ArcSwap**: Lock-free connection state tracking

### String Interning

```rust
use ustr::Ustr;

let venue: Ustr = Ustr::from("LONGPORT");
let symbol: Ustr = Ustr::from("700.HK");
```

**Benefits**:
- Faster comparisons (pointer vs string compare)
- Reduced allocations
- Lower memory footprint

## Error Handling Strategy

### Two-Layer Error Types

**HTTP Errors**:
```rust
pub enum LongportHttpError {
    MissingCredentials,
    LongportError { error_code: String, message: String },
    JsonError(String),
    ValidationError(String),
}
```

**WebSocket Errors**:
```rust
pub enum LongportWsError {
    ConnectionError(String),
    AuthenticationError(String),
    ParseError(String),
    RateLimited,
}
```

### Retry Logic

- Use RetryManager from nautilus_network
- Exponential backoff for transient errors
- No retry for authentication failures (401)
- Max retries configurable (default: 3)

## Testing Strategy

### Test Data Sources

**Constitution Requirement**: "All test data must come from official API documentation or live API calls - never fabricate manually"

**Approach**:
1. Capture real LongPort API responses from documentation
2. Save to `test_data/` directory
3. Use in both unit and integration tests
4. Update when SDK changes

### Deterministic Tests

**Constitution Requirement**: "Use wait_until_async helper instead of arbitrary tokio::time::sleep()"

**Pattern**:
```rust
// WRONG - flaky, slow
tokio::time::sleep(Duration::from_secs(5)).await;
assert!(connected);

// CORRECT - deterministic, fast
wait_until_async(|| client.is_connected(), Duration::from_secs(5)).await?;
```

## Documentation Standards

### Rust Documentation

```rust
/// Parses a LongPort quote tick into a Nautilus `QuoteTick`.
///
/// # Arguments
///
/// * `quote` - The LongPort quote to parse.
///
/// # Returns
///
/// A `QuoteTick` with bid/ask prices and timestamps.
///
/// # Errors
///
/// Returns an error if price or timestamp parsing fails.
pub fn parse_quote_tick(quote: &LongportQuote) -> Result<QuoteTick> {
    // ...
}
```

### Python Documentation

```python
class LongportDataClient(LiveMarketDataClient):
    """Live data client for LongPort market data.

    This client wraps the Rust implementation and provides
    real-time quote, trade, and order book data from LongPort.

    Parameters
    ----------
    config : LongportDataClientConfig
        The client configuration with explicit URLs and credentials.
    """
```

## Security Considerations

### Credential Management

**Constitution Principle IX**: Secure API key storage using Ustr and zeroization

```rust
pub struct Credential {
    pub app_key: Ustr,
    pub app_secret: Box<[u8]>,  // Zeroized on drop
    pub access_token: Box<[u8]>,  // Zeroized on drop
}
```

### Authentication Token Refresh

- SDK handles refresh automatically
- No manual token management needed
- Config object manages token lifecycle

## Next Steps: Phase 1 Design

With research complete, proceed to Phase 1:
1. **data-model.md**: Define domain entities (instruments, orders, quotes)
2. **quickstart.md**: Create developer getting started guide
3. **Update agent context**: Add LongPort SDK to agent context

All technical unknowns resolved. Ready for design phase.
