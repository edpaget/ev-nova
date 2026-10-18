//! The flight of a ship leaving by hyperspace: each [`Leg`] the high
//! level picks ([`leave::leg`](crate::ai::leave::leg), or
//! [`leave::flee_leg`](crate::ai::leave::flee_leg) for a ship fleeing
//! beyond its attacker's reach), flown as `_LowLevelAIHandler` flies the
//! sub-goal (addresses in the `EV Nova` executable).
//!
//! - [`Leg::RunOut`] (sub 3, @0x857bd-0x8585b): it turns to the
//!   whole-degree bearing out from the centre, `_CalcInterceptAngle((0,0),
//!   position)` ([`trig::intercept_angle`]; 0, straight up, at the centre
//!   itself), and thrusts while it is off that bearing by less than its
//!   turn rate and [`RUN_OUT_THRUST`], in whole degrees, strictly
//!   ([`fire::off_by`]). It flies at its top speed (+0x34 = 0).
//! - [`Leg::Brake`] (sub 1): it brakes as the autopilot's stop does, an
//!   inertialess ship as substate 1 does
//!   ([`inertialess::brake`](crate::traffic::inertialess::brake)). The
//!   original's substate 1 (@0x8536d-0x85633) is
//!   rdm:phase/escort-fleet-fidelity/phase-2-formation-flight's; its settle
//!   below 0.35 on both axes can't run while the leave brakes, as the high
//!   level sets sub 1 only while an axis is at 0.35 or more and the low
//!   level reads the same velocity.
//! - [`Leg::Jump`] (sub 4): it leaves the system that tick
//!   ([`Outcome::JumpedOut`]). Sub 4 is the warp out (`_LowLevelAIHandler`
//!   @0x8585e-0x859c5), which
//!   rdm:phase/ship-handling-fidelity/phase-8-warp-out ports; until then the
//!   ship departs as it reaches sub 4, so only the moment of departure
//!   differs from the original's. That phase replaces this arm.

use crate::ai::fire;
use crate::ai::leave::{self, Leg};
use crate::flight::{Controls, ShipState};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::rulebook::RuleSource;
use crate::traffic::autopilot::{self, Outcome, steer};
use crate::traffic::inertialess::Inertia;
use crate::traffic::npc::Npc;
use crate::trig;

/// How far beyond a turn, in degrees, a ship running out still thrusts
/// (3.0 @0xdd664).
pub const RUN_OUT_THRUST: f32 = 3.0;

/// The controls a ship at `state` on `handling` flies `leg` by, as
/// `inertia` says, or, on [`Leg::Jump`], that it jumped out (see the
/// module docs).
///
/// # Errors
///
/// [`Outcome::JumpedOut`] on [`Leg::Jump`].
pub fn controls(
    state: &ShipState,
    leg: Leg,
    handling: &Handling,
    inertia: Inertia,
) -> Result<Controls, Outcome> {
    match leg {
        Leg::RunOut => Ok(run_out(state, handling)),
        Leg::Brake => Ok(autopilot::brake(state, handling, inertia)),
        Leg::Jump => Err(Outcome::JumpedOut),
    }
}

/// The controls `npc` leaving (state 2) flies by on `handling`, as
/// `inertia` says, or that it jumped out: [`leave::leg`]'s leg, flown by
/// [`controls`].
///
/// # Errors
///
/// [`Outcome::JumpedOut`] once its leg is [`Leg::Jump`].
pub fn leaving(npc: &Npc, handling: &Handling, inertia: Inertia) -> Result<Controls, Outcome> {
    let leg = leave::leg(&npc.state, leave::expedites(npc));
    controls(&npc.state, leg, handling, inertia)
}

/// The controls `npc` fleeing beyond its attacker's reach (state 3's far
/// branch) flies by on `handling`, as `inertia` says, or that it jumped
/// out: [`leave::flee_leg`]'s leg, whether it can jump read by
/// `jump_fuel` ([`leave::can_jump`]), flown by [`controls`].
///
/// # Errors
///
/// [`Outcome::JumpedOut`] once its leg is [`Leg::Jump`].
pub fn fleeing_afar(
    npc: &Npc,
    handling: &Handling,
    inertia: Inertia,
    jump_fuel: RuleSource,
) -> Result<Controls, Outcome> {
    let leg = leave::flee_leg(
        &npc.state,
        leave::expedites(npc),
        leave::can_jump(npc, jump_fuel),
    );
    controls(&npc.state, leg, handling, inertia)
}

