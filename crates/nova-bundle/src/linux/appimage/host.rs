//! What `nova-appimage` reads from its environment, turned into an
//! [`AppImageHost`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::linux::host::{PackHostError, check_platform, out_dir};

/// The builder's usage line.
pub const USAGE: &str = "usage: nova-appimage [OUT_DIR]   \
     (run it as `mise run appimage -- [OUT_DIR]`)";

/// The tool's name in a refusal.
const TOOL: &str = "the AppImage builder (appimagetool)";

/// What the edge supplies: the tools, the runtime and the output
/// directory asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppImageHost {
    /// The `cargo` to read the spec and build with (`$CARGO` under
    /// `cargo run`).
    pub cargo: OsString,
    /// The `appimagetool` to run (`$NOVA_APPIMAGETOOL`, else the one on
    /// `PATH`).
    pub appimagetool: OsString,
    /// The type2 runtime to embed (`$NOVA_APPIMAGE_RUNTIME`), if given.
    pub runtime: Option<PathBuf>,
    /// The output directory asked for, if any.
    pub out: Option<PathBuf>,
}

/// The host from the arguments after the program name, `$CARGO`,
/// `$NOVA_APPIMAGETOOL` (run as given, like a program on `PATH`) and
/// `$NOVA_APPIMAGE_RUNTIME` (each ignored when empty), and the working
/// directory, which the runtime and output directory are taken against, on the host `os` and `arch` (`std::env::consts`), which must
/// be x86-64 Linux.
pub fn appimage_host(
    args: Vec<OsString>,
    cargo: Option<OsString>,
    appimagetool: Option<OsString>,
    runtime: Option<OsString>,
    cwd: &Path,
    os: &str,
    arch: &str,
) -> Result<AppImageHost, PackHostError> {
    check_platform(TOOL, os, arch)?;
    let out = out_dir(args, cwd).map_err(|_| PackHostError::Usage(USAGE))?;
    let set = |value: Option<OsString>| value.filter(|v| !v.is_empty());
    Ok(AppImageHost {
        cargo: cargo.unwrap_or_else(|| OsString::from("cargo")),
        appimagetool: set(appimagetool).unwrap_or_else(|| OsString::from("appimagetool")),
        runtime: set(runtime).map(|runtime| cwd.join(runtime)),
        out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn host(
        args: &[&str],
        tool: Option<&str>,
        runtime: Option<&str>,
    ) -> Result<AppImageHost, PackHostError> {
        appimage_host(
            args.iter().map(OsString::from).collect(),
            Some("/rust/cargo".into()),
            tool.map(OsString::from),
            runtime.map(OsString::from),
            &cwd(),
            "linux",
            "x86_64",
        )
    }

    #[test]
    fn with_nothing_set_it_runs_the_tool_on_the_path_and_has_no_runtime() {
        assert_eq!(
            host(&[], None, None),
            Ok(AppImageHost {
                cargo: "/rust/cargo".into(),
                appimagetool: "appimagetool".into(),
                runtime: None,
                out: None,
            })
        );
    }

    #[test]
    fn empty_variables_are_unset() {
        let host = host(&[], Some(""), Some("")).expect("host");
        assert_eq!(host.appimagetool, "appimagetool");
        assert_eq!(host.runtime, None);
    }

    #[test]
    fn the_runtime_and_output_are_taken_against_the_working_directory() {
        let host = host(&["dist"], Some("/opt/appimagetool"), Some("tools/runtime")).expect("host");
        assert_eq!(host.appimagetool, "/opt/appimagetool");
        assert_eq!(host.runtime, Some(cwd().join("tools/runtime")));
        assert_eq!(host.out, Some(cwd().join("dist")));
    }

    #[test]
    fn without_cargo_set_it_runs_the_one_on_the_path() {
        let host = appimage_host(vec![], None, None, None, &cwd(), "linux", "x86_64");
        assert_eq!(host.expect("host").cargo, "cargo");
    }

    #[test]
    fn an_option_or_a_second_directory_is_a_usage_error() {
        for args in [&["--help"][..], &["a", "b"], &["-o"]] {
            assert_eq!(
                host(args, None, None),
                Err(PackHostError::Usage(USAGE)),
                "{args:?}"
            );
        }
    }

    #[test]
    fn a_host_other_than_x86_64_linux_is_refused() {
        for (os, arch) in [
            ("macos", "aarch64"),
            ("windows", "x86_64"),
            ("linux", "aarch64"),
        ] {
            let refused = appimage_host(vec![], None, None, None, &cwd(), os, arch);
            assert!(
                matches!(
                    refused,
                    Err(PackHostError::NotLinux { .. } | PackHostError::NotX86_64 { .. })
                ),
                "{os} {arch}: {refused:?}"
            );
        }
        let refused = appimage_host(
            vec!["--bad".into()],
            None,
            None,
            None,
            &cwd(),
            "macos",
            "x86_64",
        );
        assert!(
            matches!(refused, Err(PackHostError::NotLinux { .. })),
            "the host is checked first: {refused:?}"
        );
    }
}
