<!--
Sync Impact Report:
- Version: Initial (0.0.0 → 1.0.0)
- Rationale: MAJOR bump - initial constitution establishment with 15 core principles
- Modified principles: N/A (initial creation)
- Added sections:
  * Core Principles (15 principles covering architecture, build environment, adapter standards, code quality, testing, LongPort specifics, performance, UX, security, implementation patterns, documentation, workflow, project requirements, success metrics)
  * Non-Negotiable Standards
  * Governance
- Removed sections: N/A (initial creation)
- Templates requiring updates:
  * ✅ .specify/templates/plan-template.md - Reviewed, "Constitution Check" section aligns
  * ✅ .specify/templates/spec-template.md - Reviewed, requirements section aligns
  * ✅ .specify/templates/tasks-template.md - Reviewed, task organization aligns with principles
- Follow-up TODOs: None
-->

# NautilusTrader LongPort Adapter Constitution

## Core Principles

### I. Architecture Excellence

- Maintain clear separation between Rust core and Python API layers
- Follow established adapter pattern across all exchange integrations
- Implement strict event-driven architecture using the project's message bus
- Ensure all components align with NautilusTrader's domain-driven design
- **Reference implementation**: Follow patterns from OKX, BitMEX, and Bybit adapters

**Rationale**: The hybrid Rust/Python architecture is foundational to NautilusTrader's performance and usability. Clear separation ensures type safety and performance in critical paths while maintaining Python ergonomics for strategy development. Event-driven architecture enables real-time responsiveness required for HFT scenarios.

### II. Build & Development Environment

- **Mandatory build command**: Always use `make clean && make build-debug` to compile or build the project
- **Python environment**: Must use `.venv` Python virtual environment for all Python operations
- **Testing script**: Run `./scripts/run_debug.sh` for testing and validation
- **Data storage**: Save all trade or market data in `data_catalog` folder, organized in sub-folders by adapter name (e.g., `data_catalog/longport/`)

**Rationale**: Consistent build and testing environments prevent integration issues and ensure reproducibility. Virtual environment isolation prevents dependency conflicts. Structured data storage enables efficient data management and analysis.

### III. Adapter Development Standards

- **Phase-driven implementation**: Follow the 7-phase implementation sequence (Rust core → Instruments → Market data → Execution → Advanced features → Configuration → Testing)
- **Rust-first architecture**: Implement Rust core before any Python layer
- **Two-layer HTTP client**: Separate raw client (low-level API) from domain client (high-level Nautilus types)
- **Dual-tier WebSocket architecture**: Outer client (orchestrator) with Arc<DashMap> for Python access, inner handler (I/O boundary) with AHashMap for performance
- **Type qualification**: Use short names for adapter-specific and Nautilus domain types; fully qualify only `anyhow` and `tokio` types
- **String interning**: Use `ustr::Ustr` for repeated strings (venues, symbols, instrument IDs)
- **Instrument cache standardization**: Implement `cache_instruments()`, `cache_instrument()`, and `get_instrument()` methods

**Rationale**: Phase-driven development ensures dependencies are respected and prevents rework. Two-layer architectures enable efficient Python bindings via Arc while maintaining clean separation of concerns. Type conventions improve readability and reduce compilation errors. String interning optimizes memory for repeated string comparisons.

### IV. Code Quality Standards

- **Rust type safety**: Leverage Rust's type system for all performance-critical paths
- **Python type annotations**: Use comprehensive Python type annotations for strategy APIs
- **Error handling**: Follow project's Result<T, E> Rust pattern and Python exception handling
- **Code style**: Comply with Rust standard guidelines and PEP 8 for Python
- **Documentation**: All public APIs must have complete docstrings following the project's documentation style guide
- **PyO3 type compatibility**: When calling Rust functions from Python through PyO3, ensure type compatibility - especially for Python-defined types, convert to Rust-defined types when necessary

**Rationale**: Type safety catches errors at compile time rather than runtime, critical for trading systems. Comprehensive documentation ensures maintainability and enables correct usage. PyO3 type compatibility prevents subtle bugs at language boundaries.

### V. Testing Requirements

- **Unit test coverage**: just run command:"sh ./scripts/run_debug.sh", check error msg

### VI. LongPort Integration Specifics

