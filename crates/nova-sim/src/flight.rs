//! Flight: one tick of the player ship's Newtonian motion.
//!
//! Each [`step`]:
//!
//! 1. Turns. Left and right turn by the ship's turn rate. Otherwise reverse
//!    turns towards the heading opposite the ship's velocity, by at most the
//!    turn rate, landing on it exactly once it is within one tick's turn. A
//!    ship at rest has no velocity to turn from, so reverse does nothing;
//!    one slower than [`AT_REST_SPEED`] counts as at rest.
//! 2. Thrusts: thrust adds the ship's acceleration along its new heading to
//!    its velocity.
//! 3. Caps the speed at the ship's top speed, keeping the direction. There
//!    is no drag: a ship coasts for ever.
//! 4. Moves the ship by its new velocity (semi-implicit Euler).
//!
//! A heading is in degrees: 0 faces up the screen (-y), and it grows
//! clockwise, kept in `[0, 360)`. Facing `h` is the unit vector
//! `(sin h, -cos h)`.

use crate::geometry::Vec2;
use crate::handling::{ACCEL_PER_PIXEL_PER_TICK_SQUARED, Handling};
use crate::reserves::Reserves;

/// The speed, in pixels a tick, at or below which reverse treats a ship as
/// at rest: half of one tick's thrust from the weakest engine (`Accel` 1).
/// Thrust never changes a speed by less than that, so anything slower is the
/// rounding left over from braking, whose direction is meaningless.
pub const AT_REST_SPEED: f32 = 0.5 / ACCEL_PER_PIXEL_PER_TICK_SQUARED;

/// Which way the turn keys turn the ship.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Turn {
    /// Neither, or both.
    #[default]
    None,
    /// Anticlockwise.
    Left,
    /// Clockwise.
    Right,
}

/// The player's controls for one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Controls {
    /// Accelerate along the heading.
    pub thrust: bool,
    /// Turn left or right; takes precedence over `reverse`.
    pub turn: Turn,
    /// Turn to face against the velocity.
    pub reverse: bool,
}

/// A ship in flight.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShipState {
    /// Where it is, in pixels from the system's centre.
    pub position: Vec2,
    /// How far it moves each tick, in pixels.
    pub velocity: Vec2,
    /// Which way it faces, in degrees clockwise from up, in `[0, 360)`.
    pub heading: f32,
    /// Its shield, armour and fuel; flight leaves them as they are.
    pub reserves: Reserves,
}

/// Advances `state` one tick under `controls`, flying as `handling` allows.
pub fn step(state: &mut ShipState, handling: &Handling, controls: Controls) {
    let rate = handling.turn_rate;
    state.heading = match controls.turn {
        Turn::Left => normalized(state.heading - rate),
        Turn::Right => normalized(state.heading + rate),
        Turn::None if controls.reverse && state.velocity.length() > AT_REST_SPEED => {
            let behind = heading_of(state.velocity * -1.0);
            let off = shortest_turn(state.heading, behind);
            if off.abs() <= rate {
                behind
            } else {
                normalized(rate.copysign(off) + state.heading)
            }
        }
        Turn::None => state.heading,
    };
    if controls.thrust {
        state.velocity = state.velocity + facing(state.heading) * handling.accel;
    }
    let speed = state.velocity.length();
    if speed > handling.max_speed {
        // Divided first, so a velocity along an axis lands on the top speed
        // exactly.
        let direction = Vec2::new(state.velocity.x / speed, state.velocity.y / speed);
        state.velocity = direction * handling.max_speed;
    }
    state.position = state.position + state.velocity;
}

/// The unit vector a ship heading `heading` degrees faces.
#[must_use]
pub fn facing(heading: f32) -> Vec2 {
    let (sin, cos) = heading.to_radians().sin_cos();
    Vec2::new(sin, -cos)
}

/// The heading that faces along `v`, which must not be zero.
#[must_use]
pub fn heading_of(v: Vec2) -> f32 {
    normalized(v.x.atan2(-v.y).to_degrees())
}

/// The signed turn, in degrees in `(-180, 180]`, that takes heading `from`
/// to heading `to` the short way: positive is clockwise.
#[must_use]
pub fn shortest_turn(from: f32, to: f32) -> f32 {
    let clockwise = normalized(to - from);
    if clockwise > 180.0 {
        clockwise - 360.0
    } else {
        clockwise
    }
}

