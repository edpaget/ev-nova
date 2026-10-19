//! What `main` reads from its environment, turned into a [`Host`].

use std::ffi::OsString;
use std::path::Path;

use crate::bundle::Host;

/// The bundler's usage line.
pub const USAGE: &str = "usage: nova-bundle [OUT_DIR]   (run it as `mise run bundle -- [OUT_DIR]`)";

/// Why the environment gave no [`Host`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HostError {
    /// The host is not macOS, which has the tools bundling runs.
    #[error("bundling needs macOS's lipo, codesign and hdiutil; this host is {0}")]
    NotMacOs(String),
    /// More than one argument, or an option.
    #[error("{USAGE}")]
    Usage,
    /// `MACOSX_DEPLOYMENT_TARGET` is unset, so the bundler wasn't run
    /// through cargo (whose `.cargo/config.toml` sets it).
    #[error(
        "MACOSX_DEPLOYMENT_TARGET is unset: run the bundler through cargo \
         (`mise run bundle`), whose .cargo/config.toml sets it"
    )]
    NoDeploymentTarget,
}

/// The host from the arguments after the program name, `$CARGO`,
/// `$MACOSX_DEPLOYMENT_TARGET` and the working directory, which a relative
/// output directory is taken against, on the host `os`
/// (`std::env::consts::OS`).
pub fn host(
    args: Vec<OsString>,
    cargo: Option<OsString>,
    deployment_target: Option<String>,
    cwd: &Path,
    os: &str,
) -> Result<Host, HostError> {
    if os != "macos" {
        return Err(HostError::NotMacOs(os.to_owned()));
    }
    let mut args = args.into_iter();
    let out = args.next();
    if args.next().is_some()
        || out
            .as_ref()
            .is_some_and(|a| a.to_string_lossy().starts_with('-'))
    {
        return Err(HostError::Usage);
    }
    let min_system = deployment_target
        .filter(|t| !t.is_empty())
        .ok_or(HostError::NoDeploymentTarget)?;
    Ok(Host {
        cargo: cargo.unwrap_or_else(|| OsString::from("cargo")),
        min_system,
        out: out.map(|out| cwd.join(out)),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_argument_leaves_the_default_output() {
        let host = host(
            vec![],
            Some("/bin/cargo".into()),
            Some("13.0".into()),
            &cwd(),
            "macos",
        )
        .expect("host");
        assert_eq!(
            host,
            Host {
                cargo: "/bin/cargo".into(),
                min_system: "13.0".into(),
                out: None,
            }
        );
    }

    #[test]
    fn a_relative_output_is_taken_against_the_working_directory() {
        let host = host(os(&["dist"]), None, Some("13.0".into()), &cwd(), "macos").expect("host");
        assert_eq!(host.out, Some(cwd().join("dist")));
    }

    #[test]
    fn an_absolute_output_is_kept() {
        let out = std::env::temp_dir().join("nova-out");
        let host = host(
            vec![out.clone().into()],
            None,
            Some("13.0".into()),
            &cwd(),
            "macos",
        )
        .expect("host");
        assert_eq!(host.out, Some(out));
    }

    #[test]
    fn cargo_defaults_to_the_one_on_the_path() {
        let host = host(vec![], None, Some("13.0".into()), &cwd(), "macos").expect("host");
        assert_eq!(host.cargo, "cargo");
    }

    #[test]
    fn two_arguments_or_an_option_are_a_usage_error() {
        for args in [os(&["a", "b"]), os(&["--help"]), os(&["-h"])] {
            assert_eq!(
                host(args.clone(), None, Some("13.0".into()), &cwd(), "macos"),
                Err(HostError::Usage),
                "{args:?}"
            );
        }
    }

    #[test]
    fn an_unset_or_empty_deployment_target_is_an_error() {
        for target in [None, Some(String::new())] {
            assert_eq!(
                host(vec![], None, target, &cwd(), "macos"),
                Err(HostError::NoDeploymentTarget)
            );
        }
    }

    #[test]
    fn a_host_other_than_macos_is_refused() {
        for os in ["linux", "windows"] {
            assert_eq!(
                host(vec![], None, Some("13.0".into()), &cwd(), os),
                Err(HostError::NotMacOs(os.to_owned()))
            );
        }
    }
}
