//! Inertialess flight: a ship that holds a speed along its heading, and
//! whose velocity is pulled toward that heading each tick, so it turns
//! without drifting.
//!
//! The original (`EV Nova.app/Contents/MacOS/EV Nova`, i386) keeps the
//! speed at ship +0x48. Thrust and brakes change the speed, not the
//! velocity, and `_AdjustInertialessShipVelocity` (@0x3349c-0x33580)
//! pulls the velocity toward heading x speed ([`adjust_velocity`]).
//!
//! - **Who is inertialess.** `_ShipIsInertialess` (@0x79c2-0x7a81,
//!   [`is_inertialess`]): not an NPC keeping close formation with a
//!   leader (@0x79d4-0x79f2); otherwise a hull whose `shïp` `Flags2` holds
//!   0x0040 (@0x7a06); otherwise, for the player only (@0x7a10), one
//!   carrying an `oütf` of `ModType` 38, the inertial dampener, whatever
//!   its `ModVal` (@0x7a2a-0x7a6d). [`RuleKey::InertialDampener`] chooses
//!   between that and the Bible's reading, under which the outfit makes
//!   any ship inertialess. The original caches the player's outfit scan
//!   (*0x230524, reset by `_ResetPlayerPrecalcedValues` @0xc3ca); the
//!   engine's stats are worked out afresh whenever the outfits change, so
//!   the predicate needs no cache.
//! - **The speed.** [`step_speed`] moves it by a signed thrust in units
//!   of the ship's acceleration ([`THRUST`], [`REVERSE`], [`BRAKE`]),
//!   capped at the top speed and floored at none, as the player's thrust
//!   and reverse keys (@0x6cbb2-0x6cc3e, @0x6c770-0x6c7d3), its pre-jump
//!   brake (@0x6b6ba-0x6b719) and an NPC's thrust word (`_HandleShip`
//!   @0x34ae9-0x34b6d) move it.
//! - **The velocity.** [`adjust_velocity`]: each axis moves toward the
//!   heading's whole degree times the speed, by at most [`ADJUST_RATE`]
//!   ticks' acceleration.

use crate::flight::{Flyer, facing};
use crate::geometry::Vec2;
#[cfg(any(doc, test))]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;
use crate::speed_cap::whole_degree;

/// How many ticks' acceleration an inertialess ship's velocity may move on
/// each axis a tick: `_ShipAccelRate` x 4.0 (f32 @0xdd680, @0x334f8).
pub const ADJUST_RATE: f32 = 4.0;

/// The thrust key's and an NPC's thrust: one tick's acceleration added to
/// the speed (@0x6cbe2-0x6cc05, @0x34aef-0x34b5d).
pub const THRUST: f32 = 1.0;

/// The player's reverse key and the pre-jump's brake: one tick's
/// acceleration taken off the speed (@0x6c77c-0x6c7cc, @0x6b6c3-0x6b719).
pub const REVERSE: f32 = -1.0;

/// An NPC's brake, `_LowLevelAIHandler` substate 1: half a tick's
/// acceleration taken off the speed (f32 -0.5 @0xdda34, @0x853f6).
pub const BRAKE: f32 = -0.5;

/// The factor a disabled inertialess player's speed is multiplied by each
/// tick (the double at 0xdd9b0, @0x6d386). A disabled NPC's speed slows
/// by its velocity's [`DISABLED_DRAG`](crate::flight::DISABLED_DRAG)
/// instead (@0x3398f-0x339a1).
pub const PLAYER_DISABLED_DECAY: f32 = 0.985;

/// Whose `ModType` 38 outfits make a ship inertialess, by
/// [`RuleKey::InertialDampener`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InertialDampener {
    /// The original's: the player's alone (@0x7a10-0x7a26).
    #[default]
    PlayerOnly,
    /// The Bible's "makes ship inertialess": any ship's.
    AnyShip,
}

