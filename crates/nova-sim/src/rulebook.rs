//! Rules where the Nova Bible and the original `EV Nova` engine disagree:
//! each such rule can follow either, by the [`Rulebook`].
//!
//! The project's strategy is that wherever the Bible says one thing and
//! the engine does another, the rule is written both ways and the player
//! chooses. A [`Rulebook`] holds a default [`RuleSource`] for every such
//! rule (the engine's, unless chosen otherwise) and an override for any
//! single rule, named by its [`RuleKey`]. A rule asks the rulebook
//! [`Rulebook::source_for`] its key, and the rule's implementation makes
//! the decision. The settings file names the default `"rules"` and the
//! overrides `"rule_overrides"`, by each key's [`RuleKey::key`].
//!
//! The rules here so far:
//!
//! | [`RuleKey`] | settings key | engine | Bible |
//! |---|---|---|---|
//! | [`CrimeGains`](RuleKey::CrimeGains) | `crime_gains` | a crime pleases every government not allied with the victim's ([`NovaLaw`](crate::NovaLaw)) | only the victim's enemies |
//!
//! # Adding a rule
//!
//! 1. Add a variant, its doc comment and its settings key to the
//!    `rule_keys!` table below (and a row to the table above). That is the
//!    only list: [`RuleKey::ALL`], [`RuleKey::key`] and
//!    [`RuleKey::from_key`] come from it, and the settings file reads the
//!    key with no further change.
//! 2. Give the rule's implementation its [`RuleSource`], taken from the
//!    rulebook with [`Rulebook::source_for`] where the rule is built (as
//!    [`NovaLaw::from_rulebook`](crate::NovaLaw::from_rulebook) does), and
//!    make the decision there.
//! 3. Test both readings, [`RuleSource::Engine`] and [`RuleSource::Bible`],
//!    and that the rule follows its rulebook entry; extend this module's
//!    key test to the new key. The second rule to join also drops the
//!    `RuleKey::index` exclusion from `.cargo/mutants.toml`, which only
//!    holds while there is one rule.
//!
//! At the edge of the program, every rule is built from the one rulebook
//! the settings give (see `nova::rulebook`).

/// Which of the two a disputed rule follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuleSource {
    /// What the original `EV Nova` executable does: the default.
    #[default]
    Engine,
    /// What the Nova Bible says.
    Bible,
}

impl RuleSource {
    /// Both, the engine's first.
    pub const ALL: [Self; 2] = [Self::Engine, Self::Bible];

    /// The source's name in the settings: `engine` or `bible`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Bible => "bible",
        }
    }

    /// The source named `name` (see [`RuleSource::name`]), if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.name() == name)
    }
}

/// Declares [`RuleKey`]: each variant with its doc comment and its
/// settings key, the one list of the disputed rules.
macro_rules! rule_keys {
    ($($(#[$doc:meta])* $variant:ident => $key:literal,)+) => {
        /// A rule where the Bible and the engine disagree (see the module
        /// docs).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum RuleKey {
            $($(#[$doc])* $variant,)+
        }

        impl RuleKey {
            /// Every disputed rule, in the order declared.
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            /// The rule's key in the settings' `rule_overrides`.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }
        }
    };
}

rule_keys! {
    /// Which governments a crime against a ship improves the player's
    /// record with: by the engine, every government not allied with the
    /// victim's; by the Bible, only its enemies (see
    /// [`legal`](crate::legal)).
    CrimeGains => "crime_gains",
}

impl RuleKey {
    /// The rule whose settings key is `key` (see [`RuleKey::key`]), if
    /// any.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.key() == key)
    }

    /// The rule's place in [`RuleKey::ALL`], which lists the variants in
    /// the order declared.
    const fn index(self) -> usize {
        self as usize
    }
}

/// Which source each disputed rule follows: a default for all, and an
/// override for any one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rulebook {
    default: RuleSource,
    overrides: [Option<RuleSource>; RuleKey::ALL.len()],
}

impl Rulebook {
    /// Every rule following `default`.
    #[must_use]
    pub const fn new(default: RuleSource) -> Self {
        Self {
            default,
            overrides: [None; RuleKey::ALL.len()],
        }
    }

    /// This rulebook, with `rule` following `source` whatever the
    /// default.
    #[must_use]
    pub fn with_override(mut self, rule: RuleKey, source: RuleSource) -> Self {
        self.overrides[rule.index()] = Some(source);
        self
    }

    /// The source every rule without an override follows.
    #[must_use]
    pub fn default_source(&self) -> RuleSource {
        self.default
    }

    /// The source `rule` follows: its override, else the default.
    #[must_use]
    pub fn source_for(&self, rule: RuleKey) -> RuleSource {
        self.overrides[rule.index()].unwrap_or(self.default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_follows_the_engine_by_default() {
        assert_eq!(RuleSource::default(), RuleSource::Engine);
        let rulebook = Rulebook::default();
        assert_eq!(rulebook, Rulebook::new(RuleSource::Engine));
        assert_eq!(rulebook.default_source(), RuleSource::Engine);
        for rule in RuleKey::ALL {
            assert_eq!(rulebook.source_for(rule), RuleSource::Engine, "{rule:?}");
        }
    }

    #[test]
    fn the_default_source_sets_every_rule() {
        let rulebook = Rulebook::new(RuleSource::Bible);
        assert_eq!(rulebook.default_source(), RuleSource::Bible);
        for rule in RuleKey::ALL {
            assert_eq!(rulebook.source_for(rule), RuleSource::Bible, "{rule:?}");
        }
    }

    #[test]
    fn an_override_beats_the_default_both_ways() {
        for (default, other) in [
            (RuleSource::Engine, RuleSource::Bible),
            (RuleSource::Bible, RuleSource::Engine),
        ] {
            let rulebook = Rulebook::new(default).with_override(RuleKey::CrimeGains, other);
            assert_eq!(rulebook.source_for(RuleKey::CrimeGains), other);
            assert_eq!(rulebook.default_source(), default, "unchanged");
            let again = rulebook.with_override(RuleKey::CrimeGains, default);
            assert_eq!(
                again.source_for(RuleKey::CrimeGains),
                default,
                "the last wins"
            );
        }
    }

    #[test]
    fn each_rule_has_a_settings_key_found_by_name() {
        assert_eq!(RuleKey::ALL, [RuleKey::CrimeGains]);
        assert_eq!(RuleKey::CrimeGains.key(), "crime_gains");
        for rule in RuleKey::ALL {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
        }
        assert_eq!(RuleKey::from_key("Crime_Gains"), None);
        assert_eq!(RuleKey::from_key(""), None);
    }

    #[test]
    fn each_source_has_a_name_found_by_name() {
        assert_eq!(RuleSource::ALL, [RuleSource::Engine, RuleSource::Bible]);
        assert_eq!(RuleSource::Engine.name(), "engine");
        assert_eq!(RuleSource::Bible.name(), "bible");
        for source in RuleSource::ALL {
            assert_eq!(RuleSource::from_name(source.name()), Some(source));
        }
        assert_eq!(RuleSource::from_name("Bible"), None);
        assert_eq!(RuleSource::from_name("both"), None);
    }
}
