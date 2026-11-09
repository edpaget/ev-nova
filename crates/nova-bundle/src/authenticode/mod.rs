//! Authenticode signing of the Windows builds with Azure Artifact Signing:
//! the mode, chosen from three environment variables, and the steps that
//! sign and verify files with `signtool` and the Artifact Signing dlib.
//!
//! [`Authenticode::from_env`] picks Artifact Signing when all of [`VARS`]
//! are set and non-empty, and unsigned when none is. A mix is an error
//! naming the missing variables, so a half-configured CI fails instead of
//! shipping an unsigned build. The variables are named as the
//! `windows-signing` environment's variables
//! (`infra/windows-signing/github.tf`), which only main's Windows `build`
//! leg enters.

pub mod host;
pub mod sign;
pub mod tools;

use std::ffi::OsString;
use std::path::PathBuf;

use crate::build_id;
use crate::bundle::{self, BundleError, execute, metadata};
use crate::ports::{Env, Files, Process};
use crate::spec::BundleSpec;
use tools::AuthenticodeLayout;

/// The Artifact Signing account's endpoint, e.g.
/// `https://eus.codesigning.azure.net/`.
pub const ENDPOINT: &str = "ARTIFACT_SIGNING_ENDPOINT";
/// The Artifact Signing account's name.
pub const ACCOUNT: &str = "ARTIFACT_SIGNING_ACCOUNT";
/// The certificate profile's name.
pub const PROFILE: &str = "ARTIFACT_SIGNING_PROFILE";

/// The three signing variables.
pub const VARS: [&str; 3] = [ENDPOINT, ACCOUNT, PROFILE];

/// How the Windows files are signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authenticode {
    /// Not at all: what runs without the signing variables (a local build,
    /// a branch, a fork's pull request).
    Unsigned,
    /// With Artifact Signing, through `signtool` and its dlib.
    ArtifactSigning(Account),
}

/// The Artifact Signing account and certificate profile to sign with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The account's endpoint ([`ENDPOINT`]).
    pub endpoint: String,
    /// The account's name ([`ACCOUNT`]).
    pub account: String,
    /// The certificate profile's name ([`PROFILE`]).
    pub profile: String,
    /// The `CorrelationId` the sign requests carry: the build ID
    /// (`NOVA_BUILD_ID`) when it is set.
    pub correlation: Option<String>,
}

/// Why the Windows files could not be signed.
#[derive(Debug, thiserror::Error)]
pub enum AuthenticodeError {
    /// Some of the variables are set and these are not.
    #[error(
        "the Artifact Signing variables are only partly set: {} unset or \
         empty. Set all three of {} to sign, or none to build unsigned",
        missing.join(", "),
        VARS.join(", ")
    )]
    Partial {
        /// The unset or empty variables, in [`VARS`]' order.
        missing: Vec<&'static str>,
    },
    /// A file to sign is not there.
    #[error("{} is missing: build it before signing it", .0.display())]
    MissingFile(PathBuf),
    /// `cargo metadata`, a download, a digest check or `signtool` failed.
    #[error(transparent)]
    Step(#[from] BundleError),
}

/// What [`sign_files`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signed {
    /// Nothing: the mode is [`Authenticode::Unsigned`].
    Skipped,
    /// Signed and verified these files, in place.
    Signed(Vec<PathBuf>),
}

/// Signs and verifies `paths` in place under `signing`
/// ([`sign::steps`]), staging the tools under the target directory
/// `cargo metadata` names. Unsigned runs nothing at all. A missing file is
/// an error before anything runs.
pub fn sign_files(
    process: &mut impl Process,
    files: &mut impl Files,
    cargo: &OsString,
    signing: &Authenticode,
    paths: &[PathBuf],
) -> Result<Signed, AuthenticodeError> {
    if *signing == Authenticode::Unsigned {
        return Ok(Signed::Skipped);
    }
    if let Some(missing) = paths.iter().find(|path| !files.is_file(path)) {
        return Err(AuthenticodeError::MissingFile(missing.clone()));
    }
    let json = bundle::run(process, &metadata(cargo))?;
    let spec = BundleSpec::from_metadata(&json).map_err(BundleError::from)?;
    let layout = AuthenticodeLayout::new(&spec.target_dir);
    for step in sign::steps(signing, &layout, paths) {
        execute(&step, process, files)?;
    }
    Ok(Signed::Signed(paths.to_vec()))
}

