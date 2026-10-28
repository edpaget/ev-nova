//! The update feed `nova.exe` compiles in: the channel's Windows appcast
//! and the public key its updates are checked with, worked out from
//! `NOVA_CHANNEL` and `[package.metadata.bundle] update_public_key` in
//! nova's `Cargo.toml`.
//!
//! nova's build script passes the result to rustc for Windows targets, and
//! the game hands it to WinSparkle. A build names a feed only with both a
//! channel and a committed key, so it never names a feed it couldn't check.
//! The bundler makes the same decision for the macOS bundle and the
//! installer (`nova_bundle::channel::feed_status`); a test there pins the
//! two to each other, so they can't drift.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// The variable that names the channel, as the bundler reads it.
pub const CHANNEL_VAR: &str = "NOVA_CHANNEL";

/// The channels a build can name, by their names.
const CHANNELS: [&str; 1] = ["nightly"];

/// The repository whose releases carry the feeds.
const REPO: &str = "edpaget/ev-nova";

/// The feed a Windows build compiles in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateFeed {
    /// The channel's Windows appcast.
    pub url: String,
    /// The committed public key, as standard base64.
    pub public_key: String,
}

/// Why no feed could be worked out.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeedError {
    /// The manifest is not TOML.
    #[error("the manifest is not TOML: {0}")]
    Toml(String),
    /// It has no `[package.metadata.bundle]` table.
    #[error("the manifest has no [package.metadata.bundle] table")]
    NoBundleTable,
    /// `NOVA_CHANNEL` names a channel there is none of. The wording is the
    /// bundler's, which refuses the same value.
    #[error(
        "{CHANNEL_VAR} is `{0}`: set it to {known}, or leave it unset for a development bundle",
        known = CHANNELS.join(" or ")
    )]
    UnknownChannel(String),
    /// `update_public_key` is set, but not to a key. It never holds the
    /// value.
    #[error("[package.metadata.bundle] update_public_key: {0}")]
    Key(&'static str),
}

/// The feed a Windows build of the Cargo manifest `manifest` (its text)
/// compiles in when it is built for `channel` (`NOVA_CHANNEL`'s value).
///
/// It is `None` without a channel (unset or empty: a development build),
/// and without a committed key (`update_public_key` absent or empty).
///
/// # Errors
///
/// An unknown channel; a key that isn't standard base64 of 32 bytes; a
/// manifest that isn't TOML or has no `[package.metadata.bundle]` table.
pub fn windows_update_feed(
    manifest: &str,
    channel: Option<&str>,
) -> Result<Option<UpdateFeed>, FeedError> {
    let channel = match channel {
        None | Some("") => return Ok(None),
        Some(name) if CHANNELS.contains(&name) => name,
        Some(name) => return Err(FeedError::UnknownChannel(name.to_owned())),
    };
    let manifest: toml::Table = manifest
        .parse()
        .map_err(|error: toml::de::Error| FeedError::Toml(error.to_string()))?;
    let bundle = manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("bundle"))
        .and_then(toml::Value::as_table)
        .ok_or(FeedError::NoBundleTable)?;
    let key = match bundle.get("update_public_key") {
        None => return Ok(None),
        Some(value) => value.as_str().ok_or(FeedError::Key("it is not a string"))?,
    };
    if key.is_empty() {
        return Ok(None);
    }
    let bytes = STANDARD
        .decode(key.trim())
        .map_err(|_| FeedError::Key("it is not standard base64"))?;
    if bytes.len() != 32 {
        return Err(FeedError::Key("it doesn't decode to 32 bytes"));
    }
    Ok(Some(UpdateFeed {
        url: format!(
            "https://github.com/{REPO}/releases/download/{channel}/appcast-windows-{channel}.xml"
        ),
        public_key: STANDARD.encode(bytes),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(key: Option<&str>) -> String {
        let key = key.map_or(String::new(), |key| format!("update_public_key = {key}\n"));
        format!(
            "[package]\nname = \"nova\"\n\n[package.metadata.bundle]\nidentifier = \
             \"io.example.nova\"\n{key}"
        )
    }

    fn base64_of(len: usize) -> String {
        format!("\"{}\"", STANDARD.encode(vec![7_u8; len]))
    }

    #[test]
    fn no_channel_or_an_empty_one_is_a_development_build() {
        let with_key = manifest(Some(&base64_of(32)));
        assert_eq!(windows_update_feed(&with_key, None), Ok(None));
        assert_eq!(windows_update_feed(&with_key, Some("")), Ok(None));
        // A development build reads nothing, so even a broken manifest is fine.
        assert_eq!(windows_update_feed("[package", None), Ok(None));
    }

    #[test]
    fn nightly_without_a_committed_key_has_no_feed() {
        for manifest in [manifest(None), manifest(Some("\"\""))] {
            assert_eq!(
                windows_update_feed(&manifest, Some("nightly")),
                Ok(None),
                "{manifest}"
            );
        }
    }

    #[test]
    fn nightly_with_a_key_is_the_nightly_windows_appcast_and_the_key() {
        let key = STANDARD.encode(std::array::from_fn::<u8, 32, _>(|i| i as u8));
        assert_eq!(
            windows_update_feed(&manifest(Some(&format!("\"{key}\""))), Some("nightly")),
            Ok(Some(UpdateFeed {
                url: "https://github.com/edpaget/ev-nova/releases/download/nightly/\
                      appcast-windows-nightly.xml"
                    .to_owned(),
                public_key: key.clone(),
            }))
        );
        // Surrounding whitespace aside, as the bundler reads it.
        assert_eq!(
            windows_update_feed(&manifest(Some(&format!("\" {key}\\n\""))), Some("nightly"))
                .map(|feed| feed.map(|feed| feed.public_key)),
            Ok(Some(key))
        );
    }

    #[test]
    fn an_unknown_channel_is_an_error_naming_it() {
        for value in ["Nightly", "stable"] {
            let error = windows_update_feed(&manifest(None), Some(value)).expect_err("unknown");
            assert_eq!(error, FeedError::UnknownChannel(value.to_owned()));
            assert_eq!(
                error.to_string(),
                format!(
                    "NOVA_CHANNEL is `{value}`: set it to nightly, or leave it unset for a \
                     development bundle"
                )
            );
        }
    }

    #[test]
    fn a_key_that_isnt_base64_of_32_bytes_is_an_error_that_never_shows_it() {
        for (key, shown) in [
            (base64_of(31), STANDARD.encode([7_u8; 31])),
            (base64_of(33), STANDARD.encode([7_u8; 33])),
            ("\"not base64!\"".to_owned(), "not base64!".to_owned()),
            ("7".to_owned(), "7".to_owned()),
        ] {
            let error =
                windows_update_feed(&manifest(Some(&key)), Some("nightly")).expect_err("not a key");
            assert!(matches!(error, FeedError::Key(_)), "{key}: {error:?}");
            let message = error.to_string();
            assert!(message.contains("update_public_key"), "{message}");
            assert!(!message.contains(&shown), "{message} shows {shown}");
        }
    }

    #[test]
    fn a_manifest_without_the_table_is_an_error() {
        for manifest in [
            "[package]\nname = \"nova\"\n",
            "[package.metadata]\nbundle = 3\n",
        ] {
            assert_eq!(
                windows_update_feed(manifest, Some("nightly")),
                Err(FeedError::NoBundleTable),
                "{manifest}"
            );
        }
        assert!(matches!(
            windows_update_feed("[package\nname =", Some("nightly")),
            Err(FeedError::Toml(_))
        ));
    }
}
