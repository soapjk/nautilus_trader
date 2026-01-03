# Tasks: LongPort Adapter API Support

**Input**: Design documents from `/specs/001-longport-adapter/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, quickstart.md

**Tests**: Tests are generated following constitution requirement: "just run command: sh ./scripts/run_debug.sh, check error msg"

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Adapter structure**: `crates/adapters/longport/src/` (Rust), `nautilus_trader/adapters/longport/` (Python)
- **Tests**: `crates/adapters/longport/tests/` (Rust), `tests/integration_tests/adapters/longport/` (Python)
- **Test data**: `crates/adapters/longport/test_data/` (LongPort API response samples)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Verify LongPort SDK installation in .venv with `pip list | grep longport`
- [ ] T002 Run `make clean && make build-debug` to verify project builds successfully
- [ ] T003 [P] Create Rust module structure in crates/adapters/longport/src/ (common/, http/, websocket/, data/, execution/, python/)
- [ ] T004 [P] Create Python module structure in nautilus_trader/adapters/longport/ with __init__.py
- [ ] T005 [P] Add longport adapter to Cargo.toml workspace members
- [ ] T006 [P] Create test data directory at crates/adapters/longport/test_data/
- [ ] T007 [P] Create integration test directory at tests/integration_tests/adapters/longport/
- [ ] T008 [P] Add longport to pytest test discovery in tests/integration_tests/conftest.py

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Common Types & Utilities

- [ ] T009 [P] Create venue constants in crates/adapters/longport/src/common/consts.rs (LONGPORT = "LONGPORT", market IDs)
- [ ] T010 [P] Define LongportSecurityType enum in crates/adapters/longport/src/common/enums.rs (Stock, ETF, Warrant, CBBC, Bond, Index, Option, Future)
- [ ] T011 [P] Define LongportMarket enum in crates/adapters/longport/src/common/enums.rs (HK, US, SH, SZ, HK_Futures, US_Options)
- [ ] T012 [P] Define LongportOrderType enum in crates/adapters/longport/src/common/enums.rs (LO, MO, ELO, AO, OTO, OCO)
- [ ] T013 [P] Define LongportSide enum in crates/adapters/longport/src/common/enums.rs (Buy, Sell)
- [ ] T014 [P] Create LongportInstrument model in crates/adapters/longport/src/common/models.rs with instrument attributes
- [ ] T015 [P] Create credential management in crates/adapters/longport/src/common/credential.rs with zeroization for secrets
- [ ] T016 Implement type conversion functions in crates/adapters/longport/src/common/convert.rs (Longport → Nautilus enums)
- [ ] T017 [P] Create LongportHttpError enum in crates/adapters/longport/src/http/error.rs (MissingCredentials, LongportError, JsonError, ValidationError, NetworkError, RateLimited, Unauthorized)
- [ ] T018 [P] Create LongportWsError enum in crates/adapters/longport/src/websocket/error.rs (ConnectionError, AuthenticationError, ParseError, RateLimited, InvalidMessage, SubscriptionFailed)
- [ ] T019 [P] Create test fixtures in crates/adapters/longport/src/common/testing.rs (sample credentials, test instruments)

### Configuration Layer

- [ ] T020 [P] Create Rust config structure in crates/adapters/longport/src/config.rs with explicit URL params (http_url, wss_quote_url, wss_trade_url)
- [ ] T021 Create Python LongportDataClientConfig in nautilus_trader/adapters/longport/config.py with app_key, app_secret, access_token, http_url, wss_quote_url, wss_trade_url (NO env vars)
- [ ] T022 Create Python LongportExecClientConfig in nautilus_trader/adapters/longport/config.py with trader_id, account_id, explicit URLs
- [ ] T023 Add config validation to ensure all required parameters are present (no None defaults for credentials/URLs)

### PyO3 Bindings Infrastructure

- [ ] T024 [P] Create PyO3 module skeleton in crates/adapters/longport/src/python/mod.rs with pymodule initialization
- [ ] T025 [P] Export LongportSecurityType, LongportMarket, LongportOrderType, LongportSide enums to Python in crates/adapters/longport/src/python/enums.rs
- [ ] T026 [P] Create type conversion helpers in crates/adapters/longport/src/python/convert.rs (instrument_any_to_pyobject, pyobject_to_instrument_any)
- [ ] T027 Ensure all Python-exposed functions use Rust-defined types (convert Python types before FFI boundary)

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Connect to LongPort Market Data (Priority: P1) 🎯 MVP

**Goal**: Enable traders to connect to LongPort market data APIs and receive real-time quotes, trades, and order book updates

**Independent Test**: Connect to LongPort test environment, subscribe to market data for single instrument "700.HK", validate quote ticks, trade ticks, and order book deltas are received and parsed correctly

### Rust HTTP Client (Instrument Loading for US1)

- [ ] T028 [P] [US1] Create LongportRawHttpClient in crates/adapters/longport/src/http/client.rs with SDK HTTP wrapper methods
- [ ] T029 [P] [US1] Implement request methods in LongportRawHttpClient for fetching instruments (get_security_info, get_security_snapshot)
- [ ] T030 [P] [US1] Create LongportHttpClient in crates/adapters/longport/src/http/client.rs with Arc wrapper and caching
- [ ] T031 [P] [US1] Implement parse functions in crates/adapters/longport/src/http/parse.rs (parse_instrument, parse_quote, parse_trade_tick)
- [ ] T032 [US1] Add retry logic with exponential backoff to LongportRawHttpClient using RetryManager from nautilus_network

### Rust WebSocket Client (Market Data for US1)

- [ ] T033 [P] [US1] Create LongportWebSocketClient in crates/adapters/longport/src/websocket/client.rs with connection lifecycle management
- [ ] T034 [P] [US1] Create HandlerCommand enum in crates/adapters/longport/src/websocket/client.rs (Subscribe, Unsubscribe, Authenticate, Disconnect)
- [ ] T035 [P] [US1] Create LongportWsFeedHandler in crates/adapters/longport/src/websocket/handler.rs as I/O boundary task
- [ ] T036 [P] [US1] Implement subscription state tracking in crates/adapters/longport/src/websocket/handler.rs using Arc<DashMap> (pending/confirmed)
- [ ] T037 [P] [US1] Create parse functions in crates/adapters/longport/src/websocket/parse.rs (parse_quote_tick, parse_trade_tick, parse_order_book_delta)
- [ ] T038 [P] [US1] Implement automatic reconnection logic in LongportWsFeedHandler with exponential backoff
- [ ] T039 [P] [US1] Implement subscription restoration after reconnection in LongportWebSocketClient (track and resubscribe)
- [ ] T040 [US1] Emit RECONNECTED sentinel message when connection is restored in LongportWsFeedHandler

### Rust Data Client Integration (US1)

- [ ] T041 [P] [US1] Create LongportDataClient in crates/adapters/longport/src/data/mod.rs with QuoteContext integration
- [ ] T042 [P] [US1] Implement subscribe_quote_ticks method in LongportDataClient with instrument validation
- [ ] T043 [P] [US1] Implement subscribe_trade_ticks method in LongportDataClient with instrument validation
- [ ] T044 [P] [US1] Implement subscribe_order_book_updates method in LongportDataClient with depth parameter
- [ ] T045 [US1] Publish QuoteTick, TradeTick, OrderBookDeltas to MessageBus in LongportDataClient

### PyO3 Data Client Bindings (US1)

- [ ] T046 [P] [US1] Create PyO3 bindings for LongportWebSocketClient in crates/adapters/longport/src/python/websocket.rs
- [ ] T047 [P] [US1] Create PyO3 bindings for LongportHttpClient in crates/adapters/longport/src/python/http.rs
- [ ] T048 [US1] Expose LongportDataClient to Python in crates/adapters/longport/src/python/mod.rs with #[pyclass]

### Python Data Client Implementation (US1)

- [ ] T049 [P] [US1] Create LongportDataClient in nautilus_trader/adapters/longport/data.py extending LiveMarketDataClient
- [ ] T050 [P] [US1] Implement _connect method in LongportDataClient with WebSocket initialization
- [ ] T051 [P] [US1] Implement _disconnect method in LongportDataClient with subscription cleanup
- [ ] T052 [P] [US1] Implement subscribe_quote_ticks in LongportDataClient with instrument_id validation
- [ ] T053 [P] [US1] Implement subscribe_trade_ticks in LongportDataClient with instrument_id validation
- [ ] T054 [P] [US1] Implement subscribe_order_book_snapshots in LongportDataClient with depth limit
- [ ] T055 [P] [US1] Implement subscribe_order_book_deltas in LongportDataClient with book_type validation
- [ ] T056 [US1] Add type conversions for instruments using instrument_any_to_pyobject in all subscription methods

### Testing - Market Data (US1)

- [ ] T057 [P] [US1] Capture real LongPort API quote responses from official docs and save to crates/adapters/longport/test_data/quote_responses.json
- [ ] T058 [P] [US1] Capture real LongPort API trade responses from official docs and save to crates/adapters/longport/test_data/trade_responses.json
- [ ] T059 [P] [US1] Create unit tests in crates/adapters/longport/tests/http.rs for quote parsing with real data
- [ ] T060 [P] [US1] Create unit tests in crates/adapters/longport/tests/websocket.rs for trade tick parsing with real data
- [ ] T061 [US1] Create integration test in tests/integration_tests/adapters/longport/test_data.py for WebSocket connection and quote subscription
- [ ] T062 [US1] Create integration test in tests/integration_tests/adapters/longport/test_data.py for trade tick subscription and data validation
- [ ] T063 [US1] Create integration test in tests/integration_tests/adapters/longport/test_data.py for reconnection and subscription restoration
- [ ] T064 [US1] Run `sh ./scripts/run_debug.sh` and verify no errors in data client functionality

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently - traders can connect to LongPort market data and receive real-time quotes, trades, and order book updates

---

## Phase 4: User Story 2 - Manage LongPort Instruments (Priority: P2)

**Goal**: Enable traders to retrieve and manage instrument definitions from LongPort with accurate attributes (tick size, lot size, price precision)

**Independent Test**: Load all available instruments for Hong Kong market, validate instruments are correctly parsed with accurate attributes, filter by market or symbol patterns

### Rust Instrument Provider (US2)

- [ ] T065 [P] [US2] Create LongportInstrumentProvider in crates/adapters/longport/src/common/providers.rs with load_all_async, load_ids_async, load_async methods
- [ ] T066 [P] [US2] Implement instrument caching in LongportInstrumentProvider using Arc<DashMap<InstrumentId, InstrumentAny>>
- [ ] T067 [P] [US2] Implement cache_instruments method in LongportInstrumentProvider for bulk instrument replacement
- [ ] T068 [P] [US2] Implement cache_instrument method in LongportInstrumentProvider for single instrument upsert
- [ ] T069 [P] [US2] Implement get_instrument method in LongportInstrumentProvider with symbol lookup
- [ ] T070 [P] [US2] Add market filtering logic in LongportInstrumentProvider (HK, US, SH, SZ)
- [ ] T071 [P] [US2] Implement parse_instrument_any function in crates/adapters/longport/src/common/parse.rs with full attribute mapping
- [ ] T072 [US2] Handle instrument attribute parsing in parse_instrument_any (price_precision, tick_size, lot_size, base_currency)
- [ ] T073 [P] [US2] Create PyO3 bindings for LongportInstrumentProvider in crates/adapters/longport/src/python/providers.rs

### Python Instrument Provider (US2)

- [ ] T074 [P] [US2] Create LongportInstrumentProvider in nautilus_trader/adapters/longport/providers.py with load_all_async, load_ids_async, load_async
- [ ] T075 [P] [US2] Implement _load_instruments method in LongportInstrumentProvider using Rust HTTP client
- [ ] T076 [P] [US2] Implement instrument filtering by markets list in LongportInstrumentProvider.load_all_async
- [ ] T077 [P] [US2] Add instrument caching in Python LongportInstrumentProvider using dict with instrument_id keys
- [ ] T078 [P] [US2] Implement get_instrument method in LongportInstrumentProvider with symbol string lookup
- [ ] T079 [P] [US2] Add type conversion using pyobject_to_instrument_any in all instrument lookups

### Testing - Instruments (US2)

- [ ] T080 [P] [US2] Capture real LongPort instrument data from official docs and save to crates/adapters/longport/test_data/instrument_data.json
- [ ] T081 [P] [US2] Create unit test in crates/adapters/longport/tests/http.rs for instrument parsing with real data
- [ ] T082 [P] [US2] Create integration test in tests/integration_tests/adapters/longport/test_providers.py for loading HK market instruments
- [ ] T083 [P] [US2] Create integration test in tests/integration_tests/adapters/longport/test_providers.py for filtering instruments by market
- [ ] T084 [P] [US2] Create integration test in tests/integration_tests/adapters/longport/test_providers.py for instrument lookup by symbol
- [ ] T085 [US2] Validate instrument attributes in tests (price_precision, tick_size, lot_size match LongPort data)
- [ ] T086 [US2] Run `sh ./scripts/run_debug.sh` and verify no errors in instrument provider functionality

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - traders can load instruments dynamically and use them for market data subscriptions

---

## Phase 5: User Story 3 - Execute Orders on LongPort (Priority: P3)

**Goal**: Enable traders to submit, modify, and cancel orders through LongPort trading API with real-time status updates and fill confirmations

**Independent Test**: Submit test orders to LongPort demo environment, validate order submission, modification, cancellation, and fill reporting workflows

### Rust Execution Client (US3)

- [ ] T087 [P] [US3] Create LongportExecutionClient in crates/adapters/longport/src/execution/mod.rs with TradeContext integration
- [ ] T088 [P] [US3] Implement submit_order method in LongportExecutionClient with order validation (quantity % lot_size == 0, price % tick_size == 0)
- [ ] T089 [P] [US3] Implement modify_order method in LongportExecutionClient with order_id lookup
- [ ] T090 [P] [US3] Implement cancel_order method in LongportExecutionClient with order_id validation
- [ ] T091 [P] [US3] Implement cancel_all_orders method in LongportExecutionClient with venue filtering
- [ ] T092 [P] [US3] Create order state tracking in LongportExecutionClient using Arc<DashMap<ClientOrderId, LongportOrder>>
- [ ] T093 [P] [US3] Implement order reconciliation in LongportExecutionClient (match LongPort state with Nautilus state)
- [ ] T094 [P] [US3] Handle order status updates from LongPort WebSocket in LongportWsFeedHandler (executions channel)
- [ ] T095 [P] [US3] Publish OrderAccepted, OrderRejected, OrderFilled, OrderCanceled, OrderUpdated events to MessageBus
- [ ] T096 [P] [US3] Implement parse_order_status_report function in crates/adapters/longport/src/execution/parse.rs with fill tracking

### PyO3 Execution Bindings (US3)

- [ ] T097 [P] [US3] Create PyO3 bindings for LongportExecutionClient in crates/adapters/longport/src/python/execution.rs
- [ ] T098 [P] [US3] Expose order submission methods to Python with proper type conversions (convert Python Order to Rust order)

### Python Execution Client (US3)

- [ ] T099 [P] [US3] Create LongportExecutionClient in nautilus_trader/adapters/longport/execution.py extending LiveExecutionClient
- [ ] T100 [P] [US3] Implement _connect method in LongportExecutionClient with TradeContext initialization
- [ ] T101 [P] [US3] Implement submit_order in LongportExecutionClient with order type conversion (Nautilus → Longport)
- [ ] T102 [P] [US3] Implement modify_order in LongportExecutionClient with order_id validation
- [ ] T103 [P] [US3] Implement cancel_order in LongportExecutionClient with venue_order_id lookup
- [ ] T104 [P] [US3] Implement cancel_all_orders in LongportExecutionClient with instrument filtering
- [ ] T105 [P] [US3] Add order list management in LongportExecutionClient (track active orders)
- [ ] T106 [P] [US3] Handle OrderAccepted events in LongportExecutionClient and publish to MessageBus
- [ ] T107 [P] [US3] Handle OrderFilled events in LongportExecutionClient with fill price/quantity aggregation
- [ ] T108 [P] [US3] Handle OrderCanceled events in LongportExecutionClient with status updates
- [ ] T109 [P] [US3] Handle OrderRejected events in LongportExecutionClient with error message extraction

### Testing - Execution (US3)

- [ ] T110 [P] [US3] Capture real LongPort order responses from official docs and save to crates/adapters/longport/test_data/order_responses.json
- [ ] T111 [P] [US3] Create unit test in crates/adapters/longport/tests/execution.rs for order submission parsing with real data
- [ ] T112 [P] [US3] Create integration test in tests/integration_tests/adapters/longport/test_execution.py for market order submission
- [ ] T113 [P] [US3] Create integration test in tests/integration_tests/adapters/longport/test_execution.py for limit order submission
- [ ] T114 [P] [US3] Create integration test in tests/integration_tests/adapters/longport/test_execution.py for order modification
- [ ] T115 [P] [US3] Create integration test in tests/integration_tests/adapters/longport/test_execution.py for order cancellation
- [ ] T116 [P] [US3] Create integration test in tests/integration_tests/adapters/longport/test_execution.py for fill reporting
- [ ] T117 [P] [US3] Create integration test in tests/integration_tests/adapters/longport/test_execution.py for order reconciliation
- [ ] T118 [US3] Run `sh ./scripts/run_debug.sh` and verify no errors in execution client functionality

**Checkpoint**: All user stories should now be independently functional - complete end-to-end trading workflow from market data to order execution

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

### Documentation

- [ ] T119 [P] Add /// doc comments to all Rust modules in crates/adapters/longport/src/ using third-person declarative voice
- [ ] T120 [P] Add comprehensive docstrings to all Python classes and methods in nautilus_trader/adapters/longport/
- [ ] T121 Create integration guide in docs/integrations/longport.md with setup instructions and examples
- [ ] T122 [P] Create runnable example in examples/live/longport/longport_market_data.py demonstrating data subscription
- [ ] T123 [P] Create runnable example in examples/live/longport/longport_execution.py demonstrating order submission
- [ ] T124 Add type hints to all Python methods in nautilus_trader/adapters/longport/ following PEP 484

### Error Handling & Logging

- [ ] T125 [P] Add clear error messages with recovery suggestions to all LongportHttpError and LongportWsError variants
- [ ] T126 [P] Add logging at appropriate levels (DEBUG, INFO, WARNING, ERROR) with use_pyo3=True throughout adapter
- [ ] T127 [P] Add audit logging for all trading operations (order submit/modify/cancel, fills)

### Performance Optimization

- [ ] T128 [P] Use ustr::Ustr for string interning of repeated fields (symbols, venues, instrument IDs) throughout Rust code
- [ ] T129 [P] Use Arc<DashMap> for concurrent access in instrument cache and order tracking
- [ ] T130 [P] Use AHashMap in single-threaded hot paths (WebSocket handler inner loop)
- [ ] T131 Add rate limiting configuration in crates/adapters/longport/src/common/consts.rs using LazyLock<Quota> statics

### Final Testing & Validation

- [ ] T132 Run `sh ./scripts/run_debug.sh` and verify no errors across entire adapter
- [ ] T133 [P] Run all Rust unit tests with `cargo test --package nautilus-longport`
- [ ] T134 [P] Run all Python integration tests with `pytest tests/integration_tests/adapters/longport/`
- [ ] T135 Verify test coverage is 80%+ across Rust and Python codebases
- [ ] T136 [P] Validate all type conversions at PyO3 boundaries (no Python-defined types passed to Rust)
- [ ] T137 Validate all configuration parameters are explicit (no environment variable reads)
- [ ] T138 [P] Test with make clean && make build-debug to ensure project builds successfully

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1) - Market Data**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2) - Instruments**: Can start after Foundational (Phase 2) - No dependencies on US1, but US1 benefits from instrument caching
- **User Story 3 (P3) - Execution**: Can start after Foundational (Phase 2) - May integrate with US1 (market data) and US2 (instruments) but should be independently testable

### Within Each User Story

- Rust HTTP/WebSocket clients can be developed in parallel (marked with [P])
- Parser functions can be developed in parallel within each client layer
- Python implementation depends on corresponding Rust PyO3 bindings
- Tests should accompany each implementation task
- Core implementation before integration

### Parallel Opportunities

**Phase 1 (Setup)**:
```bash
# Launch all setup tasks together:
Task: "Create Rust module structure in crates/adapters/longport/src/"
Task: "Create Python module structure in nautilus_trader/adapters/longport/"
Task: "Add longport adapter to Cargo.toml workspace members"
Task: "Create test data directory at crates/adapters/longport/test_data/"
```

**Phase 2 (Foundational)**:
```bash
# Launch all common types in parallel:
Task: "Define LongportSecurityType enum in crates/adapters/longport/src/common/enums.rs"
Task: "Define LongportMarket enum in crates/adapters/longport/src/common/enums.rs"
Task: "Define LongportOrderType enum in crates/adapters/longport/src/common/enums.rs"
Task: "Define LongportSide enum in crates/adapters/longport/src/common/enums.rs"
```

**User Story 1 (Market Data)**:
```bash
# Launch HTTP and WebSocket parsing in parallel:
Task: "Implement parse functions in crates/adapters/longport/src/http/parse.rs"
Task: "Implement parse functions in crates/adapters/longport/src/websocket/parse.rs"

