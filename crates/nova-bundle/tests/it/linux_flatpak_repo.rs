//! The signed Flatpak repo, published for real by `nova-flatpak-repo`'s
//! `stage` and `sign` (`nova_bundle::linux::flatpak::repo`) into a
//! directory the test owns, served over HTTP from `127.0.0.1`, installed
//! and updated with `flatpak --user` in homes the test owns (Linux only,
//! opt-in).
//!
//! It skips, passing, unless `NOVA_FLATPAK` names a built `.flatpak` on
//! the `NOVA_CHANNEL` branch, so neither `mise run ci` nor a developer's
//! test run publishes anything. CI's Flatpak leg runs it after building
//! the bundle; it needs `gpg`, `ostree`, `flatpak` and `curl`, and the
//! network for the runtime from Flathub.
//!
//! It signs with a throwaway key made in a gpg home it owns
//! (`throwaway_key`), whose secret goes only into the in-memory
//! environment `sign` reads. It then:
//!
//! 1. publishes the bundle as the first publish (the site has no repo
//!    yet), and "deploys" it by moving the pages tree to the served
//!    `flatpak/`;
//! 2. installs from the served `.flatpakref`, and checks the remote's URL
//!    and the installed commit;
//! 3. publishes the same bundle again, mirroring the live repo over HTTP,
//!    GPG-verified, and checks the head didn't move;
//! 4. publishes three changed builds (a marker file added to the app), and
//!    checks `flatpak update` follows the first, and that the branch keeps
//!    `KEPT_COMMITS` commits after the last;
//! 5. installs, in a second home, the bundle `sign` built for the first
//!    changed build, checks its origin remote is the repo's URL, and that
//!    `flatpak update` takes it to the current head;
//! 6. promotes a held nightly build that isn't the head onto beta, with a
//!    beta host and no bundle (`nova_bundle::linux::flatpak::repo::promote`),
//!    and checks the beta head has the nightly commit's tree (`ostree
//!    diff` prints nothing), its subject names the build, and the nightly
//!    branch is untouched;
//! 7. installs `ev-nova-beta.flatpakref` in a third home, onto the `beta`
//!    branch at its head;
//! 8. promotes the newer nightly head, and checks `flatpak update` takes
//!    the beta install to it while the nightly install sees nothing new;
//! 9. publishes another nightly build after the promotions, and checks the
//!    mirror kept the `beta` branch and both `.flatpakref`s were written;
//! 10. promotes a build the nightly branch no longer holds, which fails
//!     naming the builds it does hold.
//!
//! What it finds about flatpak's behaviour (its version, the prune depth,
//! whether an identical build commits, the bundle's origin) is printed as
//! `finding:` lines before it is asserted, and the checks that don't stop
//! the run are collected and asserted at the end, so one run answers them
//! all.

use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use nova_bundle::appcast::Channel;
use nova_bundle::build_id;
use nova_bundle::bundle::metadata;
use nova_bundle::channel;
use nova_bundle::linux::flatpak::repo::plan::{Outcome, sign_repo, stage_repo};
use nova_bundle::linux::flatpak::repo::probe::Live;
use nova_bundle::linux::flatpak::repo::{
    KEPT_COMMITS, PAGES_DIR, REMOTE_NAME, RepoError, RepoHost, SECRET_SET_VAR, SECRET_VAR,
    SignPaths, StagePaths, app_ref, flatpakref_name, repo,
};
use nova_bundle::linux::flatpak::{ASSET, REMOTE, RUNTIME_REPO};
use nova_bundle::ports::{Env, Process};
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::{StdEnv, StdFiles, StdProcess};

use crate::flatpak_home::{Dirs, Guard, os, refs, stdout};
use crate::throwaway_key::ThrowawayKey;

const VARIABLE: &str = "NOVA_FLATPAK";

fn spec() -> BundleSpec {
    let json = StdProcess
        .run(&metadata(&OsString::from(env!("CARGO"))))
        .expect("cargo metadata");
    BundleSpec::from_metadata(&json).expect("spec")
}

