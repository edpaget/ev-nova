//! `nova-appcast check-promotion`'s decision: whether a nightly build may
//! be promoted to beta, and the commit of `main` it was built from.
//!
//! [`check_promotion`] refuses, in this order:
//!
//! 1. a run off `main` ([`REF_VAR`] isn't [`MAIN_REF`]), so `promote-beta`
//!    runs only on `main`;
//! 2. a `--build` that isn't a build ID;
//! 3. a build the nightly release doesn't hold: its asset list must name
//!    the build's update files for both platforms ([`held_builds`]), which
//!    are the ones phase 3's pruning keeps ([`super::prune::KEEP_BUILDS`]);
//! 4. a build not newer than every current beta feed's
//!    ([`refuse_unless_newer_than_beta`]), the same build included;
//! 5. a build whose commit the nightly release doesn't record, or records
//!    malformed ([`recorded_commit`]);
//! 6. a recorded commit that isn't on `main`, or whose committer time on
//!    `main` isn't the build ID ([`verify_commit`]).
//!
//! The nightly's publish records the commit it built in the build's commit
//! file ([`super::commit::commit_asset`]), so the commit is read, never
//! guessed: the build ID is HEAD's committer time ([`crate::build_id`]),
//! but landing's rebase gives a run of commits the same committer second,
//! so the time alone can't name one. `main`'s history then confirms the
//! record: [`commit_times`] reads it with `git log`, through the
//! [`Process`] port. Like `publish`, it reads release data only from
//! local files CI fetched first, through [`ReadFiles`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::commit::{Commit, CommitError, commit_asset};
use super::publish::{AppcastError, current_feeds, read_asset_list, read_at};
use super::{Channel, CurrentFeed, Platform, parse_update_asset};
use crate::build_id::{BuildId, BuildIdError};
use crate::ports::{Env, Invocation, Process, ReadFiles, RunError};

/// The variable GitHub Actions sets to the ref a run is on.
pub const REF_VAR: &str = "GITHUB_REF";

/// The only ref a promotion runs on.
pub const MAIN_REF: &str = "refs/heads/main";

/// Where `check-promotion` reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionPaths {
    /// A checkout of `main`, with its full history.
    pub repo: PathBuf,
    /// The nightly release's assets, one name per line.
    pub nightly_assets: PathBuf,
    /// The nightly release's commit files, downloaded.
    pub nightly_commits: PathBuf,
    /// The beta release's assets, one name per line: empty before the
    /// first promotion.
    pub beta_assets: PathBuf,
    /// The beta release's current feeds, downloaded.
    pub beta_current: PathBuf,
}

/// A build cleared for promotion, and the commit it was built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Promotion {
    /// The nightly's build ID.
    pub build: BuildId,
    /// The commit of `main` the nightly recorded building.
    pub commit: Commit,
}

impl Promotion {
    /// The lines CI appends to `$GITHUB_OUTPUT`: `build=<id>` and
    /// `commit=<sha>`.
    #[must_use]
    pub fn output(&self) -> String {
        format!("build={}\ncommit={}\n", self.build, self.commit)
    }
}

