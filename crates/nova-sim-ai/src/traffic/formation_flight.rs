//! An escort keeping formation, goal 0xa, as the `EV Nova` executable
//! flies it: `_HighLevelAIHandler`'s goal 0xa (@0x8ed15-0x8ef37), which
//! picks the substate ([`sub`]), and the substates of `_LowLevelAIHandler`
//! it leaves the escort in, with `_AIMaintainFormation`
//! (@0x84f04-0x8503f, [`maintain`]). It flies the player's escort
//! ([`Goal::Formation`]) and, by the engine's reading of
//! [`RuleKey::NpcFormation`](crate::RuleKey::NpcFormation), a fleet
//! escort keeping with its NPC lead ([`Goal::Follow`]), each on the slot
//! its goal keeps ([`Npc::formation_slot`]): the player's formation's,
//! or the NPC parent's, which the formation pass works out
//! ([`fleet_formation`](crate::traffic::fleet_formation)). It flies by
//! the engine's reading of
//! [`RuleKey::FormationFlight`](crate::RuleKey::FormationFlight)
//! ([`FormationFlight::Original`](crate::flight::FormationFlight::Original));
//! the other reading is the port's earlier formation, which the
//! [`autopilot`](crate::traffic::autopilot) keeps.
//!
//! - **The substate** ([`sub`], goal 0xa @0x8ed15-0x8ef37): measured from
//!   the escort's **leader**, never its slot, inclusive on each axis.
//!   Seeing its leader (`_ShipVisibleToShip(leader, ship)` @0x8ed5f), it
//!   closes formation within [`KEEP_FORMATION`] (sub 0xc, @0x8ee8e),
//!   closes in within [`APPROACH_FORMATION`] (sub 0xb, @0x8ef29) and
//!   approaches beyond (sub 9, @0x8ef37). Not seeing it, it stops within
//!   [`KEEP_FORMATION`] (sub 1, @0x8edfc) and approaches beyond. The
//!   leader's sight is the fly pass's to give
//!   ([`Beside::leader_seen`]).
//! - **Approach and close** (subs 9 and 0xb, @0x8769a-0x878f4,
//!   @0x87337-0x87692) fly as a pirate's docking does
//!   ([`plunder_dock::approach`]), steering at the escort's guard (+0x70)
//!   while it has one there, and at its leader (nav +0x6c) otherwise:
//!   the 200 and 100 boxes are measured from the ship it steers at, while
//!   the high level picks the substate by the leader. Closing caps the
//!   thrust at half its top speed within 100 of it. With a swarm leader
//!   (+0xc906 > 0), sub 0xb first nudges the escort toward its slot
//!   ([`maintain`]) and copies the swarm leader's glow
//!   ([`plunder_dock::swarm_branch`], @0x87364-0x873a3). `_HandleShip`
//!   turns it to the heading it wants as in close formation
//!   ([`flight::turn_to`]).
//! - **Stop** (sub 1, @0x8536d-0x85633): [`leave_flight::stop`], which
//!   brakes a moving escort and settles a slow one, turned as
//!   `_HandleShip` turns every substate ([`flight::turn_to`]).
//! - **Close formation** (sub 0xc, @0x878fc-0x87e01, [`keep`]): Δv is its
//!   velocity less its leader's. Its own `_ShipIsInertialess` test
//!   (@0x87962) runs with the substate at 0 (@0x8792b), so it reads the
//!   hull and dampener alone ([`Npc::hull_inertialess`]); the flight that
//!   follows reads substate 0xc and is inertial
//!   ([`Npc::keeps_close_formation`]).
//!   - **Counter-thrust**, not inertialess by that test and Δv
//!     [`MATCH_SLACK`] or more on an axis (@0x8796f-0x87b4a): it wants the
//!     heading against Δv (`_CalcInterceptAngle((0,0), Δv × 100)` + 180),
//!     thrusts at [`COUNTER_THRUST`] times its thrust, capped at its own
//!     top speed, while off that heading by less than its turn and
//!     [`THRUST_SLACK`] whole degrees, and with Δv at most [`DAMP_BELOW`]
//!     on either axis cuts Δv to [`DAMPING`] of itself, its velocity its
//!     leader's plus that. It skips the heading step, the nudge and the
//!     glow copy.
//!   - **Matched** (@0x87b4f-0x87c7a): it takes its leader's velocity. It
//!     wants its whole heading a degree toward its leader's while it is
//!     off it by more than none and less than the limit, trunc(
//!     [`HEADING_LIMIT`] / its skill) ([`Npc::skill`]), or
//!     [`HEADING_LIMIT`] with a skill of none or less; otherwise its
//!     leader's whole heading. Then it is nudged toward its slot
//!     ([`maintain`], @0x87dbb-0x87dc9) and copies its leader's glow
//!     (@0x87dce-0x87deb): the player's, or a fleet escort's NPC lead's
//!     as the fly pass began (see [`fly_pass`](crate::traffic::fly_pass)).
//!   - `_HandleShip` then turns it to the heading it wants ([`flight::turn_to`],
//!     @0x34460-0x3458e): by its turn while it is off by more than that,
//!     whole degrees, and onto it otherwise. So it steps a degree a
//!     frame while it is close to its leader's heading.
//! - **The nudge** ([`maintain`], `_AIMaintainFormation(ship, 0)`): the
//!   nudge is [`FORMATION_NUDGE`] times its `_ShipAccelRate`, the thrust
//!   it flies by (ionized, under the thrust divisor), cut to (1 − f) with
//!   an ion charge above none, f its ionization factor held to
//!   [`ION_HOLD`]. Each axis [`FORMATION_SLACK`] or more off its slot,
//!   inclusive, moves by the whole nudge toward it, which can carry it
//!   past. Nothing moves while its warp stage runs. The nudge comes
//!   before `_HandleShip`'s move, so it is nudged, then moves.
//! - The slot is where the engine puts it ([`FormationSlot::position`]),
//!   from the leader as the fly pass found it, after the player's move
//!   (an NPC lead's, as the fly half began; see
//!   [`fleet_formation`](crate::traffic::fleet_formation) for how that
//!   differs from the original).
//!   The original works the player's escorts' slots out in
//!   `_HandlePlayer`, after the player's move
//!   (`_AICalcFormationPositions(player, 0)` @0x6f1df), which runs before
//!   `_AIDispatch` (`_DoPlayGameWork` @0x43e19, @0x4429d): the same
//!   place.
//! - None of these routines plays a sound or draws a random number.
//!
//! # Not built
//!
//! - Goal 0xc (@0x8ef40-0x8f0fb), the goal `_AIMakeEscortFlyInForm` gives
//!   a hired escort: `_EscortAI` sets goal 0xa again on every think
//!   (@0x83da3, @0x83e40, @0x83f00), and at full speed every frame is a
//!   think, so the high level never sees it. Its player sight (@0x8efba)
//!   goes with it.
//! - Goal 0xa's +0x8c gate, as a defence-fleet ship runs
//!   `_DefenseShipAI`, not `_EscortAI`; its leader -1 gate, as a ship of
//!   goal 0xa never flies it with no leader: `_AICheckParentExists` moves
//!   a lost lead's followers to its successor or out of the goal
//!   ([`succession`](crate::traffic::succession)), and `_EscortAI`'s lost
//!   parent resets a leaderless ship to goal 0 (@0x83942-0x8397b,
//!   [`Decision::leaves_fleet`](crate::ai::Decision::leaves_fleet)). Here
//!   an escort with no leader or slot brakes, and one in the regroup pause
//!   keeping formation with no leader holds ([`Goal::flown`]).
//! - `_AIMaintainFormation`'s snap (snap 1) is passed only by
//!   `_AICalcFormationPositions(parent, 1)` from `_SetupShipsInSystem`
//!   and `_HyperSpawnFleet`, which the session's arrival places and the
//!   traffic's arrival snap
//!   ([`fleet_formation::snap`](crate::traffic::fleet_formation::snap))
//!   cover.
//!   Its formation-parent gate (+0xc92e above 63) stands as the slot:
//!   a ship with none is never nudged.
//!
//! # Left out
//!
//! Sub 0xc's inspection hold under goal 7 (@0x87c7e-0x87db9) and goal 5's
//! return flight (@0x8ebd7-0x8ed0c) are
//! rdm:phase/escort-fleet-fidelity-followups/phase-2-left-out-of-formation-flight's.

