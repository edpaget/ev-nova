//! What `nova-linux` reads from its environment, turned into a
//! [`LinuxHost`], and what the Flatpak tool takes of its own:
//! one optional output directory, and an x86-64 Linux host
//! ([`check_platform`]).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The writer's usage line.
pub const USAGE: &str = "usage: nova-linux [OUT_DIR]   \
     (run it as `mise run linux-desktop -- [OUT_DIR]`)";

/// What the edge supplies: the `cargo` to read the spec with and the
/// output directory asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxHost {
    /// The `cargo` whose `cargo metadata` gives the spec (`$CARGO` under
    /// `cargo run`).
    pub cargo: OsString,
    /// The output directory asked for, if any.
    pub out: Option<PathBuf>,
}

/// An option, or more than one output directory.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{USAGE}")]
pub struct UsageError;

/// The host from the arguments after the program name, `$CARGO` and the
/// working directory, which a relative output directory is taken
/// against. Any host will do: rendering needs no Linux tools.
pub fn linux_host(
    args: Vec<OsString>,
    cargo: Option<OsString>,
    cwd: &Path,
) -> Result<LinuxHost, UsageError> {
    Ok(LinuxHost {
        cargo: cargo.unwrap_or_else(|| OsString::from("cargo")),
        out: out_dir(args, cwd)?,
    })
}

/// The one optional output directory in `args`, taken against `cwd`. An
/// option, or a second directory, is a usage error.
pub fn out_dir(args: Vec<OsString>, cwd: &Path) -> Result<Option<PathBuf>, UsageError> {
    let mut out = None;
    for arg in args {
        if arg.to_string_lossy().starts_with('-') || out.is_some() {
            return Err(UsageError);
        }
        out = Some(cwd.join(arg));
    }
    Ok(out)
}

/// The one host the Flatpak tool runs on.
pub const OS: &str = "linux";
/// The one architecture they build for: the asset names say `x86_64`.
pub const ARCH: &str = "x86_64";

/// Why the environment gave no host to pack a Linux download on.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PackHostError {
    /// The host is not Linux, where the tool runs.
    #[error("{tool} runs on Linux; this host is {os}")]
    NotLinux {
        /// The tool that refused.
        tool: &'static str,
        /// `std::env::consts::OS`.
        os: String,
    },
    /// The host is not x86-64, the one architecture packed.
    #[error("{tool} builds for x86_64 alone; this host is {arch}")]
    NotX86_64 {
        /// The tool that refused.
        tool: &'static str,
        /// `std::env::consts::ARCH`.
        arch: String,
    },
    /// An option, or more than one output directory.
    #[error("{0}")]
    Usage(&'static str),
}

/// Refuses any host but x86-64 Linux for `tool`, given the host's `os`
/// and `arch` (`std::env::consts::OS` and `ARCH`).
pub fn check_platform(tool: &'static str, os: &str, arch: &str) -> Result<(), PackHostError> {
    if os != OS {
        return Err(PackHostError::NotLinux {
            tool,
            os: os.to_owned(),
        });
    }
    if arch != ARCH {
        return Err(PackHostError::NotX86_64 {
            tool,
            arch: arch.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    #[test]
    fn with_no_arguments_it_writes_to_the_default() {
        let host = linux_host(Vec::new(), Some("/rust/cargo".into()), &cwd()).expect("host");
        assert_eq!(
            host,
            LinuxHost {
                cargo: "/rust/cargo".into(),
                out: None,
            }
        );
    }

    #[test]
    fn without_cargo_set_it_runs_the_one_on_the_path() {
        let host = linux_host(Vec::new(), None, &cwd()).expect("host");
        assert_eq!(host.cargo, "cargo");
    }

    #[test]
    fn an_output_directory_is_taken_against_the_working_directory() {
        let host = linux_host(vec!["dist/x".into()], None, &cwd()).expect("host");
        assert_eq!(host.out, Some(cwd().join("dist/x")));
        let absolute = cwd().join("abs");
        let host = linux_host(vec![absolute.clone().into()], None, &cwd()).expect("host");
        assert_eq!(host.out, Some(absolute));
    }

    #[test]
    fn an_option_or_a_second_directory_is_a_usage_error() {
        for args in [vec!["--help"], vec!["a", "b"], vec!["-o"]] {
            let args = args.into_iter().map(OsString::from).collect();
            assert_eq!(linux_host(args, None, &cwd()), Err(UsageError));
        }
    }

    #[test]
    fn x86_64_linux_is_the_one_host_packed_on() {
        assert_eq!(check_platform("tool", "linux", "x86_64"), Ok(()));
        for os in ["macos", "windows", "freebsd"] {
            assert_eq!(
                check_platform("tool", os, "x86_64"),
                Err(PackHostError::NotLinux {
                    tool: "tool",
                    os: os.to_owned(),
                })
            );
        }
        assert_eq!(
            check_platform("tool", "linux", "aarch64"),
            Err(PackHostError::NotX86_64 {
                tool: "tool",
                arch: "aarch64".to_owned(),
            })
        );
    }

    #[test]
    fn the_refusals_name_the_tool_and_the_host() {
        let error = check_platform("the Flatpak builder", "macos", "aarch64").expect_err("no");
        assert_eq!(
            error.to_string(),
            "the Flatpak builder runs on Linux; this host is macos"
        );
        let error = check_platform("the Flatpak builder", "linux", "aarch64").expect_err("no");
        assert_eq!(
            error.to_string(),
            "the Flatpak builder builds for x86_64 alone; this host is aarch64"
        );
    }
}
