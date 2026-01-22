#!/usr/bin/env python3
"""
币安历史数据获取工具 - 使用项目内部 HTTP client

使用 Nautilus 项目内部的 Binance HTTP client 获取数据，
并转换为 Nautilus 格式保存到 ParquetDataCatalog。

优势:
- 使用项目内部统一的 HTTP client
- 完整的 SBE 编码支持
- 自动处理速率限制
- 与项目其他组件无缝集成

Usage:
    # 获取 BTCUSDT 现货最近 30 天的 1 小时 K 线
    python binance_data_fetcher_internal.py klines --symbol BTCUSDT --interval 1h --days 30

    # 获取 ETHUSDT 现货最近 1000 条成交
    python binance_data_fetcher_internal.py trades --symbol ETHUSDT --limit 1000

    # 查看已保存的数据
    python binance_data_fetcher_internal.py list --catalog ./binance_catalog
"""

import argparse
import logging
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import List, Optional
import asyncio

from nautilus_trader.adapters.binance import get_cached_binance_http_client
from nautilus_trader.adapters.binance.common.constants import BINANCE_VENUE
from nautilus_trader.adapters.binance.common.enums import (
    BinanceAccountType,
    BinanceKlineInterval,
)
from nautilus_trader.adapters.binance.http.market import BinanceMarketHttpAPI
from nautilus_trader.adapters.binance.spot.providers import BinanceSpotInstrumentProvider
from nautilus_trader.common.component import LiveClock
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.core.datetime import millis_to_nanos
from nautilus_trader.model.data import Bar, TradeTick, BarSpecification, BarType
from nautilus_trader.model.enums import BarAggregation, PriceType, AggregationSource
from nautilus_trader.model.identifiers import InstrumentId, Symbol, TradeId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.persistence.catalog.parquet import ParquetDataCatalog


# ============================================================================
# 配置日志
# ============================================================================

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(sys.stdout),
        logging.FileHandler('binance_fetcher_internal.log')
    ]
)
logger = logging.getLogger(__name__)


# ============================================================================
# 配置参数 - 在代码中直接修改
# ============================================================================

# 数据获取配置
DATA_CONFIG = {
    # 默认交易对
    "symbol": "BTCUSDT",

    # 默认 K 线周期
    "interval": "1m",

    # 默认获取天数
    "days": 365,

    # 默认成交数据数量
    "limit": 1000,

    # 数据保存路径
    "output_path": "/Volumes/T9/data/binance_catalog",

    # 是否使用备用 URL（适合国内访问）
    "use_backup_url": False,

    # 请求超时时间（秒）
    "timeout": 30,

    # 是否为测试网
    "is_testnet": False,

    # 代理配置（国内访问需要）
    # 格式: "http://127.0.0.1:7890" 或 "socks5://127.0.0.1:1080"
    # 留空则不使用代理
    "proxy_url": "http://127.0.0.1:7897",
    # "proxy_url": "http://127.0.0.1:7890",  # 示例：本地 HTTP 代理
    # "proxy_url": "socks5://127.0.0.1:1080",  # 示例：本地 SOCKS5 代理
}


# ============================================================================
# 数据转换器
# ============================================================================

