# NautilusTrader Longport Adapter

[NautilusTrader](http://nautilustrader.io) adapter for [Longport](https://longport.com) securities trading.

The `nautilus-longport` crate provides client bindings, data models and helper utilities that wrap the official Longport OpenAPI.

## Features

- **Multi-market support**: Hong Kong (HK) and US stock markets
- **Market data**: Real-time quotes, trades, order book, and bars via WebSocket
- **Trading**: Full order lifecycle management with comprehensive order type support
- **Account management**: Real-time account state and balance updates
- **High performance**: Built on top of Longport Rust SDK 3.0.8

## Supported Markets

- **Hong Kong (HK)**: Stocks, ETFs, Warrants
- **US**: Stocks, ETFs

## Configuration

### Environment Variables

```bash
export LONGPORT_APP_KEY="your_app_key"
export LONGPORT_APP_SECRET="your_app_secret"
export LONGPORT_ACCESS_TOKEN="your_access_token"
```

### Or via configuration

```python
from nautilus_trader.adapters.longport import LONGPORT, LongportMarket
from nautilus_trader.adapters.longport import LongportDataClientConfig, LongportExecClientConfig
from nautilus_trader.config import TradingNodeConfig

config_node = TradingNodeConfig(
    data_clients={
        LONGPORT: LongportDataClientConfig(
            app_key="your_app_key",
            app_secret="your_app_secret",
            access_token="your_access_token",
            markets=[LongportMarket::HK, LongportMarket::US],
        ),
    },
    exec_clients={
        LONGPORT: LongportExecClientConfig(
            trader_id=TraderId("TRADER-001"),
            account_id=AccountId("LONGPORT-001"),
            app_key="your_app_key",
            app_secret="your_app_secret",
            access_token="your_access_token",
            markets=[LongportMarket::HK, LongportMarket::US],
        ),
    },
)
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
nautilus-longport = { version = "*", features = ["live"] }
```

## License

Licensed under the GNU Lesser General Public License v3.0 (LGPL-3.0). See the [LICENSE](LICENSE) file for details.
