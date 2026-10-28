//! What the bundle is made of, read from `cargo metadata`: the workspace's
//! paths, and the game package's version, licence and
//! `[package.metadata.bundle]` table, which every platform's packaging
//! reads its identity from (the macOS `Info.plist`, the Windows MSI, the
//! Linux desktop file and AppStream metainfo).

use std::path::PathBuf;

use serde::Deserialize;

use crate::appcast::key::PublicKey;

/// The package whose binary is the app's executable.
pub const GAME_PACKAGE: &str = "nova";

/// The bundle's identity and where the workspace builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleSpec {
    /// The workspace root, holding `LICENSE` and `packaging/`.
    pub workspace_root: PathBuf,
    /// Cargo's target directory.
    pub target_dir: PathBuf,
    /// The game package's version (`CFBundleShortVersionString`; the
    /// build ID is `CFBundleVersion`).
    pub version: String,
    /// The game package's SPDX licence expression.
    pub license: String,
    /// `CFBundleIdentifier`.
    pub identifier: String,
    /// `CFBundleName` and `CFBundleDisplayName`.
    pub name: String,
    /// `LSApplicationCategoryType`.
    pub category: String,
    /// A line saying what the game is: the Linux desktop file's
    /// `Comment` and the AppStream `summary`.
    pub summary: String,
    /// A paragraph saying what the game is: the AppStream `description`.
    pub description: String,
    /// The project's web page: the AppStream homepage `url`.
    pub homepage: String,
    /// Who makes it: the AppStream `developer`.
    pub developer: String,
    /// The update-signing public key the apps check update feeds with,
    /// from the optional `update_public_key` (standard base64). `None`
    /// while the operator hasn't committed one (it is missing or empty):
    /// then the nightly publishes no update feeds.
    pub update_public_key: Option<PublicKey>,
}

/// Why `cargo metadata`'s output gave no spec.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SpecError {
    /// The output is not the JSON `cargo metadata` writes.
    #[error("cargo metadata's output could not be read: {0}")]
    Json(String),
    /// The workspace has no game package.
    #[error("the workspace has no `{GAME_PACKAGE}` package")]
    NoGamePackage,
    /// The game package has no `[package.metadata.bundle]` table.
    #[error("`{GAME_PACKAGE}`'s Cargo.toml has no [package.metadata.bundle] table")]
    NoBundleTable,
    /// The bundle table lacks a key.
    #[error("`{GAME_PACKAGE}`'s Cargo.toml has no `{0}` in [package.metadata.bundle]")]
    MissingKey(&'static str),
    /// The game package has no `license`.
    #[error("`{GAME_PACKAGE}`'s Cargo.toml has no `license`")]
    NoLicense,
    /// `update_public_key` is set, but not to a key.
    #[error("`{GAME_PACKAGE}`'s update_public_key in [package.metadata.bundle]: {0}")]
    UpdateKey(String),
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_root: PathBuf,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    license: Option<String>,
    #[serde(default)]
    metadata: Option<PackageMetadata>,
}

#[derive(Deserialize)]
struct PackageMetadata {
    bundle: Option<BundleTable>,
}

#[derive(Deserialize)]
struct BundleTable {
    identifier: Option<String>,
    name: Option<String>,
    category: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    homepage: Option<String>,
    developer: Option<String>,
    update_public_key: Option<String>,
}

impl BundleSpec {
    /// Reads the spec from the JSON of `cargo metadata --format-version 1`.
    pub fn from_metadata(json: &[u8]) -> Result<Self, SpecError> {
        let metadata: Metadata =
            serde_json::from_slice(json).map_err(|e| SpecError::Json(e.to_string()))?;
        let game = metadata
            .packages
            .into_iter()
            .find(|p| p.name == GAME_PACKAGE)
            .ok_or(SpecError::NoGamePackage)?;
        let table = game
            .metadata
            .and_then(|m| m.bundle)
            .ok_or(SpecError::NoBundleTable)?;
        Ok(Self {
            workspace_root: metadata.workspace_root,
            target_dir: metadata.target_directory,
            version: game.version,
            license: game.license.ok_or(SpecError::NoLicense)?,
            identifier: table
                .identifier
                .ok_or(SpecError::MissingKey("identifier"))?,
            name: table.name.ok_or(SpecError::MissingKey("name"))?,
            category: table.category.ok_or(SpecError::MissingKey("category"))?,
            summary: table.summary.ok_or(SpecError::MissingKey("summary"))?,
            description: table
                .description
                .ok_or(SpecError::MissingKey("description"))?,
            homepage: table.homepage.ok_or(SpecError::MissingKey("homepage"))?,
            developer: table.developer.ok_or(SpecError::MissingKey("developer"))?,
            update_public_key: update_public_key(table.update_public_key.as_deref())?,
        })
    }
}

