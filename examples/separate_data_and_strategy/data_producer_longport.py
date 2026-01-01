#!/usr/bin/env python3
"""
数据采集进程 - 专门负责从Longport获取和发布港股/美股/A股市场数据
可以独立运行，策略进程重启不影响数据采集

支持市场：
- 港股 (HK)
- 美股 (US)
- A股 (CN)
"""

import os
from pathlib import Path
from dotenv import load_dotenv

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
    StreamingConfig,
    TradingNodeConfig,
)
from nautilus_trader.live.node import TradingNode
from nautilus_trader.model.data import BarType
from nautilus_trader.model.identifiers import InstrumentId, TraderId, ClientId
from nautilus_trader.test_kit.strategies.tester_data import DataTester
from nautilus_trader.test_kit.strategies.tester_data import DataTesterConfig


# ========================================================================
# 从 .env 文件加载 Longport 认证信息
# ========================================================================

# 加载 .env 文件 (从当前目录或项目根目录)
env_path = Path(__file__).parent.parent / ".env"
load_dotenv(env_path)
# 也尝试加载项目根目录的 .env
load_dotenv(Path.cwd() / ".env", override=True)

# 读取 Longport 认证信息
LONGPORT_APP_KEY = os.getenv("LONGPORT_APP_KEY", "")
LONGPORT_APP_SECRET = os.getenv("LONGPORT_APP_SECRET", "")
LONGPORT_ACCESS_TOKEN = os.getenv("LONGPORT_ACCESS_TOKEN", "")
print("longport app key:",LONGPORT_APP_KEY[:10])
# 验证认证信息是否已配置
if not all([LONGPORT_APP_KEY, LONGPORT_APP_SECRET, LONGPORT_ACCESS_TOKEN]):
    print("警告: Longport 认证信息未完整配置!")
    print("请在 .env 文件中设置以下环境变量:")
    print("  - LONGPORT_APP_KEY")
    print("  - LONGPORT_APP_SECRET")
    print("  - LONGPORT_ACCESS_TOKEN")
    print("")
    print("获取凭证: https://open.longport.app")
    print("")


# ========================================================================
# 配置要订阅的交易对
# ========================================================================

# 市场选择: HK, US, 或 CN
MARKET = [LongportMarket.HK,LongportMarket.US] 

# 交易对列表 (港股示例)
# 格式: 股票代码.市场 (例如: 700.HK, 9988.HK)
INSTRUMENTS = [
    "AAPL.US",   # 苹果 (示例中包含美股代码以展示多市场支持)
]

# 或者选择美股示例
# MARKET = LongportMarket.US
# INSTRUMENTS = [
#     "AAPL.US",   # 苹果
#     "TSLA.US",   # 特斯拉
#     # "MSFT.US",  # 微软
#     # "NVDA.US",  # 英伟达
# ]

# 或者选择A股示例
# MARKET = LongportMarket.CN
# INSTRUMENTS = [
#     "600519.CN",  # 贵州茅台
#     "000858.CN",  # 五粮液
# ]

# 生成 instrument IDs
instrument_ids = [InstrumentId.from_str(s) for s in INSTRUMENTS]

# 同时生成字符串列表用于Rust配置
instrument_id_strs = [str(instr_id) for instr_id in instrument_ids]


# ========================================================================
# 数据采集节点配置
# ========================================================================
config = TradingNodeConfig(
    trader_id=TraderId("DATA-PRODUCER-LONGPORT-001"),

    logging=LoggingConfig(
        log_level="INFO",
        log_colors=True,
    ),

    # Message Bus 配置 - 发布数据到 Redis
    message_bus=MessageBusConfig(
        # Redis 后端配置
        database=DatabaseConfig(
            type="redis",
            host="localhost",
            port=6379,
            # password="your_redis_password",  # 如果需要密码
            timeout=20,
        ),

        # Stream 命名配置
        streams_prefix="market_data_longport",  # Stream 名称: "market_data_longport"
        use_trader_id=False,               # 不添加 trader ID 到 stream 名
        use_trader_prefix=False,           # 不添加 "trader-" 前缀
        use_instance_id=False,             # 不添加实例 UUID

        # 性能配置
        encoding="msgpack",                # 使用 msgpack 序列化（更快）
        buffer_interval_ms=100,            # 批量发送间隔
        stream_per_topic=False,            # 所有数据用单个 stream（减少 Redis 连接数）
        autotrim_mins=60,                  # Redis stream 只保留 60 分钟数据

        # 可选：过滤高频数据
        # types_filter=[],  # 留空表示不过滤，如需过滤可以传入 [QuoteTick]
    ),

    # 数据客户端配置
    data_clients={
        LONGPORT: LongportDataClientConfig(
            # 从 .env 文件读取的认证信息
            http_url="https://openapi.longportapp.cn",
            ws_url="wss://openapi-quote.longportapp.cn",
            app_key=LONGPORT_APP_KEY,
            app_secret=LONGPORT_APP_SECRET,
            access_token=LONGPORT_ACCESS_TOKEN,
            markets=["HK", "US"],  # Use string representation (Python config)
            instrument_provider=InstrumentProviderConfig(
                load_ids=frozenset(instrument_id_strs),  # 只加载指定的交易对（使用字符串格式）
            ),
        ),
    },

    # 可选：同时持久化到本地 Parquet 文件
    streaming=StreamingConfig(
        catalog_path="./data_catalog/longport",     # 数据存储目录
        fs_protocol="file",                # 本地文件系统
        flush_interval_ms=5000,            # 每 5 秒刷新一次
        max_file_size=1024 * 1024 * 100,  # 单文件最大 100MB
        rotation_mode="size",              # 按大小轮转文件
    ),

    # 注意：不配置 exec_clients - 这是数据-only 节点
)


def main():
    """启动数据采集节点"""
    print("=" * 60)
    print("数据采集进程启动 (Longport)")
    print("=" * 60)
    print("功能:")
    print(f"  1. 从 Longport ({str(MARKET)}) 获取实时市场数据")
    print("  2. 发布到 Redis Stream: market_data_longport")
    print("  3. 持久化到本地目录: ./data_catalog/longport")
    print("=" * 60)
    print(f"订阅交易对: {INSTRUMENTS}")
    print(f"市场: {str(MARKET)}")
    print("=" * 60)

    # 创建节点
    node = TradingNode(config=config)

    # 配置数据采集器（订阅哪些数据）
    config_tester = DataTesterConfig(
        instrument_ids=instrument_ids,
        # K线数据配置
        bar_types=[
            BarType.from_str(f"{instr_id}-1-MINUTE-LAST-EXTERNAL")
            for instr_id in instrument_ids
        ],
        # 订阅配置
        subscribe_instrument=False,      # 订阅交易工具信息
        subscribe_quotes=True,           # 订阅报价数据
        subscribe_trades=True,           # 订阅成交数据
        subscribe_book_deltas=True,      # 订阅订单簿增量
        subscribe_bars=True,             # 订阅K线数据
        # 指定客户端 ID，让数据引擎知道使用 LONGPORT 客户端来处理 HK 市场的订阅
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

    # 运行节点（阻塞）
    try:
        node.run()
    except KeyboardInterrupt:
        print("\n收到停止信号，正在关闭数据采集节点...")
        node.dispose()
        print("数据采集节点已停止")


if __name__ == "__main__":
    main()