impl Authenticode {
    /// The mode the signing variables in `env` choose.
    pub fn from_env(env: &impl Env) -> Result<Self, AuthenticodeError> {
        let values = VARS.map(|name| text(env, name));
        let missing: Vec<_> = VARS
            .iter()
            .zip(&values)
            .filter(|(_, value)| value.is_none())
            .map(|(name, _)| *name)
            .collect();
        if missing.len() == VARS.len() {
            return Ok(Self::Unsigned);
        }
        if !missing.is_empty() {
            return Err(AuthenticodeError::Partial { missing });
        }
        let [endpoint, account, profile] = values.map(Option::unwrap_or_default);
        Ok(Self::ArtifactSigning(Account {
            endpoint,
            account,
            profile,
            correlation: text(env, build_id::VAR),
        }))
    }

    /// The mode, as the signer and the packer report it.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Unsigned => "unsigned: the ARTIFACT_SIGNING_* variables are unset".to_owned(),
            Self::ArtifactSigning(account) => format!(
                "signing with Artifact Signing ({}/{})",
                account.account, account.profile
            ),
        }
    }
}

/// The variable `name` as text, or `None` when it is unset or empty.
fn text(env: &impl Env, name: &str) -> Option<String> {
    env.var(name)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::test_support::FakeEnv;

    fn env(vars: &[(&'static str, &str)]) -> FakeEnv {
        FakeEnv(
            vars.iter()
                .map(|(name, value)| (*name, (*value).into()))
                .collect::<HashMap<_, _>>(),
        )
    }

    const ALL: [(&str, &str); 3] = [
        (ENDPOINT, "https://eus.codesigning.azure.net/"),
        (ACCOUNT, "ev-nova-signing"),
        (PROFILE, "ev-nova-public"),
    ];

    fn account() -> Account {
        Account {
            endpoint: "https://eus.codesigning.azure.net/".into(),
            account: "ev-nova-signing".into(),
            profile: "ev-nova-public".into(),
            correlation: None,
        }
    }

    fn missing(result: Result<Authenticode, AuthenticodeError>) -> Vec<&'static str> {
        match result {
            Err(AuthenticodeError::Partial { missing }) => missing,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn no_variable_is_unsigned() {
        assert_eq!(
            Authenticode::from_env(&env(&[])).expect("mode"),
            Authenticode::Unsigned
        );
    }

    #[test]
    fn all_three_sign_with_that_account() {
        assert_eq!(
            Authenticode::from_env(&env(&ALL)).expect("mode"),
            Authenticode::ArtifactSigning(account())
        );
    }

    #[test]
    fn each_partial_set_names_exactly_the_missing_ones_in_order() {
        for set in 1..(1 << VARS.len()) - 1 {
            let given: Vec<_> = ALL
                .iter()
                .enumerate()
                .filter(|(i, _)| set & (1 << i) != 0)
                .map(|(_, v)| *v)
                .collect();
            let expected: Vec<_> = VARS
                .iter()
                .enumerate()
                .filter(|(i, _)| set & (1 << i) == 0)
                .map(|(_, v)| *v)
                .collect();
            assert_eq!(
                missing(Authenticode::from_env(&env(&given))),
                expected,
                "{given:?}"
            );
        }
    }

    #[test]
    fn an_empty_value_counts_as_unset() {
        let mut given = ALL;
        given[1].1 = "";
        assert_eq!(missing(Authenticode::from_env(&env(&given))), [ACCOUNT]);
        let empty = VARS.map(|name| (name, ""));
        assert_eq!(
            Authenticode::from_env(&env(&empty)).expect("mode"),
            Authenticode::Unsigned
        );
    }

    #[test]
    fn the_build_id_is_the_correlation_and_alone_signs_nothing() {
        let mut given = ALL.to_vec();
        given.push((build_id::VAR, "20261108120000"));
        assert_eq!(
            Authenticode::from_env(&env(&given)).expect("mode"),
            Authenticode::ArtifactSigning(Account {
                correlation: Some("20261108120000".into()),
                ..account()
            })
        );
        assert_eq!(
            Authenticode::from_env(&env(&[(build_id::VAR, "20261108120000")])).expect("mode"),
            Authenticode::Unsigned
        );
    }

    #[test]
    fn describes_each_mode() {
        assert_eq!(
            Authenticode::Unsigned.describe(),
            "unsigned: the ARTIFACT_SIGNING_* variables are unset"
        );
        assert_eq!(
            Authenticode::ArtifactSigning(account()).describe(),
            "signing with Artifact Signing (ev-nova-signing/ev-nova-public)"
        );
    }

    #[test]
    fn the_error_names_all_three_variables() {
        let message = Authenticode::from_env(&env(&ALL[..1]))
            .expect_err("partial")
            .to_string();
        assert!(
            message.contains("ARTIFACT_SIGNING_ACCOUNT, ARTIFACT_SIGNING_PROFILE unset"),
            "{message}"
        );
        for name in VARS {
            assert!(message.contains(name), "{message}");
        }
    }

    mod signing_files {
        use std::path::Path;
        use std::rc::Rc;

        use super::*;
        use crate::authenticode::sign::{sign, verify};
        use crate::authenticode::tools::{AuthenticodeLayout, DLIB, SIGNTOOL};
        use crate::bundle::metadata;
        use crate::digest::DigestTool;
        use crate::ports::{Invocation, RunError};
        use crate::spec::BundleSpec;
        use crate::spec::tests::sample_metadata;
        use crate::test_support::{Event, FakeFiles, Log, Recorder, failing};

        fn cargo() -> OsString {
            "C:/rust/cargo.exe".into()
        }

        fn layout() -> AuthenticodeLayout {
            let spec = BundleSpec::from_metadata(&sample_metadata()).expect("spec");
            AuthenticodeLayout::new(&spec.target_dir)
        }

        fn exes() -> Vec<PathBuf> {
            let release = Path::new("C:/work").join("release");
            vec![release.join("nova.exe"), release.join("nova-dump.exe")]
        }

        struct SignRun {
            result: Result<Signed, AuthenticodeError>,
            events: Vec<Event>,
        }

        impl SignRun {
            fn runs(&self) -> Vec<Invocation> {
                self.events
                    .iter()
                    .filter_map(|e| match e {
                        Event::Run(i) => Some(i.clone()),
                        _ => None,
                    })
                    .collect()
            }

            fn programs(&self) -> Vec<String> {
                self.runs()
                    .iter()
                    .map(|i| {
                        Path::new(&i.program)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    })
                    .collect()
            }
        }

        fn sign_with(
            signing: &Authenticode,
            respond: impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static,
            absent: impl Fn(&Path) -> bool + 'static,
        ) -> SignRun {
            let log = Log::default();
            let mut process = Recorder {
                log: Rc::clone(&log),
                archs: Vec::new(),
                respond: Box::new(respond),
            };
            let mut files = FakeFiles {
                log: Rc::clone(&log),
                fails: Box::new(|_| false),
                absent: Box::new(absent),
            };
            let result = sign_files(&mut process, &mut files, &cargo(), signing, &exes());
            let events = log.borrow().clone();
            SignRun { result, events }
        }

        fn signing() -> Authenticode {
            Authenticode::ArtifactSigning(account())
        }

        fn is_signtool(i: &Invocation, command: &str) -> bool {
            Path::new(&i.program).ends_with("signtool.exe")
                && i.args.first().is_some_and(|a| a == command)
        }

        #[test]
        fn unsigned_runs_nothing() {
            let run = sign_with(&Authenticode::Unsigned, |_| None, |_| true);
            assert_eq!(run.result.expect("skipped"), Signed::Skipped);
            assert_eq!(run.events, []);
        }

        #[test]
        fn signing_reads_the_target_dir_then_stages_signs_and_verifies() {
            let run = sign_with(&signing(), |_| None, |_| false);
            assert_eq!(
                run.result.as_ref().expect("signed"),
                &Signed::Signed(exes())
            );
            assert_eq!(
                run.programs(),
                [
                    "cargo.exe",
                    "curl",
                    "certutil",
                    "tar",
                    "curl",
                    "certutil",
                    "tar",
                    "signtool.exe",
                    "signtool.exe",
                    "signtool.exe",
                    "signtool.exe",
                ]
            );
            let runs = run.runs();
            let layout = layout();
            assert_eq!(runs[0], metadata(&cargo()));
            assert_eq!(
                runs[2],
                DigestTool::Certutil.invocation(&layout.nupkg(&DLIB))
            );
            assert_eq!(
                runs[5],
                DigestTool::Certutil.invocation(&layout.nupkg(&SIGNTOOL))
            );
            let exes = exes();
            assert_eq!(
                runs[7..],
                [
                    sign(&layout, &exes[0]),
                    sign(&layout, &exes[1]),
                    verify(&layout, &exes[0]),
                    verify(&layout, &exes[1]),
                ]
            );
            assert!(
                run.events
                    .iter()
                    .any(|e| matches!(e, Event::Write(path, _) if *path == layout.metadata())),
                "{:?}",
                run.events
            );
        }

        #[test]
        fn a_missing_file_is_an_error_before_anything_runs() {
            let run = sign_with(&signing(), |_| None, |p| p.ends_with("nova-dump.exe"));
            let Err(AuthenticodeError::MissingFile(path)) = &run.result else {
                panic!("{:?}", run.result);
            };
            assert_eq!(path, &exes()[1]);
            assert_eq!(run.events, []);
        }

        #[test]
        fn a_package_with_another_digest_stops_before_unpacking_or_signing() {
            let name = SIGNTOOL.file_name();
            let run = sign_with(
                &signing(),
                move |i| {
                    (i.program == "certutil"
                        && i.args.get(1).is_some_and(|p| Path::new(p).ends_with(&name)))
                    .then(|| Ok(format!("hash:\r\n{}\r\nok\r\n", "0".repeat(64)).into_bytes()))
                },
                |_| false,
            );
            assert!(
                matches!(
                    &run.result,
                    Err(AuthenticodeError::Step(BundleError::Checksum { pinned, .. }))
                        if *pinned == SIGNTOOL.pin.what
                ),
                "{:?}",
                run.result
            );
            assert_eq!(
                run.programs(),
                ["cargo.exe", "curl", "certutil", "tar", "curl", "certutil"]
            );
        }

        #[test]
        fn a_failing_sign_names_its_command_line_and_verifies_nothing() {
            let run = sign_with(&signing(), failing(|i| is_signtool(i, "sign")), |_| false);
            let message = run.result.as_ref().expect_err("fails").to_string();
            assert!(
                message.contains("signtool.exe sign /v /fd SHA256"),
                "{message}"
            );
            assert!(message.contains("nova.exe"), "{message}");
            assert!(message.contains("exited with code 1"), "{message}");
            assert_eq!(
                run.runs().iter().filter(|i| is_signtool(i, "sign")).count(),
                1
            );
            assert!(!run.runs().iter().any(|i| is_signtool(i, "verify")));
        }

        #[test]
        fn an_untimestamped_file_fails_its_verify() {
            let run = sign_with(
                &signing(),
                |i| is_signtool(i, "verify").then_some(Err(RunError::Failed(Some(2)))),
                |_| false,
            );
            let message = run.result.as_ref().expect_err("fails").to_string();
            assert!(
                message.contains("signtool.exe verify /pa /tw /v"),
                "{message}"
            );
            assert!(message.contains("exited with code 2"), "{message}");
        }
    }
}
