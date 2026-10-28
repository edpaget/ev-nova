//! The Linux desktop integration: the desktop file, the AppStream
//! metainfo and the hicolor icons, written as a `share/` tree that
//! installs into `~/.local/share` (or `/usr/share`).
//!
//! ```text
//! <OUT_DIR>/share/
//!   applications/<identifier>.desktop
//!   metainfo/<identifier>.metainfo.xml
//!   icons/hicolor/<n>x<n>/apps/<identifier>.png
//! ```
//!
//! Each file is named for `[package.metadata.bundle] identifier`, the
//! same key the game's window takes its Wayland app id and X11 `WM_CLASS`
//! from (`nova::platform::APP_ID`), so docks and task switchers match the
//! running window to the desktop file and show its icon and name.
//!
//! [`desktop::desktop_entry`] and [`metainfo::metainfo`] render the files
//! from the [`BundleSpec`](crate::spec::BundleSpec), as
//! [`info_plist`](crate::info_plist) renders `Info.plist`; [`share`] lays
//! the tree out and writes it through the [`Files`](crate::ports::Files)
//! port. Nothing here needs a Linux host: the `nova-linux` binary runs
//! anywhere.
//!
//! [`appimage`] and [`flatpak`] pack the game with that tree into the
//! nightly's two other Linux downloads. Their tools run on x86-64 Linux
//! alone.

pub mod appimage;
pub mod desktop;
pub mod flatpak;
pub mod host;
pub mod icons;
pub mod metainfo;
pub mod share;

use std::path::{Path, PathBuf};

use crate::spec::GAME_PACKAGE;

/// The command the desktop file starts and the metainfo says the game
/// provides: the game package's binary, found on `PATH`.
pub const EXECUTABLE: &str = GAME_PACKAGE;

/// The desktop file's menu categories: a game (the main category), and a
/// role-playing one. Not the Apple category in the spec, which is a
/// different vocabulary.
pub const CATEGORIES: &str = "Game;RolePlaying;";

/// An output directory that is, holds or lies in one of a package's work
/// directories: CI uploads everything in the output directory, and the
/// work directories are removed or rebuilt on each run.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error(
    "the output directory {} overlaps the work directory {}: choose another",
    out.display(),
    work.display()
)]
pub struct OutputOverlapsWork {
    /// The work directory it overlaps.
    pub work: PathBuf,
    /// The output directory asked for.
    pub out: PathBuf,
}

impl OutputOverlapsWork {
    /// Fails on the first of `work` that `out` is, holds or lies in.
    pub fn check(out: &Path, work: &[&Path]) -> Result<(), Self> {
        match work
            .iter()
            .find(|work| work.starts_with(out) || out.starts_with(work))
        {
            Some(work) => Err(Self {
                work: work.to_path_buf(),
                out: out.to_path_buf(),
            }),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_output_beside_the_work_directories_is_fine() {
        let root = Path::new("/t").join("pkg");
        let (a, b) = (root.join("a"), root.join("b"));
        assert_eq!(
            OutputOverlapsWork::check(&root.join("out"), &[&a, &b]),
            Ok(())
        );
        assert_eq!(
            OutputOverlapsWork::check(&root.join("ab"), &[&a, &b]),
            Ok(())
        );
    }

    #[test]
    fn the_first_work_directory_overlapped_is_named() {
        let root = Path::new("/t").join("pkg");
        let (a, b) = (root.join("a"), root.join("b"));
        for (out, work) in [
            (b.clone(), &b),
            (b.join("x"), &b),
            (root.clone(), &a),
            (a.clone(), &a),
        ] {
            assert_eq!(
                OutputOverlapsWork::check(&out, &[&a, &b]),
                Err(OutputOverlapsWork {
                    work: work.clone(),
                    out: out.clone(),
                }),
                "{}",
                out.display()
            );
        }
    }
}
