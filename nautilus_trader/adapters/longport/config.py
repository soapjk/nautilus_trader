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
Longport adapter configuration - wrapping Rust implementations.

This module provides Python configuration classes that wrap the high-performance
Rust implementations of the Longport adapter configurations.
"""

from nautilus_trader.config import InstrumentProviderConfig, NautilusConfig, RoutingConfig


class LongportDataClientConfig(NautilusConfig, frozen=True):
    """
    Configuration for the Longport data client.

    This class wraps the Rust implementation and adds Python-specific fields.
    """

    app_key: str | None = None
    app_secret: str | None = None
    access_token: str | None = None
    markets: list[str] = []  # Store as list of strings like ["HK", "US", "CN"]
    http_timeout_secs: int | None = None
    http_url: str | None = None
    ws_url: str | None = None
    instrument_provider: InstrumentProviderConfig = InstrumentProviderConfig()
    routing: RoutingConfig = RoutingConfig()


class LongportExecClientConfig(NautilusConfig, frozen=True):
    """
    Configuration for the Longport execution client.

    This class wraps the Rust implementation and adds Python-specific fields.
    """

    trader_id: str
    account_id: str
    app_key: str | None = None
    app_secret: str | None = None
    access_token: str | None = None
    markets: list[str] = []  # Store as list of strings like ["HK", "US", "CN"]
    http_timeout_secs: int | None = None
    http_url: str | None = None
    ws_url: str | None = None
    instrument_provider: InstrumentProviderConfig = InstrumentProviderConfig()
    routing: RoutingConfig = RoutingConfig()


__all__ = [
    "LongportDataClientConfig",
    "LongportExecClientConfig",
]
