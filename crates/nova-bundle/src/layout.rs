//! Where the bundle is staged and where the `.dmg` goes.
//!
//! The staging tree is the `.dmg`'s volume:
//!
//! ```text
//! <target>/bundle/staging/
//!   Nova.app/Contents/Info.plist
//!   Nova.app/Contents/MacOS/nova
//!   Nova.app/Contents/Frameworks/Sparkle.framework
//!   Nova.app/Contents/Resources/Nova.icns
//!   Nova.app/Contents/Resources/Sparkle-LICENSE.txt
//!   nova-dump
//!   README.txt
//!   LICENSE
//!   Applications -> /Applications
//! ```
//!
//! The `.dmg` goes to the output directory, `<target>/bundle/out` unless
//! one is given. Staging never lies under it: CI uploads everything in its
//! output directory (`dist/`).
//!
//! A Developer ID run keeps its secrets and scratch files beside staging,
//! in a directory it removes when it ends, and the output directory never
//! overlaps it:
//!
//! ```text
//! <target>/bundle/signing/
//!   developer-id.p12          the certificate, decoded
//!   AuthKey_<key id>.p8       the notary API key, decoded
//!   nova-signing.keychain-db  the temporary keychain
//!   Nova.zip                  the app as sent to the notary
//! ```
//!
//! Every run downloads Sparkle afresh into a third directory beside
//! staging, which it empties first, and the output directory never
//! overlaps it either:
//!
//! ```text
//! <target>/bundle/downloads/
//!   Sparkle-<version>.tar.xz  the release archive, checked
//!   Sparkle.framework         unpacked, trimmed and thinned
//!   LICENSE                   Sparkle's licence
//! ```

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::icon::ICON_FILE;
use crate::info_plist::EXECUTABLE;
use crate::sparkle;

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

/// The staging, signing and output directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The tree that becomes the `.dmg`'s volume.
    pub staging: PathBuf,
    /// Where a Developer ID run keeps its secrets and scratch files.
    signing: PathBuf,
    /// Where Sparkle is downloaded and unpacked.
    downloads: PathBuf,
    /// Where the `.dmg` is written.
    pub out: PathBuf,
}

/// The output directory asked for can't be used.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LayoutError {
    /// It would hold the staging tree, which CI would then upload.
    #[error(
        "the output directory {} holds the staging directory {}: choose another",
        out.display(),
        staging.display()
    )]
    StagingInOutput {
        /// The staging directory.
        staging: PathBuf,
        /// The output directory asked for.
        out: PathBuf,
    },
    /// It is, holds or lies in the signing directory, which holds secrets
    /// and is removed when the run ends.
    #[error(
        "the output directory {} overlaps the signing directory {}: choose another",
        out.display(),
        signing.display()
    )]
    OutputInSigning {
        /// The signing directory.
        signing: PathBuf,
        /// The output directory asked for.
        out: PathBuf,
    },
    /// It is, holds or lies in the downloads directory, which each run
    /// empties.
    #[error(
        "the output directory {} overlaps the downloads directory {}: choose another",
        out.display(),
        downloads.display()
    )]
    OutputInDownloads {
        /// The downloads directory.
        downloads: PathBuf,
        /// The output directory asked for.
        out: PathBuf,
    },
}

impl Layout {
    /// The layout under Cargo's `target_dir`, writing the `.dmg` to `out`
    /// or, when none is given, to `<target_dir>/bundle/out`.
    pub fn new(target_dir: &Path, out: Option<PathBuf>) -> Result<Self, LayoutError> {
        let bundle = target_dir.join("bundle");
        let staging = bundle.join("staging");
        let signing = bundle.join("signing");
        let downloads = bundle.join("downloads");
        let out = out.unwrap_or_else(|| bundle.join("out"));
        if staging.starts_with(&out) {
            return Err(LayoutError::StagingInOutput { staging, out });
        }
        if signing.starts_with(&out) || out.starts_with(&signing) {
            return Err(LayoutError::OutputInSigning { signing, out });
        }
        if downloads.starts_with(&out) || out.starts_with(&downloads) {
            return Err(LayoutError::OutputInDownloads { downloads, out });
        }
        Ok(Self {
            staging,
            signing,
            downloads,
            out,
        })
    }

    /// The signing directory, beside staging.
    #[must_use]
    pub fn signing(&self) -> PathBuf {
        self.signing.clone()
    }

    /// The temporary keychain the certificate is imported into.
    #[must_use]
    pub fn keychain(&self) -> PathBuf {
        self.signing.join("nova-signing.keychain-db")
    }

    /// The decoded Developer ID certificate.
    #[must_use]
    pub fn cert(&self) -> PathBuf {
        self.signing.join("developer-id.p12")
    }

