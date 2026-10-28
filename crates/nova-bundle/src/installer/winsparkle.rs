//! WinSparkle, the Windows updater the installer ships beside `nova.exe`:
//! the pinned release, and the steps that download it, check it and unpack
//! its x64 DLL and licences into the downloads directory
//! ([`InstallerLayout::downloads`]), where `wix build` packs them from.
//!
//! [`stage`] downloads the release archive with `curl`, checks its SHA-256
//! against [`ARCHIVE_PIN`] before anything unpacks it, unpacks the DLL and
//! the two licences with `tar` (Windows' own bsdtar, which reads zip),
//! their paths stripped, and checks the DLL against [`DLL_PIN`]: a DLL that
//! isn't the pinned one stops the pack before `wix`. Both checks run
//! `certutil`, which every Windows has. The DLL carries no Authenticode
//! signature of its own; the signing phase signs the staged file after the
//! pin check.
//!
//! # Bumping the pin
//!
//! 1. Change every constant here together: the archive's digest is
//!    `certutil -hashfile <zip> SHA256` (or `shasum -a 256`) of the
//!    release's `.zip`, which GitHub also reports for the asset (`gh
//!    release view v<version> -R vslavik/winsparkle --json assets`), and
//!    the DLL's is that of `<top>/x64/Release/WinSparkle.dll` inside it.
//! 2. Re-run the investigation the phase body records (rdm
//!    `phase/auto-updates/phase-5-winsparkle-on-windows`, "Findings"): the
//!    archive's layout, the licences, `include/winsparkle.h`'s calls and
//!    `src/ui.cpp`'s install flow (the shutdown request).
//! 3. Run the dispatched `ci.yml`'s Windows legs: the installer test and
//!    nova-win32's real-DLL test (`NOVA_WINSPARKLE_DLL`).

use super::layout::InstallerLayout;
use crate::bundle::Step;
use crate::digest::{DigestTool, Pin};
use crate::ports::Invocation;

/// The pinned WinSparkle release.
pub const VERSION: &str = "0.9.4";
/// Its archive's file name.
pub const ARCHIVE: &str = "WinSparkle-0.9.4.zip";
/// Where the archive is downloaded from: the release's asset on GitHub.
pub const URL: &str =
    "https://github.com/vslavik/winsparkle/releases/download/v0.9.4/WinSparkle-0.9.4.zip";
/// The archive's SHA-256, lower-case hex.
pub const ARCHIVE_SHA256: &str = "6037df37fc263bd1650a1c4949681a9d40ffe991d01f35892a406cb5d103c976";
/// The archive's pin, as the check names it.
pub const ARCHIVE_PIN: Pin = Pin {
    what: "WinSparkle 0.9.4 archive",
    sha256: ARCHIVE_SHA256,
};
/// The archive's top-level directory.
pub const TOP: &str = "WinSparkle-0.9.4";
/// The DLL's file name, beside `nova.exe`.
pub const DLL: &str = "WinSparkle.dll";
/// The x64 DLL's path in the archive.
pub const DLL_MEMBER: &str = "WinSparkle-0.9.4/x64/Release/WinSparkle.dll";
/// The x64 DLL's SHA-256, lower-case hex.
pub const DLL_SHA256: &str = "9b43b1c16ee39fb9a91b5bd75138767898779510e0836be2919250607cdbe8ab";
/// The DLL's pin, as the check names it.
pub const DLL_PIN: Pin = Pin {
    what: "WinSparkle 0.9.4 DLL",
    sha256: DLL_SHA256,
};
/// The licences: each one's name under [`TOP`], and the name it is
/// installed as beside `nova.exe`. `COPYING` is WinSparkle's (with the
/// OpenSSL notice), `COPYING.expat` Expat's.
pub const LICENCES: [(&str, &str); 2] = [
    ("COPYING", "WinSparkle-LICENSE.txt"),
    ("COPYING.expat", "WinSparkle-expat-LICENSE.txt"),
];

