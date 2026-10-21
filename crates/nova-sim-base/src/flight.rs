//! Flight: one tick of a ship's Newtonian motion, the player's or an
//! NPC's ([`Flyer`]).
//!
//! An NPC's tick is a [`step`]; the player's is a [`player_step`], which
//! turns as `_HandlePlayer`'s turn block does (see the
//! [`player_turn`](crate::player_turn) module) and then moves as a step
//! does from step 2 on ([`advance`]). Each [`step`]:
//!
//! 1. Turns. Left and right turn by the ship's turn rate, the full rate
//!    (`_HandleShip`). Otherwise reverse turns towards the heading
//!    opposite the ship's velocity, by at most the turn rate, landing on
//!    it exactly once it is within one tick's turn. A ship at rest has no
//!    velocity to turn from, so reverse does nothing; one no faster than
//!    [`FlightRules::at_rest_speed`] counts as at rest. (The original's
//!    NPCs have no reverse: the AI turns them, and
//!    rdm task `brake-leaves-sub-thrust-drift` owns their brake.)
//! 2. Thrusts: thrust adds the ship's acceleration along its new heading to
//!    its velocity. By default ([`SpeedCap::PerAxis`], the original's), as
//!    `_AdjustedAccel` adds it ([`adjusted_accel`], @0x2ae4b-0x2af43): on
//!    each axis only while that axis is short of the top speed's share
//!    along the heading, and along the heading's whole degree, as the
//!    original passes it (@0x6cd16, @0x34b9d). Under [`SpeedCap::Circle`]
//!    it is added whole, along the heading itself. An NPC thrusts toward
//!    its top speed; the player toward its [`Caps`]' thrust top, 1.8
//!    times that while its afterburner is on.
//! 3. Caps the speed. By default only the player's: its velocity is
//!    clamped on each axis to its box, `_playerMaxVel`, which
//!    `_HandlePlayer` clamps to before the move (@0x6d43f-0x6d4aa,
//!    [`speed_limit`]) and writes only after it, so the clamp reads the
//!    box the last tick left: the top speed once settled, wider while
//!    the afterburner is on, bleeding back after it (see the
//!    [`afterburner`](crate::afterburner) module), and none on a run's
//!    first tick. An NPC's isn't clamped (`_HandleShip` has no clamp), so
//!    nothing but its own thrust slows a ship past its top speed, and one
//!    at top speed that turns a quarter and thrusts ends near 1.41 times
//!    as fast. Under [`SpeedCap::Circle`] every ship's speed is rescaled
//!    to its top speed, keeping the direction ([`circle`]), the player's
//!    to the box's half-width.
//!    `step` adds no drag, so an intact ship coasts for ever. A disabled or
//!    breaking-up ship is slowed by [`drag`] before its step, by its caller
//!    (the session for the player, traffic for NPCs), as the original slows
//!    a hull `_IsDisabled` reports (0x68abf-0x68af8, 0x33950-0x339a1) before
//!    that frame's position step.
//! 4. Moves the ship by its new velocity (semi-implicit Euler).
//!
//! An inertialess ship (see [`inertialess`](crate::inertialess)) holds a
//! speed along its heading instead, which its owner keeps (the session
//! for the player, traffic for an NPC). Its tick is an
//! [`inertialess_step`] or [`inertialess_player_step`]: it turns as
//! above, but reverse never turns it (an NPC's turns by its keys alone,
//! and the player's reverse gives no aim). Thrust and reverse move the
//! speed, not the velocity, up to the top speed; the player's speed is
//! then capped at its box (@0x6d356). Then the velocity is pulled toward
//! heading x speed on each axis, capped as in step 3, and the ship moves
//! ([`inertialess_advance`]).
//!
//! A heading is in degrees: 0 faces up the screen (-y), and it grows
//! clockwise, kept in `[0, 360)`. Facing `h` is the unit vector
//! `(sin h, -cos h)`.

use core::num::NonZeroU32;

use crate::geometry::Vec2;
use crate::handling::{Handling, ORIGINAL_THRUST_DIVISOR};
use crate::inertialess::{
    ADJUST_RATE, BRAKE, InertialDampener, REVERSE, THRUST, adjust_velocity, step_speed,
};
use crate::player_turn::{PlayerTurn, turn};
use crate::rulebook::{RuleKey, RuleSource, RuleValue, Rulebook};
use crate::speed_cap::{SpeedCap, adjusted_accel, circle, speed_limit};

/// The speed, in pixels a tick, at or below which reverse treats a ship as
/// at rest under the original's thrust ([`at_rest_speed`] of
/// [`ORIGINAL_THRUST_DIVISOR`]).
pub const AT_REST_SPEED: f32 = 0.5 / ORIGINAL_THRUST_DIVISOR;

/// The speed, in pixels a tick, at or below which an NPC's reverse
/// ([`step`]) treats a ship as at rest when `Accel` is divided by
/// `divisor`: half of one tick's thrust from the weakest engine (`Accel`
/// 1). Thrust never changes a speed by less than that, so anything slower
/// is the rounding left over from braking, whose direction is
/// meaningless. The player's reverse tests each axis against the
/// original's 0.05 instead
/// ([`reverse_aim`](crate::player_turn::reverse_aim)).
#[must_use]
pub fn at_rest_speed(divisor: NonZeroU32) -> f32 {
    (0.5 / f64::from(divisor.get())) as f32
}

/// The rules a ship flies by, read from the [`Rulebook`] at each use: the
/// thrust's divisor ([`RuleValue::ThrustDivisor`]), the speed cap
/// ([`RuleKey::SpeedCap`]), how the player turns
/// ([`RuleKey::PlayerTurn`]), whose inertial dampener counts
/// ([`RuleKey::InertialDampener`]) and how an escort keeps formation
/// ([`RuleKey::FormationFlight`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightRules {
    /// What a ship's `Accel` total is divided by to give its thrust.
    pub thrust_divisor: NonZeroU32,
    /// How a ship's speed is capped.
    pub speed_cap: SpeedCap,
    /// How the player's ship turns.
    pub player_turn: PlayerTurn,
    /// Whose `ModType` 38 outfits make a ship inertialess.
    pub inertial_dampener: InertialDampener,
    /// How an escort keeps formation, which its flight and whether it
    /// flies inertialess both read.
    pub formation_flight: FormationFlight,
}

