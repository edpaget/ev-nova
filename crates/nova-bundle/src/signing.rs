//! The signing mode, chosen from six environment variables, and the
//! Developer ID steps: a temporary keychain holding the certificate,
//! `codesign` with the hardened runtime, notarization and stapling.
//!
//! [`Signing::from_env`] picks Developer ID when all of [`VARS`] are set
//! and non-empty, and ad-hoc when none is. A mix is an error naming the
//! missing variables, so a half-configured CI fails instead of shipping an
//! ad-hoc build.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::bundle::{BundleError, Step, make_dmg, run, verify};
use crate::layout::Layout;
use crate::notary::Notarization;
use crate::ports::{Env, Invocation, Process};
use crate::sparkle;

/// The base64 of the Developer ID Application `.p12` (certificate and
/// private key).
pub const CERT_P12: &str = "MACOS_CERT_P12_BASE64";
/// The `.p12`'s export password.
pub const CERT_PASSWORD: &str = "MACOS_CERT_PASSWORD";
/// The `codesign --sign` identity, `Developer ID Application: <name>
/// (<team>)`.
pub const IDENTITY: &str = "MACOS_SIGNING_IDENTITY";
/// The base64 of the App Store Connect API key's `AuthKey_<id>.p8`.
pub const API_KEY_P8: &str = "APPLE_API_KEY_P8_BASE64";
/// The API key's id, `notarytool --key-id`.
pub const API_KEY_ID: &str = "APPLE_API_KEY_ID";
/// The API key's issuer, `notarytool --issuer`.
pub const API_ISSUER: &str = "APPLE_API_ISSUER_ID";

/// The six signing variables, named as the CI secrets that fill them.
pub const VARS: [&str; 6] = [
    CERT_P12,
    CERT_PASSWORD,
    IDENTITY,
    API_KEY_P8,
    API_KEY_ID,
    API_ISSUER,
];

/// How the bundle is signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signing {
    /// Ad-hoc (`codesign --sign -`), not notarized: what runs without the
    /// signing variables (a local build, a fork's pull request).
    AdHoc,
    /// Developer ID with the hardened runtime, notarized and stapled.
    DeveloperId(Credentials),
}

/// The Developer ID inputs, decoded.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    /// The `codesign --sign` identity.
    pub identity: OsString,
    /// The `.p12`, decoded.
    pub cert_p12: Vec<u8>,
    /// The `.p12`'s password.
    pub cert_password: OsString,
    /// The `.p8` API key, decoded.
    pub api_key_p8: Vec<u8>,
    /// The API key's id.
    pub api_key_id: OsString,
    /// The API key's issuer.
    pub api_issuer: OsString,
}

/// As derived, but with the password and both keys left out.
impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("identity", &self.identity)
            .field("cert_p12", &"<redacted>")
            .field("cert_password", &"<redacted>")
            .field("api_key_p8", &"<redacted>")
            .field("api_key_id", &self.api_key_id)
            .field("api_issuer", &self.api_issuer)
            .finish()
    }
}

/// Why the signing variables give no mode. Never holds a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SigningError {
    /// Some of the variables are set and these are not.
    #[error(
        "the Developer ID signing variables are only partly set: {} unset or \
         empty. Set all six of {} for a Developer ID build, or none for an \
         ad-hoc one",
        missing.join(", "),
        VARS.join(", ")
    )]
    Partial {
        /// The unset or empty variables, in [`VARS`]' order.
        missing: Vec<&'static str>,
    },
    /// The variable is not base64.
    #[error("{0} is not valid base64")]
    NotBase64(&'static str),
}

