//! The Windows installer: a per-user MSI holding `nova.exe`,
//! `nova-dump.exe`, `README.txt` and `LICENSE`, with a Start menu
//! shortcut and an Apps & features entry, built by WiX's `wix` from
//! executables that are already built.
//!
//! Packing never builds: [`plan::pack`] takes `nova.exe` and
//! `nova-dump.exe` as they are from the directory it is given
//! ([`layout::InstallerLayout::exes`]), so a signing step can run between
//! the build and the pack, and another can sign the `.msi` after it.
//!
//! It installs into `%LOCALAPPDATA%\Programs\Nova`, for the user alone
//! and without administrator rights. Not `…\Programs\EV Nova`: that is
//! where a player may have unzipped the original game, whose `Nova Files`
//! the game looks for there. Settings and pilots (`%APPDATA%\nova`) are
//! never part of it, so uninstalling and upgrading leave them alone.
//!
//! [`wxs::wxs`] writes the WiX source, [`plan::plan`] is the sequence as
//! data, and [`host`] reads the edge's arguments.

pub mod host;
pub mod layout;
pub mod plan;
pub mod wxs;

use std::path::PathBuf;

use nova_winres::{IconError, VersionError, version_quad};

use crate::bundle::BundleError;
use layout::InstallerLayoutError;

/// The MSI's `UpgradeCode`, which ties every build of the installer into
/// one product, so a newer one replaces an older one in place.
///
/// It must never change: a different one would make the next installer a
/// separate product, installed beside every copy already installed.
pub const UPGRADE_CODE: &str = "6ADE71A2-E471-45E8-B438-E2853F77D989";
/// The package's Manufacturer, shown in Apps & features.
pub const MANUFACTURER: &str = "ev-nova contributors";
/// The install folder's name under `%LOCALAPPDATA%\Programs`.
pub const INSTALL_DIR: &str = "Nova";
/// The Start menu shortcut's name (`Nova.lnk`).
pub const SHORTCUT: &str = "Nova";
/// The one target packed.
pub const TARGET: &str = "x86_64-pc-windows-msvc";
/// The `.msi`'s file name: the nightly's per-target asset name.
pub const MSI: &str = "ev-nova-x86_64-pc-windows-msvc.msi";
/// The game.
pub const GAME_EXE: &str = "nova.exe";
/// The data dumper.
pub const DUMP_EXE: &str = "nova-dump.exe";
/// The executables packed, in the order they are checked and installed.
pub const EXES: [&str; 2] = [GAME_EXE, DUMP_EXE];
/// The `HKCU` key whose values are the installed components' key paths,
/// as Windows Installer requires of files under the user's profile.
pub const REGISTRY_KEY: &str = r"Software\ev-nova\Nova";

/// The largest major or minor version an MSI's `ProductVersion` holds.
pub const MAX_MAJOR_MINOR: u16 = 255;

/// Why the installer could not be packed.
#[derive(Debug, thiserror::Error)]
pub enum InstallerError {
    /// The package version is not `major.minor.patch`.
    #[error(transparent)]
    Version(#[from] VersionError),
    /// The major or minor version is over [`MAX_MAJOR_MINOR`].
    #[error("{0:?} can't be an MSI's version: its major and minor numbers must be at most 255")]
    VersionTooLarge(String),
    /// The output directory can't be used.
    #[error(transparent)]
    Layout(#[from] InstallerLayoutError),
    /// An executable to pack is not there.
    #[error(
        "{} is missing: build the executables first (`mise run build-windows`), \
         or pass their directory with --exes",
        .0.display()
    )]
    MissingExe(PathBuf),
    /// The icon could not be made.
    #[error("the icon: {0}")]
    Icon(#[from] IconError),
    /// `cargo metadata`, a file step or `wix` failed.
    #[error(transparent)]
    Step(#[from] BundleError),
}

/// The MSI `ProductVersion` for the package's semver `version`:
/// `major.minor.patch`, with any pre-release or build part dropped.
pub fn msi_version(version: &str) -> Result<String, InstallerError> {
    let [major, minor, patch, _] = version_quad(version)?;
    if major > MAX_MAJOR_MINOR || minor > MAX_MAJOR_MINOR {
        return Err(InstallerError::VersionTooLarge(version.to_owned()));
    }
    Ok(format!("{major}.{minor}.{patch}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_is_its_three_numbers() {
        assert_eq!(msi_version("0.1.0").expect("version"), "0.1.0");
        assert_eq!(
            msi_version("255.255.65535").expect("version"),
            "255.255.65535"
        );
    }

    #[test]
    fn a_pre_release_or_build_part_is_dropped() {
        assert_eq!(msi_version("1.2.3-rc.1+abc").expect("version"), "1.2.3");
    }

    #[test]
    fn a_major_or_minor_over_255_is_refused() {
        for version in ["256.0.0", "0.256.0"] {
            assert!(
                matches!(msi_version(version), Err(InstallerError::VersionTooLarge(v)) if v == version),
                "{version}"
            );
        }
    }

    #[test]
    fn a_version_that_is_not_three_numbers_is_refused() {
        assert!(matches!(
            msi_version("1.2"),
            Err(InstallerError::Version(_))
        ));
    }
}
