//! A pirate docking with the disabled ship it plunders, or a ship with the
//! disabled player it repairs: goal 0xd's
//! disabled half of `_HighLevelAIHandler` (@0x8e873-0x8ea44 in the `EV
//! Nova` executable, [`high_level`]) and the substates 9, 0xb and 0xf of
//! `_LowLevelAIHandler` it leaves the pirate in (@0x8769a-0x878f4,
//! @0x87337-0x87692, @0x86a08-0x8709a, [`low_level`]). The docking it keeps from
//! tick to tick, its sub-goal and nav (+0xc8ca, +0x6c), is
//! [`Npc::docking`].
//!
//! - **The high level** ([`high_level`]), every think of a plunder whose
//!   victim is disabled: with the victim boarded (+0xb9) and no AI wait
//!   left, it gives up (goal 0, sub 0, target and nav -1, @0x8e88f-0x8e8b9);
//!   otherwise nav = target and, within [`dock_box`] of the victim on both
//!   axes, inclusive, it docks (sub 0xf); within twice the box it closes
//!   (sub 0xb); farther off it approaches (sub 9). An inertialess ship's
//!   box ([`Npc::flies_inertialess`]) is [`INERTIALESS_BOX`] times as wide
//!   (@0x8e8f6-0x8e916). The original then takes the box as a 16-bit word
//!   (`movswl` @0x8e91b), which isn't replicated. A pirate's every-tick
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
//!   ([`CLOSE_CAP`]). Closing with a swarm leader (+0xc906 > 0), it first
//!   takes its swarm leader's glow ([`swarm_branch`], @0x87364-0x873a3);
//!   a docking ship is of AI type 4 or below, so the marking pass leaves
//!   its formation parent (+0xc92e) alone: with none,
//!   `_AIMaintainFormation` returns at once (@0x84f30), and with a stale
//!   one it nudges the ship toward the place it holds
//!   ([`swarm_low_level::slot_place`]). Both fire nothing: they end in
//!   `_AIEscortFireUnprovoked`, which does nothing for AI type 4 or below.
//!   An escort keeping formation flies them too
//!   ([`formation_flight`](crate::traffic::formation_flight)).
//! - **Dock** (sub 0xf, [`DockSub::Dock`]), while it is intact
//!   (`_LowLevelAIHandler`'s -0x32d flag, `!_IsDisabled`, @0x85240):
//!   - **Matching** while its velocity differs from the victim's by
//!     [`MATCH_SLACK`] or more on either axis (@0x86a30-0x86aa4): it turns
//!     against the difference (the table angle of the difference + 180,
//!     @0x86b3d-0x86ba0) and thrusts within its turn and
//!     [`THRUST_SLACK`]; with the difference at most [`DAMP_BELOW`] on
//!     either axis, it is cut to [`DAMPING`] of itself and the velocity set
//!     to the victim's plus it (@0x86c08-0x86cb2). An inertialess ship
//!     matching instead takes a tick's acceleration off its speed
//!     ([`Npc::speed`]), down to none and not capped at its top speed, with
//!     no thrust or damping (@0x86aaa-0x86b38), wanting its own whole
//!     heading (the low level's default +0x68, @0x8522d); its velocity
//!     follows its heading at that speed as it flies.
//!   - **Aligned** (@0x86cb7): it wants the victim's heading and takes its
//!     velocity. Outside [`DOCKED_BOX`] on either axis it creeps a pixel a
//!     tick towards it, its speed zeroed, inertialess or not
//!     (@0x8700f-0x87095, @0x8702d). Inside, its speed is left alone:
//!     while its AI wait is above [`DOCK_WAIT`] it holds; above none, under
//!     goal 0xd it plunders its nav ([`DockOutcome::Completed`],
//!     @0x86f3c-0x86f93, the traffic's `dispatch_dock`). A pirate's
//!     think turns a live boarded victim's plunder into goal 4 first, so
//!     it gets here once the victim's slot has gone and its think, under
//!     the dock wait, writes goal 0xd on another ship, the warship's pick
//!     (see [`pirate`](crate::ai::pirate)): it plunders the record, which
//!     yields nothing, or a ship that took the berth since. Under goal 0xf it
//!     repairs its nav, the player or an NPC ([`DockOutcome::Repaired`],
//!     @0x86d84-0x86f37, see
//!     [`repair_dock`](crate::traffic::repair_dock)), under goal 4 or 9
//!     nothing; at none, unless drifting (goal 0xe), it starts the dock
//!     (@0x86fa4-0x8700a, [`DockOutcome::Started`]): the victim boarded,
//!     its AI wait [`DOCK_WAIT`] + `Rand(`[`DOCK_WAIT_SPREAD`]`)`, its
//!     velocity the victim's. The start and the completion are two
//!     branches of one pass, so a pass that starts a dock completes none.
//! - **At the slot.** All of this is the low level ([`low_level`]), which
//!   runs at the ship's own slot of `_AIDispatch`'s loop (the traffic's
//!   `dispatch_dock`): the start, the completion, the creep, the match and
//!   the approach's and close's swarm branch are written there, so a
//!   higher slot's think reads them that tick and a lower one's as the
//!   tick began. Double time's second pass runs no low level.
//! - **The turn**: subs 9, 0xb, 0xf and 1 set only the heading they want
//!   (+0x68; sub 1's is [`leave_flight::stop_wanting`]'s), and
//!   `_HandleShip` turns the ship to it ([`flight::turn_to`],
//!   @0x34460-0x3458e): by its turn while it is off by more than that,
//!   whole degrees, and onto it otherwise ([`handle_ship`], in the fly
//!   half, on both passes, by the [`DockFlight`] the low level left).
//! - While its AI wait runs `_HandleShip` skips its turn and thrust
//!   (@0x3443f-0x34454, @0x34a54-0x34a72): it moves on at the velocity
//!   the substate left it, its heading where it was.
//!
//! A ship repairing the player, or an NPC, flies these substates too, by its own high level
//! ([`repair_dock::high_level`](crate::traffic::repair_dock::high_level)),
//! and a ship refuelling another flies subs 0xb and 1, the stop
//! ([`DockSub::Stop`], [`refuel_dock`](crate::traffic::refuel_dock)).
//! Nothing here plays a sound.