impl Signing {
    /// The mode, as the bundler reports it.
    #[must_use]
    pub fn describe(&self) -> &'static str {
        match self {
            Self::AdHoc => "signing ad-hoc (the signing variables are unset)",
            Self::DeveloperId(_) => "signing with Developer ID",
        }
    }

    /// The mode the signing variables in `env` choose.
    pub fn from_env(env: &impl Env) -> Result<Self, SigningError> {
        let values = VARS.map(|name| env.var(name).filter(|v| !v.is_empty()));
        let missing: Vec<_> = VARS
            .iter()
            .zip(&values)
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| *name)
            .collect();
        if missing.len() == VARS.len() {
            return Ok(Self::AdHoc);
        }
        if !missing.is_empty() {
            return Err(SigningError::Partial { missing });
        }
        let [
            cert_p12,
            cert_password,
            identity,
            api_key_p8,
            api_key_id,
            api_issuer,
        ] = values.map(Option::unwrap_or_default);
        Ok(Self::DeveloperId(Credentials {
            identity,
            cert_p12: decode(CERT_P12, &cert_p12)?,
            cert_password,
            api_key_p8: decode(API_KEY_P8, &api_key_p8)?,
            api_key_id,
            api_issuer,
        }))
    }
}

/// Decodes `value`, the variable `name`, ignoring ASCII whitespace so a
/// wrapped encoding works.
fn decode(name: &'static str, value: &OsStr) -> Result<Vec<u8>, SigningError> {
    let text: Vec<u8> = value
        .as_encoded_bytes()
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    STANDARD
        .decode(text)
        .map_err(|_| SigningError::NotBase64(name))
}

/// Bytes that `Debug` never shows: a certificate or key on its way to
/// disk.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretBytes(pub Vec<u8>);

impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} secret bytes>", self.0.len())
    }
}

/// `security`, with these leading words.
fn security(words: &[&str]) -> Invocation {
    Invocation::new("security").args(words)
}

/// `codesign --sign` with the Developer ID found in the temporary
/// keychain, a secure timestamp and, for code (`runtime`), the hardened
/// runtime. No entitlements: the game needs none (the phase body lists
/// those considered).
fn sign(credentials: &Credentials, layout: &Layout, runtime: bool, path: PathBuf) -> Step {
    let mut invocation = Invocation::new("codesign")
        .args(["--force", "--sign"])
        .arg(&credentials.identity)
        .arg("--keychain")
        .arg(layout.keychain());
    if runtime {
        invocation = invocation.args(["--options", "runtime"]);
    }
    Step::Run(invocation.arg("--timestamp").arg(path))
}

/// `spctl --assess` with these words on `path`: Gatekeeper's verdict, which
/// fails the run when it rejects it.
fn assess(words: &[&str], path: PathBuf) -> Step {
    Step::Run(
        Invocation::new("spctl")
            .arg("--assess")
            .args(words)
            .arg("--verbose=2")
            .arg(path),
    )
}

/// `xcrun stapler <action>` on a notarized file.
fn stapler(action: &str, path: PathBuf) -> Step {
    Step::Run(Invocation::new("xcrun").args(["stapler", action]).arg(path))
}

