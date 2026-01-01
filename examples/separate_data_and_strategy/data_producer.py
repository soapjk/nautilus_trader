#!/usr/bin/env python3
"""
数据采集进程 - 专门负责获取和发布市场数据
可以独立运行，策略进程重启不影响数据采集
"""

from nautilus_trader.adapters.binance import (
    BINANCE,
    BinanceAccountType,
    BinanceDataClientConfig,
    BinanceLiveDataClientFactory,
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
from nautilus_trader.model.identifiers import InstrumentId, TraderId
from nautilus_trader.test_kit.strategies.tester_data import DataTester
from nautilus_trader.test_kit.strategies.tester_data import DataTesterConfig


# ========================================================================
# 配置要订阅的交易对
# ========================================================================

# 账户类型: SPOT 或 USDT_FUTURES
ACCOUNT_TYPE = BinanceAccountType.SPOT  # 现货
# ACCOUNT_TYPE = BinanceAccountType.USDT_FUTURES  # U本位合约

# 交易对列表
INSTRUMENTS = [
    "BTCUSDT",   # 比特币
    # "ETHUSDT",   # 以太坊
    # "SOLUSDT",  # Solana
    # "BNBUSDT",  # BNB
]

# 根据账户类型确定交易对后缀
if ACCOUNT_TYPE == BinanceAccountType.SPOT:
    instrument_ids = [InstrumentId.from_str(f"{s}.{BINANCE}") for s in INSTRUMENTS]
else:
    # 合约市场添加 -PERP 后缀
    instrument_ids = [InstrumentId.from_str(f"{s}-PERP.{BINANCE}") for s in INSTRUMENTS]


# 数据采集节点配置
config = TradingNodeConfig(
    trader_id=TraderId("DATA-PRODUCER-001"),

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
        streams_prefix="market_data",      # Stream 名称: "market_data"
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
        BINANCE: BinanceDataClientConfig(
            api_key=None,  # 公共数据不需要 API key
            api_secret=None,
            account_type=ACCOUNT_TYPE,
            testnet=True,  # 使用测试网
            instrument_provider=InstrumentProviderConfig(
                load_ids=frozenset(instrument_ids),  # 只加载指定的交易对
            ),
        ),
    },

    # 可选：同时持久化到本地 Parquet 文件
    streaming=StreamingConfig(
        catalog_path="./data_catalog",     # 数据存储目录
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
    print("数据采集进程启动")
    print("=" * 60)
    print("功能:")
    print("  1. 从 Binance 获取实时市场数据")
    print("  2. 发布到 Redis Stream: market_data")
    print("  3. 持久化到本地目录: ./data_catalog")
    print("=" * 60)
    print(f"订阅交易对: {INSTRUMENTS}")
    print(f"账户类型: {ACCOUNT_TYPE.value}")
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
        subscribe_quotes=False,          # 订阅报价数据
        subscribe_trades=True,          # 订阅成交数据
        subscribe_book_deltas=False,     # 订阅订单簿增量
        subscribe_bars=True,            # 订阅K线数据
    )

    # 创建数据采集器
    tester = DataTester(config=config_tester)

    # 注册数据客户端工厂
    node.add_data_client_factory(BINANCE, BinanceLiveDataClientFactory)

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
