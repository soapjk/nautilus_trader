# NautilusTrader 期权回测数据指南

NautilusTrader 完整支持期权数据的回测，包括：
- 期权合约 (OptionContract)
- 期权报价 (QuoteTick)
- 期权行权 (OptionExerciseModule)
- Greeks 数据
- 期权策略 (OptionSpread)

## 期权数据类型

### 1. 期权合约定义

```python
from nautilus_trader.model.instruments import OptionContract
from nautilus_trader.model.enums import OptionKind, AssetClass
from nautilus_trader.core.datetime import dt_to_unix_nanos
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.currencies import USD
import pandas as pd

# 创建期权合约
option = OptionContract(
    instrument_id=InstrumentId(
        symbol=Symbol("AAPL240315C00150000"),
        venue=Venue("NASDAQ"),
    ),
    raw_symbol=Symbol("AAPL240315C00150000"),
    asset_class=AssetClass.EQUITY,
    underlying="AAPL",  # 标的资产
    option_kind=OptionKind.CALL,  # CALL 或 PUT
    strike_price=Price("150.00", precision=2),  # 行权价
    currency=USD,
    activation_ns=dt_to_unix_nanos(pd.Timestamp("2024-03-01", tz="UTC")),
    expiration_ns=dt_to_unix_nanos(pd.Timestamp("2024-03-15 16:00:00", tz="UTC")),
    price_precision=2,
    price_increment=Price("0.01", precision=2),
    multiplier=Quantity.from_int(100),  # 期权乘数 (每份期权代表100股)
    lot_size=Quantity.from_int(1),
)
```

### 2. 期权数据格式

期权的数据格式与标准数据相同，主要使用：

#### QuoteTick (期权报价)
```csv
timestamp,bid_price,ask_price,bid_size,ask_size
2024-03-01 09:30:00,5.25,5.30,100,150
```

#### Bars (期权K线)
```csv
timestamp,open,high,low,close,volume
2024-03-01 09:30:00,5.20,5.35,5.15,5.30,5000
```

### 3. Greeks 数据

NautilusTrader 支持 Greeks 数据用于高级期权分析：

```python
from nautilus_trader.model.greeks_data import GreeksData

greeks_data = GreeksData(
    timestamp_ns=timestamp_ns,
    delta=0.50,
    gamma=0.05,
    theta=-0.02,
    vega=0.15,
    rho=0.01,
)
```

## 期权回测配置

### 添加期权行权模块

```python
from nautilus_trader.backtest.option_exercise import OptionExerciseConfig, OptionExerciseModule
from nautilus_trader.backtest.modules import OptionExerciseModule

# 配置行权模块
exercise_config = OptionExerciseConfig(
    autoexercise=True,  # 自动行权价内期权
    check_margin=True,  # 检查保证金要求
)

exercise_module = OptionExerciseModule(config=exercise_config)

# 添加到交易所
engine.add_venue(
    venue=Venue("CME"),
    modules=[exercise_module],  # 添加行权模块
)
```

### 完整期权回测示例

```python
from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.backtest.config import BacktestEngineConfig
from nautilus_trader.backtest.option_exercise import OptionExerciseModule
from nautilus_trader.model.instruments import OptionContract, Equity
from nautilus_trader.model.currencies import USD
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.enums import OptionKind, AssetClass, AccountType, OmsType
from nautilus_trader.persistence.wranglers import BarDataWrangler
from nautilus_trader.model.data import BarType

# 配置引擎
config = BacktestEngineConfig(trader_id="BACKTESTER-001")
engine = BacktestEngine(config=config)

# 创建标的资产
underlying = Equity(
    instrument_id=InstrumentId(Symbol("AAPL"), Venue("NASDAQ")),
    raw_symbol=Symbol("AAPL"),
    currency=USD,
    price_precision=2,
    price_increment=Price("0.01", precision=2),
)

# 创建期权合约
call_option = OptionContract(
    instrument_id=InstrumentId(Symbol("AAPL240315C00150000"), Venue("NASDAQ")),
    raw_symbol=Symbol("AAPL240315C00150000"),
    asset_class=AssetClass.EQUITY,
    underlying="AAPL",
    option_kind=OptionKind.CALL,
    strike_price=Price("150.00", precision=2),
    currency=USD,
    activation_ns=dt_to_unix_nanos(pd.Timestamp("2024-03-01", tz="UTC")),
    expiration_ns=dt_to_unix_nanos(pd.Timestamp("2024-03-15", tz="UTC")),
    price_precision=2,
    price_increment=Price("0.01", precision=2),
    multiplier=Quantity.from_int(100),
    lot_size=Quantity.from_int(1),
)

# 添加行权模块
exercise_module = OptionExerciseModule(
    config=OptionExerciseConfig(autoexercise=True)
)

# 添加交易所
engine.add_venue(
    venue=Venue("NASDAQ"),
    oms_type=OmsType.HEDGING,
    account_type=AccountType.CASH,
    base_currency=USD,
    starting_balances=[USD(100_000)],
    modules=[exercise_module],  # 启用期权行权
)

# 添加标的资产和期权
engine.add_instrument(underlying)
engine.add_instrument(call_option)

# 添加数据
underlying_bars = wrangler.process(underlying_df)
option_bars = option_wrangler.process(option_df)
engine.add_data(underlying_bars)
engine.add_data(option_bars)

# 运行回测
engine.run()
```

