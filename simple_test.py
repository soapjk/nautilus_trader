#!/usr/bin/env python3
import sys

print("Step 1: Python OK", flush=True)

print("Step 2: Importing nautilus_trader...", flush=True)
import nautilus_trader
print("Step 2: OK", flush=True)

print("Step 3: Importing BINANCE...", flush=True)
from nautilus_trader.adapters.binance import BINANCE
print(f"Step 3: OK - BINANCE={BINANCE}", flush=True)

print("Step 4: Importing TradingNode...", flush=True)
from nautilus_trader.live.node import TradingNode
print("Step 4: OK", flush=True)

print("Step 5: Importing DataTester...", flush=True)
from nautilus_trader.test_kit.strategies.tester_data import DataTester
print("Step 5: OK", flush=True)

print("\n✅ All imports successful!")
