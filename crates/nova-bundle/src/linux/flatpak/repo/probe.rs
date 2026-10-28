//! Two checks on the outside world, each a pure reading of a program's
//! output: whether a repo is live on Pages ([`live_from_probe`]), and
//! whether the tree fits Pages ([`check_size`]).

use std::path::Path;

use super::{MAX_FILE, PAGES_BUDGET};
use crate::ports::{Invocation, RunError};

/// Whether the live site has a repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Live {
    /// It answers `200` for the repo's `config`: mirror it.
    Present,
    /// It answers `404`: the first publish, from an empty repo.
    Absent,
}

/// Why the live repo couldn't be probed. Neither is taken for "no repo",
/// which would drop every other branch.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// `curl` failed (DNS, the connection).
    #[error("could not reach {url}: `{command}` failed: {source}")]
    Failed {
        /// The URL probed.
        url: String,
        /// The command line.
        command: String,
        /// How it failed.
        source: RunError,
    },
    /// The site answered with neither `200` nor `404`.
    #[error(
        "{url} answered `{answer}`, neither 200 nor 404: refusing to guess whether a repo is live"
    )]
    Answer {
        /// The URL probed.
        url: String,
        /// What `curl` printed.
        answer: String,
    },
}

/// The URL probed: the repo's `config`, which an archive repo always
/// serves.
#[must_use]
pub fn config_url(url: &str) -> String {
    format!("{url}config")
}

/// `curl` printing the HTTP status of the repo's `config` and nothing
/// else, following redirects. With no `--fail`, a `404` exits 0.
#[must_use]
pub fn probe(url: &str) -> Invocation {
    Invocation::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}",
        ])
        .arg(config_url(url))
        .captured()
}

/// What `curl`'s run of [`probe`] on `url` says.
///
/// # Errors
///
/// A failed `curl`, or any answer but `200` or `404`.
pub fn live_from_probe(url: &str, outcome: Result<Vec<u8>, RunError>) -> Result<Live, ProbeError> {
    let command = probe(url).to_string();
    let url = config_url(url);
    let stdout = outcome.map_err(|source| ProbeError::Failed {
        command,
        url: url.clone(),
        source,
    })?;
    match String::from_utf8_lossy(&stdout).trim() {
        "200" => Ok(Live::Present),
        "404" => Ok(Live::Absent),
        answer => Err(ProbeError::Answer {
            url,
            answer: answer.to_owned(),
        }),
    }
}

/// Why the tree doesn't fit Pages.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SizeError {
    /// `find` printed a line that isn't `<size>\t<path>`.
    #[error("could not read `find`'s line `{0}`")]
    Listing(String),
    /// One file is over [`MAX_FILE`].
    #[error(
        "{path} is {size} bytes, over Pages' {MAX_FILE}-byte file limit (the tree holds {total} bytes)"
    )]
    TooLarge {
        /// The file, relative to the tree.
        path: String,
        /// Its size.
        size: u64,
        /// The whole tree's.
        total: u64,
    },
    /// The tree is over [`PAGES_BUDGET`].
    #[error(
        "the tree holds {total} bytes, over the {PAGES_BUDGET}-byte budget under Pages' 1 GB site limit"
    )]
    OverBudget {
        /// The whole tree's size.
        total: u64,
    },
}

/// `find` printing each file under `tree` as `<size>\t<path>`, the path
/// relative to `tree`.
#[must_use]
pub fn listing(tree: &Path) -> Invocation {
    Invocation::new("find")
        .arg(tree)
        .args(["-type", "f", "-printf", "%s\\t%P\\n"])
        .captured()
}

/// Each file [`listing`] printed, as its path and size.
///
/// # Errors
///
/// A line that isn't `<size>\t<path>`.
pub fn parse_listing(stdout: &[u8]) -> Result<Vec<(String, u64)>, SizeError> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (size, path) = line
                .split_once('\t')
                .ok_or_else(|| SizeError::Listing(line.to_owned()))?;
            let size = size
                .parse()
                .map_err(|_| SizeError::Listing(line.to_owned()))?;
            Ok((path.to_owned(), size))
        })
        .collect()
}

