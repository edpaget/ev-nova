//! The autopilot's low level: at the ship's slot of `_AIDispatch`'s loop,
//! the work `_LowLevelAIHandler` does for the goal the engine's autopilot
//! flies (and the high level's goal block it flies by), which leaves the
//! word `_HandleShip` flies at the ship's step ([`SlotWord`], see
//! [`slot_word`](crate::traffic::slot_word)). It turns no ship and moves
//! none: each flight hands back the heading it wants (+0x68) and its
//! thrust behind its window, read on the handling the ship has at its
//! slot, and makes the low level's direct writes there (a sniper's
//! damping, the formation's nudge and glow copy, a miner's ease, nudge and
//! brake, a docking fighter's pull, a landing's drag and stay).

use super::{ALIGNED, APPROACH_THRUST, Beside, DOGFIGHT_THRUST, FLEE_THRUST, Outcome};
use super::{FOLLOW_DISTANCE, Readings, SWARM_THRUST, TopSpeeds, hand_off, within_reach};
use crate::ai::lost_sight;
use crate::ai::pursuit::{self, BREAK_THRUST, Body, RUSH_SPEED, RUSH_THRUST};
use crate::ai::{Goal, fire, fire::Manoeuvre};
use crate::bay::DOCK_TURN_SLACK;
use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::combat::armament::Arsenal;
use crate::escort::{APPROACH_FORMATION, FORMATION_NUDGE, FORMATION_SLACK, KEEP_FORMATION};
use crate::flight::{
    self, AT_REST_SPEED, Controls, FlightRules, FormationFlight, NpcFormation, ShipState,
    heading_of, shortest_turn,
};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::landing::{self, LANDING_SPEED, TypeWidth};
use crate::rulebook::RuleSource;
use crate::traffic::formation_flight;
use crate::traffic::gate_entry;
use crate::traffic::held_burst::{Flight, Word};
use crate::traffic::inertialess::{self, Inertia};
use crate::traffic::land_flight;
use crate::traffic::leave_flight;
use crate::traffic::mine_flight;
use crate::traffic::npc::Npc;
use crate::traffic::pod_flight;
use crate::traffic::ship_display::{self, GlowCopy};
use crate::traffic::slot_word::{Cap, SlotWord};
use crate::traffic::snipe;
use crate::traffic::stellar_attack;
use crate::trig;
use std::collections::BTreeMap;

/// `npc`'s low level at its slot of the AI loop, among `sites`, the gates
/// among them as wide as `type_widths` says, the ship its goal is about at
/// `other` when it is there, disabled as `other_disabled` says, with
/// `beside` what it reads beside that ship ([`Beside`]), at its top speed
/// `speeds.own`, under the flight rules, by the disputed rules' readings,
/// leading by the weapon records (`flight`, `readings`, `weapons`), rolling
/// a landing's wait on `chance`: the word it leaves `_HandleShip`, and how
/// its flight ended at the slot, if it did (a landing, a jump begun, a
/// gate's box reached by a ship the high level's gate block didn't
/// reach), in which case its step flies nothing that pass. Everything it
/// reads of `npc`'s handling is read now ([`Npc::flying_handling`]): its
/// charge as it stands at the slot.
///
/// A ship whose flight is `_HandleShip`'s own (a gate's waits, its glide
/// and burst, a disabled ship's drift, see [`step()`](super::step())), or
/// whose AI wait runs (but a count wait), leaves the head's word: its own
/// whole heading and no thrust ([`SlotWord::none`]).
pub fn low_level(
    npc: &mut Npc,
    sites: &[LandingSite],
    type_widths: &BTreeMap<StellarId, TypeWidth>,
    (other, other_disabled, beside): (Option<&ShipState>, bool, Beside),
    speeds: TopSpeeds,
    (flight, readings, weapons): (FlightRules, Readings, &Arsenal),
    chance: &mut (impl Chance + ?Sized),
) -> (SlotWord, Option<Outcome>) {
    let handling = npc.flying_handling(speeds.own, flight);
    let inertia = Inertia::of(npc, other, beside.leader_seen, flight);
    let quarry = (other, other_disabled, beside);
    let rules = (flight, readings, weapons);
    match decide(
        npc,
        (sites, type_widths),
        quarry,
        &handling,
        inertia,
        rules,
        chance,
    ) {
        Ok(flown) => (SlotWord::of(flown, &handling, inertia), None),
        Err(outcome) => (SlotWord::none(npc, &handling, inertia), Some(outcome)),
    }
}

