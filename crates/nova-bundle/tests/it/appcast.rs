//! `nova-appcast`'s publish over real files in a temporary directory each
//! test owns, its re-run and rollback refusals and their exit statuses,
//! the binary itself, and its `check-promotion` over a git repository and
//! release files the test makes, with every variable it reads (and every
//! git setting) set on its own child process only. The one seed is a
//! fixed test value, never a real key.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, SigningKey, Verifier as _, VerifyingKey};
use nova_bundle::appcast::host::{ALREADY_PUBLISHED, FAILURE_STATUS, exit_status};
use nova_bundle::appcast::publish::{
    AppcastError, FeedSigning, PublishPaths, Published, SEED_VAR, publish,
};
use nova_bundle::appcast::{Channel, Platform, SPARKLE_NS, update_asset};
use nova_bundle::build_id::BuildId;
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::{Ed25519Signer, StdFiles};
use serde_json::json;

const SEED: [u8; 32] = [0x42; 32];

fn seed_base64() -> String {
    STANDARD.encode(SEED)
}

fn verifying_key() -> VerifyingKey {
    VerifyingKey::from(&SigningKey::from_bytes(&SEED))
}

fn signer() -> Ed25519Signer {
    Ed25519Signer::from_base64_seed(&seed_base64()).expect("a seed")
}

fn id(text: &str) -> BuildId {
    BuildId::parse(text).expect("an ID")
}

/// A spec read from fixture `cargo metadata` output, committing the test
/// seed's public key.
fn spec(root: &Path) -> BundleSpec {
    let public = STANDARD.encode(verifying_key().to_bytes());
    let metadata = json!({
        "packages": [{
            "name": "nova",
            "version": "0.1.0",
            "license": "GPL-3.0-or-later",
            "metadata": { "bundle": {
                "identifier": "io.example.nova",
                "name": "EV Nova",
                "category": "public.app-category.role-playing-games",
                "summary": "Fly the stars",
                "description": "A space trading and combat game.",
                "homepage": "https://example.com/nova",
                "developer": "the nova authors",
                "update_public_key": public,
            }},
        }],
        "workspace_root": root,
        "target_directory": root.join("target"),
        "version": 1,
    });
    BundleSpec::from_metadata(metadata.to_string().as_bytes()).expect("spec")
}

const DMG: &[u8] = b"\x00a real-ish dmg\xff";
const MSI: &[u8] = b"\x00a real-ish msi, longer\xff";

/// The run's directories under `root`, with both installers in `dist` and
/// the release listing `assets`.
fn layout(root: &Path, assets: &[String]) -> PublishPaths {
    let paths = PublishPaths {
        dist: root.join("dist"),
        assets: root.join("assets.txt"),
        current: root.join("current"),
        feeds: root.join("feeds"),
        prune_list: root.join("prune.txt"),
    };
    fs::create_dir_all(&paths.dist).expect("dist");
    fs::create_dir_all(&paths.current).expect("current");
    fs::write(paths.dist.join(Platform::Macos.stable_asset()), DMG).expect("dmg");
    fs::write(paths.dist.join(Platform::Windows.stable_asset()), MSI).expect("msi");
    let listed = assets.iter().fold(String::new(), |text, a| text + a + "\n");
    fs::write(&paths.assets, listed).expect("assets");
    paths
}

fn publish_real(root: &Path, paths: &PublishPaths, build: &str) -> Result<Published, AppcastError> {
    publish(
        &StdFiles,
        &mut StdFiles,
        &spec(root),
        &id(build),
        Channel::Nightly,
        &FeedSigning::Sign(signer()),
        paths,
    )
}

const BUILD: &str = "20261028123456";

