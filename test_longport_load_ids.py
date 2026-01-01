#!/usr/bin/env python3
"""简单测试脚本验证 load_ids 功能"""
import os
import signal
import sys
from pathlib import Path
from dotenv import load_dotenv

# 设置超时
def timeout_handler(signum, frame):
    print("\n=== 测试完成 - 退出 ===")
    sys.exit(0)

signal.signal(signal.SIGALRM, timeout_handler)
signal.alarm(15)  # 15秒后退出

# 添加项目路径
sys.path.insert(0, str(Path(__file__).parent))

from nautilus_trader.adapters.longport import (
    LONGPORT,
    LONGPORT_VENUE,
    LongportMarket,
    LongportDataClientConfig,
    LongportLiveDataClientFactory,
)
from nautilus_trader.config import (
    DatabaseConfig,
    InstrumentProviderConfig,
    LoggingConfig,
    MessageBusConfig,
    TradingNodeConfig,
)
from nautilus_trader.live.node import TradingNode
from nautilus_trader.model.data import BarType
from nautilus_trader.model.identifiers import InstrumentId, TraderId, ClientId
from nautilus_trader.test_kit.strategies.tester_data import DataTester
from nautilus_trader.test_kit.strategies.tester_data import DataTesterConfig

# 加载环境变量
load_dotenv(Path.cwd() / ".env")

# 读取认证信息
LONGPORT_APP_KEY = os.getenv("LONGPORT_APP_KEY", "")
LONGPORT_APP_SECRET = os.getenv("LONGPORT_APP_SECRET", "")
LONGPORT_ACCESS_TOKEN = os.getenv("LONGPORT_ACCESS_TOKEN", "")

# 配置
MARKET = [LongportMarket.HK, LongportMarket.US]
INSTRUMENTS = ["AAPL.US"]
instrument_ids = [InstrumentId.from_str(s) for s in INSTRUMENTS]

# 创建配置
config = TradingNodeConfig(
    trader_id=TraderId("TEST-LOAD-IDS-001"),
    logging=LoggingConfig(log_level="INFO", log_colors=True),
    message_bus=MessageBusConfig(
        database=DatabaseConfig(type="redis", host="localhost", port=6379, timeout=20),
        streams_prefix="test_longport",
        use_trader_id=False,
        use_trader_prefix=False,
        use_instance_id=False,
        encoding="msgpack",
        buffer_interval_ms=100,
    ),
    data_clients={
        LONGPORT: LongportDataClientConfig(
            app_key=LONGPORT_APP_KEY,
            app_secret=LONGPORT_APP_SECRET,
            access_token=LONGPORT_ACCESS_TOKEN,
            markets=["HK", "US"],
            instrument_provider=InstrumentProviderConfig(
                load_ids=frozenset(instrument_ids),  # 关键：使用 load_ids
            ),
        ),
    },
)

def main():
    print("=" * 60)
    print("测试 load_ids 功能")
    print("=" * 60)
    print(f"订阅交易对: {INSTRUMENTS}")
    print(f"市场: {MARKET}")
    print("=" * 60)

    # 创建节点
    node = TradingNode(config=config)

    # 配置数据采集器
    config_tester = DataTesterConfig(
        instrument_ids=instrument_ids,
        bar_types=[
            BarType.from_str(f"{instr_id}-1-MINUTE-LAST-EXTERNAL")
            for instr_id in instrument_ids
        ],
        subscribe_instrument=False,
        subscribe_quotes=True,
        subscribe_trades=True,
        subscribe_book_deltas=True,
        subscribe_bars=True,
        client_id=ClientId(LONGPORT),
    )

    # 创建数据采集器
    tester = DataTester(config=config_tester)

    # 注册数据客户端工厂
    node.add_data_client_factory(LONGPORT, LongportLiveDataClientFactory)

    # 添加数据采集器到节点
    node.trader.add_actor(tester)

    # 构建节点
    node.build()

    # 运行节点
    try:
        node.run()
    except KeyboardInterrupt:
        print("\n收到停止信号，正在关闭...")
        node.dispose()
        print("已停止")

if __name__ == "__main__":
    main()
