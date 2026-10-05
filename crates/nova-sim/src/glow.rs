//! The engine glow: how bright a ship's `shän` glow layer is drawn, from
//! a base level that thrust ramps up and coasting ramps down, and a
//! flicker rolled afresh each tick.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`, each ship keeps a glow base
//! level, an `i16` at ship-object offset `+0xc8d4`. Each frame
//! `_DoPlayGameWork` (0x43de5) first updates it, in `_HandlePlayer`
//! (0x68390) for the player, then draws it, in `_HandleShipDisplay`
//! (0x2b514).
//!
//! **The base.** In ordinary flight `_HandlePlayer` steps it once a frame,
//! in whole steps that are not scaled by `_gSpeedMult`:
//!
//! 1. While the accelerate key is held (and the ship is neither disabled
//!    nor in a hyperspace sequence), it rises by 1 if it is at most 23
//!    (`cmpw $0x17` at 0x6cd2e); otherwise it falls by 1 if it is above 0
//!    (at 0x6cd40).
//! 2. With no afterburner engaged, it falls by 1 if it is above 24 (at
//!    0x6fd3f).
//!
//! So steady thrust holds it at [`GLOW_CRUISE`], 24, and coasting brings
//! it back to 0. It starts at 0 (`_ResetPlayer`, 0x1d40d, at 0x1d5bc) and
//! returns to 0 when the player lands (`_HandlePlayerDockRequest`,
//! 0x66691, at 0x67010). [`ramp_glow`] is one such step.
//!
//! **The drawn level.** `_HandleShipDisplay` draws the glow of any ship
//! whose class has a glow sheet (the gate at 0x2c195), at
//! `base + _Rand(6) - 4` in 16-bit arithmetic (at 0x2c1b5-0x2c1f3).
//! `_Rand` (0xa4c76) is uniform over 0 to n - 1, so the flicker is -4 to
//! +1, rolled anew every frame. A level of 1 or less hides the glow (at
//! 0x2c1cc), and one above 32 is drawn at 32 (at 0x2c1f3). The glow is
//! blitted with the same draw proc and field writes as the running lights
//! (at 0x2c2b6-0x2c2d6), so it is added scaled by level/32 like them.
//! [`glow_level`] is that level, rolling on the [`BlinkChance`] port.
//!
//! # Per tick, not per frame
//!
//! The original steps the base and rolls the flicker once a frame, and
//! its frame rate is not established: it has no frame limiter. Here both
//! happen once a 1/30 s tick (the simulation's [`TICK`](crate::TICK), the
//! frame `_gSpeedMult` normalises to), so steady thrust brightens the glow
//! from 0 to 24 in 24 ticks (0.8 s) and releasing it fades the glow out in
//! as many, at any display rate. The rolls keep the original's
//! distribution but not its generator.
//!
//! # Not modelled
//!
//! The cloak's path (at 0x2c1f6), which caps the level by the hull's
//! bias instead; 8-bit screens' alpha-table blits; the afterburner, which
//! drives the base to 32 (at 0x6fc63); the banking glow, shän flag
//! 0x0002 with banking (at 0x6cef6), which no stock ship has;
//! inertialess ships, whose base follows their speed (at 0x6d36e); AI
//! ships' own rules (`_HandleShip`, 0x33581, and `_LowLevelAIHandler`,
//! 0x851da); and the hyperspace run-up, during which the glow blazes to
//! 32 (at 0x6b81b and 0x6d320): here a jump puts the glow out as it
//! begins.

use crate::blink::{BlinkChance, FULL};

/// The base level steady thrust holds the glow at.
pub const GLOW_CRUISE: u8 = 24;

/// The glow base level after one tick from `base`, thrusting or not:
/// thrust raises it by 1 up to [`GLOW_CRUISE`], coasting lowers it by 1
/// down to 0, and a base above [`GLOW_CRUISE`] then falls by 1 more.
#[must_use]
pub fn ramp_glow(base: u8, thrusting: bool) -> u8 {
    // Thrust or coast (at 0x6cd2e and 0x6cd40).
    let stepped = match (thrusting, base < GLOW_CRUISE) {
        (true, true) => base + 1,
        (true, false) => base,
        (false, _) => base.saturating_sub(1),
    };
    // Above cruise, with no afterburner, one step more down (at 0x6fd3f).
    if stepped > GLOW_CRUISE {
        stepped - 1
    } else {
        stepped
    }
}

/// The glow's level at `tick` for a base level of `base`:
/// `Some(2..=32)` out of [`FULL`] when it is drawn, `None` when the
/// flicker hides it. Asks `chance` for one roll of 6, keyed by `tick`.
#[must_use]
pub fn glow_level(base: u8, tick: u64, chance: &(impl BlinkChance + ?Sized)) -> Option<u8> {
    let roll = chance.roll(tick, FLICKER_SIDES).min(FLICKER_SIDES - 1);
    let level = i32::from(base) + i32::from(roll) - FLICKER_DROP;
    (level > 1).then(|| level.min(i32::from(FULL)) as u8)
}

