//! What `nova-appcast` reads from its arguments, turned into a
//! [`Command`], and the exit status each outcome gives.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::promote::{PromoteError, PromotionPaths};
use super::publish::{AppcastError, PublishPaths};

/// The tool's usage line.
pub const USAGE: &str = "usage: nova-appcast publish --dist <DIR> --assets <FILE> \
     --current <DIR> --feeds <DIR> --prune-list <FILE> | nova-appcast check-promotion \
     --build <ID> --repo <DIR> --nightly-assets <FILE> --beta-assets <FILE> \
     --beta-current <DIR> | nova-appcast public-key   (run it as `mise run appcast -- ...`)";

/// The exit status of a usage error.
pub const USAGE_STATUS: u8 = 2;

/// The exit status of a failure.
pub const FAILURE_STATUS: u8 = 1;

/// The exit status when the current feeds already offer this very build
/// (a re-run of a publish whose feeds went up): CI carries on without
/// uploading feeds. An older build is a failure.
pub const ALREADY_PUBLISHED: u8 = 3;

/// What `nova-appcast` is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Publish the feeds, over these paths.
    Publish(PublishPaths),
    /// Check that a nightly build may be promoted to beta, and find its
    /// commit.
    CheckPromotion {
        /// `--build`'s text, as given: the check itself refuses a
        /// malformed one, after the main-only check.
        build: String,
        /// Where it reads.
        paths: PromotionPaths,
    },
    /// Print the public key of the seed in `UPDATE_SIGNING_KEY`.
    PublicKey,
}

/// A missing, repeated or unknown option or subcommand, or a stray
/// argument.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{USAGE}")]
pub struct UsageError;

/// The command from the arguments after the program name and the working
/// directory, which relative paths are taken against.
pub fn command(args: Vec<OsString>, cwd: &Path) -> Result<Command, UsageError> {
    let mut args = args.into_iter();
    match args.next().as_ref().and_then(|a| a.to_str()) {
        Some("public-key") => match args.next() {
            None => Ok(Command::PublicKey),
            Some(_) => Err(UsageError),
        },
        Some("publish") => publish(args, cwd).map(Command::Publish),
        Some("check-promotion") => check_promotion(args, cwd),
        _ => Err(UsageError),
    }
}

/// The values of `OPTIONS`, each given exactly once, in any order.
fn options<const N: usize>(
    mut args: impl Iterator<Item = OsString>,
    options: [&str; N],
) -> Result<[OsString; N], UsageError> {
    let mut given: [Option<OsString>; N] = std::array::from_fn(|_| None);
    while let Some(option) = args.next() {
        let index = options
            .iter()
            .position(|o| option.to_str() == Some(o))
            .ok_or(UsageError)?;
        let value = args.next().ok_or(UsageError)?;
        if given[index].replace(value).is_some() {
            return Err(UsageError);
        }
    }
    let mut values = Vec::with_capacity(N);
    for value in given {
        values.push(value.ok_or(UsageError)?);
    }
    values.try_into().map_err(|_| UsageError)
}

/// `publish`'s five options, each exactly once, in any order.
fn publish(args: impl Iterator<Item = OsString>, cwd: &Path) -> Result<PublishPaths, UsageError> {
    let [dist, assets, current, feeds, prune_list] = options(
        args,
        ["--dist", "--assets", "--current", "--feeds", "--prune-list"],
    )?
    .map(|value| cwd.join(value));
    Ok(PublishPaths {
        dist,
        assets,
        current,
        feeds,
        prune_list,
    })
}

/// `check-promotion`'s five options, each exactly once, in any order. The
/// build stays text; the paths are taken against `cwd`.
fn check_promotion(
    args: impl Iterator<Item = OsString>,
    cwd: &Path,
) -> Result<Command, UsageError> {
    let [build, repo, nightly_assets, beta_assets, beta_current] = options(
        args,
        [
            "--build",
            "--repo",
            "--nightly-assets",
            "--beta-assets",
            "--beta-current",
        ],
    )?;
    let path = |value: OsString| -> PathBuf { cwd.join(value) };
    Ok(Command::CheckPromotion {
        build: build.to_string_lossy().into_owned(),
        paths: PromotionPaths {
            repo: path(repo),
            nightly_assets: path(nightly_assets),
            beta_assets: path(beta_assets),
            beta_current: path(beta_current),
        },
    })
}