/// How an escort keeps formation, by [`RuleKey::FormationFlight`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FormationFlight {
    /// The original's goal 0xa and its substates, measured from the
    /// leader.
    #[default]
    Original,
    /// The port's earlier formation, measured from the slot.
    Earlier,
}

impl From<RuleSource> for FormationFlight {
    /// The engine's reading is the original's flight; the other, the
    /// port's earlier one.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Original,
            RuleSource::Bible => Self::Earlier,
        }
    }
}

impl FlightRules {
    /// The flight rules `rules` sets.
    #[must_use]
    pub fn from_rulebook(rules: &Rulebook) -> Self {
        Self {
            thrust_divisor: rules.value_for(RuleValue::ThrustDivisor),
            speed_cap: SpeedCap::from(rules.source_for(RuleKey::SpeedCap)),
            player_turn: PlayerTurn::from(rules.source_for(RuleKey::PlayerTurn)),
            inertial_dampener: InertialDampener::from(rules.source_for(RuleKey::InertialDampener)),
            formation_flight: FormationFlight::from(rules.source_for(RuleKey::FormationFlight)),
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

/// `velocity` × `by` a component, each product worked in f64 and stored
/// as an f32, as the jump's drags are: the player's brake
/// (`_HandlePlayer` @0x6b832-0x6b85c) and warp stage (@0x6bb32-0x6bb59),
/// and an NPC's warp-out (`_HandleShip` @0x34762-0x3478c) and a follow's
/// stage start (`_LowLevelAIHandler` @0x85c10-0x85c42). Each caller
/// passes its own factor.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn drag_by(velocity: Vec2, by: f64) -> Vec2 {
    let scale = |component: f32| (f64::from(component) * by) as f32;
    Vec2::new(scale(velocity.x), scale(velocity.y))
}

/// What a tick caps a ship's motion at: the top speed its thrust pushes
/// toward, and the box (or circle) its velocity is held to. An NPC's are
/// both its top speed ([`Caps::top`]). The player's come from its
/// afterburner (see the [`afterburner`](crate::afterburner) module): the
/// thrust's top is 1.8 times the top speed while the afterburner is on
/// (@0x6cc43-0x6cc80), and the box is `_playerMaxVel`, the box the last
/// tick left (@0x6d43f-0x6d4aa), which also caps an inertialess player's
/// speed (@0x6d356).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caps {
    /// The top speed thrust pushes toward (`_AdjustedAccel`'s).
    pub thrust_top: f32,
    /// The box the velocity is clamped to on each axis, or the circle's
    /// radius under [`SpeedCap::Circle`], and an inertialess player's
    /// speed's cap.
    pub max_vel: f32,
}

impl Caps {
    /// Both caps at `max_speed`: an NPC's, or the player's with the
    /// afterburner off and its box settled.
    #[must_use]
    pub fn top(max_speed: f32) -> Self {
        Self {
            thrust_top: max_speed,
            max_vel: max_speed,
        }
    }
}

/// Whose ship flies: the player's, which `_HandlePlayer` clamps to its
/// top speed on each axis, or an NPC's, which `_HandleShip` doesn't.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flyer {
    /// The player's ship.
    Player,
    /// An NPC.
    Npc,
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

/// The face-target key (the original's `autopilot`, keyPrefs +0xe), as
/// held this tick, and whether with Option: what the player's ship turns
/// to face (see `nova_sim::Session::tick`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Face {
    /// Not held.
    #[default]
    Off,
    /// Held alone: the ship target, or the nav stellar with none.
    Target,
    /// Held with Option: the nav stellar, or the ship target with none.
    Stellar,
}

/// The player's controls for one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Controls {
    /// Accelerate along the heading.
    pub thrust: bool,
    /// Turn left or right. For an NPC ([`step`]) it takes precedence over
    /// `reverse`; the player's are held off while reverse or the face key
    /// aims ([`player_turn`](crate::player_turn)).
    pub turn: Turn,
    /// Turn to face against the velocity: in an NPC's [`step`], and in the
    /// session's aim for the player's
    /// ([`reverse_aim`](crate::player_turn::reverse_aim)).
    pub reverse: bool,
    /// The face-target key, which the session aims the player's ship by.
    pub face: Face,
    /// The afterburner key (keyPrefs +0x30), which the session reads
    /// (see the [`afterburner`](crate::afterburner) module).
    pub afterburner: bool,
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

/// Advances `state`, `flyer`'s ship, one tick under `controls`, flying as
/// `handling` allows under `rules` (see the module docs): an NPC's tick.
pub fn step(
    state: &mut ShipState,
    handling: &Handling,
    controls: Controls,
    rules: FlightRules,
    flyer: Flyer,
) {
    let rate = handling.turn_rate;
    state.heading = match controls.turn {
        Turn::None if controls.reverse && state.velocity.length() > rules.at_rest_speed() => {
            turn_toward(state.heading, heading_of(state.velocity * -1.0), rate)
        }
        turn => turned(state.heading, turn, rate),
    };
    let caps = Caps::top(handling.max_speed);
    advance(state, handling, controls.thrust, caps, rules, flyer);
}

