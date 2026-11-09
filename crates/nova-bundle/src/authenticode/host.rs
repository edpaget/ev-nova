//! What `nova-sign-windows` reads from its arguments, turned into a
//! [`SignHost`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The signer's usage line.
pub const USAGE: &str = "usage: nova-sign-windows <FILE>...   \
     (run it as `mise run sign-windows -- <FILE>...`)";

/// What the edge supplies: `cargo` and the files to sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignHost {
    /// The `cargo` whose `cargo metadata` names the target directory the
    /// tools are staged in (`$CARGO` under `cargo run`).
    pub cargo: OsString,
    /// The files to sign in place, in order.
    pub files: Vec<PathBuf>,
}

/// Why the arguments gave no [`SignHost`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HostError {
    /// The host is not Windows, where `signtool` runs.
    #[error("signing needs `signtool`, which runs on Windows; this host is {0}")]
    NotWindows(String),
    /// No file, or an option (there are none).
    #[error("{USAGE}")]
    Usage,
}

/// The host from the arguments after the program name, `$CARGO` and the
/// working directory, which relative paths are taken against, on the host
/// `os` (`std::env::consts::OS`).
pub fn sign_host(
    args: Vec<OsString>,
    cargo: Option<OsString>,
    cwd: &Path,
    os: &str,
) -> Result<SignHost, HostError> {
    if os != "windows" {
        return Err(HostError::NotWindows(os.to_owned()));
    }
    if args.is_empty() || args.iter().any(|a| a.to_string_lossy().starts_with('-')) {
        return Err(HostError::Usage);
    }
    Ok(SignHost {
        cargo: cargo.unwrap_or_else(|| OsString::from("cargo")),
        files: args.into_iter().map(|file| cwd.join(file)).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn parse(args: &[&str]) -> Result<SignHost, HostError> {
        sign_host(
            args.iter().map(OsString::from).collect(),
            None,
            &cwd(),
            "windows",
        )
    }

    #[test]
    fn relative_files_are_taken_against_the_working_directory_in_order() {
        let release = Path::new("target")
            .join("x86_64-pc-windows-msvc")
            .join("release");
        let args = vec![
            release.join("nova.exe").into_os_string(),
            release.join("nova-dump.exe").into_os_string(),
        ];
        let host = sign_host(args, Some("C:/cargo.exe".into()), &cwd(), "windows").expect("host");
        assert_eq!(
            host,
            SignHost {
                cargo: "C:/cargo.exe".into(),
                files: vec![
                    cwd().join(&release).join("nova.exe"),
                    cwd().join(&release).join("nova-dump.exe"),
                ],
            }
        );
    }

    #[test]
    fn absolute_files_are_kept_and_cargo_defaults_to_the_path_s() {
        let file = std::env::temp_dir().join("nova.exe");
        let host = sign_host(vec![file.clone().into()], None, &cwd(), "windows").expect("host");
        assert_eq!(host.files, [file]);
        assert_eq!(host.cargo, "cargo");
    }

    #[test]
    fn no_file_or_an_option_is_a_usage_error() {
        for args in [&[][..], &["--help"], &["-h"], &["a.exe", "--stage-only"]] {
            assert_eq!(parse(args), Err(HostError::Usage), "{args:?}");
        }
    }

    #[test]
    fn a_host_other_than_windows_is_refused() {
        for os in ["macos", "linux"] {
            assert_eq!(
                sign_host(vec!["a.exe".into()], None, &cwd(), os),
                Err(HostError::NotWindows(os.to_owned()))
            );
        }
    }
}
