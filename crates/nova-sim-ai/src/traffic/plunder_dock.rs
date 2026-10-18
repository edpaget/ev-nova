//! A pirate docking with the disabled ship it plunders: goal 0xd's
//! disabled half of `_HighLevelAIHandler` (@0x8e873-0x8ea44 in the `EV
//! Nova` executable, [`high_level`]) and the substates 9, 0xb and 0xf of
//! `_LowLevelAIHandler` it leaves the pirate in (@0x8769a-0x878f4,
//! @0x87337-0x87692, @0x86a08-0x8709a, [`fly`]). The docking it keeps from
//! tick to tick, its sub-goal and nav (+0xc8ca, +0x6c), is
//! [`Npc::docking`].
//!
//! - **The high level** ([`high_level`]), every think of a plunder whose
//!   victim is disabled: with the victim boarded (+0xb9) and no AI wait
//!   left, it gives up (goal 0, sub 0, target and nav -1, @0x8e88f-0x8e8b9);
//!   otherwise nav = target and, within [`dock_box`] of the victim on both
//!   axes, inclusive, it docks (sub 0xf); within twice the box it closes
//!   (sub 0xb); farther off it approaches (sub 9). A pirate's every-tick
//!   think finds a boarded victim boarded first (see
//!   [`pirate`](crate::ai::pirate)), so the give-up never runs for one.
//! - **Approach and close** (subs 9 and 0xb, [`DockSub::Approach`],
//!   [`DockSub::Close`]): outside [`STEER_BOX`] of the victim on either
//!   axis it turns to its bearing (`_CalcInterceptAngle`); inside on both,
//!   only when its velocity's direction is more than [`VELOCITY_SLACK`]
//!   degrees off the bearing (a raw subtraction of the two table angles,
//!   with no wrap) and its velocity is more than [`SPEED_SLACK`] off its
//!   top speed along the bearing on an axis does it turn to that
//!   difference; otherwise it keeps its heading (the low level's default
//!   +0x68, @0x8522d). It thrusts while off the heading it wants by less
//!   than its turn and [`THRUST_SLACK`], whole degrees. Closing, within
//!   [`CLOSE_CAP_BOX`] on both axes, it thrusts up to half its top speed
//!   ([`CLOSE_CAP`]). Both fire nothing: they end in
//!   `_AIEscortFireUnprovoked`, which does nothing for AI type 4 or below.
//! - **Dock** (sub 0xf, [`DockSub::Dock`]), while it is intact
//!   (`_LowLevelAIHandler`'s -0x32d flag, `!_IsDisabled`, @0x85240):
//!   - **Matching** while its velocity differs from the victim's by
//!     [`MATCH_SLACK`] or more on either axis (@0x86a30-0x86aa4): it turns
//!     against the difference (the table angle of the difference + 180,
//!     @0x86b3d-0x86ba0) and thrusts within its turn and
//!     [`THRUST_SLACK`]; with the difference at most [`DAMP_BELOW`] on
//!     either axis, it is cut to [`DAMPING`] of itself and the velocity set
//!     to the victim's plus it (@0x86c08-0x86cb2).
//!   - **Aligned** (@0x86cb7): it wants the victim's heading and takes its
//!     velocity. Outside [`DOCKED_BOX`] on either axis it creeps a pixel a
//!     tick towards it (@0x8700f-0x87095). Inside: while its AI wait is
//!     above [`DOCK_WAIT`] it holds; above none, under goal 0xd it would
//!     plunder ([`DockOutcome::Completed`], @0x86f3c-0x86f93: never reached
//!     by a pirate, whose think turns a boarded victim's plunder into goal
//!     4 first), under goal 4 nothing; at none, unless drifting (goal 0xe),
//!     it starts the dock (@0x86fa4-0x8700a): the victim boarded, its AI
//!     wait [`DOCK_WAIT`] + `Rand(`[`DOCK_WAIT_SPREAD`]`)`, its velocity the
//!     victim's ([`DockOutcome::Started`]).
//! - While its AI wait runs `_HandleShip` skips its turn and thrust
//!   (@0x34a54-0x34a72): it moves on at the velocity the substate left it.
//!
//! Left out, each owned elsewhere: the inertialess branch of sub 0xf
//! (@0x86aaa-0x86b38) and the inertialess box ×4 (@0x8e8f6-0x8e916) are
//! rdm:phase/ship-handling-fidelity/phase-19-inertialess-snipe-and-pursuit's;
//! sub 0xb's formation branch (@0x87364-0x873a3) is
//! rdm:phase/escort-fleet-fidelity/phase-2-formation-flight's; the repair
//! half of sub 0xf (the completion under goal 0xf, @0x86d84-0x86f37) is
//! rdm:phase/combat-ai-fidelity/phase-19-left-out-of-phase-10's. Nothing
//! here plays a sound.

