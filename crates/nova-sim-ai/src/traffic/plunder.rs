//! A pirate's plunder among the traffic: the docking's high level and
//! low level ([`high_level`], [`dock_low_level`]), the plunder its think decides
//! (`_PirateWarshipAI` @0x8c74c-0x8c7ae in the `EV Nova` executable,
//! [`resolve`]) and the drift that follows it (goal 0xe, `_HighLevelAIHandler`
//! @0x8eb6b-0x8ebd4, [`drift`]). The rules of the plunder itself are
//! [`plunder_contents`]'s; the player's side of it, its cargo, credits,
//! lines, sounds and failed missions, the session's (`nova_sim`'s
//! `pirate_plunder` module), which drains [`PirateEvent`]s.
//!
//! - **The high level** ([`high_level`]): while a ship's AI wait runs
//!   ([`Npc::ai_wait`]) `_HighLevelAIHandler` returns before every goal but
//!   10 and 0xe (@0x8d4b9-0x8d4d2), so the traffic keeps its pursuit and
//!   docking as they were. Goal 0x16's block comes before that return
//!   (@0x8d462-0x8d4ac), so a pirate's count wait ([`Goal::CountWait`])
//!   flies on as a fight with no quarry; a ship turned out of it onto
//!   another goal keeps the wait and is skipped. A plunder whose ship has
//!   gone, its target cleared by the slot test, takes goal 0 alone,
//!   keeping its docking and nav for that tick's low level
//!   ([`HighLevel::LetsGo`], @0x8e761 → @0x8eb5f); the next tick's goal
//!   drops them. The goal's ship stands in for its target (+0x70), which
//!   only the slot test clears under goal 0xd, so a plunder of a ship
//!   still there reads it whatever target the traffic holds. A plunder of
//!   a disabled ship docks
//!   ([`plunder_dock::high_level`]), as does a repair of the player
//!   ([`repair_dock::high_level`], nothing changing with no target); a
//!   refuel with a target refuels ([`HighLevel::Refuels`], its docking
//!   set by the refuel pass, [`refuel_dock`](crate::traffic::refuel_dock)),
//!   nothing changing with none (@0x8f3f3); any
//!   other goal flies its fight, which
//!   drops any docking, as goal 4's and goal 0xd's other substates do.
//!   While the wait skips it, a docking outlives whatever goal the think
//!   writes, unless the think writes the sub itself
//!   ([`plunder_dock::kept_by`]).
//! - **The plunder** ([`plunder()`]), over the decisions that plunder
//!   ([`Decision::plunders`], [`resolve`]), in berth order, and in sub
//!   0xf's completion under goal 0xd at the pirate's slot (the traffic's
//!   `dispatch_dock`): the pirate takes goal 0xe ([`Goal::Drift`]), an AI
//!   wait of [`PLUNDER_HOLD`] and +0x8c -1 ([`Npc::defends`]), then
//!   plunders its nav, then its target and nav are cleared. A ship gone
//!   from the system, read by its slot's record, or breaking up yields
//!   nothing (@0x83fbd-0x83fd4). Against an NPC it moves the
//!   victim's cargo into its free holds ([`steal`]) and zeroes the
//!   victim's credits (+0xa0, read by no one, so not kept); a mission ship
//!   keeps nothing else, and any other may be captured ([`captures`]).
//!   Captured, the victim joins the pirate's fleet: its person (+0xc8d0,
//!   its berth's name for `<OSN>` too, though the person stays alive, as
//!   `_AIPlunderShipContents` leaves its +0x620) and former kind are
//!   cleared, its leader is the pirate (AI type 6), its goal and sub 0,
//!   unboarded, of the pirate's government, waiting [`CAPTURED_WAIT`],
//!   its armour [`CAPTURED_ARMOR_SHARE`] of its capacity, its shield none,
//!   and its condition as the [`World`]'s disable rule reads that
//!   (@0x84946-0x849cd). An announced capture ([`taken`]) first drops
//!   every ship in slots 1-63 that targets the victim: target and nav -1,
//!   goal and sub 0, provocation 0 (@0x8484a-0x848b1), the pirate among
//!   them, so it doesn't drift; a silent one drops none. Against the
//!   player it asks the session ([`PirateEvent::PlunderPlayer`]).
//! - **The drift** ([`drift`]), on each think of a drifting ship: target,
//!   nav and sub cleared, 0.7 pixels along its whole-degree heading
//!   (`_Accel` @0x8ebba), and goal 0 once its wait reads none. The tick
//!   that plunders drifts too, so with the wait counted down after each
//!   tick's flight it drifts 101 times, on the 101st setting goal 0, and
//!   thinks afresh on the next. The flight moves it on at its velocity
//!   meanwhile ([`autopilot`](crate::traffic::autopilot)).
//!
//! The original plunders inside each pirate's think or its low level, and
//! so does the traffic (`Traffic::think_run`, `dispatch_dock`): a later slot's think reads an earlier
//! slot's capture that frame, a ship the capture dropped thinking from goal
//! 0 and a ship taken above its pirate thinking at once as its escort.
//! Nothing here plays a sound: the session raises `snd ` 151 with each
//! line.

#[cfg(doc)]
use crate::ai::plunder_contents;
use crate::ai::plunder_contents::{
    CAPTURED_ARMOR_SHARE, CAPTURED_WAIT, CargoLine, Led, PLUNDER_HOLD, Side, Taken, Victim,
    captures, cargo_line, roll_odds, steal, taken,
};
use crate::ai::swarm::SwarmLead;
use crate::ai::{Decision, Goal, Surroundings};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::{self, FlightRules, ShipState};
use crate::hail::Help;
use crate::inertialess::InertialDampener;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{NPC_COMMODITIES, Npc, NpcHold, NpcId};
use crate::traffic::plunder_dock::{self, DockOutcome, DockStep, Docking};
use crate::traffic::repair_dock;
use crate::traffic::{Traffic, World};
use nova_sim_base::trig;
use nova_sim_catalog::catalog::ShipId;

/// How far a drifting ship moves along its heading each think, in pixels
/// (the double 0.7 @0xdd0d0, by the frame).
pub const DRIFT: f32 = 0.7;

/// What the pirates did that their session answers, queued on the line
/// at the actor's berth ([`OnLine::Pirate`](crate::traffic::on_line::OnLine::Pirate)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PirateEvent {
    /// A pirate, or a ship repairing the player, started its dock with the
    /// disabled player: the player is boarded (+0xb9 = 1, @0x86fc7).
    PlayerDockedWith {
        /// The pirate or repairer.
        pirate: NpcId,
    },
    /// A ship docked with the player repaired it (`_LowLevelAIHandler`
    /// @0x86d84-0x86f37; see [`repair_dock`]): the session unboards the
    /// player, raises its armour until it is no longer disabled and says
    /// "Repairs complete" with `snd ` 151.
    PlayerRepaired {
        /// The repairer.
        helper: NpcId,
        /// Its ship type, whose `CommName` the line names.
        ship: ShipId,
    },
    /// A ship docked with a disabled NPC repaired it (`_LowLevelAIHandler`
    /// @0x86d84-0x86dcf with an NPC nav; see [`repair_dock`]): the session
    /// unboards the NPC and raises its armour until it is no longer
    /// disabled, silently (@0x86dd4 skips the line and sound).
    NpcRepaired {
        /// The repairer.
        helper: NpcId,
        /// The ship it repaired.
        ship: NpcId,
    },
    /// A pirate plunders the player: the session takes its cargo and
    /// credits, says the line and fails its missions.
    PlunderPlayer {
        /// The pirate.
        pirate: NpcId,
    },
    /// A pirate plundered an NPC.
    Plundered {
        /// The pirate.
        pirate: NpcId,
        /// The victim.
        victim: NpcId,
        /// The tons it took.
        tons: u32,
        /// The cargo line said, if any.
        line: Option<CargoLine>,
        /// How the victim was captured, if it was.
        taken: Option<Taken>,
    },
}

