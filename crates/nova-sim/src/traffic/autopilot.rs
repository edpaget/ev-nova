//! The NPC autopilot: each tick it turns an NPC's goal into [`Controls`]
//! and flies them with the player's [`flight::step`], on the NPC's own
//! handling. It decides nothing (the [`ai`](crate::ai) does); it only
//! flies the goal it is given.
//!
//! - [`Goal::Land`]: it heads for the stellar at full speed, steering out
//!   any sideways drift, and brakes (Down and thrust) once its stopping
//!   distance reaches the distance left. It lands within the stellar's
//!   [`landing_radius`] at or below [`LANDING_SPEED`].
//! - [`Goal::JumpOut`]: it heads straight out from the centre at full
//!   speed, and jumps once it is its stats' jump distance out.
//! - [`Goal::Follow`]: it keeps within [`FOLLOW_DISTANCE`] of its lead,
//!   matching its lead's velocity there.
//! - [`Goal::Assist`]: outside its reach of the player (see
//!   [`assist`]) it follows the player as an escort its lead; within it,
//!   it brakes.
//! - [`Goal::Idle`], or a goal it cannot fly (a stellar, lead or player
//!   that is not there): it brakes to a stop.
//! - A ship that is not [`Condition::Intact`] (disabled, breaking up or
//!   destroyed) drifts: it flies on with no controls, and never lands or
//!   jumps.
//! - While it jumps in ([`Mode::JumpingIn`]) it glides towards the centre,
//!   [`glide_speed`] a tick, with no steering, for
//!   [`JUMP_IN_TICKS`](crate::traffic::spawn::JUMP_IN_TICKS); then it flies
//!   on at its top speed.

use crate::ai::{Goal, fire};
use crate::catalog::{LandingSite, StellarId};
use crate::combat::hull::Condition;
use crate::flight::{self, AT_REST_SPEED, Controls, ShipState, Turn, heading_of, shortest_turn};
use crate::geometry::Vec2;
use crate::hail::assist;
use crate::handling::Handling;
use crate::hyperspace::JUMP_FUEL;
use crate::landing::{LANDING_SPEED, landing_radius};
use crate::traffic::npc::{Mode, Npc};
use crate::traffic::spawn::{JUMP_IN_TICKS, glide_speed};

/// How far from its lead, in pixels, an escort keeps.
pub const FOLLOW_DISTANCE: f32 = 100.0;
/// How far off its course, in degrees, a ship still thrusts.
pub const ALIGNED: f32 = 5.0;
/// How many turns' worth off the heading it wants an attacker approaching
/// still thrusts (4.0 @0xdd680).
pub const APPROACH_THRUST: f32 = 4.0;
/// How far beyond a turn, in degrees, a dogfighter still thrusts (15.0
/// @0xdda30).
pub const DOGFIGHT_THRUST: f32 = 15.0;
/// How far beyond a turn, in degrees, a ship fleeing close still thrusts
/// (20.0 @0xdd668).
pub const FLEE_THRUST: f32 = 20.0;
/// How near, in pixels on either axis, an attacker must be for a ship
/// fleeing to turn straight away from it (low mode 5, @0x85c62).
pub const FLEE_CLOSE: f32 = 250.0;

/// What became of an NPC this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It flies on.
    Flying,
    /// It landed on this stellar, and leaves the system.
    Landed(StellarId),
    /// It jumped out of the system.
    JumpedOut,
}

/// Flies `npc` one tick towards its goal, among `sites`, the ship its
/// goal is about (the lead it follows, the ship it fights or inspects)
/// at `other`, when it is there.
pub fn fly(npc: &mut Npc, sites: &[LandingSite], other: Option<&ShipState>) -> Outcome {
    if npc.condition != Condition::Intact {
        flight::step(&mut npc.state, &npc.stats.handling, Controls::default());
        return Outcome::Flying;
    }
    if let Mode::JumpingIn { ticks_left } = npc.mode {
        glide(npc, ticks_left);
        return Outcome::Flying;
    }
    let handling = npc.stats.handling;
    let state = npc.state;
    let controls = match npc.goal {
        Goal::Land(stellar) => match sites.iter().find(|site| site.id == stellar) {
            Some(site) if over(&state, site) => return Outcome::Landed(stellar),
            Some(site) => approach(&state, &handling, site.position),
            None => brake(&state, &handling),
        },
        Goal::JumpOut => {
            if state.position.length() >= npc.stats.jump_distance {
                return Outcome::JumpedOut;
            }
            run_out(&state, &handling)
        }
        Goal::Follow(_) | Goal::Inspect(_) => match other {
            Some(other) => follow(&state, &handling, other),
            None => brake(&state, &handling),
        },
        Goal::Attack(_) => match other {
            Some(target) => attack(npc, target),
            None => brake(&state, &handling),
        },
        Goal::Snipe(_) => match other {
            Some(target) if state.velocity.length() <= AT_REST_SPEED => Controls {
                turn: steer(state.heading, fire::wanted_heading(npc, target), &handling),
                ..Controls::default()
            },
            _ => brake(&state, &handling),
        },
        Goal::Flee(_) => match other {
            Some(attacker) => {
                let off = state.position - attacker.position;
                if off.x.abs() <= FLEE_CLOSE && off.y.abs() <= FLEE_CLOSE {
                    let away = heading_of(off);
                    Controls {
                        thrust: shortest_turn(state.heading, away).abs()
                            <= handling.turn_rate + FLEE_THRUST,
                        turn: steer(state.heading, away, &handling),
                        reverse: false,
                    }
                } else {
                    let out = state.position.length();
                    if out >= npc.stats.jump_distance && npc.reserves.fuel.now >= JUMP_FUEL {
                        return Outcome::JumpedOut;
                    }
                    run_out(&state, &handling)
                }
            }
            None => brake(&state, &handling),
        },
        Goal::Assist(help) => match other {
            Some(player)
                if !assist::within(&state, player, assist::reach(help, handling.turn_rate)) =>
            {
                follow(&state, &handling, player)
            }
            _ => brake(&state, &handling),
        },
        Goal::Idle => brake(&state, &handling),
    };
    flight::step(&mut npc.state, &handling, controls);
    Outcome::Flying
}

