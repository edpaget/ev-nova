//! `stage` and `sign` as sequences: whether to publish at all
//! ([`decide`]), then the steps each runs through the ports, stopping at
//! the first failure. Each step that made the gpg home ends with its
//! cleanup, whether the steps succeeded or not: the agent is stopped and
//! the home and the secret removed.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use super::key::{self, CommittedKey, Primary};
use super::probe::{self, Live};
use super::{
    MIRROR_DEPTH, PRUNE_DEPTH, PUBLIC_KEY, RepoError, RepoHost, SECRET_SET_VAR, SECRET_VAR,
    SignPaths, StagePaths, TITLE, app_ref, flatpakref, flatpakref_name, gnupg, repo, secret_dir,
    src,
};
use crate::appcast::Channel;
use crate::build_id::{self, BuildId};
use crate::bundle::{self, BundleError, Step, execute, metadata};
use crate::linux::flatpak::{ASSET, RUNTIME_REPO};
use crate::ports::{Env, Files, Invocation, Process};
use crate::signing::SecretBytes;
use crate::spec::BundleSpec;

/// The live repo's remote while it is mirrored, deleted after, so the
/// published `config` names no remote.
pub const MIRROR_REMOTE: &str = "live";

/// Why nothing is published this run. The nightly still publishes, with
/// the plain bundle and no `.flatpakref`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    /// `NOVA_CHANNEL` is unset: a development build.
    NoChannel,
    /// No public key is committed.
    NoKey,
    /// The secret is unavailable to this job.
    NoSecret,
}

impl Skip {
    /// Why, as the warning says it.
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Self::NoChannel => "NOVA_CHANNEL is unset (a development build)".to_owned(),
            Self::NoKey => format!(
                "no {PUBLIC_KEY} is committed: see infra/github/README.md, \"The Flatpak \
                 repository key\""
            ),
            Self::NoSecret => format!(
                "{PUBLIC_KEY} is committed, but {SECRET_VAR} is unset: the update-signing \
                 environment's secret is unavailable to this job"
            ),
        }
    }
}

/// Whether to publish, and with what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision<S> {
    /// Publish nothing, for this reason.
    Skip(Skip),
    /// Publish `channel`, signed by the committed key's secret.
    Publish {
        /// The channel's branch.
        channel: Channel,
        /// The committed key.
        key: CommittedKey,
        /// The secret, or the word that `sign` will hold it.
        secret: S,
    },
}

/// The decision table: no channel, no committed key, or no secret is a
/// skip, in that order; `key` reads the committed key (`None` when it is
/// absent) only once there is a channel, and a key it refuses is an
/// error whatever the secret.
///
/// # Errors
///
/// `key`'s.
pub fn decide<S>(
    channel: Option<Channel>,
    key: impl FnOnce() -> Result<Option<CommittedKey>, RepoError>,
    secret: Option<S>,
) -> Result<Decision<S>, RepoError> {
    let Some(channel) = channel else {
        return Ok(Decision::Skip(Skip::NoChannel));
    };
    let Some(key) = key()? else {
        return Ok(Decision::Skip(Skip::NoKey));
    };
    Ok(
        secret.map_or(Decision::Skip(Skip::NoSecret), |secret| Decision::Publish {
            channel,
            key,
            secret,
        }),
    )
}

/// What a step did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing, for this reason.
    Skipped(Skip),
    /// `stage` filled the work and pages trees from the repo at `url`.
    Staged {
        /// The repo's URL.
        url: String,
        /// Whether it was live, or this is the first publish.
        live: Live,
    },
    /// `sign` published.
    Published {
        /// The `.flatpakref` written.
        flatpakref: PathBuf,
        /// The repo-following bundle.
        bundle: PathBuf,
    },
}

/// `--<name>=<path>`, as one argument.
fn opt(name: &str, path: &Path) -> OsString {
    let mut arg = OsString::from(format!("--{name}="));
    arg.push(path);
    arg
}

/// `ostree --repo=<repo>` with these words.
fn ostree(repo: &Path, words: &[&str]) -> Invocation {
    Invocation::new("ostree").arg(opt("repo", repo)).args(words)
}

/// An empty archive-mode repository at `repo`.
#[must_use]
pub fn init(repo: &Path) -> Invocation {
    ostree(repo, &["init", "--mode=archive"])
}

/// The live repo at `url` as the [`MIRROR_REMOTE`] remote of `repo`, its
/// summary checked against `key_file`.
#[must_use]
pub fn remote_add(repo: &Path, key_file: &Path, url: &str) -> Invocation {
    ostree(repo, &["remote", "add"])
        .arg(opt("gpg-import", key_file))
        .args(["--set=gpg-verify-summary=true", MIRROR_REMOTE, url])
}

/// Every ref the live summary names (each channel's branch and the
/// appstream branches), with [`MIRROR_DEPTH`] parents, into `repo`.
#[must_use]
pub fn pull(repo: &Path) -> Invocation {
    ostree(
        repo,
        &[
            "pull",
            "--mirror",
            &format!("--depth={MIRROR_DEPTH}"),
            MIRROR_REMOTE,
        ],
    )
}

/// Drops the [`MIRROR_REMOTE`] remote from `repo`.
#[must_use]
pub fn remote_delete(repo: &Path) -> Invocation {
    ostree(repo, &["remote", "delete", MIRROR_REMOTE])
}

/// The plain bundle into the repository `src`.
#[must_use]
pub fn import_bundle(src: &Path, bundle: &Path) -> Invocation {
    Invocation::new("flatpak")
        .args(["build-import-bundle", "--no-update-summary"])
        .arg(src)
        .arg(bundle)
}

