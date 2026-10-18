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
//! - [`Leg::Brake`] (sub 1, @0x8536d-0x85633): an inertial ship brakes as
//!   substate 1's inertial branch does (@0x85408-0x855e1, [`brake`]). It
//!   turns to its retro heading, the table angle of its velocity + 180
//!   (`_CalcInterceptAngle((0,0), velocity × 100.0)`), and thrusts while
//!   off it by less than its turn and [`BRAKE_THRUST_SLACK`], whole
//!   degrees, strictly. With an axis at [`FULL_BRAKE`] or more it thrusts
//!   its full thrust (@0x85510); below that on both, half of it
//!   ([`SOFT_BRAKE`]), its velocity cut to [`BRAKE_DAMPING`] of itself
//!   first (@0x85526-0x8558d). It has no cutoff near rest: it thrusts as
//!   long as the high level keeps it in sub 1, and may overshoot, turning
//!   about to brake the other way. Its half thrust and damping bring any
//!   thrust below 0.35 × (1 + 0.94) × 2, about 1.36 a tick (the stock's
//!   most is 0.43, the Krypt Pod's under a divisor of 3000), to below
//!   0.35 on both axes; a stronger one can settle swinging about the
//!   0.35, as the original's does. An inertialess ship brakes as the
//!   inertialess branch does
//!   ([`inertialess::brake`](crate::traffic::inertialess::brake)).
//!
//!   Left out of sub 1, as it can't run on a leave: its settle below 0.35
//!   on both axes (velocity × 0.95, sub 0, @0x855e3-0x85624), as the high
//!   level sets sub 1 only while an axis is at 0.35 or more and the low
//!   level reads the same velocity; and its damping off the retro heading
//!   (velocity × 0.94, @0x85592-0x855e1), which runs only under goal 9
//!   (+0xc8c8), not a leave's 2 or 3. Both, and the formation's use of sub
//!   1, are rdm:phase/escort-fleet-fidelity/phase-2-formation-flight's. Its
//!   cut of +0x48, the speed along the heading, with the velocity is
//!   rdm:task/ai-drags-scale-the-npc-speed's. Nothing here plays a sound.
//! - [`Leg::Jump`] (sub 4): it leaves the system that tick
//!   ([`Outcome::JumpedOut`]). Sub 4 is the warp out (`_LowLevelAIHandler`
//!   @0x8585e-0x859c5), which
//!   rdm:phase/ship-handling-fidelity/phase-8-warp-out ports; until then the
//!   ship departs as it reaches sub 4, so only the moment of departure
//!   differs from the original's. That phase replaces this arm.

