# Feature Specification: Complete Rust Implementation for Longport Adapter

**Feature Branch**: `001-longport-rust-impl`
**Created**: 2025-12-30
**Status**: Draft
**Input**: User description: "现在集中于如何实现rust版本的longport完整支持，包括交易和实时数据流。 目前项目中含有python和rust两个版本的实现。我要求你完成rust实现，以及基于pyo3的rust->python的绑定。并将examples/separate_data_and_strategy/data_producer_longport.py修改为支持rust包的版本。"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Real-Time Market Data Collection (Priority: P1)

**Description**: A quantitative trader needs to collect high-frequency real-time market data from Longport (Hong Kong, US, or China markets) for algorithmic strategy backtesting and live trading. The trader wants to run a dedicated data collection process that streams quotes, trades, order book updates, and candlestick bars to a message bus (Redis) for consumption by separate strategy processes.

**Why this priority**: This is the foundation of any trading system. Without reliable real-time data, no trading decisions can be made. The user's example explicitly focuses on data collection as the primary use case.

**Independent Test**: Can be tested by running the updated data producer example with Rust implementation and verifying that:
- Market data connects successfully for configured instruments
- All data types (quotes, trades, order book, bars) are received
- Data is published to Redis streams in the expected format
- Data persistence to local Parquet files works correctly

**Acceptance Scenarios**:

1. **Given** valid Longport credentials and a list of Hong Kong stock instruments (e.g., "700.HK", "9988.HK"), **When** the data producer is started with the Rust implementation, **Then** the system successfully establishes WebSocket connection and begins receiving real-time market data within 5 seconds
2. **Given** an active data streaming session, **When** quote ticks, trade ticks, order book deltas, and bar data are received from Longport, **Then** each data type is correctly published to Redis streams using msgpack encoding
3. **Given** streaming configuration with local file persistence enabled, **When** market data is received, **Then** data is written to Parquet files in the configured catalog path with file rotation at the specified size limit
4. **Given** a network disconnection event, **When** connection to Longport is lost, **Then** the system automatically reconnects and resumes data streaming without manual intervention
5. **Given** invalid or expired credentials, **When** the data producer attempts to connect, **Then** a clear error message is displayed explaining the authentication failure

---

### User Story 2 - Order Execution and Management (Priority: P2)

**Description**: An algorithmic trader needs to submit, monitor, cancel, and modify orders through Longport for automated trading execution. The trader requires full order lifecycle management including order status tracking, position updates, and account balance information.

**Why this priority**: Once data collection is working, the next critical capability is actual trading execution. This enables automated strategies to execute trades based on their signals.

**Independent Test**: Can be tested by implementing a simple strategy that:
- Submits a market order
- Monitors order status through fill events
- Cancels a pending limit order
- Modifies an existing order's price or quantity

**Acceptance Scenarios**:

1. **Given** a connected execution client with valid trading permissions, **When** a market order is submitted for a valid instrument, **Then** the order is accepted by Longport and an order submitted confirmation is received
2. **Given** an active limit order, **When** the order is partially or fully filled, **Then** fill events are received with correct execution price and quantity
3. **Given** a pending limit order, **When** a cancel request is submitted, **Then** the order is cancelled and a cancel confirmation is received
4. **Given** an active limit order, **When** a modify request is submitted with new price or quantity, **Then** the order is updated and a modification confirmation is received
5. **Given** multiple orders across different instruments, **When** order status is queried, **Then** all active, filled, and cancelled orders are correctly reported with their current status
6. **Given** trading activity has occurred, **When** account balance is queried, **Then** current cash balance and available buying power are accurately reported
7. **Given** open positions exist, **When** positions are queried, **Then** all positions are reported with correct quantity, average price, and unrealized profit/loss

---

### User Story 3 - Python Integration for Existing Users (Priority: P3)

**Description**: An existing NautilusTrader user who has built strategies in Python wants to use the high-performance Rust implementation of the Longport adapter without rewriting their code. The user expects to import and use the Rust implementation through Python with the same API as the existing Python adapter.

**Why this priority**: This enables adoption of the performance benefits of Rust without requiring users to migrate their existing Python codebases, ensuring backward compatibility and smooth transition path.