/// `stage`'s steps once the live repo is probed: the pages repo (an
/// empty one, then the mirror when one is `live`), then the plain bundle
/// imported into the work tree's own repo.
#[must_use]
pub fn stage_steps(host: &RepoHost, paths: &StagePaths, key_file: &Path, live: Live) -> Vec<Step> {
    let repo = repo(&paths.pages);
    let src = src(&paths.work);
    let mut steps = vec![
        Step::RemoveDir(repo.clone()),
        Step::CreateDir(paths.pages.clone()),
        Step::Run(init(&repo)),
    ];
    if live == Live::Present {
        steps.extend([
            Step::Run(remote_add(&repo, key_file, &host.url)),
            Step::Run(pull(&repo)),
            Step::Run(remote_delete(&repo)),
        ]);
    }
    steps.extend([
        Step::RemoveDir(src.clone()),
        Step::Run(init(&src)),
        Step::Run(import_bundle(&src, &paths.bundle_dir.join(ASSET))),
    ]);
    steps
}

/// The new build onto `channel`'s branch of the pages repo, a child of
/// its head, signed by `fingerprint`. Without `--force`, a build whose
/// content is the head's commits nothing.
#[must_use]
pub fn commit_from(
    spec: &BundleSpec,
    channel: Channel,
    work: &Path,
    pages: &Path,
    fingerprint: &str,
    build: &BuildId,
) -> Invocation {
    let app = app_ref(spec, channel);
    Invocation::new("flatpak")
        .arg("build-commit-from")
        .arg(opt("src-repo", &src(work)))
        .arg(format!("--src-ref={app}"))
        .arg(format!("--gpg-sign={fingerprint}"))
        .arg(opt("gpg-homedir", &gnupg(work)))
        .arg(format!(
            "--subject={TITLE} {} build {build}",
            channel.name()
        ))
        .arg("--no-update-summary")
        .arg(repo(pages))
        .arg(app)
}

/// The signed summary, pruned to [`PRUNE_DEPTH`] parents per branch, with
/// static deltas to each head.
#[must_use]
pub fn update_repo(work: &Path, pages: &Path, fingerprint: &str) -> Invocation {
    Invocation::new("flatpak")
        .arg("build-update-repo")
        .arg(format!("--gpg-sign={fingerprint}"))
        .arg(opt("gpg-homedir", &gnupg(work)))
        .args([
            "--generate-static-deltas",
            "--prune",
            &format!("--prune-depth={PRUNE_DEPTH}"),
            &format!("--title={TITLE}"),
        ])
        .arg(repo(pages))
}

/// The bundle of `channel`'s branch from the signed repo, naming the repo
/// at `url` and its key, so an install of it follows the repo.
#[must_use]
pub fn build_bundle(
    spec: &BundleSpec,
    channel: Channel,
    url: &str,
    key_file: &Path,
    pages: &Path,
    dist: &Path,
) -> Invocation {
    Invocation::new("flatpak")
        .arg("build-bundle")
        .arg(format!("--runtime-repo={RUNTIME_REPO}"))
        .arg(format!("--repo-url={url}"))
        .arg(opt("gpg-keys", key_file))
        .arg(repo(pages))
        .arg(dist.join(ASSET))
        .arg(&spec.identifier)
        .arg(channel.name())
}

/// The secret key's file, in [`secret_dir`].
#[must_use]
pub fn secret_file(work: &Path) -> PathBuf {
    secret_dir(work).join("key.gpg")
}

/// The secret onto disk, owner-only, imported into the gpg home, and
/// removed.
#[must_use]
pub fn import_steps(work: &Path, secret: SecretBytes) -> Vec<Step> {
    let dir = secret_dir(work);
    let file = secret_file(work);
    vec![
        Step::RemoveDir(dir.clone()),
        Step::CreateDir(dir.clone()),
        Step::WriteSecret {
            path: file.clone(),
            bytes: secret,
        },
        Step::Run(key::import(&gnupg(work), &file)),
        Step::RemoveDir(dir),
    ]
}

/// What runs once the gpg home exists, whatever the steps did.
#[must_use]
pub fn cleanup(work: &Path, signed: bool) -> Vec<Step> {
    let mut steps = vec![
        Step::Run(key::kill_agent(&gnupg(work))),
        Step::RemoveDir(gnupg(work)),
    ];
    if signed {
        steps.push(Step::RemoveDir(secret_dir(work)));
    }
    steps
}

/// The spec, from `cargo metadata`.
fn read_spec(process: &mut impl Process, host: &RepoHost) -> Result<BundleSpec, RepoError> {
    let json = bundle::run(process, &metadata(&host.cargo))?;
    Ok(BundleSpec::from_metadata(&json).map_err(BundleError::from)?)
}

/// Runs `steps`, stopping at the first failure.
fn run_steps(
    steps: &[Step],
    process: &mut impl Process,
    files: &mut impl Files,
) -> Result<(), RepoError> {
    steps
        .iter()
        .try_for_each(|step| execute(step, process, files))
        .map_err(RepoError::from)
}

/// Whether `env` holds a value for `name` other than the empty one.
fn set(env: &impl Env, name: &str) -> Option<OsString> {
    env.var(name).filter(|value| !value.is_empty())
}

/// The committed key at `key_file`, read in a fresh gpg home in `work`,
/// or `None` when no file is committed. `touched` is set once the home
/// exists, so its cleanup runs.
fn committed_key(
    process: &mut impl Process,
    files: &mut impl Files,
    work: &Path,
    key_file: &Path,
    touched: &mut bool,
) -> Result<Option<CommittedKey>, RepoError> {
    if !files.is_file(key_file) {
        return Ok(None);
    }
    *touched = true;
    let home = gnupg(work);
    run_steps(
        &[Step::RemoveDir(home.clone()), Step::CreateDir(home.clone())],
        process,
        files,
    )?;
    key::inspect(process, &home, key_file).map(Some)
}

/// Runs `cleanup` after `result`, keeping `result`'s error, else the
/// first cleanup error.
fn finish<T>(
    result: Result<T, RepoError>,
    cleanup: &[Step],
    process: &mut impl Process,
    files: &mut impl Files,
) -> Result<T, RepoError> {
    let mut cleaned = Ok(());
    for step in cleanup {
        if let Err(e) = execute(step, process, files) {
            cleaned = cleaned.and(Err(RepoError::from(e)));
        }
    }
    let value = result?;
    cleaned.map(|()| value)
}