/// The flight [`low_level`] leaves `npc` on `handling`, as `inertia`
/// says, or how its flight ended at the slot.
fn decide(
    npc: &mut Npc,
    (sites, type_widths): (&[LandingSite], &BTreeMap<StellarId, TypeWidth>),
    (other, other_disabled, beside): (Option<&ShipState>, bool, Beside),
    handling: &Handling,
    inertia: Inertia,
    (flight, readings, weapons): (FlightRules, Readings, &Arsenal),
    chance: &mut (impl Chance + ?Sized),
) -> Result<Flight, Outcome> {
    if pod_flight::is_pod(npc) {
        return Ok(pod_flight::flight(npc));
    }
    if !super::step::steered(npc) || (npc.ai_wait > 0 && !npc.count_wait()) {
        // The head's word (@0x851f1, @0x8522d). While +0x4c runs, the high
        // level's wait gate returns before state 1 runs (@0x8d4b9-0x8d4d2),
        // so a landing ship keeps its goal through the wait and hands a
        // gate off only after it.
        return Ok(none(npc));
    }
    hand_off(npc, sites);
    Ok(match npc.goal {
        Goal::Land(stellar) => {
            let by = (inertia, readings.npc_landing);
            return land(npc, sites, stellar, handling, by, chance);
        }
        Goal::EnterGate(gate) => {
            let by = (inertia, readings.npc_landing);
            return enter_box(npc, sites, type_widths, gate, handling, by);
        }
        Goal::JumpOut => return leave_flight::leaving(npc, handling, inertia),
        Goal::AttackStellar(_) => {
            return stellar_attack::low_level(npc, sites, weapons, handling, inertia);
        }
        Goal::Inspect(_) => follow_or_brake(npc, handling, other, inertia),
        // By the earlier follow, a fleet escort follows its lead as an
        // inspector does; by the original's, it keeps formation below.
        Goal::Follow { .. } if flight.npc_formation == NpcFormation::Earlier => {
            follow_or_brake(npc, handling, other, inertia)
        }
        // Goal 0xd against a disabled ship docks with it, which the
        // traffic flies ([`plunder_dock`](crate::traffic::plunder_dock));
        // with no docking yet, its high level not having run, it brakes.
        // Searching for a target out of sight: sub 1, a stop (@0x8d814).
        Goal::Attack(_) | Goal::Snipe(_) | Goal::Plunder { .. } if lost_sight::searching(npc) => {
            braking(npc, handling, inertia)
        }
        Goal::Plunder { .. } if other_disabled => braking(npc, handling, inertia),
        Goal::Attack(_) | Goal::Snipe(_) | Goal::Flee(_) | Goal::Plunder { .. } => match other {
            Some(target) => {
                let quarry = (target, other_disabled, beside);
                return pursue(npc, quarry, handling, inertia, (readings, weapons));
            }
            // A sniper whose target is gone coasts ([`snipe::fly`]).
            None if matches!(npc.goal, Goal::Snipe(_)) => {
                let (controls, wanted) =
                    snipe::fly(npc, None, handling, readings.snipe_lead, weapons);
                Flight::writes(controls, wanted, Cap::Slot)
            }
            // A paused fight with +0x70 -1 skips sub 5's block (@0x85c70):
            // no thrust (@0x851f1), its heading truncated (@0x8522d).
            None if beside.paused => none(npc),
            None => braking(npc, handling, inertia),
        },
        Goal::Formation { .. } | Goal::Follow { .. }
            if flight.formation_flight == FormationFlight::Original =>
        {
            formation_flight::fly(npc, other, beside, handling, flight, inertia)
        }
        Goal::Formation { .. } | Goal::Follow { .. } => formation(npc, other, handling, inertia),
        Goal::Dock(_) => dock(npc, other, handling, inertia),
        // A count wait is sub 1, a stop (`_HighLevelAIHandler` @0x8d48c), as
        // is a wait (state 6, @0x8f101). The regroup pause flies its kept
        // substate's goal in its place (`Npc::fly_as_flown`, which the
        // dispatch runs), so it reaches here only flown by itself: a stop.
        // State 16 flies order 1, a stop, with slot 0 empty, else order
        // 0x13 or 0x14; state 17 flies order 0x15.
        Goal::Mine => mine_flight::mine(npc, beside.rock.as_ref(), handling, weapons)
            .unwrap_or_else(|| braking(npc, handling, inertia)),
        Goal::Scoop => mine_flight::scoop(npc, beside.scoop_box, handling),
        Goal::Idle | Goal::Wait | Goal::CountWait | Goal::Regroup { .. } => {
            braking(npc, handling, inertia)
        }
        // A drift moves on with no turn or thrust (its wait above), as
        // does an assister its docking doesn't fly: sub 0xb with no target
        // does nothing (@0x87345-0x87351). +0x68 is the low level's
        // default, so its heading is truncated (@0x8522d).
        Goal::Hold | Goal::Drift | Goal::Assist(_) => none(npc),
    })
}

