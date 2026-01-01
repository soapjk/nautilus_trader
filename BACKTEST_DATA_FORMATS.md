# NautilusTrader 回测数据格式指南

本指南说明了如何将本地历史数据转换为 NautilusTrader 回测所需的格式。

## 目录

1. [支持的数据类型](#支持的数据类型)
2. [数据格式要求](#数据格式要求)
3. [数据转换方法](#数据转换方法)
4. [完整示例](#完整示例)

---

## 支持的数据类型

NautilusTrader 回测支持以下数据类型：

### 基础市场数据

| 数据类型 | 说明 | 用途 |
|---------|------|------|
| **Bars (OHLCV)** | 开高低收成交量数据 | K线回测 |
| **Quote Ticks** | 报价数据（买卖价） | 逐笔回测 |
| **Trade Ticks** | 成交数据 | 逐笔回测 |
| **Order Book Deltas** | 订单簿增量数据 | 高频回测 |
| **Order Book Depth** | 订单簿深度快照 | 高频回测 |

### 期权数据

| 数据类型 | 说明 | 用途 |
|---------|------|------|
| **Option Quotes** | 期权报价数据 | 期权策略回测 |
| **Option Contracts** | 期权合约数据 | 期权回测 |
| **Greeks Data** | 期权Greeks数据 | 期权风险分析 |

> **期权数据支持**: NautilusTrader 完整支持期权回测，包括期权合约定义、行权模拟、Greeks 计算等。详细信息请参阅 `OPTION_DATA_GUIDE.md`。

### 衍生品专有数据

| 数据类型 | 说明 | 用途 |
|---------|------|------|
| **Mark Price Update** | 标记价格更新 | 永续合约保证金计算 |
| **Index Price Update** | 指数价格更新 | 永续合约标的参考价 |
| **Funding Rate Update** | 资金费率更新 | 永续合约资金费率 |
| **Instrument Status** | 交易状态更新 | 市场开盘/收盘/暂停 |
| **Instrument Close** | 合约收盘价 | 最终结算价 |

> **衍生品数据**: 主要用于永续合约和期货回测，包括资金费率、标记价格等市场机制数据。

---

## 数据格式要求

### 1. Bars (K线数据)

#### 支持的文件格式
- **CSV** (推荐用于小数据集)
- **Parquet** (推荐用于大数据集)

#### CSV 格式 (两种标准)

**格式 A: 简单格式** (类似 FXCM)
```csv
timestamp,open,high,low,close,volume
2012-02-01 00:00:00+00:00,1.57597,1.57606,1.57576,1.57576,1000
2012-02-01 00:01:00+00:00,1.57576,1.57583,1.57543,1.57543,1500
```

**格式 B: 完整格式** (类似 Binance)
```csv
timestamp,open,high,low,close,volume,close_time,quote_volume,trades,taker_buy_base,taker_buy_quote,ignore
1637971200000,0.00002853,0.00002854,0.00002851,0.00002854,36304.2,1637971259999,1.03547608,79,19523.0,0.55694227,0
```

#### Parquet 格式
使用 PyArrow 序列化的二进制格式，包含以下列：
- `timestamp` (pd.DatetimeIndex, UTC)
- `open` (float64 or Decimal)
- `high` (float64 or Decimal)
- `low` (float64 or Decimal)
- `close` (float64 or Decimal)
- `volume` (float64)

---

### 2. Quote Ticks (报价数据)

#### CSV 格式
```csv
timestamp,bid_price,ask_price,bid_size,ask_size
20200101 170000065,1.121200,1.121720,1000000,1200000
20200101 170010447,1.121200,1.121920,1000000,1300000
```

#### Parquet 格式
包含以下列：
- `timestamp` (pd.DatetimeIndex, UTC)
- `bid_price` (Price)
- `ask_price` (Price)
- `bid_size` (Quantity)
- `ask_size` (Quantity)

---

### 3. Trade Ticks (成交数据)

#### CSV 格式 (Binance)
```csv
timestamp,trade_id,price,quantity,buyer_maker
2020-08-14 10:00:00.223000+00:00,148568980,423.76,2.67900,True
2020-08-14 10:00:00.976000+00:00,148568981,423.74,2.31976,True
```

#### 必需字段
- `timestamp` - 成交时间 (UTC)
- `trade_id` - 成交ID (字符串)
- `price` - 成交价格
- `quantity` - 成交数量
- `side` 或 `buyer_maker` - 买卖方向

---

### 4. Order Book Deltas (订单簿增量)

#### CSV 格式 (Binance 深度数据)
```csv
symbol,timestamp,first_update_id,last_update_id,side,update_type,price,qty,pu
BTCUSDT,1667347199939,2098041693435,2098041696700,a,set,20472.80,0.000,2098041693400
BTCUSDT,1667347199939,2098041693435,2098041696700,b,set,20472.70,0.000,2098041693400
```

#### 字段说明
- `side`: `a`=ask(卖), `b`=bid(买)
- `update_type`: `set`=设置, `delete`=删除
- `price`: 价格
- `qty`: 数量
- `pu`: 交易对ID

---

## 数据转换方法

### 方法 1: 使用内置 Wranglers (推荐)

#### BarDataWrangler

```python
from nautilus_trader.persistence.wranglers import BarDataWrangler
from nautilus_trader.model.data import BarType
from nautilus_trader.model.instruments import Instrument

# 创建 wrangler
wrangler = BarDataWrangler(
    bar_type=BarType.from_str("BTC/USDT.BINANCE-1-MINUTE-BID-EXTERNAL"),
    instrument=instrument,  # 你的 Instrument 对象
)

# 处理数据
bars = wrangler.process(data=df)
```

#### QuoteTickDataWrangler

```python
from nautilus_trader.persistence.wranglers import QuoteTickDataWrangler

wrangler = QuoteTickDataWrangler(
    instrument_id=instrument.id,
)

quotes = wrangler.process(data=df)
```

#### TradeTickDataWrangler

```python
from nautilus_trader.persistence.wranglers import TradeTickDataWrangler

wrangler = TradeTickDataWrangler(
    instrument_id=instrument.id,
)

trades = wrangler.process(data=df)
```

### 方法 2: 使用 CSV 加载器

```python
from nautilus_trader.persistence.loaders import CSVBarDataLoader
from nautilus_trader.persistence.loaders import CSVTickDataLoader

# 加载 Bar 数据
df = CSVBarDataLoader.load("path/to/bars.csv")

# 加载 Tick 数据
df = CSVTickDataLoader.load("path/to/ticks.csv")
```

### 方法 3: 使用 Parquet 加载器

```python
from nautilus_trader.persistence.loaders import ParquetBarDataLoader
from nautilus_trader.persistence.loaders import ParquetTickDataLoader

# 加载 Bar 数据
df = ParquetBarDataLoader.load("path/to/bars.parquet")

# 加载 Tick 数据
df = ParquetTickDataLoader.load("path/to/ticks.parquet", timestamp_column="timestamp")
```

---

## 数据预处理要求

### 时间戳格式

**重要**: NautilusTrader 使用 **Unix 纳秒时间戳** (uint64)

```python
from nautilus_trader.core.datetime import dt_to_unix_nanos

# 转换为纳秒时间戳
timestamp_ns = dt_to_unix_nanos(datetime(2024, 1, 1, 0, 0, 0))
```

### 价格和数量精度

NautilusTrader 使用 `FixedPrecision` (16字节) 存储价格和数量：

```python
from nautilus_trader.model.objects import Price, Quantity

# 创建价格和数量
price = Price("40000.50", precision=2)
quantity = Quantity("1.5", precision=8)
```

### Instrument ID 格式

```python
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue

# 格式: SYMBOL.VENUE
instrument_id = InstrumentId(
    symbol=Symbol("BTCUSDT"),
    venue=Venue("BINANCE"),
)
# 结果: BTCUSDT.BINANCE
```

---

## 完整示例

### 示例 1: 从 CSV Bar 数据回测

```python
import pandas as pd
from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.backtest.config import BacktestEngineConfig
from nautilus_trader.persistence.wranglers import BarDataWrangler
from nautilus_trader.model.data import BarType
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.instruments import CurrencyPair
from nautilus_trader.model.currencies import BTC, USDT
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.enums import AccountType, OmsType

# 1. 读取你的 CSV 数据
df = pd.read_csv("btc_usdt_1m.csv", index_col="timestamp", parse_dates=True)

# 2. 创建 Instrument
instrument = CurrencyPair(
    instrument_id=InstrumentId(
        symbol=Symbol("BTCUSDT"),
        venue=Venue("BINANCE"),
    ),
    base_currency=BTC,
    quote_currency=USDT,
    price_precision=2,
    size_precision=8,
    price_increment=Price("0.01", precision=2),
    size_increment=Quantity("0.00000001", precision=8),
)

# 3. 创建 BarType
bar_type = BarType.from_str("BTC/USDT.BINANCE-1-MINUTE-BID-EXTERNAL")

# 4. 创建 Wrangler 并处理数据
wrangler = BarDataWrangler(bar_type=bar_type, instrument=instrument)
bars = wrangler.process(data=df)

# 5. 配置并运行回测引擎
config = BacktestEngineConfig(trader_id="BACKTESTER-001")
engine = BacktestEngine(config=config)

engine.add_venue(
    venue=Venue("BINANCE"),
    oms_type=OmsType.HEDGING,
    account_type=AccountType.CASH,
    base_currency=USDT,
    starting_balances=[USDT(100_000)],
)

engine.add_instrument(instrument)
engine.add_data(bars)

# 添加你的策略
# engine.add_strategy(strategy=my_strategy)

engine.run()
```

### 示例 2: 从 Tick 数据回测

```python
from nautilus_trader.persistence.wranglers import TradeTickDataWrangler
from nautilus_trader.persistence.loaders import CSVTickDataLoader

# 1. 加载 CSV
df = CSVTickDataLoader.load("trades.csv")

# 2. 确保列名正确
# 必需列: timestamp, price, size, side (可选 trade_id)

# 3. 处理为 TradeTick
wrangler = TradeTickDataWrangler(instrument_id=instrument.id)
trades = wrangler.process(data=df)

# 4. 添加到回测引擎
engine.add_data(trades)
```

### 示例 3: 批量处理多个数据文件

```python
from pathlib import Path

data_dir = Path("historical_data/bars")

# 批量读取并合并
all_bars = []
for csv_file in data_dir.glob("*.csv"):
    df = CSVBarDataLoader.load(csv_file)
    bars = wrangler.process(data=df)
    all_bars.extend(bars)

# 添加到引擎
engine.add_data(all_bars)
```

---

## 数据验证

### 检查数据格式

```python
from nautilus_trader.test_kit.providers import TestDataProvider

provider = TestDataProvider()

# 验证 CSV 格式是否正确
df = provider.read_csv_bars("your_data.csv")

# 检查列
print(df.columns)
# 期望: Index(['timestamp'], dtype='object'),
#       ['open', 'high', 'low', 'close', 'volume']
```

### 常见问题

1. **时间戳格式错误**
   ```
   解决方案: 确保时间戳是 UTC 时区，格式为 datetime64[ns]
   df.index = pd.to_datetime(df.index).tz_localize(None)
   ```

2. **价格精度错误**
   ```
   解决方案: 使用 Price 和 Quantity 类指定正确的精度
   price = Price("40000.50", precision=2)  # 2位小数
   ```

3. **Instrument ID 不匹配**
   ```
   解决方案: 确保 Instrument ID 格式为 SYMBOL.VENUE
   instrument_id = InstrumentId(symbol=Symbol("BTCUSDT"), venue=Venue("BINANCE"))
   # 结果: BTCUSDT.BINANCE
   ```

---

## 高级功能

### ParquetDataCatalog (推荐用于生产环境)

```python
from nautilus_trader.persistence.catalog import ParquetDataCatalog

# 创建数据目录
catalog = ParquetDataCatalog(path="./data_catalog")

# 写入数据
catalog.write_data(bars)

# 从目录读取
bars = catalog.bars(
    instrument_id="BTCUSDT.BINANCE",
    start_time="2024-01-01",
    end_time="2024-01-31",
)
```

### 过滤和查询

```python
# 使用表达式过滤
from nautilus_trader.backtest.config import BacktestDataConfig

config = BacktestDataConfig(
    catalog_path="./data_catalog",
    data_cls=Bar,
    instrument_id="EUR/USD.SIM",
    start_time="2024-01-01",
    end_time="2024-01-31",
    filter_expr='field("price") > 1.0',  # 价格过滤
)
```

---

## 数据来源示例

### 适配器特定加载器

```python
# Binance 数据
from nautilus_trader.adapters.binance.loaders import BinanceOrderBookDeltaDataLoader

loader = BinanceOrderBookDeltaDataLoader()
deltas = loader.load_from_files("path/to/binance/depth")

# Databento 数据
from nautilus_trader.adapters.databento.loaders import DatabentoDataLoader

loader = DatabentoDataLoader()
data = loader.load("path/to/databento.dbn")
```

---

## 衍生品数据格式

### Mark Price Update (标记价格更新)

用于永续合约的标记价格，用于计算未实现盈亏和保证金要求。

```python
from nautilus_trader.model.data import MarkPriceUpdate
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.objects import Price

mark_price = MarkPriceUpdate(
    instrument_id=InstrumentId.from_str("BTCUSDT-PERP.BINANCE"),
    value=Price("43250.50", precision=2),
    ts_event=1640995200000000000,  # 纳秒时间戳
    ts_init=1640995200000000000,
)
```

**CSV 格式**:
```csv
timestamp,instrument_id,mark_price
2024-01-01 00:00:00+00:00,BTCUSDT-PERP.BINANCE,43250.50
2024-01-01 00:01:00+00:00,BTCUSDT-PERP.BINANCE,43255.75
```

### Index Price Update (指数价格更新)

永续合约的参考指数价格，通常用于计算标记价格和资金费率。

```python
from nautilus_trader.model.data import IndexPriceUpdate

index_price = IndexPriceUpdate(
    instrument_id=InstrumentId.from_str("BTCUSDT-PERP.BINANCE"),
    value=Price("43248.20", precision=2),
    ts_event=1640995200000000000,
    ts_init=1640995200000000000,
)
```

**CSV 格式**:
```csv
timestamp,instrument_id,index_price
2024-01-01 00:00:00+00:00,BTCUSDT-PERP.BINANCE,43248.20
2024-01-01 00:01:00+00:00,BTCUSDT-PERP.BINANCE,43252.30
```

### Funding Rate Update (资金费率更新)

永续合约的资金费率数据，用于模拟资金费率结算。

```python
from nautilus_trader.model.data import FundingRateUpdate
from decimal import Decimal

funding_rate = FundingRateUpdate(
    instrument_id=InstrumentId.from_str("BTCUSDT-PERP.BINANCE"),
    rate=Decimal("0.0001"),  # 0.01% 资金费率
    ts_event=1640995200000000000,
    ts_init=1640995200000000000,
    next_funding_ns=1641081600000000000,  # 下次结算时间
)
```

**CSV 格式**:
```csv
timestamp,instrument_id,funding_rate,next_funding_time
2024-01-01 00:00:00+00:00,BTCUSDT-PERP.BINANCE,0.0001,2024-01-01 08:00:00+00:00
2024-01-01 08:00:00+00:00,BTCUSDT-PERP.BINANCE,0.00015,2024-01-01 16:00:00+00:00
```

**说明**:
- `funding_rate`: 资金费率（通常为 8 小时结算一次）
- `next_funding_time`: 下次资金费率结算时间（可选）

### Instrument Status (交易状态)

表示交易品种的市场状态变化（开盘、收盘、暂停交易等）。

```python
from nautilus_trader.model.data import InstrumentStatus
from nautilus_trader.model.enums import MarketStatusAction

status = InstrumentStatus(
    instrument_id=InstrumentId.from_str("AAPL.NASDAQ"),
    action=MarketStatusAction.TRADING_RESUMED,  # 恢复交易
    ts_event=1640995200000000000,
    ts_init=1640995200000000000,
    reason="Market opened",
    trading_event="Morning session",
    is_trading=True,
    is_quoting=True,
    is_short_sell_restricted=False,
)
```

**CSV 格式**:
```csv
timestamp,instrument_id,action,reason,is_trading,is_quoting
2024-01-01 09:30:00+00:00,AAPL.NASDAQ,TRADING_RESUMED,Market opened,True,True
2024-01-01 16:00:00+00:00,AAPL.NASDAQ,TRADING_HALTED,Market closed,False,False
```

**常用 MarketStatusAction**:
- `TRADING_RESUMED`: 恢复交易
- `TRADING_HALTED`: 暂停交易
- `PRE_OPEN`: 开盘前
- `POST_CLOSE`: 收盘后

### Instrument Close (合约收盘价)

合约的最终收盘价格或结算价格。

```python
from nautilus_trader.model.data import InstrumentClose
from nautilus_trader.model.enums import InstrumentCloseType

close = InstrumentClose(
    instrument_id=InstrumentId.from_str("ESM4.CME"),
    close_price=Price("5230.50", precision=2),
    ts_event=1640995200000000000,
    ts_init=1640995200000000000,
    close_type=InstrumentCloseType.FINAL_SETTLE,  # 最终结算价
)
```

**CSV 格式**:
```csv
timestamp,instrument_id,close_price,close_type
2024-03-15 16:00:00+00:00,ESM4.CME,5230.50,FINAL_SETTLE
2024-03-15 16:00:00+00:00,AAPL240315C00150000.NASDAQ,5.30,EXPIRATION
```

**常用 InstrumentCloseType**:
- `FINAL_SETTLE`: 最终结算价（期货）
- `EXPIRATION`: 到期价格（期权）
- `END_OF_DAY`: 日终收盘价

---

## 总结

### 数据转换步骤

1. **准备原始数据** (CSV/Parquet/其他格式)
2. **转换为 DataFrame** (使用加载器或直接 pandas)
3. **创建 Instrument** (定义交易对)
4. **创建 Wrangler** (匹配数据类型)
5. **处理数据** (生成 Nautilus 对象)
6. **添加到回测引擎** (运行回测)

### 最佳实践

- ✅ 使用 Parquet 格式存储大数据集
- ✅ 确保时间戳为 UTC 时区
- ✅ 使用纳秒精度的时间戳
- ✅ 验证 Instrument ID 格式
- ✅ 测试小数据集后再处理全部数据

### 相关文件

- 数据加载器: `nautilus_trader/persistence/loaders.py`
- 数据处理器: `nautilus_trader/persistence/wranglers.pyx`
- 回测配置: `nautilus_trader/backtest/config.py`
- 回测引擎: `nautilus_trader/backtest/engine.py`
- 示例数据: `tests/test_data/`
- 示例脚本: `examples/backtest/`
