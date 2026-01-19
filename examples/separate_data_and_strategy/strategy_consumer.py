#!/usr/bin/env python3
"""
策略交易进程 - 从 Redis 接收数据，运行策略，执行交易
可以随时重启而不影响数据采集
"""

from nautilus_trader.adapters.binance import (
    BINANCE,
    BinanceExecClientConfig,
    BinanceLiveExecClientFactory,
)
from nautilus_trader.config import (
    DatabaseConfig,
    LiveDataEngineConfig,
    LoggingConfig,
    MessageBusConfig,
    TradingNodeConfig,
)
from nautilus_trader.live.node import TradingNode
from nautilus_trader.model.identifiers import ClientId, InstrumentId, TraderId, Symbol

# ============================================
# 简单示例策略
# ============================================

from nautilus_trader.core.message import Event
from nautilus_trader.model.data import QuoteTick
from nautilus_trader.model.enums import OrderSide
from nautilus_trader.model.orders import MarketOrder
from nautilus_trader.trading import Strategy
from nautilus_trader.adapters.longport import LONGPORT


class SimpleMarketMakingStrategy(Strategy):
    """
    简单的做市策略示例
    从 Redis 接收报价数据，发送订单到执行客户端
    """

    def __init__(self, instrument_id: str, trade_size: float = 0.001):
        super().__init__()
        self.instrument_id_str = instrument_id
        self.instrument_id = InstrumentId.from_str(instrument_id)
        self.trade_size = trade_size
        self.order_count = 0

    def on_start(self):
        """策略启动时调用"""
        self.log.info(f"策略启动 - 交易对: {self.instrument_id_str}")

        # 获取交易工具
        instrument = self.cache.instrument(self.instrument_id)
        if instrument is None:
            self.log.error(f"找不到交易工具: {self.instrument_id_str}")
            return

        self.log.info(f"交易工具信息: {instrument}")

        # 订阅报价数据
        self.subscribe_quote_ticks(instrument.id)
        self.log.info(f"已订阅报价数据: {instrument.id}")

    def on_stop(self):
        """策略停止时调用"""
        self.log.info(f"策略停止 - 共发送 {self.order_count} 个订单")

    def on_quote_tick(self, tick: QuoteTick):
        """
        收到报价数据时调用
        这里的数据来自 Redis Stream，由数据采集进程发布
        """
        # 只处理每 100 个报价，避免日志过多
        self.order_count += 1
        if self.order_count % 100 == 0:
            self.log.info(
                f"收到报价 #{self.order_count}: "
                f"Bid={tick.bid_price} Ask={tick.ask_price} "
                f"Spread={tick.ask_price - tick.bid_price}"
            )

        # 这里可以添加你的交易逻辑
        # 例如：当价差大于某个阈值时下单
        # spread = tick.ask_price - tick.bid_price
        # if spread > self.threshold:
        #     self.submit_order(
        #         MarketOrder(
        #             trader_id=self.trader_id,
        #             strategy_id=self.id,
        #             instrument_id=tick.instrument_id,
        #             order_side=OrderSide.BUY,
        #             quantity=self.trade_size,
        #         )
        #     )

    def on_event(self, event: Event):
        """处理其他事件"""
        pass


# ============================================
# 策略交易节点配置
# ============================================

config = TradingNodeConfig(
    trader_id=TraderId("STRATEGY-CONSUMER-001"),

    logging=LoggingConfig(
        log_level="INFO",
        log_colors=True,
    ),

    # Message Bus 配置 - 从 Redis 订阅数据
    message_bus=MessageBusConfig(
        # Redis 后端配置（必须与数据采集进程相同）
        database=DatabaseConfig(
            type="redis",
            host="localhost",
            port=6379,
            timeout=20,
        ),

        # 声明要订阅的外部流
        external_streams=["market_data_longport"],  # 订阅数据采集进程发布的 "market_data" stream

        encoding="msgpack",                # 必须与数据采集进程一致
    ),

    # 数据引擎配置 - 声明外部数据客户端
    data_engine=LiveDataEngineConfig(
        # 声明 BINANCE 是外部客户端（数据来自 Redis，不是直接连接）
        external_clients=[ClientId(LONGPORT)],
        # 注意：这里不需要配置真正的 data_clients
    ),

    # 执行客户端配置 - 连接真实的交易所执行交易
    exec_clients={
        BINANCE: BinanceExecClientConfig(
            api_key="your_api_key",
            api_secret="your_api_secret",
            testnet=True,  # 使用测试网
            # account_type="SPOT",  # 可选：指定账户类型
        ),
    },

    # 可选：加载之前保存的策略状态
    # load_state=True,
)


def main():
    """启动策略交易节点"""
    print("=" * 60)
    print("策略交易进程启动")
    print("=" * 60)
    print("功能:")
    print("  1. 从 Redis Stream: market_data 接收数据")
    print("  2. 运行交易策略")
    print("  3. 发送订单到 Binance")
    print("=" * 60)

    # 创建节点
    node = TradingNode(config=config)

    # 注册执行客户端工厂
    # node.add_exec_client_factory(BINANCE, BinanceLiveExecClientFactory)

    # 添加策略
    instrument_id = "AAPL.US"  # 根据实际交易所调整格式
    strategy = SimpleMarketMakingStrategy(
        instrument_id=instrument_id,
        trade_size=0.001,
    )
    node.trader.add_strategy(strategy)

    # 构建节点
    node.build()

    # 运行节点（阻塞）
    try:
        node.run()
    except KeyboardInterrupt:
        print("\n收到停止信号，正在关闭策略交易节点...")

        # 可选：保存策略状态
        # node.trader.save()

        node.dispose()
        print("策略交易进程已停止")


if __name__ == "__main__":
    main()