/// Advances the player's ship, `state`, one tick under `controls`, flying
/// as `handling` allows under `rules`: it turns as `_HandlePlayer`'s turn
/// block does, toward `aim` while the auto-turn aims (reverse or the face
/// key, which the session works out), the turn keys held off, and by
/// them otherwise ([`player_turn`](crate::player_turn)); then it thrusts,
/// is capped by `caps` and moves ([`advance`]).
pub fn player_step(
    state: &mut ShipState,
    handling: &Handling,
    controls: Controls,
    aim: Option<i16>,
    caps: Caps,
    rules: FlightRules,
) {
    (state.heading, _) = turn(
        state.heading,
        handling.turn_rate,
        aim,
        controls.turn,
        rules.player_turn,
    );
    advance(state, handling, controls.thrust, caps, rules, Flyer::Player);
}

/// The rest of a tick after the turn (see the module docs, steps 2-4):
/// `state`, `flyer`'s ship, thrusts along its heading if `thrust`, toward
/// `caps`' thrust top, is capped to its box and moves, flying as
/// `handling` allows under `rules`.
pub fn advance(
    state: &mut ShipState,
    handling: &Handling,
    thrust: bool,
    caps: Caps,
    rules: FlightRules,
    flyer: Flyer,
) {
    match rules.speed_cap {
        SpeedCap::PerAxis => {
            if thrust {
                state.velocity = adjusted_accel(
                    state.velocity,
                    state.heading,
                    handling.accel,
                    caps.thrust_top,
                );
            }
            if flyer == Flyer::Player {
                state.velocity = speed_limit(state.velocity, caps.max_vel);
            }
        }
        SpeedCap::Circle => {
            if thrust {
                state.velocity = state.velocity + facing(state.heading) * handling.accel;
            }
            state.velocity = circle(state.velocity, caps.max_vel);
        }
    }
    state.position = state.position + state.velocity;
}

/// Advances `state`, an inertialess NPC's ship holding `speed` along its
/// heading, one tick under `controls`, flying as `handling` allows under
/// `rules` (see the module docs): it turns by the turn keys alone, as
/// reverse never turns an inertialess ship; thrust adds a tick's
/// [`THRUST`] to its speed and reverse is the AI's brake, [`BRAKE`]
/// (`_LowLevelAIHandler` substate 1, @0x853d7-0x85403); then
/// [`inertialess_advance`].
pub fn inertialess_step(
    state: &mut ShipState,
    speed: &mut f32,
    handling: &Handling,
    controls: Controls,
    rules: FlightRules,
) {
    state.heading = turned(state.heading, controls.turn, handling.turn_rate);
    let thrust = if controls.thrust {
        THRUST
    } else if controls.reverse {
        BRAKE
    } else {
        0.0
    };
    let caps = Caps::top(handling.max_speed);
    inertialess_advance(state, speed, handling, thrust, caps, rules, Flyer::Npc);
}

/// Advances the player's inertialess ship, `state`, holding `speed` along
/// its heading, one tick under `controls`, flying as `handling` allows
/// under `rules`: it turns as [`player_step`] does, toward `aim` or by the
/// turn keys (reverse gives an inertialess ship no aim); reverse takes a
/// tick's acceleration off its speed (@0x6c770-0x6c7d3) and then thrust
/// adds one (@0x6cbb2-0x6cc3e), each floored and capped as
/// [`step_speed`] does, at the top speed whatever `caps` say (the thrust
/// ignores the afterburner, @0x6cbb2-0x6cc3e); then
/// [`inertialess_advance`], capped by `caps`.
pub fn inertialess_player_step(
    state: &mut ShipState,
    speed: &mut f32,
    handling: &Handling,
    controls: Controls,
    aim: Option<i16>,
    caps: Caps,
    rules: FlightRules,
) {
    (state.heading, _) = turn(
        state.heading,
        handling.turn_rate,
        aim,
        controls.turn,
        rules.player_turn,
    );
    if controls.reverse {
        *speed = step_speed(*speed, REVERSE, handling.accel, handling.max_speed);
    }
    let thrust = if controls.thrust { THRUST } else { 0.0 };
    inertialess_advance(state, speed, handling, thrust, caps, rules, Flyer::Player);
}

/// The rest of an inertialess ship's tick after its turn: `state`,
/// `flyer`'s ship, holding `speed` along its heading, moves its speed by
/// `thrust` ticks of its acceleration when that is not none
/// ([`step_speed`], at the top speed); the player's speed is then capped
/// at `caps`' box (@0x6d356-0x6d369). Its velocity is pulled toward
/// heading x speed by [`ADJUST_RATE`] ticks' acceleration
/// ([`adjust_velocity`], @0x6d43a, @0x3425f), capped as [`advance`] caps
/// it, and it moves.
///
/// The original's NPC is adjusted and moved (@0x34250-0x342aa) before its
/// thrust word moves the speed (@0x34acc-0x34b6d), so an NPC's thrust
/// reaches its velocity a tick later there than here: a one-tick lag the
/// player never sees, left as it is.
pub fn inertialess_advance(
    state: &mut ShipState,
    speed: &mut f32,
    handling: &Handling,
    thrust: f32,
    caps: Caps,
    rules: FlightRules,
    flyer: Flyer,
) {
    if thrust != 0.0 {
        *speed = step_speed(*speed, thrust, handling.accel, handling.max_speed);
    }
    let top = caps.max_vel;
    if flyer == Flyer::Player {
        *speed = speed.min(top);
    }
    state.velocity = adjust_velocity(
        state.velocity,
        state.heading,
        *speed,
        handling.accel * ADJUST_RATE,
    );
    state.velocity = match (rules.speed_cap, flyer) {
        (SpeedCap::PerAxis, Flyer::Player) => speed_limit(state.velocity, top),
        (SpeedCap::PerAxis, Flyer::Npc) => state.velocity,
        (SpeedCap::Circle, _) => circle(state.velocity, top),
    };
    state.position = state.position + state.velocity;
}