    /// The decoded notary API key, named as App Store Connect names it.
    #[must_use]
    pub fn api_key(&self, key_id: &OsStr) -> PathBuf {
        let mut name = OsStr::new("AuthKey_").to_owned();
        name.push(key_id);
        name.push(".p8");
        self.signing.join(name)
    }

    /// The zip of `Nova.app` sent to the notary, which takes no bare
    /// bundle.
    #[must_use]
    pub fn app_zip(&self) -> PathBuf {
        self.signing.join("Nova.zip")
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

    /// Where the bundler downloads and unpacks what it ships but doesn't
    /// build (Sparkle), beside staging.
    #[must_use]
    pub fn downloads(&self) -> PathBuf {
        self.downloads.clone()
    }

    /// The Sparkle release archive, downloaded.
    #[must_use]
    pub fn sparkle_archive(&self) -> PathBuf {
        self.downloads.join(sparkle::ARCHIVE)
    }

    /// `Nova.app/Contents/Frameworks`.
    #[must_use]
    pub fn frameworks(&self) -> PathBuf {
        self.contents().join("Frameworks")
    }

    /// `Sparkle.framework` in `Contents/Frameworks`.
    #[must_use]
    pub fn sparkle_framework(&self) -> PathBuf {
        self.frameworks().join(sparkle::FRAMEWORK)
    }

    /// Sparkle's licence, which its notice asks copies to carry, in
    /// `Contents/Resources`.
    #[must_use]
    pub fn sparkle_license(&self) -> PathBuf {
        self.resources().join("Sparkle-LICENSE.txt")
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
                Err(LayoutError::StagingInOutput {
                    staging: target().join("bundle").join("staging"),
                    out,
                })
            );
        }
    }

    #[test]
    fn an_output_dir_in_or_at_the_signing_dir_is_refused() {
        let signing = target().join("bundle").join("signing");
        for out in [signing.clone(), signing.join("out")] {
            let refused = Layout::new(&target(), Some(out.clone()));
            assert_eq!(
                refused,
                Err(LayoutError::OutputInSigning {
                    signing: signing.clone(),
                    out: out.clone(),
                })
            );
            let text = refused.expect_err("refused").to_string();
            assert!(text.contains(&out.display().to_string()), "{text}");
        }
    }

    #[test]
    fn names_the_signing_paths_beside_staging() {
        let layout = Layout::new(&target(), None).expect("layout");
        let signing = target().join("bundle").join("signing");
        assert_eq!(layout.signing(), signing);
        assert_eq!(layout.keychain(), signing.join("nova-signing.keychain-db"));
        assert_eq!(layout.cert(), signing.join("developer-id.p12"));
        assert_eq!(
            layout.api_key(std::ffi::OsStr::new("KEY123")),
            signing.join("AuthKey_KEY123.p8")
        );
        assert_eq!(layout.app_zip(), signing.join("Nova.zip"));
        assert!(!layout.signing().starts_with(&layout.staging));
        assert!(!layout.signing().starts_with(&layout.out));
        let downloads = target().join("bundle").join("downloads");
        assert_eq!(layout.downloads(), downloads);
        assert_eq!(
            layout.sparkle_archive(),
            downloads.join("Sparkle-2.10.1.tar.xz")
        );
    }

    #[test]
    fn the_downloads_dir_is_beside_staging_and_never_in_the_output() {
        let dist = Path::new("/work").join("dist");
        for out in [None, Some(dist)] {
            let layout = Layout::new(&target(), out).expect("layout");
            assert_eq!(layout.downloads().parent(), layout.staging.parent());
            assert!(!layout.downloads().starts_with(&layout.staging));
            assert!(!layout.downloads().starts_with(&layout.out));
            assert!(!layout.downloads().starts_with(layout.signing()));
        }
    }

    #[test]
    fn an_output_dir_in_or_at_the_downloads_dir_is_refused() {
        let downloads = target().join("bundle").join("downloads");
        for out in [downloads.clone(), downloads.join("out")] {
            let refused = Layout::new(&target(), Some(out.clone()));
            assert_eq!(
                refused,
                Err(LayoutError::OutputInDownloads {
                    downloads: downloads.clone(),
                    out: out.clone(),
                })
            );
            let text = refused.expect_err("refused").to_string();
            assert!(text.contains(&out.display().to_string()), "{text}");
            assert!(text.contains(&downloads.display().to_string()), "{text}");
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
        let frameworks = contents.join("Frameworks");
        assert_eq!(layout.frameworks(), frameworks);
        assert_eq!(
            layout.sparkle_framework(),
            frameworks.join("Sparkle.framework")
        );
        assert_eq!(
            layout.sparkle_license(),
            contents.join("Resources").join("Sparkle-LICENSE.txt")
        );
    }
}
