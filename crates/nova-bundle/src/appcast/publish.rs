//! `nova-appcast publish`'s decision: whether to sign the feeds
//! ([`FeedSigning::decide`]), then, when signing, the build-ID copies of
//! the installers and the two feeds; and always the list of release
//! assets to delete ([`super::prune`]).
//!
//! It reads only local files, through the [`ReadFiles`] port: CI lists
//! the release's assets and downloads its current feeds first, so nothing
//! here reaches the network. It writes through the [`Files`] port, and
//! only once every check has passed, so a refusal leaves no file behind.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};

use super::key::{KeyError, PublicKey};
use super::prune::{KEEP_BUILDS, LiveFeed, prune};
use super::{Channel, CurrentFeed, Feed, FeedError, Platform, feed_name, read_feed, render};
use crate::build_id::{self, BuildId, ResolveError};
use crate::bundle::metadata;
use crate::channel::{self, ChannelError};
use crate::ports::{Env, Files, Process, ReadFiles, RunError, Signer};
use crate::spec::{BundleSpec, SpecError};

/// The variable holding the update-signing seed: 32 bytes as standard
/// base64. CI sets it from the `update-signing` environment's secret, and
/// GitHub gives a job without the secret an empty value, which counts as
/// unset.
pub const SEED_VAR: &str = "UPDATE_SIGNING_KEY";

/// Why `publish` writes no feeds this run. The nightly still publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Neither half of the key is in place.
    Neither,
    /// The seed is set, but no public key is committed.
    NoPublicKey,
    /// The public key is committed, but the seed is unavailable.
    NoSeed,
}

impl SkipReason {
    /// The warning's text.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::Neither => {
                "no update_public_key is committed in crates/nova/Cargo.toml's \
                 [package.metadata.bundle], and UPDATE_SIGNING_KEY is unset (the \
                 update-signing environment's secret is unavailable): see \
                 infra/github/README.md, \"The update-signing environment\""
            }
            Self::NoPublicKey => {
                "UPDATE_SIGNING_KEY is set, but no update_public_key is committed in \
                 crates/nova/Cargo.toml's [package.metadata.bundle]: commit the key \
                 `mise run appcast -- public-key` prints"
            }
            Self::NoSeed => {
                "update_public_key is committed, but UPDATE_SIGNING_KEY is unset: the \
                 update-signing environment's secret is unavailable to this job"
            }
        }
    }
}

/// Whether this run signs the feeds, and with what.
#[derive(Debug)]
pub enum FeedSigning<S> {
    /// Signs with this signer, whose public key is the committed one.
    Sign(S),
    /// Writes no feeds, for this reason.
    Skip(SkipReason),
}

/// Why `nova-appcast` stopped.
#[derive(Debug, thiserror::Error)]
pub enum AppcastError {
    /// There is no build ID.
    #[error(transparent)]
    BuildId(#[from] ResolveError),
    /// `NOVA_CHANNEL` is unset or empty: a publish names its channel, and
    /// never defaults to the nightly.
    #[error(
        "{} is unset or empty: set it to the channel to publish (the nightly's \
         publish sets nightly, promote-beta's sets beta)",
        channel::VAR
    )]
    NoChannel,
    /// `NOVA_CHANNEL` names no channel.
    #[error(transparent)]
    Channel(#[from] ChannelError),
    /// A program failed.
    #[error("`{command}` failed: {source}")]
    Run {
        /// The command line.
        command: String,
        /// How it failed.
        source: RunError,
    },
    /// `cargo metadata` gave no spec.
    #[error(transparent)]
    Spec(#[from] SpecError),
    /// The seed variable is unset or empty.
    #[error("{SEED_VAR} is unset or empty")]
    NoSeed,
    /// The seed variable is set to something that is no seed. The value is
    /// never shown.
    #[error("{SEED_VAR}: {0}")]
    Seed(KeyError),
    /// The seed's public key isn't the committed one, so every install
    /// would reject the feeds it signs.
    #[error(
        "{SEED_VAR}'s public key is {derived}, but [package.metadata.bundle]'s \
         update_public_key is {committed}: the feeds would fail every install's check"
    )]
    KeyMismatch {
        /// The committed key.
        committed: PublicKey,
        /// The seed's key.
        derived: PublicKey,
    },
    /// The build isn't newer than one a current feed offers.
    #[error(
        "build {new} is not newer than build {current}, which {feed} already offers: \
         refusing to publish a feed that would roll installs back"
    )]
    NotNewer {
        /// The feed's release asset.
        feed: String,
        /// The build it offers.
        current: BuildId,
        /// This run's build.
        new: BuildId,
    },
    /// The release lists a feed that wasn't downloaded.
    #[error(
        "the release lists {feed}, but it isn't in {}: download every listed feed first",
        dir.display()
    )]
    FeedNotDownloaded {
        /// The feed's release asset.
        feed: String,
        /// The directory it should be in.
        dir: PathBuf,
    },
    /// A current feed can't be read.
    #[error(
        "the published {feed} can't be read: {source}. Delete it from the release to \
         publish a fresh one"
    )]
    BadFeed {
        /// The feed's release asset.
        feed: String,
        /// Why it can't be read.
        source: FeedError,
    },
    /// A file couldn't be read.
    #[error("could not read {}: {source}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// A file step failed.
    #[error("could not {action} {}: {source}", path.display())]
    File {
        /// What the step did.
        action: &'static str,
        /// The path it did it to.
        path: PathBuf,
        /// Why it failed.
        source: io::Error,
    },
}

impl AppcastError {
    /// Whether this is a refusal of the very build the feeds already
    /// offer: a re-run of a publish whose feeds went up, not a rollback.
    #[must_use]
    pub fn is_already_published(&self) -> bool {
        matches!(self, Self::NotNewer { current, new, .. } if current == new)
    }
}