/// `heading` turned a tick by the turn keys, `turn`, at `rate` degrees a
/// tick: an NPC's turn, kept within `[0, 360)` ([`normalized`]).
#[must_use]
pub fn turned(heading: f32, turn: Turn, rate: f32) -> f32 {
    match turn {
        Turn::Left => normalized(heading - rate),
        Turn::Right => normalized(heading + rate),
        Turn::None => heading,
    }
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
    use crate::rulebook::{RuleKey, RuleSource, RuleValue, Rulebook};
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
        face: Face::Off,
        afterburner: false,
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
        face: Face::Off,
        afterburner: false,
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

    /// `state` after `ticks` steps of `controls` by an NPC.
    fn after(mut state: ShipState, controls: Controls, ticks: usize) -> ShipState {
        for _ in 0..ticks {
            step(
                &mut state,
                &SHIP,
                controls,
                FlightRules::default(),
                Flyer::Npc,
            );
        }
        state
    }

    const FLYERS: [Flyer; 2] = [Flyer::Player, Flyer::Npc];

    /// The caps of [`SHIP`]'s top speed.
    const TOP: Caps = Caps {
        thrust_top: 3.0,
        max_vel: 3.0,
    };

    /// The original's flight rules but the speed cap's other reading.
    const CIRCLE: FlightRules = FlightRules {
        thrust_divisor: match NonZeroU32::new(5000) {
            Some(divisor) => divisor,
            None => panic!(),
        },
        speed_cap: SpeedCap::Circle,
        player_turn: PlayerTurn::WholeDegrees,
        inertial_dampener: InertialDampener::PlayerOnly,
        formation_flight: FormationFlight::Original,
    };

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
    fn a_turn_by_the_keys_wraps_within_0_to_360() {
        assert_eq!(turned(90.0, Turn::Left, 3.0), 87.0);
        assert_eq!(turned(359.0, Turn::Right, 3.0), 2.0);
        assert_eq!(turned(1.0, Turn::Left, 3.0), 358.0);
        assert_eq!(turned(90.0, Turn::None, 3.0), 90.0);
        assert_eq!(
            turned(2.0, Turn::Left, 2.000_000_2),
            0.0,
            "a hair past north rounds to 360, which wraps"
        );
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
            Flyer::Player,
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
    fn drag_by_works_each_component_in_f64_and_stores_it_as_an_f32() {
        // 0.03 × 0.95 in f64 stores 0.0285; worked in f32 it is a bit
        // short.
        let dragged = drag_by(Vec2::new(0.03, -0.03), 0.95);
        assert_eq!(dragged.x.to_bits(), 0.0285_f32.to_bits());
        assert_eq!(dragged.y.to_bits(), (-0.0285_f32).to_bits());
        assert_ne!(0.03_f32 * 0.95_f32, 0.0285_f32);
        let dragged = drag_by(Vec2::new(0.5, -0.25), 0.98);
        assert_eq!(dragged, Vec2::new(0.49, -0.245));
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
            for flyer in FLYERS {
                for rules in [FlightRules::default(), CIRCLE] {
                    let mut state = moving(velocity, 0.0);
                    step(&mut state, &at_top, Controls::default(), rules, flyer);
                    assert_eq!(state.velocity, velocity, "{flyer:?} {rules:?}");
                }
            }
        }
    }

    #[test]
    fn under_the_circle_speed_is_capped_at_the_top_speed_and_reaches_it_exactly() {
        for flyer in FLYERS {
            let mut state = heading(0.0);
            for _ in 0..100 {
                step(&mut state, &SHIP, THRUST, CIRCLE, flyer);
                assert!(state.velocity.length() <= SHIP.max_speed, "{state:?}");
            }
            assert_eq!(state.velocity, Vec2::new(0.0, -3.0), "{flyer:?}");
        }
    }

    #[test]
    fn under_the_circle_turning_at_top_speed_swings_the_velocity_round_and_never_exceeds_it() {
        for flyer in FLYERS {
            let mut state = flown(heading(0.0), THRUST, 40, CIRCLE, flyer);
            let carving = Controls {
                thrust: true,
                turn: Turn::Right,
                reverse: false,
                face: Face::Off,
                afterburner: false,
            };
            // While the heading is within a few turns of the velocity,
            // thrust holds the ship at its top speed.
            for _ in 0..10 {
                step(&mut state, &SHIP, carving, CIRCLE, flyer);
                let speed = state.velocity.length();
                assert!((speed - SHIP.max_speed).abs() < 1e-5, "{speed}: {state:?}");
            }
            assert!(state.velocity.x > 0.0, "swinging right: {state:?}");
            // Thrusting further round slows it, and nothing speeds it past
            // the cap.
            for _ in 0..60 {
                step(&mut state, &SHIP, carving, CIRCLE, flyer);
                assert!(
                    state.velocity.length() <= SHIP.max_speed + 1e-5,
                    "{state:?}"
                );
            }
            assert!(state.velocity.length() < SHIP.max_speed - 0.1, "{state:?}");
        }
    }

    // The per-axis cap.

    /// `state` after `ticks` steps of `controls` by `flyer` under `rules`.
    fn flown(
        mut state: ShipState,
        controls: Controls,
        ticks: usize,
        rules: FlightRules,
        flyer: Flyer,
    ) -> ShipState {
        for _ in 0..ticks {
            step(&mut state, &SHIP, controls, rules, flyer);
        }
        state
    }

    #[test]
    fn by_default_a_ship_at_top_speed_turned_a_quarter_and_thrusting_ends_near_c_by_minus_c() {
        // At top speed east, turned to face up the screen.
        let east = moving(Vec2::new(3.0, 0.0), 0.0);
        let player = flown(east, THRUST, 100, FlightRules::default(), Flyer::Player);
        assert!(near(player.velocity, Vec2::new(3.0, -3.0)), "{player:?}");
        assert!((player.velocity.length() - 3.0 * 2.0_f32.sqrt()).abs() < 1e-4);
        let npc = flown(east, THRUST, 100, FlightRules::default(), Flyer::Npc);
        assert!(
            (npc.velocity - Vec2::new(3.0, -3.0)).length() <= SHIP.accel,
            "the last push may carry it past: {npc:?}"
        );
        assert!(npc.velocity.length() > 3.0 * 1.41, "{npc:?}");
        for flyer in FLYERS {
            let circled = flown(east, THRUST, 100, CIRCLE, flyer);
            assert!(
                (circled.velocity.length() - 3.0).abs() < 1e-5,
                "{flyer:?}: {circled:?}"
            );
            assert!(
                circled.velocity.x < 0.5 && circled.velocity.y < -2.9,
                "swung round toward the heading: {circled:?}"
            );
        }
    }

    #[test]
    fn an_npc_has_no_clamp_and_the_players_box_clamps_each_axis_to_the_top_speed() {
        let coast = Controls::default();
        for (velocity, boxed) in [
            (Vec2::new(6.0, 0.0), Vec2::new(3.0, 0.0)),
            (Vec2::new(-6.0, 9.0), Vec2::new(-3.0, 3.0)),
            (Vec2::new(2.0, -2.5), Vec2::new(2.0, -2.5)),
        ] {
            let start = moving(velocity, 0.0);
            let npc = flown(start, coast, 1, FlightRules::default(), Flyer::Npc);
            assert_eq!(npc.velocity, velocity, "kept");
            assert_eq!(npc.position, velocity, "and moved by");
            let player = flown(start, coast, 1, FlightRules::default(), Flyer::Player);
            assert_eq!(player.velocity, boxed, "clamped before the move");
            assert_eq!(player.position, boxed);
        }
        for flyer in FLYERS {
            for velocity in [Vec2::new(6.0, 0.0), Vec2::new(-6.0, 9.0)] {
                let circled = flown(moving(velocity, 0.0), coast, 1, CIRCLE, flyer);
                assert!(
                    (circled.velocity.length() - 3.0).abs() < 1e-5,
                    "{flyer:?}: {circled:?}"
                );
                let direction = velocity * (1.0 / velocity.length());
                assert!(near(circled.velocity, direction * 3.0), "{circled:?}");
            }
        }
    }

    #[test]
    fn thrust_along_an_axis_reaches_the_top_speed_and_stops_adding() {
        let player = flown(
            heading(0.0),
            THRUST,
            100,
            FlightRules::default(),
            Flyer::Player,
        );
        assert_eq!(player.velocity, Vec2::new(0.0, -3.0), "exactly, by the box");
        let npc = flown(
            heading(0.0),
            THRUST,
            100,
            FlightRules::default(),
            Flyer::Npc,
        );
        assert!(npc.velocity.x.abs() < 1e-5, "{npc:?}");
        assert!(
            npc.velocity.y <= -3.0 && npc.velocity.y > -3.0 - SHIP.accel,
            "at it, or past it by under a push: {npc:?}"
        );
        let later = flown(npc, THRUST, 10, FlightRules::default(), Flyer::Npc);
        assert_eq!(later.velocity, npc.velocity, "no more");
    }

    #[test]
    fn by_default_each_axis_stays_within_the_headings_share_or_the_box() {
        let carving = Controls {
            thrust: true,
            turn: Turn::Right,
            reverse: false,
            face: Face::Off,
            afterburner: false,
        };
        let mut player = heading(0.0);
        let mut npc = heading(0.0);
        for _ in 0..200 {
            step(
                &mut player,
                &SHIP,
                carving,
                FlightRules::default(),
                Flyer::Player,
            );
            assert!(
                player.velocity.x.abs() <= 3.0 && player.velocity.y.abs() <= 3.0,
                "{player:?}"
            );
            let before = npc.velocity;
            step(&mut npc, &SHIP, carving, FlightRules::default(), Flyer::Npc);
            let share = facing(npc.heading) * SHIP.max_speed;
            for (now, was, top) in [
                (npc.velocity.x, before.x, share.x),
                (npc.velocity.y, before.y, share.y),
            ] {
                // An axis gains toward its share only while short of it.
                let gained = (now - was) * top.signum() > 1e-6;
                if gained && top.abs() > 1e-3 {
                    assert!(
                        was * top.signum() < top.abs() + 1e-5,
                        "{was} to {now} for {top}"
                    );
                }
            }
            assert!(
                npc.velocity.x.abs() <= 3.0 + SHIP.accel
                    && npc.velocity.y.abs() <= 3.0 + SHIP.accel,
                "{npc:?}"
            );
        }
    }

    #[test]
    fn by_default_thrust_goes_along_the_whole_degree_and_under_the_circle_along_the_heading() {
        for flyer in FLYERS {
            let player = flown(heading(45.9), THRUST, 1, FlightRules::default(), flyer);
            assert_eq!(player.velocity, facing(45.0) * SHIP.accel, "{flyer:?}");
            assert_eq!(player.heading, 45.9, "the heading itself is kept");
            let circled = flown(heading(45.9), THRUST, 1, CIRCLE, flyer);
            assert!(
                near(circled.velocity, facing(45.9) * SHIP.accel),
                "{circled:?}"
            );
            assert_ne!(circled.velocity, facing(45.0) * SHIP.accel);
        }
    }

    #[test]
    fn the_speed_cap_reads_the_rulebook() {
        assert_eq!(FlightRules::default().speed_cap, SpeedCap::PerAxis);
        assert_eq!(CIRCLE.speed_cap, SpeedCap::Circle);
        assert_eq!(
            FlightRules::from_rulebook(
                &Rulebook::default().with_override(RuleKey::SpeedCap, RuleSource::Bible)
            ),
            CIRCLE
        );
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
                face: Face::Off,
                afterburner: false,
            },
            FlightRules::default(),
            Flyer::Player,
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
    fn an_npcs_left_or_right_overrides_reverse() {
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
    fn reverse_then_thrust_brakes_the_ship_to_a_stop_under_either_cap() {
        for flyer in FLYERS {
            for rules in [FlightRules::default(), CIRCLE] {
                let mut state = moving(Vec2::new(0.0, -3.0), 0.0);
                for _ in 0..60 {
                    step(&mut state, &SHIP, REVERSE, rules, flyer);
                }
                assert_eq!(state.heading, 180.0);
                let brake = Controls {
                    thrust: true,
                    ..REVERSE
                };
                // Facing back, each tick of thrust takes 0.1 off the speed.
                for _ in 0..30 {
                    step(&mut state, &SHIP, brake, rules, flyer);
                    assert!((state.heading - 180.0).abs() < 1e-3, "{state:?}");
                }
                assert!(
                    state.velocity.length() < 1e-4,
                    "{flyer:?} {rules:?}: {state:?}"
                );
            }
        }
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
        step(&mut slow, &SHIP, REVERSE, under(3000), Flyer::Player);
        assert_eq!(slow.heading, 90.0, "at rest under 3000");
        let mut fast = creeping;
        step(
            &mut fast,
            &SHIP,
            REVERSE,
            FlightRules::default(),
            Flyer::Player,
        );
        assert_eq!(fast.heading, 87.0, "moving under 5000");
    }

    // The player's step.

    /// A ship of `Maneuver` 25: 2.5° a tick.
    const PEGASUS: Handling = Handling {
        turn_rate: 2.5,
        ..SHIP
    };

    fn player_after(
        mut state: ShipState,
        controls: Controls,
        aim: Option<i16>,
        rules: FlightRules,
        ticks: usize,
    ) -> ShipState {
        for _ in 0..ticks {
            player_step(
                &mut state,
                &PEGASUS,
                controls,
                aim,
                Caps::top(PEGASUS.max_speed),
                rules,
            );
        }
        state
    }

    #[test]
    fn the_player_turns_whole_degrees_and_an_npc_the_full_rate() {
        let right = turning(Turn::Right);
        let player = player_after(heading(90.0), right, None, FlightRules::default(), 1);
        assert_eq!(player.heading, 92.0);
        let mut npc = heading(90.0);
        step(
            &mut npc,
            &PEGASUS,
            right,
            FlightRules::default(),
            Flyer::Npc,
        );
        assert_eq!(npc.heading, 92.5);
        let full = FlightRules {
            player_turn: PlayerTurn::Full,
            ..FlightRules::default()
        };
        assert_eq!(
            player_after(heading(90.0), right, None, full, 1).heading,
            92.5
        );
    }

    #[test]
    fn the_players_auto_turn_holds_the_turn_keys_off_and_stops_short_of_its_aim() {
        let left = turning(Turn::Left);
        let aimed = player_after(heading(0.0), left, Some(9), FlightRules::default(), 6);
        assert_eq!(
            aimed.heading, 8.0,
            "turned right toward 9, the keys held off"
        );
        assert_eq!(
            player_after(heading(0.0), left, None, FlightRules::default(), 1).heading,
            358.0
        );
    }

    #[test]
    fn the_players_step_turns_before_it_thrusts_and_caps_its_speed() {
        let quarter = Handling {
            turn_rate: 90.0,
            ..SHIP
        };
        let mut state = heading(0.0);
        let controls = Controls {
            thrust: true,
            turn: Turn::Right,
            ..Controls::default()
        };
        player_step(
            &mut state,
            &quarter,
            controls,
            None,
            TOP,
            FlightRules::default(),
        );
        assert!(near(state.velocity, Vec2::new(0.1, 0.0)), "{state:?}");
        assert!(near(state.position, Vec2::new(0.1, 0.0)), "{state:?}");
        let mut fast = moving(Vec2::new(6.0, 0.0), 90.0);
        player_step(
            &mut fast,
            &SHIP,
            Controls::default(),
            None,
            TOP,
            FlightRules::default(),
        );
        assert_eq!(fast.velocity, Vec2::new(3.0, 0.0), "the player's box");
        assert_eq!(fast.position, Vec2::new(3.0, 0.0));
    }

    #[test]
    fn the_players_thrust_pushes_toward_its_caps_thrust_top() {
        let caps = Caps {
            thrust_top: 5.4,
            max_vel: 5.4,
        };
        let mut state = moving(Vec2::new(3.0, 0.0), 90.0);
        player_step(
            &mut state,
            &SHIP,
            THRUST,
            None,
            caps,
            FlightRules::default(),
        );
        assert!(
            near(state.velocity, Vec2::new(3.1, 0.0)),
            "past the top: {state:?}"
        );
        let mut plain = moving(Vec2::new(3.0, 0.0), 90.0);
        player_step(&mut plain, &SHIP, THRUST, None, TOP, FlightRules::default());
        assert!(
            near(plain.velocity, Vec2::new(3.0, 0.0)),
            "held at it: {plain:?}"
        );
    }

    #[test]
    fn the_players_velocity_is_clamped_to_its_caps_box_under_either_reading() {
        let narrow = Caps {
            thrust_top: 3.0,
            max_vel: 1.0,
        };
        let mut boxed = moving(Vec2::new(2.0, -2.0), 90.0);
        player_step(
            &mut boxed,
            &SHIP,
            Controls::default(),
            None,
            narrow,
            FlightRules::default(),
        );
        assert_eq!(boxed.velocity, Vec2::new(1.0, -1.0), "{boxed:?}");
        let wide = Caps {
            thrust_top: 3.0,
            max_vel: 5.0,
        };
        let mut fast = moving(Vec2::new(4.0, 0.0), 90.0);
        player_step(
            &mut fast,
            &SHIP,
            Controls::default(),
            None,
            wide,
            FlightRules::default(),
        );
        assert_eq!(fast.velocity, Vec2::new(4.0, 0.0), "within the box");
        let mut circled = moving(Vec2::new(4.0, 4.0), 90.0);
        player_step(&mut circled, &SHIP, Controls::default(), None, wide, CIRCLE);
        assert!(
            (circled.velocity.length() - 5.0).abs() < 1e-5,
            "{circled:?}"
        );
        let mut rested = moving(Vec2::new(4.0, 4.0), 90.0);
        let start = Caps {
            thrust_top: 3.0,
            max_vel: 0.0,
        };
        player_step(
            &mut rested,
            &SHIP,
            THRUST,
            None,
            start,
            FlightRules::default(),
        );
        assert_eq!(
            rested.velocity,
            Vec2::ZERO,
            "a box of none holds it at rest"
        );
        assert_eq!(rested.position, Vec2::ZERO);
    }

    #[test]
    fn the_players_inertialess_speed_is_capped_at_its_box_after_its_thrust_at_the_top_speed() {
        let wide = Caps {
            thrust_top: 5.4,
            max_vel: 5.4,
        };
        let mut state = moving(Vec2::new(4.0, 0.0), 90.0);
        let mut speed = 4.0;
        inertialess_player_step(
            &mut state,
            &mut speed,
            &SHIP,
            Controls::default(),
            None,
            wide,
            FlightRules::default(),
        );
        assert_eq!(speed, 4.0, "within the box");
        assert!(near(state.velocity, Vec2::new(4.0, 0.0)), "{state:?}");
        inertialess_player_step(
            &mut state,
            &mut speed,
            &SHIP,
            THRUST,
            None,
            wide,
            FlightRules::default(),
        );
        assert_eq!(speed, 3.0, "thrust caps it at the top speed");
        let narrow = Caps {
            thrust_top: 3.0,
            max_vel: 2.0,
        };
        inertialess_player_step(
            &mut state,
            &mut speed,
            &SHIP,
            Controls::default(),
            None,
            narrow,
            FlightRules::default(),
        );
        assert_eq!(speed, 2.0, "and the box below it");
    }

    #[test]
    fn the_player_turn_reads_the_rulebook() {
        assert_eq!(FlightRules::default().player_turn, PlayerTurn::WholeDegrees);
        assert_eq!(
            FlightRules::from_rulebook(
                &Rulebook::default().with_override(RuleKey::PlayerTurn, RuleSource::Bible)
            )
            .player_turn,
            PlayerTurn::Full
        );
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

    // Inertialess flight.

    /// A ship that turns a quarter a tick, so a test turns it in one.
    const QUARTER: Handling = Handling {
        turn_rate: 90.0,
        ..SHIP
    };

    #[test]
    fn an_inertialess_ship_turned_a_quarter_follows_its_heading_within_the_ticks_its_rate_allows() {
        for flyer in FLYERS {
            // East at full speed, then a quarter left to north.
            let mut state = moving(Vec2::new(3.0, 0.0), 90.0);
            let mut speed = 3.0;
            let left = turning(Turn::Left);
            match flyer {
                Flyer::Npc => inertialess_step(
                    &mut state,
                    &mut speed,
                    &QUARTER,
                    left,
                    FlightRules::default(),
                ),
                Flyer::Player => inertialess_player_step(
                    &mut state,
                    &mut speed,
                    &QUARTER,
                    left,
                    None,
                    TOP,
                    FlightRules::default(),
                ),
            }
            assert_eq!(state.heading, 0.0, "{flyer:?}");
            let rate = QUARTER.accel * crate::inertialess::ADJUST_RATE;
            // Each axis has 3 to make up at 0.4 a tick: 8 ticks, the first
            // the turn's.
            let ticks = (3.0 / rate).ceil() as usize;
            assert_eq!(ticks, 8);
            for _ in 1..ticks {
                inertialess_step(
                    &mut state,
                    &mut speed,
                    &QUARTER,
                    Controls::default(),
                    FlightRules::default(),
                );
            }
            assert!(
                near(state.velocity, Vec2::new(0.0, -3.0)),
                "{flyer:?}: {state:?}"
            );
            assert_eq!(speed, 3.0, "the speed is kept through the turn");
            let at = state.position;
            inertialess_step(
                &mut state,
                &mut speed,
                &QUARTER,
                Controls::default(),
                FlightRules::default(),
            );
            assert!(near(state.position - at, Vec2::new(0.0, -3.0)), "no drift");
        }
    }

    #[test]
    fn an_inertialess_npc_turns_by_its_keys_either_way() {
        for (turn, turned) in [(Turn::Left, 87.0), (Turn::Right, 93.0), (Turn::None, 90.0)] {
            let mut state = heading(90.0);
            let mut speed = 0.0;
            inertialess_step(
                &mut state,
                &mut speed,
                &SHIP,
                turning(turn),
                FlightRules::default(),
            );
            assert_eq!(state.heading, turned, "{turn:?}");
        }
    }

    #[test]
    fn the_velocity_moves_by_the_rate_a_tick_until_it_reaches_the_heading() {
        let mut state = moving(Vec2::new(3.0, 0.0), 0.0);
        let mut speed = 3.0;
        inertialess_step(
            &mut state,
            &mut speed,
            &SHIP,
            Controls::default(),
            FlightRules::default(),
        );
        assert!(near(state.velocity, Vec2::new(2.6, -0.4)), "{state:?}");
        assert!(
            near(state.position, Vec2::new(2.6, -0.4)),
            "moves by the new velocity"
        );
    }

    #[test]
    fn an_inertialess_ship_coasting_keeps_its_speed_along_its_heading() {
        let mut state = moving(facing(30.0) * 2.0, 30.0);
        let mut speed = 2.0;
        for _ in 0..50 {
            inertialess_step(
                &mut state,
                &mut speed,
                &SHIP,
                Controls::default(),
                FlightRules::default(),
            );
        }
        assert_eq!(speed, 2.0);
        assert!(near(state.velocity, facing(30.0) * 2.0), "{state:?}");
    }

    #[test]
    fn thrust_adds_to_the_speed_and_the_velocity_follows_it() {
        let mut state = heading(90.0);
        let mut speed = 0.0;
        inertialess_step(
            &mut state,
            &mut speed,
            &SHIP,
            THRUST,
            FlightRules::default(),
        );
        assert!((speed - 0.1).abs() < 1e-6, "{speed}");
        assert!(near(state.velocity, Vec2::new(0.1, 0.0)), "{state:?}");
        for _ in 0..100 {
            inertialess_step(
                &mut state,
                &mut speed,
                &SHIP,
                THRUST,
                FlightRules::default(),
            );
        }
        assert_eq!(speed, 3.0, "up to the top speed");
    }

    #[test]
    fn an_npcs_inertialess_brake_takes_half_a_ticks_thrust_off_and_never_turns() {
        let mut state = moving(Vec2::new(0.0, -2.0), 0.0);
        let mut speed = 2.0;
        inertialess_step(
            &mut state,
            &mut speed,
            &SHIP,
            REVERSE,
            FlightRules::default(),
        );
        assert_eq!(state.heading, 0.0, "reverse turns no inertialess ship");
        assert!((speed - 1.95).abs() < 1e-6, "{speed}");
        assert!(near(state.velocity, Vec2::new(0.0, -1.95)), "{state:?}");
    }

    #[test]
    fn the_players_reverse_takes_a_whole_ticks_thrust_off_before_its_thrust_adds_one() {
        let mut state = moving(Vec2::new(0.0, -2.0), 0.0);
        let mut speed = 2.0;
        inertialess_player_step(
            &mut state,
            &mut speed,
            &SHIP,
            REVERSE,
            None,
            TOP,
            FlightRules::default(),
        );
        assert_eq!(state.heading, 0.0);
        assert!((speed - 1.9).abs() < 1e-6, "{speed}");
        let mut slow = 0.05;
        let both = Controls {
            thrust: true,
            ..REVERSE
        };
        inertialess_player_step(
            &mut heading(0.0),
            &mut slow,
            &SHIP,
            both,
            None,
            TOP,
            FlightRules::default(),
        );
        assert!(
            (slow - 0.1).abs() < 1e-6,
            "floored at none, then thrust: {slow}"
        );
    }

    #[test]
    fn the_players_inertialess_step_turns_by_its_turn_block_toward_an_aim() {
        let mut state = heading(0.0);
        let mut speed = 0.0;
        inertialess_player_step(
            &mut state,
            &mut speed,
            &PEGASUS,
            turning(Turn::Left),
            Some(9),
            TOP,
            FlightRules::default(),
        );
        assert_eq!(state.heading, 2.0, "toward 9, the keys held off");
    }

    #[test]
    fn the_players_inertialess_speed_and_velocity_stay_within_its_box() {
        let mut state = moving(Vec2::new(5.0, 0.0), 90.0);
        let mut speed = 5.0;
        inertialess_player_step(
            &mut state,
            &mut speed,
            &SHIP,
            Controls::default(),
            None,
            TOP,
            FlightRules::default(),
        );
        assert_eq!(speed, 3.0, "capped at the top speed");
        assert!(near(state.velocity, Vec2::new(3.0, 0.0)), "{state:?}");
        // An NPC's speed above its top speed is left alone while it coasts.
        let mut npc = moving(Vec2::new(5.0, 0.0), 90.0);
        let mut fast = 5.0;
        inertialess_step(
            &mut npc,
            &mut fast,
            &SHIP,
            Controls::default(),
            FlightRules::default(),
        );
        assert_eq!(fast, 5.0);
        assert!(near(npc.velocity, Vec2::new(5.0, 0.0)), "{npc:?}");
    }

    #[test]
    fn under_the_circle_an_inertialess_ship_is_held_to_its_top_speed() {
        let mut state = moving(Vec2::new(3.0, 3.0), 90.0);
        let mut speed = 3.0;
        inertialess_step(&mut state, &mut speed, &SHIP, Controls::default(), CIRCLE);
        assert!((state.velocity.length() - 3.0).abs() < 1e-5, "{state:?}");
        let mut player = moving(Vec2::new(3.0, 3.0), 90.0);
        let mut speed = 3.0;
        inertialess_player_step(
            &mut player,
            &mut speed,
            &SHIP,
            Controls::default(),
            None,
            TOP,
            CIRCLE,
        );
        assert!((player.velocity.length() - 3.0).abs() < 1e-5, "{player:?}");
    }

    #[test]
    fn the_inertial_dampener_rule_reads_the_rulebook() {
        assert_eq!(
            FlightRules::default().inertial_dampener,
            InertialDampener::PlayerOnly
        );
        assert_eq!(
            FlightRules::from_rulebook(
                &Rulebook::default().with_override(RuleKey::InertialDampener, RuleSource::Bible)
            )
            .inertial_dampener,
            InertialDampener::AnyShip
        );
    }

    #[test]
    fn the_formation_flight_rule_reads_the_rulebook() {
        assert_eq!(
            FlightRules::default().formation_flight,
            FormationFlight::Original
        );
        assert_eq!(
            FlightRules::from_rulebook(
                &Rulebook::default().with_override(RuleKey::FormationFlight, RuleSource::Bible)
            )
            .formation_flight,
            FormationFlight::Earlier
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
            step(
                &mut state,
                &SHIP,
                REVERSE,
                FlightRules::default(),
                Flyer::Player,
            );
        }
        for _ in 0..30 {
            step(
                &mut state,
                &SHIP,
                brake,
                FlightRules::default(),
                Flyer::Player,
            );
        }
        assert!(state.velocity != Vec2::ZERO, "leftover: {state:?}");
        let stopped = state.heading;
        let held = after(state, REVERSE, 10);
        assert_eq!(held.heading, stopped, "{held:?}");
    }
}