/// The head's flight (see [`SlotWord::none`]): `npc`'s own whole heading
/// (@0x8522d), no thrust (@0x851f1), +0x34 its slot's top speed.
fn none(npc: &Npc) -> Flight {
    Flight::writes(
        Controls::default(),
        trig::whole(npc.state.heading),
        Cap::Slot,
    )
}

/// `npc`'s fight at `target` (`other_disabled` or not, `beside` its
/// swarm), on `handling`, as `inertia` says, by the disputed rules'
/// readings, leading by `weapons`: by its pursuit's frame ([`fight`]),
/// or, fleeing beyond its attacker's reach, state 3's far branch
/// ([`leave_flight::fleeing_afar`]).
fn pursue(
    npc: &mut Npc,
    (target, other_disabled, beside): (&ShipState, bool, Beside),
    handling: &Handling,
    inertia: Inertia,
    (readings, weapons): (Readings, &Arsenal),
) -> Result<Flight, Outcome> {
    if let Some((body, speed_cap)) =
        pursuit::frame_of(npc, target, other_disabled, beside.swarm, weapons)
            .and_then(|frame| Some((frame.last()?, frame.speed_cap)))
    {
        let by = (readings.snipe_lead, weapons);
        return Ok(fight(npc, target, (body, speed_cap), handling, inertia, by));
    }
    leave_flight::fleeing_afar(npc, handling, inertia, readings.jump_fuel)
}

