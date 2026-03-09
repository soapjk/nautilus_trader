#!/usr/bin/env python3
"""诊断脚本 - 测试 Binance data tester 所需的导入"""

import sys
import traceback

def test_imports():
    """测试所有必需的导入"""
    print("=" * 60)
    print("测试导入...")
    print("=" * 60)

    tests = [
        ("BINANCE constant", "from nautilus_trader.adapters.binance import BINANCE"),
        ("BinanceAccountType", "from nautilus_trader.adapters.binance import BinanceAccountType"),
        ("BinanceDataClientConfig", "from nautilus_trader.adapters.binance import BinanceDataClientConfig"),
        ("BinanceLiveDataClientFactory", "from nautilus_trader.adapters.binance import BinanceLiveDataClientFactory"),
        ("InstrumentProviderConfig", "from nautilus_trader.config import InstrumentProviderConfig"),
        ("LoggingConfig", "from nautilus_trader.config import LoggingConfig"),
        ("TradingNodeConfig", "from nautilus_trader.config import TradingNodeConfig"),
        ("TradingNode", "from nautilus_trader.live.node import TradingNode"),
        ("BarType", "from nautilus_trader.model.data import BarType"),
        ("InstrumentId", "from nautilus_trader.model.identifiers import InstrumentId"),
        ("TraderId", "from nautilus_trader.model.identifiers import TraderId"),
        ("DataTester", "from nautilus_trader.test_kit.strategies.tester_data import DataTester"),
        ("DataTesterConfig", "from nautilus_trader.test_kit.strategies.tester_data import DataTesterConfig"),
    ]

    failed = []
    for name, import_stmt in tests:
        try:
            exec(import_stmt)
            print(f"✅ {name}")
        except Exception as e:
            print(f"❌ {name}: {e}")
            failed.append((name, e, traceback.format_exc()))

    return failed

def test_config_creation():
    """测试配置创建"""
    print("\n" + "=" * 60)
    print("测试配置创建...")
    print("=" * 60)

    try:
        from nautilus_trader.adapters.binance import BINANCE, BinanceAccountType, BinanceDataClientConfig
        from nautilus_trader.config import InstrumentProviderConfig, LoggingConfig, TradingNodeConfig
        from nautilus_trader.model.identifiers import InstrumentId, TraderId

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
        print("✅ TradingNodeConfig 创建成功")
        return True
    except Exception as e:
        print(f"❌ 配置创建失败: {e}")
        traceback.print_exc()
        return False

def test_rust_extension():
    """测试 Rust 扩展"""
    print("\n" + "=" * 60)
    print("测试 Rust 扩展...")
    print("=" * 60)

    try:
        import nautilus_trader.core.nautilus_pyo3
        print("✅ Rust 扩展加载成功")

        # 测试基本功能
        from nautilus_trader.core.nautilus_pyo3 import UUID4
        uuid = UUID4()
        print(f"✅ UUID4 工作正常: {uuid}")
        return True
    except Exception as e:
        print(f"❌ Rust 扩展失败: {e}")
        traceback.print_exc()
        return False

if __name__ == "__main__":
    print("开始诊断...\n")

    # 测试 Rust 扩展
    rust_ok = test_rust_extension()

    # 测试导入
    failed_imports = test_imports()

    # 测试配置创建
    config_ok = test_config_creation()

    # 总结
    print("\n" + "=" * 60)
    print("诊断总结")
    print("=" * 60)
    print(f"Rust 扩展: {'✅ OK' if rust_ok else '❌ FAIL'}")
    print(f"导入测试: {'✅ ALL OK' if not failed_imports else f'❌ {len(failed_imports)} 失败'}")
    print(f"配置创建: {'✅ OK' if config_ok else '❌ FAIL'}")

    if failed_imports:
        print("\n失败的导入:")
        for name, e, tb in failed_imports:
            print(f"  - {name}: {e}")

    sys.exit(0 if (rust_ok and not failed_imports and config_ok) else 1)