#[test]
fn publish_over_real_files_writes_feeds_whose_signatures_verify() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dropped = "ev-nova-x86_64-unknown-linux-gnu.AppImage".to_owned();
    let paths = layout(dir.path(), std::slice::from_ref(&dropped));
    let published = publish_real(dir.path(), &paths, BUILD).expect("published");
    assert_eq!(
        published.feeds,
        [
            paths.feeds.join("appcast-macos-nightly.xml"),
            paths.feeds.join("appcast-windows-nightly.xml"),
        ]
    );
    for (platform, bytes, feed) in [
        (Platform::Macos, DMG, &published.feeds[0]),
        (Platform::Windows, MSI, &published.feeds[1]),
    ] {
        let copy = paths.dist.join(update_asset(platform, &id(BUILD)));
        assert_eq!(fs::read(&copy).expect("the build-ID copy"), bytes);
        let text = fs::read_to_string(feed).expect("the feed");
        let doc = roxmltree::Document::parse(&text).expect("XML");
        let enclosure = doc
            .descendants()
            .find(|n| n.has_tag_name("enclosure"))
            .expect("an enclosure");
        let url = enclosure.attribute("url").expect("a url");
        assert!(url.ends_with(&update_asset(platform, &id(BUILD))), "{url}");
        let length = bytes.len().to_string();
        assert_eq!(enclosure.attribute("length"), Some(length.as_str()));
        let signature = STANDARD
            .decode(
                enclosure
                    .attribute((SPARKLE_NS, "edSignature"))
                    .expect("signed"),
            )
            .expect("base64");
        let signature = Signature::from_slice(&signature).expect("64 bytes");
        verifying_key()
            .verify(bytes, &signature)
            .expect("the signature verifies");
    }
    assert_eq!(published.prune, std::slice::from_ref(&dropped));
    assert_eq!(
        fs::read_to_string(&paths.prune_list).expect("the prune list"),
        format!("{dropped}\n")
    );
}

/// The release after a publish of `BUILD`: the feeds it wrote are
/// listed and downloaded.
fn after_a_publish(root: &Path) -> PublishPaths {
    let first = layout(root, &[]);
    let published = publish_real(root, &first, BUILD).expect("first publish");
    let mut listed = Vec::new();
    for feed in &published.feeds {
        let name = feed.file_name().expect("a name");
        fs::copy(feed, first.current.join(name)).expect("downloaded");
        listed.push(name.to_string_lossy().into_owned());
    }
    listed.push(Platform::Macos.stable_asset().to_owned());
    listed.push(update_asset(Platform::Macos, &id(BUILD)));
    let paths = layout(root, &listed);
    fs::remove_file(&paths.prune_list).expect("the first run's list");
    paths
}

#[test]
fn a_rerun_of_the_same_build_exits_already_published_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let paths = after_a_publish(dir.path());
    let error = publish_real(dir.path(), &paths, BUILD).expect_err("already published");
    assert!(error.to_string().contains(BUILD), "{error}");
    assert_eq!(exit_status(&error), ALREADY_PUBLISHED);
    assert!(!paths.prune_list.exists());
}

#[test]
fn an_older_build_fails_naming_both_ids() {
    let dir = tempfile::tempdir().expect("temp dir");
    let paths = after_a_publish(dir.path());
    let older = "20261027000000";
    let error = publish_real(dir.path(), &paths, older).expect_err("rollback");
    let text = error.to_string();
    assert!(text.contains(BUILD) && text.contains(older), "{text}");
    assert_eq!(exit_status(&error), FAILURE_STATUS);
    assert!(!paths.prune_list.exists());
}

#[test]
fn a_newer_build_publishes_over_the_last_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let paths = after_a_publish(dir.path());
    let published = publish_real(dir.path(), &paths, "20261029000000").expect("newer");
    assert_eq!(published.feeds.len(), 2);
    assert_eq!(published.prune, Vec::<String>::new());
}

// The binary.

fn nova_appcast(args: &[&std::ffi::OsStr], seed: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nova-appcast"));
    command
        .args(args)
        // In the workspace, so its `cargo metadata` (which only reads)
        // finds it.
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CARGO", env!("CARGO"))
        .env("NOVA_BUILD_ID", BUILD)
        .env("NOVA_CHANNEL", "nightly")
        .env_remove(SEED_VAR);
    if let Some(seed) = seed {
        command.env(SEED_VAR, seed);
    }
    command.output().expect("nova-appcast runs")
}

