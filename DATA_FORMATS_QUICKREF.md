# NautilusTrader 回测数据格式快速参考

## CSV 格式速查表

### 1. Bars (K线数据)

```csv
timestamp,open,high,low,close,volume
2012-02-01 00:00:00+00:00,1.57597,1.57606,1.57576,1.57576,1000
2012-02-01 00:01:00+00:00,1.57576,1.57583,1.57543,1.57543,1500
```

**必需列**: `timestamp`, `open`, `high`, `low`, `close`, `volume`

### 2. Quote Ticks (报价)

```csv
timestamp,bid_price,ask_price,bid_size,ask_size
20200101 170000065,1.121200,1.121720,1000000,1200000
```

**必需列**: `timestamp`, `bid_price`, `ask_price`
**可选列**: `bid_size`, `ask_size`

### 3. Trade Ticks (成交)

```csv
timestamp,trade_id,price,quantity,buyer_maker
2020-08-14 10:00:00.223000+00:00,148568980,423.76,2.67900,True
```

**必需列**: `timestamp`, `price`, `quantity`
**可选列**: `trade_id`, `side`, `buyer_maker`

### 4. Order Book Deltas (订单簿)

```csv
symbol,timestamp,first_update_id,last_update_id,side,update_type,price,qty
BTCUSDT,1667347199939,2098041693435,2098041696700,a,set,20472.80,0.500
```

**必需列**: `timestamp`, `side`, `update_type`, `price`, `quantity`

### 5. Mark Price Update (标记价格)

```csv
timestamp,instrument_id,mark_price
2024-01-01 00:00:00+00:00,BTCUSDT-PERP.BINANCE,43250.50
```

**必需列**: `timestamp`, `mark_price`

### 6. Funding Rate Update (资金费率)

```csv
timestamp,instrument_id,funding_rate,next_funding_time
2024-01-01 00:00:00+00:00,BTCUSDT-PERP.BINANCE,0.0001,2024-01-01 08:00:00+00:00
```

**必需列**: `timestamp`, `funding_rate`
**可选列**: `next_funding_time`

---

## 快速转换代码

### Bars → Nautilus

```python
import pandas as pd
from nautilus_trader.persistence.wranglers import BarDataWrangler
from nautilus_trader.model.data import BarType

# 读取 CSV
df = pd.read_csv("bars.csv", index_col="timestamp", parse_dates=True)

# 转换为 Bar 对象
wrangler = BarDataWrangler(bar_type=BarType.from_str("BTC/USDT.BINANCE-1-MINUTE-BID"))
bars = wrangler.process(df)

# 添加到回测引擎
engine.add_data(bars)
```

### Trades → Nautilus

```python
from nautilus_trader.persistence.wranglers import TradeTickDataWrangler

# 读取 CSV
df = pd.read_csv("trades.csv", parse_dates=["timestamp"])

# 转换为 TradeTick 对象
wrangler = TradeTickDataWrangler(instrument_id="BTCUSDT.BINANCE")
trades = wrangler.process(df)

# 添加到回测引擎
engine.add_data(trades)
```

---

## 时间戳处理

### UTC 转换

```python
# 方式 1: 本地时间 → UTC
df.index = pd.to_datetime(df.index).tz_localize("UTC")

# 方式 2: 其他时区 → UTC
df.index = pd.to_datetime(df.index).tz_convert("UTC")

# 方式 3: 移除时区信息 (Nautilus 会假设为 UTC)
df.index = df.to_datetime(df.index).tz_localize(None)
```

### Unix 纳秒转换

```python
from nautilus_trader.core.datetime import dt_to_unix_nanos

# datetime → nanoseconds
timestamp_ns = dt_to_unix_nanos(pd.Timestamp("2024-01-01"))
```

---

## Instrument ID 格式

### 标准: `SYMBOL.VENUE`

```python
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue

# 创建 ID
instrument_id = InstrumentId(
    symbol=Symbol("BTCUSDT"),
    venue=Venue("BINANCE"),
)
# 结果: BTCUSDT.BINANCE

# 从字符串解析
instrument_id = InstrumentId.from_str("BTCUSDT.BINANCE")
```

---

## 衍生品数据加载

### Mark Price / Index Price / Funding Rate

