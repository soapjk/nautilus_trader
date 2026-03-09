#!/usr/bin/env python3
import sys

print("Testing full nautilus_trader package...", flush=True)

# 逐步导入
print("1. Importing adapters.binance...", flush=True)
from nautilus_trader.adapters import binance
print("   OK", flush=True)

print("2. Importing adapters.binance.constants...", flush=True)
from nautilus_trader.adapters.binance.common.constants import BINANCE
print(f"   OK: {BINANCE}", flush=True)

print("3. Importing config...", flush=True)
from nautilus_trader.config import LoggingConfig, TradingNodeConfig
print("   OK", flush=True)

print("4. Importing model.identifiers...", flush=True)
from nautilus_trader.model.identifiers import TraderId, InstrumentId
print("   OK", flush=True)

print("5. Importing live.node...", flush=True)
from nautilus_trader.live.node import TradingNode
print("   OK", flush=True)

print("\n✅ All imports successful!")
