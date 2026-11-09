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

use crate::build_id;
use crate::ports::Env;

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
}