use crate::ai::fire;
use crate::ai::leave::{self, Leg};
use crate::flight::{Controls, FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::rulebook::RuleSource;
use crate::traffic::autopilot::{self, Outcome, steer};
use crate::traffic::inertialess::{self, Inertia};
use crate::traffic::npc::Npc;
use crate::trig;

/// How far beyond a turn, in degrees, a ship running out still thrusts
/// (3.0 @0xdd664).
pub const RUN_OUT_THRUST: f32 = 3.0;
/// How far beyond a turn, in degrees, a braking ship still thrusts (the
/// float 1.0 @0xdd068).
pub const BRAKE_THRUST_SLACK: f32 = 1.0;
/// The speed on an axis from which a braking ship thrusts its full thrust
/// (the float 1.75 @0xdda38).
pub const FULL_BRAKE: f32 = 1.75;
/// The share of its thrust a braking ship thrusts below [`FULL_BRAKE`] on
/// both axes (the float 0.5 @0xdd694).
pub const SOFT_BRAKE: f32 = 0.5;
/// What a braking ship's velocity is cut to there, each tick it thrusts
/// (the double 0.94 @0xdda78).
pub const BRAKE_DAMPING: f64 = 0.94;

/// Flies `npc` one tick of `leg` on `handling` under `flight`, as
/// `inertia` says (see the module docs): flying on, or, on [`Leg::Jump`],
/// jumped out where it is.
pub fn fly(
    npc: &mut Npc,
    leg: Leg,
    handling: &Handling,
    flight: FlightRules,
    inertia: Inertia,
) -> Outcome {
    let (controls, flown) = match (leg, inertia) {
        (Leg::Jump, _) => return Outcome::JumpedOut,
        (Leg::RunOut, _) => (run_out(&npc.state, handling), *handling),
        (Leg::Brake, Inertia::Inertial) => brake(&mut npc.state, handling),
        (Leg::Brake, Inertia::Inertialess { .. }) => (inertialess::brake(&npc.state), *handling),
    };
    autopilot::step(npc, &flown, controls, flight, inertia);
    Outcome::Flying
}

/// Flies `npc` leaving (state 2) one tick on `handling` under `flight`,
/// as `inertia` says: [`leave::leg`]'s leg, by [`fly`].
pub fn leaving(
    npc: &mut Npc,
    handling: &Handling,
    flight: FlightRules,
    inertia: Inertia,
) -> Outcome {
    let leg = leave::leg(&npc.state, leave::expedites(npc));
    fly(npc, leg, handling, flight, inertia)
}

/// Flies `npc` fleeing beyond its attacker's reach (state 3's far branch)
/// one tick on `handling` under `flight`, as `inertia` says:
/// [`leave::flee_leg`]'s leg, whether it can jump read by `jump_fuel`
/// ([`leave::can_jump`]), by [`fly`].
pub fn fleeing_afar(
    npc: &mut Npc,
    handling: &Handling,
    flight: FlightRules,
    inertia: Inertia,
    jump_fuel: RuleSource,
) -> Outcome {
    let leg = leave::flee_leg(
        &npc.state,
        leave::expedites(npc),
        leave::can_jump(npc, jump_fuel),
    );
    fly(npc, leg, handling, flight, inertia)
}

/// Substate 1's brake for an inertial ship at `state` on `handling`
/// (@0x85408-0x855e1, see the module docs): the controls, and the
/// handling they fly by, its velocity cut as the substate cuts it.
fn brake(state: &mut ShipState, handling: &Handling) -> (Controls, Handling) {
    let moving = trig::intercept_angle(Vec2::ZERO, state.velocity * 100.0);
    // In 0..360.
    #[allow(clippy::cast_possible_truncation)]
    let retro = (i32::from(moving) + 180).rem_euclid(360) as i16;
    let thrust = fire::off_by(state.heading, retro) < handling.turn_rate + BRAKE_THRUST_SLACK;
    let controls = Controls {
        thrust,
        turn: steer(state.heading, f32::from(retro), handling),
        ..Controls::default()
    };
    let full = |axis: f32| axis.abs() >= FULL_BRAKE;
    if !thrust || full(state.velocity.x) || full(state.velocity.y) {
        return (controls, *handling);
    }
    // Doubles, back to floats (@0x85557-0x85574).
    #[allow(clippy::cast_possible_truncation)]
    let cut = |axis: f32| (f64::from(axis) * BRAKE_DAMPING) as f32;
    state.velocity = Vec2::new(cut(state.velocity.x), cut(state.velocity.y));
    let soft = Handling {
        accel: handling.accel * SOFT_BRAKE,
        ..*handling
    };
    (controls, soft)
}

/// Sub 3's controls for a ship at `state` on `handling`: turned to the
/// bearing out from the centre, thrusting within its turn rate and
/// [`RUN_OUT_THRUST`] of it.
fn run_out(state: &ShipState, handling: &Handling) -> Controls {
    let out = trig::intercept_angle(Vec2::ZERO, state.position);
    Controls {
        thrust: fire::off_by(state.heading, out).abs() < handling.turn_rate + RUN_OUT_THRUST,
        turn: steer(state.heading, f32::from(out), handling),
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
        super::run_out(state, &HANDLING)
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
    fn the_brakes_named_values_are_pinned() {
        assert_eq!(BRAKE_THRUST_SLACK, 1.0);
        assert_eq!(FULL_BRAKE, 1.75);
        assert_eq!(SOFT_BRAKE, 0.5);
        assert_eq!(BRAKE_DAMPING, 0.94);
    }

    /// A ship at `heading` moving at `(vx, vy)`, beyond 1000.
    fn braking(vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            velocity: Vec2::new(vx, vy),
            ..at(0.0, -1200.0, heading)
        }
    }

    #[test]
    fn braking_it_turns_to_its_retro_heading_and_thrusts_only_within_a_turn_and_one_whole_degree() {
        // Moving up (heading 0), its retro heading is 180.
        let slack = HANDLING.turn_rate + BRAKE_THRUST_SLACK;
        for (heading, thrusts, turn) in [
            (180.0, true, Turn::None),
            (180.0 - slack + 1.0, true, Turn::Right),
            (180.0 - slack + 0.99, false, Turn::Right),
            (180.0 - slack, false, Turn::Right),
            (180.0 + slack, false, Turn::Left),
            (90.0, false, Turn::Right),
        ] {
            let mut state = braking(0.0, -3.0, heading);
            let (flown, _) = brake(&mut state, &HANDLING);
            assert_eq!(flown.thrust, thrusts, "facing {heading}");
            assert_eq!(flown.turn, turn, "facing {heading}");
            assert!(!flown.reverse, "it steers to the heading itself");
            assert_eq!(state.velocity, Vec2::new(0.0, -3.0), "facing {heading}");
        }
        // Moving left (heading 270), it turns to 90.
        let mut left = braking(-3.0, 0.0, 90.0);
        assert!(brake(&mut left, &HANDLING).0.thrust);
    }

    #[test]
    fn braking_from_1_75_on_an_axis_it_thrusts_fully_and_below_it_at_half_cutting_its_velocity() {
        for (vx, vy, heading) in [(0.0, -1.75, 180.0), (-1.75, 0.01, 90.0)] {
            let mut state = braking(vx, vy, heading);
            let (flown, by) = brake(&mut state, &HANDLING);
            assert!(flown.thrust, "({vx}, {vy})");
            assert_eq!(by, HANDLING, "({vx}, {vy})");
            assert_eq!(state.velocity, Vec2::new(vx, vy), "({vx}, {vy})");
        }
        let below = f32::from_bits(1.75_f32.to_bits() - 1);
        let mut state = braking(0.0, -below, 180.0);
        let (flown, by) = brake(&mut state, &HANDLING);
        assert!(flown.thrust);
        assert_eq!(by.accel, HANDLING.accel * SOFT_BRAKE);
        assert_eq!((by.max_speed, by.turn_rate), (5.0, 4.0));
        #[allow(clippy::cast_possible_truncation)]
        let cut = (f64::from(-below) * BRAKE_DAMPING) as f32;
        assert_eq!(state.velocity, Vec2::new(0.0, cut));
        // Off its retro heading, it neither thrusts nor cuts.
        let mut turning = braking(0.0, -0.5, 0.0);
        assert!(!brake(&mut turning, &HANDLING).0.thrust);
        assert_eq!(turning.velocity, Vec2::new(0.0, -0.5));
    }

    #[test]
    fn jumping_it_jumps_out_where_it_is() {
        let start = braking(0.0, -3.0, 180.0);
        for inertia in [Inertia::Inertial, Inertia::Inertialess { speed: 3.0 }] {
            let mut leaver = ship(ShipStats::new(FAST, &[]), Goal::JumpOut, start);
            let outcome = fly(
                &mut leaver,
                Leg::Jump,
                &HANDLING,
                FlightRules::default(),
                inertia,
            );
            assert_eq!(outcome, Outcome::JumpedOut);
            assert_eq!(leaver.state, start);
        }
    }

    #[test]
    fn braking_an_inertial_ship_flies_substate_1s_brake_and_an_inertialess_one_the_inertialess_brake()
     {
        let start = braking(0.0, -1.0, 180.0);
        let mut inertial = ship(ShipStats::new(FAST, &[]), Goal::JumpOut, start);
        let rules = FlightRules::default();
        let outcome = fly(
            &mut inertial,
            Leg::Brake,
            &HANDLING,
            rules,
            Inertia::Inertial,
        );
        assert_eq!(outcome, Outcome::Flying);
        let mut by_hand = start;
        let (controls, by) = brake(&mut by_hand, &HANDLING);
        crate::flight::step(
            &mut by_hand,
            &by,
            controls,
            rules,
            crate::flight::Flyer::Npc,
        );
        assert_eq!(inertial.state, by_hand);
        let mut inertialess = ship(ShipStats::new(FAST, &[]), Goal::JumpOut, start);
        inertialess.speed = 1.0;
        let inertia = Inertia::Inertialess { speed: 1.0 };
        fly(&mut inertialess, Leg::Brake, &HANDLING, rules, inertia);
        let mut expected = ship(ShipStats::new(FAST, &[]), Goal::JumpOut, start);
        expected.speed = 1.0;
        let controls = crate::traffic::inertialess::brake(&start);
        autopilot::step(&mut expected, &HANDLING, controls, rules, inertia);
        assert_eq!(
            (inertialess.state, inertialess.speed),
            (expected.state, expected.speed)
        );
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
        tick_under(npc, other, FlightRules::default(), readings)
    }

    /// [`tick`] under the flight rules `flight`.
    fn tick_under(
        npc: &mut Npc,
        other: Option<&ShipState>,
        flight: FlightRules,
        readings: Readings,
    ) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        autopilot::fly(
            npc,
            &[],
            &BTreeMap::new(),
            other,
            false,
            speeds,
            flight,
            readings,
            &mut Draws::of(&[]),
        )
    }

    /// Flies `npc` until it leaves, at most `limit` ticks: each tick's
    /// starting state, and how it left.
    fn flown(npc: &mut Npc, other: Option<&ShipState>, limit: u32) -> (Vec<ShipState>, Outcome) {
        flown_under(npc, other, FlightRules::default(), limit)
    }

    /// [`flown`] under the flight rules `flight`.
    fn flown_under(
        npc: &mut Npc,
        other: Option<&ShipState>,
        flight: FlightRules,
        limit: u32,
    ) -> (Vec<ShipState>, Outcome) {
        let mut states = Vec::new();
        for _ in 0..limit {
            states.push(npc.state);
            let outcome = tick_under(npc, other, flight, Readings::default());
            if outcome != Outcome::Flying {
                return (states, outcome);
            }
        }
        (states, Outcome::Flying)
    }

    /// Flight rules whose thrust divisor is 1000: the Krypt Pod's `Accel`
    /// 1300 thrusts 1.3 a tick, above the 0.7 at which the autopilot's
    /// brake, stopping at half a thrust, could leave an axis at 0.35 or
    /// more.
    fn strong_thrust() -> FlightRules {
        FlightRules {
            thrust_divisor: std::num::NonZeroU32::new(1000).expect("non-zero"),
            ..FlightRules::default()
        }
    }

    #[test]
    fn a_leaver_thrusting_above_0_7_a_tick_still_brakes_below_035_and_jumps() {
        let krypt = ShipStats::new(KRYPT, &[]);
        assert!(strong_thrust().handling(krypt.handling).accel > 0.7);
        for heading in [0.0, 180.0] {
            let start = moving(0.0, -1100.0, 0.0, -0.5, heading);
            let mut leaver = ship(krypt, Goal::JumpOut, start);
            let (ticks, outcome) = flown_under(&mut leaver, None, strong_thrust(), 3000);
            assert_eq!(outcome, Outcome::JumpedOut, "facing {heading}");
            assert!(below_035(ticks.last().expect("flew")), "facing {heading}");
        }
    }

    #[test]
    fn a_ship_fleeing_from_afar_thrusting_above_0_7_a_tick_still_brakes_and_jumps() {
        let attacker = moving(251.0, -10.0, 0.0, 0.0, 0.0);
        let start = moving(0.0, -1100.0, 0.0, -0.5, 0.0);
        let mut runner = ship(
            ShipStats::new(KRYPT, &[]),
            Goal::Flee(ShipRef::Player),
            start,
        );
        let (states, outcome) = flown_under(&mut runner, Some(&attacker), strong_thrust(), 3000);
        assert_eq!(outcome, Outcome::JumpedOut);
        assert!(below_035(states.last().expect("flew")));
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

    /// `npc` alone in a system with no stellars, ticked once through
    /// [`Traffic::tick_in`](crate::traffic::Traffic::tick_in) by Nova's AI
    /// in a world reading `jump_fuel` as its
    /// [`npc_jump_fuel`](crate::traffic::World::npc_jump_fuel), the player
    /// at `player`: the NPC after the tick, or `None` once it departed, and
    /// how each ship that left did.
    fn traffic_tick(
        npc: Npc,
        player: Option<(f32, f32)>,
        jump_fuel: RuleSource,
    ) -> (Option<Npc>, Vec<Outcome>) {
        use crate::ai::NovaAi;
        use crate::traffic::{Traffic, World};

        let mut traffic = Traffic::new();
        let id = traffic.add_npc(npc);
        let world = World {
            player: player.map(|(x, y)| crate::ai::fixture::player(x, y)),
            npc_jump_fuel: jump_fuel,
            ..World::new(&[])
        };
        traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
        let after = traffic.npcs().iter().find(|npc| npc.id == id).cloned();
        let departed = traffic.departed().iter();
        assert!(departed.clone().all(|&(left, _)| left == id));
        (after, departed.map(|&(_, how)| how).collect())
    }

    #[test]
    fn the_traffic_tick_decides_an_idle_ships_jump_by_the_worlds_npc_jump_fuel() {
        let mut idle = ship(ShipStats::new(FAST, &[]), Goal::Idle, ShipState::default());
        idle.reserves.fuel.now = 0.0;
        assert_eq!(idle.class_fuel, 300);
        for (reading, goal) in [
            (RuleSource::Engine, Goal::JumpOut),
            (RuleSource::Bible, Goal::Idle),
        ] {
            let (after, _) = traffic_tick(idle.clone(), None, reading);
            assert_eq!(after.expect("still here").goal, goal, "{reading:?}");
        }
    }

    #[test]
    fn the_traffic_tick_flies_a_ship_fleeing_from_afar_by_the_worlds_npc_jump_fuel() {
        let (mut runner, attacker) = fleeing(moving(0.0, -1001.0, 0.0, 0.0, 0.0));
        runner.reserves.fuel.now = 0.0;
        // Provoked by the player, it decides to keep fleeing it.
        runner.provoked = 10.0;
        runner.target = Some(ShipRef::Player);
        assert_eq!(runner.class_fuel, 300);
        let player = Some((attacker.position.x, attacker.position.y));
        let (after, departed) = traffic_tick(runner.clone(), player, RuleSource::Engine);
        assert!(after.is_none(), "it left: {after:?}");
        assert_eq!(departed, [Outcome::JumpedOut]);
        let (after, departed) = traffic_tick(runner, player, RuleSource::Bible);
        assert_eq!(departed, []);
        let after = after.expect("it keeps flying");
        assert_eq!(after.goal, Goal::Flee(ShipRef::Player));
        assert!(beyond(&after.state), "{:?}", after.state);
    }

    #[test]
    fn a_hunter_giving_up_on_a_disabled_player_flies_out_past_1000_brakes_and_jumps() {
        use crate::ai::NovaAi;
        use crate::combat::hull::Condition;
        use crate::traffic::npc::MissionMark;
        use crate::traffic::{ShipBehav, Traffic, World};

        let mut hunter = ship(
            ShipStats::new(FAST, &[]),
            Goal::Attack(ShipRef::Player),
            moving(0.0, -600.0, 0.0, -2.0, 0.0),
        );
        hunter.target = Some(ShipRef::Player);
        hunter.ai_type = crate::traffic::npc::AiType::Warship;
        hunter.reserves.fuel.now = 0.0;
        hunter.mission = Some(MissionMark {
            slot: 1,
            behav: ShipBehav::AttackPlayer,
            formation: None,
            order: None,
        });
        let mut traffic = Traffic::new();
        let id = traffic.add_npc(hunter);
        let mut player = crate::ai::fixture::player(0.0, 0.0);
        player.condition = Condition::Disabled;
        let world = World {
            player: Some(player),
            ..World::new(&[])
        };
        let mut last = None;
        for _ in 0..3000 {
            traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
            if let Some(npc) = traffic.npcs().iter().find(|npc| npc.id == id) {
                assert_ne!(npc.goal, Goal::Attack(ShipRef::Player));
                assert_eq!(npc.target, None, "its target dropped");
                last = Some(npc.state);
                continue;
            }
            assert_eq!(traffic.departed(), [(id, Outcome::JumpedOut)]);
            let last = last.expect("it flew");
            assert!(last.position.length() > 1000.0, "{last:?}");
            assert!(below_035(&last), "it braked: {last:?}");
            return;
        }
        panic!("it never left");
    }
}
