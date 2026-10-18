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
//!    (`cmpw $0x17` at 0x6cd2e) and is left as it is above that (`jg`
//!    past the store, @0x6cd32). Otherwise it falls by 1 if it is above 0
//!    (at 0x6cd40).
//! 2. With no afterburner engaged, it falls by 1 if it is above 24 (at
//!    0x6fd3f). With the afterburner on, instead, it rises by 1 if it is
//!    at most 31, and by 1 more while the accelerate key is up; then it
//!    falls by 1 if it is above [`GLOW_AFTERBURNER`], 32
//!    (@0x6fc60-0x6fcc0; [`ramp_afterburner_glow`]).
//!
//! So steady thrust holds it at [`GLOW_CRUISE`], 24, and coasting brings
//! it back to 0. The afterburner brings it to 32: coasting, the fall and
//! the two rises make 1 a tick; thrusting, the thrust's rise and the
//! afterburner's make 2 a tick up to cruise, and the afterburner's alone 1
//! a tick from there. Once the afterburner is off, the fall above cruise
//! brings it back down to 24 a tick at a time, under thrust (which holds
//! it above 23) as well as coasting (whose fall doubles it). It starts at 0 (`_ResetPlayer`, 0x1d40d, at 0x1d5bc) and
//! returns to 0 when the player lands (`_HandlePlayerDockRequest`,
//! 0x66691, at 0x67010). [`ramp_glow`] is one such step.
//!
//! **An inertialess ship's base** (see `nova_sim::flight`'s inertialess
//! tick) takes one more step between those two, at 0x6d36e-0x6d42d, in
//! the same frame, after step 1 and before step 2
//! ([`ramp_inertialess_glow`]):
//!
//! - In flight it follows the speed: with a target of
//!   min(24, trunc(32 x speed x 0.75 / `_ShipMaxSpeed`)) (32.0 f32
//!   @0xdd654, 0.75 @0xdd1f8; [`speed_target`]), it rises by 1 below the
//!   target and falls by 1 otherwise, on the target too
//!   ([`follow_speed`]). Step 1 still runs, so coasting (-1) and
//!   following (+1) cancel below the target, and steady thrust at top
//!   speed holds the base at 23, one under the target, after it has
//!   climbed 2 a tick.
//! - Disabled, it falls by 1 more while above 0 (@0x6d397-0x6d3a3), 2 a
//!   tick with step 1's coasting.
//!
//! The base is an `i16` here too: following the speed from 0 at rest
//! takes it to -1, and the next tick back to 0, as the original's does.
//! A level of 1 or less is hidden, so a base of 0 or less draws nothing
//! ([`drawn_base`]).
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
//! bias instead; 8-bit screens' alpha-table blits; the banking glow, shän flag
//! 0x0002 with banking (at 0x6cef6), which no stock ship has; AI
//! ships' own rules (`_HandleShip`, 0x33581, and `_LowLevelAIHandler`,
//! 0x851da); and the hyperspace run-up, during which the glow blazes to
//! 32 (at 0x6b81b and 0x6d320): here a jump puts the glow out as it
//! begins.

use crate::blink::{BlinkChance, FULL};

/// The base level steady thrust holds the glow at.
pub const GLOW_CRUISE: u8 = 24;

/// The base level the afterburner drives the glow to (`cmpw $0x20`
/// @0x6fcab).
pub const GLOW_AFTERBURNER: i16 = 32;

/// The glow base level after one tick from `base`, thrusting or not:
/// thrust raises it by 1 up to [`GLOW_CRUISE`], coasting lowers it by 1
/// down to 0, and a base above [`GLOW_CRUISE`] then falls by 1 more.
#[must_use]
pub fn ramp_glow(base: i16, thrusting: bool) -> i16 {
    settle(thrust_step(base, thrusting))
}

/// The glow base level after one tick from `base` with the afterburner
/// on, thrusting or not (see the module docs): [`ramp_glow`]'s thrust or
/// coast step, then the afterburner's in place of the fall above cruise.
#[must_use]
pub fn ramp_afterburner_glow(base: i16, thrusting: bool) -> i16 {
    after_move(thrust_step(base, thrusting), thrusting, true)
}

/// What an inertialess ship's glow base takes from its speed this tick
/// (see the module docs): `Speed`'s in flight, `Disabled`'s disabled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Follow {
    /// Flying at `speed` of a top speed of `max_speed`
    /// (`_ShipMaxSpeed`).
    Speed {
        /// Its speed along its heading.
        speed: f32,
        /// Its top speed.
        max_speed: f32,
    },
    /// Disabled.
    Disabled,
}