/// The Developer ID steps after staging: the keychain, the signatures
/// (Sparkle's inside out, then `nova-dump`, then the app), notarization,
/// stapling and Gatekeeper's assessment of the app, then the `.dmg` made
/// from the stapled app, signed, notarized, stapled and assessed.
///
/// The keychain's password is the certificate's: the keychain lives in the
/// signing directory, which the run removes, so its password guards
/// nothing beyond the run. It is set not to lock (`-lut 21600`, six hours)
/// during the notary's waits.
#[must_use]
pub fn developer_id(credentials: &Credentials, layout: &Layout) -> Vec<Step> {
    let keychain = layout.keychain();
    let password = &credentials.cert_password;
    let api_key = layout.api_key(&credentials.api_key_id);
    let auth: Vec<OsString> = vec![
        "--key".into(),
        api_key.clone().into(),
        "--key-id".into(),
        credentials.api_key_id.clone(),
        "--issuer".into(),
        credentials.api_issuer.clone(),
    ];
    let notarize = |path: PathBuf| {
        Step::Notarize(Notarization {
            path,
            auth: auth.clone(),
        })
    };
    let mut steps = vec![
        Step::RemoveDir(layout.signing()),
        Step::CreateDir(layout.signing()),
        Step::WriteSecret {
            path: layout.cert(),
            bytes: SecretBytes(credentials.cert_p12.clone()),
        },
        Step::WriteSecret {
            path: api_key,
            bytes: SecretBytes(credentials.api_key_p8.clone()),
        },
        Step::Run(
            security(&["create-keychain", "-p"])
                .secret(password)
                .arg(&keychain),
        ),
        Step::Run(security(&["set-keychain-settings", "-lut", "21600"]).arg(&keychain)),
        Step::Run(
            security(&["unlock-keychain", "-p"])
                .secret(password)
                .arg(&keychain),
        ),
        Step::Run(
            security(&["import"])
                .arg(layout.cert())
                .arg("-k")
                .arg(&keychain)
                .arg("-P")
                .secret(password)
                .args(["-T", "/usr/bin/codesign"]),
        ),
        // Lets codesign use the key without a prompt. Captured, so the key
        // attributes it prints stay out of the log.
        Step::Run(
            security(&[
                "set-key-partition-list",
                "-S",
                "apple-tool:,apple:",
                "-s",
                "-k",
            ])
            .secret(password)
            .arg(&keychain)
            .captured(),
        ),
        Step::AddToSearchList(keychain),
    ];
    // Sparkle's code inside out, each with the hardened runtime and the
    // app's Team ID, so library validation lets `nova` load it.
    steps.extend(
        sparkle::nested_code(&layout.sparkle_framework())
            .into_iter()
            .map(|path| sign(credentials, layout, true, path)),
    );
    steps.extend([
        // nova-dump too: the .dmg's notarization fails on any code in it
        // not signed with the hardened runtime.
        sign(credentials, layout, true, layout.dump()),
        sign(credentials, layout, true, layout.app()),
        verify(layout.app()),
        verify(layout.dump()),
        // The notary takes no bare bundle: the app goes up zipped, and the
        // ticket is stapled to the app itself.
        Step::Run(
            Invocation::new("ditto")
                .args(["-c", "-k", "--keepParent"])
                .arg(layout.app())
                .arg(layout.app_zip()),
        ),
        notarize(layout.app_zip()),
        stapler("staple", layout.app()),
        stapler("validate", layout.app()),
        // Gatekeeper's verdict on the stapled app, offline from its ticket.
        assess(&["--type", "execute"], layout.app()),
        verify(layout.app()),
        Step::CreateDir(layout.out.clone()),
        make_dmg(layout),
        // A disk image isn't code: no hardened runtime.
        sign(credentials, layout, false, layout.dmg()),
        Step::Run(
            Invocation::new("codesign")
                .args(["--verify", "--strict", "--verbose=2"])
                .arg(layout.dmg()),
        ),
        // nova-dump can't be stapled (a bare Mach-O), but the .dmg's ticket
        // covers the copy in it, and notarizing registers it online.
        notarize(layout.dmg()),
        stapler("staple", layout.dmg()),
        stapler("validate", layout.dmg()),
        // A disk image is assessed as a file opened, by its own signature.
        assess(
            &["--type", "open", "--context", "context:primary-signature"],
            layout.dmg(),
        ),
    ]);
    steps
}

/// What runs after the Developer ID steps whatever they did: the keychain
/// deleted, which also takes it off the search list, then the signing
/// directory with the secrets removed.
#[must_use]
pub fn teardown(layout: &Layout) -> Vec<Step> {
    vec![
        Step::Run(security(&["delete-keychain"]).arg(layout.keychain())),
        Step::RemoveDir(layout.signing()),
    ]
}

/// `security list-keychains -d user`: the user's keychain search list,
/// read.
#[must_use]
pub fn list_keychains() -> Invocation {
    security(&["list-keychains", "-d", "user"]).captured()
}

