//! The update-signing public key: 32 bytes, written as standard base64,
//! the encoding of Sparkle's `SUPublicEDKey`.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// An Ed25519 public key: the half the apps embed to check each update's
/// `sparkle:edSignature`. It is not secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(pub [u8; 32]);

/// Why text is not a 32-byte key. It never holds the text, so a seed
/// that fails to read is never shown.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    /// The text is not standard base64.
    #[error("it is not standard base64")]
    NotBase64,
    /// The text decodes to the wrong number of bytes.
    #[error("it decodes to {0} bytes, not 32")]
    Length(usize),
}

/// The 32 bytes `text` holds as standard base64, surrounding whitespace
/// aside.
pub(crate) fn decode_32(text: &str) -> Result<[u8; 32], KeyError> {
    let bytes = STANDARD
        .decode(text.trim())
        .map_err(|_| KeyError::NotBase64)?;
    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| KeyError::Length(bytes.len()))
}

impl PublicKey {
    /// Reads a key from standard base64.
    ///
    /// # Errors
    ///
    /// Text that isn't base64, or isn't 32 bytes once decoded.
    pub fn from_base64(text: &str) -> Result<Self, KeyError> {
        decode_32(text).map(Self)
    }

    /// The key as standard base64.
    #[must_use]
    pub fn to_base64(&self) -> String {
        STANDARD.encode(self.0)
    }
}

/// As [`PublicKey::to_base64`].
impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_public_key_round_trips_through_base64() {
        let key = PublicKey(std::array::from_fn(|i| u8::try_from(i * 7).expect("small")));
        let text = key.to_base64();
        assert_eq!(text.len(), 44, "{text}");
        assert_eq!(PublicKey::from_base64(&text), Ok(key));
        assert_eq!(key.to_string(), text);
        assert_eq!(PublicKey::from_base64(&format!(" {text}\n")), Ok(key));
    }

    #[test]
    fn a_public_key_that_isnt_32_bytes_or_base64_is_an_error() {
        assert_eq!(
            PublicKey::from_base64(&STANDARD.encode([1; 31])),
            Err(KeyError::Length(31))
        );
        assert_eq!(
            PublicKey::from_base64(&STANDARD.encode([1; 33])),
            Err(KeyError::Length(33))
        );
        assert_eq!(
            PublicKey::from_base64("not base64!"),
            Err(KeyError::NotBase64)
        );
        assert_eq!(PublicKey::from_base64(""), Err(KeyError::Length(0)));
    }

    #[test]
    fn a_key_error_never_shows_the_text() {
        let text = "c2VjcmV0LXNlZWQtdmFsdWU=";
        for error in [
            decode_32(text).expect_err("23 bytes"),
            decode_32("sec ret!").expect_err("not base64"),
        ] {
            assert!(!error.to_string().contains("sec"), "{error}");
            assert!(!format!("{error:?}").contains("sec"), "{error:?}");
        }
    }
}
