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
Longport adapter configuration.

Configuration classes for the Longport adapter, supporting the official
Longport OpenAPI SDK endpoints.

References:
- https://github.com/longportapp/openapi
- https://longportapp.github.io/openapi
"""

from nautilus_trader.config import InstrumentProviderConfig, NautilusConfig, RoutingConfig

# Default Longport API endpoints (from official SDK)
LONGPORT_HTTP_URL_DEFAULT = "https://openapi.longportapp.com"
LONGPORT_QUOTE_WS_URL_DEFAULT = "wss://openapi-quote.longportapp.com/v2"
LONGPORT_TRADE_WS_URL_DEFAULT = "wss://openapi-trade.longportapp.com/v2"


class LongportDataClientConfig(NautilusConfig, frozen=True):
    """
    Configuration for the Longport data client.

    Parameters
    ----------
    app_key : str, optional
        The Longport app key. Can also be set via LONGPORT_APP_KEY env var.
    app_secret : str, optional
        The Longport app secret. Can also be set via LONGPORT_APP_SECRET env var.
    access_token : str, optional
        The Longport access token. Can also be set via LONGPORT_ACCESS_TOKEN env var.
    markets : list[str], default []
        The markets to connect to (e.g., ["HK", "US", "CN"]).
    http_url : str, optional
        The HTTP endpoint URL. Default is https://openapi.longportapp.com
        Can also be set via LONGPORT_HTTP_URL env var.
    quote_ws_url : str, optional
        The quote WebSocket endpoint URL. Default is wss://openapi-quote.longportapp.com/v2
        Can also be set via LONGPORT_QUOTE_WS_URL env var.
    trade_ws_url : str, optional
        The trade WebSocket endpoint URL (for order status updates).
        Default is wss://openapi-trade.longportapp.com/v2
        Can also be set via LONGPORT_TRADE_WS_URL env var.
    http_timeout_secs : int, optional
        HTTP request timeout in seconds.
    instrument_provider : InstrumentProviderConfig, default InstrumentProviderConfig()
        Configuration for the instrument provider.
    routing : RoutingConfig, default RoutingConfig()
        Routing configuration.

    Notes
    -----
    The official Longport SDK supports separate WebSocket endpoints for quotes and trades:
    - Quote WebSocket: Real-time market data
    - Trade WebSocket: Order status and execution updates

    Environment Variables
    ---------------------
    If a config value is not provided, the SDK will check these environment variables:
    - LONGPORT_HTTP_URL: HTTP endpoint URL
    - LONGPORT_QUOTE_WS_URL: Quote WebSocket URL
    - LONGPORT_TRADE_WS_URL: Trade WebSocket URL
    - LONGPORT_APP_KEY: App key
    - LONGPORT_APP_SECRET: App secret
    - LONGPORT_ACCESS_TOKEN: Access token
    """

    app_key: str | None = None
    app_secret: str | None = None
    access_token: str | None = None
    markets: list[str] = []  # Store as list of strings like ["HK", "US", "CN"]
    http_timeout_secs: int | None = None
    http_url: str | None = None
    quote_ws_url: str | None = None
    trade_ws_url: str | None = None
    instrument_provider: InstrumentProviderConfig = InstrumentProviderConfig()
    routing: RoutingConfig = RoutingConfig()

    def get_app_key(self) -> str | None:
        """Get the app key from config or environment."""
        return self.app_key

    def get_app_secret(self) -> str | None:
        """Get the app secret from config or environment."""
        return self.app_secret

    def get_access_token(self) -> str | None:
        """Get the access token from config or environment."""
        return self.access_token


class LongportExecClientConfig(NautilusConfig, frozen=True):
    """
    Configuration for the Longport execution client.

    Parameters
    ----------
    trader_id : str
        The trader ID for the execution client.
    account_id : str
        The account ID for the execution client.
    app_key : str, optional
        The Longport app key. Can also be set via LONGPORT_APP_KEY env var.
    app_secret : str, optional
        The Longport app secret. Can also be set via LONGPORT_APP_SECRET env var.
    access_token : str, optional
        The Longport access token. Can also be set via LONGPORT_ACCESS_TOKEN env var.
    markets : list[str], default []
        The markets to connect to (e.g., ["HK", "US", "CN"]).
    http_url : str, optional
        The HTTP endpoint URL. Default is https://openapi.longportapp.com
        Can also be set via LONGPORT_HTTP_URL env var.
    quote_ws_url : str, optional
        The quote WebSocket endpoint URL. Default is wss://openapi-quote.longportapp.com/v2
        Can also be set via LONGPORT_QUOTE_WS_URL env var.
    trade_ws_url : str, optional
        The trade WebSocket endpoint URL. Default is wss://openapi-trade.longportapp.com/v2
        Can also be set via LONGPORT_TRADE_WS_URL env var.
    http_timeout_secs : int, optional
        HTTP request timeout in seconds.
    instrument_provider : InstrumentProviderConfig, default InstrumentProviderConfig()
        Configuration for the instrument provider.
    routing : RoutingConfig, default RoutingConfig()
        Routing configuration.

    Notes
    -----
    The execution client requires the trade WebSocket URL for order status updates.
    """

    trader_id: str
    account_id: str
    app_key: str | None = None
    app_secret: str | None = None
    access_token: str | None = None
    markets: list[str] = []  # Store as list of strings like ["HK", "US", "CN"]
    http_timeout_secs: int | None = None
    http_url: str | None = None
    quote_ws_url: str | None = None
    trade_ws_url: str | None = None
    instrument_provider: InstrumentProviderConfig = InstrumentProviderConfig()
    routing: RoutingConfig = RoutingConfig()

    def get_app_key(self) -> str | None:
        """Get the app key from config or environment."""
        return self.app_key

    def get_app_secret(self) -> str | None:
        """Get the app secret from config or environment."""
        return self.app_secret

    def get_access_token(self) -> str | None:
        """Get the access token from config or environment."""
        return self.access_token


__all__ = [
    "LongportDataClientConfig",
    "LongportExecClientConfig",
    "LONGPORT_HTTP_URL_DEFAULT",
    "LONGPORT_QUOTE_WS_URL_DEFAULT",
    "LONGPORT_TRADE_WS_URL_DEFAULT",
]