use crate::ai::fire::off_by;
use crate::ai::swarm::SwarmLead;
use crate::ai::{Decision, Goal};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::flight::{self, Controls, FlightRules, ShipState};
use crate::geometry::Vec2;
use crate::hail::Help;
use crate::handling::Handling;
use crate::inertialess::InertialDampener;
use crate::traffic::formation_flight;
use crate::traffic::held_burst::{self, Word};
use crate::traffic::inertialess::{self, Inertia};
use crate::traffic::leave_flight;
use crate::traffic::npc::Npc;
use crate::traffic::ship_display::GlowCopy;
use crate::traffic::swarm_low_level;
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
/// How many times the docking box an inertialess ship's is (`leal
/// (,%ebx,4)`: goal 0xd's @0x8e904, goal 0xf's @0x8f786).
pub const INERTIALESS_BOX: i32 = 4;

/// The docking a pirate keeps from tick to tick: the ship it docks with
/// (its nav, +0x6c) and the substate it flies (+0xc8ca).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Docking {
    /// The ship it docks with.
    pub ship: ShipRef,
    /// The substate it flies.
    pub sub: DockSub,
}

/// Whether `npc` keeps its docking as it takes `decision`, its think's:
/// under a fight, a plunder or an assist, whose high level then decides
/// it; under any other goal only while the AI wait skips the high level
/// that goal's state would rewrite the sub in (@0x8d4b9-0x8d4d2,
/// [`Npc::high_level_waits_under`]), as sub 0xf and the nav fly on at the
/// ship docked with whatever the think writes. A think that ends its wait
/// ([`Decision::ends_wait`]) is left to that tick's high level, which then
/// runs and drops it. A think that writes the sub itself
/// ([`Decision::writes_sub`]) ends it either way.
#[must_use]
pub fn kept_by(npc: &Npc, decision: &Decision) -> bool {
    if decision.writes_sub {
        return false;
    }
    let goal = decision.goal;
    matches!(
        goal,
        Goal::Attack(_) | Goal::Snipe(_) | Goal::Plunder { .. } | Goal::Assist(_)
    ) || npc.high_level_waits_under(goal)
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
    /// Sub 1: it stops, with goal 9's damping off its retro heading
    /// ([`leave_flight::stop_wanting`]). A refuel's alone
    /// ([`refuel_dock`](crate::traffic::refuel_dock)).
    Stop,
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

/// The box `side`, [`INERTIALESS_BOX`] times as wide when `npc` flies
/// inertialess, a dampener counting as `rule` says: `_ShipIsInertialess`
/// with no leader, as a plunder and a repair are no formation. The ×4 is
/// on the truncated integer, so a fractional box truncates first.
#[must_use]
pub(crate) fn inertialess_box(npc: &Npc, side: i32, rule: InertialDampener) -> i32 {
    if npc.hull_inertialess(rule) {
        side * INERTIALESS_BOX
    } else {
        side
    }
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
/// `boarded` saying whether the victim is boarded, its box
/// [`INERTIALESS_BOX`] times as wide when it flies inertialess, a
/// dampener counting as `rule` says (see the module docs).
#[must_use]
pub fn high_level(npc: &Npc, victim: Vec2, boarded: bool, rule: InertialDampener) -> DockStep {
    if boarded && npc.ai_wait == 0 {
        return DockStep::GiveUp;
    }
    // f32: the box is converted back to a float for the test.
    #[allow(clippy::cast_precision_loss)]
    let side = inertialess_box(npc, dock_box(npc.hull.class_turn), rule) as f32;
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
    /// Docked with no wait left, it started the dock (@0x86fa4-0x8700a):
    /// its wait [`DOCK_WAIT`] + `Rand(`[`DOCK_WAIT_SPREAD`]`)` and its
    /// velocity the victim's. Boarding the victim (+0xb9 = 1, @0x86fc7) is
    /// its caller's.
    Started,
    /// Docked under goal 0xd with its wait at most [`DOCK_WAIT`]: it
    /// plunders its nav (see the module docs), its caller's
    /// ([`plunder::plunder`](crate::traffic::plunder::plunder)).
    Completed,
    /// Docked under goal 0xf (a repair of the player) with its wait above
    /// none and at most [`DOCK_WAIT`]: the repair completes
    /// ([`repair_dock::complete`](crate::traffic::repair_dock::complete)).
    Repaired,
}

/// What a docking ship's low level leaves `_HandleShip` to fly: its
/// heading wanted (+0x68) and its thrust word (+0x30, +0x34), with the
/// handling and the inertia it flies them by. `_HandleShip` flies it on
/// both of double time's passes, the second running no low level
/// ([`handle_ship`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockFlight {
    /// The controls but the turn.
    controls: Controls,
    /// The handling it turns by.
    handling: Handling,
    /// The handling it thrusts by: closing's cap within [`CLOSE_CAP_BOX`],
    /// sub 1's brake.
    flown: Handling,
    /// The heading it wants, +0x68.
    wanted: i16,
    /// How it flies.
    inertia: Inertia,
    /// What it does to +0x34: sub 1, the refuel's stop, leaves it alone, so
    /// its brake bursts an owed burst out.
    word: Word,
}

impl DockFlight {
    /// The flight of a ship whose low level set no heading and no thrust
    /// (the low level's default +0x68, its own whole heading, @0x8522d):
    /// `npc`'s on `handling`, under `flight`.
    #[must_use]
    pub fn none(npc: &Npc, handling: &Handling, flight: FlightRules) -> Self {
        Self {
            controls: Controls::default(),
            handling: *handling,
            flown: *handling,
            wanted: trig::whole(npc.state.heading),
            inertia: Inertia::of(npc, None, true, flight),
            word: Word::Writes,
        }
    }
}

/// `npc`'s low level of its docking with a victim at `victim`, on
/// `handling` under `flight` (an inertialess ship as the autopilot flies
/// one, see [`inertialess`](crate::traffic::inertialess)), with `swarm`
/// its swarm lead, rolling a dock's wait on `chance` (see the module
/// docs): its writes to its own velocity, speed and place (matching, the
/// creep, the start), the flight it leaves `_HandleShip`
/// ([`handle_ship`]), and what became of the docking. It runs at the
/// ship's own slot of `_AIDispatch`'s loop (the traffic's
/// `dispatch_dock`). With no docking it sets nothing.
pub fn low_level(
    npc: &mut Npc,
    victim: &ShipState,
    handling: &Handling,
    flight: FlightRules,
    swarm: Option<SwarmLead>,
    chance: &mut (impl Chance + ?Sized),
) -> (DockFlight, DockOutcome) {
    let Some(docking) = npc.docking else {
        return (DockFlight::none(npc, handling, flight), DockOutcome::Flying);
    };
    // A plunderer holds no formation goal, so the sight is moot.
    let inertia = Inertia::of(npc, Some(victim), true, flight);
    let (controls, flown, wanted, outcome) = match docking.sub {
        DockSub::Stop => {
            let (controls, flown, wanted) =
                leave_flight::stop_wanting(npc, handling, inertia, true);
            (controls, flown, wanted, DockOutcome::Flying)
        }
        DockSub::Dock => {
            let (controls, wanted, outcome) = dock(npc, victim, handling, inertia, chance);
            (controls, *handling, wanted, outcome)
        }
        sub => {
            let close = sub == DockSub::Close;
            if close {
                let slot = swarm_low_level::slot_place(npc, flight);
                swarm_branch(npc, swarm, slot, handling);
            }
            let (controls, flown, wanted) = approach(npc, victim, handling, close);
            (controls, flown, wanted, DockOutcome::Flying)
        }
    };
    // The refuel's stop is sub 1, which leaves +0x34 alone: its brake
    // bursts an owed burst out.
    let word = if docking.sub == DockSub::Stop {
        Word::stop(controls, &flown, inertia)
    } else {
        Word::Writes
    };
    let flight = DockFlight {
        controls,
        handling: *handling,
        flown,
        wanted,
        inertia,
        word,
    };
    (flight, outcome)
}

/// `_HandleShip`'s turn and thrust of `npc` by the `flown` its low level
/// left ([`low_level`]), under `flight`: while its AI wait runs it only
/// moves on at its velocity (inertialess, by its speed along its heading),
/// as `_HandleShip` skips its turn and thrust (@0x3443f-0x34454,
/// @0x34a54-0x34a72); otherwise it turns to the heading wanted
/// ([`flight::turn_to`]) and thrusts as the low level said.
pub fn handle_ship(npc: &mut Npc, flown: &DockFlight, flight: FlightRules) {
    if npc.ai_wait > 0 {
        match flown.inertia {
            Inertia::Inertial => npc.state.position = npc.state.position + npc.state.velocity,
            Inertia::Inertialess { .. } => inertialess::drift(npc, &flown.flown),
        }
    } else {
        let turn = flight::turn_to(&mut npc.state, flown.wanted, flown.handling.turn_rate);
        let controls = Controls {
            turn,
            ..flown.controls
        };
        let (handling, inertia, word) = (flown.flown, flown.inertia, flown.word);
        held_burst::step(npc, &handling, controls, flight, inertia, word);
    }
}

/// The swarm branch of subs 5, 7, 0xb and 0x11 (@0x85c87-0x85cbf,
/// @0x867e0-0x86818, @0x87364-0x873a3, @0x8660b-0x8663d): with a swarm
/// leader (+0xc906 above 0, tested signed, `testw; jle`,
/// [`Npc::swarm_leader`]), `npc` is nudged toward its formation slot at
/// `slot`, when it has one, by its thrust on `handling`
/// (`_AIMaintainFormation(ship, 0)`, [`formation_flight::maintain`]), and
/// takes the glow of its swarm leader, `swarm`'s, when that ship is there
/// (the original reads whatever ship holds the slot). Nothing with a word
/// of 0 or none. Sub 0xb runs it before it steers (see
/// [`swarm_low_level`](crate::traffic::swarm_low_level) for the others).
pub(crate) fn swarm_branch(
    npc: &mut Npc,
    swarm: Option<SwarmLead>,
    slot: Option<Vec2>,
    handling: &Handling,
) {
    if npc.swarm_leader.is_none_or(|word| word == 0) {
        return;
    }
    if let Some(slot) = slot {
        formation_flight::maintain(npc, slot, handling, false);
    }
    if let Some(lead) = swarm {
        npc.display.drive.glow = GlowCopy::Swarm(lead.glow);
    }
}

/// Subs 9 and 0xb (see the module docs) of `npc` steering at a ship at
/// `victim`: the controls but the turn, the handling they fly by,
/// closing (sub 0xb) capped within [`CLOSE_CAP_BOX`], and the heading it
/// wants (+0x68), which `_HandleShip` turns it to
/// ([`flight::turn_to`]).
pub(crate) fn approach(
    npc: &Npc,
    victim: &ShipState,
    handling: &Handling,
    close: bool,
) -> (Controls, Handling, i16) {
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
        ..Controls::default()
    };
    (controls, flown, wanted)
}

