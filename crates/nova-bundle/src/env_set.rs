//! A set of environment variables that configure one thing together, such
//! as the macOS signing credentials ([`crate::signing::VARS`]) or the
//! Artifact Signing account ([`crate::authenticode::VARS`]).
//!
//! [`all_or_none`] holds the rule both share: every variable set and
//! non-empty configures it, none of them leaves it off, and a mix is an
//! error naming the missing ones, so a half-configured CI fails instead of
//! shipping an unsigned build. Each caller maps the answer onto its own
//! mode and error.

use std::ffi::OsString;

use crate::ports::Env;

/// The values of `names` in `env` when all are set and non-empty, `None`
/// when none is, or, for a mix, the unset or empty names in `names`' order.
pub fn all_or_none<const N: usize>(
    env: &impl Env,
    names: [&'static str; N],
) -> Result<Option<[OsString; N]>, Vec<&'static str>> {
    let values = names.map(|name| env.var(name).filter(|value| !value.is_empty()));
    let missing: Vec<_> = names
        .iter()
        .zip(&values)
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| *name)
        .collect();
    if missing.len() == N {
        Ok(None)
    } else if missing.is_empty() {
        Ok(Some(values.map(Option::unwrap_or_default)))
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::test_support::FakeEnv;

    const NAMES: [&str; 3] = ["ONE", "TWO", "THREE"];

    fn env(vars: &[(&'static str, &str)]) -> FakeEnv {
        FakeEnv(
            vars.iter()
                .map(|(name, value)| (*name, (*value).into()))
                .collect::<HashMap<_, _>>(),
        )
    }

    #[test]
    fn none_set_is_none() {
        assert_eq!(all_or_none(&env(&[]), NAMES), Ok(None));
    }

    #[test]
    fn empty_counts_as_unset() {
        let empty = env(&[("ONE", ""), ("TWO", ""), ("THREE", "")]);
        assert_eq!(all_or_none(&empty, NAMES), Ok(None));
    }

    #[test]
    fn all_set_are_their_values_in_order() {
        let all = env(&[("THREE", "3"), ("ONE", "1"), ("TWO", "2")]);
        assert_eq!(
            all_or_none(&all, NAMES),
            Ok(Some(["1".into(), "2".into(), "3".into()]))
        );
    }

    #[test]
    fn variables_outside_the_set_are_ignored() {
        assert_eq!(all_or_none(&env(&[("OTHER", "x")]), NAMES), Ok(None));
    }

    #[test]
    fn each_mix_names_exactly_the_missing_ones_in_order() {
        for set in 1..(1 << NAMES.len()) - 1 {
            let given: Vec<_> = NAMES
                .iter()
                .enumerate()
                .map(|(i, name)| (*name, if set & (1 << i) != 0 { "v" } else { "" }))
                .collect();
            let expected: Vec<_> = NAMES
                .iter()
                .enumerate()
                .filter(|(i, _)| set & (1 << i) == 0)
                .map(|(_, name)| *name)
                .collect();
            assert_eq!(all_or_none(&env(&given), NAMES), Err(expected), "{given:?}");
        }
    }
}
