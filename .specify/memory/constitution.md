<!--
Sync Impact Report
==================
Version change: INITIAL → 1.0.0
Constitution established for NautilusTrader extension development.

Modified principles: N/A (initial version)

Added sections:
  - Core Principles (5 principles)
  - Development Standards
  - Adapter Architecture
  - Governance

Removed sections: N/A (initial version)

Templates requiring updates:
  - .specify/templates/plan-template.md (Constitution Check section updated)
  - .specify/templates/spec-template.md (requirements alignment verified)
  - .specify/templates/tasks-template.md (task categorization verified)

Follow-up TODOs: None
-->

# NautilusTrader Development Constitution

## Core Principles

### I. Adapter Modularity

Every trading venue, broker, and data provider integration MUST be implemented as a self-contained adapter following the established template pattern.

- Adapters MUST be located in `nautilus_trader/adapters/[adapter_name]/` (Python) and `crates/adapters/[adapter_name]/` (Rust)
- Each adapter MUST implement the core adapter interfaces: `DataClient`, `ExecutionClient`, and supporting factories
- Adapters MUST be independently testable in isolation from the core system
- All adapters MUST expose a common configuration pattern via `config.py`
- Adapter-specific implementations MUST NOT leak into core trading logic

**Rationale**: The modular adapter architecture is fundamental to NautilusTrader's design. It enables:
- Universal integration with any REST/WebSocket API
- Clear separation of concerns between venue-specific APIs and the unified domain model
- Independent testing and maintenance of each integration
- Easy addition of new venues and data providers without modifying core code

### II. Code Reuse and Composability

Extensions MUST maximize reuse of existing NautilusTrader components, patterns, and infrastructure.

- New features MUST leverage existing message bus, cache, and actor infrastructure
- Data types MUST use core value types (`Price`, `Quantity`, `Money`, `Instrument`) from `nautilus_core`
- Order management MUST use the existing `Order`, `OrderList`, and related structures
- Execution logic MUST follow the established `ExecutionClient` and `msgbus` patterns
- Custom data types MUST extend the base `Data` class hierarchy

**Rationale**: NautilusTrader provides a rich set of core abstractions. Reusing these ensures:
- Consistency across the codebase
- Reduced maintenance burden
- Proper integration with the event-driven architecture
- Type safety and performance from Rust core components

### III. Hybrid Language Architecture

The platform is built on a Rust + Python hybrid architecture. Extensions MUST respect this design.

- Performance-critical code (networking, data parsing, core operations) MUST be implemented in Rust
- Python layer provides configuration, orchestration, and user-facing APIs
- All Rust components MUST expose Python bindings via PyO3
- Python and Rust code MUST be version-locked and tested together
- Build system MUST handle both Rust compilation and Python packaging

**Rationale**: This hybrid approach delivers:
- C-level performance for hot paths (Rust)
- Python-native development experience for strategies and configuration
- Memory safety and thread safety from Rust ownership model
- Flexibility to extend in either language as appropriate

### IV. Backtest-Live Parity

Implementations MUST maintain identical behavior between backtesting and live trading environments.

- Strategies MUST run without code changes between backtest and live
- Data ingestion MUST use the same code paths regardless of data source
- Order management MUST preserve semantic equivalence across environments
- Timing and event handling MUST be consistent
- Configuration differences between backtest and live MUST be explicit and minimal

**Rationale**: Parity eliminates a critical class of production bugs by ensuring:
- Backtest results accurately reflect live behavior
- Strategy development cycle is fast (no reimplementation)
- Risk is reduced through validated code paths
- Confidence in deployments is higher

### V. Observability and Reliability

All adapters and core components MUST provide comprehensive observability and error handling.

- All external API calls MUST be logged with request/response details
- Errors MUST be propagated through the message bus with context
- WebSocket connections MUST have automatic reconnection with exponential backoff
- Data feeds MUST validate incoming messages and log anomalies
- Every component MUST handle disconnection gracefully without data loss
- Critical operations MUST emit metrics (latency, message counts, error rates)

**Rationale**: In production trading, visibility and reliability are non-negotiable:
- Rapid debugging of production issues
- Detection of data quality problems
- Monitoring of system health
- Audit trails for compliance and analysis

## Development Standards

### Testing Requirements

- Unit tests MUST cover all adapter-specific logic
- Integration tests MUST verify correct behavior with test environments (sandbox where available)
- Contract tests MUST validate adherence to NautilusTrader adapter interfaces
- All new adapters MUST include example configuration and documentation
- Performance tests MUST validate that the adapter meets latency requirements

### Code Organization