## 期权策略数据

### 期权价差 (Option Spread)

```python
from nautilus_trader.model.instruments import OptionSpread
from nautilus_trader.model.identifiers import new_generic_spread_id

# 创建价差组合
spread_id = new_generic_spread_id([
    (call_option_id, -1),  # 卖出看涨期权
    (put_option_id, 1),    # 买入看跌期权
])

# 价差策略示例
# - Straddle: 同一行权价的 Call + Put
# - Strangle: 不同行权价的 OTM Call + OTM Put
# - Vertical Spread: 同一类型，不同行权价
# - Calendar Spread: 不同到期日
# - Iron Condor: 组合策略
```

## 数据来源

### 1. Databento 期权数据

NautilusTrader 内置了 Databento 适配器，支持期权数据：

```python
from nautilus_trader.adapters.databento.loaders import DatabentoDataLoader

loader = DatabentoDataLoader()
option_data = loader.load(
    dataset="OPTIONS",
    schema="bbo-1m",  # Best Bid Offer
    symbols=["ESM4 P5230", "ESM4 P5250"],
    start="2024-05-09",
    end="2024-05-10",
)
```

### 2. CSV 期权数据转换

如果你的期权数据是 CSV 格式：

```python
import pandas as pd
from nautilus_trader.persistence.wranglers import BarDataWrangler
from nautilus_trader.model.data import BarType

# 读取期权报价数据
df = pd.read_csv("option_quotes.csv", index_col="timestamp", parse_dates=True)

# 确保 UTC 时区
df.index = df.index.tz_localize("UTC")

# 创建 wrangler
bar_type = BarType.from_str("AAPL240315C00150000.NASDAQ-1-MINUTE-BID-EXTERNAL")
wrangler = BarDataWrangler(bar_type=bar_type, instrument=call_option)

# 处理数据
option_bars = wrangler.process(df)
```

## 期权数据 CSV 格式

### 期权报价数据

```csv
timestamp,bid_price,ask_price,bid_size,ask_size,implied_volatility,delta,gamma,theta,vega
2024-03-01 09:30:00,5.25,5.30,100,150,0.25,0.52,0.05,-0.02,0.15
2024-03-01 09:31:00,5.28,5.32,120,130,0.25,0.53,0.05,-0.02,0.15
```

**必需列**: `timestamp`, `bid_price`, `ask_price`
**可选列**: `bid_size`, `ask_size`, Greeks 指标

### 期权成交数据

```csv
timestamp,price,quantity,side,trade_id,open_interest
2024-03-01 09:30:05,5.30,10,BUY,12345,5000
2024-03-01 09:30:10,5.25,5,SELL,12346,5000
```

**必需列**: `timestamp`, `price`, `quantity`
**可选列**: `side`, `trade_id`, `open_interest`

## 特殊注意事项

### 1. 期权乘数 (Multiplier)

期权合约的乘数很重要，通常：
- **股票期权**: 100 (每份期权代表100股)
- **指数期权**: 根据指数而定
- **期货期权**: 根据合约规格

```python
# 设置正确的乘数
option = OptionContract(
    ...,
    multiplier=Quantity.from_int(100),  # 股票期权
)
```

### 2. 行权价精度

确保行权价精度与价格精度匹配：

```python
option = OptionContract(
    ...,
    strike_price=Price("150.00", precision=2),  # 精度为2
    price_precision=2,  # 价格精度也是2
    price_increment=Price("0.01", precision=2),
)
```

### 3. 到期时间

期权的到期时间处理很重要：

```python
from nautilus_trader.core.datetime import dt_to_unix_nanos
import pandas as pd

# 美式期权通常在到期日第三个周五收盘
expiration = pd.Timestamp("2024-03-15 16:00:00", tz="America/New_York")
expiration_utc = expiration.tz_convert("UTC")
expiration_ns = dt_to_unix_nanos(expiration_utc)
```

## 期权 Greeks 计算

如果需要计算 Greeks：

```python
from nautilus_trader.analysis.greeks import calculate_greeks
from nautilus_trader.model.enums import OptionKind

# 计算 Greeks
greeks = calculate_greeks(
    option_kind=OptionKind.CALL,
    underlying_price=152.50,
    strike_price=150.00,
    time_to_expiry=15.0,  # 天
    risk_free_rate=0.05,
    volatility=0.25,
)
```

## 完整示例：期权策略回测

参见示例文件：
- `examples/backtest/notebooks/databento_option_greeks.py`
- `tests/unit_tests/backtest/test_option_exercise.py`

## 总结

期权数据回测要点：

1. ✅ **期权合约**: 使用 `OptionContract` 定义
2. ✅ **数据格式**: QuoteTick 或 Bars，与普通数据相同
3. ✅ **行权模块**: 使用 `OptionExerciseModule` 处理到期行权
4. ✅ **Greeks**: 支持 Greeks 数据和计算
5. ✅ **期权策略**: 支持价差等组合策略
6. ✅ **数据来源**: Databento、CSV 转换等

与普通数据的主要区别：
- 需要 OptionContract 定义期权参数
- 可能需要行权模块
- 需要考虑期权乘数
- 到期时间处理更重要
