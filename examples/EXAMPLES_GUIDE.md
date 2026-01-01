# Nautilus Trader Examples Guide

This guide provides a comprehensive overview of all example code in the Nautilus Trader framework, organized by use case and environment.

## Table of Contents

- [Overview](#overview)
- [Environment Contexts](#environment-contexts)
- [Backtest Examples](#backtest-examples)
- [Live Trading Examples](#live-trading-examples)
- [Sandbox Examples](#sandbox-examples)
- [Other Examples](#other-examples)
- [Utilities](#utilities)
- [Common Trading Patterns](#common-trading-patterns)

---

## Overview

Nautilus Trader examples are organized by system environment context:

- **Backtest**: Historical data with simulated venues
- **Sandbox**: Real-time data with simulated venues (testnet environments)
- **Live**: Real-time data with live venues (paper trading or real accounts)
- **Other**: Specialized examples beyond trading strategies

---

## Environment Contexts

### Backtest Environment
Use for strategy development and testing with historical data. All venue operations are simulated with realistic fill models.

### Sandbox Environment
Use for testing strategies in real-time market conditions without risking real capital. Connects to exchange testnets.

### Live Environment
Use for production trading with real accounts. Supports both paper trading and live trading.

---

## Backtest Examples

### Tutorial Series (example_01 - example_11)

A progressive series of examples designed to teach Nautilus Trader fundamentals:

#### [example_01_load_bars_from_custom_csv](backtest/example_01_load_bars_from_custom_csv/)
**Purpose**: Demonstrates loading bar data from custom CSV files

**Key Concepts**:
- Custom data loading from CSV
- Bar data processing
- Basic strategy structure

**Files**: `strategy.py`, `run_example.py`

---

#### [example_02_use_clock_timer](backtest/example_02_use_clock_timer/)
**Purpose**: Shows how to use clock timers for scheduled strategy execution

**Key Concepts**:
- Clock management in backtesting
- Timed events and scheduling
- Strategy lifecycle management

**Files**: `strategy.py`, `run_example.py`

---

#### [example_03_bar_aggregation](backtest/example_03_bar_aggregation/)
**Purpose**: Demonstrates bar aggregation from tick data

**Key Concepts**:
- Tick-to-bar conversion
- Different timeframe aggregations
- Bar type specifications

**Files**: `strategy.py`, `run_example.py`

---

#### [example_04_using_data_catalog](backtest/example_04_using_data_catalog/)
**Purpose**: Uses the data catalog for data management and persistence

**Key Concepts**:
- Data catalog integration
- Data persistence and retrieval
- Querying historical data

**Files**: `strategy.py`, `run_example.py`

---

#### [example_05_using_portfolio](backtest/example_05_using_portfolio/)
**Purpose**: Comprehensive portfolio management example

**Key Concepts**:
- Portfolio tracking and P&L calculation
- Position management
- Bracket orders (entry + take profit + stop loss)

**Files**: `strategy.py`, `run_example.py`

---

#### [example_06_using_cache](backtest/example_06_using_cache/)
**Purpose**: Demonstrates caching for performance optimization

**Key Concepts**:
- Data caching strategies
- Performance optimization
- Memory management

**Files**: `strategy.py`, `run_example.py`

---

#### [example_07_using_indicators](backtest/example_07_using_indicators/)
**Purpose**: Shows technical indicator integration and usage

**Key Concepts**:
- Technical indicators (EMA, RSI, etc.)
- Indicator configuration
- Signal generation

**Files**: `strategy.py`, `run_example.py`

---

#### [example_08_cascaded_indicator](backtest/example_08_cascaded_indicator/)
**Purpose**: Demonstrates cascading indicators (indicators built from other indicators)

**Key Concepts**:
- Indicator composition
- Complex signal generation
- Multi-layer indicators

**Files**: `strategy.py`, `run_example.py`

---

#### [example_09_messaging_with_msgbus](backtest/example_09_messaging_with_msgbus/)
**Purpose**: Message bus integration for inter-component communication

**Key Concepts**:
- Message bus pattern
- Event-driven architecture
- Component communication

**Files**: `strategy.py`, `run_example.py`

---

#### [example_10_messaging_with_actor_data](backtest/example_10_messaging_with_actor_data/)
**Purpose**: Actor-based data processing pipeline

**Key Concepts**:
- Actor model
- Asynchronous data processing
- Actor messaging

**Files**: `strategy.py`, `run_example.py`

---

#### [example_11_messaging_with_actor_signals](backtest/example_11_messaging_with_actor_signals/)
**Purpose**: Actor-based signal processing

**Key Concepts**:
- Actor-based signal generation
- Signal routing
- Async processing

**Files**: `strategy.py`, `run_example.py`

---

### Strategy Examples

#### [fx_market_maker_gbpusd_bars.py](backtest/fx_market_maker_gbpusd_bars.py)
**Purpose**: FX market maker strategy with volatility-based pricing

**Key Concepts**:
- Market making strategy
- Volatility-based pricing (ATR)
- FX rollover interest simulation
- Quote tick data handling

**Strategy**: VolatilityMarketMaker

**Data**: GBP/USD minute bars

---

#### [fx_ema_cross_audusd_bars_from_ticks.py](backtest/fx_ema_cross_audusd_bars_from_ticks.py)
**Purpose**: EMA crossover strategy on AUD/USD, aggregating bars from ticks

**Key Concepts**:
- EMA crossover signals
- Tick-to-bar aggregation
- Trend following strategy

**Strategy**: EMACross

---

#### [fx_ema_cross_audusd_ticks.py](backtest/fx_ema_cross_audusd_ticks.py)
**Purpose**: EMA crossover strategy trading on tick data directly

**Key Concepts**:
- Tick-based trading
- Real-time signal generation
- Order book handling

**Strategy**: EMACross

---

#### [fx_ema_cross_bracket_gbpusd_bars_internal.py](backtest/fx_ema_cross_bracket_gbpusd_bars_internal.py)
**Purpose**: EMA crossover with bracket orders (internal implementation)

**Key Concepts**:
- Bracket orders (entry + TP + SL)
- Risk management
- Position sizing

**Strategy**: EMACrossBracket

---

#### [fx_ema_cross_bracket_gbpusd_bars_external.py](backtest/fx_ema_cross_bracket_gbpusd_bars_external.py)
**Purpose**: EMA crossover with bracket orders using external algo execution

**Key Concepts**:
- External algorithm execution
- TWAP (Time-Weighted Average Price)
- Order routing

**Strategy**: EMACrossBracket

---

### Cryptocurrency Examples

#### [crypto_ema_cross_ethusdt_trade_ticks.py](backtest/crypto_ema_cross_ethusdt_trade_ticks.py)
**Purpose**: EMA crossover on ETH/USDT trade ticks

**Key Concepts**:
- Trade tick analysis
- Cryptocurrency markets
- EMA signals

**Strategy**: EMACross

---

#### [crypto_ema_cross_ethusdt_trailing_stop.py](backtest/crypto_ema_cross_ethusdt_trailing_stop.py)
**Purpose**: EMA crossover with trailing stop loss

**Key Concepts**:
- Trailing stop implementation
- Dynamic exit management
- Trend following with protection

**Strategy**: EMACrossTrailingStop

---

#### [crypto_ema_cross_with_binance_provider.py](backtest/crypto_ema_cross_with_binance_provider.py)
**Purpose**: EMA strategy using Binance data provider

**Key Concepts**:
- Binance integration
- Live instrument data loading
- USDT Futures markets

**Strategy**: EMACrossTrailingStop

---

#### [crypto_orderbook_imbalance.py](backtest/crypto_orderbook_imbalance.py)
**Purpose**: Order book imbalance strategy for crypto markets

**Key Concepts**:
- Order book analysis
- Imbalance signals
- Mean reversion

**Strategy**: OrderBookImbalance

---

### Databento Examples

#### [databento_cme_quoter.py](backtest/databento_cme_quoter.py)
**Purpose**: Simple quoter strategy using Databento data for CME futures

**Key Concepts**:
- Databento data loading
- Market making on futures
- ES futures (E-mini S&P 500)

**Strategy**: SimpleQuoterStrategy

**Data**: Databento DBN files (GLBX MDP3)

---

#### [databento_ema_cross_long_only_aapl_bars.py](backtest/databento_ema_cross_long_only_aapl_bars.py)
**Purpose**: Long-only EMA crossover on AAPL bars

**Key Concepts**:
- Long-only trading
- US equities data
- Trend following

**Strategy**: EMACrossLongOnly

**Data**: Databento

---

#### [databento_ema_cross_long_only_spy_trades.py](backtest/databento_ema_cross_long_only_spy_trades.py)
**Purpose**: Long-only EMA crossover on SPY trade ticks

**Key Concepts**:
- ETF trading
- Trade tick analysis
- Long-only signals

**Strategy**: EMACrossLongOnly

**Data**: Databento

---

#### [databento_ema_cross_long_only_tsla_trades.py](backtest/databento_ema_cross_long_only_tsla_trades.py)
**Purpose**: Long-only EMA crossover on TSLA trade ticks

**Key Concepts**:
- High volatility stock trading
- Trade tick analysis

**Strategy**: EMACrossLongOnly

**Data**: Databento

---

### Prediction Market Examples

#### [polymarket_simple_quoter.py](backtest/polymarket_simple_quoter.py)
**Purpose**: Simple quoter for Polymarket binary options

**Key Concepts**:
- Prediction market trading
- Binary options
- Order book imbalance strategy
- Polymarket API integration

**Data Sources**:
- Markets API: https://gamma-api.polymarket.com/markets
- Order book history: https://clob.polymarket.com/orderbook-history
- Trades/Prices: https://clob.polymarket.com/prices-history

**Strategy**: OrderBookImbalance

---

#### [betfair_backtest_orderbook_imbalance.py](backtest/betfair_backtest_orderbook_imbalance.py)
**Purpose**: Order book imbalance strategy for Betfair betting markets

**Key Concepts**:
- Sports betting markets
- Order book analysis
- Betting exchange specifics

**Strategy**: OrderBookImbalance

---

### Utility Examples

#### [model_configs_example.py](backtest/model_configs_example.py)
**Purpose**: Demonstrates model configuration patterns

**Key Concepts**:
- Model configuration
- Parameter management
- Config validation

---

#### [synthetic_data_pnl_test.py](backtest/synthetic_data_pnl_test.py)
**Purpose**: P&L calculation testing with synthetic data

**Key Concepts**:
- Synthetic data generation
- P&L verification
- Portfolio accounting

---

### Notebooks

The [backtest/notebooks](backtest/notebooks/) directory contains Jupyter notebook examples:

#### [databento_download.py](backtest/notebooks/databento_download.py)
**Purpose**: Download data from Databento

---

#### [databento_backtest_with_data_client.py](backtest/notebooks/databento_backtest_with_data_client.py)
**Purpose**: Backtesting using Databento data client

---

#### [databento_test_request_bars.py](backtest/notebooks/databento_test_request_bars.py)
**Purpose**: Testing bar requests from Databento

---

#### [databento_option_greeks.py](backtest/notebooks/databento_option_greeks.py)
**Purpose**: Options Greeks calculation with Databento data

---

## Live Trading Examples

### Binance

#### [binance_spot_ema_cross.py](live/binance/binance_spot_ema_cross.py)
**Purpose**: Basic EMA crossover on Binance spot market

**Key Concepts**:
- Live trading setup
- Spot market orders
- EMA strategy

**Venue**: Binance Spot

---

#### [binance_futures_testnet_ema_cross.py](live/binance/binance_futures_testnet_ema_cross.py)
**Purpose**: EMA strategy on Binance futures testnet

**Key Concepts**:
- Futures trading
- Testnet environment
- Perpetual contracts

**Venue**: Binance Futures Testnet

---

#### [binance_spot_and_futures_market_maker.py](live/binance/binance_spot_and_futures_market_maker.py)
**Purpose**: Market maker trading both spot and futures simultaneously

**Key Concepts**:
- Multi-venue trading
- Basis trading
- Dual market making

**Venue**: Binance (Spot + Futures)

---

#### [binance_spot_ema_cross_bracket_algo.py](live/binance/binance_spot_ema_cross_bracket_algo.py)
**Purpose**: EMA strategy with bracket orders and algorithm execution

**Key Concepts**:
- Bracket orders
- Algorithm execution
- Risk management

**Venue**: Binance Spot

---

#### [binance_spot_exec_tester.py](live/binance/binance_spot_exec_tester.py)
**Purpose**: Execution testing utilities for Binance spot

**Key Concepts**:
- Order testing
- Execution verification
- Development tools

---

#### [binance_spot_testnet_exec_tester.py](live/binance/binance_spot_testnet_exec_tester.py)
**Purpose**: Testnet execution testing

**Key Concepts**:
- Testnet testing
- Order validation

---

#### [binance_futures_testnet_exec_tester.py](live/binance/binance_futures_testnet_exec_tester.py)
**Purpose**: Futures testnet execution testing

---

#### [binance_data_tester.py](live/binance/binance_data_tester.py)
**Purpose**: Market data testing for Binance

---

### Bybit

#### [bybit_ema_cross.py](live/bybit/bybit_ema_cross.py)
**Purpose**: Basic EMA crossover strategy on Bybit

**Key Concepts**:
- Derivatives trading
- EMA signals
- Bybit API

**Venue**: Bybit Derivatives

---

#### [bybit_ema_cross_bracket_algo.py](live/bybit/bybit_ema_cross_bracket_algo.py)
**Purpose**: EMA strategy with bracket orders and TWAP execution

**Key Concepts**:
- Bracket orders
- TWAP algorithm
- Advanced execution

**Venue**: Bybit

---

#### [bybit_ema_cross_with_trailing_stop.py](live/bybit/bybit_ema_cross_with_trailing_stop.py)
**Purpose**: EMA strategy with trailing stop loss

**Key Concepts**:
- Trailing stops
- Dynamic exit management

**Venue**: Bybit

---

#### [bybit_ema_cross_stop_entry.py](live/bybit/bybit/bybit_ema_cross_stop_entry.py)
**Purpose**: EMA strategy with stop entry orders

**Key Concepts**:
- Stop entry orders
- Breakout trading

**Venue**: Bybit

---

#### [bybit_exec_tester.py](live/bybit/bybit_exec_tester.py)
**Purpose**: Execution testing for Bybit

---

#### [bybit_data_tester.py](live/bybit/bybit_data_tester.py)
**Purpose**: Data testing for Bybit

---

#### [bybit_options_data_collector.py](live/bybit/bybit_options_data_collector.py)
**Purpose**: Collect options market data from Bybit

**Key Concepts**:
- Options data collection
- Volatility surface
- See [README_options_data_collector.md](live/bybit/README_options_data_collector.md)

**Venue**: Bybit Options

---

#### [bybit_request_custom_endpoint.py](live/bybit/bybit_request_custom_endpoint.py)
**Purpose**: Custom API endpoint requests

**Key Concepts**:
- Custom API integration
- Advanced endpoints

---

### Interactive Brokers

#### [connect_with_tws.py](live/interactive_brokers/connect_with_tws.py)
**Purpose**: Connect to Interactive Brokers via TWS (Trader Workstation)

**Key Concepts**:
- IB TWS connection
- Institutional trading
- Stock market integration

**Venue**: Interactive Brokers

---

#### [connect_with_dockerized_gateway.py](live/interactive_brokers/connect_with_dockerized_gateway.py)
**Purpose**: Connect via Dockerized IB Gateway

**Key Concepts**:
- Docker deployment
- Gateway connection
- Production setup

**Venue**: Interactive Brokers

---

#### [with_databento_instrument_id_example.py](live/interactive_brokers/with_databento_instrument_id_example.py)
**Purpose**: Using Databento instrument IDs with IB data

**Key Concepts**:
- Multi-provider integration
- Instrument mapping

**Venue**: Interactive Brokers + Databento

---

#### [historical_download.py](live/interactive_brokers/historical_download.py)
**Purpose**: Download historical data from IB

**Key Concepts**:
- Historical data retrieval
- Data management

**Venue**: Interactive Brokers

---

#### [contract_download.py](live/interactive_brokers/contract_download.py)
**Purpose**: Download contract details from IB

**Key Concepts**:
- Contract metadata
- Instrument discovery

**Venue**: Interactive Brokers

---

### Interactive Brokers Notebooks

#### [bracket_order_example.py](live/interactive_brokers/notebooks/bracket_order_example.py)
**Purpose**: Bracket order examples for IB

**Key Concepts**:
- Bracket orders
- Parent-child orders
- Risk management

---

#### [oca_group_example.py](live/interactive_brokers/notebooks/oca_group_example.py)
**Purpose**: OCA (One-Cancels-All) group examples

**Key Concepts**:
- OCA groups
- Order linkage
- Conditional orders

---

#### [simple_conditions_example.py](live/interactive_brokers/notebooks/simple_conditions_example.py)
**Purpose**: Order conditions and triggers

**Key Concepts**:
- Order conditions
- Price triggers
- Time conditions

---

#### [spread_example.py](live/interactive_brokers/notebooks/spread_example.py)
**Purpose**: Spread trading examples

**Key Concepts**:
- Spread orders
- Multi-leg strategies
- Basis trading

---

### dYdX

#### [dydx_v4_market_maker.py](live/dydx/dydx_v4_market_maker.py)
**Purpose**: Market maker on dYdX v4 using Rust-backed adapter

**Key Concepts**:
- Decentralized trading
- Market making
- Rust-backed HTTP/WS/gRPC clients
- Perpetual futures

**Prerequisites**:
- Environment variables: `DYDX_WALLET_ADDRESS`, `DYDX_MNEMONIC` (or testnet variants)

**Venue**: dYdX v4

---

#### [dydx_market_maker.py](live/dydx/dydx_market_maker.py)
**Purpose**: Market maker on dYdX v3

**Venue**: dYdX v3

---

#### [dydx_v4_exec_tester.py](live/dydx/dydx_v4_exec_tester.py)
**Purpose**: Execution testing for dYdX v4

**Venue**: dYdX v4

---

#### [dydx_v4_data_tester.py](live/dydx/dydx_v4_data_tester.py)
**Purpose**: Data testing for dYdX v4

**Venue**: dYdX v4

---

### Other Exchanges

#### Kraken ([live/kraken](live/kraken))
- EMA crossover strategies
- Execution testers

#### OKX ([live/okx](live/okx))
- EMA crossover strategies
- Market data testing

#### Deribit ([live/deribit](live/deribit))
- Options trading examples

#### BitMEX ([live/bitmex](live/bitmex))
- Perpetual futures strategies

#### Hyperliquid ([live/hyperliquid](live/hyperliquid))
- DEX trading examples

#### Coinbase INTX ([live/coinbase_intx](live/coinbase_intx))
- Institutional trading

#### Tardis ([live/tardis](live/tardis))
- Historical data examples

#### Polymarket ([live/polymarket](live/polymarket))
- Prediction market trading

#### Betfair ([live/betfair](live/betfair))
- Sports betting markets

#### Databento ([live/databento](live/databento))
- Institutional data integration

---

## Sandbox Examples

Sandbox examples use real-time market data from exchange testnets with simulated execution.

### [binance_spot_futures_sandbox.py](sandbox/binance_spot_futures_sandbox.py)
**Purpose**: Multi-account sandbox (spot + futures)

**Key Concepts**:
- Sandbox testing
- Multi-account management
- Simulated trading
- Real-time market data

**Venue**: Binance Testnet

---

### [binance_futures_testnet_sandbox.py](sandbox/binance_futures_testnet_sandbox.py)
**Purpose**: Futures testnet sandbox

**Venue**: Binance Futures Testnet

---

### [bybit_sandbox.py](sandbox/bybit_sandbox.py)
**Purpose**: Bybit testnet simulation

**Venue**: Bybit Testnet

---

### [interactive_brokers_sandbox.py](sandbox/interactive_brokers_sandbox.py)
**Purpose**: IB simulation environment

**Venue**: Interactive Brokers (Paper Trading)

---

### [dydx_sandbox.py](sandbox/dydx_sandbox.py)
**Purpose**: dYdX testnet simulation

**Venue**: dYdX Testnet

---

### [hyperliquid_testnet_sandbox.py](sandbox/hyperliquid_testnet_sandbox.py)
**Purpose**: Hyperliquid testnet

**Venue**: Hyperliquid Testnet

---

### [databento_cme_sandbox.py](sandbox/databento_cme_sandbox.py)
**Purpose**: Databento CME data sandbox

**Data**: Databento

---

### [betfair_sandbox.py](sandbox/betfair_sandbox.py)
**Purpose**: Betfair simulation

**Venue**: Betfair

---

## Other Examples

### [minimal_reproducible_example](other/minimal_reproducible_example/)
**Purpose**: Minimal strategy template for quick testing

**Key Concepts**:
- Basic strategy structure
- Order execution
- Minimal dependencies

**Files**: `strategy.py`, `run_example.py`

---

### [state_machine](other/state_machine/)
**Purpose**: State machine implementation example

**Key Concepts**:
- State management
- Strategy states
- Event-driven transitions

**Files**: `strategy.py`, `run_example.py`

---

### [debugging](other/debugging/)
**Purpose**: Debugging utilities and examples

**Files**:
- `debug_mixed_jupyter.ipynb`: Jupyter debugging notebook

---

## Utilities

### [data_provider.py](utils/data_provider.py)
**Purpose**: Data preparation utilities

**Key Concepts**:
- Data wrangling
- CSV processing
- Bar data preparation
- Test data generation

---

## Common Trading Patterns

### Trend Following
- EMA Crossover strategies (multiple examples)
- Long-only variants
- Trailing stops

### Mean Reversion
- Order book imbalance strategies
- Volatility-based entries

### Market Making
- Volatility market maker
- Simple quoter
- Multi-venue market making

### Risk Management
- Bracket orders (entry + TP + SL)
- Trailing stops
- OCA groups
- Position sizing

### Execution Algorithms
- TWAP (Time-Weighted Average Price)
- Market orders
- Limit orders
- Stop orders

---

## Running Examples

From the `examples` directory:

```bash
# Backtest examples
python backtest/fx_ema_cross_audusd_bars_from_ticks.py

# Live examples (ensure proper configuration)
python live/binance/binance_spot_ema_cross.py

# Sandbox examples
python sandbox/binance_spot_futures_sandbox.py
```

---

## Prerequisites

1. Install Nautilus Trader:
   ```bash
   pip install nautilus_trader
   ```
   Or compile from source. See the [installation guide](https://nautilustrader.io/docs/latest/getting_started/installation).

2. Set up required API credentials for live trading (stored in environment variables or config files)

3. For exchange-specific examples, refer to the adapter documentation

---

## Related Documentation

- [Main README](README.md)
- [Adapter Documentation](https://nautilustrader.io/docs/latest/adapters/index)
- [Strategy Development](https://nautilustrader.io/docs/latest/strategies/)
- [Backtesting Guide](https://nautilustrader.io/docs/latest/backtesting/)
