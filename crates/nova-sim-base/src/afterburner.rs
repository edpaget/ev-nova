//! The player's afterburner: what `_HandlePlayer` (`EV Nova.app`, i386)
//! does while the afterburner key is held, and the box the player's
//! velocity is clamped to, `_playerMaxVel`, which the afterburner widens
//! and which bleeds back after it.
//!
//! **Engaging** (@0x6cad4-0x6cb59, [`engages`]). `_abOn` (*0xdee28 →
//! 0x316944) is cleared every frame (@0x6cae3) and set again when the key
//! (keyPrefs +0x30) is down, the ship is not disabled (-0x2cb),
//! `_HasAfterburner` holds (the player carries an outfit with a `ModType`
//! 15 slot, @0x3a6d-0x3afe), its jump is not under way (+0x50 ≤ 0), it has
//! fuel (+0x38 > 0), its +0x4c is at most 0, and its fuel is at least
//! `_ShipAfterburnerFuelUsage` (@0x7a82-0x7b0f), compared raw.
//!
//! - The player's +0x4c is never above 0 (ship-handling-fidelity phase 2,
//!   finding 6) and the engine has no player hold-off, so that test
//!   always passes and takes no state here.
//! - `_HasAfterburner` is true for anyone while cheats are on; the engine
//!   has no cheats.
//!
//! **While on**, after the move (@0x6f970-0x6fcc0):
//!
//! - The box is set to `_ShipMaxSpeed` x [`AFTERBURNER_SPEED`] on both
//!   axes, unless gravity sources are present (`_gravitySourcesPresent`,
//!   *0x3b5568), when it is left as it is (@0x6f97e-0x6f9d9;
//!   [`MaxVel::burn`]).
//! - An inertialess ship's speed gains `_ShipAccelRate` x [`PUSH`], up
//!   to `_ShipMaxSpeed` x [`AFTERBURNER_SPEED`] (@0x6f9e4-0x6faa4;
//!   [`push_speed`]). Any other ship's velocity is pushed along its
//!   heading ([`push_velocity`], @0x6faa9-0x6fbee).
//! - The fuel falls by the usage ([`drain`], @0x6fbf3-0x6fc4a).
//!
//! **Off** (@0x6fcc5-0x6fd3c), the box bleeds back to the top speed
//! ([`MaxVel::bleed`]).
//!
//! None of the pushes, the bleed or the drain is scaled by a frame
//! factor but the drain's `_gSpeedMult`, which is 1 a tick here. The
//! box's two components are written with the same value on every path,
//! so they are always equal, and [`MaxVel`] keeps one. None of these
//! blocks plays a sound.

use crate::flight::facing;
use crate::geometry::Vec2;
use crate::speed_cap::whole_degree;

/// The afterburner's top speed, in units of `_ShipMaxSpeed` (the double
/// at 0xdd6e0): the thrust's top (@0x6cc80), the box (@0x6f9a7), the
/// pushes' (@0x6fa65, @0x6fb27) and the knockback's cap (@0x3ab56).
pub const AFTERBURNER_SPEED: f64 = 1.8;

/// The afterburner's push, in ticks of `_ShipAccelRate`: f32 2.75 at
/// 0xdd96c for an inertial ship's velocity (@0x6fae8), and the double
/// 2.75 at 0xdd9b8 for an inertialess ship's speed (@0x6fa1a).
pub const PUSH: f32 = 2.75;

/// The factor an axis of the velocity is multiplied by once the push has
/// carried it past the afterburner's top speed (the double at 0xdd9c0).
pub const DAMP: f64 = 0.99;

/// How many ticks of `_ShipAccelRate` the box bleeds by a tick once the
/// afterburner is off (the double at 0xdd1e8, @0x6fcf1).
pub const BLEED: f64 = 0.4;

/// The fuel an afterburner uses a tick per unit of its `ModVal` (the
/// double at 0xdd100, @0x7aae): the Bible's "fuel usage per second" over
/// 30 frames.
pub const FUEL_PER_MOD_VAL: f64 = 0.0333;