class BinanceDataConverter:
    """将币安数据转换为 Nautilus 格式"""

    @staticmethod
    def parse_interval(interval_str: str) -> BinanceKlineInterval:
        """
        解析 K 线间隔字符串

        Returns:
            BinanceKlineInterval: 币安 K 线间隔枚举
        """
        interval_map = {
            "1s": BinanceKlineInterval.SECOND_1,
            "1m": BinanceKlineInterval.MINUTE_1,
            "3m": BinanceKlineInterval.MINUTE_3,
            "5m": BinanceKlineInterval.MINUTE_5,
            "15m": BinanceKlineInterval.MINUTE_15,
            "30m": BinanceKlineInterval.MINUTE_30,
            "1h": BinanceKlineInterval.HOUR_1,
            "2h": BinanceKlineInterval.HOUR_2,
            "4h": BinanceKlineInterval.HOUR_4,
            "6h": BinanceKlineInterval.HOUR_6,
            "8h": BinanceKlineInterval.HOUR_8,
            "12h": BinanceKlineInterval.HOUR_12,
            "1d": BinanceKlineInterval.DAY_1,
            "3d": BinanceKlineInterval.DAY_3,
            "1w": BinanceKlineInterval.WEEK_1,
            "1M": BinanceKlineInterval.MONTH_1,
        }

        return interval_map.get(interval_str, BinanceKlineInterval.HOUR_1)

    @staticmethod
    def parse_interval_to_bar_spec(interval_str: str) -> tuple:
        """
        解析 K 线间隔字符串为 BarSpecification 参数

        Returns:
            tuple: (聚合类型, 数量)
        """
        interval_map = {
            "1m": (BarAggregation.MINUTE, 1),
            "3m": (BarAggregation.MINUTE, 3),
            "5m": (BarAggregation.MINUTE, 5),
            "15m": (BarAggregation.MINUTE, 15),
            "30m": (BarAggregation.MINUTE, 30),
            "1h": (BarAggregation.HOUR, 1),
            "2h": (BarAggregation.HOUR, 2),
            "4h": (BarAggregation.HOUR, 4),
            "6h": (BarAggregation.HOUR, 6),
            "8h": (BarAggregation.HOUR, 8),
            "12h": (BarAggregation.HOUR, 12),
            "1d": (BarAggregation.DAY, 1),
            "3d": (BarAggregation.DAY, 3),
            "1w": (BarAggregation.WEEK, 1),
            "1M": (BarAggregation.MONTH, 1),
        }

        return interval_map.get(interval_str, (BarAggregation.HOUR, 1))

    @staticmethod
    def create_bar_type(instrument_id: InstrumentId, interval: str) -> BarType:
        """创建 BarType"""
        aggregation, step = BinanceDataConverter.parse_interval_to_bar_spec(interval)

        return BarType(
            instrument_id=instrument_id,
            bar_spec=BarSpecification(
                step=step,
                aggregation=aggregation,
                price_type=PriceType.LAST,
            ),
            aggregation_source=AggregationSource.EXTERNAL,
        )


# ============================================================================
# Catalog 管理器
# ============================================================================

class BinanceCatalogManager:
    """币安数据 Catalog 管理器"""

    def __init__(self, catalog_path: str = DATA_CONFIG["output_path"]):
        self.catalog_path = Path(catalog_path)
        self.catalog_path.mkdir(parents=True, exist_ok=True)
        self.catalog = ParquetDataCatalog(str(self.catalog_path))

    def save_instruments(self, instruments: List):
        """保存交易对"""
        self.catalog.write_data(instruments)
        logger.info(f"已保存 {len(instruments)} 个交易对")

    def save_bars(self, bars: List[Bar]):
        """保存 K 线数据"""
        if bars:
            self.catalog.write_data(bars)
            logger.info(f"已保存 {len(bars)} 条 K 线数据")

    def save_trades(self, trades: List[TradeTick]):
        """保存成交数据"""
        if trades:
            self.catalog.write_data(trades)
            logger.info(f"已保存 {len(trades)} 条成交数据")

    def list_data(self):
        """列出 catalog 中的所有数据"""
        print("\n" + "="*80)
        print(f"Catalog: {self.catalog_path.absolute()}")
        print("="*80)

        # 交易对
        instruments = self.catalog.instruments()
        print(f"\n交易对 ({len(instruments)}):")
        for inst in instruments:
            print(f"  - {inst.id}")

        # K 线数据
        try:
            bars = self.catalog.bars()
            print(f"\nK 线数据: {len(bars)} 条")
            if bars:
                print(f"  时间范围: {bars[0].ts_event} 到 {bars[-1].ts_event}")
        except Exception as e:
            logger.warning(f"无法读取 K 线数据: {e}")

        # 成交数据
        try:
            trades = self.catalog.trade_ticks()
            print(f"\n成交数据: {len(trades)} 条")
            if trades:
                print(f"  时间范围: {trades[0].ts_event} 到 {trades[-1].ts_event}")
        except Exception as e:
            logger.warning(f"无法读取成交数据: {e}")

        print("="*80 + "\n")


# ============================================================================
# 主程序
# ============================================================================

async def fetch_instrument(
    symbol: str,
    client,
) -> Optional:
    """从币安加载交易对信息"""
    clock = LiveClock()

    provider = BinanceSpotInstrumentProvider(
        client=client,
        clock=clock,
        config=InstrumentProviderConfig(load_all=True),
    )

    await provider.load_all_async()

    # 构造 instrument_id
    instrument_id = InstrumentId(Symbol(symbol), BINANCE_VENUE)

    instrument = provider.find(instrument_id)

    if not instrument:
        # 如果找不到，尝试带 -PERP 后缀
        instrument_id = InstrumentId(Symbol(f"{symbol}-PERP"), BINANCE_VENUE)
        instrument = provider.find(instrument_id)

    return instrument