/// The paths [`list_keychains`] prints indented and quoted, one a line.
#[must_use]
pub fn search_list(listed: &[u8]) -> Vec<PathBuf> {
    String::from_utf8_lossy(listed)
        .lines()
        .map(|line| line.trim().trim_matches('"'))
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Puts `keychain` first in the user's keychain search list, keeping the
/// rest in order, with `security list-keychains -d user -s`.
pub fn add_to_search_list(process: &mut impl Process, keychain: &Path) -> Result<(), BundleError> {
    let listed = run(process, &list_keychains())?;
    let rest = search_list(&listed)
        .into_iter()
        .filter(|path| path != keychain);
    run(
        process,
        &security(&["list-keychains", "-d", "user", "-s"])
            .arg(keychain)
            .args(rest),
    )
    .map(drop)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::bundle::{BundleError, plan};
    use crate::ports::Invocation;
    use crate::spec::BundleSpec;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{
        self, Event, FakeEnv, KEYCHAIN_PATHS, P8, P12, ad_hoc_host, credentials, developer_id_host,
        failing, is, paths, run_ok_with, starts,
    };

    /// Every variable set, to values none of which holds another.
    fn full() -> FakeEnv {
        FakeEnv(HashMap::from([
            (CERT_P12, STANDARD.encode(P12).into()),
            (CERT_PASSWORD, "pw-secret-value".into()),
            (IDENTITY, "Developer ID Application: Ed (TEAM123456)".into()),
            (API_KEY_P8, STANDARD.encode(P8).into()),
            (API_KEY_ID, "KEYID12345".into()),
            (API_ISSUER, "issuer-uuid".into()),
        ]))
    }

    #[test]
    fn each_mode_describes_itself_without_a_value() {
        assert_eq!(
            Signing::AdHoc.describe(),
            "signing ad-hoc (the signing variables are unset)"
        );
        assert_eq!(
            Signing::DeveloperId(credentials()).describe(),
            "signing with Developer ID"
        );
    }

    #[test]
    fn no_variables_sign_ad_hoc() {
        assert_eq!(Signing::from_env(&FakeEnv::default()), Ok(Signing::AdHoc));
    }

    #[test]
    fn six_empty_variables_sign_ad_hoc() {
        let env = FakeEnv(VARS.iter().map(|v| (*v, OsString::new())).collect());
        assert_eq!(Signing::from_env(&env), Ok(Signing::AdHoc));
    }

    #[test]
    fn all_six_sign_with_developer_id() {
        assert_eq!(
            Signing::from_env(&full()),
            Ok(Signing::DeveloperId(credentials()))
        );
    }

    #[test]
    fn wrapped_base64_decodes() {
        let mut env = full();
        let wrap = |bytes: &[u8]| {
            let text = STANDARD.encode(bytes);
            let (a, b) = text.split_at(4);
            OsString::from(format!(" {a}\n{b}\r\n\t"))
        };
        env.0.insert(CERT_P12, wrap(P12));
        env.0.insert(API_KEY_P8, wrap(P8));
        assert_eq!(
            Signing::from_env(&env),
            Ok(Signing::DeveloperId(credentials()))
        );
    }

    #[test]
    fn each_missing_variable_is_named_and_no_value_shown() {
        for var in VARS {
            for absent in [None, Some(OsString::new())] {
                let mut env = full();
                match &absent {
                    None => env.0.remove(var),
                    Some(empty) => env.0.insert(var, empty.clone()),
                };
                let error = Signing::from_env(&env).expect_err("partial");
                assert_eq!(error, SigningError::Partial { missing: vec![var] });
                let text = error.to_string();
                assert!(text.contains(var), "{text}");
                for value in full().0.values() {
                    let value = value.to_string_lossy();
                    assert!(!text.contains(value.as_ref()), "{text} shows {value}");
                }
            }
        }
    }

    #[test]
    fn several_missing_are_named_in_order() {
        let mut env = full();
        env.0.remove(API_ISSUER);
        env.0.remove(CERT_P12);
        assert_eq!(
            Signing::from_env(&env),
            Err(SigningError::Partial {
                missing: vec![CERT_P12, API_ISSUER]
            })
        );
    }

    #[test]
    fn invalid_base64_names_its_variable() {
        for var in [CERT_P12, API_KEY_P8] {
            let mut env = full();
            env.0.insert(var, "not base64!".into());
            assert_eq!(Signing::from_env(&env), Err(SigningError::NotBase64(var)));
        }
    }

    #[test]
    fn credentials_debug_hides_the_password_and_keys() {
        let debug = format!("{:?}", credentials());
        assert!(!debug.contains("pw-secret-value"), "{debug}");
        assert!(!debug.contains("p12 bytes"), "{debug}");
        assert!(!debug.contains("PRIVATE KEY"), "{debug}");
        for byte in [P12, P8] {
            assert!(!debug.contains(&format!("{:?}", byte.to_vec())), "{debug}");
        }
        assert!(debug.contains("KEYID12345"), "{debug}");
    }

    const ID: &str = "Developer ID Application: Ed (TEAM123456)";
    const PW: &str = "pw-secret-value";

    fn security(words: &str) -> Invocation {
        Invocation::new("security").args(words.split_whitespace())
    }

    fn codesign(words: &str, path: &Path) -> Invocation {
        Invocation::new("codesign")
            .args(words.split_whitespace())
            .arg(path)
    }

    /// `codesign --sign` with the Developer ID from the temporary keychain.
    fn sign(runtime: &str, path: &Path) -> Invocation {
        let p = paths();
        Invocation::new("codesign")
            .args(["--force", "--sign", ID, "--keychain"])
            .arg(&p.keychain)
            .args(runtime.split_whitespace())
            .arg("--timestamp")
            .arg(path)
    }

    /// `xcrun notarytool <sub> <first>`, then the API-key arguments, then
    /// `rest`.
    fn notarytool(sub: &str, first: &OsStr, rest: &str) -> Invocation {
        Invocation::new("xcrun")
            .args(["notarytool", sub])
            .arg(first)
            .arg("--key")
            .arg(paths().key)
            .args(["--key-id", "KEYID12345", "--issuer", "issuer-uuid"])
            .args(rest.split_whitespace())
    }

    /// The three notary runs for `path`, accepted as `abc`.
    fn notarized(path: &Path) -> Vec<Event> {
        vec![
            Event::Run(notarytool("submit", path.as_os_str(), "--output-format json").captured()),
            Event::Run(notarytool("wait", OsStr::new("abc"), "--timeout 1h")),
            Event::Run(notarytool("info", OsStr::new("abc"), "--output-format json").captured()),
        ]
    }

    /// `spctl --assess <words> --verbose=2 <path>`.
    fn spctl(words: &str, path: &Path) -> Invocation {
        Invocation::new("spctl")
            .arg("--assess")
            .args(words.split_whitespace())
            .arg("--verbose=2")
            .arg(path)
    }

    #[test]
    fn a_rejected_assessment_of_the_app_makes_no_dmg_and_cleans_up() {
        let app = paths().app;
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(move |i| starts(i, "spctl", &[]) && i.args.last() == Some(&app.clone().into())),
            |_| false,
        );
        let error = run.result.as_ref().expect_err("fails").to_string();
        assert!(
            error.starts_with("`spctl --assess --type execute"),
            "{error}"
        );
        assert!(!run.ran("hdiutil"));
        assert!(ends_with_cleanup(&run));
    }

    #[test]
    fn a_rejected_assessment_of_the_dmg_fails_the_bundle_and_cleans_up() {
        let dmg = paths().dmg;
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(move |i| starts(i, "spctl", &[]) && i.args.last() == Some(&dmg.clone().into())),
            |_| false,
        );
        let error = run.result.as_ref().expect_err("fails").to_string();
        assert!(error.starts_with("`spctl --assess --type open"), "{error}");
        assert!(run.ran("hdiutil"));
        assert!(ends_with_cleanup(&run));
    }

    fn stapler(action: &str, path: &Path) -> Event {
        Event::Run(Invocation::new("xcrun").args(["stapler", action]).arg(path))
    }

    /// Phase 5's events up to signing: the build and the staging tree.
    fn staged() -> Vec<Event> {
        let mut events = run_ok_with(&ad_hoc_host()).events;
        let symlink = events
            .iter()
            .position(|e| matches!(e, Event::Symlink(..)))
            .expect("the Applications link");
        events.truncate(symlink + 1);
        events
    }

    fn cleanup() -> Vec<Event> {
        let p = paths();
        vec![
            Event::Run(security("delete-keychain").arg(&p.keychain)),
            Event::RemoveDir(p.signing),
        ]
    }

    fn ends_with_cleanup(run: &test_support::Run) -> bool {
        run.events.ends_with(&cleanup())
    }

    #[test]
    fn signs_notarizes_and_staples_the_app_then_the_dmg() {
        let p = paths();
        let kc = &p.keychain;
        let sparkle_b = p.framework.join("Versions").join("B");
        let run = run_ok_with(&developer_id_host());
        assert_eq!(run.result.as_ref().expect("bundled"), &p.dmg);

        let mut expected = staged();
        expected.extend([
            Event::RemoveDir(p.signing.clone()),
            Event::CreateDir(p.signing.clone()),
            Event::WriteSecret(p.cert.clone(), P12.to_vec()),
            Event::WriteSecret(p.key.clone(), P8.to_vec()),
            Event::Run(security("create-keychain -p").secret(PW).arg(kc)),
            Event::Run(security("set-keychain-settings -lut 21600").arg(kc)),
            Event::Run(security("unlock-keychain -p").secret(PW).arg(kc)),
            Event::Run(
                security("import")
                    .arg(&p.cert)
                    .arg("-k")
                    .arg(kc)
                    .arg("-P")
                    .secret(PW)
                    .args(["-T", "/usr/bin/codesign"]),
            ),
            Event::Run(
                security("set-key-partition-list -S apple-tool:,apple: -s -k")
                    .secret(PW)
                    .arg(kc)
                    .captured(),
            ),
            Event::Run(security("list-keychains -d user").captured()),
            Event::Run(
                security("list-keychains -d user -s")
                    .arg(kc)
                    .args(KEYCHAIN_PATHS),
            ),
            Event::Run(sign("--options runtime", &sparkle_b.join("Autoupdate"))),
            Event::Run(sign("--options runtime", &sparkle_b.join("Updater.app"))),
            Event::Run(sign("--options runtime", &p.framework)),
            Event::Run(sign("--options runtime", &p.dump)),
            Event::Run(sign("--options runtime", &p.app)),
            Event::Run(codesign("--verify --deep --strict --verbose=2", &p.app)),
            Event::Run(codesign("--verify --deep --strict --verbose=2", &p.dump)),
            Event::Run(
                Invocation::new("ditto")
                    .args(["-c", "-k", "--keepParent"])
                    .arg(&p.app)
                    .arg(&p.zip),
            ),
        ]);
        expected.extend(notarized(&p.zip));
        expected.extend([
            stapler("staple", &p.app),
            stapler("validate", &p.app),
            Event::Run(spctl("--type execute", &p.app)),
            Event::Run(codesign("--verify --deep --strict --verbose=2", &p.app)),
            Event::CreateDir(p.out.clone()),
            Event::Run(
                Invocation::new("hdiutil")
                    .args(["create", "-volname", "EV Nova", "-srcfolder"])
                    .arg(p.app.parent().expect("staging"))
                    .args(["-fs", "HFS+", "-format", "UDZO", "-ov"])
                    .arg(&p.dmg),
            ),
            Event::Run(sign("", &p.dmg)),
            Event::Run(codesign("--verify --strict --verbose=2", &p.dmg)),
        ]);
        expected.extend(notarized(&p.dmg));
        expected.extend([
            stapler("staple", &p.dmg),
            stapler("validate", &p.dmg),
            Event::Run(spctl(
                "--type open --context context:primary-signature",
                &p.dmg,
            )),
        ]);
        expected.extend(cleanup());
        assert_eq!(run.events, expected);
    }

    #[test]
    fn no_signature_carries_entitlements() {
        let run = run_ok_with(&developer_id_host());
        let signs: Vec<_> = run
            .runs()
            .into_iter()
            .filter(|i| starts(i, "codesign", &["--force"]))
            .collect();
        assert_eq!(
            signs.len(),
            6,
            "Sparkle's three, nova-dump, the app and the dmg"
        );
        for sign in signs {
            assert!(!sign.args.iter().any(|a| a == "--entitlements"), "{sign}");
        }
    }

    #[test]
    fn the_password_is_secret_wherever_it_is_passed() {
        let run = run_ok_with(&developer_id_host());
        let with_password: Vec<_> = run
            .runs()
            .into_iter()
            .filter(|i| i.args.iter().any(|a| a == PW))
            .collect();
        assert_eq!(with_password.len(), 4);
        for invocation in with_password {
            assert!(!invocation.to_string().contains(PW), "{invocation}");
        }
    }

    #[test]
    fn the_keychain_goes_first_in_the_search_list_once() {
        let p = paths();
        let listed =
            format!("  \"{}\"\n\n  \"/a/b.keychain-db\"\n", p.keychain.display()).into_bytes();
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            move |i| starts(i, "security", &["list-keychains"]).then(|| Ok(listed.clone())),
            |_| false,
        );
        assert!(run.result.is_ok(), "{:?}", run.result);
        let set = run
            .runs()
            .into_iter()
            .find(|i| starts(i, "security", &["list-keychains", "-d", "user", "-s"]))
            .expect("the list is set");
        assert_eq!(
            set,
            security("list-keychains -d user -s")
                .arg(&p.keychain)
                .arg("/a/b.keychain-db")
        );
    }

    #[test]
    fn a_failing_list_of_keychains_stops_before_setting_it() {
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(|i| *i == security("list-keychains -d user").captured()),
            |_| false,
        );
        assert!(matches!(run.result, Err(BundleError::Run { .. })));
        assert!(!run.ran_args("security", &["list-keychains", "-d", "user", "-s"]));
        assert!(!run.ran("codesign"));
        assert!(ends_with_cleanup(&run));
    }

    #[test]
    fn a_failing_import_signs_nothing_cleans_up_and_hides_the_password() {
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(is("security", "import")),
            |_| false,
        );
        let error = run.result.as_ref().expect_err("fails").to_string();
        assert!(error.contains("security import"), "{error}");
        assert!(error.contains("***"), "{error}");
        assert!(!error.contains(PW), "{error}");
        assert!(!run.ran("codesign"));
        assert!(ends_with_cleanup(&run));
    }

    #[test]
    fn a_failing_secret_write_stops_before_the_keychain() {
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            |_| None,
            |e| matches!(e, Event::WriteSecret(..)),
        );
        let Err(BundleError::File { action, path, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert_eq!(*action, "write");
        assert_eq!(path, &paths().cert);
        assert!(!run.ran_args("security", &["create-keychain"]));
        assert!(ends_with_cleanup(&run));
    }

    #[test]
    fn a_failing_staple_or_validation_of_the_app_makes_no_dmg() {
        for action in ["staple", "validate"] {
            let run = test_support::run(
                &developer_id_host(),
                b"arm64",
                failing(move |i| starts(i, "xcrun", &["stapler", action])),
                |_| false,
            );
            let error = run.result.as_ref().expect_err("fails").to_string();
            assert!(
                error.contains(&format!("xcrun stapler {action}")),
                "{error}"
            );
            assert!(!run.ran("hdiutil"), "{action}");
            assert!(ends_with_cleanup(&run), "{action}");
        }
    }

    #[test]
    fn a_failing_signature_of_the_dmg_stops_before_notarizing_it() {
        let dmg = paths().dmg;
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(move |i| {
                starts(i, "codesign", &["--force"]) && i.args.last() == Some(&dmg.clone().into())
            }),
            |_| false,
        );
        assert!(run.result.is_err());
        let submits = run
            .runs()
            .into_iter()
            .filter(|i| starts(i, "xcrun", &["notarytool", "submit"]))
            .count();
        assert_eq!(submits, 1, "only the app was submitted");
        assert!(ends_with_cleanup(&run));
    }

    #[test]
    fn a_failing_cleanup_after_a_good_run_is_the_error() {
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(is("security", "delete-keychain")),
            |_| false,
        );
        let error = run.result.as_ref().expect_err("fails").to_string();
        assert!(error.contains("security delete-keychain"), "{error}");
        assert!(ends_with_cleanup(&run), "both cleanup steps ran");
    }

    #[test]
    fn a_failing_removal_of_the_signing_dir_is_the_error() {
        let signing = paths().signing;
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            |_| None,
            move |e| *e == Event::RemoveDir(signing.clone()),
        );
        let Err(BundleError::File { action, path, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert_eq!(*action, "remove");
        assert_eq!(path, &paths().signing);
        // The first removal (before the secrets) failed: nothing was
        // written, and cleanup still ran.
        assert!(
            !run.events
                .iter()
                .any(|e| matches!(e, Event::WriteSecret(..)))
        );
        assert!(ends_with_cleanup(&run));
    }

    #[test]
    fn a_main_failure_wins_over_a_cleanup_failure_and_cleanup_goes_on() {
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(|i| starts(i, "ditto", &[]) || starts(i, "security", &["delete-keychain"])),
            |_| false,
        );
        let error = run.result.as_ref().expect_err("fails").to_string();
        assert!(error.starts_with("`ditto"), "{error}");
        assert!(ends_with_cleanup(&run), "both cleanup steps ran");
    }

    #[test]
    fn the_first_cleanup_failure_wins_over_a_later_one() {
        let p = paths();
        let signing = p.signing.clone();
        // Fail only the second removal of the signing dir: the cleanup's.
        let removals = std::cell::Cell::new(0);
        let run = test_support::run(
            &developer_id_host(),
            b"arm64",
            failing(is("security", "delete-keychain")),
            move |e| {
                if *e != Event::RemoveDir(signing.clone()) {
                    return false;
                }
                removals.set(removals.get() + 1);
                removals.get() == 2
            },
        );
        let Err(BundleError::Run { command, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert_eq!(
            command,
            &security("delete-keychain").arg(&p.keychain).to_string()
        );
        assert!(ends_with_cleanup(&run), "both cleanup steps ran");
    }

    #[test]
    fn the_cleanup_is_the_keychain_then_the_signing_dir() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        let layout = Layout::new(&spec.target_dir, None).expect("layout");
        assert_eq!(
            plan(&spec, &layout, &developer_id_host(), &test_support::build()).cleanup,
            teardown(&layout)
        );
        assert_eq!(
            teardown(&layout),
            [
                Step::Run(security("delete-keychain").arg(layout.keychain())),
                Step::RemoveDir(layout.signing()),
            ]
        );
    }

    #[test]
    fn secret_bytes_show_only_their_length() {
        assert_eq!(
            format!("{:?}", SecretBytes(P12.to_vec())),
            format!("<{} secret bytes>", P12.len())
        );
    }

    #[test]
    fn a_secret_write_step_hides_its_bytes() {
        let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
        let layout = Layout::new(&spec.target_dir, None).expect("layout");
        let debug = format!("{:?}", developer_id(&credentials(), &layout));
        assert!(!debug.contains(&format!("{:?}", P12.to_vec())), "{debug}");
        assert!(!debug.contains(&format!("{:?}", P8.to_vec())), "{debug}");
        assert!(!debug.contains(PW), "{debug}");
    }
}