use crate::ai::Goal;
use crate::ai::fire::off_by;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::flight::{self, Controls, ShipState};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::traffic::autopilot::steer;
use crate::traffic::npc::Npc;
use nova_sim_base::trig;

/// The turns a docking box is worked out from, less the class's turn (the
/// float 10.0 @0xdd65c).
pub const BOX_TURNS: f32 = 10.0;
/// The pixels of docking box per turn (the float 30.0 @0xdd084).
pub const BOX_PER_TURN: f32 = 30.0;
/// How near the victim, on each axis, the approach steers by its velocity
/// (the float 200.0 @0xdd088).
pub const STEER_BOX: f32 = 200.0;
/// How far its velocity's direction may be off the bearing before the
/// approach corrects it, in whole degrees (`cmpl $0xf` @0x874d2).
pub const VELOCITY_SLACK: i32 = 15;
/// How far its velocity may be off its top speed along the bearing, on an
/// axis, before it turns to the difference (the double 0.35 @0xdda68).
pub const SPEED_SLACK: f64 = 0.35;
/// How far beyond its turn, in degrees, a docking ship still thrusts (the
/// float 1.0 @0xdd068).
pub const THRUST_SLACK: f32 = 1.0;
/// How near the victim, on each axis, closing caps its speed (the float
/// 100.0 @0xdd060).
pub const CLOSE_CAP_BOX: f32 = 100.0;
/// The share of its top speed closing is capped at there (the float 0.5
/// @0xdd694).
pub const CLOSE_CAP: f32 = 0.5;
/// The velocity difference, on either axis, from which it matches the
/// victim's velocity (the double 0.525 @0xdda80).
pub const MATCH_SLACK: f64 = 0.524_999_999_999_999_9;
/// The velocity difference, on an axis, at or below which matching damps
/// it (the float 1.75 @0xdda38).
pub const DAMP_BELOW: f32 = 1.75;
/// What matching cuts the difference to (the double 0.95 @0xdd618).
pub const DAMPING: f64 = 0.95;
/// How near the victim, on each axis, it is docked (the float 3.0
/// @0xdd664).
pub const DOCKED_BOX: f32 = 3.0;
/// How far a tick it creeps towards the victim outside that, in pixels
/// (`_Accel` by the frame, @0x87057-0x87095).
pub const CREEP: f32 = 1.0;
/// The least of the dock's wait, and the wait above which it holds
/// (+0x4c = `Rand(80)` + 100 @0x86fcf-0x86fea; the float 100.0 @0xdd060).
pub const DOCK_WAIT: u32 = 100;
/// `Rand(80)`: the spread of the dock's wait.
pub const DOCK_WAIT_SPREAD: u16 = 80;

/// The docking a pirate keeps from tick to tick: the ship it docks with
/// (its nav, +0x6c) and the substate it flies (+0xc8ca).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Docking {
    /// The ship it docks with.
    pub ship: ShipRef,
    /// The substate it flies.
    pub sub: DockSub,
}