async def fetch_klines(args):
    """获取 K 线数据"""
    logger.info("="*80)
    logger.info("开始获取 K 线数据 (使用项目内部 HTTP client)")
    logger.info("="*80)

    # 从配置读取参数
    symbol = DATA_CONFIG["symbol"]
    interval = DATA_CONFIG["interval"]
    days = DATA_CONFIG["days"]

    # 创建 HTTP client
    clock = LiveClock()

    # base_url 可以设置为备用 URL
    base_url = None
    if DATA_CONFIG["use_backup_url"]:
        base_url = "https://api1.binance.com"  # 备用 URL

    client = get_cached_binance_http_client(
        clock=clock,
        account_type=BinanceAccountType.SPOT,
        base_url=base_url,
        is_testnet=DATA_CONFIG["is_testnet"],
        proxy_url=DATA_CONFIG.get("proxy_url"),  # 添加代理配置
    )

    # 创建市场 API
    market_api = BinanceMarketHttpAPI(
        client=client,
        account_type=BinanceAccountType.SPOT,
    )

    # 获取交易对信息
    logger.info(f"\n加载交易对信息: {symbol}...")
    instrument = await fetch_instrument(symbol, client)

    if not instrument:
        logger.error(f"无法找到交易对: {symbol}")
        logger.error("尝试使用通用交易对定义...")
        # 使用测试交易对作为后备
        from nautilus_trader.test_kit.providers import TestInstrumentProvider
        instrument = TestInstrumentProvider.btcusdt_binance() if "BTC" in symbol.upper() else None

    if not instrument:
        logger.error("无法获取交易对信息，退出")
        return

    logger.info(f"交易对: {instrument.id}")

    # 创建 BarType
    converter = BinanceDataConverter()
    bar_type = converter.create_bar_type(instrument.id, interval)
    logger.info(f"BarType: {bar_type}")

    # 计算时间范围
    now = datetime.now(timezone.utc)
    end_time = now
    start_time = now - timedelta(days=days)

    logger.info(f"时间范围: {start_time} 到 {end_time}")

    # 转换为毫秒时间戳
    start_time_ms = int(start_time.timestamp() * 1000)
    end_time_ms = int(end_time.timestamp() * 1000)

    # 获取 K 线数据 - 使用 BinanceMarketHttpAPI
    logger.info("\n开始获取 K 线数据...")
    binance_interval = converter.parse_interval(interval)

    binance_bars = await market_api.request_binance_bars(
        bar_type=bar_type,
        interval=binance_interval,
        limit=None,
        start_time=start_time_ms,
        end_time=end_time_ms,
    )

    if not binance_bars:
        logger.error("未获取到任何数据")
        logger.error("可能的原因:")
        logger.error("  1. 网络连接问题")
        logger.error("  2. 参数错误（交易对不存在）")
        logger.error("  3. 时间范围问题（币安只保留最近的数据）")
        return

    logger.info(f"成功获取 {len(binance_bars)} 条 BinanceBar")

    # BinanceBar 已经是 Bar 的子类，可以直接使用
    bars: List[Bar] = binance_bars  # type: ignore

    # 保存到 catalog
    logger.info("\n保存到 Data Catalog...")
    catalog_manager = BinanceCatalogManager()

    catalog_manager.save_instruments([instrument])
    catalog_manager.save_bars(bars)

    # 显示统计
    logger.info("\n" + "="*80)
    logger.info("数据统计")
    logger.info("="*80)
    logger.info(f"交易对: {instrument.id}")
    logger.info(f"K线周期: {interval}")
    logger.info(f"K线数量: {len(bars)}")
    if bars:
        logger.info(f"时间范围: {datetime.fromtimestamp(bars[0].ts_event / 1_000_000_000, tz=timezone.utc)} 到 {datetime.fromtimestamp(bars[-1].ts_event / 1_000_000_000, tz=timezone.utc)}")
    logger.info(f"保存路径: {Path(DATA_CONFIG['output_path']).absolute()}")
    logger.info("="*80)