/// The heading against the gap between the velocity of a ship at `state`
/// and `velocity`, the one it matches (the table angle of the gap + 180),
/// the gap at most [`DAMP_BELOW`] on either axis cut to [`DAMPING`] of
/// itself in doubles and `state`'s velocity set to `velocity` plus it:
/// matching's turn and damping, sub 0xf's (@0x86b3d-0x86cb2) and sub
/// 0xc's (@0x879b9-0x87b45).
pub(crate) fn counter(state: &mut ShipState, velocity: Vec2) -> i16 {
    let gap = state.velocity - velocity;
    let against = trig::intercept_angle(Vec2::ZERO, gap * 100.0);
    // In 0..360.
    #[allow(clippy::cast_possible_truncation)]
    let wanted = (i32::from(against) + 180).rem_euclid(360) as i16;
    if gap.x.abs() <= DAMP_BELOW || gap.y.abs() <= DAMP_BELOW {
        // Doubles, back to floats (@0x86c52-0x86c7c, @0x87aed-0x87b14).
        #[allow(clippy::cast_possible_truncation)]
        let damped = Vec2::new(
            (f64::from(gap.x) * DAMPING) as f32,
            (f64::from(gap.y) * DAMPING) as f32,
        );
        state.velocity = velocity + damped;
    }
    wanted
}

/// Whether a velocity `gap` off the one matched is [`MATCH_SLACK`] or
/// more on either axis, each axis's `_FloatAbs` widened to a double
/// (@0x86a30-0x86aa4, @0x8796f-0x879b3).
#[must_use]
pub(crate) fn mismatched(gap: Vec2) -> bool {
    let off = |axis: f32| f64::from(axis.abs()) >= MATCH_SLACK;
    off(gap.x) || off(gap.y)
}