impl<S: Signer> FeedSigning<S> {
    /// Signs when the public key is committed (`committed`) and the seed
    /// variable is set (`seed`, from [`SEED_VAR`]; empty counts as unset),
    /// making the signer with `make`; skips when either is missing.
    ///
    /// # Errors
    ///
    /// With both in place, a seed that isn't one, or one whose public key
    /// isn't `committed`.
    pub fn decide(
        committed: Option<&PublicKey>,
        seed: Option<&OsStr>,
        make: impl FnOnce(&str) -> Result<S, KeyError>,
    ) -> Result<Self, AppcastError> {
        let seed = seed.filter(|s| !s.is_empty());
        let (committed, seed) = match (committed, seed) {
            (None, None) => return Ok(Self::Skip(SkipReason::Neither)),
            (None, Some(_)) => return Ok(Self::Skip(SkipReason::NoPublicKey)),
            (Some(_), None) => return Ok(Self::Skip(SkipReason::NoSeed)),
            (Some(committed), Some(seed)) => (committed, seed),
        };
        let signer = signer_of(seed, make)?;
        let derived = signer.public_key();
        if derived != *committed {
            return Err(AppcastError::KeyMismatch {
                committed: *committed,
                derived,
            });
        }
        Ok(Self::Sign(signer))
    }
}

/// The signer of the seed `seed`, made with `make`. A seed that isn't
/// UTF-8 is no base64.
fn signer_of<S>(
    seed: &OsStr,
    make: impl FnOnce(&str) -> Result<S, KeyError>,
) -> Result<S, AppcastError> {
    let text = seed
        .to_str()
        .ok_or(AppcastError::Seed(KeyError::NotBase64))?;
    make(text).map_err(AppcastError::Seed)
}

/// Where `publish` reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPaths {
    /// Every asset this run uploads; the stable installers are read here,
    /// and their build-ID copies written beside them.
    pub dist: PathBuf,
    /// The release's assets before this run, one name per line.
    pub assets: PathBuf,
    /// The release's current feeds, downloaded.
    pub current: PathBuf,
    /// Where the new feeds are written.
    pub feeds: PathBuf,
    /// Where the assets to delete are written, one name per line.
    pub prune_list: PathBuf,
}

/// What a publish did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    /// The feeds written, in [`Platform::ALL`]'s order; none on a skip.
    pub feeds: Vec<PathBuf>,
    /// Why no feeds were written, on a skip.
    pub skipped: Option<SkipReason>,
    /// The assets to delete, as written to the prune list.
    pub prune: Vec<String>,
}

/// Publishes `build` of `spec` on `channel` as `signing` says, over
/// `paths`: `channel`'s feeds are the ones compared against and written.
///
/// # Errors
///
/// A listed feed that wasn't downloaded or can't be read, a build that
/// isn't newer than a current feed's (when signing), an installer
/// missing from `dist`, or a failed read or write.
pub fn publish<S: Signer>(
    read: &impl ReadFiles,
    files: &mut impl Files,
    spec: &BundleSpec,
    build: &BuildId,
    channel: Channel,
    signing: &FeedSigning<S>,
    paths: &PublishPaths,
) -> Result<Published, AppcastError> {
    let assets = read_asset_list(read, &paths.assets)?;
    let current = current_feeds(read, &assets, &paths.current, channel)?;
    let mut live: Vec<LiveFeed> = current
        .iter()
        .map(|(name, feed)| LiveFeed {
            name: name.clone(),
            file: feed.file.clone(),
        })
        .collect();
    let mut uploads = BTreeSet::new();
    let (feeds, skipped) = match signing {
        FeedSigning::Skip(reason) => (Vec::new(), Some(*reason)),
        FeedSigning::Sign(signer) => {
            refuse_unless_newer(&current, build)?;
            let mut installers = Vec::new();
            for platform in Platform::ALL {
                let path = paths.dist.join(platform.stable_asset());
                installers.push((platform, path.clone(), read_at(read, &path)?));
            }
            let mut written = Vec::new();
            let mut feeds = Vec::new();
            for (platform, path, bytes) in &installers {
                let feed = Feed::signed(
                    *platform,
                    channel,
                    &spec.name,
                    build,
                    &spec.version,
                    bytes,
                    signer,
                );
                let copy = paths.dist.join(&feed.asset);
                files
                    .copy(path, &copy)
                    .map_err(|source| file_error("copy", &copy, source))?;
                uploads.insert(feed.asset.clone());
                feeds.push(feed);
            }
            files
                .create_dir_all(&paths.feeds)
                .map_err(|source| file_error("create", &paths.feeds, source))?;
            for feed in &feeds {
                let name = feed_name(feed.platform, channel);
                let path = paths.feeds.join(&name);
                files
                    .write(&path, render(feed).as_bytes())
                    .map_err(|source| file_error("write", &path, source))?;
                live.push(LiveFeed {
                    name: name.clone(),
                    file: feed.asset.clone(),
                });
                uploads.insert(name);
                written.push(path);
            }
            (written, None)
        }
    };
    let dist = read
        .file_names(&paths.dist)
        .map_err(|source| AppcastError::Read {
            path: paths.dist.clone(),
            source,
        })?;
    uploads.extend(dist);
    let uploads: Vec<String> = uploads.into_iter().collect();
    let prune = prune(&assets, &uploads, &live, KEEP_BUILDS);
    files
        .write(&paths.prune_list, one_per_line(&prune).as_bytes())
        .map_err(|source| file_error("write", &paths.prune_list, source))?;
    Ok(Published {
        feeds,
        skipped,
        prune,
    })
}

/// Each name followed by a newline, as CI's `while read` loop reads them.
pub(crate) fn one_per_line(names: &[impl AsRef<str>]) -> String {
    names.iter().fold(String::new(), |mut text, name| {
        text.push_str(name.as_ref());
        text.push('\n');
        text
    })
}

fn file_error(action: &'static str, path: &Path, source: io::Error) -> AppcastError {
    AppcastError::File {
        action,
        path: path.to_owned(),
        source,
    }
}

pub(crate) fn read_at(read: &impl ReadFiles, path: &Path) -> Result<Vec<u8>, AppcastError> {
    read.read(path).map_err(|source| AppcastError::Read {
        path: path.to_owned(),
        source,
    })
}

