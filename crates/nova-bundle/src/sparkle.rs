//! Sparkle, the macOS updater `Nova.app` ships: the pinned release, the
//! steps that put a trimmed, arm64-only `Sparkle.framework` in
//! `Contents/Frameworks`, and the order its code is signed in.
//!
//! [`stage`] downloads the release archive with `curl` into the downloads
//! directory, beside staging ([`Layout::downloads`]), checks its SHA-256
//! against [`PIN`] with `shasum` ([`crate::digest::check_sha256`]) before
//! anything unpacks it,
//! unpacks the framework and its licence, removes the XPC services (the
//! game isn't sandboxed, and Sparkle uses them only when
//! `SUEnableInstallerLauncherService` or `SUEnableDownloaderService` is
//! set), thins each Mach-O to arm64, checks it, and copies the framework,
//! links kept, into the app, with the licence in `Contents/Resources`.
//! [`nested_code`] is the inside-out order both signing modes sign it in,
//! before the app.
//!
//! # Bumping the pin
//!
//! 1. Change [`VERSION`], [`ARCHIVE`], [`URL`] and [`SHA256`] together: the
//!    digest is `shasum -a 256` of the release's `.tar.xz`, which GitHub
//!    also reports for the asset (`gh release view <version> -R
//!    sparkle-project/Sparkle --json assets`).
//! 2. Re-run the investigation the phase body records (rdm
//!    `phase/auto-updates/phase-4-sparkle-on-macos`, "Findings"): the
//!    archive's tree, each Mach-O's architectures (thinning needs a
//!    universal binary), the entitlements, the XPC services' paths and an
//!    ad-hoc trial signature in the order [`nested_code`] gives.
//! 3. Bundle, then run the gated `macos_dmg` test (`NOVA_DMG`) and
//!    nova-appkit's `updater` test (`NOVA_SPARKLE_FRAMEWORKS`) on it.

use std::path::{Path, PathBuf};

use crate::bundle::{ARCH, Step};
use crate::digest::{DigestTool, Pin};
use crate::layout::Layout;
use crate::ports::Invocation;

/// The pinned Sparkle release.
pub const VERSION: &str = "2.10.1";
/// Its archive's file name.
pub const ARCHIVE: &str = "Sparkle-2.10.1.tar.xz";
/// Where the archive is downloaded from: the release's asset on GitHub.
pub const URL: &str =
    "https://github.com/sparkle-project/Sparkle/releases/download/2.10.1/Sparkle-2.10.1.tar.xz";
/// The archive's SHA-256, lower-case hex.
pub const SHA256: &str = "1a856d2a9edb8e628de88121d7be04cf0e2167f8641650b78fca00b14244cf27";
/// The archive's pin, as the check names it.
pub const PIN: Pin = Pin {
    what: "Sparkle 2.10.1",
    sha256: SHA256,
};
/// The framework's directory name.
pub const FRAMEWORK: &str = "Sparkle.framework";

/// The framework's `Versions/B`, where its code is.
fn version_b(framework: &Path) -> PathBuf {
    framework.join("Versions").join("B")
}

/// The framework's Mach-O files, each thinned to [`ARCH`]: the framework's
/// own, `Autoupdate`, and `Updater.app`'s executable.
fn binaries(framework: &Path) -> [PathBuf; 3] {
    let version = version_b(framework);
    [
        version.join("Sparkle"),
        version.join("Autoupdate"),
        version
            .join("Updater.app")
            .join("Contents")
            .join("MacOS")
            .join("Updater"),
    ]
}

/// The steps that put the pinned, checked, trimmed and thinned framework in
/// `Contents/Frameworks` and its licence in `Contents/Resources` (see the
/// module docs). `ditto` copies the framework with its links, which
/// `Files::copy` wouldn't keep. Removing the `XPCServices` link (left
/// dangling once `Versions/B/XPCServices` is gone) removes the link alone.
#[must_use]
pub fn stage(layout: &Layout) -> Vec<Step> {
    let downloads = layout.downloads();
    let archive = layout.sparkle_archive();
    let unpacked = downloads.join(FRAMEWORK);
    let mut steps = vec![
        Step::RemoveDir(downloads.clone()),
        Step::CreateDir(downloads.clone()),
        Step::Run(
            Invocation::new("curl")
                .args([
                    "--fail",
                    "--silent",
                    "--show-error",
                    "--location",
                    "--proto",
                    "=https",
                    "--retry",
                    "3",
                    "--output",
                ])
                .arg(&archive)
                .arg(URL),
        ),
        Step::CheckSha256 {
            path: archive.clone(),
            pin: PIN,
            tool: DigestTool::Shasum,
        },
        Step::Run(
            Invocation::new("tar")
                .arg("-xJf")
                .arg(&archive)
                .arg("-C")
                .arg(&downloads)
                .args([format!("./{FRAMEWORK}"), "./LICENSE".to_owned()]),
        ),
        Step::RemoveDir(version_b(&unpacked).join("XPCServices")),
        Step::RemoveDir(unpacked.join("XPCServices")),
    ];
    let binaries = binaries(&unpacked);
    steps.extend(binaries.iter().map(|binary| {
        Step::Run(
            Invocation::new("lipo")
                .arg(binary)
                .args(["-thin", ARCH, "-output"])
                .arg(binary),
        )
    }));
    steps.extend(binaries.into_iter().map(Step::CheckArchs));
    steps.extend([
        Step::CreateDir(layout.frameworks()),
        Step::Run(
            Invocation::new("ditto")
                .arg(&unpacked)
                .arg(layout.sparkle_framework()),
        ),
        Step::Copy {
            from: downloads.join("LICENSE"),
            to: layout.sparkle_license(),
        },
    ]);
    steps
}

/// The framework's code that is signed before the framework itself, then
/// the framework, inside out: `Autoupdate`, `Updater.app`, then
/// `Sparkle.framework`. The XPC services, which come first in Sparkle's
/// own order, are removed ([`stage`]).
#[must_use]
pub fn nested_code(framework: &Path) -> [PathBuf; 3] {
    let version = version_b(framework);
    [
        version.join("Autoupdate"),
        version.join("Updater.app"),
        framework.to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pin_downloads_the_version_s_release_archive() {
        assert_eq!(PIN.what, format!("Sparkle {VERSION}"));
        assert_eq!(PIN.sha256, SHA256);
        assert!(
            URL.ends_with(&format!("/releases/download/{VERSION}/{ARCHIVE}")),
            "{URL}"
        );
        assert_eq!(ARCHIVE, format!("Sparkle-{VERSION}.tar.xz"));
        assert!(URL.starts_with("https://"), "{URL}");
        assert_eq!(SHA256.len(), 64);
        assert!(
            SHA256
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{SHA256}"
        );
    }

    #[test]
    fn nested_code_is_listed_inside_out() {
        let framework = Path::new("/x").join("Sparkle.framework");
        let version = framework.join("Versions").join("B");
        assert_eq!(
            nested_code(&framework),
            [
                version.join("Autoupdate"),
                version.join("Updater.app"),
                framework.clone(),
            ]
        );
    }
}
