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
Longport OpenSDK integration adapter for Hong Kong, US and China A-share markets.

This subpackage provides an instrument provider, data and execution clients,
configurations, data types and constants for connecting to and interacting with
Longport's OpenSDK API.

The adapter is implemented in Rust for high performance, with Python bindings
provided through PyO3.
"""

# Python configuration classes (for serialization)
from nautilus_trader.adapters.longport.config import (
    LongportDataClientConfig,
    LongportExecClientConfig,
)

# Python factory classes (wrap Rust clients)
from nautilus_trader.adapters.longport.factories import (
    LongportLiveDataClientFactory,
    LongportLiveExecClientFactory,
)

# Rust types from nautilus_pyo3.longport
from nautilus_trader.core.nautilus_pyo3.longport import (
    LongportMarket,
)

# Constants
from nautilus_trader.adapters.longport.common.constants import LONGPORT
from nautilus_trader.adapters.longport.common.constants import LONGPORT_VENUE

# Register encoders for Rust config types
from nautilus_trader.common.config import register_config_encoding

# Register encoder for LongportDataClientConfig - convert to dict for serialization
def _encode_longport_data_config(obj):
    """Encode LongportDataClientConfig to a placeholder for msgspec serialization."""
    # Rust config fields are private, so we can't access them
    # Just return a placeholder indicating the config type
    return "<LongportDataClientConfig (Rust)>"


# Register encoder for LongportExecClientConfig
def _encode_longport_exec_config(obj):
    """Encode LongportExecClientConfig to a placeholder for msgspec serialization."""
    # Rust config fields are private, so we can't access them
    return "<LongportExecClientConfig (Rust)>"


# Register the encoders
# register_config_encoding(LongportDataClientConfig, _encode_longport_data_config)
# register_config_encoding(LongportExecClientConfig, _encode_longport_exec_config)

# Also register encoder for LongportMarket enum
def _encode_longport_market(obj):
    """Encode LongportMarket to string for msgspec serialization."""
    return str(obj)  # Will give "LongportMarket.HK" etc


register_config_encoding(LongportMarket, _encode_longport_market)


__all__ = [
    "LONGPORT",
    "LONGPORT_VENUE",
    "LongportDataClientConfig",
    "LongportExecClientConfig",
    "LongportLiveDataClientFactory",
    "LongportLiveExecClientFactory",
    "LongportMarket",
]