/// A docking's substate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSub {
    /// Sub 9: it approaches.
    Approach,
    /// Sub 0xb: it closes in, capped near the victim.
    Close,
    /// Sub 0xf: it matches the victim's velocity and docks.
    Dock,
}

/// The docking box of a class whose turn (+0x38) is `class_turn`:
/// trunc((10 − turn) × 30), in floats (@0x8e8c1-0x8e8f2).
#[must_use]
pub fn dock_box(class_turn: f32) -> i32 {
    // Within an i32 for any class's turn.
    #[allow(clippy::cast_possible_truncation)]
    let side = ((BOX_TURNS - class_turn) * BOX_PER_TURN) as i32;
    side
}

/// What the high level of a plunder of a disabled ship does (see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockStep {
    /// It gives up: goal 0, sub 0, target and nav -1.
    GiveUp,
    /// It flies this substate at the victim.
    Sub(DockSub),
}

/// The high level of `npc`'s plunder of a disabled victim at `victim`,
/// `boarded` saying whether the victim is boarded (see the module docs).
#[must_use]
pub fn high_level(npc: &Npc, victim: Vec2, boarded: bool) -> DockStep {
    if boarded && npc.ai_wait == 0 {
        return DockStep::GiveUp;
    }
    // f32: the box is converted back to a float for the test.
    #[allow(clippy::cast_precision_loss)]
    let side = dock_box(npc.hull.class_turn) as f32;
    let off = npc.state.position - victim;
    let within = |reach: f32| off.x.abs() <= reach && off.y.abs() <= reach;
    DockStep::Sub(if within(side) {
        DockSub::Dock
    } else if within(2.0 * side) {
        DockSub::Close
    } else {
        DockSub::Approach
    })
}

/// What became of a docking ship this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockOutcome {
    /// It flies on.
    Flying,
    /// It started the dock: the victim is boarded, and it waits `wait`.
    Started {
        /// The ship it docked with.
        victim: ShipRef,
        /// Its AI wait.
        wait: u32,
    },
    /// Docked under goal 0xd with its wait at most [`DOCK_WAIT`]: the
    /// original plunders here, which no pirate reaches (see the module
    /// docs).
    Completed,
}

/// Flies `npc` one tick of its docking with a victim at `victim`, on
/// `handling`, the dock's wait rolled on `chance` (see the module docs).
/// It flies nothing without a docking.
pub fn fly(
    npc: &mut Npc,
    victim: &ShipState,
    handling: &Handling,
    chance: &mut (impl Chance + ?Sized),
) -> DockOutcome {
    let Some(docking) = npc.docking else {
        return DockOutcome::Flying;
    };
    let (controls, flown, outcome) = match docking.sub {
        DockSub::Dock => {
            let (controls, outcome) = dock(npc, docking.ship, victim, handling, chance);
            (controls, *handling, outcome)
        }
        sub => {
            let (controls, flown) = approach(npc, victim, handling, sub == DockSub::Close);
            (controls, flown, DockOutcome::Flying)
        }
    };
    if npc.ai_wait > 0 {
        npc.state.position = npc.state.position + npc.state.velocity;
    } else {
        flight::step(&mut npc.state, &flown, controls);
    }
    outcome
}

