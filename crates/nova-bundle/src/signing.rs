//! The signing mode, chosen from six environment variables, and the
//! Developer ID steps: a temporary keychain holding the certificate,
//! `codesign` with the hardened runtime, notarization and stapling.
//!
//! [`Signing::from_env`] picks Developer ID when all of [`VARS`] are set
//! and non-empty, and ad-hoc when none is. A mix is an error naming the
//! missing variables, so a half-configured CI fails instead of shipping an
//! ad-hoc build.

use std::ffi::{OsStr, OsString};
use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::ports::Env;

/// The base64 of the Developer ID Application `.p12` (certificate and
/// private key).
pub const CERT_P12: &str = "MACOS_CERT_P12_BASE64";
/// The `.p12`'s export password.
pub const CERT_PASSWORD: &str = "MACOS_CERT_PASSWORD";
/// The `codesign --sign` identity, `Developer ID Application: <name>
/// (<team>)`.
pub const IDENTITY: &str = "MACOS_SIGNING_IDENTITY";
/// The base64 of the App Store Connect API key's `AuthKey_<id>.p8`.
pub const API_KEY_P8: &str = "APPLE_API_KEY_P8_BASE64";
/// The API key's id, `notarytool --key-id`.
pub const API_KEY_ID: &str = "APPLE_API_KEY_ID";
/// The API key's issuer, `notarytool --issuer`.
pub const API_ISSUER: &str = "APPLE_API_ISSUER_ID";

/// The six signing variables, named as the CI secrets that fill them.
pub const VARS: [&str; 6] = [
    CERT_P12,
    CERT_PASSWORD,
    IDENTITY,
    API_KEY_P8,
    API_KEY_ID,
    API_ISSUER,
];

/// How the bundle is signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signing {
    /// Ad-hoc (`codesign --sign -`), not notarized: what runs without the
    /// signing variables (a local build, a fork's pull request).
    AdHoc,
    /// Developer ID with the hardened runtime, notarized and stapled.
    DeveloperId(Credentials),
}

/// The Developer ID inputs, decoded.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    /// The `codesign --sign` identity.
    pub identity: OsString,
    /// The `.p12`, decoded.
    pub cert_p12: Vec<u8>,
    /// The `.p12`'s password.
    pub cert_password: OsString,
    /// The `.p8` API key, decoded.
    pub api_key_p8: Vec<u8>,
    /// The API key's id.
    pub api_key_id: OsString,
    /// The API key's issuer.
    pub api_issuer: OsString,
}

/// As derived, but with the password and both keys left out.
impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("identity", &self.identity)
            .field("cert_p12", &"<redacted>")
            .field("cert_password", &"<redacted>")
            .field("api_key_p8", &"<redacted>")
            .field("api_key_id", &self.api_key_id)
            .field("api_issuer", &self.api_issuer)
            .finish()
    }
}

/// Why the signing variables give no mode. Never holds a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SigningError {
    /// Some of the variables are set and these are not.
    #[error(
        "the Developer ID signing variables are only partly set: {} unset or \
         empty. Set all six of {} for a Developer ID build, or none for an \
         ad-hoc one",
        missing.join(", "),
        VARS.join(", ")
    )]
    Partial {
        /// The unset or empty variables, in [`VARS`]' order.
        missing: Vec<&'static str>,
    },
    /// The variable is not base64.
    #[error("{0} is not valid base64")]
    NotBase64(&'static str),
}

impl Signing {
    /// The mode the signing variables in `env` choose.
    pub fn from_env(env: &impl Env) -> Result<Self, SigningError> {
        let values = VARS.map(|name| env.var(name).filter(|v| !v.is_empty()));
        let missing: Vec<_> = VARS
            .iter()
            .zip(&values)
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| *name)
            .collect();
        if missing.len() == VARS.len() {
            return Ok(Self::AdHoc);
        }
        if !missing.is_empty() {
            return Err(SigningError::Partial { missing });
        }
        let [
            cert_p12,
            cert_password,
            identity,
            api_key_p8,
            api_key_id,
            api_issuer,
        ] = values.map(Option::unwrap_or_default);
        Ok(Self::DeveloperId(Credentials {
            identity,
            cert_p12: decode(CERT_P12, &cert_p12)?,
            cert_password,
            api_key_p8: decode(API_KEY_P8, &api_key_p8)?,
            api_key_id,
            api_issuer,
        }))
    }
}