impl PirateEvent {
    /// The ship that acted: the pirate, or the ship repairing the player.
    #[must_use]
    pub fn actor(&self) -> NpcId {
        match *self {
            Self::PlayerDockedWith { pirate }
            | Self::PlunderPlayer { pirate }
            | Self::Plundered { pirate, .. } => pirate,
            Self::PlayerRepaired { helper, .. } | Self::NpcRepaired { helper, .. } => helper,
        }
    }
}

/// What a ship's high level does this think (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighLevel {
    /// Nothing: its AI wait runs.
    Skipped,
    /// It docks with `ship`, disabled, as `step` says.
    Docks {
        /// The ship it docks with.
        ship: ShipRef,
        /// What the docking's high level says.
        step: DockStep,
        /// The slot of the ship it docks with, as +0x6c holds it
        /// ([`travel::slot_of`](crate::ai::travel::slot_of)): what the
        /// repair writes (@0x8f74c) and the plunder of a wreck too
        /// (@0x8e8c9), as its decision does.
        slot: Option<crate::catalog::StellarId>,
    },
    /// It flies its fight, if any.
    Fights,
    /// Its plunder's ship has gone, so the slot test cleared its target
    /// (+0x70 -1, @0x8d4e4-0x8d50d), the only clear under goal 0xd; a ship
    /// still there is read whatever target the traffic holds, as a fresh
    /// plunder's is taken after the high level. Goal 0xd writes goal 0 alone
    /// (@0x8e74e-0x8e761 → @0x8eb5f), keeping its docking and nav, so that
    /// tick's low level flies its sub at the ship's record. Its +0x50 = 0
    /// (@0x8e751) is the hold's ([`warp_out::hold_stage`](crate::traffic::warp_out::hold_stage),
    /// @0x8d52e), which runs first.
    LetsGo,
    /// It stops (sub 1) while it searches for a target it lost sight of
    /// ([`lost_sight::searching`](crate::ai::lost_sight::searching)),
    /// dropping any docking.
    Stops,
    /// It refuels its target (goal 9), keeping its docking for the refuel
    /// pass to set ([`refuel_dock`](crate::traffic::refuel_dock)).
    Refuels,
}

/// What `npc`'s high level does this think among `around` (see the module
/// docs), counting a dampener as `rule` says
/// ([`RuleKey::InertialDampener`](crate::RuleKey::InertialDampener)).
#[must_use]
pub fn high_level(npc: &Npc, around: &Surroundings, rule: InertialDampener) -> HighLevel {
    if npc.ai_wait > 0 && !matches!(npc.goal, Goal::Hold | Goal::Drift | Goal::CountWait) {
        return HighLevel::Skipped;
    }
    if npc.goal == Goal::Assist(Help::Refuel) {
        // No target, no block (@0x8f3f3).
        return if npc.target.is_some() {
            HighLevel::Refuels
        } else {
            HighLevel::Skipped
        };
    }
    if npc.goal == Goal::Assist(Help::Repair) {
        return match (repair_dock::high_level(npc, around, rule), npc.target) {
            (Some(step), Some(ship)) => HighLevel::Docks {
                ship,
                step,
                slot: crate::ai::travel::slot_of(ship, around),
            },
            _ => HighLevel::Skipped,
        };
    }
    let Goal::Plunder { ship, .. } = npc.goal else {
        return HighLevel::Fights;
    };
    // The slot test clears +0x70 only for a slot gone from the system, and
    // a fresh plunder's target is taken after the high level here.
    let Some(at) = around.state_of(ship) else {
        return HighLevel::LetsGo;
    };
    if !around.condition_of(ship).is_some_and(Condition::disabled) {
        return HighLevel::Fights;
    }
    let boarded = match ship {
        ShipRef::Player => around.player.is_some_and(|player| player.boarded),
        ShipRef::Npc(id) => around.npc(id).is_some_and(|other| other.boarded),
    };
    HighLevel::Docks {
        ship,
        step: plunder_dock::high_level(npc, at.position, boarded, rule),
        slot: crate::ai::travel::slot_of(ship, around),
    }
}

/// Applies `high`, `npc`'s high level this think: nothing while its wait
/// runs, a docking set or given up (goal 0, sub 0, target and nav -1, its
/// travel stellar cleared: @0x8e8b3, @0x8f732), a docking set holding
/// its ship's slot as its travel stellar (@0x8e8c9, @0x8f74c), goal 0
/// alone for a plunder whose ship has gone ([`HighLevel::LetsGo`]), or any
/// docking dropped as it flies its fight.
pub fn take_high_level(npc: &mut Npc, high: HighLevel) {
    match high {
        HighLevel::Skipped | HighLevel::Refuels => {}
        HighLevel::Docks {
            step: DockStep::GiveUp,
            ..
        } => {
            npc.goal = Goal::Idle;
            npc.target = None;
            npc.docking = None;
            npc.travel = None;
        }
        HighLevel::Docks {
            ship,
            step: DockStep::Sub(sub),
            slot,
        } => {
            npc.docking = Some(Docking { ship, sub });
            npc.travel = slot.or(npc.travel);
        }
        HighLevel::Fights | HighLevel::Stops => npc.docking = None,
        HighLevel::LetsGo => npc.goal = Goal::Idle,
    }
}

/// The low level of `npc`'s docking at `victim` (none when that ship is
/// gone, which drops the docking), at its own top speed `own` under
/// `flight`, with `swarm` its swarm lead, rolling a dock's wait on
/// `chance` ([`plunder_dock::low_level`]): the flight it leaves
/// `_HandleShip` and what became of the docking, or none when it has no
/// docking to fly, is not intact or not flying, for the autopilot to fly
/// it instead. It runs at the ship's slot of the AI loop (the traffic's
/// `dispatch_dock`).
pub fn dock_low_level(
    npc: &mut Npc,
    victim: Option<&ShipState>,
    own: f32,
    flight: FlightRules,
    swarm: Option<SwarmLead>,
    chance: &mut (impl Chance + ?Sized),
) -> Option<(plunder_dock::DockFlight, DockOutcome)> {
    npc.docking?;
    if npc.condition != Condition::Intact || npc.mode != crate::traffic::npc::Mode::Flying {
        return None;
    }
    let Some(victim) = victim else {
        npc.docking = None;
        return None;
    };
    let handling = npc.flying_handling(own, flight);
    Some(plunder_dock::low_level(
        npc, victim, &handling, flight, swarm, chance,
    ))
}

/// `npc`'s whole tick of its docking at `victim`, its low level then
/// `_HandleShip` ([`dock_low_level`], [`plunder_dock::handle_ship`]), the
/// ships the player leads at `led_speed`, under `flight` with `swarm` its
/// swarm lead, a dock's wait rolling 0: for the tests of the flight it
/// shares. Each half reads its top speed as the traffic's does: the low
/// level at its slot (`dispatch_dock`), `_HandleShip` at its step
/// ([`plunder_dock::AtStep::of_npc`]).
#[cfg(test)]
pub(crate) fn fly_docking(
    npc: &mut Npc,
    victim: Option<&ShipState>,
    led_speed: f32,
    flight: FlightRules,
    swarm: Option<SwarmLead>,
) -> Option<DockOutcome> {
    let chance = &mut crate::chance::NeverFires;
    let own = npc.top_speed(led_speed);
    let (flown, outcome) = dock_low_level(npc, victim, own, flight, swarm, chance)?;
    let at_step = plunder_dock::AtStep::of_npc(npc, led_speed, flight);
    plunder_dock::handle_ship(npc, &flown, at_step, flight);
    Some(outcome)
}

/// A tick off each AI wait among `npcs` that has any left (`_HandleShip`
/// @0x35037-0x35048), on each of double time's passes.
pub fn count_down(npcs: &mut [Npc]) {
    for npc in npcs {
        npc.ai_wait = npc.ai_wait.saturating_sub(1);
    }
}

/// The drift of each ship among `npcs` whose think was `due` and which
/// drifts (see the module docs).
pub fn drift(npcs: &mut [Npc], due: &[bool]) {
    for (npc, _) in npcs.iter_mut().zip(due).filter(|(_, due)| **due) {
        if npc.goal != Goal::Drift {
            continue;
        }
        npc.target = None;
        npc.docking = None;
        let along = flight::facing(f32::from(trig::whole(npc.state.heading)));
        npc.state.position = npc.state.position + along * DRIFT;
        if npc.ai_wait == 0 {
            npc.goal = Goal::Idle;
        }
    }
}