/// Heads straight out from the centre at full speed; from the centre
/// itself, ahead.
fn run_out(state: &ShipState, handling: &Handling) -> Controls {
    let away = if state.position.length() > 0.0 {
        heading_of(state.position)
    } else {
        state.heading
    };
    match_velocity(state, handling, flight::facing(away) * handling.max_speed)
}

/// Turns from `heading` towards `wanted`, not at all within half a tick's
/// turn, where turning would only overshoot.
fn steer(heading: f32, wanted: f32, handling: &Handling) -> Turn {
    let off = shortest_turn(heading, wanted);
    let half_turn = handling.turn_rate / 2.0;
    if off > half_turn {
        Turn::Right
    } else if off < -half_turn {
        Turn::Left
    } else {
        Turn::None
    }
}

/// Attacks a target at `target`: faces the heading it wants to fire along
/// ([`fire::wanted_heading`]), thrusting within [`APPROACH_THRUST`] turns
/// of it approaching from beyond the dogfight box, and within a turn and
/// [`DOGFIGHT_THRUST`] dogfighting inside it.
fn attack(npc: &Npc, target: &ShipState) -> Controls {
    let handling = npc.stats.handling;
    let wanted = fire::wanted_heading(npc, target);
    let window = match fire::manoeuvre(npc.goal, &npc.state, target) {
        Some(fire::Manoeuvre::Dogfight) => handling.turn_rate + DOGFIGHT_THRUST,
        _ => APPROACH_THRUST * handling.turn_rate,
    };
    Controls {
        thrust: shortest_turn(npc.state.heading, wanted).abs() <= window,
        turn: steer(npc.state.heading, wanted, &handling),
        reverse: false,
    }
}

/// One tick of `npc`'s glide in from hyperspace, `ticks_left` of it left:
/// along its heading at the glide's speed, and on at its top speed once
/// the glide is over.
fn glide(npc: &mut Npc, ticks_left: u32) {
    let inward = flight::facing(npc.state.heading);
    let tick = JUMP_IN_TICKS.saturating_sub(ticks_left);
    npc.state.velocity = inward * glide_speed(tick);
    npc.state.position = npc.state.position + npc.state.velocity;
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        npc.state.velocity = inward * npc.stats.handling.max_speed;
        Mode::Flying
    } else {
        Mode::JumpingIn { ticks_left: left }
    };
}

/// Whether a ship at `state` can land on `site`: over it, and slow enough.
fn over(state: &ShipState, site: &LandingSite) -> bool {
    (site.position - state.position).length() <= landing_radius(site)
        && state.velocity.length() <= LANDING_SPEED
}

/// Heads for `target` at full speed, and brakes once the ship's stopping
/// distance reaches the distance left.
fn approach(state: &ShipState, handling: &Handling, target: Vec2) -> Controls {
    let to = target - state.position;
    let distance = to.length();
    if distance <= stopping_distance(state, handling) {
        return brake(state, handling);
    }
    match_velocity(state, handling, to * (handling.max_speed / distance))
}

/// How far a ship at `state` travels while it turns to face against its
/// motion and brakes to a stop; none at rest, and for ever when it cannot
/// turn or thrust.
fn stopping_distance(state: &ShipState, handling: &Handling) -> f32 {
    let speed = state.velocity.length();
    if speed <= AT_REST_SPEED {
        return 0.0;
    }
    if handling.turn_rate <= 0.0 || handling.accel <= 0.0 {
        return f32::INFINITY;
    }
    let behind = heading_of(state.velocity * -1.0);
    let turning = shortest_turn(state.heading, behind).abs() / handling.turn_rate;
    // Braking a tick at a time covers about half a tick's speed more
    // than the continuous v² / 2a; a whole tick's is the margin.
    speed * (turning.ceil() + 1.0) + speed * speed / (2.0 * handling.accel)
}

