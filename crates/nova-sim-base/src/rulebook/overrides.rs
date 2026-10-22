//! The rulebook's per-rule overrides, two bits a rule: whether one is
//! set, and if so whether it is the Bible's. A `[Option<RuleSource>; N]`
//! takes a byte a rule, which pushed [`Rulebook`](super::Rulebook) past
//! the 256 bytes it can be passed by value in.

use super::{RuleKey, RuleSource};

/// The words each bit set takes: one bit for every rule.
const WORDS: usize = RuleKey::ALL.len().div_ceil(u64::BITS as usize);

/// An override for any rule, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Overrides {
    /// The rules that have an override.
    set: [u64; WORDS],
    /// Of those, the rules whose override is the Bible's; clear for every
    /// rule without one, so two equal sets of overrides compare equal.
    bible: [u64; WORDS],
}

impl Overrides {
    /// No rule overridden.
    pub(super) const NONE: Self = Self {
        set: [0; WORDS],
        bible: [0; WORDS],
    };

    /// These overrides with `rule` following `source`.
    pub(super) fn with(mut self, rule: RuleKey, source: RuleSource) -> Self {
        let (word, bit) = place(rule);
        self.set[word] |= bit;
        match source {
            RuleSource::Bible => self.bible[word] |= bit,
            RuleSource::Engine => self.bible[word] &= !bit,
        }
        self
    }

    /// The source `rule` is overridden to, if it is.
    pub(super) fn of(&self, rule: RuleKey) -> Option<RuleSource> {
        let (word, bit) = place(rule);
        (self.set[word] & bit != 0).then(|| {
            if self.bible[word] & bit == 0 {
                RuleSource::Engine
            } else {
                RuleSource::Bible
            }
        })
    }
}

/// The word and the bit within it that hold `rule`.
fn place(rule: RuleKey) -> (usize, u64) {
    let index = rule.index();
    let bits = u64::BITS as usize;
    (index / bits, 1 << (index % bits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_rule_is_overridden_until_one_is() {
        for rule in RuleKey::ALL {
            assert_eq!(Overrides::NONE.of(rule), None, "{rule:?}");
        }
    }

    #[test]
    fn each_rule_keeps_its_own_override_across_the_words() {
        let mut overrides = Overrides::NONE;
        for (n, rule) in RuleKey::ALL.into_iter().enumerate() {
            let source = if n % 3 == 0 {
                RuleSource::Bible
            } else {
                RuleSource::Engine
            };
            if n % 5 != 4 {
                overrides = overrides.with(rule, source);
            }
        }
        for (n, rule) in RuleKey::ALL.into_iter().enumerate() {
            let expected = (n % 5 != 4).then_some(if n % 3 == 0 {
                RuleSource::Bible
            } else {
                RuleSource::Engine
            });
            assert_eq!(overrides.of(rule), expected, "{rule:?}");
        }
    }

    #[test]
    fn an_override_reset_to_the_engine_clears_its_bible_bit() {
        let rule = RuleKey::CloakOnReload;
        let reset = Overrides::NONE
            .with(rule, RuleSource::Bible)
            .with(rule, RuleSource::Engine);
        assert_eq!(reset.of(rule), Some(RuleSource::Engine));
        assert_eq!(reset, Overrides::NONE.with(rule, RuleSource::Engine));
    }
}