#[cfg(doc)]
use crate::ai::Goal;
use crate::ai::fire::off_by;
#[cfg(doc)]
use crate::escort::FormationSlot;
use crate::escort::{APPROACH_FORMATION, FORMATION_NUDGE, FORMATION_SLACK, KEEP_FORMATION};
use crate::flight::{self, Controls, FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::traffic::autopilot::Beside;
use crate::traffic::held_burst::{self, Word};
use crate::traffic::inertialess::Inertia;
use crate::traffic::leave_flight;
use crate::traffic::npc::Npc;
use crate::traffic::plunder_dock::{self, THRUST_SLACK};
#[cfg(doc)]
use crate::traffic::plunder_dock::{DAMP_BELOW, DAMPING, MATCH_SLACK};
use crate::traffic::ship_display::GlowCopy;
use crate::trig;

/// The share of its thrust close formation counter-thrusts at (the double
/// 0.66 @0xdd5b8, @0x87a8c).
pub const COUNTER_THRUST: f64 = 0.66;
/// The skill's heading limit: a matched escort steps its heading while it
/// is off its leader's by less than this over its skill (the float 25.0
/// @0xdda40), or this with a skill of none or less (`movl $0x19`
/// @0x87b8f).
pub const HEADING_LIMIT: f32 = 25.0;
/// The ionization factor above which the nudge's cut is held (the double
/// 0.8 @0xdd6f8, @0x84f7d).
pub const ION_HOLD_ABOVE: f64 = 0.8;
/// The factor it is held to there (the float 0.8 @0xdda2c, @0x84f87).
pub const ION_HOLD: f32 = 0.8;
/// The substate goal 0xa flies (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    /// Sub 1: it stops, its leader unseen.
    Stop,
    /// Sub 9: it approaches at full speed.
    Approach,
    /// Sub 0xb: it closes in, capped near the ship it steers at.
    Close,
    /// Sub 0xc: close formation.
    Keep,
}

/// The substate an escort at `at` flies beside its leader at `leader`,
/// which it sees when `leader_seen` (goal 0xa, see the module docs).
#[must_use]
pub fn sub(at: Vec2, leader: Vec2, leader_seen: bool) -> Sub {
    let off = at - leader;
    let within = |reach: f32| off.x.abs() <= reach && off.y.abs() <= reach;
    match (leader_seen, within(KEEP_FORMATION)) {
        (false, true) => Sub::Stop,
        (true, true) => Sub::Keep,
        (true, false) if within(APPROACH_FORMATION) => Sub::Close,
        _ => Sub::Approach,
    }
}

/// `_AIMaintainFormation` (see the module docs): `npc` nudged toward
/// `slot` by its thrust on `handling`, or put on it when `snap`.
pub fn maintain(npc: &mut Npc, slot: Vec2, handling: &Handling, snap: bool) {
    if snap {
        npc.state.position = slot;
        return;
    }
    if npc.warp.under_way() {
        return;
    }
    let mut nudge = handling.accel * FORMATION_NUDGE;
    if npc.ion.charge > 0.0 {
        let factor = f64::from(npc.ion_factor());
        let held = if factor > ION_HOLD_ABOVE {
            f64::from(ION_HOLD)
        } else {
            factor
        };
        // Doubles, back to a float (@0x84f93-0x84fa7).
        #[allow(clippy::cast_possible_truncation)]
        let cut = (f64::from(nudge) * (1.0 - held)) as f32;
        nudge = cut;
    }
    let towards = |at: f32, to: f32| {
        if to - FORMATION_SLACK >= at {
            at + nudge
        } else if at >= to + FORMATION_SLACK {
            at - nudge
        } else {
            at
        }
    };
    let at = npc.state.position;
    npc.state.position = Vec2::new(towards(at.x, slot.x), towards(at.y, slot.y));
}

