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
Longport execution client - placeholder for future implementation.
"""

import asyncio

from nautilus_trader.adapters.longport.config import LongportExecClientConfig
from nautilus_trader.adapters.longport.common.constants import LONGPORT_VENUE
from nautilus_trader.adapters.longport.providers import LongportInstrumentProvider
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.live.execution_client import LiveExecutionClient
from nautilus_trader.model.identifiers import ClientId


class LongportExecutionClient(LiveExecutionClient):
    """
    Longport execution client - placeholder for future implementation.

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
    config : LongportExecClientConfig
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
        config: LongportExecClientConfig,
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
            account_id=config.account_id if config else None,
        )
        self._config = config
        self._is_connected = False

    async def _connect(self) -> None:
        """Connect to the Longport API."""
        if self._is_connected:
            self._log.warning("Already connected to Longport execution")
            return

        self._log.info("Connecting to Longport execution API...")

        try:
            # Initialize instrument provider
            await self._instrument_provider.load_all_async()

            # The Rust LongportExecutionClient handles the actual connection
            # and WebSocket communication. This Python client provides
            # the interface layer for Nautilus integration.
            # The actual TradeContext is managed by the Rust implementation.

            self._is_connected = True
            self._log.info("Connected to Longport execution API", LogColor.GREEN)

        except Exception as e:
            self._log.error(f"Failed to connect to Longport execution API: {e}")
            raise

    async def _disconnect(self) -> None:
        """Disconnect from the Longport API."""
        pass

    def is_connected(self) -> bool:
        """Return whether the client is connected."""
        return self._is_connected
