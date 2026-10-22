//! The NPC autopilot: each tick it turns an NPC's goal into [`Controls`]
//! and flies them with the player's [`flight::step`], on the NPC's own
//! handling at its top speed [`TopSpeeds::own`], its `_ShipMaxSpeed`
//! ([`Npc::top_speed`]), its thrust under the world's
//! [`FlightRules`]. It decides nothing (the [`ai`](crate::ai) does); it only
//! flies the goal it is given. An inertialess ship
//! ([`Npc::flies_inertialess`]) flies its controls through
//! [`flight::inertialess_step`] instead, its brake and planning as the
//! [`inertialess`] module says.
//!
//! - [`Goal::Land`]: it heads for the stellar at full speed, steering out
//!   any sideways drift, and brakes (Down and thrust) once its stopping
//!   distance reaches the distance left. It lands within the stellar's
//!   [`landing_radius`] at or below [`LANDING_SPEED`]. That circle is not
//!   the player's landing distance
//!   ([`RuleKey::LandingDistance`](crate::RuleKey::LandingDistance)).
//!   The original's NPC landing is `_HighLevelAIHandler` state 1, its box
//!   by the ship's turn rate
//!   (`phase/stellar-fidelity/phase-5-npc-landing-reach`); the quarter of
//!   the `Type` width at @0x8dbdf is state 20's gate entry, below. A
//!   landing goal whose stellar is a hypergate or wormhole becomes
//!   [`Goal::EnterGate`] and is flown so the same tick (state 1's
//!   hand-off, @0x8d8b8-0x8d8fe).
//! - [`Goal::EnterGate`]: outside the gate's box
//!   ([`gate_entry::inside`], a quarter of its `Type` width on each axis,
//!   inclusive) it heads for the gate as for a landing, with no wait.
//!   Inside it stops dead and starts entering the gate
//!   ([`Mode::EnteringGate`], its wait at [`ENTRY_TICKS`], counted down
//!   that tick); a gate not shown, it brakes (see
//!   [`gate_entry`](crate::traffic::gate_entry)).
//! - While it enters a gate ([`Mode::EnteringGate`]) it holds still,
//!   whatever its condition and goal, its wait counting down, and leaves
//!   the system ([`Outcome::EnteredGate`]) as it runs out: on the 16th
//!   tick, counting the one it reached the box.
//! - [`Goal::JumpOut`]: it leaves by the leg
//!   [`leave::leg`](crate::ai::leave::leg) gives, as
//!   [`leave_flight`] flies it: within 1000 of the centre (dist² ≤ 1e6),
//!   it turns out from the centre and thrusts within its turn rate and 3
//!   whole degrees of that bearing (sub 3); beyond, an expediting ship
//!   begins its warp out at once, and any other brakes until it moves
//!   below 0.35 on both axes, then begins it (`_HighLevelAIHandler` state
//!   2, @0x8dd99-0x8de57; the warp out is
//!   [`warp_out`](crate::traffic::warp_out)'s). The 1000 is every NPC's: its outfits' hyperspace
//!   distance is the player's alone ([`Npc::stats`]' `jump_distance` is
//!   not read).
//! - [`Goal::Follow`]: it keeps within [`FOLLOW_DISTANCE`] of its lead,
//!   matching its lead's velocity there.
//! - [`Goal::Assist`]: its docking flies it
//!   ([`repair_dock`](crate::traffic::repair_dock),
//!   [`refuel_dock`](crate::traffic::refuel_dock)); holding none (with no
//!   target), it coasts, as sub 0xb with no target does nothing
//!   (@0x87345-0x87351).
//! - [`Goal::Formation`]: the player's escort, hired or a mission escort,
//!   keeps its slot beside the player ([`Npc::formation`],
//!   [`slot_position`](crate::escort::slot_position)); with no slot it brakes. Within
//!   [`KEEP_FORMATION`] of it on each axis, it takes the player's
//!   velocity and heading and moves with it, nudged back towards the slot
//!   on each axis more than [`FORMATION_SLACK`] off by at most
//!   [`FORMATION_NUDGE`] ticks' acceleration; within
//!   [`APPROACH_FORMATION`], it closes on the slot at the speed it can
//!   still brake from; farther off, it flies at full speed towards it.
//! - [`Goal::Dock`]: a fighter flies back to its
//!   [`Carrier`](crate::bay::Carrier)
//!   (`_LowLevelAIHandler` sub 8 and 0xb, see [`bay`](crate::bay)).
//!   Outside its dock window on either axis it closes on the carrier as
//!   an escort on its slot, at full speed beyond [`APPROACH_FORMATION`];
//!   inside, it turns at the carrier, thrusting within its turn rate and
//!   [`DOCK_TURN_SLACK`] of it, and each axis more than
//!   [`FORMATION_NUDGE`] ticks' acceleration off is pulled that much
//!   towards it. Within the carrier's reach on both axes it docks, and
//!   leaves the system ([`Outcome::Docked`]).
//! - [`Goal::Attack`], [`Goal::Snipe`] and [`Goal::Flee`]: it flies the
//!   last body of its frame ([`pursuit::frame_of`]), reading whether its
//!   target is disabled, as that body's thrust, cap and heading win
//!   (see [`pursuit`]). It turns to the heading the body wants;
//!   approaching, it thrusts while off it by less than
//!   [`APPROACH_THRUST`] turns; dogfighting, by less than a turn and
//!   [`DOGFIGHT_THRUST`], an inertialess one up to the speed its frame
//!   matches ([`pursuit::Frame::speed_cap`]); fleeing close (sub 5, @0x85c62-0x85d72), turned
//!   to the table bearing from its attacker to itself, by less than a
//!   turn and [`FLEE_THRUST`], as a stand-off ship backing away inside
//!   the dogfight box does, never running out to jump (see
//!   [`stand_off`](crate::ai::stand_off)); swarming (sub 0x12,
//!   @0x852eb-0x8533d), by less than a turn and [`SWARM_THRUST`], at its
//!   plain acceleration and own top speed; each in whole degrees,
//!   strictly ([`fire::off_by`]). Breaking off it thrusts every tick at
//!   [`BREAK_THRUST`] times its acceleration; rushing, at [`RUSH_THRUST`]
//!   times it, up to [`RUSH_SPEED`] times its top speed. Sniping, it
//!   flies as [`snipe::fly`] says (below). With its target gone an
//!   attacker brakes and a sniper coasts.
//! - Sniping, as [`snipe::fly`] says: moving 0.35 or more on an axis it
//!   brakes, turning to its retro heading (or, with a last gun and by the
//!   engine, its target's lead) and thrusting within its turn rate and
//!   [`snipe::THRUST_SLACK`]; at rest it slows by [`snipe::DAMPING`] a
//!   tick and turns to its target's plain bearing (`_LowLevelAIHandler`
//!   sub-goal 0xe, @0x8709d-0x87334). Moving and inertialess, it takes a
//!   whole tick's acceleration off its speed with no turn
//!   ([`snipe::inertialess_brake`], @0x8710f-0x8713a).
//! - [`Goal::Attack`], [`Goal::Snipe`] or [`Goal::Plunder`] while it
//!   searches for a target it lost sight of
//!   ([`lost_sight::searching`]): it brakes to a stop, sub 1, as a wait
//!   does (`_HighLevelAIHandler` @0x8d814).
//! - [`Goal::Plunder`] of an intact ship flies as an attack, or as a
//!   snipe when hopeless, by its frame (see [`pursuit`]); a disabled one it
//!   docks with, as the traffic flies its [`Npc::docking`] (see
//!   [`plunder_dock`](crate::traffic::plunder_dock)), braking until its
//!   high level gives it one.
//! - While its AI wait runs ([`Npc::ai_wait`]: a pirate's dock or hold, a
//!   ship a pirate took into its fleet, or a pirate turned out of its
//!   count wait onto another goal), whatever its goal but the count wait
//!   itself, it moves on at its velocity with no turn, thrust or slowing,
//!   as `_HandleShip` skips them while +0x4c runs (@0x34a54-0x34a72) but
//!   for goal 0x16 (@0x3443f-0x34454); a drift
//!   ([`Goal::Drift`]) so too, its 0.7 a tick along its heading being the
//!   high level's (see [`plunder`](crate::traffic::plunder)).
//! - [`Goal::Flee`] beyond [`pursuit::FLEE_REACH`] of its attacker on
//!   either axis, truncated (its frame has no body): beyond 1000 and able
//!   to jump ([`leave::can_jump`](crate::ai::leave::can_jump), by
//!   [`Readings::jump_fuel`]) it leaves as a leaving ship does; otherwise
//!   it runs out (state 3's far branch, @0x8e0e1-0x8e21a;
//!   [`leave::flee_leg`](crate::ai::leave::flee_leg)).
//! - [`Goal::Hold`]: no thrust and no turn (`_LowLevelAIHandler` zeroes
//!   the thrust @0x851f1, and has no handler for state 0): it coasts on
//!   the velocity it has, unclamped (under
//!   [`RuleKey::SpeedCap`](crate::RuleKey::SpeedCap)'s circle, capped at
//!   its top speed).
//! - [`Goal::Idle`], [`Goal::Wait`] (state 6, a stop, @0x8f101),
//!   [`Goal::Mine`] (order 1 for now, see its docs),
//!   [`Goal::CountWait`] (sub 1, a stop, @0x8d48c, its AI wait running),
//!   or a
//!   goal it cannot fly (a stellar, lead, player or carrier that is not
//!   there): it brakes to a stop.
//! - While it jumps in ([`Mode::JumpingIn`]) it glides towards the centre,
//!   [`glide_speed`] a tick, with no steering, and flies on at that speed
//!   once the next tick's would be at or below its cap
//!   ([`TopSpeeds::burst_cap`]), as it does bursting out of a gate (see
//!   below). A disabled ship glides on too: `_AIDispatch`
//!   cancels a disabled ship's jump out but lets its jump in run
//!   (@0x8fccb-0x8fd08), so a rescue mission's ship held disabled as it
//!   comes in ends its glide in the system.
//! - While it waits at the gate it comes out of ([`Mode::Emerging`]) it
//!   holds still where it was put, at rest, its wait counting down
//!   whatever its condition, as `_HandleShip` counts the timer down for
//!   every ship (@0x3501f-0x35048) and the original's goal 21 holds it
//!   until its +0x4c runs out (`_HighLevelAIHandler` @0x8dd61-0x8dd93).
//!   Then ([`Mode::LeavingGate`]) it bursts out along its heading at
//!   [`exit_speed`], from 30, or from 15 for a ship
//!   [`led_out`](Npc::led_out) of the gate by the player, 1.165 slower
//!   each tick (sub-goal 0xa, as the glide in),
//!   with no steering, and flies on at that speed once
//!   the next tick's would be at or below its cap
//!   ([`TopSpeeds::burst_cap`]: the lower of its own top speed and its
//!   lead's, the player's for a ship the player leads; see
//!   [`emerge`](crate::traffic::emerge)), then flies its goal. A ship not
//!   intact by then drifts instead of bursting, flying from there on.
//!   It is out of its gate ([`Npc::emerges_from`] cleared) from its first
//!   burst tick, as `_HighLevelAIHandler` sets its travel stellar (+0x6c)
//!   to -1 in the pass that starts the burst (@0x8dd93), and
//!   `_AIMakeShipStopJumping` sets it to -1 again as the burst ends
//!   (@0x82eaa); a ship not intact drifts out still carrying it, cleared
//!   as it flies. While it waits it holds the gate's sprite open
//!   (`nova_sim::gate_sprite`). Hit while it waits and
//!   answering the hit, its wait is cut to at most [`HIT_WAIT`] ticks; its
//!   trigger, which may hold its point defence through the wait, is
//!   released as it bursts out.
//! - Held at its gate ([`Mode::HeldAtGate`]), the player having hit it
//!   there, it holds still where it waited, at rest, whatever its
//!   condition and goal, its wait counting down, then flies its goal from
//!   rest, with no burst and no wait after it, its goal kept. So does the
//!   original: `_LowLevelAIHandler` zeroes the thrust word +0x30 each
//!   frame (@0x851e9-0x8520d), so `_HandleShip` skips the thrust and burst
//!   block (@0x34a7b-0x34a84) until the attack first thrusts, and every
//!   goal-4 substate the engine reaches writes the burst word +0x34 in
//!   that same step (sub-goal 5 @0x85d44-0x85d59, 6 @0x85f7b-0x85f82 or
//!   @0x860a6, 7 @0x86957-0x8695e, 0x10 @0x8645a-0x86474, 0x11
//!   @0x865b2-0x865e3). The negative +0x34 the emerge left (@0x895aa-0x895bd)
//!   is gone, so the ship thrusts as usual (@0x34a8a-0x34bbb), never bursts
//!   (@0x34bc0-0x34d32), never reaches `_AIMakeShipStopJumping` (@0x82e77)
//!   and never takes its Rand(30)+30 wait (@0x34d5c-0x34d8b). Only
//!   sub-goal 0x12's swarm thrust (@0x8533d, which a held swarmer with a
//!   swarm leader not its own leader flies: rdm
//!   phase escort-fleet-fidelity/phase-12-held-ship-goal-4-burst) and the
//!   braking thrusts of sub-goals 1 and 0xe
//!   (@0x853fe, @0x8551e, @0x85552, @0x87137, which run only at a speed
//!   of 0.35 or more, never at rest) leave +0x34 alone and would burst.
//! - A ship with no lead, its glide or burst over, holds its course
//!   ([`Mode::Coasting`]) for [`POST_BURST_WAIT`] +
//!   `Rand(`[`POST_BURST_SPREAD`]`)` ticks (`_HandleShip`
//!   @0x34d5c-0x34d8b): it moves on at the speed the glide or burst left
//!   it, above its top speed, with no steering, thrust or fire, and its
//!   trigger is released. Then it flies its goal. Nothing but its own
//!   thrust slows it: `_AIMakeShipStopJumping` (@0x82e77-0x82eba) writes
//!   no velocity and `_HandleShip` clamps none, so [`flight::step`] leaves
//!   an NPC's speed as it is (under the circle reading of
//!   [`RuleKey::SpeedCap`](crate::RuleKey::SpeedCap), it brings it down
//!   to its top speed). A ship with a lead flies at once.
//!   One not intact while it waits drifts instead, its wait still counting
//!   down (@0x34424 → @0x34679 → @0x34a54 → @0x35026), so once intact
//!   again it holds its course for what is left. A mission's ship boarded
//!   on the assist path waits so too ([`Npc::wait_on_course`]). Hit while it waits,
//!   it answers the hit as a flying ship does, and its wait is cut to
//!   at most [`HIT_WAIT`] ticks.
//! - Otherwise a ship that is not [`Condition::Intact`] (disabled,
//!   breaking up or destroyed) drifts: it flies on with no controls, and
//!   never lands or jumps. A disabled or breaking-up one also slows by
//!   [`flight::DISABLED_DRAG`] each tick before it moves (0x33950-0x339a1).

use crate::ai::lost_sight;
use crate::ai::pursuit::{self, BREAK_THRUST, Body, RUSH_SPEED, RUSH_THRUST};
use crate::ai::swarm::SwarmLead;
use crate::ai::{Goal, fire, fire::Manoeuvre};
use crate::bay::DOCK_TURN_SLACK;
use crate::catalog::{LandingSite, ShipId, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::aim::bearing;
use crate::combat::armament::{Arsenal, Trigger};
use crate::combat::hull::Condition;
use crate::escort::{APPROACH_FORMATION, FORMATION_NUDGE, FORMATION_SLACK, KEEP_FORMATION};
use crate::flight::{
    self, AT_REST_SPEED, Controls, FlightRules, Flyer, ShipState, Turn, heading_of, shortest_turn,
};
use crate::gate::GateKind;
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::landing::TypeWidth;
use crate::landing::{LANDING_SPEED, landing_radius};
use crate::rulebook::RuleSource;
use crate::traffic::emerge::exit_speed;
use crate::traffic::gate_entry::{self, ENTRY_TICKS};
use crate::traffic::inertialess::{self, Inertia};
use crate::traffic::leave_flight;
use crate::traffic::npc::{Mode, Npc};
use crate::traffic::pod_flight;
use crate::traffic::ship_display;
use crate::traffic::snipe;
use crate::traffic::spawn::glide_speed;
use std::collections::BTreeMap;

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
/// How far beyond a turn, in whole degrees, a swarmer flying at its swarm
/// leader still thrusts (20.0 @0xdd668, sub 0x12 @0x85322).
pub const SWARM_THRUST: f32 = 20.0;
/// The least a leaderless ship holds its course after its glide in or
/// burst out of a gate, in ticks (`Rand(30) + 30`, @0x34d74-0x34d81).
pub const POST_BURST_WAIT: u32 = 30;
/// `Rand(30)`: the spread of that wait.
pub const POST_BURST_SPREAD: u32 = 30;
/// The most of that wait a ship hit while it holds its course has left,
/// in ticks: `_DamageShip` cuts its AI wait (+0x4c) to 20.0 (const
/// @0xdd668) when the victim answers the hit (@0x3be6e-0x3be7c).
pub const HIT_WAIT: u32 = 20;

/// The readings of the disputed rules the autopilot flies by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Readings {
    /// Where a moving sniper with a last gun turns as it brakes
    /// ([`snipe::fly`],
    /// [`RuleKey::SnipeBrakeLead`](crate::RuleKey::SnipeBrakeLead)).
    pub snipe_lead: RuleSource,
    /// Who needs a jump's fuel now to jump out, which a ship fleeing beyond
    /// its attacker's reach reads ([`leave::can_jump`](crate::ai::leave::can_jump),
    /// [`RuleKey::NpcJumpFuel`](crate::RuleKey::NpcJumpFuel)).
    pub jump_fuel: RuleSource,
}

/// What became of an NPC this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It flies on.
    Flying,
    /// It landed on this stellar, and leaves the system.
    Landed(StellarId),
    /// It jumped out of the system.
    JumpedOut,
    /// It entered this hypergate or wormhole, and leaves the system
    /// through it (see [`gate_entry`](crate::traffic::gate_entry)).
    EnteredGate(StellarId),
    /// A fighter of ship type `ship` docked with `carrier`, and leaves
    /// the system for its bay.
    Docked {
        /// The ship it docked with.
        carrier: ShipRef,
        /// Its ship type.
        ship: ShipId,
    },
    /// An escape pod's wait ran out, and its slot was freed silently
    /// (see [`pod_flight`](crate::traffic::pod_flight)).
    Freed,
}