/// A release's asset list at `path`: one name per line, blank lines and
/// surrounding spaces ignored.
pub(crate) fn read_asset_list(
    read: &impl ReadFiles,
    path: &Path,
) -> Result<Vec<String>, AppcastError> {
    let listed = read_at(read, path)?;
    Ok(String::from_utf8_lossy(&listed)
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Each of `channel`'s feeds the release lists, by its asset name, read
/// from its download in `dir`.
pub(crate) fn current_feeds(
    read: &impl ReadFiles,
    assets: &[String],
    dir: &Path,
    channel: Channel,
) -> Result<Vec<(String, CurrentFeed)>, AppcastError> {
    let mut feeds = Vec::new();
    for platform in Platform::ALL {
        let name = feed_name(platform, channel);
        if !assets.contains(&name) {
            continue;
        }
        let path = dir.join(&name);
        let bytes = read.read(&path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                AppcastError::FeedNotDownloaded {
                    feed: name.clone(),
                    dir: dir.to_owned(),
                }
            } else {
                AppcastError::Read { path, source }
            }
        })?;
        let feed = std::str::from_utf8(&bytes)
            .map_err(|e| FeedError::Xml(e.to_string()))
            .and_then(|text| read_feed(platform, text))
            .map_err(|source| AppcastError::BadFeed {
                feed: name.clone(),
                source,
            })?;
        feeds.push((name, feed));
    }
    Ok(feeds)
}

/// Refuses `build` unless it is newer than every current feed's. An older
/// feed's build is refused first, so a rollback is never taken for a
/// re-run.
fn refuse_unless_newer(
    current: &[(String, CurrentFeed)],
    build: &BuildId,
) -> Result<(), AppcastError> {
    let refusal = |(feed, offered): &(String, CurrentFeed)| AppcastError::NotNewer {
        feed: feed.clone(),
        current: offered.build.clone(),
        new: build.clone(),
    };
    if let Some(newer) = current.iter().find(|(_, f)| f.build > *build) {
        return Err(refusal(newer));
    }
    if let Some(same) = current.iter().find(|(_, f)| f.build == *build) {
        return Err(refusal(same));
    }
    Ok(())
}

/// The whole of `nova-appcast publish`: the channel (from
/// [`channel::VAR`], which must name one), the build ID (from
/// [`build_id::VAR`], else HEAD in `cwd`), the spec from `cargo metadata`,
/// the signing decision from [`SEED_VAR`], then [`publish`].
///
/// # Errors
///
/// As [`publish`], or no channel, no build ID, no spec, or a bad seed.
#[allow(clippy::too_many_arguments)]
pub fn run<S: Signer>(
    process: &mut impl Process,
    env: &impl Env,
    read: &impl ReadFiles,
    files: &mut impl Files,
    cargo: &OsString,
    cwd: &Path,
    paths: &PublishPaths,
    make: impl FnOnce(&str) -> Result<S, KeyError>,
) -> Result<Published, AppcastError> {
    let channel = channel::from_env(env)?.ok_or(AppcastError::NoChannel)?;
    let build = build_id::resolve(env, process, cwd)?;
    let invocation = metadata(cargo);
    let json = process
        .run(&invocation)
        .map_err(|source| AppcastError::Run {
            command: invocation.to_string(),
            source,
        })?;
    let spec = BundleSpec::from_metadata(&json)?;
    let seed = env.var(SEED_VAR);
    let signing = FeedSigning::decide(spec.update_public_key.as_ref(), seed.as_deref(), make)?;
    publish(read, files, &spec, &build, channel, &signing, paths)
}

