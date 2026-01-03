# Feature Specification: LongPort Adapter API Support

**Feature Branch**: `001-longport-adapter`
**Created**: 2026-01-03
**Status**: Draft
**Input**: User description: " develop longport adapter api support,by learning okx and bitmex architecture. include implement rust websockets and http client, and python data client."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Connect to LongPort Market Data (Priority: P1)

Quantitative traders need to connect to LongPort's market data APIs to receive real-time quotes, trades, and order book updates for Hong Kong, US, and A-share markets. This enables trading strategies to make informed decisions based on current market conditions.

**Why this priority**: Market data connection is foundational - without it, no strategies can function. This is the minimum viable product that delivers immediate value.

**Independent Test**: Can be fully tested by connecting to LongPort test environment, subscribing to market data for a single instrument, and validating that quote ticks, trade ticks, and order book deltas are received and parsed correctly.

**Acceptance Scenarios**:

1. **Given** a configured LongPort data client with valid credentials, **When** the client connects to LongPort WebSocket APIs, **Then** the connection establishes successfully and authentication completes
2. **Given** an active data connection, **When** subscribing to quote ticks for an instrument (e.g., "700.HK"), **Then** real-time quote ticks are received and published to the message bus
3. **Given** an active data connection, **When** subscribing to trade ticks, **Then** real-time trade ticks are received with accurate price, size, and timestamp data
4. **Given** an active data connection, **When** subscribing to order book updates, **Then** incremental order book deltas are received and correctly applied to maintain an accurate order book state
5. **Given** a network disconnection, **When** the connection is lost, **Then** the client automatically reconnects and resubscribes to all previously subscribed data streams without data loss

---

### User Story 2 - Manage LongPort Instruments (Priority: P2)

Traders need to retrieve and manage instrument definitions from LongPort, including available securities, their attributes (tick size, lot size, price precision), and contract specifications. This ensures strategies have accurate metadata for order placement and risk management.

**Why this priority**: Instrument metadata is required for correct order sizing and validation. While P1 can use hardcoded instruments, P2 enables dynamic instrument loading necessary for production use.

**Independent Test**: Can be tested by loading all available instruments for a specific market (e.g., Hong Kong stocks), validating that instruments are correctly parsed with accurate attributes, and filtering instruments by market or symbol patterns.

**Acceptance Scenarios**:

1. **Given** a configured LongPort instrument provider, **When** loading all instruments for Hong Kong market, **Then** all available instruments are retrieved and cached with correct attributes
2. **Given** loaded instruments, **When** querying for a specific instrument by symbol (e.g., "700.HK"), **Then** the instrument is returned with accurate base currency, quote currency, price precision, and tick size
3. **Given** multiple markets configured, **When** loading instruments, **Then** instruments from each market are correctly tagged with their respective market identifiers
4. **Given** instrument loading in progress, **When** the API returns instrument data, **Then** each instrument is parsed into Nautilus Instrument type with correct AssetClass (Equity, Crypto, etc.)

---

### User Story 3 - Execute Orders on LongPort (Priority: P3)

Traders need to submit, modify, and cancel orders through LongPort's trading API. This enables automated strategy execution with real-time order status updates and fill confirmations.

**Why this priority**: Order execution enables fully automated trading but depends on P1 (market data) and P2 (instruments). It's the final piece for complete strategy automation.

**Independent Test**: Can be tested by submitting test orders to LongPort demo environment, validating order submission, modification, cancellation, and fill reporting workflows.

**Acceptance Scenarios**:

1. **Given** an authenticated execution client, **When** submitting a market buy order for a valid instrument, **Then** the order is accepted by LongPort and an OrderAccepted event is generated
2. **Given** a working order, **When** the order is partially or fully filled, **Then** OrderFilled events are generated with accurate fill price, quantity, and timestamp
3. **Given** a working order, **When** modifying the order quantity or price, **Then** the modification is accepted and OrderUpdated events reflect the changes
4. **Given** a working order, **When** canceling the order, **Then** the cancellation is confirmed and OrderCanceled events are generated
5. **Given** multiple orders active, **When** querying order status, **Then** all orders are reconciled with LongPort's order state

---

### Edge Cases

- What happens when LongPort API rate limits are exceeded during high-frequency data requests?
- How does the system handle invalid or expired authentication tokens?
- What happens when subscribing to an instrument that doesn't exist or has been delisted?
- How does the system handle malformed WebSocket messages from LongPort APIs?
- What happens when network connectivity is intermittent during order submission?
- How does the system handle partial instrument data or missing required fields?
- What happens when LongPort introduces breaking changes to their API schema?

## Requirements *(mandatory)*

### Functional Requirements