/// The steps that put the pinned, checked DLL and its licences in the
/// downloads directory (see the module docs). `--strip-components` drops
/// the archive's directories, so the DLL lands at
/// [`InstallerLayout::winsparkle_dll`], a stable path for a signing step.
#[must_use]
pub fn stage(layout: &InstallerLayout) -> Vec<Step> {
    let downloads = &layout.downloads;
    let archive = layout.winsparkle_archive();
    let depth = DLL_MEMBER.split('/').count() - 1;
    vec![
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
            pin: ARCHIVE_PIN,
            tool: DigestTool::Certutil,
        },
        Step::Run(
            Invocation::new("tar")
                .arg("-xf")
                .arg(&archive)
                .arg("-C")
                .arg(downloads)
                .arg("--strip-components")
                .arg(depth.to_string())
                .arg(DLL_MEMBER),
        ),
        Step::Run(
            Invocation::new("tar")
                .arg("-xf")
                .arg(&archive)
                .arg("-C")
                .arg(downloads)
                .args(["--strip-components", "1"])
                .args(LICENCES.map(|(member, _)| format!("{TOP}/{member}"))),
        ),
        Step::CheckSha256 {
            path: layout.winsparkle_dll(),
            pin: DLL_PIN,
            tool: DigestTool::Certutil,
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn is_digest(text: &str) -> bool {
        text.len() == 64
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    }

    #[test]
    fn the_pin_downloads_the_version_s_release_archive() {
        assert!(
            URL.ends_with(&format!("/releases/download/v{VERSION}/{ARCHIVE}")),
            "{URL}"
        );
        assert!(URL.starts_with("https://"), "{URL}");
        assert_eq!(ARCHIVE, format!("WinSparkle-{VERSION}.zip"));
        assert_eq!(TOP, format!("WinSparkle-{VERSION}"));
        assert_eq!(DLL_MEMBER, format!("{TOP}/x64/Release/{DLL}"));
        for (pin, sha256) in [(ARCHIVE_PIN, ARCHIVE_SHA256), (DLL_PIN, DLL_SHA256)] {
            assert_eq!(pin.sha256, sha256);
            assert!(is_digest(sha256), "{sha256}");
            assert!(
                pin.what.starts_with(&format!("WinSparkle {VERSION} ")),
                "{pin:?}"
            );
        }
        assert_ne!(ARCHIVE_PIN.what, DLL_PIN.what);
    }

    #[test]
    fn stage_downloads_checks_unpacks_and_checks_the_dll() {
        let target = Path::new("/work").join("target");
        let layout = InstallerLayout::new(&target, None, None).expect("layout");
        let downloads = target.join("installer").join("downloads");
        let archive = downloads.join("WinSparkle-0.9.4.zip");
        assert_eq!(
            stage(&layout),
            vec![
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
                        .arg(
                            "https://github.com/vslavik/winsparkle/releases/download/v0.9.4/\
                             WinSparkle-0.9.4.zip"
                        ),
                ),
                Step::CheckSha256 {
                    path: archive.clone(),
                    pin: ARCHIVE_PIN,
                    tool: DigestTool::Certutil,
                },
                Step::Run(
                    Invocation::new("tar")
                        .arg("-xf")
                        .arg(&archive)
                        .arg("-C")
                        .arg(&downloads)
                        .args([
                            "--strip-components",
                            "3",
                            "WinSparkle-0.9.4/x64/Release/WinSparkle.dll",
                        ]),
                ),
                Step::Run(
                    Invocation::new("tar")
                        .arg("-xf")
                        .arg(&archive)
                        .arg("-C")
                        .arg(&downloads)
                        .args([
                            "--strip-components",
                            "1",
                            "WinSparkle-0.9.4/COPYING",
                            "WinSparkle-0.9.4/COPYING.expat",
                        ]),
                ),
                Step::CheckSha256 {
                    path: downloads.join("WinSparkle.dll"),
                    pin: DLL_PIN,
                    tool: DigestTool::Certutil,
                },
            ]
        );
    }
}