/// The environment `stage` and `sign` read: the build ID and the secret,
/// in memory only.
struct MapEnv(HashMap<&'static str, OsString>);

impl Env for MapEnv {
    fn var(&self, name: &str) -> Option<OsString> {
        self.0.get(name).cloned()
    }
}

/// A static file server for `root` on `127.0.0.1`: HTTP/1.0, one thread
/// per connection, `404` for anything that isn't a file under `root`.
/// Dropping it stops it.
struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn start(root: &Path) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("address").port();
        let stop = Arc::new(AtomicBool::new(false));
        let root = root.to_owned();
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let root = root.clone();
                thread::spawn(move || serve(&root, &stream));
            }
        });
        Self {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}/{path}", self.port)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so it sees the flag.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Answers one request on `stream` from `root`.
fn serve(root: &Path, stream: &TcpStream) {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    if reader.read_line(&mut request).is_err() {
        return;
    }
    // Read the headers, so closing doesn't reset the connection.
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) if line == "\r\n" || line == "\n" => break,
            Ok(_) => {}
        }
    }
    let mut words = request.split_whitespace();
    let method = words.next().unwrap_or_default();
    let target = words.next().unwrap_or_default();
    let path = target.split(['?', '#']).next().unwrap_or_default();
    let file = Path::new(path.trim_start_matches('/'));
    let safe = file.components().all(|c| matches!(c, Component::Normal(_)));
    let body = safe
        .then(|| root.join(file))
        .filter(|p| p.is_file())
        .and_then(|p| fs::read(p).ok());
    let mut stream = stream;
    let _ = match body {
        Some(body) => {
            let head = format!(
                "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nContent-Type: \
                 application/octet-stream\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).and_then(|()| {
                if method == "HEAD" {
                    Ok(())
                } else {
                    stream.write_all(&body)
                }
            })
        }
        None => stream
            .write_all(b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
    };
    let _ = stream.flush();
}

/// Runs `program` with `args` outside any flatpak home, panicking with its
/// output unless it succeeds, and returns its standard output.
fn run(program: &str, args: &[&OsStr]) -> String {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("{program} {args:?}: {e}"));
    stdout(&format!("{program} {args:?}"), &output)
}

/// `--repo=<repo>`.
fn repo_flag(repo: &Path) -> OsString {
    let mut flag = OsString::from("--repo=");
    flag.push(repo);
    flag
}

/// `ostree --repo=<repo>` with these words. The published repo has no
/// `tmp/` (it isn't uploaded), which ostree needs to open a repo, so it
/// is made first.
fn ostree(repo: &Path, words: &[&str]) -> String {
    fs::create_dir_all(repo.join("tmp")).expect("the repo's tmp");
    let mut args = vec![repo_flag(repo)];
    args.extend(os(words));
    run("ostree", &refs(&args))
}

/// The commit `app` points at in `repo`.
fn head(repo: &Path, app: &str) -> String {
    ostree(repo, &["rev-parse", app]).trim().to_owned()
}

/// How many commits `ostree log` shows for `app` in `repo`, and the log.
fn commits(repo: &Path, app: &str) -> (usize, String) {
    let log = ostree(repo, &["log", app]);
    let count = log.lines().filter(|l| l.starts_with("commit ")).count();
    (count, log)
}

/// The test's run of the publisher: the served site, the host and the
/// throwaway key's secret.
struct Publisher<'a> {
    tmp: &'a Path,
    site: PathBuf,
    host: RepoHost,
    secret: String,
    runs: usize,
}

/// What one publish found and wrote.
struct Published {
    /// How `stage` found the live repo.
    live: Live,
    /// The bundle `sign` built.
    bundle: PathBuf,
    /// The `.flatpakref`s `sign` wrote, by file name.
    flatpakrefs: Vec<String>,
}

