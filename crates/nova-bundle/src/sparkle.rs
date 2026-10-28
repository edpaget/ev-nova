//! Sparkle, the macOS updater `Nova.app` ships: the pinned release, the
//! steps that put a trimmed, arm64-only `Sparkle.framework` in
//! `Contents/Frameworks`, and the order its code is signed in.
//!
//! [`stage`] downloads the release archive with `curl` into the downloads
//! directory, beside staging ([`Layout::downloads`]), checks its SHA-256
//! against [`SHA256`] ([`check_sha256`]) before anything unpacks it,
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

use crate::bundle::{ARCH, BundleError, Step, run};
use crate::layout::Layout;
use crate::ports::{Invocation, Process};

/// The pinned Sparkle release.
pub const VERSION: &str = "2.10.1";
/// Its archive's file name.
pub const ARCHIVE: &str = "Sparkle-2.10.1.tar.xz";
/// Where the archive is downloaded from: the release's asset on GitHub.
pub const URL: &str =
    "https://github.com/sparkle-project/Sparkle/releases/download/2.10.1/Sparkle-2.10.1.tar.xz";
/// The archive's SHA-256, lower-case hex.
pub const SHA256: &str = "1a856d2a9edb8e628de88121d7be04cf0e2167f8641650b78fca00b14244cf27";
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
            sha256: SHA256,
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

/// `shasum -a 256 <path>`, captured: the digest the check reads.
#[must_use]
pub fn shasum(path: &Path) -> Invocation {
    Invocation::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .captured()
}

/// Runs [`shasum`] on `path` and fails unless the digest it prints first
/// is `expected`, ignoring case: a download that isn't the pinned archive
/// stops the run before anything unpacks it.
pub fn check_sha256(
    process: &mut impl Process,
    path: &Path,
    expected: &str,
) -> Result<(), BundleError> {
    let stdout = run(process, &shasum(path))?;
    let text = String::from_utf8_lossy(&stdout);
    let found = text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if found == expected.to_ascii_lowercase() {
        Ok(())
    } else {
        Err(BundleError::Checksum {
            path: path.to_owned(),
            expected: expected.to_owned(),
            found,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::RunError;

    /// Answers every run with `stdout`, recording each.
    struct Prints {
        stdout: Vec<u8>,
        runs: Vec<Invocation>,
    }

    impl Process for Prints {
        fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError> {
            self.runs.push(invocation.clone());
            Ok(self.stdout.clone())
        }
    }

    fn prints(stdout: &str) -> Prints {
        Prints {
            stdout: stdout.as_bytes().to_vec(),
            runs: Vec::new(),
        }
    }

    fn archive() -> PathBuf {
        Path::new("/d").join(ARCHIVE)
    }

    #[test]
    fn a_matching_digest_passes_however_shasum_spaces_it() {
        let upper = SHA256.to_ascii_uppercase();
        for stdout in [
            format!("{SHA256}  /d/{ARCHIVE}\n"),
            format!("{SHA256}\n"),
            format!("  {SHA256} *x"),
            format!("{upper}  x\n"),
        ] {
            let mut process = prints(&stdout);
            check_sha256(&mut process, &archive(), SHA256).expect(&stdout);
            assert_eq!(process.runs, [shasum(&archive())]);
        }
        assert_eq!(
            shasum(&archive()),
            Invocation::new("shasum")
                .args(["-a", "256"])
                .arg(archive())
                .captured()
        );
    }

    #[test]
    fn another_digest_is_an_error_naming_the_file_and_both_digests() {
        let other = "0".repeat(64);
        let mut process = prints(&format!("{other}  x\n"));
        let error = check_sha256(&mut process, &archive(), SHA256).expect_err("mismatch");
        let BundleError::Checksum {
            path,
            expected,
            found,
        } = &error
        else {
            panic!("{error:?}");
        };
        assert_eq!(path, &archive());
        assert_eq!(expected, SHA256);
        assert_eq!(found, &other);
        assert_eq!(
            error.to_string(),
            format!(
                "{} has SHA-256 `{other}`, not `{SHA256}`, the pinned Sparkle 2.10.1's",
                archive().display()
            )
        );
    }

    #[test]
    fn an_empty_shasum_output_is_an_error() {
        for stdout in ["", "  \n"] {
            let error =
                check_sha256(&mut prints(stdout), &archive(), SHA256).expect_err("no digest");
            assert!(
                matches!(&error, BundleError::Checksum { found, .. } if found.is_empty()),
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_failing_shasum_is_its_run_error() {
        struct Fails;
        impl Process for Fails {
            fn run(&mut self, _invocation: &Invocation) -> Result<Vec<u8>, RunError> {
                Err(RunError::Failed(Some(1)))
            }
        }
        let error = check_sha256(&mut Fails, &archive(), SHA256).expect_err("fails");
        assert!(
            matches!(&error, BundleError::Run { command, .. } if *command == shasum(&archive()).to_string()),
            "{error:?}"
        );
    }

    #[test]
    fn the_pin_downloads_the_version_s_release_archive() {
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