/// The exit status of a failed run: [`ALREADY_PUBLISHED`] when the feeds
/// already offer this build, else [`FAILURE_STATUS`].
#[must_use]
pub fn exit_status(error: &AppcastError) -> u8 {
    if error.is_already_published() {
        ALREADY_PUBLISHED
    } else {
        FAILURE_STATUS
    }
}

/// The exit status of a refused promotion: every refusal, a malformed
/// `--build` included, is a [`FAILURE_STATUS`]. Only a usage error is
/// [`USAGE_STATUS`].
#[must_use]
pub fn promotion_exit_status(_error: &PromoteError) -> u8 {
    FAILURE_STATUS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_id::{BuildId, BuildIdError};

    fn cwd() -> PathBuf {
        Path::new("/work").join("ev-nova")
    }

    fn args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn publish_args() -> Vec<&'static str> {
        vec![
            "publish",
            "--dist",
            "dist",
            "--assets",
            "assets.txt",
            "--current",
            "current",
            "--feeds",
            "feeds",
            "--prune-list",
            "prune.txt",
        ]
    }

    #[test]
    fn publish_takes_its_paths_against_the_working_directory() {
        assert_eq!(
            command(args(&publish_args()), &cwd()),
            Ok(Command::Publish(PublishPaths {
                dist: cwd().join("dist"),
                assets: cwd().join("assets.txt"),
                current: cwd().join("current"),
                feeds: cwd().join("feeds"),
                prune_list: cwd().join("prune.txt"),
            }))
        );
        let absolute = std::env::temp_dir().join("feeds");
        let mut given = args(&publish_args());
        given[8] = absolute.clone().into();
        let Ok(Command::Publish(paths)) = command(given, &cwd()) else {
            panic!("publishes");
        };
        assert_eq!(paths.feeds, absolute);
    }

    #[test]
    fn publish_s_options_come_in_any_order() {
        let given = args(&[
            "publish",
            "--prune-list",
            "p",
            "--feeds",
            "f",
            "--current",
            "c",
            "--assets",
            "a",
            "--dist",
            "d",
        ]);
        assert_eq!(
            command(given, &cwd()),
            Ok(Command::Publish(PublishPaths {
                dist: cwd().join("d"),
                assets: cwd().join("a"),
                current: cwd().join("c"),
                feeds: cwd().join("f"),
                prune_list: cwd().join("p"),
            }))
        );
    }

    #[test]
    fn public_key_takes_no_arguments() {
        assert_eq!(
            command(args(&["public-key"]), &cwd()),
            Ok(Command::PublicKey)
        );
        assert_eq!(
            command(args(&["public-key", "extra"]), &cwd()),
            Err(UsageError)
        );
    }

    #[test]
    fn a_missing_or_unknown_option_or_subcommand_is_a_usage_error() {
        let full = publish_args();
        let mut bad: Vec<Vec<&str>> = vec![
            vec![],
            vec!["--help"],
            vec!["sign"],
            vec!["publish"],
            vec!["publish", "--dist"],
        ];
        // Each option left out, in turn.
        for skip in (1..full.len()).step_by(2) {
            let mut without = full.clone();
            without.drain(skip..skip + 2);
            bad.push(without);
        }
        let mut repeated = full.clone();
        repeated.extend(["--dist", "again"]);
        bad.push(repeated);
        let mut unknown = full.clone();
        unknown.extend(["--other", "x"]);
        bad.push(unknown);
        let mut stray = full.clone();
        stray.push("stray");
        bad.push(stray);
        for bad in bad {
            assert_eq!(command(args(&bad), &cwd()), Err(UsageError), "{bad:?}");
        }
    }

    #[test]
    fn the_usage_error_shows_the_usage() {
        assert_eq!(UsageError.to_string(), USAGE);
        assert!(USAGE.contains("publish") && USAGE.contains("public-key"));
        assert!(USAGE.contains("check-promotion --build <ID>"), "{USAGE}");
    }

    fn check_args() -> Vec<&'static str> {
        vec![
            "check-promotion",
            "--build",
            "20261028123456",
            "--repo",
            ".",
            "--nightly-assets",
            "nightly.txt",
            "--beta-assets",
            "beta.txt",
            "--beta-current",
            "beta-current",
        ]
    }

    #[test]
    fn check_promotion_keeps_the_build_as_text_and_takes_paths_against_the_cwd() {
        let expected = Command::CheckPromotion {
            build: "20261028123456".into(),
            paths: PromotionPaths {
                repo: cwd().join("."),
                nightly_assets: cwd().join("nightly.txt"),
                beta_assets: cwd().join("beta.txt"),
                beta_current: cwd().join("beta-current"),
            },
        };
        assert_eq!(command(args(&check_args()), &cwd()), Ok(expected.clone()));
        // Any order.
        let mut reordered = check_args();
        let tail = reordered.split_off(1);
        let pairs: Vec<_> = tail.chunks(2).rev().flatten().copied().collect();
        reordered.extend(pairs);
        assert_eq!(command(args(&reordered), &cwd()), Ok(expected));
        // An absolute path is kept.
        let absolute = std::env::temp_dir().join("main");
        let mut given = args(&check_args());
        given[4] = absolute.clone().into();
        let Ok(Command::CheckPromotion { paths, .. }) = command(given, &cwd()) else {
            panic!("checks");
        };
        assert_eq!(paths.repo, absolute);
    }

    #[test]
    fn a_malformed_build_is_no_usage_error() {
        let mut given = check_args();
        given[2] = "not-an-id";
        assert!(
            matches!(command(args(&given), &cwd()), Ok(Command::CheckPromotion { build, .. }) if build == "not-an-id")
        );
        let refused = PromoteError::Malformed(BuildIdError::Malformed("not-an-id".into()));
        assert_eq!(promotion_exit_status(&refused), FAILURE_STATUS);
        assert_eq!(
            promotion_exit_status(&PromoteError::NotOnMain("refs/heads/x".into())),
            FAILURE_STATUS
        );
    }

    #[test]
    fn a_missing_repeated_or_unknown_check_promotion_option_is_a_usage_error() {
        let full = check_args();
        let mut bad: Vec<Vec<&str>> =
            vec![vec!["check-promotion"], vec!["check-promotion", "--build"]];
        for skip in (1..full.len()).step_by(2) {
            let mut without = full.clone();
            without.drain(skip..skip + 2);
            bad.push(without);
        }
        let mut repeated = full.clone();
        repeated.extend(["--build", "20261028123457"]);
        bad.push(repeated);
        let mut unknown = full.clone();
        unknown.extend(["--dist", "x"]);
        bad.push(unknown);
        let mut stray = full.clone();
        stray.push("stray");
        bad.push(stray);
        for bad in bad {
            assert_eq!(command(args(&bad), &cwd()), Err(UsageError), "{bad:?}");
        }
    }

    fn not_newer(current: &str) -> AppcastError {
        AppcastError::NotNewer {
            feed: "appcast-macos-nightly.xml".into(),
            current: BuildId::parse(current).expect("an ID"),
            new: BuildId::parse("20261028123456").expect("an ID"),
        }
    }

    #[test]
    fn the_same_build_exits_already_published_and_an_older_one_fails() {
        assert_eq!(exit_status(&not_newer("20261028123456")), ALREADY_PUBLISHED);
        assert_eq!(exit_status(&not_newer("20261029000000")), FAILURE_STATUS);
        assert_eq!(exit_status(&AppcastError::NoSeed), FAILURE_STATUS);
        assert_eq!([FAILURE_STATUS, USAGE_STATUS, ALREADY_PUBLISHED], [1, 2, 3]);
    }
}