/// How near none a component of the heading counts as none: the
/// original's whole-degree tables give exactly none at 0°, 90°, 180° and
/// 270°, where `sin`/`cos` of the port's radians leave a little.
const ZERO_SLACK: f32 = 1e-6;

/// The afterburner's top speed for a top speed of `max_speed`:
/// `max_speed` x [`AFTERBURNER_SPEED`], in doubles, as f32.
#[must_use]
pub fn afterburner_top(max_speed: f32) -> f32 {
    (f64::from(max_speed) * AFTERBURNER_SPEED) as f32
}

/// Whether the afterburner is on this tick (see the module docs): its
/// `key` is held, the ship is `intact` (not disabled), it carries an
/// afterburner using `usage` fuel a tick (none without one), its jump is
/// not under way (`jumping`), and `fuel` is above none and at least the
/// usage.
#[must_use]
pub fn engages(key: bool, intact: bool, usage: Option<f32>, jumping: bool, fuel: f32) -> bool {
    key && intact && !jumping && fuel > 0.0 && usage.is_some_and(|usage| fuel >= usage)
}

/// `velocity` after an afterburner tick along `heading` for a ship of
/// `accel` (`_ShipAccelRate`) and top speed `max_speed`
/// (@0x6faa9-0x6fbee): with A = `accel` x [`PUSH`] and B =
/// [`afterburner_top`] along the heading's whole degree, on each axis (y
/// then x), if A is above none it adds A while the axis is below B; if A
/// is none or below it adds A while the axis is above B; otherwise the
/// axis is multiplied by [`DAMP`]. A heading on a quarter leaves one
/// component at none, so that axis keeps a velocity above none and damps
/// one at or below it.
#[must_use]
pub fn push_velocity(velocity: Vec2, heading: f32, accel: f32, max_speed: f32) -> Vec2 {
    let along = facing(whole_degree(heading));
    let snap = |share: f32| if share.abs() < ZERO_SLACK { 0.0 } else { share };
    let (push, top) = (accel * PUSH, afterburner_top(max_speed));
    let axis = |v: f32, share: f32| {
        let (a, b) = (snap(share) * push, snap(share) * top);
        let short = if a > 0.0 { v < b } else { v > b };
        if short {
            v + a
        } else {
            (f64::from(v) * DAMP) as f32
        }
    };
    Vec2::new(axis(velocity.x, along.x), axis(velocity.y, along.y))
}

/// An inertialess ship's `speed` after an afterburner tick for a ship of
/// `accel` and top speed `max_speed` (@0x6f9e4-0x6faa4): `accel` x
/// [`PUSH`] more, in doubles, down to [`afterburner_top`] when past it.
#[must_use]
pub fn push_speed(speed: f32, accel: f32, max_speed: f32) -> f32 {
    let pushed = f64::from(speed) + f64::from(accel) * f64::from(PUSH);
    let top = afterburner_top(max_speed);
    if pushed > f64::from(top) {
        top
    } else {
        pushed as f32
    }
}

/// `fuel` after an afterburner tick using `usage`: down by it, floored at
/// none (@0x6fbf3-0x6fc4a).
#[must_use]
pub fn drain(fuel: f32, usage: f32) -> f32 {
    let left = (f64::from(fuel) - f64::from(usage)) as f32;
    if left < 0.0 { 0.0 } else { left }
}

/// The box the player's velocity is clamped to on each axis before it
/// moves, `_playerMaxVel` (*0x3b564c → 0x230538; the clamp
/// @0x6d43f-0x6d4aa, an inertialess speed's cap @0x6d356): written only
/// after the move, so each tick's clamp reads the box the last tick left.
/// Its two components are always equal, so it keeps one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaxVel(f32);

impl MaxVel {
    /// The box a run starts with: none, the original's static data, so the
    /// first flight tick clamps the player to rest.
    pub const START: Self = Self(0.0);

