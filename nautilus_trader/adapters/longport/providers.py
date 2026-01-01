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
Longport instrument provider.

Provides Nautilus instrument definitions from Longport Open API.
"""

from typing import Any

from nautilus_trader.common.providers import InstrumentProvider
from nautilus_trader.config import InstrumentProviderConfig
from nautilus_trader.core import nautilus_pyo3
from nautilus_trader.core.correctness import PyCondition
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.instruments import instruments_from_pyo3


class LongportInstrumentProvider(InstrumentProvider):
    """
    Provides Nautilus instrument definitions from Longport.

    Parameters
    ----------
    markets : tuple[str, ...]
        The markets to load instruments for (e.g., "HK", "US", "CN").
    config : InstrumentProviderConfig, optional
        The instrument provider configuration.

    """

    def __init__(
        self,
        markets: tuple[str, ...],
        config: InstrumentProviderConfig | None = None,
    ) -> None:
        super().__init__(config=config)
        self._markets = markets
        self._log_warnings = config.log_warnings if config else True

        self._instruments_pyo3: list[nautilus_pyo3.Instrument] = []

    @property
    def markets(self) -> tuple[str, ...]:
        """Return the markets configured for the provider."""
        return self._markets

    def instruments_pyo3(self) -> list[Any]:
        """
        Return all Longport PyO3 instrument definitions held by the provider.

        Returns
        -------
        list[nautilus_pyo3.Instrument]

        """
        return self._instruments_pyo3

    async def load_all_async(self, filters: dict | None = None) -> None:
        """
        Load all instruments asynchronously.

        For Longport, if `load_ids` is specified in the config, instruments
        are created from those IDs. Otherwise, instruments are fetched from
        the Longport API.

        Parameters
        ----------
        filters : dict, optional
            Not currently used, included for compatibility.

        """
        filters_str = "..." if not filters else f" with filters {filters}..."
        self._log.info(f"Loading instruments for markets {self._markets}{filters_str}")

        if self._config and self._config.load_ids:
            # Load specific instruments from IDs
            self._log.info(f"Loading {len(self._config.load_ids)} instruments from load_ids")
            await self._load_from_ids()
        else:
            # Load all instruments from API
            self._log.warning("Loading all instruments from API is not yet supported")
            self._log.warning("Please specify load_ids in the config to load instruments")

    async def _load_from_ids(self) -> None:
        """Load instruments from the configured load_ids."""
        PyCondition.not_none(self._config, "load_ids requires config to be set")
        PyCondition.not_none(self._config.load_ids, "load_ids cannot be empty")

        from nautilus_trader.model.identifiers import InstrumentId

        instruments_pyo3 = []
        for instrument_id in self._config.load_ids:
            try:
                # Convert InstrumentId to string
                instrument_id_str = instrument_id.value if isinstance(instrument_id, InstrumentId) else str(instrument_id)

                # Create a minimal instrument from the ID
                # The Longport Rust client can create minimal instruments
                instrument = nautilus_pyo3.longport.create_minimal_instrument_py(instrument_id_str)
                instruments_pyo3.append(instrument)
                self._log.debug(f"Created minimal instrument for {instrument_id_str}")
            except Exception as e:
                self._log.error(f"Failed to create instrument for {instrument_id}: {e}")

        self._instruments_pyo3 = instruments_pyo3

        # Convert to Nautilus instruments
        instruments = instruments_from_pyo3(instruments_pyo3)
        for instrument in instruments:
            self.add(instrument=instrument)
            self._log.info(f"✅ Added instrument to provider: {instrument.id}")

        self._log.info(f"Loaded {len(instruments)} instruments into provider")
        self._log.info(f"Provider now has {len(self._instruments)} instruments: {list(self._instruments.keys())}")
