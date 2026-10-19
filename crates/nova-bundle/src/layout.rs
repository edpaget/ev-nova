//! Where the bundle is staged and where the `.dmg` goes.
//!
//! The staging tree is the `.dmg`'s volume:
//!
//! ```text
//! <target>/bundle/staging/
//!   Nova.app/Contents/Info.plist
//!   Nova.app/Contents/MacOS/nova
//!   Nova.app/Contents/Resources/Nova.icns
//!   nova-dump
//!   README.txt
//!   LICENSE
//!   Applications -> /Applications
//! ```
//!
//! The `.dmg` goes to the output directory, `<target>/bundle/out` unless
//! one is given. Staging never lies under it: CI uploads everything in its
//! output directory (`dist/`).

use std::path::{Path, PathBuf};

use crate::icon::ICON_FILE;
use crate::info_plist::EXECUTABLE;

/// The bundle's file name. Not `EV Nova.app`: dragged into
/// `/Applications`, that would replace the original game, where the game
/// looks for the player's data.
pub const APP: &str = "Nova.app";
/// The data dumper, shipped beside the bundle.
pub const DUMP: &str = "nova-dump";
/// The `.dmg`'s file name: the nightly's per-target asset name.
pub const DMG: &str = "ev-nova-aarch64-apple-darwin.dmg";
/// The `.dmg`'s volume name.
pub const VOLUME: &str = "EV Nova";

/// The staging and output directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The tree that becomes the `.dmg`'s volume.
    pub staging: PathBuf,
    /// Where the `.dmg` is written.
    pub out: PathBuf,
}

/// The output directory would hold the staging tree.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error(
    "the output directory {} holds the staging directory {}: choose another",
    out.display(),
    staging.display()
)]
pub struct StagingInOutput {
    /// The staging directory.
    pub staging: PathBuf,
    /// The output directory asked for.
    pub out: PathBuf,
}

impl Layout {
    /// The layout under Cargo's `target_dir`, writing the `.dmg` to `out`
    /// or, when none is given, to `<target_dir>/bundle/out`.
    pub fn new(target_dir: &Path, out: Option<PathBuf>) -> Result<Self, StagingInOutput> {
        let bundle = target_dir.join("bundle");
        let staging = bundle.join("staging");
        let out = out.unwrap_or_else(|| bundle.join("out"));
        if staging.starts_with(&out) {
            return Err(StagingInOutput { staging, out });
        }
        Ok(Self { staging, out })
    }

    /// `Nova.app`.
    #[must_use]
    pub fn app(&self) -> PathBuf {
        self.staging.join(APP)
    }

    /// `Nova.app/Contents`.
    #[must_use]
    pub fn contents(&self) -> PathBuf {
        self.app().join("Contents")
    }

    /// `Nova.app/Contents/MacOS`.
    #[must_use]
    pub fn macos(&self) -> PathBuf {
        self.contents().join("MacOS")
    }

    /// `Nova.app/Contents/Resources`.
    #[must_use]
    pub fn resources(&self) -> PathBuf {
        self.contents().join("Resources")
    }

    /// `Nova.app/Contents/Info.plist`.
    #[must_use]
    pub fn info_plist(&self) -> PathBuf {
        self.contents().join("Info.plist")
    }

    /// The icon in `Contents/Resources`.
    #[must_use]
    pub fn icon(&self) -> PathBuf {
        self.resources().join(ICON_FILE)
    }

    /// The app's executable in `Contents/MacOS`.
    #[must_use]
    pub fn executable(&self) -> PathBuf {
        self.macos().join(EXECUTABLE)
    }

    /// `nova-dump`, at the volume's root.
    #[must_use]
    pub fn dump(&self) -> PathBuf {
        self.staging.join(DUMP)
    }

    /// The `.dmg`.
    #[must_use]
    pub fn dmg(&self) -> PathBuf {
        self.out.join(DMG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> PathBuf {
        Path::new("/work").join("target")
    }

    #[test]
    fn stages_and_writes_under_the_target_dir_by_default() {
        let layout = Layout::new(&target(), None).expect("layout");
        assert_eq!(layout.staging, target().join("bundle").join("staging"));
        assert_eq!(layout.out, target().join("bundle").join("out"));
        assert!(!layout.staging.starts_with(&layout.out));
        assert_eq!(layout.dmg(), layout.out.join(DMG));
    }

    #[test]
    fn writes_to_the_output_dir_given_with_staging_outside_it() {
        let dist = Path::new("/work").join("dist");
        let layout = Layout::new(&target(), Some(dist.clone())).expect("layout");
        assert_eq!(layout.out, dist);
        assert!(!layout.staging.starts_with(&layout.out));
        assert_eq!(layout.dmg(), dist.join("ev-nova-aarch64-apple-darwin.dmg"));
    }

    #[test]
    fn an_output_dir_holding_staging_is_refused() {
        for out in [target(), target().join("bundle")] {
            assert_eq!(
                Layout::new(&target(), Some(out.clone())),
                Err(StagingInOutput {
                    staging: target().join("bundle").join("staging"),
                    out,
                })
            );
        }
    }

    #[test]
    fn names_the_bundle_s_paths() {
        let layout = Layout::new(&target(), None).expect("layout");
        let app = layout.staging.join("Nova.app");
        let contents = app.join("Contents");
        assert_eq!(layout.app(), app);
        assert_eq!(layout.contents(), contents);
        assert_eq!(layout.info_plist(), contents.join("Info.plist"));
        assert_eq!(layout.macos(), contents.join("MacOS"));
        assert_eq!(layout.executable(), contents.join("MacOS").join("nova"));
        assert_eq!(layout.resources(), contents.join("Resources"));
        assert_eq!(layout.icon(), contents.join("Resources").join("Nova.icns"));
        assert_eq!(layout.dump(), layout.staging.join("nova-dump"));
    }
}