/// An inertialess ship's glow base level after one tick from `base`,
/// thrusting or not, with the `afterburner` on or not (see the module
/// docs): [`ramp_glow`]'s thrust or coast step, then `follow`'s
/// ([`follow_speed`], or 1 more down while above 0 disabled), then
/// [`ramp_glow`]'s step down above cruise, or the afterburner's
/// ([`ramp_afterburner_glow`]).
#[must_use]
pub fn ramp_inertialess_glow(base: i16, thrusting: bool, follow: Follow, afterburner: bool) -> i16 {
    let stepped = thrust_step(base, thrusting);
    let followed = match follow {
        Follow::Speed { speed, max_speed } => follow_speed(stepped, speed, max_speed),
        Follow::Disabled => fade(stepped),
    };
    after_move(followed, thrusting, afterburner)
}

/// The base an inertialess ship's glow follows at `speed` of a top speed
/// of `max_speed`: min(24, trunc(32 x speed x 0.75 / `max_speed`))
/// (@0x6d3af-0x6d408); none when it has no top speed, whose quotient the
/// original's `cvttsd2si` turns to a word of 0.
#[must_use]
pub fn speed_target(speed: f32, max_speed: f32) -> i16 {
    if max_speed <= 0.0 {
        return 0;
    }
    let scaled = f64::from(SPEED_GLOW * speed) * SPEED_GLOW_SHARE / f64::from(max_speed);
    scaled.trunc().clamp(0.0, f64::from(GLOW_CRUISE)) as i16
}

/// `base` stepped toward [`speed_target`] at `speed` of `max_speed`: up
/// by 1 below it, down by 1 at or above it (@0x6d40f-0x6d42d), to -1 at
/// the least.
#[must_use]
pub fn follow_speed(base: i16, speed: f32, max_speed: f32) -> i16 {
    if speed_target(speed, max_speed) > base {
        base + 1
    } else {
        base - 1
    }
}

/// The base the glow is drawn from for a base level of `base`: none below
/// none, which draws the same, as every level of 1 or less is hidden.
#[must_use]
pub fn drawn_base(base: i16) -> u8 {
    u8::try_from(base.max(0)).unwrap_or(u8::MAX)
}

/// The thrust or coast step (at 0x6cd2e and 0x6cd40): thrust raises
/// `base` by 1 while it is at most 23; coasting lowers it by 1 while it is
/// above 0.
fn thrust_step(base: i16, thrusting: bool) -> i16 {
    match (thrusting, base < i16::from(GLOW_CRUISE)) {
        (true, true) => base + 1,
        (true, false) => base,
        (false, _) => fade(base),
    }
}

/// `base` 1 lower while it is above 0.
fn fade(base: i16) -> i16 {
    if base > 0 { base - 1 } else { base }
}

/// The step after the move: the afterburner's ([`burn`]) while it is on,
/// thrusting or not, else the fall above cruise ([`settle`]).
fn after_move(base: i16, thrusting: bool, afterburner: bool) -> i16 {
    if afterburner {
        burn(base, thrusting)
    } else {
        settle(base)
    }
}

/// The afterburner's step (@0x6fc60-0x6fcc0): up 1 while at most 31, and
/// 1 more then when not `thrusting`; then down 1 above
/// [`GLOW_AFTERBURNER`].
fn burn(base: i16, thrusting: bool) -> i16 {
    let raised = if base < GLOW_AFTERBURNER {
        base + if thrusting { 1 } else { 2 }
    } else {
        base
    };
    if raised > GLOW_AFTERBURNER {
        raised - 1
    } else {
        raised
    }
}

/// Above cruise, with no afterburner, one step more down (at 0x6fd3f).
fn settle(base: i16) -> i16 {
    if base > i16::from(GLOW_CRUISE) {
        base - 1
    } else {
        base
    }
}

/// The glow's full scale at top speed (32.0 f32 @0xdd654).
const SPEED_GLOW: f32 = 32.0;

