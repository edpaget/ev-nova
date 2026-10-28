//! Where the installer is staged, where the executables are taken from,
//! and where the `.msi` goes.
//!
//! ```text
//! <target>/installer/staging/
//!   nova.ico   the icon of the shortcut and of Apps & features
//!   nova.wxs   the WiX source
//! <target>/installer/out/
//!   ev-nova-x86_64-pc-windows-msvc.msi
//! ```
//!
//! The executables are read from `<target>/x86_64-pc-windows-msvc/release`
//! unless another directory is given, and the `.msi` goes to
//! `<target>/installer/out` unless another is. The output directory never
//! overlaps staging: CI uploads everything in its output directory.

use std::path::{Path, PathBuf};

use super::{MSI, TARGET};

/// The staging, executables and output directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerLayout {
    /// Where the icon and the WiX source are written.
    pub staging: PathBuf,
    /// Where `nova.exe` and `nova-dump.exe` are read from.
    pub exes: PathBuf,
    /// Where the `.msi` is written.
    pub out: PathBuf,
}

/// The output directory asked for can't be used.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InstallerLayoutError {
    /// It is, holds or lies in the staging directory.
    #[error(
        "the output directory {} overlaps the staging directory {}: choose another",
        out.display(),
        staging.display()
    )]
    OutputOverlapsStaging {
        /// The staging directory.
        staging: PathBuf,
        /// The output directory asked for.
        out: PathBuf,
    },
}

impl InstallerLayout {
    /// The layout under Cargo's `target_dir`, writing the `.msi` to `out`
    /// and reading the executables from `exes`, or the defaults when they
    /// are not given.
    pub fn new(
        target_dir: &Path,
        out: Option<PathBuf>,
        exes: Option<PathBuf>,
    ) -> Result<Self, InstallerLayoutError> {
        let installer = target_dir.join("installer");
        let staging = installer.join("staging");
        let out = out.unwrap_or_else(|| installer.join("out"));
        if staging.starts_with(&out) || out.starts_with(&staging) {
            return Err(InstallerLayoutError::OutputOverlapsStaging { staging, out });
        }
        let exes = exes.unwrap_or_else(|| target_dir.join(TARGET).join("release"));
        Ok(Self { staging, exes, out })
    }

    /// The icon, made from the macOS `.icns`.
    #[must_use]
    pub fn icon(&self) -> PathBuf {
        self.staging.join("nova.ico")
    }

    /// The WiX source.
    #[must_use]
    pub fn wxs(&self) -> PathBuf {
        self.staging.join("nova.wxs")
    }

    /// The `.msi`.
    #[must_use]
    pub fn msi(&self) -> PathBuf {
        self.out.join(MSI)
    }

    /// The executable `name` to pack.
    #[must_use]
    pub fn exe(&self, name: &str) -> PathBuf {
        self.exes.join(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> PathBuf {
        Path::new("/work").join("ev-nova").join("target")
    }

    #[test]
    fn stages_reads_and_writes_under_the_target_dir_by_default() {
        let layout = InstallerLayout::new(&target(), None, None).expect("layout");
        let installer = target().join("installer");
        assert_eq!(
            layout,
            InstallerLayout {
                staging: installer.join("staging"),
                exes: target().join("x86_64-pc-windows-msvc").join("release"),
                out: installer.join("out"),
            }
        );
        assert_eq!(layout.icon(), installer.join("staging").join("nova.ico"));
        assert_eq!(layout.wxs(), installer.join("staging").join("nova.wxs"));
        assert_eq!(
            layout.msi(),
            installer
                .join("out")
                .join("ev-nova-x86_64-pc-windows-msvc.msi")
        );
        assert_eq!(
            layout.exe("nova.exe"),
            target()
                .join("x86_64-pc-windows-msvc")
                .join("release")
                .join("nova.exe")
        );
    }

    #[test]
    fn takes_the_executables_and_output_dirs_given() {
        let exes = Path::new("/signed").join("exes");
        let out = Path::new("/work").join("dist");
        let layout =
            InstallerLayout::new(&target(), Some(out.clone()), Some(exes.clone())).expect("layout");
        assert_eq!(layout.exes, exes);
        assert_eq!(layout.out, out);
        assert_eq!(layout.staging, target().join("installer").join("staging"));
    }

    #[test]
    fn an_output_dir_overlapping_staging_is_refused() {
        let staging = target().join("installer").join("staging");
        for out in [
            staging.clone(),
            staging.join("out"),
            target().join("installer"),
            target(),
        ] {
            assert_eq!(
                InstallerLayout::new(&target(), Some(out.clone()), None),
                Err(InstallerLayoutError::OutputOverlapsStaging {
                    staging: staging.clone(),
                    out: out.clone(),
                }),
                "{}",
                out.display()
            );
        }
    }

    #[test]
    fn a_sibling_of_staging_is_a_fine_output_dir() {
        let out = target().join("installer").join("previous");
        let layout = InstallerLayout::new(&target(), Some(out.clone()), None).expect("layout");
        assert_eq!(layout.out, out);
    }
}
