//! The signed Flatpak repository on GitHub Pages: what `nova-flatpak-repo`
//! publishes on each `main` run, so a Flatpak installed from the
//! `.flatpakref`, or from the bundle built here, updates with `flatpak
//! update`.
//!
//! ```text
//! <work>/                      (--work: scratch, never uploaded)
//!   gnupg/                     the gpg home: the key's checks, then the secret
//!   secret/key.gpg             the decoded secret key, removed once imported
//!   src/                       the tested plain bundle, imported (archive mode),
//!                              or, for beta, the nightly commit copied in
//! <pages>/                     (--pages: the `pages-flatpak` artifact, /flatpak/)
//!   repo/                      the OSTree repository, every channel's branch
//!   ev-nova-nightly.flatpakref the one-click install of the nightly
//!   ev-nova-beta.flatpakref    the beta's, once the repo holds a beta branch
//! <dist>/                      (--dist: replaces the flatpak job's dist-* hand-off)
//!   ev-nova-x86_64-unknown-linux-gnu.flatpak   the channel's, follows the repo
//! ```
//!
//! # The two steps
//!
//! [`plan::stage_repo`] (`stage`) holds no secret and does all the network
//! I/O. It decides whether to publish at all ([`plan::decide`]), probes
//! the live repo ([`probe::live_from_probe`]), mirrors it from Pages when
//! there is one, GPG-verified against the committed key, to
//! [`MIRROR_DEPTH`] parents, and imports the plain bundle the `flatpak` job
//! built and tested. A Pages deploy replaces the whole site, so every run
//! rebuilds the whole tree from the live one. A `404` for the live repo's
//! `config` is the first publish and starts an empty repo; any other
//! answer, or a failed `curl`, fails the run, because taking an outage
//! for "no repo" would drop every other branch.
//!
//! [`plan::sign_repo`] (`sign`) holds the secret key alone, in its own CI
//! step with no token and no network. It imports the key into a gpg home
//! the work tree owns, checks its fingerprint is the committed key's,
//! commits the imported build onto the channel's branch with `flatpak
//! build-commit-from` (whose parent is the mirrored head; a build with the
//! same content as the head commits nothing), signs the summary, prunes
//! to [`KEPT_COMMITS`] commits per branch and makes static deltas with
//! `flatpak build-update-repo`, writes the `.flatpakref`
//! ([`flatpakref::render`]), builds the bundle with `--repo-url` and
//! `--gpg-keys`, from the signed repo, so an install of it follows the
//! repo, drops ostree's scratch `tmp/`, and checks the tree fits Pages
//! ([`probe::check_size`]).
//!
//! Every `sign` lists the repo's refs after the commit and writes the
//! `.flatpakref` of each channel whose branch the repo holds
//! ([`promote::served_channels`]), so a deploy never drops another
//! channel's.
//!
//! # Promotion
//!
//! A promoted channel (beta, [`Channel::promoted_from`]) is never built.
//! `stage` takes no `--bundle-dir` ([`bundle_dir_refusal`]): it mirrors the
//! live repo and imports nothing, and a `404` for the live repo is an
//! error, since there is no nightly branch to promote from. `sign` finds
//! the nightly branch's commit whose subject names the build
//! (`NOVA_BUILD_ID`), copies it into the work tree's repo, and commits it
//! onto the beta branch with `flatpak build-commit-from`, so the beta
//! commit's tree is the nightly's. See [`promote`].
//!
//! # When nothing is published
//!
//! Without a channel, without the committed key [`PUBLIC_KEY`], or
//! without the [`SECRET_VAR`] secret, `stage` exits [`NOT_PUBLISHED`] with
//! one `::warning::` and writes nothing, and CI runs no later step: the
//! nightly publishes exactly as before, with the plain bundle and no
//! `.flatpakref` link. A committed key that holds a secret key, or no
//! public key, and a secret whose key isn't the committed one, fail the
//! run instead ([`key`]).
//!
//! # Caching
//!
//! Pages' CDN caches files for about ten minutes. A client whose cached
//! summary still names the previous head finds its objects, because the
//! history is kept; a static delta pruned since can 404 until the cache
//! expires, and a retry succeeds.