/// The share of [`SPEED_GLOW`] top speed gives (0.75 @0xdd1f8).
const SPEED_GLOW_SHARE: f64 = 0.75;

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
        let cruise = i16::from(GLOW_CRUISE);
        let mut base = 0;
        for k in 1..=30 {
            base = ramp_glow(base, true);
            assert_eq!(base, k.min(cruise), "{k} ticks of thrust");
        }
        for k in 1..=30 {
            base = ramp_glow(base, false);
            assert_eq!(base, (cruise - k).max(0), "{k} ticks coasting");
        }
    }

    #[test]
    fn steady_thrust_at_or_above_cruise_holds_the_base_before_the_fall_above_cruise() {
        // The thrust step leaves a base above 23 alone (@0x6cd32), and the
        // fall above cruise then takes it down 1 a tick to 24.
        assert_eq!(thrust_step(24, true), 24);
        assert_eq!(thrust_step(30, true), 30);
        let mut base = 32;
        for k in 1..=8 {
            base = ramp_glow(base, true);
            assert_eq!(base, 32 - k, "{k} ticks of thrust from 32");
        }
        assert_eq!(ramp_glow(base, true), 24);
    }

    // The afterburner.

    /// `ticks` afterburner ticks from `base`, thrusting or not.
    fn burnt(mut base: i16, thrusting: bool, ticks: usize) -> Vec<i16> {
        (0..ticks)
            .map(|_| {
                base = ramp_afterburner_glow(base, thrusting);
                base
            })
            .collect()
    }

    #[test]
    fn coasting_on_the_afterburner_the_base_climbs_1_a_tick_to_32_and_holds() {
        assert_eq!(
            burnt(24, false, 10),
            [25, 26, 27, 28, 29, 30, 31, 32, 32, 32]
        );
        assert_eq!(burnt(0, false, 3), [2, 3, 4], "nothing to fall from none");
        assert_eq!(burnt(40, false, 2), [38, 36], "coasting from above, 2 down");
        assert_eq!(burnt(40, true, 2), [39, 38], "thrusting from above, 1 down");
    }

    #[test]
    fn thrusting_on_the_afterburner_the_base_climbs_2_a_tick_to_cruise_then_1_to_32() {
        let climb = burnt(0, true, 20);
        assert_eq!(climb[..12], [2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24]);
        assert_eq!(climb[12..], [25, 26, 27, 28, 29, 30, 31, 32]);
        assert_eq!(burnt(23, true, 3), [25, 26, 27], "the thrust's last rise");
        assert_eq!(burnt(32, true, 2), [32, 32]);
    }

    #[test]
    fn released_at_32_the_base_falls_1_a_tick_to_cruise_under_thrust_and_2_coasting() {
        let mut thrusting = 32;
        let mut coasting = 32;
        for _ in 0..4 {
            thrusting = ramp_glow(thrusting, true);
            coasting = ramp_glow(coasting, false);
        }
        assert_eq!((thrusting, coasting), (28, 24));
        for _ in 0..4 {
            thrusting = ramp_glow(thrusting, true);
        }
        assert_eq!(thrusting, 24);
    }

    #[test]
    fn an_inertialess_afterburner_steps_after_the_follow() {
        let at_top = Follow::Speed {
            speed: 6.0,
            max_speed: 6.0,
        };
        // Coast -1, follow +1 (24 above), afterburner +2.
        assert_eq!(ramp_inertialess_glow(10, false, at_top, true), 12);
        // Thrust +1, follow -1 (on 24), afterburner +1.
        assert_eq!(ramp_inertialess_glow(23, true, at_top, true), 24);
        // Coast -1, follow -1 (24 below), afterburner +2: held at 31.
        assert_eq!(ramp_inertialess_glow(31, false, at_top, true), 31);
        assert_eq!(
            ramp_inertialess_glow(30, false, at_top, false),
            27,
            "off: coast, follow and the fall above cruise"
        );
    }

    // An inertialess ship's base.

    #[test]
    fn the_speed_target_is_three_quarters_of_32_at_top_speed_capped_at_24() {
        for (speed, max, toward) in [
            (6.0, 6.0, 24),
            (3.0, 6.0, 12),
            (0.0, 6.0, 0),
            (10.8, 6.0, 24),
            (1.0, 6.0, 4),
            (0.24, 6.0, 0),
            (0.26, 6.0, 1),
            (3.0, 0.0, 0),
            (0.0, 0.0, 0),
        ] {
            assert_eq!(speed_target(speed, max), toward, "{speed} of {max}");
        }
    }

    #[test]
    fn following_the_speed_steps_up_below_the_target_and_down_at_or_above_it() {
        assert_eq!(follow_speed(10, 3.0, 6.0), 11, "12 above 10");
        assert_eq!(follow_speed(12, 3.0, 6.0), 11, "on the target, down");
        assert_eq!(follow_speed(13, 3.0, 6.0), 12);
        assert_eq!(follow_speed(0, 0.0, 6.0), -1, "an i16 dips below none");
        assert_eq!(follow_speed(-1, 0.0, 6.0), 0);
    }

    /// `ticks` inertialess ticks of `base`, thrusting or not, at `speed`
    /// of top speed 6.
    fn inertialess_after(mut base: i16, thrusting: bool, speed: f32, ticks: usize) -> i16 {
        for _ in 0..ticks {
            base = ramp_inertialess_glow(
                base,
                thrusting,
                Follow::Speed {
                    speed,
                    max_speed: 6.0,
                },
                false,
            );
        }
        base
    }

    #[test]
    fn coasting_at_top_speed_the_coast_step_and_the_follow_cancel_and_hold_the_base() {
        // Coast: -1; follow (24 above): +1.
        for base in [1, 12, 20, 24] {
            assert_eq!(inertialess_after(base, false, 6.0, 1), base, "{base}");
            assert_eq!(inertialess_after(base, false, 6.0, 30), base, "{base}");
        }
        // From none the coast step does nothing and the follow lifts it.
        assert_eq!(inertialess_after(0, false, 6.0, 1), 1);
        assert_eq!(inertialess_after(0, false, 6.0, 30), 1);
    }

    #[test]
    fn thrusting_at_top_speed_it_climbs_two_a_tick_and_settles_at_23() {
        // Thrust: +1; follow (24 above): +1, until the base reaches 24,
        // where the thrust holds it and the follow takes it back to 23.
        assert_eq!(inertialess_after(0, true, 6.0, 1), 2);
        assert_eq!(inertialess_after(0, true, 6.0, 11), 22);
        assert_eq!(inertialess_after(0, true, 6.0, 12), 24);
        assert_eq!(inertialess_after(0, true, 6.0, 13), 23);
        assert_eq!(inertialess_after(0, true, 6.0, 40), 23);
    }

    #[test]
    fn coasting_at_half_speed_from_cruise_it_falls_two_a_tick_to_the_target_of_12() {
        // Coast: -1; follow (12 at or below): -1, until the base is 12.
        assert_eq!(inertialess_after(24, false, 3.0, 1), 22);
        assert_eq!(inertialess_after(24, false, 3.0, 6), 12);
        assert_eq!(inertialess_after(24, false, 3.0, 30), 12);
        // An odd base passes the target and holds below it.
        assert_eq!(inertialess_after(13, false, 3.0, 1), 11);
        assert_eq!(inertialess_after(13, false, 3.0, 30), 11);
    }

    #[test]
    fn coasting_at_half_speed_from_none_it_rises_only_to_1() {
        // Coast: nothing at none, -1 at 1; follow (12 above): +1.
        assert_eq!(inertialess_after(0, false, 3.0, 1), 1);
        assert_eq!(inertialess_after(0, false, 3.0, 30), 1);
    }

    #[test]
    fn thrusting_at_half_speed_it_climbs_to_the_target_and_holds_it() {
        // Thrust: +1; follow: +1 below 12, -1 from it.
        assert_eq!(inertialess_after(0, true, 3.0, 5), 10);
        assert_eq!(inertialess_after(0, true, 3.0, 6), 12);
        assert_eq!(inertialess_after(0, true, 3.0, 30), 12);
    }

    #[test]
    fn at_rest_an_inertialess_glow_dips_to_minus_1_and_back_which_draws_nothing() {
        assert_eq!(inertialess_after(0, false, 0.0, 1), -1);
        assert_eq!(inertialess_after(0, false, 0.0, 2), 0);
        assert_eq!(
            inertialess_after(0, true, 0.0, 1),
            0,
            "thrust lifts it first"
        );
        assert_eq!(inertialess_after(-1, true, 0.0, 1), -1);
        for base in [-1, 0] {
            assert_eq!(rolled(drawn_base(base), 5), None, "{base}");
        }
    }

    #[test]
    fn disabled_the_coast_step_and_the_disabled_step_take_two_a_tick_off_down_to_none() {
        assert_eq!(ramp_inertialess_glow(10, false, Follow::Disabled, false), 8);
        assert_eq!(ramp_inertialess_glow(1, false, Follow::Disabled, false), 0);
        assert_eq!(ramp_inertialess_glow(0, false, Follow::Disabled, false), 0);
        assert_eq!(
            ramp_inertialess_glow(-1, false, Follow::Disabled, false),
            -1
        );
    }

    #[test]
    fn the_drawn_base_is_the_base_with_none_below_none() {
        assert_eq!(drawn_base(-1), 0);
        assert_eq!(drawn_base(0), 0);
        assert_eq!(drawn_base(24), 24);
        assert_eq!(drawn_base(400), u8::MAX);
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
