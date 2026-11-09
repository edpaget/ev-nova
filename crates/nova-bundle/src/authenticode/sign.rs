//! The `signtool` runs: sign with the Artifact Signing dlib, SHA-256 and
//! Microsoft's timestamp server, then verify.
//!
//! Verification runs `signtool verify /pa /tw /v`. `/pa` checks against
//! the default Authenticode policy, and `/tw` warns when the signature
//! carries no timestamp, which makes `signtool` exit 2 ("completed with
//! warnings"): the [`crate::ports::Process`] port's non-zero failure stops
//! the run, so an untimestamped file never ships. Artifact Signing's
//! certificates live three days, so a signature without a timestamp would
//! stop verifying soon after.

use std::path::{Path, PathBuf};

use super::Authenticode;
use super::tools::{self, AuthenticodeLayout};
use crate::bundle::Step;
use crate::ports::Invocation;

/// Microsoft's RFC 3161 timestamp server for Artifact Signing.
pub const TIMESTAMP_URL: &str = "http://timestamp.acs.microsoft.com";

/// `signtool sign` of `file` in place, with the staged dlib and
/// `metadata.json`.
#[must_use]
pub fn sign(layout: &AuthenticodeLayout, file: &Path) -> Invocation {
    Invocation::new(layout.signtool_exe())
        .args([
            "sign",
            "/v",
            "/fd",
            "SHA256",
            "/tr",
            TIMESTAMP_URL,
            "/td",
            "SHA256",
        ])
        .arg("/dlib")
        .arg(layout.dlib())
        .arg("/dmdf")
        .arg(layout.metadata())
        .arg(file)
}

/// `signtool verify` of `file`, failing unless it is validly signed and
/// timestamped (see the module docs).
#[must_use]
pub fn verify(layout: &AuthenticodeLayout, file: &Path) -> Invocation {
    Invocation::new(layout.signtool_exe())
        .args(["verify", "/pa", "/tw", "/v"])
        .arg(file)
}

/// Signs every one of `files`, then verifies every one: signing them all
/// first keeps the time from `azure/login` to the last signature short.
/// One run per file, so a failure names its file. The tools must already
/// be staged ([`tools::stage`]).
#[must_use]
pub fn sign_and_verify(layout: &AuthenticodeLayout, files: &[PathBuf]) -> Vec<Step> {
    let signs = files.iter().map(|file| sign(layout, file));
    let verifies = files.iter().map(|file| verify(layout, file));
    signs.chain(verifies).map(Step::Run).collect()
}

/// Every step that signs `files` under `signing`: none when unsigned,
/// else the tools staged, then [`sign_and_verify`].
#[must_use]
pub fn steps(signing: &Authenticode, layout: &AuthenticodeLayout, files: &[PathBuf]) -> Vec<Step> {
    match signing {
        Authenticode::Unsigned => Vec::new(),
        Authenticode::ArtifactSigning(account) => {
            let mut steps = tools::stage(layout, account);
            steps.extend(sign_and_verify(layout, files));
            steps
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authenticode::Account;

    fn layout() -> AuthenticodeLayout {
        AuthenticodeLayout::new(&Path::new("/work").join("ev-nova").join("target"))
    }

    fn file(name: &str) -> PathBuf {
        Path::new("/work").join("ev-nova").join(name)
    }

    fn account() -> Account {
        Account {
            endpoint: "https://eus.codesigning.azure.net/".into(),
            account: "ev-nova-signing".into(),
            profile: "ev-nova-public".into(),
            correlation: None,
        }
    }

    #[test]
    fn signs_with_the_dlib_sha256_and_microsoft_s_timestamp() {
        let layout = layout();
        let root = Path::new("/work")
            .join("ev-nova")
            .join("target")
            .join("authenticode");
        assert_eq!(
            sign(&layout, &file("nova.exe")),
            Invocation::new(root.join("signtool").join("signtool.exe"))
                .args([
                    "sign",
                    "/v",
                    "/fd",
                    "SHA256",
                    "/tr",
                    "http://timestamp.acs.microsoft.com",
                    "/td",
                    "SHA256",
                    "/dlib",
                ])
                .arg(root.join("dlib").join("Azure.CodeSigning.Dlib.dll"))
                .arg("/dmdf")
                .arg(root.join("metadata.json"))
                .arg(file("nova.exe"))
        );
    }

    #[test]
    fn verifies_the_policy_and_the_timestamp() {
        let layout = layout();
        assert_eq!(
            verify(&layout, &file("nova.exe")),
            Invocation::new(layout.signtool_exe())
                .args(["verify", "/pa", "/tw", "/v"])
                .arg(file("nova.exe"))
        );
    }

    #[test]
    fn unsigned_has_no_step() {
        assert_eq!(
            steps(&Authenticode::Unsigned, &layout(), &[file("nova.exe")]),
            []
        );
    }

    #[test]
    fn signing_stages_then_signs_in_order_then_verifies() {
        let layout = layout();
        let files = [file("nova.exe"), file("nova-dump.exe")];
        let mut expected = tools::stage(&layout, &account());
        expected.extend([
            Step::Run(sign(&layout, &files[0])),
            Step::Run(sign(&layout, &files[1])),
            Step::Run(verify(&layout, &files[0])),
            Step::Run(verify(&layout, &files[1])),
        ]);
        assert_eq!(
            steps(&Authenticode::ArtifactSigning(account()), &layout, &files),
            expected
        );
        assert_eq!(
            sign_and_verify(&layout, &files),
            expected[expected.len() - 4..]
        );
    }
}