/// `stage`: decides whether to publish, then probes the live repo,
/// mirrors it when there is one, and imports the plain bundle. The
/// secret's availability is [`SECRET_VAR`] being set, or
/// [`SECRET_SET_VAR`] being `true`: `stage` itself never holds it.
///
/// # Errors
///
/// A refused committed key, a failed probe, or a failed program or file
/// step.
pub fn stage_repo(
    process: &mut impl Process,
    files: &mut impl Files,
    env: &impl Env,
    host: &RepoHost,
    paths: &StagePaths,
) -> Result<Outcome, RepoError> {
    let spec = read_spec(process, host)?;
    let key_file = host.key_file(&spec.workspace_root);
    let available =
        set(env, SECRET_VAR).is_some() || env.var(SECRET_SET_VAR).is_some_and(|v| v == "true");
    let mut touched = false;
    let decision = decide(
        host.channel,
        || committed_key(process, files, &paths.work, &key_file, &mut touched),
        available.then_some(()),
    );
    let staged = decision.and_then(|decision| match decision {
        Decision::Skip(skip) => Ok(Outcome::Skipped(skip)),
        Decision::Publish { .. } => {
            let live = probe::live_from_probe(&host.url, process.run(&probe::probe(&host.url)))?;
            run_steps(&stage_steps(host, paths, &key_file, live), process, files)?;
            Ok(Outcome::Staged {
                url: host.url.clone(),
                live,
            })
        }
    });
    if touched {
        finish(staged, &cleanup(&paths.work, false), process, files)
    } else {
        staged
    }
}

/// `sign`: decides again, imports the secret, checks it is the committed
/// key's, commits the staged build onto the channel's branch, signs,
/// prunes and makes deltas, checks the tree fits Pages, then writes the
/// `.flatpakref` and the repo-following bundle.
///
/// # Errors
///
/// A refused committed key or secret, a secret that isn't the committed
/// key's, a tree too large for Pages, or a failed program or file step.
pub fn sign_repo(
    process: &mut impl Process,
    files: &mut impl Files,
    env: &impl Env,
    host: &RepoHost,
    paths: &SignPaths,
) -> Result<Outcome, RepoError> {
    let spec = read_spec(process, host)?;
    let build = build_id::resolve(env, process, &spec.workspace_root).map_err(BundleError::from)?;
    let key_file = host.key_file(&spec.workspace_root);
    let mut touched = false;
    let decision = decide(
        host.channel,
        || committed_key(process, files, &paths.work, &key_file, &mut touched),
        set(env, SECRET_VAR),
    );
    let signed = decision.and_then(|decision| match decision {
        Decision::Skip(skip) => Ok(Outcome::Skipped(skip)),
        Decision::Publish {
            channel,
            key,
            secret,
        } => publish(
            process, files, &spec, host, paths, &key_file, &build, channel, &key, &secret,
        ),
    });
    if touched {
        finish(signed, &cleanup(&paths.work, true), process, files)
    } else {
        signed
    }
}