pub mod flatpakref;
pub mod host;
pub mod key;
pub mod plan;
pub mod probe;
pub mod promote;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use self::key::KeyError;
use self::probe::{ProbeError, SizeError};
use self::promote::NotOnBranch;
use crate::appcast::Channel;
use crate::appcast::prune::KEEP_BUILDS;
use crate::bundle::BundleError;
use crate::site::PAGES;
use crate::spec::BundleSpec;

/// The committed public key, relative to the workspace root, as messages
/// name it ([`public_key`] is the path).
pub const PUBLIC_KEY: &str = "packaging/linux/flatpak/repo-key.asc";

/// The variable holding the secret key: standard base64, on one line, of
/// `gpg --export-secret-keys --export-options export-minimal <fpr>`. CI
/// sets it from the `update-signing` environment's secret in `sign`'s
/// step alone; GitHub gives a job without the secret an empty value,
/// which counts as unset.
pub const SECRET_VAR: &str = "FLATPAK_SIGNING_KEY";

/// The variable telling `stage`, which never holds the secret, whether
/// `sign` will: `true` when it will. CI sets it from
/// `secrets.FLATPAK_SIGNING_KEY != ''`. A set [`SECRET_VAR`] counts too.
pub const SECRET_SET_VAR: &str = "FLATPAK_SIGNING_KEY_SET";

/// The remote name the `.flatpakref` suggests, the same for every
/// channel, so a later channel's ref reuses the remote.
pub const REMOTE_NAME: &str = "ev-nova";

/// The repo's directory on the Pages site: the `pages-flatpak` artifact.
pub const PAGES_DIR: &str = "flatpak";

/// The repository's directory, in `--pages` and on the site.
pub const REPO_DIR: &str = "repo";

/// The one architecture the repo holds.
pub const ARCH: &str = "x86_64";

/// The repo's title, in its summary.
pub const TITLE: &str = "EV Nova";

/// The exit status when nothing is published: no channel, no committed
/// key, or no secret.
pub const NOT_PUBLISHED: u8 = 4;

/// The largest file Pages takes.
pub const MAX_FILE: u64 = 100_000_000;

/// The most the tree may hold: the Pages site limit is 1 GB, and the
/// download page needs a little of it.
pub const PAGES_BUDGET: u64 = 900_000_000;

/// The commits each branch keeps: as many builds as the Sparkle side
/// keeps, so a build either platform still holds can be promoted.
/// Keeping more than one also keeps the previous head's objects for
/// clients whose cached summary still names it.
pub const KEPT_COMMITS: usize = KEEP_BUILDS;

/// `ostree pull --depth`: the parents pulled beside each head, so the
/// mirror holds [`KEPT_COMMITS`] commits of each branch.
pub const MIRROR_DEPTH: usize = KEPT_COMMITS - 1;

/// `flatpak build-update-repo --prune-depth`: the parents kept beside
/// each head, so pruning leaves [`KEPT_COMMITS`] commits of each branch.
/// flatpak counts parents, as `ostree pull --depth` does: the dispatched
/// run's repo test publishes four builds and finds [`KEPT_COMMITS`] left.
pub const PRUNE_DEPTH: usize = KEPT_COMMITS - 1;

/// The repo's URL on the Pages site, ending in `/`.
#[must_use]
pub fn url() -> String {
    format!("{PAGES}/{PAGES_DIR}/{REPO_DIR}/")
}

/// The `.flatpakref`'s file name: `ev-nova-<channel>.flatpakref`.
#[must_use]
pub fn flatpakref_name(channel: Channel) -> String {
    format!("ev-nova-{}.flatpakref", channel.name())
}

/// The `.flatpakref`'s URL on the Pages site, beside the repo.
#[must_use]
pub fn flatpakref_url(channel: Channel) -> String {
    format!("{PAGES}/{PAGES_DIR}/{}", flatpakref_name(channel))
}

/// The app's ref on `channel`'s branch: `app/<identifier>/x86_64/<channel>`.
#[must_use]
pub fn app_ref(spec: &BundleSpec, channel: Channel) -> String {
    format!("app/{}/{ARCH}/{}", spec.identifier, channel.name())
}