/// Sub 0xc for `npc` beside its leader at `leader`, its slot at `slot`,
/// on `handling` under `flight` (see the module docs): the controls, and
/// the handling they fly by, its velocity, heading, position and glow
/// copy set as the substate sets them.
fn keep(
    npc: &mut Npc,
    leader: &ShipState,
    slot: Vec2,
    handling: &Handling,
    flight: FlightRules,
) -> (Controls, Handling) {
    let rate = handling.turn_rate;
    let mismatched = plunder_dock::mismatched(npc.state.velocity - leader.velocity);
    if mismatched && !npc.hull_inertialess(flight.inertial_dampener) {
        let wanted = plunder_dock::counter(&mut npc.state, leader.velocity);
        // Off by its heading before `_HandleShip` turns it (@0x87a24).
        let thrust = off_by(npc.state.heading, wanted) < rate + THRUST_SLACK;
        let turn = flight::turn_to(&mut npc.state, wanted, rate);
        // `_ShipAccelRate` × the double 0.66, back to a float, and +0x34 =
        // 0, its own top speed (@0x87a70-0x87a9d).
        #[allow(clippy::cast_possible_truncation)]
        let accel = (f64::from(handling.accel) * COUNTER_THRUST) as f32;
        let controls = Controls {
            thrust,
            turn,
            ..Controls::default()
        };
        return (controls, Handling { accel, ..*handling });
    }
    npc.state.velocity = leader.velocity;
    let wanted = matched_heading(npc.state.heading, leader.heading, npc.skill);
    let turn = flight::turn_to(&mut npc.state, wanted, rate);
    maintain(npc, slot, handling, false);
    npc.display.drive.glow = GlowCopy::Leader;
    let controls = Controls {
        turn,
        ..Controls::default()
    };
    (controls, *handling)
}

/// The whole heading a matched escort at `heading` of skill `skill` wants
/// beside a leader at `theirs` (@0x87b74-0x87c7a, see the module docs):
/// a degree toward the leader's while off it by more than none and less
/// than its skill's limit, otherwise the leader's whole heading.
fn matched_heading(heading: f32, theirs: f32, skill: f32) -> i16 {
    let whole = trig::whole(heading);
    let off = trig::angular_difference(whole, trig::whole(theirs));
    // trunc(25.0 / skill) above a skill of 0.0 (@0xdd054), else 25. The
    // original compares the limit's low word; it isn't replicated.
    #[allow(clippy::cast_possible_truncation)]
    let limit = if skill > 0.0 {
        (HEADING_LIMIT / skill) as i32
    } else {
        HEADING_LIMIT as i32
    };
    if !(off > 0 && limit > i32::from(off)) {
        return trig::whole(theirs);
    }
    // trunc(heading − theirs), brought round once each way
    // (@0x87bf8-0x87c19).
    #[allow(clippy::cast_possible_truncation)]
    let mut way = (heading - theirs) as i32;
    if way >= 360 {
        way -= 360;
    }
    if way <= -1 {
        way += 360;
    }
    let mut wanted = if way > 180 {
        i32::from(whole) + 1
    } else {
        i32::from(whole) - 1
    };
    if wanted > 359 {
        wanted -= 360;
    }
    if wanted < 0 {
        wanted += 360;
    }
    // In 0..360.
    #[allow(clippy::cast_possible_truncation)]
    let wanted = wanted as i16;
    wanted
}