**Independent Test**: Can be tested by running an existing Python strategy that imports from the Longport adapter, verifying that:
- The Rust-backed classes can be instantiated from Python
- All methods work correctly through the PyO3 bindings
- Data and execution events flow correctly to Python callbacks

**Acceptance Scenarios**:

1. **Given** an existing Python script that imports Longport adapter classes, **When** the import statements are updated to use the Rust implementation, **Then** the script runs without modification to the business logic
2. **Given** a Python script creating a LongportDataClientConfig, **When** configuration parameters are set and passed to the factory, **Then** the Rust-backed client is initialized with the correct configuration
3. **Given** a Python callback registered for market data events, **When** data is received by the Rust client, **Then** the callback is invoked with correctly converted Python objects
4. **Given** a Python script submitting orders through the execution client, **When** orders are submitted via the Rust implementation, **Then** order confirmations and fill events are received in Python callbacks
5. **Given** error conditions occur in the Rust layer (network failure, invalid order, etc.), **When** errors are propagated to Python, **Then** appropriate Python exceptions are raised with clear error messages

---

### Edge Cases

- **Connection instability**: What happens when the network connection to Longport intermittently fails and recovers? The system must automatically reconnect and resume operation without data loss or state corruption.
- **Order rejection**: How does the system handle orders rejected by Longport due to insufficient margin, invalid instrument, or trading restrictions? Clear error messages must be provided to the user.
- **High-frequency data bursts**: What happens when an extremely high volume of market data is received (e.g., during market open/close or major news events)? The system must handle bursts without memory exhaustion or significant data loss.
- **Concurrent operations**: How does the system handle simultaneous order submissions, cancellations, and queries across multiple instruments? Operations must be correctly serialized and race conditions prevented.
- **Partial fills**: What happens when an order is partially filled over multiple executions? Each partial fill must generate a separate event and the remaining quantity must be correctly tracked.
- **Market status changes**: How does the system handle market opening, closing, and trading halts? The system must respect market hours and halt data/trading appropriately.
- **Credential expiration**: What happens when access tokens expire during an active session? The system must detect and handle re-authentication or provide clear guidance.
- **Invalid configuration**: How does the system handle invalid or conflicting configuration parameters? Clear validation errors must be provided at startup.

## Requirements *(mandatory)*

### Functional Requirements

#### Real-Time Data Streaming (P1)

- **FR-001**: The system MUST provide a high-performance data client implemented in Rust that connects to Longport WebSocket APIs for real-time market data
- **FR-002**: The data client MUST support subscription to quote ticks (bid/ask prices and sizes) for multiple instruments simultaneously
- **FR-003**: The data client MUST support subscription to trade ticks (last price and volume) for multiple instruments
- **FR-004**: The data client MUST support subscription to order book deltas (L2 depth updates) for multiple instruments
- **FR-005**: The data client MUST support subscription to bar/candlestick data (OHLCV) with configurable timeframes (1-minute, 5-minute, 15-minute, 1-hour, 1-day)
- **FR-006**: The data client MUST support Hong Kong (HK), United States (US), and China (CN) markets
- **FR-007**: The data client MUST automatically reconnect to Longport WebSocket when connection is lost
- **FR-008**: The data client MUST handle all WebSocket event types from Longport SDK (quotes, trades, order book, candlesticks)
- **FR-009**: The data client MUST integrate with NautilusTrader message bus for publishing market data events
- **FR-010**: The data client MUST support Redis streaming backend with msgpack encoding for high-performance data distribution

#### Order Execution (P2)

- **FR-011**: The system MUST provide an execution client implemented in Rust that connects to Longport trading APIs
- **FR-012**: The execution client MUST support submitting market orders with immediate execution
- **FR-013**: The execution client MUST support submitting limit orders with specified price and time-in-force (Day, GTC, IOC, FOK)
- **FR-014**: The execution client MUST support submitting stop orders with specified stop price
- **FR-015**: The execution client MUST support submitting stop-limit orders with specified stop and limit prices
- **FR-016**: The execution client MUST support canceling individual orders by order ID
- **FR-017**: The execution client MUST support modifying existing orders (price and quantity)
- **FR-018**: The execution client MUST support querying order status for active orders
- **FR-019**: The execution client MUST support querying account balance including cash and buying power
- **FR-020**: The execution client MUST support querying current positions with quantity, average price, and unrealized P&L
- **FR-021**: The execution client MUST emit order submitted, order accepted, order filled, order cancelled, and order rejected events
- **FR-022**: The execution client MUST validate order parameters (instrument, quantity, price) before submission
- **FR-023**: The execution client MUST handle partial order fills correctly with proper event generation