/// Turns to face against the ship's motion (Down) and thrusts once it
/// does, until thrust would only push it the other way.
fn brake(state: &ShipState, handling: &Handling) -> Controls {
    let speed = state.velocity.length();
    if speed <= AT_REST_SPEED {
        return Controls::default();
    }
    let behind = heading_of(state.velocity * -1.0);
    // Down lands exactly on the heading behind within one tick's turn,
    // before this tick's thrust.
    let facing_back = shortest_turn(state.heading, behind).abs() <= handling.turn_rate;
    Controls {
        thrust: facing_back && speed > handling.accel / 2.0,
        turn: Turn::None,
        reverse: true,
    }
}

/// Keeps within [`FOLLOW_DISTANCE`] of a lead at `lead`: closing at the
/// speed it can still brake from in the distance beyond it, on top of the
/// lead's velocity, and matching the lead's velocity within it.
fn follow(state: &ShipState, handling: &Handling, lead: &ShipState) -> Controls {
    let to = lead.position - state.position;
    let distance = to.length();
    let beyond = (distance - FOLLOW_DISTANCE).max(0.0);
    let speed = braking_speed(beyond, handling).min(handling.max_speed);
    // None within the distance, where `to` may be no direction at all.
    let closing = to * (speed / distance.max(FOLLOW_DISTANCE));
    match_velocity(state, handling, lead.velocity + closing)
}

/// The fastest a ship can go and still turn round and stop within
/// `distance`: the `v` with `v T + v² / 2a = distance`, `T` the ticks a
/// half turn takes. None when it cannot turn or thrust.
fn braking_speed(distance: f32, handling: &Handling) -> f32 {
    if handling.turn_rate <= 0.0 || handling.accel <= 0.0 {
        return 0.0;
    }
    let half_turn = 180.0 / handling.turn_rate;
    let a = handling.accel;
    a * (half_turn.mul_add(half_turn, 2.0 * distance / a).sqrt() - half_turn)
}