/// How many outcomes the flicker rolls among: `_Rand(6)`.
const FLICKER_SIDES: u16 = 6;

/// How far the lowest roll draws the glow below its base.
const FLICKER_DROP: i32 = 4;

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::blink::HashedRolls;

    /// Answers every roll from a script, by call (the last answer repeats
    /// past its end), and records every `(change, sides)` it is asked.
    #[derive(Default)]
    struct ScriptedRolls {
        answers: Vec<u16>,
        asked: RefCell<Vec<(u64, u16)>>,
    }

    impl ScriptedRolls {
        fn new(answers: &[u16]) -> Self {
            Self {
                answers: answers.to_vec(),
                asked: RefCell::default(),
            }
        }

        fn asked(&self) -> Vec<(u64, u16)> {
            self.asked.borrow().clone()
        }
    }

    impl BlinkChance for ScriptedRolls {
        fn roll(&self, change: u64, sides: u16) -> u16 {
            let mut asked = self.asked.borrow_mut();
            let at = asked.len();
            asked.push((change, sides));
            self.answers
                .get(at)
                .or(self.answers.last())
                .copied()
                .unwrap_or(0)
        }
    }

    /// The level for `base` with a roll of `roll`.
    fn rolled(base: u8, roll: u16) -> Option<u8> {
        glow_level(base, 0, &ScriptedRolls::new(&[roll]))
    }

    // The base.

    #[test]
    fn thrust_raises_the_base_to_cruise_and_coasting_lowers_it_to_zero() {
        for (base, thrusting, after) in [
            (0, true, 1),
            (23, true, 24),
            (24, true, 24),
            (25, true, 24),
            (40, true, 39),
            (24, false, 23),
            (1, false, 0),
            (0, false, 0),
            (25, false, 24),
            (30, false, 28),
            (255, true, 254),
            (255, false, 253),
        ] {
            assert_eq!(
                ramp_glow(base, thrusting),
                after,
                "{base}, thrusting {thrusting}"
            );
        }
    }

    #[test]
    fn the_base_ramps_a_step_a_tick() {
        let mut base = 0;
        for k in 1..=30 {
            base = ramp_glow(base, true);
            assert_eq!(base, k.min(GLOW_CRUISE), "{k} ticks of thrust");
        }
        for k in 1..=30 {
            base = ramp_glow(base, false);
            assert_eq!(base, GLOW_CRUISE.saturating_sub(k), "{k} ticks coasting");
        }
    }

    // The drawn level.

    #[test]
    fn at_cruise_the_glow_flickers_from_20_to_25() {
        for roll in 0..=5 {
            assert_eq!(rolled(24, roll), Some(20 + roll as u8), "roll {roll}");
        }
        let chance = ScriptedRolls::new(&[3]);
        assert_eq!(glow_level(24, 7, &chance), Some(23));
        assert_eq!(chance.asked(), [(7, 6)], "one roll of 6, keyed by the tick");
    }

    #[test]
    fn a_level_of_one_or_less_hides_the_glow() {
        assert_eq!(rolled(5, 0), None);
        assert_eq!(rolled(5, 1), Some(2));
        assert_eq!(rolled(0, 5), None);
        assert_eq!(rolled(1, 5), Some(2));
        assert_eq!(rolled(1, 4), None);
    }

    #[test]
    fn the_level_is_at_most_full() {
        assert_eq!(rolled(31, 5), Some(FULL));
        assert_eq!(rolled(32, 5), Some(FULL));
        assert_eq!(rolled(255, 0), Some(FULL));
    }

    #[test]
    fn a_roll_past_the_sides_counts_as_the_highest() {
        assert_eq!(rolled(24, 9), Some(25));
        assert_eq!(rolled(24, u16::MAX), Some(25));
    }

    #[test]
    fn hashed_rolls_flicker_the_glow_within_its_range() {
        let rolls = HashedRolls::new(1);
        let levels: Vec<_> = (0..1000).map(|tick| glow_level(24, tick, &rolls)).collect();
        assert!(
            levels
                .iter()
                .all(|level| level.is_some_and(|n| (20..=25).contains(&n))),
            "{levels:?}"
        );
        let mut distinct = levels.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(distinct.len() >= 2, "it flickers: {distinct:?}");
        assert!((0..1000).all(|tick| glow_level(6, tick, &rolls).is_some()));
        assert!((0..1000).all(|tick| glow_level(0, tick, &rolls).is_none()));
        assert_eq!(glow_level(24, 42, &rolls), glow_level(24, 42, &rolls));
        let _ = glow_level(24, u64::MAX, &rolls);
    }
}
