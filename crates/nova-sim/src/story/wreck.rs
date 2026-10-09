//! A stellar's destruction and regeneration, as the original's
//! `_EvalSetExp` (`Y` @0x15a38-0x15ae6) and its day's stellars
//! (`_IncrementGameTime` @0xb5c4-0xb71e) count them: pure rules over a
//! stellar's [`StellarLife`] and its [`Countdown`]. See the module docs'
//! "Checked for phase 5".

use std::ops::RangeInclusive;

use super::Countdown;
use crate::catalog::{StellarId, StellarLife};
use crate::rulebook::RuleSource;

/// The `spöb` `Flags2` bit of a stellar that starts the game destroyed
/// (`_DoNewPilot` @0x192d6).
pub(crate) const STARTS_DESTROYED: u16 = 0x0040;

/// The stellars `Y` and `U` act on: the original's 2048 slots
/// (@0x15a3f-0x15a46, @0x15af2-0x15af9). Any other ID does nothing.
const STELLARS: RangeInclusive<i16> = 128..=2175;

/// Whether `Y` and `U` act on `stellar`.
pub(crate) fn in_range(stellar: StellarId) -> bool {
    STELLARS.contains(&stellar.0)
}

/// The countdown a stellar is destroyed with: its `DeadTime`, or 1 when
/// that is negative (@0x15ac8-0x15ad5, @0x192f4-0x19301).
pub(crate) fn countdown_on_destroy(life: &StellarLife) -> Countdown {
    Countdown {
        days_left: u16::try_from(life.dead_time).unwrap_or(1),
    }
}

/// Whether destroying the stellar `life` makes it destroyed, as
/// [`RuleKey::DestroyInvincible`](crate::RuleKey::DestroyInvincible)
/// reads: by the engine, only when its `Strength` is above 0, since
/// `_StellarIsDestroyed` holds no invincible stellar destroyed (@0x4e1a);
/// by the Bible's "destroy stellar xxx", always.
pub(crate) fn destroys(life: &StellarLife, reading: RuleSource) -> bool {
    match reading {
        RuleSource::Engine => life.strength > 0,
        RuleSource::Bible => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Script;

    /// Stellar 128 of `strength`, with `dead_time`.
    fn life(strength: i32, dead_time: i16) -> StellarLife {
        StellarLife {
            id: StellarId(128),
            strength,
            dead_time,
            starts_destroyed: false,
            on_regen: Script::default(),
        }
    }

    fn left(days_left: u16) -> Countdown {
        Countdown { days_left }
    }

    #[test]
    fn a_stellar_is_destroyed_for_its_dead_time_or_a_day_when_that_is_negative() {
        for (dead_time, days) in [(5, 5), (1, 1), (0, 0), (-1, 1), (i16::MAX, 32_767)] {
            assert_eq!(
                countdown_on_destroy(&life(100, dead_time)),
                left(days),
                "{dead_time}"
            );
        }
    }

    #[test]
    fn by_the_engine_only_a_stellar_of_some_strength_is_destroyed() {
        for (strength, engine) in [(0, false), (-1, false), (1, true), (100, true)] {
            let life = life(strength, 0);
            assert_eq!(destroys(&life, RuleSource::Engine), engine, "{strength}");
            assert!(destroys(&life, RuleSource::Bible), "{strength}");
        }
    }

    #[test]
    fn y_and_u_act_on_the_originals_slots_alone() {
        for (id, acts) in [
            (127, false),
            (128, true),
            (2175, true),
            (2176, false),
            (-1, false),
        ] {
            assert_eq!(in_range(StellarId(id)), acts, "{id}");
        }
    }
}
