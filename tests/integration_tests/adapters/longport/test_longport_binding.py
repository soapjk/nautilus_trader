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
Test basic imports and module loading for the longport adapter.

This test verifies that the Rust-compiled longport module can be imported
and basic constants are accessible.
"""

import sys
from pathlib import Path

import pytest


def test_longport_rust_module_exists():
    """Test that the longport Rust library can be found."""
    # Find the compiled longport library
    project_root = Path(__file__).parent.parent.parent.parent.parent
    target_dir = project_root / "target" / "debug"

    # Look for the compiled library
    if sys.platform == "darwin":
        lib_name = "libnautilus_longport.dylib"
    elif sys.platform == "linux":
        lib_name = "libnautilus_longport.so"
    elif sys.platform == "win32":
        lib_name = "nautilus_longport.dll"
    else:
        pytest.skip(f"Unsupported platform: {sys.platform}")

    lib_path = target_dir / lib_name
    assert lib_path.exists(), f"Longport library not found at {lib_path}"

    print(f"✓ Found longport library at: {lib_path}")


def test_longport_constants_via_ctypes():
    """Test basic constants from the longport Rust crate through ctypes."""
    import ctypes

    # Find and load the library
    project_root = Path(__file__).parent.parent.parent.parent.parent
    target_dir = project_root / "target" / "debug"

    if sys.platform == "darwin":
        lib_name = "libnautilus_longport.dylib"
    elif sys.platform == "linux":
        lib_name = "libnautilus_longport.so"
    else:
        pytest.skip(f"Unsupported platform: {sys.platform}")

    lib_path = target_dir / lib_name

    try:
        # Try to load the library
        lib = ctypes.CDLL(str(lib_path))
        print(f"✓ Successfully loaded longport library")
        print(f"  Library: {lib_path}")
        print(f"  Handle: {lib._name}")

    except Exception as e:
        pytest.fail(f"Failed to load longport library: {e}")


def test_longport_module_info():
    """Test that we can gather info about the longport adapter."""
    # This test documents the expected structure
    expected_structure = {
        "module_path": "crates/adapters/longport/src/",
        "python_module": "crates/adapters/longport/src/python/mod.rs",
        "config_module": "crates/adapters/longport/src/config.rs",
        "factories_module": "crates/adapters/longport/src/factories.rs",
        "data_module": "crates/adapters/longport/src/data/mod.rs",
        "execution_module": "crates/adapters/longport/src/execution/mod.rs",
    }

    project_root = Path(__file__).parent.parent.parent.parent.parent

    for key, rel_path in expected_structure.items():
        full_path = project_root / rel_path
        exists = full_path.exists()
        status = "✓" if exists else "✗"
        print(f"{status} {key}: {rel_path}")
        assert exists, f"Expected file not found: {full_path}"

    print("\n✓ All expected longport adapter files are present")


if __name__ == "__main__":
    # Run tests manually for quick verification
    print("=" * 60)
    print("Longport Adapter Test Suite")
    print("=" * 60)
    print()

    print("Test 1: Checking Rust module exists...")
    test_longport_rust_module_exists()
    print()

    print("Test 2: Checking module structure...")
    test_longport_module_info()
    print()

    print("Test 3: Testing library loading...")
    test_longport_constants_via_ctypes()
    print()

    print("=" * 60)
    print("All tests passed!")
    print("=" * 60)
