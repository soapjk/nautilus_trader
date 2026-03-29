# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
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
Longport data client providing market data from Longport Open API.

This client follows the OKX architecture pattern where:
- Python implements the full LiveMarketDataClient
- Rust provides HTTP and WebSocket clients that wrap QuoteContext
- QuoteContext is created in Python and shared via Arc with Rust clients

References:
- https://github.com/longportapp/openapi
- https://longportapp.github.io/openapi
"""

import asyncio
from typing import Any

from nautilus_trader.adapters.longport.config import LongportDataClientConfig
from nautilus_trader.adapters.longport.common.constants import LONGPORT_VENUE
from nautilus_trader.adapters.longport.providers import LongportInstrumentProvider
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.common.enums import LogColor
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.core.correctness import PyCondition
from nautilus_trader.data.messages import RequestBars
from nautilus_trader.data.messages import RequestInstrument
from nautilus_trader.data.messages import RequestInstruments
from nautilus_trader.data.messages import RequestQuoteTicks
from nautilus_trader.data.messages import RequestTradeTicks
from nautilus_trader.data.messages import SubscribeBars
from nautilus_trader.data.messages import SubscribeOrderBook
from nautilus_trader.data.messages import SubscribeQuoteTicks
from nautilus_trader.data.messages import SubscribeTradeTicks
from nautilus_trader.data.messages import UnsubscribeBars
from nautilus_trader.data.messages import UnsubscribeOrderBook
from nautilus_trader.data.messages import UnsubscribeQuoteTicks
from nautilus_trader.data.messages import UnsubscribeTradeTicks
from nautilus_trader.live.cancellation import DEFAULT_FUTURE_CANCELLATION_TIMEOUT
from nautilus_trader.live.cancellation import cancel_tasks_with_timeout
from nautilus_trader.live.data_client import LiveMarketDataClient
from nautilus_trader.model.data import capsule_to_data
from nautilus_trader.model.enums import BookType
from nautilus_trader.model.enums import book_type_to_str
from nautilus_trader.model.identifiers import ClientId


class LongportDataClient(LiveMarketDataClient):
    """
    Provides a data client for the Longport securities trading.

    This client follows the OKX architecture pattern:
    - Creates QuoteContext in Python
    - Passes it to Rust HTTP/WebSocket clients via Arc
    - Implements all subscription/request logic in Python

    Parameters
    ----------
    loop : asyncio.AbstractEventLoop
        The event loop for the client.
    msgbus : MessageBus
        The message bus for the client.
    cache : Cache
        The cache for the client.
    clock : LiveClock
        The clock for the client.
    instrument_provider : LongportInstrumentProvider
        The instrument provider.
    config : LongportDataClientConfig
        The configuration for the client.
    name : str, optional
        The custom client ID.

    """

    def __init__(
        self,
        loop: asyncio.AbstractEventLoop,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
        instrument_provider: LongportInstrumentProvider,
        config: LongportDataClientConfig,
        name: str | None = None,
    ) -> None:
        # PyCondition.not_empty(config.markets, "config.markets")
        super().__init__(
            loop=loop,
            client_id=ClientId(name or LONGPORT_VENUE.value),
            venue=LONGPORT_VENUE,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=instrument_provider,
        )

        self._config = config
        self._is_connected = False

        # Create QuoteContext in Python (this is shared with Rust clients)
        self._quote_ctx = None
        self._event_receiver = None

        # HTTP and WebSocket clients (will be initialized with QuoteContext)
        self._http_client = None
        self._ws_client = None

        # Task management
        self._ws_tasks: list[asyncio.Task] = []
        self._event_task: asyncio.Task | None = None

        # Subscription tracking
        self._subscribed_quotes: set[str] = set()
        self._subscribed_trades: set[str] = set()
        self._subscribed_books: set[str] = set()

        self._log.info(f"config.markets={config.markets}", LogColor.BLUE)

    @property
    def instrument_provider(self) -> LongportInstrumentProvider:
        return self._instrument_provider

    async def _connect(self) -> None:
        """Connect to the Longport API."""
        if self._is_connected:
            self._log.warning("Already connected to Longport")
            return

        self._log.info(f"Connecting to Longport (markets={self._config.markets})...")

        try:
            # Initialize Rust TLS crypto provider before any TLS connections
            nautilus_pyo3.HttpClient()

            # Create QuoteContext with optional custom URLs
            self._log.info("Creating QuoteContext...")
            self._quote_ctx, self._event_receiver = nautilus_pyo3.longport.create_quote_context(
                app_key=self._config.get_app_key(),
                app_secret=self._config.get_app_secret(),
                access_token=self._config.get_access_token(),
                http_url=self._config.http_url,
                quote_ws_url=self._config.quote_ws_url,
                trade_ws_url=self._config.trade_ws_url,
            )

            # Log custom URLs if provided
            if self._config.http_url:
                self._log.info(f"Custom HTTP URL: {self._config.http_url}")
            if self._config.quote_ws_url:
                self._log.info(f"Custom quote WS URL: {self._config.quote_ws_url}")
            if self._config.trade_ws_url:
                self._log.info(f"Custom trade WS URL: {self._config.trade_ws_url}")

            # Create HTTP client with shared QuoteContext
            self._log.info("Creating HTTP client...")
            self._http_client = nautilus_pyo3.longport.LongportHttpClient(
                self._quote_ctx
            )

            # Create WebSocket client with shared QuoteContext
            self._log.info("Creating WebSocket client...")
            self._ws_client = nautilus_pyo3.longport.LongportWebSocketClient(
                self._quote_ctx
            )

            # Initialize instrument provider
            self._log.info("Initializing instrument provider...")
            await self._instrument_provider.load_all_async()

            # Add all instruments from provider to cache
            for instrument in self._instrument_provider._instruments.values():
                self._cache.add_instrument(instrument)
                self._log.debug(f"Added instrument to cache: {instrument.id}")

            self._log.info(f"Total instruments in cache: {len(list(self._cache.instruments()))}")

            # TODO: Cache instruments in HTTP/WebSocket clients and event receiver for precision info
            # This will be needed when push event conversion is implemented
            # for instrument in self._instrument_provider._instruments.values():
            #     self._http_client.cache_instrument(instrument)
            #     self._ws_client.cache_instrument(instrument)
            #     self._event_receiver.cache_instrument(instrument)

            # Start event consumption task
            self._log.info("Starting event consumption task...")
            self._event_task = self._loop.create_task(self._consume_events())

            self._is_connected = True
            self._log.info("Connected to Longport", LogColor.GREEN)

        except Exception as e:
            self._log.error(f"Failed to connect to Longport: {e}")
            raise

    async def _disconnect(self) -> None:
        """Disconnect from the Longport API."""
        self._log.info("Disconnecting from Longport...")

        # Cancel event consumption task
        if self._event_task:
            self._event_task.cancel()
            try:
                await self._event_task
            except asyncio.CancelledError:
                pass

        # Cancel all tasks
        await cancel_tasks_with_timeout(
            set(self._ws_tasks),
            self._log,
            timeout_secs=DEFAULT_FUTURE_CANCELLATION_TIMEOUT,
        )

        self._is_connected = False
        self._log.info("Disconnected from Longport")

    def is_connected(self) -> bool:
        """Return whether the client is connected."""
        return self._is_connected

    async def _consume_events(self) -> None:
        """Consume WebSocket push events from Longport."""
        self._log.info("Starting event consumption loop...")

        while True:
            try:
                # recv_async is a synchronous non-blocking call from Rust
                event = self._event_receiver.recv_async()
                if event is None:
                    # No event available, sleep briefly to avoid busy loop
                    await asyncio.sleep(0.01)
                    continue

                # Handle the event
                self._handle_push_event(event)
            except asyncio.CancelledError:
                self._log.info("Event consumption cancelled")
                break
            except Exception as e:
                self._log.error(f"Error consuming event: {e}")
                # Sleep briefly before retrying
                await asyncio.sleep(0.1)

    def _handle_push_event(self, event: Any) -> None:
        """
        Handle a push event from Longport WebSocket.

        All events from Rust are returned as PyCapsule containing Nautilus Data.
        This includes QuoteTick, TradeTick, OrderBookDeltas, and Bar types.
        """
        try:
            if nautilus_pyo3.is_pycapsule(event):
                # The capsule contains a pointer to `Data` owned and managed by Rust
                data = capsule_to_data(event)
                self._handle_data(data)
            else:
                # Unexpected: should not happen as Rust always returns PyCapsule
                event_type = type(event).__name__ if hasattr(event, "__class__") else type(event).__name__
                self._log.warning(f"Unexpected non-PyCapsule event type: {event_type}, event: {event}")
        except Exception as e:
            self._log.error(f"Error handling push event: {e}")

    # ========== Subscription handlers ==========

    async def _subscribe_quote_ticks(self, command: SubscribeQuoteTicks) -> None:
        """Subscribe to quote tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Subscribing to quote ticks for {instrument_id_str}")

        # Check if instrument exists in cache
        instrument = self._cache.instrument(command.instrument_id)
        if instrument:
            self._log.debug(f"Instrument found in cache: {instrument.id}")
        else:
            self._log.warning(f"Instrument NOT found in cache: {command.instrument_id}")

        # Subscribe via WebSocket client
        await self._ws_client.subscribe_quotes(instrument_id_str)
        self._subscribed_quotes.add(instrument_id_str)

        self._log.info(f"Subscribed {instrument_id_str} quotes", LogColor.GREEN)

    async def _unsubscribe_quote_ticks(self, command: UnsubscribeQuoteTicks) -> None:
        """Unsubscribe from quote tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Unsubscribing from quote ticks for {instrument_id_str}")

        await self._ws_client.unsubscribe_quotes(instrument_id_str)
        self._subscribed_quotes.discard(instrument_id_str)

    async def _subscribe_trade_ticks(self, command: SubscribeTradeTicks) -> None:
        """Subscribe to trade tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Subscribing to trade ticks for {instrument_id_str}")

        instrument = self._cache.instrument(command.instrument_id)
        if instrument:
            self._log.debug(f"Instrument found in cache: {instrument.id}")

        await self._ws_client.subscribe_trades(instrument_id_str)
        self._subscribed_trades.add(instrument_id_str)

        self._log.info(f"Subscribed {instrument_id_str} trades", LogColor.GREEN)

    async def _unsubscribe_trade_ticks(self, command: UnsubscribeTradeTicks) -> None:
        """Unsubscribe from trade tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Unsubscribing from trade ticks for {instrument_id_str}")

        await self._ws_client.unsubscribe_trades(instrument_id_str)
        self._subscribed_trades.discard(instrument_id_str)

    async def _subscribe_bars(self, command: SubscribeBars) -> None:
        """Subscribe to bar data."""
        bar_type = command.bar_type
        self._log.info(f"Subscribing to bars {bar_type}")

        # Bar subscriptions are handled through quote updates
        self._log.info(f"Subscribed {bar_type} bars (will derive from quotes)", LogColor.GREEN)

    async def _unsubscribe_bars(self, command: UnsubscribeBars) -> None:
        """Unsubscribe from bar data."""
        bar_type = command.bar_type
        self._log.info(f"Unsubscribing from bars {bar_type}")

    async def _subscribe_order_book_deltas(self, command: SubscribeOrderBook) -> None:
        """Subscribe to order book deltas."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Subscribing to order book deltas for {instrument_id_str}")

        if command.book_type != BookType.L2_MBP:
            self._log.warning(
                f"Book type {book_type_to_str(command.book_type)} not supported by Longport, "
                f"skipping subscription for {instrument_id_str}"
            )
            return

        instrument = self._cache.instrument(command.instrument_id)
        if instrument:
            self._log.debug(f"Instrument found in cache: {instrument.id}")

        await self._ws_client.subscribe_depth(instrument_id_str)
        self._subscribed_books.add(instrument_id_str)

        self._log.info(f"Subscribed {instrument_id_str} order book deltas (depth={command.depth})", LogColor.GREEN)

    async def _unsubscribe_order_book_deltas(self, command: UnsubscribeOrderBook) -> None:
        """Unsubscribe from order book deltas."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Unsubscribing from order book deltas for {instrument_id_str}")

        await self._ws_client.unsubscribe_depth(instrument_id_str)
        self._subscribed_books.discard(instrument_id_str)

    # ========== Request handlers ==========

    async def _request_bars(self, command: RequestBars) -> None:
        """Request historical bar data."""
        self._log.info(f"Requesting bars: {command.bar_type}")
        # TODO: Implement via HTTP client

    async def _request_quote_ticks(self, command: RequestQuoteTicks) -> None:
        """Request historical quote tick data."""
        self._log.warning("Cannot request historical quotes: not published by Longport")

    async def _request_trade_ticks(self, command: RequestTradeTicks) -> None:
        """Request historical trade tick data."""
        self._log.info(f"Requesting trade ticks for {command.instrument_id}")
        # TODO: Implement via HTTP client

    async def _request_instrument(self, command: RequestInstrument) -> None:
        """Request instrument definition."""
        self._log.info(f"Requesting instrument: {command.instrument_id}")
        # TODO: Implement via HTTP client

    async def _request_instruments(self, command: RequestInstruments) -> None:
        """Request instrument definitions."""
        self._log.info(f"Requesting instruments for {command.venue}")
        # Instruments are already loaded in _connect, just send them
        instruments = list(self._instrument_provider.get_all().values())
        self._handle_instruments(
            command.venue,
            instruments,
            command.id,
            command.start,
            command.end,
            command.params,
        )