/// Why a build can't be promoted.
#[derive(Debug, thiserror::Error)]
pub enum PromoteError {
    /// The run isn't on `main`.
    #[error("promote-beta runs only on main; this run is on {0}")]
    NotOnMain(String),
    /// [`REF_VAR`] is unset or empty, so the run's ref is unknown.
    #[error("promote-beta runs only on main; {REF_VAR} is unset, so this run's ref is unknown")]
    NoRef,
    /// `--build` isn't a build ID.
    #[error("--build: {0}")]
    Malformed(BuildIdError),
    /// The nightly release doesn't hold both platforms' update files of
    /// the build.
    #[error("build {build} isn't on the nightly release, which {}", holds(held))]
    NotOnNightly {
        /// The build asked for.
        build: BuildId,
        /// The builds it does hold, oldest first.
        held: Vec<BuildId>,
    },
    /// A current beta feed offers this very build.
    #[error("build {build} is already the beta")]
    AlreadyBeta {
        /// The build asked for.
        build: BuildId,
    },
    /// A current beta feed offers a newer build.
    #[error("build {build} is not newer than build {beta}, the current beta")]
    NotNewerThanBeta {
        /// The build asked for.
        build: BuildId,
        /// The newest build a current beta feed offers.
        beta: BuildId,
    },
    /// The nightly release holds no commit file for the build.
    #[error(
        "build {build} has no recorded commit: the nightly release holds no {}, so it was \
         built before commits were recorded, and can't be promoted",
        commit_asset(build)
    )]
    NoRecordedCommit {
        /// The build asked for.
        build: BuildId,
    },
    /// The build's commit file doesn't hold a commit.
    #[error(
        "build {build}'s {} on the nightly release is malformed: {source}",
        commit_asset(build)
    )]
    MalformedCommit {
        /// The build asked for.
        build: BuildId,
        /// What it holds instead.
        source: CommitError,
    },
    /// The recorded commit isn't in `main`'s history.
    #[error("build {build}'s recorded commit {commit} is not on main")]
    CommitNotOnMain {
        /// The build asked for.
        build: BuildId,
        /// The commit its file records.
        commit: Commit,
    },
    /// The recorded commit's committer time on `main` isn't the build ID.
    #[error(
        "build {build}'s recorded commit {commit} has committer time {} on main, not {build}",
        committed_as_text(*committed)
    )]
    CommitTimeMismatch {
        /// The build asked for.
        build: BuildId,
        /// The commit its file records.
        commit: Commit,
        /// Its committer time, in Unix seconds.
        committed: i64,
    },
    /// `git log` failed.
    #[error("`{command}` failed: {source}")]
    Git {
        /// The command line.
        command: String,
        /// How it failed.
        source: RunError,
    },
    /// `git log` printed a line that isn't a commit and a time.
    #[error("`git log` printed `{0}`, not a commit and a committer time")]
    GitOutput(String),
    /// The asset lists, the commit file or the beta feeds can't be read.
    #[error(transparent)]
    Release(#[from] AppcastError),
}

/// "holds builds A, B and C", or "holds no builds".
fn holds(held: &[BuildId]) -> String {
    match held {
        [] => "holds no builds".to_owned(),
        [one] => format!("holds build {one}"),
        _ => format!("holds builds {}", and_list(held)),
    }
}

/// A committer time as the build ID it is, or as Unix seconds when it
/// names none.
fn committed_as_text(secs: i64) -> String {
    BuildId::from_unix(secs).map_or_else(|_| format!("{secs} (Unix seconds)"), |b| b.to_string())
}