/// The top speeds a ship flies by this tick: its own `_ShipMaxSpeed`
/// ([`Npc::top_speed`]), and its lead's, while it has a lead in the
/// system, which caps its burst out of a gate (`_HandleShip`
/// @0x34c68-0x34ce6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TopSpeeds {
    /// Its own top speed.
    pub own: f32,
    /// Its lead's top speed: the player's for a ship the player leads, the
    /// fleet lead's for a fleet escort; `None` without a lead, or with
    /// one gone from the system, as `_AICheckParentExists` clears it.
    pub lead: Option<f32>,
}

impl TopSpeeds {
    /// `npc`'s own handling's top speed ([`Npc::own_handling`]), with no
    /// lead.
    #[must_use]
    pub fn own_of(npc: &Npc) -> Self {
        Self {
            own: npc.own_handling().max_speed,
            lead: None,
        }
    }

    /// The speed its burst out of a gate ends at: the lower of its own top
    /// speed and its lead's, the lead's on a tie (@0x34c68-0x34ce6), or its
    /// own without a lead. `_HandleShip` raises a cap below -1.0 to -1.0
    /// (@0x34cf3-0x34d0d), which never binds, as no top speed is negative.
    #[must_use]
    pub fn burst_cap(self) -> f32 {
        self.lead.map_or(self.own, |lead| lead.min(self.own))
    }
}

/// Flies `npc` one tick towards its goal, among `sites`, the gates among
/// them as wide as `type_widths` says ([`gate_entry::gate_reach`]), the
/// ship its goal is about (the lead it follows, the ship it fights or
/// inspects) at `other`, when it is there, disabled as `other_disabled`
/// says (a stand-off ship's band halves against a disabled target, see
/// [`stand_off`](crate::ai::stand_off)), with `swarm` its swarm lead
/// ([`swarm::lead`](crate::ai::swarm::lead)) for a frame its step has not
/// worked out, at its top speed `speeds.own`, its burst out of a gate or glide in capped at
/// [`TopSpeeds::burst_cap`], under the flight rules `flight` (its thrust
/// [`FlightRules::handling`]'s, which every plan here reads), by
/// `readings` of the disputed rules (a moving sniper with a last gun
/// turning as [`Readings::snipe_lead`] says, a fleeing ship jumping out as
/// [`Readings::jump_fuel`] says), the wait a leaderless ship takes after
/// it rolled on `chance`. A fight's frame its step has not worked out
/// leads by `weapons`, the global weapon records
/// ([`pursuit::frame_of`]).
///
/// An [`immobile`](Npc::immobile) ship's velocity is set to none before
/// it flies and its move is undone after, as `_HandleShip` zeroes it and
/// skips the move (@0x34209-0x3423b) before the AI and the thrust run.
// Each is a separate input of the tick: the ship, the system's stellars
// and gate widths, its quarry, whether it is disabled and its swarm lead,
// its top speeds, the flight rules, the disputed rules' readings, the
// weapon records and the dice.
#[allow(clippy::too_many_arguments)]
pub fn fly(
    npc: &mut Npc,
    sites: &[LandingSite],
    type_widths: &BTreeMap<StellarId, TypeWidth>,
    other: Option<&ShipState>,
    other_disabled: bool,
    swarm: Option<SwarmLead>,
    speeds: TopSpeeds,
    flight: FlightRules,
    readings: Readings,
    weapons: &Arsenal,
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    let rules = (flight, readings, weapons);
    let quarry = (other, other_disabled, swarm);
    if !npc.immobile {
        return fly_mobile(npc, sites, type_widths, quarry, speeds, rules, chance);
    }
    let at = npc.state.position;
    npc.state.velocity = Vec2::ZERO;
    let outcome = fly_mobile(npc, sites, type_widths, quarry, speeds, rules, chance);
    npc.state.position = at;
    outcome
}

/// [`fly`] for a ship that may move, at its quarry `(other,
/// other_disabled, swarm)`, under the flight rules and the disputed rules'
/// readings, leading by the weapon records `(flight, readings, weapons)`.
fn fly_mobile(
    npc: &mut Npc,
    sites: &[LandingSite],
    type_widths: &BTreeMap<StellarId, TypeWidth>,
    (other, other_disabled, swarm): (Option<&ShipState>, bool, Option<SwarmLead>),
    speeds: TopSpeeds,
    (flight, readings, weapons): (FlightRules, Readings, &Arsenal),
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    let snipe_lead = readings.snipe_lead;
    let handling = npc.flying_handling(speeds.own, flight);
    let inertia = Inertia::of(npc, other, flight);
    if pod_flight::is_pod(npc) {
        let controls = pod_flight::controls(npc, &handling);
        step(npc, &handling, controls, flight, inertia);
        return Outcome::Flying;
    }
    if let Some(outcome) = unsteered(npc, &handling, speeds, flight, inertia, chance) {
        return outcome;
    }
    hand_off(npc, sites);
    if npc.ai_wait > 0 && !npc.count_wait() {
        // `_HandleShip` skips its turn and thrust while +0x4c runs
        // (@0x34a54-0x34a72) and moves it first (@0x34274-0x342aa), an
        // inertialess ship's velocity following its speed (@0x3425f);
        // goal 0x16 is exempt (`_AIIsShipDoingCountWait`, @0x3443f-0x34454)
        // and brakes below.
        position_step(npc, &handling, inertia);
        return Outcome::Flying;
    }
    let state = npc.state;
    let controls = match npc.goal {
        Goal::Land(stellar) => match sites.iter().find(|site| site.id == stellar) {
            Some(site) if over(&state, site) => return Outcome::Landed(stellar),
            Some(site) => land_approach(npc, site, &handling, inertia),
            None => brake(&state, &handling, inertia),
        },
        Goal::EnterGate(gate) => {
            match enter_box(npc, sites, type_widths, gate, &handling, inertia) {
                Ok(controls) => controls,
                Err(outcome) => return outcome,
            }
        }
        Goal::JumpOut => return leave_flight::leaving(npc, &handling, flight, inertia),
        Goal::Follow(_) | Goal::Inspect(_) => match other {
            Some(other) => follow(&state, &handling, other, inertia),
            None => brake(&state, &handling, inertia),
        },
        // Goal 0xd against a disabled ship docks with it, which the
        // traffic flies ([`plunder_dock`](crate::traffic::plunder_dock));
        // with no docking yet, its high level not having run, it brakes.
        // Searching for a target out of sight: sub 1, a stop (@0x8d814).
        Goal::Attack(_) | Goal::Snipe(_) | Goal::Plunder { .. } if lost_sight::searching(npc) => {
            brake(&state, &handling, inertia)
        }
        Goal::Plunder { .. } if other_disabled => brake(&state, &handling, inertia),
        Goal::Attack(_) | Goal::Snipe(_) | Goal::Flee(_) | Goal::Plunder { .. } => match other {
            Some(target) => {
                if let Some((body, speed_cap)) =
                    pursuit::frame_of(npc, target, other_disabled, swarm, weapons)
                        .and_then(|frame| Some((frame.last()?, frame.speed_cap)))
                {
                    let flown = Flown {
                        handling: &handling,
                        inertia,
                    };
                    fight(
                        npc,
                        target,
                        (body, speed_cap),
                        flown,
                        (flight, snipe_lead, weapons),
                    );
                    return Outcome::Flying;
                }
                // Fleeing beyond its attacker's reach: state 3's far branch.
                return leave_flight::fleeing_afar(
                    npc,
                    &handling,
                    flight,
                    inertia,
                    readings.jump_fuel,
                );
            }
            // A sniper whose target is gone coasts ([`snipe::fly`]).
            None if matches!(npc.goal, Goal::Snipe(_)) => {
                snipe::fly(npc, None, &handling, snipe_lead, weapons)
            }
            None => brake(&state, &handling, inertia),
        },
        Goal::Formation { .. } => match formation(npc, other, &handling, inertia) {
            Ok(controls) => controls,
            Err(outcome) => return outcome,
        },
        Goal::Dock(_) => match dock(npc, other, &handling, flight, inertia) {
            Ok(controls) => controls,
            Err(outcome) => return outcome,
        },
        // A count wait is sub 1, a stop (`_HighLevelAIHandler` @0x8d48c), as
        // is a wait (state 6, @0x8f101).
        Goal::Idle | Goal::Wait | Goal::Mine | Goal::CountWait => brake(&state, &handling, inertia),
        // A drift moves on with no turn or thrust (its wait above), as
        // does an assister its docking doesn't fly: sub 0xb with no target
        // does nothing (@0x87345-0x87351).
        Goal::Hold | Goal::Drift | Goal::Assist(_) => Controls::default(),
    };
    step(npc, &handling, controls, flight, inertia);
    Outcome::Flying
}

/// Flies `npc` one tick under `controls` as `inertia` says: an inertial
/// ship's [`flight::step`], an inertialess one's
/// [`flight::inertialess_step`] (see [`inertialess`]).
pub(crate) fn step(
    npc: &mut Npc,
    handling: &Handling,
    controls: Controls,
    flight: FlightRules,
    inertia: Inertia,
) {
    ship_display::record(npc, handling, controls, flight, inertia);
    match inertia {
        Inertia::Inertial => flight::step(&mut npc.state, handling, controls, flight, Flyer::Npc),
        Inertia::Inertialess { .. } => {
            flight::inertialess_step(&mut npc.state, &mut npc.speed, handling, controls, flight);
        }
    }
}

/// State 1's hand-off (@0x8d8b8-0x8d8fe): a landing goal whose stellar
/// among `sites` is a hypergate or wormhole becomes [`Goal::EnterGate`].
fn hand_off(npc: &mut Npc, sites: &[LandingSite]) {
    if let Goal::Land(stellar) = npc.goal
        && sites
            .iter()
            .any(|site| site.id == stellar && GateKind::of(site.flags2).is_some())
    {
        npc.goal = Goal::EnterGate(stellar);
    }
}

/// How `npc` flies into `gate` among `sites`, of the widths
/// `type_widths` gives (see the module docs): outside its box, the
/// controls of the approach, or braking with no gate there; inside it,
/// stopped and entering it.
fn enter_box(
    npc: &mut Npc,
    sites: &[LandingSite],
    type_widths: &BTreeMap<StellarId, TypeWidth>,
    gate: StellarId,
    handling: &Handling,
    inertia: Inertia,
) -> Result<Controls, Outcome> {
    let Some(site) = sites.iter().find(|site| site.id == gate) else {
        return Ok(brake(&npc.state, handling, inertia));
    };
    let reach = gate_entry::gate_reach(site, type_widths);
    if !gate_entry::inside(site.position, npc.state.position, reach) {
        return Ok(approach(&npc.state, handling, site.position, inertia));
    }
    npc.state.velocity = Vec2::ZERO;
    npc.mode = Mode::EnteringGate {
        gate,
        ticks_left: ENTRY_TICKS - 1,
    };
    Err(Outcome::Flying)
}

/// How `npc` keeps its slot in the formation of its leader at `leader`
/// (see the module docs): the controls to rejoin it, or braking with no
/// leader or slot; or, within [`KEEP_FORMATION`] of it, having flown.
fn formation(
    npc: &mut Npc,
    leader: Option<&ShipState>,
    handling: &Handling,
    inertia: Inertia,
) -> Result<Controls, Outcome> {
    let (Some(leader), Some(place)) = (leader, npc.formation()) else {
        return Ok(brake(&npc.state, handling, inertia));
    };
    let slot = place.position(leader);
    let off = slot - npc.state.position;
    if off.x.abs() <= KEEP_FORMATION && off.y.abs() <= KEEP_FORMATION {
        keep_formation(npc, leader, slot, handling);
        return Err(Outcome::Flying);
    }
    Ok(rejoin(&npc.state, handling, slot, leader.velocity, inertia))
}

/// How `npc` flies back to its carrier at `carrier` (see the module
/// docs), under `flight`: the controls to fly outside its dock window, or
/// braking with no carrier there; or, inside it, what became of it,
/// having flown.
fn dock(
    npc: &mut Npc,
    carrier: Option<&ShipState>,
    handling: &Handling,
    flight: FlightRules,
    inertia: Inertia,
) -> Result<Controls, Outcome> {
    let (Some(carrier), Some(dock)) = (carrier, npc.carrier) else {
        return Ok(brake(&npc.state, handling, inertia));
    };
    let off = carrier.position - npc.state.position;
    let within = |reach: f32| off.x.abs() <= reach && off.y.abs() <= reach;
    if !within(dock.window) {
        return Ok(rejoin(
            &npc.state,
            handling,
            carrier.position,
            carrier.velocity,
            inertia,
        ));
    }
    if within(dock.reach) {
        return Err(Outcome::Docked {
            carrier: dock.ship,
            ship: npc.ship,
        });
    }
    dock_in(npc, carrier, handling, flight, inertia);
    Err(Outcome::Flying)
}

/// Flies back to `to`, which moves at `velocity`: within
/// [`APPROACH_FORMATION`] of it on each axis, closing at the speed it can
/// still brake from; farther off, at full speed towards it.
fn rejoin(
    state: &ShipState,
    handling: &Handling,
    to: Vec2,
    velocity: Vec2,
    inertia: Inertia,
) -> Controls {
    let off = to - state.position;
    if off.x.abs() <= APPROACH_FORMATION && off.y.abs() <= APPROACH_FORMATION {
        close_on(state, handling, to, velocity, 0.0, inertia)
    } else {
        let distance = off.length();
        match_velocity(
            state,
            handling,
            off * (handling.max_speed / distance),
            inertia,
        )
    }
}

/// One tick of `npc` docking with a carrier at `carrier`, inside its dock
/// window (see the module docs): it turns at the carrier, thrusting
/// within its turn rate and [`DOCK_TURN_SLACK`] of it, flies, and is
/// pulled towards it on each axis farther off than
/// [`FORMATION_NUDGE`] ticks' acceleration, under `flight`, as `inertia`
/// says.
fn dock_in(
    npc: &mut Npc,
    carrier: &ShipState,
    handling: &Handling,
    flight: FlightRules,
    inertia: Inertia,
) {
    let wanted = bearing(npc.state.position, carrier.position);
    let controls = Controls {
        thrust: shortest_turn(npc.state.heading, wanted).abs()
            <= handling.turn_rate + DOCK_TURN_SLACK,
        turn: steer(npc.state.heading, wanted, handling),
        reverse: false,
        ..Controls::default()
    };
    step(npc, handling, controls, flight, inertia);
    let pull = handling.accel * FORMATION_NUDGE;
    let towards = |at: f32, to: f32| {
        let off = to - at;
        if off.abs() > pull {
            at + off.signum() * pull
        } else {
            at
        }
    };
    let at = npc.state.position;
    npc.state.position = Vec2::new(
        towards(at.x, carrier.position.x),
        towards(at.y, carrier.position.y),
    );
}

