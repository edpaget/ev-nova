//! `_LowLevelAIHandler`'s substate 2 (@0x8563b-0x857bd in the `EV Nova`
//! executable): a ship closing in on the stellar its +0x6c holds, as the
//! stellar attacker flies its approach
//! ([`stellar_attack`](crate::traffic::stellar_attack)).
//! rdm:phase/stellar-fidelity/phase-5-npc-landing-reach's state 1 can fly
//! its approach ("outside → sub 2") by [`toward`] too.
//!
//! - It turns to `_CalcInterceptAngle(ship, stellar)`, the whole-degree
//!   bearing of the stellar ([`trig::intercept_angle`]).
//! - It thrusts while off that bearing by less than its turn rate and
//!   [`TURN_SLACK`] degrees, strictly ([`fire::off_by`]), at its full
//!   `_ShipAccelRate`: within [`NEAR`] of the stellar on both axes,
//!   strictly, capped at [`NEAR_CAP`] of its `_ShipMaxSpeed` (+0x34);
//!   beyond, at its top speed (+0x34 = 0). A position that is not a number
//!   reads as far.
//! - Otherwise it does not thrust.
//!
//! It writes +0x34 with +0x30, so its thrust drops a gate burst it owes
//! ([`held_burst`](crate::traffic::held_burst)'s `Word::Writes`). The
//! substate's gate on +0x6c and `_IsDisabled` is the caller's: a stellar
//! attacker always has its stellar, and a disabled ship flies no goal. It
//! plays no sound.

use crate::ai::fire;
use crate::flight::{Controls, ShipState};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::traffic::autopilot::steer;
use crate::trig;

/// How far beyond a turn, in degrees, a ship closing in still thrusts
/// (5.0 @0xdd67c).
pub const TURN_SLACK: f32 = 5.0;
/// Within this of the stellar on both axes, strictly, its speed is capped
/// (500.0 @0xdd080).
pub const NEAR: f32 = 500.0;
/// The share of its top speed it is capped at there (0.25 @0xdd684).
pub const NEAR_CAP: f32 = 0.25;

/// Substate 2's controls for a ship at `state` on `handling` closing in on
/// a stellar at `stellar`, and the handling they fly by: its top speed
/// capped near the stellar (see the module docs).
#[must_use]
pub fn toward(state: &ShipState, stellar: Vec2, handling: &Handling) -> (Controls, Handling) {
    let bearing = trig::intercept_angle(state.position, stellar);
    let controls = Controls {
        thrust: fire::off_by(state.heading, bearing) < handling.turn_rate + TURN_SLACK,
        turn: steer(state.heading, f32::from(bearing), handling),
        ..Controls::default()
    };
    let off = state.position - stellar;
    let flown = if off.x.abs() < NEAR && off.y.abs() < NEAR {
        Handling {
            max_speed: handling.max_speed * NEAR_CAP,
            ..*handling
        }
    } else {
        *handling
    };
    (controls, flown)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::flight::Turn;

    const HANDLING: Handling = Handling {
        max_speed: 8.0,
        accel: 0.2,
        turn_rate: 4.0,
    };

    /// A ship at (`x`, `y`) heading `heading`.
    fn at(x: f32, y: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            heading,
            ..ShipState::default()
        }
    }

    /// The bearing from (`x`, `y`) to the centre.
    fn bearing(x: f32, y: f32) -> f32 {
        f32::from(trig::intercept_angle(Vec2::new(x, y), Vec2::ZERO))
    }

    #[test]
    fn closing_in_it_turns_to_the_bearing_and_thrusts_within_its_turn_and_5_degrees() {
        let (x, y) = (0.0, 800.0);
        let to = bearing(x, y);
        let (facing, _) = toward(&at(x, y, to), Vec2::ZERO, &HANDLING);
        assert!(facing.thrust);
        assert_eq!(facing.turn, Turn::None);
        let limit = HANDLING.turn_rate + TURN_SLACK;
        let (inside, _) = toward(&at(x, y, to + limit - 1.0), Vec2::ZERO, &HANDLING);
        assert!(inside.thrust, "8 degrees off");
        let (edge, _) = toward(&at(x, y, to + limit), Vec2::ZERO, &HANDLING);
        assert!(!edge.thrust, "9 degrees off is not within");
        assert_eq!(edge.turn, Turn::Left, "it turns back");
        let (other, _) = toward(&at(x, y, to - limit), Vec2::ZERO, &HANDLING);
        assert!(!other.thrust);
        assert_eq!(other.turn, Turn::Right);
        assert!(!facing.reverse);
    }

    #[test]
    fn within_500_on_both_axes_it_is_capped_at_a_quarter_of_its_top_speed() {
        let flown = |x: f32, y: f32| toward(&at(x, y, bearing(x, y)), Vec2::ZERO, &HANDLING).1;
        let capped = Handling {
            max_speed: 2.0,
            ..HANDLING
        };
        assert_eq!(flown(499.0, -499.0), capped);
        assert_eq!(flown(500.0, 0.0), HANDLING, "strictly");
        assert_eq!(flown(0.0, -500.0), HANDLING);
        assert_eq!(flown(600.0, 10.0), HANDLING);
        assert_eq!(
            toward(&at(f32::NAN, 0.0, 0.0), Vec2::ZERO, &HANDLING).1,
            HANDLING,
            "not a number reads as far"
        );
        assert_eq!((TURN_SLACK, NEAR, NEAR_CAP), (5.0, 500.0, 0.25));
    }
}
