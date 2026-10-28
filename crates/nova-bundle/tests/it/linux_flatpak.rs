//! The built Flatpak bundle, installed for real with `flatpak install
//! --user` into a home the test owns (Linux only, opt-in).
//!
//! It skips, passing, unless `NOVA_FLATPAK` names a built `.flatpak`, so
//! neither `mise run ci` nor a developer's test run installs anything.
//! CI's Flatpak leg builds the bundle and runs this under `xvfb-run`:
//! zenity, like any GTK program, needs a display even for `--version`.
//! It needs the network, to pull the runtime from Flathub into the
//! test's own installation.
//!
//! Every `flatpak` it runs has `HOME` and the `XDG_*` directories pointed
//! into a temporary directory, so the user's own installation, remotes
//! and app data are never touched. It checks that the installed app is on
//! the channel's branch (`NOVA_CHANNEL`, as the build read it), has
//! exactly the recorded permissions ([`FINISH_ARGS`], as the host's
//! flatpak stores them: see `flatpak_metadata`), that zenity runs in
//! its sandbox, and that it exports `applications/<id>.desktop` (passing
//! `desktop-file-validate`, named for the app id the window sets) and the
//! hicolor icons under the app id. Whatever happens, a guard uninstalls
//! it.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use nova_bundle::bundle::metadata;
use nova_bundle::channel;
use nova_bundle::linux::flatpak::{FINISH_ARGS, REMOTE, RUNTIME_REPO, branch};
use nova_bundle::linux::icons::ICONS;
use nova_bundle::ports::Process;
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::{StdEnv, StdProcess};

use crate::flatpak_metadata::{expected_permissions, flatpak_version, permissions};

const VARIABLE: &str = "NOVA_FLATPAK";

fn spec() -> BundleSpec {
    let json = StdProcess
        .run(&metadata(&OsString::from(env!("CARGO"))))
        .expect("cargo metadata");
    BundleSpec::from_metadata(&json).expect("spec")
}

/// The test's own home and XDG directories.
struct Dirs {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    data: PathBuf,
    config: PathBuf,
    cache: PathBuf,
    runtime: PathBuf,
}

impl Dirs {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("temp dir");
        let dir = |name: &str| {
            let dir = tmp.path().join(name);
            fs::create_dir_all(&dir).expect("a test dir");
            dir
        };
        let home = dir("home");
        let runtime = dir("runtime");
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).expect("0700");
        Self {
            data: dir("data"),
            config: dir("config"),
            cache: dir("cache"),
            home,
            runtime,
            _tmp: tmp,
        }
    }

    /// `program` with `args`, its home and XDG directories the test's.
    fn command(&self, program: &str, args: &[&OsStr]) -> Command {
        let mut command = Command::new(program);
        command
            .args(args)
            .env("HOME", &self.home)
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_CACHE_HOME", &self.cache)
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .stdin(Stdio::null());
        command
    }

    /// Runs `flatpak` with `args`, panicking with its output unless it
    /// succeeds, and returns its standard output.
    fn flatpak(&self, args: &[&OsStr]) -> String {
        let output = self
            .command("flatpak", args)
            .output()
            .unwrap_or_else(|e| panic!("flatpak {args:?}: {e}"));
        stdout(&format!("flatpak {args:?}"), &output)
    }

    /// The exported share tree of the user installation.
    fn exports(&self) -> PathBuf {
        self.data.join("flatpak").join("exports").join("share")
    }
}

fn stdout(what: &str, output: &Output) -> String {
    assert!(
        output.status.success(),
        "{what}: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn refs(args: &[OsString]) -> Vec<&OsStr> {
    args.iter().map(OsString::as_os_str).collect()
}

/// Uninstalls the app (and the runtime it pulled) from the test's
/// installation, even when an assertion failed.
struct Guard<'a> {
    home: &'a Dirs,
    id: String,
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        for args in [
            os(&["uninstall", "--user", "-y", "--noninteractive", &self.id]),
            os(&["uninstall", "--user", "-y", "--noninteractive", "--unused"]),
        ] {
            let status = self.home.command("flatpak", &refs(&args)).status();
            if !status.as_ref().is_ok_and(std::process::ExitStatus::success) {
                eprintln!("cleanup: flatpak {args:?} failed: {status:?}");
            }
        }
    }
}

