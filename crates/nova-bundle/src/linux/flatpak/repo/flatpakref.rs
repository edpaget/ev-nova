//! The `.flatpakref`: one file that adds the repo as a remote, trusting
//! its key, and installs the app on a channel's branch from it, in one
//! click in Discover or GNOME Software, or with `flatpak install --from`.

use super::REMOTE_NAME;
use super::key::CommittedKey;
use crate::appcast::Channel;
use crate::linux::flatpak::RUNTIME_REPO;
use crate::spec::BundleSpec;

/// The `.flatpakref` for `spec`'s app on `channel`'s branch of the repo at
/// `url`, trusting `key`. The remote is [`REMOTE_NAME`] for every channel,
/// so a second channel's ref reuses it; the runtime comes from Flathub
/// ([`RUNTIME_REPO`]).
#[must_use]
pub fn render(spec: &BundleSpec, channel: Channel, url: &str, key: &CommittedKey) -> String {
    let branch = channel.name();
    format!(
        "[Flatpak Ref]\n\
         Name={}\n\
         Branch={branch}\n\
         Title={} ({branch})\n\
         Url={url}\n\
         SuggestRemoteName={REMOTE_NAME}\n\
         Homepage={}\n\
         IsRuntime=false\n\
         RuntimeRepo={RUNTIME_REPO}\n\
         GPGKey={}\n",
        spec.identifier,
        spec.name,
        spec.homepage,
        key.base64()
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::linux::flatpak::repo::key::tests::FPR;
    use crate::linux::flatpak::repo::url;
    use crate::spec::tests::sample_metadata;

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn key() -> CommittedKey {
        CommittedKey {
            fingerprint: FPR.into(),
            binary: vec![0x98, 0x33, 0x04, 0x65, 0xff],
        }
    }

    /// The file's one group, as its name and its keys.
    fn parse(text: &str) -> (String, BTreeMap<String, String>) {
        let mut lines = text.lines();
        let group = lines.next().expect("a group line").to_owned();
        let keys = lines
            .map(|line| {
                let (key, value) = line.split_once('=').expect("key=value");
                (key.to_owned(), value.to_owned())
            })
            .collect();
        (group, keys)
    }

    #[test]
    fn names_the_pages_repo_the_app_the_branch_and_the_committed_key() {
        let spec = spec();
        let text = render(&spec, Channel::Nightly, &url(), &key());
        let (group, keys) = parse(&text);
        assert_eq!(group, "[Flatpak Ref]");
        let want: BTreeMap<String, String> = [
            ("Name", spec.identifier.clone()),
            ("Branch", Channel::Nightly.name().to_owned()),
            ("Title", format!("{} (nightly)", spec.name)),
            ("Url", url()),
            ("SuggestRemoteName", REMOTE_NAME.to_owned()),
            ("Homepage", spec.homepage.clone()),
            ("IsRuntime", "false".to_owned()),
            ("RuntimeRepo", RUNTIME_REPO.to_owned()),
            ("GPGKey", key().base64()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        assert_eq!(keys, want);
        assert_eq!(keys["GPGKey"], "mDMEZf8=");
        assert_eq!(text.lines().count(), 10, "no key twice: {text}");
    }

    #[test]
    fn the_beta_ref_names_the_beta_branch_and_shares_the_nightly_s_remote_and_key() {
        let spec = spec();
        let (_, beta) = parse(&render(&spec, Channel::Beta, &url(), &key()));
        let (_, nightly) = parse(&render(&spec, Channel::Nightly, &url(), &key()));
        assert_eq!(beta["Branch"], "beta");
        assert_eq!(beta["Title"], format!("{} (beta)", spec.name));
        for same in ["Url", "Name", "GPGKey", "SuggestRemoteName", "RuntimeRepo"] {
            assert_eq!(beta[same], nightly[same], "{same}");
        }
    }

    #[test]
    fn the_runtime_comes_from_flathub() {
        let text = render(&spec(), Channel::Nightly, &url(), &key());
        assert_eq!(
            parse(&text).1["RuntimeRepo"],
            "https://dl.flathub.org/repo/flathub.flatpakrepo"
        );
    }

    #[test]
    fn the_file_ends_with_a_newline() {
        let text = render(&spec(), Channel::Nightly, &url(), &key());
        assert!(text.ends_with('\n'), "{text}");
        assert!(!text.ends_with("\n\n"), "{text}");
    }
}
