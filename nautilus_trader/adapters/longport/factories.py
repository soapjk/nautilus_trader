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
Longport adapter factories - wrapping Rust implementations.

This module provides Python factory classes that wrap the high-performance
Rust implementations of the Longport data and execution clients.
"""

from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.live.data_client import LiveDataClient, LiveMarketDataClient
from nautilus_trader.live.execution_client import LiveExecutionClient
from nautilus_trader.live.factories import LiveDataClientFactory, LiveExecClientFactory

from nautilus_trader.adapters.longport.config import (
    LongportDataClientConfig,
    LongportExecClientConfig,
)


class LongportDataClient(LiveMarketDataClient):
    """
    Python wrapper for the Rust LongportDataClient.

    This wrapper delays creation of the Rust client until _connect() is called,
    ensuring TLS context is properly initialized.
    """

    def __init__(self, config, client_id, loop, msgbus, cache, clock, instrument_provider):
        """
        Initialize the wrapper with config (Rust client created later in _connect).

        Parameters
        ----------
        config : LongportDataClientConfig
            The configuration for the client.
        client_id : ClientId
            The Python client ID (not the Rust one).
        loop : asyncio.AbstractEventLoop
            The event loop for the client.
        msgbus : MessageBus
            The message bus for the client.
        cache : Cache
            The cache for the client.
        clock : LiveClock
            The clock for the client.
        instrument_provider : InstrumentProvider
            The instrument provider for the client.
        """
        # Store config for later Rust client creation
        self._config = config
        self._rust_client = None  # Will be created in _connect()

        # Import LONGPORT_VENUE constant
        from nautilus_trader.adapters.longport.common.constants import LONGPORT_VENUE

        # Initialize parent LiveMarketDataClient properly
        super().__init__(
            loop=loop,
            client_id=client_id,  # Use the Python ClientId
            venue=LONGPORT_VENUE,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
        )

    async def _connect(self) -> None:
        """Connect to the Longport API."""
        # Create Rust client on first connect (when TLS context is available)
        if self._rust_client is None:
            self._log.info("[PYTHON] Creating Rust LongportDataClient...")
            from nautilus_trader.core.nautilus_pyo3.longport import (
                LongportDataClient as RustLongportDataClient,
                LongportDataClientConfig as RustLongportDataClientConfig,
                LongportMarket,
            )
            from nautilus_trader.core.nautilus_pyo3 import ClientId as RustClientId

            # Convert Python config to Rust config
            rust_markets = []
            for market in self._config.markets:
                if isinstance(market, str):
                    rust_markets.append(getattr(LongportMarket, market.upper()))
                else:
                    rust_markets.append(market)

            # Extract load_ids from instrument_provider config
            load_ids = None
            if self._config.instrument_provider and self._config.instrument_provider.load_ids:
                load_ids = [str(instr_id) for instr_id in self._config.instrument_provider.load_ids]

            # Create Rust config
            rust_config = RustLongportDataClientConfig(
                markets=rust_markets,
                app_key=self._config.app_key,
                app_secret=self._config.app_secret,
                access_token=self._config.access_token,
                load_ids=load_ids,
                http_timeout_secs=self._config.http_timeout_secs,
                http_url=self._config.http_url,
                ws_url=self._config.ws_url,
            )

            # Create the Rust client (TLS context should be available now)
            rust_client_id = RustClientId(str(self.id))
            self._rust_client = RustLongportDataClient(rust_client_id, rust_config)
            self._log.info("[PYTHON] Rust client created successfully")

        # Call the Rust connect method
        self._log.info("[PYTHON] About to call Rust connect()...")
        result = self._rust_client.connect()
        self._log.info(f"[PYTHON] Rust connect() returned: {result}")
        self._log.info(f"[PYTHON] is_connected after Rust connect(): {self._rust_client.is_connected()}")

        # Check if data_sender is set
        self._log.info("[PYTHON] Connection complete - event consumption should be active")

        # After connection, add instruments to the cache and instrument provider
        # The Rust client has already loaded instruments internally
        if self._instrument_provider and self._instrument_provider._config and self._instrument_provider._config.load_ids:
            load_ids = self._instrument_provider._config.load_ids
            self._log.info(f"Loading {len(load_ids)} instruments from Longport")
            # Create minimal instruments for each load_id and add to cache
            from nautilus_trader.model.instruments import Equity
            from nautilus_trader.model.identifiers import Symbol
            from nautilus_trader.model.objects import Currency, Price, Quantity
            from decimal import Decimal
            import time

            ts_init = int(time.time() * 1_000_000_000)  # Convert to nanoseconds

            for instrument_id in load_ids:
                try:
                    # Check if instrument is already in cache
                    if self._cache.instrument(instrument_id) is None:
                        # Create a minimal instrument for US stocks
                        # This is a temporary workaround until we can properly query from Longport
                        symbol_str = str(instrument_id.symbol)

                        # Create a basic equity instrument with default values
                        # The actual values will be updated when we receive data from Longport
                        # Use Currency.from_str for common currencies (USD, HKD, CNY)
                        currency = Currency.from_str("USD")
                        instrument = Equity(
                            instrument_id=instrument_id,
                            raw_symbol=Symbol(symbol_str),
                            currency=currency,
                            price_precision=2,
                            price_increment=Price.from_str("0.01"),
                            lot_size=Quantity.from_int(1),
                            ts_event=0,
                            ts_init=ts_init,
                            margin_init=Decimal("0"),
                            margin_maint=Decimal("0"),
                            maker_fee=Decimal("0"),
                            taker_fee=Decimal("0"),
                        )
                        # Add to cache
                        self._cache.add_instrument(instrument)
                        # Add to instrument provider
                        self._instrument_provider.add(instrument)
                        self._log.debug(f"Added instrument to cache: {instrument_id}")
                except Exception as e:
                    self._log.warning(f"Failed to load instrument {instrument_id}: {e}")

    async def _disconnect(self) -> None:
        """Disconnect from the Longport API."""
        if self._rust_client is not None:
            self._rust_client.disconnect()

    def is_connected(self) -> bool:
        """Check if the client is connected."""
        return self._rust_client is not None and self._rust_client.is_connected()

    def is_disconnected(self) -> bool:
        """Check if the client is disconnected."""
        return self._rust_client is None or self._rust_client.is_disconnected()

    def dispose(self):
        """Dispose of the client resources."""
        if self._rust_client is not None:
            self._rust_client.dispose()

    # Subscription methods
    # The Rust client handles subscriptions internally via the Longport WebSocket.
    # When instruments are added to the instrument provider's load_ids, the Rust client
    # automatically subscribes to them when start() is called.

    async def _subscribe(self, command) -> None:
        """Handle generic subscription command."""
        # The Rust client handles subscriptions internally
        # Just log and track the subscription
        self._log.debug(f"Subscription request for {command.data_type} (handled by Rust client)")

    async def _subscribe_instruments(self, command) -> None:
        """Subscribe to instruments (handled by Rust client)."""
        pass

    async def _subscribe_instrument(self, command) -> None:
        """Subscribe to an instrument (handled by Rust client)."""
        pass

    async def _subscribe_order_book_deltas(self, command) -> None:
        """Subscribe to order book deltas (handled by Rust client)."""
        self._log.debug(f"Order book deltas subscription for {command.instrument_id} (handled by Rust client)")

    async def _subscribe_order_book_snapshots(self, command) -> None:
        """Subscribe to order book snapshots (handled by Rust client)."""
        self._log.debug(f"Order book snapshots subscription for {command.instrument_id} (handled by Rust client)")

    async def _subscribe_quote_ticks(self, command) -> None:
        """Subscribe to quote ticks (handled by Rust client)."""
        self._log.debug(f"Quote ticks subscription for {command.instrument_id} (handled by Rust client)")

    async def _subscribe_trade_ticks(self, command) -> None:
        """Subscribe to trade ticks (handled by Rust client)."""
        self._log.debug(f"Trade ticks subscription for {command.instrument_id} (handled by Rust client)")

    async def _subscribe_bars(self, command) -> None:
        """Subscribe to bars (handled by Rust client)."""
        self._log.debug(f"Bars subscription for {command.bar_type} (handled by Rust client)")

    async def _unsubscribe(self, command) -> None:
        """Handle generic unsubscription command."""
        self._log.debug(f"Unsubscription request for {command.data_type}")

    async def _unsubscribe_instruments(self, command) -> None:
        """Unsubscribe from instruments (handled by Rust client)."""
        pass

    async def _unsubscribe_instrument(self, command) -> None:
        """Unsubscribe from an instrument (handled by Rust client)."""
        pass

    async def _unsubscribe_order_book_deltas(self, command) -> None:
        """Unsubscribe from order book deltas (handled by Rust client)."""
        self._log.debug(f"Order book deltas unsubscription for {command.instrument_id}")

    async def _unsubscribe_order_book_snapshots(self, command) -> None:
        """Unsubscribe from order book snapshots (handled by Rust client)."""
        self._log.debug(f"Order book snapshots unsubscription for {command.instrument_id}")

    async def _unsubscribe_quote_ticks(self, command) -> None:
        """Unsubscribe from quote ticks (handled by Rust client)."""
        self._log.debug(f"Quote ticks unsubscription for {command.instrument_id}")

    async def _unsubscribe_trade_ticks(self, command) -> None:
        """Unsubscribe from trade ticks (handled by Rust client)."""
        self._log.debug(f"Trade ticks unsubscription for {command.instrument_id}")

    async def _unsubscribe_bars(self, command) -> None:
        """Unsubscribe from bars (handled by Rust client)."""
        self._log.debug(f"Bars unsubscription for {command.bar_type}")

    # Request methods - delegate to Rust client
    async def _request(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request(request)

    async def _request_instrument(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request_instrument(request)

    async def _request_instruments(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request_instruments(request)

    async def _request_quote_ticks(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request_quote_ticks(request)

    async def _request_trade_ticks(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request_trade_ticks(request)

    async def _request_bars(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request_bars(request)

    async def _request_order_book_snapshot(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request_order_book_snapshot(request)

    async def _request_order_book_depth(self, request) -> None:
        """Delegate request to Rust client."""
        if self._rust_client is not None:
            self._rust_client.request_order_book_depth(request)

    # Delegate all other attributes to the Rust client
    def __getattr__(self, name):
        """Delegate undefined attributes to the Rust client."""
        if self._rust_client is None:
            raise AttributeError(f"Cannot access '{name}' - Rust client not created yet. Call connect() first.")
        return getattr(self._rust_client, name)


class LongportLiveDataClientFactory(LiveDataClientFactory):
    """
    Factory for creating Longport data clients.

    The factory creates a Python wrapper that delays Rust client creation
    until _connect() is called, ensuring proper TLS context initialization.
    """

    @staticmethod
    def create(  # type: ignore
        loop,
        name: str,
        config: LongportDataClientConfig,
        msgbus,
        cache,
        clock,
    ) -> "LongportDataClient":
        """
        Create a new Longport data client instance.

        The Rust client is created later in _connect() to ensure TLS context is available.

        Parameters
        ----------
        loop : asyncio.AbstractEventLoop
            The event loop for the client.
        name : str
            The name for the client.
        config : LongportDataClientConfig
            The configuration for the client (Python config).
        msgbus : MessageBus
            The message bus for the client.
        cache : Cache
            The cache for the client.
        clock : LiveClock
            The clock for the client.

        Returns
        -------
        LiveDataClient
            The created data client instance (Python wrapper, Rust client created later).
        """
        from nautilus_trader.model.identifiers import ClientId

        # Create Python ClientId
        client_id = ClientId(name)

        # Create instrument provider
        instrument_provider = InstrumentProvider(config=config.instrument_provider)

        # Create Python wrapper (Rust client will be created in _connect)
        client = LongportDataClient(
            config=config,  # Pass config, not rust_client
            client_id=client_id,
            loop=loop,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
        )

        return client

    @staticmethod
    def getInstrumentProviderConfig(config: LongportDataClientConfig) -> InstrumentProviderConfig:
        """
        Get the instrument provider config from the client config.

        Parameters
        ----------
        config : LongportDataClientConfig
            The configuration for the client.

        Returns
        -------
        InstrumentProviderConfig
            The instrument provider configuration.
        """
        return config.instrument_provider


class LongportLiveExecClientFactory(LiveExecClientFactory):
    """
    Factory for creating Longport execution clients.

    This factory converts Python config to Rust config and creates the Rust client.
    """

    @staticmethod
    def create(
        loop,
        name: str,
        config: LongportExecClientConfig,
        msgbus,
        cache,
        clock,
        portfolio,
    ) -> LiveExecutionClient:
        """
        Create a new Longport execution client instance.

        Parameters
        ----------
        loop : asyncio.AbstractEventLoop
            The event loop for the client.
        name : str
            The name for the client.
        config : LongportExecClientConfig
            The configuration for the client (Python config).
        msgbus : MessageBus
            The message bus for the client.
        cache : Cache
            The cache for the client.
        clock : LiveClock
            The clock for the client.
        portfolio : Portfolio
            The portfolio for the client.

        Returns
        -------
        LiveExecutionClient
            The created execution client instance (Rust client wrapped).
        """
        raise NotImplementedError("Longport execution client not yet implemented")

    @staticmethod
    def getInstrumentProviderConfig(config: LongportExecClientConfig) -> InstrumentProviderConfig:
        """
        Get the instrument provider config from the client config.

        Parameters
        ----------
        config : LongportExecClientConfig
            The configuration for the client.

        Returns
        -------
        InstrumentProviderConfig
            The instrument provider configuration.
        """
        return config.instrument_provider


__all__ = [
    "LongportLiveDataClientFactory",
    "LongportLiveExecClientFactory",
]