async def fetch_trades(args):
    """获取成交数据"""
    logger.info("="*80)
    logger.info("开始获取成交数据 (使用项目内部 HTTP client)")
    logger.info("="*80)

    # 从配置读取参数
    symbol = DATA_CONFIG["symbol"]
    limit = DATA_CONFIG["limit"]

    # 创建 HTTP client
    clock = LiveClock()

    base_url = None
    if DATA_CONFIG["use_backup_url"]:
        base_url = "https://api1.binance.com"

    client = get_cached_binance_http_client(
        clock=clock,
        account_type=BinanceAccountType.SPOT,
        base_url=base_url,
        is_testnet=DATA_CONFIG["is_testnet"],
        proxy_url=DATA_CONFIG.get("proxy_url"),  # 添加代理配置
    )

    # 创建市场 API
    market_api = BinanceMarketHttpAPI(
        client=client,
        account_type=BinanceAccountType.SPOT,
    )

    # 获取交易对
    logger.info(f"\n加载交易对信息: {symbol}...")
    instrument = await fetch_instrument(symbol, client)

    if not instrument:
        logger.error(f"无法找到交易对: {symbol}")
        return

    logger.info(f"交易对: {instrument.id}")

    # 获取成交数据 - 使用 BinanceMarketHttpAPI
    logger.info(f"\n开始获取成交数据 (limit={limit})...")
    trades = await market_api.request_trade_ticks(
        instrument_id=instrument.id,
        limit=limit,
    )

    if not trades:
        logger.error("未获取到任何数据")
        return

    logger.info(f"成功获取 {len(trades)} 条成交数据")

    # 保存
    logger.info("\n保存到 Data Catalog...")
    catalog_manager = BinanceCatalogManager()

    catalog_manager.save_instruments([instrument])
    catalog_manager.save_trades(trades)

    # 统计
    logger.info("\n" + "="*80)
    logger.info("数据统计")
    logger.info("="*80)
    logger.info(f"交易对: {instrument.id}")
    logger.info(f"成交数量: {len(trades)}")
    if trades:
        logger.info(f"时间范围: {datetime.fromtimestamp(trades[0].ts_event / 1_000_000_000, tz=timezone.utc)} 到 {datetime.fromtimestamp(trades[-1].ts_event / 1_000_000_000, tz=timezone.utc)}")
    logger.info(f"保存路径: {Path(DATA_CONFIG['output_path']).absolute()}")
    logger.info("="*80)


def list_catalog(args):
    """列出 catalog 内容"""
    catalog_manager = BinanceCatalogManager()
    catalog_manager.list_data()


def main():
    parser = argparse.ArgumentParser(
        description="币安历史数据获取工具 - 使用项目内部 HTTP client",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 获取现货 K 线（使用 DATA_CONFIG 中的配置）
  %(prog)s klines

  # 获取成交数据
  %(prog)s trades

  # 查看已保存的数据
  %(prog)s list

配置:
  所有参数都在代码中的 DATA_CONFIG 字典中配置，
  无需通过命令行传入。直接修改 DATA_CONFIG 即可。

  国内访问配置:
  - proxy_url: 设置代理地址，如 "http://127.0.0.1:7890"
  - use_backup_url: 使用备用端点 (api1.binance.com)
  - is_testnet: 使用测试网 (testnet.binance.vision)

网络问题:
  如果无法访问 api.binance.com，建议按顺序尝试:
  1. 设置 proxy_url (推荐)
  2. 设置 use_backup_url = True
  3. 设置 is_testnet = True

内部实现:
  - 使用 get_cached_binance_http_client 获取 HTTP client
  - 使用 BinanceMarketHttpAPI 进行数据查询
  - 支持完整的 SBE 编码
  - 自动处理速率限制
  - 支持代理配置
        """
    )

    subparsers = parser.add_subparsers(dest="command", help="命令")

    # K 线命令
    klines_parser = subparsers.add_parser("klines", help="获取 K 线数据")

    # 成交命令
    trades_parser = subparsers.add_parser("trades", help="获取成交数据")

    # 列表命令
    list_parser = subparsers.add_parser("list", help="列出 catalog 内容")

    args = parser.parse_args()

    if args.command == "klines":
        asyncio.run(fetch_klines(args))
    elif args.command == "trades":
        asyncio.run(fetch_trades(args))
    elif args.command == "list":
        list_catalog(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
