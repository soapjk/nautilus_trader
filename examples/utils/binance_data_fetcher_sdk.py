#!/usr/bin/env python3
"""
币安历史数据获取工具 - 使用官方 SDK 版本

使用 binance-connector 官方 SDK 获取现货数据，
并转换为 Nautilus 格式保存到 ParquetDataCatalog。

注意: binance-connector SDK 只支持现货 API
如需合约数据，请使用 binance_data_fetcher.py (直接 HTTP 请求)

相比直接 HTTP 请求的优势：
- 官方维护，API 更新及时
- 自动处理签名和认证
- 内置重试逻辑和错误处理
- 显示 API 限流使用情况

安装 SDK:
    pip install binance-connector

Usage:
    # 获取 BTCUSDT 现货最近 30 天的 1 小时 K 线
    python binance_data_fetcher_sdk.py klines --symbol BTCUSDT --interval 1h --days 30

    # 获取 ETHUSDT 现货最近 1000 条成交
    python binance_data_fetcher_sdk.py trades --symbol ETHUSDT --limit 1000

    # 查看已保存的数据
    python binance_data_fetcher_sdk.py list --catalog ./binance_catalog

合约数据获取:
    python binance_data_fetcher.py --futures klines --symbol BTCUSDT --interval 1h --days 30
"""

import argparse
import logging
import os
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Optional
import asyncio

from nautilus_trader.adapters.binance import get_cached_binance_http_client
from nautilus_trader.adapters.binance.common.enums import BinanceAccountType
from nautilus_trader.adapters.binance.futures.providers import BinanceFuturesInstrumentProvider
from nautilus_trader.adapters.binance.spot.providers import BinanceSpotInstrumentProvider
from nautilus_trader.common.component import LiveClock
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.core.datetime import millis_to_nanos
from nautilus_trader.model.data import Bar, TradeTick, BarSpecification, BarType
from nautilus_trader.model.enums import BarAggregation, PriceType
from nautilus_trader.model.identifiers import InstrumentId, Symbol, TradeId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.persistence.catalog.parquet import ParquetDataCatalog

from binance.spot import Spot as SpotClient
# 注意: binance-connector 只包含现货 API
# 合约 API 需要使用不同的 base_url 或者直接 HTTP 请求
# 为简化，本示例主要演示现货，合约功能建议使用直接 HTTP 方式


# ============================================================================
# 配置日志
# ============================================================================

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(sys.stdout),
        logging.FileHandler('binance_fetcher_sdk.log')
    ]
)
logger = logging.getLogger(__name__)


# ============================================================================
# 币安 API 客户端 (使用官方 SDK)
# ============================================================================

