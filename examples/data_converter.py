#!/usr/bin/env python3
# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

"""
数据转换脚本：将本地历史数据转换为 NautilusTrader 回测格式。

使用示例:
    python data_converter.py --type bars --input data.csv --output bars.parquet
    python data_converter.py --type trades --input trades.csv --output trades.parquet
"""

import argparse
from pathlib import Path
from typing import Literal

import pandas as pd

from nautilus_trader.core.datetime import dt_to_unix_nanos
from nautilus_trader.model.identifiers import InstrumentId, Symbol, Venue
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.persistence.loaders import CSVBarDataLoader, CSVTickDataLoader
from nautilus_trader.persistence.wranglers import BarDataWrangler, TradeTickDataWrangler


def convert_bars_csv_to_parquet(
    input_path: str,
    output_path: str,
    instrument_id: str,
    price_precision: int = 2,
    size_precision: int = 8,
    timestamp_format: str = "mixed",
) -> None:
    """
    将 Bar CSV 数据转换为 Nautilus 格式的 Parquet 文件。

    Parameters
    ----------
    input_path : str
        输入 CSV 文件路径
    output_path : str
        输出 Parquet 文件路径
    instrument_id : str
        交易对 ID (格式: SYMBOL.VENUE)
    price_precision : int
        价格精度 (小数位数)
    size_precision : int
        数量精度 (小数位数)
    timestamp_format : str
        时间戳格式 ("mixed", "ISO8601", 或自定义格式)
    """
    print(f"读取 CSV 文件: {input_path}")

    # 读取 CSV
    df = pd.read_csv(
        input_path,
        index_col="timestamp",
        parse_dates=True,
    )

    # 确保 UTC 时区
    if df.index.tz is None:
        df.index = df.index.tz_localize("UTC")
    else:
        df.index = df.index.tz_convert("UTC")

    print(f"数据行数: {len(df)}")
    print(f"时间范围: {df.index.min} 到 {df.index.max}")
    print(f"\n数据预览:\n{df.head()}")

    # 转换为 Parquet
    output_file = Path(output_path)
    output_file.parent.mkdir(parents=True, exist_ok=True)

    df.to_parquet(output_path)
    print(f"\n✓ 数据已保存到: {output_path}")

    # 生成使用示例代码
    instrument = InstrumentId.from_str(instrument_id)
    print(f"\n使用示例:")
    print(f"```python")
    print(f"from nautilus_trader.persistence.wranglers import BarDataWrangler")
    print(f"from nautilus_trader.model.data import BarType")
    print(f"from nautilus_trader.persistence.loaders import ParquetBarDataLoader")
    print(f"")
    print(f"# 加载数据")
    print(f"df = ParquetBarDataLoader.load('{output_path}')")
    print(f"")
    print(f"# 创建 wrangler")
    print(f"bar_type = BarType.from_str('{instrument.symbol.value}/{instrument.venue.value}-1-MINUTE-BID-EXTERNAL')")
    print(f"wrangler = BarDataWrangler(bar_type=bar_type, instrument=instrument)")
    print(f"")
    print(f"# 转换为 Bar 对象")
    print(f"bars = wrangler.process(data=df)")
    print(f"```")


