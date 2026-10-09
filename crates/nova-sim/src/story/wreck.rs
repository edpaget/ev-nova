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
const STARTS_DESTROYED: u16 = 0x0040;

/// Whether the stellar `life` starts the game destroyed: its `Flags2` has
/// 0x0040 (`_DoNewPilot` @0x192d6), whatever its other bits.
pub(crate) fn starts_destroyed(life: &StellarLife) -> bool {
    life.flags2 & STARTS_DESTROYED != 0
}

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

/// What a day does to a destroyed stellar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tick {
    /// It stays destroyed, with this countdown.
    Stays(Countdown),
    /// It regenerates, and runs its `OnRegen`.
    Regenerates,
}

/// What a day does to the destroyed stellar `life` with `countdown` left:
/// one with a negative `DeadTime` stays destroyed with 1 day left
/// (@0xb6f5), so it never regenerates on its own; any other counts a day
/// down and regenerates at 0 or below (@0xb68a-0xb69d).
pub(crate) fn day(life: &StellarLife, countdown: Countdown) -> Tick {
    if life.dead_time < 0 {
        return Tick::Stays(Countdown { days_left: 1 });
    }
    match countdown.days_left {
        0 | 1 => Tick::Regenerates,
        days => Tick::Stays(Countdown {
            days_left: days - 1,
        }),
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
            flags2: 0,
            explosion: None,
            on_destroy: Script::default(),
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
    fn a_day_counts_down_and_regenerates_at_one_day_left_or_none() {
        let life = life(100, 2);
        assert_eq!(day(&life, left(3)), Tick::Stays(left(2)));
        assert_eq!(day(&life, left(2)), Tick::Stays(left(1)));
        assert_eq!(day(&life, left(1)), Tick::Regenerates);
        assert_eq!(day(&life, left(0)), Tick::Regenerates);
        assert_eq!(day(&life, left(u16::MAX)), Tick::Stays(left(u16::MAX - 1)));
    }

    #[test]
    fn a_stellar_of_negative_dead_time_never_regenerates_on_its_own() {
        for days in [0, 1, 5] {
            assert_eq!(day(&life(100, -1), left(days)), Tick::Stays(left(1)));
        }
    }

    #[test]
    fn a_stellar_starts_the_game_destroyed_when_its_flags2_has_0x0040() {
        for (flags2, starts) in [
            (0x0040, true),
            (0xffff, true),
            (0x0060, true),
            (0, false),
            (0x0020, false),
            (!0x0040, false),
        ] {
            let life = StellarLife {
                flags2,
                ..life(100, 0)
            };
            assert_eq!(starts_destroyed(&life), starts, "{flags2:#x}");
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