/// The committed public key under the workspace root.
#[must_use]
pub fn public_key(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("packaging")
        .join("linux")
        .join("flatpak")
        .join("repo-key.asc")
}

/// The repository in the pages tree.
#[must_use]
pub fn repo(pages: &Path) -> PathBuf {
    pages.join(REPO_DIR)
}

/// The gpg home in the work tree.
#[must_use]
pub fn gnupg(work: &Path) -> PathBuf {
    work.join("gnupg")
}

/// The directory the secret key is written into, in the work tree.
#[must_use]
pub fn secret_dir(work: &Path) -> PathBuf {
    work.join("secret")
}

/// The imported build's repository, in the work tree.
#[must_use]
pub fn src(work: &Path) -> PathBuf {
    work.join("src")
}

/// Why `nova-flatpak-repo` stopped.
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    /// A program, a file step, the spec or the build ID failed.
    #[error(transparent)]
    Step(#[from] BundleError),
    /// The committed key file can't be published.
    #[error("{}: {source}", path.display())]
    Key {
        /// The key file.
        path: PathBuf,
        /// What it holds instead.
        source: KeyError,
    },
    /// The secret isn't a secret key gpg can use. Its value is never
    /// shown.
    #[error("{SECRET_VAR}: {0}")]
    Secret(KeyError),
    /// The secret's key isn't the committed one, so every install would
    /// refuse what it signs.
    #[error(
        "{SECRET_VAR}'s key has fingerprint {secret}, but {PUBLIC_KEY}'s is {committed}: \
         every install would refuse the repo it signs"
    )]
    Mismatch {
        /// The committed key's fingerprint.
        committed: String,
        /// The secret key's.
        secret: String,
    },
    /// The live repo couldn't be probed.
    #[error(transparent)]
    Probe(#[from] ProbeError),
    /// The tree doesn't fit Pages.
    #[error(transparent)]
    Size(#[from] SizeError),
    /// `stage`'s bundle directory doesn't fit the channel
    /// ([`bundle_dir_refusal`]).
    #[error("{0}")]
    BundleDir(&'static str),
    /// A promotion found no live repo to copy its commit from.
    #[error(
        "no live Flatpak repo at {url} to promote from: {} publishes onto it first",
        from.name()
    )]
    NoLiveRepo {
        /// The repo's URL.
        url: String,
        /// The channel promoted from.
        from: Channel,
    },
    /// The branch a promotion copies from holds no commit of the build.
    #[error(transparent)]
    NotOnBranch(#[from] NotOnBranch),
}

/// `stage` imports the plain bundle for a channel built from `main` (the
/// nightly, or none), so it needs `--bundle-dir`.
pub const BUNDLE_DIR_NEEDED: &str = "stage needs --bundle-dir <DIR> for a channel built from \
     main (the nightly): it imports the plain bundle the flatpak job built";

/// `stage` imports nothing for a promoted channel, so it refuses
/// `--bundle-dir`.
pub const BUNDLE_DIR_REFUSED: &str = "stage takes no --bundle-dir for a promoted channel \
     (beta): sign copies the nightly's commit, and nothing is imported";

/// Why `stage`'s bundle directory doesn't fit `channel`, if it doesn't:
/// a promoted channel ([`Channel::promoted_from`]) takes none, and every
/// other (the nightly, or no channel) needs one. `given` is whether one
/// was.
#[must_use]
pub fn bundle_dir_refusal(channel: Option<Channel>, given: bool) -> Option<&'static str> {
    let promoted = channel.and_then(Channel::promoted_from).is_some();
    match (promoted, given) {
        (true, true) => Some(BUNDLE_DIR_REFUSED),
        (false, false) => Some(BUNDLE_DIR_NEEDED),
        _ => None,
    }
}

/// What the edge supplies to both steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoHost {
    /// The `cargo` to read the spec with (`$CARGO` under `cargo run`).
    pub cargo: OsString,
    /// The channel published (`NOVA_CHANNEL`); `None` publishes nothing.
    pub channel: Option<Channel>,
    /// The repo's URL: [`url`] from the binary, a local server's in the
    /// repo test.
    pub url: String,
    /// The public key file, or `None` for the committed one
    /// ([`public_key`] under the workspace root).
    pub key: Option<PathBuf>,
}

