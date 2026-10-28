//! Where the AppDir is staged and where the AppImage goes: see the
//! [module docs](super).

use std::path::{Path, PathBuf};

use super::ASSET;
use crate::linux::{EXECUTABLE, OutputOverlapsWork};

/// The staging and output directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppImageLayout {
    /// Where the AppDir is laid out.
    pub staging: PathBuf,
    /// Where the AppImage is written.
    pub out: PathBuf,
}

impl AppImageLayout {
    /// The layout under Cargo's `target_dir`, writing the AppImage to
    /// `out` or, when none is given, to `<target_dir>/appimage/out`. An
    /// output directory that is, holds or lies in staging is refused: CI
    /// uploads everything in it.
    pub fn new(target_dir: &Path, out: Option<PathBuf>) -> Result<Self, OutputOverlapsWork> {
        let appimage = target_dir.join("appimage");
        let staging = appimage.join("staging");
        let out = out.unwrap_or_else(|| appimage.join("out"));
        OutputOverlapsWork::check(&out, &[&staging])?;
        Ok(Self { staging, out })
    }

    /// The AppDir, which `appimagetool` packs.
    #[must_use]
    pub fn app_dir(&self) -> PathBuf {
        self.staging.join("AppDir")
    }

    /// The AppDir's `usr`, which holds the binary and the share tree.
    #[must_use]
    pub fn usr(&self) -> PathBuf {
        self.app_dir().join("usr")
    }

    /// The staged `nova`.
    #[must_use]
    pub fn executable(&self) -> PathBuf {
        self.usr().join("bin").join(EXECUTABLE)
    }

    /// The AppImage.
    #[must_use]
    pub fn appimage(&self) -> PathBuf {
        self.out.join(ASSET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> PathBuf {
        Path::new("/work").join("ev-nova").join("target")
    }

    #[test]
    fn stages_and_writes_under_the_target_dir_by_default() {
        let layout = AppImageLayout::new(&target(), None).expect("layout");
        let appimage = target().join("appimage");
        let app_dir = appimage.join("staging").join("AppDir");
        assert_eq!(
            layout,
            AppImageLayout {
                staging: appimage.join("staging"),
                out: appimage.join("out"),
            }
        );
        assert_eq!(layout.app_dir(), app_dir);
        assert_eq!(layout.usr(), app_dir.join("usr"));
        assert_eq!(
            layout.executable(),
            app_dir.join("usr").join("bin").join("nova")
        );
        assert_eq!(
            layout.appimage(),
            appimage
                .join("out")
                .join("ev-nova-x86_64-unknown-linux-gnu.AppImage")
        );
    }

    #[test]
    fn writes_to_the_output_dir_given() {
        let out = Path::new("/work").join("dist");
        let layout = AppImageLayout::new(&target(), Some(out.clone())).expect("layout");
        assert_eq!(layout.out, out);
        assert_eq!(layout.appimage(), out.join(ASSET));
    }

    #[test]
    fn an_output_dir_overlapping_staging_is_refused() {
        let staging = target().join("appimage").join("staging");
        for out in [
            staging.clone(),
            staging.join("AppDir"),
            target().join("appimage"),
            target(),
        ] {
            assert_eq!(
                AppImageLayout::new(&target(), Some(out.clone())),
                Err(OutputOverlapsWork {
                    work: staging.clone(),
                    out: out.clone(),
                }),
                "{}",
                out.display()
            );
        }
    }
}
