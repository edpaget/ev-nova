//! `Nova.app/Contents/Info.plist`: the bundle's identity, its executable
//! and icon, and the system it needs.

use plist::{Dictionary, Value};

use crate::build_id::BuildId;
use crate::channel::UpdateFeed;
use crate::icon::ICON_FILE;
use crate::spec::{BundleSpec, GAME_PACKAGE};

/// The app's executable in `Contents/MacOS`: the game package's binary.
pub const EXECUTABLE: &str = GAME_PACKAGE;

/// Renders the plist as XML.
///
/// `CFBundleVersion`, which Sparkle compares, is the build's ID;
/// `CFBundleShortVersionString`, which the player sees, is the package's
/// version.
///
/// `min_system` is `LSMinimumSystemVersion`: the edge passes the
/// `MACOSX_DEPLOYMENT_TARGET` the binaries are built for, so the two can't
/// drift apart.
///
/// With a `feed`, it adds Sparkle's three keys: `SUFeedURL`, the feed;
/// `SUPublicEDKey`, the key its updates are checked with; and
/// `SUEnableAutomaticChecks`, so the player isn't asked on the second
/// launch. Without one it writes no `SU*` key at all, and the app runs no
/// updater. `SUScheduledCheckInterval` is left at Sparkle's day,
/// `SUAutomaticallyUpdate` unset (the player installs each build), and
/// the XPC services' keys unset, as the bundle carries none.
#[must_use]
pub fn info_plist(
    spec: &BundleSpec,
    build: &BuildId,
    min_system: &str,
    feed: Option<&UpdateFeed>,
) -> Vec<u8> {
    let mut dict = Dictionary::new();
    let mut string = |key: &str, value: &str| {
        dict.insert(key.to_owned(), Value::String(value.to_owned()));
    };
    string("CFBundleIdentifier", &spec.identifier);
    string("CFBundleName", &spec.name);
    string("CFBundleDisplayName", &spec.name);
    string("CFBundleExecutable", EXECUTABLE);
    string("CFBundlePackageType", "APPL");
    string("CFBundleInfoDictionaryVersion", "6.0");
    string("CFBundleShortVersionString", &spec.version);
    string("CFBundleVersion", build.as_str());
    string("CFBundleIconFile", ICON_FILE);
    string("LSMinimumSystemVersion", min_system);
    string("LSApplicationCategoryType", &spec.category);
    string(
        "NSHumanReadableCopyright",
        &format!(
            "Copyright the ev-nova contributors. Free software under {}.",
            spec.license
        ),
    );
    dict.insert("NSHighResolutionCapable".to_owned(), Value::Boolean(true));
    dict.insert(
        "NSSupportsAutomaticGraphicsSwitching".to_owned(),
        Value::Boolean(true),
    );
    if let Some(feed) = feed {
        dict.insert("SUFeedURL".to_owned(), Value::String(feed.url.clone()));
        dict.insert(
            "SUPublicEDKey".to_owned(),
            Value::String(feed.public_key.to_base64()),
        );
        dict.insert("SUEnableAutomaticChecks".to_owned(), Value::Boolean(true));
    }
    let mut xml = Vec::new();
    Value::Dictionary(dict)
        .to_writer_xml(&mut xml)
        .expect("a plist of strings and booleans writes to memory");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appcast::key::PublicKey;
    use crate::spec::tests::sample_metadata;

    fn build() -> BuildId {
        BuildId::parse("20261028123456").expect("an ID")
    }

    fn read_back(xml: &[u8]) -> Dictionary {
        Value::from_reader_xml(xml)
            .expect("xml plist")
            .into_dictionary()
            .expect("a dictionary")
    }

    fn string<'a>(dict: &'a Dictionary, key: &str) -> &'a str {
        dict.get(key)
            .and_then(Value::as_string)
            .unwrap_or_else(|| panic!("{key} is a string"))
    }

    #[test]
    fn holds_the_bundle_keys_with_the_package_version() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        let dict = read_back(&info_plist(&spec, &build(), "13.0", None));
        for (key, value) in [
            ("CFBundleIdentifier", "io.example.nova"),
            ("CFBundleName", "EV Nova"),
            ("CFBundleDisplayName", "EV Nova"),
            ("CFBundleExecutable", "nova"),
            ("CFBundlePackageType", "APPL"),
            ("CFBundleInfoDictionaryVersion", "6.0"),
            ("CFBundleShortVersionString", "0.3.1"),
            ("CFBundleVersion", "20261028123456"),
            ("CFBundleIconFile", ICON_FILE),
            ("LSMinimumSystemVersion", "13.0"),
            (
                "LSApplicationCategoryType",
                "public.app-category.role-playing-games",
            ),
        ] {
            assert_eq!(string(&dict, key), value, "{key}");
        }
        for key in [
            "NSHighResolutionCapable",
            "NSSupportsAutomaticGraphicsSwitching",
        ] {
            assert_eq!(
                dict.get(key).and_then(Value::as_boolean),
                Some(true),
                "{key}"
            );
        }
        assert!(
            string(&dict, "NSHumanReadableCopyright").contains("GPL-3.0-or-later"),
            "names the licence"
        );
        assert_eq!(dict.len(), 14, "no other keys");
    }

    fn feed() -> UpdateFeed {
        UpdateFeed {
            url: "https://example.com/appcast-macos-nightly.xml".into(),
            public_key: PublicKey([7; 32]),
        }
    }

    #[test]
    fn a_bundle_with_a_feed_holds_its_url_key_and_automatic_checks() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        let dict = read_back(&info_plist(&spec, &build(), "13.0", Some(&feed())));
        assert_eq!(
            string(&dict, "SUFeedURL"),
            "https://example.com/appcast-macos-nightly.xml"
        );
        assert_eq!(
            string(&dict, "SUPublicEDKey"),
            PublicKey([7; 32]).to_base64()
        );
        assert_eq!(
            dict.get("SUEnableAutomaticChecks")
                .and_then(Value::as_boolean),
            Some(true)
        );
        assert_eq!(dict.len(), 17, "the bundle's 14 and Sparkle's 3");
        assert_eq!(string(&dict, "CFBundleVersion"), "20261028123456");
    }

    #[test]
    fn a_bundle_without_a_feed_has_no_sparkle_key() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        let dict = read_back(&info_plist(&spec, &build(), "13.0", None));
        let sparkle: Vec<&String> = dict.keys().filter(|key| key.starts_with("SU")).collect();
        assert!(sparkle.is_empty(), "{sparkle:?}");
    }

    #[test]
    fn the_minimum_system_is_the_one_passed_in() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        let dict = read_back(&info_plist(&spec, &build(), "14.2", None));
        assert_eq!(string(&dict, "LSMinimumSystemVersion"), "14.2");
    }

    #[test]
    fn the_short_version_is_the_spec_s_and_the_bundle_version_the_build_s() {
        let mut spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        spec.version = "2.0.0-beta.1".into();
        let build = BuildId::parse("20010203040506").expect("an ID");
        let dict = read_back(&info_plist(&spec, &build, "13.0", None));
        assert_eq!(string(&dict, "CFBundleShortVersionString"), "2.0.0-beta.1");
        assert_eq!(string(&dict, "CFBundleVersion"), "20010203040506");
    }

    #[test]
    fn escapes_markup_in_the_values() {
        let mut spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        spec.name = "EV <Nova> & co".into();
        let dict = read_back(&info_plist(&spec, &build(), "13.0", None));
        assert_eq!(string(&dict, "CFBundleName"), "EV <Nova> & co");
    }
}
