//! Where the Flatpak is staged and built, and where the bundle goes: see
//! the [module docs](super).

use std::path::{Path, PathBuf};

use super::ASSET;
use crate::linux::OutputOverlapsWork;
use crate::spec::BundleSpec;

/// The work and output directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatpakLayout {
    /// The manifest, the cargo config, the vendored crates and the share
    /// tree.
    pub staging: PathBuf,
    /// `flatpak-builder`'s build directory.
    pub build: PathBuf,
    /// The `OSTree` repository it exports to.
    pub repo: PathBuf,
    /// Its cache, instead of `.flatpak-builder` in the working directory.
    pub state: PathBuf,
    /// Where the bundle is written.
    pub out: PathBuf,
}

impl FlatpakLayout {
    /// The layout under Cargo's `target_dir`, writing the bundle to `out`
    /// or, when none is given, to `<target_dir>/flatpak/out`. An output
    /// directory that is, holds or lies in a work directory is refused.
    pub fn new(target_dir: &Path, out: Option<PathBuf>) -> Result<Self, OutputOverlapsWork> {
        let flatpak = target_dir.join("flatpak");
        let layout = Self {
            staging: flatpak.join("staging"),
            build: flatpak.join("build"),
            repo: flatpak.join("repo"),
            state: flatpak.join("state"),
            out: out.unwrap_or_else(|| flatpak.join("out")),
        };
        OutputOverlapsWork::check(
            &layout.out,
            &[&layout.staging, &layout.build, &layout.repo, &layout.state],
        )?;
        Ok(layout)
    }

    /// The manifest, named for the app id as Flathub names it.
    #[must_use]
    pub fn manifest(&self, spec: &BundleSpec) -> PathBuf {
        self.staging.join(format!("{}.json", spec.identifier))
    }

    /// `cargo vendor`'s tree.
    #[must_use]
    pub fn vendor(&self) -> PathBuf {
        self.staging.join("vendor")
    }

    /// The cargo config replacing crates.io with [`vendor`](Self::vendor).
    #[must_use]
    pub fn cargo_config(&self) -> PathBuf {
        self.staging.join("cargo-config.toml")
    }

    /// The share tree's directory: [`share::steps`](crate::linux::share::steps)
    /// writes under `<staging>/share`.
    #[must_use]
    pub fn share(&self) -> PathBuf {
        self.staging.join("share")
    }

    /// The bundle.
    #[must_use]
    pub fn bundle(&self) -> PathBuf {
        self.out.join(ASSET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::tests::sample_metadata;

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn target() -> PathBuf {
        spec().target_dir
    }

    #[test]
    fn works_and_writes_under_the_target_dir_by_default() {
        let layout = FlatpakLayout::new(&target(), None).expect("layout");
        let flatpak = target().join("flatpak");
        let staging = flatpak.join("staging");
        assert_eq!(
            layout,
            FlatpakLayout {
                staging: staging.clone(),
                build: flatpak.join("build"),
                repo: flatpak.join("repo"),
                state: flatpak.join("state"),
                out: flatpak.join("out"),
            }
        );
        assert_eq!(
            layout.manifest(&spec()),
            staging.join("io.example.nova.json")
        );
        assert_eq!(layout.vendor(), staging.join("vendor"));
        assert_eq!(layout.cargo_config(), staging.join("cargo-config.toml"));
        assert_eq!(layout.share(), staging.join("share"));
        assert_eq!(
            layout.bundle(),
            flatpak
                .join("out")
                .join("ev-nova-x86_64-unknown-linux-gnu.flatpak")
        );
    }

    #[test]
    fn writes_to_the_output_dir_given() {
        let out = Path::new("/work").join("dist");
        let layout = FlatpakLayout::new(&target(), Some(out.clone())).expect("layout");
        assert_eq!(layout.bundle(), out.join(ASSET));
    }

    #[test]
    fn an_output_dir_overlapping_a_work_dir_is_refused() {
        let flatpak = target().join("flatpak");
        for (out, work) in [
            (flatpak.join("staging"), "staging"),
            (flatpak.join("build").join("x"), "build"),
            (flatpak.join("repo"), "repo"),
            (flatpak.join("state"), "state"),
            (flatpak.clone(), "staging"),
        ] {
            assert_eq!(
                FlatpakLayout::new(&target(), Some(out.clone())),
                Err(OutputOverlapsWork {
                    work: flatpak.join(work),
                    out: out.clone(),
                }),
                "{}",
                out.display()
            );
        }
    }
}