#### Python Bindings (P3)

- **FR-024**: The system MUST provide PyO3 bindings allowing Python code to instantiate and use the Rust data client
- **FR-025**: The system MUST provide PyO3 bindings allowing Python code to instantiate and use the Rust execution client
- **FR-026**: The system MUST expose Longport configuration classes (LongportDataClientConfig, LongportExecClientConfig) to Python
- **FR-027**: The system MUST expose Longport factory classes (LongportLiveDataClientFactory, LongportExecClientFactory) to Python
- **FR-028**: The system MUST expose Longport enums (LongportMarket) to Python
- **FR-029**: Python bindings MUST convert NautilusTrader core types correctly between Python and Rust representations
- **FR-030**: Python bindings MUST propagate errors from Rust to Python as appropriate Python exceptions
- **FR-031**: Python bindings MUST support async/await patterns compatible with Python's asyncio
- **FR-032**: The data producer example (data_producer_longport.py) MUST be updated to demonstrate using the Rust-backed implementation

#### Instrument and Market Support

- **FR-033**: The system MUST provide an instrument provider that loads instrument definitions from Longport for HK, US, and CN markets
- **FR-034**: The instrument provider MUST support loading all instruments or a filtered subset by instrument ID
- **FR-035**: The system MUST correctly parse Longport instrument symbols (e.g., "700.HK", "AAPL.US")
- **FR-036**: The system MUST handle instrument-specific lot sizes and trading units

#### Error Handling and Logging

- **FR-037**: The system MUST log all connection attempts, successes, and failures at appropriate log levels
- **FR-038**: The system MUST log all order submissions, cancellations, and modifications with confirmation details
- **FR-039**: The system MUST handle Longport API errors gracefully with clear error messages
- **FR-040**: The system MUST validate credentials at startup and provide clear error messages for authentication failures
- **FR-041**: The system MUST handle rate limiting from Longport APIs without crashing

### Key Entities

#### **MarketDataEvent**
Represents any real-time market data update from Longport. Subtypes include:
- QuoteTick: Bid and ask prices with quantities and timestamp
- TradeTick: Last execution price and volume with timestamp
- OrderBookDelta: Incremental update to order book depth with price-level changes
- Bar: Candlestick data with open, high, low, close, volume, and timestamp

#### **Order**
Represents a trading order submitted to Longport with attributes:
- Order ID: Unique identifier assigned by Longport
- Instrument ID: The financial instrument being traded
- Side: Buy or sell direction
- Type: Market, limit, stop, or stop-limit
- Quantity: Number of shares/contracts
- Price: Limit price (for limit and stop-limit orders)
- Stop Price: Stop trigger price (for stop and stop-limit orders)
- Time in Force: Day, GTC, IOC, or FOK
- Status: Pending, submitted, accepted, partially filled, filled, cancelled, or rejected

#### **AccountBalance**
Represents the trading account state with attributes:
- Cash Balance: Available cash in the account
- Total Asset Value: Cash plus market value of all positions
- Unrealized P&L: Profit/loss from open positions
- Realized P&L: Profit/loss from closed positions
- Available Buying Power: Maximum value of new positions that can be opened

#### **Position**
Represents a holding in a specific instrument with attributes:
- Instrument ID: The financial instrument held
- Quantity: Number of shares/contracts (positive for long, negative for short)
- Average Price: Weighted average entry price
- Market Value: Current market value of the position
- Unrealized P&L: Current profit/loss if position were closed

#### **LongportCredential**
Represents authentication credentials for Longport API with attributes:
- App Key: Application identifier from Longport developer portal
- App Secret: Application secret for authentication
- Access Token: User-specific access token for API access
- HTTP URL: Optional custom API endpoint
- WebSocket URL: Optional custom WebSocket endpoint

## Success Criteria *(mandatory)*

### Measurable Outcomes

#### Performance Metrics

