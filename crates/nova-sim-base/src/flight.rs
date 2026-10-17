//! Flight: one tick of the player ship's Newtonian motion.
//!
//! Each [`step`]:
//!
//! 1. Turns. Left and right turn by the ship's turn rate. Otherwise reverse
//!    turns towards the heading opposite the ship's velocity, by at most the
//!    turn rate, landing on it exactly once it is within one tick's turn. A
//!    ship at rest has no velocity to turn from, so reverse does nothing;
//!    one no faster than [`FlightRules::at_rest_speed`] counts as at rest.
//! 2. Thrusts: thrust adds the ship's acceleration along its new heading to
//!    its velocity.
//! 3. Caps the speed at the ship's top speed, keeping the direction.
//!    `step` adds no drag, so an intact ship coasts for ever. A disabled or
//!    breaking-up ship is slowed by [`drag`] before its step, by its caller
//!    (the session for the player, traffic for NPCs), as the original slows
//!    a hull `_IsDisabled` reports (0x68abf-0x68af8, 0x33950-0x339a1) before
//!    that frame's position step.
//! 4. Moves the ship by its new velocity (semi-implicit Euler).
//!
//! A heading is in degrees: 0 faces up the screen (-y), and it grows
//! clockwise, kept in `[0, 360)`. Facing `h` is the unit vector
//! `(sin h, -cos h)`.

use core::num::NonZeroU32;

use crate::geometry::Vec2;
use crate::handling::{Handling, ORIGINAL_THRUST_DIVISOR};
use crate::rulebook::{RuleValue, Rulebook};

/// The speed, in pixels a tick, at or below which reverse treats a ship as
/// at rest under the original's thrust ([`at_rest_speed`] of
/// [`ORIGINAL_THRUST_DIVISOR`]).
pub const AT_REST_SPEED: f32 = 0.5 / ORIGINAL_THRUST_DIVISOR;

/// The speed, in pixels a tick, at or below which reverse treats a ship as
/// at rest when `Accel` is divided by `divisor`: half of one tick's thrust
/// from the weakest engine (`Accel` 1). Thrust never changes a speed by
/// less than that, so anything slower is the rounding left over from
/// braking, whose direction is meaningless. (Reverse's rest test becomes
/// the original's per-axis 0.05, 0xdd990, with
/// `ship-handling-fidelity/phase-4-turning-and-reverse`.)
#[must_use]
pub fn at_rest_speed(divisor: NonZeroU32) -> f32 {
    (0.5 / f64::from(divisor.get())) as f32
}

/// The rules a ship flies by, read from the [`Rulebook`] at each use: the
/// thrust's divisor ([`RuleValue::ThrustDivisor`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightRules {
    /// What a ship's `Accel` total is divided by to give its thrust.
    pub thrust_divisor: NonZeroU32,
}

impl FlightRules {
    /// The flight rules `rules` sets.
    #[must_use]
    pub fn from_rulebook(rules: &Rulebook) -> Self {
        Self {
            thrust_divisor: rules.value_for(RuleValue::ThrustDivisor),
        }
    }

    /// `handling` as these rules fly it: its thrust under the divisor
    /// ([`Handling::thrust_under`]).
    #[must_use]
    pub fn handling(self, handling: Handling) -> Handling {
        handling.thrust_under(self.thrust_divisor)
    }

    /// The speed reverse treats as at rest under these rules
    /// ([`at_rest_speed`]).
    #[must_use]
    pub fn at_rest_speed(self) -> f32 {
        at_rest_speed(self.thrust_divisor)
    }
}

/// The original's flight rules: those of [`Rulebook::default`].
impl Default for FlightRules {
    fn default() -> Self {
        Self::from_rulebook(&Rulebook::default())
    }
}

/// The factor a disabled or breaking-up hull's velocity is multiplied by each
/// tick: the double at 0xdd5f0 (bytes `d7 a3 70 3d 0a d7 ef 3f`), whose only
/// two loads are `_HandlePlayer` @0x68ad1 and `_HandleShip` @0x33960.
pub const DISABLED_DRAG: f32 = 0.995;