def convert_trades_csv_to_parquet(
    input_path: str,
    output_path: str,
    instrument_id: str,
    price_precision: int = 2,
    size_precision: int = 8,
) -> None:
    """
    将 Trade Tick CSV 数据转换为 Nautilus 格式的 Parquet 文件。

    Parameters
    ----------
    input_path : str
        输入 CSV 文件路径
    output_path : str
        输出 Parquet 文件路径
    instrument_id : str
        交易对 ID (格式: SYMBOL.VENUE)
    price_precision : int
        价格精度
    size_precision : int
        数量精度
    """
    print(f"读取 CSV 文件: {input_path}")

    # 读取 CSV
    df = pd.read_csv(
        input_path,
        parse_dates=["timestamp"],
    )

    # 确保 UTC 时区
    if df["timestamp"].dt.tz is None:
        df["timestamp"] = df["timestamp"].dt.tz_localize("UTC")
    else:
        df["timestamp"] = df["timestamp"].dt.tz_convert("UTC")

    # 设置时间戳为索引
    df = df.set_index("timestamp")

    print(f"数据行数: {len(df)}")
    print(f"时间范围: {df.index.min} 到 {df.index.max}")
    print(f"\n数据预览:\n{df.head()}")

    # 转换为 Parquet
    output_file = Path(output_path)
    output_file.parent.mkdir(parents=True, exist_ok=True)

    df.to_parquet(output_path)
    print(f"\n✓ 数据已保存到: {output_path}")


def validate_csv_format(file_path: str, data_type: Literal["bars", "trades"]) -> None:
    """
    验证 CSV 文件格式是否符合 NautilusTrader 要求。

    Parameters
    ----------
    file_path : str
        CSV 文件路径
    data_type : str
        数据类型 ("bars" 或 "trades")
    """
    print(f"\n验证文件: {file_path}")

    if data_type == "bars":
        required_columns = ["open", "high", "low", "close"]
        optional_columns = ["volume"]
    else:  # trades
        required_columns = ["price", "quantity"]
        optional_columns = ["trade_id", "side", "buyer_maker"]

    # 读取 CSV (只读取前几行)
    df = pd.read_csv(file_path, nrows=5)

    print(f"找到的列: {list(df.columns)}")

    # 检查必需列
    missing = [col for col in required_columns if col not in df.columns]
    if missing:
        print(f"✗ 缺少必需列: {missing}")
        return False

    print(f"✓ 包含所有必需列: {required_columns}")

    # 检查可选列
    found_optional = [col for col in optional_columns if col in df.columns]
    if found_optional:
        print(f"✓ 包含可选列: {found_optional}")

    return True


def main() -> None:
    """主函数。"""
    parser = argparse.ArgumentParser(
        description="将本地历史数据转换为 NautilusTrader 回测格式"
    )
    parser.add_argument(
        "--type",
        type=str,
        choices=["bars", "trades", "validate"],
        required=True,
        help="数据类型或验证模式",
    )
    parser.add_argument("--input", type=str, required=True, help="输入 CSV 文件路径")
    parser.add_argument("--output", type=str, help="输出 Parquet 文件路径 (仅转换模式)")
    parser.add_argument(
        "--instrument-id",
        type=str,
        default="BTCUSDT.BINANCE",
        help="交易对 ID (格式: SYMBOL.VENUE)",
    )
    parser.add_argument(
        "--price-precision",
        type=int,
        default=2,
        help="价格精度 (默认: 2)",
    )
    parser.add_argument(
        "--size-precision",
        type=int,
        default=8,
        help="数量精度 (默认: 8)",
    )
    parser.add_argument(
        "--timestamp-format",
        type=str,
        default="mixed",
        help="时间戳格式 (默认: mixed)",
    )

    args = parser.parse_args()

    if args.type == "validate":
        # 验证模式
        data_type = "bars" if "bar" in args.input.lower() else "trades"
        validate_csv_format(args.input, data_type)

    elif args.type == "bars":
        if not args.output:
            parser.error("--output 参数在转换模式下是必需的")

        convert_bars_csv_to_parquet(
            input_path=args.input,
            output_path=args.output,
            instrument_id=args.instrument_id,
            price_precision=args.price_precision,
            size_precision=args.size_precision,
            timestamp_format=args.timestamp_format,
        )

    elif args.type == "trades":
        if not args.output:
            parser.error("--output 参数在转换模式下是必需的")

        convert_trades_csv_to_parquet(
            input_path=args.input,
            output_path=args.output,
            instrument_id=args.instrument_id,
            price_precision=args.price_precision,
            size_precision=args.size_precision,
        )


if __name__ == "__main__":
    main()