#[test]
fn installs_with_exactly_the_recorded_permissions_zenity_and_the_exports() {
    let Some(bundle) = env::var_os(VARIABLE)
        .filter(|v| !v.is_empty())
        .map(|v| std::path::absolute(PathBuf::from(v)).expect("an absolute path"))
    else {
        eprintln!("{VARIABLE} is unset: skipping the Flatpak test");
        return;
    };
    let spec = spec();
    let id = spec.identifier.clone();
    let home = Dirs::new();
    home.flatpak(&refs(&os(&[
        "remote-add",
        "--user",
        "--if-not-exists",
        REMOTE,
        RUNTIME_REPO,
    ])));
    let _guard = Guard {
        home: &home,
        id: id.clone(),
    };
    let mut install = os(&["install", "--user", "-y", "--noninteractive", "--bundle"]);
    install.push(bundle.into_os_string());
    home.flatpak(&refs(&install));

    // On the channel's branch: `nightly` in CI, `master` for a
    // development build.
    let channel = channel::from_env(&StdEnv).expect("NOVA_CHANNEL");
    let shown_ref = home.flatpak(&refs(&os(&["info", "--show-ref", &id])));
    assert!(
        shown_ref
            .trim()
            .ends_with(&format!("/x86_64/{}", branch(channel))),
        "{shown_ref}"
    );

    // Exactly the recorded permissions, as this flatpak (which finished
    // the build) stores them, and nothing beyond [Context].
    let flatpak = flatpak_version(&home.flatpak(&refs(&os(&["--version"]))));
    let shown = home.flatpak(&refs(&os(&["info", "--show-permissions", &id])));
    let groups = permissions(&shown);
    assert_eq!(
        groups.get("Context"),
        Some(&expected_permissions(&FINISH_ARGS, flatpak)),
        "flatpak {flatpak:?}:\n{shown}"
    );
    for (group, keys) in &groups {
        assert!(
            group == "Context" || keys.is_empty(),
            "[{group}] in\n{shown}"
        );
    }

    // zenity is on the sandbox's PATH, and runs.
    let found = home.flatpak(&refs(&os(&[
        "run",
        "--command=sh",
        &id,
        "-c",
        "command -v zenity",
    ])));
    assert!(found.trim().ends_with("/zenity"), "{found}");
    let version = home.flatpak(&refs(&os(&["run", "--command=zenity", &id, "--version"])));
    assert!(
        !version.trim().is_empty(),
        "zenity --version printed nothing"
    );

    // The exports: one desktop file, named for the app id the window sets
    // (`nova::platform::APP_ID`, the bundle identifier), and its icons.
    let exports = home.exports();
    let applications = exports.join("applications");
    let desktops: Vec<PathBuf> = fs::read_dir(&applications)
        .unwrap_or_else(|e| panic!("{}: {e}", applications.display()))
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "desktop"))
        .collect();
    assert_eq!(
        desktops,
        [applications.join(format!("{id}.desktop"))],
        "the exported desktop files"
    );
    assert_eq!(
        desktops[0].file_stem().and_then(OsStr::to_str),
        Some(id.as_str())
    );
    let validate = home
        .command("desktop-file-validate", &[desktops[0].as_os_str()])
        .output()
        .expect("desktop-file-validate");
    stdout("desktop-file-validate", &validate);
    for icon in ICONS {
        let path = exports
            .join("icons")
            .join("hicolor")
            .join(format!("{0}x{0}", icon.size))
            .join("apps")
            .join(format!("{id}.png"));
        assert!(path_exists(&path), "{}", path.display());
    }
}

/// Whether `path` exists, following the export's links into the
/// installation.
fn path_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}
