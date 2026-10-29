//! The update feeds: one Sparkle appcast per platform per channel
//! (`appcast-macos-nightly.xml`, `appcast-windows-nightly.xml`, and the
//! same for `beta`), each offering one build, signed with Ed25519, which
//! Sparkle on macOS and `WinSparkle` on Windows read. Each channel's feeds
//! are assets of its own rolling prerelease ([`Channel::tag`]). The beta
//! channel is promoted from the nightly ([`Channel::promoted_from`]):
//! [`promote`] is `nova-appcast check-promotion`'s decision, which build may
//! be promoted and which commit of `main` it was built from.
//!
//! Each feed's item names its update file by the build ID
//! ([`update_asset`], `ev-nova-<id>-<target>.<ext>`), a copy of the
//! stable-named installer ([`Platform::stable_asset`]), so a feed never
//! points at a file a later upload replaces mid-download. Its
//! `sparkle:version` is the build in the form the platform's app reports
//! its own in ([`Platform::sparkle_version`]): the 14 digits for Sparkle,
//! `YYYYMMDD.HHMMSS` for `WinSparkle`, whose `atoi` can't read them.
//! [`render`]
//! writes a feed and [`read_feed`] reads a published one back.
//! [`publish`] is `nova-appcast publish`'s whole decision, for the channel
//! `NOVA_CHANNEL` names: whether to sign
//! ([`publish::FeedSigning`]), refusing a build that isn't newer than the
//! feeds already offer, and which release assets to delete ([`prune`]).
//! [`host`] reads `nova-appcast`'s arguments.
//!
//! The signature is plain Ed25519 over the whole file, as Sparkle's
//! `sign_update` makes, through the [`Signer`] port. The public key is
//! committed as `update_public_key` in the game package's
//! `[package.metadata.bundle]` ([`BundleSpec::update_public_key`]); the
//! seed reaches CI as [`publish::SEED_VAR`], from the `update-signing`
//! environment.

pub mod host;
pub mod key;
pub mod promote;
pub mod prune;
pub mod publish;

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::build_id::{BuildId, BuildIdError};
use crate::bundle;
use crate::installer::{self, RELAUNCH_PROPERTY};
use crate::layout;
use crate::ports::Signer;
use crate::site::{self, REPO};
#[cfg(doc)]
use crate::spec::BundleSpec;
use crate::xml::escape;

/// Sparkle's XML namespace, which `WinSparkle` reads too.
pub const SPARKLE_NS: &str = "http://www.andymatuschak.org/xml-namespaces/sparkle";

/// A platform with an update feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Platform {
    /// Sparkle, updating `Nova.app` from the `.dmg`.
    Macos,
    /// `WinSparkle`, running the MSI.
    Windows,
}

impl Platform {
    /// Every platform, in the order the feeds are written.
    pub const ALL: [Self; 2] = [Self::Macos, Self::Windows];

