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
Longport adapter factories.

Provides factory classes for creating Longport data and execution clients.
"""

import asyncio
from functools import lru_cache

from nautilus_trader.adapters.longport.config import (
    LongportDataClientConfig,
    LongportExecClientConfig,
)
from nautilus_trader.adapters.longport.execution import LongportExecutionClient
from nautilus_trader.adapters.longport.providers import LongportInstrumentProvider
from nautilus_trader.cache.cache import Cache
from nautilus_trader.common.component import LiveClock
from nautilus_trader.common.component import MessageBus
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.live.factories import LiveDataClientFactory
from nautilus_trader.live.factories import LiveExecClientFactory


@lru_cache(1)
def get_cached_longport_instrument_provider(
    markets: tuple[str, ...],
    config: InstrumentProviderConfig | None = None,
) -> LongportInstrumentProvider:
    """
    Cache and return a Longport instrument provider.

    If a cached provider already exists, then that provider will be returned.

    Parameters
    ----------
    markets : tuple[str, ...]
        The markets to load instruments for.
    config : InstrumentProviderConfig, optional
        The instrument provider configuration.

    Returns
    -------
    LongportInstrumentProvider

    """
    return LongportInstrumentProvider(
        markets=markets,
        config=config,
    )


class LongportLiveDataClientFactory(LiveDataClientFactory):
    """
    Provides a Longport live data client factory using Rust implementation.
    """

    @staticmethod
    def create(  # type: ignore
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: LongportDataClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ):
        """
        Create a new Longport data client using the Rust implementation.

        Parameters
        ----------
        loop : asyncio.AbstractEventLoop
            The event loop for the client.
        name : str
            The custom client ID.
        config : LongportDataClientConfig
            The client configuration.
        msgbus : MessageBus
            The message bus for the client.
        cache : Cache
            The cache for the client.
        clock : LiveClock
            The clock for the client.

        Returns
        -------
        Rust LongportDataClient wrapper

        """
        # Create the Rust LongportDataClient
        # The Rust implementation handles all WebSocket connections and data processing
        # Pass the name as string directly - Rust will convert it to ClientId
        rust_client = nautilus_pyo3.longport.LongportDataClient(
            client_id=name,
            config=config,
        )

        # Still need to load instruments into Python cache for compatibility
        provider = get_cached_longport_instrument_provider(
            markets=tuple(config.markets),
            config=config.instrument_provider,
        )

        # Load instruments asynchronously and add to cache
        # This is needed for the DataEngine to find instruments
        async def load_instruments():
            await provider.load_all_async()
            for instrument in provider._instruments.values():
                cache.add_instrument(instrument)

        # Schedule instrument loading
        loop.create_task(load_instruments())

        return rust_client


class LongportLiveExecClientFactory(LiveExecClientFactory):
    """
    Provides a Longport live execution client factory.
    """

    @staticmethod
    def create(  # type: ignore
        loop: asyncio.AbstractEventLoop,
        name: str,
        config: LongportExecClientConfig,
        msgbus: MessageBus,
        cache: Cache,
        clock: LiveClock,
    ) -> LongportExecutionClient:
        """
        Create a new Longport execution client.

        Parameters
        ----------
        loop : asyncio.AbstractEventLoop
            The event loop for the client.
        name : str
            The custom client ID.
        config : LongportExecClientConfig
            The client configuration.
        msgbus : MessageBus
            The message bus for the client.
        cache : Cache
            The cache for the client.
        clock : LiveClock
            The clock for the client.

        Returns
        -------
        LongportExecutionClient

        """
        provider = get_cached_longport_instrument_provider(
            markets=tuple(config.markets),
            config=config.instrument_provider,
        )
        return LongportExecutionClient(
            loop=loop,
            msgbus=msgbus,
            cache=cache,
            clock=clock,
            instrument_provider=provider,
            config=config,
            name=name,
        )
