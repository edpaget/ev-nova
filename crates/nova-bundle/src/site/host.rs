//! What `nova-site` reads from its arguments, turned into a [`Command`]:
//! write the page, or print one portable archive's name for `ci.yml`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::archive::{self, Archive};

/// The tool's usage line.
pub const USAGE: &str = "usage: nova-site [OUT_DIR] | --archive <TARGET> | --archive-dir <TARGET>   \
     (run it as `mise run site -- [OUT_DIR]`)";

/// What `nova-site` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Write the page into this directory.
    Write {
        /// The output directory.
        out: PathBuf,
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
/// directory, which a relative output directory, and the default
/// `target/site`, are taken against.
pub fn command(args: Vec<OsString>, cwd: &Path) -> Result<Command, UsageError> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Command::Write {
            out: cwd.join("target").join("site"),
        });
    };
    let archive_of = |target: Option<OsString>| {
        target
            .and_then(|t| t.to_str().and_then(archive::for_target))
            .ok_or(UsageError)
    };
    let command = match first.to_str() {
        Some("--archive") => Command::ArchiveFile(archive_of(args.next())?),
        Some("--archive-dir") => Command::ArchiveDir(archive_of(args.next())?),
        _ if first.to_string_lossy().starts_with('-') => return Err(UsageError),
        _ => Command::Write {
            out: cwd.join(first),
        },
    };
    if args.next().is_some() {
        return Err(UsageError);
    }
    Ok(command)
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

    #[test]
    fn with_no_arguments_it_writes_to_target_site() {
        assert_eq!(
            command(Vec::new(), &cwd()),
            Ok(Command::Write {
                out: cwd().join("target").join("site"),
            })
        );
    }

    #[test]
    fn an_output_directory_is_taken_against_the_working_directory() {
        assert_eq!(
            command(args(&["_site"]), &cwd()),
            Ok(Command::Write {
                out: cwd().join("_site"),
            })
        );
        let absolute = cwd().join("abs");
        assert_eq!(
            command(vec![absolute.clone().into()], &cwd()),
            Ok(Command::Write { out: absolute })
        );
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
    }
}