#[test]
fn the_binary_prints_the_public_key_of_its_seed() {
    let output = nova_appcast(&["public-key".as_ref()], Some(&seed_base64()));
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        format!("{}\n", STANDARD.encode(verifying_key().to_bytes()))
    );
}

#[test]
fn the_binary_without_a_seed_fails() {
    for seed in [None, Some("")] {
        let output = nova_appcast(&["public-key".as_ref()], seed);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        let stderr = String::from_utf8(output.stderr).expect("utf-8");
        assert!(stderr.contains(SEED_VAR), "{stderr}");
    }
}

#[test]
fn the_binary_with_bad_arguments_exits_2_with_the_usage() {
    let output = nova_appcast(&["sign".as_ref()], None);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    assert!(stderr.contains("usage: nova-appcast"), "{stderr}");
}

#[test]
fn the_binary_without_a_key_publishes_no_feeds_warns_and_writes_the_prune_list() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dropped = "ev-nova-x86_64-unknown-linux-gnu.AppImage".to_owned();
    let paths = layout(dir.path(), std::slice::from_ref(&dropped));
    let args = [
        "publish".as_ref(),
        "--dist".as_ref(),
        paths.dist.as_os_str(),
        "--assets".as_ref(),
        paths.assets.as_os_str(),
        "--current".as_ref(),
        paths.current.as_os_str(),
        "--feeds".as_ref(),
        paths.feeds.as_os_str(),
        "--prune-list".as_ref(),
        paths.prune_list.as_os_str(),
    ];
    let output = nova_appcast(&args, None);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    assert!(
        stdout.starts_with("::warning title=Update feeds not published::"),
        "{stdout}"
    );
    assert!(stdout.contains(SEED_VAR), "{stdout}");
    assert!(!paths.feeds.exists() || fs::read_dir(&paths.feeds).expect("feeds").next().is_none());
    assert_eq!(
        fs::read_to_string(&paths.prune_list).expect("the prune list"),
        format!("{dropped}\n")
    );
    let copies: Vec<_> = fs::read_dir(&paths.dist)
        .expect("dist")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(copies.len(), 2, "{copies:?}");
}

// check-promotion, over a git repository and release files in a
// temporary directory each test owns.

/// `command` with every `GIT_*` variable this test process inherited
/// removed from its environment. A git hook runs the tests with
/// `GIT_DIR`, `GIT_INDEX_FILE` and the like naming the project's own
/// repository; a child git that inherited them would act on it instead
/// of the test's.
fn without_git_env(command: &mut Command) -> &mut Command {
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
}

/// `git` in `dir`'s repository, with every setting on the child only: no
/// inherited `GIT_*` variable, no system or global config, and fixed
/// names and times.
fn git(dir: &Path, args: &[&str], secs: i64) -> String {
    let config = dir.join("gitconfig");
    if !config.exists() {
        fs::write(&config, "").expect("an empty git config");
    }
    let date = format!("@{secs} +0000");
    let output = without_git_env(&mut Command::new("git"))
        .arg("-C")
        .arg(dir.join("main"))
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .trim()
        .to_owned()
}

/// `20261027000000` and `20261028123456` as Unix times.
const MID_SECS: i64 = 1_793_059_200;
const BUILD_SECS: i64 = 1_793_190_896;
const MID: &str = "20261027000000";

/// A `main` with one commit at each of `times`, oldest first, returning
/// each commit's SHA.
fn main_repo(dir: &Path, times: &[i64]) -> Vec<String> {
    fs::create_dir_all(dir.join("main")).expect("the repo dir");
    git(dir, &["init", "-q", "-b", "main"], 0);
    times
        .iter()
        .map(|&secs| {
            git(
                dir,
                &[
                    "commit",
                    "-q",
                    "--allow-empty",
                    "--no-gpg-sign",
                    "--no-verify",
                    "-m",
                    "c",
                ],
                secs,
            );
            git(dir, &["rev-parse", "HEAD"], secs)
        })
        .collect()
}