impl RepoHost {
    /// The key file, for a workspace rooted at `workspace_root`.
    #[must_use]
    pub fn key_file(&self, workspace_root: &Path) -> PathBuf {
        self.key
            .clone()
            .unwrap_or_else(|| public_key(workspace_root))
    }
}

/// `stage`'s directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagePaths {
    /// Where the `flatpak` job's plain bundle is: `None` for a promoted
    /// channel, which imports nothing ([`bundle_dir_refusal`]).
    pub bundle_dir: Option<PathBuf>,
    /// The scratch tree.
    pub work: PathBuf,
    /// The tree served at `/flatpak/`.
    pub pages: PathBuf,
}

/// `sign`'s directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignPaths {
    /// The scratch tree `stage` filled.
    pub work: PathBuf,
    /// The tree `stage` filled, served at `/flatpak/`.
    pub pages: PathBuf,
    /// Where the repo-following bundle goes.
    pub dist: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::tests::sample_metadata;

    #[test]
    fn the_repo_is_under_the_pages_site_at_flatpak_repo() {
        assert_eq!(url(), format!("{PAGES}/flatpak/repo/"));
        assert_eq!(url(), "https://edpaget.github.io/ev-nova/flatpak/repo/");
    }

    #[test]
    fn the_nightly_flatpakref_is_named_for_its_channel_beside_the_repo() {
        assert_eq!(
            flatpakref_name(Channel::Nightly),
            "ev-nova-nightly.flatpakref"
        );
        assert_eq!(
            flatpakref_url(Channel::Nightly),
            "https://edpaget.github.io/ev-nova/flatpak/ev-nova-nightly.flatpakref"
        );
    }

    #[test]
    fn the_app_ref_is_on_the_channel_s_branch() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        assert_eq!(
            app_ref(&spec, Channel::Nightly),
            "app/io.example.nova/x86_64/nightly"
        );
        assert_eq!(
            app_ref(&spec, Channel::Beta),
            "app/io.example.nova/x86_64/beta"
        );
    }

    #[test]
    fn the_beta_flatpakref_is_named_for_its_channel_beside_the_nightly_s() {
        assert_eq!(flatpakref_name(Channel::Beta), "ev-nova-beta.flatpakref");
        assert_eq!(
            flatpakref_url(Channel::Beta),
            "https://edpaget.github.io/ev-nova/flatpak/ev-nova-beta.flatpakref"
        );
    }

    #[test]
    fn only_a_promoted_channel_stages_without_a_bundle_dir() {
        assert_eq!(bundle_dir_refusal(Some(Channel::Beta), false), None);
        assert_eq!(
            bundle_dir_refusal(Some(Channel::Beta), true),
            Some(BUNDLE_DIR_REFUSED)
        );
        for channel in [None, Some(Channel::Nightly)] {
            assert_eq!(bundle_dir_refusal(channel, true), None, "{channel:?}");
            assert_eq!(
                bundle_dir_refusal(channel, false),
                Some(BUNDLE_DIR_NEEDED),
                "{channel:?}"
            );
        }
    }

    #[test]
    fn each_branch_keeps_as_many_builds_as_the_sparkle_side() {
        assert_eq!(KEPT_COMMITS, KEEP_BUILDS);
        assert_eq!(MIRROR_DEPTH + 1, KEPT_COMMITS);
        assert_eq!(PRUNE_DEPTH + 1, KEPT_COMMITS);
    }

    #[test]
    fn the_committed_key_is_under_packaging_and_a_host_may_name_another() {
        let root = Path::new("/work").join("ev-nova");
        let committed = root
            .join("packaging")
            .join("linux")
            .join("flatpak")
            .join("repo-key.asc");
        assert_eq!(public_key(&root), committed);
        assert!(committed.ends_with(PUBLIC_KEY));
        let host = RepoHost {
            cargo: "cargo".into(),
            channel: None,
            url: url(),
            key: None,
        };
        assert_eq!(host.key_file(&root), committed);
        let other = root.join("k.asc");
        assert_eq!(
            RepoHost {
                key: Some(other.clone()),
                ..host
            }
            .key_file(&root),
            other
        );
    }
}
