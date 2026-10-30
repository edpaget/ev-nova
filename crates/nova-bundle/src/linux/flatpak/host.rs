//! What `nova-flatpak` reads from its environment, turned into a
//! [`FlatpakHost`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::appcast::Channel;
use crate::linux::host::{PackHostError, check_platform, out_dir};

/// The builder's usage line.
pub const USAGE: &str = "usage: nova-flatpak [OUT_DIR]   \
     (run it as `mise run flatpak -- [OUT_DIR]`)";

/// The tool's name in a refusal.
const TOOL: &str = "the Flatpak builder (flatpak-builder)";

/// What the edge supplies: the `cargo` and the output directory asked
/// for. `flatpak-builder` and `flatpak` are the ones on `PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatpakHost {
    /// The `cargo` to read the spec and vendor the crates with (`$CARGO`
    /// under `cargo run`).
    pub cargo: OsString,
    /// The output directory asked for, if any.
    pub out: Option<PathBuf>,
    /// The release channel it is built for (`NOVA_CHANNEL`, read with
    /// [`crate::channel::from_env`]), whose branch it exports onto;
    /// `None` for a development build.
    pub channel: Option<Channel>,
}

/// The host from the arguments after the program name, `$CARGO`, the
/// channel and the working directory, which a relative output directory
/// is taken against, on the host `os` and `arch` (`std::env::consts`),
/// which must be x86-64 Linux.
pub fn flatpak_host(
    args: Vec<OsString>,
    cargo: Option<OsString>,
    channel: Option<Channel>,
    cwd: &Path,
    os: &str,
    arch: &str,
) -> Result<FlatpakHost, PackHostError> {
    check_platform(TOOL, os, arch)?;
    Ok(FlatpakHost {
        cargo: cargo.unwrap_or_else(|| OsString::from("cargo")),
        out: out_dir(args, cwd).map_err(|_| PackHostError::Usage(USAGE))?,
        channel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn host(args: &[&str]) -> Result<FlatpakHost, PackHostError> {
        let args = args.iter().map(OsString::from).collect();
        flatpak_host(
            args,
            Some("/rust/cargo".into()),
            None,
            &cwd(),
            "linux",
            "x86_64",
        )
    }

    #[test]
    fn the_channel_is_carried_through() {
        let host = flatpak_host(
            vec![],
            None,
            Some(Channel::Nightly),
            &cwd(),
            "linux",
            "x86_64",
        )
        .expect("host");
        assert_eq!(host.channel, Some(Channel::Nightly));
        assert_eq!(self::host(&[]).expect("host").channel, None);
    }

    #[test]
    fn with_no_arguments_it_writes_to_the_default() {
        assert_eq!(
            host(&[]),
            Ok(FlatpakHost {
                cargo: "/rust/cargo".into(),
                out: None,
                channel: None,
            })
        );
        let host = flatpak_host(vec![], None, None, &cwd(), "linux", "x86_64").expect("host");
        assert_eq!(host.cargo, "cargo");
    }

    #[test]
    fn an_output_directory_is_taken_against_the_working_directory() {
        assert_eq!(host(&["dist"]).expect("host").out, Some(cwd().join("dist")));
    }

    #[test]
    fn an_option_or_a_second_directory_is_a_usage_error() {
        for args in [&["--help"][..], &["a", "b"]] {
            assert_eq!(host(args), Err(PackHostError::Usage(USAGE)), "{args:?}");
        }
    }

    #[test]
    fn a_host_other_than_x86_64_linux_is_refused() {
        for (os, arch) in [("macos", "aarch64"), ("windows", "x86_64")] {
            assert_eq!(
                flatpak_host(vec![], None, None, &cwd(), os, arch),
                Err(PackHostError::NotLinux {
                    tool: TOOL,
                    os: os.to_owned(),
                })
            );
        }
        assert_eq!(
            flatpak_host(vec![], None, None, &cwd(), "linux", "aarch64"),
            Err(PackHostError::NotX86_64 {
                tool: TOOL,
                arch: "aarch64".to_owned(),
            })
        );
    }
}
