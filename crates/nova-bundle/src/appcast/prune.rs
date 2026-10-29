//! Which of the nightly release's assets a publish deletes: anything that
//! is neither uploaded by this run, nor a live feed or a file a feed
//! names, nor one of the newest few update files of its platform, nor
//! the commit file ([`super::commit::commit_asset`]) of a build one of
//! whose update files is kept.

use std::collections::{BTreeMap, BTreeSet};

use super::commit::parse_commit_asset;
use super::{Platform, parse_update_asset};
use crate::build_id::BuildId;

/// How many build-ID update files each platform keeps, newest first.
pub const KEEP_BUILDS: usize = 3;

/// A feed, published before this run or written by it, and the update
/// file it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveFeed {
    /// The feed's release asset, as `appcast-macos-nightly.xml`.
    pub name: String,
    /// The update file its enclosure names.
    pub file: String,
}

/// The assets of `current` (the release's assets before this run) to
/// delete, sorted. An asset is kept if `uploads` (this run's files and
/// feeds) holds it, if it is a feed in `feeds` or a file one names, or if
/// it is among the `keep` newest update files of its platform, counted
/// over `current` and `uploads` together. A build's commit file is kept
/// while any update file of that build is, by any of those rules.
#[must_use]
pub fn prune(
    current: &[String],
    uploads: &[String],
    feeds: &[LiveFeed],
    keep: usize,
) -> Vec<String> {
    let mut kept: BTreeSet<&str> = uploads.iter().map(String::as_str).collect();
    for feed in feeds {
        kept.insert(&feed.name);
        kept.insert(&feed.file);
    }
    let mut builds: BTreeMap<Platform, BTreeSet<(BuildId, &str)>> = BTreeMap::new();
    for name in current.iter().chain(uploads) {
        if let Some((platform, build)) = parse_update_asset(name) {
            builds.entry(platform).or_default().insert((build, name));
        }
    }
    for newest in builds.values() {
        kept.extend(newest.iter().rev().take(keep).map(|&(_, name)| name));
    }
    let kept_builds: BTreeSet<BuildId> = kept
        .iter()
        .filter_map(|name| parse_update_asset(name))
        .map(|(_, build)| build)
        .collect();
    let deleted: BTreeSet<&String> = current
        .iter()
        .filter(|name| {
            let with_its_build =
                parse_commit_asset(name).is_some_and(|build| kept_builds.contains(&build));
            !kept.contains(name.as_str()) && !with_its_build
        })
        .collect();
    deleted.into_iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appcast::commit::commit_asset;
    use crate::appcast::{Channel, feed_name, update_asset};
    use crate::archive;
    use crate::linux::flatpak;

    fn id(text: &str) -> BuildId {
        BuildId::parse(text).expect("an ID")
    }

    fn mac(build: &str) -> String {
        update_asset(Platform::Macos, &id(build))
    }

    fn win(build: &str) -> String {
        update_asset(Platform::Windows, &id(build))
    }

    fn feed(platform: Platform, file: String) -> LiveFeed {
        LiveFeed {
            name: feed_name(platform, Channel::Nightly),
            file,
        }
    }

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|&n| n.to_owned()).collect()
    }

    fn stable() -> Vec<String> {
        strings(&[
            Platform::Macos.stable_asset(),
            Platform::Windows.stable_asset(),
            archive::WINDOWS.file,
            archive::LINUX.file,
            flatpak::ASSET,
        ])
    }

    #[test]
    fn keeps_uploads_feed_named_files_and_the_newest_builds_and_deletes_the_rest() {
        let macs = [
            "20261001000000",
            "20261002000000",
            "20261003000000",
            "20261004000000",
            "20261005000000",
        ];
        let wins = [
            "20261002000000",
            "20261003000000",
            "20261004000000",
            "20261005000000",
        ];
        let mut current = stable();
        current.extend(macs.iter().map(|b| mac(b)));
        current.extend(wins.iter().map(|b| win(b)));
        // Older than every newest-3 file, but the current Windows feed
        // still names it.
        current.push(win("20260901000000"));
        current.push(feed_name(Platform::Macos, Channel::Nightly));
        current.push(feed_name(Platform::Windows, Channel::Nightly));
        current.push("ev-nova-x86_64-unknown-linux-gnu.AppImage".to_owned());

        let new = "20261006000000";
        let mut uploads = stable();
        uploads.push(mac(new));
        uploads.push(win(new));
        uploads.push(feed_name(Platform::Macos, Channel::Nightly));
        uploads.push(feed_name(Platform::Windows, Channel::Nightly));

        let feeds = [
            feed(Platform::Macos, mac("20261005000000")),
            feed(Platform::Windows, win("20260901000000")),
            feed(Platform::Macos, mac(new)),
            feed(Platform::Windows, win(new)),
        ];
        let mut want = vec![
            mac("20261001000000"),
            mac("20261002000000"),
            mac("20261003000000"),
            win("20261002000000"),
            win("20261003000000"),
            "ev-nova-x86_64-unknown-linux-gnu.AppImage".to_owned(),
        ];
        want.sort();
        assert_eq!(prune(&current, &uploads, &feeds, KEEP_BUILDS), want);
    }

    #[test]
    fn a_feed_named_file_is_kept_even_beyond_the_newest_n() {
        let current = vec![
            mac("20260101000000"),
            mac("20260102000000"),
            mac("20260103000000"),
        ];
        let feeds = [feed(Platform::Macos, mac("20260101000000"))];
        assert_eq!(prune(&current, &[], &feeds, 1), vec![mac("20260102000000")]);
    }

    #[test]
    fn newest_n_is_counted_per_platform() {
        let current = vec![
            mac("20260101000000"),
            mac("20260102000000"),
            win("20260103000000"),
            win("20260104000000"),
        ];
        assert_eq!(
            prune(&current, &[], &[], 1),
            vec![mac("20260101000000"), win("20260103000000")]
        );
    }

    #[test]
    fn the_newest_n_counts_this_runs_uploads() {
        let current = vec![mac("20260101000000"), mac("20260102000000")];
        let uploads = vec![mac("20260103000000")];
        assert_eq!(
            prune(&current, &uploads, &[], 2),
            vec![mac("20260101000000")]
        );
    }

    #[test]
    fn a_live_feed_not_in_this_runs_uploads_is_kept() {
        let current = vec![
            feed_name(Platform::Macos, Channel::Nightly),
            mac("20260101000000"),
        ];
        let feeds = [feed(Platform::Macos, mac("20260101000000"))];
        assert_eq!(prune(&current, &[], &feeds, 0), Vec::<String>::new());
        assert_eq!(
            prune(&current, &[], &[], 0),
            vec![
                feed_name(Platform::Macos, Channel::Nightly),
                mac("20260101000000")
            ]
        );
    }

    fn commit(build: &str) -> String {
        commit_asset(&id(build))
    }

    #[test]
    fn a_build_s_commit_file_goes_with_its_update_files() {
        let (oldest, old, mid, new) = (
            "20260101000000",
            "20260102000000",
            "20260103000000",
            "20260104000000",
        );
        let current = vec![
            mac(oldest),
            win(oldest),
            commit(oldest),
            mac(old),
            win(old),
            commit(old),
            mac(mid),
            win(mid),
            commit(mid),
            // No update file of this build is left: an orphan.
            commit("20251231000000"),
        ];
        let uploads = vec![mac(new), win(new), commit(new)];
        // Keeping 2, `oldest` and `old` fall out; a feed still names
        // `old`'s Windows file, which keeps its commit file.
        let feeds = [feed(Platform::Windows, win(old))];
        let mut want = vec![
            mac(oldest),
            win(oldest),
            commit(oldest),
            mac(old),
            commit("20251231000000"),
        ];
        want.sort();
        assert_eq!(prune(&current, &uploads, &feeds, 2), want);
        // With no update file of its build, a commit file goes.
        assert_eq!(prune(&[commit(mid)], &[], &[], 3), vec![commit(mid)]);
        // An uploaded commit file is kept whatever else is.
        assert_eq!(
            prune(&[commit(mid)], &[commit(mid)], &[], 0),
            Vec::<String>::new()
        );
    }

    #[test]
    fn nothing_to_delete_on_a_first_publish() {
        assert_eq!(
            prune(&[], &stable(), &[], KEEP_BUILDS),
            Vec::<String>::new()
        );
        assert_eq!(
            prune(&stable(), &stable(), &[], KEEP_BUILDS),
            Vec::<String>::new()
        );
    }
}