/// Flies `npc` one tick of goal 0xa beside its leader at `leader`, with
/// `beside` its guard, its sight of its leader and its swarm lead, on
/// `handling` under `flight`, as `inertia` says (see the module docs).
/// With no leader or no slot it brakes.
pub(crate) fn fly(
    npc: &mut Npc,
    leader: Option<&ShipState>,
    beside: Beside,
    handling: &Handling,
    flight: FlightRules,
    inertia: Inertia,
) {
    let flying = match (leader, npc.formation_slot()) {
        (Some(leader), Some(place)) => {
            let sub = sub(npc.state.position, leader.position, beside.leader_seen);
            (sub != Sub::Stop).then_some((leader, place, sub))
        }
        _ => None,
    };
    // With no leader or slot, or in sub 1: a stop, which keeps +0x34.
    let Some((leader, place, sub)) = flying else {
        let stop = leave_flight::stop_flight(npc, handling, inertia);
        let flown = stop.flown.unwrap_or(*handling);
        held_burst::step(npc, &flown, stop.controls, flight, inertia, stop.word);
        return;
    };
    let slot = place.position(leader);
    let (controls, flown) = match sub {
        Sub::Keep => keep(npc, leader, slot, handling, flight),
        sub => {
            let close = sub == Sub::Close;
            if close {
                plunder_dock::swarm_branch(npc, beside.swarm, Some(slot), handling);
            }
            let target = beside.guard.unwrap_or(*leader);
            let (controls, flown, wanted) = plunder_dock::approach(npc, &target, handling, close);
            let turn = flight::turn_to(&mut npc.state, wanted, handling.turn_rate);
            (Controls { turn, ..controls }, flown)
        }
    };
    held_burst::step(npc, &flown, controls, flight, inertia, Word::Writes);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::ai::swarm::SwarmLead;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::flight::Turn;
    use crate::handling::ShipFields;
    use crate::stats::{INERTIALESS_HULL, ShipStats};
    use crate::traffic::autopilot::{self, Outcome, Readings, TopSpeeds};
    use nova_sim_base::fixture::{Draws, FAST};
    use std::collections::BTreeMap;

    /// Top speed 4, thrust 0.5, 3 degrees a tick.
    const HANDLING: Handling = Handling {
        max_speed: 4.0,
        accel: 0.5,
        turn_rate: 3.0,
    };

    /// A nudge of 0.5 a tick.
    const GENTLE: Handling = Handling {
        accel: 0.05,
        ..HANDLING
    };

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// An escort of `fields` in slot 2 of 3, spaced `spacing` apart, from
    /// `start`, keeping formation.
    fn escort_of(fields: ShipFields, spacing: f32, start: ShipState) -> Npc {
        Npc {
            state: start,
            goal: Goal::Formation { guard: None },
            escort: Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 3,
                    spacing,
                },
                order: None,
            }),
            ..crate::traffic::fixture::npc(1, ShipStats::new(fields, &[]))
        }
    }

    /// A [`FAST`] escort spaced 30 apart from `start`.
    fn escort(start: ShipState) -> Npc {
        escort_of(FAST, 30.0, start)
    }

    /// Slot 2 of 3, spaced 30 apart, behind `leader`.
    fn slot_of(leader: &ShipState) -> Vec2 {
        crate::escort::slot_position(leader, 2, 3, 30.0)
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-5
    }

    /// `npc` flown a tick by the autopilot beside its leader at `leader`,
    /// with `beside`, under the original's rules.
    fn flown(npc: &mut Npc, leader: Option<&ShipState>, beside: Beside) -> Outcome {
        flown_under(npc, leader, beside, FlightRules::default())
    }

    /// `npc` flown a tick by the autopilot beside its leader at `leader`,
    /// with `beside`, under `rules`.
    fn flown_under(
        npc: &mut Npc,
        leader: Option<&ShipState>,
        beside: Beside,
        rules: FlightRules,
    ) -> Outcome {
        autopilot::fly(
            npc,
            &[],
            &BTreeMap::new(),
            leader,
            false,
            beside,
            TopSpeeds::own_of(npc),
            rules,
            Readings::default(),
            &crate::ai::NO_WEAPONS,
            &mut Draws::of(&[]),
        )
    }

    // The substate.

    #[test]
    fn the_substate_goes_by_the_leaders_distance_on_each_axis_inclusive() {
        let leader = Vec2::new(100.0, -50.0);
        let off = |x: f32, y: f32| leader + Vec2::new(x, y);
        assert_eq!(sub(off(300.0, -300.0), leader, true), Sub::Keep);
        assert_eq!(sub(off(-300.0, 300.0), leader, true), Sub::Keep);
        assert_eq!(sub(off(300.01, 0.0), leader, true), Sub::Close);
        assert_eq!(sub(off(0.0, -300.01), leader, true), Sub::Close);
        assert_eq!(sub(off(600.0, -600.0), leader, true), Sub::Close);
        assert_eq!(sub(off(600.01, 0.0), leader, true), Sub::Approach);
        assert_eq!(sub(off(0.0, -600.01), leader, true), Sub::Approach);
    }

    #[test]
    fn with_its_leader_unseen_it_stops_within_300_and_approaches_beyond() {
        let leader = Vec2::new(100.0, -50.0);
        let off = |x: f32, y: f32| leader + Vec2::new(x, y);
        assert_eq!(sub(off(300.0, -300.0), leader, false), Sub::Stop);
        assert_eq!(sub(off(301.0, 0.0), leader, false), Sub::Approach);
        assert_eq!(sub(off(0.0, 450.0), leader, false), Sub::Approach);
    }

    // The nudge.

    /// An escort at `at`, nudged once toward a slot at (100, 200) on
    /// `handling`: where it ends.
    fn nudged_from(at: Vec2, handling: &Handling) -> Vec2 {
        let mut ship = escort(ShipState {
            position: at,
            ..ShipState::default()
        });
        maintain(&mut ship, Vec2::new(100.0, 200.0), handling, false);
        ship.state.position
    }

    #[test]
    fn the_nudge_moves_each_axis_from_8_off_inclusive_by_ten_ticks_thrust() {
        assert_eq!(
            nudged_from(Vec2::new(108.0, 192.0), &GENTLE),
            Vec2::new(107.5, 192.5)
        );
        assert_eq!(
            nudged_from(Vec2::new(92.0, 208.0), &GENTLE),
            Vec2::new(92.5, 207.5)
        );
        assert_eq!(
            nudged_from(Vec2::new(107.99, 192.01), &GENTLE),
            Vec2::new(107.99, 192.01),
            "within the slack"
        );
        assert_eq!(
            nudged_from(Vec2::new(109.0, 202.0), &GENTLE),
            Vec2::new(108.5, 202.0),
            "each axis on its own"
        );
        assert_eq!(
            nudged_from(Vec2::new(100.0, 180.0), &GENTLE),
            Vec2::new(100.0, 180.5)
        );
    }

    #[test]
    fn a_strong_nudge_carries_it_past_the_slot() {
        // 3.0 a tick: a nudge of 30.
        let strong = Handling {
            accel: 3.0,
            ..HANDLING
        };
        assert_eq!(
            nudged_from(Vec2::new(109.0, 180.0), &strong),
            Vec2::new(79.0, 210.0)
        );
    }

    /// An escort with an `IonizeMax` of 100 holding `charge`, 9 off the
    /// slot at (100, 200) on x: where the nudge leaves it.
    fn ionized_nudge(charge: f32) -> f32 {
        let mut ship = escort_of(
            ShipFields {
                ionize_max: 100,
                ..FAST
            },
            30.0,
            at(109.0, 200.0, 0.0, 0.0, 0.0),
        );
        ship.ion.charge = charge;
        maintain(&mut ship, Vec2::new(100.0, 200.0), &GENTLE, false);
        ship.state.position.x
    }

    #[test]
    fn an_ion_charge_cuts_the_nudge_by_its_factor_held_to_0_8() {
        assert_eq!(ionized_nudge(0.0), 108.5);
        assert_eq!(ionized_nudge(50.0), 108.75, "halved");
        #[allow(clippy::cast_possible_truncation)]
        let held = (f64::from(0.5_f32) * (1.0 - f64::from(0.8_f32))) as f32;
        assert_eq!(ionized_nudge(90.0), 109.0 - held, "0.9 held to 0.8");
        assert_eq!(ionized_nudge(-50.0), 108.5, "no charge, no cut");
        assert_eq!(
            (COUNTER_THRUST, HEADING_LIMIT, ION_HOLD_ABOVE, ION_HOLD),
            (0.66, 25.0, 0.8, 0.8)
        );
    }

    #[test]
    fn nothing_is_nudged_while_its_warp_runs_but_a_snap_puts_it_on_the_slot() {
        let slot = Vec2::new(100.0, 200.0);
        let mut ship = escort(at(150.0, 120.0, 0.0, 0.0, 0.0));
        ship.warp.stage = 1.0;
        maintain(&mut ship, slot, &GENTLE, false);
        assert_eq!(ship.state.position, Vec2::new(150.0, 120.0));
        maintain(&mut ship, slot, &GENTLE, true);
        assert_eq!(ship.state.position, slot, "even while warping");
    }

    // Close formation: the counter-thrust.

    /// An escort at `heading` moving at `velocity`, kept a tick beside a
    /// leader at rest heading 90, its slot where it is: the controls and
    /// handling, and the escort.
    fn kept(velocity: Vec2, heading: f32) -> (Controls, Handling, Npc) {
        let leader = at(0.0, 0.0, 0.0, 0.0, 90.0);
        let mut ship = escort(at(50.0, 0.0, velocity.x, velocity.y, heading));
        let (controls, flown) = keep(
            &mut ship,
            &leader,
            Vec2::new(50.0, 0.0),
            &HANDLING,
            FlightRules::default(),
        );
        (controls, flown, ship)
    }

    #[test]
    fn counter_thrusting_it_faces_against_its_velocity_off_its_leaders_at_two_thirds_thrust() {
        let (controls, flown, ship) = kept(Vec2::new(1.0, 0.0), 270.0);
        assert!(controls.thrust);
        assert_eq!(controls.turn, Turn::None, "on 270 already");
        assert_eq!(ship.state.heading, 270.0);
        #[allow(clippy::cast_possible_truncation)]
        let accel = (0.5_f64 * 0.66) as f32;
        assert_eq!(flown, Handling { accel, ..HANDLING });
        assert_eq!(ship.state.velocity, Vec2::new(0.95, 0.0), "damped");
        // Facing up, it turns left to 270 and holds its thrust.
        let (controls, _, ship) = kept(Vec2::new(1.0, 0.0), 0.0);
        assert_eq!((controls.thrust, controls.turn), (false, Turn::Left));
        assert_eq!(ship.state.heading, 0.0);
    }

    #[test]
    fn it_thrusts_only_strictly_within_its_turn_and_one_whole_degree() {
        let (controls, ..) = kept(Vec2::new(1.0, 0.0), 274.0);
        assert!(!controls.thrust, "4 off");
        let (controls, ..) = kept(Vec2::new(1.0, 0.0), 273.9);
        assert!(controls.thrust, "3 whole degrees off");
        let (controls, ..) = kept(Vec2::new(1.0, 0.0), 266.0);
        assert!(!controls.thrust);
    }

    #[test]
    fn it_damps_with_either_axis_at_1_75_or_below() {
        let (_, _, ship) = kept(Vec2::new(2.0, 2.0), 0.0);
        assert_eq!(ship.state.velocity, Vec2::new(2.0, 2.0), "no damping");
        let (_, _, ship) = kept(Vec2::new(1.75, -3.0), 0.0);
        #[allow(clippy::cast_possible_truncation)]
        let damped = |axis: f32| (f64::from(axis) * 0.95) as f32;
        assert_eq!(ship.state.velocity, Vec2::new(damped(1.75), damped(-3.0)));
    }

    #[test]
    fn it_counter_thrusts_from_the_double_0_525_on_an_axis() {
        // The float 0.525 widens below the double; the next float up is
        // above it.
        let above = f32::from_bits(0.525_f32.to_bits() + 1);
        let (controls, _, ship) = kept(Vec2::new(0.525, 0.0), 270.0);
        assert!(!controls.thrust);
        assert_eq!(ship.state.velocity, Vec2::ZERO, "matched: its leader's");
        let (controls, _, ship) = kept(Vec2::new(0.0, above), 0.0);
        assert!(controls.thrust, "facing against it");
        assert_ne!(ship.state.velocity, Vec2::ZERO);
        let (_, _, ship) = kept(Vec2::new(0.52, -0.52), 0.0);
        assert_eq!(ship.state.velocity, Vec2::ZERO);
    }

    #[test]
    fn counter_thrusting_it_is_neither_nudged_nor_given_its_leaders_glow() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 90.0);
        let mut ship = escort(at(50.0, 0.0, 1.0, 0.0, 270.0));
        keep(
            &mut ship,
            &leader,
            Vec2::new(80.0, 30.0),
            &HANDLING,
            FlightRules::default(),
        );
        assert_eq!(ship.state.position, Vec2::new(50.0, 0.0));
        assert_eq!(ship.display.drive.glow, GlowCopy::None);
    }

    // Close formation: matched.

    /// An escort at `heading` of skill `skill` matched with a leader at
    /// rest heading `theirs`, kept a tick on `rate`: its heading after
    /// `_HandleShip`'s turn.
    fn stepped(heading: f32, theirs: f32, skill: f32, rate: f32) -> f32 {
        let leader = at(0.0, 0.0, 0.0, 0.0, theirs);
        let mut ship = escort(at(50.0, 0.0, 0.0, 0.0, heading));
        ship.skill = skill;
        let handling = Handling {
            turn_rate: rate,
            ..HANDLING
        };
        let (controls, ..) = keep(
            &mut ship,
            &leader,
            Vec2::new(50.0, 0.0),
            &handling,
            FlightRules::default(),
        );
        crate::flight::turned(ship.state.heading, controls.turn, rate)
    }

    #[test]
    fn matched_it_steps_its_heading_a_degree_toward_its_leaders() {
        assert_eq!(stepped(10.0, 13.0, 1.0, 3.0), 11.0);
        assert_eq!(stepped(13.0, 10.0, 1.0, 3.0), 12.0);
        assert_eq!(stepped(359.0, 2.0, 1.0, 3.0), 0.0, "across 0");
        assert_eq!(stepped(1.0, 358.0, 1.0, 3.0), 0.0);
        assert_eq!(stepped(0.0, 358.0, 1.0, 3.0), 359.0);
        assert_eq!(stepped(10.4, 10.9, 1.0, 3.0), 10.0, "on its whole degree");
        assert_eq!(stepped(10.0, 34.0, 1.0, 3.0), 11.0, "24 is within 25");
        assert_eq!(stepped(11.0, 10.0, 1.0, 3.0), 10.0, "a degree above");
        assert_eq!(stepped(11.5, 10.0, 1.0, 3.0), 10.0);
        // Half a turn off, with a limit past it (a skill of 0.1): the
        // degree below its own.
        assert_eq!(stepped(190.0, 10.0, 0.1, 3.0), 189.0);
        assert_eq!(stepped(10.0, 190.0, 0.1, 3.0), 9.0);
    }

    #[test]
    fn matched_it_turns_to_its_leaders_whole_heading_from_its_skills_limit() {
        // Skill 5: a limit of 5.
        assert_eq!(stepped(10.0, 14.0, 5.0, 3.0), 11.0, "4 steps");
        assert_eq!(stepped(10.0, 15.5, 5.0, 3.0), 13.0, "5 turns at its rate");
        assert_eq!(stepped(10.0, 15.5, 5.0, 6.0), 15.0, "onto it in one turn");
        assert_eq!(stepped(10.0, 35.0, 1.0, 3.0), 13.0, "25 turns");
        // A skill of none or less: a limit of 25.
        for skill in [0.0, -1.0] {
            assert_eq!(stepped(10.0, 34.0, skill, 3.0), 11.0, "{skill}");
            assert_eq!(stepped(10.0, 35.0, skill, 3.0), 13.0, "{skill}");
        }
        // Skill 2: trunc(12.5), a limit of 12.
        assert_eq!(stepped(10.0, 21.0, 2.0, 3.0), 11.0);
        assert_eq!(stepped(10.0, 22.0, 2.0, 3.0), 13.0);
    }

    #[test]
    fn matched_it_takes_its_leaders_velocity_is_nudged_and_copies_its_glow() {
        let leader = at(0.0, 0.0, 1.0, -2.0, 90.0);
        let mut ship = escort(at(120.0, 0.0, 1.2, -2.3, 90.0));
        let (controls, flown) = keep(
            &mut ship,
            &leader,
            Vec2::new(100.0, 0.0),
            &GENTLE,
            FlightRules::default(),
        );
        assert_eq!(ship.state.velocity, leader.velocity);
        assert_eq!(ship.state.position, Vec2::new(119.5, 0.0));
        assert_eq!(ship.display.drive.glow, GlowCopy::Leader);
        assert!(!controls.thrust);
        assert_eq!(flown, GENTLE);
    }

    #[test]
    fn an_inertialess_hull_matches_at_once_and_flies_inertial_in_close_formation() {
        let raven = ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        };
        let leader = at(0.0, 0.0, 0.0, 0.0, 90.0);
        let mut ship = escort_of(raven, 30.0, at(50.0, 0.0, 3.0, 0.0, 90.0));
        assert_eq!(
            Inertia::of(&ship, Some(&leader), true, FlightRules::default()),
            Inertia::Inertial
        );
        keep(
            &mut ship,
            &leader,
            Vec2::new(50.0, 0.0),
            &HANDLING,
            FlightRules::default(),
        );
        assert_eq!(ship.state.velocity, Vec2::ZERO);
    }

    // The flight.

    #[test]
    fn on_its_slot_with_another_velocity_it_converges_on_its_leaders_without_copying_it_at_once() {
        let mut leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_of(&leader);
        let mut ship = escort(at(slot.x, slot.y, 2.0, -2.0, 30.0));
        flown(&mut ship, Some(&leader), Beside::default());
        assert_ne!(ship.state.velocity, leader.velocity, "not on tick one");
        let mut matched = false;
        for _ in 0..200 {
            leader.position = leader.position + leader.velocity;
            flown(&mut ship, Some(&leader), Beside::default());
            matched |= ship.state.velocity == leader.velocity;
        }
        assert!(matched, "{:?}", ship.state);
        assert_eq!(ship.state.velocity, leader.velocity);
        let off = ship.state.position - slot_of(&leader);
        assert!(off.x.abs() < 10.0 && off.y.abs() < 10.0, "{off:?}");
    }

    #[test]
    fn matched_its_heading_closes_a_degree_a_tick_and_it_moves_with_its_leader() {
        let mut leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_of(&leader);
        let mut ship = escort(at(slot.x, slot.y, 1.0, -2.0, 27.0));
        let mut headings = Vec::new();
        for _ in 0..4 {
            let before = ship.state.position;
            flown(&mut ship, Some(&leader), Beside::default());
            assert_eq!(ship.state.position, before + leader.velocity);
            assert_eq!(ship.display.drive.glow, GlowCopy::Leader);
            headings.push(ship.state.heading);
            leader.position = leader.position + leader.velocity;
        }
        assert_eq!(headings, [28.0, 29.0, 30.0, 30.0]);
    }

    #[test]
    fn with_its_leader_unseen_nearby_it_brakes_then_settles() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let unseen = Beside {
            leader_seen: false,
            ..Beside::default()
        };
        // Moving down, facing up: its retro heading, so it brakes.
        let mut ship = escort(at(100.0, 100.0, 0.0, 2.0, 0.0));
        flown(&mut ship, Some(&leader), unseen);
        assert!(ship.state.velocity.y < 2.0, "{:?}", ship.state);
        assert_eq!(ship.display.drive.glow, GlowCopy::None);
        // Slow on both axes: it settles, with no thrust or turn.
        let mut ship = escort(at(100.0, 100.0, 0.2, -0.1, 45.0));
        flown(&mut ship, Some(&leader), unseen);
        #[allow(clippy::cast_possible_truncation)]
        let settled = |axis: f32| (f64::from(axis) * 0.95) as f32;
        assert_eq!(ship.state.velocity, Vec2::new(settled(0.2), settled(-0.1)));
        assert_eq!(ship.state.heading, 45.0);
    }

    #[test]
    fn closing_in_it_steers_at_its_guard_while_it_has_one_and_else_at_its_leader_not_its_slot() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // Spaced 300 apart, its slot lies down and to the side of its
        // leader, off the bearing to the leader.
        let start = at(450.0, 0.0, 0.0, 0.0, 250.0);
        let slot = crate::escort::slot_position(&leader, 2, 3, 300.0);
        let to_slot = trig::intercept_angle(start.position, slot);
        assert!(to_slot < 250, "{to_slot}");
        let mut ship = escort_of(FAST, 300.0, start);
        flown(&mut ship, Some(&leader), Beside::default());
        assert_eq!(ship.state.heading, 253.0, "toward 270, its leader");
        // A guard straight down, at 180: it turns toward it.
        let guard = at(450.0, 1000.0, 0.0, 0.0, 0.0);
        let guarding = Beside {
            guard: Some(guard),
            ..Beside::default()
        };
        let mut ship = escort_of(FAST, 300.0, start);
        flown(&mut ship, Some(&leader), guarding);
        assert_eq!(ship.state.heading, 247.0, "{:?}", ship.state);
    }

    #[test]
    fn closing_in_or_approaching_it_turns_as_handle_ship_does_onto_a_heading_within_its_turn() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // Its leader at 270, 1.4 off: within half a turn, yet
        // `_HandleShip` puts it on 270.
        for x in [450.0, 900.0] {
            let mut ship = escort_of(FAST, 300.0, at(x, 0.0, 0.0, 0.0, 271.4));
            flown(&mut ship, Some(&leader), Beside::default());
            assert_eq!(ship.state.heading, 270.0, "{x}");
        }
    }

    #[test]
    fn closing_in_within_100_of_its_guard_it_thrusts_to_half_its_top_speed() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // FAST: a top speed of 6, half of it 3; moving up at 3, facing up.
        let start = at(450.0, 0.0, 0.0, -3.0, 0.0);
        let guard = at(450.0, -90.0, 0.0, 0.0, 0.0);
        let guarding = Beside {
            guard: Some(guard),
            ..Beside::default()
        };
        let mut ship = escort(start);
        flown(&mut ship, Some(&leader), guarding);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, -3.0), "capped");
        // Beyond 600 of its leader it approaches, with no cap.
        let far = at(0.0, 700.0, 0.0, 0.0, 0.0);
        let mut ship = escort(start);
        flown(&mut ship, Some(&far), guarding);
        assert!(ship.state.velocity.y < -3.0, "{:?}", ship.state);
    }

    #[test]
    fn closing_in_with_a_swarm_leader_it_is_nudged_to_its_slot_and_takes_its_glow() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_of(&leader);
        // At rest, facing away from its leader: no thrust.
        let start = at(450.0, 0.0, 0.0, 0.0, 90.0);
        let swarm = Beside {
            swarm: Some(SwarmLead {
                aim: Vec2::ZERO,
                leads_it: false,
                glow: 12,
            }),
            ..Beside::default()
        };
        let mut ship = escort(start);
        ship.swarm_leader = Some(5);
        flown(&mut ship, Some(&leader), swarm);
        // FAST: 0.18 a tick, a nudge of 1.8 toward the slot on each axis.
        let nudge = 0.18_f32 * 10.0;
        let toward = |at: f32, to: f32| at + nudge.copysign(to - at);
        assert!(
            near(
                ship.state.position,
                Vec2::new(toward(450.0, slot.x), toward(0.0, slot.y))
            ),
            "{:?}",
            ship.state
        );
        assert_eq!(ship.display.drive.glow, GlowCopy::Swarm(12));
        // With no swarm leader, neither.
        let mut ship = escort(start);
        flown(&mut ship, Some(&leader), Beside::default());
        assert_eq!(ship.state.position, start.position);
        assert_eq!(ship.display.drive.glow, GlowCopy::None);
    }

    #[test]
    fn an_ionized_escort_is_nudged_by_its_ionized_thrust_cut_again_by_its_factor() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 90.0);
        let slot = slot_of(&leader);
        let mut ship = escort_of(
            ShipFields {
                ionize_max: 100,
                ..FAST
            },
            30.0,
            at(slot.x + 20.0, slot.y, 0.0, 0.0, 90.0),
        );
        ship.ion.charge = 50.0;
        let handling = ship.flying_handling(TopSpeeds::own_of(&ship).own, FlightRules::default());
        assert!(handling.accel < 0.18, "the ionized thrust: {handling:?}");
        flown(&mut ship, Some(&leader), Beside::default());
        #[allow(clippy::cast_possible_truncation)]
        let nudge = (f64::from(handling.accel * 10.0) * 0.5) as f32;
        assert_eq!(ship.state.position.x, slot.x + 20.0 - nudge);
    }

    // A fleet escort ([`Goal::Follow`]).

    /// A [`FAST`] fleet escort following NPC 7 from `start`, in slot 2 of
    /// 3 of its lead's formation, spaced `spacing` apart; it holds a place
    /// in the player's formation too (slot 3, spaced 60), which it doesn't
    /// keep.
    fn fleet_escort_of(spacing: f32, start: ShipState) -> Npc {
        let lead = crate::traffic::npc::NpcId(7);
        Npc {
            goal: Goal::Follow { lead, guard: None },
            leader: Some(lead),
            fleet_slot: Some(FormationSlot {
                slot: 2,
                ships: 3,
                spacing,
            }),
            escort: Some(EscortDuty {
                formation: FormationSlot {
                    slot: 3,
                    ships: 3,
                    spacing: 60.0,
                },
                order: None,
            }),
            ..escort(start)
        }
    }

    /// The original's flight rules but the port's earlier NPC follow.
    fn earlier_follow() -> FlightRules {
        FlightRules {
            npc_formation: crate::flight::NpcFormation::Earlier,
            ..FlightRules::default()
        }
    }

    #[test]
    fn a_matched_fleet_escort_takes_its_leads_velocity_steps_its_heading_and_is_nudged_to_its_fleet_slot()
     {
        let leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_of(&leader);
        let mut ship = fleet_escort_of(30.0, at(slot.x + 20.0, slot.y, 1.0, -2.0, 27.0));
        flown(&mut ship, Some(&leader), Beside::default());
        assert_eq!(ship.state.velocity, leader.velocity);
        assert_eq!(ship.state.heading, 28.0);
        assert_eq!(ship.display.drive.glow, GlowCopy::Leader);
        // FAST: 0.18 a tick, a nudge of 1.8 on x alone, then its move.
        let nudged = Vec2::new(slot.x + 20.0 - 0.18 * 10.0, slot.y);
        assert!(
            near(ship.state.position, nudged + leader.velocity),
            "{:?}",
            ship.state
        );
    }

    #[test]
    fn a_fleet_escort_450_off_closes_at_its_guard_and_beyond_600_approaches() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let start = at(450.0, 0.0, 0.0, 0.0, 250.0);
        let mut ship = fleet_escort_of(300.0, start);
        flown(&mut ship, Some(&leader), Beside::default());
        assert_eq!(ship.state.heading, 253.0, "toward 270, its lead");
        let guarding = Beside {
            guard: Some(at(450.0, 1000.0, 0.0, 0.0, 0.0)),
            ..Beside::default()
        };
        let mut ship = fleet_escort_of(300.0, start);
        flown(&mut ship, Some(&leader), guarding);
        assert_eq!(ship.state.heading, 247.0, "toward its guard");
        // Moving up at 3, half its top speed, its guard 90 above: capped
        // closing in, not approaching beyond 600.
        let start = at(450.0, 0.0, 0.0, -3.0, 0.0);
        let guarding = Beside {
            guard: Some(at(450.0, -90.0, 0.0, 0.0, 0.0)),
            ..Beside::default()
        };
        let mut ship = fleet_escort_of(30.0, start);
        flown(&mut ship, Some(&leader), guarding);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, -3.0), "capped");
        let mut ship = fleet_escort_of(30.0, start);
        flown(&mut ship, Some(&at(0.0, 700.0, 0.0, 0.0, 0.0)), guarding);
        assert!(ship.state.velocity.y < -3.0, "{:?}", ship.state);
    }

    #[test]
    fn a_fleet_escort_beside_a_lead_it_cannot_see_brakes_then_settles() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let unseen = Beside {
            leader_seen: false,
            ..Beside::default()
        };
        let mut ship = fleet_escort_of(30.0, at(100.0, 100.0, 0.0, 2.0, 0.0));
        flown(&mut ship, Some(&leader), unseen);
        assert!(ship.state.velocity.y < 2.0, "{:?}", ship.state);
        assert_eq!(ship.display.drive.glow, GlowCopy::None);
        let mut ship = fleet_escort_of(30.0, at(100.0, 100.0, 0.2, -0.1, 45.0));
        flown(&mut ship, Some(&leader), unseen);
        #[allow(clippy::cast_possible_truncation)]
        let settled = |axis: f32| (f64::from(axis) * 0.95) as f32;
        assert_eq!(ship.state.velocity, Vec2::new(settled(0.2), settled(-0.1)));
        assert_eq!(ship.state.heading, 45.0);
    }

    #[test]
    fn by_the_earlier_follow_a_fleet_escort_follows_within_100_of_its_lead() {
        // It flies as a ship inspecting its lead does, and not as the
        // original's formation does.
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let start = at(250.0, 0.0, 0.0, 0.0, 0.0);
        let mut follower = fleet_escort_of(30.0, start);
        let mut inspector = Npc {
            goal: Goal::Inspect(crate::combat::ShipRef::Npc(crate::traffic::npc::NpcId(7))),
            ..follower.clone()
        };
        let mut formation = follower.clone();
        flown(&mut formation, Some(&leader), Beside::default());
        let mut closest = f32::MAX;
        for tick in 0..300 {
            flown_under(
                &mut follower,
                Some(&leader),
                Beside::default(),
                earlier_follow(),
            );
            flown_under(
                &mut inspector,
                Some(&leader),
                Beside::default(),
                earlier_follow(),
            );
            assert_eq!(follower.state, inspector.state, "tick {tick}");
            closest = closest.min((follower.state.position - leader.position).length());
        }
        assert!(
            closest <= autopilot::FOLLOW_DISTANCE,
            "never closed: {closest}"
        );
        let mut first = fleet_escort_of(30.0, start);
        flown_under(
            &mut first,
            Some(&leader),
            Beside::default(),
            earlier_follow(),
        );
        assert_ne!(first.state, formation.state, "not the formation flight");
    }

    #[test]
    fn an_escort_with_no_leader_or_no_slot_brakes() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut lost = escort(at(0.0, 100.0, 0.0, 2.0, 180.0));
        flown(&mut lost, None, Beside::default());
        assert_ne!(lost.state.heading, 180.0, "{:?}", lost.state);
        let mut dutiless = escort(at(0.0, 100.0, 0.0, 2.0, 180.0));
        dutiless.escort = None;
        flown(&mut dutiless, Some(&leader), Beside::default());
        assert_ne!(dutiless.state.heading, 180.0, "{:?}", dutiless.state);
    }

    #[test]
    fn an_escort_with_no_leader_flies_sub_1_settling_below_0_35() {
        // Moving 0.2 a tick, facing back: sub 1 settles it × 0.95 with no
        // thrust (@0x855e3-0x85624), with no cutoff short of 0.35 above.
        let mut lost = escort(at(0.0, 100.0, 0.0, -0.2, 180.0));
        flown(&mut lost, None, Beside::default());
        #[allow(clippy::cast_possible_truncation)]
        let settled = (f64::from(-0.2_f32) * 0.95) as f32;
        assert_eq!(lost.state.velocity, Vec2::new(0.0, settled));
        // Moving 1.0, it thrusts half its thrust, its velocity × 0.94.
        let mut soft = escort(at(0.0, 100.0, 0.0, -1.0, 180.0));
        let accel = soft.stats.handling.accel;
        flown(&mut soft, None, Beside::default());
        #[allow(clippy::cast_possible_truncation)]
        let cut = (f64::from(-1.0_f32) * 0.94) as f32;
        assert!(
            (soft.state.velocity.y - (cut + accel * 0.5)).abs() < 1e-5,
            "{:?}",
            soft.state
        );
    }

    #[test]
    fn an_inertialess_escort_with_no_leader_settling_has_its_speed_cut_to_0_95() {
        // Sub 1's settle cuts +0x48 too (@0x8560d-0x8561f).
        let raven = ShipFields {
            flags2: INERTIALESS_HULL,
            ..FAST
        };
        let mut lost = escort_of(raven, 30.0, at(0.0, 100.0, 0.0, -0.2, 0.0));
        lost.speed = 0.2;
        flown(&mut lost, None, Beside::default());
        #[allow(clippy::cast_possible_truncation)]
        let settled = (f64::from(0.2_f32) * 0.95) as f32;
        assert_eq!(lost.speed, settled);
    }
}
