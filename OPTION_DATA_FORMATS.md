# NautilusTrader 期权数据格式

## 期权数据 CSV 格式

### 1. 期权报价数据 (Option Quote Ticks)

```csv
timestamp,bid_price,ask_price,bid_size,ask_size,implied_volatility,delta,gamma,theta,vega
2024-03-01 09:30:00+00:00,5.25,5.30,100,150,0.25,0.52,0.05,-0.02,0.15
2024-03-01 09:31:00+00:00,5.28,5.32,120,130,0.25,0.53,0.05,-0.02,0.15
2024-03-01 09:32:00+00:00,5.30,5.35,80,100,0.26,0.54,0.05,-0.02,0.15
```

**列说明**:
- `timestamp`: 时间戳 (UTC, ISO8601格式)
- `bid_price`: 买入价格
- `ask_price`: 卖出价格
- `bid_size`: 买入数量
- `ask_size`: 卖出数量
- `implied_volatility`: 隐含波动率 (可选)
- `delta`: Delta值 (可选)
- `gamma`: Gamma值 (可选)
- `theta`: Theta值 (可选)
- `vega`: Vega值 (可选)

**必需列**: `timestamp`, `bid_price`, `ask_price`
**可选列**: `bid_size`, `ask_size`, Greeks 指标

### 2. 期权 K线数据 (Option Bars)

```csv
timestamp,open,high,low,close,volume
2024-03-01 09:30:00+00:00,5.20,5.35,5.15,5.30,5000
2024-03-01 09:31:00+00:00,5.30,5.32,5.28,5.31,3000
2024-03-01 09:32:00+00:00,5.31,5.36,5.30,5.35,2000
```

**列说明**:
- `timestamp`: 时间戳 (UTC, ISO8601格式)
- `open`: 开盘价
- `high`: 最高价
- `low`: 最低价
- `close`: 收盘价
- `volume`: 成交量 (合约数量)

### 3. 期权成交数据 (Option Trade Ticks)

```csv
timestamp,trade_id,price,quantity,side,open_interest
2024-03-01 09:30:05+00:00,12345,5.30,10,BUY,5000
2024-03-01 09:30:10+00:00,12346,5.25,5,SELL,5000
2024-03-01 09:30:15+00:00,12347,5.28,7,BUY,2000
```

**列说明**:
- `timestamp`: 成交时间 (UTC, ISO8601格式)
- `trade_id`: 成交ID (唯一标识)
- `price`: 成交价格
- `quantity`: 成交数量 (合约数)
- `side`: 方向 (BUY/SELL)
- `open_interest`: 未平仓合约数 (可选)

**必需列**: `timestamp`, `price`, `quantity`
**可选列**: `trade_id`, `side`, `open_interest`

### 4. 期权链数据 (Option Chain)

期权链通常存储为 JSON 或多个 CSV 文件：

```json
[
  {
    "symbol": "AAPL240315C00150000",
    "underlying": "AAPL",
    "option_kind": "CALL",
    "strike_price": 150.0,
    "expiration": "2024-03-15T16:00:00Z",
    "multiplier": 100,
    "price_precision": 2,
    "size_precision": 0,
    "currency": "USD"
  },
  {
    "symbol": "AAPL240315P00150000",
    "underlying": "AAPL",
    "option_kind": "PUT",
    "strike_price": 150.0,
    "expiration": "2024-03-15T16:00:00Z",
    "multiplier": 100,
    "price_precision": 2,
    "size_precision": 0,
    "currency": "USD"
  }
]
```

## 期权合约命名规范

### OCC 格式 (美国期权)

```
AAPL240315C00150000
│   │   │ │   └行权价 (8位，×1000)
│   │   │ └─看涨/看跌 (C=Call, P=Put)
│   │   └───到期日 (YYMMDD)
│   └───────期权类型标识
└───────────标的代码
```

示例:
- `AAPL240315C00150000`: AAPL 2024年3月15日 到期 $150 看涨期权
- `AAPL240315P00145000`: AAPL 2024年3月15日 到期 $145 看跌期权

### 其他格式

```
# CME 格式
ESM4 P5230 (ES 2024年6月 Put, 5230行权价)

# 长格式
AAPL-20240315-150-CALL (AAPL, 2024-03-15, $150, Call)

# 简化格式
AAPL_150_C_0324 (AAPL, $150, Call, 3月24日)
```

## 期权数据特殊处理

### 1. 期权乘数

不同期权的乘数不同：

