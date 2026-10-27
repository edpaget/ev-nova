//! What `nova-installer` reads from its environment, turned into an
//! [`InstallerHost`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The packer's usage line.
pub const USAGE: &str = "usage: nova-installer [--exes <DIR>] [OUT_DIR]   \
     (run it as `mise run pack-installer -- --exes <DIR> [OUT_DIR]`)";

/// What the edge supplies: the tools and the directories asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerHost {
    /// The `cargo` whose `cargo metadata` gives the spec (`$CARGO` under
    /// `cargo run`).
    pub cargo: OsString,
    /// WiX's `wix`.
    pub wix: OsString,
    /// The directory holding the built `nova.exe` and `nova-dump.exe`, if
    /// one was given.
    pub exes: Option<PathBuf>,
    /// The output directory asked for, if any.
    pub out: Option<PathBuf>,
}

/// Why the environment gave no [`InstallerHost`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HostError {
    /// The host is not Windows, where WiX's `wix` runs.
    #[error("packing the installer needs WiX's `wix`, which runs on Windows; this host is {0}")]
    NotWindows(String),
    /// An unknown option, `--exes` without a directory or twice, or more
    /// than one output directory.
    #[error("{USAGE}")]
    Usage,
}

/// The host from the arguments after the program name, `$CARGO`, the
/// `wix` to run (an override, else the one on `PATH`) and the working
/// directory, which relative directories are taken against, on the host
/// `os` (`std::env::consts::OS`).
pub fn installer_host(
    args: Vec<OsString>,
    cargo: Option<OsString>,
    wix: Option<OsString>,
    cwd: &Path,
    os: &str,
) -> Result<InstallerHost, HostError> {
    if os != "windows" {
        return Err(HostError::NotWindows(os.to_owned()));
    }
    let mut exes = None;
    let mut out = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--exes" {
            let dir = args.next().ok_or(HostError::Usage)?;
            if exes.replace(cwd.join(dir)).is_some() {
                return Err(HostError::Usage);
            }
        } else if arg.to_string_lossy().starts_with('-') || out.is_some() {
            return Err(HostError::Usage);
        } else {
            out = Some(cwd.join(arg));
        }
    }
    Ok(InstallerHost {
        cargo: cargo.unwrap_or_else(|| OsString::from("cargo")),
        wix: wix
            .filter(|w| !w.is_empty())
            .unwrap_or_else(|| OsString::from("wix")),
        exes,
        out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn parse(args: &[&str]) -> Result<InstallerHost, HostError> {
        installer_host(os(args), None, None, &cwd(), "windows")
    }

    #[test]
    fn no_argument_leaves_the_defaults() {
        assert_eq!(
            installer_host(vec![], Some("C:/cargo.exe".into()), None, &cwd(), "windows"),
            Ok(InstallerHost {
                cargo: "C:/cargo.exe".into(),
                wix: "wix".into(),
                exes: None,
                out: None,
            })
        );
    }

    #[test]
    fn cargo_defaults_to_the_one_on_the_path() {
        assert_eq!(parse(&[]).expect("host").cargo, "cargo");
    }

    #[test]
    fn a_wix_override_is_used_unless_empty() {
        let host = installer_host(
            vec![],
            None,
            Some("D:/wix/wix.exe".into()),
            &cwd(),
            "windows",
        );
        assert_eq!(host.expect("host").wix, "D:/wix/wix.exe");
        let host = installer_host(vec![], None, Some(OsString::new()), &cwd(), "windows");
        assert_eq!(host.expect("host").wix, "wix");
    }

    #[test]
    fn relative_directories_are_taken_against_the_working_directory() {
        let release = Path::new("target")
            .join("x86_64-pc-windows-msvc")
            .join("release");
        let args = vec![
            OsString::from("--exes"),
            release.clone().into(),
            OsString::from("dist"),
        ];
        let host = installer_host(args, None, None, &cwd(), "windows").expect("host");
        assert_eq!(host.exes, Some(cwd().join(release)));
        assert_eq!(host.out, Some(cwd().join("dist")));
    }

    #[test]
    fn the_output_may_come_before_the_executables() {
        let host = parse(&["dist", "--exes", "signed"]).expect("host");
        assert_eq!(host.exes, Some(cwd().join("signed")));
        assert_eq!(host.out, Some(cwd().join("dist")));
    }

    #[test]
    fn absolute_directories_are_kept() {
        let exes = std::env::temp_dir().join("signed");
        let out = std::env::temp_dir().join("out");
        let args = vec![
            OsString::from("--exes"),
            exes.clone().into(),
            out.clone().into(),
        ];
        let host = installer_host(args, None, None, &cwd(), "windows").expect("host");
        assert_eq!(host.exes, Some(exes));
        assert_eq!(host.out, Some(out));
    }

    #[test]
    fn bad_arguments_are_a_usage_error() {
        for args in [
            &["--exes"][..],
            &["--exes", "a", "--exes", "b"],
            &["a", "b"],
            &["--help"],
            &["-h"],
            &["--out", "x"],
        ] {
            assert_eq!(parse(args), Err(HostError::Usage), "{args:?}");
        }
    }

    #[test]
    fn a_host_other_than_windows_is_refused() {
        for os in ["macos", "linux"] {
            assert_eq!(
                installer_host(vec![], None, None, &cwd(), os),
                Err(HostError::NotWindows(os.to_owned()))
            );
        }
    }
}