/// Turns towards the change that takes the ship's velocity to `desired`,
/// and thrusts once it faces it; coasts once the velocity is within half
/// a tick's thrust of it.
fn match_velocity(state: &ShipState, handling: &Handling, desired: Vec2) -> Controls {
    let change = desired - state.velocity;
    if change.length() <= handling.accel / 2.0 {
        return Controls::default();
    }
    let wanted = heading_of(change);
    let off = shortest_turn(state.heading, wanted);
    Controls {
        thrust: off.abs() <= ALIGNED.max(handling.turn_rate),
        turn: steer(state.heading, wanted, handling),
        reverse: false,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::hail::{Help, assist};
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, planet};
    use crate::traffic::npc::NpcId;
    use crate::traffic::spawn::{hyperspace_entry, jump_in_distance};

    /// An average ship: 3 pixels a tick at most, 0.1 more a tick, 1° a
    /// tick.
    const AVERAGE: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 10,
        ..FAST
    };

    fn npc(fields: ShipFields, goal: Goal, start: ShipState) -> Npc {
        Npc {
            state: start,
            goal,
            ..crate::testkit::npc(1, ShipStats::new(fields, &[]))
        }
    }

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// Flies `npc` until it lands or jumps, or `limit` ticks pass, and
    /// gives how it ended and after how many ticks.
    fn until_gone(npc: &mut Npc, sites: &[LandingSite], limit: u32) -> (Outcome, u32) {
        for tick in 0..limit {
            let outcome = fly(npc, sites, None);
            if outcome != Outcome::Flying {
                return (outcome, tick);
            }
        }
        (Outcome::Flying, limit)
    }

    #[test]
    fn the_named_values() {
        assert_eq!((FOLLOW_DISTANCE, ALIGNED), (100.0, 5.0));
    }

    #[test]
    fn a_ship_at_rest_flies_to_a_stellar_and_lands_on_it() {
        let sites = [planet(140, 500.0, 300.0), planet(141, -800.0, 0.0)];
        for fields in [FAST, AVERAGE] {
            for heading in [0.0, 90.0, 225.0] {
                let mut ship = npc(
                    fields,
                    Goal::Land(StellarId(140)),
                    at(0.0, 0.0, 0.0, 0.0, heading),
                );
                let (outcome, ticks) = until_gone(&mut ship, &sites, 2000);
                assert_eq!(
                    outcome,
                    Outcome::Landed(StellarId(140)),
                    "{fields:?} {heading}"
                );
                assert!(ticks < 1200, "{ticks} ticks");
                let off = (ship.state.position - sites[0].position).length();
                assert!(off <= 50.0, "{off}");
                assert!(ship.state.velocity.length() <= LANDING_SPEED);
            }
        }
    }

    #[test]
    fn a_ship_crossing_past_a_stellar_turns_back_and_lands_on_it() {
        let sites = [planet(140, 0.0, 0.0)];
        for (fields, start) in [
            (FAST, at(-400.0, 200.0, 6.0, 0.0, 90.0)),
            (FAST, at(0.0, -100.0, 0.0, 6.0, 180.0)),
            (AVERAGE, at(300.0, 300.0, 0.0, -3.0, 0.0)),
            (AVERAGE, at(-20.0, 0.0, 3.0, 0.0, 90.0)),
        ] {
            let mut ship = npc(fields, Goal::Land(StellarId(140)), start);
            let (outcome, ticks) = until_gone(&mut ship, &sites, 3000);
            assert_eq!(outcome, Outcome::Landed(StellarId(140)), "{start:?}");
            assert!(ticks < 2000, "{start:?}: {ticks} ticks");
        }
    }

    #[test]
    fn a_ship_over_a_stellar_slow_enough_lands_at_once() {
        let sites = [planet(140, 10.0, 0.0)];
        let mut ship = npc(
            FAST,
            Goal::Land(StellarId(140)),
            at(0.0, 0.0, 1.0, 0.0, 0.0),
        );
        assert_eq!(
            fly(&mut ship, &sites, None),
            Outcome::Landed(StellarId(140))
        );
        assert_eq!(ship.state.position, Vec2::ZERO, "where it was");
        let mut fast = npc(
            FAST,
            Goal::Land(StellarId(140)),
            at(0.0, 0.0, 1.5, 0.0, 0.0),
        );
        assert_eq!(fly(&mut fast, &sites, None), Outcome::Flying, "too fast");
        let mut far = npc(
            FAST,
            Goal::Land(StellarId(140)),
            at(61.0, 0.0, 0.0, 0.0, 0.0),
        );
        assert_eq!(fly(&mut far, &sites, None), Outcome::Flying, "too far");
    }

    #[test]
    fn a_ship_not_intact_drifts_and_never_lands_or_jumps() {
        let sites = [planet(140, 0.0, 0.0)];
        for condition in [
            Condition::Disabled,
            Condition::Dying { ticks_left: 4 },
            Condition::Destroyed,
        ] {
            let mut lander = npc(
                AVERAGE,
                Goal::Land(StellarId(140)),
                at(0.0, 0.0, 0.0, 0.0, 0.0),
            );
            lander.condition = condition;
            assert_eq!(
                fly(&mut lander, &sites, None),
                Outcome::Flying,
                "{condition:?}"
            );
            assert_eq!(
                lander.state,
                at(0.0, 0.0, 0.0, 0.0, 0.0),
                "at rest, it stays"
            );
            let mut jumper = npc(AVERAGE, Goal::JumpOut, at(0.0, -2000.0, 1.0, -2.0, 90.0));
            jumper.condition = condition;
            jumper.mode = Mode::JumpingIn { ticks_left: 3 };
            assert_eq!(fly(&mut jumper, &sites, None), Outcome::Flying);
            assert_eq!(jumper.state, at(1.0, -2002.0, 1.0, -2.0, 90.0), "drifting");
            assert_eq!(jumper.mode, Mode::JumpingIn { ticks_left: 3 }, "no glide");
        }
    }

    #[test]
    fn a_ship_jumping_out_heads_out_and_jumps_at_its_jump_distance() {
        for start in [
            at(100.0, -50.0, 0.0, 0.0, 180.0),
            at(0.0, 0.0, 0.0, 0.0, 30.0),
        ] {
            let mut ship = npc(FAST, Goal::JumpOut, start);
            let mut farthest = 0.0_f32;
            let (outcome, ticks) = (0..2000)
                .find_map(|tick| {
                    let outcome = fly(&mut ship, &[], None);
                    farthest = farthest.max(ship.state.position.length());
                    (outcome != Outcome::Flying).then_some((outcome, tick))
                })
                .expect("jumps");
            assert_eq!(outcome, Outcome::JumpedOut);
            assert!(ticks < 400, "{ticks}");
            assert!(farthest >= ship.stats.jump_distance, "{farthest}");
            assert!(farthest < ship.stats.jump_distance + 10.0, "{farthest}");
        }
    }

    #[test]
    fn a_ship_at_its_jump_distance_jumps_without_moving() {
        let mut ship = npc(FAST, Goal::JumpOut, at(0.0, -1000.0, 0.0, 0.0, 0.0));
        assert_eq!(fly(&mut ship, &[], None), Outcome::JumpedOut);
        assert_eq!(ship.state.position, Vec2::new(0.0, -1000.0));
        let mut short = npc(FAST, Goal::JumpOut, at(0.0, -999.0, 0.0, 0.0, 0.0));
        assert_eq!(fly(&mut short, &[], None), Outcome::Flying);
    }

    #[test]
    fn a_jump_in_glides_43_ticks_to_about_1000_out_without_steering() {
        let start = hyperspace_entry(&mut crate::testkit::Draws::of(&[60]));
        let mut ship = npc(FAST, Goal::Land(StellarId(140)), start);
        ship.mode = Mode::JumpingIn { ticks_left: 43 };
        let sites = [planet(140, 0.0, 0.0)];
        let mut last = start.position.length();
        for tick in 0..43 {
            assert!(matches!(ship.mode, Mode::JumpingIn { .. }), "{tick}");
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying);
            let now = ship.state.position.length();
            assert!((last - now - glide_speed(tick)).abs() < 1e-2, "{tick}");
            assert_eq!(ship.state.heading, start.heading, "no steering");
            last = now;
        }
        assert_eq!(ship.mode, Mode::Flying);
        assert!((last - 1000.0).abs() < 0.1, "{last}");
        let inward = start.position * (-1.0 / jump_in_distance());
        let top = ship.stats.handling.max_speed;
        assert!(
            (ship.state.velocity - inward * top).length() < 1e-3,
            "{:?}",
            ship.state
        );
    }

    #[test]
    fn a_ship_with_no_speed_never_moves() {
        let still = ShipFields { speed: 0, ..FAST };
        let sites = [planet(140, 300.0, 0.0)];
        let lead = at(500.0, 0.0, 2.0, 0.0, 90.0);
        for goal in [
            Goal::Land(StellarId(140)),
            Goal::JumpOut,
            Goal::Follow(NpcId(0)),
            Goal::Idle,
        ] {
            let mut ship = npc(still, goal, at(0.0, 0.0, 0.0, 0.0, 45.0));
            for _ in 0..300 {
                assert_eq!(fly(&mut ship, &sites, Some(&lead)), Outcome::Flying);
            }
            assert_eq!(ship.state.position, Vec2::ZERO, "{goal:?}");
        }
    }

    #[test]
    fn an_idle_ship_or_one_whose_target_is_gone_brakes_to_a_stop() {
        for goal in [
            Goal::Idle,
            Goal::Land(StellarId(999)),
            Goal::Follow(NpcId(9)),
        ] {
            let mut ship = npc(FAST, goal, at(0.0, 0.0, 4.0, -3.0, 10.0));
            for _ in 0..200 {
                assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
            }
            assert!(
                ship.state.velocity.length() < 0.5,
                "{goal:?}: {:?}",
                ship.state
            );
        }
    }

    #[test]
    fn an_assisting_ship_comes_to_the_player_and_waits_docked_within_its_reach() {
        for help in [Help::Refuel, Help::Repair] {
            let player = at(0.0, 0.0, 0.0, 0.0, 0.0);
            let mut ship = npc(FAST, Goal::Assist(help), at(-600.0, 400.0, 0.0, 0.0, 0.0));
            for _ in 0..600 {
                assert_eq!(fly(&mut ship, &[], Some(&player)), Outcome::Flying);
            }
            let reach = assist::reach(help, ship.stats.handling.turn_rate);
            assert!(
                assist::within(&ship.state, &player, reach),
                "{help:?} {:?}",
                ship.state
            );
            assert!(assist::docked(&ship.state), "{help:?} {:?}", ship.state);
        }
    }

    #[test]
    fn outside_its_reach_an_assisting_ship_follows_the_player() {
        let player = at(0.0, 0.0, 1.0, 0.0, 0.0);
        // FAST turns 3° a tick: a refuel's reach is 105.
        for start in [
            at(-106.0, 0.0, 0.0, 0.0, 0.0),
            at(0.0, 300.0, 2.0, 1.0, 45.0),
        ] {
            let mut ship = npc(FAST, Goal::Assist(Help::Refuel), start);
            let mut escort = npc(FAST, Goal::Follow(NpcId(0)), start);
            fly(&mut ship, &[], Some(&player));
            fly(&mut escort, &[], Some(&player));
            assert_eq!(ship.state, escort.state, "{start:?}");
        }
    }

    #[test]
    fn within_its_reach_or_with_the_player_gone_an_assisting_ship_brakes() {
        let player = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let start = at(-105.0, 105.0, 2.0, -1.0, 30.0);
        for (help, other) in [
            (Help::Refuel, Some(&player)),
            (Help::Repair, Some(&player)),
            (Help::Refuel, None),
        ] {
            let mut ship = npc(FAST, Goal::Assist(help), start);
            let mut idle = npc(FAST, Goal::Idle, start);
            for _ in 0..30 {
                fly(&mut ship, &[], other);
                fly(&mut idle, &[], other);
                assert_eq!(ship.state, idle.state, "{help:?} {other:?}");
            }
        }
        let moving = at(0.0, 0.0, 1.0, 0.0, 0.0);
        let mut ship = npc(
            FAST,
            Goal::Assist(Help::Refuel),
            at(-100.0, 0.0, 0.0, 0.0, 90.0),
        );
        for _ in 0..30 {
            fly(&mut ship, &[], Some(&moving));
        }
        assert_eq!(
            ship.state.position,
            Vec2::new(-100.0, 0.0),
            "within reach of a moving player it does not chase"
        );
        let far = at(-1000.0, 0.0, 2.0, 0.0, 30.0);
        let mut ship = npc(FAST, Goal::Assist(Help::Repair), far);
        let mut idle = npc(FAST, Goal::Idle, far);
        fly(&mut ship, &[], None);
        fly(&mut idle, &[], None);
        assert_eq!(ship.state, idle.state, "gone, from afar");
    }

    #[test]
    fn an_escort_catches_up_with_its_lead_and_keeps_with_it() {
        let mut lead = at(0.0, 0.0, 2.0, 0.0, 90.0);
        let mut ship = npc(
            FAST,
            Goal::Follow(NpcId(0)),
            at(-600.0, 400.0, 0.0, 0.0, 0.0),
        );
        for _ in 0..600 {
            lead.position = lead.position + lead.velocity;
            assert_eq!(fly(&mut ship, &[], Some(&lead)), Outcome::Flying);
        }
        let apart = (ship.state.position - lead.position).length();
        assert!(apart <= FOLLOW_DISTANCE * 1.5, "{apart}");
        assert!(
            (ship.state.velocity - lead.velocity).length() < 1.0,
            "{:?}",
            ship.state
        );
    }
    // The helpers, exactly.

    /// 6 pixels a tick at most, 0.3 more a tick, 3° a tick.
    fn fast() -> Handling {
        ShipStats::new(FAST, &[]).handling
    }

    #[test]
    fn the_stopping_distance_is_the_turn_then_the_braking() {
        let handling = Handling {
            max_speed: 3.0,
            accel: 0.1,
            turn_rate: 3.0,
        };
        // Moving up at 3, facing up: a half turn, 60 ticks, and one more,
        // then 3² / 0.2.
        let ahead = at(0.0, 0.0, 0.0, -3.0, 0.0);
        assert!((stopping_distance(&ahead, &handling) - (3.0 * 61.0 + 45.0)).abs() < 1e-3);
        // Facing 100° round: 80° to go, 27 ticks (26.7 rounded up).
        let partway = at(0.0, 0.0, 0.0, -3.0, 100.0);
        assert!((stopping_distance(&partway, &handling) - (3.0 * 28.0 + 45.0)).abs() < 1e-3);
        // Already facing back: no turn, but the tick's margin.
        let back = at(0.0, 0.0, 0.0, -2.0, 180.0);
        assert!((stopping_distance(&back, &handling) - (2.0 + 20.0)).abs() < 1e-3);
        assert_eq!(
            stopping_distance(&at(0.0, 0.0, 0.0, 0.0, 0.0), &handling),
            0.0
        );
    }

    #[test]
    fn a_ship_that_cannot_turn_or_thrust_never_stops() {
        let back = at(0.0, 0.0, 0.0, -2.0, 180.0);
        for handling in [
            Handling {
                turn_rate: 0.0,
                ..fast()
            },
            Handling {
                accel: 0.0,
                ..fast()
            },
        ] {
            assert_eq!(
                stopping_distance(&back, &handling),
                f32::INFINITY,
                "{handling:?}"
            );
            assert_eq!(braking_speed(500.0, &handling), 0.0, "{handling:?}");
        }
    }

    #[test]
    fn the_braking_speed_turns_and_stops_within_the_distance() {
        let handling = fast();
        let half_turn = 180.0 / handling.turn_rate;
        for distance in [0.0, 50.0, 250.0, 1000.0] {
            let speed = braking_speed(distance, &handling);
            let needed = speed.mul_add(half_turn, speed * speed / (2.0 * handling.accel));
            assert!((needed - distance).abs() < 1e-2, "{distance}: {speed}");
        }
        assert!(braking_speed(250.0, &handling) > braking_speed(50.0, &handling));
    }

    #[test]
    fn braking_turns_back_then_thrusts_while_thrust_slows_the_ship() {
        let handling = fast();
        let a = handling.accel;
        let brake_at =
            |speed: f32, heading: f32| brake(&at(0.0, 0.0, 0.0, -speed, heading), &handling);
        let turning = brake_at(3.0, 0.0);
        assert_eq!(
            (turning.reverse, turning.thrust, turning.turn),
            (true, false, Turn::None)
        );
        assert!(
            brake_at(3.0, 177.0).thrust,
            "within one turn of facing back"
        );
        assert!(!brake_at(3.0, 176.9).thrust);
        assert!(brake_at(a * 0.6, 180.0).thrust);
        assert!(!brake_at(a * 0.5, 180.0).thrust, "would only push it back");
        assert_eq!(brake_at(0.0, 90.0), Controls::default(), "at rest");
    }

    #[test]
    fn matching_a_velocity_coasts_within_half_a_ticks_thrust() {
        let handling = fast();
        let a = handling.accel;
        let still = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let up = |speed: f32| Vec2::new(0.0, -speed);
        assert_eq!(
            match_velocity(&still, &handling, up(a * 0.4)),
            Controls::default()
        );
        let thrusting = match_velocity(&still, &handling, up(a * 0.6));
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::None));
    }

    #[test]
    fn matching_a_velocity_turns_only_beyond_half_a_ticks_turn() {
        let handling = fast();
        let facing = |heading: f32| {
            match_velocity(
                &at(0.0, 0.0, 0.0, 0.0, heading),
                &handling,
                Vec2::new(0.0, -6.0),
            )
        };
        // Wanting heading 0, from either side.
        assert_eq!(facing(358.5).turn, Turn::None, "1.5° short: half a turn");
        assert_eq!(facing(358.4).turn, Turn::Right);
        assert_eq!(facing(1.5).turn, Turn::None);
        assert_eq!(facing(1.6).turn, Turn::Left);
        assert_eq!(facing(358.8).turn, Turn::None);
        // Thrust within the aligned cone or a tick's turn, whichever is
        // wider.
        assert!(facing(5.0).thrust && !facing(5.1).thrust);
        let nimble = Handling {
            turn_rate: 8.0,
            ..handling
        };
        let wide = match_velocity(&at(0.0, 0.0, 0.0, 0.0, 7.9), &nimble, Vec2::new(0.0, -6.0));
        assert!(wide.thrust);
    }

    #[test]
    fn an_escort_beyond_its_distance_closes_no_faster_than_it_can_brake() {
        let handling = fast();
        let lead = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 150 below the lead, moving up at 2 and facing it: 50 beyond its
        // distance it can close at about 0.8, so it turns to slow down.
        let closing = at(0.0, 150.0, 0.0, -2.0, 0.0);
        let controls = follow(&closing, &handling, &lead);
        assert!(!controls.thrust, "{controls:?}");
        assert_ne!(controls.turn, Turn::None);
        // From 350 below it can close faster than 2: it thrusts on.
        let far = at(0.0, 350.0, 0.0, -2.0, 0.0);
        assert!(follow(&far, &handling, &lead).thrust);
    }

    #[test]
    fn an_escort_within_its_distance_matches_its_leads_velocity() {
        let handling = fast();
        let lead = at(10.0, 0.0, 1.0, 0.0, 90.0);
        let alongside = at(0.0, 0.0, 1.0, 0.0, 90.0);
        assert_eq!(follow(&alongside, &handling, &lead), Controls::default());
        let on_top = at(10.0, 0.0, 1.0, 0.0, 90.0);
        assert_eq!(follow(&on_top, &handling, &lead), Controls::default());
        let at_the_edge = at(-90.0, 0.0, 1.0, 0.0, 90.0);
        assert_eq!(follow(&at_the_edge, &handling, &lead), Controls::default());
    }

    #[test]
    fn a_ship_jumps_out_straight_away_from_the_centre() {
        let start = at(300.0, -400.0, 0.0, 0.0, 200.0);
        let mut ship = npc(FAST, Goal::JumpOut, start);
        let mut last = start.position;
        while fly(&mut ship, &[], None) == Outcome::Flying {
            last = ship.state.position;
        }
        let off = shortest_turn(heading_of(start.position), heading_of(last));
        assert!(off.abs() < 3.0, "{off}: {last:?}");
        // From the centre itself, straight ahead.
        let mut ship = npc(FAST, Goal::JumpOut, at(0.0, 0.0, 0.0, 0.0, 30.0));
        let mut last = Vec2::ZERO;
        while fly(&mut ship, &[], None) == Outcome::Flying {
            last = ship.state.position;
        }
        let off = shortest_turn(30.0, heading_of(last));
        assert!(off.abs() < 1e-3, "{off}: {last:?}");
    }

    // Fights.

    use crate::combat::ShipRef;

    const FOE: ShipRef = ShipRef::Player;

    /// A ship at rest at the centre facing `heading`, 3 degrees a tick,
    /// with `goal`.
    fn fighter(goal: Goal, heading: f32) -> Npc {
        npc(FAST, goal, at(0.0, 0.0, 0.0, 0.0, heading))
    }

    fn controls_of(ship: &Npc, other: &ShipState) -> Controls {
        let mut flown = ship.clone();
        let before = flown.state;
        fly(&mut flown, &[], Some(other));
        // Undo the step to read the controls from the change it made.
        let turned = shortest_turn(before.heading, flown.state.heading);
        let speed_change = (flown.state.velocity - before.velocity).length();
        Controls {
            thrust: speed_change > 0.0,
            turn: if turned > 0.0 {
                Turn::Right
            } else if turned < 0.0 {
                Turn::Left
            } else {
                Turn::None
            },
            reverse: false,
        }
    }

    #[test]
    fn attacking_from_afar_it_turns_to_the_lead_and_thrusts_within_four_turns() {
        // 3 degrees a tick: thrust within 12 of the target 1000 above.
        let target = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        let thrusting = controls_of(&fighter(Goal::Attack(FOE), 12.0), &target);
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::Left));
        let turning = controls_of(&fighter(Goal::Attack(FOE), 12.1), &target);
        assert_eq!((turning.thrust, turning.turn), (false, Turn::Left));
        let other_way = controls_of(&fighter(Goal::Attack(FOE), 300.0), &target);
        assert_eq!(other_way.turn, Turn::Right);
    }

    #[test]
    fn dogfighting_within_165_it_thrusts_within_a_turn_and_15_degrees() {
        // 3 + 15: within 18 of the target 165 above.
        let target = at(165.0, -165.0, 0.0, 0.0, 0.0);
        let bearing = 45.0;
        let thrusting = controls_of(&fighter(Goal::Attack(FOE), bearing + 18.0), &target);
        assert!(thrusting.thrust);
        let turning = controls_of(&fighter(Goal::Attack(FOE), bearing + 18.1), &target);
        assert!(!turning.thrust);
        let afar = at(165.1, -165.0, 0.0, 0.0, 0.0);
        let approaching = controls_of(&fighter(Goal::Attack(FOE), bearing + 13.0), &afar);
        assert!(!approaching.thrust, "outside the box, within 12 only");
    }

    #[test]
    fn an_attacker_leads_a_moving_target_with_its_gun() {
        use crate::catalog::WeaponRecord;
        use crate::combat::armament::Armament;
        use crate::combat::weapon::WeaponSpec;
        let mut ship = fighter(Goal::Attack(FOE), 0.0);
        ship.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                speed: 1000,
                count: 50,
                ..crate::testkit::weapon(128)
            }),
            1,
        )]);
        // 1000 above crossing right at 10 a tick: led 1000 to the right,
        // 45 degrees.
        let crossing = at(0.0, -1000.0, 10.0, 0.0, 0.0);
        let mut flown = ship.clone();
        for _ in 0..30 {
            fly(&mut flown, &[], Some(&crossing));
        }
        assert!(
            (flown.state.heading - 45.0).abs() < 3.0,
            "{:?}",
            flown.state
        );
    }

    #[test]
    fn sniping_it_brakes_to_a_stop_then_faces_its_target() {
        let target = at(-300.0, 0.0, 0.0, 0.0, 0.0);
        let mut moving = npc(FAST, Goal::Snipe(FOE), at(0.0, 0.0, 0.0, -3.0, 0.0));
        for _ in 0..200 {
            fly(&mut moving, &[], Some(&target));
        }
        assert!(moving.state.velocity.length() <= crate::flight::AT_REST_SPEED);
        let bearing = crate::combat::aim::bearing(moving.state.position, target.position);
        assert!(
            shortest_turn(moving.state.heading, bearing).abs() < 3.0,
            "{:?}",
            moving.state
        );
        let still = fighter(Goal::Snipe(FOE), 0.0);
        let controls = controls_of(&still, &target);
        assert_eq!((controls.thrust, controls.turn), (false, Turn::Left));
    }

    #[test]
    fn fleeing_close_it_turns_straight_away_and_thrusts_within_a_turn_and_20() {
        // The attacker 250 below: away is up, 0; thrust within 23.
        let attacker = at(0.0, 250.0, 0.0, 0.0, 0.0);
        let thrusting = controls_of(&fighter(Goal::Flee(FOE), 23.0), &attacker);
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::Left));
        let turning = controls_of(&fighter(Goal::Flee(FOE), 23.1), &attacker);
        assert!(!turning.thrust);
    }

    #[test]
    fn fleeing_from_afar_it_runs_out_from_the_centre_and_jumps_with_the_fuel() {
        // The attacker 251 right: run straight out, up from (0, -10).
        let attacker = at(251.0, -10.0, 0.0, 0.0, 0.0);
        let mut runner = npc(FAST, Goal::Flee(FOE), at(0.0, -10.0, 0.0, 0.0, 0.0));
        let (outcome, _) = (0..1000)
            .find_map(|tick| {
                let outcome = fly(&mut runner, &[planet(140, 0.0, -10.0)], Some(&attacker));
                (outcome != Outcome::Flying).then_some((outcome, tick))
            })
            .expect("leaves");
        assert_eq!(outcome, Outcome::JumpedOut, "never lands");
        assert!(runner.state.position.x.abs() < 1.0, "{:?}", runner.state);
        assert!(
            runner.state.position.length() >= runner.stats.jump_distance,
            "only past its jump distance: {:?}",
            runner.state
        );
        let mut dry = npc(FAST, Goal::Flee(FOE), at(0.0, -1000.0, 0.0, -6.0, 0.0));
        dry.reserves.fuel.now = 99.0;
        for _ in 0..30 {
            assert_eq!(fly(&mut dry, &[], Some(&attacker)), Outcome::Flying);
        }
        assert!(dry.state.position.y < -1100.0, "runs on: {:?}", dry.state);
    }

    #[test]
    fn inspecting_it_closes_in_and_keeps_alongside() {
        let target = at(400.0, 0.0, 0.0, 0.0, 0.0);
        let mut inspector = fighter(Goal::Inspect(FOE), 0.0);
        for _ in 0..600 {
            fly(&mut inspector, &[], Some(&target));
        }
        let apart = (inspector.state.position - target.position).length();
        assert!(
            apart <= FOLLOW_DISTANCE * 1.1,
            "{apart}: {:?}",
            inspector.state
        );
        assert!(inspector.state.velocity.length() < 0.5);
    }

    #[test]
    fn a_fight_whose_ship_is_gone_brakes() {
        for goal in [
            Goal::Attack(FOE),
            Goal::Snipe(FOE),
            Goal::Flee(FOE),
            Goal::Inspect(FOE),
        ] {
            let mut ship = npc(FAST, goal, at(0.0, 0.0, 4.0, -3.0, 10.0));
            for _ in 0..200 {
                assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
            }
            assert!(
                ship.state.velocity.length() < 0.5,
                "{goal:?}: {:?}",
                ship.state
            );
        }
    }
}