/// `sign`'s work once it has decided to publish.
#[allow(clippy::too_many_arguments)]
fn publish(
    process: &mut impl Process,
    files: &mut impl Files,
    spec: &BundleSpec,
    host: &RepoHost,
    paths: &SignPaths,
    key_file: &Path,
    build: &BuildId,
    channel: Channel,
    key: &CommittedKey,
    secret: &OsStr,
) -> Result<Outcome, RepoError> {
    let secret = key::decode_secret(secret).map_err(RepoError::Secret)?;
    run_steps(&import_steps(&paths.work, secret), process, files)?;
    let listing = bundle::run(process, &key::list_secret(&gnupg(&paths.work)))?;
    let fingerprint = key::fingerprint(&String::from_utf8_lossy(&listing), Primary::Secret)
        .map_err(RepoError::Secret)?;
    if !fingerprint.eq_ignore_ascii_case(&key.fingerprint) {
        return Err(RepoError::Mismatch {
            committed: key.fingerprint.clone(),
            secret: fingerprint,
        });
    }
    let repo = repo(&paths.pages);
    run_steps(
        &[
            Step::Run(commit_from(
                spec,
                channel,
                &paths.work,
                &paths.pages,
                &fingerprint,
                build,
            )),
            Step::Run(update_repo(&paths.work, &paths.pages, &fingerprint)),
            Step::RemoveDir(repo.join("tmp")),
        ],
        process,
        files,
    )?;
    let tree = bundle::run(process, &probe::listing(&paths.pages))?;
    probe::check_size(&probe::parse_listing(&tree)?)?;
    let flatpakref = paths.pages.join(flatpakref_name(channel));
    let bundle = paths.dist.join(ASSET);
    run_steps(
        &[
            Step::Write {
                path: flatpakref.clone(),
                bytes: flatpakref::render(spec, channel, &host.url, key).into_bytes(),
            },
            Step::CreateDir(paths.dist.clone()),
            Step::Run(build_bundle(
                spec,
                channel,
                &host.url,
                key_file,
                &paths.pages,
                &paths.dist,
            )),
        ],
        process,
        files,
    )?;
    Ok(Outcome::Published { flatpakref, bundle })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::rc::Rc;

    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;

    use super::*;
    use crate::linux::flatpak::repo::key::KeyError;
    use crate::linux::flatpak::repo::key::tests::{FPR, OTHER_FPR, PUB_LISTING, sec_listing};
    use crate::linux::flatpak::repo::probe::SizeError;
    use crate::linux::flatpak::repo::{MAX_FILE, public_key};
    use crate::ports::RunError;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{BUILD, Event, FakeEnv, FakeFiles, Log, Recorder, build, starts};

    const URL: &str = "https://example.com/ev-nova/flatpak/repo/";
    /// The binary key `gpg --dearmor` prints.
    const BINARY: &[u8] = &[0x98, 0x33, 0x04, 0x65, 0xff];
    /// The secret key's bytes, which no other value holds.
    const SECRET: &[u8] = b"\x95\x01secret-key-bytes";

    fn secret_text() -> String {
        STANDARD.encode(SECRET)
    }

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn host() -> RepoHost {
        RepoHost {
            cargo: "/rust/bin/cargo".into(),
            channel: Some(Channel::Nightly),
            url: URL.into(),
            key: None,
        }
    }

    fn base() -> PathBuf {
        Path::new("/t").join("run")
    }

    fn stage_paths() -> StagePaths {
        StagePaths {
            bundle_dir: base().join("plain"),
            work: base().join("fr-work"),
            pages: base().join("fr-pages"),
        }
    }

    fn sign_paths() -> SignPaths {
        SignPaths {
            work: base().join("fr-work"),
            pages: base().join("fr-pages"),
            dist: base().join("fr-dist"),
        }
    }

    /// The committed key's path, as the spec's workspace root spells it.
    fn key_file() -> PathBuf {
        public_key(&spec().workspace_root)
    }

    fn committed() -> CommittedKey {
        CommittedKey {
            fingerprint: FPR.into(),
            binary: BINARY.to_vec(),
        }
    }

    fn env(vars: &[(&'static str, &str)]) -> FakeEnv {
        let mut map = HashMap::from([(build_id::VAR, OsString::from(BUILD))]);
        map.extend(vars.iter().map(|(k, v)| (*k, OsString::from(v))));
        FakeEnv(map)
    }

    /// gpg's listings, `curl`'s answer `live`, and the secret listing's
    /// fingerprint `secret_fpr`; every other run succeeds.
    fn answers(
        live: &'static str,
        secret_fpr: &'static str,
    ) -> impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + Clone + 'static {
        move |i| {
            let has = |word: &str| i.args.iter().any(|a| a == word);
            if i.program == "gpg" && has("show-only") {
                Some(Ok(PUB_LISTING.as_bytes().to_vec()))
            } else if i.program == "gpg" && has("--dearmor") {
                Some(Ok(BINARY.to_vec()))
            } else if i.program == "gpg" && has("--list-secret-keys") {
                Some(Ok(sec_listing(secret_fpr).into_bytes()))
            } else if i.program == "curl" {
                Some(Ok(live.as_bytes().to_vec()))
            } else {
                None
            }
        }
    }

    struct Ran {
        result: Result<Outcome, RepoError>,
        events: Vec<Event>,
    }

    impl Ran {
        fn runs(&self) -> Vec<Invocation> {
            self.events
                .iter()
                .filter_map(|e| match e {
                    Event::Run(i) => Some(i.clone()),
                    _ => None,
                })
                .collect()
        }

        fn ran(&self, program: &str, first: &str) -> bool {
            self.runs().iter().any(|i| starts(i, program, &[first]))
        }

        fn message(&self) -> String {
            self.result.as_ref().expect_err("fails").to_string()
        }
    }

    type Respond = Box<dyn Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>>>;

    struct Doubles {
        respond: Respond,
        fails: Box<dyn Fn(&Event) -> bool>,
        absent: Box<dyn Fn(&Path) -> bool>,
    }

    fn doubles(
        respond: impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static,
    ) -> Doubles {
        Doubles {
            respond: Box::new(respond),
            fails: Box::new(|_| false),
            absent: Box::new(|_| false),
        }
    }

    fn run_with(
        doubles: Doubles,
        go: impl FnOnce(&mut Recorder, &mut FakeFiles) -> Result<Outcome, RepoError>,
    ) -> Ran {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: doubles.respond,
        };
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: doubles.fails,
            absent: doubles.absent,
        };
        let result = go(&mut process, &mut files);
        let events = log.borrow().clone();
        Ran { result, events }
    }

    fn stage(doubles: Doubles, env: &FakeEnv, host: &RepoHost) -> Ran {
        run_with(doubles, |p, f| stage_repo(p, f, env, host, &stage_paths()))
    }

    fn sign(doubles: Doubles, env: &FakeEnv, host: &RepoHost) -> Ran {
        run_with(doubles, |p, f| sign_repo(p, f, env, host, &sign_paths()))
    }

    fn stage_env() -> FakeEnv {
        env(&[(SECRET_SET_VAR, "true")])
    }

    fn sign_env() -> FakeEnv {
        env(&[(SECRET_VAR, &secret_text())])
    }

    /// `ostree --repo=<repo>` with these words.
    fn ostree_at(repo: &Path, words: &[&str]) -> Invocation {
        let mut flag = OsString::from("--repo=");
        flag.push(repo);
        Invocation::new("ostree").arg(flag).args(words)
    }

    /// `--<name>=<path>`.
    fn flag(name: &str, path: &Path) -> OsString {
        let mut flag = OsString::from(format!("--{name}="));
        flag.push(path);
        flag
    }

    fn gpg_at(home: &Path, words: &[&str]) -> Invocation {
        Invocation::new("gpg")
            .arg("--homedir")
            .arg(home)
            .args(["--batch", "--no-permission-warning"])
            .args(words)
    }

    /// The events reading the committed key, from making the gpg home on.
    fn key_events(work: &Path) -> Vec<Event> {
        let home = work.join("gnupg");
        vec![
            Event::RemoveDir(home.clone()),
            Event::CreateDir(home.clone()),
            Event::Run(
                gpg_at(
                    &home,
                    &["--with-colons", "--import-options", "show-only", "--import"],
                )
                .arg(key_file())
                .captured(),
            ),
            Event::Run(
                gpg_at(&home, &["--output", "-", "--dearmor"])
                    .arg(key_file())
                    .captured(),
            ),
        ]
    }

    fn cleanup_events(work: &Path, signed: bool) -> Vec<Event> {
        let home = work.join("gnupg");
        let mut events = vec![
            Event::Run(
                Invocation::new("gpgconf")
                    .arg("--homedir")
                    .arg(&home)
                    .args(["--kill", "gpg-agent"]),
            ),
            Event::RemoveDir(home),
        ];
        if signed {
            events.push(Event::RemoveDir(work.join("secret")));
        }
        events
    }

    fn metadata_event() -> Event {
        Event::Run(metadata(&OsString::from("/rust/bin/cargo")))
    }

    // The decision table: one test per row.

    #[test]
    fn without_a_channel_nothing_is_read_and_it_skips() {
        let decided = decide(None, || panic!("the key is not read"), Some(()));
        assert_eq!(decided.expect("decided"), Decision::Skip(Skip::NoChannel));
    }

    #[test]
    fn without_a_committed_key_it_skips_whatever_the_secret() {
        for secret in [None, Some(())] {
            let decided = decide(Some(Channel::Nightly), || Ok(None), secret);
            assert_eq!(decided.expect("decided"), Decision::Skip(Skip::NoKey));
        }
    }

    #[test]
    fn a_refused_committed_key_is_an_error_whatever_the_secret() {
        for secret in [None, Some(())] {
            let decided = decide(
                Some(Channel::Nightly),
                || {
                    Err(RepoError::Key {
                        path: key_file(),
                        source: KeyError::Secret,
                    })
                },
                secret,
            );
            assert!(
                matches!(
                    decided,
                    Err(RepoError::Key {
                        source: KeyError::Secret,
                        ..
                    })
                ),
                "{decided:?}"
            );
        }
    }

    #[test]
    fn with_the_key_but_no_secret_it_skips() {
        let decided = decide::<()>(Some(Channel::Nightly), || Ok(Some(committed())), None);
        assert_eq!(decided.expect("decided"), Decision::Skip(Skip::NoSecret));
    }

    #[test]
    fn with_the_channel_the_key_and_the_secret_it_publishes() {
        let secret = SecretBytes(SECRET.to_vec());
        let decided = decide(
            Some(Channel::Nightly),
            || Ok(Some(committed())),
            Some(secret.clone()),
        )
        .expect("decided");
        assert_eq!(
            decided,
            Decision::Publish {
                channel: Channel::Nightly,
                key: committed(),
                secret,
            }
        );
        let shown = format!("{decided:?}");
        assert!(!shown.contains("secret-key-bytes"), "{shown}");
    }

    #[test]
    fn each_skip_says_why() {
        assert_eq!(
            Skip::NoChannel.message(),
            "NOVA_CHANNEL is unset (a development build)"
        );
        assert!(
            Skip::NoKey
                .message()
                .contains("packaging/linux/flatpak/repo-key.asc")
        );
        let no_secret = Skip::NoSecret.message();
        assert!(
            no_secret.contains("FLATPAK_SIGNING_KEY is unset"),
            "{no_secret}"
        );
        assert!(no_secret.contains("update-signing"), "{no_secret}");
    }

    // stage.

    #[test]
    fn a_first_publish_inits_an_empty_repo_and_imports_the_bundle() {
        let paths = stage_paths();
        let ran = stage(doubles(answers("404", FPR)), &stage_env(), &host());
        assert_eq!(
            ran.result.as_ref().expect("staged"),
            &Outcome::Staged {
                url: URL.into(),
                live: Live::Absent
            }
        );
        let repo = paths.pages.join("repo");
        let src = paths.work.join("src");
        let mut expected = vec![metadata_event()];
        expected.extend(key_events(&paths.work));
        expected.extend([
            Event::Run(probe::probe(URL)),
            Event::RemoveDir(repo.clone()),
            Event::CreateDir(paths.pages.clone()),
            Event::Run(ostree_at(&repo, &["init", "--mode=archive"])),
            Event::RemoveDir(src.clone()),
            Event::Run(ostree_at(&src, &["init", "--mode=archive"])),
            Event::Run(
                Invocation::new("flatpak")
                    .args(["build-import-bundle", "--no-update-summary"])
                    .arg(&src)
                    .arg(
                        paths
                            .bundle_dir
                            .join("ev-nova-x86_64-unknown-linux-gnu.flatpak"),
                    ),
            ),
        ]);
        expected.extend(cleanup_events(&paths.work, false));
        assert_eq!(ran.events, expected);
        assert_eq!(
            probe::probe(URL).args.last(),
            Some(&OsString::from(format!("{URL}config")))
        );
    }

    #[test]
    fn a_live_repo_is_mirrored_gpg_verified_to_the_kept_depth_then_its_remote_is_dropped() {
        let paths = stage_paths();
        let ran = stage(doubles(answers("200", FPR)), &stage_env(), &host());
        assert_eq!(
            ran.result.as_ref().expect("staged"),
            &Outcome::Staged {
                url: URL.into(),
                live: Live::Present
            }
        );
        let repo = paths.pages.join("repo");
        let runs = ran.runs();
        let init = runs
            .iter()
            .position(|i| *i == ostree_at(&repo, &["init", "--mode=archive"]))
            .expect("the pages repo is made");
        assert_eq!(
            runs[init + 1..init + 4],
            [
                ostree_at(&repo, &["remote", "add"])
                    .arg(flag("gpg-import", &key_file()))
                    .args(["--set=gpg-verify-summary=true", "live", URL]),
                ostree_at(&repo, &["pull", "--mirror", "--depth=2", "live"]),
                ostree_at(&repo, &["remote", "delete", "live"]),
            ]
        );
        assert!(
            ran.ran("flatpak", "build-import-bundle"),
            "the bundle is still imported"
        );
    }

    #[test]
    fn a_failed_probe_stages_nothing_more() {
        for answer in [Err(RunError::Failed(Some(6))), Ok(b"503".to_vec())] {
            let answer = Rc::new(answer.map_err(|_| ()));
            let respond = {
                let answer = Rc::clone(&answer);
                let base = answers("200", FPR);
                move |i: &Invocation| {
                    if i.program == "curl" {
                        Some(match answer.as_ref() {
                            Ok(bytes) => Ok(bytes.clone()),
                            Err(()) => Err(RunError::Failed(Some(6))),
                        })
                    } else {
                        base(i)
                    }
                }
            };
            let ran = stage(doubles(respond), &stage_env(), &host());
            assert!(
                matches!(ran.result, Err(RepoError::Probe(_))),
                "{:?}",
                ran.result
            );
            assert!(
                !ran.runs()
                    .iter()
                    .any(|i| i.program == "ostree" || i.program == "flatpak"),
                "nothing is staged after a failed probe"
            );
            assert!(
                !ran.events
                    .iter()
                    .any(|e| matches!(e, Event::CreateDir(d) if *d == stage_paths().pages)),
                "no pages tree"
            );
            // The gpg home is still cleaned up.
            assert_eq!(
                ran.events[ran.events.len() - 2..],
                cleanup_events(&stage_paths().work, false)[..]
            );
        }
    }

    #[test]
    fn without_a_key_or_channel_it_stages_nothing_and_says_why() {
        let no_channel = stage(
            doubles(answers("200", FPR)),
            &stage_env(),
            &RepoHost {
                channel: None,
                ..host()
            },
        );
        assert_eq!(
            no_channel.result.as_ref().expect("skipped"),
            &Outcome::Skipped(Skip::NoChannel)
        );
        assert_eq!(no_channel.events, [metadata_event()]);

        let mut absent = doubles(answers("200", FPR));
        absent.absent = Box::new(|p| p == key_file());
        let no_key = stage(absent, &stage_env(), &host());
        assert_eq!(
            no_key.result.as_ref().expect("skipped"),
            &Outcome::Skipped(Skip::NoKey)
        );
        assert_eq!(no_key.events, [metadata_event()]);
    }

    #[test]
    fn without_the_secret_it_checks_the_key_then_stages_nothing() {
        for unavailable in [
            env(&[]),
            env(&[(SECRET_SET_VAR, "false")]),
            env(&[(SECRET_VAR, "")]),
        ] {
            let ran = stage(doubles(answers("200", FPR)), &unavailable, &host());
            assert_eq!(
                ran.result.as_ref().expect("skipped"),
                &Outcome::Skipped(Skip::NoSecret)
            );
            let mut expected = vec![metadata_event()];
            expected.extend(key_events(&stage_paths().work));
            expected.extend(cleanup_events(&stage_paths().work, false));
            assert_eq!(ran.events, expected);
        }
        let set = stage(
            doubles(answers("404", FPR)),
            &env(&[(SECRET_VAR, &secret_text())]),
            &host(),
        );
        assert!(
            matches!(set.result, Ok(Outcome::Staged { .. })),
            "a set secret counts as available: {:?}",
            set.result
        );
    }

    #[test]
    fn a_committed_secret_key_fails_stage_loudly() {
        let respond = {
            let base = answers("200", FPR);
            move |i: &Invocation| {
                if i.args.iter().any(|a| a == "show-only") {
                    Some(Ok(sec_listing(FPR).into_bytes()))
                } else {
                    base(i)
                }
            }
        };
        for env in [stage_env(), env(&[])] {
            let ran = stage(doubles(respond.clone()), &env, &host());
            assert!(
                matches!(
                    ran.result,
                    Err(RepoError::Key {
                        source: KeyError::Secret,
                        ..
                    })
                ),
                "{:?}",
                ran.result
            );
            let message = ran.message();
            assert!(
                message.contains("repo-key.asc: it holds a secret key"),
                "{message}"
            );
            assert!(!ran.ran("curl", "--silent"));
        }
    }

    #[test]
    fn each_failing_stage_program_stops_it_naming_its_command_line() {
        let paths = stage_paths();
        let repo = paths.pages.join("repo");
        let src = paths.work.join("src");
        let cases: Vec<(Invocation, &str)> = vec![
            (
                gpg_at(
                    &paths.work.join("gnupg"),
                    &["--with-colons", "--import-options", "show-only", "--import"],
                )
                .arg(key_file())
                .captured(),
                "`gpg --homedir",
            ),
            (
                ostree_at(&repo, &["init", "--mode=archive"]),
                "init --mode=archive`",
            ),
            (
                ostree_at(&repo, &["pull", "--mirror", "--depth=2", "live"]),
                "pull --mirror --depth=2 live`",
            ),
            (
                ostree_at(&src, &["init", "--mode=archive"]),
                "init --mode=archive`",
            ),
            (
                Invocation::new("flatpak")
                    .args(["build-import-bundle", "--no-update-summary"])
                    .arg(&src)
                    .arg(paths.bundle_dir.join(ASSET)),
                "`flatpak build-import-bundle --no-update-summary",
            ),
        ];
        for (failing, shown) in cases {
            let target = failing.clone();
            let base = answers("200", FPR);
            let ran = stage(
                doubles(move |i| {
                    if *i == target {
                        Some(Err(RunError::Failed(Some(1))))
                    } else {
                        base(i)
                    }
                }),
                &stage_env(),
                &host(),
            );
            let message = ran.message();
            assert!(message.contains(shown), "{shown}: {message}");
            assert!(message.contains("exited with code 1"), "{message}");
            let runs = ran.runs();
            let at = runs.iter().position(|i| *i == failing).expect("it ran");
            assert_eq!(
                runs[at + 1..],
                [key::kill_agent(&paths.work.join("gnupg"))],
                "{shown}: only the cleanup runs after it"
            );
        }
    }

    // sign.

    fn sign_events() -> Vec<Event> {
        let spec = spec();
        let paths = sign_paths();
        let home = paths.work.join("gnupg");
        let secret = paths.work.join("secret");
        let repo = paths.pages.join("repo");
        let mut expected = vec![metadata_event()];
        expected.extend(key_events(&paths.work));
        expected.extend([
            Event::RemoveDir(secret.clone()),
            Event::CreateDir(secret.clone()),
            Event::WriteSecret(secret.join("key.gpg"), SECRET.to_vec()),
            Event::Run(gpg_at(&home, &["--import"]).arg(secret.join("key.gpg"))),
            Event::RemoveDir(secret.clone()),
            Event::Run(gpg_at(&home, &["--with-colons", "--list-secret-keys"]).captured()),
            Event::Run(
                Invocation::new("flatpak")
                    .arg("build-commit-from")
                    .arg(flag("src-repo", &paths.work.join("src")))
                    .arg("--src-ref=app/io.example.nova/x86_64/nightly")
                    .arg(format!("--gpg-sign={FPR}"))
                    .arg(flag("gpg-homedir", &home))
                    .arg(format!("--subject=EV Nova nightly build {BUILD}"))
                    .arg("--no-update-summary")
                    .arg(&repo)
                    .arg("app/io.example.nova/x86_64/nightly"),
            ),
            Event::Run(
                Invocation::new("flatpak")
                    .arg("build-update-repo")
                    .arg(format!("--gpg-sign={FPR}"))
                    .arg(flag("gpg-homedir", &home))
                    .args([
                        "--generate-static-deltas",
                        "--prune",
                        "--prune-depth=2",
                        "--title=EV Nova",
                    ])
                    .arg(&repo),
            ),
            Event::RemoveDir(repo.join("tmp")),
            Event::Run(probe::listing(&paths.pages)),
            Event::Write(
                paths.pages.join("ev-nova-nightly.flatpakref"),
                flatpakref::render(&spec, Channel::Nightly, URL, &committed()).into_bytes(),
            ),
            Event::CreateDir(paths.dist.clone()),
            Event::Run(
                Invocation::new("flatpak")
                    .args([
                        "build-bundle",
                        "--runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo",
                        &format!("--repo-url={URL}"),
                    ])
                    .arg(flag("gpg-keys", &key_file()))
                    .arg(&repo)
                    .arg(paths.dist.join("ev-nova-x86_64-unknown-linux-gnu.flatpak"))
                    .args(["io.example.nova", "nightly"]),
            ),
        ]);
        expected.extend(cleanup_events(&paths.work, true));
        expected
    }

    #[test]
    fn signs_onto_the_channel_branch_prunes_deltas_and_writes_the_ref_and_bundle() {
        let paths = sign_paths();
        let ran = sign(doubles(answers("200", FPR)), &sign_env(), &host());
        assert_eq!(
            ran.result.as_ref().expect("published"),
            &Outcome::Published {
                flatpakref: paths.pages.join("ev-nova-nightly.flatpakref"),
                bundle: paths.dist.join(ASSET),
            }
        );
        assert_eq!(ran.events, sign_events());
    }

    #[test]
    fn the_subject_names_the_build_from_the_environment() {
        let ran = sign(
            doubles(answers("200", FPR)),
            &env(&[
                (SECRET_VAR, &secret_text()),
                (build_id::VAR, "20270101000000"),
            ]),
            &host(),
        );
        let commit = ran
            .runs()
            .into_iter()
            .find(|i| starts(i, "flatpak", &["build-commit-from"]))
            .expect("committed");
        assert!(
            commit
                .args
                .contains(&"--subject=EV Nova nightly build 20270101000000".into()),
            "{commit}"
        );
        assert_ne!(build().as_str(), "20270101000000");
    }

    #[test]
    fn the_secret_is_written_owner_only_and_removed_before_any_flatpak_tool_runs() {
        let ran = sign(doubles(answers("200", FPR)), &sign_env(), &host());
        let secret = sign_paths().work.join("secret");
        let written = ran
            .events
            .iter()
            .position(
                |e| matches!(e, Event::WriteSecret(p, b) if p.starts_with(&secret) && b == SECRET),
            )
            .expect("written owner-only");
        assert!(
            !ran.events
                .iter()
                .any(|e| matches!(e, Event::Write(_, b) if b == SECRET)),
            "never written as a plain file"
        );
        let removed = ran
            .events
            .iter()
            .enumerate()
            .skip(written)
            .find(|(_, e)| **e == Event::RemoveDir(secret.clone()))
            .map(|(at, _)| at)
            .expect("removed");
        let first_flatpak = ran
            .events
            .iter()
            .position(|e| matches!(e, Event::Run(i) if i.program == "flatpak"))
            .expect("a flatpak tool ran");
        assert!(
            written < removed && removed < first_flatpak,
            "{:#?}",
            ran.events
        );
    }

    #[test]
    fn the_secret_is_never_in_an_argument_or_a_message() {
        let text = secret_text();
        let mismatched = sign(doubles(answers("200", OTHER_FPR)), &sign_env(), &host());
        let bad = sign(
            doubles(answers("200", FPR)),
            &env(&[(SECRET_VAR, "!!not-base64!!")]),
            &host(),
        );
        let ok = sign(doubles(answers("200", FPR)), &sign_env(), &host());
        for ran in [&mismatched, &ok] {
            for invocation in ran.runs() {
                for arg in &invocation.args {
                    let arg = arg.to_string_lossy();
                    assert!(!arg.contains(&text), "{invocation}");
                    assert!(!arg.contains("secret-key-bytes"), "{invocation}");
                }
                assert!(invocation.env.is_empty(), "{invocation}");
            }
        }
        for message in [mismatched.message(), bad.message()] {
            assert!(!message.contains(&text), "{message}");
            assert!(!message.contains("not-base64"), "{message}");
        }
        assert_eq!(
            bad.message(),
            "FLATPAK_SIGNING_KEY: it is not standard base64"
        );
        assert!(
            !bad.events
                .iter()
                .any(|e| matches!(e, Event::WriteSecret(..))),
            "nothing is written from a bad secret"
        );
    }

    #[test]
    fn a_mismatched_secret_names_both_fingerprints_and_commits_nothing() {
        let ran = sign(doubles(answers("200", OTHER_FPR)), &sign_env(), &host());
        let Err(RepoError::Mismatch { committed, secret }) = &ran.result else {
            panic!("{:?}", ran.result);
        };
        assert_eq!((committed.as_str(), secret.as_str()), (FPR, OTHER_FPR));
        let message = ran.message();
        assert!(
            message.contains(FPR) && message.contains(OTHER_FPR),
            "{message}"
        );
        assert!(!ran.runs().iter().any(|i| i.program == "flatpak"));
        assert_eq!(
            ran.events[ran.events.len() - 3..],
            cleanup_events(&sign_paths().work, true)[..]
        );
    }

    #[test]
    fn a_lower_case_fingerprint_from_the_secret_is_the_same_key() {
        let lower: &'static str = Box::leak(FPR.to_lowercase().into_boxed_str());
        let ran = sign(doubles(answers("200", lower)), &sign_env(), &host());
        assert!(
            matches!(ran.result, Ok(Outcome::Published { .. })),
            "{:?}",
            ran.result
        );
    }

    #[test]
    fn without_the_secret_it_writes_neither_pages_nor_dist() {
        for unset in [env(&[]), env(&[(SECRET_VAR, "")])] {
            let ran = sign(doubles(answers("200", FPR)), &unset, &host());
            assert_eq!(
                ran.result.as_ref().expect("skipped"),
                &Outcome::Skipped(Skip::NoSecret)
            );
            let paths = sign_paths();
            assert!(
                !ran.events.iter().any(|e| match e {
                    Event::Write(p, _) | Event::CreateDir(p) | Event::WriteSecret(p, _) =>
                        p.starts_with(&paths.pages) || p.starts_with(&paths.dist),
                    _ => false,
                }),
                "{:#?}",
                ran.events
            );
            assert!(!ran.runs().iter().any(|i| i.program == "flatpak"));
        }
        let no_channel = sign(
            doubles(answers("200", FPR)),
            &sign_env(),
            &RepoHost {
                channel: None,
                ..host()
            },
        );
        assert_eq!(
            no_channel.result.as_ref().expect("skipped"),
            &Outcome::Skipped(Skip::NoChannel)
        );
        assert_eq!(no_channel.events, [metadata_event()]);
    }

    #[test]
    fn an_oversized_tree_fails_after_update_repo() {
        let big = format!("{}\trepo/deltas/xx/big\n12\trepo/config\n", MAX_FILE + 1);
        let respond = {
            let base = answers("200", FPR);
            move |i: &Invocation| {
                if i.program == "find" {
                    Some(Ok(big.clone().into_bytes()))
                } else {
                    base(i)
                }
            }
        };
        let ran = sign(doubles(respond), &sign_env(), &host());
        assert!(
            matches!(ran.result, Err(RepoError::Size(SizeError::TooLarge { .. }))),
            "{:?}",
            ran.result
        );
        assert!(
            ran.message().contains("repo/deltas/xx/big"),
            "{}",
            ran.message()
        );
        assert!(ran.ran("flatpak", "build-update-repo"));
        assert!(!ran.ran("flatpak", "build-bundle"));
        assert!(
            !ran.events.iter().any(|e| matches!(e, Event::Write(..))),
            "no .flatpakref"
        );
    }

    #[test]
    fn each_failing_sign_program_stops_it_naming_its_command_line() {
        for (program, first, shown) in [
            ("gpg", "--homedir", "`gpg --homedir"),
            (
                "flatpak",
                "build-commit-from",
                "`flatpak build-commit-from --src-repo=",
            ),
            (
                "flatpak",
                "build-update-repo",
                "`flatpak build-update-repo --gpg-sign=",
            ),
            ("find", "", "`find "),
            (
                "flatpak",
                "build-bundle",
                "`flatpak build-bundle --runtime-repo=",
            ),
        ] {
            let base = answers("200", FPR);
            let ran = sign(
                doubles(move |i| {
                    let picked = i.program == program
                        && (first.is_empty() || i.args.first().is_some_and(|a| a == first));
                    if picked {
                        Some(Err(RunError::Failed(Some(1))))
                    } else {
                        base(i)
                    }
                }),
                &sign_env(),
                &host(),
            );
            let message = ran.message();
            assert!(message.contains(shown), "{shown}: {message}");
            assert!(message.contains("exited with code 1"), "{message}");
            assert_eq!(
                ran.events[ran.events.len() - 3..],
                cleanup_events(&sign_paths().work, true)[..],
                "{shown}: the cleanup still runs"
            );
        }
    }

    #[test]
    fn a_failing_import_leaves_no_secret_behind() {
        let base = answers("200", FPR);
        let ran = sign(
            doubles(move |i| {
                if i.program == "gpg"
                    && i.args
                        .last()
                        .is_some_and(|a| Path::new(a).ends_with("key.gpg"))
                {
                    Some(Err(RunError::Failed(Some(2))))
                } else {
                    base(i)
                }
            }),
            &sign_env(),
            &host(),
        );
        assert!(ran.result.is_err());
        assert_eq!(
            ran.events.last(),
            Some(&Event::RemoveDir(sign_paths().work.join("secret")))
        );
        assert!(!ran.runs().iter().any(|i| i.program == "flatpak"));
    }

    #[test]
    fn a_failing_cleanup_is_the_error_when_the_steps_succeeded() {
        let mut failing = doubles(answers("200", FPR));
        let home = sign_paths().work.join("gnupg");
        let removals = Rc::new(std::cell::Cell::new(0));
        let seen = Rc::clone(&removals);
        failing.fails = Box::new(move |e| {
            if *e == Event::RemoveDir(home.clone()) {
                seen.set(seen.get() + 1);
                seen.get() == 2
            } else {
                false
            }
        });
        let ran = sign(failing, &sign_env(), &host());
        let message = ran.message();
        assert!(message.starts_with("could not remove"), "{message}");
        assert!(message.contains("gnupg"), "{message}");
        assert_eq!(
            ran.events.last(),
            Some(&Event::RemoveDir(sign_paths().work.join("secret"))),
            "every cleanup step runs"
        );
    }

    #[test]
    fn a_failing_step_s_error_wins_over_a_failing_cleanup() {
        let mut failing = doubles(answers("503", FPR));
        let home = stage_paths().work.join("gnupg");
        let removals = Rc::new(std::cell::Cell::new(0));
        let seen = Rc::clone(&removals);
        failing.fails = Box::new(move |e| {
            if *e == Event::RemoveDir(home.clone()) {
                seen.set(seen.get() + 1);
                seen.get() == 2
            } else {
                false
            }
        });
        let ran = stage(failing, &stage_env(), &host());
        assert!(
            matches!(ran.result, Err(RepoError::Probe(_))),
            "{:?}",
            ran.result
        );
    }
}