impl Publisher<'_> {
    /// `stage` then `sign` of the bundle in `bundle_dir`, as the build
    /// `build`, on the host's channel, then the deploy.
    fn publish(&mut self, bundle_dir: &Path, build: &str) -> (Live, PathBuf) {
        let host = self.host.clone();
        let published = self
            .run(&host, Some(bundle_dir), build)
            .unwrap_or_else(|e| panic!("publish {}: {e}", self.runs));
        let own = flatpakref_name(host.channel.expect("a channel"));
        assert!(
            published.flatpakrefs.contains(&own),
            "{own}: {:?}",
            published.flatpakrefs
        );
        (published.live, published.bundle)
    }

    /// The promotion of `build` onto `channel`: `stage` with no bundle,
    /// then `sign`, then the deploy; or the error that stopped it, with
    /// nothing deployed.
    fn promote(&mut self, channel: Channel, build: &str) -> Result<Published, RepoError> {
        let host = RepoHost {
            channel: Some(channel),
            ..self.host.clone()
        };
        self.run(&host, None, build)
    }

    /// `stage` then `sign` for `host`, as the build `build`, then the
    /// deploy: the pages tree becomes the served `flatpak/`.
    fn run(
        &mut self,
        host: &RepoHost,
        bundle_dir: Option<&Path>,
        build: &str,
    ) -> Result<Published, RepoError> {
        self.runs += 1;
        let run = self.tmp.join(format!("run-{}", self.runs));
        let (work, pages, dist) = (run.join("work"), run.join("pages"), run.join("dist"));
        let env = MapEnv(HashMap::from([
            (build_id::VAR, OsString::from(build)),
            (SECRET_VAR, OsString::from(&self.secret)),
            (SECRET_SET_VAR, OsString::from("true")),
        ]));
        let stage = StagePaths {
            bundle_dir: bundle_dir.map(Path::to_owned),
            work: work.clone(),
            pages: pages.clone(),
        };
        let staged = stage_repo(&mut StdProcess, &mut StdFiles, &env, host, &stage)?;
        let Outcome::Staged { live, .. } = staged else {
            panic!("stage {}: {staged:?}", self.runs);
        };
        let sign = SignPaths {
            work,
            pages: pages.clone(),
            dist,
        };
        let signed = sign_repo(&mut StdProcess, &mut StdFiles, &env, host, &sign)?;
        let Outcome::Published {
            bundle,
            flatpakrefs,
        } = signed
        else {
            panic!("sign {}: {signed:?}", self.runs);
        };
        // The pages-flatpak artifact's layout: the signed repo, without its
        // scratch tmp/, beside the refs.
        for flatpakref in &flatpakrefs {
            assert_eq!(flatpakref.parent(), Some(pages.as_path()));
            assert!(flatpakref.is_file(), "{}", flatpakref.display());
        }
        assert!(repo(&pages).join("config").is_file());
        assert!(repo(&pages).join("summary.sig").is_file());
        assert!(!repo(&pages).join("tmp").exists(), "tmp/ is not published");
        assert!(bundle.is_file(), "{}", bundle.display());
        let served = self.site.join(PAGES_DIR);
        if served.exists() {
            fs::remove_dir_all(&served).expect("the old deploy removed");
        }
        fs::rename(&pages, &served).expect("deployed");
        let flatpakrefs = flatpakrefs
            .iter()
            .map(|p| {
                p.file_name()
                    .expect("a file")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        Ok(Published {
            live,
            bundle,
            flatpakrefs,
        })
    }

    /// The served repo.
    fn repo(&self) -> PathBuf {
        repo(&self.site.join(PAGES_DIR))
    }
}

/// A build of the app with a marker file added: `bundle`'s app imported
/// into a scratch repo, checked out, marked, exported onto the same
/// branch, and bundled into `<dir>/out`.
fn changed_build(
    bundle: &Path,
    dir: &Path,
    app: &str,
    spec: &BundleSpec,
    branch: &str,
    marker: &str,
) {
    let scratch = dir.join("repo");
    let checkout = dir.join("checkout");
    let out = dir.join("out");
    ostree(&scratch, &["init", "--mode=archive"]);
    run(
        "flatpak",
        &[
            OsStr::new("build-import-bundle"),
            scratch.as_os_str(),
            bundle.as_os_str(),
        ],
    );
    run(
        "ostree",
        &[
            repo_flag(&scratch).as_os_str(),
            OsStr::new("checkout"),
            OsStr::new("--user-mode"),
            OsStr::new(app),
            checkout.as_os_str(),
        ],
    );
    let share_dir = checkout
        .join("files")
        .join("share")
        .join("ev-nova-repo-test");
    fs::create_dir_all(&share_dir).expect("the marker's dir");
    fs::write(share_dir.join("marker"), marker).expect("the marker");
    run(
        "flatpak",
        &[
            OsStr::new("build-export"),
            scratch.as_os_str(),
            checkout.as_os_str(),
            OsStr::new(branch),
        ],
    );
    fs::create_dir_all(&out).expect("the bundle's dir");
    let runtime_repo = format!("--runtime-repo={RUNTIME_REPO}");
    run(
        "flatpak",
        &[
            OsStr::new("build-bundle"),
            OsStr::new(&runtime_repo),
            scratch.as_os_str(),
            out.join(ASSET).as_os_str(),
            OsStr::new(&spec.identifier),
            OsStr::new(branch),
        ],
    );
}

/// The URL of the user remote `name` in `home`, from `flatpak remotes`.
fn remote_url(home: &Dirs, name: &str) -> Option<String> {
    let remotes = home.flatpak(&refs(&os(&[
        "remotes",
        "--user",
        "--show-disabled",
        "--columns=name,url",
    ])));
    eprintln!("remotes:\n{remotes}");
    remotes.lines().find_map(|line| {
        let mut cells = line.split_whitespace();
        (cells.next() == Some(name)).then(|| cells.next().unwrap_or_default().to_owned())
    })
}

/// The commit of the app `id` installed in `home`.
fn installed(home: &Dirs, id: &str) -> String {
    home.flatpak(&refs(&os(&["info", "--user", "--show-commit", id])))
        .trim()
        .to_owned()
}

/// A home with Flathub added, for the runtime.
fn home_with_flathub() -> Dirs {
    let home = Dirs::new();
    home.flatpak(&refs(&os(&[
        "remote-add",
        "--user",
        "--if-not-exists",
        REMOTE,
        RUNTIME_REPO,
    ])));
    home
}

/// Runs `flatpak` with `args` in `home`, printing its outcome as a
/// finding about `what`, then requires it succeeded.
fn flatpak_finding(home: &Dirs, what: &str, args: &[&str]) {
    let output = home
        .command("flatpak", &refs(&os(args)))
        .output()
        .unwrap_or_else(|e| panic!("flatpak {args:?}: {e}"));
    eprintln!(
        "finding: {what}: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    stdout(&format!("flatpak {args:?}"), &output);
}

/// Installs the app in `home` from the served `flatpakref`, and checks the
/// remote it adds is the repo at `url` and the install is at `head`.
fn install_from_the_ref(
    home: &Dirs,
    flatpakref: &Path,
    id: &str,
    url: &str,
    head: &str,
    problems: &mut Vec<String>,
) {
    let mut install = os(&["install", "--user", "-y", "--noninteractive", "--from"]);
    install.push(flatpakref.as_os_str().to_owned());
    home.flatpak(&refs(&install));
    let remote = remote_url(home, REMOTE_NAME);
    if remote.as_deref() != Some(url) {
        problems.push(format!(
            "the {REMOTE_NAME} remote's URL is {remote:?}, not {url}"
        ));
    }
    assert_eq!(installed(home, id), head, "installed the repo's head");
}

/// Installs `bundle` (one `sign` built) in a new home, and checks its
/// origin remote is the repo at `url` and that `flatpak update` takes it
/// to `current`, the repo's head, pushing what's wrong onto `problems`.
fn the_bundle_follows_the_repo(
    bundle: &Path,
    id: &str,
    url: &str,
    current: &str,
    problems: &mut Vec<String>,
) {
    let second = home_with_flathub();
    let _guard = Guard::new(&second, id);
    let bundle = bundle.to_string_lossy().into_owned();
    flatpak_finding(
        &second,
        "I2: installing the --gpg-keys bundle",
        &[
            "install",
            "--user",
            "-y",
            "--noninteractive",
            "--bundle",
            &bundle,
        ],
    );
    let origin = second
        .flatpak(&refs(&os(&["info", "--user", "--show-origin", id])))
        .trim()
        .to_owned();
    let origin_url = remote_url(&second, &origin);
    eprintln!("finding: I2: the bundle's origin remote {origin} is at {origin_url:?}");
    if origin_url.as_deref() != Some(url) {
        problems.push(format!(
            "the bundle's origin remote {origin} is at {origin_url:?}, not {url} (AC3)"
        ));
    }
    flatpak_finding(
        &second,
        "I2: updating the bundle's install",
        &["update", "--user", "-y", "--noninteractive", id],
    );
    if installed(&second, id) != current {
        problems.push("the bundle's install didn't update to the repo's head (I2)".to_owned());
    }
}

/// Checks that publishing the same build again left the head at `first`
/// (`again` is the head after it), pushing a new commit onto `problems`
/// (I3).
fn the_identical_build_commits_nothing(first: &str, again: &str, problems: &mut Vec<String>) {
    eprintln!(
        "finding: I3: build-commit-from with the head's content {}",
        if again == first {
            "commits nothing"
        } else {
            "makes a new commit"
        }
    );
    if again != first {
        problems.push("an identical build made a new commit (I3)".to_owned());
    }
}

#[test]
fn publishes_a_signed_repo_that_installs_updates_and_prunes() {
    let Some(bundle) = env::var_os(VARIABLE)
        .filter(|v| !v.is_empty())
        .map(|v| std::path::absolute(PathBuf::from(v)).expect("an absolute path"))
    else {
        eprintln!("{VARIABLE} is unset: skipping the Flatpak repo test");
        return;
    };
    let channel = channel::from_env(&StdEnv)
        .expect("NOVA_CHANNEL")
        .expect("NOVA_CHANNEL names the bundle's channel");
    let branch = channel.name();
    let spec = spec();
    let id = spec.identifier.clone();
    let app = app_ref(&spec, channel);
    eprintln!(
        "finding: {}",
        run("flatpak", &[OsStr::new("--version")]).trim()
    );

    let key = ThrowawayKey::new();
    let tmp = key.dir().to_owned();
    let public = tmp.join("repo-key.asc");
    key.write_public(&public);
    let site = tmp.join("site");
    fs::create_dir_all(&site).expect("the site");
    let server = Server::start(&site);
    let url = server.url(&format!("{PAGES_DIR}/repo/"));
    let mut publisher = Publisher {
        tmp: &tmp,
        site: site.clone(),
        host: RepoHost {
            cargo: env!("CARGO").into(),
            channel: Some(channel),
            url: url.clone(),
            key: Some(public),
        },
        secret: key.secret_base64(),
        runs: 0,
    };
    let plain = bundle.parent().expect("the bundle's dir").to_owned();
    assert_eq!(
        plain.join(ASSET),
        bundle,
        "NOVA_FLATPAK is the packed asset"
    );
    let mut problems: Vec<String> = Vec::new();

    // 1. The first publish.
    let (live, _) = publisher.publish(&plain, "20261028000001");
    assert_eq!(live, Live::Absent, "the site had no repo");
    let first = head(&publisher.repo(), &app);
    let flatpakref = site.join(PAGES_DIR).join(flatpakref_name(channel));

    // 2. Installed from the .flatpakref.
    let home = home_with_flathub();
    let _guard = Guard::new(&home, &id);
    install_from_the_ref(&home, &flatpakref, &id, &url, &first, &mut problems);

    // 3. The same bundle again: mirrored over HTTP, GPG-verified.
    let (live, _) = publisher.publish(&plain, "20261028000002");
    assert_eq!(live, Live::Present, "the second publish mirrors");
    the_identical_build_commits_nothing(&first, &head(&publisher.repo(), &app), &mut problems);

    // 4. Three changed builds.
    let mut bundles = Vec::new();
    for n in 2..=4 {
        let dir = tmp.join(format!("build-{n}"));
        changed_build(&bundle, &dir, &app, &spec, branch, &format!("build {n}"));
        let (live, signed) = publisher.publish(&dir.join("out"), &format!("2026102800001{n}"));
        assert_eq!(live, Live::Present, "publish of build {n} mirrors");
        bundles.push(signed);
        if n == 2 {
            home.flatpak(&refs(&os(&[
                "update",
                "--user",
                "-y",
                "--noninteractive",
                &id,
            ])));
            let now = head(&publisher.repo(), &app);
            assert_ne!(now, first, "build 2 is a new head");
            if installed(&home, &id) != now {
                problems.push("flatpak update didn't follow the repo to build 2".to_owned());
            }
        }
    }
    let (count, log) = commits(&publisher.repo(), &app);
    eprintln!(
        "finding: I1: after the commits onto {app}, ostree log shows {count} (KEPT_COMMITS is \
         {KEPT_COMMITS}):\n{log}"
    );
    if count != KEPT_COMMITS {
        problems.push(format!(
            "the branch keeps {count} commits, not {KEPT_COMMITS} (I1)"
        ));
    }
    let current = head(&publisher.repo(), &app);

    // 5. The bundle sign built for build 2, installed in a second home,
    // follows the repo to build 4.
    the_bundle_follows_the_repo(&bundles[0], &id, &url, &current, &mut problems);

    // 6 to 10. The promotions onto the channel promoted from this one.
    promotions(&mut publisher, &home, &bundle, &mut problems);

    assert!(problems.is_empty(), "{problems:#?}");
    drop(server);
}

/// Steps 6 to 10, once the publisher's channel's branch holds
/// `KEPT_COMMITS` builds (`2026102800001{2,3,4}`) and `nightly_home` holds
/// an install from its `.flatpakref`: run when a channel is promoted from
/// it, and printed as a finding when none is.
fn promotions(
    publisher: &mut Publisher<'_>,
    nightly_home: &Dirs,
    bundle: &Path,
    problems: &mut Vec<String>,
) {
    let channel = publisher.host.channel.expect("a channel");
    let Some(promoted) = Channel::ALL
        .into_iter()
        .find(|c| c.promoted_from() == Some(channel))
    else {
        eprintln!(
            "finding: promotion: no channel is promoted from {}",
            channel.name()
        );
        return;
    };
    let spec = spec();
    let id = spec.identifier.clone();
    let from_app = app_ref(&spec, channel);
    let served = publisher.repo();
    let from_head = head(&served, &from_app);
    let (_, from_log) = commits(&served, &from_app);
    // The nightly install is current before any promotion.
    nightly_home.flatpak(&refs(&os(&[
        "update",
        "--user",
        "-y",
        "--noninteractive",
        &id,
    ])));
    assert_eq!(installed(nightly_home, &id), from_head);
    let mut p = Promotion {
        to_app: app_ref(&spec, promoted),
        publisher,
        spec,
        bundle,
        nightly_home,
        channel,
        promoted,
        id,
        from_app,
        from_head,
        from_log,
    };
    let promoted_head = p.promote_a_held_build(problems);
    let beta_home = home_with_flathub();
    let _guard = Guard::new(&beta_home, &p.id);
    p.install_the_promoted_ref(&beta_home, &promoted_head, problems);
    let newer_head = p.promote_the_newer_head(&beta_home, &promoted_head, problems);
    p.publish_a_nightly_after(&newer_head);
    p.promote_a_pruned_build();
}

/// What the promotions share.
struct Promotion<'p, 'a> {
    publisher: &'p mut Publisher<'a>,
    spec: BundleSpec,
    /// The plain bundle under test.
    bundle: &'p Path,
    /// The home holding the install from the channel's own `.flatpakref`.
    nightly_home: &'p Dirs,
    /// The channel the builds were published on.
    channel: Channel,
    /// The channel promoted from it.
    promoted: Channel,
    id: String,
    from_app: String,
    to_app: String,
    /// The channel's head and log before any promotion.
    from_head: String,
    from_log: String,
}

impl Promotion<'_, '_> {
    /// Both channels' `.flatpakref` names, in `Channel::ALL`'s order.
    fn both(&self) -> Vec<String> {
        [self.channel, self.promoted].map(flatpakref_name).to_vec()
    }

    /// Step 6: a held build that isn't the head, promoted: the same tree, a
    /// subject naming it, and the channel's own branch untouched. Returns
    /// the promoted head.
    fn promote_a_held_build(&mut self, problems: &mut Vec<String>) -> String {
        let held = "20261028000013";
        let published = self
            .publisher
            .promote(self.promoted, held)
            .unwrap_or_else(|e| panic!("promoting {held}: {e}"));
        eprintln!(
            "finding: promotion: {held} onto {} wrote {:?}",
            self.promoted.name(),
            published.flatpakrefs
        );
        assert_eq!(published.live, Live::Present);
        let served = self.publisher.repo();
        let promoted_head = head(&served, &self.to_app);
        let source = commit_with_subject(
            &self.from_log,
            &format!("EV Nova {} build {held}", self.channel.name()),
        );
        let diff = ostree(&served, &["diff", &source, &promoted_head]);
        eprintln!("finding: promotion: ostree diff {source} {promoted_head}: {diff:?}");
        if !diff.trim().is_empty() {
            problems.push(format!(
                "the promoted commit's tree differs from {source}: {diff}"
            ));
        }
        let (_, promoted_log) = commits(&served, &self.to_app);
        let subject = format!("EV Nova {} build {held}", self.promoted.name());
        if !promoted_log.contains(&subject) {
            problems.push(format!(
                "the {} head's subject isn't {subject:?}: {promoted_log}",
                self.to_app
            ));
        }
        assert_eq!(
            head(&served, &self.from_app),
            self.from_head,
            "the nightly head is untouched"
        );
        assert_eq!(
            commits(&served, &self.from_app).1,
            self.from_log,
            "the nightly log is untouched"
        );
        assert_eq!(published.flatpakrefs, self.both());
        promoted_head
    }

    /// Step 7: installed from the promoted channel's `.flatpakref` in
    /// `beta_home`: on its branch, at `promoted_head`.
    fn install_the_promoted_ref(
        &self,
        beta_home: &Dirs,
        promoted_head: &str,
        problems: &mut Vec<String>,
    ) {
        let flatpakref = self
            .publisher
            .site
            .join(PAGES_DIR)
            .join(flatpakref_name(self.promoted));
        let url = self.publisher.host.url.clone();
        install_from_the_ref(
            beta_home,
            &flatpakref,
            &self.id,
            &url,
            promoted_head,
            problems,
        );
        let installed_ref = beta_home
            .flatpak(&refs(&os(&["info", "--user", "--show-ref", &self.id])))
            .trim()
            .to_owned();
        eprintln!("finding: promotion: the beta install is {installed_ref}");
        assert!(
            installed_ref.ends_with(&format!("/x86_64/{}", self.promoted.name())),
            "{installed_ref}"
        );
    }

    /// Step 8: the channel's head promoted: the beta install follows, the
    /// nightly install sees nothing new. Returns the new promoted head.
    fn promote_the_newer_head(
        &mut self,
        beta_home: &Dirs,
        promoted_head: &str,
        problems: &mut Vec<String>,
    ) -> String {
        let newer = "20261028000014";
        self.publisher
            .promote(self.promoted, newer)
            .unwrap_or_else(|e| panic!("promoting {newer}: {e}"));
        let newer_head = head(&self.publisher.repo(), &self.to_app);
        assert_ne!(
            newer_head, promoted_head,
            "the newer promotion is a new head"
        );
        let update = os(&["update", "--user", "-y", "--noninteractive", &self.id]);
        beta_home.flatpak(&refs(&update));
        if installed(beta_home, &self.id) != newer_head {
            problems.push(
                "flatpak update didn't take the beta install to the newer promotion".to_owned(),
            );
        }
        self.nightly_home.flatpak(&refs(&update));
        if installed(self.nightly_home, &self.id) != self.from_head {
            problems.push("the nightly install moved after a promotion".to_owned());
        }
        newer_head
    }

    /// Step 9: a nightly publish after the promotions keeps the beta branch at
    /// `newer_head` and both `.flatpakref`s.
    fn publish_a_nightly_after(&mut self, newer_head: &str) {
        let dir = self.publisher.tmp.join("build-5");
        changed_build(
            self.bundle,
            &dir,
            &self.from_app,
            &self.spec,
            self.channel.name(),
            "build 5",
        );
        let host = self.publisher.host.clone();
        let after = self
            .publisher
            .run(&host, Some(&dir.join("out")), "20261028000015")
            .unwrap_or_else(|e| panic!("the nightly after the promotions: {e}"));
        assert_eq!(
            after.flatpakrefs,
            self.both(),
            "a nightly keeps the beta ref"
        );
        assert_eq!(
            head(&self.publisher.repo(), &self.to_app),
            newer_head,
            "the mirror kept the beta branch"
        );
    }

    /// Step 10: a build the channel's branch no longer holds is refused,
    /// naming the builds it does hold.
    fn promote_a_pruned_build(&mut self) {
        let pruned = "20261028000001";
        match self.publisher.promote(self.promoted, pruned) {
            Err(e @ RepoError::NotOnBranch(_)) => {
                let message = e.to_string();
                eprintln!("finding: promotion: {message}");
                assert!(
                    message.contains(pruned) && message.contains("20261028000015"),
                    "{message}"
                );
            }
            other => panic!("promoting {pruned}: {:?}", other.map(|p| p.flatpakrefs)),
        }
    }
}

/// The checksum of the commit in `log` (`ostree log`'s output) whose
/// subject is `subject`.
fn commit_with_subject(log: &str, subject: &str) -> String {
    let mut current = None;
    for line in log.lines() {
        if let Some(checksum) = line.strip_prefix("commit ") {
            current = Some(checksum.trim().to_owned());
        } else if line.trim() == subject {
            return current.unwrap_or_else(|| panic!("no commit before {subject:?}"));
        }
    }
    panic!("no commit {subject:?} in:\n{log}");
}
