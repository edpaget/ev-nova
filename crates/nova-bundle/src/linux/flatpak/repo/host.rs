//! What `nova-flatpak-repo` reads from its arguments, turned into a
//! [`Command`], and what each outcome prints and exits with
//! ([`report`]).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::plan::Outcome;
use super::probe::Live;
use super::{NOT_PUBLISHED, RepoError, SignPaths, StagePaths};
use crate::linux::host::{PackHostError, check_platform};

/// The tool's usage line.
pub const USAGE: &str = "usage: nova-flatpak-repo stage --bundle-dir <DIR> --work <DIR> \
     --pages <DIR> | nova-flatpak-repo sign --work <DIR> --pages <DIR> --dist <DIR>   \
     (run it as `mise run flatpak-repo -- ...`)";

/// The tool's name in a refusal.
const TOOL: &str = "the Flatpak repo publisher (flatpak, ostree)";

/// The exit status of a usage error, an unknown channel or the wrong
/// host.
pub const USAGE_STATUS: u8 = 2;

/// The exit status of a failure.
pub const FAILURE_STATUS: u8 = 1;

/// The warning's title, which CI shows on the run.
pub const WARNING_TITLE: &str = "Flatpak repo not published";

/// What `nova-flatpak-repo` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Mirror the live repo and import the plain bundle.
    Stage(StagePaths),
    /// Sign, prune and write the `.flatpakref` and the bundle.
    Sign(SignPaths),
}

/// The command from the arguments after the program name and the working
/// directory, which relative paths are taken against, on the host `os`
/// and `arch` (`std::env::consts`), which must be x86-64 Linux.
///
/// # Errors
///
/// The wrong host, an unknown subcommand, or a missing, repeated or
/// unknown option.
pub fn command(
    args: Vec<OsString>,
    cwd: &Path,
    os: &str,
    arch: &str,
) -> Result<Command, PackHostError> {
    check_platform(TOOL, os, arch)?;
    let mut args = args.into_iter();
    let usage = PackHostError::Usage(USAGE);
    match args.next().as_ref().and_then(|a| a.to_str()) {
        Some("stage") => {
            let [bundle_dir, work, pages] =
                options(args, cwd, ["--bundle-dir", "--work", "--pages"]).ok_or(usage)?;
            Ok(Command::Stage(StagePaths {
                bundle_dir,
                work,
                pages,
            }))
        }
        Some("sign") => {
            let [work, pages, dist] =
                options(args, cwd, ["--work", "--pages", "--dist"]).ok_or(usage)?;
            Ok(Command::Sign(SignPaths { work, pages, dist }))
        }
        _ => Err(usage),
    }
}

/// Each of `names`, exactly once, in any order, with a path taken
/// against `cwd`; `None` for anything else.
fn options<const N: usize>(
    mut args: impl Iterator<Item = OsString>,
    cwd: &Path,
    names: [&str; N],
) -> Option<[PathBuf; N]> {
    let mut given: [Option<PathBuf>; N] = std::array::from_fn(|_| None);
    while let Some(option) = args.next() {
        let index = names.iter().position(|n| option.to_str() == Some(n))?;
        let value = args.next()?;
        if given[index].replace(cwd.join(value)).is_some() {
            return None;
        }
    }
    let all: Vec<PathBuf> = given.into_iter().collect::<Option<_>>()?;
    all.try_into().ok()
}

/// What `main` prints and exits with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The exit status.
    pub status: u8,
    /// The lines for standard output, which CI reads.
    pub stdout: Vec<String>,
    /// The lines for standard error.
    pub stderr: Vec<String>,
}

