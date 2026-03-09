// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Testing utilities for the Longport adapter.

use crate::common::{enums::LongportMarket, models::LongportInstrument};

/// Creates a test Longport instrument for Hong Kong market.
#[must_use]
pub fn test_longport_instrument_hk() -> LongportInstrument {
    LongportInstrument {
        symbol: "700".to_string(),
        name: "Tencent Holdings Limited".to_string(),
        market: LongportMarket::HK,
        lot_size: 100,
        tick_size: 0.05,
        shortable: true,
        marginable: true,
    }
}

/// Creates a test Longport instrument for US market.
#[must_use]
pub fn test_longport_instrument_us() -> LongportInstrument {
    LongportInstrument {
        symbol: "AAPL".to_string(),
        name: "Apple Inc.".to_string(),
        market: LongportMarket::US,
        lot_size: 1,
        tick_size: 0.01,
        shortable: true,
        marginable: true,
    }
}
