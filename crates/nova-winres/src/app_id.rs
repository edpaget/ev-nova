//! The app id: `[package.metadata.bundle] identifier` in nova's
//! `Cargo.toml`, which nova's build script reads into the window's
//! Wayland app id and X11 `WM_CLASS`, and nova-bundle into the bundle
//! identifier and the Linux desktop file's name.

/// Why a manifest gave no identifier.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AppIdError {
    /// The manifest is not TOML.
    #[error("the manifest is not TOML: {0}")]
    Toml(String),
    /// It has no `[package.metadata.bundle]` table.
    #[error("the manifest has no [package.metadata.bundle] table")]
    NoBundleTable,
    /// The table has no `identifier` string.
    #[error("[package.metadata.bundle] has no `identifier` string")]
    NoIdentifier,
}

/// The `[package.metadata.bundle] identifier` of the Cargo manifest
/// `manifest` (its text).
///
/// # Errors
///
/// When the text isn't TOML, or has no such table or string.
pub fn bundle_identifier(manifest: &str) -> Result<String, AppIdError> {
    let manifest: toml::Table = manifest
        .parse()
        .map_err(|error: toml::de::Error| AppIdError::Toml(error.to_string()))?;
    let bundle = manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("bundle"))
        .and_then(toml::Value::as_table)
        .ok_or(AppIdError::NoBundleTable)?;
    bundle
        .get("identifier")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or(AppIdError::NoIdentifier)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
[package]
name = "nova"
version.workspace = true

[dependencies]
winit.workspace = true

[package.metadata.bundle]
identifier = "io.example.nova"
name = "EV Nova"
"#;

    #[test]
    fn reads_the_bundle_identifier() {
        assert_eq!(
            bundle_identifier(MANIFEST),
            Ok("io.example.nova".to_owned())
        );
    }

    #[test]
    fn a_manifest_without_the_table_is_an_error() {
        for manifest in [
            "[package]\nname = \"nova\"\n",
            "[package.metadata]\nother = 1\n",
            "[package.metadata]\nbundle = 3\n",
        ] {
            assert_eq!(
                bundle_identifier(manifest),
                Err(AppIdError::NoBundleTable),
                "{manifest}"
            );
        }
    }

    #[test]
    fn a_table_without_an_identifier_string_is_an_error() {
        for manifest in [
            "[package.metadata.bundle]\nname = \"EV Nova\"\n",
            "[package.metadata.bundle]\nidentifier = 7\n",
        ] {
            assert_eq!(
                bundle_identifier(manifest),
                Err(AppIdError::NoIdentifier),
                "{manifest}"
            );
        }
    }

    #[test]
    fn text_that_is_not_toml_is_an_error() {
        assert!(matches!(
            bundle_identifier("[package\nname ="),
            Err(AppIdError::Toml(_))
        ));
    }
}
