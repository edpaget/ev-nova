//! The pre-jump stage: what the ship does between the jump key being
//! accepted and the jump itself beginning.
//!
//! The original's `_HandlePlayer` (`EV Nova.app/Contents/MacOS/EV Nova`,
//! i386) flies the player's ship through it while the ship's jump state
//! (field 0x6a) is 3:
//!
//! 1. **Brake** (@0x6b6a2-0x6b868) while the ship is not [`slow_enough`]:
//!    its desired heading is the one opposite its velocity
//!    ([`brake_aim`]), and it thrusts only while that heading is within
//!    [`BRAKE_THRUST_ARC`] (or its turn rate plus one, if wider) of the
//!    way it faces.
//! 2. **Turn** (@0x6b03c-0x6b0d3) to the map bearing of the next system,
//!    in whole degrees as `_CalcInterceptAngle` gives it, coasting on what
//!    drift is left.
//! 3. Then the jump begins.
//!
//! Both set the auto-turn local −0x2cd (@0x6b861, @0x6b0d3), so both turn
//! as the player's turn block does
//! ([`player_turn`](crate::player_turn)): by the turn rate's whole
//! degrees, stopping within a turn of the aim without snapping onto it.
//! The turn is done once the ship is within that turn ([`settled`]). The
//! warp's own facing test, within max(`_ShipTurnRate`, 30°) of the
//! bearing (@0x6d16d-0x6d1a1), and the brake's whole-degree arc
//! (@0x6b765-0x6b7ef) are
//! rdm:phase/ship-handling-fidelity/phase-8-warp-out's.
//!
//! The original's warp stage starts as the brake ends, not after the
//! turn: on the first frame the ship is slow enough, or at once when it
//! can expedite its jumps (@0x6b56a-0x6b596), `_HandlePlayer` sets +0x50
//! to 2.0 (@0x6b5e9), stamps +0xa4 once (@0x6b5f0-0x6b5f5) and starts the
//! warp-up sound (@0x6b60d-0x6b690); while the ship brakes it sets +0x50
//! to 1.0 and restamps +0xa4 every frame (@0x6b6a2-0x6b6b1). The turn
//! needs +0x50 above 0 and a slow ship, so it runs inside the warp stage.
//! The session's warp clock starts on the first tick whose stage is not
//! [`PreJump::Brake`] (see the [`warp`](crate::warp) module).
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
//!
//! **An inertialess ship** (@0x6b6ba-0x6b719) brakes its speed along its
//! heading by a tick's acceleration instead, down to none
//! ([`REVERSE`]), neither turning (it sets no auto-turn, -0x2cd) nor
//! aiming (+0x68), and its velocity follows the speed (@0x6d43a, which
//! both stages reach); so it needs only thrust to brake. Both branches
//! go on to the same code (@0x6b941 and @0x6babc each join @0x6bad1 with
//! the same flag), so its brake, too, ends once the ship is slow enough,
//! usually before its speed is none. It turns as any ship does, its
//! velocity following its heading.

use crate::flight::{
    FlightRules, Flyer, ShipState, Turn, advance, heading_of, inertialess_advance, shortest_turn,
};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::inertialess::REVERSE;
use crate::player_turn::{PlayerTurn, brake_aim, settled, step_of, turn};

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

/// The stage `ship`, flying as `handling` allows and turning as `rule`
/// says, is in on its way to a jump towards `bearing`, in whole degrees
/// (none when there is nothing to turn to), braking first when
/// `slow_down`. A ship that turns less than a whole degree a tick turns
/// none ([`step_of`]), so it neither brakes nor turns, unless it is
/// `inertialess`, whose brake needs no turn (see the module docs).
#[must_use]
pub fn stage(
    ship: &ShipState,
    handling: &Handling,
    bearing: Option<i16>,
    slow_down: bool,
    rule: PlayerTurn,
    inertialess: bool,
) -> PreJump {
    let rate = handling.turn_rate;
    let can_turn = step_of(rate, rule) > 0.0;
    let can_brake = (can_turn || inertialess) && handling.accel > 0.0;
    if slow_down && !slow_enough(ship.velocity) && can_brake {
        PreJump::Brake
    } else if can_turn && bearing.is_some_and(|to| !settled(ship.heading, rate, to, rule)) {
        PreJump::Turn
    } else {
        PreJump::Ready
    }
}

