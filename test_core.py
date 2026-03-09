#!/usr/bin/env python3
import sys

print("Testing core modules...", flush=True)

print("1. core.nautilus_pyo3...", flush=True)
import nautilus_trader.core.nautilus_pyo3
print("   OK", flush=True)

print("2. core.datetime...", flush=True)
from nautilus_trader.core.datetime import UnixNanos
print(f"   OK: {UnixNanos}", flush=True)

print("3. core.uuid...", flush=True)
from nautilus_trader.core.uuid import UUID4
uuid = UUID4()
print(f"   OK: {uuid}", flush=True)

print("4. core.message...", flush=True)
from nautilus_trader.core.message import MessageType
print(f"   OK: {MessageType}", flush=True)

print("\n✅ Core modules OK!")