- **API compliance**: Strictly follow LongPort official API documentation and specifications
- **WebSocket stability**: Implement stable WebSocket connections for real-time market data with proper reconnection logic
- **Order management**: Support all LongPort order types and execution instructions
- **Data reconciliation**: Ensure accurate state reconciliation with LongPort systems on connect and reconnect
- **Rate limiting**: Implement proper rate limiting for LongPort API quotas
- **Authentication**: Handle LongPort-specific authentication mechanisms including token refresh.
- **Configuration**: support longport config class with full param,especailly wss_trade_url,wss_trade_url,http_url. not read envrionment variable but ask for params when init

**Rationale**: Strict API compliance ensures reliability and prevents breaking changes from venue updates. WebSocket stability and reconciliation are critical for real-time trading. Proper authentication and rate limiting prevent API bans and ensure continuous operation.

### VII. Performance Requirements

- **Latency target**: Order execution latency <100ms at P99
- **Throughput**: Support 100+ orders per second
- **Memory efficiency**: Control memory usage with proper cleanup and zero-copy where possible
- **Concurrency**: Leverage async I/O and multi-threading appropriately using Rust's async ecosystem
- **Lock-free hot paths**: Use Arc<DashMap> for concurrent access, AHashMap for single-threaded performance

**Rationale**: Sub-100ms P99 latency is required for competitive HFT strategies. High throughput enables scaling to multiple instruments and strategies. Memory efficiency prevents OOM kills in long-running production systems. Lock-free data structures minimize contention on hot paths.

### VIII. User Experience Consistency

- **Configuration patterns**: Use consistent configuration patterns with existing adapters (BinanceDataClientConfig, OKXExecClientConfig, etc.)
- **Logging standards**: Follow project logging levels (DEBUG, INFO, WARNING, ERROR) and formats with use_pyo3=True
- **Error messages**: Provide clear, actionable error messages with recovery suggestions
- **Monitoring metrics**: Expose consistent monitoring metrics across all adapters
- **Environment variables**: Support credential resolution from environment variables (e.g., `LONGPORT_API_KEY`, `LONGPORT_API_SECRET`)

**Rationale**: Consistency across adapters reduces cognitive load and enables swapping adapters without relearning patterns. Clear error messages reduce debugging time. Monitoring metrics enable operational visibility. Environment variable support enables secure credential management.

### IX. Security & Reliability

**Rationale**: Security is non-negotiable for trading systems handling real funds. Zeroization prevents credential leakage in memory dumps. Audit logging enables post-mortem analysis and compliance. Least privilege minimizes blast radius of compromised credentials.

### X. Rust Implementation Patterns

- **HTTP client structure**: Two-layer architecture with `LongportRawHttpClient` (low-level) and `LongportHttpClient` (domain-level with Arc wrapper)
- **WebSocket client structure**: Two-layer with `LongportWebSocketClient` (orchestrator) and `LongportWsFeedHandler` (I/O boundary)
- **Parser functions**: Place in `common/parse.rs` for cross-cutting or `http/parse.rs`/`websocket/parse.rs` for specific transformations
- **Query builders**: Use `derive_builder` with `#[builder(setter(into, strip_option), default)]` for ergonomic APIs
- **Rate limiting**: Configure using `LazyLock<Quota>` static variables with proper naming (e.g., `LONGPORT_REST_QUOTA`)
- **Python bindings**: Expose through PyO3 with `#[pyclass]` and `#[pymethods]`, prefixing internal methods with `py_` and using `#[pyo3(name = "...")]` for clean Python APIs
- **Subscription management**: Use shared `SubscriptionState` via `Arc<DashMap>` between client and handler
- **Reconnection logic**: Track subscriptions for restoration, emit `RECONNECTED` sentinel on reconnect

**Rationale**: Standardized patterns across adapters improve maintainability and enable code reuse. Two-layer architectures enable efficient cloning for Python bindings. derive_builder reduces boilerplate. Static rate limiting prevents global rate limit violations. Consistent naming enables code navigation and understanding.

### XI. Python Implementation Patterns

- **InstrumentProvider**: Implement `load_all_async`, `load_ids_async`, and `load_async` methods
- **LiveDataClient**: Extend `LiveDataClient` or `LiveMarketDataClient` based on data type
- **LiveExecutionClient**: Extend `LiveExecutionClient` with order management methods
- **Factories**: Implement factory functions for client instantiation from configuration
- **Configuration**: Create `LongportDataClientConfig` and `LongportExecClientConfig` following adapter patterns
- **Type conversion**: Use `instrument_any_to_pyobject()` for Rust→Python, `pyobject_to_instrument_any()` for Python→Rust
- **Rust call**: be carefully about calling rust func. make sure throgh rust support type to rust,not pass a class what is totoally define in python