**Core Architecture**
- **FR-001**: System MUST implement a two-layer HTTP client architecture with `LongportRawHttpClient` (low-level API) and `LongportHttpClient` (domain-level with Arc wrapper)
- **FR-002**: System MUST implement a dual-tier WebSocket architecture with `LongportWebSocketClient` (orchestrator) and `LongportWsFeedHandler` (I/O boundary)
- **FR-003**: System MUST follow the 7-phase implementation sequence: Rust core → Instruments → Market data → Execution → Advanced features → Configuration → Testing
- **FR-004**: System MUST use Rust core for all performance-critical networking operations
- **FR-005**: System MUST provide Python API layer that wraps Rust implementation through PyO3 bindings

**HTTP Client**
- **FR-006**: System MUST implement HTTP client supporting LongPort REST API endpoints
- **FR-007**: System MUST handle request signing using LongPort-specific authentication methods (HMAC-SHA256 or token-based)
- **FR-008**: System MUST implement retry logic with exponential backoff for failed HTTP requests
- **FR-009**: System MUST respect LongPort API rate limits using `LazyLock<Quota>` static variables
- **FR-010**: System MUST support configurable HTTP base URLs (`http_url` parameter in config)
- **FR-011**: System MUST handle HTTP proxy configuration via `http_proxy_url` parameter

**WebSocket Client**
- **FR-012**: System MUST implement WebSocket client for real-time market data feeds
- **FR-013**: System MUST support configurable WebSocket URLs for both market data (`wss_quote_url`) and trade execution (`wss_trade_url`)
- **FR-014**: System MUST implement automatic reconnection with exponential backoff
- **FR-015**: System MUST track subscription state and restore subscriptions after reconnection
- **FR-016**: System MUST emit `RECONNECTED` sentinel message when connection is restored
- **FR-017**: System MUST handle WebSocket authentication token refresh automatically
- **FR-018**: System MUST parse WebSocket messages into Nautilus domain models (QuoteTick, TradeTick, OrderBookDeltas)

**Instrument Provider**
- **FR-019**: System MUST implement `InstrumentProvider` with `load_all_async`, `load_ids_async`, and `load_async` methods
- **FR-020**: System MUST parse LongPort instrument definitions into Nautilus `InstrumentAny` types
- **FR-021**: System MUST cache instruments using `cache_instruments()`, `cache_instrument()`, and `get_instrument()` methods
- **FR-022**: System MUST support filtering instruments by market (Hong Kong, US, A-shares)
- **FR-023**: System MUST handle instrument attribute parsing (price precision, tick size, lot size, margin requirements)

**Data Client**
- **FR-024**: System MUST implement `LongportDataClient` extending `LiveMarketDataClient`
- **FR-025**: System MUST support subscribing to quote ticks, trade ticks, and order book deltas
- **FR-026**: System MUST support historical data requests for bars, quote ticks, and trade ticks
- **FR-027**: System MUST publish all market data events to the Nautilus message bus
- **FR-028**: System MUST handle instrument lookups using cached instrument data
- **FR-029**: System MUST use `instrument_any_to_pyobject()` for Rust→Python instrument conversion

**Configuration**
- **FR-030**: System MUST provide `LongportDataClientConfig` class with all required parameters
- **FR-031**: System MUST provide `LongportExecClientConfig` class with all required parameters
- **FR-032**: Config classes MUST support explicit URL parameters: `http_url`, `wss_quote_url`, `wss_trade_url`
- **FR-033**: Config classes MUST NOT read environment variables directly - all parameters must be passed explicitly during initialization
- **FR-034**: Config classes MUST support optional parameters: `http_proxy_url`, `ws_proxy_url`, `http_timeout_secs`, `max_retries`
- **FR-035**: Config classes MUST use frozen dataclasses for immutability

**Type Safety & PyO3**
- **FR-036**: System MUST ensure all Python-defined types are converted to Rust-defined types before passing to Rust functions
- **FR-037**: System MUST use `pyobject_to_instrument_any()` for Python→Rust instrument conversion
- **FR-038**: System MUST use `#[pyclass]` and `#[pymethods]` for Python-exposed Rust types
- **FR-039**: System MUST prefix internal Rust methods with `py_` and expose clean names via `#[pyo3(name = "...")]`

**Error Handling**
- **FR-040**: System MUST provide clear, actionable error messages with recovery suggestions
- **FR-041**: System MUST implement proper error types for HTTP (`LongportHttpError`) and WebSocket (`LongportWsError`) failures
- **FR-042**: System MUST distinguish between retryable and non-retryable errors
- **FR-043**: System MUST log all errors at appropriate levels (DEBUG, INFO, WARNING, ERROR) with `use_pyo3=True`

**Performance**
- **FR-044**: System MUST achieve sub-100ms P99 latency for order execution operations
- **FR-045**: System MUST support 100+ orders per second throughput
- **FR-046**: System MUST use lock-free data structures (Arc<DashMap>, AHashMap) for concurrent access
- **FR-047**: System MUST use `ustr::Ustr` for string interning of repeated fields (symbols, venues, instrument IDs)