/// `builds` as prose: "A", "A and B", "A, B and C"; empty for none.
pub(crate) fn and_list(builds: &[BuildId]) -> String {
    let names: Vec<String> = builds.iter().map(ToString::to_string).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Refuses a run whose ref (`git_ref`, [`REF_VAR`]'s value) isn't
/// [`MAIN_REF`].
///
/// # Errors
///
/// [`PromoteError::NoRef`] for an unset or empty ref, and
/// [`PromoteError::NotOnMain`] naming any other.
pub fn refuse_off_main(git_ref: Option<&str>) -> Result<(), PromoteError> {
    match git_ref {
        None | Some("") => Err(PromoteError::NoRef),
        Some(MAIN_REF) => Ok(()),
        Some(other) => Err(PromoteError::NotOnMain(other.to_owned())),
    }
}

/// The builds a release whose assets are `assets` holds the update files
/// of for every platform, oldest first.
#[must_use]
pub fn held_builds(assets: &[String]) -> Vec<BuildId> {
    let mut platforms: BTreeMap<BuildId, BTreeSet<Platform>> = BTreeMap::new();
    for (platform, build) in assets.iter().filter_map(|a| parse_update_asset(a)) {
        platforms.entry(build).or_default().insert(platform);
    }
    platforms
        .into_iter()
        .filter(|(_, held)| held.len() == Platform::ALL.len())
        .map(|(build, _)| build)
        .collect()
}

/// Refuses `build` unless it is newer than every current beta feed's.
///
/// # Errors
///
/// [`PromoteError::NotNewerThanBeta`] naming the newest current beta when
/// one is newer, else [`PromoteError::AlreadyBeta`] when one offers
/// `build`.
pub fn refuse_unless_newer_than_beta(
    build: &BuildId,
    beta: &[CurrentFeed],
) -> Result<(), PromoteError> {
    match beta.iter().map(|feed| &feed.build).max() {
        Some(newest) if newest > build => Err(PromoteError::NotNewerThanBeta {
            build: build.clone(),
            beta: newest.clone(),
        }),
        Some(newest) if newest == build => Err(PromoteError::AlreadyBeta {
            build: build.clone(),
        }),
        _ => Ok(()),
    }
}

/// The commit `build`'s commit file records: `assets` (the nightly
/// release's) must list the file, which CI downloaded into `dir`.
///
/// # Errors
///
/// [`PromoteError::NoRecordedCommit`] when `assets` doesn't list it,
/// [`PromoteError::Release`] when it wasn't downloaded or can't be read,
/// and [`PromoteError::MalformedCommit`] when it holds no commit.
pub fn recorded_commit(
    read: &impl ReadFiles,
    assets: &[String],
    dir: &Path,
    build: &BuildId,
) -> Result<Commit, PromoteError> {
    let name = commit_asset(build);
    if !assets.contains(&name) {
        return Err(PromoteError::NoRecordedCommit {
            build: build.clone(),
        });
    }
    let bytes = read_at(read, &dir.join(&name))?;
    Commit::parse(&String::from_utf8_lossy(&bytes)).map_err(|source| {
        PromoteError::MalformedCommit {
            build: build.clone(),
            source,
        }
    })
}

/// `git log` printing every commit reachable from HEAD in the checkout at
/// `repo`, newest first, as `<sha> <committer time>` lines.
#[must_use]
pub fn commit_times(repo: &Path) -> Invocation {
    Invocation::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", "--format=%H %ct", "HEAD"])
        .captured()
}

/// Checks `commit`, `build`'s recorded commit, against `log`
/// ([`commit_times`]'s output, `main`'s history): it must be there, with
/// `build` as its committer time.
///
/// # Errors
///
/// [`PromoteError::GitOutput`] for a line that isn't a commit and a time,
/// [`PromoteError::CommitNotOnMain`] when `commit` isn't in `log`, and
/// [`PromoteError::CommitTimeMismatch`] when its time isn't `build`.
pub fn verify_commit(log: &str, build: &BuildId, commit: &Commit) -> Result<(), PromoteError> {
    let mut committed = None;
    for line in log.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let garbage = || PromoteError::GitOutput(line.to_owned());
        let (sha, time) = line.split_once(' ').ok_or_else(garbage)?;
        let sha = Commit::parse(sha).map_err(|_| garbage())?;
        let secs: i64 = time.parse().map_err(|_| garbage())?;
        if sha == *commit {
            committed = Some(secs);
        }
    }
    let Some(secs) = committed else {
        return Err(PromoteError::CommitNotOnMain {
            build: build.clone(),
            commit: commit.clone(),
        });
    };
    if BuildId::from_unix(secs).ok().as_ref() == Some(build) {
        Ok(())
    } else {
        Err(PromoteError::CommitTimeMismatch {
            build: build.clone(),
            commit: commit.clone(),
            committed: secs,
        })
    }
}

/// The whole of `nova-appcast check-promotion --build <build>`: the
/// checks in the module docs' order, then the commit.
///
/// # Errors
///
/// Each refusal in the module docs, or an asset list, a commit file, a
/// beta feed or `git log` that can't be read.
pub fn check_promotion(
    env: &impl Env,
    read: &impl ReadFiles,
    process: &mut impl Process,
    build: &str,
    paths: &PromotionPaths,
) -> Result<Promotion, PromoteError> {
    let git_ref = env.var(REF_VAR).map(|r| r.to_string_lossy().into_owned());
    refuse_off_main(git_ref.as_deref())?;
    let build = BuildId::parse(build).map_err(PromoteError::Malformed)?;
    let nightly = read_asset_list(read, &paths.nightly_assets)?;
    let held = held_builds(&nightly);
    if !held.contains(&build) {
        return Err(PromoteError::NotOnNightly { build, held });
    }
    let beta_assets = read_asset_list(read, &paths.beta_assets)?;
    let beta: Vec<CurrentFeed> =
        current_feeds(read, &beta_assets, &paths.beta_current, Channel::Beta)?
            .into_iter()
            .map(|(_, feed)| feed)
            .collect();
    refuse_unless_newer_than_beta(&build, &beta)?;
    let commit = recorded_commit(read, &nightly, &paths.nightly_commits, &build)?;
    let invocation = commit_times(&paths.repo);
    let log = process
        .run(&invocation)
        .map_err(|source| PromoteError::Git {
            command: invocation.to_string(),
            source,
        })?;
    verify_commit(&String::from_utf8_lossy(&log), &build, &commit)?;
    Ok(Promotion { build, commit })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::rc::Rc;

    use super::*;
    use crate::appcast::publish::one_per_line;
    use crate::appcast::tests::test_signer;
    use crate::appcast::{Feed, feed_name, render, update_asset};
    use crate::test_support::{Event, FakeEnv, FakeRead, Log, Recorder};

    fn id(text: &str) -> BuildId {
        BuildId::parse(text).expect("an ID")
    }

    const OLD: &str = "20261026000000";
    const MID: &str = "20261027000000";
    const NEW: &str = "20261028123456";

    /// `NEW`'s Unix time.
    const NEW_SECS: i64 = 1_793_190_896;

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const SHA_C: &str = "cccccccccccccccccccccccccccccccccccccccc";
    const SHA_D: &str = "dddddddddddddddddddddddddddddddddddddddd";

    fn sha(text: &str) -> Commit {
        Commit::parse(text).expect("a commit")
    }

    fn root() -> PathBuf {
        Path::new("/work").join("run")
    }

    fn paths() -> PromotionPaths {
        PromotionPaths {
            repo: root().join("main"),
            nightly_assets: root().join("nightly-assets.txt"),
            nightly_commits: root().join("nightly-commits"),
            beta_assets: root().join("beta-assets.txt"),
            beta_current: root().join("beta-current"),
        }
    }

    fn both(build: &str) -> Vec<String> {
        Platform::ALL
            .into_iter()
            .map(|p| update_asset(p, &id(build)))
            .collect()
    }

    fn main_env() -> FakeEnv {
        FakeEnv(HashMap::from([(REF_VAR, MAIN_REF.into())]))
    }

    /// The nightly holding `OLD`, `MID` and `NEW`, with `NEW`'s commit
    /// file recording `SHA_B`; no beta yet; and `git log` answering `log`,
    /// where `SHA_B` and `SHA_C` share `NEW`'s second, as a landing's
    /// rebase leaves them.
    struct World {
        env: FakeEnv,
        read: FakeRead,
        log: String,
    }

    impl World {
        fn new() -> Self {
            let mut read = FakeRead::default();
            let mut nightly = vec!["ev-nova-aarch64-apple-darwin.dmg".to_owned()];
            for build in [OLD, MID, NEW] {
                nightly.extend(both(build));
            }
            nightly.push(commit_asset(&id(NEW)));
            read.0
                .insert(paths().nightly_assets, one_per_line(&nightly).into_bytes());
            read.0.insert(paths().beta_assets, Vec::new());
            let mut world = Self {
                env: main_env(),
                read,
                log: format!(
                    "{SHA_A} {}\n{SHA_C} {NEW_SECS}\n{SHA_B} {NEW_SECS}\n",
                    NEW_SECS + 60
                ),
            };
            world.record(NEW, SHA_B);
            world
        }

        /// `build`'s downloaded commit file holding `text` and a newline.
        fn record(&mut self, build: &str, text: &str) {
            self.read.0.insert(
                paths().nightly_commits.join(commit_asset(&id(build))),
                format!("{text}\n").into_bytes(),
            );
        }

        /// The nightly release without `NEW`'s commit file.
        fn unrecorded(mut self) -> Self {
            let listed = paths().nightly_assets;
            let text = String::from_utf8(self.read.0[&listed].clone()).expect("utf-8");
            let without: Vec<&str> = text
                .lines()
                .filter(|l| *l != commit_asset(&id(NEW)))
                .collect();
            self.read
                .0
                .insert(listed, one_per_line(&without).into_bytes());
            self.read
                .0
                .remove(&paths().nightly_commits.join(commit_asset(&id(NEW))));
            self
        }

        /// The beta release offering `build` on both feeds.
        fn beta(mut self, build: &str) -> Self {
            let mut assets = both(build);
            for platform in Platform::ALL {
                let name = feed_name(platform, Channel::Beta);
                assets.push(name.clone());
                let feed = Feed::signed(
                    platform,
                    Channel::Beta,
                    "EV Nova",
                    &id(build),
                    "0.1.0",
                    b"x",
                    &test_signer(),
                );
                self.read
                    .0
                    .insert(paths().beta_current.join(name), render(&feed).into_bytes());
            }
            self.read
                .0
                .insert(paths().beta_assets, one_per_line(&assets).into_bytes());
            self
        }

        fn check(&self, build: &str) -> (Result<Promotion, PromoteError>, Vec<Event>) {
            let log = Log::default();
            let answer = self.log.clone().into_bytes();
            let mut process = Recorder {
                log: Rc::clone(&log),
                archs: Vec::new(),
                respond: Box::new(move |i| (i.program == "git").then(|| Ok(answer.clone()))),
            };
            let result = check_promotion(&self.env, &self.read, &mut process, build, &paths());
            let events = log.borrow().clone();
            (result, events)
        }
    }

    // The ref.

    #[test]
    fn only_main_passes_the_ref_check() {
        assert!(refuse_off_main(Some(MAIN_REF)).is_ok());
        for other in [
            "refs/heads/roadmap/auto-updates",
            "refs/tags/nightly",
            "refs/heads/mainline",
            "refs/pull/1/merge",
        ] {
            let error = refuse_off_main(Some(other)).expect_err("off main");
            assert!(
                matches!(&error, PromoteError::NotOnMain(r) if r == other),
                "{error:?}"
            );
            assert_eq!(
                error.to_string(),
                format!("promote-beta runs only on main; this run is on {other}")
            );
        }
        for unset in [None, Some("")] {
            let error = refuse_off_main(unset).expect_err("no ref");
            assert!(matches!(error, PromoteError::NoRef), "{error:?}");
            assert!(error.to_string().contains("GITHUB_REF"), "{error}");
        }
    }

    // Held builds.

    #[test]
    fn a_build_is_held_only_with_both_platforms_update_files() {
        let mut assets = both(OLD);
        assets.push(update_asset(Platform::Macos, &id(MID)));
        assets.push(update_asset(Platform::Windows, &id("20261027000001")));
        assets.extend(both(NEW));
        assets.push("ev-nova-aarch64-apple-darwin.dmg".to_owned());
        assets.push("appcast-macos-nightly.xml".to_owned());
        assets.push(commit_asset(&id(MID)));
        assert_eq!(held_builds(&assets), vec![id(OLD), id(NEW)]);
        assert_eq!(held_builds(&[]), Vec::<BuildId>::new());
    }

    #[test]
    fn not_on_nightly_names_the_build_and_every_held_one() {
        let error = |held: &[&str]| PromoteError::NotOnNightly {
            build: id(NEW),
            held: held.iter().map(|b| id(b)).collect(),
        };
        assert_eq!(
            error(&[]).to_string(),
            format!("build {NEW} isn't on the nightly release, which holds no builds")
        );
        assert_eq!(
            error(&[OLD]).to_string(),
            format!("build {NEW} isn't on the nightly release, which holds build {OLD}")
        );
        assert_eq!(
            error(&[OLD, MID, "20261029000000"]).to_string(),
            format!(
                "build {NEW} isn't on the nightly release, which holds builds {OLD}, {MID} and \
                 20261029000000"
            )
        );
    }

    // Newer than the beta.

    fn current(build: &str) -> CurrentFeed {
        CurrentFeed {
            build: id(build),
            file: "x".into(),
        }
    }

    #[test]
    fn a_first_promotion_or_a_newer_build_passes() {
        assert!(refuse_unless_newer_than_beta(&id(NEW), &[]).is_ok());
        assert!(refuse_unless_newer_than_beta(&id(NEW), &[current(OLD), current(MID)]).is_ok());
    }

    #[test]
    fn the_same_build_is_already_the_beta() {
        let error = refuse_unless_newer_than_beta(&id(NEW), &[current(OLD), current(NEW)])
            .expect_err("equal");
        assert!(
            matches!(&error, PromoteError::AlreadyBeta { build } if *build == id(NEW)),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            format!("build {NEW} is already the beta")
        );
    }

    #[test]
    fn an_older_build_is_refused_naming_both_and_the_newest_beta_wins() {
        let error = refuse_unless_newer_than_beta(&id(MID), &[current(NEW), current(MID)])
            .expect_err("older");
        assert!(
            matches!(&error, PromoteError::NotNewerThanBeta { build, beta }
                if *build == id(MID) && *beta == id(NEW)),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            format!("build {MID} is not newer than build {NEW}, the current beta")
        );
    }

    // The recorded commit.

    #[test]
    fn the_recorded_commit_is_read_from_its_downloaded_file() {
        let world = World::new();
        let nightly = read_asset_list(&world.read, &paths().nightly_assets).expect("a list");
        assert_eq!(
            recorded_commit(&world.read, &nightly, &paths().nightly_commits, &id(NEW))
                .expect("recorded"),
            sha(SHA_B)
        );
    }

    #[test]
    fn a_build_without_a_listed_commit_file_was_built_before_commits_were_recorded() {
        // MID's file is downloaded but not listed: the list decides.
        let mut world = World::new();
        world.record(MID, SHA_A);
        let nightly = read_asset_list(&world.read, &paths().nightly_assets).expect("a list");
        let error = recorded_commit(&world.read, &nightly, &paths().nightly_commits, &id(MID))
            .expect_err("unrecorded");
        assert!(
            matches!(&error, PromoteError::NoRecordedCommit { build } if *build == id(MID)),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            format!(
                "build {MID} has no recorded commit: the nightly release holds no \
                 ev-nova-{MID}-commit.txt, so it was built before commits were recorded, and \
                 can't be promoted"
            )
        );
    }

    #[test]
    fn a_listed_commit_file_that_wasnt_downloaded_is_an_error_naming_it() {
        let mut world = World::new();
        let file = paths().nightly_commits.join(commit_asset(&id(NEW)));
        world.read.0.remove(&file);
        let nightly = read_asset_list(&world.read, &paths().nightly_assets).expect("a list");
        let error = recorded_commit(&world.read, &nightly, &paths().nightly_commits, &id(NEW))
            .expect_err("missing");
        assert!(
            matches!(&error, PromoteError::Release(AppcastError::Read { path, .. }) if *path == file),
            "{error:?}"
        );
    }

    #[test]
    fn a_malformed_commit_file_is_refused_naming_the_build_and_its_text() {
        for text in ["HEAD", &SHA_B[1..], "", "\u{fffd}"] {
            let mut world = World::new();
            world.record(NEW, text);
            let (result, events) = world.check(NEW);
            let error = result.expect_err("malformed");
            assert!(
                matches!(&error, PromoteError::MalformedCommit { build, source: CommitError(t) }
                    if *build == id(NEW) && t == text),
                "{text}: {error:?}"
            );
            let message = error.to_string();
            assert!(
                message.starts_with(&format!(
                    "build {NEW}'s ev-nova-{NEW}-commit.txt on the nightly release is malformed: "
                )) && message.contains(&format!("`{text}`")),
                "{message}"
            );
            assert_eq!(events, [], "{text}");
        }
        // Bytes that aren't UTF-8 are no commit either.
        let mut world = World::new();
        world.read.0.insert(
            paths().nightly_commits.join(commit_asset(&id(NEW))),
            vec![0xff; 40],
        );
        let (result, _) = world.check(NEW);
        assert!(
            matches!(&result, Err(PromoteError::MalformedCommit { .. })),
            "{result:?}"
        );
    }

    // The commit against main.

    #[test]
    fn the_recorded_commit_is_taken_though_others_share_its_second() {
        let log = format!(
            "{SHA_A} {}\n{SHA_B} {NEW_SECS}\n\n{SHA_C} {NEW_SECS}\n{SHA_D} {NEW_SECS}\n",
            NEW_SECS + 1,
        );
        assert_eq!(BuildId::from_unix(NEW_SECS).expect("an ID"), id(NEW));
        for recorded in [SHA_B, SHA_C, SHA_D] {
            assert!(
                verify_commit(&log, &id(NEW), &sha(recorded)).is_ok(),
                "{recorded}"
            );
        }
        let sha256 = "e".repeat(64);
        assert!(verify_commit(&format!("{sha256} {NEW_SECS}\n"), &id(NEW), &sha(&sha256)).is_ok());
    }

    #[test]
    fn a_recorded_commit_not_in_main_s_history_is_refused_naming_both() {
        for log in [String::new(), format!("{SHA_A} {NEW_SECS}\n")] {
            let error = verify_commit(&log, &id(NEW), &sha(SHA_B)).expect_err("not on main");
            assert!(
                matches!(&error, PromoteError::CommitNotOnMain { build, commit }
                    if *build == id(NEW) && *commit == sha(SHA_B)),
                "{error:?}"
            );
            assert_eq!(
                error.to_string(),
                format!("build {NEW}'s recorded commit {SHA_B} is not on main")
            );
        }
    }

    #[test]
    fn a_recorded_commit_at_another_time_is_refused_naming_both_times() {
        let log = format!("{SHA_A} {NEW_SECS}\n{SHA_B} {}\n", NEW_SECS + 60);
        let error = verify_commit(&log, &id(NEW), &sha(SHA_B)).expect_err("mismatch");
        assert!(
            matches!(&error, PromoteError::CommitTimeMismatch { build, commit, committed }
                if *build == id(NEW) && *commit == sha(SHA_B) && *committed == NEW_SECS + 60),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            format!(
                "build {NEW}'s recorded commit {SHA_B} has committer time 20261028123556 on \
                 main, not {NEW}"
            )
        );
        // A time no build ID can name is shown as seconds.
        let error =
            verify_commit(&format!("{SHA_B} -5\n"), &id(NEW), &sha(SHA_B)).expect_err("mismatch");
        assert!(
            error
                .to_string()
                .contains("has committer time -5 (Unix seconds) on main"),
            "{error}"
        );
    }

    #[test]
    fn a_line_that_isnt_a_commit_and_a_time_is_an_error() {
        for line in [
            "garbage".to_owned(),
            SHA_A.to_owned(),
            format!("{SHA_A} soon"),
            format!("{} {NEW_SECS}", &SHA_A[1..]),
            format!("{}x {NEW_SECS}", &SHA_A[1..]),
            format!("{} {NEW_SECS}", "e".repeat(41)),
            format!("{SHA_A} {NEW_SECS} extra"),
        ] {
            // Before or after the recorded commit's line.
            for log in [
                format!("{line}\n{SHA_B} {NEW_SECS}\n"),
                format!("{SHA_B} {NEW_SECS}\n{line}\n"),
            ] {
                let error = verify_commit(&log, &id(NEW), &sha(SHA_B)).expect_err("garbage");
                assert!(
                    matches!(&error, PromoteError::GitOutput(l) if *l == line),
                    "{line}: {error:?}"
                );
                assert!(error.to_string().contains(&line), "{error}");
            }
        }
    }

    #[test]
    fn the_git_command_lists_head_s_history_with_committer_times() {
        let repo = paths().repo;
        assert_eq!(
            commit_times(&repo),
            Invocation::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["log", "--format=%H %ct", "HEAD"])
                .captured()
        );
    }

    #[test]
    fn the_output_is_the_build_and_commit_lines() {
        let promotion = Promotion {
            build: id(NEW),
            commit: sha(SHA_B),
        };
        assert_eq!(promotion.output(), format!("build={NEW}\ncommit={SHA_B}\n"));
    }

    // The whole check.

    #[test]
    fn a_held_newer_build_on_main_resolves_to_its_recorded_commit() {
        let (result, events) = World::new().beta(MID).check(NEW);
        assert_eq!(
            result.expect("promotable"),
            Promotion {
                build: id(NEW),
                commit: sha(SHA_B),
            }
        );
        assert_eq!(events, [Event::Run(commit_times(&paths().repo))]);
        // SHA_C shares the second; recorded, it is the one taken.
        let mut world = World::new();
        world.record(NEW, SHA_C);
        assert_eq!(world.check(NEW).0.expect("promotable").commit, sha(SHA_C));
    }

    #[test]
    fn a_first_promotion_has_no_beta_to_compare_with() {
        let (result, _) = World::new().check(NEW);
        assert_eq!(result.expect("promotable").commit, sha(SHA_B));
    }

    #[test]
    fn the_checks_run_in_order_and_stop_at_the_first_refusal() {
        // Off main, a malformed ID still gets the main-only refusal.
        let mut world = World::new();
        world.env = FakeEnv(HashMap::from([(REF_VAR, "refs/heads/x".into())]));
        let (result, events) = world.check("not-an-id");
        assert!(
            matches!(result, Err(PromoteError::NotOnMain(_))),
            "{result:?}"
        );
        assert_eq!(events, []);

        // On main, a malformed ID is refused naming it, before any read.
        let mut world = World::new();
        world.read = FakeRead::default();
        let (result, events) = world.check("2026-10-28");
        let error = result.expect_err("malformed");
        assert!(
            matches!(&error, PromoteError::Malformed(BuildIdError::Malformed(t)) if t == "2026-10-28"),
            "{error:?}"
        );
        assert!(error.to_string().contains("`2026-10-28`"), "{error}");
        assert_eq!(events, []);

        // A build the nightly doesn't hold, though the beta is newer.
        let unheld = "20261025000000";
        let (result, events) = World::new().beta(NEW).check(unheld);
        assert!(
            matches!(&result, Err(PromoteError::NotOnNightly { build, held })
                if *build == id(unheld) && *held == [id(OLD), id(MID), id(NEW)]),
            "{result:?}"
        );
        assert_eq!(events, []);

        // A held build not newer than the beta, though it records no
        // commit and none is on main.
        let mut world = World::new().unrecorded().beta(NEW);
        world.log = String::new();
        let (result, events) = world.check(MID);
        assert!(
            matches!(&result, Err(PromoteError::NotNewerThanBeta { build, beta })
                if *build == id(MID) && *beta == id(NEW)),
            "{result:?}"
        );
        assert_eq!(events, []);
        let (result, _) = world.check(NEW);
        assert!(
            matches!(&result, Err(PromoteError::AlreadyBeta { .. })),
            "{result:?}"
        );

        // Then the recorded commit, before git runs, though none is on
        // main.
        let mut world = World::new().unrecorded();
        world.log = String::new();
        let (result, events) = world.check(NEW);
        assert!(
            matches!(&result, Err(PromoteError::NoRecordedCommit { build }) if *build == id(NEW)),
            "{result:?}"
        );
        assert_eq!(events, []);

        // Then main's history.
        let mut world = World::new();
        world.log = String::new();
        let (result, events) = world.check(NEW);
        assert!(
            matches!(&result, Err(PromoteError::CommitNotOnMain { build, commit })
                if *build == id(NEW) && *commit == sha(SHA_B)),
            "{result:?}"
        );
        assert_eq!(events, [Event::Run(commit_times(&paths().repo))]);
        let mut world = World::new();
        world.log = format!("{SHA_B} {}\n", NEW_SECS - 1);
        let (result, _) = world.check(NEW);
        assert!(
            matches!(&result, Err(PromoteError::CommitTimeMismatch { committed, .. })
                if *committed == NEW_SECS - 1),
            "{result:?}"
        );
    }

    #[test]
    fn a_listed_beta_feed_that_wasnt_downloaded_is_an_error() {
        let mut world = World::new().beta(MID);
        world.read.0.remove(
            &paths()
                .beta_current
                .join(feed_name(Platform::Windows, Channel::Beta)),
        );
        let (result, events) = world.check(NEW);
        assert!(
            matches!(&result, Err(PromoteError::Release(AppcastError::FeedNotDownloaded { feed, dir }))
                if feed == "appcast-windows-beta.xml" && *dir == paths().beta_current),
            "{result:?}"
        );
        assert_eq!(events, []);
    }

    #[test]
    fn a_malformed_beta_feed_is_an_error_naming_it() {
        let mut world = World::new().beta(MID);
        world.read.0.insert(
            paths()
                .beta_current
                .join(feed_name(Platform::Macos, Channel::Beta)),
            b"<rss/>".to_vec(),
        );
        let (result, _) = world.check(NEW);
        let error = result.expect_err("malformed");
        assert!(
            matches!(&error, PromoteError::Release(AppcastError::BadFeed { feed, .. })
                if feed == "appcast-macos-beta.xml"),
            "{error:?}"
        );
        assert!(
            error.to_string().contains("appcast-macos-beta.xml"),
            "{error}"
        );
    }

    #[test]
    fn nightly_feeds_among_the_beta_s_assets_are_not_compared() {
        let mut world = World::new();
        let names: Vec<String> = Platform::ALL
            .into_iter()
            .map(|p| feed_name(p, Channel::Nightly))
            .collect();
        world
            .read
            .0
            .insert(paths().beta_assets, one_per_line(&names).into_bytes());
        let (result, _) = world.check(NEW);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn a_missing_asset_list_is_an_error_naming_it() {
        for missing in [paths().nightly_assets, paths().beta_assets] {
            let mut world = World::new();
            world.read.0.remove(&missing);
            let (result, _) = world.check(NEW);
            assert!(
                matches!(&result, Err(PromoteError::Release(AppcastError::Read { path, .. }))
                    if *path == missing),
                "{result:?}"
            );
        }
    }

    #[test]
    fn a_failing_git_is_an_error_naming_the_command() {
        let world = World::new();
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(|_| Some(Err(RunError::Failed(Some(128))))),
        };
        let error = check_promotion(&world.env, &world.read, &mut process, NEW, &paths())
            .expect_err("git fails");
        assert!(
            matches!(
                &error,
                PromoteError::Git {
                    source: RunError::Failed(Some(128)),
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(error.to_string().contains("git -C"), "{error}");
    }
}