impl From<RuleSource> for InertialDampener {
    /// The engine's reading counts the player's outfit alone; the Bible's,
    /// any ship's.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::PlayerOnly,
            RuleSource::Bible => Self::AnyShip,
        }
    }
}

/// Whether a ship flies inertialess (`_ShipIsInertialess`, see the module
/// docs): one flown by `flyer`, whose hull is inertialess (`hull`) or
/// that carries a dampener (`dampener`), counted as `rule` says. An NPC
/// keeping close formation with a leader (`close_formation`) never is;
/// the player always may be.
#[must_use]
pub fn is_inertialess(
    hull: bool,
    dampener: bool,
    flyer: Flyer,
    close_formation: bool,
    rule: InertialDampener,
) -> bool {
    if flyer == Flyer::Npc && close_formation {
        return false;
    }
    hull || (dampener && (flyer == Flyer::Player || rule == InertialDampener::AnyShip))
}

/// `speed` moved by `thrust` ticks of `accel` (a signed thrust: see
/// [`THRUST`], [`REVERSE`] and [`BRAKE`]), capped at `top` and floored at
/// none.
#[must_use]
pub fn step_speed(speed: f32, thrust: f32, accel: f32, top: f32) -> f32 {
    thrust.mul_add(accel, speed).min(top).max(0.0)
}