/// Turns from `heading` towards `wanted`, not at all within half a tick's
/// turn, where turning would only overshoot.
pub(crate) fn steer(heading: f32, wanted: f32, handling: &Handling) -> Turn {
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

/// Flies one tick of `npc`'s fight at a target at `target` on
/// `handling` by its frame's last `body` and its `speed_cap`, whose writes to the thrust
/// (+0x30), the cap (+0x34) and the heading (+0x68) win (see
/// [`pursuit`](crate::ai::pursuit)): it turns to the heading the body
/// wants, and thrusts approaching (7) while off it by less than
/// [`APPROACH_THRUST`] turns, dogfighting (6) by less than a turn and
/// [`DOGFIGHT_THRUST`], fleeing close or backing away (5) by less than a
/// turn and [`FLEE_THRUST`], each in whole degrees, strictly
/// ([`fire::off_by`]); breaking off (0x10) it thrusts every tick at
/// [`BREAK_THRUST`] times its acceleration, and rushing (0x11) at
/// [`RUSH_THRUST`] times it up to [`RUSH_SPEED`] times its top speed;
/// sniping, it flies as [`snipe::fly`] says, a moving sniper with a last
/// gun turning as `snipe_lead` says, leading by that gun's record in
/// `weapons`. It flies under `flight` as
/// `flown_by` says ([`step`]); a moving inertialess sniper brakes as
/// substate 0xe does ([`snipe::inertialess_brake`]). An inertialess
/// dogfighter whose frame matches its target's speed
/// ([`Frame::speed_cap`](pursuit::Frame::speed_cap)) flies at most at
/// that speed, a cap of none being none, as `_HandleShip` reads +0x34
/// (@0x34a8a-0x34aab); a later body's cap (0x10's none, 0x11's) wins.
fn fight(
    npc: &mut Npc,
    target: &ShipState,
    (body, speed_cap): (Body, Option<f32>),
    flown_by: Flown,
    (flight, snipe_lead, weapons): (FlightRules, RuleSource, &Arsenal),
) {
    let handling = flown_by.handling;
    let window = match body.manoeuvre {
        Manoeuvre::Approach => APPROACH_THRUST * handling.turn_rate,
        Manoeuvre::Dogfight => handling.turn_rate + DOGFIGHT_THRUST,
        Manoeuvre::Flee => handling.turn_rate + FLEE_THRUST,
        Manoeuvre::Swarm => handling.turn_rate + SWARM_THRUST,
        // Breaking off and rushing thrust whatever the heading.
        Manoeuvre::BreakOff | Manoeuvre::Rush => f32::INFINITY,
        Manoeuvre::Snipe => {
            let controls = match flown_by.inertia {
                Inertia::Inertialess { .. } if snipe::moving(&npc.state) => {
                    snipe::inertialess_brake(npc, handling)
                }
                _ => snipe::fly(npc, Some(target), handling, snipe_lead, weapons),
            };
            step(npc, handling, controls, flight, flown_by.inertia);
            return;
        }
    };
    let flown = match body.manoeuvre {
        Manoeuvre::BreakOff => Handling {
            accel: handling.accel * BREAK_THRUST,
            ..*handling
        },
        Manoeuvre::Rush => Handling {
            accel: handling.accel * RUSH_THRUST,
            // `_ShipMaxSpeed` × the double 1.8, back to a float
            // (@0x865cf-0x865e3).
            #[allow(clippy::cast_possible_truncation)]
            max_speed: (f64::from(handling.max_speed) * RUSH_SPEED) as f32,
            ..*handling
        },
        Manoeuvre::Dogfight => match (flown_by.inertia, speed_cap) {
            (Inertia::Inertialess { .. }, Some(cap)) if cap > 0.0 => Handling {
                max_speed: cap,
                ..*handling
            },
            _ => *handling,
        },
        _ => *handling,
    };
    let controls = Controls {
        thrust: fire::off_by(npc.state.heading, body.heading) < window,
        turn: steer(npc.state.heading, f32::from(body.heading), handling),
        reverse: false,
        ..Controls::default()
    };
    step(npc, &flown, controls, flight, flown_by.inertia);
}

/// Tick `tick` of `npc`'s glide in from hyperspace: along its heading at
/// [`glide_speed`], ending as [`burst`] says.
///
/// `_HandleShip` runs the glide (+0x34 set to -50.0 and +0x30 to -1.165 by
/// low mode 10, @0x88114-0x88136) through the same negative-speed path as
/// the burst out of a gate (@0x34bc0-0x34d32): it ends against the same
/// cap, the lower of its own and its lead's top speed (@0x34c68-0x34d0d),
/// once the next speed would reach it (@0x34d10-0x34d2a), and leaves the
/// velocity at the last speed, which lies within 1.165 above the cap.
fn glide(
    npc: &mut Npc,
    tick: u32,
    speeds: TopSpeeds,
    flown: Flown,
    chance: &mut (impl Chance + ?Sized),
) {
    if !burst(
        npc,
        glide_speed(tick),
        glide_speed(tick + 1),
        speeds,
        flown,
        chance,
    ) {
        npc.mode = Mode::JumpingIn { tick: tick + 1 };
    }
}

/// Flies `npc` one tick when it has no say in its flight: entering a
/// gate or waiting at one, drifting while not intact, gliding in from
/// hyperspace (intact or disabled), bursting out of a gate, or holding
/// its course after either (see the module docs), on `handling` under
/// `flight`, as `inertia` says, its burst ending at `speeds`'
/// [`burst_cap`](TopSpeeds::burst_cap) and its wait rolled on `chance`.
/// What became of it, if it did. A disabled or breaking-up ship's speed
/// along its heading slows with its velocity, inertialess or not
/// (@0x3398f-0x339a1).
fn unsteered(
    npc: &mut Npc,
    handling: &Handling,
    speeds: TopSpeeds,
    flight: FlightRules,
    inertia: Inertia,
    chance: &mut (impl Chance + ?Sized),
) -> Option<Outcome> {
    let flown = Flown { handling, inertia };
    match (npc.mode, npc.condition == Condition::Intact) {
        (Mode::EnteringGate { gate, ticks_left }, _) => {
            return Some(enter_gate(npc, gate, ticks_left, flown));
        }
        (Mode::Emerging { ticks_left }, _) => wait_at_gate(npc, ticks_left),
        (Mode::HeldAtGate { ticks_left }, _) => hold_at_gate(npc, ticks_left),
        (Mode::JumpingIn { tick }, false) if npc.condition == Condition::Disabled => {
            glide(npc, tick, speeds, flown, chance);
        }
        (mode, false) => {
            match mode {
                Mode::LeavingGate { .. } => {
                    npc.mode = Mode::Flying;
                    npc.emerges_from = None;
                }
                // `_HandleShip` counts +0x4c down for a disabled ship too
                // (@0x34424 → @0x34679 → @0x34a54 → @0x35026).
                Mode::Coasting { ticks_left } => count_down(npc, ticks_left),
                _ => {}
            }
            if matches!(npc.condition, Condition::Disabled | Condition::Dying { .. }) {
                flight::drag(&mut npc.state);
                npc.speed *= flight::DISABLED_DRAG;
            }
            step(npc, handling, Controls::default(), flight, inertia);
        }
        (Mode::JumpingIn { tick }, true) => glide(npc, tick, speeds, flown, chance),
        (Mode::LeavingGate { tick }, true) => leave_gate(npc, tick, speeds, flown, chance),
        (Mode::Coasting { ticks_left }, true) => coast(npc, ticks_left, flown),
        (Mode::Flying | Mode::Adrift { .. }, true) => return None,
    }
    Some(Outcome::Flying)
}

/// One tick of `npc` entering `gate`, `ticks_left` of its wait left
/// (see [`gate_entry`]): still, whatever its condition and goal, as
/// `_HandleShip` counts +0x4c down for every ship and the fade reads only
/// the sub-goal; and gone once the wait is out. An inertialess ship's
/// velocity follows its speed meanwhile, as `flown` says
/// ([`inertialess::drift`]).
fn enter_gate(npc: &mut Npc, gate: StellarId, ticks_left: u32, flown: Flown) -> Outcome {
    flown.hold(npc);
    let left = ticks_left.saturating_sub(1);
    if left == 0 {
        return Outcome::EnteredGate(gate);
    }
    npc.mode = Mode::EnteringGate {
        gate,
        ticks_left: left,
    };
    Outcome::Flying
}

/// One tick of `npc`'s wait at the gate it comes out of, `ticks_left` of
/// it left: still, and bursting out once it is over, its trigger (the
/// point defence it may hold while it waits) released, as the burst's
/// sub-goal 0xa makes no `_AIEscortFireUnprovoked` call (@0x8f3d9).
fn wait_at_gate(npc: &mut Npc, ticks_left: u32) {
    npc.state.velocity = Vec2::ZERO;
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        npc.trigger = Trigger::default();
        Mode::LeavingGate { tick: 0 }
    } else {
        Mode::Emerging { ticks_left: left }
    };
}

/// One tick of `npc` held at its gate ([`Mode::HeldAtGate`]),
/// `ticks_left` of its wait left: still, whatever its condition and goal,
/// as `_HandleShip` counts +0x4c down for every ship (@0x3501f-0x35048)
/// and skips its steering, thrust and fire while it runs
/// (@0x34a54-0x34a72); and flying its goal from rest once the wait is
/// out, with no burst and no wait after it, as the original's attack
/// rewrites the burst word +0x34 the step it first thrusts (see the
/// module docs).
fn hold_at_gate(npc: &mut Npc, ticks_left: u32) {
    npc.state.velocity = Vec2::ZERO;
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        Mode::Flying
    } else {
        Mode::HeldAtGate { ticks_left: left }
    };
}

/// Tick `tick` of `npc`'s burst out of a gate: along its heading at
/// [`exit_speed`], ending as [`burst`] says. On its first tick it is out
/// of its gate ([`Npc::emerges_from`] cleared, @0x8dd8a-0x8dd93).
fn leave_gate(
    npc: &mut Npc,
    tick: u32,
    speeds: TopSpeeds,
    flown: Flown,
    chance: &mut (impl Chance + ?Sized),
) {
    if tick == 0 {
        // The pass that starts the burst sets goal 8 and +0x6c = -1
        // (`_HighLevelAIHandler` @0x8dd8a-0x8dd93).
        npc.emerges_from = None;
    }
    if !burst(
        npc,
        exit_speed(npc.led_out, tick),
        exit_speed(npc.led_out, tick + 1),
        speeds,
        flown,
        chance,
    ) {
        npc.mode = Mode::LeavingGate { tick: tick + 1 };
    }
}

/// One tick of a glide in or a burst out of a gate (`_HandleShip`
/// @0x34bc0-0x34d8b): `npc` moves along its heading at `speed`. Once the
/// `next` tick's speed would be at or below its cap
/// ([`TopSpeeds::burst_cap`]) the burst is over, and whether it is, the
/// caller stepping the mode on while it is not. It keeps this tick's
/// speed. With a lead (`speeds.lead`: a fleet escort's, or the player
/// for a ship the player leads) it flies on; with none it holds its
/// course for [`POST_BURST_WAIT`] + `Rand(`[`POST_BURST_SPREAD`]`)`
/// ticks first ([`Mode::Coasting`], @0x34d5c-0x34d8b), its trigger
/// released.
///
/// An inertialess ship (as `flown` says) takes `speed` as its speed along
/// its heading instead (@0x34bdf-0x34bf0), after its velocity has
/// followed the last tick's and it has moved (@0x34250-0x342aa), and its
/// velocity is zeroed as the burst ends (@0x34d46-0x34d59).
fn burst(
    npc: &mut Npc,
    speed: f32,
    next: f32,
    speeds: TopSpeeds,
    flown: Flown,
    chance: &mut (impl Chance + ?Sized),
) -> bool {
    if let Inertia::Inertialess { .. } = flown.inertia {
        inertialess::drift(npc, flown.handling);
        npc.speed = speed;
    } else {
        npc.state.velocity = flight::facing(npc.state.heading) * speed;
        npc.state.position = npc.state.position + npc.state.velocity;
    }
    if next > speeds.burst_cap() {
        return false;
    }
    if let Inertia::Inertialess { .. } = flown.inertia {
        npc.state.velocity = Vec2::ZERO;
    }
    npc.mode = if speeds.lead.is_some() {
        Mode::Flying
    } else {
        npc.trigger = Trigger::default();
        Mode::Coasting {
            ticks_left: POST_BURST_WAIT + chance.below(POST_BURST_SPREAD),
        }
    };
    true
}

/// Cuts the wait of `npc`, hit and answering it, to at most
/// [`HIT_WAIT`] ticks: `_DamageShip` sets the victim's AI wait (+0x4c) to
/// 20.0 when it is above that (@0x3be6e-0x3be7c), whoever the attacker.
/// Only a ship waiting in its gate ([`Mode::Emerging`]), held at it
/// ([`Mode::HeldAtGate`]) or holding its course after its glide or burst
/// ([`Mode::Coasting`]) has a wait to cut: a glide or burst has none
/// (+0x4c = -1, @0x8dd83). A ship the player hit in its gate was cut as
/// it was held, but a Good Samaritan held there kept its wait
/// (@0x82b82-0x82b97), which a later hit cuts. So is a flying ship's
/// [`Npc::ai_wait`]: a pirate's dock or hold, a ship it took into its
/// fleet, or a pirate's count wait ([`Goal::CountWait`], goal 0x16), as
/// `_DamageShip` reads no goal; the player's hit turns that pirate on it
/// (goal 4) with what is left of the wait, 20 at most.
pub(crate) fn cut_wait(npc: &mut Npc) {
    if let Mode::Coasting { ticks_left }
    | Mode::Emerging { ticks_left }
    | Mode::HeldAtGate { ticks_left } = &mut npc.mode
    {
        *ticks_left = (*ticks_left).min(HIT_WAIT);
    }
    npc.ai_wait = npc.ai_wait.min(HIT_WAIT);
}

/// One tick of `npc`'s wait after its burst, `ticks_left` of it left: it
/// moves on at its velocity, neither turning, thrusting, slowing nor
/// firing, as `_HandleShip` skips its steering, thrust, over-speed and
/// fire while +0x4c runs (@0x34a54-0x34a72 to @0x3501f) but moves it
/// first (@0x34274-0x342aa), and flies once the wait is over
/// (@0x3501f-0x35048). An inertialess ship's velocity follows its speed
/// as it moves (@0x3425f; [`inertialess::drift`]).
fn coast(npc: &mut Npc, ticks_left: u32, flown: Flown) {
    position_step(npc, flown.handling, flown.inertia);
    count_down(npc, ticks_left);
}

/// `_HandleShip`'s move alone (@0x34274-0x342aa), with neither turn nor
/// thrust: `npc` moves by its velocity, an inertialess one's velocity
/// following its speed at `handling`'s rate first (@0x3425f,
/// [`inertialess::drift`]).
fn position_step(npc: &mut Npc, handling: &Handling, inertia: Inertia) {
    match inertia {
        Inertia::Inertial => npc.state.position = npc.state.position + npc.state.velocity,
        Inertia::Inertialess { .. } => inertialess::drift(npc, handling),
    }
}

/// `npc`'s flight in a pass no `_AIDispatch` steered it in, the ship its
/// goal is about at `other`, under the flight rules `flight`: only
/// `_HandleShip`'s move ([`position_step`]), as for a fighter launched
/// into a later slot of the ship loop (`_LaunchFighter` @0x3c90b,
/// @0x3d680). An [`immobile`](Npc::immobile) ship's velocity goes to none
/// and it stays put (@0x34209-0x3423b).
pub(crate) fn coast_unsteered(npc: &mut Npc, other: Option<&ShipState>, flight: FlightRules) {
    if npc.immobile {
        npc.state.velocity = Vec2::ZERO;
        return;
    }
    let handling = flight.handling(npc.stats.handling);
    let inertia = Inertia::of(npc, other, flight);
    position_step(npc, &handling, inertia);
}

/// What a ship with no say in its flight flies on: its handling, and
/// whether it is inertialess.
#[derive(Clone, Copy)]
struct Flown<'a> {
    /// Its handling.
    handling: &'a Handling,
    /// Whether it is inertialess.
    inertia: Inertia,
}

impl Flown<'_> {
    /// Holds `npc` where it is, as a ship entering a gate is held: still
    /// with inertia; inertialess, its velocity following its speed
    /// ([`inertialess::drift`]).
    fn hold(self, npc: &mut Npc) {
        match self.inertia {
            Inertia::Inertial => npc.state.velocity = Vec2::ZERO,
            Inertia::Inertialess { .. } => inertialess::drift(npc, self.handling),
        }
    }
}

/// Counts `npc`'s wait on its course down a tick from `ticks_left`
/// (@0x35026-0x35048): it flies once the wait is out.
fn count_down(npc: &mut Npc, ticks_left: u32) {
    let left = ticks_left.saturating_sub(1);
    npc.mode = if left == 0 {
        Mode::Flying
    } else {
        Mode::Coasting { ticks_left: left }
    };
}

/// Whether a ship at `state` can land on `site`: over it, and slow enough.
fn over(state: &ShipState, site: &LandingSite) -> bool {
    (site.position - state.position).length() <= landing_radius(site)
        && state.velocity.length() <= LANDING_SPEED
}

/// How `npc`, landing on `site` but not over it, flies on `handling` as
/// `inertia` says: toward it ([`approach`]), a folding ship unfolding
/// once within its landing radius (`_HighLevelAIHandler` state 1
/// @0x8da0c-0x8da38, see [`ship_display`]).
fn land_approach(
    npc: &mut Npc,
    site: &LandingSite,
    handling: &Handling,
    inertia: Inertia,
) -> Controls {
    let state = npc.state;
    if (site.position - state.position).length() <= landing_radius(site) {
        ship_display::landing_unfold(npc);
    }
    approach(&state, handling, site.position, inertia)
}

/// Heads for `target` at full speed, and brakes once the ship's stopping
/// distance reaches the distance left.
fn approach(state: &ShipState, handling: &Handling, target: Vec2, inertia: Inertia) -> Controls {
    let to = target - state.position;
    let distance = to.length();
    if distance <= stopping_distance(state, handling, inertia) {
        return brake(state, handling, inertia);
    }
    match_velocity(
        state,
        handling,
        to * (handling.max_speed / distance),
        inertia,
    )
}

/// How far a ship at `state` travels while it turns to face against its
/// motion and brakes to a stop; none at rest, and for ever when it cannot
/// turn or thrust. At rest is at or below [`AT_REST_SPEED`], the
/// original's thrust's, whatever the thrust divisor: a threshold of the
/// engine's own planning, not a flight rule. An inertialess ship's is
/// [`inertialess::stopping_distance`].
fn stopping_distance(state: &ShipState, handling: &Handling, inertia: Inertia) -> f32 {
    if let Inertia::Inertialess { speed } = inertia {
        return inertialess::stopping_distance(speed, handling);
    }
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
/// does, until thrust would only push it the other way; nothing at or
/// below [`AT_REST_SPEED`] (see [`stopping_distance`]). An inertialess
/// ship brakes as substate 1 does ([`inertialess::brake`]).
pub(crate) fn brake(state: &ShipState, handling: &Handling, inertia: Inertia) -> Controls {
    if let Inertia::Inertialess { .. } = inertia {
        return inertialess::brake(state);
    }
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
        ..Controls::default()
    }
}

/// Keeps within [`FOLLOW_DISTANCE`] of a lead at `lead`: closing at the
/// speed it can still brake from in the distance beyond it, on top of the
/// lead's velocity, and matching the lead's velocity within it.
fn follow(state: &ShipState, handling: &Handling, lead: &ShipState, inertia: Inertia) -> Controls {
    close_on(
        state,
        handling,
        lead.position,
        lead.velocity,
        FOLLOW_DISTANCE,
        inertia,
    )
}

/// Keeps within `margin` of `target`, which moves at `velocity`: closing
/// at the speed it can still brake from in the distance beyond the
/// margin, on top of that velocity, and matching it within the margin.
fn close_on(
    state: &ShipState,
    handling: &Handling,
    target: Vec2,
    velocity: Vec2,
    margin: f32,
    inertia: Inertia,
) -> Controls {
    let to = target - state.position;
    let distance = to.length();
    let beyond = (distance - margin).max(0.0);
    let speed = braking_speed(beyond, handling, inertia).min(handling.max_speed);
    // None within the margin, where `to` may be no direction at all.
    let closing = if distance > margin {
        to * (speed / distance)
    } else {
        Vec2::ZERO
    };
    match_velocity(state, handling, velocity + closing, inertia)
}

/// Keeps formation on `slot` beside a leader at `leader`
/// (`_AIMaintainFormation` @0x84f04): `npc` takes the leader's velocity
/// and heading and moves with it, then each axis more than
/// [`FORMATION_SLACK`] off the slot is nudged towards it by
/// [`FORMATION_NUDGE`] ticks' acceleration at most, never past it: the
/// thrust `handling` flies it with.
fn keep_formation(npc: &mut Npc, leader: &ShipState, slot: Vec2, handling: &Handling) {
    npc.display.drive.formation = true;
    let state = &mut npc.state;
    state.velocity = leader.velocity;
    state.heading = leader.heading;
    state.position = state.position + state.velocity;
    let nudge = handling.accel * FORMATION_NUDGE;
    let towards = |at: f32, to: f32| {
        let off = to - at;
        if off.abs() > FORMATION_SLACK {
            at + off.signum() * nudge.min(off.abs())
        } else {
            at
        }
    };
    state.position = Vec2::new(
        towards(state.position.x, slot.x),
        towards(state.position.y, slot.y),
    );
}