    /// The target its installer is built for.
    #[must_use]
    pub fn target(self) -> &'static str {
        match self {
            Self::Macos => bundle::TARGET,
            Self::Windows => installer::TARGET,
        }
    }

    /// Its installer's extension.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Macos => "dmg",
            Self::Windows => "msi",
        }
    }

    /// The installer's stable release asset, as its packer names it.
    #[must_use]
    pub fn stable_asset(self) -> &'static str {
        match self {
            Self::Macos => layout::DMG,
            Self::Windows => installer::MSI,
        }
    }

    /// Its name in a feed's name and in `sparkle:os`.
    #[must_use]
    pub fn os(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
        }
    }

    /// The `sparkle:version` its feed offers `build` as: the form its app
    /// reports its own build in, as its updater compares them. Sparkle
    /// reads `Nova.app`'s `CFBundleVersion`, the 14 digits, a part at a
    /// time as a `long long`; WinSparkle reads `nova.exe`'s build version,
    /// [`BuildId::windows_version`], a part at a time with `atoi`.
    #[must_use]
    pub fn sparkle_version(self, build: &BuildId) -> String {
        match self {
            Self::Macos => build.as_str().to_owned(),
            Self::Windows => build.windows_version(),
        }
    }

    /// Reads [`Platform::sparkle_version`]'s form back.
    ///
    /// # Errors
    ///
    /// [`BuildIdError::Malformed`] for text that isn't this platform's
    /// form of a build ID.
    pub fn parse_sparkle_version(self, text: &str) -> Result<BuildId, BuildIdError> {
        match self {
            Self::Macos => BuildId::parse(text),
            Self::Windows => BuildId::parse_windows_version(text),
        }
    }

    /// The arguments the updater runs the installer with: for the MSI,
    /// `/passive`, so it upgrades without the wizard, and
    /// [`RELAUNCH_PROPERTY`]`=1`, so it relaunches the game once it is
    /// done.
    #[must_use]
    pub fn installer_arguments(self) -> Option<String> {
        match self {
            Self::Macos => None,
            Self::Windows => Some(format!("/passive {RELAUNCH_PROPERTY}=1")),
        }
    }
}

/// A release channel with feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// The rolling prerelease of `main`.
    Nightly,
    /// The rolling prerelease of nightlies promoted by hand
    /// (`promote-beta`): each the same commit and build ID as its nightly,
    /// rebuilt for this channel.
    Beta,
}

impl Channel {
    /// Every channel.
    pub const ALL: [Self; 2] = [Self::Nightly, Self::Beta];

    /// The channel this one's builds are promoted from: the nightly for
    /// beta, nothing for the nightly, which builds `main` itself. The
    /// Flatpak publish copies a promoted channel's commit from this
    /// channel's branch instead of importing a bundle.
    #[must_use]
    pub fn promoted_from(self) -> Option<Self> {
        match self {
            Self::Nightly => None,
            Self::Beta => Some(Self::Nightly),
        }
    }

    /// The channel named `name` ([`Channel::name`]), if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|channel| channel.name() == name)
    }

    /// Its name, in a feed's name and title.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Nightly => "nightly",
            Self::Beta => "beta",
        }
    }

    /// The tag of the release its feeds and files are assets of.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Nightly => site::NIGHTLY,
            Self::Beta => site::BETA,
        }
    }
}

/// The feed's release asset: `appcast-<os>-<channel>.xml`.
#[must_use]
pub fn feed_name(platform: Platform, channel: Channel) -> String {
    format!("appcast-{}-{}.xml", platform.os(), channel.name())
}

/// The update file's release asset: `ev-nova-<id>-<target>.<ext>`.
#[must_use]
pub fn update_asset(platform: Platform, build: &BuildId) -> String {
    format!(
        "ev-nova-{build}-{}.{}",
        platform.target(),
        platform.extension()
    )
}

/// The platform and build an update file's name holds, or `None` for any
/// other name ([`update_asset`]'s inverse).
#[must_use]
pub fn parse_update_asset(name: &str) -> Option<(Platform, BuildId)> {
    let rest = name.strip_prefix("ev-nova-")?;
    Platform::ALL.into_iter().find_map(|platform| {
        let suffix = format!("-{}.{}", platform.target(), platform.extension());
        let id = rest.strip_suffix(&suffix)?;
        BuildId::parse(id).ok().map(|build| (platform, build))
    })
}

/// Everything one feed says.
#[derive(Clone, PartialEq, Eq)]
pub struct Feed {
    /// The platform it updates.
    pub platform: Platform,
    /// Its channel.
    pub channel: Channel,
    /// The game's name, as `EV Nova`.
    pub name: String,
    /// The build it offers: `sparkle:version`, in the platform's form
    /// ([`Platform::sparkle_version`]).
    pub build: BuildId,
    /// The build's semver: `sparkle:shortVersionString`.
    pub version: String,
    /// The update file's release asset.
    pub asset: String,
    /// The update file's length in bytes.
    pub length: u64,
    /// The Ed25519 signature of the update file's bytes.
    pub signature: [u8; 64],
}