    /// The box's half-width, on each axis.
    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }

    /// The afterburner widens the box to [`afterburner_top`] of
    /// `max_speed`, unless `gravity` sources are present, when it is left
    /// as it is (@0x6f97e-0x6f9d9).
    pub fn burn(&mut self, max_speed: f32, gravity: bool) {
        if !gravity {
            self.0 = afterburner_top(max_speed);
        }
    }

    /// With the afterburner off, a box above `max_speed` narrows by
    /// `accel` x [`BLEED`], and one below it is raised to it
    /// (@0x6fcc5-0x6fd3c).
    pub fn bleed(&mut self, max_speed: f32, accel: f32) {
        let step = (f64::from(accel) * BLEED) as f32;
        if self.0 > max_speed {
            self.0 -= step;
        }
        if max_speed > self.0 {
            self.0 = max_speed;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const ACCEL: f32 = 0.2;
    const TOP: f32 = 5.0;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(AFTERBURNER_SPEED, 1.8);
        assert_eq!(PUSH, 2.75);
        assert_eq!(DAMP, 0.99);
        assert_eq!(BLEED, 0.4);
        assert_eq!(FUEL_PER_MOD_VAL, 0.0333);
        assert_eq!(MaxVel::START.get(), 0.0);
        assert!(near(afterburner_top(TOP), 9.0));
    }

    // Engaging.

    #[test]
    fn the_afterburner_engages_held_intact_carried_out_of_a_jump_and_fuelled() {
        assert!(engages(true, true, Some(0.5), false, 10.0));
        assert!(
            engages(true, true, Some(0.5), false, 0.5),
            "fuel at the usage"
        );
    }

    #[test]
    fn the_afterburner_does_not_engage_if_any_one_test_fails() {
        assert!(!engages(false, true, Some(0.5), false, 10.0), "no key");
        assert!(!engages(true, false, Some(0.5), false, 10.0), "disabled");
        assert!(!engages(true, true, None, false, 10.0), "none carried");
        assert!(!engages(true, true, Some(0.5), true, 10.0), "jumping");
        assert!(!engages(true, true, Some(0.5), false, 0.0), "no fuel");
        assert!(
            !engages(true, true, Some(0.5), false, 0.4),
            "below the usage"
        );
    }

    #[test]
    fn an_afterburner_using_no_fuel_still_needs_some() {
        assert!(engages(true, true, Some(0.0), false, 0.01));
        assert!(!engages(true, true, Some(0.0), false, 0.0));
    }

    // The push.

    #[test]
    fn below_the_afterburners_top_each_axis_gains_the_push_along_the_heading() {
        let push = ACCEL * PUSH;
        let east = push_velocity(Vec2::new(1.0, 0.0), 90.0, ACCEL, TOP);
        assert!(near(east.x, 1.0 + push), "{east:?}");
        let diagonal = push_velocity(Vec2::ZERO, 135.0, ACCEL, TOP);
        let along = facing(135.0) * push;
        assert!(near(diagonal.x, along.x) && near(diagonal.y, along.y));
    }

    #[test]
    fn at_or_past_the_afterburners_top_an_axis_is_damped() {
        let top = afterburner_top(TOP);
        let at = push_velocity(Vec2::new(top, 0.0), 90.0, ACCEL, TOP);
        assert!(near(at.x, top * 0.99), "{at:?}");
        let past = push_velocity(Vec2::new(10.0, 0.0), 90.0, ACCEL, TOP);
        assert!(near(past.x, 9.9));
    }

    #[test]
    fn a_negative_heading_component_pushes_mirrored() {
        let top = afterburner_top(TOP);
        let west = push_velocity(Vec2::new(-1.0, 0.0), 270.0, ACCEL, TOP);
        assert!(near(west.x, -1.0 - ACCEL * PUSH), "{west:?}");
        let past = push_velocity(Vec2::new(-top - 1.0, 0.0), 270.0, ACCEL, TOP);
        assert!(near(past.x, (-top - 1.0) * 0.99));
        let north = push_velocity(Vec2::new(0.0, -1.0), 0.0, ACCEL, TOP);
        assert!(near(north.y, -1.0 - ACCEL * PUSH), "{north:?}");
        // Facing up the share is exactly -1, so the top is exactly -9.
        let on = push_velocity(Vec2::new(0.0, -top), 0.0, ACCEL, TOP);
        assert!(near(on.y, -top * 0.99), "at the top, damped: {on:?}");
    }

    #[test]
    fn on_a_quarter_the_axis_across_keeps_a_positive_velocity_and_damps_any_other() {
        let up = push_velocity(Vec2::new(1.0, 2.0), 90.0, ACCEL, TOP);
        assert_eq!(up.y, 2.0, "kept: {up:?}");
        let down = push_velocity(Vec2::new(1.0, -2.0), 90.0, ACCEL, TOP);
        assert!(near(down.y, -1.98), "damped: {down:?}");
        let still = push_velocity(Vec2::new(1.0, 0.0), 90.0, ACCEL, TOP);
        assert_eq!(still.y, 0.0);
        let across = push_velocity(Vec2::new(3.0, -1.0), 0.0, ACCEL, TOP);
        assert_eq!(across.x, 3.0, "{across:?}");
        let back = push_velocity(Vec2::new(-3.0, -1.0), 180.0, ACCEL, TOP);
        assert!(near(back.x, -2.97), "{back:?}");
    }

    #[test]
    fn the_push_reads_the_headings_whole_degree() {
        let pushed = push_velocity(Vec2::ZERO, 45.9, ACCEL, TOP);
        let along = facing(45.0) * (ACCEL * PUSH);
        assert!(
            near(pushed.x, along.x) && near(pushed.y, along.y),
            "{pushed:?}"
        );
    }

    #[test]
    fn an_inertialess_speed_gains_the_push_up_to_the_afterburners_top() {
        assert!(near(push_speed(1.0, ACCEL, TOP), 1.0 + ACCEL * PUSH));
        let top = afterburner_top(TOP);
        assert_eq!(push_speed(top - 0.1, ACCEL, TOP), top);
        assert_eq!(push_speed(20.0, ACCEL, TOP), top, "down to it");
        assert!(
            near(push_speed(top - 0.55, ACCEL, TOP), top),
            "exactly on it"
        );
    }

    // The fuel.

    #[test]
    fn the_fuel_falls_by_the_usage_to_none() {
        assert!(near(drain(10.0, 0.5), 9.5));
        assert_eq!(drain(0.3, 0.5), 0.0);
        assert_eq!(drain(0.5, 0.5), 0.0);
    }

    // The box.

    #[test]
    fn the_afterburner_widens_the_box_unless_gravity_sources_are_present() {
        let mut box_ = MaxVel::START;
        box_.burn(TOP, false);
        assert!(near(box_.get(), 9.0));
        let mut kept = MaxVel(TOP);
        kept.burn(TOP, true);
        assert_eq!(kept.get(), TOP);
    }

    #[test]
    fn off_the_box_bleeds_by_accel_x_0_4_a_tick_and_settles_at_the_top_speed() {
        // A bleed of 0.125 a tick, which f32 holds exactly.
        let accel = 0.3125;
        let mut box_ = MaxVel(afterburner_top(TOP));
        let mut ticks = 0;
        while box_.get() > TOP {
            let was = box_.get();
            box_.bleed(TOP, accel);
            ticks += 1;
            assert_eq!(was - box_.get(), 0.125, "{was} to {box_:?}");
        }
        assert_eq!(box_.get(), TOP);
        assert_eq!(ticks, 32, "4 over at 0.125 a tick");
        let mut short = MaxVel(TOP + 0.1);
        short.bleed(TOP, accel);
        assert_eq!(
            short.get(),
            TOP,
            "a last bleed past the top is raised to it"
        );
        box_.bleed(TOP, accel);
        assert_eq!(box_.get(), TOP, "settled");
    }

    #[test]
    fn a_box_below_the_top_speed_is_raised_to_it_at_once() {
        let mut box_ = MaxVel::START;
        box_.bleed(TOP, ACCEL);
        assert_eq!(box_.get(), TOP);
    }
}