/// One tick of `npc`'s fight at a target at `target` on `handling` by its
/// frame's last `body` and its `speed_cap`, whose writes to the thrust
/// (+0x30), the cap (+0x34) and the heading (+0x68) win (see
/// [`pursuit`](crate::ai::pursuit)): it wants the heading the body wants,
/// and, by its heading at the slot, thrusts approaching (7) while off it
/// by less than [`APPROACH_THRUST`] turns, dogfighting (6) by less than a
/// turn and [`DOGFIGHT_THRUST`], fleeing close or backing away (5) by less
/// than a turn and [`FLEE_THRUST`], swarming (0x12) by less than a turn
/// and [`SWARM_THRUST`], each in whole degrees, strictly
/// ([`fire::off_by`]), each turn its `_ShipTurnRate` at the slot; breaking
/// off (0x10) it thrusts every tick at [`BREAK_THRUST`] times its
/// acceleration, and rushing (0x11) at [`RUSH_THRUST`] times it up to
/// [`RUSH_SPEED`] times its top speed at the slot; sniping, it flies as
/// [`snipe::fly`] says, a moving sniper with a last gun turning as
/// `snipe_lead` says, leading by that gun's record in `weapons`, and a
/// moving inertialess sniper brakes as substate 0xe does (its word's
/// whole-acceleration brake). An inertialess dogfighter whose frame
/// matches its target's speed
/// ([`Frame::speed_cap`](pursuit::Frame::speed_cap)) flies at most at
/// that speed, a cap of none being none, as `_HandleShip` reads +0x34
/// (@0x34a8a-0x34aab); a later body's cap (0x10's none, 0x11's) wins.
fn fight(
    npc: &mut Npc,
    target: &ShipState,
    (body, speed_cap): (Body, Option<f32>),
    handling: &Handling,
    inertia: Inertia,
    (snipe_lead, weapons): (RuleSource, &Arsenal),
) -> Flight {
    if speed_cap.is_some() {
        // Sub 6 wrote +0x34, the cap, this frame, thrust or not
        // (@0x860a6): what the ship owed of a gate burst is gone.
        npc.owed_burst = None;
    }
    let window = match body.manoeuvre {
        Manoeuvre::Approach => APPROACH_THRUST * handling.turn_rate,
        Manoeuvre::Dogfight => handling.turn_rate + DOGFIGHT_THRUST,
        Manoeuvre::Flee => handling.turn_rate + FLEE_THRUST,
        Manoeuvre::Swarm => handling.turn_rate + SWARM_THRUST,
        // Breaking off and rushing thrust whatever the heading.
        Manoeuvre::BreakOff | Manoeuvre::Rush => f32::INFINITY,
        // Sub 0xe leaves +0x34 alone: its thrust bursts an owed burst out.
        Manoeuvre::Snipe => {
            if let Inertia::Inertialess { .. } = inertia
                && snipe::moving(&npc.state)
            {
                // +0x30 = -`_ShipAccelRate` (@0x8710f-0x8713a), no +0x68
                // written: the low level's default. The burst takes its
                // place, or else the brake.
                return Flight {
                    speed_off: true,
                    ..Flight::writes(
                        Controls::default(),
                        trig::whole(npc.state.heading),
                        Cap::Slot,
                    )
                    .doing(Word::Keeps {
                        thrust: handling.accel,
                    })
                };
            }
            let (controls, wanted) = snipe::fly(npc, Some(target), handling, snipe_lead, weapons);
            return Flight::writes(controls, wanted, Cap::Slot)
                .doing(Word::thrusting(controls, handling));
        }
    };
    let (flown, cap) = match body.manoeuvre {
        Manoeuvre::BreakOff => (
            Handling {
                accel: handling.accel * BREAK_THRUST,
                ..*handling
            },
            Cap::Step,
        ),
        Manoeuvre::Rush => (
            Handling {
                accel: handling.accel * RUSH_THRUST,
                // `_ShipMaxSpeed` × the double 1.8, back to a float
                // (@0x865cf-0x865e3).
                #[allow(clippy::cast_possible_truncation)]
                max_speed: (f64::from(handling.max_speed) * RUSH_SPEED) as f32,
                ..*handling
            },
            Cap::Slot,
        ),
        Manoeuvre::Dogfight => match (inertia, speed_cap) {
            (Inertia::Inertialess { .. }, Some(cap)) if cap > 0.0 => (
                Handling {
                    max_speed: cap,
                    ..*handling
                },
                Cap::Slot,
            ),
            _ => (*handling, Cap::Step),
        },
        // The swarm keeps the head's +0x34 (@0x8533d).
        Manoeuvre::Swarm => (*handling, Cap::Slot),
        _ => (*handling, Cap::Step),
    };
    // By its heading at the slot, before `_HandleShip` turns it to the
    // body's +0x68.
    let thrust = fire::off_by(npc.state.heading, body.heading) < window;
    let controls = Controls {
        thrust,
        ..Controls::default()
    };
    // The swarm (sub 0x12) writes +0x30 alone (@0x8533d); every other
    // body writes +0x34 as it thrusts.
    let word = if body.manoeuvre == Manoeuvre::Swarm {
        Word::thrusting(controls, &flown)
    } else {
        Word::Writes
    };
    Flight::writes(controls, body.heading, cap)
        .by(flown)
        .doing(word)
}

