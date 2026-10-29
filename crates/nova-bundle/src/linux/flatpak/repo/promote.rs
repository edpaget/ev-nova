//! A promoted channel's Flatpak publish: the commit its build came from,
//! found on the channel it is promoted from, copied onto its own branch
//! with no rebuild.
//!
//! A beta is a nightly promoted by hand ([`Channel::promoted_from`]).
//! `sign` reads the nightly branch's history in the mirrored pages repo
//! ([`log`]), finds the commit whose subject names the build
//! ([`find_build`], the subject [`super::plan::subject`] writes), copies
//! it into the work tree's own repo ([`copy_steps`]: `ostree pull-local`
//! by checksum, then a ref onto it), and commits it onto the beta branch
//! with `flatpak build-commit-from` from that ref. The new commit's tree is
//! the nightly commit's.
//!
//! Every publish then lists the pages repo's refs ([`refs`],
//! [`parse_refs`]) and writes the `.flatpakref` of every channel whose app
//! ref the repo holds ([`served_channels`]): a Pages deploy replaces the
//! whole site, so a nightly publish keeps `ev-nova-beta.flatpakref`, and a
//! promotion keeps the nightly's.

use std::path::Path;

use super::plan::{init, ostree, subject};
use super::{app_ref, repo, src};
use crate::appcast::Channel;
use crate::appcast::promote::and_list;
use crate::build_id::BuildId;
use crate::bundle::Step;
use crate::ports::Invocation;
use crate::spec::BundleSpec;

/// The checksum of the commit on `channel`'s branch whose subject names
/// `build`, from `log` (`ostree log`'s output for the branch).
///
/// # Errors
///
/// [`NotOnBranch`] naming the builds the branch does hold when no
/// commit's subject names `build`.
pub fn find_build(log: &str, channel: Channel, build: &BuildId) -> Result<String, NotOnBranch> {
    let commits = commits(log);
    let wanted = subject(channel, build);
    if let Some((checksum, _)) = commits.iter().find(|(_, s)| *s == wanted) {
        return Ok((*checksum).to_owned());
    }
    let prefix = subject_prefix(channel);
    let mut held: Vec<BuildId> = commits
        .iter()
        .filter_map(|(_, s)| s.strip_prefix(&prefix))
        .filter_map(|id| BuildId::parse(id).ok())
        .collect();
    held.sort();
    held.dedup();
    Err(NotOnBranch {
        channel,
        build: build.clone(),
        held,
    })
}

/// The subject's text before the build ID, as `EV Nova nightly build `.
fn subject_prefix(channel: Channel) -> String {
    let probe = BuildId::from_unix(0).expect("the epoch is a build ID");
    let full = subject(channel, &probe);
    full.strip_suffix(probe.as_str())
        .expect("the subject ends in the build ID")
        .to_owned()
}

/// Each commit in `log` with its subject, in log order: a `commit
/// <checksum>` line, then the first line indented four spaces. Anything
/// else (headers, signatures, ostree's "History beyond this commit not
/// fetched") is skipped.
fn commits(log: &str) -> Vec<(&str, &str)> {
    let mut commits = Vec::new();
    let mut current: Option<&str> = None;
    for line in log.lines() {
        if let Some(checksum) = line.strip_prefix("commit ") {
            let checksum = checksum.trim();
            current = is_checksum(checksum).then_some(checksum);
        } else if let Some(text) = line.strip_prefix("    ") {
            let text = text.trim();
            if let (Some(checksum), false) = (current, text.is_empty()) {
                commits.push((checksum, text));
                current = None;
            }
        }
    }
    commits
}

/// Whether `text` is an ostree commit checksum: 64 hex digits.
fn is_checksum(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The branch a promoted build came from holds no commit of it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "the {} branch holds no build {build}; it holds {}",
    channel.name(),
    if held.is_empty() { "none".to_owned() } else { and_list(held) }
)]
pub struct NotOnBranch {
    /// The branch searched.
    pub channel: Channel,
    /// The build asked for.
    pub build: BuildId,
    /// The builds it does hold, oldest first.
    pub held: Vec<BuildId>,
}

/// `ostree log` of `channel`'s app ref in the pages repo, captured.
#[must_use]
pub fn log(spec: &BundleSpec, channel: Channel, pages: &Path) -> Invocation {
    ostree(&repo(pages), &["log", &app_ref(spec, channel)]).captured()
}

/// `ostree refs` of the pages repo, captured.
#[must_use]
pub fn refs(pages: &Path) -> Invocation {
    ostree(&repo(pages), &["refs"]).captured()
}