/// `nova-appcast public-key`: the public key of the seed in
/// [`SEED_VAR`], made with `make`.
///
/// # Errors
///
/// An unset, empty or malformed seed.
pub fn public_key<S: Signer>(
    env: &impl Env,
    make: impl FnOnce(&str) -> Result<S, KeyError>,
) -> Result<PublicKey, AppcastError> {
    let seed = env
        .var(SEED_VAR)
        .filter(|s| !s.is_empty())
        .ok_or(AppcastError::NoSeed)?;
    signer_of(&seed, make).map(|signer| signer.public_key())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use serde_json::json;

    use super::*;
    use crate::appcast::tests::{seed_base64, test_signer, verifies};
    use crate::appcast::{SPARKLE_NS, update_asset};
    use crate::archive;
    use crate::bundle::metadata as metadata_run;
    use crate::channel;
    use crate::spec::tests::{full_bundle, metadata_with, sample_metadata};
    use crate::system::Ed25519Signer;
    use crate::test_support::{Event, FakeEnv, FakeFiles, FakeRead, Log, Recorder};

    /// The channel most tests publish.
    const CHANNEL: Channel = Channel::Nightly;

    /// An environment pinning the build ID and naming the nightly channel.
    fn build_env() -> FakeEnv {
        let mut env = crate::test_support::build_env();
        env.0.insert(channel::VAR, "nightly".into());
        env
    }

    /// A public key no test seed derives.
    fn other_key() -> PublicKey {
        PublicKey([9; 32])
    }

    /// Records every message signed; signs each with 64 copies of its
    /// length's low byte.
    #[derive(Debug)]
    struct RecordingSigner {
        key: PublicKey,
        signed: Rc<RefCell<Vec<Vec<u8>>>>,
    }

    impl Signer for RecordingSigner {
        fn sign(&self, message: &[u8]) -> [u8; 64] {
            self.signed.borrow_mut().push(message.to_vec());
            [message.len().to_le_bytes()[0]; 64]
        }
        fn public_key(&self) -> PublicKey {
            self.key
        }
    }

    fn recording() -> (RecordingSigner, Rc<RefCell<Vec<Vec<u8>>>>) {
        let signed = Rc::default();
        (
            RecordingSigner {
                key: other_key(),
                signed: Rc::clone(&signed),
            },
            signed,
        )
    }

    fn make_ok(text: &str) -> Result<Ed25519Signer, KeyError> {
        Ed25519Signer::from_base64_seed(text)
    }

    fn decide(
        committed: Option<PublicKey>,
        seed: Option<&str>,
    ) -> Result<FeedSigning<Ed25519Signer>, AppcastError> {
        FeedSigning::decide(committed.as_ref(), seed.map(OsStr::new), make_ok)
    }

    fn skipped<S>(signing: &Result<FeedSigning<S>, AppcastError>) -> Option<SkipReason> {
        match signing {
            Ok(FeedSigning::Skip(reason)) => Some(*reason),
            _ => None,
        }
    }

    #[test]
    fn no_key_and_no_seed_skips() {
        assert_eq!(skipped(&decide(None, None)), Some(SkipReason::Neither));
    }

    #[test]
    fn a_seed_without_a_committed_key_skips() {
        assert_eq!(
            skipped(&decide(None, Some(&seed_base64()))),
            Some(SkipReason::NoPublicKey)
        );
        // The seed isn't read without a key to check it against.
        assert_eq!(
            skipped(&decide(None, Some("not a seed"))),
            Some(SkipReason::NoPublicKey)
        );
    }

    #[test]
    fn a_committed_key_without_a_seed_skips() {
        let key = test_signer().public_key();
        assert_eq!(skipped(&decide(Some(key), None)), Some(SkipReason::NoSeed));
    }

    #[test]
    fn an_empty_variable_counts_as_unset() {
        let key = test_signer().public_key();
        assert_eq!(
            skipped(&decide(Some(key), Some(""))),
            Some(SkipReason::NoSeed)
        );
        assert_eq!(skipped(&decide(None, Some(""))), Some(SkipReason::Neither));
    }

    #[test]
    fn a_malformed_seed_is_an_error_that_never_shows_it() {
        let key = test_signer().public_key();
        for seed in ["c2VjcmV0LXNlZWQ=", "secret seed!"] {
            let error = decide(Some(key), Some(seed)).expect_err("malformed");
            assert!(matches!(error, AppcastError::Seed(_)), "{error:?}");
            let text = error.to_string();
            assert!(text.starts_with("UPDATE_SIGNING_KEY: "), "{text}");
            assert!(!text.contains(seed), "{text}");
            assert!(!format!("{error:?}").contains(seed), "{error:?}");
        }
    }

    #[test]
    fn a_mismatched_key_names_both_public_keys_and_never_the_seed() {
        let error = decide(Some(other_key()), Some(&seed_base64())).expect_err("mismatch");
        let derived = test_signer().public_key();
        assert!(
            matches!(&error, AppcastError::KeyMismatch { committed, derived: d }
                if *committed == other_key() && *d == derived),
            "{error:?}"
        );
        let text = error.to_string();
        assert!(text.contains(&other_key().to_base64()), "{text}");
        assert!(text.contains(&derived.to_base64()), "{text}");
        assert!(!text.contains(&seed_base64()), "{text}");
    }

    #[test]
    fn a_matching_key_and_seed_signs() {
        let key = test_signer().public_key();
        let signing = decide(Some(key), Some(&format!("{}\n", seed_base64()))).expect("signs");
        let FeedSigning::Sign(signer) = signing else {
            panic!("{signing:?}");
        };
        assert_eq!(signer.public_key(), key);
    }

    #[test]
    fn each_skip_reason_says_what_is_missing() {
        assert!(SkipReason::Neither.message().contains("update_public_key"));
        assert!(SkipReason::Neither.message().contains("UPDATE_SIGNING_KEY"));
        assert!(SkipReason::NoPublicKey.message().contains("commit"));
        assert!(
            SkipReason::NoSeed
                .message()
                .contains("update-signing environment")
        );
    }

    // publish

    fn root() -> PathBuf {
        Path::new("/work").join("run")
    }

    fn paths() -> PublishPaths {
        PublishPaths {
            dist: root().join("dist"),
            assets: root().join("assets.txt"),
            current: root().join("current"),
            feeds: root().join("feeds"),
            prune_list: root().join("prune.txt"),
        }
    }

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn id(text: &str) -> BuildId {
        BuildId::parse(text).expect("an ID")
    }

    const DMG: &[u8] = b"the dmg";
    const MSI: &[u8] = b"the msi, longer";

    fn dist_files(read: &mut FakeRead) {
        let dist = paths().dist;
        read.0
            .insert(dist.join(Platform::Macos.stable_asset()), DMG.to_vec());
        read.0
            .insert(dist.join(Platform::Windows.stable_asset()), MSI.to_vec());
        read.0
            .insert(dist.join(archive::WINDOWS.file), b"zip".to_vec());
    }

    /// The release's asset list holding `names`.
    fn listed(read: &mut FakeRead, names: &[String]) {
        read.0
            .insert(paths().assets, one_per_line(names).into_bytes());
    }

    /// A current feed for `platform` offering `build`, downloaded.
    fn current_feed(read: &mut FakeRead, platform: Platform, build: &str) {
        current_feed_of(read, CHANNEL, platform, build);
    }

    /// A current feed of `channel`'s for `platform` offering `build`,
    /// downloaded.
    fn current_feed_of(read: &mut FakeRead, channel: Channel, platform: Platform, build: &str) {
        let feed = Feed::signed(
            platform,
            channel,
            "EV Nova",
            &id(build),
            "0.1.0",
            b"old",
            &test_signer(),
        );
        read.0.insert(
            paths().current.join(feed_name(platform, channel)),
            render(&feed).into_bytes(),
        );
    }

    fn feeds_listed() -> Vec<String> {
        feeds_listed_of(CHANNEL)
    }

    fn feeds_listed_of(channel: Channel) -> Vec<String> {
        Platform::ALL
            .into_iter()
            .map(|p| feed_name(p, channel))
            .collect()
    }

    fn stable() -> Vec<String> {
        vec![
            Platform::Macos.stable_asset().to_owned(),
            Platform::Windows.stable_asset().to_owned(),
            archive::WINDOWS.file.to_owned(),
        ]
    }

    struct Outcome {
        result: Result<Published, AppcastError>,
        events: Vec<Event>,
    }

    fn files(log: &Log) -> FakeFiles {
        FakeFiles {
            log: Rc::clone(log),
            fails: Box::new(|_| false),
            absent: Box::new(|_| false),
        }
    }

    fn publish_with<S: Signer>(read: &FakeRead, signing: &FeedSigning<S>, build: &str) -> Outcome {
        publish_on(read, CHANNEL, signing, build)
    }

    fn publish_on<S: Signer>(
        read: &FakeRead,
        channel: Channel,
        signing: &FeedSigning<S>,
        build: &str,
    ) -> Outcome {
        let log = Log::default();
        let result = publish(
            read,
            &mut files(&log),
            &spec(),
            &id(build),
            channel,
            signing,
            &paths(),
        );
        let events = log.borrow().clone();
        Outcome { result, events }
    }

    fn feed_bytes(platform: Platform, build: &str, bytes: &[u8], signer: &impl Signer) -> Vec<u8> {
        feed_bytes_of(CHANNEL, platform, build, bytes, signer)
    }

    fn feed_bytes_of(
        channel: Channel,
        platform: Platform,
        build: &str,
        bytes: &[u8],
        signer: &impl Signer,
    ) -> Vec<u8> {
        render(&Feed::signed(
            platform,
            channel,
            "EV Nova",
            &id(build),
            &spec().version,
            bytes,
            signer,
        ))
        .into_bytes()
    }

    fn prune_text(names: &[&str]) -> Vec<u8> {
        one_per_line(names).into_bytes()
    }

    const NEW: &str = "20261028123456";

    #[test]
    fn a_first_publish_copies_both_installers_writes_both_feeds_and_prunes() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let mut assets = stable();
        assets.push("ev-nova-x86_64-unknown-linux-gnu.AppImage".to_owned());
        listed(&mut read, &assets);
        let (signer, messages) = recording();
        let signing = FeedSigning::Sign(signer);
        let out = publish_with(&read, &signing, NEW);
        let published = out.result.expect("published");
        let p = paths();
        let mac_feed = p.feeds.join("appcast-macos-nightly.xml");
        let win_feed = p.feeds.join("appcast-windows-nightly.xml");
        let FeedSigning::Sign(signer) = &signing else {
            unreachable!()
        };
        let expected_feeds = [
            feed_bytes(Platform::Macos, NEW, DMG, signer),
            feed_bytes(Platform::Windows, NEW, MSI, signer),
        ];
        assert_eq!(
            out.events,
            [
                Event::Copy(
                    p.dist.join(Platform::Macos.stable_asset()),
                    p.dist.join(update_asset(Platform::Macos, &id(NEW))),
                ),
                Event::Copy(
                    p.dist.join(Platform::Windows.stable_asset()),
                    p.dist.join(update_asset(Platform::Windows, &id(NEW))),
                ),
                Event::CreateDir(p.feeds.clone()),
                Event::Write(mac_feed.clone(), expected_feeds[0].clone()),
                Event::Write(win_feed.clone(), expected_feeds[1].clone()),
                Event::Write(
                    p.prune_list.clone(),
                    prune_text(&["ev-nova-x86_64-unknown-linux-gnu.AppImage"]),
                ),
            ]
        );
        // The publish signed each installer's bytes, in order; the test's
        // own feed_bytes calls signed them again after.
        assert_eq!(messages.borrow()[..2], [DMG.to_vec(), MSI.to_vec()]);
        assert_eq!(
            published,
            Published {
                feeds: vec![mac_feed, win_feed],
                skipped: None,
                prune: vec!["ev-nova-x86_64-unknown-linux-gnu.AppImage".to_owned()],
            }
        );
    }

    #[test]
    fn the_feeds_carry_the_spec_s_name_and_version_and_a_real_signature() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        listed(&mut read, &stable());
        let out = publish_with(&read, &FeedSigning::Sign(test_signer()), NEW);
        out.result.expect("published");
        let feeds: HashMap<_, _> = out
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Write(path, bytes) if path.starts_with(paths().feeds) => Some((
                    path.clone(),
                    String::from_utf8(bytes.clone()).expect("utf-8"),
                )),
                _ => None,
            })
            .collect();
        for (platform, bytes) in [(Platform::Macos, DMG), (Platform::Windows, MSI)] {
            let text = &feeds[&paths().feeds.join(feed_name(platform, CHANNEL))];
            let doc = roxmltree::Document::parse(text).expect("XML");
            let enclosure = doc
                .descendants()
                .find(|n| n.has_tag_name("enclosure"))
                .expect("enclosure");
            let signature = enclosure
                .attribute((SPARKLE_NS, "edSignature"))
                .expect("signature");
            assert!(verifies(signature, bytes), "{platform:?}");
            assert!(
                text.contains("<sparkle:shortVersionString>0.3.1<"),
                "{text}"
            );
            assert!(
                text.contains("<title>EV Nova 0.3.1 (20261028123456)</title>"),
                "{text}"
            );
        }
    }

    #[test]
    fn a_newer_build_than_the_current_feeds_publishes() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let old = "20261027000000";
        let mut assets = stable();
        assets.extend(feeds_listed());
        assets.push(update_asset(Platform::Macos, &id(old)));
        assets.push(update_asset(Platform::Windows, &id(old)));
        listed(&mut read, &assets);
        current_feed(&mut read, Platform::Macos, old);
        current_feed(&mut read, Platform::Windows, old);
        let (signer, _) = recording();
        let out = publish_with(&read, &FeedSigning::Sign(signer), NEW);
        let published = out.result.expect("published");
        assert_eq!(published.feeds.len(), 2);
        // The old update files are among the newest three, and the old
        // feeds named them: nothing is deleted.
        assert_eq!(published.prune, Vec::<String>::new());
        assert!(
            out.events
                .contains(&Event::Write(paths().prune_list, Vec::new()))
        );
    }

    #[test]
    fn a_beta_publish_writes_the_beta_feeds_with_beta_urls() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        listed(&mut read, &stable());
        let out = publish_on(&read, Channel::Beta, &FeedSigning::Sign(test_signer()), NEW);
        let published = out.result.expect("published");
        let p = paths();
        let mac_feed = p.feeds.join("appcast-macos-beta.xml");
        let win_feed = p.feeds.join("appcast-windows-beta.xml");
        assert_eq!(published.feeds, vec![mac_feed.clone(), win_feed.clone()]);
        for (path, platform, bytes) in [
            (&mac_feed, Platform::Macos, DMG),
            (&win_feed, Platform::Windows, MSI),
        ] {
            let expected = feed_bytes_of(Channel::Beta, platform, NEW, bytes, &test_signer());
            assert!(
                out.events
                    .contains(&Event::Write(path.clone(), expected.clone())),
                "{platform:?}: {:?}",
                out.events
            );
            let text = String::from_utf8(expected).expect("utf-8");
            assert!(
                text.contains("https://github.com/edpaget/ev-nova/releases/download/beta/"),
                "{text}"
            );
        }
        assert!(out.events.contains(&Event::Copy(
            p.dist.join(Platform::Macos.stable_asset()),
            p.dist.join(update_asset(Platform::Macos, &id(NEW))),
        )));
    }

    #[test]
    fn a_publish_compares_only_against_its_own_channel_s_feeds() {
        let newer = "20261029000000";
        for (channel, other) in [
            (Channel::Beta, Channel::Nightly),
            (Channel::Nightly, Channel::Beta),
        ] {
            // The other channel's feeds offer a newer build, and are ignored.
            let mut read = FakeRead::default();
            dist_files(&mut read);
            let mut assets = stable();
            assets.extend(feeds_listed_of(other));
            listed(&mut read, &assets);
            for platform in Platform::ALL {
                current_feed_of(&mut read, other, platform, newer);
            }
            let (signer, _) = recording();
            let out = publish_on(&read, channel, &FeedSigning::Sign(signer), NEW);
            assert_eq!(out.result.expect("published").feeds.len(), 2, "{channel:?}");

            // Its own feeds offering the newer build refuse it.
            let mut read = FakeRead::default();
            dist_files(&mut read);
            let mut assets = stable();
            assets.extend(feeds_listed_of(channel));
            listed(&mut read, &assets);
            for platform in Platform::ALL {
                current_feed_of(&mut read, channel, platform, newer);
            }
            let (signer, _) = recording();
            let out = publish_on(&read, channel, &FeedSigning::Sign(signer), NEW);
            let error = out.result.expect_err("not newer");
            assert!(
                matches!(&error, AppcastError::NotNewer { feed, current, new }
                    if *feed == feed_name(Platform::Macos, channel)
                        && *current == id(newer) && *new == id(NEW)),
                "{channel:?}: {error:?}"
            );
            assert_eq!(out.events, [], "{channel:?}");
        }
    }

    #[test]
    fn a_beta_re_run_of_the_same_build_is_already_published() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let mut assets = stable();
        assets.extend(feeds_listed_of(Channel::Beta));
        listed(&mut read, &assets);
        for platform in Platform::ALL {
            current_feed_of(&mut read, Channel::Beta, platform, NEW);
        }
        let (signer, _) = recording();
        let error = publish_on(&read, Channel::Beta, &FeedSigning::Sign(signer), NEW)
            .result
            .expect_err("refused");
        assert!(error.is_already_published(), "{error:?}");
        assert!(
            error.to_string().contains("appcast-macos-beta.xml"),
            "{error}"
        );
    }

    #[test]
    fn a_build_equal_to_or_older_than_a_current_feed_is_refused_naming_both_ids() {
        for (current, already) in [(NEW, true), ("20261029000000", false)] {
            let mut read = FakeRead::default();
            dist_files(&mut read);
            let mut assets = stable();
            assets.extend(feeds_listed());
            listed(&mut read, &assets);
            current_feed(&mut read, Platform::Macos, "20261001000000");
            current_feed(&mut read, Platform::Windows, current);
            let (signer, messages) = recording();
            let out = publish_with(&read, &FeedSigning::Sign(signer), NEW);
            let error = out.result.expect_err("refused");
            assert!(
                matches!(&error, AppcastError::NotNewer { feed, current: c, new }
                    if feed == "appcast-windows-nightly.xml" && *c == id(current) && *new == id(NEW)),
                "{error:?}"
            );
            let text = error.to_string();
            assert!(text.contains(NEW) && text.contains(current), "{text}");
            assert!(text.contains("appcast-windows-nightly.xml"), "{text}");
            assert_eq!(error.is_already_published(), already, "{text}");
            assert_eq!(out.events, [], "{current}");
            assert!(messages.borrow().is_empty());
        }
    }

    #[test]
    fn an_older_build_wins_over_an_equal_one() {
        for (mac, win) in [(NEW, "20261029000000"), ("20261029000000", NEW)] {
            let mut read = FakeRead::default();
            dist_files(&mut read);
            let mut assets = stable();
            assets.extend(feeds_listed());
            listed(&mut read, &assets);
            current_feed(&mut read, Platform::Macos, mac);
            current_feed(&mut read, Platform::Windows, win);
            let (signer, _) = recording();
            let error = publish_with(&read, &FeedSigning::Sign(signer), NEW)
                .result
                .expect_err("refused");
            assert!(
                matches!(&error, AppcastError::NotNewer { current, .. } if *current == id("20261029000000")),
                "{error:?}"
            );
            assert!(!error.is_already_published());
        }
    }

    #[test]
    fn only_a_refusal_of_the_same_build_is_already_published() {
        assert!(!AppcastError::NoSeed.is_already_published());
    }

    #[test]
    fn a_listed_feed_that_wasnt_downloaded_is_an_error() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let mut assets = stable();
        assets.extend(feeds_listed());
        listed(&mut read, &assets);
        current_feed(&mut read, Platform::Macos, "20261001000000");
        let (signer, _) = recording();
        let out = publish_with(&read, &FeedSigning::Sign(signer), NEW);
        let error = out.result.expect_err("missing");
        assert!(
            matches!(&error, AppcastError::FeedNotDownloaded { feed, dir }
                if feed == "appcast-windows-nightly.xml" && *dir == paths().current),
            "{error:?}"
        );
        assert!(
            error
                .to_string()
                .contains(&paths().current.display().to_string()),
            "{error}"
        );
        assert_eq!(out.events, []);
    }

    #[test]
    fn a_malformed_current_feed_is_an_error_naming_it() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let mut assets = stable();
        assets.push(feed_name(Platform::Macos, CHANNEL));
        listed(&mut read, &assets);
        read.0.insert(
            paths().current.join(feed_name(Platform::Macos, CHANNEL)),
            b"<rss/>".to_vec(),
        );
        let (signer, _) = recording();
        let out = publish_with(&read, &FeedSigning::Sign(signer), NEW);
        let error = out.result.expect_err("malformed");
        assert!(
            matches!(&error, AppcastError::BadFeed { feed, source: FeedError::NoVersion }
                if feed == "appcast-macos-nightly.xml"),
            "{error:?}"
        );
        let text = error.to_string();
        assert!(
            text.contains("appcast-macos-nightly.xml") && text.contains("Delete"),
            "{text}"
        );
        assert_eq!(out.events, []);
    }

    #[test]
    fn a_current_feed_that_isnt_utf8_is_malformed() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let mut assets = stable();
        assets.push(feed_name(Platform::Macos, CHANNEL));
        listed(&mut read, &assets);
        read.0.insert(
            paths().current.join(feed_name(Platform::Macos, CHANNEL)),
            b"\xff\xfe".to_vec(),
        );
        let (signer, _) = recording();
        let error = publish_with(&read, &FeedSigning::Sign(signer), NEW)
            .result
            .expect_err("malformed");
        assert!(
            matches!(
                &error,
                AppcastError::BadFeed {
                    source: FeedError::Xml(_),
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn skipping_writes_no_feeds_or_copies_and_keeps_the_current_feeds_and_their_files() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let builds = [
            "20260101000000",
            "20260102000000",
            "20260103000000",
            "20260104000000",
        ];
        let mut assets = stable();
        assets.extend(feeds_listed());
        assets.extend(builds.iter().map(|b| update_asset(Platform::Macos, &id(b))));
        assets.push(update_asset(Platform::Windows, &id(builds[0])));
        listed(&mut read, &assets);
        // A newer build than this run's: a skip never compares.
        current_feed(&mut read, Platform::Macos, "20261231000000");
        current_feed(&mut read, Platform::Windows, builds[0]);
        let out =
            publish_with::<RecordingSigner>(&read, &FeedSigning::Skip(SkipReason::NoSeed), NEW);
        let published = out.result.expect("skips");
        let oldest_mac = update_asset(Platform::Macos, &id(builds[0]));
        assert_eq!(
            published,
            Published {
                feeds: Vec::new(),
                skipped: Some(SkipReason::NoSeed),
                prune: vec![oldest_mac.clone()],
            }
        );
        assert_eq!(
            out.events,
            [Event::Write(paths().prune_list, prune_text(&[&oldest_mac]))]
        );
    }

    #[test]
    fn a_missing_installer_in_dist_is_an_error() {
        for platform in Platform::ALL {
            let mut read = FakeRead::default();
            dist_files(&mut read);
            let missing = paths().dist.join(platform.stable_asset());
            read.0.remove(&missing);
            listed(&mut read, &stable());
            let (signer, _) = recording();
            let out = publish_with(&read, &FeedSigning::Sign(signer), NEW);
            let error = out.result.expect_err("missing");
            assert!(
                matches!(&error, AppcastError::Read { path, source }
                    if *path == missing && source.kind() == io::ErrorKind::NotFound),
                "{error:?}"
            );
            assert!(
                error.to_string().contains(&missing.display().to_string()),
                "{error}"
            );
            assert_eq!(out.events, []);
        }
    }

    #[test]
    fn a_missing_asset_list_is_an_error() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        let out =
            publish_with::<RecordingSigner>(&read, &FeedSigning::Skip(SkipReason::Neither), NEW);
        let error = out.result.expect_err("no list");
        assert!(
            matches!(&error, AppcastError::Read { path, .. } if *path == paths().assets),
            "{error:?}"
        );
    }

    #[test]
    fn the_asset_list_s_blank_lines_and_spaces_are_ignored() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        read.0
            .insert(paths().assets, b"\n  old-asset.txt  \r\n\n".to_vec());
        let out =
            publish_with::<RecordingSigner>(&read, &FeedSigning::Skip(SkipReason::Neither), NEW);
        assert_eq!(out.result.expect("skips").prune, ["old-asset.txt"]);
    }

    #[test]
    fn a_failed_write_is_an_error_naming_the_file() {
        let mut read = FakeRead::default();
        dist_files(&mut read);
        listed(&mut read, &stable());
        let log = Log::default();
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(|e| matches!(e, Event::Write(..))),
            absent: Box::new(|_| false),
        };
        let (signer, _) = recording();
        let error = publish(
            &read,
            &mut files,
            &spec(),
            &id(NEW),
            CHANNEL,
            &FeedSigning::Sign(signer),
            &paths(),
        )
        .expect_err("fails");
        let feed = paths().feeds.join("appcast-macos-nightly.xml");
        assert!(
            matches!(&error, AppcastError::File { action: "write", path, .. } if *path == feed),
            "{error:?}"
        );
        for (action, fails) in [
            (
                "copy",
                Box::new(|e: &Event| matches!(e, Event::Copy(..))) as Box<dyn Fn(&Event) -> bool>,
            ),
            (
                "create",
                Box::new(|e: &Event| matches!(e, Event::CreateDir(..))),
            ),
        ] {
            let mut files = FakeFiles {
                log: Log::default(),
                fails,
                absent: Box::new(|_| false),
            };
            let (signer, _) = recording();
            let error = publish(
                &read,
                &mut files,
                &spec(),
                &id(NEW),
                CHANNEL,
                &FeedSigning::Sign(signer),
                &paths(),
            )
            .expect_err("fails");
            assert!(
                matches!(&error, AppcastError::File { action: a, .. } if *a == action),
                "{error:?}"
            );
        }
        let mut files = FakeFiles {
            log: Log::default(),
            fails: Box::new(|e| matches!(e, Event::Write(..))),
            absent: Box::new(|_| false),
        };
        let error = publish::<RecordingSigner>(
            &read,
            &mut files,
            &spec(),
            &id(NEW),
            CHANNEL,
            &FeedSigning::Skip(SkipReason::Neither),
            &paths(),
        )
        .expect_err("fails");
        assert!(
            matches!(&error, AppcastError::File { action: "write", path, .. } if *path == paths().prune_list),
            "{error:?}"
        );
    }

    // run and public_key

    fn metadata_with_key(key: &PublicKey) -> Vec<u8> {
        let mut bundle = full_bundle();
        bundle["update_public_key"] = json!(key.to_base64());
        metadata_with(&bundle).to_string().into_bytes()
    }

    fn run_with(
        env: &FakeEnv,
        metadata_json: Vec<u8>,
    ) -> (Result<Published, AppcastError>, Vec<Event>) {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(move |i| {
                i.args
                    .first()
                    .is_some_and(|a| a == "metadata")
                    .then(|| Ok(metadata_json.clone()))
            }),
        };
        let mut read = FakeRead::default();
        dist_files(&mut read);
        listed(&mut read, &stable());
        let result = run(
            &mut process,
            env,
            &read,
            &mut files(&log),
            &OsString::from("/rust/bin/cargo"),
            &root(),
            &paths(),
            make_ok,
        );
        let events = log.borrow().clone();
        (result, events)
    }

    fn env_with_seed(seed: &str) -> FakeEnv {
        let mut env = build_env();
        env.0.insert(SEED_VAR, seed.into());
        env
    }

    #[test]
    fn run_reads_the_build_id_and_the_spec_and_signs_with_the_seed() {
        let key = test_signer().public_key();
        let (result, events) = run_with(&env_with_seed(&seed_base64()), metadata_with_key(&key));
        let published = result.expect("published");
        assert_eq!(published.skipped, None);
        assert_eq!(published.feeds.len(), 2);
        assert_eq!(
            events.first(),
            Some(&Event::Run(metadata_run(&OsString::from(
                "/rust/bin/cargo"
            ))))
        );
        assert!(events.contains(&Event::Copy(
            paths().dist.join(Platform::Macos.stable_asset()),
            paths().dist.join(update_asset(Platform::Macos, &id(NEW))),
        )));
    }

    #[test]
    fn run_without_a_channel_fails_naming_the_variable_and_writes_nothing() {
        for value in [None, Some("")] {
            let mut env = env_with_seed(&seed_base64());
            match value {
                None => env.0.remove(channel::VAR),
                Some(v) => env.0.insert(channel::VAR, v.into()),
            };
            let key = test_signer().public_key();
            let (result, events) = run_with(&env, metadata_with_key(&key));
            let error = result.expect_err("no channel");
            assert!(matches!(error, AppcastError::NoChannel), "{error:?}");
            assert!(error.to_string().contains("NOVA_CHANNEL"), "{error}");
            assert_eq!(events, [], "{value:?}");
        }
    }

    #[test]
    fn run_with_an_unknown_channel_fails_naming_it() {
        let mut env = env_with_seed(&seed_base64());
        env.0.insert(channel::VAR, "stable".into());
        let (result, events) = run_with(&env, sample_metadata());
        let error = result.expect_err("unknown");
        assert!(matches!(error, AppcastError::Channel(_)), "{error:?}");
        assert!(error.to_string().contains("`stable`"), "{error}");
        assert_eq!(events, []);
    }

    #[test]
    fn run_publishes_the_channel_the_environment_names() {
        let mut env = env_with_seed(&seed_base64());
        env.0.insert(channel::VAR, "beta".into());
        let key = test_signer().public_key();
        let (result, _) = run_with(&env, metadata_with_key(&key));
        assert_eq!(
            result.expect("published").feeds,
            vec![
                paths().feeds.join("appcast-macos-beta.xml"),
                paths().feeds.join("appcast-windows-beta.xml"),
            ]
        );
    }

    #[test]
    fn run_without_a_committed_key_skips() {
        let (result, _) = run_with(&env_with_seed(&seed_base64()), sample_metadata());
        assert_eq!(
            result.expect("skips").skipped,
            Some(SkipReason::NoPublicKey)
        );
        let (result, _) = run_with(&build_env(), sample_metadata());
        assert_eq!(result.expect("skips").skipped, Some(SkipReason::Neither));
    }

    #[test]
    fn run_with_a_mismatched_seed_fails_before_any_write() {
        let (result, events) = run_with(
            &env_with_seed(&seed_base64()),
            metadata_with_key(&other_key()),
        );
        assert!(
            matches!(result, Err(AppcastError::KeyMismatch { .. })),
            "{result:?}"
        );
        assert!(
            events.iter().all(|e| matches!(e, Event::Run(_))),
            "{events:?}"
        );
    }

    #[test]
    fn run_without_a_build_id_or_a_spec_fails() {
        let mut env = FakeEnv::default();
        env.0.insert(SEED_VAR, seed_base64().into());
        env.0.insert(channel::VAR, "nightly".into());
        let (result, _) = run_with(&env, sample_metadata());
        assert!(
            matches!(result, Err(AppcastError::BuildId(_))),
            "{result:?}"
        );
        let (result, _) = run_with(&build_env(), b"not json".to_vec());
        assert!(matches!(result, Err(AppcastError::Spec(_))), "{result:?}");
    }

    #[test]
    fn run_with_a_failing_cargo_fails_naming_it() {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(|i| {
                i.args
                    .first()
                    .is_some_and(|a| a == "metadata")
                    .then_some(Err(RunError::Failed(Some(101))))
            }),
        };
        let error = run(
            &mut process,
            &build_env(),
            &FakeRead::default(),
            &mut files(&log),
            &OsString::from("cargo"),
            &root(),
            &paths(),
            make_ok,
        )
        .expect_err("fails");
        assert!(
            matches!(
                &error,
                AppcastError::Run {
                    source: RunError::Failed(Some(101)),
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(error.to_string().contains("cargo metadata"), "{error}");
    }

    #[test]
    fn the_public_key_is_the_seed_s() {
        let env = FakeEnv(HashMap::from([(SEED_VAR, seed_base64().into())]));
        assert_eq!(
            public_key(&env, make_ok).expect("a key"),
            test_signer().public_key()
        );
    }

    #[test]
    fn the_public_key_needs_a_well_formed_seed() {
        for env in [
            FakeEnv::default(),
            FakeEnv(HashMap::from([(SEED_VAR, "".into())])),
        ] {
            assert!(matches!(
                public_key(&env, make_ok),
                Err(AppcastError::NoSeed)
            ));
        }
        let env = FakeEnv(HashMap::from([(SEED_VAR, "short".into())]));
        let error = public_key(&env, make_ok).expect_err("malformed");
        assert!(matches!(error, AppcastError::Seed(_)), "{error:?}");
        assert!(!error.to_string().contains("short"), "{error}");
    }
}