/// The fastest a ship can go and still turn round and stop within
/// `distance`: the `v` with `v T + v² / 2a = distance`, `T` the ticks a
/// half turn takes. None when it cannot turn or thrust. An inertialess
/// ship's is [`inertialess::braking_speed`].
fn braking_speed(distance: f32, handling: &Handling, inertia: Inertia) -> f32 {
    if let Inertia::Inertialess { .. } = inertia {
        return inertialess::braking_speed(distance, handling);
    }
    if handling.turn_rate <= 0.0 || handling.accel <= 0.0 {
        return 0.0;
    }
    let half_turn = 180.0 / handling.turn_rate;
    let a = handling.accel;
    a * (half_turn.mul_add(half_turn, 2.0 * distance / a).sqrt() - half_turn)
}

/// Turns towards the change that takes the ship's velocity to `desired`,
/// and thrusts once it faces it; coasts once the velocity is within half
/// a tick's thrust of it. An inertialess ship matches its speed instead
/// ([`inertialess::match_speed`]).
fn match_velocity(
    state: &ShipState,
    handling: &Handling,
    desired: Vec2,
    inertia: Inertia,
) -> Controls {
    if let Inertia::Inertialess { speed } = inertia {
        return inertialess::match_speed(state, speed, handling, desired);
    }
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
        ..Controls::default()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::hail::Help;
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::traffic::emerge::EMERGE_TICKS;
    use crate::traffic::gate_entry::ENTRY_TICKS;
    use crate::traffic::npc::NpcId;
    use crate::traffic::spawn::{hyperspace_entry, jump_in_distance};
    use crate::traffic::warp_out::WarpGoal;
    use nova_sim_base::fixture::{Draws, FAST};
    use nova_sim_catalog::fixture::planet;
    use std::collections::BTreeMap;

    // The planning helpers flying with inertia, as their tests here fly
    // them (see `traffic::inertialess` for the inertialess arms).

    fn stopping_distance(state: &ShipState, handling: &Handling) -> f32 {
        super::stopping_distance(state, handling, Inertia::Inertial)
    }

    fn braking_speed(distance: f32, handling: &Handling) -> f32 {
        super::braking_speed(distance, handling, Inertia::Inertial)
    }

    fn brake(state: &ShipState, handling: &Handling) -> Controls {
        super::brake(state, handling, Inertia::Inertial)
    }

    fn match_velocity(state: &ShipState, handling: &Handling, desired: Vec2) -> Controls {
        super::match_velocity(state, handling, desired, Inertia::Inertial)
    }

    fn follow(state: &ShipState, handling: &Handling, lead: &ShipState) -> Controls {
        super::follow(state, handling, lead, Inertia::Inertial)
    }

    /// Flies `npc` at its own top speed, with no lead: the autopilot as
    /// most tests see it.
    fn fly(npc: &mut Npc, sites: &[LandingSite], other: Option<&ShipState>) -> Outcome {
        fly_at(npc, sites, other, TopSpeeds::own_of(npc))
    }

    /// The original's flight rules but `speed_cap`'s other reading.
    fn circle() -> FlightRules {
        FlightRules::from_rulebook(
            &crate::Rulebook::default()
                .with_override(crate::RuleKey::SpeedCap, crate::rulebook::RuleSource::Bible),
        )
    }

    /// The original's flight rules but `Accel` divided by `divisor`.
    fn thrust_over(divisor: u32) -> FlightRules {
        FlightRules::from_rulebook(&crate::Rulebook::default().with_value(
            crate::rulebook::RuleValue::ThrustDivisor,
            std::num::NonZeroU32::new(divisor).unwrap(),
        ))
    }

    /// Flies `npc` at its own top speed, with no lead, under `rules`.
    fn fly_own_under(npc: &mut Npc, other: Option<&ShipState>, rules: FlightRules) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        fly_under(npc, &[], other, speeds, rules)
    }

    /// Flies `npc` at `speeds` under `rules`, any wait it draws its
    /// longest.
    fn fly_under(
        npc: &mut Npc,
        sites: &[LandingSite],
        other: Option<&ShipState>,
        speeds: TopSpeeds,
        rules: FlightRules,
    ) -> Outcome {
        let weapons = crate::ai::fixture::records_of([&*npc]);
        super::fly(
            npc,
            sites,
            &BTreeMap::new(),
            other,
            false,
            None,
            speeds,
            rules,
            Readings::default(),
            &weapons,
            &mut Draws::of(&[]),
        )
    }

    /// Flies `npc` at `speeds`, any wait it draws its longest.
    fn fly_at(
        npc: &mut Npc,
        sites: &[LandingSite],
        other: Option<&ShipState>,
        speeds: TopSpeeds,
    ) -> Outcome {
        let weapons = crate::ai::fixture::records_of([&*npc]);
        super::fly(
            npc,
            sites,
            &BTreeMap::new(),
            other,
            false,
            None,
            speeds,
            FlightRules::default(),
            Readings::default(),
            &weapons,
            &mut Draws::of(&[]),
        )
    }

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
            ..crate::traffic::fixture::npc(1, ShipStats::new(fields, &[]))
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

    /// An escort in slot 2 of 3, spaced 30 apart, flying `fields` from
    /// `start`, keeping formation.
    fn escort(fields: ShipFields, start: ShipState) -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 3,
                    spacing: 30.0,
                },
                order: None,
            }),
            ..npc(fields, Goal::Formation { guard: None }, start)
        }
    }

    /// Slot 2 of 3, spaced 30 apart, behind `leader`.
    fn slot_of(leader: &ShipState) -> Vec2 {
        crate::escort::slot_position(leader, 2, 3, 30.0)
    }

    #[test]
    fn an_escort_on_its_slot_matches_the_leaders_velocity_and_heading() {
        let leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_of(&leader);
        let mut ship = escort(FAST, at(slot.x, slot.y, 0.0, 0.0, 200.0));
        assert_eq!(fly(&mut ship, &[], Some(&leader)), Outcome::Flying);
        assert_eq!(ship.state.velocity, leader.velocity);
        assert_eq!(ship.state.heading, 30.0);
        assert_eq!(
            ship.state.position,
            slot + leader.velocity,
            "moving with it"
        );
    }

    #[test]
    fn an_escort_keeping_formation_is_nudged_back_beyond_8_pixels_off_by_at_most_ten_ticks_thrust()
    {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_of(&leader);
        let off = |x: f32, y: f32| at(slot.x + x, slot.y + y, 0.0, 0.0, 0.0);
        let nudged = |fields: ShipFields, x: f32, y: f32| {
            let mut ship = escort(fields, off(x, y));
            fly(&mut ship, &[], Some(&leader));
            ship.state.position - slot
        };
        let near = |a: Vec2, b: Vec2| (a - b).length() < 1e-4;
        assert_eq!(
            nudged(FAST, 8.0, -8.0),
            Vec2::new(8.0, -8.0),
            "within the slack"
        );
        // FAST's 900 / 5000 is 0.18 a tick, ten ticks' 1.8.
        let fast = nudged(FAST, 9.0, -9.0);
        assert!(
            near(fast, Vec2::new(7.2, -7.2)),
            "0.18 x 10 a tick: {fast:?}"
        );
        let fast = nudged(FAST, 9.0, 2.0);
        assert!(
            near(fast, Vec2::new(7.2, 2.0)),
            "each axis on its own: {fast:?}"
        );
        let average = nudged(AVERAGE, -9.0, 0.0);
        assert!(
            near(average, Vec2::new(-8.4, 0.0)),
            "0.06 x 10: {average:?}"
        );
        let fast = nudged(FAST, 299.0, -300.0);
        assert!(near(fast, Vec2::new(297.2, -298.2)), "{fast:?}");
        let strong = ShipFields {
            accel: 30_000,
            ..FAST
        };
        assert_eq!(nudged(strong, 9.0, -20.0), Vec2::ZERO, "never past it");
        assert_eq!(
            (
                KEEP_FORMATION,
                APPROACH_FORMATION,
                FORMATION_SLACK,
                FORMATION_NUDGE
            ),
            (300.0, 600.0, 8.0, 10.0)
        );
    }

    #[test]
    fn an_escort_keeping_formation_is_nudged_by_ten_ticks_of_its_rules_thrust() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_of(&leader);
        let nudged = |rules: FlightRules| {
            let mut ship = escort(FAST, at(slot.x + 20.0, slot.y - 20.0, 0.0, 0.0, 0.0));
            fly_own_under(&mut ship, Some(&leader), rules);
            ship.state.position - slot
        };
        let near = |a: Vec2, b: Vec2| (a - b).length() < 1e-4;
        // FAST's 900 / 3000 is 0.3 a tick, ten ticks' 3; / 5000, 1.8.
        let quicker = nudged(thrust_over(3000));
        assert!(
            near(quicker, Vec2::new(17.0, -17.0)),
            "0.3 x 10 a tick: {quicker:?}"
        );
        let original = nudged(FlightRules::default());
        assert!(
            near(original, Vec2::new(18.2, -18.2)),
            "0.18 x 10 a tick: {original:?}"
        );
    }

    #[test]
    fn an_escort_off_its_slot_approaches_it_and_far_off_flies_at_full_speed() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_of(&leader);
        // At full speed (3) straight down at the slot from above.
        let mut near = escort(AVERAGE, at(slot.x, slot.y - 400.0, 0.0, 3.0, 180.0));
        let mut far = escort(AVERAGE, at(slot.x, slot.y - 700.0, 0.0, 3.0, 180.0));
        let before = (near.state, far.state);
        fly(&mut near, &[], Some(&leader));
        fly(&mut far, &[], Some(&leader));
        assert_ne!(
            near.state.heading, before.0.heading,
            "400 off, it turns to slow down"
        );
        assert_eq!(
            far.state.heading, before.1.heading,
            "700 off, it keeps going"
        );
        assert_eq!(far.state.velocity, before.1.velocity);
        assert_eq!(far.state.position, before.1.position + before.1.velocity);
        // From rest, facing the slot, either thrusts towards it.
        for distance in [301.0, 400.0, 700.0] {
            let mut ship = escort(AVERAGE, at(slot.x, slot.y - distance, 0.0, 0.0, 180.0));
            fly(&mut ship, &[], Some(&leader));
            assert!(ship.state.velocity.y > 0.0, "{distance}: {:?}", ship.state);
            assert_eq!(ship.state.heading, 180.0, "{distance}");
        }
        // Beyond 300 on one axis only, it flies rather than keeps formation.
        let mut aside = escort(AVERAGE, at(slot.x + 301.0, slot.y, 0.0, 0.0, 270.0));
        fly(&mut aside, &[], Some(&leader));
        assert!(aside.state.velocity.x < 0.0, "{:?}", aside.state);
        assert_eq!(aside.state.heading, 270.0, "not the leader's");
    }

    #[test]
    fn an_escort_whose_leader_is_gone_or_with_no_duty_brakes() {
        // Moving down and facing down, it turns to face back to brake.
        let mut lost = escort(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        fly(&mut lost, &[], None);
        assert_ne!(lost.state.heading, 180.0, "{:?}", lost.state);
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut dutiless = escort(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        dutiless.escort = None;
        fly(&mut dutiless, &[], Some(&leader));
        assert_ne!(dutiless.state.heading, 180.0, "{:?}", dutiless.state);
        let mut unjoined = escort(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        unjoined.escort = None;
        unjoined.mission = Some(mark(None));
        fly(&mut unjoined, &[], Some(&leader));
        assert_ne!(unjoined.state.heading, 180.0, "{:?}", unjoined.state);
    }

    /// A mark of slot 0's escorting mission, holding `formation`.
    fn mark(formation: Option<FormationSlot>) -> crate::traffic::npc::MissionMark {
        crate::traffic::npc::MissionMark {
            order: None,
            slot: 0,
            behav: crate::traffic::ShipBehav::EscortPlayer,
            formation,
        }
    }

    #[test]
    fn a_mission_escort_flies_its_formation_slot_with_no_escort_duty() {
        let leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_of(&leader);
        let mut ship = escort(FAST, at(slot.x, slot.y, 0.0, 0.0, 200.0));
        ship.escort = None;
        ship.mission = Some(mark(Some(FormationSlot {
            slot: 2,
            ships: 3,
            spacing: 30.0,
        })));
        assert_eq!(fly(&mut ship, &[], Some(&leader)), Outcome::Flying);
        assert_eq!(ship.state.velocity, leader.velocity);
        assert_eq!(ship.state.position, slot + leader.velocity);
        assert_eq!(ship.escort, None);
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
    fn a_ship_whose_outfits_drive_its_turn_to_none_still_turns_1_a_tick() {
        let unturning = [crate::fuel::OutfitMod {
            mod_type: crate::stats::TURN_CHANGE,
            mod_val: -200,
            count: 1,
        }];
        let stats = ShipStats::new(AVERAGE, &unturning);
        assert_eq!(stats.handling.turn_rate, 1.0, "floored");
        let sites = [planet(140, 800.0, 0.0)];
        let mut ship = Npc {
            state: at(0.0, 0.0, 0.0, 0.0, 0.0),
            goal: Goal::Land(StellarId(140)),
            ..crate::traffic::fixture::npc(1, stats)
        };
        for tick in 1..=5_u8 {
            fly(&mut ship, &sites, None);
            assert_eq!(ship.state.heading, f32::from(tick), "toward the east");
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
    fn a_folding_ship_landing_unfolds_near_its_stellar_too_fast_to_land() {
        let sites = [planet(140, 10.0, 0.0)];
        let folding = |x: f32, vx: f32| {
            let mut ship = npc(FAST, Goal::Land(StellarId(140)), at(x, 0.0, vx, 0.0, 0.0));
            ship.hull.frames.flags = crate::combat::ship_frames::FOLDS;
            ship.hull.frames.base_sets = 6;
            ship.armament.fold_mut().set = 5;
            ship
        };
        let mut near = folding(0.0, 1.5);
        assert_eq!(fly(&mut near, &sites, None), Outcome::Flying, "too fast");
        assert_eq!(near.armament.fold().dir, -1, "near: unfolding");
        let mut far = folding(-80.0, 0.0);
        fly(&mut far, &sites, None);
        assert_eq!(far.armament.fold().dir, 0, "too far");
        // Near a stellar far from the centre.
        let distant = [planet(140, 600.0, 0.0)];
        let mut there = folding(600.0, 1.5);
        fly(&mut there, &distant, None);
        assert_eq!(there.armament.fold().dir, -1, "near it, wherever it is");
    }

    #[test]
    fn a_step_records_its_drive_and_keeping_formation_marks_it() {
        let mut ship = npc(FAST, Goal::Idle, at(0.0, 0.0, 0.0, 0.0, 0.0));
        let flight = FlightRules::default();
        let handling = flight.handling(ship.stats.handling);
        let controls = Controls {
            thrust: true,
            turn: Turn::Left,
            ..Controls::default()
        };
        step(&mut ship, &handling, controls, flight, Inertia::Inertial);
        assert_eq!(ship.display.drive.turn, -1);
        assert_eq!(ship.display.drive.thrust, ship_display::Thrust::Plain);
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_of(&leader);
        let mut kept = escort(FAST, at(slot.x, slot.y, 0.0, 0.0, 0.0));
        fly(&mut kept, &[], Some(&leader));
        assert!(kept.display.drive.formation, "close formation");
        let mut far = escort(FAST, at(slot.x + 400.0, slot.y, 0.0, 0.0, 0.0));
        fly(&mut far, &[], Some(&leader));
        assert!(!far.display.drive.formation, "rejoining");
    }

    #[test]
    fn a_ship_not_intact_drifts_and_never_lands_or_jumps() {
        let sites = [planet(140, 0.0, 0.0)];
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 4,
                starting: false,
            },
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
            assert_eq!(fly(&mut jumper, &sites, None), Outcome::Flying);
            assert_eq!(jumper.state, drifted(condition), "drifting");
        }
    }

    /// Where a ship of `condition` at (0, -2000) moving (1, -2), heading
    /// 90°, is one drifting tick later: slowed first when disabled or
    /// breaking up, coasting when destroyed.
    fn drifted(condition: Condition) -> ShipState {
        let v = if condition == Condition::Destroyed {
            Vec2::new(1.0, -2.0)
        } else {
            Vec2::new(1.0, -2.0) * flight::DISABLED_DRAG
        };
        at(v.x, -2000.0 + v.y, v.x, v.y, 90.0)
    }

    #[test]
    fn a_disabled_or_breaking_up_ship_slows_by_0_995_a_tick_before_it_moves() {
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 4,
                starting: false,
            },
        ] {
            let mut ship = npc(AVERAGE, Goal::JumpOut, at(0.0, -2000.0, 1.0, -2.0, 90.0));
            ship.condition = condition;
            fly(&mut ship, &[], None);
            let damped = Vec2::new(1.0, -2.0) * flight::DISABLED_DRAG;
            assert_eq!(ship.state.velocity, damped, "{condition:?}");
            assert_eq!(ship.state.position, Vec2::new(0.0, -2000.0) + damped);
            let mut expected = damped;
            for _ in 1..10 {
                fly(&mut ship, &[], None);
                expected = expected * flight::DISABLED_DRAG;
            }
            assert_eq!(ship.state.velocity, expected, "{condition:?}");
        }
    }

    #[test]
    fn a_ship_breaking_up_or_destroyed_jumping_in_drifts_with_no_glide() {
        for condition in [
            Condition::Dying {
                ticks_left: 4,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            let mut jumper = npc(AVERAGE, Goal::JumpOut, at(0.0, -2000.0, 1.0, -2.0, 90.0));
            jumper.condition = condition;
            jumper.mode = Mode::JumpingIn { tick: 3 };
            assert_eq!(fly(&mut jumper, &[], None), Outcome::Flying);
            assert_eq!(jumper.state, drifted(condition), "drifting");
            assert_eq!(jumper.mode, Mode::JumpingIn { tick: 3 }, "no glide");
        }
    }

    /// A ship of `fields` coming out of a gate at (100, -200), heading 90°,
    /// `ticks_left` of its wait to go, bent on landing.
    fn emerging(fields: ShipFields, ticks_left: u32) -> Npc {
        let mut ship = npc(
            fields,
            Goal::Land(StellarId(140)),
            at(100.0, -200.0, 0.0, 0.0, 90.0),
        );
        ship.mode = Mode::Emerging { ticks_left };
        ship
    }

    #[test]
    fn a_ship_coming_out_of_a_gate_waits_there_still_then_leaves_it() {
        let sites = [planet(140, 0.0, 0.0)];
        let mut ship = emerging(AVERAGE, EMERGE_TICKS);
        assert!(ship.emerging());
        for left in (1..EMERGE_TICKS).rev() {
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying);
            assert_eq!(ship.mode, Mode::Emerging { ticks_left: left });
            assert_eq!(ship.state, at(100.0, -200.0, 0.0, 0.0, 90.0), "still");
        }
        fly(&mut ship, &sites, None);
        assert_eq!(ship.mode, Mode::LeavingGate { tick: 0 });
        assert!(!ship.emerging());
        assert_eq!(
            ship.state,
            at(100.0, -200.0, 0.0, 0.0, 90.0),
            "not yet moving"
        );
    }

    /// The last tick of a burst out of a gate, flown at `speeds`.
    fn burst_ends(speeds: TopSpeeds) -> u32 {
        let sites = [planet(140, 0.0, 0.0)];
        let mut ship = emerging(AVERAGE, 1);
        fly(&mut ship, &sites, None);
        for _ in 0..64 {
            let Mode::LeavingGate { tick } = ship.mode else {
                panic!("not bursting: {:?}", ship.mode);
            };
            fly_at(&mut ship, &sites, None, speeds);
            if !matches!(ship.mode, Mode::LeavingGate { .. }) {
                return tick;
            }
        }
        panic!("never ended")
    }

    #[test]
    fn a_burst_ends_at_the_lower_of_its_own_and_its_leads_top_speed() {
        let speeds = |own, lead| TopSpeeds { own, lead };
        // 30 - 1.165t: the burst ends once the next tick's is at or below
        // the cap, so at 3 after tick 23 (3.205, then 2.04), and at 6
        // after tick 20 (6.7, then 5.535).
        assert!((exit_speed(false, 23) - 3.205).abs() < 1e-4);
        assert_eq!(burst_ends(speeds(6.0, Some(3.0))), 23, "the lead's 3");
        assert_eq!(burst_ends(speeds(6.0, Some(9.0))), 20, "its own 6");
        assert_eq!(burst_ends(speeds(6.0, Some(6.0))), 20, "a tie: 6");
        assert_eq!(burst_ends(speeds(6.0, None)), 20, "no lead: its own");
        assert_eq!(burst_ends(speeds(3.0, None)), 23);
        assert_eq!(speeds(6.0, Some(3.0)).burst_cap(), 3.0);
        assert_eq!(speeds(3.0, Some(6.0)).burst_cap(), 3.0);
        assert_eq!(speeds(6.0, None).burst_cap(), 6.0);
    }

    #[test]
    fn flying_at_a_raised_top_speed_it_reaches_it_and_drifting_never_passes_it() {
        let raised = TopSpeeds {
            own: 4.5,
            lead: None,
        };
        let leader = at(20_000.0, 0.0, 0.0, 0.0, 90.0);
        let mut ship = escort(AVERAGE, at(0.0, 0.0, 0.0, 0.0, 90.0));
        let mut fastest: f32 = 0.0;
        for _ in 0..300 {
            fly_at(&mut ship, &[], Some(&leader), raised);
            fastest = fastest.max(ship.state.velocity.length());
        }
        assert!((fastest - 4.5).abs() < 1e-3, "{fastest}");
        let mut own = escort(AVERAGE, at(0.0, 0.0, 0.0, 0.0, 90.0));
        for _ in 0..300 {
            fly(&mut own, &[], Some(&leader));
        }
        assert!(own.state.velocity.length() <= 3.0 + 1e-4);
        let drifting = |speed: f32, rules: FlightRules| {
            let mut wreck = npc(AVERAGE, Goal::Idle, at(0.0, 0.0, speed, 0.0, 90.0));
            wreck.condition = Condition::Disabled;
            fly_under(&mut wreck, &[], None, raised, rules);
            wreck.state.velocity.length()
        };
        for rules in [FlightRules::default(), circle()] {
            assert!(
                (drifting(4.0, rules) - 4.0 * flight::DISABLED_DRAG).abs() < 1e-4,
                "under 4.5: kept, but for the drag"
            );
        }
        assert!(
            (drifting(10.0, FlightRules::default()) - 10.0 * flight::DISABLED_DRAG).abs() < 1e-4,
            "an NPC isn't clamped: dragged alone"
        );
        assert!(
            (drifting(10.0, circle()) - 4.5).abs() < 1e-4,
            "under the circle, dragged, then capped at 4.5"
        );
    }

    #[test]
    fn a_ship_led_out_of_a_gate_bursts_from_15() {
        let sites = [planet(140, 0.0, 0.0)];
        // Top speed 3: 15 - 1.165t for ticks 0 to 10, then on at 3.35
        // once 2.185 is next. Led by the player, whose top speed is its
        // lead's.
        let speeds = TopSpeeds {
            own: 3.0,
            lead: Some(3.0),
        };
        let mut ship = emerging(AVERAGE, 1);
        ship.led_out = true;
        fly(&mut ship, &sites, None);
        let mut x = 100.0;
        for tick in 0..=10 {
            let speed = 1.165f32.mul_add(-(tick as f32), 15.0);
            assert_eq!(ship.mode, Mode::LeavingGate { tick });
            fly_at(&mut ship, &sites, None, speeds);
            x += speed;
            assert!((ship.state.velocity - Vec2::new(speed, 0.0)).length() < 1e-4);
            assert!((ship.state.position - Vec2::new(x, -200.0)).length() < 1e-3);
        }
        assert_eq!(ship.mode, Mode::Flying);
    }

    #[test]
    fn leaving_a_gate_it_bursts_out_on_its_heading_until_it_is_down_to_its_top_speed() {
        let sites = [planet(140, 0.0, 0.0)];
        // Top speed 3: 30 - 1.165t for ticks 0 to 23, then on at 3.205
        // once 2.04 is next.
        let mut ship = emerging(AVERAGE, 1);
        fly(&mut ship, &sites, None);
        let mut x = 100.0;
        for tick in 0..=23 {
            let speed = 1.165f32.mul_add(-(tick as f32), 30.0);
            assert_eq!(ship.mode, Mode::LeavingGate { tick });
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying);
            x += speed;
            assert!(
                (ship.state.velocity - Vec2::new(speed, 0.0)).length() < 1e-4,
                "{tick}: {:?}",
                ship.state
            );
            assert!((ship.state.position - Vec2::new(x, -200.0)).length() < 1e-3);
            assert_eq!(ship.state.heading, 90.0, "no steering");
        }
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 59 }, "no lead");
        // A ship of top speed 30 or more is out after the first tick.
        let mut fast = emerging(
            ShipFields {
                speed: 3000,
                ..AVERAGE
            },
            1,
        );
        fly(&mut fast, &sites, None);
        fly(&mut fast, &sites, None);
        assert_eq!(fast.mode, Mode::Coasting { ticks_left: 59 });
        assert!((fast.state.velocity - Vec2::new(30.0, 0.0)).length() < 1e-4);
        // One of no speed bursts out until its speed would reach 0.
        let mut still = emerging(
            ShipFields {
                speed: 0,
                ..AVERAGE
            },
            1,
        );
        fly(&mut still, &sites, None);
        let ticks = (1..64)
            .find(|_| {
                fly(&mut still, &sites, None);
                !matches!(still.mode, Mode::LeavingGate { .. })
            })
            .expect("out");
        assert_eq!(ticks, 26, "30 down to 0.875");
    }

    #[test]
    fn a_ship_bursting_out_of_its_gate_is_out_of_it_from_its_first_burst_tick() {
        let sites = [planet(140, 0.0, 0.0)];
        let mut ship = emerging(AVERAGE, 1);
        ship.emerges_from = Some(StellarId(129));
        fly(&mut ship, &sites, None);
        assert_eq!(ship.mode, Mode::LeavingGate { tick: 0 });
        assert_eq!(ship.emerges_from, Some(StellarId(129)), "goal still 0x15");
        let before = ship.state.position;
        fly(&mut ship, &sites, None);
        assert_eq!(ship.mode, Mode::LeavingGate { tick: 1 });
        assert_ne!(ship.state.position, before, "bursting");
        assert_eq!(ship.emerges_from, None, "+0x6c = -1, @0x8dd93");
    }

    #[test]
    fn a_ship_not_intact_still_waits_out_its_gate_but_drifts_instead_of_bursting() {
        let mut ship = emerging(AVERAGE, 2);
        ship.condition = Condition::Disabled;
        ship.emerges_from = Some(StellarId(129));
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::Emerging { ticks_left: 1 });
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::LeavingGate { tick: 0 });
        assert_eq!(ship.emerges_from, Some(StellarId(129)));
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::Flying, "no burst");
        assert_eq!(ship.emerges_from, None, "out of the gate");
        assert_eq!(ship.state, at(100.0, -200.0, 0.0, 0.0, 90.0), "drifting");
    }

    #[test]
    fn a_ship_emerging_from_a_gate_holds_at_rest_for_its_ticks_then_flies_its_goal() {
        let start = at(100.0, 200.0, 3.0, -4.0, 45.0);
        let mut ship = npc(FAST, Goal::Attack(ShipRef::Player), start);
        ship.mode = Mode::Emerging { ticks_left: 3 };
        let player = at(600.0, 200.0, 0.0, 0.0, 0.0);
        for left in [2, 1, 0] {
            assert_eq!(fly(&mut ship, &[], Some(&player)), Outcome::Flying);
            assert_eq!(ship.state, at(100.0, 200.0, 0.0, 0.0, 45.0), "{left}");
            let mode = if left == 0 {
                Mode::LeavingGate { tick: 0 }
            } else {
                Mode::Emerging { ticks_left: left }
            };
            assert_eq!(ship.mode, mode);
        }
        while ship.mode != Mode::Flying {
            fly(&mut ship, &[], Some(&player));
            assert_eq!(ship.state.heading, 45.0, "bursting out, unsteered");
        }
        fly(&mut ship, &[], Some(&player));
        assert_ne!(ship.state.heading, 45.0, "turns on the player");
        let mut wreck = npc(FAST, Goal::Idle, start);
        wreck.condition = Condition::Disabled;
        wreck.mode = Mode::Emerging { ticks_left: 2 };
        fly(&mut wreck, &[], None);
        assert_eq!(wreck.state, at(100.0, 200.0, 0.0, 0.0, 45.0), "held too");
        assert_eq!(wreck.mode, Mode::Emerging { ticks_left: 1 });
    }

    #[test]
    fn a_hit_cuts_a_pirates_count_wait_to_20() {
        // `_DamageShip` reads no goal (@0x3be6e-0x3be7c): goal 0x16's
        // +0x4c is cut as any other.
        for (left, cut) in [(120, 20), (21, 20), (20, 20), (19, 19), (0, 0)] {
            let mut pirate = npc(AVERAGE, Goal::CountWait, at(0.0, 0.0, 0.0, 0.0, 0.0));
            pirate.ai_wait = left;
            cut_wait(&mut pirate);
            assert_eq!(pirate.ai_wait, cut, "{left}");
            assert_eq!(pirate.goal, Goal::CountWait, "{left}");
            assert_eq!(pirate.mode, Mode::Flying);
        }
    }

    #[test]
    fn a_hit_cuts_a_flying_ships_ai_wait_to_20() {
        for (left, cut) in [(179, 20), (21, 20), (20, 20), (19, 19), (0, 0)] {
            let mut pirate = npc(AVERAGE, Goal::Drift, at(0.0, 0.0, 0.0, 0.0, 0.0));
            pirate.ai_wait = left;
            cut_wait(&mut pirate);
            assert_eq!(pirate.ai_wait, cut, "{left}");
        }
    }

    #[test]
    fn a_hit_cuts_a_wait_in_or_at_a_gate_or_on_course_to_20_and_leaves_any_other_mode_alone() {
        let cut = |mode| {
            let mut ship = npc(AVERAGE, Goal::Idle, at(0.0, 0.0, 0.0, 0.0, 0.0));
            ship.mode = mode;
            cut_wait(&mut ship);
            ship.mode
        };
        assert_eq!(
            cut(Mode::Emerging { ticks_left: 60 }),
            Mode::Emerging { ticks_left: 20 }
        );
        assert_eq!(
            cut(Mode::Emerging { ticks_left: 10 }),
            Mode::Emerging { ticks_left: 10 }
        );
        assert_eq!(
            cut(Mode::Coasting { ticks_left: 45 }),
            Mode::Coasting { ticks_left: 20 }
        );
        assert_eq!(
            cut(Mode::HeldAtGate { ticks_left: 59 }),
            Mode::HeldAtGate { ticks_left: 20 },
            "a Good Samaritan's uncut hold"
        );
        assert_eq!(
            cut(Mode::HeldAtGate { ticks_left: 19 }),
            Mode::HeldAtGate { ticks_left: 19 }
        );
        for mode in [
            Mode::JumpingIn { tick: 2 },
            Mode::LeavingGate { tick: 2 },
            Mode::Flying,
        ] {
            assert_eq!(cut(mode), mode);
        }
    }

    #[test]
    fn a_ship_held_at_its_gate_holds_still_whatever_its_goal_then_flies_from_rest() {
        // `_HandleShip` skips its steering, thrust and fire while +0x4c
        // runs (@0x34a54-0x34a72), and `_HighLevelAIHandler` returns
        // (@0x8d4b9-0x8d4d2); goal 4 never becomes goal 8 (@0x8dd61).
        let player = at(600.0, 200.0, 0.0, 0.0, 0.0);
        for condition in [Condition::Intact, Condition::Disabled] {
            let mut ship = npc(
                FAST,
                Goal::Attack(ShipRef::Player),
                at(100.0, 200.0, 3.0, -4.0, 45.0),
            );
            ship.condition = condition;
            ship.mode = Mode::HeldAtGate { ticks_left: 3 };
            for left in [2, 1] {
                assert_eq!(fly(&mut ship, &[], Some(&player)), Outcome::Flying);
                assert_eq!(ship.state, at(100.0, 200.0, 0.0, 0.0, 45.0), "{left}");
                assert_eq!(ship.mode, Mode::HeldAtGate { ticks_left: left });
            }
            fly(&mut ship, &[], Some(&player));
            assert_eq!(ship.state, at(100.0, 200.0, 0.0, 0.0, 45.0));
            assert_eq!(ship.mode, Mode::Flying, "{condition:?}: no burst, no wait");
        }
        let mut ship = npc(
            FAST,
            Goal::Attack(ShipRef::Player),
            at(100.0, 200.0, 0.0, 0.0, 45.0),
        );
        ship.mode = Mode::HeldAtGate { ticks_left: 1 };
        fly(&mut ship, &[], Some(&player));
        fly(&mut ship, &[], Some(&player));
        assert_ne!(ship.state.heading, 45.0, "turns on the player");
    }

    #[test]
    fn a_ship_held_at_its_gate_flies_its_attack_from_rest_with_no_burst_and_keeps_goal_4() {
        // Every goal-4 substate the engine reaches writes +0x34 in the step
        // it first thrusts (sub 5 @0x85d44-0x85d59, 6 @0x85f7b, 7 @0x86957,
        // 0x10 @0x8645a, 0x11 @0x865b2), so `_HandleShip` takes ordinary
        // thrust (@0x34a8a-0x34bbb), never the burst (@0x34bc0) nor
        // `_AIMakeShipStopJumping` (@0x82e77) and its Rand(30)+30
        // (@0x34d5c-0x34d8b).
        let player = at(600.0, 200.0, 0.0, 0.0, 0.0);
        let mut ship = npc(
            FAST,
            Goal::Attack(ShipRef::Player),
            at(100.0, 200.0, 0.0, 0.0, 45.0),
        );
        ship.mode = Mode::HeldAtGate { ticks_left: 2 };
        let top = TopSpeeds::own_of(&ship);
        let mut draws = Draws::of(&[]);
        for tick in 0..120 {
            super::fly(
                &mut ship,
                &[],
                &BTreeMap::new(),
                Some(&player),
                false,
                None,
                top,
                FlightRules::default(),
                Readings::default(),
                &crate::ai::NO_WEAPONS,
                &mut draws,
            );
            assert!(
                !matches!(ship.mode, Mode::LeavingGate { .. } | Mode::Coasting { .. }),
                "tick {tick}: {:?}",
                ship.mode
            );
            assert_eq!(ship.goal, Goal::Attack(ShipRef::Player), "tick {tick}");
            // Each axis within its top speed, or a push past it.
            let reach = top.own + ship.stats.handling.accel + 1e-3;
            assert!(
                ship.state.velocity.x.abs() <= reach && ship.state.velocity.y.abs() <= reach,
                "tick {tick}: {:?} over {}",
                ship.state.velocity,
                top.own
            );
        }
        assert_eq!(ship.mode, Mode::Flying);
        assert!(ship.state.velocity.length() > 0.0, "it flies its attack");
        assert_eq!(draws.asked, Vec::<u32>::new(), "no post-burst roll");
    }

    #[test]
    fn a_ship_whose_gate_wait_ends_lets_go_of_its_trigger_as_it_bursts_out() {
        let mut ship = emerging(AVERAGE, 1);
        ship.trigger = Trigger {
            point_defence: true,
            ..Trigger::default()
        };
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::LeavingGate { tick: 0 });
        assert_eq!(ship.trigger, Trigger::default());
    }

    /// A ship of `fields` jumping in, at (0, -2000) heading 180°, on the
    /// glide's first tick.
    fn jumping_in(fields: ShipFields) -> Npc {
        let mut ship = npc(fields, Goal::Idle, at(0.0, -2000.0, 0.0, 0.0, 180.0));
        ship.mode = Mode::JumpingIn { tick: 0 };
        ship
    }

    /// The last tick of `ship`'s glide in, flown at `speeds`, and its
    /// speed after it.
    fn glide_ends(ship: &mut Npc, speeds: TopSpeeds) -> (u32, f32) {
        for _ in 0..50 {
            let Mode::JumpingIn { tick } = ship.mode else {
                panic!("not gliding: {:?}", ship.mode);
            };
            fly_at(ship, &[], None, speeds);
            if !matches!(ship.mode, Mode::JumpingIn { .. }) {
                return (tick, ship.state.velocity.length());
            }
            assert_eq!(ship.mode, Mode::JumpingIn { tick: tick + 1 });
        }
        panic!("never ended")
    }

    #[test]
    fn an_arriving_ship_ends_its_glide_on_the_first_tick_its_next_speed_reaches_the_cap_keeping_its_last_speed()
     {
        // 50 - 1.165 x 38 = 5.73 is at or below 6; 50 - 1.165 x 37 = 6.895
        // is not.
        let speeds = TopSpeeds {
            own: 6.0,
            lead: Some(9.0),
        };
        let mut ship = jumping_in(AVERAGE);
        let (last, speed) = glide_ends(&mut ship, speeds);
        assert_eq!(last, 37);
        assert_eq!(ship.mode, Mode::Flying);
        assert!((speed - glide_speed(37)).abs() < 1e-4, "{speed}");
        assert!((speed - 6.895).abs() < 1e-3, "not snapped to 6: {speed}");
        let along = flight::facing(180.0) * glide_speed(37);
        assert!((ship.state.velocity - along).length() < 1e-4);
    }

    #[test]
    fn a_jumping_in_escort_whose_lead_is_slower_ends_its_glide_at_its_leads_top_speed() {
        // A cap of 9 would end it on tick 35 (50 - 1.165 x 36 = 8.06); the
        // lead's 3 ends it on tick 40 (50 - 1.165 x 41 = 2.235).
        let speeds = TopSpeeds {
            own: 9.0,
            lead: Some(3.0),
        };
        let mut ship = jumping_in(AVERAGE);
        let (last, speed) = glide_ends(&mut ship, speeds);
        assert_eq!(last, 40);
        assert!((speed - glide_speed(40)).abs() < 1e-4, "{speed}");
        assert!((speed - 3.4).abs() < 1e-3, "{speed}");
    }

    #[test]
    fn a_ship_whose_cap_is_the_glides_second_speed_glides_one_tick_at_50() {
        for cap in [glide_speed(1), 100.0] {
            let speeds = TopSpeeds {
                own: cap,
                lead: Some(cap + 1.0),
            };
            let mut ship = jumping_in(AVERAGE);
            assert_eq!(glide_ends(&mut ship, speeds), (0, 50.0), "{cap}");
            assert!((ship.state.position - Vec2::new(0.0, -1950.0)).length() < 1e-3);
        }
    }

    #[test]
    fn a_disabled_ship_jumping_in_glides_in_as_an_intact_one_does() {
        let speeds = TopSpeeds {
            own: 6.0,
            lead: Some(9.0),
        };
        let mut intact = jumping_in(AVERAGE);
        let mut disabled = intact.clone();
        disabled.condition = Condition::Disabled;
        while intact.mode != Mode::Flying {
            assert_eq!(fly_at(&mut disabled, &[], None, speeds), Outcome::Flying);
            fly_at(&mut intact, &[], None, speeds);
            assert_eq!(disabled.state, intact.state);
            assert_eq!(disabled.mode, intact.mode);
        }
        assert_eq!(disabled.condition, Condition::Disabled);
    }

    #[test]
    fn a_glide_in_ends_at_the_burst_cap_keeping_its_speed_intact_or_disabled() {
        // A lead's 3 caps it below its raised 4.5: 50 - 1.165 x 41 = 2.235
        // is the first at or below 3, so it keeps tick 40's 3.4.
        let speeds = TopSpeeds {
            own: 4.5,
            lead: Some(3.0),
        };
        for condition in [Condition::Intact, Condition::Disabled] {
            let mut ship = jumping_in(AVERAGE);
            ship.condition = condition;
            ship.mode = Mode::JumpingIn { tick: 40 };
            fly_at(&mut ship, &[], None, speeds);
            assert_eq!(ship.mode, Mode::Flying, "{condition:?}");
            let along = flight::facing(180.0) * glide_speed(40);
            assert!(
                (ship.state.velocity - along).length() < 1e-4,
                "{condition:?}: {:?}",
                ship.state.velocity
            );
        }
    }

    /// The leaderless ship of `fields`, bent on landing on planet 140 at
    /// the centre, out of a gate at (100, -200) heading 90° once its burst
    /// is over, drawing `draws`: its burst's last tick done.
    fn burst_out(fields: ShipFields, speeds: TopSpeeds, draws: &mut Draws) -> Npc {
        let sites = [planet(140, 0.0, 0.0)];
        let mut ship = emerging(fields, 1);
        ship.trigger = Trigger {
            primary: true,
            ..Trigger::default()
        };
        super::fly(
            &mut ship,
            &sites,
            &BTreeMap::new(),
            None,
            false,
            None,
            speeds,
            FlightRules::default(),
            Readings::default(),
            &crate::ai::NO_WEAPONS,
            draws,
        );
        while let Mode::LeavingGate { .. } = ship.mode {
            super::fly(
                &mut ship,
                &sites,
                &BTreeMap::new(),
                None,
                false,
                None,
                speeds,
                FlightRules::default(),
                Readings::default(),
                &crate::ai::NO_WEAPONS,
                draws,
            );
        }
        ship
    }

    #[test]
    fn a_leaderless_ship_leaving_a_gate_holds_its_course_for_30_to_59_ticks_before_its_ai_acts() {
        let sites = [planet(140, 0.0, 0.0)];
        let speeds = TopSpeeds {
            own: 3.0,
            lead: None,
        };
        assert_eq!((POST_BURST_WAIT, POST_BURST_SPREAD), (30, 30));
        for (draw, wait) in [(0, 30), (29, 59)] {
            let mut draws = Draws::of(&[draw]);
            let mut ship = burst_out(AVERAGE, speeds, &mut draws);
            assert_eq!(ship.mode, Mode::Coasting { ticks_left: wait });
            assert_eq!(draws.asked, [30], "one Rand(30)");
            assert_eq!(ship.trigger, Trigger::default(), "holding fire");
            // Top speed 3: on at 3.205, its last burst speed.
            let velocity = ship.state.velocity;
            assert!((velocity - Vec2::new(3.205, 0.0)).length() < 1e-4);
            for left in (0..wait).rev() {
                let before = ship.state;
                assert_eq!(
                    super::fly(
                        &mut ship,
                        &sites,
                        &BTreeMap::new(),
                        None,
                        false,
                        None,
                        speeds,
                        FlightRules::default(),
                        Readings::default(),
                        &crate::ai::NO_WEAPONS,
                        &mut draws
                    ),
                    Outcome::Flying
                );
                assert_eq!(ship.state.velocity, velocity, "{left}: not slowed");
                assert_eq!(ship.state.heading, 90.0, "{left}: not turned");
                assert_eq!(ship.state.position, before.position + velocity);
                let mode = if left == 0 {
                    Mode::Flying
                } else {
                    Mode::Coasting { ticks_left: left }
                };
                assert_eq!(ship.mode, mode);
            }
            assert_eq!(draws.asked, [30], "no draw while it waits");
            super::fly(
                &mut ship,
                &sites,
                &BTreeMap::new(),
                None,
                false,
                None,
                speeds,
                FlightRules::default(),
                Readings::default(),
                &crate::ai::NO_WEAPONS,
                &mut draws,
            );
            assert_ne!(ship.state.heading, 90.0, "turns for the planet");
            // Nothing clamps an NPC: only its own thrust would slow it.
            assert!(
                ship.state.velocity.length() <= velocity.length() + ship.stats.handling.accel,
                "at the burst's speed or slowing: {:?}",
                ship.state
            );
        }
    }

    #[test]
    fn a_ship_with_a_lead_does_not_wait_after_its_burst() {
        // A fleet escort's lead, or the player for a ship the player leads:
        // both are a lead's top speed here.
        for lead in [3.0, 9.0] {
            let speeds = TopSpeeds {
                own: 3.0,
                lead: Some(lead),
            };
            let mut draws = Draws::of(&[]);
            let ship = burst_out(AVERAGE, speeds, &mut draws);
            assert_eq!(ship.mode, Mode::Flying, "{lead}");
            assert!(draws.asked.is_empty(), "{lead}: no draw");
            let mut arriving = jumping_in(AVERAGE);
            glide_ends(&mut arriving, speeds);
            assert_eq!(arriving.mode, Mode::Flying, "{lead}");
        }
    }

    #[test]
    fn a_leaderless_arriving_ship_waits_after_its_glide() {
        let speeds = TopSpeeds {
            own: 6.0,
            lead: None,
        };
        for (draw, wait) in [(0, 30), (29, 59)] {
            let mut ship = jumping_in(AVERAGE);
            let mut draws = Draws::of(&[draw]);
            while let Mode::JumpingIn { .. } = ship.mode {
                super::fly(
                    &mut ship,
                    &[],
                    &BTreeMap::new(),
                    None,
                    false,
                    None,
                    speeds,
                    FlightRules::default(),
                    Readings::default(),
                    &crate::ai::NO_WEAPONS,
                    &mut draws,
                );
            }
            assert_eq!(ship.mode, Mode::Coasting { ticks_left: wait });
            assert_eq!(draws.asked, [30]);
            let along = flight::facing(180.0) * glide_speed(37);
            assert!((ship.state.velocity - along).length() < 1e-4);
            let before = ship.state;
            super::fly(
                &mut ship,
                &[],
                &BTreeMap::new(),
                None,
                false,
                None,
                speeds,
                FlightRules::default(),
                Readings::default(),
                &crate::ai::NO_WEAPONS,
                &mut draws,
            );
            assert_eq!(ship.state.velocity, before.velocity, "coasting at 6.895");
            assert_eq!(ship.state.position, before.position + before.velocity);
        }
    }

    #[test]
    fn a_ship_not_intact_while_it_waits_drifts_instead_its_wait_running_down() {
        let speeds = TopSpeeds {
            own: 3.0,
            lead: None,
        };
        let mut ship = burst_out(AVERAGE, speeds, &mut Draws::of(&[0]));
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 30 });
        ship.condition = Condition::Disabled;
        fly_at(&mut ship, &[], None, speeds);
        assert_eq!(
            ship.mode,
            Mode::Coasting { ticks_left: 29 },
            "a disabled ship reaches the countdown too (@0x34424 → @0x34679 → @0x34a54 → @0x35026)"
        );
        assert!(
            ship.state.velocity.length() > 3.0,
            "drifting on past its top speed, as nothing clamps an NPC"
        );
        let mut circled = burst_out(AVERAGE, speeds, &mut Draws::of(&[0]));
        circled.condition = Condition::Disabled;
        fly_under(&mut circled, &[], None, speeds, circle());
        assert!(
            circled.state.velocity.length() <= 3.0 + 1e-4,
            "under the circle, drifting at its top speed"
        );
    }

    #[test]
    fn a_ship_intact_again_before_its_wait_is_out_holds_its_course_for_the_rest() {
        let mut ship = npc(AVERAGE, Goal::JumpOut, at(0.0, 0.0, 1.0, 0.0, 90.0));
        ship.mode = Mode::Coasting { ticks_left: 3 };
        ship.condition = Condition::Disabled;
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 2 });
        let drifted = ship.state;
        assert!(drifted.velocity.x < 1.0, "slowed as it drifts");
        ship.condition = Condition::Intact;
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 1 });
        assert_eq!(ship.state.velocity, drifted.velocity, "no thrust");
        assert_eq!(ship.state.heading, drifted.heading, "no turn");
        assert_eq!(
            ship.state.position,
            drifted.position + drifted.velocity,
            "on its course"
        );
        fly(&mut ship, &[], None);
        assert_eq!(ship.mode, Mode::Flying, "the wait is out");
    }

    // Entering a gate.

    /// Hypergate 300 at (500, 0), 100 wide: a reach of 25.
    fn hypergate() -> LandingSite {
        LandingSite {
            flags2: crate::gate::HYPERGATE,
            ..planet(300, 500.0, 0.0)
        }
    }

    const GATE: StellarId = StellarId(300);

    /// Flies `npc` among `sites`, its gates as wide as `widths` says.
    fn fly_widths(
        npc: &mut Npc,
        sites: &[LandingSite],
        widths: &BTreeMap<StellarId, TypeWidth>,
    ) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        super::fly(
            npc,
            sites,
            widths,
            None,
            false,
            None,
            speeds,
            FlightRules::default(),
            Readings::default(),
            &crate::ai::NO_WEAPONS,
            &mut Draws::of(&[]),
        )
    }

    #[test]
    fn outside_the_box_a_ship_entering_a_gate_flies_to_it_with_no_wait() {
        let sites = [hypergate()];
        let mut ship = npc(FAST, Goal::EnterGate(GATE), at(0.0, 0.0, 0.0, 0.0, 90.0));
        for _ in 0..20 {
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying);
            assert_eq!(ship.mode, Mode::Flying, "no countdown (+0x4c -1)");
        }
        assert!(ship.state.position.x > 10.0, "{:?}", ship.state);
        assert_eq!(ship.goal, Goal::EnterGate(GATE));
    }

    #[test]
    fn inside_the_box_it_stops_and_leaves_on_the_16th_tick() {
        let sites = [hypergate()];
        let start = at(525.9, -25.9, 1.0, -0.5, 90.0);
        let mut ship = npc(FAST, Goal::EnterGate(GATE), start);
        assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying);
        assert_eq!(ship.state.velocity, Vec2::ZERO, "stopped");
        assert_eq!(ship.state.position, start.position, "where it was");
        assert_eq!(
            ship.mode,
            Mode::EnteringGate {
                gate: GATE,
                ticks_left: ENTRY_TICKS - 1
            },
            "16, counted down this tick"
        );
        for tick in 2..ENTRY_TICKS {
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying, "{tick}");
            assert_eq!(ship.state.position, start.position);
            assert_eq!(
                ship.mode,
                Mode::EnteringGate {
                    gate: GATE,
                    ticks_left: ENTRY_TICKS - tick
                }
            );
        }
        assert_eq!(fly(&mut ship, &sites, None), Outcome::EnteredGate(GATE));
        let mut beyond = npc(FAST, Goal::EnterGate(GATE), at(526.0, 0.0, 0.0, 0.0, 90.0));
        fly(&mut beyond, &sites, None);
        assert_eq!(beyond.mode, Mode::Flying, "26 out: outside");
        let mut below = npc(FAST, Goal::EnterGate(GATE), at(500.0, 26.0, 0.0, 0.0, 90.0));
        fly(&mut below, &sites, None);
        assert_eq!(below.mode, Mode::Flying, "26 out on y: outside");
    }

    #[test]
    fn the_box_goes_by_the_gates_type_width() {
        let start = at(530.0, 0.0, 0.0, 0.0, 90.0);
        let entering = |site: LandingSite, widths: &BTreeMap<StellarId, TypeWidth>| {
            let mut ship = npc(FAST, Goal::EnterGate(GATE), start);
            fly_widths(&mut ship, &[site], widths);
            matches!(ship.mode, Mode::EnteringGate { .. })
        };
        let none = BTreeMap::new();
        assert!(!entering(hypergate(), &none), "100 wide: 25");
        let unloaded = BTreeMap::from([(GATE, TypeWidth::Unloaded)]);
        assert!(entering(hypergate(), &unloaded), "150 wide: 37");
        let near = at(508.0, 0.0, 0.0, 0.0, 90.0);
        let out_of_range = LandingSite {
            type_counts_frames: false,
            ..hypergate()
        };
        let mut ship = npc(FAST, Goal::EnterGate(GATE), near);
        fly_widths(&mut ship, &[out_of_range], &none);
        assert!(matches!(ship.mode, Mode::EnteringGate { .. }), "32 wide: 8");
        let mut ship = npc(FAST, Goal::EnterGate(GATE), at(509.0, 0.0, 0.0, 0.0, 90.0));
        fly_widths(&mut ship, &[out_of_range], &none);
        assert_eq!(ship.mode, Mode::Flying, "9 out of 8");
    }

    #[test]
    fn a_landing_goal_at_a_gate_is_gate_entry_from_its_first_tick() {
        let wormhole = LandingSite {
            flags2: crate::gate::WORMHOLE,
            ..planet(301, -500.0, 0.0)
        };
        let sites = [hypergate(), wormhole, planet(302, 0.0, 500.0)];
        let mut far = npc(FAST, Goal::Land(GATE), at(0.0, 0.0, 0.0, 0.0, 90.0));
        fly(&mut far, &sites, None);
        assert_eq!(far.goal, Goal::EnterGate(GATE));
        for (gate, x) in [(300, 500.0), (301, -500.0)] {
            let mut ship = npc(
                FAST,
                Goal::Land(StellarId(gate)),
                at(x, 0.0, 0.0, 0.0, 90.0),
            );
            assert_eq!(fly(&mut ship, &sites, None), Outcome::Flying, "not landed");
            assert_eq!(ship.goal, Goal::EnterGate(StellarId(gate)));
            assert_eq!(
                ship.mode,
                Mode::EnteringGate {
                    gate: StellarId(gate),
                    ticks_left: 15
                }
            );
            let (outcome, _) = until_gone(&mut ship, &sites, 100);
            assert_eq!(outcome, Outcome::EnteredGate(StellarId(gate)));
        }
        let mut lander = npc(
            FAST,
            Goal::Land(StellarId(302)),
            at(0.0, 500.0, 0.0, 0.0, 90.0),
        );
        assert_eq!(
            fly(&mut lander, &sites, None),
            Outcome::Landed(StellarId(302))
        );
        let mut lost = npc(
            FAST,
            Goal::Land(StellarId(999)),
            at(0.0, 0.0, 0.0, 0.0, 90.0),
        );
        fly(&mut lost, &sites, None);
        assert_eq!(
            lost.goal,
            Goal::Land(StellarId(999)),
            "no stellar: no hand-off"
        );
    }

    #[test]
    fn a_ship_disabled_or_set_on_another_goal_as_it_enters_still_leaves() {
        let sites = [hypergate()];
        for (condition, goal) in [
            (Condition::Disabled, Goal::EnterGate(GATE)),
            (Condition::Intact, Goal::Attack(ShipRef::Player)),
        ] {
            let mut ship = npc(FAST, goal, at(500.0, 0.0, 0.0, 0.0, 90.0));
            ship.mode = Mode::EnteringGate {
                gate: GATE,
                ticks_left: 3,
            };
            ship.condition = condition;
            ship.state.velocity = Vec2::new(2.0, 0.0);
            let player = at(0.0, 0.0, 0.0, 0.0, 0.0);
            assert_eq!(fly(&mut ship, &sites, Some(&player)), Outcome::Flying);
            assert_eq!(ship.state.position, Vec2::new(500.0, 0.0), "held still");
            assert_eq!(fly(&mut ship, &sites, Some(&player)), Outcome::Flying);
            assert_eq!(
                fly(&mut ship, &sites, Some(&player)),
                Outcome::EnteredGate(GATE),
                "{condition:?} {goal:?}"
            );
        }
        let mut gone = npc(FAST, Goal::Idle, at(0.0, 0.0, 0.0, 0.0, 90.0));
        gone.mode = Mode::EnteringGate {
            gate: GATE,
            ticks_left: 1,
        };
        assert_eq!(
            fly(&mut gone, &[], None),
            Outcome::EnteredGate(GATE),
            "the gate need not be shown"
        );
    }

    #[test]
    fn a_ship_entering_a_gate_not_shown_brakes() {
        let mut ship = npc(
            FAST,
            Goal::EnterGate(StellarId(999)),
            at(0.0, 0.0, 4.0, -3.0, 10.0),
        );
        for _ in 0..200 {
            assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
        }
        assert!(ship.state.velocity.length() < 0.5, "{:?}", ship.state);
        assert_eq!(ship.mode, Mode::Flying);
    }

    #[test]
    fn a_ship_at_rest_past_1000_begins_its_warp_out_without_moving() {
        let mut ship = npc(FAST, Goal::JumpOut, at(0.0, -1000.5, 0.0, 0.0, 0.0));
        assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
        assert_eq!(ship.warp.goal, Some(WarpGoal::Own), "sub-goal 4");
        assert!(ship.jumping());
        assert_eq!(ship.state.position, Vec2::new(0.0, -1000.5));
        let mut at_1000 = npc(FAST, Goal::JumpOut, at(0.0, -1000.0, 0.0, 0.0, 0.0));
        assert_eq!(fly(&mut at_1000, &[], None), Outcome::Flying);
        assert_eq!(at_1000.warp.goal, None);
    }

    #[test]
    fn a_jump_in_glides_from_about_2098_out_without_steering_until_its_cap() {
        let start = hyperspace_entry(&mut nova_sim_base::fixture::Draws::of(&[60]));
        assert!((start.position.length() - jump_in_distance()).abs() < 1e-2);
        let mut ship = npc(FAST, Goal::Land(StellarId(140)), start);
        ship.mode = Mode::JumpingIn { tick: 0 };
        let speeds = TopSpeeds {
            own: 6.0,
            lead: Some(9.0),
        };
        let sites = [planet(140, 0.0, 0.0)];
        let mut last = start.position.length();
        for tick in 0..=37 {
            assert_eq!(ship.mode, Mode::JumpingIn { tick });
            assert_eq!(fly_at(&mut ship, &sites, None, speeds), Outcome::Flying);
            let now = ship.state.position.length();
            assert!((last - now - glide_speed(tick)).abs() < 1e-2, "{tick}");
            assert_eq!(ship.state.heading, start.heading, "no steering");
            last = now;
        }
        assert_eq!(ship.mode, Mode::Flying);
        // Five ticks short of the 43 that would bring it 1000 out.
        let short: f32 = (38..43).map(glide_speed).sum();
        assert!((last - (1000.0 + short)).abs() < 0.1, "{last}");
        let inward = start.position * (-1.0 / jump_in_distance());
        assert!(
            (ship.state.velocity - inward * glide_speed(37)).length() < 1e-3,
            "{:?}",
            ship.state
        );
    }

    #[test]
    fn a_ship_with_no_speed_never_moves_but_to_match_a_lead_by_default() {
        let still = ShipFields { speed: 0, ..FAST };
        let sites = [planet(140, 300.0, 0.0)];
        let lead = at(500.0, 0.0, 2.0, 0.0, 90.0);
        // A leaver's run out thrusts whatever its top speed (sub 3,
        // `leave_flight`), so it is no goal that wants no speed.
        for goal in [
            Goal::Land(StellarId(140)),
            Goal::Follow(NpcId(0)),
            Goal::Idle,
        ] {
            let mut ship = npc(still, goal, at(0.0, 0.0, 0.0, 0.0, 45.0));
            let own = TopSpeeds::own_of(&ship);
            for _ in 0..300 {
                let flown = fly_under(&mut ship, &sites, Some(&lead), own, circle());
                assert_eq!(flown, Outcome::Flying);
            }
            assert_eq!(ship.state.position, Vec2::ZERO, "the circle: {goal:?}");
        }
        // A top speed of none caps nothing per axis (`_AdjustedAccel`
        // @0x2aecd-0x2aeee), so a follower matching its lead's velocity
        // gets it; the other goals want no speed and stay put.
        for goal in [Goal::Land(StellarId(140)), Goal::Idle] {
            let mut ship = npc(still, goal, at(0.0, 0.0, 0.0, 0.0, 45.0));
            for _ in 0..300 {
                assert_eq!(fly(&mut ship, &sites, Some(&lead)), Outcome::Flying);
            }
            assert_eq!(ship.state.position, Vec2::ZERO, "{goal:?}");
        }
        let mut follower = npc(still, Goal::Follow(NpcId(0)), at(0.0, 0.0, 0.0, 0.0, 45.0));
        for _ in 0..300 {
            fly(&mut follower, &sites, Some(&lead));
        }
        let off = follower.state.velocity - lead.velocity;
        assert!(
            off.length() <= follower.stats.handling.accel,
            "{:?}",
            follower.state
        );
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
    fn a_waiting_ship_brakes_to_a_stop_as_state_6_does() {
        // `_HighLevelAIHandler` flies state 6 as +0x50 = 0, sub 1 (@0x8f101).
        let start = at(0.0, 0.0, 4.0, -3.0, 10.0);
        let mut waiting = npc(FAST, Goal::Wait, start);
        let mut idle = npc(FAST, Goal::Idle, start);
        for _ in 0..200 {
            assert_eq!(fly(&mut waiting, &[], None), Outcome::Flying);
            fly(&mut idle, &[], None);
            assert_eq!(waiting.state, idle.state, "the idle ship's brake");
        }
        assert!(waiting.state.velocity.length() < 0.5, "{:?}", waiting.state);
    }

    #[test]
    fn a_mining_ship_brakes_to_a_stop_as_order_1_does() {
        let start = at(0.0, 0.0, 4.0, -3.0, 10.0);
        let mut miner = npc(FAST, Goal::Mine, start);
        let mut idle = npc(FAST, Goal::Idle, start);
        for _ in 0..200 {
            assert_eq!(fly(&mut miner, &[], None), Outcome::Flying);
            fly(&mut idle, &[], None);
            assert_eq!(miner.state, idle.state, "the idle ship's brake");
        }
        assert!(miner.state.velocity.length() < 0.5, "{:?}", miner.state);
    }

    #[test]
    fn an_assisting_ship_its_docking_doesnt_fly_coasts_as_a_drifting_one_does() {
        let player = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let start = at(-600.0, 400.0, 2.0, -1.0, 30.0);
        for (help, other) in [
            (Help::Refuel, Some(&player)),
            (Help::Repair, Some(&player)),
            (Help::Refuel, None),
        ] {
            let mut ship = npc(FAST, Goal::Assist(help), start);
            let mut drifting = npc(FAST, Goal::Hold, start);
            for _ in 0..30 {
                fly(&mut ship, &[], other);
                fly(&mut drifting, &[], other);
                assert_eq!(ship.state, drifting.state, "{help:?} {other:?}");
            }
            assert_eq!(ship.state.velocity, Vec2::new(2.0, -1.0), "no thrust");
        }
    }

    #[test]
    fn an_immobile_ship_given_a_velocity_stays_where_it_is() {
        let sites = [planet(140, 300.0, 0.0)];
        let lead = at(500.0, 0.0, 2.0, 0.0, 90.0);
        for goal in [
            Goal::Hold,
            Goal::Idle,
            Goal::JumpOut,
            Goal::Land(StellarId(140)),
            Goal::Follow(NpcId(0)),
        ] {
            let mut ship = npc(FAST, goal, at(10.0, 20.0, 3.0, -1.0, 45.0));
            ship.immobile = true;
            let mut thrust: f32 = 0.0;
            for tick in 0..30 {
                // A ship that may move, at rest where this one is: its
                // velocity after a tick is only that tick's thrust.
                let mut at_rest = ship.clone();
                at_rest.immobile = false;
                at_rest.state.velocity = Vec2::ZERO;
                fly(&mut at_rest, &sites, Some(&lead));
                assert_eq!(fly(&mut ship, &sites, Some(&lead)), Outcome::Flying);
                assert_eq!(ship.state.position, Vec2::new(10.0, 20.0), "{goal:?}");
                assert_eq!(
                    ship.state.velocity, at_rest.state.velocity,
                    "{goal:?} {tick}: the velocity is set to none before the thrust"
                );
                if matches!(goal, Goal::Hold | Goal::Idle) {
                    assert_eq!(ship.state.velocity, Vec2::ZERO, "{goal:?} {tick}");
                }
                thrust = thrust.max(ship.state.velocity.length());
                // A shove lasts only until its next tick.
                ship.state.velocity = ship.state.velocity + Vec2::new(5.0, 5.0);
            }
            if matches!(goal, Goal::Land(_) | Goal::Follow(_)) {
                assert!(thrust > 0.0, "{goal:?} thrusts from rest");
            }
        }
        let mut drifting = npc(FAST, Goal::Hold, at(10.0, 20.0, 3.0, -1.0, 45.0));
        fly(&mut drifting, &[], None);
        assert_eq!(
            drifting.state.position,
            Vec2::new(13.0, 19.0),
            "not immobile"
        );
    }

    #[test]
    fn a_held_ship_coasts_with_no_thrust_or_turn() {
        let mut ship = npc(AVERAGE, Goal::Hold, at(10.0, 20.0, 2.0, -1.0, 30.0));
        for _ in 0..10 {
            assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
        }
        assert_eq!(ship.state.velocity, Vec2::new(2.0, -1.0), "velocity kept");
        assert_eq!(ship.state.heading, 30.0, "no turn");
        assert_eq!(ship.state.position, Vec2::new(30.0, 10.0), "it coasts");
        let mut fast = npc(AVERAGE, Goal::Hold, at(0.0, 0.0, 10.0, 0.0, 30.0));
        fly(&mut fast, &[], None);
        assert_eq!(
            fast.state.velocity,
            Vec2::new(10.0, 0.0),
            "nothing clamps it"
        );
        let mut circled = npc(AVERAGE, Goal::Hold, at(0.0, 0.0, 10.0, 0.0, 30.0));
        let own = TopSpeeds::own_of(&circled);
        fly_under(&mut circled, &[], None, own, circle());
        let speed = circled.state.velocity.length();
        assert!(speed <= 3.0 + 1e-3, "under the circle, capped: {speed}");
        assert!(circled.state.velocity.y.abs() < 1e-3, "along its course");
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
        while ship.warp.goal.is_none() {
            assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
            last = ship.state.position;
        }
        let off = shortest_turn(heading_of(start.position), heading_of(last));
        assert!(off.abs() < 3.0, "{off}: {last:?}");
        // From the centre itself, straight up: the bearing out of (0, 0)
        // is the table's 0.
        let mut ship = npc(FAST, Goal::JumpOut, at(0.0, 0.0, 0.0, 0.0, 30.0));
        let mut last = Vec2::ZERO;
        while ship.warp.goal.is_none() {
            assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
            last = ship.state.position;
        }
        assert!(last.x.abs() < 25.0 && last.y < -1000.0, "{last:?}");
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
            ..Controls::default()
        }
    }

    #[test]
    fn attacking_from_afar_it_turns_to_the_lead_and_thrusts_within_four_turns() {
        // 3 degrees a tick: thrust under 12 off the target 1000 above,
        // strictly, in whole degrees (@0x868fb-0x8695e).
        let target = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        let thrusting = controls_of(&fighter(Goal::Attack(FOE), 11.9), &target);
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::Left));
        let turning = controls_of(&fighter(Goal::Attack(FOE), 12.0), &target);
        assert_eq!((turning.thrust, turning.turn), (false, Turn::Left));
        let other_way = controls_of(&fighter(Goal::Attack(FOE), 300.0), &target);
        assert_eq!(other_way.turn, Turn::Right);
    }

    #[test]
    fn dogfighting_within_165_it_thrusts_within_a_turn_and_15_degrees() {
        // 3 + 15: under 18 off the target 165 above, strictly, in whole
        // degrees (@0x85f1c-0x85f82), which the table bears at 46, its
        // diagonal a degree past 45.
        let target = at(165.0, -165.0, 0.0, 0.0, 0.0);
        let bearing = 46.0;
        let thrusting = controls_of(&fighter(Goal::Attack(FOE), bearing + 17.9), &target);
        assert!(thrusting.thrust);
        let turning = controls_of(&fighter(Goal::Attack(FOE), bearing + 18.0), &target);
        assert!(!turning.thrust);
        let afar = at(165.1, -165.0, 0.0, 0.0, 0.0);
        let approaching = controls_of(&fighter(Goal::Attack(FOE), bearing + 13.0), &afar);
        assert!(!approaching.thrust, "outside the box, within 12 only");
    }

    #[test]
    fn an_attacker_with_a_last_gun_leads_a_moving_target_with_its_pick() {
        use crate::catalog::{WeaponId, WeaponRecord};
        use crate::combat::armament::{Armament, LauncherRounds, Shooter};
        use crate::combat::weapon::WeaponSpec;
        let mut ship = fighter(Goal::Attack(FOE), 0.0);
        ship.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                speed: 1000,
                count: 50,
                ..nova_sim_catalog::fixture::weapon(128)
            }),
            1,
        )]);
        // It has fired its gun, its last gun, and holds it as its pick.
        let mut fuel = ship.reserves.fuel;
        ship.armament.fire_pick(
            WeaponId(128),
            Condition::Intact,
            Shooter::PLAYER,
            &mut LauncherRounds::default(),
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut crate::combat::report::Reports::default(),
            RuleSource::Engine,
            &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
            None,
        );
        ship.pick = Some(WeaponId(128));
        // 1000 above crossing right at 10 a tick: led about 1000 to the
        // right, near 45 degrees, in whole degrees as the ship moves.
        let crossing = at(0.0, -1000.0, 10.0, 0.0, 0.0);
        let mut flown = ship.clone();
        for _ in 0..30 {
            fly(&mut flown, &[], Some(&crossing));
        }
        let wanted = f32::from(fire::approach_lead(
            &flown,
            &crossing,
            flown.pick,
            RuleSource::Engine,
            &crate::ai::fixture::records_of([&flown]),
        ));
        assert_eq!(wanted, wanted.trunc(), "a whole degree");
        assert!((wanted - 45.0).abs() < 3.0, "{wanted}");
        let half_turn = flown.stats.handling.turn_rate / 2.0;
        assert!(
            shortest_turn(flown.state.heading, wanted).abs() <= half_turn,
            "{:?} {wanted}",
            flown.state
        );
    }

    #[test]
    fn sniping_it_brakes_to_a_stop_then_faces_its_target() {
        let target = at(-300.0, 0.0, 0.0, 0.0, 0.0);
        let mut moving = npc(FAST, Goal::Snipe(FOE), at(0.0, 0.0, 0.0, -3.0, 0.0));
        // It turns to its retro heading and thrusts down to 0.35 a tick on
        // each axis, then slows by 0.95 a tick.
        for _ in 0..400 {
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
        // The attacker 250 below: away is up, the table's 0; thrust under
        // 23 off it, strictly, in whole degrees (@0x85cf3-0x85d59).
        let attacker = at(0.0, 250.0, 0.0, 0.0, 0.0);
        let thrusting = controls_of(&fighter(Goal::Flee(FOE), 22.9), &attacker);
        assert_eq!((thrusting.thrust, thrusting.turn), (true, Turn::Left));
        let turning = controls_of(&fighter(Goal::Flee(FOE), 23.0), &attacker);
        assert!(!turning.thrust);
    }

    #[test]
    fn fleeing_from_afar_it_runs_out_from_the_centre_and_begins_its_warp_out_past_1000() {
        // The attacker 251 right: run straight out, up from (0, -10).
        let attacker = at(251.0, -10.0, 0.0, 0.0, 0.0);
        let mut runner = npc(FAST, Goal::Flee(FOE), at(0.0, -10.0, 0.0, 0.0, 0.0));
        (0..1000)
            .find(|_| {
                let outcome = fly(&mut runner, &[planet(140, 0.0, -10.0)], Some(&attacker));
                assert_eq!(outcome, Outcome::Flying, "never lands");
                runner.warp.goal.is_some()
            })
            .expect("its warp out begins");
        assert_eq!(runner.goal, Goal::Flee(FOE), "still fleeing");
        assert!(runner.jumping());
        assert!(runner.state.position.x.abs() < 1.0, "{:?}", runner.state);
        assert!(
            runner.state.position.length() > 1000.0,
            "only past 1000: {:?}",
            runner.state
        );
        // Who can jump, and braking first, are `leave_flight`'s tests.
    }

    /// A ship at rest at the centre facing `heading`, 3 degrees a tick,
    /// with `goal`, its hull standing off, carrying a beam 1000 long: a
    /// band of 850, and of 425 against a disabled target.
    fn stand_off(goal: Goal, heading: f32) -> Npc {
        use crate::catalog::WeaponRecord;
        use crate::combat::armament::Armament;
        use crate::combat::weapon::WeaponSpec;
        let mut ship = fighter(goal, heading);
        ship.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                guidance: 0,
                beam_length: 1000,
                ..nova_sim_catalog::fixture::weapon(128)
            }),
            1,
        )]);
        ship.hull.stands_off = true;
        ship
    }

    /// Flies `npc` a tick at a target at `other`, disabled as `disabled`
    /// says.
    fn fly_at_quarry(npc: &mut Npc, other: &ShipState, disabled: bool) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        super::fly(
            npc,
            &[],
            &BTreeMap::new(),
            Some(other),
            disabled,
            None,
            speeds,
            FlightRules::default(),
            Readings::default(),
            &crate::ai::NO_WEAPONS,
            &mut Draws::of(&[]),
        )
    }

    /// Whether `ship` thrusts this tick at a target at `other`, disabled
    /// as `disabled` says, and which way it turns.
    fn thrust_turn(ship: &Npc, other: &ShipState, disabled: bool) -> (bool, Turn) {
        let mut flown = ship.clone();
        let before = flown.state;
        fly_at_quarry(&mut flown, other, disabled);
        let turned = shortest_turn(before.heading, flown.state.heading);
        let turn = if turned > 0.0 {
            Turn::Right
        } else if turned < 0.0 {
            Turn::Left
        } else {
            Turn::None
        };
        (flown.state.velocity != before.velocity, turn)
    }

    #[test]
    fn a_stand_off_attacker_beyond_its_band_approaches() {
        // 3 degrees a tick: thrust under 12 off the target 1000 above.
        let target = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        let ship = stand_off(Goal::Attack(FOE), 11.9);
        assert_eq!(thrust_turn(&ship, &target, false), (true, Turn::Left));
        let ship = stand_off(Goal::Attack(FOE), 12.0);
        assert_eq!(thrust_turn(&ship, &target, false), (false, Turn::Left));
    }

    #[test]
    fn a_stand_off_attacker_within_its_band_brakes_then_faces_its_target() {
        let target = at(-600.0, 0.0, 0.0, 0.0, 0.0);
        let mut moving = stand_off(Goal::Attack(FOE), 0.0);
        moving.state.velocity = Vec2::new(0.0, -3.0);
        for _ in 0..200 {
            fly_at_quarry(&mut moving, &target, false);
        }
        // It snipes as `snipe::fly` says: it brakes below 0.35 on each
        // axis, then slows by `snipe::DAMPING` a tick.
        assert!(!snipe::moving(&moving.state), "{:?}", moving.state);
        assert!(moving.state.velocity.length() < 0.001, "{:?}", moving.state);
        let bearing = crate::combat::aim::bearing(moving.state.position, target.position);
        assert!(
            shortest_turn(moving.state.heading, bearing).abs() < 3.0,
            "{:?}",
            moving.state
        );
        let still = stand_off(Goal::Attack(FOE), 0.0);
        assert_eq!(thrust_turn(&still, &target, false), (false, Turn::Left));
    }

    #[test]
    fn a_stand_off_attacker_inside_165_turns_away_and_thrusts() {
        // The target 165 below: away is up, 0; thrust under 3 + 20 off.
        let target = at(0.0, 165.0, 0.0, 0.0, 0.0);
        let ship = stand_off(Goal::Attack(FOE), 22.9);
        assert_eq!(thrust_turn(&ship, &target, false), (true, Turn::Left));
        let ship = stand_off(Goal::Attack(FOE), 23.0);
        assert_eq!(thrust_turn(&ship, &target, false), (false, Turn::Left));
        let ship = stand_off(Goal::Attack(FOE), 0.0);
        assert_eq!(
            thrust_turn(&ship, &target, false),
            (true, Turn::None),
            "facing away already"
        );
    }

    #[test]
    fn a_stand_off_attacker_keeps_approaching_a_disabled_target_inside_its_full_band() {
        // 600 above: within 850, beyond 425.
        let target = at(0.0, -600.0, 0.0, 0.0, 0.0);
        let ship = stand_off(Goal::Attack(FOE), 0.0);
        assert_eq!(thrust_turn(&ship, &target, true), (true, Turn::None));
        assert_eq!(thrust_turn(&ship, &target, false), (false, Turn::None));
    }

    #[test]
    fn a_stand_off_sniper_flies_its_band_too() {
        let far = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        let ship = stand_off(Goal::Snipe(FOE), 0.0);
        assert_eq!(thrust_turn(&ship, &far, false), (true, Turn::None));
        let close = at(0.0, 100.0, 0.0, 0.0, 0.0);
        assert_eq!(thrust_turn(&ship, &close, false), (true, Turn::None));
        let within = at(0.0, -600.0, 0.0, 0.0, 0.0);
        assert_eq!(thrust_turn(&ship, &within, false), (false, Turn::None));
    }

    #[test]
    fn an_attacker_that_does_not_stand_off_closes_on_a_target_inside_the_band() {
        let mut ship = stand_off(Goal::Attack(FOE), 0.0);
        ship.hull.stands_off = false;
        let within = at(0.0, -600.0, 0.0, 0.0, 0.0);
        assert_eq!(thrust_turn(&ship, &within, false), (true, Turn::None));
        // Below and to the right, it turns to it rather than away.
        let close = at(10.0, 100.0, 0.0, 0.0, 0.0);
        assert_eq!(thrust_turn(&ship, &close, false).1, Turn::Right);
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
    fn a_sniper_whose_target_is_gone_coasts() {
        // `_LowLevelAIHandler`'s sub-goal 0xe does nothing with no target
        // (@0x870ab-0x870b9), its thrust zeroed at entry (@0x851f1).
        let mut ship = npc(FAST, Goal::Snipe(FOE), at(0.0, 0.0, 0.1, -0.2, 10.0));
        for _ in 0..10 {
            assert_eq!(fly(&mut ship, &[], None), Outcome::Flying);
        }
        assert_eq!(ship.state.velocity, Vec2::new(0.1, -0.2));
        assert_eq!(ship.state.heading, 10.0);
    }

    #[test]
    fn a_moving_sniper_turns_as_the_lead_reading_it_is_given_says() {
        // Moving down at 2 at heading 76, its target 300 right at rest,
        // having fired its gun of 10 a tick: by the engine it leads its
        // target (78.69) and thrusts; by the other reading it turns to
        // its retro heading, 0.
        let gun = crate::catalog::WeaponRecord {
            speed: 1000,
            count: 20,
            ..nova_sim_catalog::fixture::weapon(128)
        };
        let mut ship = npc(FAST, Goal::Snipe(FOE), at(0.0, 0.0, 0.0, 2.0, 76.0));
        ship.armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&gun),
            1,
        )]);
        let mut fuel = ship.reserves.fuel;
        ship.armament.fire_pick(
            crate::catalog::WeaponId(128),
            Condition::Intact,
            crate::combat::armament::Shooter::PLAYER,
            &mut crate::combat::armament::LauncherRounds::default(),
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut crate::combat::report::Reports::default(),
            RuleSource::Engine,
            &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
            None,
        );
        let target = at(300.0, 0.0, 0.0, 0.0, 0.0);
        let records = crate::ai::fixture::records_of([&ship]);
        let flown = |lead: RuleSource| {
            let mut flown = ship.clone();
            super::fly(
                &mut flown,
                &[],
                &BTreeMap::new(),
                Some(&target),
                false,
                None,
                TopSpeeds::own_of(&ship),
                FlightRules::default(),
                Readings {
                    snipe_lead: lead,
                    ..Readings::default()
                },
                &records,
                &mut Draws::of(&[]),
            );
            flown.state
        };
        let engine = flown(RuleSource::Engine);
        assert_eq!(engine.heading, 79.0, "a turn right");
        assert!(engine.velocity.y < 2.0, "thrust toward its target");
        let other = flown(RuleSource::Bible);
        assert_eq!(other.heading, 73.0, "a turn left");
        assert_eq!(other.velocity, Vec2::new(0.0, 2.0), "no thrust");
    }

    #[test]
    fn a_sniper_at_rest_slows_by_0_95_a_tick() {
        let target = at(-300.0, 0.0, 0.0, 0.0, 0.0);
        let mut ship = npc(FAST, Goal::Snipe(FOE), at(0.0, 0.0, 0.2, 0.0, 270.0));
        fly(&mut ship, &[], Some(&target));
        assert_eq!(ship.state.velocity, Vec2::new(0.2 * 0.95, 0.0));
        assert_eq!(ship.state.position, Vec2::new(0.2 * 0.95, 0.0));
    }

    #[test]
    fn a_fight_whose_ship_is_gone_brakes() {
        for goal in [Goal::Attack(FOE), Goal::Flee(FOE), Goal::Inspect(FOE)] {
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

    /// `npc` flown `ticks` ticks at a target at `other`, disabled as
    /// `disabled` says: where it ends up.
    fn flown_for(mut npc: Npc, other: &ShipState, disabled: bool, ticks: u32) -> ShipState {
        for _ in 0..ticks {
            fly_at_quarry(&mut npc, other, disabled);
        }
        npc.state
    }

    #[test]
    fn plundering_an_intact_ship_flies_as_an_attack_and_a_hopeless_one_as_a_snipe() {
        let start = at(0.0, 0.0, 1.0, -0.5, 30.0);
        let plunder = |hopeless| Goal::Plunder {
            ship: FOE,
            hopeless,
        };
        for target in [
            at(400.0, -300.0, 0.0, 0.0, 0.0),
            at(60.0, 40.0, 0.0, 0.0, 90.0),
        ] {
            assert_eq!(
                flown_for(npc(FAST, plunder(false), start), &target, false, 30),
                flown_for(npc(FAST, Goal::Attack(FOE), start), &target, false, 30),
                "{target:?}"
            );
        }
        let far = at(900.0, -600.0, 0.0, 0.0, 0.0);
        let sniped = flown_for(npc(FAST, Goal::Snipe(FOE), start), &far, false, 30);
        assert_eq!(
            flown_for(npc(FAST, plunder(true), start), &far, false, 30),
            sniped
        );
        assert_ne!(
            flown_for(npc(FAST, plunder(false), start), &far, false, 30),
            sniped
        );
    }

    #[test]
    fn plundering_a_disabled_ship_with_no_docking_yet_brakes() {
        let start = at(0.0, 0.0, 1.0, -0.5, 30.0);
        let victim = at(400.0, -300.0, 0.0, 0.0, 0.0);
        for hopeless in [false, true] {
            let plunder = Goal::Plunder {
                ship: FOE,
                hopeless,
            };
            assert_eq!(
                flown_for(npc(FAST, plunder, start), &victim, true, 300),
                flown_for(npc(FAST, Goal::Idle, start), &victim, true, 300)
            );
            assert_ne!(
                flown_for(npc(FAST, plunder, start), &victim, true, 300),
                flown_for(npc(FAST, plunder, start), &victim, false, 300),
                "intact, it fights"
            );
        }
    }

    #[test]
    fn while_its_ai_wait_runs_it_moves_on_at_its_velocity_whatever_its_goal() {
        // Above its top speed, turned away from its quarry.
        let start = at(10.0, 20.0, 9.0, -4.0, 30.0);
        for goal in [Goal::Attack(FOE), Goal::Idle, Goal::Drift, Goal::Hold] {
            let mut ship = npc(FAST, goal, start);
            ship.ai_wait = 1;
            let flown = flown_for(ship, &at(0.0, -400.0, 0.0, 0.0, 0.0), false, 1);
            assert_eq!(flown, at(19.0, 16.0, 9.0, -4.0, 30.0), "{goal:?}");
        }
        let mut ship = npc(FAST, Goal::Drift, start);
        ship.ai_wait = 0;
        assert_eq!(
            flown_for(ship, &start, false, 1),
            flown_for(npc(FAST, Goal::Hold, start), &start, false, 1),
            "a drift with none left coasts as a hold"
        );
    }

    #[test]
    fn a_count_wait_brakes_as_an_idle_ship_does() {
        let start = at(0.0, 0.0, 4.0, -3.0, 10.0);
        for ticks_left in [0, 1, 120] {
            let waiting = Npc {
                ai_wait: ticks_left,
                ..npc(FAST, Goal::CountWait, start)
            };
            assert_eq!(
                flown_for(waiting, &start, false, 40),
                flown_for(npc(FAST, Goal::Idle, start), &start, false, 40),
                "{ticks_left}"
            );
        }
    }

    // Docking.

    /// A fighter of `fields` returning from `start` to the player, its
    /// dock window 100 and the player's sprite 40 across.
    fn docking(fields: ShipFields, start: ShipState) -> Npc {
        Npc {
            ship: crate::catalog::ShipId(144),
            carrier: Some(crate::bay::Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            }),
            ..npc(fields, Goal::Dock(ShipRef::Player), start)
        }
    }

    #[test]
    fn a_fighter_outside_its_dock_window_closes_as_on_a_slot_and_far_off_at_full_speed() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // At full speed (3) straight down at the carrier from above.
        let mut near = docking(AVERAGE, at(0.0, -400.0, 0.0, 3.0, 180.0));
        let mut far = docking(AVERAGE, at(0.0, -700.0, 0.0, 3.0, 180.0));
        let before = (near.state, far.state);
        assert_eq!(fly(&mut near, &[], Some(&carrier)), Outcome::Flying);
        fly(&mut far, &[], Some(&carrier));
        assert_ne!(
            near.state.heading, before.0.heading,
            "400 off, it turns to slow down"
        );
        assert_eq!(
            far.state.heading, before.1.heading,
            "700 off, it keeps going"
        );
        assert_eq!(far.state.velocity, before.1.velocity);
        for distance in [101.0, 400.0, 700.0] {
            let mut ship = docking(AVERAGE, at(0.0, -distance, 0.0, 0.0, 180.0));
            fly(&mut ship, &[], Some(&carrier));
            assert!(ship.state.velocity.y > 0.0, "{distance}: {:?}", ship.state);
        }
    }

    #[test]
    fn inside_its_window_a_fighter_turns_at_the_carrier_and_thrusts_within_a_turn_and_a_degree() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 60 above the carrier, facing away: it turns, and only turns.
        let mut away = docking(AVERAGE, at(0.0, -60.0, 0.0, 0.0, 0.0));
        fly(&mut away, &[], Some(&carrier));
        assert_ne!(away.state.heading, 0.0, "turning");
        assert_eq!(away.state.velocity, Vec2::ZERO, "no thrust");
        // A turn (1) and 1.0 off the bearing (180): it thrusts.
        for heading in [178.0, 182.0, 180.0] {
            let mut aimed = docking(AVERAGE, at(0.0, -60.0, 0.0, 0.0, heading));
            fly(&mut aimed, &[], Some(&carrier));
            assert!(aimed.state.velocity.length() > 0.0, "{heading}");
        }
        for heading in [177.0, 183.0] {
            let mut aside = docking(AVERAGE, at(0.0, -60.0, 0.0, 0.0, heading));
            fly(&mut aside, &[], Some(&carrier));
            assert_eq!(aside.state.velocity, Vec2::ZERO, "{heading}");
        }
        assert_eq!(DOCK_TURN_SLACK, 1.0);
    }

    #[test]
    fn inside_its_window_a_fighter_is_pulled_by_ten_ticks_thrust_on_each_axis_beyond_it() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // At rest, facing away from the carrier at (-70, 1.5): FAST pulls
        // 0.18 x 10 = 1.8 a tick, and y is within that.
        let near = |a: Vec2, b: Vec2| (a - b).length() < 1e-4;
        let mut ship = docking(FAST, at(-70.0, 1.5, 0.0, 0.0, 270.0));
        fly(&mut ship, &[], Some(&carrier));
        assert!(
            near(ship.state.position, Vec2::new(-68.2, 1.5)),
            "{:?}",
            ship.state
        );
        let mut both = docking(FAST, at(50.0, 60.0, 0.0, 0.0, 135.0));
        fly(&mut both, &[], Some(&carrier));
        assert!(
            near(both.state.position, Vec2::new(48.2, 58.2)),
            "{:?}",
            both.state
        );
        // An axis exactly the pull off is within it, and stays.
        let pull = ShipStats::new(FAST, &[]).handling.accel * FORMATION_NUDGE;
        let mut edge = docking(FAST, at(-pull, 60.0, 0.0, 0.0, 135.0));
        fly(&mut edge, &[], Some(&carrier));
        assert_eq!(edge.state.position.x, -pull, "not farther than the pull");
        assert_eq!(edge.state.position.y, 60.0 - pull);
    }

    #[test]
    fn inside_its_window_a_fighter_is_pulled_by_ten_ticks_of_its_rules_thrust() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let near = |a: Vec2, b: Vec2| (a - b).length() < 1e-4;
        let pulled = |start: ShipState, rules: FlightRules| {
            let mut ship = docking(FAST, start);
            fly_own_under(&mut ship, Some(&carrier), rules);
            ship.state.position
        };
        // FAST's 900 / 3000 is 0.3 a tick, ten ticks' 3; y's 1.5 within it.
        let quicker = pulled(at(-70.0, 1.5, 0.0, 0.0, 270.0), thrust_over(3000));
        assert!(near(quicker, Vec2::new(-67.0, 1.5)), "{quicker:?}");
        let both = pulled(at(50.0, 60.0, 0.0, 0.0, 135.0), thrust_over(3000));
        assert!(near(both, Vec2::new(47.0, 57.0)), "{both:?}");
        let original = pulled(at(50.0, 60.0, 0.0, 0.0, 135.0), FlightRules::default());
        assert!(near(original, Vec2::new(48.2, 58.2)), "{original:?}");
    }

    #[test]
    fn a_sluggish_fighter_far_off_on_one_axis_flies_at_full_speed() {
        // 0.01 of acceleration and half a degree a tick: it could not
        // brake from its top speed within 700.
        const SLUGGISH: ShipFields = ShipFields {
            accel: 30,
            maneuver: 5,
            ..AVERAGE
        };
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let start = at(0.0, -700.0, 0.0, 3.0, 180.0);
        let mut far = docking(SLUGGISH, start);
        fly(&mut far, &[], Some(&carrier));
        assert_eq!(far.state.heading, 180.0, "700 off on y, it keeps going");
        assert_eq!(far.state.velocity, start.velocity);
        let mut near = docking(SLUGGISH, at(0.0, -500.0, 0.0, 3.0, 180.0));
        fly(&mut near, &[], Some(&carrier));
        assert_ne!(near.state.heading, 180.0, "500 off, it turns to brake");
    }

    #[test]
    fn within_the_carriers_reach_on_both_axes_a_fighter_docks() {
        let carrier = at(500.0, 500.0, 1.0, 1.0, 0.0);
        for (x, y) in [(40.0, 40.0), (-40.0, 0.0), (0.0, -40.0), (0.0, 0.0)] {
            let mut ship = docking(FAST, at(500.0 + x, 500.0 + y, 0.0, 0.0, 0.0));
            assert_eq!(
                fly(&mut ship, &[], Some(&carrier)),
                Outcome::Docked {
                    carrier: ShipRef::Player,
                    ship: crate::catalog::ShipId(144)
                },
                "({x}, {y})"
            );
        }
        for (x, y) in [(41.0, 0.0), (0.0, -41.0), (60.0, 30.0)] {
            let mut ship = docking(FAST, at(500.0 + x, 500.0 + y, 0.0, 0.0, 0.0));
            assert_eq!(
                fly(&mut ship, &[], Some(&carrier)),
                Outcome::Flying,
                "({x}, {y})"
            );
        }
    }

    #[test]
    fn a_fighter_whose_carrier_is_gone_or_with_none_brakes() {
        let mut lost = docking(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        assert_eq!(fly(&mut lost, &[], None), Outcome::Flying);
        assert_ne!(lost.state.heading, 180.0, "{:?}", lost.state);
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut carrierless = docking(AVERAGE, at(0.0, 0.0, 0.0, 2.0, 180.0));
        carrierless.carrier = None;
        assert_eq!(fly(&mut carrierless, &[], Some(&carrier)), Outcome::Flying);
        assert_ne!(carrierless.state.heading, 180.0, "{:?}", carrierless.state);
    }
}
