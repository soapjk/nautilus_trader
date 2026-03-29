# Fix Longport Rustls Crypto Provider Issue

## Problem Summary

When running the live trading script, the Longport adapter crashes with a rustls panic:

```
thread '<unnamed>' panicked at rustls/crypto/mod.rs:249:14:
Could not automatically determine the process-level CryptoProvider from Rustls crate features.
Call CryptoProvider::install_default() before this point to select a provider manually,
or make sure exactly one of the 'aws-lc-rs' and 'ring' features is enabled.
```

## Error Location

- **File**: `nautilus_trader/adapters/longport/data.py:149`
- **Function**: `nautilus_pyo3.longport.create_quote_context()`
- **Panic**: Rust TLS library (rustls) crypto provider not initialized

## Root Cause Analysis

1. The Longport adapter is a Rust extension (PyO3) that depends on `longport` SDK v3.0
2. The `longport` SDK uses `rustls` for TLS connections
3. **rustls 0.23+ architecture change**: Crypto implementation is now abstract via `CryptoProvider` trait
4. When loaded as a Python extension, the main process (Python) doesn't initialize rustls crypto provider
5. Previous versions had auto-detection, which was removed in rustls 0.23+

## Solution

Initialize the rustls crypto provider in the Python binding layer before creating QuoteContext.

### Reference Implementation

The same issue was already solved in the `dydx` adapter:
- **File**: `crates/adapters/dydx/bin/grpc_exec.rs:151-153`

```rust
// Initialize rustls crypto provider (required for TLS connections)
rustls::crypto::aws_lc_rs::default_provider()
    .install_default()
    .expect("Failed to install rustls crypto provider");
```

## Fix Instructions

### File to Modify

**`crates/adapters/longport/src/python/quote_context.rs`**

### Changes Required

#### Change 1: Add Import

After line 21 (existing imports), add:

```rust
use rustls::crypto::aws_lc_rs;
```

**Location**: After the existing imports section, before `use std::hash::{Hash, Hasher};`

#### Change 2: Initialize Crypto Provider

In the `create_quote_context` function (starting at line 521), add initialization at the very beginning:

```rust
#[pyfunction]
#[pyo3(signature = (
    app_key,
    app_secret,
    access_token,
    http_url=None,
    quote_ws_url=None,
    trade_ws_url=None
))]
pub fn create_quote_context(
    py: Python<'_>,
    app_key: String,
    app_secret: String,
    access_token: String,
    http_url: Option<String>,
    quote_ws_url: Option<String>,
    trade_ws_url: Option<String>,
) -> PyResult<(PyQuoteContext, PyPushEventReceiver)> {
    // Initialize rustls crypto provider (required for TLS connections)
    aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let _ = (http_url, quote_ws_url, trade_ws_url);  // Suppress unused warnings
    // ... rest of the function unchanged
```

**Location**: Right after the function opening brace, before `let _ = (http_url, quote_ws_url, trade_ws_url);`

## Rebuild Instructions

After applying the changes, rebuild the Rust extension:

```bash
cd /Volumes/T9/projects/trade/nautilus_trader
pip install -e .
```

## Verification

After rebuilding, test the live trading:

```bash
cd /Volumes/T9/projects/trade/czx_autotrader
./live_trading/scripts/run_live_trading.sh
```

Expected behavior:
- No rustls panic
- QuoteContext connects successfully
- Trading starts normally

## Technical Details

### Why This Fix Works

1. **Explicit Initialization**: rustls 0.23+ requires `CryptoProvider::install_default()` before any TLS operations
2. **Python Extension Context**: As a dynamically loaded extension, the adapter cannot rely on the parent process (Python) to initialize rustls
3. **aws_lc_rs Provider**: AWS's cryptographic library, the recommended default provider for rustls
4. **One-Time Setup**: The call is idempotent; multiple calls are safe

### Dependencies

- `rustls` crate features must include `aws-lc-rs` or `ring`
- The fix uses `aws_lc_rs` which is typically available by default in rustls 0.23+

## Additional Notes

- This fix should be applied before any `longport` SDK calls that involve network I/O
- The initialization is thread-safe and global for the process
- If `aws_lc_rs` is not available, an alternative is to use `rustls::crypto::ring::default_provider()`
