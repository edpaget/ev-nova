//! `NOVA_CHANNEL`: the release channel a bundle is built for, and the
//! update feed its bundle names.
//!
//! CI sets [`VAR`] to the channel it builds (`nightly`); a local build
//! sets nothing, and makes a development bundle, which names no feed.
//! [`feed_status`] is the decision: a bundle names its channel's feed only
//! when the game package commits the key that verifies it
//! ([`BundleSpec::update_public_key`]), so a bundle never names a feed it
//! couldn't check. The Windows installer's host reads the channel the same
//! way, and [`feed_url`] is the one source of each feed's URL.

use crate::appcast::key::PublicKey;
use crate::appcast::{Channel, Platform, feed_name};
use crate::ports::Env;
use crate::site::{REPO, release_asset};
use crate::spec::{BundleSpec, GAME_PACKAGE};

/// The variable that names the channel.
pub const VAR: &str = "NOVA_CHANNEL";

/// Why `NOVA_CHANNEL` names no channel.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChannelError {
    /// It names a channel there is none of.
    #[error(
        "{VAR} is `{0}`: set it to {known}, or leave it unset for a development bundle",
        known = known()
    )]
    Unknown(String),
}

/// The channels' names, as the error lists them: "nightly", or "nightly
/// or beta".
fn known() -> String {
    let names: Vec<&str> = Channel::ALL.iter().map(|c| c.name()).collect();
    names.join(" or ")
}

/// The channel `env`'s [`VAR`] names: `None`, a development bundle, when
/// it is unset or empty.
pub fn from_env(env: &impl Env) -> Result<Option<Channel>, ChannelError> {
    match env.var(VAR) {
        None => Ok(None),
        Some(value) if value.is_empty() => Ok(None),
        Some(value) => {
            let value = value.to_string_lossy();
            Channel::from_name(&value)
                .map(Some)
                .ok_or_else(|| ChannelError::Unknown(value.into_owned()))
        }
    }
}

/// The URL of `platform`'s feed for `channel`: an asset of the channel's
/// release.
#[must_use]
pub fn feed_url(platform: Platform, channel: Channel) -> String {
    release_asset(REPO, channel.tag(), &feed_name(platform, channel))
}

/// The feed a bundle names, and the key its updates are checked with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateFeed {
    /// The feed's URL ([`feed_url`]).
    pub url: String,
    /// The committed public key.
    pub public_key: PublicKey,
}

/// Whether a bundle names a feed, and why not when it doesn't.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedStatus {
    /// No channel: a development bundle.
    Development,
    /// A channel, but the game package commits no `update_public_key`.
    NoKey(Channel),
    /// A channel and its key: the bundle names this feed.
    Feed(UpdateFeed),
}

impl FeedStatus {
    /// The feed, when there is one.
    #[must_use]
    pub fn feed(&self) -> Option<&UpdateFeed> {
        match self {
            Self::Feed(feed) => Some(feed),
            Self::Development | Self::NoKey(_) => None,
        }
    }

    /// The line the bundler prints about it.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Feed(feed) => format!("update feed: {}", feed.url),
            Self::NoKey(channel) => format!(
                "no update feed: {VAR} is {} but the {GAME_PACKAGE} package commits no \
                 update_public_key",
                channel.name()
            ),
            Self::Development => format!("no update feed: {VAR} is unset (a development bundle)"),
        }
    }
}

