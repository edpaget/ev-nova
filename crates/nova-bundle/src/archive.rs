//! The portable archives the nightly ships beside the installers: the
//! Windows `.zip` and the Linux `.tar.gz`, each holding one top-level
//! directory with the binaries, `README.txt` and `LICENSE`. `ci.yml`'s
//! "Package" step asks `nova-site --archive-dir` and `--archive` for the
//! names, and the download page links the same files, so neither spells
//! them. macOS has none: the `.dmg` replaced its tarball.

use crate::installer;
use crate::linux::appimage;

/// One archive: its top-level directory and its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Archive {
    /// The one directory at the archive's root.
    pub dir: &'static str,
    /// The archive's file name, the release asset's name.
    pub file: &'static str,
}

/// The Windows zip.
pub const WINDOWS: Archive = Archive {
    dir: "ev-nova-x86_64-pc-windows-msvc",
    file: "ev-nova-x86_64-pc-windows-msvc.zip",
};

/// The Linux tarball.
pub const LINUX: Archive = Archive {
    dir: "ev-nova-x86_64-unknown-linux-gnu",
    file: "ev-nova-x86_64-unknown-linux-gnu.tar.gz",
};

/// The archive packed for `target`, if one is.
#[must_use]
pub fn for_target(target: &str) -> Option<Archive> {
    match target {
        installer::TARGET => Some(WINDOWS),
        appimage::TARGET => Some(LINUX),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle;

    #[test]
    fn windows_and_linux_have_an_archive_and_macos_none() {
        assert_eq!(for_target(installer::TARGET), Some(WINDOWS));
        assert_eq!(for_target(appimage::TARGET), Some(LINUX));
        assert_eq!(for_target(bundle::TARGET), None);
        assert_eq!(for_target("junk"), None);
        assert_eq!(for_target(""), None);
    }

    #[test]
    fn each_directory_is_the_project_and_target_and_each_file_adds_its_format() {
        assert_eq!(WINDOWS.dir, format!("ev-nova-{}", installer::TARGET));
        assert_eq!(WINDOWS.file, format!("{}.zip", WINDOWS.dir));
        assert_eq!(LINUX.dir, format!("ev-nova-{}", appimage::TARGET));
        assert_eq!(LINUX.file, format!("{}.tar.gz", LINUX.dir));
    }
}