/// How `npc` flies a tick landing on `stellar` among `sites` on
/// `handling`, as its inertia says, by a reading of
/// [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding) (`by`): its flight,
/// or how its own flight ended. By the engine's, [`land_flight::low_level`];
/// by the other, landed and staying when [`over`] the stellar, else its
/// approach ([`land_approach`]). A stellar not among `sites` brakes it:
/// the original's guard on a travel stellar of -1 (@0x8d8c9-0x8d8d1) does
/// nothing, but a landing goal always names its stellar, so it can't be
/// reached.
fn land(
    npc: &mut Npc,
    sites: &[LandingSite],
    stellar: StellarId,
    handling: &Handling,
    (inertia, reading): (Inertia, RuleSource),
    chance: &mut (impl Chance + ?Sized),
) -> Result<Flight, Outcome> {
    let Some(site) = sites.iter().find(|site| site.id == stellar) else {
        return Ok(braking(npc, handling, inertia));
    };
    if reading == RuleSource::Engine {
        return land_flight::low_level(npc, site, handling, inertia, chance);
    }
    if over(&npc.state, site, handling, reading) {
        land_flight::stay(npc, stellar, chance);
        return Err(Outcome::Landed(stellar));
    }
    Ok(land_approach(npc, site, handling, inertia, reading))
}

/// How `npc` flies into `gate` among `sites`, of the widths
/// `type_widths` gives, as its inertia says, by a reading of
/// [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding) (`by`; see the
/// module docs of [`autopilot`](super)): outside its box, sub 2 by the
/// engine's ([`gate_entry::approach`]) or the port's earlier [`approach`]
/// by the other, or braking with no gate there; inside it, stopped, with
/// no step that pass. Its entry, the 16-count wait and the release of its
/// followers, is its high level's, at its slot of the AI loop (the
/// traffic's `dispatch_leave`).
fn enter_box(
    npc: &mut Npc,
    sites: &[LandingSite],
    type_widths: &BTreeMap<StellarId, TypeWidth>,
    gate: StellarId,
    handling: &Handling,
    (inertia, reading): (Inertia, RuleSource),
) -> Result<Flight, Outcome> {
    let Some(site) = sites.iter().find(|site| site.id == gate) else {
        return Ok(braking(npc, handling, inertia));
    };
    let reach = gate_entry::gate_reach(site, type_widths);
    if !gate_entry::inside(site.position, npc.state.position, reach) {
        return Ok(match reading {
            RuleSource::Engine => gate_entry::approach(&npc.state, site.position, handling),
            RuleSource::Bible => {
                let (controls, wanted) = approach(&npc.state, handling, site.position, inertia);
                Flight::writes(controls, wanted, Cap::Step)
            }
        });
    }
    npc.state.velocity = Vec2::ZERO;
    Err(Outcome::Flying)
}

/// How `npc` keeps its slot in the formation of its leader at `leader`
/// by the port's earlier formation ([`FormationFlight::Earlier`], see the
/// module docs of [`autopilot`](super)): rejoining it, or braking with no
/// leader or slot; or, within [`KEEP_FORMATION`] of it, keeping formation
/// ([`keep_formation`]).
fn formation(
    npc: &mut Npc,
    leader: Option<&ShipState>,
    handling: &Handling,
    inertia: Inertia,
) -> Flight {
    let (Some(leader), Some(place)) = (leader, npc.formation_slot()) else {
        return braking(npc, handling, inertia);
    };
    let slot = place.position(leader);
    let off = slot - npc.state.position;
    if off.x.abs() <= KEEP_FORMATION && off.y.abs() <= KEEP_FORMATION {
        return keep_formation(npc, leader, slot, handling);
    }
    let (controls, wanted) = rejoin(&npc.state, handling, slot, leader.velocity, inertia);
    Flight::writes(controls, wanted, Cap::Step)
}