- **SC-001**: Market data latency from Longport WebSocket receipt to internal event processing is under 10 milliseconds for 95% of events
- **SC-002**: Order submission latency from client API call to confirmation receipt is under 500 milliseconds for market orders during normal market conditions
- **SC-003**: The Rust implementation uses at least 50% less memory than the equivalent Python implementation when handling 100 concurrent instrument subscriptions
- **SC-004**: The data producer can handle data bursts of 10,000 events per second without event loss or significant processing delay (>50ms)
- **SC-005**: WebSocket reconnection after network failure completes within 5 seconds and resumes data streaming automatically

#### Functional Completeness

- **SC-006**: All existing Python data client functionality is available through the Rust implementation as verified by passing the existing integration test suite
- **SC-007**: All order types (market, limit, stop, stop-limit) can be submitted successfully through the Rust execution client
- **SC-008**: Order cancellation and modification operations work correctly for 100% of test cases
- **SC-009**: Account balance and position queries return results matching Longport web interface values
- **SC-010**: The updated data producer example successfully collects and publishes all four data types (quotes, trades, order book, bars) to Redis streams

#### Integration Quality

- **SC-011**: Python scripts using the Rust implementation run without modification to business logic for all documented use cases
- **SC-012**: All error conditions from Rust are properly converted to Python exceptions with clear, actionable error messages
- **SC-013**: The implementation passes all existing adapter integration tests in the test suite
- **SC-014**: Code coverage for new Rust implementation is at least 80% as measured by unit and integration tests

#### Reliability and Stability

- **SC-015**: The data producer runs continuously for 24 hours without memory leaks or crashes under normal market conditions
- **SC-016**: The system handles 100 sequential reconnect events without degradation in functionality or performance
- **SC-017**: Zero data corruption events occur during normal operation as verified by data consistency checks
- **SC-018**: The system gracefully handles and recovers from all documented error conditions (network failures, API errors, invalid orders)

## Dependencies and Assumptions

### Dependencies

- **NautilusTrader Core**: The implementation depends on NautilusTrader core abstractions (DataClient, ExecutionClient, TradingNode) being available and stable
- **Longport OpenSDK (Rust)**: The implementation requires the official longport-rust SDK version 3.x or higher
- **PyO3**: Python bindings require PyO3 0.20 or higher with async runtime support
- **Tokio**: Async runtime requires Tokio 1.x with multi-threaded executor
- **Redis**: For Redis streaming functionality, requires Redis 6.x or higher with streams support

### Assumptions

- **Longport API Stability**: The Longport API maintains backward compatibility for the features used. Breaking API changes will require implementation updates.
- **Market Data Permissions**: Users have appropriate Longport account permissions for the markets and data types they wish to access
- **Network Connectivity**: Users have reliable internet connectivity with latency under 100ms to Longport API endpoints for optimal performance
- **Credential Management**: Users obtain and securely store their Longport API credentials outside of the codebase (environment variables or secure configuration files)
- **Python 3.10+**: Python binding users are running Python 3.10 or higher with compatible NautilusTrader installation
- **Platform Support**: The implementation targets macOS and Linux platforms. Windows support is not guaranteed due to Tokio/PyO3 limitations
- **Async Runtime**: Only one async runtime (Tokio in Rust, asyncio in Python) is active per process to avoid runtime conflicts
- **Order Types**: Longport supports all specified order types for the target instruments. Some instruments may have restrictions (e.g., no shorting, no stop orders)
- **Market Hours**: Trading and data collection only occurs during market hours for the selected market. Pre-market and after-hours data availability varies by market

## Out of Scope

The following items are explicitly out of scope for this feature:

- **Historical Data Backfill**: The implementation focuses on real-time streaming. Historical data retrieval for backtesting is handled separately.
- **WebSocket Proxy Support**: Direct connection to Longport WebSocket endpoints. Custom proxy configuration is not included.
- **Order Routing Algorithms**: Basic order submission is included. Advanced order routing (TWAP, VWAP, implementation shortfall) is not included.
- **Risk Management**: Basic order validation is included. Portfolio-level risk management (position limits, drawdown limits) is handled by the strategy layer.
- **Machine Learning Integration**: No ML model serving or inference capabilities are included in the adapter.
- **Alternative Python Bindings**: PyO3 is the only supported binding method. Alternates like PyBind11 or CFFI are not considered.
- **GUI Applications**: The adapter is designed for headless trading systems. No GUI or visualization components are included.
- **Multi-Account Support**: Single account per configuration. Multi-account aggregation is not included.