/// As derived, with the signature in base64.
impl fmt::Debug for Feed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Feed")
            .field("platform", &self.platform)
            .field("channel", &self.channel)
            .field("name", &self.name)
            .field("build", &self.build)
            .field("version", &self.version)
            .field("asset", &self.asset)
            .field("length", &self.length)
            .field("signature", &STANDARD.encode(self.signature))
            .finish()
    }
}

impl Feed {
    /// The feed offering `bytes`, the installer of `build` (semver
    /// `version`) for `platform`, signed by `signer`.
    #[must_use]
    pub fn signed(
        platform: Platform,
        channel: Channel,
        name: &str,
        build: &BuildId,
        version: &str,
        bytes: &[u8],
        signer: &impl Signer,
    ) -> Self {
        Self {
            platform,
            channel,
            name: name.to_owned(),
            build: build.clone(),
            version: version.to_owned(),
            asset: update_asset(platform, build),
            length: bytes.len() as u64,
            signature: signer.sign(bytes),
        }
    }
}

/// The feed as a Sparkle appcast, every value escaped.
#[must_use]
pub fn render(feed: &Feed) -> String {
    let tag = feed.channel.tag();
    let title = escape(&format!("{} {}", feed.name, feed.channel.name()));
    let link = escape(&format!("{REPO}/releases/tag/{tag}"));
    let item_title = escape(&format!("{} {} ({})", feed.name, feed.version, feed.build));
    let build = escape(&feed.platform.sparkle_version(&feed.build));
    let version = escape(&feed.version);
    let url = escape(&site::release_asset(REPO, tag, &feed.asset));
    let arguments = feed
        .platform
        .installer_arguments()
        .map_or_else(String::new, |a| {
            format!(" sparkle:installerArguments=\"{}\"", escape(&a))
        });
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <rss version=\"2.0\" xmlns:sparkle=\"{SPARKLE_NS}\">\n\
         \x20 <channel>\n\
         \x20   <title>{title}</title>\n\
         \x20   <link>{link}</link>\n\
         \x20   <item>\n\
         \x20     <title>{item_title}</title>\n\
         \x20     <sparkle:version>{build}</sparkle:version>\n\
         \x20     <sparkle:shortVersionString>{version}</sparkle:shortVersionString>\n\
         \x20     <enclosure url=\"{url}\" length=\"{length}\" type=\"application/octet-stream\" \
         sparkle:os=\"{os}\"{arguments} sparkle:edSignature=\"{signature}\"/>\n\
         \x20   </item>\n\
         \x20 </channel>\n\
         </rss>\n",
        length = feed.length,
        os = feed.platform.os(),
        signature = STANDARD.encode(feed.signature),
    )
}

/// What a published feed offers: its build, and the release asset its
/// enclosure names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentFeed {
    /// Its `sparkle:version`, read back to the build ID.
    pub build: BuildId,
    /// The last path segment of its enclosure's URL.
    pub file: String,
}

/// Why a published feed could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeedError {
    /// It is not XML.
    #[error("it is not XML: {0}")]
    Xml(String),
    /// Its item has no `sparkle:version`.
    #[error("its item has no sparkle:version")]
    NoVersion,
    /// Its `sparkle:version` is not a build ID in its platform's form.
    #[error("its sparkle:version: {0}")]
    BadVersion(BuildIdError),
    /// Its item has no enclosure with a URL.
    #[error("its item has no enclosure url")]
    NoEnclosure,
}

