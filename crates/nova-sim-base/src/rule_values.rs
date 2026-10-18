//! Numeric rule values: rules whose setting is a number, not a choice
//! between the engine and the Bible.
//!
//! A [`RuleValue`] names a number the original works out its own way but
//! a player may want another: its default is the original's, and the
//! settings file sets any other, as a whole number from 1 to `u32::MAX`
//! (see `nova::rulebook`). The [`Rulebook`] holds an override for each
//! ([`Rulebook::with_value`]), and a rule reads its number with
//! [`Rulebook::value_for`]. A value is not an engine/Bible pair, so the
//! rulebook's global source (`"rules"`) never moves one.
//!
//! | [`RuleValue`] | settings key | default |
//! |---|---|---|
//! | [`ThrustDivisor`](RuleValue::ThrustDivisor) | `thrust_divisor` | 5000 |

use core::num::NonZeroU32;

#[cfg(doc)]
use crate::rulebook::Rulebook;

/// `value` as a [`NonZeroU32`], for the defaults below: a 0 fails to
/// compile.
const fn nonzero(value: u32) -> NonZeroU32 {
    match NonZeroU32::new(value) {
        Some(value) => value,
        None => panic!("a rule value is never 0"),
    }
}

/// Declares [`RuleValue`]: each variant with its doc comment, its
/// settings key and its default, the one list of the numeric rules.
macro_rules! rule_values {
    ($($(#[$doc:meta])* $variant:ident => $key:literal = $default:literal,)+) => {
        /// A rule whose setting is a number (see the module docs).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum RuleValue {
            $($(#[$doc])* $variant,)+
        }

        impl RuleValue {
            /// Every numeric rule, in the order declared.
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            /// The rule's key in the settings' `rule_values`.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }

            /// The number the rule takes when the settings set none: the
            /// original's.
            #[must_use]
            pub const fn default_value(self) -> NonZeroU32 {
                match self {
                    $(Self::$variant => nonzero($default),)+
                }
            }
        }
    };
}

rule_values! {
    /// What a ship's `Accel` total is divided by to give its thrust, in
    /// pixels a tick added each tick (see [`handling`](crate::handling)). The
    /// original's is 5000: `_ShipAccelRate` (@0x353b) gives `Accel` /
    /// 10000 (`_LoadObjectData` @0x7a0b3-0x7a0c7) doubled (@0x35f8,
    /// @0x3667), a frame of `_gSpeedMult` 1.0 being the engine's tick.
    /// The port's earlier rate, 5/3 as hard, is 3000.
    ThrustDivisor => "thrust_divisor" = 5000,
}

impl RuleValue {
    /// The rule whose settings key is `key` (see [`RuleValue::key`]), if
    /// any.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|value| value.key() == key)
    }

    /// The rule's place in [`RuleValue::ALL`].
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rulebook::{RuleSource, Rulebook};

    fn n(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    #[test]
    fn the_thrust_divisor_is_found_by_its_key() {
        assert_eq!(RuleValue::ThrustDivisor.key(), "thrust_divisor");
        assert_eq!(
            RuleValue::from_key("thrust_divisor"),
            Some(RuleValue::ThrustDivisor)
        );
        assert_eq!(RuleValue::from_key("thrust"), None);
        for value in RuleValue::ALL {
            assert_eq!(RuleValue::from_key(value.key()), Some(value), "{value:?}");
        }
    }

    #[test]
    fn the_thrust_divisor_defaults_to_the_originals_5000() {
        assert_eq!(RuleValue::ThrustDivisor.default_value(), n(5000));
        assert_eq!(
            Rulebook::default().value_for(RuleValue::ThrustDivisor),
            n(5000)
        );
    }

    #[test]
    fn an_override_wins_and_the_last_one_counts() {
        let rulebook = Rulebook::default().with_value(RuleValue::ThrustDivisor, n(3000));
        assert_eq!(rulebook.value_for(RuleValue::ThrustDivisor), n(3000));
        let again = rulebook.with_value(RuleValue::ThrustDivisor, n(7919));
        assert_eq!(again.value_for(RuleValue::ThrustDivisor), n(7919));
        assert_ne!(rulebook, Rulebook::default());
    }

    #[test]
    fn the_global_source_leaves_every_value_at_its_default() {
        for source in RuleSource::ALL {
            let rulebook = Rulebook::new(source);
            for value in RuleValue::ALL {
                assert_eq!(
                    rulebook.value_for(value),
                    value.default_value(),
                    "{source:?}: {value:?}"
                );
            }
        }
    }

    #[test]
    fn an_override_on_one_value_leaves_the_others_and_the_sources_alone() {
        for value in RuleValue::ALL {
            let set = n(value.default_value().get().wrapping_add(1).max(1));
            let rulebook = Rulebook::default().with_value(value, set);
            for other in RuleValue::ALL {
                let expected = if other == value {
                    set
                } else {
                    other.default_value()
                };
                assert_eq!(rulebook.value_for(other), expected, "{value:?}: {other:?}");
            }
            assert_eq!(rulebook.global_source(), None);
        }
    }
}