```python
# 股票期权 (每份100股)
multiplier = Quantity.from_int(100)

# 指数期权 (根据指数)
multiplier = Quantity.from_int(100)  # SPX, OEX 等
multiplier = Quantity.from_int(250)   # VIX

# 期货期权 (根据合约)
multiplier = Quantity.from_int(50)   # ES (E-mini S&P 500)
multiplier = Quantity.from_int(20)   # NQ (E-mini Nasdaq-100)
```

### 2. 价格精度

期权价格通常为：

```python
# 股票期权
price_precision = 2
price_increment = Price("0.01", precision=2)

# 指数期权 (SPX, OEX)
price_precision = 2
price_increment = Price("0.01", precision=2)

# 高价值期权
price_precision = 2
price_increment = Price("0.05", precision=2)
```

### 3. 到期日处理

期权到期日需要注意时区：

```python
import pandas as pd
from nautilus_trader.core.datetime import dt_to_unix_nanos

# 美式期权到期日 (通常是第三个周五)
# 芝加哥时间 16:00 (CST)
expiration = pd.Timestamp("2024-03-15 16:00:00", tz="America/Chicago")
expiration_utc = expiration.tz_convert("UTC")
expiration_ns = dt_to_unix_nanos(expiration_utc)
```

### 4. 行权价格式

行权价需要转换为正确的格式：

```python
# 从字符串解析行权价
strike_str = "00150000"  # 150.00 × 1000
strike_price = float(strike_str) / 1000
strike = Price(str(strike_price), precision=2)

# 从 OCC 符号提取
symbol = "AAPL240315C00150000"
strike_part = symbol.split('C')[1].split('P')[0]  # "00150000"
strike_price = float(strike_part) / 1000
```

## 期权 Greeks 数据

### Greeks CSV 格式

```csv
timestamp,underlying_price,strike_price,option_kind,expiration,implied_vol,delta,gamma,theta,vega,rho
2024-03-01 09:30:00,152.50,150.00,CALL,2024-03-15,0.25,0.52,0.05,-0.02,0.15,0.01
2024-03-01 09:31:00,152.55,150.00,CALL,2024-03-15,0.25,0.53,0.05,-0.02,0.15,0.01
```

## 期权策略数据格式

### 期权价差组合

价差组合可以定义为 JSON：

```json
{
  "spread_id": "IRON_CONDOR_1",
  "spread_type": "iron_condor",
  "legs": [
    {
      "instrument_id": "AAPL240315C00145000.NASDAQ",
      "action": "SELL",
      "quantity": 1
    },
    {
      "instrument_id": "AAPL240315P00145000.NASDAQ",
      "action": "SELL",
      "quantity": 1
    },
    {
      "instrument_id": "AAPL240315C00155000.NASDAQ",
      "action": "BUY",
      "quantity": 1
    },
    {
      "instrument_id": "AAPL240315P00155000.NASDAQ",
      "action": "BUY",
      "quantity": 1
    }
  ]
}
```

## 数据转换脚本示例

### 期权报价数据转换

```python
import pandas as pd
from nautilus_trader.persistence.wranglers import QuoteTickDataWrangler
from nautilus_trader.model.instruments import OptionContract

# 读取期权报价 CSV
df = pd.read_csv("option_quotes.csv", parse_dates=["timestamp"])

# 确保 UTC 时区
df["timestamp"] = df["timestamp"].dt.tz_localize("UTC")

# 创建期权合约
option = OptionContract(
    instrument_id=InstrumentId.from_str("AAPL240315C00150000.NASDAQ"),
    raw_symbol=Symbol("AAPL240315C00150000"),
    ...  # 其他参数
)

# 创建 wrangler
wrangler = QuoteTickDataWrangler(instrument_id=option.id)

# 处理数据
quotes = wrangler.process(df)

# 添加到回测引擎
engine.add_data(quotes)
```

### 期权 K线数据转换

```python
from nautilus_trader.persistence.wranglers import BarDataWrangler
from nautilus_trader.model.data import BarType

# 读取期权 K线 CSV
df = pd.read_csv("option_bars.csv", index_col="timestamp", parse_dates=True)

# 创建 bar_type
bar_type = BarType.from_str("AAPL/USD.NASDAQ-1-MINUTE-BID-EXTERNAL")

# 创建 wrangler
wrangler = BarDataWrangler(bar_type=bar_type, instrument=option)

# 处理数据
bars = wrangler.process(df)

# 添加到回测引擎
engine.add_data(bars)
```

## 参考资源

- 完整期权指南: `OPTION_DATA_GUIDE.md`
- 示例文件: `examples/backtest/notebooks/databento_option_greeks.py`
- 测试文件: `tests/unit_tests/backtest/test_option_exercise.py`