/// The tree's total size, when every file is at most [`MAX_FILE`] and the
/// total at most [`PAGES_BUDGET`].
///
/// # Errors
///
/// The first file over the limit, naming the total, or a total over the
/// budget.
pub fn check_size(files: &[(String, u64)]) -> Result<u64, SizeError> {
    let total = files.iter().map(|(_, size)| size).sum();
    if let Some((path, size)) = files.iter().find(|(_, size)| *size > MAX_FILE) {
        return Err(SizeError::TooLarge {
            path: path.clone(),
            size: *size,
            total,
        });
    }
    if total > PAGES_BUDGET {
        return Err(SizeError::OverBudget { total });
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "https://example.com/flatpak/repo/";

    #[test]
    fn the_probe_asks_for_the_config_s_status_alone() {
        let probe = probe(URL);
        assert_eq!(probe.program, "curl");
        assert!(probe.capture);
        assert_eq!(
            probe.args.last().map(|a| a.to_string_lossy().into_owned()),
            Some("https://example.com/flatpak/repo/config".to_owned())
        );
        assert!(
            !probe.args.iter().any(|a| a == "--fail" || a == "-f"),
            "{probe}"
        );
        assert_eq!(
            probe.to_string(),
            "curl --silent --show-error --location --output /dev/null --write-out %{http_code} \
             https://example.com/flatpak/repo/config"
        );
    }

    #[test]
    fn a_200_is_a_live_repo_and_a_404_the_first_publish() {
        assert_eq!(
            live_from_probe(URL, Ok(b"200".to_vec())).expect("live"),
            Live::Present
        );
        assert_eq!(
            live_from_probe(URL, Ok(b"404\n".to_vec())).expect("absent"),
            Live::Absent
        );
    }

    #[test]
    fn any_other_answer_is_an_error_naming_the_url_and_the_answer() {
        for answer in ["500", "000", "301", "garbage", ""] {
            let error = live_from_probe(URL, Ok(answer.as_bytes().to_vec())).expect_err(answer);
            let message = error.to_string();
            assert!(
                message.contains("https://example.com/flatpak/repo/config"),
                "{message}"
            );
            assert!(message.contains(&format!("`{answer}`")), "{message}");
        }
    }

    #[test]
    fn a_failed_curl_is_an_error_naming_the_url_and_the_command() {
        let error = live_from_probe(URL, Err(RunError::Failed(Some(6)))).expect_err("failed");
        let message = error.to_string();
        assert!(
            message.contains("could not reach https://example.com/flatpak/repo/config"),
            "{message}"
        );
        assert!(message.contains("`curl --silent"), "{message}");
        assert!(message.contains("exited with code 6"), "{message}");
    }

    #[test]
    fn find_prints_each_file_s_size_and_relative_path() {
        let tree = Path::new("/p").join("pages");
        let listing = listing(&tree);
        assert!(listing.capture);
        assert_eq!(listing.args[0], tree.into_os_string());
        assert_eq!(listing.args[1..], ["-type", "f", "-printf", "%s\\t%P\\n"]);
    }

    #[test]
    fn a_listing_parses_into_paths_and_sizes() {
        assert_eq!(
            parse_listing(b"12\trepo/config\n0\tev-nova-nightly.flatpakref\n\n").expect("parsed"),
            [
                ("repo/config".to_owned(), 12),
                ("ev-nova-nightly.flatpakref".to_owned(), 0)
            ]
        );
        assert_eq!(parse_listing(b"").expect("empty"), []);
        for bad in ["12 repo/config", "x\trepo/config", "-1\tf"] {
            assert_eq!(
                parse_listing(bad.as_bytes()),
                Err(SizeError::Listing(bad.to_owned()))
            );
        }
    }

    #[test]
    fn a_file_over_the_limit_is_refused_by_name_with_the_total() {
        let files = [
            ("repo/a".to_owned(), 5),
            ("repo/deltas/big".to_owned(), MAX_FILE + 1),
        ];
        let error = check_size(&files).expect_err("too large");
        assert_eq!(
            error,
            SizeError::TooLarge {
                path: "repo/deltas/big".into(),
                size: MAX_FILE + 1,
                total: MAX_FILE + 6,
            }
        );
        assert!(error.to_string().contains("repo/deltas/big"), "{error}");
        assert_eq!(check_size(&[("f".to_owned(), MAX_FILE)]), Ok(MAX_FILE));
    }

    #[test]
    fn a_tree_over_the_budget_is_refused_with_its_total() {
        let files: Vec<_> = (0..10).map(|i| (format!("repo/{i}"), MAX_FILE)).collect();
        let error = check_size(&files).expect_err("over budget");
        assert_eq!(
            error,
            SizeError::OverBudget {
                total: 10 * MAX_FILE
            }
        );
        assert!(error.to_string().contains("1000000000"), "{error}");
        let exactly: Vec<_> = (0..9).map(|i| (format!("repo/{i}"), MAX_FILE)).collect();
        assert_eq!(check_size(&exactly), Ok(PAGES_BUDGET));
    }

    #[test]
    fn an_empty_or_small_tree_passes() {
        assert_eq!(check_size(&[]), Ok(0));
        assert_eq!(check_size(&[("repo/config".to_owned(), 120)]), Ok(120));
    }
}