/// The nightly release holding both update files of each of `builds`,
/// and no beta release yet: the four `check-promotion` arguments after
/// `--build <ID>`.
fn releases(dir: &Path, builds: &[&str]) -> Vec<std::ffi::OsString> {
    let mut nightly = vec![Platform::Macos.stable_asset().to_owned()];
    for build in builds {
        for platform in Platform::ALL {
            nightly.push(update_asset(platform, &id(build)));
        }
    }
    let nightly_list = dir.join("nightly.txt");
    fs::write(&nightly_list, nightly.join("\n") + "\n").expect("nightly list");
    let beta_list = dir.join("beta.txt");
    fs::write(&beta_list, "").expect("beta list");
    let beta_current = dir.join("beta-current");
    fs::create_dir_all(&beta_current).expect("beta current");
    vec![
        "--repo".into(),
        dir.join("main").into(),
        "--nightly-assets".into(),
        nightly_list.into(),
        "--beta-assets".into(),
        beta_list.into(),
        "--beta-current".into(),
        beta_current.into(),
    ]
}

fn check_promotion(build: &str, rest: &[std::ffi::OsString], git_ref: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nova-appcast"));
    // Its `git log` must read the test's repository, never the project's.
    without_git_env(&mut command)
        .arg("check-promotion")
        .arg("--build")
        .arg(build)
        .args(rest)
        .env("GITHUB_REF", git_ref)
        .env_remove(SEED_VAR);
    command.output().expect("nova-appcast runs")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8")
}

#[test]
fn check_promotion_prints_a_held_newer_build_and_its_commit() {
    assert_eq!(BuildId::from_unix(MID_SECS).expect("an ID"), id(MID));
    assert_eq!(BuildId::from_unix(BUILD_SECS).expect("an ID"), id(BUILD));
    let dir = tempfile::tempdir().expect("temp dir");
    let shas = main_repo(dir.path(), &[MID_SECS, BUILD_SECS, BUILD_SECS + 60]);
    let rest = releases(dir.path(), &[MID, BUILD]);
    let output = check_promotion(BUILD, &rest, "refs/heads/main");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        format!("build={BUILD}\ncommit={}\n", shas[1])
    );
}

#[test]
fn check_promotion_refuses_a_build_the_nightly_doesnt_hold_naming_the_ids() {
    let dir = tempfile::tempdir().expect("temp dir");
    main_repo(dir.path(), &[MID_SECS, BUILD_SECS]);
    let rest = releases(dir.path(), &[MID]);
    let output = check_promotion(BUILD, &rest, "refs/heads/main");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        stderr(&output).contains(&format!(
            "build {BUILD} isn't on the nightly release, which holds build {MID}"
        )),
        "{output:?}"
    );
}

#[test]
fn check_promotion_refuses_a_run_off_main_and_a_malformed_build_with_exit_1() {
    let dir = tempfile::tempdir().expect("temp dir");
    main_repo(dir.path(), &[BUILD_SECS]);
    let rest = releases(dir.path(), &[BUILD]);
    for (build, git_ref, says) in [
        (
            BUILD,
            "refs/heads/roadmap/auto-updates",
            "promote-beta runs only on main; this run is on refs/heads/roadmap/auto-updates",
        ),
        (
            "not-an-id",
            "refs/heads/x",
            "promote-beta runs only on main; this run is on refs/heads/x",
        ),
        ("not-an-id", "refs/heads/main", "--build: `not-an-id`"),
    ] {
        let output = check_promotion(build, &rest, git_ref);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        assert!(stderr(&output).contains(says), "{says}: {output:?}");
    }
}

#[test]
fn check_promotion_refuses_two_commits_at_the_build_s_time_naming_both() {
    let dir = tempfile::tempdir().expect("temp dir");
    let shas = main_repo(dir.path(), &[MID_SECS, BUILD_SECS, BUILD_SECS]);
    let rest = releases(dir.path(), &[BUILD]);
    let output = check_promotion(BUILD, &rest, "refs/heads/main");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stderr(&output);
    assert!(
        text.contains(&shas[1]) && text.contains(&shas[2]) && text.contains(BUILD),
        "{text}"
    );
}

#[test]
fn check_promotion_without_an_option_exits_2_with_the_usage() {
    let output = check_promotion(BUILD, &[], "refs/heads/main");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("usage: nova-appcast"),
        "{output:?}"
    );
}
