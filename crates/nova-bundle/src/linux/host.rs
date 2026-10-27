//! What `nova-linux` reads from its environment, turned into a
//! [`LinuxHost`].

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
    let mut out = None;
    for arg in args {
        if arg.to_string_lossy().starts_with('-') || out.is_some() {
            return Err(UsageError);
        }
        out = Some(cwd.join(arg));
    }
    Ok(LinuxHost {
        cargo: cargo.unwrap_or_else(|| OsString::from("cargo")),
        out,
    })
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
}