/// Sub 3's controls for a ship at `state` on `handling`: turned to the
/// bearing out from the centre, thrusting within its turn rate and
/// [`RUN_OUT_THRUST`] of it.
fn run_out(state: &ShipState, handling: &Handling) -> Controls {
    let out = trig::intercept_angle(Vec2::ZERO, state.position);
    Controls {
        thrust: fire::off_by(state.heading, out).abs() < handling.turn_rate + RUN_OUT_THRUST,
        turn: steer(state.heading, f32::from(out), handling),
        reverse: false,
        ..Controls::default()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::flight::Turn;

    const HANDLING: Handling = Handling {
        max_speed: 5.0,
        accel: 0.1,
        turn_rate: 4.0,
    };

    fn at(x: f32, y: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            heading,
            ..ShipState::default()
        }
    }

    fn run_out(state: &ShipState) -> Controls {
        controls(state, Leg::RunOut, &HANDLING, Inertia::Inertial).expect("flies")
    }

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(RUN_OUT_THRUST, 3.0);
    }

    #[test]
    fn running_out_it_thrusts_facing_out_from_the_centre() {
        // Straight up from the centre is heading 0; to its right, 90.
        for (state, out) in [(at(0.0, -500.0, 0.0), 0.0), (at(500.0, 0.0, 90.0), 90.0)] {
            let flown = run_out(&state);
            assert!(flown.thrust, "{state:?}");
            assert_eq!(flown.turn, Turn::None);
            assert!(!flown.reverse);
            assert_eq!(state.heading, out);
        }
    }

    #[test]
    fn running_out_it_thrusts_only_off_the_bearing_by_less_than_a_turn_and_three_whole_degrees() {
        let off = HANDLING.turn_rate + RUN_OUT_THRUST;
        let edge = run_out(&at(0.0, -500.0, off));
        assert!(!edge.thrust, "7 degrees off is not within");
        assert_eq!(edge.turn, Turn::Left, "it turns back to the bearing");
        assert!(run_out(&at(0.0, -500.0, off - 0.01)).thrust, "6.99 reads 6");
        assert!(!run_out(&at(0.0, -500.0, 360.0 - off)).thrust);
        let right = run_out(&at(0.0, -500.0, 360.0 - off));
        assert_eq!(right.turn, Turn::Right);
        assert!(!run_out(&at(0.0, -500.0, 180.0)).thrust, "facing in");
    }

    #[test]
    fn at_the_centre_it_runs_out_straight_up() {
        assert!(run_out(&at(0.0, 0.0, 0.0)).thrust);
        assert_eq!(run_out(&at(0.0, 0.0, 90.0)).turn, Turn::Left);
    }

    #[test]
    fn braking_it_brakes_as_the_autopilots_stop_and_jumping_it_jumps_out() {
        let moving = ShipState {
            velocity: Vec2::new(0.0, -3.0),
            ..at(0.0, -1200.0, 180.0)
        };
        for inertia in [Inertia::Inertial, Inertia::Inertialess { speed: 3.0 }] {
            assert_eq!(
                controls(&moving, Leg::Brake, &HANDLING, inertia),
                Ok(autopilot::brake(&moving, &HANDLING, inertia))
            );
            assert_eq!(
                controls(&moving, Leg::Jump, &HANDLING, inertia),
                Err(Outcome::JumpedOut)
            );
        }
        let braking = controls(&moving, Leg::Brake, &HANDLING, Inertia::Inertial).expect("flies");
        assert!(braking.reverse && braking.thrust, "facing back, it thrusts");
    }

    // The leave flown through the autopilot.

    use crate::ai::Goal;
    use crate::combat::ShipRef;
    use crate::flight::FlightRules;
    use crate::fuel::OutfitMod;
    use crate::handling::ShipFields;
    use crate::stats::{HYPERSPACE_DISTANCE, INERTIALESS_HULL, ShipStats};
    use crate::traffic::autopilot::{Readings, TopSpeeds};
    use crate::traffic::npc::NpcPerson;
    use nova_sim_base::fixture::{Draws, FAST};
    use std::collections::BTreeMap;

    /// An expediting hull (`shïp` `Flags2` 0x0020).
    const EXPEDITING: u16 = 0x0020;

    fn ship(stats: ShipStats, goal: Goal, start: ShipState) -> Npc {
        Npc {
            state: start,
            goal,
            ..crate::traffic::fixture::npc(1, stats)
        }
    }

    fn moving(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// Flies `npc` one tick, its quarry at `other`, by `readings`.
    fn tick(npc: &mut Npc, other: Option<&ShipState>, readings: Readings) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        autopilot::fly(
            npc,
            &[],
            &BTreeMap::new(),
            other,
            false,
            speeds,
            FlightRules::default(),
            readings,
            &mut Draws::of(&[]),
        )
    }

    /// Flies `npc` until it leaves, at most `limit` ticks: each tick's
    /// starting state, and how it left.
    fn flown(npc: &mut Npc, other: Option<&ShipState>, limit: u32) -> (Vec<ShipState>, Outcome) {
        let mut states = Vec::new();
        for _ in 0..limit {
            states.push(npc.state);
            let outcome = tick(npc, other, Readings::default());
            if outcome != Outcome::Flying {
                return (states, outcome);
            }
        }
        (states, Outcome::Flying)
    }

    fn beyond(state: &ShipState) -> bool {
        let at = state.position;
        at.x * at.x + at.y * at.y > crate::ai::leave::LEAVE_REACH_SQ
    }

    fn below_035(state: &ShipState) -> bool {
        f64::from(state.velocity.x.abs()) < 0.35 && f64::from(state.velocity.y.abs()) < 0.35
    }

    /// The stock's fastest thrust: the Krypt Pod's `Accel` 1300.
    const KRYPT: ShipFields = ShipFields {
        accel: 1300,
        ..FAST
    };

    #[test]
    fn a_leaving_ship_flies_past_1000_brakes_below_035_per_axis_and_only_then_jumps() {
        let average = ShipFields {
            speed: 300,
            accel: 300,
            maneuver: 10,
            ..FAST
        };
        for fields in [average, FAST, KRYPT] {
            let start = moving(0.0, -500.0, 0.0, 0.0, 0.0);
            let mut leaver = ship(ShipStats::new(fields, &[]), Goal::JumpOut, start);
            let (states, outcome) = flown(&mut leaver, None, 3000);
            assert_eq!(outcome, Outcome::JumpedOut, "{fields:?}");
            let last = states.last().expect("flew");
            assert!(beyond(last), "past 1000: {last:?}");
            assert!(below_035(last), "braked: {last:?}");
            assert!(last.position.x.abs() < 1.0, "straight out: {last:?}");
            let past = states.iter().position(beyond).expect("past 1000");
            assert!(
                !below_035(&states[past]),
                "it came past 1000 moving: {:?}",
                states[past]
            );
            assert!(
                states[past..states.len() - 1].iter().all(|s| !below_035(s)),
                "it jumps on the first tick below 0.35"
            );
        }
    }

    #[test]
    fn an_expediting_ship_jumps_on_its_first_tick_beyond_1000_without_braking() {
        let hull = ShipFields {
            flags2: EXPEDITING,
            ..FAST
        };
        let start = moving(0.0, -990.0, 0.0, -6.0, 0.0);
        let mut leaver = ship(ShipStats::new(hull, &[]), Goal::JumpOut, start);
        let (states, outcome) = flown(&mut leaver, None, 10);
        assert_eq!(outcome, Outcome::JumpedOut);
        let last = states.last().expect("flew");
        assert!(beyond(last), "{last:?}");
        assert_eq!(
            states.iter().filter(|s| beyond(s)).count(),
            1,
            "the first tick"
        );
        assert!(last.velocity.y < -5.9, "at full speed: {last:?}");
    }

    #[test]
    fn a_ship_whose_outfits_widen_its_jump_distance_still_jumps_just_past_1000() {
        let wider = OutfitMod {
            mod_type: HYPERSPACE_DISTANCE,
            mod_val: 1000,
            count: 1,
        };
        let stats = ShipStats::new(FAST, &[wider]);
        assert_eq!(stats.jump_distance, 2000.0);
        let mut leaver = ship(stats, Goal::JumpOut, moving(0.0, -1000.5, 0.0, 0.0, 0.0));
        assert_eq!(
            tick(&mut leaver, None, Readings::default()),
            Outcome::JumpedOut
        );
        let mut inside = ship(stats, Goal::JumpOut, moving(0.0, -1000.0, 0.0, 0.0, 0.0));
        assert_eq!(
            tick(&mut inside, None, Readings::default()),
            Outcome::Flying
        );
    }

    #[test]
    fn an_inertialess_leaver_brakes_through_the_inertialess_brake_and_jumps() {
        let hull = ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        };
        let mut leaver = ship(
            ShipStats::new(hull, &[]),
            Goal::JumpOut,
            moving(0.0, -1100.0, 0.0, -6.0, 0.0),
        );
        leaver.speed = 6.0;
        let first = leaver.state;
        assert_eq!(
            tick(&mut leaver, None, Readings::default()),
            Outcome::Flying
        );
        assert!(leaver.speed < 6.0, "it brakes its speed: {}", leaver.speed);
        assert_eq!(leaver.state.heading, first.heading, "with no turn");
        let (states, outcome) = flown(&mut leaver, None, 3000);
        assert_eq!(outcome, Outcome::JumpedOut);
        assert!(below_035(states.last().expect("flew")));
    }

    /// A ship fleeing the player at (251, -10), beyond its reach, from
    /// `start`.
    fn fleeing(start: ShipState) -> (Npc, ShipState) {
        let attacker = moving(251.0, -10.0, 0.0, 0.0, 0.0);
        let runner = ship(
            ShipStats::new(FAST, &[]),
            Goal::Flee(ShipRef::Player),
            start,
        );
        (runner, attacker)
    }

    #[test]
    fn a_ship_fleeing_from_afar_runs_out_and_jumps_with_an_empty_tank_by_its_classs_fuel() {
        let (mut runner, attacker) = fleeing(moving(0.0, -10.0, 0.0, 0.0, 0.0));
        runner.reserves.fuel.now = 0.0;
        let (states, outcome) = flown(&mut runner, Some(&attacker), 3000);
        assert_eq!(outcome, Outcome::JumpedOut);
        let last = states.last().expect("flew");
        assert!(beyond(last) && below_035(last), "{last:?}");
        assert!(last.position.x.abs() < 1.0, "straight out: {last:?}");
    }

    #[test]
    fn a_fleeing_person_that_starts_with_zero_fuel_keeps_running_out_with_none() {
        let (mut runner, attacker) = fleeing(moving(0.0, -1001.0, 0.0, 0.0, 0.0));
        runner.reserves.fuel.now = 0.0;
        runner.person = Some(NpcPerson {
            zero_fuel: true,
            ..NpcPerson::of(&nova_sim_catalog::fixture::person(151, 128), false)
        });
        for _ in 0..60 {
            assert_eq!(
                tick(&mut runner, Some(&attacker), Readings::default()),
                Outcome::Flying
            );
        }
        assert!(
            runner.state.position.y < -1100.0,
            "runs on: {:?}",
            runner.state
        );
        runner.reserves.fuel.now = 100.0;
        let (_, outcome) = flown(&mut runner, Some(&attacker), 3000);
        assert_eq!(outcome, Outcome::JumpedOut, "with a jump's fuel it leaves");
    }

    #[test]
    fn by_the_other_reading_a_fleeing_ship_with_an_empty_tank_keeps_running_out() {
        let bible = Readings {
            jump_fuel: RuleSource::Bible,
            ..Readings::default()
        };
        let (mut runner, attacker) = fleeing(moving(0.0, -1001.0, 0.0, 0.0, 0.0));
        runner.reserves.fuel.now = 99.0;
        for _ in 0..60 {
            assert_eq!(tick(&mut runner, Some(&attacker), bible), Outcome::Flying);
        }
        let (mut fuelled, attacker) = fleeing(moving(0.0, -1001.0, 0.0, 0.0, 0.0));
        assert_eq!(
            tick(&mut fuelled, Some(&attacker), bible),
            Outcome::JumpedOut
        );
    }
}