class BinanceAPIClient:
    """
    币安 API 客户端 - 使用官方 SDK

    支持:
    - 现货交易 (SpotClient)
    - U本位合约 (UMFutures)
    - 自动代理配置
    - 错误重试
    """

    # K 线间隔映射
    INTERVALS = {
        "1m": BarAggregation.MINUTE,
        "3m": BarAggregation.MINUTE,
        "5m": BarAggregation.MINUTE,
        "15m": BarAggregation.MINUTE,
        "30m": BarAggregation.MINUTE,
        "1h": BarAggregation.HOUR,
        "2h": BarAggregation.HOUR,
        "4h": BarAggregation.HOUR,
        "6h": BarAggregation.HOUR,
        "8h": BarAggregation.HOUR,
        "12h": BarAggregation.HOUR,
        "1d": BarAggregation.DAY,
        "3d": BarAggregation.DAY,
        "1w": BarAggregation.WEEK,
        "1M": BarAggregation.MONTH,
    }

    def __init__(
        self,
        account_type: BinanceAccountType = BinanceAccountType.SPOT,
        use_backup_url: bool = False,
        timeout: int = 30,
        api_key: Optional[str] = None,
        api_secret: Optional[str] = None,
    ):
        """
        初始化币安 API 客户端

        Args:
            account_type: 账户类型（现货/合约）
            use_backup_url: 是否使用备用 URL
            timeout: 请求超时时间（秒）
            api_key: API 密钥（可选，公开数据不需要）
            api_secret: API 密钥（可选，公开数据不需要）
        """
        self.account_type = account_type
        self.timeout = timeout

        # 检测代理
        proxies = None
        http_proxy = os.environ.get("HTTP_PROXY") or os.environ.get("http_proxy")
        https_proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("https_proxy")

        if http_proxy or https_proxy:
            proxies = {
                "http": http_proxy or https_proxy,
                "https": https_proxy or http_proxy,
            }
            logger.info(f"使用代理: {proxies}")

        # 基础 URL 配置
        if use_backup_url:
            spot_url = "https://api1.binance.com"
            futures_url = "https://fapi1.binance.com"
        else:
            spot_url = "https://api.binance.com"
            futures_url = "https://fapi.binance.com"

        # 创建客户端
        # 注意: binance-connector 只支持现货，合约 API 建议使用直接 HTTP 方式
        if account_type != BinanceAccountType.SPOT:
            logger.warning("binance-connector SDK 只支持现货 API")
            logger.warning("合约功能请使用 binance_data_fetcher.py (直接 HTTP 请求)")
            raise ValueError("SDK 版本只支持现货，合约请使用直接 HTTP 版本")

        self.client = SpotClient(
            api_key=api_key or "",
            api_secret=api_secret or "",
            base_url=spot_url,
            proxies=proxies,
            timeout=timeout,
            show_limit_usage=True,  # 显示 API 限流使用情况
        )
        logger.info(f"使用现货 API (SDK): {spot_url}")

    def get_klines(
        self,
        symbol: str,
        interval: str,
        start_time: Optional[int] = None,
        end_time: Optional[int] = None,
        limit: int = 500,
    ) -> List[List]:
        """
        获取 K 线数据

        SDK 会自动处理:
        - 参数验证
        - 请求签名（如果有 API key）
        - 错误重试
        - API 限流

        Args:
            symbol: 交易对，如 BTCUSDT
            interval: K线间隔，如 1m, 5m, 1h, 1d
            start_time: 开始时间戳（毫秒）
            end_time: 结束时间戳（毫秒）
            limit: 每次请求的数量，默认 500（最大 1000）

        Returns:
            List[List]: K线数据列表
        """
        params = {
            "symbol": symbol.upper(),
            "interval": interval,
            "limit": max(1, min(limit, 1000)),
        }

        if start_time:
            params["startTime"] = start_time
        if end_time:
            params["endTime"] = end_time

        try:
            # SDK 的 klines 方法返回格式与直接 HTTP 相同
            response = self.client.klines(**params)

            # SDK 返回的是字典格式（如果有错误）
            if isinstance(response, dict):
                if "code" in response:
                    logger.error(f"API 错误: {response}")
                    return []
                if "msg" in response:
                    logger.error(f"API 错误: {response['msg']}")
                    return []

            return response

        except Exception as e:
            logger.error(f"请求失败: {e}")
            logger.error("提示: 如果在国内访问，可能需要使用代理或添加 --backup-url 参数")
            return []

    def get_klines_batch(
        self,
        symbol: str,
        interval: str,
        start_time: int,
        end_time: int,
    ) -> List[List]:
        """
        批量获取 K 线数据，自动分页

        Args:
            symbol: 交易对
            interval: K线间隔
            start_time: 开始时间戳（毫秒）
            end_time: 结束时间戳（毫秒）

        Returns:
            List[List]: 所有 K 线数据
        """
        all_klines = []
        current_start = start_time

        logger.info(f"开始获取 {symbol} {interval} K线数据")
        logger.info(f"时间范围: {datetime.fromtimestamp(start_time/1000)} 到 {datetime.fromtimestamp(end_time/1000)}")

        batch_num = 1
        while current_start < end_time:
            klines = self.get_klines(
                symbol=symbol,
                interval=interval,
                start_time=current_start,
                end_time=end_time,
                limit=1000,
            )

            if not klines:
                logger.info(f"批次 {batch_num}: 没有更多数据")
                break

            all_klines.extend(klines)
            logger.info(f"批次 {batch_num}: 获取 {len(klines)} 条，累计 {len(all_klines)} 条")

            # 更新起始时间为最后一根 K 线的时间 + 1ms
            current_start = klines[-1][0] + 1

            # 如果返回的数据少于 1000 条，说明已经到最新数据
            if len(klines) < 1000:
                logger.info("已获取到最新数据")
                break

            batch_num += 1

        return all_klines

    def get_trades(
        self,
        symbol: str,
        limit: int = 1000,
        from_id: Optional[int] = None,
    ) -> List[dict]:
        """
        获取最近成交数据

        Args:
            symbol: 交易对
            limit: 数量限制，最大 1000
            from_id: 从哪个成交 ID 开始获取

        Returns:
            List[dict]: 成交数据列表
        """
        params = {
            "symbol": symbol.upper(),
            "limit": min(limit, 1000),
        }

        if from_id:
            params["fromId"] = from_id

        try:
            # SDK 只支持现货
            response = self.client.trades(**params)

            if isinstance(response, dict) and "code" in response:
                logger.error(f"API 错误: {response}")
                return []

            return response

        except Exception as e:
            logger.error(f"请求失败: {e}")
            return []

    def get_trades_batch(
        self,
        symbol: str,
        limit: int = 1000,
    ) -> List[dict]:
        """批量获取成交数据"""
        all_trades = []
        from_id = None

        while len(all_trades) < limit:
            batch_size = min(1000, limit - len(all_trades))
            trades = self.get_trades(
                symbol=symbol,
                limit=batch_size,
                from_id=from_id,
            )

            if not trades:
                break

            all_trades.extend(trades)
            logger.info(f"已获取 {len(all_trades)}/{limit} 条成交数据")

            from_id = trades[-1]["id"] + 1

            # 如果返回的数据少于请求数量，说明已经到最新
            if len(trades) < batch_size:
                break

        return all_trades

    def close(self):
        """关闭客户端"""
        # SDK 不需要显式关闭
        pass