**Rationale**: Following existing adapter patterns ensures consistency and reduces learning curve. Proper type conversion prevents crashes at language boundaries. Factory pattern enables clean dependency injection and testing.

### XII. Documentation Standards

- **Rust documentation**: Every module, struct, and public method must have `///` doc comments using third-person declarative voice
- **Python documentation**: Comprehensive docstrings for all public classes and methods
- **API documentation**: Maintain integration guide in `docs/integrations/longport.md`
- **Examples**: Provide runnable examples demonstrating data subscription and order execution
- **Changelog**: Document all breaking changes and new features


**Rationale**: Comprehensive documentation is essential for developer onboarding and correct API usage. Runnable examples serve as both documentation and integration tests. Changelog enables tracking of API evolution.

### XIII. Development Workflow

1. Build project: `make clean && make build-debug`
2. Activate venv: `source .venv/bin/activate`
3. Run tests: `./scripts/run_debug.sh`
4. Code review: Focus on architecture alignment, type safety, and performance
5. Documentation updates: Accompany all code changes with documentation
6. Performance benchmarks: Track latency and throughput for all critical paths

**Rationale**: Standardized workflow ensures quality gates are met and prevents integration issues. Code review focuses on high-impact areas. Performance benchmarks prevent regression.

### XIV. Project-Specific Requirements

- **Adapter development**: Strictly follow `docs/developer_guide/adapters.md` specifications
- **Type compatibility in PyO3**: When calling Rust from Python, ensure type compatibility - convert Python-defined types to Rust-defined types when necessary
- **Data catalog**: Use `data_catalog/longport/` for all LongPort market data and trade data
- **Makefile usage**: Always use Makefile commands for building, never direct cargo commands
- **Virtual environment**: All Python operations must use `.venv` virtual environment

**Rationale**: Project-specific requirements capture hard-won lessons from existing adapters. Strict adherence prevents common pitfalls and ensures compatibility with the broader NautilusTrader ecosystem.

### XV. Success Metrics

- Zero data loss in production deployments
- Sub-100ms order execution latency at P99
- 99.9% uptime for WebSocket connections
- Complete compatibility with existing NautilusTrader strategies
- 80%+ test coverage across Rust and Python codebases
- All integration tests passing with mock servers
- Positive developer feedback on API ergonomics
- Full reconciliation accuracy on connect/reconnect

**Rationale**: Quantifiable metrics enable objective assessment of adapter quality. High uptime and low latency are critical for production trading. Test coverage and developer feedback ensure maintainability.

## Non-Negotiable Standards

- Never skip build step: `make clean && make build-debug` is mandatory before testing
- Never fabricate test data: All test data must come from live API or official docs
- Never use bare `tokio::time::sleep()`: Use `wait_until_async` for deterministic tests
- Never call `.into_py_any()` directly: Use `instrument_any_to_pyobject()` for Rust→Python instrument conversion
- Never use `reqwest::Client` directly: Always use `nautilus_network::http::HttpClient`
- Never skip type conversion in PyO3: Always ensure Python types are compatible with Rust expectations

**Rationale**: These standards represent critical safeguards that prevent data corruption, production incidents, flaky tests, and system failures. Violations have directly caused bugs in production systems.

## Governance

### Amendment Procedure

1. Propose changes with clear rationale referencing production incidents or technical requirements
2. Update constitution version according to semantic versioning
3. Review against existing adapter implementations to ensure consistency
4. Update dependent templates (plan, spec, tasks) to reflect new principles
5. Communicate changes to all development teams

### Versioning Policy

- **MAJOR**: Backward incompatible governance/principle removals or redefinitions
- **MINOR**: New principle/section added or materially expanded guidance
- **PATCH**: Clarifications, wording, typo fixes, non-semantic refinements

### Compliance Review

All pull requests MUST verify compliance with applicable principles:
- Architecture changes: Principles I, III, VII, X
- Build system changes: Principle II
- Code changes: Principles IV, V, IX
- Testing changes: Principles V, XV
- Documentation changes: Principle XII
- Python/Rust FFI changes: Principles IV, X, XI

### Complexity Justification

Deviations from principles (especially Non-Negotiable Standards) require explicit justification in:
- Pull request description explaining why deviation is necessary
- Alternative approaches considered and rejected
- Risk mitigation strategies for the deviation
- Plan for eventual compliance restoration

**Version**: 1.0.0 | **Ratified**: 2026-01-03 | **Last Amended**: 2026-01-03