/// Subs 9 and 0xb (see the module docs): the controls, and the handling
/// they fly by, closing (sub 0xb) capped within [`CLOSE_CAP_BOX`].
fn approach(
    npc: &Npc,
    victim: &ShipState,
    handling: &Handling,
    close: bool,
) -> (Controls, Handling) {
    let state = &npc.state;
    let bearing = trig::intercept_angle(state.position, victim.position);
    let off = state.position - victim.position;
    let within = |reach: f32| off.x.abs() <= reach && off.y.abs() <= reach;
    let wanted = if within(STEER_BOX) {
        let moving = trig::intercept_angle(Vec2::ZERO, state.velocity * 100.0);
        if (i32::from(moving) - i32::from(bearing)).abs() > VELOCITY_SLACK {
            let top = flight::facing(f32::from(bearing)) * handling.max_speed;
            let gap = top - state.velocity;
            if f64::from(gap.x.abs()) > SPEED_SLACK || f64::from(gap.y.abs()) > SPEED_SLACK {
                trig::intercept_angle(Vec2::ZERO, gap)
            } else {
                trig::whole(state.heading)
            }
        } else {
            trig::whole(state.heading)
        }
    } else {
        bearing
    };
    let thrust = off_by(state.heading, wanted) < handling.turn_rate + THRUST_SLACK;
    let flown = if thrust && close && within(CLOSE_CAP_BOX) {
        Handling {
            max_speed: handling.max_speed * CLOSE_CAP,
            ..*handling
        }
    } else {
        *handling
    };
    let controls = Controls {
        thrust,
        turn: steer(state.heading, f32::from(wanted), handling),
        reverse: false,
    };
    (controls, flown)
}