**Testing**
- **FR-048**: System MUST run test command `sh ./scripts/run_debug.sh` and check error messages for validation
- **FR-049**: System MUST include integration tests in `tests/` directory with mock servers
- **FR-050**: System MUST include Python integration tests in `tests/integration_tests/adapters/longport/`
- **FR-051**: System MUST use real test data from LongPort API or official documentation - never fabricate test data
- **FR-052**: System MUST use `wait_until_async` helper instead of arbitrary `tokio::time::sleep()` for deterministic tests

**Documentation**
- **FR-053**: System MUST include `///` doc comments for all Rust modules, structs, and public methods using third-person declarative voice
- **FR-054**: System MUST include comprehensive docstrings for all Python classes and methods
- **FR-055**: System MUST provide integration guide in `docs/integrations/longport.md`
- **FR-056**: System MUST provide runnable examples demonstrating data subscription and order execution

### Key Entities

- **LongportHttpClient**: Low-level HTTP client for LongPort REST API communication, handles authentication, request signing, and rate limiting
- **LongportWebSocketClient**: WebSocket client orchestrator managing connection lifecycle, subscriptions, and reconnection logic
- **LongportWsFeedHandler**: I/O boundary handler running in dedicated task, processes WebSocket messages and emits Nautilus domain events
- **LongportInstrumentProvider**: Loads and caches instrument definitions from LongPort, handles filtering by market and symbol
- **LongportDataClient**: Python data client wrapping Rust implementation, subscribes to market data and publishes to message bus
- **LongportExecutionClient**: Python execution client wrapping Rust implementation, manages order lifecycle
- **LongportDataClientConfig**: Configuration for data client with explicit URL parameters (http_url, wss_quote_url, wss_trade_url)
- **LongportExecClientConfig**: Configuration for execution client with explicit URL parameters and authentication credentials
- **SubscriptionState**: Shared state tracking subscription status (pending/confirmed) across client and handler using Arc<DashMap>
- **InstrumentAny**: Nautilus unified instrument type representing various instrument classes (Equity, Crypto, Future, Option)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: System can establish WebSocket connection to LongPort market data API and successfully authenticate within 5 seconds
- **SC-002**: System receives and correctly parses at least 100 quote ticks per second without data loss
- **SC-003**: System maintains accurate order book state through incremental deltas for 10+ concurrent instruments
- **SC-004**: System can load all available instruments for Hong Kong market (2000+ securities) in under 30 seconds
- **SC-005**: System correctly parses 100% of instrument attributes (price precision, tick size, lot size) from LongPort API responses
- **SC-006**: Order submission latency (client to LongPort API confirmation) is under 100ms at P99 percentile
- **SC-007**: System successfully reconnects and resubscribes to all data streams within 10 seconds after network disconnection
- **SC-008**: All integration tests pass with mock servers simulating LongPort API responses
- **SC-009**: Code achieves 80%+ test coverage across Rust and Python codebases
- **SC-010**: Zero data loss occurs during normal operation and reconnection scenarios
- **SC-011**: System handles LongPort API rate limits without exceeding quotas during high-frequency operations
- **SC-012**: Configuration requires all URL parameters to be explicitly specified (no environment variable reads)
- **SC-013**: All Rust functions called from Python use correct type conversions (no Python-defined types passed directly)

## Assumptions

1. LongPort API documentation is accurate and up-to-date for REST and WebSocket endpoints
2. LongPort provides test/demo environment for development and testing
3. LongPort uses token-based or HMAC-SHA256 request signing for API authentication
4. LongPort WebSocket APIs support reconnection and subscription resumption
5. LongPort API rate limits are documented and can be implemented client-side
6. NautilusTrader core framework provides necessary base classes (LiveDataClient, LiveExecutionClient, InstrumentProvider)
7. Rust compilation with PyO3 bindings works correctly in the development environment
8. OKX and BitMEX adapter implementations provide valid reference architecture patterns
9. `make clean && make build-debug` command successfully builds the project
10. `.venv` virtual environment is activated for all Python operations

## Dependencies

### External Dependencies
- LongPort API credentials (api_key, api_secret, passphrase if applicable)
- LongPort REST API base URL (HTTP endpoint)
- LongPort WebSocket API URLs (market data and trade execution)
- Access to LongPort demo/test environment for integration testing

### Internal Dependencies
- NautilusTrader core framework (nautilus_core, nautilus_model, nautilus_execution)
- NautilusTrader adapter base classes (LiveDataClient, LiveExecutionClient, InstrumentProvider)
- PyO3 bindings for Python-Rust FFI
- Existing OKX/BitMEX adapter implementations as architectural reference
- Constitution principles defined in `.specify/memory/constitution.md`
- Developer guide in `docs/developer_guide/adapters.md`

### Technical Dependencies
- Rust toolchain (stable version matching project requirements)
- Python 3.12+ virtual environment
- Tokio async runtime for Rust
- PyO3 for Python bindings
- Serialization libraries (serde for JSON in Rust)