# Launch all data client subscription methods in parallel:
Task: "Implement subscribe_quote_ticks in LongportDataClient"
Task: "Implement subscribe_trade_ticks in LongportDataClient"
Task: "Implement subscribe_order_book_snapshots in LongportDataClient"
```

**User Story 2 (Instruments)**:
```bash
# Launch instrument provider methods in parallel:
Task: "Implement cache_instruments method in LongportInstrumentProvider"
Task: "Implement cache_instrument method in LongportInstrumentProvider"
Task: "Implement get_instrument method in LongportInstrumentProvider"
```

**User Story 3 (Execution)**:
```bash
# Launch all order operations in parallel:
Task: "Implement submit_order method in LongportExecutionClient"
Task: "Implement modify_order method in LongportExecutionClient"
Task: "Implement cancel_order method in LongportExecutionClient"
Task: "Implement cancel_all_orders method in LongportExecutionClient"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 - Market Data Connection
4. **STOP and VALIDATE**: Test market data connection independently
5. Deploy/demo if ready - traders can now receive real-time quotes and trades from LongPort

**MVP Delivers**:
- Real-time quote tick subscription for HK/US/CN markets
- Real-time trade tick subscription
- Order book delta subscription
- Automatic reconnection with subscription restoration
- Zero data loss during normal operation and reconnections

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo (Dynamic instrument loading!)
4. Add User Story 3 → Test independently → Deploy/Demo (Full trading capability!)
5. Each story adds value without breaking previous stories

**Incremental Value**:
- After US1: Market data streaming operational
- After US2: Dynamic instrument management (no hardcoding)
- After US3: Complete algorithmic trading capability

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - **Developer A**: User Story 1 - Market Data (Rust HTTP/WebSocket clients)
   - **Developer B**: User Story 2 - Instruments (Rust instrument provider)
   - **Developer C**: User Story 3 - Execution (Rust execution client) - after basic HTTP client ready
3. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies, can run in parallel
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests pass and run `sh ./scripts/run_debug.sh` after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- **Constitution Compliance**: All tasks follow constitution principles (no env vars, explicit URLs, type-safe PyO3, real test data, sh ./scripts/run_debug.sh for testing)