/// `heading` brought into `[0, 360)`.
#[must_use]
pub fn normalized(heading: f32) -> f32 {
    let wrapped = heading.rem_euclid(360.0);
    // A tiny negative heading wraps to 360 itself in f32.
    if wrapped >= 360.0 { 0.0 } else { wrapped }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// An average ship: 3 pixels a tick at most, 0.1 more each tick, 3° a
    /// tick.
    const SHIP: Handling = Handling {
        max_speed: 3.0,
        accel: 0.1,
        turn_rate: 3.0,
    };

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };

    fn turning(turn: Turn) -> Controls {
        Controls {
            turn,
            ..Controls::default()
        }
    }

    const REVERSE: Controls = Controls {
        thrust: false,
        turn: Turn::None,
        reverse: true,
    };

    fn heading(heading: f32) -> ShipState {
        ShipState {
            heading,
            ..ShipState::default()
        }
    }

    fn moving(velocity: Vec2, heading: f32) -> ShipState {
        ShipState {
            velocity,
            heading,
            ..ShipState::default()
        }
    }

    fn after(mut state: ShipState, controls: Controls, ticks: usize) -> ShipState {
        for _ in 0..ticks {
            step(&mut state, &SHIP, controls);
        }
        state
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-5
    }

    // Headings.

    #[test]
    fn headings_normalise_into_0_to_360() {
        for (raw, expected) in [
            (0.0, 0.0),
            (359.5, 359.5),
            (360.0, 0.0),
            (725.0, 5.0),
            (-90.0, 270.0),
            (-720.0, 0.0),
        ] {
            assert_eq!(normalized(raw), expected, "{raw}");
        }
        assert_eq!(normalized(-1e-6), 0.0, "rounds to 360, which wraps");
    }

    #[test]
    fn the_shortest_turn_is_signed_clockwise_and_at_most_half_a_circle() {
        for (from, to, expected) in [
            (0.0, 90.0, 90.0),
            (90.0, 0.0, -90.0),
            (350.0, 10.0, 20.0),
            (10.0, 350.0, -20.0),
            (0.0, 180.0, 180.0),
            (180.0, 0.0, 180.0),
            (30.0, 30.0, 0.0),
        ] {
            assert_eq!(shortest_turn(from, to), expected, "{from} to {to}");
        }
    }

    #[test]
    fn facing_and_heading_of_are_inverse() {
        assert_eq!(facing(0.0), Vec2::new(0.0, -1.0));
        assert!(near(facing(90.0), Vec2::new(1.0, 0.0)));
        assert_eq!(heading_of(Vec2::new(0.0, -5.0)), 0.0);
        assert_eq!(heading_of(Vec2::new(5.0, 0.0)), 90.0);
        assert_eq!(heading_of(Vec2::new(0.0, 5.0)), 180.0);
        assert_eq!(heading_of(Vec2::new(-5.0, 0.0)), 270.0);
        for h in [10.0, 135.0, 200.0, 359.0] {
            assert!((heading_of(facing(h)) - h).abs() < 1e-3, "{h}");
        }
    }

    // Turning.

    #[test]
    fn right_turns_clockwise_and_left_anticlockwise_at_the_turn_rate() {
        assert_eq!(after(heading(90.0), turning(Turn::Right), 1).heading, 93.0);
        assert_eq!(
            after(heading(90.0), turning(Turn::Right), 10).heading,
            120.0
        );
        assert_eq!(after(heading(90.0), turning(Turn::Left), 1).heading, 87.0);
        assert_eq!(after(heading(90.0), turning(Turn::Left), 10).heading, 60.0);
        assert_eq!(after(heading(90.0), Controls::default(), 10).heading, 90.0);
    }

    #[test]
    fn headings_wrap_at_0_and_360() {
        assert_eq!(after(heading(359.0), turning(Turn::Right), 1).heading, 2.0);
        assert_eq!(after(heading(0.0), turning(Turn::Left), 1).heading, 357.0);
        assert_eq!(after(heading(357.0), turning(Turn::Right), 1).heading, 0.0);
        assert_eq!(after(heading(0.0), turning(Turn::Right), 120).heading, 0.0);
    }

    #[test]
    fn a_tiny_left_turn_from_0_stays_below_360() {
        let mut state = heading(0.0);
        let creep = Handling {
            turn_rate: 1e-6,
            ..SHIP
        };
        step(&mut state, &creep, turning(Turn::Left));
        assert!((0.0..360.0).contains(&state.heading), "{state:?}");
    }

    #[test]
    fn turning_alone_does_not_move_the_ship() {
        let state = after(heading(0.0), turning(Turn::Right), 10);
        assert_eq!((state.position, state.velocity), (Vec2::ZERO, Vec2::ZERO));
    }

    // Thrust.

    #[test]
    fn thrust_accelerates_along_the_heading_and_then_moves() {
        // Heading 0 faces up the screen.
        let once = after(heading(0.0), THRUST, 1);
        assert_eq!(once.velocity, Vec2::new(0.0, -0.1));
        assert_eq!(once.position, Vec2::new(0.0, -0.1), "the new velocity");
        let twice = after(once, THRUST, 1);
        assert_eq!(twice.velocity, Vec2::new(0.0, -0.2));
        assert!(near(twice.position, Vec2::new(0.0, -0.3)), "{twice:?}");
        // 90 faces right; 180 down; 270 left.
        for (h, way) in [
            (90.0, Vec2::new(0.1, 0.0)),
            (180.0, Vec2::new(0.0, 0.1)),
            (270.0, Vec2::new(-0.1, 0.0)),
        ] {
            let state = after(heading(h), THRUST, 1);
            assert!(near(state.velocity, way), "{h}: {state:?}");
            assert_eq!(state.heading, h);
        }
    }

    #[test]
    fn a_ship_coasts_without_drag() {
        let state = ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(1.0, -2.0),
            heading: 45.0,
            ..ShipState::default()
        };
        let coasted = after(state, Controls::default(), 10);
        assert_eq!(coasted.velocity, state.velocity);
        assert_eq!(coasted.position, Vec2::new(20.0, 0.0));
        assert_eq!(coasted.heading, 45.0);
    }

    #[test]
    fn a_ship_coasting_at_exactly_its_top_speed_keeps_its_velocity() {
        // Off the axes, rescaling a velocity to its own length can move it
        // by a rounding step; one exactly at the top speed is left alone.
        for velocity in [
            Vec2::new(3.0, -4.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.1, 0.7),
            Vec2::new(1.3, -2.9),
            Vec2::new(-2.2, 0.35),
        ] {
            let at_top = Handling {
                max_speed: velocity.length(),
                ..SHIP
            };
            let mut state = moving(velocity, 0.0);
            step(&mut state, &at_top, Controls::default());
            assert_eq!(state.velocity, velocity, "not rescaled");
        }
    }

    #[test]
    fn speed_is_capped_at_the_top_speed_and_reaches_it_exactly() {
        let mut state = heading(0.0);
        for _ in 0..100 {
            step(&mut state, &SHIP, THRUST);
            assert!(state.velocity.length() <= SHIP.max_speed, "{state:?}");
        }
        assert_eq!(state.velocity, Vec2::new(0.0, -3.0));
    }

    #[test]
    fn turning_at_top_speed_swings_the_velocity_round_and_never_exceeds_it() {
        let mut state = after(heading(0.0), THRUST, 40);
        let carving = Controls {
            thrust: true,
            turn: Turn::Right,
            reverse: false,
        };
        // While the heading is within a few turns of the velocity, thrust
        // holds the ship at its top speed.
        for _ in 0..10 {
            step(&mut state, &SHIP, carving);
            let speed = state.velocity.length();
            assert!((speed - SHIP.max_speed).abs() < 1e-5, "{speed}: {state:?}");
        }
        assert!(state.velocity.x > 0.0, "swinging right: {state:?}");
        // Thrusting further round slows it, and nothing speeds it past the
        // cap.
        for _ in 0..60 {
            step(&mut state, &SHIP, carving);
            assert!(
                state.velocity.length() <= SHIP.max_speed + 1e-5,
                "{state:?}"
            );
        }
        assert!(state.velocity.length() < SHIP.max_speed - 0.1, "{state:?}");
    }

    #[test]
    fn a_turn_comes_before_the_thrust_in_the_same_tick() {
        let quarter_turn = Handling {
            turn_rate: 90.0,
            ..SHIP
        };
        let mut state = heading(0.0);
        step(
            &mut state,
            &quarter_turn,
            Controls {
                thrust: true,
                turn: Turn::Right,
                reverse: false,
            },
        );
        assert!(near(state.velocity, Vec2::new(0.1, 0.0)), "{state:?}");
    }

    // Reverse.

    #[test]
    fn reverse_turns_towards_the_heading_against_the_velocity() {
        // Moving up (heading 0); behind is 180, as far one way as the other,
        // and the turn goes clockwise.
        let up = moving(Vec2::new(0.0, -3.0), 0.0);
        assert_eq!(after(up, REVERSE, 1).heading, 3.0);
        assert_eq!(after(up, REVERSE, 59).heading, 177.0);
        assert_eq!(after(up, REVERSE, 60).heading, 180.0);
        assert_eq!(after(up, REVERSE, 90).heading, 180.0, "and stays there");
        // Moving left, facing up: behind is 90, clockwise.
        let left = moving(Vec2::new(-2.0, 0.0), 0.0);
        assert_eq!(after(left, REVERSE, 1).heading, 3.0);
        // Moving right, facing up: behind is 270, anticlockwise.
        let right = moving(Vec2::new(2.0, 0.0), 0.0);
        assert_eq!(after(right, REVERSE, 1).heading, 357.0);
        assert_eq!(after(right, REVERSE, 30).heading, 270.0);
        // Reverse alone neither thrusts nor brakes.
        assert_eq!(after(right, REVERSE, 30).velocity, Vec2::new(2.0, 0.0));
    }

    #[test]
    fn reverse_lands_on_the_heading_behind_without_overshooting() {
        // Moving down and right at 45°: behind is 315.
        let state = moving(Vec2::new(1.0, 1.0), 313.5);
        let reversed = after(state, REVERSE, 1);
        assert!((reversed.heading - 315.0).abs() < 1e-4, "{reversed:?}");
        let again = after(reversed, REVERSE, 1);
        assert_eq!(again.heading, reversed.heading, "no oscillation");
        // Exactly one tick's turn away lands on it too.
        let one_turn = moving(Vec2::new(0.0, -1.0), 177.0);
        assert_eq!(after(one_turn, REVERSE, 1).heading, 180.0);
    }

    #[test]
    fn reverse_across_0_turns_the_short_way() {
        // Moving down, facing just left of up: behind is 0, a short turn
        // clockwise across 360.
        let state = moving(Vec2::new(0.0, 1.0), 350.0);
        assert_eq!(after(state, REVERSE, 1).heading, 353.0);
        let landed = after(state, REVERSE, 4).heading;
        assert!(landed < 1e-4 || landed > 360.0 - 1e-4, "{landed}");
    }

    #[test]
    fn reverse_at_rest_does_nothing() {
        let state = heading(45.0);
        assert_eq!(after(state, REVERSE, 5), state);
    }

    #[test]
    fn left_or_right_overrides_reverse() {
        let up = moving(Vec2::new(0.0, -3.0), 0.0);
        let left = Controls {
            turn: Turn::Left,
            ..REVERSE
        };
        assert_eq!(after(up, left, 1).heading, 357.0);
        let right = Controls {
            turn: Turn::Right,
            ..REVERSE
        };
        let moving_right = moving(Vec2::new(2.0, 0.0), 0.0);
        assert_eq!(after(moving_right, right, 1).heading, 3.0);
    }

    #[test]
    fn reverse_then_thrust_brakes_the_ship_to_a_stop() {
        let mut state = moving(Vec2::new(0.0, -3.0), 0.0);
        for _ in 0..60 {
            step(&mut state, &SHIP, REVERSE);
        }
        assert_eq!(state.heading, 180.0);
        let brake = Controls {
            thrust: true,
            ..REVERSE
        };
        // Facing back, each tick of thrust takes 0.1 off the speed.
        for _ in 0..30 {
            step(&mut state, &SHIP, brake);
            assert!((state.heading - 180.0).abs() < 1e-3, "{state:?}");
        }
        assert!(state.velocity.length() < 1e-4, "{state:?}");
    }

    #[test]
    fn reverse_counts_a_ship_at_or_below_the_rest_speed_as_at_rest() {
        let creeping = moving(Vec2::new(0.0, AT_REST_SPEED), 90.0);
        assert_eq!(after(creeping, REVERSE, 1).heading, 90.0);
        // One tick's thrust from the weakest engine is motion.
        let weakest = Handling::from_fields(crate::handling::ShipFields {
            speed: 300,
            accel: 1,
            maneuver: 10,
            ..crate::handling::ShipFields::default()
        });
        let nudged = moving(Vec2::new(0.0, weakest.accel), 90.0);
        assert_eq!(after(nudged, REVERSE, 1).heading, 87.0, "towards 0");
    }

    #[test]
    fn reverse_after_braking_to_a_stop_leaves_the_heading_alone() {
        // Braking leaves a rounding error's worth of velocity, pointing
        // anywhere; reverse treats that as at rest.
        let mut state = moving(Vec2::new(0.0, -3.0), 0.0);
        let brake = Controls {
            thrust: true,
            ..REVERSE
        };
        for _ in 0..60 {
            step(&mut state, &SHIP, REVERSE);
        }
        for _ in 0..30 {
            step(&mut state, &SHIP, brake);
        }
        assert!(state.velocity != Vec2::ZERO, "leftover: {state:?}");
        let stopped = state.heading;
        let held = after(state, REVERSE, 10);
        assert_eq!(held.heading, stopped, "{held:?}");
    }
}