/// The plunders `decisions`, one for each of `npcs` in order, decide: each
/// pirate's index and its victim.
#[must_use]
pub fn plunders(decisions: &[Option<Decision>]) -> Vec<(usize, ShipRef)> {
    decisions
        .iter()
        .enumerate()
        .filter_map(|(at, decision)| {
            decision
                .and_then(|decision| decision.plunders)
                .map(|ship| (at, ship))
        })
        .collect()
}

/// Runs each plunder of `plunders` among `npcs` in berth order, in
/// `world`, rolling on `chance` (see the module docs): what happened, for
/// the session.
pub fn resolve(
    npcs: &mut [Npc],
    mut plunders: Vec<(usize, ShipRef)>,
    world: &World,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<PirateEvent> {
    plunders.sort_by_key(|&(at, _)| npcs[at].berth);
    plunders
        .into_iter()
        .filter_map(|(at, victim)| plunder(npcs, at, victim, world, chance))
        .collect()
}

/// The pirate at `at` among `npcs` plunders `victim`, the ship its nav
/// names, in `world`, rolling on `chance`, as the type think's plunder
/// (@0x8c772-0x8c7ae, [`resolve`]) and sub 0xf's completion
/// (@0x86f46-0x86f8d, the traffic's `dispatch_dock`) both do: goal 0xe
/// ([`Goal::Drift`]) and an AI wait of [`PLUNDER_HOLD`], its +0x8c -1
/// (@0x8c791, @0x86f67: [`Npc::defends`]), then `_AIPlunderShipContents`
/// (see the module docs), then its target, docking and travel stellar
/// cleared (@0x8c7a8-0x8c7ae, @0x86f87-0x86f8d). What happened, for the
/// session: nothing for an NPC gone from the system or breaking up
/// (@0x83fbd-0x83fd4), whose record is left as it was.
pub fn plunder(
    npcs: &mut [Npc],
    at: usize,
    victim: ShipRef,
    world: &World,
    chance: &mut (impl Chance + ?Sized),
) -> Option<PirateEvent> {
    npcs[at].goal = Goal::Drift;
    npcs[at].ai_wait = PLUNDER_HOLD;
    npcs[at].defends = None;
    let pirate = npcs[at].id;
    let event = match victim {
        ShipRef::Player => Some(PirateEvent::PlunderPlayer { pirate }),
        ShipRef::Npc(id) => plunder_npc(npcs, at, id, world, chance),
    };
    npcs[at].target = None;
    npcs[at].docking = None;
    npcs[at].travel = None;
    event
}

impl Traffic {
    /// Queues `event`, a plunder's, on the line at its pirate's berth
    /// ([`on_line`](crate::traffic::on_line)); a captured ship's berth
    /// forgets its person's name, as its slot's +0xc8d0 is cleared
    /// (@0x84953).
    pub(super) fn settle_plunder(&mut self, event: PirateEvent) {
        if let PirateEvent::Plundered {
            victim,
            taken: Some(_),
            ..
        } = event
            && let Some(berth) = (self.npcs.iter())
                .find(|npc| npc.id == victim)
                .map(|npc| npc.berth)
        {
            self.forget_berth_person(berth);
        }
        self.queue_pirate(event);
    }
}

/// The pirate at `at` among `npcs` plunders NPC `victim`, if it is in the
/// system, in `world`, rolling on `chance` (see the module docs).
fn plunder_npc(
    npcs: &mut [Npc],
    at: usize,
    victim: NpcId,
    world: &World,
    chance: &mut (impl Chance + ?Sized),
) -> Option<PirateEvent> {
    let into = npcs.iter().position(|npc| npc.id == victim)?;
    // `_IsDying` or inactive (@0x83fbd-0x83fd4).
    if !npcs[into].condition.standing() {
        return None;
    }
    let side = |npc: &Npc, reading: RuleSource| Side {
        strength: npc.marines.strength,
        marines: npc.marines.read(reading),
    };
    let odds = roll_odds(
        side(&npcs[at], world.plunder_marines),
        side(&npcs[into], world.plunder_marines),
        chance,
    );
    let pirate_holds = npcs[at].holds;
    let mut words = hold_words(&npcs[into].cargo);
    let mut hold = npcs[at].cargo;
    let tons = steal(
        &mut words,
        u32::from(npcs[into].holds),
        &mut hold,
        pirate_holds,
        chance,
    );
    npcs[at].cargo = hold;
    npcs[into].cargo = NpcHold::from_words(words);
    let ship = &npcs[into];
    let led = match ship.leader_ref() {
        None => Led::ByNone,
        Some(ShipRef::Player) => Led::ByPlayer,
        Some(ShipRef::Npc(_)) => Led::ByNpc,
    };
    let former = ship.former.map(|former| former.kind());
    let mission = ship.mission.is_some();
    let line = cargo_line(
        Victim::Npc {
            led,
            former,
            mission,
        },
        tons,
        0,
    );
    let pirate = npcs[at].id;
    let mut event = PirateEvent::Plundered {
        pirate,
        victim,
        tons,
        line,
        taken: None,
    };
    if mission || !captures(odds, chance) {
        return Some(event);
    }
    let how = taken(led, former);
    if how != Taken::Silently {
        for other in npcs.iter_mut() {
            if other.target == Some(ShipRef::Npc(victim)) && other.condition.active() {
                other.target = None;
                other.travel = None;
                other.docking = None;
                other.goal = Goal::Idle;
                other.pursuit = crate::ai::pursuit::Pursuit::default();
                other.provoked = 0.0;
            }
        }
    }
    let pirate_ship = npcs[at].clone();
    let ship = &mut npcs[into];
    ship.take_govt_of(&pirate_ship);
    ship.person = None;
    ship.former = None;
    ship.escort = None;
    ship.leader = Some(pirate);
    // +0x88 = 6 (@0x8495c) overwrites a bribe's 1, its only word
    // (`_DoCommDialog` @0x961f8): the taken ship spares the player no more.
    ship.overwrite_bribe();
    // It defends no stellar now. The original writes no +0x8c of the ship
    // it takes (@0x8c791 is the pirate's): see
    // rdm:phase/combat-ai-fidelity-cross-roadmap/phase-16-left-out-of-phase-13.
    ship.defends = None;
    ship.goal = Goal::Idle;
    ship.pursuit = crate::ai::pursuit::Pursuit::default();
    ship.docking = None;
    ship.boarded = false;
    ship.ai_wait = CAPTURED_WAIT;
    // Doubles, back to a float (@0x849b4-0x849c8).
    #[allow(clippy::cast_possible_truncation)]
    let armor = (f64::from(ship.reserves.armor.max) * CAPTURED_ARMOR_SHARE) as f32;
    ship.reserves.armor.now = armor;
    ship.reserves.shield.now = 0.0;
    ship.condition = if armor > 0.0 && world.disable.disabled(ship.reserves.armor, &ship.hull) {
        Condition::Disabled
    } else {
        Condition::Intact
    };
    if let PirateEvent::Plundered { taken, .. } = &mut event {
        *taken = Some(how);
    }
    Some(event)
}

/// `hold`'s six commodity words.
fn hold_words(hold: &NpcHold) -> [u32; NPC_COMMODITIES] {
    std::array::from_fn(|commodity| {
        // In 0..6.
        #[allow(clippy::cast_possible_truncation)]
        hold.tons(commodity as u8)
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Behaviour;
    use crate::ai::pirate::PirateWarship;
    use crate::ai::plunder_contents::{ClassMarines, Marines};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::fleet::Escort;
    use crate::geometry::Vec2;
    use crate::reserves::Reserves;
    use crate::self_repair::FormerEscort;
    use crate::stats::ShipStats;
    use crate::traffic::Traffic;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::MissionMark;
    use crate::traffic::plunder_dock::DockSub;
    use nova_sim_base::fixture::{Draws, FAST};
    use nova_sim_catalog::fixture::weapon;

    const PIRATE: NpcId = NpcId(1);
    const VICTIM: NpcId = NpcId(9);

    /// The pirate, NPC 1 in berth 1 at the centre, armed, holding 20 tons,
    /// of strength `strength`.
    fn pirate(strength: i32) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.berth = 1;
        ship.govt = Some(crate::catalog::GovtId(130));
        ship.armament = Armament::new([(WeaponSpec::new(&weapon(128)), 1)]);
        ship.holds = 20;
        ship.hull.class_turn = 3.0;
        ship.marines = ClassMarines {
            strength,
            ..ClassMarines::default()
        };
        ship
    }

    /// The victim, NPC 9 in berth 9, a disabled trader with 30 tons of
    /// commodity 2 in holds of 50, 2 pixels off the pirate, at rest.
    fn victim() -> Npc {
        let mut ship = npc(9, ShipStats::new(FAST, &[]));
        ship.berth = 9;
        ship.govt = Some(crate::catalog::GovtId(131));
        ship.condition = Condition::Disabled;
        ship.hull.crew = 1;
        ship.hull.inherent_ai = 2;
        ship.holds = 50;
        ship.cargo.add(2, 30);
        ship.reserves = Reserves::full(10.0, 100.0, 100.0);
        ship.reserves.armor.now = 10.0;
        ship.state.position = Vec2::new(2.0, 0.0);
        ship.marines = ClassMarines {
            strength: 10,
            ..ClassMarines::default()
        };
        ship
    }

    fn traffic_of(npcs: Vec<Npc>) -> Traffic {
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = npcs;
        traffic
    }

    /// `traffic` a tick on with pirates, every draw `drawn`.
    fn tick(traffic: &mut Traffic, drawn: u32) {
        traffic.tick_in(
            &PirateWarship::default(),
            World::new(&[]),
            &[],
            &mut Draws::of(&[drawn; 64]),
        );
    }

    fn ship(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("in the system")
    }

    #[test]
    fn a_docking_given_up_clears_the_travel_stellar_and_one_kept_keeps_it() {
        // The plunder's give-up (@0x8e8b3) and the repair's (@0x8f732)
        // write +0x6c -1; a docking set writes the victim's slot.
        let mut docking = pirate(1000);
        docking.travel = Some(crate::catalog::StellarId(150));
        let mut kept = docking.clone();
        take_high_level(
            &mut docking,
            HighLevel::Docks {
                ship: ShipRef::Npc(VICTIM),
                step: DockStep::GiveUp,
                slot: None,
            },
        );
        assert!(!docking.travel_set());
        take_high_level(
            &mut kept,
            HighLevel::Docks {
                ship: ShipRef::Npc(VICTIM),
                step: DockStep::Sub(DockSub::Close),
                slot: Some(crate::ai::travel::of_slot(4)),
            },
        );
        assert_eq!(kept.travel, Some(crate::ai::travel::of_slot(4)));
    }

    #[test]
    fn a_pirate_docks_with_its_disabled_victim_boarding_it_and_waiting_its_dock() {
        let mut traffic = traffic_of(vec![pirate(100), victim()]);
        tick(&mut traffic, 0);
        let docked = ship(&traffic, PIRATE);
        assert_eq!(
            docked.docking,
            Some(Docking {
                ship: ShipRef::Npc(VICTIM),
                sub: DockSub::Dock
            })
        );
        assert_eq!(docked.ai_wait, 99, "Rand(80) of 0 + 100, a tick off");
        assert!(ship(&traffic, VICTIM).boarded);
        assert_eq!(traffic.take_pirate_events(), [], "no dock with the player");
    }

    #[test]
    fn a_dampened_pirate_docks_within_four_times_its_box_only_when_the_worlds_rule_counts_its_dampener()
     {
        // Class turn 3: a box of 210, 840 for an inertialess ship. The
        // pirate's high level reads the world's flight rule.
        let sub = |rule: InertialDampener| {
            let mut dampened = pirate(100);
            dampened.stats.dampener = true;
            assert!(!dampened.stats.inertialess_hull);
            let mut far = victim();
            far.state.position = Vec2::new(840.0, 0.0);
            let mut traffic = traffic_of(vec![dampened, far]);
            let world = World {
                flight: FlightRules {
                    inertial_dampener: rule,
                    ..FlightRules::default()
                },
                ..World::new(&[])
            };
            traffic.tick_in(
                &PirateWarship::default(),
                world,
                &[],
                &mut Draws::of(&[0; 64]),
            );
            ship(&traffic, PIRATE).docking.map(|docking| docking.sub)
        };
        assert_eq!(sub(InertialDampener::AnyShip), Some(DockSub::Dock));
        assert_eq!(
            sub(InertialDampener::PlayerOnly),
            Some(DockSub::Approach),
            "beyond twice 210"
        );
    }

    #[test]
    fn the_plunder_comes_on_the_dock_waits_last_tick_and_takes_the_cargo() {
        let mut traffic = traffic_of(vec![pirate(1), victim()]);
        tick(&mut traffic, 0);
        for _ in 1..100 {
            tick(&mut traffic, 0);
            assert_eq!(
                ship(&traffic, PIRATE).goal,
                Goal::Attack(ShipRef::Npc(VICTIM))
            );
        }
        assert_eq!(ship(&traffic, PIRATE).ai_wait, 0);
        tick(&mut traffic, 0);
        let pirate = ship(&traffic, PIRATE);
        assert_eq!(pirate.goal, Goal::Drift, "the 100th tick after the start");
        assert_eq!(pirate.ai_wait, PLUNDER_HOLD - 1);
        assert_eq!((pirate.target, pirate.docking), (None, None));
        assert_eq!(pirate.cargo.tons(2), 20, "its free holds");
        assert_eq!(ship(&traffic, VICTIM).cargo.tons(2), 10);
        assert_eq!(
            traffic.take_pirate_events(),
            [PirateEvent::Plundered {
                pirate: PIRATE,
                victim: VICTIM,
                tons: 20,
                line: None,
                taken: None,
            }],
            "a stranger: no line; a ratio of 5, odds of 15: no capture"
        );
    }

    /// The traffic the tick after a plunder at `strength`, the capture
    /// draw `drawn`, of `victim`.
    fn plundered(strength: i32, victim: Npc, drawn: u32) -> Traffic {
        let mut docked = pirate(strength);
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = victim;
        boarded.boarded = true;
        let mut traffic = traffic_of(vec![docked, boarded]);
        tick(&mut traffic, drawn);
        traffic
    }

    #[test]
    fn without_a_capture_the_pirate_drifts_101_ticks_and_then_searches() {
        let mut traffic = plundered(1, victim(), 0);
        let start = ship(&traffic, PIRATE).state.position;
        assert_eq!(start, Vec2::new(0.0, -DRIFT), "the plunder's tick drifts");
        for tick_after in 1..=100 {
            tick(&mut traffic, 0);
            let pirate = ship(&traffic, PIRATE);
            let expected = if tick_after == 100 {
                Goal::Idle
            } else {
                Goal::Drift
            };
            assert_eq!(pirate.goal, expected, "{tick_after}");
        }
        let drifted = ship(&traffic, PIRATE).state.position;
        assert!((drifted.y + 101.0 * DRIFT).abs() < 1e-3, "{drifted:?}");
        tick(&mut traffic, 0);
        assert_eq!(
            ship(&traffic, PIRATE).goal,
            Goal::JumpOut,
            "the 101st searches, passing over the victim, which stays boarded"
        );
        assert!(ship(&traffic, VICTIM).boarded);
    }

    /// A former hired escort of the player's.
    fn former() -> Npc {
        let mut ship = victim();
        ship.former = Some(FormerEscort {
            record: Escort {
                ship: ShipId(128),
                reserves: Reserves::full(1.0, 1.0, 1.0),
                order: None,
                carried: false,
                wage: Some(10),
                person: None,
                mark: None,
            },
        });
        ship
    }

    #[test]
    fn a_defence_ship_a_pirate_takes_into_its_fleet_defends_no_stellar() {
        let mut taken = former();
        taken.defends = Some(crate::catalog::StellarId(128));
        let mut docked = pirate(1000);
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        taken.boarded = true;
        let mut traffic = traffic_of(vec![docked, taken]);
        tick(&mut traffic, 0);
        assert_eq!(ship(&traffic, VICTIM).leader, Some(PIRATE));
        assert_eq!(ship(&traffic, VICTIM).defends, None, "see phase 16");
    }

    /// The traffic once a docked pirate has taken a former escort into
    /// its fleet, a tick on.
    fn captured() -> Traffic {
        let mut docked = pirate(1000);
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = former();
        boarded.boarded = true;
        let mut traffic = traffic_of(vec![docked, boarded]);
        tick(&mut traffic, 0);
        assert_eq!(ship(&traffic, VICTIM).leader, Some(PIRATE));
        assert_eq!(ship(&traffic, VICTIM).ai_wait, CAPTURED_WAIT - 1);
        traffic
    }

    #[test]
    fn a_captured_ships_wait_ends_at_its_first_think_as_a_fleet_escort() {
        // Its first think as an AI type 6 escort with order -1 runs the
        // formation block, which sets +0x4c = -1.0 (@0x83ea8-0x83eb0).
        let mut traffic = captured();
        traffic.tick_in(
            &crate::ai::NovaAi::default(),
            World::new(&[]),
            &[],
            &mut Draws::of(&[0; 64]),
        );
        let taken = ship(&traffic, VICTIM);
        assert_eq!(taken.ai_wait, 0);
        assert_eq!(
            taken.goal,
            Goal::Follow {
                lead: PIRATE,
                guard: None
            }
        );
        // By the other reading of `npc_escort_ai`, its 150 ticks run out.
        let mut traffic = captured();
        let bible = crate::rulebook::Rulebook::default().with_override(
            crate::rulebook::RuleKey::NpcEscortAi,
            crate::rulebook::RuleSource::Bible,
        );
        traffic.tick_in(
            &crate::ai::NovaAi::from_rulebook(&bible),
            World::new(&[]),
            &[],
            &mut Draws::of(&[0; 64]),
        );
        assert_eq!(ship(&traffic, VICTIM).ai_wait, CAPTURED_WAIT - 2);
    }

    /// The pirate thinks as one, and every other ship as Nova's AI has it.
    #[derive(Debug, Default)]
    struct AmongNova {
        pirate: PirateWarship,
        nova: crate::ai::NovaAi,
    }

    impl Behaviour for AmongNova {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            if npc.id == PIRATE {
                self.pirate.decision(npc, around, chance)
            } else {
                self.nova.decision(npc, around, chance)
            }
        }
    }

    /// A tick of a docked pirate at `berth` taking a former escort at
    /// berth 9, every other ship as Nova's AI has it: the ship it took.
    fn taken_on_the_tick(berth: u32) -> Npc {
        let mut docked = pirate(1000);
        docked.berth = berth;
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = former();
        boarded.boarded = true;
        let mut traffic = traffic_of(vec![docked, boarded]);
        traffic.tick_in(
            &AmongNova::default(),
            World::new(&[]),
            &[],
            &mut Draws::of(&[0; 64]),
        );
        ship(&traffic, VICTIM).clone()
    }

    #[test]
    fn a_ship_taken_above_its_pirate_thinks_as_its_escort_on_the_tick_it_is_taken() {
        // Its slot comes after the pirate's: its `_EscortAI` runs on the
        // capture's words that frame, and its formation block ends the
        // wait (@0x83ea8-0x83eb0).
        let taken = taken_on_the_tick(1);
        assert_eq!(taken.leader, Some(PIRATE));
        assert_eq!(taken.ai_wait, 0);
        assert_eq!(
            taken.goal,
            Goal::Follow {
                lead: PIRATE,
                guard: None
            }
        );
    }

    #[test]
    fn a_ship_taken_below_its_pirate_first_thinks_the_next_tick() {
        // Its slot came before the pirate's, disabled.
        let taken = taken_on_the_tick(12);
        assert_eq!(taken.leader, Some(PIRATE));
        assert_eq!(taken.ai_wait, CAPTURED_WAIT - 1);
        assert_eq!(taken.goal, Goal::Idle);
    }

    #[test]
    fn a_ship_below_the_pirate_thinks_on_and_the_capture_then_drops_its_target() {
        // Its `_AIDispatch` came first, keeping the attack; the walk
        // (@0x84872-0x8489b) then zeroes what it kept.
        let mut onlooker = npc(5, ShipStats::new(FAST, &[]));
        onlooker.berth = 5;
        onlooker.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        onlooker.target = Some(ShipRef::Npc(VICTIM));
        onlooker.provoked = 3.0;
        onlooker.armament = Armament::new([(WeaponSpec::new(&weapon(128)), 1)]);
        let mut docked = pirate(1000);
        docked.berth = 7;
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = former();
        boarded.boarded = true;
        let mut traffic = traffic_of(vec![docked, boarded, onlooker]);
        tick(&mut traffic, 0);
        assert_eq!(ship(&traffic, VICTIM).leader, Some(PIRATE));
        let onlooker = ship(&traffic, NpcId(5));
        assert_eq!((onlooker.goal, onlooker.target), (Goal::Idle, None));
        assert_eq!(onlooker.provoked, 0.0);
    }

    #[test]
    fn a_bribed_ship_taken_by_a_pirate_reads_as_its_escort_not_its_bribe() {
        // `_AIPlunderShipContents` writes +0x88 = 6 (@0x8495c) over the
        // bribe's 1, the bribe's only word (`_DoCommDialog` @0x961f8).
        let mut bribed = victim();
        bribed.spared = true;
        let traffic = plundered(1000, bribed, 0);
        let taken = ship(&traffic, VICTIM);
        assert_eq!(taken.leader, Some(PIRATE));
        assert!(!taken.spared, "the bribe is overwritten");
        assert_eq!(taken.original_ai_type(), 6);
        assert!(crate::hail::fixture::hail(taken).escort_ai());
    }

    #[test]
    fn a_captured_former_escort_joins_the_pirates_fleet_and_every_ship_targeting_it_drops_it() {
        let mut onlooker = npc(5, ShipStats::new(FAST, &[]));
        onlooker.berth = 5;
        onlooker.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        onlooker.target = Some(ShipRef::Npc(VICTIM));
        onlooker.provoked = 3.0;
        // Disabled, so it doesn't think: only the drop writes its +0x6c.
        let mut wrecked = npc(6, ShipStats::new(FAST, &[]));
        wrecked.berth = 6;
        wrecked.condition = Condition::Disabled;
        wrecked.target = Some(ShipRef::Npc(VICTIM));
        wrecked.travel = Some(crate::catalog::StellarId(150));
        let mut docked = pirate(1000);
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        docked.skill_mult = 2.5;
        let mut boarded = former();
        boarded.boarded = true;
        boarded.skill = 0.75;
        let mut traffic = traffic_of(vec![docked, boarded, onlooker, wrecked]);
        traffic.set_berth_person(9, "Someone");
        tick(&mut traffic, 0);
        let taken = ship(&traffic, VICTIM);
        assert_eq!(taken.leader, Some(PIRATE));
        assert_eq!(taken.govt, Some(crate::catalog::GovtId(130)));
        assert_eq!(
            (taken.skill_mult, taken.skill),
            (2.5, 0.75),
            "the pirate's factor, its own skill"
        );
        assert_eq!(
            (taken.person, taken.former, taken.boarded),
            (None, None, false)
        );
        // A higher slot's `_AIDispatch` comes after the capture's writes
        // (goal and sub 0, @0x8497c-0x84985), and the loop rereads its
        // +0x88, now 6 (@0x8495c, @0x4427e), so it thinks as led that frame.
        assert_eq!(
            taken.goal,
            Goal::Follow {
                lead: PIRATE,
                guard: None
            }
        );
        assert_eq!(taken.ai_wait, CAPTURED_WAIT - 1);
        assert_eq!(taken.reserves.armor.now, 66.0);
        assert_eq!(taken.reserves.shield.now, 0.0);
        assert_eq!(taken.condition, Condition::Intact);
        assert_eq!(traffic.berth_person(9), None);
        let pirate = ship(&traffic, PIRATE);
        assert_eq!(pirate.goal, Goal::Idle, "dropped with the rest: no drift");
        assert_eq!(pirate.ai_wait, PLUNDER_HOLD - 1);
        // The capture's walk (@0x84872-0x8489b) runs in the pirate's think,
        // before the onlooker's in a higher slot, which starts from goal 0
        // with no target and nothing to hunt, and leaves.
        let onlooker = ship(&traffic, NpcId(5));
        assert_eq!((onlooker.goal, onlooker.target), (Goal::JumpOut, None));
        assert_eq!(onlooker.provoked, 0.0);
        let wrecked = ship(&traffic, NpcId(6));
        assert_eq!(wrecked.target, None);
        assert!(!wrecked.travel_set(), "+0x6c -1, @0x8487a");
        assert!(matches!(
            traffic.take_pirate_events()[..],
            [PirateEvent::Plundered {
                taken: Some(Taken::Escort),
                line: Some(CargoLine {
                    tons: 20,
                    credits: 0
                }),
                ..
            }]
        ));
    }

    #[test]
    fn an_announced_capture_drops_only_the_ships_targeting_the_victim() {
        let mut busy = npc(6, ShipStats::new(FAST, &[]));
        busy.berth = 6;
        busy.goal = Goal::Attack(ShipRef::Player);
        busy.target = Some(ShipRef::Player);
        // With no AI, so no think of its own rewrites its target.
        busy.no_ai = true;
        let mut docked = pirate(1000);
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = former();
        boarded.boarded = true;
        let mut traffic = traffic_of(vec![docked, boarded, busy]);
        tick(&mut traffic, 0);
        assert_eq!(ship(&traffic, VICTIM).leader, Some(PIRATE));
        assert_eq!(
            ship(&traffic, NpcId(6)).target,
            Some(ShipRef::Player),
            "it targets another ship"
        );
    }

    #[test]
    fn a_captured_ship_with_no_armour_capacity_is_left_intact() {
        #[derive(Debug)]
        struct AlwaysDisabled;
        impl crate::combat::hull::DisableRule for AlwaysDisabled {
            fn disabled(
                &self,
                _armor: crate::reserves::Gauge,
                _hull: &crate::combat::hull::HullSpec,
            ) -> bool {
                true
            }
        }
        let mut docked = pirate(1000);
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = victim();
        boarded.boarded = true;
        boarded.reserves.armor = crate::reserves::Gauge { now: 0.0, max: 0.0 };
        let mut traffic = traffic_of(vec![docked, boarded]);
        let world = World {
            disable: &AlwaysDisabled,
            ..World::new(&[])
        };
        traffic.tick_in(
            &PirateWarship::default(),
            world,
            &[],
            &mut Draws::of(&[0; 64]),
        );
        assert_eq!(
            ship(&traffic, VICTIM).condition,
            Condition::Intact,
            "the rule isn't asked"
        );
    }

    #[test]
    fn a_docking_flies_only_an_intact_flying_ship_at_its_own_top_speed() {
        let victim = ShipState::default();
        let mut far = pirate(1);
        far.state.position = Vec2::new(0.0, 1000.0);
        // Held in a tractor beam, its own top speed is a third of its
        // class's: at it, `_AdjustedAccel` adds no push to an axis already
        // at its share.
        let holder = ShipRef::Npc(NpcId(2));
        far.tractor = Some(crate::combat::impact::TractorLock::by(holder));
        let top = far.top_speed(1.0);
        assert!(top < far.class_handling().max_speed, "held");
        far.state.velocity = Vec2::new(0.0, -top);
        far.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Approach,
        });
        let mut flown = far.clone();
        assert_eq!(
            fly_docking(&mut flown, Some(&victim), 1.0, FlightRules::default(), None),
            Some(DockOutcome::Flying)
        );
        assert_eq!(
            flown.state.velocity.length(),
            top,
            "held at its own top speed"
        );
        for (condition, mode) in [
            (Condition::Disabled, crate::traffic::npc::Mode::Flying),
            (
                Condition::Intact,
                crate::traffic::npc::Mode::Coasting { ticks_left: 3 },
            ),
        ] {
            let mut down = far.clone();
            down.condition = condition;
            down.mode = mode;
            assert_eq!(
                fly_docking(&mut down, Some(&victim), 1.0, FlightRules::default(), None),
                None,
                "{condition:?} {mode:?}"
            );
        }
        let mut lost = far.clone();
        assert_eq!(
            fly_docking(&mut lost, None, 1.0, FlightRules::default(), None),
            None
        );
        assert_eq!(lost.docking, None, "its victim gone");
    }

    #[test]
    fn a_plunder_whose_victims_slot_went_docks_with_the_record() {
        // Its dock wait skips the high level (@0x8d4b9-0x8d4d2), so goal
        // 0xd keeps sub 0xf and its nav, which no slot test clears: it
        // matches the record's velocity and creeps to its place
        // (@0x86a3f-0x86a51, @0x8702d). Its think keeps its goal here; a
        // pirate's own think on the gone victim is
        // `plunder_gone_tests`'s.
        #[derive(Debug)]
        struct Keeps;
        impl crate::ai::Behaviour for Keeps {
            fn decide(
                &self,
                npc: &Npc,
                _: &Surroundings,
                _: &mut dyn crate::chance::Chance,
            ) -> Goal {
                npc.goal
            }
        }
        let mut docked = pirate(1);
        docked.goal = Goal::Plunder {
            ship: ShipRef::Npc(VICTIM),
            hopeless: false,
        };
        docked.ai_wait = 150;
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut drifting = victim();
        drifting.state.position = Vec2::new(10.0, 0.0);
        drifting.state.velocity = Vec2::new(0.25, 0.0);
        docked.state.velocity = Vec2::new(0.25, 0.0);
        let mut traffic = traffic_of(vec![docked, drifting]);
        traffic.remove_destroyed(VICTIM);
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut Draws::of(&[0; 8]));
        let pirate = ship(&traffic, PIRATE);
        assert_eq!(
            pirate.docking,
            Some(Docking {
                ship: ShipRef::Npc(VICTIM),
                sub: DockSub::Dock
            }),
            "kept on the record"
        );
        assert_eq!(pirate.state.velocity, Vec2::new(0.25, 0.0));
        assert!(pirate.state.position.x > 0.25, "crept toward it");
    }

    /// A pirate plundering the victim with no AI wait, closing on it (sub
    /// 0xb) from (0, 600), its travel stellar the victim's slot.
    fn closing_plunder() -> Npc {
        let mut closing = pirate(1);
        closing.goal = Goal::Plunder {
            ship: ShipRef::Npc(VICTIM),
            hopeless: false,
        };
        closing.target = Some(ShipRef::Npc(VICTIM));
        closing.travel = Some(crate::ai::travel::of_slot(9));
        closing.state.position = Vec2::new(0.0, 600.0);
        closing.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Close,
        });
        closing
    }

    #[test]
    fn a_plunder_whose_ship_has_gone_takes_goal_0_keeping_its_docking_and_nav() {
        // The slot test makes +0x70 -1 for a slot gone (@0x8d4e4-0x8d50d),
        // and goal 0xd's block then writes goal 0 alone (@0x8e74e-0x8e761
        // → @0x8eb5f): its sub and nav are kept for that frame's low level.
        // The goal's ship stands in for +0x70, so the target the traffic
        // holds isn't read.
        let world = World::new(&[]);
        for target in [Some(ShipRef::Npc(VICTIM)), None] {
            let mut plunder = closing_plunder();
            plunder.target = target;
            let npcs = [plunder];
            let high = high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly);
            assert_eq!(high, HighLevel::LetsGo, "{target:?}");
            let mut taken = npcs[0].clone();
            take_high_level(&mut taken, high);
            assert_eq!(taken.goal, Goal::Idle, "{target:?}");
            assert_eq!(taken.docking, npcs[0].docking, "{target:?}");
            assert_eq!(taken.travel, npcs[0].travel, "{target:?}");
        }
    }

    #[test]
    fn a_plunder_of_a_disabled_ship_still_there_docks_though_the_traffic_holds_no_target() {
        // Only the slot test clears +0x70 under goal 0xd, and only for a
        // slot gone; a fresh plunder's target is taken after the high
        // level, so the goal's ship there is +0x70 set: it docks.
        let mut plunder = closing_plunder();
        plunder.target = None;
        let npcs = [plunder, victim()];
        let world = World::new(&[]);
        let high = high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly);
        assert!(
            matches!(
                high,
                HighLevel::Docks {
                    ship: ShipRef::Npc(VICTIM),
                    step: DockStep::Sub(_),
                    ..
                }
            ),
            "{high:?}"
        );
        let mut taken = npcs[0].clone();
        take_high_level(&mut taken, high);
        assert_eq!(taken.goal, npcs[0].goal, "no goal 0");
    }

    #[test]
    fn a_plunder_whose_victims_slot_went_steers_one_tick_at_the_record_then_drops_its_dock() {
        // Sub 0xb steers at +0x6c with +0x70 -1 (@0x87345-0x87351) the
        // tick goal 0xd lets go; the next tick's goal 0 drops it.
        #[derive(Debug)]
        struct Keeps;
        impl crate::ai::Behaviour for Keeps {
            fn decide(
                &self,
                npc: &Npc,
                _: &Surroundings,
                _: &mut dyn crate::chance::Chance,
            ) -> Goal {
                npc.goal
            }
        }
        let record = victim();
        let mut traffic = traffic_of(vec![closing_plunder(), record.clone()]);
        traffic.remove_destroyed(VICTIM);
        let mut flown = closing_plunder();
        fly_docking(
            &mut flown,
            Some(&record.state),
            1.0,
            FlightRules::default(),
            None,
        );
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut Draws::of(&[0; 8]));
        let pirate = ship(&traffic, PIRATE);
        assert_eq!(pirate.goal, Goal::Idle);
        assert_eq!(pirate.docking, closing_plunder().docking, "kept");
        assert_eq!(pirate.state, flown.state, "it closed on the record");
        traffic.tick_in(&Keeps, World::new(&[]), &[], &mut Draws::of(&[0; 8]));
        assert_eq!(ship(&traffic, PIRATE).docking, None, "goal 0 drops it");
    }

    #[test]
    fn a_docked_pirate_holds_with_its_victim_whatever_ship_it_targets() {
        // Waiting its dock, its goal an attack on another ship: it moves
        // with the victim it docked with.
        let mut docked = pirate(1);
        docked.goal = Goal::Attack(ShipRef::Npc(NpcId(5)));
        docked.target = Some(ShipRef::Npc(NpcId(5)));
        docked.ai_wait = 50;
        docked.state.position = Vec2::new(2.0, 0.0);
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut drifting = victim();
        drifting.state.position = Vec2::ZERO;
        drifting.state.velocity = Vec2::new(0.25, 0.0);
        let mut other = npc(5, ShipStats::new(FAST, &[]));
        other.berth = 5;
        other.state.position = Vec2::new(0.0, 500.0);
        other.state.velocity = Vec2::new(0.0, 2.0);
        let mut traffic = traffic_of(vec![docked, drifting, other]);
        // Its low level at its slot (`dispatch_dock`), then its flight.
        traffic.dock_in_dispatch(0, &World::new(&[]), &mut Draws::of(&[]));
        let still = std::collections::BTreeSet::new();
        traffic.fly_holding(World::new(&[]), &still, &mut Draws::of(&[]));
        assert_eq!(ship(&traffic, PIRATE).state.velocity, Vec2::new(0.25, 0.0));
    }

    #[test]
    fn a_repairs_high_level_is_skipped_while_its_wait_runs_and_docks_after() {
        let mut helper = npc(1, ShipStats::new(FAST, &[]));
        helper.goal = Goal::Assist(Help::Repair);
        helper.target = Some(ShipRef::Player);
        helper.ai_wait = 5;
        let downed = crate::ai::PlayerSide {
            condition: Condition::Disabled,
            ..crate::ai::fixture::player(2.0, 0.0)
        };
        let world = World {
            player: Some(downed),
            ..World::new(&[])
        };
        let npcs = [helper.clone()];
        assert_eq!(
            high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly,),
            HighLevel::Skipped
        );
        helper.ai_wait = 0;
        let npcs = [helper.clone()];
        assert_eq!(
            high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly,),
            HighLevel::Docks {
                ship: ShipRef::Player,
                step: DockStep::Sub(DockSub::Dock),
                slot: Some(crate::ai::travel::of_slot(0)),
            }
        );
        helper.target = None;
        let npcs = [helper];
        assert_eq!(
            high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly,),
            HighLevel::Skipped,
            "no target: nothing changes"
        );
    }

    #[test]
    fn a_repairer_holds_its_targets_slot() {
        // Goal 0xf, its target there and disabled: +0x6c = +0x70 (@0x8f74c).
        use crate::ai::travel;
        let mut helper = npc(1, ShipStats::new(FAST, &[]));
        helper.goal = Goal::Assist(Help::Repair);
        let mut wreck = npc(2, ShipStats::new(FAST, &[]));
        wreck.berth = 6;
        wreck.condition = Condition::Disabled;
        wreck.state.position = Vec2::new(2.0, 0.0);
        let downed = crate::ai::PlayerSide {
            condition: Condition::Disabled,
            ..crate::ai::fixture::player(2.0, 0.0)
        };
        let world = World {
            player: Some(downed),
            ..World::new(&[])
        };
        for (target, slot) in [(ShipRef::Player, 0), (ShipRef::Npc(wreck.id), 6)] {
            helper.target = Some(target);
            let npcs = [helper.clone(), wreck.clone()];
            let high = high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly);
            assert!(matches!(high, HighLevel::Docks { .. }), "{high:?}");
            let mut repairer = helper.clone();
            take_high_level(&mut repairer, high);
            assert_eq!(repairer.travel, Some(travel::of_slot(slot)), "{target:?}");
        }
    }

    #[test]
    fn the_repair_high_level_takes_the_dampener_rule() {
        let mut helper = npc(1, ShipStats::new(FAST, &[]));
        helper.goal = Goal::Assist(Help::Repair);
        helper.target = Some(ShipRef::Player);
        helper.hull.class_turn = 3.0;
        helper.state.position = Vec2::new(1040.0, 0.0);
        helper.stats.dampener = true;
        let downed = crate::ai::PlayerSide {
            condition: Condition::Disabled,
            ..crate::ai::fixture::player(0.0, 0.0)
        };
        let world = World {
            player: Some(downed),
            ..World::new(&[])
        };
        let npcs = [helper];
        let docks = |sub| HighLevel::Docks {
            ship: ShipRef::Player,
            step: DockStep::Sub(sub),
            slot: Some(crate::ai::travel::of_slot(0)),
        };
        assert_eq!(
            high_level(&npcs[0], &world.around(&npcs), InertialDampener::AnyShip),
            docks(DockSub::Dock)
        );
        assert_eq!(
            high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly),
            docks(DockSub::Close)
        );
    }

    #[test]
    fn a_refuels_high_level_is_skipped_while_its_wait_runs_or_with_no_target_and_refuels_after() {
        let mut helper = npc(1, ShipStats::new(FAST, &[]));
        helper.goal = Goal::Assist(Help::Refuel);
        helper.target = Some(ShipRef::Player);
        helper.ai_wait = 5;
        helper.docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Close,
        });
        let world = World {
            player: Some(crate::ai::fixture::player(2.0, 0.0)),
            ..World::new(&[])
        };
        let high = |helper: &Npc| {
            let npcs = [helper.clone()];
            high_level(&npcs[0], &world.around(&npcs), InertialDampener::PlayerOnly)
        };
        assert_eq!(high(&helper), HighLevel::Skipped);
        helper.ai_wait = 0;
        assert_eq!(high(&helper), HighLevel::Refuels);
        let mut kept = helper.clone();
        take_high_level(&mut kept, HighLevel::Refuels);
        assert_eq!(kept.docking, helper.docking, "the refuel pass sets it");
        helper.target = None;
        assert_eq!(high(&helper), HighLevel::Skipped, "no target, @0x8f3f3");
    }

    #[test]
    fn a_queued_event_is_taken_once() {
        let mut traffic = traffic_of(vec![]);
        let event = PirateEvent::PlunderPlayer { pirate: PIRATE };
        traffic.queue_pirate_event(event);
        assert_eq!(traffic.take_pirate_events(), [event]);
        assert_eq!(traffic.take_pirate_events(), []);
    }

    #[test]
    fn a_stranger_is_taken_silently_and_the_pirate_drifts() {
        let traffic = plundered(1000, victim(), 0);
        assert_eq!(ship(&traffic, VICTIM).leader, Some(PIRATE));
        assert_eq!(ship(&traffic, PIRATE).goal, Goal::Drift);
        assert!(matches!(
            traffic.clone().take_pirate_events()[..],
            [PirateEvent::Plundered {
                taken: Some(Taken::Silently),
                line: None,
                ..
            }]
        ));
    }

    #[test]
    fn a_mission_ship_loses_its_cargo_and_is_never_captured() {
        let mut marked = former();
        marked.mission = Some(MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::Own,
            formation: None,
            order: None,
        });
        let mut traffic = plundered(1000, marked, 0);
        assert_eq!(ship(&traffic, VICTIM).leader, None);
        assert_eq!(ship(&traffic, VICTIM).cargo.tons(2), 10);
        assert!(matches!(
            traffic.take_pirate_events()[..],
            [PirateEvent::Plundered {
                taken: None,
                line: None,
                ..
            }]
        ));
    }

    #[test]
    fn a_captured_ship_is_left_disabled_when_the_worlds_rule_says_so() {
        #[derive(Debug)]
        struct AlwaysDisabled;
        impl crate::combat::hull::DisableRule for AlwaysDisabled {
            fn disabled(
                &self,
                _armor: crate::reserves::Gauge,
                _hull: &crate::combat::hull::HullSpec,
            ) -> bool {
                true
            }
        }
        let mut docked = pirate(1000);
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = victim();
        boarded.boarded = true;
        let mut traffic = traffic_of(vec![docked, boarded]);
        let world = World {
            disable: &AlwaysDisabled,
            ..World::new(&[])
        };
        traffic.tick_in(
            &PirateWarship::default(),
            world,
            &[],
            &mut Draws::of(&[0; 64]),
        );
        assert_eq!(ship(&traffic, VICTIM).condition, Condition::Disabled);
    }

    #[test]
    fn a_dock_with_the_player_boards_it_through_the_session() {
        let mut traffic = traffic_of(vec![pirate(100)]);
        let mut downed = crate::ai::PlayerSide {
            condition: Condition::Disabled,
            ..crate::ai::fixture::player(2.0, 0.0)
        };
        downed.hull.crew = 1;
        let world = World {
            player: Some(downed),
            ..World::new(&[])
        };
        traffic.tick_in(
            &PirateWarship::default(),
            world,
            &[],
            &mut Draws::of(&[0; 64]),
        );
        assert_eq!(
            traffic.take_pirate_events(),
            [PirateEvent::PlayerDockedWith { pirate: PIRATE }]
        );
        let docked_on = World {
            player: Some(crate::ai::PlayerSide {
                boarded: true,
                ..downed
            }),
            ..World::new(&[])
        };
        for _ in 1..100 {
            traffic.tick_in(
                &PirateWarship::default(),
                docked_on,
                &[],
                &mut Draws::of(&[0; 64]),
            );
        }
        traffic.tick_in(
            &PirateWarship::default(),
            docked_on,
            &[],
            &mut Draws::of(&[0; 64]),
        );
        assert_eq!(
            traffic.take_pirate_events(),
            [PirateEvent::PlunderPlayer { pirate: PIRATE }]
        );
        assert_eq!(ship(&traffic, PIRATE).goal, Goal::Drift);
    }

    #[test]
    fn each_events_actor_is_its_pirate_or_repairer() {
        let helper = NpcId(4);
        assert_eq!(
            PirateEvent::PlayerDockedWith { pirate: PIRATE }.actor(),
            PIRATE
        );
        assert_eq!(
            PirateEvent::PlunderPlayer { pirate: PIRATE }.actor(),
            PIRATE
        );
        let plundered = PirateEvent::Plundered {
            pirate: PIRATE,
            victim: VICTIM,
            tons: 0,
            line: None,
            taken: None,
        };
        assert_eq!(plundered.actor(), PIRATE);
        let repaired = PirateEvent::PlayerRepaired {
            helper,
            ship: ShipId(128),
        };
        assert_eq!(repaired.actor(), helper);
    }

    #[test]
    fn a_plunder_is_queued_at_its_pirates_berth_among_the_calls() {
        use crate::ai::CommCall;
        use crate::traffic::on_line::OnLine;
        let mut docked = pirate(1);
        docked.berth = 3;
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = victim();
        boarded.boarded = true;
        let mut traffic = traffic_of(vec![docked, boarded]);
        tick(&mut traffic, 0);
        let call = |id| OnLine::Call(NpcId(id), CommCall::Distress, 0.0);
        traffic.on_line.push(2, call(2));
        traffic.on_line.push(4, call(4));
        let work = traffic.take_on_line();
        assert_eq!(work.len(), 3, "{work:?}");
        assert_eq!(work[0], call(2));
        assert!(
            matches!(
                work[1],
                OnLine::Pirate(PirateEvent::Plundered { pirate: PIRATE, .. })
            ),
            "{work:?}"
        );
        assert_eq!(work[2], call(4));
    }

    #[test]
    fn double_times_second_pass_counts_the_ai_wait_down_and_drifts_nothing() {
        let mut waiting = pirate(100);
        waiting.goal = Goal::Drift;
        waiting.ai_wait = 5;
        let mut traffic = traffic_of(vec![waiting]);
        traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
        let moved = ship(&traffic, PIRATE);
        assert_eq!(moved.ai_wait, 4);
        assert_eq!(moved.state.position, Vec2::ZERO, "no high level");
    }

    #[test]
    fn the_fights_countdown_leaves_a_pods_adrift_wait_alone() {
        // `_HandleShip`'s countdown skips class 0x2ff (@0x3502f): a pod's
        // wait is its own flight's (`pod_flight::count_down`).
        use crate::traffic::npc::Mode;
        let mut pod = npc(0, ShipStats::new(FAST, &[]));
        pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        pod.mode = Mode::Adrift { ticks_left: 3 };
        let mut npcs = [pod];
        count_down(&mut npcs);
        assert_eq!(npcs[0].mode, Mode::Adrift { ticks_left: 3 });
    }

    #[test]
    fn the_strengths_marines_follow_the_worlds_reading() {
        // By the place, the pirate's crew of 500 gives odds of 100 and a
        // capture; by its own outfit, none: odds of 15.
        let mut strong = pirate(1);
        strong.marines.by_place = Marines {
            crew: 500,
            defence: 0,
        };
        let mut docked = strong;
        docked.goal = Goal::Attack(ShipRef::Npc(VICTIM));
        docked.target = Some(ShipRef::Npc(VICTIM));
        docked.docking = Some(Docking {
            ship: ShipRef::Npc(VICTIM),
            sub: DockSub::Dock,
        });
        let mut boarded = victim();
        boarded.boarded = true;
        for (reading, captured) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut traffic = traffic_of(vec![docked.clone(), boarded.clone()]);
            let world = World {
                plunder_marines: reading,
                ..World::new(&[])
            };
            traffic.tick_in(
                &PirateWarship::default(),
                world,
                &[],
                &mut Draws::of(&[0; 64]),
            );
            assert_eq!(
                ship(&traffic, VICTIM).leader.is_some(),
                captured,
                "{reading:?}"
            );
        }
    }
}