- Python code MUST follow the project's `ruff` configuration for formatting and linting
- Rust code MUST use `cargo-clippy` and follow standard Rust conventions
- Type annotations MUST be complete on all public Python APIs
- Documentation strings MUST be provided for all public functions and classes
- Examples MUST be provided for adapter configuration and usage

### Versioning and Compatibility

- Adapters MUST follow semantic versioning (MAJOR.MINOR.PATCH)
- Breaking changes MUST increment MAJOR version and be documented in release notes
- New features MUST increment MINOR version
- Bug fixes MUST increment PATCH version
- All changes MUST preserve backward compatibility where possible
- must use makefile to build the project

## Adapter Architecture

### Adapter Components

Each adapter MUST provide:

1. **Configuration** (`config.py` / `config.rs`)
   - Venue-specific configuration options
   - Authentication credentials structure
   - Connection parameters
   - Feature flags

2. **Factories** (`factories.py`)
   - Instrument creation and parsing
   - Venue-specific data type conversions
   - Account ID and client ID generation

3. **Data Client** (`data.py` / Rust implementation)
   - REST API integration for historical data
   - WebSocket integration for live feeds
   - Data normalization to NautilusTrader types
   - Subscription management

4. **Execution Client** (`execution.py` / Rust implementation)
   - Order submission, modification, cancellation
   - Account state synchronization
   - Position and balance updates
   - Trade execution reporting

5. **HTTP Client** (`http/client.py`)
   - Rate-limited REST API wrapper
   - Authentication handling
   - Error parsing and retry logic

6. **WebSocket Client** (`websocket/client.py`)
   - WebSocket connection management
   - Heartbeat and ping/pong handling
   - Subscription channel management
   - Message parsing and dispatch

### Rust Backend Requirements

- All networking MUST use `tokio` for async I/O
- WebSocket connections MUST use `tokio-tungstenite`
- HTTP clients MUST use a maintainable async HTTP library
- Critical data structures MUST be defined in Rust and exposed via PyO3
- Performance-sensitive parsing MUST be implemented in Rust

## Extension Guidelines

### Adding New Adapters

When adding a new trading venue or data provider:

1. **RFC Required**: Open a Request for Comments issue per [ROADMAP.md](/ROADMAP.md)
2. **Template Pattern**: Start from `nautilus_trader/adapters/_template/` structure
3. **Rust Core**: Implement performance-critical components in `crates/adapters/[adapter]/`
4. **Python Interface**: Create user-facing Python API with PyO3 bindings
5. **Documentation**: Include integration guide in `docs/integrations/[adapter].md`
6. **Examples**: Provide example configuration and scripts in `examples/`
7. **Tests**: Comprehensive unit and integration tests required

### Adding Data Recording Capabilities

When extending data recording and monitoring:

1. **Use Existing Infrastructure**: Leverage the cache, message bus, and logging systems
2. **No UI Dashboards**: Per roadmap, focus on engine capabilities, not frontends
3. **Data Format Compliance**: Use existing NautilusTrader data formats and schemas
4. **Storage Abstraction**: Support pluggable storage backends (files, databases)
5. **Monitoring Integrate**: Leverage existing observability patterns

### Monitoring and Metrics Extensions

When extending monitoring capabilities:

1. **Message Bus Integration**: Subscribe to relevant message bus channels
2. **Non-Invasive**: Monitoring MUST NOT impact trading performance
3. **Configurable**: All monitoring MUST be toggleable via configuration
4. **Standard Formats**: Emit metrics in standard formats (Prometheus, StatsD, or structured logs)

## Governance

### Constitution Authority

This constitution governs all extension development for NautilusTrader. It supersedes conflicting local practices. All pull requests for new features, adapters, or extensions MUST verify compliance with these principles.

### Amendment Process

1. Proposals for amendments MUST be submitted as GitHub issues with rationale
2. Amendments require discussion among maintainers and approval
3. Minor clarifications may be made without bumping version (PATCH)
4. Principle additions or changes require MINOR version bump
5. Principle removals or backward-incompatible changes require MAJOR version bump

### Compliance Verification

- Code review MUST check constitution compliance
- New adapters MUST pass the "Constitution Check" in the implementation plan template
- CI/CD pipelines SHOULD validate architectural patterns where possible
- Complexity violations MUST be justified with documented rationale

### Complexity Justification

If a principle must be violated:

1. Document why the principle cannot be followed
2. Explain why simpler alternatives are insufficient
3. Provide evidence that the tradeoff is necessary
4. Maintainers MUST review and approve the violation

**Version**: 1.0.0 | **Ratified**: 2025-12-30 | **Last Amended**: 2025-12-30