/// What `command`'s `result` prints and exits with.
///
/// `stage` decides the skip: it prints the one `::warning::` and exits
/// [`NOT_PUBLISHED`], which CI turns into `staged=false` and runs no later
/// step. `sign`'s own skip is a guard that CI never reaches: it exits
/// [`NOT_PUBLISHED`] with a plain line, so no second warning is shown. A
/// first publish prints a `::notice::`.
#[must_use]
pub fn report(command: &Command, result: &Result<Outcome, RepoError>) -> Report {
    let ok = |stdout: Vec<String>| Report {
        status: 0,
        stdout,
        stderr: Vec::new(),
    };
    match (command, result) {
        (_, Err(e)) => Report {
            status: FAILURE_STATUS,
            stdout: Vec::new(),
            stderr: vec![format!("nova-flatpak-repo: {e}")],
        },
        (Command::Stage(_), Ok(Outcome::Skipped(skip))) => Report {
            status: NOT_PUBLISHED,
            stdout: vec![format!(
                "::warning title={WARNING_TITLE}::{}",
                skip.message()
            )],
            stderr: Vec::new(),
        },
        (Command::Sign(_), Ok(Outcome::Skipped(skip))) => Report {
            status: NOT_PUBLISHED,
            stdout: vec![format!("{WARNING_TITLE}: {}", skip.message())],
            stderr: Vec::new(),
        },
        (_, Ok(Outcome::Staged { url, live })) => ok(match live {
            Live::Absent => vec![format!(
                "::notice title=First Flatpak publish::{url} has no repo yet: starting an \
                 empty one"
            )],
            Live::Present => vec![format!("mirrored {url}")],
        }),
        (_, Ok(Outcome::Published { flatpakref, bundle })) => ok(vec![
            flatpakref.display().to_string(),
            bundle.display().to_string(),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux::flatpak::repo::plan::Skip;
    use crate::linux::flatpak::repo::probe::ProbeError;

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn parse(args: &[&str]) -> Result<Command, PackHostError> {
        let args = args.iter().map(OsString::from).collect();
        command(args, &cwd(), "linux", "x86_64")
    }

    fn stage_paths() -> StagePaths {
        StagePaths {
            bundle_dir: cwd().join("plain"),
            work: cwd().join("w"),
            pages: cwd().join("p"),
        }
    }

    fn sign_paths() -> SignPaths {
        SignPaths {
            work: cwd().join("w"),
            pages: cwd().join("p"),
            dist: cwd().join("d"),
        }
    }

    #[test]
    fn stage_takes_the_bundle_dir_work_and_pages_against_the_cwd() {
        assert_eq!(
            parse(&[
                "stage",
                "--bundle-dir",
                "plain",
                "--work",
                "w",
                "--pages",
                "p"
            ]),
            Ok(Command::Stage(stage_paths()))
        );
        let absolute = Path::new("/abs").join("w");
        let args = vec![
            "stage".into(),
            "--pages".into(),
            "p".into(),
            "--work".into(),
            absolute.clone().into_os_string(),
            "--bundle-dir".into(),
            "plain".into(),
        ];
        assert_eq!(
            command(args, &cwd(), "linux", "x86_64"),
            Ok(Command::Stage(StagePaths {
                work: absolute,
                ..stage_paths()
            }))
        );
    }

    #[test]
    fn sign_takes_the_work_pages_and_dist_against_the_cwd() {
        assert_eq!(
            parse(&["sign", "--dist", "d", "--work", "w", "--pages", "p"]),
            Ok(Command::Sign(sign_paths()))
        );
    }

    #[test]
    fn a_missing_extra_repeated_or_unknown_option_is_a_usage_error() {
        for args in [
            &[][..],
            &["publish"],
            &["stage"],
            &["stage", "--work", "w", "--pages", "p"],
            &[
                "stage",
                "--bundle-dir",
                "b",
                "--work",
                "w",
                "--pages",
                "p",
                "--dist",
                "d",
            ],
            &["stage", "--bundle-dir", "b", "--work", "w", "--pages"],
            &[
                "stage",
                "--bundle-dir",
                "b",
                "--work",
                "w",
                "--work",
                "x",
                "--pages",
                "p",
            ],
            &["sign", "--work", "w", "--pages", "p"],
            &[
                "sign", "--work", "w", "--pages", "p", "--dist", "d", "extra",
            ],
            &["sign", "--bundle-dir", "b", "--work", "w", "--pages", "p"],
        ] {
            assert_eq!(parse(args), Err(PackHostError::Usage(USAGE)), "{args:?}");
        }
    }

    #[test]
    fn a_host_other_than_x86_64_linux_is_refused() {
        let args = || vec!["sign".into()];
        assert_eq!(
            command(args(), &cwd(), "macos", "aarch64"),
            Err(PackHostError::NotLinux {
                tool: TOOL,
                os: "macos".to_owned(),
            })
        );
        assert_eq!(
            command(args(), &cwd(), "linux", "aarch64"),
            Err(PackHostError::NotX86_64 {
                tool: TOOL,
                arch: "aarch64".to_owned(),
            })
        );
    }

    #[test]
    fn stage_s_skip_warns_once_and_exits_not_published() {
        for skip in [Skip::NoChannel, Skip::NoKey, Skip::NoSecret] {
            let report = report(&Command::Stage(stage_paths()), &Ok(Outcome::Skipped(skip)));
            assert_eq!(
                report,
                Report {
                    status: 4,
                    stdout: vec![format!(
                        "::warning title=Flatpak repo not published::{}",
                        skip.message()
                    )],
                    stderr: Vec::new(),
                }
            );
        }
    }

    #[test]
    fn sign_s_skip_exits_not_published_without_a_second_warning() {
        let report = report(
            &Command::Sign(sign_paths()),
            &Ok(Outcome::Skipped(Skip::NoSecret)),
        );
        assert_eq!(report.status, NOT_PUBLISHED);
        assert!(
            !report
                .stdout
                .iter()
                .chain(&report.stderr)
                .any(|l| l.contains("::warning")),
            "{report:?}"
        );
        assert_eq!(
            report.stdout,
            [format!(
                "Flatpak repo not published: {}",
                Skip::NoSecret.message()
            )]
        );
    }

    #[test]
    fn a_first_publish_is_a_notice_and_a_mirror_a_plain_line() {
        let url = "https://example.com/r/".to_owned();
        let first = report(
            &Command::Stage(stage_paths()),
            &Ok(Outcome::Staged {
                url: url.clone(),
                live: Live::Absent,
            }),
        );
        assert_eq!(first.status, 0);
        assert_eq!(
            first.stdout,
            [
                "::notice title=First Flatpak publish::https://example.com/r/ has no repo yet: \
              starting an empty one"
            ]
        );
        let mirrored = report(
            &Command::Stage(stage_paths()),
            &Ok(Outcome::Staged {
                url,
                live: Live::Present,
            }),
        );
        assert_eq!(mirrored.status, 0);
        assert_eq!(mirrored.stdout, ["mirrored https://example.com/r/"]);
    }

    #[test]
    fn a_publish_prints_the_ref_and_the_bundle() {
        let flatpakref = cwd().join("p").join("ev-nova-nightly.flatpakref");
        let bundle = cwd().join("d").join("x.flatpak");
        let report = report(
            &Command::Sign(sign_paths()),
            &Ok(Outcome::Published {
                flatpakref: flatpakref.clone(),
                bundle: bundle.clone(),
            }),
        );
        assert_eq!(report.status, 0);
        assert_eq!(
            report.stdout,
            [
                flatpakref.display().to_string(),
                bundle.display().to_string()
            ]
        );
    }

    #[test]
    fn a_failure_exits_1_with_the_error_on_standard_error() {
        for command in [Command::Stage(stage_paths()), Command::Sign(sign_paths())] {
            let error = RepoError::Probe(ProbeError::Answer {
                url: "u".into(),
                answer: "500".into(),
            });
            let report = report(&command, &Err(error));
            assert_eq!(report.status, FAILURE_STATUS);
            assert!(report.stdout.is_empty());
            assert_eq!(report.stderr.len(), 1);
            assert!(
                report.stderr[0].starts_with("nova-flatpak-repo: u answered `500`"),
                "{report:?}"
            );
        }
    }
}
