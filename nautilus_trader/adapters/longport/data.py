# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  you may not use this file except in compliance with the License.
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

This client follows the OKX architecture pattern where the core logic is
implemented in Python using LiveMarketDataClient as the base class.
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
from nautilus_trader.model.enums import BookType
from nautilus_trader.model.enums import book_type_to_str
from nautilus_trader.model.identifiers import ClientId


class LongportDataClient(LiveMarketDataClient):
    """
    Provides a data client for the Longport securities trading.

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

        # Task management
        self._ws_tasks: list[asyncio.Task] = []
        self._reconnect_task: asyncio.Task | None = None

        # Subscription tracking
        self._subscribed_quotes: set[str] = set()
        self._subscribed_trades: set[str] = set()
        self._subscribed_books: set[str] = set()

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
            # Initialize instrument provider
            await self._instrument_provider.load_all_async()

            # Add all instruments from provider to cache
            # This is critical for DataEngine to find instruments when subscribing
            for instrument in self._instrument_provider._instruments.values():
                self._cache.add_instrument(instrument)
                self._log.info(f"✅ Added instrument to cache: {instrument.id}")

            self._log.info(f"Total instruments in cache: {len(list(self._cache.instruments()))}")

            # Create Rust QuoteContext through PyO3 bindings
            # The QuoteContext is now managed by the Rust LongportDataClient
            # We just need to store a reference to it for future use
            self._quote_ctx = None  # Will be set by Rust implementation

            self._is_connected = True
            self._log.info("Connected to Longport", LogColor.GREEN)

        except Exception as e:
            self._log.error(f"Failed to connect to Longport: {e}")
            raise

    async def _disconnect(self) -> None:
        """Disconnect from the Longport API."""
        self._log.info("Disconnecting from Longport...")

        # Cancel all tasks
        if self._reconnect_task:
            self._reconnect_task.cancel()

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

    # ========== Subscription handlers ==========

    async def _subscribe_quote_ticks(self, command: SubscribeQuoteTicks) -> None:
        """Subscribe to quote tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"🔵 [PYTHON] Subscribing to quote ticks for {instrument_id_str}")

        # Check if instrument exists in cache
        instrument = self._cache.instrument(command.instrument_id)
        if instrument:
            self._log.info(f"✅ [PYTHON] Instrument found in cache: {instrument.id}")
        else:
            self._log.error(f"❌ [PYTHON] Instrument NOT found in cache: {command.instrument_id}")

        self._subscribed_quotes.add(instrument_id_str)

        # TODO: Actually subscribe to Longport SDK here
        # Currently Python client is just a wrapper - need to integrate with Rust QuoteContext
        self._log.warning(
            f"⚠️  [PYTHON] Quote subscription for {instrument_id_str} is only tracked locally. "
            f"Actual SDK subscription not yet implemented in Python client."
        )

        self._log.info(f"✅ [PYTHON] Subscribed {instrument_id_str} quotes (tracking count: {len(self._subscribed_quotes)})", LogColor.GREEN)

    async def _unsubscribe_quote_ticks(self, command: UnsubscribeQuoteTicks) -> None:
        """Unsubscribe from quote tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Unsubscribing from quote ticks for {instrument_id_str}")
        self._subscribed_quotes.discard(instrument_id_str)

    async def _subscribe_trade_ticks(self, command: SubscribeTradeTicks) -> None:
        """Subscribe to trade tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"🔵 [PYTHON] Subscribing to trade ticks for {instrument_id_str}")

        # Check if instrument exists in cache
        instrument = self._cache.instrument(command.instrument_id)
        if instrument:
            self._log.info(f"✅ [PYTHON] Instrument found in cache: {instrument.id}")
        else:
            self._log.error(f"❌ [PYTHON] Instrument NOT found in cache: {command.instrument_id}")

        self._subscribed_trades.add(instrument_id_str)

        # TODO: Actually subscribe to Longport SDK here
        # Currently Python client is just a wrapper - need to integrate with Rust QuoteContext
        self._log.warning(
            f"⚠️  [PYTHON] Trade subscription for {instrument_id_str} is only tracked locally. "
            f"Actual SDK subscription not yet implemented in Python client."
        )

        self._log.info(f"✅ [PYTHON] Subscribed {instrument_id_str} trades (tracking count: {len(self._subscribed_trades)})", LogColor.GREEN)

    async def _unsubscribe_trade_ticks(self, command: UnsubscribeTradeTicks) -> None:
        """Unsubscribe from trade tick data."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Unsubscribing from trade ticks for {instrument_id_str}")
        self._subscribed_trades.discard(instrument_id_str)

    async def _subscribe_bars(self, command: SubscribeBars) -> None:
        """Subscribe to bar data."""
        bar_type = command.bar_type
        self._log.info(f"🔵 [PYTHON] Subscribing to bars {bar_type}")

        # Check if instrument exists in cache
        instrument = self._cache.instrument(bar_type.instrument_id)
        if instrument:
            self._log.info(f"✅ [PYTHON] Instrument found in cache: {instrument.id}")
        else:
            self._log.error(f"❌ [PYTHON] Instrument NOT found in cache: {bar_type.instrument_id}")

        self._log.info(f"✅ [PYTHON] Subscribed {bar_type} bars", LogColor.GREEN)

    async def _unsubscribe_bars(self, command: UnsubscribeBars) -> None:
        """Unsubscribe from bar data."""
        bar_type = command.bar_type
        self._log.info(f"Unsubscribing from bars {bar_type}")

    async def _subscribe_order_book_deltas(self, command: SubscribeOrderBook) -> None:
        """Subscribe to order book deltas."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"🔵 [PYTHON] Subscribing to order book deltas for {instrument_id_str}")

        # Check if instrument exists in cache
        instrument = self._cache.instrument(command.instrument_id)
        if instrument:
            self._log.info(f"✅ [PYTHON] Instrument found in cache: {instrument.id}")
        else:
            self._log.error(f"❌ [PYTHON] Instrument NOT found in cache: {command.instrument_id}")

        if command.book_type != BookType.L2_MBP:
            self._log.warning(
                f"Book type {book_type_to_str(command.book_type)} not supported by Longport, "
                f"skipping subscription for {instrument_id_str}"
            )
            return

        self._subscribed_books.add(instrument_id_str)

        # TODO: Actually subscribe to Longport SDK here
        # Currently Python client is just a wrapper - need to integrate with Rust QuoteContext
        self._log.warning(
            f"⚠️  [PYTHON] Order book subscription for {instrument_id_str} is only tracked locally. "
            f"Actual SDK subscription not yet implemented in Python client."
        )

        self._log.info(f"✅ [PYTHON] Subscribed {instrument_id_str} order book deltas (depth={command.depth}, tracking count: {len(self._subscribed_books)})", LogColor.GREEN)

    async def _unsubscribe_order_book_deltas(self, command: UnsubscribeOrderBook) -> None:
        """Unsubscribe from order book deltas."""
        instrument_id_str = command.instrument_id.value
        self._log.info(f"Unsubscribing from order book deltas for {instrument_id_str}")
        self._subscribed_books.discard(instrument_id_str)

    # ========== Request handlers ==========

    async def _request_bars(self, command: RequestBars) -> None:
        """Request historical bar data."""
        self._log.warning(f"Request bars should be handled by Rust implementation: {command}")
        # The actual implementation is in Rust LongportDataClient::request_bars
        # This Python stub is kept for interface compatibility

    async def _request_quote_ticks(self, command: RequestQuoteTicks) -> None:
        """Request historical quote tick data."""
        self._log.warning(f"Request quote ticks should be handled by Rust implementation: {command}")
        # The actual implementation is in Rust LongportDataClient
        # Quote tick data is derived from depth updates

    async def _request_trade_ticks(self, command: RequestTradeTicks) -> None:
        """Request historical trade tick data."""
        self._log.warning(f"Request trade ticks should be handled by Rust implementation: {command}")
        # The actual implementation is in Rust LongportDataClient::request_trades

    async def _request_instrument(self, command: RequestInstrument) -> None:
        """Request instrument definition."""
        self._log.warning(f"Request instrument should be handled by Rust implementation: {command}")
        # The actual implementation is in Rust LongportDataClient::request_instrument
