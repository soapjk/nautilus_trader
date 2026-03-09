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

//! Credential management for Longport API authentication.

use std::fmt;

use ustr::Ustr;
use zeroize::ZeroizeOnDrop;

/// Longport API credentials.
///
/// These credentials are used for authenticating with Longport's OpenAPI.
/// The secret values are automatically zeroized on drop for security.
#[derive(Clone, ZeroizeOnDrop)]
pub struct Credential {
    /// The application key (public identifier, not zeroized).
    #[zeroize(skip)]
    pub app_key: Ustr,
    /// The application secret (secret, zeroized on drop).
    pub app_secret: String,
    /// The access token (secret, zeroized on drop).
    pub access_token: String,
}

impl Credential {
    /// Creates a new [`Credential`].
    ///
    /// # Arguments
    ///
    /// * `app_key` - The application key.
    /// * `app_secret` - The application secret.
    /// * `access_token` - The access token.
    pub fn new(app_key: Ustr, app_secret: String, access_token: String) -> Self {
        Self {
            app_key,
            app_secret,
            access_token,
        }
    }
}

impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credential")
            .field("app_key", &self.app_key)
            .field("app_secret", &"<REDACTED>")
            .field("access_token", &"<REDACTED>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_credential() {
        let credential = Credential::new(
            Ustr::from("test_key"),
            "test_secret".to_string(),
            "test_token".to_string(),
        );
        assert_eq!(credential.app_key, Ustr::from("test_key"));
        assert_eq!(credential.app_secret, "test_secret");
        assert_eq!(credential.access_token, "test_token");
    }

    #[test]
    fn test_credential_debug_redaction() {
        let credential = Credential::new(
            Ustr::from("test_key"),
            "my_secret_value".to_string(),
            "my_token_value".to_string(),
        );
        let debug_str = format!("{:?}", credential);

        // Check that actual secret values are redacted
        assert!(!debug_str.contains("my_secret_value"), "Debug output should not contain actual secret value");
        assert!(!debug_str.contains("my_token_value"), "Debug output should not contain actual token value");

        // Check that redaction markers are present
        assert!(debug_str.contains("<REDACTED>"), "Debug output should contain '<REDACTED>' markers");
    }
}
