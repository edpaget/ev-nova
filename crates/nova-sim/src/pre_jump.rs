//! The pre-jump stage: what the ship does between the jump key being
//! accepted and the jump itself beginning.
//!
//! The original's `_HandlePlayer` (`EV Nova.app/Contents/MacOS/EV Nova`,
//! i386) flies the player's ship through it while the ship's jump state
//! (field 0x6a) is 3:
//!
//! 1. **Brake** (@0x6b6a2-0x6b868) while the ship is not [`slow_enough`]:
//!    its desired heading is the one opposite its velocity, and it thrusts
//!    only while that heading is within [`BRAKE_THRUST_ARC`] (or its turn
//!    rate plus one, if wider) of the way it faces.
//! 2. **Turn** (@0x6b03c-0x6b0d3) to the map bearing of the next system,
//!    coasting on what drift is left.
//! 3. Then the jump begins (@0x6b5e9).
//!
//! The ship is slow enough when each component of its velocity is under
//! [`JUMP_SPEED`] (@0x6b045, @0x6b579): it slows below a speed, and does
//! not brake to a stop. The original also multiplies the velocity by 0.992
//! each frame while braking; that drag is left out, as the simulation has
//! none.
//!
//! [`stage`] says which of these a ship is in, and [`fly`] flies one tick
//! of it. A ship that cannot turn skips the turn, and one that cannot turn
//! or thrust skips the brake, so no hull is held in a stage it can never
//! finish. `slow_down` false skips the brake, as a fast jump does.

use crate::flight::{Controls, ShipState, heading_of, shortest_turn, step, turn_toward};
use crate::geometry::Vec2;
use crate::handling::Handling;

/// The speed, in pixels a tick, that each component of a ship's velocity
/// must be under for it to jump: the original's test that
/// `(u16)(trunc(v) + 1) <= 2` for each of `vx` and `vy` (@0x6b045 and
/// @0x6b579), on the same scale as our speeds (`Speed` / 100 a tick).
pub const JUMP_SPEED: f32 = 2.0;

/// How far off the heading against its motion, in degrees, a braking ship
/// may face and still thrust, unless its turn rate plus one is wider: the
/// original's `max(ShipTurnRate + 1, 20)` (@0x6b6a2-0x6b868).
pub const BRAKE_THRUST_ARC: f32 = 20.0;

/// Which part of the pre-jump stage a ship is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreJump {
    /// Slowing down: turning against its motion and thrusting.
    Brake,
    /// Turning to the bearing of the next system.
    Turn,
    /// Ready: the jump can begin.
    Ready,
}

/// Whether a ship moving at `velocity` is slow enough to jump: each
/// component under [`JUMP_SPEED`].
#[must_use]
pub fn slow_enough(velocity: Vec2) -> bool {
    velocity.x.abs() < JUMP_SPEED && velocity.y.abs() < JUMP_SPEED
}

/// The stage `ship`, flying as `handling` allows, is in on its way to a
/// jump towards `bearing` (none when there is nothing to turn to), braking
/// first when `slow_down`.
#[must_use]
pub fn stage(
    ship: &ShipState,
    handling: &Handling,
    bearing: Option<f32>,
    slow_down: bool,
) -> PreJump {
    let can_turn = handling.turn_rate > 0.0;
    if slow_down && !slow_enough(ship.velocity) && can_turn && handling.accel > 0.0 {
        PreJump::Brake
    } else if can_turn && bearing.is_some_and(|to| shortest_turn(ship.heading, to).abs() > 0.0) {
        PreJump::Turn
    } else {
        PreJump::Ready
    }
}

