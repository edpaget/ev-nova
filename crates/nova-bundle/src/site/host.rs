//! What `nova-site` reads from its arguments, turned into a [`Command`]:
//! write the page (linking each `.flatpakref` the `pages-flatpak` tree
//! given with `--pages-flatpak` serves, and a Beta row from the beta
//! release's asset list given with `--beta-assets`), or print one portable
//! archive's name for `ci.yml`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::archive::{self, Archive};

/// The tool's usage line.
pub const USAGE: &str = "usage: nova-site [--pages-flatpak <DIR>] [--beta-assets <FILE>] \
     [OUT_DIR] | --archive <TARGET> | --archive-dir <TARGET>   (run it as `mise run site -- \
     [OUT_DIR]`)";

/// What `nova-site` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Write the page into this directory.
    Write {
        /// The output directory.
        out: PathBuf,
        /// The `pages-flatpak` tree this deploy serves at `/flatpak/`, if
        /// any (`--pages-flatpak`).
        pages_flatpak: Option<PathBuf>,
        /// The beta release's asset list, if there is a beta release
        /// (`--beta-assets`).
        beta_assets: Option<PathBuf>,
    },
    /// Print this archive's file name.
    ArchiveFile(Archive),
    /// Print this archive's top-level directory.
    ArchiveDir(Archive),
}

/// An unknown option or target, a missing target, or a second argument.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{USAGE}")]
pub struct UsageError;

/// The command from the arguments after the program name and the working
/// directory, which relative paths, and the default `target/site`, are
/// taken against.
pub fn command(args: Vec<OsString>, cwd: &Path) -> Result<Command, UsageError> {
    let archive = |target: Option<&OsString>| {
        target
            .and_then(|t| t.to_str().and_then(archive::for_target))
            .ok_or(UsageError)
    };
    match args.first().and_then(|a| a.to_str()) {
        Some("--archive") if args.len() <= 2 => {
            return archive(args.get(1)).map(Command::ArchiveFile);
        }
        Some("--archive-dir") if args.len() <= 2 => {
            return archive(args.get(1)).map(Command::ArchiveDir);
        }
        _ => {}
    }
    let mut args = args.into_iter();
    let mut pages_flatpak = None;
    let mut beta_assets = None;
    let mut out = None;
    while let Some(arg) = args.next() {
        let name = arg.to_str().map(str::to_owned);
        let (slot, value) = match name.as_deref() {
            Some("--pages-flatpak") => (&mut pages_flatpak, args.next().ok_or(UsageError)?),
            Some("--beta-assets") => (&mut beta_assets, args.next().ok_or(UsageError)?),
            _ if arg.to_string_lossy().starts_with('-') => return Err(UsageError),
            _ => (&mut out, arg),
        };
        if slot.replace(cwd.join(value)).is_some() {
            return Err(UsageError);
        }
    }
    Ok(Command::Write {
        out: out.unwrap_or_else(|| cwd.join("target").join("site")),
        pages_flatpak,
        beta_assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installer;
    use crate::linux;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn write(out: PathBuf) -> Command {
        Command::Write {
            out,
            pages_flatpak: None,
            beta_assets: None,
        }
    }

    #[test]
    fn with_no_arguments_it_writes_to_target_site() {
        assert_eq!(
            command(Vec::new(), &cwd()),
            Ok(write(cwd().join("target").join("site")))
        );
    }

    #[test]
    fn an_output_directory_is_taken_against_the_working_directory() {
        assert_eq!(
            command(args(&["_site"]), &cwd()),
            Ok(write(cwd().join("_site")))
        );
        let absolute = cwd().join("abs");
        assert_eq!(
            command(vec![absolute.clone().into()], &cwd()),
            Ok(write(absolute))
        );
    }

    #[test]
    fn the_pages_tree_and_the_beta_list_come_in_any_order_with_or_without_an_output_dir() {
        let full = Command::Write {
            out: cwd().join("_site"),
            pages_flatpak: Some(cwd().join("pf")),
            beta_assets: Some(cwd().join("beta.txt")),
        };
        for given in [
            &[
                "--pages-flatpak",
                "pf",
                "--beta-assets",
                "beta.txt",
                "_site",
            ][..],
            &[
                "--beta-assets",
                "beta.txt",
                "--pages-flatpak",
                "pf",
                "_site",
            ],
            &[
                "_site",
                "--pages-flatpak",
                "pf",
                "--beta-assets",
                "beta.txt",
            ],
        ] {
            assert_eq!(command(args(given), &cwd()), Ok(full.clone()), "{given:?}");
        }
        assert_eq!(
            command(args(&["--beta-assets", "beta.txt"]), &cwd()),
            Ok(Command::Write {
                out: cwd().join("target").join("site"),
                pages_flatpak: None,
                beta_assets: Some(cwd().join("beta.txt")),
            })
        );
        assert_eq!(
            command(args(&["--pages-flatpak", "pf"]), &cwd()),
            Ok(Command::Write {
                out: cwd().join("target").join("site"),
                pages_flatpak: Some(cwd().join("pf")),
                beta_assets: None,
            })
        );
    }

    #[test]
    fn a_repeated_valueless_or_misplaced_option_is_a_usage_error() {
        for bad in [
            &["--pages-flatpak"][..],
            &["--beta-assets"],
            &["--pages-flatpak", "a", "--pages-flatpak", "b"],
            &["--beta-assets", "a", "--beta-assets", "b"],
            &["--pages-flatpak", "a", "x", "y"],
            &["--pages-flatpak", "a", "--archive", installer::TARGET],
            &["--beta-assets", "a", "--archive-dir", linux::TARGET],
            &["_site", "--archive", installer::TARGET],
            &["--flatpakref", "_site"],
        ] {
            assert_eq!(command(args(bad), &cwd()), Err(UsageError), "{bad:?}");
        }
    }

    #[test]
    fn the_archive_options_name_a_targets_archive() {
        assert_eq!(
            command(args(&["--archive", installer::TARGET]), &cwd()),
            Ok(Command::ArchiveFile(archive::WINDOWS))
        );
        assert_eq!(
            command(args(&["--archive-dir", linux::TARGET]), &cwd()),
            Ok(Command::ArchiveDir(archive::LINUX))
        );
        assert_eq!(
            command(args(&["--archive", linux::TARGET]), &cwd()),
            Ok(Command::ArchiveFile(archive::LINUX))
        );
        assert_eq!(
            command(args(&["--archive-dir", installer::TARGET]), &cwd()),
            Ok(Command::ArchiveDir(archive::WINDOWS))
        );
    }

    #[test]
    fn a_bad_target_option_or_extra_argument_is_a_usage_error() {
        for bad in [
            vec!["--archive", "aarch64-apple-darwin"],
            vec!["--archive-dir", "junk"],
            vec!["--archive"],
            vec!["--archive-dir"],
            vec!["--archive", installer::TARGET, "extra"],
            vec!["--archive-dir", linux::TARGET, "extra"],
            vec!["a", "b"],
            vec!["--help"],
            vec!["-o"],
        ] {
            assert_eq!(command(args(&bad), &cwd()), Err(UsageError), "{bad:?}");
        }
    }

    #[test]
    fn the_usage_error_shows_the_usage() {
        assert_eq!(UsageError.to_string(), USAGE);
        assert!(USAGE.contains("--pages-flatpak <DIR>"), "{USAGE}");
        assert!(USAGE.contains("--beta-assets <FILE>"), "{USAGE}");
    }
}
