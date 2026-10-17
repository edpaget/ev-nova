//! A stellar's destruction and regeneration, as the original's
//! `_EvalSetExp` (`Y` @0x15a38-0x15ae6), its shots
//! (`_CheckShotProximities` @0x371bc-0x37775) and its day's stellars
//! (`_IncrementGameTime` @0xb5c4-0xb71e) count them: pure rules over a
//! stellar's [`StellarLife`], its strength and its [`Countdown`]. See the
//! module docs' "Checked for phase 5" and "Checked for phase 8".

use std::ops::RangeInclusive;

use super::Countdown;
use crate::catalog::{StellarId, StellarLife};
use crate::combat::weapon::Explosion;
use crate::rulebook::RuleSource;

/// The `spöb` `Flags2` bit of a stellar that starts the game destroyed
/// (`_DoNewPilot` @0x192d6).
const STARTS_DESTROYED: u16 = 0x0040;

/// Whether the stellar `life` starts the game destroyed: its `Flags2` has
/// 0x0040 (`_DoNewPilot` @0x192d6), whatever its other bits.
#[must_use]
pub fn starts_destroyed(life: &StellarLife) -> bool {
    life.flags2 & STARTS_DESTROYED != 0
}

/// The stellars `Y` and `U` act on: the original's 2048 slots
/// (@0x15a3f-0x15a46, @0x15af2-0x15af9). Any other ID does nothing.
const STELLARS: RangeInclusive<i16> = 128..=2175;

/// Whether `Y` and `U` act on `stellar`.
#[must_use]
pub fn in_range(stellar: StellarId) -> bool {
    STELLARS.contains(&stellar.0)
}

/// The countdown a stellar is destroyed with: its `DeadTime`, or 1 when
/// that is negative (@0x15ac8-0x15ad5, @0x192f4-0x19301). A shot writes
/// the raw `DeadTime` (@0x374c8-0x374cf), but a negative countdown with
/// the current strength -1 reads destroyed just the same, and the next
/// day sets it to 1 (@0xb6f5), so the shot's destruction uses this too.
#[must_use]
pub fn countdown_on_destroy(life: &StellarLife) -> Countdown {
    Countdown {
        days_left: u16::try_from(life.dead_time).unwrap_or(1),
    }
}

/// Whether destroying the stellar `life` makes it destroyed, as
/// [`RuleKey::DestroyInvincible`](crate::RuleKey::DestroyInvincible)
/// reads: by the engine, only when its `Strength` is above 0, since
/// `_StellarIsDestroyed` holds no invincible stellar destroyed (@0x4e1a);
/// by the Bible's "destroy stellar xxx", always.
#[must_use]
pub fn destroys(life: &StellarLife, reading: RuleSource) -> bool {
    match reading {
        RuleSource::Engine => life.strength > 0,
        RuleSource::Bible => true,
    }
}

/// Whether shots can hit the stellar `life`: only one of `Strength` above
/// 0, as `_CheckShotProximities` skips any other (@0x3726d), and as
/// `_StellarIsDestroyed` never holds an invincible one destroyed
/// (@0x4e1a).
#[must_use]
pub fn destroyable(life: &StellarLife) -> bool {
    life.strength > 0
}

/// The strength of the stellar `life` while it is whole and undamaged:
/// its `Strength`, which the day restores (@0xb700) and a load gives
/// back (@0x76068-0x760b8).
#[must_use]
pub fn full_strength(life: &StellarLife) -> i64 {
    i64::from(life.strength)
}

/// The explosion `Y` makes at the stellar `life`: its `ExplodType`, when
/// it is `shown` (among the stellars of the player's system, the ship in
/// flight: one made while landed would be cleared on the take-off) and
/// was not destroyed before the `Y` (@0x15a59-0x15ab4). Its `Strength` does not matter, so an
/// invincible stellar, which the engine's
/// [`RuleKey::DestroyInvincible`](crate::RuleKey::DestroyInvincible)
/// leaves whole, explodes all the same.
#[must_use]
pub fn explodes_on_y(life: &StellarLife, shown: bool, destroyed_before: bool) -> Option<Explosion> {
    life.explosion.filter(|_| shown && !destroyed_before)
}

/// What a day does to a destroyed stellar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick {
    /// It stays destroyed, with this countdown.
    Stays(Countdown),
    /// It regenerates, and runs its `OnRegen`.
    Regenerates,
}

/// What a day does to the destroyed stellar `life` with `countdown` left:
/// one with a negative `DeadTime` stays destroyed with 1 day left
/// (@0xb6f5), so it never regenerates on its own; any other counts a day
/// down and regenerates at 0 or below (@0xb68a-0xb69d).
#[must_use]
pub fn day(life: &StellarLife, countdown: Countdown) -> Tick {
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
    use crate::combat::weapon::Explosion;
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
    fn only_a_stellar_of_some_strength_can_be_shot() {
        for (strength, shootable) in [(0, false), (-1, false), (1, true), (70_000, true)] {
            assert_eq!(destroyable(&life(strength, 0)), shootable, "{strength}");
        }
        assert_eq!(full_strength(&life(70_000, 0)), 70_000);
    }

    #[test]
    fn y_explodes_a_stellar_shown_and_whole_that_has_an_explosion() {
        let boom = Explosion::decode(3);
        let exploding = |strength| StellarLife {
            explosion: boom,
            ..life(strength, 0)
        };
        assert_eq!(explodes_on_y(&exploding(100), true, false), boom);
        assert_eq!(
            explodes_on_y(&exploding(0), true, false),
            boom,
            "an invincible stellar too"
        );
        assert_eq!(
            explodes_on_y(&exploding(100), false, false),
            None,
            "not shown"
        );
        assert_eq!(
            explodes_on_y(&exploding(100), true, true),
            None,
            "destroyed already"
        );
        assert_eq!(explodes_on_y(&life(100, 0), true, false), None, "none");
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