/// Slows `state` by one tick of [`DISABLED_DRAG`], leaving its position and
/// heading. Callers decide whether the ship is disabled.
pub fn drag(state: &mut ShipState) {
    state.velocity = state.velocity * DISABLED_DRAG;
}

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
}

/// Advances `state` one tick under `controls`, flying as `handling` allows
/// under `rules`.
pub fn step(state: &mut ShipState, handling: &Handling, controls: Controls, rules: FlightRules) {
    let rate = handling.turn_rate;
    state.heading = match controls.turn {
        Turn::Left => normalized(state.heading - rate),
        Turn::Right => normalized(state.heading + rate),
        Turn::None if controls.reverse && state.velocity.length() > rules.at_rest_speed() => {
            turn_toward(state.heading, heading_of(state.velocity * -1.0), rate)
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

/// Heading `from` turned towards heading `to` the short way, by at most
/// `rate` degrees: exactly `to` once it is within one turn.
#[must_use]
pub fn turn_toward(from: f32, to: f32, rate: f32) -> f32 {
    let off = shortest_turn(from, to);
    if off.abs() <= rate {
        to
    } else {
        normalized(rate.copysign(off) + from)
    }
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
    use crate::rulebook::{RuleValue, Rulebook};
    use core::num::NonZeroU32;

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
            step(&mut state, &SHIP, controls, FlightRules::default());
        }
        state
    }

    fn under(divisor: u32) -> FlightRules {
        FlightRules::from_rulebook(
            &Rulebook::default()
                .with_value(RuleValue::ThrustDivisor, NonZeroU32::new(divisor).unwrap()),
        )
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
    fn turn_toward_turns_by_the_rate_the_short_way_and_lands_on_the_target() {
        assert_eq!(turn_toward(0.0, 90.0, 3.0), 3.0, "clockwise");
        assert_eq!(turn_toward(90.0, 0.0, 3.0), 87.0, "anticlockwise");
        assert_eq!(turn_toward(350.0, 10.0, 3.0), 353.0, "clockwise across 0");
        assert_eq!(turn_toward(10.0, 350.0, 3.0), 7.0, "anticlockwise");
        assert_eq!(turn_toward(1.0, 350.0, 3.0), 358.0, "wrapping below 0");
        assert_eq!(turn_toward(88.5, 90.0, 3.0), 90.0, "within one turn");
        assert_eq!(turn_toward(87.0, 90.0, 3.0), 90.0, "exactly one turn");
        assert_eq!(turn_toward(358.0, 0.0, 3.0), 0.0, "onto 0");
        assert_eq!(turn_toward(90.0, 90.0, 3.0), 90.0, "already there");
        assert_eq!(turn_toward(0.0, 90.0, 0.0), 0.0, "cannot turn");
    }

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
        step(
            &mut state,
            &creep,
            turning(Turn::Left),
            FlightRules::default(),
        );
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
    fn drag_scales_the_velocity_by_0_995_and_leaves_position_and_heading() {
        let mut state = ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(1.0, -2.0),
            heading: 45.0,
        };
        drag(&mut state);
        assert_eq!(state.velocity, Vec2::new(0.995, -1.99));
        assert_eq!(state.position, Vec2::new(10.0, 20.0));
        assert_eq!(state.heading, 45.0);
    }

    #[test]
    fn a_ship_coasts_without_drag() {
        let state = ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(1.0, -2.0),
            heading: 45.0,
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
            step(
                &mut state,
                &at_top,
                Controls::default(),
                FlightRules::default(),
            );
            assert_eq!(state.velocity, velocity, "not rescaled");
        }
    }

    #[test]
    fn speed_is_capped_at_the_top_speed_and_reaches_it_exactly() {
        let mut state = heading(0.0);
        for _ in 0..100 {
            step(&mut state, &SHIP, THRUST, FlightRules::default());
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
            step(&mut state, &SHIP, carving, FlightRules::default());
            let speed = state.velocity.length();
            assert!((speed - SHIP.max_speed).abs() < 1e-5, "{speed}: {state:?}");
        }
        assert!(state.velocity.x > 0.0, "swinging right: {state:?}");
        // Thrusting further round slows it, and nothing speeds it past the
        // cap.
        for _ in 0..60 {
            step(&mut state, &SHIP, carving, FlightRules::default());
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
            FlightRules::default(),
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
            step(&mut state, &SHIP, REVERSE, FlightRules::default());
        }
        assert_eq!(state.heading, 180.0);
        let brake = Controls {
            thrust: true,
            ..REVERSE
        };
        // Facing back, each tick of thrust takes 0.1 off the speed.
        for _ in 0..30 {
            step(&mut state, &SHIP, brake, FlightRules::default());
            assert!((state.heading - 180.0).abs() < 1e-3, "{state:?}");
        }
        assert!(state.velocity.length() < 1e-4, "{state:?}");
    }

    #[test]
    fn reverse_counts_a_ship_at_or_below_the_rest_speed_as_at_rest() {
        let creeping = moving(Vec2::new(0.0, AT_REST_SPEED), 90.0);
        assert_eq!(after(creeping, REVERSE, 1).heading, 90.0);
        // One tick's thrust from the weakest engine is motion.
        let weakest = Handling::from_totals(300, 1, 10.0);
        let nudged = moving(Vec2::new(0.0, weakest.accel), 90.0);
        assert_eq!(after(nudged, REVERSE, 1).heading, 87.0, "towards 0");
    }

    #[test]
    fn the_rest_speed_is_half_the_weakest_engines_thrust_under_the_divisor() {
        assert_eq!(FlightRules::default().at_rest_speed(), AT_REST_SPEED);
        assert_eq!(AT_REST_SPEED, 0.5 / 5000.0);
        for by in [1, 3000, 7919, u32::MAX] {
            let rules = under(by);
            let weakest = rules.handling(Handling::from_totals(300, 1, 10.0));
            let half = rules.at_rest_speed();
            assert!(
                ((f64::from(half) * 2.0 - f64::from(weakest.accel)) / f64::from(weakest.accel))
                    .abs()
                    < 1e-6,
                "{by}: {half} against {weakest:?}"
            );
        }
        // Under 3000 a ship creeping at 0.5 / 3000 is at rest; under the
        // original's 5000 it moves.
        let creeping = moving(Vec2::new(0.0, under(3000).at_rest_speed()), 90.0);
        let mut slow = creeping;
        step(&mut slow, &SHIP, REVERSE, under(3000));
        assert_eq!(slow.heading, 90.0, "at rest under 3000");
        let mut fast = creeping;
        step(&mut fast, &SHIP, REVERSE, FlightRules::default());
        assert_eq!(fast.heading, 87.0, "moving under 5000");
    }

    #[test]
    fn the_flight_rules_take_the_rulebooks_divisor() {
        assert_eq!(
            FlightRules::default(),
            FlightRules::from_rulebook(&Rulebook::default())
        );
        assert_eq!(
            FlightRules::default().thrust_divisor,
            RuleValue::ThrustDivisor.default_value()
        );
        assert_eq!(under(3000).thrust_divisor.get(), 3000);
        let own = Handling::from_totals(300, 300, 10.0);
        assert_eq!(FlightRules::default().handling(own), own);
        assert_eq!(
            under(3000).handling(own),
            own.thrust_under(NonZeroU32::new(3000).unwrap())
        );
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
            step(&mut state, &SHIP, REVERSE, FlightRules::default());
        }
        for _ in 0..30 {
            step(&mut state, &SHIP, brake, FlightRules::default());
        }
        assert!(state.velocity != Vec2::ZERO, "leftover: {state:?}");
        let stopped = state.heading;
        let held = after(state, REVERSE, 10);
        assert_eq!(held.heading, stopped, "{held:?}");
    }
}