```python
import pandas as pd
from nautilus_trader.model.data import MarkPriceUpdate, IndexPriceUpdate, FundingRateUpdate
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.objects import Price
from nautilus_trader.core.datetime import dt_to_unix_nanos

# 读取 CSV
df = pd.read_csv("mark_prices.csv", parse_dates=["timestamp"])

# 确保 UTC 时区
df["timestamp"] = df["timestamp"].dt.tz_localize("UTC")

# 创建 MarkPriceUpdate 对象
mark_prices = []
for _, row in df.iterrows():
    mark_price = MarkPriceUpdate(
        instrument_id=InstrumentId.from_str(row["instrument_id"]),
        value=Price(str(row["mark_price"]), precision=2),
        ts_event=dt_to_unix_nanos(row["timestamp"]),
        ts_init=dt_to_unix_nanos(row["timestamp"]),
    )
    mark_prices.append(mark_price)

# 添加到回测引擎
engine.add_data(mark_prices)
```

### Funding Rate 转换

```python
from decimal import Decimal

funding_rates = []
for _, row in df.iterrows():
    funding = FundingRateUpdate(
        instrument_id=InstrumentId.from_str(row["instrument_id"]),
        rate=Decimal(str(row["funding_rate"])),
        ts_event=dt_to_unix_nanos(row["timestamp"]),
        ts_init=dt_to_unix_nanos(row["timestamp"]),
        next_funding_ns=dt_to_unix_nanos(row["next_funding_time"]) if pd.notna(row["next_funding_time"]) else 0,
    )
    funding_rates.append(funding)

engine.add_data(funding_rates)
```

---

## 数据验证

### 检查 CSV 格式

```bash
# 使用数据转换脚本验证
python examples/data_converter.py --type validate --input your_data.csv
```

### 检查数据质量

```python
# 检查缺失值
df.isnull().sum()

# 检查时间范围
print(f"开始: {df.index.min()}")
print(f"结束: {df.index.max()}")

# 检查重复时间戳
df.index.duplicated().sum()
```

---

## 常见问题解决

### 问题 1: 时间戳解析失败

```python
# 解决方案: 指定时间戳格式
df = pd.read_csv(
    "data.csv",
    index_col="timestamp",
    parse_dates=True,
)
df.index = pd.to_datetime(df.index, format="mixed")  # 自动检测
```

### 问题 2: 价格精度错误

```python
# 解决方案: 在 Instrument 中指定正确的精度
from nautilus_trader.model.objects import Price, Quantity

instrument = CurrencyPair(
    ...,
    price_precision=2,  # 2位小数
    size_precision=8,    # 8位小数
    price_increment=Price("0.01", precision=2),
    size_increment=Quantity("0.00000001", precision=8),
)
```

### 问题 3: 数据加载缓慢

```python
# 解决方案: 使用 Parquet 格式
import pandas as pd

# CSV → Parquet
df = pd.read_csv("data.csv")
df.to_parquet("data.parquet")

# 以后加载会更快
df = pd.read_parquet("data.parquet")
```

---

## 推荐工作流程

```
原始数据 (CSV/其他)
        ↓
    转换格式
        ↓
    验证数据
        ↓
    Parquet 存储
        ↓
    加载到 Nautilus
        ↓
    运行回测
```

---

## 相关命令

```bash
# 转换 Bars 数据
python examples/data_converter.py \
    --type bars \
    --input data/bars.csv \
    --output data/bars.parquet \
    --instrument-id BTCUSDT.BINANCE

# 转换 Trades 数据
python examples/data_converter.py \
    --type trades \
    --input data/trades.csv \
    --output data/trades.parquet \
    --instrument-id ETHUSDT.BINANCE

# 验证 CSV 格式
python examples/data_converter.py \
    --type validate \
    --input data/bars.csv
```

---

## 更多信息

- 详细指南: `BACKTEST_DATA_FORMATS.md`
- 期权数据: `OPTION_DATA_GUIDE.md`
- 示例数据: `tests/test_data/`
- 示例脚本: `examples/backtest/`
- 数据加载器: `nautilus_trader/persistence/loaders.py`
- 数据处理器: `nautilus_trader/persistence/wranglers.pyx`

## 数据类型完整列表

### 基础市场数据
- Bars (OHLCV)
- Quote Ticks
- Trade Ticks
- Order Book Deltas
- Order Book Depth10

### 期权数据
- Option Quotes
- Option Contracts
- Greeks Data

### 衍生品专有数据
- Mark Price Update
- Index Price Update
- Funding Rate Update
- Instrument Status
- Instrument Close