/// `velocity` pulled toward heading x `speed`, by at most `rate` on each
/// axis: `_AdjustInertialessShipVelocity` (@0x3349c-0x33580). The target
/// is `_Accel` of the heading's whole degree (`cvttss2si` @0x334d0), as
/// thrust's is ([`adjusted_accel`](crate::speed_cap::adjusted_accel)).
/// On each axis, a velocity at least `rate` short of the target gains
/// `rate`, one at least `rate` past it loses `rate`, and one nearer lands
/// on it (@0x3350c-0x3356d).
#[must_use]
pub fn adjust_velocity(velocity: Vec2, heading: f32, speed: f32, rate: f32) -> Vec2 {
    let target = facing(whole_degree(heading)) * speed;
    let toward = |at: f32, to: f32| {
        if to - rate >= at {
            at + rate
        } else if to + rate <= at {
            at - rate
        } else {
            to
        }
    };
    Vec2::new(toward(velocity.x, target.x), toward(velocity.y, target.y))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::rulebook::Rulebook;

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(ADJUST_RATE, 4.0);
        assert_eq!((THRUST, REVERSE, BRAKE), (1.0, -1.0, -0.5));
        assert_eq!(PLAYER_DISABLED_DECAY, 0.985);
    }

    // The velocity.

    #[test]
    fn within_the_rate_the_velocity_lands_on_heading_times_speed() {
        let east = adjust_velocity(Vec2::new(4.5, 0.5), 90.0, 5.0, 1.0);
        assert!(near(east, Vec2::new(5.0, 0.0)), "{east:?}");
        let up = adjust_velocity(Vec2::new(0.0, -2.0), 0.0, 3.0, 1.0);
        assert!(
            near(up, Vec2::new(0.0, -3.0)),
            "exactly the rate off: {up:?}"
        );
    }

    #[test]
    fn beyond_the_rate_each_axis_moves_by_exactly_the_rate() {
        // Turned from east to north at speed 5: x has 5 to lose, y 5 to
        // gain (north is -y).
        let turned = adjust_velocity(Vec2::new(5.0, 0.0), 0.0, 5.0, 0.4);
        assert!(near(turned, Vec2::new(4.6, -0.4)), "{turned:?}");
        let backed = adjust_velocity(Vec2::new(-5.0, 5.0), 0.0, 0.0, 2.0);
        assert!(
            near(backed, Vec2::new(-3.0, 3.0)),
            "toward rest: {backed:?}"
        );
    }

    #[test]
    fn each_axis_moves_on_its_own() {
        // y is on its target already; only x moves.
        let v = adjust_velocity(Vec2::new(-3.0, -2.0), 0.0, 2.0, 1.0);
        assert!(near(v, Vec2::new(-2.0, -2.0)), "{v:?}");
        // x within the rate, y beyond it.
        let v = adjust_velocity(Vec2::new(0.5, 3.0), 0.0, 2.0, 1.0);
        assert!(near(v, Vec2::new(0.0, 2.0)), "{v:?}");
    }

    #[test]
    fn it_aims_along_the_headings_whole_degree() {
        let speed = 10.0;
        let aimed = adjust_velocity(Vec2::ZERO, 89.7, speed, 100.0);
        assert!(near(aimed, facing(89.0) * speed), "{aimed:?}");
        assert!(!near(aimed, facing(89.7) * speed));
    }

    #[test]
    fn at_rest_with_no_speed_it_stays_at_rest() {
        assert_eq!(adjust_velocity(Vec2::ZERO, 45.0, 0.0, 1.0), Vec2::ZERO);
    }

    // The speed.

    #[test]
    fn thrust_adds_a_ticks_acceleration_up_to_the_top_speed() {
        assert_eq!(step_speed(1.0, THRUST, 0.5, 3.0), 1.5);
        assert_eq!(step_speed(2.75, THRUST, 0.5, 3.0), 3.0, "capped");
        assert_eq!(step_speed(3.0, 0.0, 0.5, 3.0), 3.0, "coasting");
        assert_eq!(step_speed(4.0, 0.0, 0.5, 3.0), 3.0, "above the top");
    }

    #[test]
    fn reverse_takes_a_ticks_acceleration_off_down_to_none() {
        assert_eq!(step_speed(1.0, REVERSE, 0.25, 3.0), 0.75);
        assert_eq!(step_speed(0.1, REVERSE, 0.25, 3.0), 0.0, "floored");
        assert_eq!(step_speed(0.0, REVERSE, 0.25, 3.0), 0.0);
    }

    #[test]
    fn the_npc_brake_takes_half_a_tick_off() {
        assert_eq!(step_speed(1.0, BRAKE, 0.25, 3.0), 0.875);
        assert_eq!(step_speed(0.1, BRAKE, 0.25, 3.0), 0.0);
    }

    // Who is inertialess.

    const ENGINE: InertialDampener = InertialDampener::PlayerOnly;
    const BIBLE: InertialDampener = InertialDampener::AnyShip;

    #[test]
    fn an_inertialess_hull_is_inertialess_for_either_flyer_under_either_rule() {
        for flyer in [Flyer::Player, Flyer::Npc] {
            for rule in [ENGINE, BIBLE] {
                assert!(is_inertialess(true, false, flyer, false, rule), "{flyer:?}");
                assert!(
                    !is_inertialess(false, false, flyer, false, rule),
                    "{flyer:?}"
                );
            }
        }
    }

    #[test]
    fn a_dampener_counts_for_the_player_alone_by_the_engine_and_for_any_ship_by_the_bible() {
        assert!(is_inertialess(false, true, Flyer::Player, false, ENGINE));
        assert!(!is_inertialess(false, true, Flyer::Npc, false, ENGINE));
        assert!(is_inertialess(false, true, Flyer::Player, false, BIBLE));
        assert!(is_inertialess(false, true, Flyer::Npc, false, BIBLE));
    }

    #[test]
    fn an_npc_in_close_formation_is_never_inertialess_but_the_player_is_never_excepted() {
        for rule in [ENGINE, BIBLE] {
            assert!(!is_inertialess(true, true, Flyer::Npc, true, rule));
            assert!(is_inertialess(true, false, Flyer::Player, true, rule));
            assert!(is_inertialess(false, true, Flyer::Player, true, rule));
        }
    }

    #[test]
    fn the_rule_reads_the_engine_by_default() {
        assert_eq!(
            InertialDampener::from(Rulebook::default().source_for(RuleKey::InertialDampener)),
            InertialDampener::PlayerOnly
        );
        assert_eq!(
            InertialDampener::from(RuleSource::Bible),
            InertialDampener::AnyShip
        );
        assert_eq!(InertialDampener::default(), InertialDampener::PlayerOnly);
    }
}