/// The optional key: `None` when missing or empty.
fn update_public_key(text: Option<&str>) -> Result<Option<PublicKey>, SpecError> {
    match text {
        None | Some("") => Ok(None),
        Some(text) => PublicKey::from_base64(text)
            .map(Some)
            .map_err(|e| SpecError::UpdateKey(e.to_string())),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::{Value, json};

    /// A `cargo metadata` output holding the game package (with `bundle`
    /// as its bundle table) beside another package.
    pub(crate) fn metadata_with(bundle: &Value) -> Value {
        json!({
            "packages": [
                {
                    "name": "nova-dump",
                    "version": "9.9.9",
                    "license": "MIT",
                    "metadata": null,
                },
                {
                    "name": "nova",
                    "version": "0.3.1",
                    "license": "GPL-3.0-or-later",
                    "metadata": { "bundle": bundle },
                },
            ],
            "workspace_root": "/work/ev-nova",
            "target_directory": "/work/ev-nova/target",
            "version": 1,
        })
    }

    pub(crate) fn full_bundle() -> Value {
        json!({
            "identifier": "io.example.nova",
            "name": "EV Nova",
            "category": "public.app-category.role-playing-games",
            "summary": "Fly the stars",
            "description": "A space trading and combat game.",
            "homepage": "https://example.com/nova",
            "developer": "the nova authors",
        })
    }

    pub(crate) fn sample_metadata() -> Vec<u8> {
        metadata_with(&full_bundle()).to_string().into_bytes()
    }

    fn parse(value: &Value) -> Result<BundleSpec, SpecError> {
        BundleSpec::from_metadata(value.to_string().as_bytes())
    }

    #[test]
    fn reads_the_game_package_and_the_workspace_paths() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        assert_eq!(
            spec,
            BundleSpec {
                workspace_root: PathBuf::from("/work/ev-nova"),
                target_dir: PathBuf::from("/work/ev-nova/target"),
                version: "0.3.1".into(),
                license: "GPL-3.0-or-later".into(),
                identifier: "io.example.nova".into(),
                name: "EV Nova".into(),
                category: "public.app-category.role-playing-games".into(),
                summary: "Fly the stars".into(),
                description: "A space trading and combat game.".into(),
                homepage: "https://example.com/nova".into(),
                developer: "the nova authors".into(),
                update_public_key: None,
            }
        );
    }

    #[test]
    fn the_update_public_key_is_optional_and_empty_means_none() {
        let key = PublicKey([7; 32]);
        for (value, want) in [
            (None, None),
            (Some(json!("")), None),
            (Some(json!(key.to_base64())), Some(key)),
        ] {
            let mut bundle = full_bundle();
            if let Some(value) = value {
                bundle["update_public_key"] = value;
            }
            assert_eq!(
                parse(&metadata_with(&bundle)).map(|s| s.update_public_key),
                Ok(want)
            );
        }
    }

    #[test]
    fn a_malformed_update_public_key_is_an_error() {
        for bad in ["not base64!", "AAAA"] {
            let mut bundle = full_bundle();
            bundle["update_public_key"] = json!(bad);
            let error = parse(&metadata_with(&bundle)).expect_err("malformed");
            assert!(matches!(error, SpecError::UpdateKey(_)), "{error:?}");
            assert!(error.to_string().contains("update_public_key"), "{error}");
        }
    }

    #[test]
    fn a_workspace_without_the_game_package_is_an_error() {
        let mut value = metadata_with(&full_bundle());
        value["packages"][1]["name"] = json!("not-nova");
        assert_eq!(parse(&value), Err(SpecError::NoGamePackage));
    }

    #[test]
    fn a_game_package_without_a_bundle_table_is_an_error() {
        let mut value = metadata_with(&full_bundle());
        value["packages"][1]["metadata"] = Value::Null;
        assert_eq!(parse(&value), Err(SpecError::NoBundleTable));
        value["packages"][1]["metadata"] = json!({ "other": 1 });
        assert_eq!(parse(&value), Err(SpecError::NoBundleTable));
    }

    #[test]
    fn each_bundle_key_is_required() {
        for key in [
            "identifier",
            "name",
            "category",
            "summary",
            "description",
            "homepage",
            "developer",
        ] {
            let mut bundle = full_bundle();
            bundle.as_object_mut().expect("object").remove(key);
            assert_eq!(
                parse(&metadata_with(&bundle)),
                Err(SpecError::MissingKey(key)),
                "{key}"
            );
        }
    }

    #[test]
    fn a_game_package_without_a_licence_is_an_error() {
        let mut value = metadata_with(&full_bundle());
        value["packages"][1]["license"] = Value::Null;
        assert_eq!(parse(&value), Err(SpecError::NoLicense));
    }

    #[test]
    fn output_that_is_not_metadata_json_is_an_error() {
        assert!(matches!(
            BundleSpec::from_metadata(b"warning: not json"),
            Err(SpecError::Json(_))
        ));
    }
}