/// Sub 0xf (see the module docs) of a docking with the ship at `victim`,
/// flown as `inertia` says, rolling a dock's wait on `chance`: the
/// controls but the turn, the heading it wants (+0x68, which `_HandleShip`
/// turns it to, [`flight::turn_to`]), and what became of the docking, its
/// velocity and speed set as the substate sets them. Its completion and
/// its start are one pass's two branches: the completion needs a wait in
/// (0, [`DOCK_WAIT`]] (@0x86d54-0x86d6d), the start none (@0x86f98-0x86f9e).
fn dock(
    npc: &mut Npc,
    victim: &ShipState,
    handling: &Handling,
    inertia: Inertia,
    chance: &mut (impl Chance + ?Sized),
) -> (Controls, i16, DockOutcome) {
    let mismatched = mismatched(npc.state.velocity - victim.velocity);
    if mismatched && matches!(inertia, Inertia::Inertialess { .. }) {
        // A tick's acceleration off its speed, down to none and not capped
        // at its top speed, in doubles (@0x86aee-0x86b33); no thrust, cap
        // or heading (@0x86ac8-0x86acd).
        #[allow(clippy::cast_possible_truncation)]
        let speed = (f64::from(npc.speed) - f64::from(handling.accel)).max(0.0) as f32;
        npc.speed = speed;
        // The low level's own whole heading (@0x8522d).
        let wanted = trig::whole(npc.state.heading);
        return (Controls::default(), wanted, DockOutcome::Flying);
    }
    if mismatched {
        let wanted = counter(&mut npc.state, victim.velocity);
        let controls = Controls {
            thrust: off_by(npc.state.heading, wanted) < handling.turn_rate + THRUST_SLACK,
            ..Controls::default()
        };
        return (controls, wanted, DockOutcome::Flying);
    }
    let wanted = trig::whole(victim.heading);
    let controls = Controls::default();
    npc.state.velocity = victim.velocity;
    let off = npc.state.position - victim.position;
    if off.x.abs() > DOCKED_BOX || off.y.abs() > DOCKED_BOX {
        // The creep zeroes +0x48 for any ship (@0x8702d).
        npc.speed = 0.0;
        let towards = trig::intercept_angle(npc.state.position, victim.position);
        npc.state.position = npc.state.position + flight::facing(f32::from(towards)) * CREEP;
        return (controls, wanted, DockOutcome::Flying);
    }
    let outcome = match npc.ai_wait {
        wait if wait > DOCK_WAIT => DockOutcome::Flying,
        1.. if matches!(npc.goal, Goal::Plunder { .. }) => DockOutcome::Completed,
        1.. if npc.goal == Goal::Assist(Help::Repair) => DockOutcome::Repaired,
        1.. => DockOutcome::Flying,
        // Unless drifting (goal 0xe, @0x86fa7-0x86faf).
        0 if npc.goal == Goal::Drift => DockOutcome::Flying,
        0 => {
            start(npc, victim, chance);
            DockOutcome::Started
        }
    };
    (controls, wanted, outcome)
}