/// Flies `ship` one tick of the stage it is in (see [`stage`]), and gives
/// whether it thrust. A ready ship does not move.
pub fn fly(
    ship: &mut ShipState,
    handling: &Handling,
    bearing: Option<f32>,
    slow_down: bool,
) -> bool {
    match stage(ship, handling, bearing, slow_down) {
        PreJump::Brake => {
            let behind = heading_of(ship.velocity * -1.0);
            let arc = BRAKE_THRUST_ARC.max(handling.turn_rate + 1.0);
            let thrust = shortest_turn(ship.heading, behind).abs() < arc;
            let controls = Controls {
                thrust,
                reverse: true,
                ..Controls::default()
            };
            step(ship, handling, controls);
            thrust
        }
        PreJump::Turn => {
            if let Some(bearing) = bearing {
                ship.heading = turn_toward(ship.heading, bearing, handling.turn_rate);
            }
            step(ship, handling, Controls::default());
            false
        }
        PreJump::Ready => false,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// 6 pixels a tick at most, 0.3 more each tick, 3° a tick.
    const SHIP: Handling = Handling {
        max_speed: 6.0,
        accel: 0.3,
        turn_rate: 3.0,
    };

    /// East, as the map bears from Sol to Alpha Centauri.
    const EAST: Option<f32> = Some(90.0);

    fn ship(velocity: Vec2, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(0.0, 1000.0),
            velocity,
            heading,
        }
    }

    // Slow enough.

    #[test]
    fn each_component_must_be_under_the_jump_speed() {
        assert!(slow_enough(Vec2::ZERO));
        assert!(slow_enough(Vec2::new(1.9, 1.9)), "though its speed is 2.69");
        assert!(slow_enough(Vec2::new(-1.99, 0.0)));
        assert!(slow_enough(Vec2::new(0.0, -1.99)));
        assert!(!slow_enough(Vec2::new(2.0, 0.0)));
        assert!(!slow_enough(Vec2::new(-2.0, 0.0)));
        assert!(!slow_enough(Vec2::new(0.0, -2.0)));
        assert!(!slow_enough(Vec2::new(0.0, 2.0)));
        assert!(!slow_enough(Vec2::new(1.0, 2.5)));
        assert!(!slow_enough(Vec2::new(2.5, 1.0)));
    }

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(JUMP_SPEED, 2.0);
        assert_eq!(BRAKE_THRUST_ARC, 20.0);
    }

    // Stages.

    #[test]
    fn a_ship_at_rest_facing_the_bearing_is_ready() {
        assert_eq!(
            stage(&ship(Vec2::ZERO, 90.0), &SHIP, EAST, true),
            PreJump::Ready
        );
    }

    #[test]
    fn a_ship_at_rest_facing_away_turns() {
        assert_eq!(
            stage(&ship(Vec2::ZERO, 0.0), &SHIP, EAST, true),
            PreJump::Turn
        );
        assert_eq!(
            stage(&ship(Vec2::ZERO, 89.9), &SHIP, EAST, true),
            PreJump::Turn
        );
    }

    #[test]
    fn a_moving_ship_brakes_unless_it_need_not_slow_down() {
        let moving = ship(Vec2::new(0.0, -3.0), 90.0);
        assert_eq!(stage(&moving, &SHIP, EAST, true), PreJump::Brake);
        assert_eq!(
            stage(&moving, &SHIP, EAST, false),
            PreJump::Ready,
            "facing it"
        );
        let away = ship(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(stage(&away, &SHIP, EAST, false), PreJump::Turn);
        let slow = ship(Vec2::new(1.9, -1.9), 90.0);
        assert_eq!(
            stage(&slow, &SHIP, EAST, true),
            PreJump::Ready,
            "slow enough"
        );
    }

    #[test]
    fn without_a_bearing_a_slow_ship_is_ready_whichever_way_it_faces() {
        assert_eq!(
            stage(&ship(Vec2::ZERO, 0.0), &SHIP, None, true),
            PreJump::Ready
        );
        let moving = ship(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(stage(&moving, &SHIP, None, true), PreJump::Brake);
    }

    #[test]
    fn a_ship_never_waits_in_a_stage_it_cannot_finish() {
        let rigid = Handling {
            turn_rate: 0.0,
            ..SHIP
        };
        let moving_away = ship(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(stage(&moving_away, &rigid, EAST, true), PreJump::Ready);
        let engineless = Handling { accel: 0.0, ..SHIP };
        assert_eq!(stage(&moving_away, &engineless, EAST, true), PreJump::Turn);
        let facing = ship(Vec2::new(0.0, -3.0), 90.0);
        assert_eq!(stage(&facing, &engineless, EAST, true), PreJump::Ready);
    }

    // Flying the stages.

    #[test]
    fn a_turning_ship_reaches_the_bearing_at_its_turn_rate_and_drifts() {
        let mut state = ship(Vec2::new(0.5, 0.0), 0.0);
        for k in 1..=30 {
            assert_eq!(stage(&state, &SHIP, EAST, true), PreJump::Turn, "tick {k}");
            let before = state;
            assert!(!fly(&mut state, &SHIP, EAST, true), "no thrust");
            assert!(state.heading <= 90.0, "never overshoots: {state:?}");
            assert_eq!(
                state.heading,
                before.heading + 3.0,
                "clockwise, the short way"
            );
            assert_eq!(state.velocity, before.velocity, "coasting");
            assert_eq!(state.position, before.position + before.velocity);
        }
        assert_eq!(state.heading, 90.0);
        assert_eq!(stage(&state, &SHIP, EAST, true), PreJump::Ready);
    }

    #[test]
    fn a_turn_goes_the_short_way_round() {
        let mut state = ship(Vec2::ZERO, 180.0);
        fly(&mut state, &SHIP, EAST, true);
        assert_eq!(state.heading, 177.0);
        let mut state = ship(Vec2::ZERO, 350.0);
        fly(&mut state, &SHIP, Some(10.0), true);
        assert_eq!(state.heading, 353.0);
    }

    #[test]
    fn a_ready_ship_does_not_move() {
        let ready = ship(Vec2::new(1.0, 1.0), 90.0);
        let mut state = ready;
        assert!(!fly(&mut state, &SHIP, EAST, true));
        assert_eq!(state, ready);
    }

    /// The braking thrust of a ship with `handling` moving up at 3 pixels a
    /// tick, facing `off` degrees clockwise of straight down (against its
    /// motion).
    fn brakes_at(handling: &Handling, off: f32) -> bool {
        let mut state = ship(Vec2::new(0.0, -3.0), 180.0 + off);
        fly(&mut state, handling, EAST, true)
    }

    #[test]
    fn a_braking_ship_thrusts_only_once_it_faces_nearly_against_its_motion() {
        assert!(brakes_at(&SHIP, 0.0));
        assert!(brakes_at(&SHIP, 19.9));
        assert!(brakes_at(&SHIP, -19.9));
        assert!(!brakes_at(&SHIP, 20.0));
        assert!(!brakes_at(&SHIP, 20.1));
        assert!(!brakes_at(&SHIP, -20.1));
        assert!(!brakes_at(&SHIP, 180.0));
    }

    #[test]
    fn a_ship_that_turns_faster_than_the_arc_thrusts_within_its_turn_plus_one() {
        let nimble = Handling {
            turn_rate: 30.0,
            ..SHIP
        };
        assert!(brakes_at(&nimble, 30.9));
        assert!(!brakes_at(&nimble, 31.0));
        let at_19 = Handling {
            turn_rate: 19.5,
            ..SHIP
        };
        assert!(brakes_at(&at_19, 20.4), "19.5 + 1 is wider than 20");
        assert!(!brakes_at(&at_19, 20.5));
    }

    #[test]
    fn braking_turns_against_the_motion_then_slows_the_ship_below_the_jump_speed() {
        // Facing along its motion, up, at top speed.
        let mut state = ship(Vec2::new(0.0, -6.0), 0.0);
        let mut thrust_began = None;
        for k in 0..200 {
            if stage(&state, &SHIP, EAST, true) != PreJump::Brake {
                break;
            }
            let off = shortest_turn(state.heading, 180.0).abs();
            let speed = state.velocity.length();
            let thrust = fly(&mut state, &SHIP, EAST, true);
            assert_eq!(thrust, off < BRAKE_THRUST_ARC, "tick {k}: {state:?}");
            if thrust {
                thrust_began.get_or_insert(k);
                assert!(state.velocity.length() < speed, "slowing: {state:?}");
            } else {
                assert_eq!(state.velocity.length(), speed, "coasting: {state:?}");
            }
        }
        // 54 ticks to come within 20 degrees of 180 at 3 a tick.
        assert_eq!(thrust_began, Some(54));
        assert!(slow_enough(state.velocity), "{state:?}");
        assert!(
            state.velocity.length() > 1.0,
            "slowed, not stopped: {state:?}"
        );
        assert_eq!(stage(&state, &SHIP, EAST, true), PreJump::Turn);
    }

    #[test]
    fn a_ship_that_need_not_slow_down_turns_while_it_coasts() {
        let mut state = ship(Vec2::new(0.0, -6.0), 0.0);
        assert!(!fly(&mut state, &SHIP, EAST, false));
        assert_eq!(state.heading, 3.0);
        assert_eq!(state.velocity, Vec2::new(0.0, -6.0));
    }
}
