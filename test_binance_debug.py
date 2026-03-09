#!/usr/bin/env python3
"""调试版本的 Binance Data Tester"""

import sys

print("=== Starting debug imports ===", flush=True)

# 步骤1: 导入 pyarrow
print("Step 1: Importing pyarrow...", flush=True)
import pyarrow as pa
print(f"  ✓ PyArrow version: {pa.__version__}", flush=True)

# 步骤2: 导入基础 nautilus 模块
print("\nStep 2: Importing base nautilus modules...", flush=True)
from nautilus_trader.adapters.binance.common.constants import BINANCE
print(f"  ✓ BINANCE constant: {BINANCE}", flush=True)

from nautilus_trader.adapters.binance.common.enums import BinanceAccountType
print(f"  ✓ BinanceAccountType imported", flush=True)

from nautilus_trader.adapters.binance.config import BinanceDataClientConfig
print(f"  ✓ BinanceDataClientConfig imported", flush=True)

from nautilus_trader.adapters.binance.factories import BinanceLiveDataClientFactory
print(f"  ✓ BinanceLiveDataClientFactory imported", flush=True)

# 步骤3: 导入配置模块
print("\nStep 3: Importing config modules...", flush=True)
from nautilus_trader.config import InstrumentProviderConfig, LoggingConfig, TradingNodeConfig
print(f"  ✓ Config modules imported", flush=True)

# 步骤4: 导入标识符
print("\nStep 4: Importing identifiers...", flush=True)
from nautilus_trader.model.identifiers import InstrumentId, TraderId
print(f"  ✓ Identifiers imported", flush=True)

# 步骤5: 导入 TradingNode
print("\nStep 5: Importing TradingNode...", flush=True)
from nautilus_trader.live.node import TradingNode
print(f"  ✓ TradingNode imported", flush=True)

# 步骤6: 导入 DataTester
print("\nStep 6: Importing DataTester...", flush=True)
from nautilus_trader.test_kit.strategies.tester_data import DataTester, DataTesterConfig
print(f"  ✓ DataTester imported", flush=True)

# 步骤7: 导入数据类型
print("\nStep 7: Importing data types...", flush=True)
from nautilus_trader.model.data import BarType
print(f"  ✓ Data types imported", flush=True)

print("\n✅ All imports successful!")
print("=== Creating configuration ===", flush=True)

# 创建配置
symbol = "BTCUSDT"
instrument_id = InstrumentId.from_str(f"{symbol}.BINANCE")

config_node = TradingNodeConfig(
    trader_id=TraderId("TESTER-001"),
    logging=LoggingConfig(log_level="INFO", use_pyo3=True),
    data_clients={
        BINANCE: BinanceDataClientConfig(
            api_key=None,
            api_secret=None,
            account_type=BinanceAccountType.SPOT,
            instrument_provider=InstrumentProviderConfig(load_ids=frozenset([instrument_id])),
        ),
    },
)

print(f"✓ Config created", flush=True)

# 创建节点
print("\n=== Creating TradingNode ===", flush=True)
node = TradingNode(config=config_node)
print(f"✓ TradingNode created", flush=True)

# 创建 DataTester 配置
print("\n=== Creating DataTester config ===", flush=True)
config_tester = DataTesterConfig(
    instrument_ids=[instrument_id],
    bar_types=[BarType.from_str(f"{instrument_id}-1-MINUTE-LAST-EXTERNAL")],
    subscribe_instrument=False,
    subscribe_book_at_interval=False,
    subscribe_bars=True,
    book_interval_ms=100,
)
print(f"✓ DataTesterConfig created", flush=True)

# 创建 tester
print("\n=== Creating DataTester ===", flush=True)
tester = DataTester(config=config_tester)
print(f"✓ DataTester created", flush=True)

print("\n=== All setup successful! ===")
print("Ready to start the node...", flush=True)