/// The refs `ostree refs` printed, one per line.
#[must_use]
pub fn parse_refs(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

/// The channels whose app ref `held` (the repo's refs) names, in
/// [`Channel::ALL`]'s order, with `published` among them whatever the
/// listing says: its ref was just committed.
#[must_use]
pub fn served_channels(spec: &BundleSpec, held: &[&str], published: Channel) -> Vec<Channel> {
    Channel::ALL
        .into_iter()
        .filter(|&channel| channel == published || held.contains(&app_ref(spec, channel).as_str()))
        .collect()
}

/// The commit `checksum` of the pages repo copied into the work tree's own
/// repo, on `from`'s app ref, for `build-commit-from` to read: a fresh
/// archive repo, `pull-local` by checksum, and the ref made onto it.
#[must_use]
pub fn copy_steps(
    spec: &BundleSpec,
    from: Channel,
    work: &Path,
    pages: &Path,
    checksum: &str,
) -> Vec<Step> {
    let src = src(work);
    vec![
        Step::RemoveDir(src.clone()),
        Step::Run(init(&src)),
        Step::Run(ostree(&src, &["pull-local"]).arg(repo(pages)).arg(checksum)),
        Step::Run(
            ostree(
                &src,
                &["refs", &format!("--create={}", app_ref(spec, from))],
            )
            .arg(checksum),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::spec::tests::sample_metadata;

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn id(text: &str) -> BuildId {
        BuildId::parse(text).expect("an ID")
    }

    fn sum(digit: char) -> String {
        digit.to_string().repeat(64)
    }

    /// An `ostree log` entry for `checksum` with `subject`.
    fn entry(checksum: &str, subject: &str) -> String {
        format!(
            "commit {checksum}\nContentChecksum:  ab\nDate:  2026-10-28 00:00:00 +0000\n\n    \
             {subject}\n\n"
        )
    }

    #[test]
    fn the_commit_whose_subject_names_the_build_is_found() {
        let log = [
            entry(&sum('a'), "EV Nova nightly build 20261029000000"),
            entry(&sum('b'), "EV Nova nightly build 20261028123456"),
            "<< History beyond this commit not fetched >>\n".to_owned(),
        ]
        .concat();
        assert_eq!(
            find_build(&log, Channel::Nightly, &id("20261028123456")),
            Ok(sum('b'))
        );
        assert_eq!(
            find_build(&log, Channel::Nightly, &id("20261029000000")),
            Ok(sum('a'))
        );
    }

    #[test]
    fn beta_subjects_signatures_and_malformed_lines_are_ignored() {
        let log = [
            entry(&sum('a'), "EV Nova beta build 20261028123456"),
            "commit nothex\n\n    EV Nova nightly build 20261028123456\n\n".to_owned(),
            format!(
                "commit {}\n\n    EV Nova nightly build 20261028123456\n\n",
                "c".repeat(63)
            ),
            entry(&sum('d'), "EV Nova nightly build 2026"),
            "Found 1 signature:\n\n  Signature made Wed 28 Oct by RSA key\n".to_owned(),
            entry(&sum('e'), "EV Nova nightly build 20261027000000"),
        ]
        .concat();
        let error = find_build(&log, Channel::Nightly, &id("20261028123456")).expect_err("absent");
        assert_eq!(
            error,
            NotOnBranch {
                channel: Channel::Nightly,
                build: id("20261028123456"),
                held: vec![id("20261027000000")],
            }
        );
        assert_eq!(
            error.to_string(),
            "the nightly branch holds no build 20261028123456; it holds 20261027000000"
        );
    }

    #[test]
    fn an_empty_branch_holds_none() {
        let error = find_build("", Channel::Nightly, &id("20261028123456")).expect_err("none");
        assert_eq!(
            error.to_string(),
            "the nightly branch holds no build 20261028123456; it holds none"
        );
    }

    #[test]
    fn the_held_builds_are_listed_oldest_first_once_each() {
        let log = [
            entry(&sum('a'), "EV Nova nightly build 20261029000000"),
            entry(&sum('b'), "EV Nova nightly build 20261027000000"),
            entry(&sum('c'), "EV Nova nightly build 20261028000000"),
            entry(&sum('d'), "EV Nova nightly build 20261027000000"),
        ]
        .concat();
        let error = find_build(&log, Channel::Nightly, &id("20261030000000")).expect_err("absent");
        assert_eq!(
            error.held,
            [
                id("20261027000000"),
                id("20261028000000"),
                id("20261029000000")
            ]
        );
        assert!(
            error
                .to_string()
                .ends_with("it holds 20261027000000, 20261028000000 and 20261029000000"),
            "{error}"
        );
    }

    #[test]
    fn refs_are_one_per_line_and_name_the_channels_served() {
        let listed = parse_refs("app/io.example.nova/x86_64/beta\n\n  appstream/x86_64 \n");
        assert_eq!(
            listed,
            ["app/io.example.nova/x86_64/beta", "appstream/x86_64"]
        );
        let spec = spec();
        assert_eq!(
            served_channels(&spec, &listed, Channel::Nightly),
            [Channel::Nightly, Channel::Beta]
        );
        assert_eq!(
            served_channels(&spec, &listed, Channel::Beta),
            [Channel::Beta]
        );
        assert_eq!(
            served_channels(&spec, &[], Channel::Beta),
            [Channel::Beta],
            "the channel just committed is served whatever the listing"
        );
        assert_eq!(
            served_channels(&spec, &["app/other/x86_64/beta"], Channel::Nightly),
            [Channel::Nightly]
        );
    }

    #[test]
    fn the_commands_read_the_pages_repo() {
        let pages = PathBuf::from("/t").join("pages");
        let mut flag = std::ffi::OsString::from("--repo=");
        flag.push(pages.join("repo"));
        assert_eq!(
            log(&spec(), Channel::Nightly, &pages),
            Invocation::new("ostree")
                .arg(&flag)
                .args(["log", "app/io.example.nova/x86_64/nightly"])
                .captured()
        );
        assert_eq!(
            refs(&pages),
            Invocation::new("ostree").arg(&flag).arg("refs").captured()
        );
    }
}