/// Decodes `value`, the variable `name`, ignoring ASCII whitespace so a
/// wrapped encoding works.
fn decode(name: &'static str, value: &OsStr) -> Result<Vec<u8>, SigningError> {
    let text: Vec<u8> = value
        .as_encoded_bytes()
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    STANDARD
        .decode(text)
        .map_err(|_| SigningError::NotBase64(name))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// The environment as a map.
    #[derive(Default)]
    struct MapEnv(HashMap<&'static str, OsString>);

    impl Env for MapEnv {
        fn var(&self, name: &str) -> Option<OsString> {
            self.0.get(name).cloned()
        }
    }

    const P12: &[u8] = b"p12 bytes\x00\xff";
    const P8: &[u8] = b"-----BEGIN PRIVATE KEY-----";

    /// Every variable set, to values none of which holds another.
    fn full() -> MapEnv {
        MapEnv(HashMap::from([
            (CERT_P12, STANDARD.encode(P12).into()),
            (CERT_PASSWORD, "pw-secret-value".into()),
            (IDENTITY, "Developer ID Application: Ed (TEAM123456)".into()),
            (API_KEY_P8, STANDARD.encode(P8).into()),
            (API_KEY_ID, "KEYID12345".into()),
            (API_ISSUER, "issuer-uuid".into()),
        ]))
    }

    pub(crate) fn credentials() -> Credentials {
        Credentials {
            identity: "Developer ID Application: Ed (TEAM123456)".into(),
            cert_p12: P12.to_vec(),
            cert_password: "pw-secret-value".into(),
            api_key_p8: P8.to_vec(),
            api_key_id: "KEYID12345".into(),
            api_issuer: "issuer-uuid".into(),
        }
    }

    #[test]
    fn no_variables_sign_ad_hoc() {
        assert_eq!(Signing::from_env(&MapEnv::default()), Ok(Signing::AdHoc));
    }

    #[test]
    fn six_empty_variables_sign_ad_hoc() {
        let env = MapEnv(VARS.iter().map(|v| (*v, OsString::new())).collect());
        assert_eq!(Signing::from_env(&env), Ok(Signing::AdHoc));
    }

    #[test]
    fn all_six_sign_with_developer_id() {
        assert_eq!(
            Signing::from_env(&full()),
            Ok(Signing::DeveloperId(credentials()))
        );
    }

    #[test]
    fn wrapped_base64_decodes() {
        let mut env = full();
        let wrap = |bytes: &[u8]| {
            let text = STANDARD.encode(bytes);
            let (a, b) = text.split_at(4);
            OsString::from(format!(" {a}\n{b}\r\n\t"))
        };
        env.0.insert(CERT_P12, wrap(P12));
        env.0.insert(API_KEY_P8, wrap(P8));
        assert_eq!(
            Signing::from_env(&env),
            Ok(Signing::DeveloperId(credentials()))
        );
    }

    #[test]
    fn each_missing_variable_is_named_and_no_value_shown() {
        for var in VARS {
            for absent in [None, Some(OsString::new())] {
                let mut env = full();
                match &absent {
                    None => env.0.remove(var),
                    Some(empty) => env.0.insert(var, empty.clone()),
                };
                let error = Signing::from_env(&env).expect_err("partial");
                assert_eq!(error, SigningError::Partial { missing: vec![var] });
                let text = error.to_string();
                assert!(text.contains(var), "{text}");
                for value in full().0.values() {
                    let value = value.to_string_lossy();
                    assert!(!text.contains(value.as_ref()), "{text} shows {value}");
                }
            }
        }
    }

    #[test]
    fn several_missing_are_named_in_order() {
        let mut env = full();
        env.0.remove(API_ISSUER);
        env.0.remove(CERT_P12);
        assert_eq!(
            Signing::from_env(&env),
            Err(SigningError::Partial {
                missing: vec![CERT_P12, API_ISSUER]
            })
        );
    }

    #[test]
    fn invalid_base64_names_its_variable() {
        for var in [CERT_P12, API_KEY_P8] {
            let mut env = full();
            env.0.insert(var, "not base64!".into());
            assert_eq!(Signing::from_env(&env), Err(SigningError::NotBase64(var)));
        }
    }

    #[test]
    fn credentials_debug_hides_the_password_and_keys() {
        let debug = format!("{:?}", credentials());
        assert!(!debug.contains("pw-secret-value"), "{debug}");
        assert!(!debug.contains("p12 bytes"), "{debug}");
        assert!(!debug.contains("PRIVATE KEY"), "{debug}");
        for byte in [P12, P8] {
            assert!(!debug.contains(&format!("{:?}", byte.to_vec())), "{debug}");
        }
        assert!(debug.contains("KEYID12345"), "{debug}");
    }
}