/// What `platform`'s bundle of `spec`, built for `channel`, names: its
/// channel's feed when the spec commits a key, and otherwise no feed.
#[must_use]
pub fn feed_status(spec: &BundleSpec, channel: Option<Channel>, platform: Platform) -> FeedStatus {
    match (channel, spec.update_public_key) {
        (None, _) => FeedStatus::Development,
        (Some(channel), None) => FeedStatus::NoKey(channel),
        (Some(channel), Some(public_key)) => FeedStatus::Feed(UpdateFeed {
            url: feed_url(platform, channel),
            public_key,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::FakeEnv;

    fn env(value: &str) -> FakeEnv {
        FakeEnv(HashMap::from([(VAR, value.into())]))
    }

    fn spec(key: Option<PublicKey>) -> BundleSpec {
        BundleSpec {
            update_public_key: key,
            ..BundleSpec::from_metadata(&sample_metadata()).expect("spec")
        }
    }

    #[test]
    fn channel_names_round_trip_and_nightly_is_the_only_one() {
        assert_eq!(Channel::ALL, [Channel::Nightly]);
        for channel in Channel::ALL {
            assert_eq!(Channel::from_name(channel.name()), Some(channel));
        }
        assert_eq!(Channel::from_name("nightly"), Some(Channel::Nightly));
        for other in ["Nightly", "beta", "", " nightly"] {
            assert_eq!(Channel::from_name(other), None, "{other:?}");
        }
    }

    #[test]
    fn an_unset_or_empty_channel_is_a_development_bundle() {
        assert_eq!(from_env(&FakeEnv::default()), Ok(None));
        assert_eq!(from_env(&env("")), Ok(None));
    }

    #[test]
    fn nightly_reads_as_the_nightly_channel() {
        assert_eq!(from_env(&env("nightly")), Ok(Some(Channel::Nightly)));
    }

    #[test]
    fn an_unknown_channel_is_an_error_naming_it_and_the_known_ones() {
        for value in ["Nightly", "stable"] {
            let error = from_env(&env(value)).expect_err("unknown");
            assert_eq!(error, ChannelError::Unknown(value.to_owned()));
            assert_eq!(
                error.to_string(),
                format!(
                    "NOVA_CHANNEL is `{value}`: set it to nightly, or leave it unset for a \
                     development bundle"
                )
            );
        }
    }

    #[test]
    fn the_nightly_macos_feed_is_the_nightly_release_s_appcast() {
        assert_eq!(
            feed_url(Platform::Macos, Channel::Nightly),
            "https://github.com/edpaget/ev-nova/releases/download/nightly/appcast-macos-nightly.xml"
        );
        assert_eq!(
            feed_url(Platform::Windows, Channel::Nightly),
            "https://github.com/edpaget/ev-nova/releases/download/nightly/appcast-windows-nightly.xml"
        );
    }

    #[test]
    fn feed_status_needs_both_a_channel_and_a_key() {
        let key = PublicKey([7; 32]);
        assert_eq!(
            feed_status(&spec(None), None, Platform::Macos),
            FeedStatus::Development
        );
        assert_eq!(
            feed_status(&spec(Some(key)), None, Platform::Macos),
            FeedStatus::Development
        );
        assert_eq!(
            feed_status(&spec(None), Some(Channel::Nightly), Platform::Macos),
            FeedStatus::NoKey(Channel::Nightly)
        );
        let feed = UpdateFeed {
            url: feed_url(Platform::Macos, Channel::Nightly),
            public_key: key,
        };
        let both = feed_status(&spec(Some(key)), Some(Channel::Nightly), Platform::Macos);
        assert_eq!(both, FeedStatus::Feed(feed.clone()));
        assert_eq!(both.feed(), Some(&feed));
        assert_eq!(FeedStatus::Development.feed(), None);
        assert_eq!(FeedStatus::NoKey(Channel::Nightly).feed(), None);
        assert_eq!(
            feed_status(&spec(Some(key)), Some(Channel::Nightly), Platform::Windows)
                .feed()
                .map(|feed| feed.url.clone()),
            Some(feed_url(Platform::Windows, Channel::Nightly))
        );
    }

    /// nova's build script works out the feed `nova.exe` compiles in with
    /// `nova_winres::windows_update_feed`; the installer that ships it
    /// reports this crate's [`feed_status`]. They must agree for every
    /// channel and key, and refuse the same unknown names.
    #[test]
    fn the_build_script_s_windows_feed_is_the_bundler_s() {
        let key = PublicKey([9; 32]);
        for committed in [None, Some(String::new()), Some(key.to_base64())] {
            let line = committed
                .as_ref()
                .map_or(String::new(), |k| format!("update_public_key = \"{k}\"\n"));
            let manifest = format!(
                "[package]\nname = \"nova\"\n\n[package.metadata.bundle]\nidentifier = \
                 \"io.example.nova\"\n{line}"
            );
            let parsed = committed
                .as_deref()
                .filter(|k| !k.is_empty())
                .map(|k| PublicKey::from_base64(k).expect("key"));
            for channel in Channel::ALL {
                let built = nova_winres::windows_update_feed(&manifest, Some(channel.name()))
                    .expect("the build script's feed");
                match feed_status(&spec(parsed), Some(channel), Platform::Windows) {
                    FeedStatus::Feed(feed) => assert_eq!(
                        built,
                        Some(nova_winres::UpdateFeed {
                            url: feed.url,
                            public_key: feed.public_key.to_base64(),
                        }),
                        "{channel:?}, {committed:?}"
                    ),
                    FeedStatus::NoKey(_) => {
                        assert_eq!(built, None, "{channel:?}, {committed:?}");
                    }
                    FeedStatus::Development => panic!("{channel:?} is a channel"),
                }
            }
            assert_eq!(
                nova_winres::windows_update_feed(&manifest, None),
                Ok(None),
                "{committed:?}"
            );
            for unknown in ["Nightly", "stable", "beta "] {
                assert_eq!(Channel::from_name(unknown), None, "{unknown:?}");
                assert_eq!(
                    nova_winres::windows_update_feed(&manifest, Some(unknown))
                        .map_err(|error| error.to_string()),
                    Err(ChannelError::Unknown(unknown.to_owned()).to_string()),
                    "{unknown:?}"
                );
            }
        }
    }

    #[test]
    fn each_feed_status_describes_itself() {
        let feed = UpdateFeed {
            url: feed_url(Platform::Macos, Channel::Nightly),
            public_key: PublicKey([7; 32]),
        };
        assert_eq!(
            FeedStatus::Feed(feed).describe(),
            "update feed: https://github.com/edpaget/ev-nova/releases/download/nightly/appcast-macos-nightly.xml"
        );
        assert_eq!(
            FeedStatus::NoKey(Channel::Nightly).describe(),
            "no update feed: NOVA_CHANNEL is nightly but the nova package commits no \
             update_public_key"
        );
        assert_eq!(
            FeedStatus::Development.describe(),
            "no update feed: NOVA_CHANNEL is unset (a development bundle)"
        );
    }
}
