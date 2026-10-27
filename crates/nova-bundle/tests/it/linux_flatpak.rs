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
//! and app data are never touched. It checks that the installed app has
//! exactly the recorded permissions ([`FINISH_ARGS`]), that zenity runs in
//! its sandbox, and that it exports `applications/<id>.desktop` (passing
//! `desktop-file-validate`, named for the app id the window sets) and the
//! hicolor icons under the app id. Whatever happens, a guard uninstalls
//! it.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use nova_bundle::bundle::metadata;
use nova_bundle::linux::flatpak::{FINISH_ARGS, REMOTE, RUNTIME_REPO};
use nova_bundle::linux::icons::ICONS;
use nova_bundle::ports::Process;
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::StdProcess;

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

/// The permissions [`FINISH_ARGS`] grant, as `flatpak info
/// --show-permissions` lists them: each `[Context]` key with its values.
fn expected_permissions() -> BTreeMap<String, BTreeSet<String>> {
    let mut expected: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for arg in FINISH_ARGS {
        let (option, value) = arg
            .strip_prefix("--")
            .and_then(|a| a.split_once('='))
            .unwrap_or_else(|| panic!("{arg} is --option=value"));
        let key = match option {
            "socket" => "sockets",
            "share" => "shared",
            "device" => "devices",
            "filesystem" => "filesystems",
            "persist" => "persistent",
            other => panic!("no [Context] key for --{other}"),
        };
        expected
            .entry(key.to_owned())
            .or_default()
            .insert(value.to_owned());
    }
    expected
}

/// `flatpak info --show-permissions`'s key file, read as each group's
/// keys with their `;`-separated values.
fn permissions(text: &str) -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let mut groups: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    let mut group = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            name.clone_into(&mut group);
            groups.entry(group.clone()).or_default();
        } else if let Some((key, values)) = line.split_once('=') {
            let values = values
                .split(';')
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect();
            groups
                .entry(group.clone())
                .or_default()
                .insert(key.to_owned(), values);
        }
    }
    groups
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

    // Exactly the recorded permissions, and nothing beyond [Context].
    let shown = home.flatpak(&refs(&os(&["info", "--show-permissions", &id])));
    let groups = permissions(&shown);
    assert_eq!(
        groups.get("Context"),
        Some(&expected_permissions()),
        "{shown}"
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