/// Sub 0xf (see the module docs): the controls, and what became of the
/// docking, its velocity and AI wait set as the substate sets them.
fn dock(
    npc: &mut Npc,
    ship: ShipRef,
    victim: &ShipState,
    handling: &Handling,
    chance: &mut (impl Chance + ?Sized),
) -> (Controls, DockOutcome) {
    let gap = npc.state.velocity - victim.velocity;
    let matching = |axis: f32| f64::from(axis.abs()) >= MATCH_SLACK;
    if matching(gap.x) || matching(gap.y) {
        let against = trig::intercept_angle(Vec2::ZERO, gap * 100.0);
        let wanted = (i32::from(against) + 180).rem_euclid(360);
        // In 0..360.
        #[allow(clippy::cast_possible_truncation)]
        let wanted = wanted as i16;
        if gap.x.abs() <= DAMP_BELOW || gap.y.abs() <= DAMP_BELOW {
            // Doubles, back to floats (@0x86c52-0x86c7c).
            #[allow(clippy::cast_possible_truncation)]
            let damped = Vec2::new(
                (f64::from(gap.x) * DAMPING) as f32,
                (f64::from(gap.y) * DAMPING) as f32,
            );
            npc.state.velocity = victim.velocity + damped;
        }
        let controls = Controls {
            thrust: off_by(npc.state.heading, wanted) < handling.turn_rate + THRUST_SLACK,
            turn: steer(npc.state.heading, f32::from(wanted), handling),
            reverse: false,
        };
        return (controls, DockOutcome::Flying);
    }
    let wanted = f32::from(trig::whole(victim.heading));
    let controls = Controls {
        thrust: false,
        turn: steer(npc.state.heading, wanted, handling),
        reverse: false,
    };
    npc.state.velocity = victim.velocity;
    let off = npc.state.position - victim.position;
    if off.x.abs() > DOCKED_BOX || off.y.abs() > DOCKED_BOX {
        let towards = trig::intercept_angle(npc.state.position, victim.position);
        npc.state.position = npc.state.position + flight::facing(f32::from(towards)) * CREEP;
        return (controls, DockOutcome::Flying);
    }
    let outcome = match npc.ai_wait {
        wait if wait > DOCK_WAIT => DockOutcome::Flying,
        1.. if matches!(npc.goal, Goal::Plunder { .. }) => DockOutcome::Completed,
        1.. => DockOutcome::Flying,
        0 if npc.goal == Goal::Drift => DockOutcome::Flying,
        0 => {
            let wait = DOCK_WAIT + u32::from(chance.roll(DOCK_WAIT_SPREAD));
            npc.ai_wait = wait;
            DockOutcome::Started { victim: ship, wait }
        }
    };
    (controls, outcome)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::NpcId;
    use crate::combat::hull::class_turn;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use nova_sim_base::fixture::{Draws, FAST, Scripted};

    const VICTIM: ShipRef = ShipRef::Npc(NpcId(9));

    /// Top speed 4, thrust 0.5, 3 degrees a tick.
    const HANDLING: Handling = Handling {
        max_speed: 4.0,
        accel: 0.5,
        turn_rate: 3.0,
    };

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// A pirate at `state` plundering the victim, docking by `sub`, its
    /// class turn 3 (a box of 210).
    fn pirate(state: ShipState, sub: DockSub) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.state = state;
        ship.goal = Goal::Plunder {
            ship: VICTIM,
            hopeless: false,
        };
        ship.hull.class_turn = 3.0;
        ship.docking = Some(Docking { ship: VICTIM, sub });
        ship
    }

    #[test]
    fn the_box_is_ten_less_the_class_turn_times_thirty_in_floats() {
        assert_eq!(dock_box(3.0), 210);
        assert_eq!(dock_box(8.0), 60);
        // In doubles (10 - 3.3) × 30 is 200.99999999999997.
        assert_eq!(dock_box(class_turn(33)), 201);
        assert_eq!(dock_box(10.5), -15);
    }

    #[test]
    fn the_high_level_docks_closes_or_approaches_each_axis_inclusive() {
        let step = |x: f32, y: f32| {
            high_level(
                &pirate(at(x, y, 0.0, 0.0, 0.0), DockSub::Approach),
                Vec2::ZERO,
                false,
            )
        };
        assert_eq!(step(210.0, -210.0), DockStep::Sub(DockSub::Dock));
        assert_eq!(step(210.5, 0.0), DockStep::Sub(DockSub::Close));
        assert_eq!(step(0.0, -210.5), DockStep::Sub(DockSub::Close));
        assert_eq!(step(420.0, 420.0), DockStep::Sub(DockSub::Close));
        assert_eq!(step(420.5, 0.0), DockStep::Sub(DockSub::Approach));
        assert_eq!(step(0.0, 420.5), DockStep::Sub(DockSub::Approach));
    }

    #[test]
    fn the_high_level_gives_up_on_a_boarded_victim_only_once_its_wait_is_out() {
        let mut ship = pirate(at(0.0, 0.0, 0.0, 0.0, 0.0), DockSub::Dock);
        assert_eq!(high_level(&ship, Vec2::ZERO, true), DockStep::GiveUp);
        ship.ai_wait = 1;
        assert_eq!(
            high_level(&ship, Vec2::ZERO, true),
            DockStep::Sub(DockSub::Dock)
        );
        ship.ai_wait = 0;
        assert_eq!(
            high_level(&ship, Vec2::ZERO, false),
            DockStep::Sub(DockSub::Dock)
        );
    }

    /// `ship` flown one tick at `victim`, and what became of it.
    fn flown(mut ship: Npc, victim: &ShipState, rolls: &[u16]) -> (Npc, DockOutcome) {
        let outcome = fly(&mut ship, victim, &HANDLING, &mut Scripted::rolling(rolls));
        (ship, outcome)
    }

    #[test]
    fn approaching_far_off_it_turns_to_the_bearing_and_thrusts_within_a_turn_and_one() {
        let victim = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        // Bearing 0; heading 3.9 is off by 3 whole degrees, 4.0 by 4.
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.0, 0.0, 3.9), DockSub::Approach),
            &victim,
            &[],
        );
        assert!((ship.state.heading - 0.9).abs() < 1e-5, "turned left");
        assert!(ship.state.velocity.y < 0.0, "thrust");
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.0, 0.0, 4.0), DockSub::Approach),
            &victim,
            &[],
        );
        assert_eq!(ship.state.velocity, Vec2::ZERO, "4 is not below 4");
    }

    #[test]
    fn approaching_near_it_keeps_its_heading_while_its_velocity_points_at_the_victim() {
        let victim = at(0.0, -150.0, 0.0, 0.0, 0.0);
        // Moving straight at it, facing across: it wants its own heading,
        // so it doesn't turn and thrusts along it.
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.0, -2.0, 90.0), DockSub::Approach),
            &victim,
            &[],
        );
        assert_eq!(ship.state.heading, 90.0);
        assert_eq!(ship.state.velocity, Vec2::new(0.5, -2.0));
        // Moving across it: it turns to make up its velocity to the top
        // speed along the bearing, (0, -4) - (2, 0), up and to the left.
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 2.0, 0.0, 90.0), DockSub::Approach),
            &victim,
            &[],
        );
        assert_eq!(ship.state.heading, 87.0, "turning towards 333");
    }

    #[test]
    fn the_velocity_test_subtracts_the_two_angles_with_no_wrap() {
        // The velocity's direction just left of the bearing, the bearing
        // just right of 0: far apart by the raw subtraction, near by the
        // wrap, so it corrects.
        let victim = at(5.0, -100.0, 0.0, 0.0, 0.0);
        let moving = at(0.0, 0.0, -0.05, -1.0, 180.0);
        let bearing = trig::intercept_angle(moving.position, victim.position);
        let direction = trig::intercept_angle(Vec2::ZERO, moving.velocity * 100.0);
        assert!((i32::from(direction) - i32::from(bearing)).abs() > VELOCITY_SLACK);
        assert!(i32::from(trig::angular_difference(direction, bearing)) <= VELOCITY_SLACK);
        let (ship, _) = flown(pirate(moving, DockSub::Approach), &victim, &[]);
        assert_ne!(ship.state.heading, 180.0, "it corrects");
        // Just right of the bearing too: it keeps its heading.
        let near = at(0.0, 0.0, 0.09, -1.0, 180.0);
        let direction = trig::intercept_angle(Vec2::ZERO, near.velocity * 100.0);
        assert!((i32::from(direction) - i32::from(bearing)).abs() <= VELOCITY_SLACK);
        let (ship, _) = flown(pirate(near, DockSub::Approach), &victim, &[]);
        assert_eq!(ship.state.heading, 180.0);
    }

    #[test]
    fn closing_caps_its_thrust_at_half_its_top_speed_within_100_and_not_outside() {
        let start = at(0.0, 0.0, 0.0, -1.9, 0.0);
        let near = at(0.0, -100.0, 0.0, 0.0, 0.0);
        let (ship, _) = flown(pirate(start, DockSub::Close), &near, &[]);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, -2.0), "capped at 2");
        let (ship, _) = flown(pirate(start, DockSub::Approach), &near, &[]);
        assert_eq!(
            ship.state.velocity.y, -2.4,
            "sub 9 has no cap but its top speed"
        );
        let far = at(0.0, -100.5, 0.0, 0.0, 0.0);
        let (ship, _) = flown(pirate(start, DockSub::Close), &far, &[]);
        assert_eq!(ship.state.velocity.y, -2.4);
    }

    #[test]
    fn docking_it_turns_against_its_velocity_off_the_victims_and_thrusts_within_a_turn_and_one() {
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 3 apart going down: it wants to face up (0), facing it.
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 0.0, 3.0, 0.0), DockSub::Dock),
            &victim,
            &[],
        );
        assert_eq!(ship.state.heading, 0.0);
        assert_eq!(
            ship.state.velocity,
            Vec2::new(0.0, 2.35),
            "damped by x's 0 to 2.85, then thrust"
        );
        // 2 apart on y, 0 on x: damped to 0.95 of it before thrust.
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 0.0, 2.0, 0.0), DockSub::Dock),
            &victim,
            &[],
        );
        assert_eq!(ship.state.velocity, Vec2::new(0.0, 1.9 - 0.5));
        // Both axes above 1.75: no damping.
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 2.0, 2.0, 130.0), DockSub::Dock),
            &victim,
            &[],
        );
        assert_eq!(ship.state.heading, 127.0, "turning to 315");
        assert_eq!(
            ship.state.velocity,
            Vec2::new(2.0, 2.0),
            "130 is far off 315: no thrust"
        );
    }

    #[test]
    fn docking_matches_from_0_525_on_either_axis() {
        let victim = at(0.0, 0.0, 1.0, 0.0, 90.0);
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 1.524, 0.0, 0.0), DockSub::Dock),
            &victim,
            &[],
        );
        assert_eq!(ship.state.velocity, victim.velocity, "aligned below 0.525");
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 1.0, 0.53, 0.0), DockSub::Dock),
            &victim,
            &[],
        );
        assert_ne!(ship.state.velocity, victim.velocity, "matching");
    }

    #[test]
    fn aligned_it_takes_the_victims_velocity_and_heading_and_creeps_a_pixel_outside_3() {
        let victim = at(0.0, 0.0, 0.5, 0.0, 12.7);
        let (ship, outcome) = flown(
            pirate(at(0.0, 10.0, 0.5, 0.2, 10.0), DockSub::Dock),
            &victim,
            &[],
        );
        assert_eq!(outcome, DockOutcome::Flying);
        assert_eq!(ship.state.heading, 13.0, "turning to 12");
        assert_eq!(ship.state.velocity, victim.velocity);
        assert_eq!(
            ship.state.position,
            Vec2::new(0.5, 9.0),
            "a pixel up, then its velocity"
        );
        assert_eq!(ship.ai_wait, 0, "no dock outside 3");
    }

    #[test]
    fn inside_3_with_no_wait_it_starts_the_dock_for_100_to_179_ticks() {
        let victim = at(0.0, 0.0, 0.5, 0.0, 0.0);
        for (roll, wait) in [(0, 100), (79, 179)] {
            let (ship, outcome) = flown(
                pirate(at(3.0, -3.0, 0.5, 0.0, 0.0), DockSub::Dock),
                &victim,
                &[roll],
            );
            assert_eq!(
                outcome,
                DockOutcome::Started {
                    victim: VICTIM,
                    wait
                }
            );
            assert_eq!(ship.ai_wait, wait);
            assert_eq!(
                ship.state.position,
                Vec2::new(3.5, -3.0),
                "moved on, its turn and thrust skipped"
            );
        }
        let mut chance = Scripted::rolling(&[0]);
        let mut ship = pirate(at(0.0, 0.0, 0.5, 0.0, 0.0), DockSub::Dock);
        fly(&mut ship, &victim, &HANDLING, &mut chance);
        assert_eq!(chance.sides_asked, [DOCK_WAIT_SPREAD]);
    }

    #[test]
    fn docked_it_holds_above_100_completes_only_under_a_plunder_and_never_starts_drifting() {
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let docked = |goal: Goal, wait: u32| {
            let mut ship = pirate(at(1.0, 1.0, 0.0, 0.0, 0.0), DockSub::Dock);
            ship.goal = goal;
            ship.ai_wait = wait;
            flown(ship, &victim, &[]).1
        };
        let plunder = Goal::Plunder {
            ship: VICTIM,
            hopeless: false,
        };
        assert_eq!(docked(plunder, 101), DockOutcome::Flying);
        assert_eq!(docked(plunder, 100), DockOutcome::Completed);
        assert_eq!(docked(plunder, 1), DockOutcome::Completed);
        assert_eq!(docked(Goal::Attack(VICTIM), 100), DockOutcome::Flying);
        assert_eq!(docked(Goal::Attack(VICTIM), 1), DockOutcome::Flying);
        assert_eq!(docked(Goal::Drift, 0), DockOutcome::Flying);
        assert!(matches!(
            docked(Goal::Attack(VICTIM), 0),
            DockOutcome::Started { .. }
        ));
    }

    #[test]
    fn with_no_docking_it_flies_nothing() {
        let mut ship = pirate(at(0.0, 0.0, 1.0, 0.0, 0.0), DockSub::Dock);
        ship.docking = None;
        let before = ship.clone();
        let outcome = fly(
            &mut ship,
            &at(0.0, 0.0, 0.0, 0.0, 0.0),
            &HANDLING,
            &mut Draws::of(&[]),
        );
        assert_eq!((ship, outcome), (before, DockOutcome::Flying));
    }
}