/// Reads a published feed of `platform`'s: its build and file.
///
/// # Errors
///
/// Text that isn't XML, or a feed whose item lacks a build ID in the
/// platform's form ([`Platform::sparkle_version`]) or an enclosure URL.
pub fn read_feed(platform: Platform, text: &str) -> Result<CurrentFeed, FeedError> {
    let doc = roxmltree::Document::parse(text).map_err(|e| FeedError::Xml(e.to_string()))?;
    let item = doc
        .descendants()
        .find(|n| n.has_tag_name("item"))
        .ok_or(FeedError::NoVersion)?;
    let version = item
        .children()
        .find(|n| n.has_tag_name((SPARKLE_NS, "version")))
        .and_then(|n| n.text())
        .ok_or(FeedError::NoVersion)?;
    let build = platform
        .parse_sparkle_version(version.trim())
        .map_err(FeedError::BadVersion)?;
    let url = item
        .children()
        .find(|n| n.has_tag_name("enclosure"))
        .and_then(|n| n.attribute("url"))
        .ok_or(FeedError::NoEnclosure)?;
    let file = url.rsplit('/').next().unwrap_or_default();
    if file.is_empty() {
        return Err(FeedError::NoEnclosure);
    }
    Ok(CurrentFeed {
        build,
        file: file.to_owned(),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use ed25519_dalek::{Signature, SigningKey, Verifier as _, VerifyingKey};
    use roxmltree::{Document, Node};

    use super::*;
    use crate::system::Ed25519Signer;

    /// The fixed test seed: never a real key.
    pub(crate) const SEED: [u8; 32] = [0x42; 32];

    /// [`SEED`] as the base64 `UPDATE_SIGNING_KEY` holds.
    pub(crate) fn seed_base64() -> String {
        STANDARD.encode(SEED)
    }

    /// The signer of [`SEED`].
    pub(crate) fn test_signer() -> Ed25519Signer {
        Ed25519Signer::from_base64_seed(&seed_base64()).expect("a seed")
    }

    /// [`SEED`]'s public key, made by ed25519-dalek directly.
    pub(crate) fn verifying_key() -> VerifyingKey {
        VerifyingKey::from(&SigningKey::from_bytes(&SEED))
    }

    pub(crate) fn build() -> BuildId {
        BuildId::parse("20261028123456").expect("an ID")
    }

    fn feed(platform: Platform, bytes: &[u8]) -> Feed {
        Feed::signed(
            platform,
            Channel::Nightly,
            "EV Nova",
            &build(),
            "0.1.0",
            bytes,
            &test_signer(),
        )
    }

    fn item<'a, 'i>(doc: &'a Document<'i>) -> Node<'a, 'i> {
        doc.descendants()
            .find(|n| n.has_tag_name("item"))
            .expect("an item")
    }

    fn sparkle<'a>(node: Node<'a, '_>, name: &str) -> &'a str {
        node.children()
            .find(|n| n.has_tag_name((SPARKLE_NS, name)))
            .and_then(|n| n.text())
            .unwrap_or_else(|| panic!("no sparkle:{name}"))
    }

    fn enclosure<'a, 'i>(doc: &'a Document<'i>) -> Node<'a, 'i> {
        item(doc)
            .children()
            .find(|n| n.has_tag_name("enclosure"))
            .expect("an enclosure")
    }

    pub(crate) fn verifies(signature: &str, bytes: &[u8]) -> bool {
        let signature = STANDARD.decode(signature).expect("base64");
        let signature = Signature::from_slice(&signature).expect("64 bytes");
        verifying_key().verify(bytes, &signature).is_ok()
    }

    #[test]
    fn stable_assets_are_the_packers_names() {
        assert_eq!(Platform::Macos.stable_asset(), layout::DMG);
        assert_eq!(Platform::Windows.stable_asset(), installer::MSI);
        for platform in Platform::ALL {
            assert_eq!(
                platform.stable_asset(),
                format!("ev-nova-{}.{}", platform.target(), platform.extension())
            );
        }
        assert_eq!(Platform::Macos.target(), "aarch64-apple-darwin");
        assert_eq!(Platform::Windows.target(), "x86_64-pc-windows-msvc");
    }

    #[test]
    fn update_assets_carry_the_build_id() {
        assert_eq!(
            update_asset(Platform::Macos, &build()),
            "ev-nova-20261028123456-aarch64-apple-darwin.dmg"
        );
        assert_eq!(
            update_asset(Platform::Windows, &build()),
            "ev-nova-20261028123456-x86_64-pc-windows-msvc.msi"
        );
    }

    #[test]
    fn update_asset_names_parse_back() {
        for platform in Platform::ALL {
            assert_eq!(
                parse_update_asset(&update_asset(platform, &build())),
                Some((platform, build()))
            );
        }
    }

    #[test]
    fn stable_and_foreign_names_are_not_update_assets() {
        for name in [
            layout::DMG,
            installer::MSI,
            "ev-nova-2026102812345-aarch64-apple-darwin.dmg",
            "ev-nova-20261328123456-aarch64-apple-darwin.dmg",
            "ev-nova-20261028123456-x86_64-unknown-linux-gnu.flatpak",
            "ev-nova-20261028123456-aarch64-apple-darwin.msi",
            "ev-nova-20261028123456-x86_64-pc-windows-msvc.dmg",
            "nova-20261028123456-aarch64-apple-darwin.dmg",
            "appcast-macos-nightly.xml",
            "",
        ] {
            assert_eq!(parse_update_asset(name), None, "{name}");
        }
    }

    #[test]
    fn feeds_are_named_by_platform_and_channel() {
        assert_eq!(
            feed_name(Platform::Macos, Channel::Nightly),
            "appcast-macos-nightly.xml"
        );
        assert_eq!(
            feed_name(Platform::Windows, Channel::Nightly),
            "appcast-windows-nightly.xml"
        );
        assert_eq!(Channel::Nightly.tag(), site::NIGHTLY);
        assert_eq!(
            feed_name(Platform::Macos, Channel::Beta),
            "appcast-macos-beta.xml"
        );
        assert_eq!(
            feed_name(Platform::Windows, Channel::Beta),
            "appcast-windows-beta.xml"
        );
    }

    #[test]
    fn a_beta_feed_links_the_beta_release_and_keeps_each_platform_s_version_form() {
        for (platform, url, version) in [
            (
                Platform::Macos,
                "https://github.com/edpaget/ev-nova/releases/download/beta/ev-nova-20261028123456-aarch64-apple-darwin.dmg",
                "20261028123456",
            ),
            (
                Platform::Windows,
                "https://github.com/edpaget/ev-nova/releases/download/beta/ev-nova-20261028123456-x86_64-pc-windows-msvc.msi",
                "20261028.123456",
            ),
        ] {
            let feed = Feed {
                channel: Channel::Beta,
                ..feed(platform, b"x")
            };
            let text = render(&feed);
            let doc = Document::parse(&text).expect("XML");
            let channel = doc
                .descendants()
                .find(|n| n.has_tag_name("channel"))
                .expect("a channel");
            let child = |name| {
                channel
                    .children()
                    .find(|n| n.has_tag_name(name))
                    .and_then(|n| n.text())
            };
            assert_eq!(child("title"), Some("EV Nova beta"), "{platform:?}");
            assert_eq!(
                child("link"),
                Some("https://github.com/edpaget/ev-nova/releases/tag/beta"),
                "{platform:?}"
            );
            assert_eq!(enclosure(&doc).attribute("url"), Some(url), "{platform:?}");
            assert_eq!(sparkle(item(&doc), "version"), version, "{platform:?}");
            assert_eq!(
                read_feed(platform, &text),
                Ok(CurrentFeed {
                    build: build(),
                    file: update_asset(platform, &build()),
                })
            );
        }
    }

    #[test]
    fn a_fixed_builds_feeds_hold_its_id_semver_url_length_and_a_verifying_signature() {
        for (platform, bytes, url, version) in [
            (
                Platform::Macos,
                &b"dmg bytes"[..],
                "https://github.com/edpaget/ev-nova/releases/download/nightly/ev-nova-20261028123456-aarch64-apple-darwin.dmg",
                "20261028123456",
            ),
            (
                Platform::Windows,
                &b"msi bytes, longer"[..],
                "https://github.com/edpaget/ev-nova/releases/download/nightly/ev-nova-20261028123456-x86_64-pc-windows-msvc.msi",
                "20261028.123456",
            ),
        ] {
            let text = render(&feed(platform, bytes));
            let doc = Document::parse(&text).expect("XML");
            let rss = doc.root_element();
            assert_eq!(rss.tag_name().name(), "rss");
            assert_eq!(rss.attribute("version"), Some("2.0"));
            let item = item(&doc);
            assert_eq!(sparkle(item, "version"), version, "{platform:?}");
            assert_eq!(sparkle(item, "shortVersionString"), "0.1.0");
            let title = item
                .children()
                .find(|n| n.has_tag_name("title"))
                .and_then(|n| n.text());
            assert_eq!(title, Some("EV Nova 0.1.0 (20261028123456)"));
            let enclosure = enclosure(&doc);
            assert_eq!(enclosure.attribute("url"), Some(url));
            assert_eq!(
                enclosure.attribute("length"),
                Some(bytes.len().to_string().as_str())
            );
            assert_eq!(
                enclosure.attribute("type"),
                Some("application/octet-stream")
            );
            assert_eq!(enclosure.attribute((SPARKLE_NS, "os")), Some(platform.os()));
            let signature = enclosure
                .attribute((SPARKLE_NS, "edSignature"))
                .expect("a signature");
            assert!(verifies(signature, bytes), "{platform:?}");
            assert!(!verifies(signature, b"altered bytes"), "{platform:?}");
            assert_eq!(
                enclosure.attribute((SPARKLE_NS, "installerArguments")),
                platform.installer_arguments().as_deref()
            );
        }
        let windows = render(&feed(Platform::Windows, b"x"));
        let doc = Document::parse(&windows).expect("XML");
        // Quietly, and relaunching the game (the installer's custom action).
        assert_eq!(
            enclosure(&doc).attribute((SPARKLE_NS, "installerArguments")),
            Some(format!("/passive {}=1", installer::RELAUNCH_PROPERTY).as_str())
        );
        assert_eq!(installer::RELAUNCH_PROPERTY, "NOVA_RELAUNCH");
        assert_eq!(
            enclosure(&doc).attribute((SPARKLE_NS, "os")),
            Some("windows")
        );
        let macos = render(&feed(Platform::Macos, b"x"));
        let doc = Document::parse(&macos).expect("XML");
        assert_eq!(
            enclosure(&doc).attribute((SPARKLE_NS, "installerArguments")),
            None
        );
        assert_eq!(enclosure(&doc).attribute((SPARKLE_NS, "os")), Some("macos"));
    }

    #[test]
    fn each_platform_s_sparkle_version_is_the_one_its_app_reports() {
        // Nova.app's CFBundleVersion: Sparkle reads each part as a long long.
        assert_eq!(Platform::Macos.sparkle_version(&build()), build().as_str());
        // nova.exe's WinSparkle build version: atoi reads each part.
        assert_eq!(
            Platform::Windows.sparkle_version(&build()),
            build().windows_version()
        );
        for platform in Platform::ALL {
            assert_eq!(
                platform.parse_sparkle_version(&platform.sparkle_version(&build())),
                Ok(build()),
                "{platform:?}"
            );
        }
    }

    #[test]
    fn a_feed_in_the_other_platform_s_form_is_a_bad_version() {
        assert_eq!(
            Platform::Windows.parse_sparkle_version("20261028123456"),
            Err(BuildIdError::Malformed("20261028123456".into()))
        );
        assert_eq!(
            Platform::Macos.parse_sparkle_version("20261028.123456"),
            Err(BuildIdError::Malformed("20261028.123456".into()))
        );
        let macos = render(&feed(Platform::Macos, b"x"));
        assert!(matches!(
            read_feed(Platform::Windows, &macos),
            Err(FeedError::BadVersion(_))
        ));
    }

    #[test]
    fn the_channel_has_a_title_and_links_its_release() {
        let text = render(&feed(Platform::Macos, b"x"));
        let doc = Document::parse(&text).expect("XML");
        let channel = doc
            .descendants()
            .find(|n| n.has_tag_name("channel"))
            .expect("a channel");
        let child = |name| {
            channel
                .children()
                .find(|n| n.has_tag_name(name))
                .and_then(|n| n.text())
        };
        assert_eq!(child("title"), Some("EV Nova nightly"));
        assert_eq!(
            child("link"),
            Some("https://github.com/edpaget/ev-nova/releases/tag/nightly")
        );
    }

    #[test]
    fn values_with_markup_are_escaped() {
        let mut feed = feed(Platform::Windows, b"x");
        feed.name = "EV & <Nova>".into();
        feed.version = "1.0.0-\"rc\"".into();
        let text = render(&feed);
        let doc = Document::parse(&text).expect("still XML");
        assert_eq!(
            item(&doc)
                .children()
                .find(|n| n.has_tag_name("title"))
                .and_then(|n| n.text()),
            Some("EV & <Nova> 1.0.0-\"rc\" (20261028123456)")
        );
        assert!(text.contains("EV &amp; &lt;Nova&gt;"), "{text}");
    }

    #[test]
    fn a_feeds_debug_shows_the_signature_as_base64() {
        let feed = feed(Platform::Macos, b"x");
        let debug = format!("{feed:?}");
        assert!(debug.contains(&STANDARD.encode(feed.signature)), "{debug}");
    }

    #[test]
    fn a_rendered_feed_reads_back_as_its_build_and_file() {
        for platform in Platform::ALL {
            assert_eq!(
                read_feed(platform, &render(&feed(platform, b"x"))),
                Ok(CurrentFeed {
                    build: build(),
                    file: update_asset(platform, &build()),
                })
            );
        }
    }

    #[test]
    fn a_feed_without_a_version_or_enclosure_or_with_a_bad_id_is_an_error() {
        let good = render(&feed(Platform::Macos, b"x"));
        let without = |from: &str, to: &str| {
            let start = good.find(from).expect("present");
            let end = good[start..].find(to).expect("closed") + start + to.len();
            format!("{}{}", &good[..start], &good[end..])
        };
        assert_eq!(
            read_feed(
                Platform::Macos,
                &without("<sparkle:version>", "</sparkle:version>")
            ),
            Err(FeedError::NoVersion)
        );
        assert_eq!(
            read_feed(Platform::Macos, &without("<enclosure", "/>")),
            Err(FeedError::NoEnclosure)
        );
        assert_eq!(
            read_feed(Platform::Macos, &without("<item>", "</item>")),
            Err(FeedError::NoVersion)
        );
        assert_eq!(
            read_feed(
                Platform::Macos,
                &good.replace("https://github.com", "https://github.com/x/")
            ),
            Ok(CurrentFeed {
                build: build(),
                file: update_asset(Platform::Macos, &build()),
            })
        );
        let trailing = good.replace(".dmg\"", ".dmg/\"");
        assert_eq!(
            read_feed(Platform::Macos, &trailing),
            Err(FeedError::NoEnclosure)
        );
        assert_eq!(
            read_feed(Platform::Macos, &good.replace(">20261028123456<", ">2026<")),
            Err(FeedError::BadVersion(BuildIdError::Malformed(
                "2026".into()
            )))
        );
        assert!(matches!(
            read_feed(Platform::Macos, "not xml"),
            Err(FeedError::Xml(_))
        ));
    }
}