/// `npc` starts its dock with the ship at `victim`: its wait 100 +
/// `Rand(80)` (@0x86fcf-0x86fea), rolled on `chance`, and its velocity the
/// victim's (@0x86ff1-0x87007). The victim's boarded flag is its caller's.
fn start(npc: &mut Npc, victim: &ShipState, chance: &mut (impl Chance + ?Sized)) {
    npc.ai_wait = DOCK_WAIT + u32::from(chance.roll(DOCK_WAIT_SPREAD));
    npc.state.velocity = victim.velocity;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::NpcId;
    use crate::combat::hull::class_turn;
    use crate::inertialess::InertialDampener;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use nova_sim_base::fixture::{FAST, Scripted};

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
                InertialDampener::PlayerOnly,
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
        assert_eq!(
            high_level(&ship, Vec2::ZERO, true, InertialDampener::PlayerOnly),
            DockStep::GiveUp
        );
        ship.ai_wait = 1;
        assert_eq!(
            high_level(&ship, Vec2::ZERO, true, InertialDampener::PlayerOnly),
            DockStep::Sub(DockSub::Dock)
        );
        ship.ai_wait = 0;
        assert_eq!(
            high_level(&ship, Vec2::ZERO, false, InertialDampener::PlayerOnly),
            DockStep::Sub(DockSub::Dock)
        );
    }

    /// `ship`'s whole tick of its docking at `victim`, its swarm lead
    /// `swarm`: its low level at its slot, then `_HandleShip`'s turn and
    /// thrust, a dock's wait rolling 0. What became of the docking.
    fn fly(
        ship: &mut Npc,
        victim: &ShipState,
        handling: &Handling,
        flight: FlightRules,
        swarm: Option<SwarmLead>,
    ) -> DockOutcome {
        let chance = &mut nova_sim_base::chance::NeverFires;
        let (flown, outcome) = low_level(ship, victim, handling, flight, swarm, chance);
        handle_ship(ship, &flown, flight);
        outcome
    }

    #[test]
    fn the_low_level_starts_or_completes_a_dock_never_both_in_one_pass() {
        // The completion needs a wait in (0, 100] (@0x86d54-0x86d6d), the
        // start none (@0x86f98-0x86f9e): a start's wait of 100 is read by
        // the next pass, not this one.
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut ship = pirate(at(1.0, 1.0, 0.0, 0.0, 0.0), DockSub::Dock);
        ship.goal = Goal::Assist(Help::Repair);
        let pass = |ship: &mut Npc| {
            let chance = &mut Scripted::rolling(&[0]);
            low_level(
                ship,
                &victim,
                &HANDLING,
                FlightRules::default(),
                None,
                chance,
            )
            .1
        };
        assert_eq!(pass(&mut ship), DockOutcome::Started);
        assert_eq!(ship.ai_wait, DOCK_WAIT, "100 + Rand(80) of 0");
        assert_eq!(pass(&mut ship), DockOutcome::Repaired, "the next pass");
        // A plunder's completion, likewise.
        let mut ship = pirate(at(1.0, 1.0, 0.0, 0.0, 0.0), DockSub::Dock);
        assert_eq!(pass(&mut ship), DockOutcome::Started);
        assert_eq!(pass(&mut ship), DockOutcome::Completed);
    }

    #[test]
    fn handle_ship_flies_the_low_levels_heading_and_thrust_without_matching_again() {
        // `_HandleShip` turns to +0x68 and thrusts by +0x30 (@0x34460,
        // @0x34a96); the match's damping is the low level's alone
        // (@0x86c08-0x86cb2), which double time's second pass doesn't run.
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut ship = pirate(at(50.0, 0.0, 0.0, 3.0, 0.0), DockSub::Dock);
        let chance = &mut nova_sim_base::chance::NeverFires;
        let flight = FlightRules::default();
        let (flown, _) = low_level(&mut ship, &victim, &HANDLING, flight, None, chance);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, 2.85), "damped");
        assert_eq!(ship.state.position, Vec2::new(50.0, 0.0), "not moved");
        handle_ship(&mut ship, &flown, flight);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, 2.35), "thrust");
        handle_ship(&mut ship, &flown, flight);
        assert_eq!(
            ship.state.velocity,
            Vec2::new(0.0, 2.35 - 0.5),
            "thrust again, no damping"
        );
    }

    /// `ship` flown one tick at `victim`, and what became of it.
    fn flown(mut ship: Npc, victim: &ShipState) -> (Npc, DockOutcome) {
        let outcome = fly(&mut ship, victim, &HANDLING, FlightRules::default(), None);
        (ship, outcome)
    }

    #[test]
    fn approaching_far_off_it_turns_to_the_bearing_and_thrusts_within_a_turn_and_one() {
        let victim = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        // Bearing 0; heading 3.9 is off by 3 whole degrees, 4.0 by 4.
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.0, 0.0, 3.9), DockSub::Approach),
            &victim,
        );
        assert_eq!(ship.state.heading, 0.0, "3 whole degrees off: onto it");
        assert!(ship.state.velocity.y < 0.0, "thrust");
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.0, 0.0, 4.0), DockSub::Approach),
            &victim,
        );
        assert_eq!(ship.state.velocity, Vec2::ZERO, "4 is not below 4");
        assert_eq!(ship.state.heading, 1.0, "4 off: it turns by 3");
    }

    #[test]
    fn approaching_and_closing_it_turns_as_handle_ship_does_onto_a_heading_within_its_turn() {
        // Bearing 0, 1.4 off it: within half a turn, yet `_HandleShip`
        // puts it on the bearing.
        let victim = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        for sub in [DockSub::Approach, DockSub::Close] {
            let (ship, _) = flown(pirate(at(0.0, 0.0, 0.0, 0.0, 1.4), sub), &victim);
            assert_eq!(ship.state.heading, 0.0, "{sub:?}");
        }
        // Near it, keeping its heading, it keeps its whole heading.
        let near = at(0.0, -150.0, 0.0, 0.0, 0.0);
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.0, -2.0, 90.6), DockSub::Approach),
            &near,
        );
        assert_eq!(ship.state.heading, 90.0);
    }

    #[test]
    fn approaching_near_it_keeps_its_heading_while_its_velocity_points_at_the_victim() {
        let victim = at(0.0, -150.0, 0.0, 0.0, 0.0);
        // Moving straight at it, facing across: it wants its own heading,
        // so it doesn't turn and thrusts along it.
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.0, -2.0, 90.0), DockSub::Approach),
            &victim,
        );
        assert_eq!(ship.state.heading, 90.0);
        assert_eq!(ship.state.velocity, Vec2::new(0.5, -2.0));
        // Moving across it: it turns to make up its velocity to the top
        // speed along the bearing, (0, -4) - (2, 0), up and to the left.
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 2.0, 0.0, 90.0), DockSub::Approach),
            &victim,
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
        let (ship, _) = flown(pirate(moving, DockSub::Approach), &victim);
        assert_ne!(ship.state.heading, 180.0, "it corrects");
        // Just right of the bearing too: it keeps its heading.
        let near = at(0.0, 0.0, 0.09, -1.0, 180.0);
        let direction = trig::intercept_angle(Vec2::ZERO, near.velocity * 100.0);
        assert!((i32::from(direction) - i32::from(bearing)).abs() <= VELOCITY_SLACK);
        let (ship, _) = flown(pirate(near, DockSub::Approach), &victim);
        assert_eq!(ship.state.heading, 180.0);
    }

    #[test]
    fn closing_with_a_stale_parent_and_a_swarm_leader_it_is_nudged_to_the_place_it_holds() {
        // A plunderer's AI type is 4 or below, so the pass left +0xc92e
        // as it was (@0x84f30 passes on a parent of 63 or below).
        let victim = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        let close = || {
            let mut ship = pirate(at(50.0, -50.0, 0.0, 0.0, 90.0), DockSub::Close);
            ship.swarm_leader = Some(4);
            ship.form_parent = Some(4);
            ship.slot_place = Some(Vec2::new(500.0, 500.0));
            ship
        };
        let flown = |mut ship: Npc, flight: FlightRules| {
            fly(&mut ship, &victim, &HANDLING, flight, None);
            ship.state.position
        };
        let by = HANDLING.accel * crate::escort::FORMATION_NUDGE;
        assert_eq!(
            flown(close(), FlightRules::default()),
            Vec2::new(50.0 + by, -50.0 + by)
        );
        // No parent, no nudge, whatever place it holds.
        let parentless = Npc {
            form_parent: None,
            ..close()
        };
        assert_eq!(
            flown(parentless, FlightRules::default()),
            Vec2::new(50.0, -50.0)
        );
        // Nor on an NPC parent's slot, by the other reading.
        let earlier = FlightRules {
            npc_formation: crate::flight::NpcFormation::Earlier,
            ..FlightRules::default()
        };
        assert_eq!(flown(close(), earlier), Vec2::new(50.0, -50.0));
    }

    #[test]
    fn closing_with_a_swarm_leader_it_takes_its_glow_and_has_no_slot_to_be_nudged_to() {
        let victim = at(0.0, -1000.0, 0.0, 0.0, 0.0);
        let lead = SwarmLead {
            aim: Vec2::ZERO,
            leads_it: false,
            glow: 9,
        };
        let glow_of = |sub: DockSub, swarm_leader: Option<u32>| {
            let mut ship = pirate(at(0.0, 0.0, 0.0, 0.0, 90.0), sub);
            ship.swarm_leader = swarm_leader;
            fly(
                &mut ship,
                &victim,
                &HANDLING,
                FlightRules::default(),
                Some(lead),
            );
            assert_eq!(ship.state.position, Vec2::ZERO, "no nudge, no thrust");
            ship.display.drive.glow
        };
        assert_eq!(glow_of(DockSub::Close, Some(4)), GlowCopy::Swarm(9));
        assert_eq!(glow_of(DockSub::Approach, Some(4)), GlowCopy::None);
        assert_eq!(glow_of(DockSub::Close, None), GlowCopy::None);
    }

    #[test]
    fn closing_caps_its_thrust_at_half_its_top_speed_within_100_and_not_outside() {
        // Already at 2, half its top speed: under the cap `_AdjustedAccel`
        // adds no push to the axis (the per-axis reading of
        // `RuleKey::SpeedCap`, which clamps nothing, so a push from below
        // the cap still carries past it).
        let start = at(0.0, 0.0, 0.0, -2.0, 0.0);
        let near = at(0.0, -100.0, 0.0, 0.0, 0.0);
        let (ship, _) = flown(pirate(start, DockSub::Close), &near);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, -2.0), "capped at 2");
        let (ship, _) = flown(pirate(start, DockSub::Approach), &near);
        assert_eq!(
            ship.state.velocity.y, -2.5,
            "sub 9 has no cap but its top speed"
        );
        let far = at(0.0, -100.5, 0.0, 0.0, 0.0);
        let (ship, _) = flown(pirate(start, DockSub::Close), &far);
        assert_eq!(ship.state.velocity.y, -2.5);
    }

    #[test]
    fn docking_it_turns_against_its_velocity_off_the_victims_and_thrusts_within_a_turn_and_one() {
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 3 apart going down: it wants to face up (0), facing it.
        let (ship, _) = flown(pirate(at(50.0, 0.0, 0.0, 3.0, 0.0), DockSub::Dock), &victim);
        assert_eq!(ship.state.heading, 0.0);
        assert_eq!(
            ship.state.velocity,
            Vec2::new(0.0, 2.35),
            "damped by x's 0 to 2.85, then thrust"
        );
        // 2 apart on y, 0 on x: damped to 0.95 of it before thrust.
        let (ship, _) = flown(pirate(at(50.0, 0.0, 0.0, 2.0, 0.0), DockSub::Dock), &victim);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, 1.9 - 0.5));
        // Both axes above 1.75: no damping.
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 2.0, 2.0, 130.0), DockSub::Dock),
            &victim,
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
        );
        assert_eq!(ship.state.velocity, victim.velocity, "aligned below 0.525");
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 1.0, 0.53, 0.0), DockSub::Dock),
            &victim,
        );
        assert_ne!(ship.state.velocity, victim.velocity, "matching");
    }

    #[test]
    fn aligned_it_takes_the_victims_velocity_and_heading_and_creeps_a_pixel_outside_3() {
        let victim = at(0.0, 0.0, 0.5, 0.0, 12.7);
        let (ship, outcome) = flown(
            pirate(at(0.0, 10.0, 0.5, 0.2, 10.0), DockSub::Dock),
            &victim,
        );
        assert_eq!(outcome, DockOutcome::Flying);
        assert_eq!(ship.state.heading, 12.0, "2 off: onto 12");
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
        // @0x86fa4-0x8700a, at its slot of the AI loop.
        let victim = at(0.0, 0.0, 0.5, 0.0, 0.0);
        for (roll, wait) in [(0, 100), (79, 179)] {
            let mut ship = pirate(at(3.0, -3.0, 0.5, 0.0, 0.0), DockSub::Dock);
            let mut chance = Scripted::rolling(&[roll]);
            let flight = FlightRules::default();
            let (flown, outcome) =
                low_level(&mut ship, &victim, &HANDLING, flight, None, &mut chance);
            assert_eq!(outcome, DockOutcome::Started, "inside 3, matched, no wait");
            assert_eq!(chance.sides_asked, [DOCK_WAIT_SPREAD]);
            assert_eq!(ship.ai_wait, wait);
            assert_eq!(ship.state.velocity, victim.velocity, "@0x86ff1-0x87007");
            // `_HandleShip` then moves it on, its turn and thrust skipped.
            handle_ship(&mut ship, &flown, flight);
            assert_eq!(ship.state.position, Vec2::new(3.5, -3.0), "{roll}");
        }
    }

    #[test]
    fn a_dock_starts_only_at_sub_0xf_matched_inside_3_with_no_wait_and_not_drifting() {
        // Whether it flies intact is its caller's (`plunder::dock_low_level`).
        let victim = at(0.0, 0.0, 0.5, 0.0, 0.0);
        let starting = |change: &dyn Fn(&mut Npc)| {
            let mut ship = pirate(at(3.0, -3.0, 0.5, 0.0, 0.0), DockSub::Dock);
            change(&mut ship);
            flown(ship, &victim).1 == DockOutcome::Started
        };
        assert!(starting(&|_| {}));
        assert!(!starting(&|ship| ship.docking = None), "no docking");
        let close = |ship: &mut Npc| {
            ship.docking = Some(Docking {
                ship: VICTIM,
                sub: DockSub::Close,
            });
        };
        assert!(!starting(&close), "sub 0xb");
        assert!(!starting(&|ship| ship.state.position.x = 3.5), "outside 3");
        assert!(!starting(&|ship| ship.state.position.y = -3.5), "outside 3");
        assert!(!starting(&|ship| ship.state.velocity.x = 2.0), "mismatched");
        assert!(!starting(&|ship| ship.state.velocity.y = 1.0), "mismatched");
        assert!(!starting(&|ship| ship.ai_wait = 1), "its wait");
        assert!(!starting(&|ship| ship.goal = Goal::Drift), "goal 0xe");
    }

    #[test]
    fn docked_it_holds_above_100_completes_only_under_a_plunder_and_never_starts_drifting() {
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let docked = |goal: Goal, wait: u32| {
            let mut ship = pirate(at(1.0, 1.0, 0.0, 0.0, 0.0), DockSub::Dock);
            ship.goal = goal;
            ship.ai_wait = wait;
            flown(ship, &victim).1
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
        assert_eq!(
            docked(Goal::Assist(Help::Refuel), 1),
            DockOutcome::Flying,
            "goal 9 completes in its high level"
        );
        assert_eq!(
            docked(Goal::Attack(VICTIM), 0),
            DockOutcome::Started,
            "at none it starts, whatever its goal but 0xe"
        );
    }

    #[test]
    fn docked_under_a_repair_it_repairs_at_100_or_below_holds_above_and_starts_at_none() {
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let docked = |wait: u32| {
            let mut ship = pirate(at(1.0, 1.0, 0.0, 0.0, 0.0), DockSub::Dock);
            ship.goal = Goal::Assist(Help::Repair);
            ship.ai_wait = wait;
            flown(ship, &victim).1
        };
        assert_eq!(docked(101), DockOutcome::Flying);
        assert_eq!(docked(100), DockOutcome::Repaired);
        assert_eq!(docked(1), DockOutcome::Repaired);
        assert_eq!(docked(0), DockOutcome::Started, "starts at none");
    }

    #[test]
    fn the_high_level_measures_the_box_from_the_victim_where_it_is() {
        let victim = Vec2::new(1000.0, -1000.0);
        let step = |x: f32, y: f32| {
            high_level(
                &pirate(at(x, y, 0.0, 0.0, 0.0), DockSub::Approach),
                victim,
                false,
                InertialDampener::PlayerOnly,
            )
        };
        assert_eq!(step(1210.0, -790.0), DockStep::Sub(DockSub::Dock));
        assert_eq!(step(1420.5, -1000.0), DockStep::Sub(DockSub::Approach));
    }

    #[test]
    fn its_turn_and_thrust_wait_while_its_ai_wait_runs() {
        // Matching across 3: it would turn and thrust, but only moves on at
        // the damped velocity.
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let mut ship = pirate(at(50.0, 0.0, 0.0, 2.0, 90.0), DockSub::Dock);
        ship.ai_wait = 150;
        let (ship, _) = flown(ship, &victim);
        assert_eq!(ship.state.heading, 90.0);
        assert_eq!(ship.state.velocity, Vec2::new(0.0, 1.9));
        assert_eq!(ship.state.position, Vec2::new(50.0, 1.9));
        // Docked and holding, a fraction off the victim's heading: not
        // put on it.
        let mut ship = pirate(at(1.0, 1.0, 0.0, 0.0, 0.5), DockSub::Dock);
        ship.ai_wait = 150;
        let (ship, _) = flown(ship, &victim);
        assert_eq!(ship.state.heading, 0.5);
        // Stopping (sub 1) within its turn of its retro heading, 0: put on
        // it with no wait, and left where it was while the wait runs.
        let stopping = || pirate(at(500.0, 0.0, 0.0, 2.0, 1.5), DockSub::Stop);
        let (ship, _) = flown(stopping(), &victim);
        assert_eq!(ship.state.heading, 0.0);
        let mut waiting = stopping();
        waiting.ai_wait = 150;
        let (ship, _) = flown(waiting, &victim);
        assert_eq!(ship.state.heading, 1.5);
    }

    #[test]
    fn approaching_reads_the_bearing_and_the_box_from_the_victim_where_it_is() {
        let victim = at(1000.0, -1000.0, 0.0, 0.0, 0.0);
        // 150 below it, moving across: inside the box it corrects to 333,
        // left of it, where the bearing, 0, would be to its right.
        let (ship, _) = flown(
            pirate(at(1000.0, -850.0, 2.0, 0.0, 340.0), DockSub::Approach),
            &victim,
        );
        assert_eq!(ship.state.heading, 337.0, "turning towards 333");
        // 250 below it, outside: it turns to the bearing, 0.
        let (ship, _) = flown(
            pirate(at(1000.0, -750.0, 2.0, 0.0, 10.0), DockSub::Approach),
            &victim,
        );
        assert_eq!(ship.state.heading, 7.0);
    }

    #[test]
    fn approaching_it_corrects_by_the_gap_to_its_top_speed_on_one_axis_alone() {
        // The bearing 90; its velocity's direction far off it, short of the
        // top speed (4, 0) by (3.9, 0.2): it turns to the gap, 92, within
        // its turn.
        let victim = at(150.0, 0.0, 0.0, 0.0, 0.0);
        let (ship, _) = flown(
            pirate(at(0.0, 0.0, 0.1, -0.2, 90.0), DockSub::Approach),
            &victim,
        );
        assert_eq!(ship.state.heading, 92.0, "onto the gap");
    }

    #[test]
    fn approaching_15_degrees_off_the_bearing_is_not_off_enough() {
        let victim = at(0.0, -150.0, 0.0, 0.0, 0.0);
        // A velocity whose table angle is `angle`, off the bearing of 0.
        let moving = |angle: i16| {
            (0_i16..400)
                .map(|tenth| flight::facing(f32::from(tenth) * 0.1) * 0.5)
                .find(|velocity| trig::intercept_angle(Vec2::ZERO, *velocity * 100.0) == angle)
                .expect("some heading reads so")
        };
        let heading_after = |velocity: Vec2| {
            let (ship, _) = flown(
                pirate(
                    at(0.0, 0.0, velocity.x, velocity.y, 180.0),
                    DockSub::Approach,
                ),
                &victim,
            );
            ship.state.heading
        };
        assert_eq!(heading_after(moving(15)), 180.0, "15 is not above 15");
        assert_ne!(heading_after(moving(16)), 180.0, "16 is");
        // To the right of it, the bearing 90: 95 is 5 off, the difference
        // of the two, not their sum.
        let right = at(150.0, 0.0, 0.0, 0.0, 0.0);
        let velocity = (0_i16..1800)
            .map(|tenth| flight::facing(f32::from(tenth) * 0.1) * 0.5)
            .find(|velocity| trig::intercept_angle(Vec2::ZERO, *velocity * 100.0) == 95)
            .expect("some heading reads so");
        let (ship, _) = flown(
            pirate(
                at(0.0, 0.0, velocity.x, velocity.y, 180.0),
                DockSub::Approach,
            ),
            &right,
        );
        assert_eq!(ship.state.heading, 180.0);
    }

    #[test]
    fn matching_turns_against_the_velocity_gap_on_either_axis() {
        // 3 apart going right: it wants to face left, 270.
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 3.0, 0.0, 260.0), DockSub::Dock),
            &victim,
        );
        assert_eq!(ship.state.heading, 263.0);
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 3.0, 0.0, 280.0), DockSub::Dock),
            &victim,
        );
        assert_eq!(ship.state.heading, 277.0);
        // 1.4 off it: `_HandleShip` puts it on 270.
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 3.0, 0.0, 271.4), DockSub::Dock),
            &victim,
        );
        assert_eq!(ship.state.heading, 270.0);
    }

    #[test]
    fn matching_damps_both_axes_of_the_gap() {
        let victim = at(0.0, 0.0, 1.0, 1.0, 0.0);
        // A gap of (0.6, 2.0), facing far off its wanted heading.
        let (ship, _) = flown(
            pirate(at(50.0, 0.0, 1.6, 3.0, 90.0), DockSub::Dock),
            &victim,
        );
        let expected = Vec2::new(1.0 + 0.57, 1.0 + 1.9);
        assert!(
            (ship.state.velocity - expected).length() < 1e-5,
            "{:?}",
            ship.state.velocity
        );
    }

    #[test]
    fn matching_thrusts_strictly_within_a_turn_and_one() {
        // It wants 0 (a gap going down); turn 3, so within 4.
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let thrusts = |heading: f32| {
            let (ship, _) = flown(
                pirate(at(50.0, 0.0, 0.0, 2.0, heading), DockSub::Dock),
                &victim,
            );
            ship.state.velocity.y < 1.89
        };
        assert!(thrusts(3.0), "3 off");
        assert!(!thrusts(4.0), "4 off");
    }

    #[test]
    fn aligned_it_reads_the_docked_box_from_the_victim_where_it_is() {
        let victim = at(1000.0, -1000.0, 0.5, 0.0, 0.0);
        let inside = pirate(at(1003.0, -997.0, 0.5, 0.0, 0.0), DockSub::Dock);
        let (ship, outcome) = flown(inside, &victim);
        assert_eq!(outcome, DockOutcome::Started, "inside 3");
        assert_eq!(ship.state.position, Vec2::new(1003.5, -997.0));
        let outside = pirate(at(1003.5, -1000.0, 0.5, 0.0, 0.0), DockSub::Dock);
        assert_eq!(flown(outside, &victim).1, DockOutcome::Flying, "outside 3");
    }

    /// `ship` on an inertialess hull ([`FAST`]'s, `Flags2` 0x0040).
    fn inertialess(mut ship: Npc) -> Npc {
        ship.stats = ShipStats::new(
            crate::handling::ShipFields {
                flags2: crate::stats::INERTIALESS_HULL,
                ..FAST
            },
            &[],
        );
        ship
    }

    #[test]
    fn an_inertialess_pirate_matching_sheds_a_ticks_acceleration_off_its_speed_with_no_turn_thrust_or_damping()
     {
        use crate::inertialess::{ADJUST_RATE, adjust_velocity};
        use crate::traffic::ship_display::Thrust;
        let victim = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 2 apart going down, facing across: an inertial one would damp
        // the gap and turn to 0.
        let start = at(50.0, 0.0, 0.0, 2.0, 90.6);
        let matched = |speed: f32| {
            let mut ship = inertialess(pirate(start, DockSub::Dock));
            ship.speed = speed;
            let (ship, outcome) = flown(ship, &victim);
            assert_eq!(outcome, DockOutcome::Flying);
            assert_eq!(
                ship.state.heading, 90.0,
                "no turn but onto its whole heading"
            );
            assert_eq!(ship.display.drive.thrust, Thrust::None);
            let velocity = adjust_velocity(
                start.velocity,
                90.0,
                ship.speed,
                HANDLING.accel * ADJUST_RATE,
            );
            assert_eq!(ship.state.velocity, velocity, "no damping");
            ship.speed
        };
        assert_eq!(matched(1.0), 0.5);
        assert_eq!(matched(0.1), 0.0, "floored");
        assert_eq!(matched(7.0), 6.5, "not capped at its top speed of 4");
        assert_eq!(matched(0.0), 0.0);
    }

    #[test]
    fn aligned_outside_3_its_speed_is_zeroed_as_it_creeps() {
        let victim = at(0.0, 0.0, 0.5, 0.0, 12.7);
        for ship in [
            pirate(at(0.0, 10.0, 0.5, 0.2, 10.0), DockSub::Dock),
            inertialess(pirate(at(0.0, 10.0, 0.5, 0.2, 10.0), DockSub::Dock)),
        ] {
            let (crept, _) = flown(
                Npc {
                    speed: 2.0,
                    ..ship.clone()
                },
                &victim,
            );
            assert_eq!(crept.speed, 0.0, "@0x8702d");
            // Inside 3 the dock starts and leaves its speed alone
            // (@0x86fa4-0x8700a).
            let inside = Npc {
                speed: 2.0,
                state: at(3.0, -3.0, 0.5, 0.0, 0.0),
                ..ship
            };
            let (docked, outcome) = flown(inside, &victim);
            assert_eq!(outcome, DockOutcome::Started);
            assert_eq!(docked.speed, 2.0);
        }
    }

    #[test]
    fn an_inertialess_pirate_docks_within_four_times_the_box_and_closes_within_eight() {
        let step = |ship: &Npc, x: f32, y: f32| {
            let ship = Npc {
                state: at(x, y, 0.0, 0.0, 0.0),
                ..ship.clone()
            };
            high_level(&ship, Vec2::ZERO, false, InertialDampener::PlayerOnly)
        };
        let raven = inertialess(pirate(ShipState::default(), DockSub::Approach));
        assert_eq!(step(&raven, 840.0, -840.0), DockStep::Sub(DockSub::Dock));
        assert_eq!(step(&raven, 841.0, 0.0), DockStep::Sub(DockSub::Close));
        assert_eq!(step(&raven, 1680.0, 0.0), DockStep::Sub(DockSub::Close));
        assert_eq!(step(&raven, 1681.0, 0.0), DockStep::Sub(DockSub::Approach));
        let plain = pirate(ShipState::default(), DockSub::Approach);
        assert_eq!(step(&plain, 210.0, 0.0), DockStep::Sub(DockSub::Dock));
        assert_eq!(step(&plain, 211.0, 0.0), DockStep::Sub(DockSub::Close));
        assert_eq!(step(&plain, 421.0, 0.0), DockStep::Sub(DockSub::Approach));
        // A dampener counts by the Bible's reading.
        let mut dampened = plain.clone();
        dampened.stats.dampener = true;
        assert_eq!(
            step(&dampened, 840.0, 0.0),
            DockStep::Sub(DockSub::Approach)
        );
        let ship = Npc {
            state: at(840.0, 0.0, 0.0, 0.0, 0.0),
            ..dampened
        };
        assert_eq!(
            high_level(&ship, Vec2::ZERO, false, InertialDampener::AnyShip),
            DockStep::Sub(DockSub::Dock)
        );
    }

    #[test]
    fn with_no_docking_its_low_level_sets_nothing() {
        let mut ship = pirate(at(0.0, 0.0, 1.0, 0.0, 0.5), DockSub::Dock);
        ship.docking = None;
        let before = ship.clone();
        let flight = FlightRules::default();
        let (flown, outcome) = low_level(
            &mut ship,
            &at(0.0, 0.0, 0.0, 0.0, 0.0),
            &HANDLING,
            flight,
            None,
            &mut nova_sim_base::chance::NeverFires,
        );
        assert_eq!((&ship, outcome), (&before, DockOutcome::Flying));
        assert_eq!(flown, DockFlight::none(&ship, &HANDLING, flight));
        // `_HandleShip` keeps its whole heading and doesn't thrust.
        handle_ship(&mut ship, &flown, flight);
        assert_eq!(ship.state.heading, 0.0);
        assert_eq!(ship.state.velocity, Vec2::new(1.0, 0.0));
    }
}