# ============================================================================
# 数据转换器
# ============================================================================

class BinanceDataConverter:
    """将币安数据转换为 Nautilus 格式"""

    @staticmethod
    def parse_interval(interval_str: str) -> tuple:
        """
        解析 K 线间隔字符串

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
    def kline_to_bar(
        kline: List,
        instrument_id: InstrumentId,
        interval: str,
    ) -> Bar:
        """
        将币安 K 线转换为 Nautilus Bar

        K线格式: [
            open_time, open, high, low, close, volume,
            close_time, quote_volume, trades, taker_buy_base, taker_buy_quote, ignore
        ]
        """
        open_time = kline[0]
        open_price = float(kline[1])
        high_price = float(kline[2])
        low_price = float(kline[3])
        close_price = float(kline[4])
        volume = float(kline[5])

        # 解析间隔
        aggregation, step = BinanceDataConverter.parse_interval(interval)

        # 创建 BarType
        bar_type = BarType(
            instrument_id=instrument_id,
            bar_spec=BarSpecification(
                step=step,
                aggregation=aggregation,
                price_type=PriceType.LAST,
            ),
        )

        return Bar(
            bar_type=bar_type,
            open=Price(open_price, precision=8),
            high=Price(high_price, precision=8),
            low=Price(low_price, precision=8),
            close=Price(close_price, precision=8),
            volume=Quantity(volume, precision=8),
            ts_event=millis_to_nanos(open_time),
            ts_init=millis_to_nanos(open_time),
        )

    @staticmethod
    def trade_to_tick(
        trade: dict,
        instrument_id: InstrumentId,
    ) -> TradeTick:
        """
        将币安成交数据转换为 Nautilus TradeTick

        成交格式: {
            "id": 成交ID,
            "price": 价格,
            "qty": 数量,
            "quoteQty": 报价货币数量,
            "time": 成交时间,
            "isBuyerMaker": 是否为买方挂单
        }
        """
        return TradeTick(
            instrument_id=instrument_id,
            trade_id=TradeId(str(trade["id"])),
            price=Price(float(trade["price"]), precision=8),
            size=Quantity(float(trade["qty"]), precision=8),
            aggressor_side="SELL" if trade.get("isBuyerMaker", False) else "BUY",
            ts_event=millis_to_nanos(trade["time"]),
            ts_init=millis_to_nanos(trade["time"]),
        )


# ============================================================================
# Catalog 管理器
# ============================================================================

class BinanceCatalogManager:
    """币安数据 Catalog 管理器"""

    def __init__(self, catalog_path: str = "./binance_catalog"):
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
    futures: bool = False,
) -> Optional:
    """从币安加载交易对信息"""
    clock = LiveClock()

    account_type = BinanceAccountType.USDT_FUTURES if futures else BinanceAccountType.SPOT

    client = get_cached_binance_http_client(
        clock=clock,
        account_type=account_type,
        is_testnet=False,
    )

    if futures:
        provider = BinanceFuturesInstrumentProvider(
            client=client,
            clock=clock,
            config=InstrumentProviderConfig(load_all=True),
        )
    else:
        provider = BinanceSpotInstrumentProvider(
            client=client,
            clock=clock,
            config=InstrumentProviderConfig(load_all=True),
        )

    await provider.load_all_async()

    # 构造 instrument_id
    symbol_str = f"{symbol}-PERP" if futures else symbol
    instrument_id = InstrumentId(Symbol(symbol_str), "BINANCE")

    instrument = provider.find(instrument_id)

    if not instrument:
        # 如果找不到，尝试不带 -PERP 后缀
        instrument_id = InstrumentId(Symbol(symbol), "BINANCE")
        instrument = provider.find(instrument_id)

    return instrument


async def fetch_klines(args):
    """获取 K 线数据"""
    logger.info("="*80)
    logger.info("开始获取 K 线数据 (使用官方 SDK - 仅现货)")
    logger.info("="*80)

    # 创建 API 客户端 (SDK 只支持现货)
    api = BinanceAPIClient(
        account_type=BinanceAccountType.SPOT,
        use_backup_url=getattr(args, 'backup_url', False),
        timeout=getattr(args, 'timeout', 30),
    )

    # 计算时间范围
    now = datetime.now()
    end_time = int(now.timestamp() * 1000)
    start_time = int((now - timedelta(days=args.days)).timestamp() * 1000)

    logger.info(f"时间范围: {datetime.fromtimestamp(start_time/1000)} 到 {datetime.fromtimestamp(end_time/1000)}")

    # 获取 K 线数据
    klines = api.get_klines_batch(
        symbol=args.symbol,
        interval=args.interval,
        start_time=start_time,
        end_time=end_time,
    )

    if not klines:
        logger.error("未获取到任何数据")
        logger.error("可能的原因:")
        logger.error("  1. 网络连接问题（在国内可能被墙）")
        logger.error("  2. 参数错误（交易对不存在）")
        logger.error("  3. 时间范围问题（币安只保留最近的数据）")
        logger.error("\n建议:")
        logger.error("  - 添加 --backup-url 参数使用备用端点")
        logger.error("  - 配置代理：export HTTP_PROXY=http://127.0.0.1:7890")
        logger.error("  - 减少 --days 参数（如 1-7 天）")
        api.close()
        return

    logger.info(f"成功获取 {len(klines)} 条 K 线数据")

    # 加载交易对信息 (现货)
    logger.info("\n加载交易对信息...")
    instrument = await fetch_instrument(args.symbol, futures=False)

    if not instrument:
        logger.error(f"无法找到交易对: {args.symbol}")
        logger.error("尝试使用通用交易对定义...")
        # 使用测试交易对作为后备
        from nautilus_trader.test_kit.providers import TestInstrumentProvider
        instrument = TestInstrumentProvider.btcusdt_binance() if "BTC" in args.symbol else None

    if not instrument:
        logger.error("无法获取交易对信息，退出")
        api.close()
        return

    logger.info(f"交易对: {instrument.id}")

    # 转换数据
    logger.info("\n转换为 Nautilus 格式...")
    converter = BinanceDataConverter()
    bars = [
        converter.kline_to_bar(k, instrument.id, args.interval)
        for k in klines
    ]

    # 保存到 catalog
    logger.info("\n保存到 Data Catalog...")
    catalog_manager = BinanceCatalogManager(args.output)

    catalog_manager.save_instruments([instrument])
    catalog_manager.save_bars(bars)

    # 显示统计
    logger.info("\n" + "="*80)
    logger.info("数据统计")
    logger.info("="*80)
    logger.info(f"交易对: {instrument.id}")
    logger.info(f"K线周期: {args.interval}")
    logger.info(f"K线数量: {len(bars)}")
    logger.info(f"时间范围: {datetime.fromtimestamp(klines[0][0]/1000)} 到 {datetime.fromtimestamp(klines[-1][0]/1000)}")
    logger.info(f"保存路径: {Path(args.output).absolute()}")
    logger.info("="*80)

    api.close()


async def fetch_trades(args):
    """获取成交数据"""
    logger.info("="*80)
    logger.info("开始获取成交数据 (使用官方 SDK - 仅现货)")
    logger.info("="*80)

    # 创建 API 客户端 (SDK 只支持现货)
    api = BinanceAPIClient(
        account_type=BinanceAccountType.SPOT
    )

    # 获取成交数据
    trades = api.get_trades_batch(
        symbol=args.symbol,
        limit=args.limit,
    )

    if not trades:
        logger.error("未获取到任何数据")
        api.close()
        return

    logger.info(f"成功获取 {len(trades)} 条成交数据")

    # 加载交易对 (现货)
    logger.info("\n加载交易对信息...")
    instrument = await fetch_instrument(args.symbol, futures=False)

    if not instrument:
        logger.error(f"无法找到交易对: {args.symbol}")
        api.close()
        return

    logger.info(f"交易对: {instrument.id}")

    # 转换数据
    logger.info("\n转换为 Nautilus 格式...")
    converter = BinanceDataConverter()
    ticks = [
        converter.trade_to_tick(t, instrument.id)
        for t in trades
    ]

    # 保存
    logger.info("\n保存到 Data Catalog...")
    catalog_manager = BinanceCatalogManager(args.output)

    catalog_manager.save_instruments([instrument])
    catalog_manager.save_trades(ticks)

    # 统计
    logger.info("\n" + "="*80)
    logger.info("数据统计")
    logger.info("="*80)
    logger.info(f"交易对: {instrument.id}")
    logger.info(f"成交数量: {len(ticks)}")
    logger.info(f"时间范围: {datetime.fromtimestamp(trades[0]['time']/1000)} 到 {datetime.fromtimestamp(trades[-1]['time']/1000)}")
    logger.info(f"保存路径: {Path(args.output).absolute()}")
    logger.info("="*80)

    api.close()


def list_catalog(args):
    """列出 catalog 内容"""
    catalog_manager = BinanceCatalogManager(args.catalog)
    catalog_manager.list_data()


def main():
    parser = argparse.ArgumentParser(
        description="币安历史数据获取工具 - 使用官方 SDK (仅支持现货)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  # 获取现货 K 线
  %(prog)s klines --symbol BTCUSDT --interval 1h --days 7

  # 获取现货 K 线（使用备用 URL）
  %(prog)s --backup-url klines --symbol ETHUSDT --interval 15m --days 3

  # 获取成交数据
  %(prog)s trades --symbol BTCUSDT --limit 1000

  # 查看已保存的数据
  %(prog)s list --catalog ./binance_catalog

注意: 此 SDK 版本只支持现货数据
  如需合约数据，请使用: binance_data_fetcher.py --futures ...

网络问题:
  如果无法访问 api.binance.com，请尝试:
  1. 添加 --backup-url 参数使用备用端点
  2. 配置代理：export HTTP_PROXY=http://127.0.0.1:7890
  3. 使用 VPN

SDK 优势:
  - 官方维护，API 更新及时
  - 自动处理签名和认证
  - 内置重试逻辑和错误处理
  - 显示 API 限流使用情况
        """
    )

    # 全局参数
    parser.add_argument("--output", default="/Volumes/T9/data/binance_catalog", help="数据保存路径")
    parser.add_argument("--backup-url", action="store_true", help="使用备用 URL（适合国内访问）")
    parser.add_argument("--timeout", type=int, default=30, help="请求超时时间（秒），默认 30")
    parser.add_argument("--api-key", help="API 密钥（可选，公开数据不需要）")
    parser.add_argument("--api-secret", help="API 密钥（可选，公开数据不需要）")

    subparsers = parser.add_subparsers(dest="command", help="命令")

    # K 线命令
    klines_parser = subparsers.add_parser("klines", help="获取 K 线数据")
    klines_parser.add_argument("--symbol", required=True, help="交易对，如 BTCUSDT")
    klines_parser.add_argument("--interval", default="1h", choices=[
        "1m", "3m", "5m", "15m", "30m",
        "1h", "2h", "4h", "6h", "8h", "12h",
        "1d", "3d",
        "1w", "1M"
    ], help="K 线周期")
    klines_parser.add_argument("--days", type=int, default=7, help="获取最近几天（建议 1-7 天）")

    # 成交命令
    trades_parser = subparsers.add_parser("trades", help="获取成交数据")
    trades_parser.add_argument("--symbol", required=True, help="交易对")
    trades_parser.add_argument("--limit", type=int, default=1000, help="获取数量（最大 1000）")

    # 列表命令
    list_parser = subparsers.add_parser("list", help="列出 catalog 内容")
    list_parser.add_argument("--catalog", default="./binance_catalog", help="Catalog 路径")

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
