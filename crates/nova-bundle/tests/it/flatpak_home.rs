//! A Flatpak user installation the test owns (Linux only), shared by the
//! opt-in Flatpak tests (`linux_flatpak`, `linux_flatpak_repo`).
//!
//! Every program run through [`Dirs::command`] has `HOME` and the `XDG_*`
//! directories pointed into a temporary directory, so the user's own
//! installation, remotes and app data are never touched. The test process's
//! own environment is never changed. A [`Guard`] uninstalls the app, and
//! the runtime it pulled, whatever happens.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// The test's own home and XDG directories.
pub struct Dirs {
    _tmp: tempfile::TempDir,
    pub home: PathBuf,
    pub data: PathBuf,
    pub config: PathBuf,
    pub cache: PathBuf,
    pub runtime: PathBuf,
}

impl Dirs {
    pub fn new() -> Self {
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
    pub fn command(&self, program: &str, args: &[&OsStr]) -> Command {
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
    pub fn flatpak(&self, args: &[&OsStr]) -> String {
        let output = self
            .command("flatpak", args)
            .output()
            .unwrap_or_else(|e| panic!("flatpak {args:?}: {e}"));
        stdout(&format!("flatpak {args:?}"), &output)
    }

    /// The exported share tree of the user installation.
    pub fn exports(&self) -> PathBuf {
        self.data.join("flatpak").join("exports").join("share")
    }
}

/// The standard output of a run that must have succeeded; otherwise a
/// panic showing `what` and both outputs.
pub fn stdout(what: &str, output: &Output) -> String {
    assert!(
        output.status.success(),
        "{what}: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

pub fn refs(args: &[OsString]) -> Vec<&OsStr> {
    args.iter().map(OsString::as_os_str).collect()
}

/// Uninstalls the app (and the runtime it pulled) from the test's
/// installation, even when an assertion failed.
pub struct Guard<'a> {
    home: &'a Dirs,
    id: String,
}

impl<'a> Guard<'a> {
    pub fn new(home: &'a Dirs, id: &str) -> Self {
        Self {
            home,
            id: id.to_owned(),
        }
    }
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