/// How `npc` flies back to its carrier at `carrier` (see the module docs
/// of [`autopilot`](super)), as `inertia` says: the flight outside its
/// dock window, or braking with no carrier there; inside it, sub 8's
/// turn and thrust ([`dock_in`]), pulled toward the carrier first
/// ([`pull_in`]) while outside its reach. It never docks here: its low
/// level docks it at its slot of the AI loop ([`docks_now`](super::docks_now)),
/// before this, so one still here within reach has no bay, and stays out,
/// unpulled.
fn dock(
    npc: &mut Npc,
    carrier: Option<&ShipState>,
    handling: &Handling,
    inertia: Inertia,
) -> Flight {
    let (Some(carrier), Some(dock)) = (carrier, npc.carrier) else {
        return braking(npc, handling, inertia);
    };
    let off = carrier.position - npc.state.position;
    if !within_reach(off, dock.window) {
        let (controls, wanted) = rejoin(
            &npc.state,
            handling,
            carrier.position,
            carrier.velocity,
            inertia,
        );
        return Flight::writes(controls, wanted, Cap::Step);
    }
    let flight = dock_in(&npc.state, carrier, handling);
    if !within_reach(off, dock.reach) {
        pull_in(npc, carrier, handling);
    }
    flight
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
) -> (Controls, i16) {
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

/// Sub 8 of a ship at `state` docking with a carrier at `carrier`, inside
/// its dock window (see the module docs of [`autopilot`](super)): it wants
/// the carrier's table bearing, thrusting while off it by less than its
/// turn rate and [`DOCK_TURN_SLACK`], whole degrees, strictly, by its
/// heading at the slot, writing +0x34 = 0 (@0x87ef8-0x87eff).
fn dock_in(state: &ShipState, carrier: &ShipState, handling: &Handling) -> Flight {
    // +0x68, the table's bearing of the carrier (@0x87e81-0x87e9d).
    let wanted = trig::intercept_angle(state.position, carrier.position);
    // Off it by less than a turn and the slack, strictly, by its heading
    // before `_HandleShip` turns it (@0x87ea9-0x87ee4), its turn rate read
    // at the slot (@0x87eb6).
    let thrust = fire::off_by(state.heading, wanted) < handling.turn_rate + DOCK_TURN_SLACK;
    let controls = Controls {
        thrust,
        ..Controls::default()
    };
    Flight::writes(controls, wanted, Cap::Step)
}

/// `npc`, docking, is pulled towards a carrier at `carrier` on each axis
/// farther off than [`FORMATION_NUDGE`] ticks' acceleration
/// (@0x87ff9 onward), at its slot: none for a fighter with no bay within
/// its carrier's reach, whose call skips it (@0x88111).
fn pull_in(npc: &mut Npc, carrier: &ShipState, handling: &Handling) {
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

/// Whether a ship at `state`, turning on `handling`, can land on `site`
/// by `reading` of [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding):
/// over it ([`landing::npc_over`], which owns the reading's region; the
/// other reading's circle, within its
/// [`landing_radius`](landing::landing_radius), ignores the turn) and at
/// or below [`LANDING_SPEED`]. [`land`] asks it only by the other
/// reading: the engine's flies [`land_flight::low_level`].
fn over(state: &ShipState, site: &LandingSite, handling: &Handling, reading: RuleSource) -> bool {
    landing::npc_over(site, state.position, handling.turn_rate, reading)
        && state.velocity.length() <= LANDING_SPEED
}

/// How `npc`, landing on `site` by `reading` of
/// [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding) (the other, from
/// [`land`]) but not [`over`] it, flies on `handling` as `inertia` says:
/// toward it ([`approach`]), a folding ship unfolding once over it by
/// [`landing::npc_over`], within its landing radius (the engine's reading
/// unfolds in its box, `_HighLevelAIHandler` state 1 @0x8da0c-0x8da38,
/// see [`land_flight`]).
fn land_approach(
    npc: &mut Npc,
    site: &LandingSite,
    handling: &Handling,
    inertia: Inertia,
    reading: RuleSource,
) -> Flight {
    if landing::npc_over(site, npc.state.position, handling.turn_rate, reading) {
        ship_display::landing_unfold(npc);
    }
    let (controls, wanted) = approach(&npc.state, handling, site.position, inertia);
    Flight::writes(controls, wanted, Cap::Step)
}

/// Heads for `target` at full speed, and brakes once the ship's stopping
/// distance reaches the distance left.
pub(super) fn approach(
    state: &ShipState,
    handling: &Handling,
    target: Vec2,
    inertia: Inertia,
) -> (Controls, i16) {
    let to = target - state.position;
    let distance = to.length();
    if distance <= stopping_distance(state, handling, inertia) {
        return approach_brake(state, handling, inertia);
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
pub(super) fn stopping_distance(state: &ShipState, handling: &Handling, inertia: Inertia) -> f32 {
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

/// The approach's brake ([`approach`]), the engine's own planner for the
/// landing and the gate approach by the other reading of
/// [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding) (the engine's
/// flies sub 2, which never brakes, and brakes inside the landing box by
/// sub 1 × 0.98, [`land_flight`]), not sub 1: it wants to face against
/// the ship at `state`'s motion and thrusts once it does, until thrust
/// would only push it the other way; nothing at or below
/// [`AT_REST_SPEED`] (see [`stopping_distance`]). Between the two it
/// creeps, which is harmless here: the landing ends at `LANDING_SPEED`
/// (1.0, above any stock `accel / 2`), and the gate box zeroes the
/// velocity. It wants sub 1's whole-degree retro heading
/// ([`snipe::retro`]), which `_HandleShip` turns it to
/// ([`flight::turn_to`]), thrusting once that turn puts it on the heading,
/// by the slot's turn rate. At rest it wants its own whole heading. An
/// inertialess ship brakes as substate 1 does ([`inertialess::brake`]),
/// wanting the heading it hands back, its own truncated.
pub(super) fn approach_brake(
    state: &ShipState,
    handling: &Handling,
    inertia: Inertia,
) -> (Controls, i16) {
    if let Inertia::Inertialess { .. } = inertia {
        return inertialess::brake(state);
    }
    let speed = state.velocity.length();
    if speed <= AT_REST_SPEED {
        return (Controls::default(), trig::whole(state.heading));
    }
    let retro = snipe::retro(state.velocity);
    // Put on its retro heading this tick, before this tick's thrust.
    let facing_back = flight::turns_onto(state.heading, retro, handling.turn_rate);
    let controls = Controls {
        thrust: facing_back && speed > handling.accel / 2.0,
        ..Controls::default()
    };
    (controls, retro)
}

/// Sub 1 (`_LowLevelAIHandler` @0x8536d-0x85633,
/// [`leave_flight::stop`]) as its flight ([`Flight::stop`]), flown by the
/// handling it hands back (half thrust on the soft brake), keeping an owed
/// burst: every goal the autopilot stops flies it. Moving
/// ([`inertialess::moving`], 0.35 or more on an axis) it brakes, with no
/// cutoff short of that; otherwise it settles, its velocity × 0.95 a tick,
/// so it comes to rest from any speed. The settle cuts the speed along
/// the heading (+0x48) × 0.95 with it (@0x8560d-0x8561f), so an
/// inertialess ship, whose velocity follows heading × speed, comes to
/// rest too. The sub-goal +0xc8ca = 0 after the settle (@0x85624) isn't
/// modelled: the high level writes sub 1 again each pass for every stop
/// goal, as this flies it every tick, so nothing differs.
pub(super) fn braking(npc: &mut Npc, handling: &Handling, inertia: Inertia) -> Flight {
    leave_flight::stop_flight(npc, handling, inertia)
}

/// [`follow`]s a lead at `lead`, or brakes with none there
/// ([`braking`]).
fn follow_or_brake(
    npc: &mut Npc,
    handling: &Handling,
    lead: Option<&ShipState>,
    inertia: Inertia,
) -> Flight {
    match lead {
        Some(lead) => {
            let (controls, wanted) = follow(&npc.state, handling, lead, inertia);
            Flight::writes(controls, wanted, Cap::Step)
        }
        None => braking(npc, handling, inertia),
    }
}

/// Keeps within [`FOLLOW_DISTANCE`] of a lead at `lead`: closing at the
/// speed it can still brake from in the distance beyond it, on top of the
/// lead's velocity, and matching the lead's velocity within it.
pub(super) fn follow(
    state: &ShipState,
    handling: &Handling,
    lead: &ShipState,
    inertia: Inertia,
) -> (Controls, i16) {
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
) -> (Controls, i16) {
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

/// Keeps formation on `slot` beside a leader at `leader` by the port's
/// earlier formation ([`FormationFlight::Earlier`]; the original's is
/// [`formation_flight`]'s): `npc` takes the leader's velocity and heading
/// at its slot, and its flight moves it with no thrust or turn, after each
/// axis more than [`FORMATION_SLACK`] off the slot from where that move
/// takes it is nudged towards it by [`FORMATION_NUDGE`] ticks' acceleration
/// at most, never past it: the thrust `handling` flies it with.
fn keep_formation(npc: &mut Npc, leader: &ShipState, slot: Vec2, handling: &Handling) -> Flight {
    npc.display.drive.glow = GlowCopy::Leader;
    let state = &mut npc.state;
    state.velocity = leader.velocity;
    state.heading = leader.heading;
    let moved = state.position + state.velocity;
    let nudge = handling.accel * FORMATION_NUDGE;
    let towards = |at: f32, to: f32| {
        let off = to - at;
        if off.abs() > FORMATION_SLACK {
            off.signum() * nudge.min(off.abs())
        } else {
            0.0
        }
    };
    state.position = state.position + Vec2::new(towards(moved.x, slot.x), towards(moved.y, slot.y));
    Flight {
        wanted: None,
        ..Flight::writes(Controls::default(), 0, Cap::Step)
    }
}

/// The fastest a ship can go and still turn round and stop within
/// `distance`: the `v` with `v T + v² / 2a = distance`, `T` the ticks a
/// half turn takes. None when it cannot turn or thrust. An inertialess
/// ship's is [`inertialess::braking_speed`].
pub(super) fn braking_speed(distance: f32, handling: &Handling, inertia: Inertia) -> f32 {
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

/// Wants the change that takes the ship's velocity to `desired`, and
/// thrusts once it faces it, by its heading at the slot; coasts once the
/// velocity is within half a tick's thrust of it, wanting its own whole
/// heading. The heading it wants is the table heading of the change,
/// `_CalcInterceptAngle((0,0), change × 100.0)`, as sub 1 reads its
/// velocity, and `_HandleShip` turns it there ([`flight::turn_to`]). An
/// inertialess ship matches its speed instead ([`inertialess::match_speed`]).
pub(super) fn match_velocity(
    state: &ShipState,
    handling: &Handling,
    desired: Vec2,
    inertia: Inertia,
) -> (Controls, i16) {
    if let Inertia::Inertialess { speed } = inertia {
        return inertialess::match_speed(state, speed, handling, desired);
    }
    let change = desired - state.velocity;
    if change.length() <= handling.accel / 2.0 {
        return (Controls::default(), trig::whole(state.heading));
    }
    let off = shortest_turn(state.heading, heading_of(change));
    let thrust = off.abs() <= ALIGNED.max(handling.turn_rate);
    let wanted = trig::intercept_angle(Vec2::ZERO, change * 100.0);
    let controls = Controls {
        thrust,
        ..Controls::default()
    };
    (controls, wanted)
}