/// Flies `ship` one tick of the stage it is in (see [`stage`]) under
/// `flight`, and gives whether it thrust. A ready ship does not move.
/// An inertialess ship flies holding `speed` along its heading (see the
/// module docs), and never thrusts.
pub fn fly(
    ship: &mut ShipState,
    handling: &Handling,
    bearing: Option<i16>,
    slow_down: bool,
    flight: FlightRules,
    speed: Option<&mut f32>,
) -> bool {
    let rule = flight.player_turn;
    let stage = stage(ship, handling, bearing, slow_down, rule, speed.is_some());
    if let Some(speed) = speed {
        let (aim, brake) = match stage {
            PreJump::Brake => (None, REVERSE),
            PreJump::Turn => (bearing, 0.0),
            PreJump::Ready => return false,
        };
        (ship.heading, _) = turn(ship.heading, handling.turn_rate, aim, Turn::None, rule);
        inertialess_advance(ship, speed, handling, brake, flight, Flyer::Player);
        return false;
    }
    let (aim, thrust) = match stage {
        PreJump::Brake => {
            let behind = heading_of(ship.velocity * -1.0);
            let arc = BRAKE_THRUST_ARC.max(handling.turn_rate + 1.0);
            let thrust = shortest_turn(ship.heading, behind).abs() < arc;
            (Some(brake_aim(ship.velocity)), thrust)
        }
        PreJump::Turn => (bearing, false),
        PreJump::Ready => return false,
    };
    (ship.heading, _) = turn(ship.heading, handling.turn_rate, aim, Turn::None, rule);
    advance(ship, handling, thrust, flight, Flyer::Player);
    thrust
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const WHOLE: PlayerTurn = PlayerTurn::WholeDegrees;

    /// 6 pixels a tick at most, 0.3 more each tick, 3° a tick.
    const SHIP: Handling = Handling {
        max_speed: 6.0,
        accel: 0.3,
        turn_rate: 3.0,
    };

    /// East, as the map bears from Sol to Alpha Centauri.
    const EAST: Option<i16> = Some(90);

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
            stage(&ship(Vec2::ZERO, 90.0), &SHIP, EAST, true, WHOLE, false),
            PreJump::Ready
        );
    }

    #[test]
    fn a_ship_at_rest_more_than_a_turn_off_the_bearing_turns() {
        assert_eq!(
            stage(&ship(Vec2::ZERO, 0.0), &SHIP, EAST, true, WHOLE, false),
            PreJump::Turn
        );
        assert_eq!(
            stage(&ship(Vec2::ZERO, 86.9), &SHIP, EAST, true, WHOLE, false),
            PreJump::Turn,
            "whole degree 86, 4 off"
        );
        assert_eq!(
            stage(&ship(Vec2::ZERO, 87.0), &SHIP, EAST, true, WHOLE, false),
            PreJump::Ready,
            "a turn off: within one"
        );
        assert_eq!(
            stage(&ship(Vec2::ZERO, 93.0), &SHIP, EAST, true, WHOLE, false),
            PreJump::Ready
        );
    }

    #[test]
    fn by_the_other_reading_a_ship_turns_until_it_faces_the_bearing() {
        let full = PlayerTurn::Full;
        assert_eq!(
            stage(&ship(Vec2::ZERO, 89.9), &SHIP, EAST, true, full, false),
            PreJump::Turn
        );
        assert_eq!(
            stage(&ship(Vec2::ZERO, 90.0), &SHIP, EAST, true, full, false),
            PreJump::Ready
        );
        let rules = FlightRules {
            player_turn: full,
            ..FlightRules::default()
        };
        let mut state = ship(Vec2::ZERO, 88.5);
        fly(&mut state, &SHIP, EAST, true, rules, None);
        assert_eq!(state.heading, 90.0, "landing on it");
    }

    #[test]
    fn a_ship_turning_under_a_whole_degree_a_tick_never_waits_to_turn_or_brake() {
        let creeping = Handling {
            turn_rate: 0.9,
            ..SHIP
        };
        let moving_away = ship(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(
            stage(&moving_away, &creeping, EAST, true, WHOLE, false),
            PreJump::Ready
        );
        assert_eq!(
            stage(&moving_away, &creeping, EAST, true, PlayerTurn::Full, false),
            PreJump::Brake
        );
    }

    #[test]
    fn a_moving_ship_brakes_unless_it_need_not_slow_down() {
        let moving = ship(Vec2::new(0.0, -3.0), 90.0);
        assert_eq!(
            stage(&moving, &SHIP, EAST, true, WHOLE, false),
            PreJump::Brake
        );
        assert_eq!(
            stage(&moving, &SHIP, EAST, false, WHOLE, false),
            PreJump::Ready,
            "facing it"
        );
        let away = ship(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(
            stage(&away, &SHIP, EAST, false, WHOLE, false),
            PreJump::Turn
        );
        let slow = ship(Vec2::new(1.9, -1.9), 90.0);
        assert_eq!(
            stage(&slow, &SHIP, EAST, true, WHOLE, false),
            PreJump::Ready,
            "slow enough"
        );
    }

    #[test]
    fn without_a_bearing_a_slow_ship_is_ready_whichever_way_it_faces() {
        assert_eq!(
            stage(&ship(Vec2::ZERO, 0.0), &SHIP, None, true, WHOLE, false),
            PreJump::Ready
        );
        let moving = ship(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(
            stage(&moving, &SHIP, None, true, WHOLE, false),
            PreJump::Brake
        );
    }

    #[test]
    fn a_ship_never_waits_in_a_stage_it_cannot_finish() {
        let rigid = Handling {
            turn_rate: 0.0,
            ..SHIP
        };
        let moving_away = ship(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(
            stage(&moving_away, &rigid, EAST, true, WHOLE, false),
            PreJump::Ready
        );
        let engineless = Handling { accel: 0.0, ..SHIP };
        assert_eq!(
            stage(&moving_away, &engineless, EAST, true, WHOLE, false),
            PreJump::Turn
        );
        let facing = ship(Vec2::new(0.0, -3.0), 90.0);
        assert_eq!(
            stage(&facing, &engineless, EAST, true, WHOLE, false),
            PreJump::Ready
        );
    }

    // Flying the stages.

    #[test]
    fn a_turning_ship_comes_within_a_turn_of_the_bearing_at_its_turn_rate_and_drifts() {
        let mut state = ship(Vec2::new(0.5, 0.0), 0.0);
        for k in 1..=29 {
            assert_eq!(
                stage(&state, &SHIP, EAST, true, WHOLE, false),
                PreJump::Turn,
                "tick {k}"
            );
            let before = state;
            assert!(
                !fly(&mut state, &SHIP, EAST, true, FlightRules::default(), None),
                "no thrust"
            );
            assert!(state.heading <= 90.0, "never overshoots: {state:?}");
            assert_eq!(
                state.heading,
                before.heading + 3.0,
                "clockwise, the short way"
            );
            assert_eq!(state.velocity, before.velocity, "coasting");
            assert_eq!(state.position, before.position + before.velocity);
        }
        assert_eq!(state.heading, 87.0, "a turn short");
        assert_eq!(
            stage(&state, &SHIP, EAST, true, WHOLE, false),
            PreJump::Ready
        );
    }

    #[test]
    fn a_turn_goes_the_short_way_round() {
        let mut state = ship(Vec2::ZERO, 180.0);
        fly(&mut state, &SHIP, EAST, true, FlightRules::default(), None);
        assert_eq!(state.heading, 177.0);
        let mut state = ship(Vec2::ZERO, 350.0);
        fly(
            &mut state,
            &SHIP,
            Some(10),
            true,
            FlightRules::default(),
            None,
        );
        assert_eq!(state.heading, 353.0);
    }

    #[test]
    fn a_ready_ship_does_not_move() {
        let ready = ship(Vec2::new(1.0, 1.0), 90.0);
        let mut state = ready;
        assert!(!fly(
            &mut state,
            &SHIP,
            EAST,
            true,
            FlightRules::default(),
            None
        ));
        assert_eq!(state, ready);
    }

    /// The braking thrust of a ship with `handling` moving up at 3 pixels a
    /// tick, facing `off` degrees clockwise of straight down (against its
    /// motion).
    fn brakes_at(handling: &Handling, off: f32) -> bool {
        let mut state = ship(Vec2::new(0.0, -3.0), 180.0 + off);
        fly(
            &mut state,
            handling,
            EAST,
            true,
            FlightRules::default(),
            None,
        )
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
            if stage(&state, &SHIP, EAST, true, WHOLE, false) != PreJump::Brake {
                break;
            }
            let off = shortest_turn(state.heading, 180.0).abs();
            let speed = state.velocity.length();
            let thrust = fly(&mut state, &SHIP, EAST, true, FlightRules::default(), None);
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
        assert_eq!(
            stage(&state, &SHIP, EAST, true, WHOLE, false),
            PreJump::Turn
        );
    }

    #[test]
    fn a_ship_that_need_not_slow_down_turns_while_it_coasts() {
        let mut state = ship(Vec2::new(0.0, -6.0), 0.0);
        assert!(!fly(
            &mut state,
            &SHIP,
            EAST,
            false,
            FlightRules::default(),
            None
        ));
        assert_eq!(state.heading, 3.0);
        assert_eq!(state.velocity, Vec2::new(0.0, -6.0));
    }

    // An inertialess ship.

    #[test]
    fn an_inertialess_ship_that_cannot_turn_still_brakes() {
        let rigid = Handling {
            turn_rate: 0.0,
            ..SHIP
        };
        let moving = ship(Vec2::new(0.0, -6.0), 0.0);
        assert_eq!(
            stage(&moving, &rigid, EAST, true, WHOLE, true),
            PreJump::Brake,
            "its brake needs no turn"
        );
        assert_eq!(
            stage(&moving, &rigid, EAST, true, WHOLE, false),
            PreJump::Ready,
            "an inertial one's does"
        );
        let engineless = Handling { accel: 0.0, ..SHIP };
        assert_eq!(
            stage(&moving, &engineless, EAST, true, WHOLE, true),
            PreJump::Turn,
            "nor can it brake with no thrust"
        );
    }

    #[test]
    fn an_inertialess_brake_takes_a_ticks_acceleration_off_the_speed_with_no_turn_or_thrust() {
        let mut state = ship(Vec2::new(0.0, -6.0), 0.0);
        let mut speed = 6.0;
        let thrust = fly(
            &mut state,
            &SHIP,
            EAST,
            true,
            FlightRules::default(),
            Some(&mut speed),
        );
        assert!(!thrust, "no engine thrust");
        assert_eq!(state.heading, 0.0, "no turn");
        assert!((speed - 5.7).abs() < 1e-6, "{speed}");
        assert!(
            (state.velocity - Vec2::new(0.0, -5.7)).length() < 1e-5,
            "the velocity follows: {state:?}"
        );
    }

    #[test]
    fn an_inertialess_ship_brakes_until_slow_enough_then_turns_its_velocity_with_it() {
        let mut state = ship(Vec2::new(0.0, -6.0), 0.0);
        let mut speed = 6.0;
        let mut ticks = 0;
        while stage(&state, &SHIP, EAST, true, WHOLE, true) == PreJump::Brake {
            fly(
                &mut state,
                &SHIP,
                EAST,
                true,
                FlightRules::default(),
                Some(&mut speed),
            );
            ticks += 1;
            assert!(ticks < 100, "{state:?}");
        }
        // 6 down to under 2 at 0.3 a tick: 14 ticks, at 1.8.
        assert_eq!(ticks, 14);
        assert!(
            (speed - 1.8).abs() < 1e-5,
            "slow enough, not stopped: {speed}"
        );
        assert_eq!(state.heading, 0.0);
        for _ in 0..40 {
            fly(
                &mut state,
                &SHIP,
                EAST,
                true,
                FlightRules::default(),
                Some(&mut speed),
            );
        }
        assert_eq!(
            stage(&state, &SHIP, EAST, true, WHOLE, true),
            PreJump::Ready
        );
        let along = crate::flight::facing(crate::speed_cap::whole_degree(state.heading)) * speed;
        assert!(
            (state.velocity - along).length() < 1e-5,
            "along its heading: {state:?}"
        );
    }
}
