//! NPC traffic: the ships that come and go in the player's system.
//!
//! - [`table`]: the [`SpawnTable`] a system's traffic is drawn from, its
//!   `DudeTypes` and `LinkSyst`s decoded and its ship types' stats
//!   resolved when the system is entered.
//! - [`spawn`]: the rolls: the initial population, the arrivals over time,
//!   the fleets and the persons (see [`person`](crate::person)), and
//!   where each ship starts.
//! - [`destination`]: the stellars an NPC picks a destination among,
//!   for the idle pick and the emerge pick.
//! - [`emerge`]: the hypergate or wormhole an arrival comes out of, and
//!   how long it waits there.
//! - [`gate_entry`]: an NPC entering a hypergate or wormhole to leave the
//!   system, and the release of the ships it leads.
//! - [`strength`]: the escort-strength timer, and the fleet it brings
//!   for a player whose escorts carry a lot of cargo.
//! - [`npc`]: an [`Npc`]: its ship, government, AI type, escort class
//!   and duty as the player's escort, stats, reserves, flight state and
//!   goal, its condition, armament and fire command, its provocation,
//!   aggression and the ship it last inspected, and the carrier that
//!   launched it from a fighter bay, if any, and the person flying it,
//!   if any ([`NpcPerson`](npc::NpcPerson)).
//! - [`autopilot`]: flying an NPC's goal each tick with the player's
//!   flight physics.
//!
//! [`Traffic`] holds a system's NPCs. Entering a system
//! ([`Traffic::enter_in`]) replaces them with its initial population, its
//! persons drawn in the [`World`]'s persons' world: the persons gone for
//! good never come, and one holding a grudge comes with it. A person
//! flies its fitted ship ([`Traffic::person`]). Each
//! tick ([`Traffic::tick_in`]) in its [`World`] (the stellars, the player,
//! the governments and the player's legal record there), in order:
//!
//! 1. the arrivals pass ([`Traffic::enter_more_ships`], which a gate
//!    exit's session also runs once on its own before the first frame):
//!    the arrival roll, while fewer NPCs than `AvgShips` that are not the
//!    player's ([`Npc::is_players`]) nor its mission escorts are in the
//!    system, each arrival coming out of one of the world's stellars
//!    ([`emerge`]) or by hyperspace, then the escort-strength timer's tick ([`strength`]):
//!    when it fires, the fleet the [`World`]'s escorts' tons call for
//!    comes, whatever `AvgShips` says, its lead an interceptor. The
//!    timer is armed by its session ([`Traffic::arm_strength_timer`]),
//!    and entering or going on into a system keeps it. Each announced
//!    fleet that comes with a `Quote` queues it for its session to show
//!    ([`Traffic::take_quotes`], see [`quote`]). Each ship the system's
//!    traffic brings, at setup or as an arrival, takes a ship-table slot
//!    below the [`World`]'s spawn reserve, 8 of the 64 by the engine
//!    ([`SPAWN_RESERVE`]), and one that finds none does not come: a
//!    fleet's lead with none brings no escorts and no quote;
//! 2. each intact NPC answers the fight's strikes since the last tick
//!    ([`Behaviour::react`]), one still coming in too unless the
//!    [`World::arrival_hits`] say otherwise: the ship it targets, its
//!    provocation, and any goal it takes at once; one hit in its gate has
//!    its wait cut, and the player's hit there holds it at the gate (see
//!    the traffic's `react`);
//! 3. an NPC not intact loses its provocation;
//! 4. the decisions due on the AI timer, never for a ship still jumping
//!    in or emerging from a gate (waiting there or bursting out of it,
//!    but held at it, [`Mode::HeldAtGate`], it decides), one entering a
//!    gate to leave, one that is not intact, or one assisting the player
//!    while
//!    unprovoked (until it is done, or provoked): first every goal, then, each NPC
//!    seeing the new goals, the fire command it holds and the ship it
//!    targets. An NPC that goes back to an idle goal (not fighting) loses
//!    its provocation, and one that stops inspecting a ship remembers it,
//!    and, the player within its reach, has scanned the player
//!    ([`Traffic::take_scans`]). Then each ship waiting in its gate or held
//!    at it holds only what the behaviour gives it for the wait
//!    ([`Behaviour::gate_wait`], `_AIEscortFireUnprovoked`), dropping its
//!    target if the behaviour says so;
//! 5. every NPC's cloak ([`cloak`]): a flying NPC's think
//!    cloaks or uncloaks it, one cloaked that can no longer cloak
//!    uncloaks, and its fade steps;
//! 6. the autopilot and a flight step for every NPC, each flying towards
//!    the ship its goal is about, the player's included (the one it
//!    assists, or escorts in formation), a fighter docking its carrier;
//!    then each NPC that reached its gate's box this tick releases the
//!    ships it leads to enter the gate on their own ([`gate_entry`]);
//! 7. and the removal of those that landed, jumped out, entered a gate or
//!    docked, each in [`Traffic::departed`] with how it left, for its
//!    session to take a fighter docked aboard.
//!
//! The fight ([`combat`](crate::combat)) damages them, and its session
//! takes out each one destroyed ([`Traffic::remove`]).
//!
//! Going on into another system ([`Traffic::retarget`]) takes along only
//! the NPCs it is told to keep. The others are dropped, or held in the
//! ship table, out of the system, until the end of the next tick
//! ([`Leave`]), their berths and their persons' names still taken; those
//! held in the system it goes on into come back as they were.
//!
//! The AI timer is the original's (`_AIDispatch`): an NPC decides on the
//! ticks where `tick % interval == id % interval`, so decisions are
//! staggered across NPCs. The original's interval is 1 at full speed and
//! grows when frames run slow; the simulation's is always
//! [`DECISION_INTERVAL`]. Steering and physics run every tick.

pub mod autopilot;
pub mod destination;
pub mod emerge;
pub mod gate_entry;
pub mod npc;
pub mod quote;
pub mod spawn;
pub mod strength;
pub mod table;

use crate::ai::interceptor::within_reach;
use crate::ai::{Behaviour, GateWait, Goal, PlayerSide, Reaction, Surroundings};
use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{GovtId, LandingSite, PersonId, ShipId, StellarId, SystemId};
use crate::chance::Chance;
use crate::cloak::{self, Cloak};
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::control::Gate;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::landing::TypeWidth;
use crate::mission::ships::ShipBehav;
use crate::person::{PersonWorld, derelict_armor};
use crate::rulebook::RuleSource;
use autopilot::{Outcome, TopSpeeds};
use npc::{AiType, Mode, Npc, NpcId, NpcPerson};
use spawn::{Entry, NewShip, PersonDraw, Stellars};
use strength::StrengthTimer;
use table::{SpawnPerson, SpawnTable};

/// The ship-table slots a traffic spawn leaves free:
/// `_GenericRandomShipSpawn(arg1, reserve)` (@0x3c89f) searches only the
/// slots 1..(63 - reserve) of the 64-slot table (@0x3c8b1-0x3c8e9), and
/// every spawn of the system's own traffic passes 8, so it stops at 55
/// NPCs: `_HyperSpawnFleet`'s lead and escorts (@0x41ce9, @0x421a5), the
/// `düde` ships (`_RandomShipSpawn` @0x43297, @0x42990), the auxiliary
/// ships (`_SpecificDudeSpawn` from `_EnterMoreShips` @0x435a8), the
/// persons (`_SpawnPerson` @0x40cba), the fighters (`_LaunchFighter`
/// @0x3d648), the mission ships (`_MissionDudeSpawn` @0x3cd50) and the
/// hired escorts (`_SpawnEscort` @0x3fe73). See [`Traffic::free_berth`].
pub const SPAWN_RESERVE: u32 = 8;

/// The ship-table slots the old ship of a "Use As My Ship" capture leaves
/// free: `_DoShipCapture` passes 1 (@0x41136), so it takes a slot in
/// 1-62. See [`Traffic::free_berth`].
pub const CAPTURE_RESERVE: u32 = 1;

/// How many ticks apart an NPC's decisions are: every tick, as the
/// original at full speed.
pub const DECISION_INTERVAL: u32 = 1;

/// Whether an NPC numbered `id` decides on tick `tick` with decisions
/// `interval` ticks apart (none apart counts as one).
#[must_use]
pub fn decision_due(tick: u64, id: NpcId, interval: u32) -> bool {
    let interval = u64::from(interval.max(1));
    tick % interval == u64::from(id.0) % interval
}

/// What the traffic flies among besides itself: the system's stellars,
/// the player, the governments, and the player's legal record there.
#[derive(Clone, Copy, Debug)]
pub struct World<'a> {
    /// The system's stellars.
    pub sites: &'a [LandingSite],
    /// The player's ship, while it flies in the system.
    pub player: Option<PlayerSide>,
    /// Every government and their relations.
    pub govts: &'a Governments,
    /// The system's government, or `None` when it is independent.
    pub system_govt: Option<GovtId>,
    /// The player's legal record with the system's government; none in an
    /// independent system.
    pub record: i16,
    /// What the persons' spawning sees: the rules, the persons gone and
    /// grudging, and the control bits.
    pub persons: PersonWorld<'a>,
    /// The tons the player's escorts that count toward the
    /// escort-strength timer carry ([`strength::escort_tons_in_system`]).
    pub escort_tons: i64,
    /// The factor on the top speed of the player and of every ship it
    /// leads: 1.5 for a pilot without Strict Play by the engine (see
    /// [`top_speed`](crate::top_speed)).
    pub led_speed: f32,
    /// The damage the shots in flight aim at each ship
    /// ([`Surroundings::targeted`]).
    pub targeted: &'a BTreeMap<ShipRef, i32>,
    /// The width each gate's `Type` master gives, which its entry box
    /// goes by ([`gate_entry::gate_reach`]); a gate it holds none for reads
    /// as a loaded master does.
    pub type_widths: &'a BTreeMap<StellarId, TypeWidth>,
    /// The ship-table slots the system's own traffic leaves free as it
    /// spawns ([`Traffic::free_berth`]): [`SPAWN_RESERVE`] by the engine,
    /// 0 by the Bible
    /// ([`RuleKey::SpawnReserve`](crate::RuleKey::SpawnReserve)).
    pub spawn_reserve: u32,
    /// Which ships answer a hit ([`Traffic::tick_in`]'s reactions): every
    /// intact one by the engine, as `_DamageShip` has it, or only those
    /// that have arrived ([`Npc::arrived`]) by the other reading
    /// ([`RuleKey::ArrivalHits`](crate::RuleKey::ArrivalHits)).
    pub arrival_hits: RuleSource,
}

/// No gate's width known: each reads as a loaded master does.
pub(crate) static NO_TYPE_WIDTHS: BTreeMap<StellarId, TypeWidth> = BTreeMap::new();

impl<'a> World<'a> {
    /// `sites` alone: no player, no governments, no record, no person
    /// gone or grudging ([`PersonWorld::NONE`]), no escorts' tons, no
    /// factor on the led ships' top speed, no damage aimed, no gate's
    /// width known, and the engine's [`SPAWN_RESERVE`].
    #[must_use]
    pub fn new(sites: &'a [LandingSite]) -> Self {
        let bare = Surroundings::new(sites, &[]);
        Self {
            sites,
            player: None,
            govts: bare.govts,
            system_govt: None,
            record: 0,
            persons: PersonWorld::NONE,
            escort_tons: 0,
            led_speed: 1.0,
            targeted: bare.targeted,
            type_widths: &NO_TYPE_WIDTHS,
            spawn_reserve: SPAWN_RESERVE,
            arrival_hits: RuleSource::Engine,
        }
    }

    /// The stellars arrivals may come out of, and the governments that
    /// judge whether one may.
    #[must_use]
    pub fn stellars(self) -> Stellars<'a> {
        Stellars {
            sites: self.sites,
            govts: self.govts,
        }
    }

    /// What `npcs` see in this world.
    #[must_use]
    pub fn around(self, npcs: &'a [Npc]) -> Surroundings<'a> {
        Surroundings {
            sites: self.sites,
            npcs,
            player: self.player,
            govts: self.govts,
            system_govt: self.system_govt,
            record: self.record,
            targeted: self.targeted,
        }
    }
}

/// A system's NPCs, and what more of them are drawn from.
#[derive(Clone, Debug, PartialEq)]
pub struct Traffic {
    /// What the system's traffic is drawn from.
    table: SpawnTable,
    /// The NPCs, in the order they appeared.
    npcs: Vec<Npc>,
    /// The next NPC's number: never reused, even across systems.
    next_id: u32,
    /// The berths held back for the player's escorts still to be placed
    /// ([`Traffic::reserve`]), kept across systems.
    reserved: BTreeSet<u32>,
    /// Ticks since the traffic began.
    ticks: u64,
    /// How many ticks apart decisions are.
    interval: u32,
    /// The NPCs that left on the last tick, and how.
    departed: Vec<(NpcId, Outcome)>,
    /// The NPCs that have looked the player over since the scans were
    /// last taken, in order.
    scans: Vec<NpcId>,
    /// The `STR#`s of the arrival quotes of the fleets added since the
    /// quotes were last taken, in order ([`NewShip::quote`]).
    quotes: Vec<i16>,
    /// The escort-strength timer, kept across systems.
    strength_timer: StrengthTimer,
    /// The ships an in-flight move left behind ([`Leave::Held`]), by the
    /// system they are in, each in the order they appeared: still in the
    /// ship table until the end of the next tick ([`Traffic::tick_in`]),
    /// holding their berths and their persons, but not in the system:
    /// not ticked, not drawn, and not among [`Traffic::npcs`].
    left_behind: BTreeMap<SystemId, Vec<LeftShip>>,
}

/// What becomes of the ships [`Traffic::retarget`] does not take along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    /// They stay behind in `from`, in the ship table, until the end of
    /// the next tick: an in-flight `M` or `N` by the engine
    /// ([`RuleKey::MoveLeftBehind`](crate::RuleKey::MoveLeftBehind)).
    /// Those left behind in `to` before then come back as they were.
    Held {
        /// The system left.
        from: SystemId,
        /// The system entered.
        to: SystemId,
    },
    /// They are dropped at once, their berths freed.
    Dropped,
}

/// A ship an in-flight move left behind ([`Leave::Held`]).
#[derive(Clone, Debug, PartialEq)]
struct LeftShip {
    /// The ship, as it was left.
    npc: Npc,
    /// The name of its person, if any, as the system it was left in gave
    /// it: the system entered may not list that person.
    person: Option<String>,
}

impl Default for Traffic {
    fn default() -> Self {
        Self::with_interval(DECISION_INTERVAL)
    }
}

impl Traffic {
    /// No traffic, deciding every [`DECISION_INTERVAL`] ticks.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// No traffic, deciding every `interval` ticks.
    #[must_use]
    pub fn with_interval(interval: u32) -> Self {
        Self {
            table: SpawnTable::default(),
            npcs: Vec::new(),
            next_id: 0,
            reserved: BTreeSet::new(),
            ticks: 0,
            interval,
            departed: Vec::new(),
            scans: Vec::new(),
            quotes: Vec::new(),
            strength_timer: StrengthTimer::default(),
            left_behind: BTreeMap::new(),
        }
    }

    /// Arms the escort-strength timer, drawing its ticks on `chance` (see
    /// [`strength`]).
    pub fn arm_strength_timer(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.strength_timer.arm(chance);
    }

    /// Whether the escort-strength timer is armed.
    #[cfg(test)]
    pub(crate) fn strength_armed(&self) -> bool {
        self.strength_timer.armed()
    }

    /// Enters a system whose traffic is drawn from `table`: its NPCs are
    /// replaced with its initial population, rolled on `chance`, with no
    /// person gone or grudging (see [`Traffic::enter_in`]).
    pub fn enter(&mut self, table: SpawnTable, chance: &mut (impl Chance + ?Sized)) {
        self.enter_in(table, World::new(&[]), chance);
    }

    /// Enters a system whose traffic is drawn from `table` in `world`: its
    /// NPCs are replaced with its initial population, rolled on `chance`,
    /// its persons as the world's persons say, and its stationary
    /// warships placed by the world's first stellar.
    pub fn enter_in(
        &mut self,
        table: SpawnTable,
        world: World,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.enter_with(table, world, Vec::new(), chance);
    }

    /// Enters a system as [`Traffic::enter_in`] does, with `missions`, the
    /// active missions' special ships set up there, first: as
    /// `_SetupShipsInSystem` sets them up before the system's own passes
    /// (@0x42dc4-0x43194), none of which they use up. Every ship, the
    /// mission ships first, takes a slot below the world's spawn reserve,
    /// those with none not coming ([`Traffic::free_berth`]): a mission
    /// ship's `_MissionDudeSpawn` passes 8 (@0x3cd50) and, on its -1,
    /// `_SetupShipsInSystem` goes on to the next ship (@0x43128), no
    /// count changed. The scans
    /// not yet taken are dropped: the original scans at once, so none
    /// outlives its system.
    pub fn enter_with(
        &mut self,
        table: SpawnTable,
        world: World,
        missions: Vec<NewShip>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.table = table;
        self.npcs.clear();
        self.left_behind.clear();
        self.departed.clear();
        self.scans.clear();
        self.quotes.clear();
        self.add(missions, world.persons.grudges, Some(world.spawn_reserve));
        let mut draw = PersonDraw::new(world.persons);
        let first_stellar = world.sites.first().map(|site| site.position);
        let ships = spawn::initial(
            &self.table,
            &mut draw,
            first_stellar,
            world.stellars(),
            chance,
        );
        self.add(ships, world.persons.grudges, Some(world.spawn_reserve));
    }

    /// Brings `ships` into the system as they are, numbered after every
    /// other NPC, with no ship-table cap.
    pub fn bring_in(&mut self, ships: Vec<NewShip>) {
        self.add(ships, &BTreeSet::new(), None);
    }

    /// Brings `ships` into the system as they are, numbered after every
    /// other NPC, each leaving `reserve` slots of the ship table free
    /// ([`Traffic::free_berth`]), and gives how many came: one that finds
    /// no slot does not come. The auxiliary ships' spawn
    /// (`_SpecificDudeSpawn` from `_EnterMoreShips`, reserve 8 @0x435a8),
    /// and the mission ships' whose appear timer ran out or that a hail
    /// swaps in (`_MissionDudeSpawn`, reserve 8 @0x3cd50).
    pub fn bring_in_within(&mut self, ships: Vec<NewShip>, reserve: u32) -> usize {
        self.add(ships, &BTreeSet::new(), Some(reserve))
    }

    /// What the system's traffic is drawn from.
    #[must_use]
    pub fn table(&self) -> &SpawnTable {
        &self.table
    }

    /// What the system's traffic is drawn from, to add the mission
    /// `düde`s to ([`SpawnTable::add_mission_dudes`]).
    pub(crate) fn table_mut(&mut self) -> &mut SpawnTable {
        &mut self.table
    }

    /// Goes on in another system whose traffic is drawn from `table`,
    /// taking along only the NPCs in `keep`, as they are: the others stay
    /// behind as `leave` says, and nothing of the new system's initial
    /// population comes, only its arrivals over time. The NPCs' numbering
    /// goes on. The scans not yet taken are dropped, as entering a system
    /// drops them.
    ///
    /// `_EvalSetExp`'s `M` and `N` (@0x15807-0x1583e, @0x1590b-0x1593d)
    /// move only the active ships whose leader is the player; every other
    /// ship keeps its system and stays in the ship table until that
    /// frame's `_HandleShipDisplay` frees it, unshown, uncounted and
    /// nothing given back (@0x2ded6-0x2dee7). [`Leave::Held`] keeps them
    /// so until the end of the next tick; one left behind in the system
    /// entered comes back as it was, in the order it appeared.
    /// [`Leave::Dropped`] drops them at once.
    pub fn retarget(&mut self, table: SpawnTable, keep: &BTreeSet<NpcId>, leave: Leave) {
        match leave {
            Leave::Held { from, to } => {
                let (kept, left): (Vec<Npc>, Vec<Npc>) = std::mem::take(&mut self.npcs)
                    .into_iter()
                    .partition(|npc| keep.contains(&npc.id));
                let left: Vec<LeftShip> = left
                    .into_iter()
                    .map(|npc| LeftShip {
                        person: self.person_name(&npc),
                        npc,
                    })
                    .collect();
                self.npcs = kept;
                self.left_behind.entry(from).or_default().extend(left);
                self.table = table;
                if let Some(back) = self.left_behind.remove(&to) {
                    // Numbers only grow, so their order is the order the
                    // ships appeared in.
                    self.npcs.extend(back.into_iter().map(|left| left.npc));
                    self.npcs.sort_by_key(|npc| npc.id);
                }
            }
            Leave::Dropped => {
                self.table = table;
                self.npcs.retain(|npc| keep.contains(&npc.id));
            }
        }
        self.departed.clear();
        self.scans.clear();
        self.quotes.clear();
    }

    /// Advances the traffic one tick among `sites` alone, NPCs deciding
    /// as `behaviour` says, rolling on `chance` (see
    /// [`Traffic::tick_in`]).
    pub fn tick(
        &mut self,
        behaviour: &(impl Behaviour + ?Sized),
        sites: &[LandingSite],
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.tick_in(behaviour, World::new(sites), &[], chance);
    }

    /// Advances the traffic one tick in `world`, NPCs answering the
    /// `strikes` since the last tick and deciding as `behaviour` says,
    /// rolling on `chance` (see the module docs).
    pub fn tick_in(
        &mut self,
        behaviour: &(impl Behaviour + ?Sized),
        world: World,
        strikes: &[Strike],
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.enter_more_ships(world, chance);
        self.react(behaviour, &world, strikes);
        for npc in &mut self.npcs {
            if npc.condition != Condition::Intact {
                npc.provoked = 0.0;
            }
        }
        let due: Vec<bool> = self
            .npcs
            .iter()
            .map(|npc| {
                matches!(npc.mode, Mode::Flying | Mode::HeldAtGate { .. })
                    && npc.condition == Condition::Intact
                    && !(matches!(npc.goal, Goal::Assist(_)) && npc.provoked <= 0.0)
                    && decision_due(self.ticks, npc.id, self.interval)
            })
            .collect();
        let around = world.around(&self.npcs);
        let goals: Vec<Option<Goal>> = self
            .npcs
            .iter()
            .zip(&due)
            .map(|(npc, &due)| due.then(|| behaviour.decide(npc, &around, &mut &mut *chance)))
            .collect();
        self.take_goals(goals, world.player.map(|player| player.state.position));
        let around = world.around(&self.npcs);
        let commands: Vec<Option<(Trigger, Option<ShipRef>)>> = self
            .npcs
            .iter()
            .zip(&due)
            .map(|(npc, &due)| {
                due.then(|| {
                    (
                        behaviour.trigger(npc, &around),
                        behaviour.target(npc, &around),
                    )
                })
            })
            .collect();
        for (npc, command) in self.npcs.iter_mut().zip(commands) {
            if let Some((trigger, target)) = command {
                npc.trigger = trigger;
                npc.target = target;
            }
        }
        let around = world.around(&self.npcs);
        let waits: Vec<Option<GateWait>> = self
            .npcs
            .iter()
            .map(|npc| {
                matches!(npc.mode, Mode::Emerging { .. } | Mode::HeldAtGate { .. })
                    .then(|| behaviour.gate_wait(npc, &around))
            })
            .collect();
        for (npc, wait) in self.npcs.iter_mut().zip(waits) {
            if let Some(wait) = wait {
                npc.trigger = wait.trigger;
                if wait.drops_target {
                    npc.target = None;
                }
            }
        }
        let states: Vec<(NpcId, ShipState)> =
            self.npcs.iter().map(|npc| (npc.id, npc.state)).collect();
        let player = world.player.map(|player| player.state);
        self.cloaks(player, &states);
        let entering: Vec<bool> = self.npcs.iter().map(Npc::entering_gate).collect();
        let outcomes = self.fly_all(&world, player, &states, chance);
        self.release_followers(&entering);
        self.departed = self
            .npcs
            .iter()
            .zip(&outcomes)
            .filter(|(_, outcome)| **outcome != Outcome::Flying)
            .map(|(npc, outcome)| (npc.id, *outcome))
            .collect();
        let mut flying = outcomes.iter().map(|outcome| *outcome == Outcome::Flying);
        self.npcs.retain(|_| flying.next().unwrap_or(false));
        // The display pass (`_DoPlayGameWork` @0x444ad) follows the
        // frame's arrivals pass (step 1) and frees each ship left behind
        // (`_HandleShipDisplay` @0x2ded6-0x2dee7). The original frees them
        // after the frame's AI and ship handling too, whose scans test the
        // system; here none of those steps sees a ship left behind. The
        // engine's move settles after the frame's traffic tick, so its one
        // arrivals pass is the next tick's: the lag of a settled script,
        // crossing no boundary, as a ship left behind is never drawn.
        self.left_behind.clear();
        self.ticks += 1;
    }

    /// The traffic's part of one `_EnterMoreShips` pass in `world`,
    /// rolling on `chance`: the arrival roll while the system is below
    /// its `AvgShips` (@0x43b14-0x43c01), then the escort-strength
    /// timer's tick (@0x43d1a-0x43dd7), as step 1 of the module docs
    /// says, each ship taking a slot below the world's spawn reserve
    /// ([`Traffic::free_berth`]): a fleet whose lead finds none does not
    /// come, nor does its quote. No NPC reacts, decides or moves, none departs, and no tick
    /// is counted. The original runs the pass once a frame
    /// (`_DoPlayGameWork` @0x44237), here first in each
    /// [`Traffic::tick_in`], and once more on a hypergate or wormhole
    /// exit, before the first frame (`_HandlePlayerDockRequest`
    /// @0x672f2), which its session runs.
    ///
    /// Two steps of the pass are left out: the stellar defence respawn
    /// between them (@0x43c45-0x43d10, `_SpawnStellarDefenseShip`
    /// @0x43c11), as no stellar holds defence ships here, and
    /// `_SpawnNagShip` (@0x43d15), the shareware nag. The pass's
    /// mission-slot loop (@0x434a2-0x43b0e) is its session's.
    pub fn enter_more_ships(&mut self, world: World, chance: &mut (impl Chance + ?Sized)) {
        let mut draw = PersonDraw {
            world: world.persons,
            here: self.person_names(),
        };
        let arrivals = spawn::arrivals(
            &self.table,
            self.strangers(),
            &mut draw,
            world.stellars(),
            chance,
        );
        self.add(arrivals, world.persons.grudges, Some(world.spawn_reserve));
        self.escort_strength(&world, draw.world.gate(), chance);
    }

    /// Flies every NPC a tick in `world` ([`autopilot::fly`]), each towards
    /// the ship its goal is about among the `player` and `states`, at its
    /// top speeds, rolling on `chance`: what became of each, in order.
    fn fly_all(
        &mut self,
        world: &World,
        player: Option<ShipState>,
        states: &[(NpcId, ShipState)],
        chance: &mut (impl Chance + ?Sized),
    ) -> Vec<Outcome> {
        let speeds = self.top_speeds(world);
        self.npcs
            .iter_mut()
            .zip(speeds)
            .map(|(npc, speeds)| {
                let other = match npc.goal {
                    Goal::Follow(lead) => Some(ShipRef::Npc(lead)),
                    Goal::Assist(_) | Goal::Formation { .. } => Some(ShipRef::Player),
                    Goal::Dock(carrier) => Some(carrier),
                    goal => goal.quarry().or(goal.inspecting()),
                };
                let other = other.and_then(|ship| match ship {
                    ShipRef::Player => player,
                    ShipRef::Npc(id) => states
                        .iter()
                        .find(|(other, _)| *other == id)
                        .map(|(_, state)| *state),
                });
                autopilot::fly(
                    npc,
                    world.sites,
                    world.type_widths,
                    other.as_ref(),
                    speeds,
                    &mut *chance,
                )
            })
            .collect()
    }

    /// Releases the followers of each NPC that reached its gate's box this
    /// tick, not `entering` one as the tick began (see
    /// [`gate_entry`](crate::traffic::gate_entry)): each ship it leads
    /// loses its lead, its carrier, its target and its provocation, takes
    /// the lead's AI type capped at 3 ([`gate_entry::released_ai_type`]),
    /// and heads into the same gate on its own, its mode as it was. A
    /// fighter so freed decides by its own AI type, as the original's AI
    /// type 5 becomes at most 3.
    ///
    /// The original releases them in the same pass as the lead's think, so
    /// its later slots start for the gate that tick; here they start on
    /// the next. That one-tick lag settles before anything the player sees
    /// is drawn, so it is not filed.
    fn release_followers(&mut self, entering: &[bool]) {
        let released: Vec<(NpcId, StellarId, AiType)> = self
            .npcs
            .iter()
            .zip(entering)
            .filter_map(|(npc, &was)| match npc.mode {
                Mode::EnteringGate { gate, .. } if !was => Some((npc.id, gate, npc.ai_type)),
                _ => None,
            })
            .collect();
        for (lead, gate, ai_type) in released {
            for follower in self.npcs.iter_mut().filter(|npc| npc.leader == Some(lead)) {
                follower.leader = None;
                follower.carrier = None;
                follower.ai_type = gate_entry::released_ai_type(ai_type);
                follower.target = None;
                follower.provoked = 0.0;
                follower.goal = Goal::EnterGate(gate);
            }
        }
    }

    /// Each NPC's top speeds this tick, in ship-table order, as
    /// `_ShipMaxSpeed` gives them
    /// (see [`top_speed`](crate::top_speed)): its own
    /// ([`Npc::top_speed`]), and its lead's. A ship the player leads has
    /// the player's ([`PlayerSide::handling`] × [`World::led_speed`]);
    /// a fleet escort its lead's; one whose lead is not in the system, as
    /// `_AICheckParentExists` clears it (@0x8946b, @0x894b4), or with
    /// none, no lead's.
    fn top_speeds(&self, world: &World) -> Vec<TopSpeeds> {
        let tops: Vec<(NpcId, f32)> = self
            .npcs
            .iter()
            .map(|npc| (npc.id, npc.top_speed(world.led_speed)))
            .collect();
        let player_top = world
            .player
            .map(|player| player.handling.max_speed * world.led_speed);
        tops.iter()
            .zip(&self.npcs)
            .map(|(&(_, own), npc)| {
                let lead = if npc.escorts_player() {
                    player_top
                } else {
                    npc.leader.and_then(|leader| {
                        tops.iter()
                            .find(|(other, _)| *other == leader)
                            .map(|&(_, top)| top)
                    })
                };
                TopSpeeds { own, lead }
            })
            .collect()
    }

    /// Each NPC's cloak a tick on (see [`cloak`]): a flying NPC thinks
    /// (`_AIHandleCloaking`), knowing whether it is disabled, retreats
    /// (fleeing), and where the ship it attacks is, among `states` and the
    /// `player`; one cloaked as the tick began that can no longer cloak
    /// uncloaks (`_HandleShip` @0x3413d); and its fade steps.
    fn cloaks(&mut self, player: Option<ShipState>, states: &[(NpcId, ShipState)]) {
        for npc in &mut self.npcs {
            let was = npc.cloak.is_cloaked();
            let disabled = npc.condition == Condition::Disabled;
            if npc.mode == Mode::Flying {
                let attacking = npc.goal.attacking().and_then(|ship| match ship {
                    ShipRef::Player => player,
                    ShipRef::Npc(id) => states
                        .iter()
                        .find(|(other, _)| *other == id)
                        .map(|(_, state)| *state),
                });
                let situation = cloak::Situation {
                    disabled,
                    retreating: matches!(npc.goal, Goal::Flee(_)),
                    attacking: attacking.map(|state| state.position - npc.state.position),
                };
                cloak::think(&mut npc.cloak, npc.cloak_gear, situation);
            }
            if was {
                let able = npc.can_cloak();
                cloak::hold(&mut npc.cloak, able);
            }
            npc.cloak.step(npc.cloak_gear.fast());
        }
    }

    /// Counts the escort-strength timer down a tick; when it fires, the
    /// fleet `world`'s escorts' tons call for comes among its stellars,
    /// its lead an interceptor, its `AppearOn` asked by `gate`, rolled on
    /// `chance`.
    fn escort_strength(&mut self, world: &World, gate: Gate, chance: &mut (impl Chance + ?Sized)) {
        if !self.strength_timer.tick() {
            return;
        }
        let Some(id) = strength::fleet_for(world.escort_tons) else {
            return;
        };
        let mut fleet = Vec::new();
        spawn::fleet(
            &self.table,
            id,
            gate,
            world.stellars(),
            spawn::FleetCall {
                lead_ai: Some(AiType::Interceptor),
                announced: true,
            },
            chance,
            &mut fleet,
        );
        self.add(fleet, world.persons.grudges, Some(world.spawn_reserve));
    }

    /// Each intact NPC answers each of `strikes` in `world`, as
    /// `behaviour` says, all against the NPCs as they were before; by the
    /// other reading of [`World::arrival_hits`] only those that have
    /// arrived ([`Npc::arrived`]) do. A hit on it that it answers (taking
    /// a target) cuts its wait, in its gate, held at it or holding its
    /// course ([`autopilot::cut_wait`], `_DamageShip` @0x3be6e-0x3be7c).
    ///
    /// A ship still coming in answers as `_DamageShip` has it (see the
    /// [`ai::react`](crate::ai::react) module):
    ///
    /// - Waiting in its gate ([`Mode::Emerging`]), it keeps waiting,
    ///   hidden, with its new target and provocation, unless the player
    ///   hit it, or a goal is given it (a Good Samaritan's): then
    ///   `_AIMakeShipAttackPlayer` (@0x3be66-0x3be69) or
    ///   `_DoGoodSamaritan` (@0x82b82-0x82b97) sets goal 4, and it is held
    ///   at its gate attacking the player ([`Npc::hold_at_gate`]).
    /// - Gliding in or bursting out ([`Mode::JumpingIn`],
    ///   [`Mode::LeavingGate`]), it takes its target and provocation, but
    ///   no goal: `_AIDispatch` puts goal 8 back on the next frame while
    ///   +0x50 is below -900 (@0x8fcf5-0x8fd18), and nothing reads the
    ///   goal 4 before it does.
    ///
    /// The strikes are the fight's since the last tick, so a ship the
    /// player hit in its gate is held, and seen, on the next traffic tick
    /// after the hit: one frame after the original shows it. That lag is
    /// filed.
    fn react(&mut self, behaviour: &(impl Behaviour + ?Sized), world: &World, strikes: &[Strike]) {
        if strikes.is_empty() {
            return;
        }
        let around = world.around(&self.npcs);
        let reactions: Vec<Vec<Reaction>> = self
            .npcs
            .iter()
            .map(|npc| {
                let answers = match world.arrival_hits {
                    RuleSource::Engine => true,
                    RuleSource::Bible => npc.arrived(),
                };
                if !answers || npc.condition != Condition::Intact {
                    return Vec::new();
                }
                strikes
                    .iter()
                    .map(|strike| behaviour.react(npc, strike, &around))
                    .collect()
            })
            .collect();
        for (npc, reactions) in self.npcs.iter_mut().zip(reactions) {
            for (reaction, strike) in reactions.into_iter().zip(strikes) {
                let mut attacks_player = false;
                if let Some(target) = reaction.target {
                    npc.target = Some(target);
                    if strike.ship == ShipRef::Npc(npc.id) {
                        autopilot::cut_wait(npc);
                        attacks_player = strike.by == ShipRef::Player;
                    }
                }
                npc.provoked += reaction.provoked;
                match npc.mode {
                    Mode::JumpingIn { .. } | Mode::LeavingGate { .. } => {}
                    Mode::Emerging { .. } if attacks_player || reaction.goal.is_some() => {
                        npc.hold_at_gate();
                    }
                    _ => {
                        if let Some(goal) = reaction.goal {
                            npc.goal = goal;
                        }
                    }
                }
            }
        }
    }

    /// The NPCs, in the order they appeared.
    #[must_use]
    pub fn npcs(&self) -> &[Npc] {
        &self.npcs
    }

    /// The NPCs, in the order they appeared, to fight with.
    pub(crate) fn npcs_mut(&mut self) -> &mut [Npc] {
        &mut self.npcs
    }

    /// The ships the player leads, in ship-table order (berth order,
    /// [`Npc::berth`]; ships at one berth in the order they appeared): its hired and captured escorts, the fighters it
    /// launched, and its mission escorts ([`Npc::escorts_player`]). In
    /// the original each is a ship whose leader (+0x9a) is the player,
    /// 0, hired or a mission's `ShipBehav` 1 escort alike. The original's
    /// sites that ask this, and which of them narrow it to the hired
    /// escorts or the fighters, are listed in the session's `player_led`
    /// module.
    pub fn player_led(&self) -> impl Iterator<Item = &Npc> + '_ {
        let mut led: Vec<&Npc> = self
            .npcs
            .iter()
            .filter(|npc| npc.escorts_player())
            .collect();
        led.sort_by_key(|npc| npc.berth);
        led.into_iter()
    }

    /// The NPCs that count toward the system's `AvgShips`: those the
    /// player does not lead ([`Traffic::player_led`]).
    /// `_EnterMoreShips` (@0x43b14-0x43b45) counts the ships in the system
    /// whose parent (+0x9a) is not the player, and rolls for an arrival
    /// only while that count is below `AvgShips` (@0x43b5c).
    fn strangers(&self) -> usize {
        self.npcs.len() - self.player_led().count()
    }

    /// How many slots of the ship table the traffic takes: its NPCs and
    /// the ships an in-flight move left behind ([`Leave::Held`]), which
    /// stay active in the ship table until the display pass frees them.
    /// The ship-table caps take the same ships slot by slot, as the
    /// original does, not as a total ([`Traffic::free_berth`]).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn berths_taken(&self) -> usize {
        self.npcs.len() + self.left_behind.values().map(Vec::len).sum::<usize>()
    }

    /// Adds `npc` to the system, numbered as the next NPC, and gives its
    /// number.
    pub(crate) fn add_npc(&mut self, npc: Npc) -> NpcId {
        let berth = self.next_berth();
        self.add_npc_at(npc, berth)
    }

    /// Holds `berths` back from the first-free numbering until an NPC is
    /// placed at each ([`Traffic::add_npc_at`]), replacing any held before.
    pub(crate) fn reserve(&mut self, berths: BTreeSet<u32>) {
        self.reserved = berths;
    }

    /// Adds `npc` to the system at berth `berth`, numbered as the next NPC,
    /// and gives its number.
    pub(crate) fn add_npc_at(&mut self, npc: Npc, berth: u32) -> NpcId {
        self.reserved.remove(&berth);
        let id = NpcId(self.next_id);
        self.next_id += 1;
        self.npcs.push(Npc { id, berth, ..npc });
        id
    }

    /// The first berth from 1 that no NPC in the system holds and none is
    /// reserved for: the slot `_GenericRandomShipSpawn` (@0x3c8b1-0x3c90b)
    /// gives a new ship.
    fn next_berth(&self) -> u32 {
        self.next_berth_skipping(&BTreeSet::new())
    }

    /// The first berth from 1 that no NPC in the system holds, no ship
    /// left behind holds ([`Leave::Held`]), none is reserved for, and
    /// `skip` does not hold: `_GenericRandomShipSpawn` (@0x3c8b1-0x3c90b)
    /// takes the first slot whose active flag is clear, with no test of
    /// the system. It has no bound: a spawn that leaves slots free takes
    /// [`Traffic::free_berth`] instead.
    pub(crate) fn next_berth_skipping(&self, skip: &BTreeSet<u32>) -> u32 {
        let held: BTreeSet<u32> = self
            .left_behind
            .values()
            .flatten()
            .map(|left| left.npc.berth)
            .collect();
        (1..=u32::MAX)
            .find(|berth| {
                !self.reserved.contains(berth)
                    && !skip.contains(berth)
                    && !held.contains(berth)
                    && self.npcs.iter().all(|npc| npc.berth != *berth)
            })
            .unwrap_or(u32::MAX)
    }

    /// The berth a spawn that leaves `reserve` slots free takes: the first
    /// free one (no NPC, ship left behind or reservation holds it), when it is below
    /// [`MAX_SHIPS_IN_SYSTEM`](crate::board::MAX_SHIPS_IN_SYSTEM) less
    /// `reserve`; otherwise `None`. `_GenericRandomShipSpawn` searches
    /// slots 1..(63 - `reserve`) for one whose active flag is clear
    /// (@0x3c8b1-0x3c8e9) and gives -1 when none is (@0x3c8eb-0x3c8f0), so
    /// it goes by slot, not by count: a ship above the bound does not
    /// block it, and a free slot above it cannot be taken.
    #[must_use]
    pub fn free_berth(&self, reserve: u32) -> Option<u32> {
        self.free_berth_skipping(reserve, &BTreeSet::new())
    }

    /// The berth [`Traffic::free_berth`] gives with `skip` held too: the
    /// first free one that `skip` does not hold, when it is below the
    /// bound `reserve` leaves. A hire passes its loaded escorts not placed
    /// yet, whose slots the original's table already holds.
    #[must_use]
    pub fn free_berth_skipping(&self, reserve: u32, skip: &BTreeSet<u32>) -> Option<u32> {
        let bound = u32::try_from(crate::board::MAX_SHIPS_IN_SYSTEM)
            .unwrap_or(u32::MAX)
            .saturating_sub(reserve);
        Some(self.next_berth_skipping(skip)).filter(|&berth| berth < bound)
    }

    /// Takes NPC `id` out of the system, if it is there.
    pub fn remove(&mut self, id: NpcId) {
        self.npcs.retain(|npc| npc.id != id);
    }

    /// The NPCs that left on the last tick, and how.
    #[must_use]
    pub fn departed(&self) -> &[(NpcId, Outcome)] {
        &self.departed
    }

    /// Gives each NPC its goal decided, if any, the player at `player_at`
    /// if there is one: one that stops inspecting a ship remembers it,
    /// and has scanned the player when it was inspecting the player
    /// within its reach; one that does not fight loses its provocation.
    fn take_goals(&mut self, goals: Vec<Option<Goal>>, player_at: Option<Vec2>) {
        for (npc, goal) in self.npcs.iter_mut().zip(goals) {
            let Some(goal) = goal else {
                continue;
            };
            if let Goal::Inspect(ship) = npc.goal
                && goal != npc.goal
            {
                npc.inspected = Some(ship);
                let scans = ship == ShipRef::Player
                    && player_at.is_some_and(|at| within_reach(at - npc.state.position));
                if scans {
                    self.scans.push(npc.id);
                }
            }
            if !goal.fights() {
                npc.provoked = 0.0;
            }
            npc.goal = goal;
        }
    }

    /// The NPCs that have looked the player over since this was last
    /// called, in order: each that stopped inspecting the player while
    /// within an interceptor's reach of it (`_HighLevelAIHandler`
    /// @0x8f11e-0x8f38a calls `_ScanPlayer` there). Taking them empties
    /// the list.
    pub fn take_scans(&mut self) -> Vec<NpcId> {
        std::mem::take(&mut self.scans)
    }

    /// The `STR#`s of the arrival quotes of the fleets added since this
    /// was last called, in order: each announced fleet's whose `flët` has
    /// a `Quote` (see [`quote`]). Taking them empties the list; entering
    /// or leaving for another system drops those not taken.
    pub fn take_quotes(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.quotes)
    }

    /// The ship types the system's traffic can spawn, by ascending ID.
    #[must_use]
    pub fn ship_types(&self) -> Vec<ShipId> {
        self.table.ship_types()
    }

    /// Person `id` as the system's traffic spawns it, if it can.
    #[must_use]
    pub fn person(&self, id: PersonId) -> Option<&SpawnPerson> {
        self.table.persons.get(&id)
    }

    /// The names of the persons in the ship table: those in the system
    /// and those an in-flight move left behind ([`Leave::Held`]), as
    /// `_SpawnPerson`'s check (@0x40bb5-0x40c48) strikes out a person
    /// whose name a person ship active in any slot has, with no test of
    /// the system.
    fn person_names(&self) -> BTreeSet<String> {
        self.npcs
            .iter()
            .filter_map(|npc| self.person_name(npc))
            .chain(
                self.left_behind
                    .values()
                    .flatten()
                    .filter_map(|left| left.person.clone()),
            )
            .collect()
    }

    /// The name of `npc`'s person, if any, as the system's traffic gives
    /// it.
    fn person_name(&self, npc: &Npc) -> Option<String> {
        self.person(npc.person?.id)
            .map(|person| person.record.name.clone())
    }

    /// The leader and the berth of `ship`, added after the ships `placed`
    /// (each one's number, or `None` for one that did not come), as
    /// [`Traffic::add`] places it with `reserve`: `None` when it is an
    /// escort whose lead did not come, or finds no slot.
    fn slot_for(
        &self,
        ship: &NewShip,
        placed: &[Option<NpcId>],
        reserve: Option<u32>,
    ) -> Option<(Option<NpcId>, u32)> {
        let leader = match ship.lead {
            Some(lead) => Some((*placed.get(lead)?)?),
            None => None,
        };
        let berth = match reserve {
            Some(reserve) => self.free_berth(reserve)?,
            None => self.next_berth(),
        };
        Some((leader, berth))
    }

    /// Adds `ships`, numbering each and pointing each escort at its lead,
    /// and gives how many came; a person flies its fitted ship, holding a
    /// grudge when `grudges` say; a ship set up derelict
    /// ([`NewShip::derelict`]) starts disabled, its shield gone and its
    /// armour cut ([`derelict_armor`]).
    ///
    /// A ship whose ship type has no record does not come, and neither
    /// does an escort whose lead did not come. With a `reserve`, each ship
    /// takes [`Traffic::free_berth`] of it, and one that finds none does
    /// not come: a fleet's lead with no slot brings no escorts
    /// (`_HyperSpawnFleet` returns @0x41d02), and an escort with none is
    /// skipped (@0x421b9 → @0x425b6). A ship that does not come adds no
    /// arrival quote, so a fleet whose lead finds no slot shows no line and
    /// beeps for none. The ships were rolled whole beforehand
    /// ([`spawn::fleet`]), the cap applied only here as they are placed, in
    /// the same call and before any frame.
    fn add(
        &mut self,
        ships: Vec<NewShip>,
        grudges: &BTreeSet<PersonId>,
        reserve: Option<u32>,
    ) -> usize {
        let mut placed: Vec<Option<NpcId>> = Vec::with_capacity(ships.len());
        for ship in ships {
            let person = ship.person.and_then(|id| self.table.persons.get(&id));
            let kind = person
                .map(|person| &person.kind)
                .or_else(|| self.table.ships.get(&ship.ship));
            let Some(((leader, berth), kind)) = self.slot_for(&ship, &placed, reserve).zip(kind)
            else {
                placed.push(None);
                continue;
            };
            let mut reserves = person.map_or_else(|| kind.stats.full(), |person| person.reserves);
            let mut condition = person.map_or(Condition::Intact, |person| person.condition);
            if ship.derelict {
                reserves.shield.now = 0.0;
                reserves.armor.now = derelict_armor(kind.stats.armor, kind.hull.tough);
                condition = Condition::Disabled;
            }
            let traits = person
                .map(|person| NpcPerson::of(&person.record, grudges.contains(&person.record.id)));
            let id = NpcId(self.next_id);
            self.next_id += 1;
            // `ShipBehav` 0 aims a mission ship at the player as it spawns
            // (`_AIMakeShipAttackPlayer` @0x3d099-0x3d0bb).
            let hunts = ship
                .mission
                .is_some_and(|mark| mark.behav == ShipBehav::AttackPlayer);
            self.quotes.extend(ship.quote);
            placed.push(Some(id));
            self.npcs.push(Npc {
                id,
                berth,
                ship: ship.ship,
                govt: ship.govt,
                ai_type: ship.ai_type,
                leader,
                class: kind.escort_class,
                escort: None,
                stats: kind.stats,
                reserves,
                state: ship.state,
                mode: match ship.entry {
                    Entry::Placed => Mode::Flying,
                    Entry::JumpingIn => Mode::JumpingIn { tick: 0 },
                    Entry::Emerging { ticks_left, .. } => Mode::Emerging { ticks_left },
                },
                goal: if hunts {
                    Goal::Attack(ShipRef::Player)
                } else {
                    Goal::Idle
                },
                condition,
                hull: kind.hull,
                armament: kind.armament.clone(),
                rounds: kind.rounds.clone(),
                trigger: Trigger::default(),
                target: hunts.then_some(ShipRef::Player),
                provoked: 0.0,
                aggression: ship.aggression,
                inspected: None,
                booty: ship.booty,
                boarded: false,
                info_types: ship.info_types,
                cargo: ship.cargo,
                spared: false,
                assisting: 0,
                carrier: None,
                person: traits,
                mission: ship.mission,
                aux: ship.aux,
                cloak: if ship.cloaked {
                    let mut cloak = Cloak::default();
                    cloak.cloak();
                    cloak
                } else {
                    Cloak::default()
                },
                cloak_gear: kind.cloak,
                led_out: false,
                emerges_from: match ship.entry {
                    Entry::Emerging { gate, .. } => Some(gate),
                    Entry::Placed | Entry::JumpingIn => None,
                },
                self_repair: kind.self_repair,
                former: None,
            });
        }
        placed.iter().flatten().count()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::WeaponId;
    use crate::catalog::{DudeId, EscortRecord, FleetId, FleetRecord, GovtId, StellarId};
    use crate::catalog::{PersonId, PersonRecord};
    use crate::cloak::{CloakGear, FADE_FULL};
    use crate::combat::ShipRef;
    use crate::combat::armament::Armament;
    use crate::combat::hull::HullSpec;
    use crate::combat::weapon::WeaponSpec;
    use crate::control::Test;
    use crate::escort::EscortClass;
    use crate::geometry::Vec2;
    use crate::hail::Help;
    use crate::person::PersonWorld;
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, planet, weapon};
    use crate::traffic::npc::{AiType, NpcPerson};
    use crate::traffic::table::{ShipKind, SpawnDude, SpawnPerson};

    /// Decides `goal`, `trigger` and `target` for everyone, recording who
    /// decided.
    #[derive(Debug)]
    struct Recording {
        goal: Goal,
        trigger: Trigger,
        target: Option<ShipRef>,
        decided: RefCell<Vec<NpcId>>,
    }

    impl Recording {
        fn deciding(goal: Goal) -> Self {
            Self {
                goal,
                trigger: Trigger::default(),
                target: None,
                decided: RefCell::default(),
            }
        }

        fn take(&self) -> Vec<NpcId> {
            std::mem::take(&mut self.decided.borrow_mut())
        }
    }

    impl Behaviour for Recording {
        fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            assert!(around.npcs.iter().any(|other| other.id == npc.id));
            self.decided.borrow_mut().push(npc.id);
            self.goal
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            self.trigger
        }

        fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
            assert!(around.npcs.iter().any(|other| other.id == npc.id));
            self.target
        }
    }

    /// Keeps every NPC's goal.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// `avg_ships` on average, all of düde 128's ship 200 (a wimpy
    /// trader, govt 130; its own `InherentAI` is 3).
    fn table(avg_ships: u32) -> SpawnTable {
        SpawnTable {
            avg_ships,
            dudes: vec![(DudeId(128), 100)],
            dude_records: BTreeMap::from([(
                DudeId(128),
                SpawnDude {
                    ai_type: 1,
                    govt: Some(GovtId(130)),
                    ships: vec![(ShipId(200), 1)],
                    booty: 0x0041,
                    info_types: 0x8000,
                },
            )]),
            ships: BTreeMap::from([(
                ShipId(200),
                ShipKind {
                    stats: ShipStats::new(FAST, &[]),
                    inherent_ai: 3,
                    escort_class: EscortClass::Medium,
                    hull: HullSpec {
                        hit_radius: 9.0,
                        ..HullSpec::default()
                    },
                    armament: Armament::new([(WeaponSpec::new(&weapon(128)), 2)]),
                    rounds: BTreeMap::from([(WeaponId(138), 7)]),
                    appear_on: Test::default(),
                    cloak: CloakGear::default(),
                    self_repair: false,
                    holds: 0,
                },
            )]),
            ..SpawnTable::default()
        }
    }

    /// One setup pass placing ship 200 in the system at
    /// (`x` - 750, `y` - 750), facing up, of aggression 0.
    fn placed(x: u32, y: u32) -> [u32; 8] {
        [6, 6, 0, 0, x, y, 0, 2]
    }

    /// Traffic deciding every `interval` ticks, entered into a system with
    /// `count` NPCs placed in it a pixel apart from the centre rightwards,
    /// brought to rest (setup starts them at their top speed).
    fn populated(count: u32, interval: u32) -> Traffic {
        let draws: Vec<u32> = (0..count).flat_map(|i| placed(750 + i, 750)).collect();
        let mut traffic = Traffic::with_interval(interval);
        traffic.enter(table(count), &mut Draws::of(&draws));
        for npc in &mut traffic.npcs {
            npc.state.velocity = Vec2::ZERO;
        }
        traffic
    }

    #[test]
    fn entering_among_stellars_places_a_still_warship_100_from_the_first() {
        // Ship 200 flown as a warship, of no speed.
        let mut warships = table(1);
        warships
            .dude_records
            .get_mut(&DudeId(128))
            .expect("there")
            .ai_type = 3;
        warships.ships.get_mut(&ShipId(200)).expect("there").stats =
            ShipStats::new(crate::handling::ShipFields { speed: 0, ..FAST }, &[]);
        let sites = [planet(128, 400.0, -300.0), planet(129, -900.0, 0.0)];
        let mut traffic = Traffic::new();
        // At angle 90 from the first stellar, facing 45.
        traffic.enter_in(
            warships.clone(),
            World::new(&sites),
            &mut Draws::of(&[6, 6, 0, 0, 90, 45, 0]),
        );
        let state = traffic.npcs()[0].state;
        assert!(
            (state.position - Vec2::new(500.0, -300.0)).length() < 1e-3,
            "{state:?}"
        );
        assert_eq!((state.velocity, state.heading), (Vec2::ZERO, 45.0));
        // Entered knowing no stellar, it is placed in the box.
        traffic.enter(warships, &mut Draws::of(&[6, 6, 0, 0, 0, 1499, 45, 0]));
        assert_eq!(traffic.npcs()[0].state.position, Vec2::new(-750.0, 749.0));
    }

    #[test]
    fn decisions_are_due_every_interval_staggered_by_npc() {
        assert!((0..10).all(|tick| decision_due(tick, NpcId(3), 1)));
        let due: Vec<u64> = (0..12)
            .filter(|&tick| decision_due(tick, NpcId(5), 4))
            .collect();
        assert_eq!(due, [1, 5, 9]);
        let due: Vec<u64> = (0..12)
            .filter(|&tick| decision_due(tick, NpcId(8), 4))
            .collect();
        assert_eq!(due, [0, 4, 8]);
        assert!(decision_due(7, NpcId(2), 0), "none apart is one apart");
        assert_eq!(DECISION_INTERVAL, 1);
    }

    #[test]
    fn entering_a_system_replaces_the_npcs_with_its_initial_population() {
        let mut traffic = populated(2, 1);
        let npcs = traffic.npcs();
        assert_eq!(npcs.len(), 2);
        assert_eq!(
            npcs.iter().map(|npc| npc.id).collect::<Vec<_>>(),
            [NpcId(0), NpcId(1)]
        );
        let first = npcs[0].clone();
        assert_eq!(first.ship, ShipId(200));
        assert_eq!(first.govt, Some(GovtId(130)));
        assert_eq!(first.ai_type, AiType::WimpyTrader);
        assert_eq!(first.class, EscortClass::Medium, "its ship type's");
        assert_eq!(first.leader, None);
        assert_eq!(first.stats, ShipStats::new(FAST, &[]));
        assert_eq!(first.reserves, first.stats.full());
        assert_eq!(first.condition, Condition::Intact);
        assert_eq!(first.hull.hit_radius, 9.0);
        assert_eq!(first.armament.mounts().len(), 1);
        assert_eq!(first.rounds, BTreeMap::from([(WeaponId(138), 7)]));
        assert_eq!(first.trigger, Trigger::default());
        assert_eq!(first.state.position, Vec2::new(0.0, 0.0));
        assert_eq!(first.mode, Mode::Flying);
        assert_eq!(first.goal, Goal::Idle);
        assert_eq!(first.booty, 0x0041, "its düde's");
        assert_eq!(first.info_types, 0x8000, "its düde's");
        assert!(!first.spared);
        assert_eq!(first.assisting, 0);
        assert!(!first.boarded);
        assert_eq!(npcs[1].state.position, Vec2::new(1.0, 0.0));
        traffic.enter(table(1), &mut Draws::of(&placed(0, 0)));
        assert_eq!(
            traffic.npcs().iter().map(|npc| npc.id).collect::<Vec<_>>(),
            [NpcId(2)],
            "replaced, numbered on"
        );
        traffic.enter(SpawnTable::default(), &mut Draws::of(&[]));
        assert_eq!(traffic.npcs(), []);
        assert_eq!(traffic.ship_types(), []);
    }

    #[test]
    fn the_ship_types_are_the_tables() {
        let mut traffic = Traffic::new();
        traffic.enter(table(0), &mut Draws::of(&[]));
        assert_eq!(traffic.ship_types(), [ShipId(200)]);
    }

    #[test]
    fn a_fleet_enters_jumping_in_with_its_escorts_following_its_lead() {
        let mut fleet_table = table(1);
        fleet_table.link_fleets.insert(FleetId(141));
        fleet_table.fleets.insert(
            FleetId(141),
            FleetRecord {
                id: FleetId(141),
                lead: Some(ShipId(200)),
                escorts: vec![EscortRecord {
                    ship: ShipId(200),
                    min: 2,
                    max: 2,
                }],
                govt: None,
                link_syst: -1,
                appear_on: Test::default(),
                quote: None,
                flags: 0,
            },
        );
        // An NPC numbered first, so the fleet's lead is NPC 1.
        let mut traffic = populated(1, 1);
        traffic.enter(fleet_table, &mut Draws::of(&[6, 0, 13]));
        let npcs = traffic.npcs();
        assert_eq!(npcs.len(), 3);
        assert_eq!(npcs[0].id, NpcId(1));
        assert_eq!(
            npcs.iter().map(|npc| npc.leader).collect::<Vec<_>>(),
            [None, Some(NpcId(1)), Some(NpcId(1))]
        );
        assert_eq!(npcs[0].ai_type, AiType::Warship, "its InherentAI");
        for npc in npcs {
            assert_eq!(npc.mode, Mode::JumpingIn { tick: 0 });
        }
    }

    #[test]
    fn an_escort_points_at_its_lead_among_the_ships_added_with_it() {
        // NPC 0 first; then a pass placing a düde ship (NPC 1), and a pass
        // bringing fleet 141 (its lead NPC 2, its escort NPC 3).
        let mut fleet_table = table(2);
        fleet_table.link_fleets.insert(FleetId(141));
        fleet_table.fleets.insert(
            FleetId(141),
            FleetRecord {
                id: FleetId(141),
                lead: Some(ShipId(200)),
                escorts: vec![EscortRecord {
                    ship: ShipId(200),
                    min: 1,
                    max: 1,
                }],
                govt: None,
                link_syst: -1,
                appear_on: Test::default(),
                quote: None,
                flags: 0,
            },
        );
        let mut traffic = populated(1, 1);
        let mut draws = placed(0, 0).to_vec();
        draws.extend([6, 0, 13]);
        traffic.enter(fleet_table, &mut Draws::of(&draws));
        assert_eq!(
            traffic
                .npcs()
                .iter()
                .map(|npc| (npc.id, npc.leader))
                .collect::<Vec<_>>(),
            [
                (NpcId(1), None),
                (NpcId(2), None),
                (NpcId(3), Some(NpcId(2)))
            ]
        );
    }

    #[test]
    fn every_npc_decides_every_tick_at_interval_1() {
        let mut traffic = populated(3, 1);
        let behaviour = Recording::deciding(Goal::Idle);
        for _ in 0..3 {
            traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
            assert_eq!(behaviour.take(), [NpcId(0), NpcId(1), NpcId(2)]);
        }
    }

    #[test]
    fn at_interval_4_each_npc_decides_on_its_own_ticks() {
        let mut traffic = populated(5, 4);
        let behaviour = Recording::deciding(Goal::Idle);
        let decided: Vec<Vec<NpcId>> = (0..5)
            .map(|_| {
                traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
                behaviour.take()
            })
            .collect();
        assert_eq!(
            decided,
            [
                vec![NpcId(0), NpcId(4)],
                vec![NpcId(1)],
                vec![NpcId(2)],
                vec![NpcId(3)],
                vec![NpcId(0), NpcId(4)],
            ]
        );
    }

    #[test]
    fn a_ship_jumping_in_never_decides_while_it_glides_and_holds_its_course() {
        let mut traffic = populated(1, 1);
        traffic.table.avg_ships = 2;
        let behaviour = Recording::deciding(Goal::Idle);
        // An arrival: 0, not a person, not a fleet, düde 128, ship 200, at
        // angle 0.
        traffic.tick(&behaviour, &[], &mut Draws::of(&[0, 6, 6, 0, 0, 0]));
        assert_eq!(traffic.npcs().len(), 2);
        assert_eq!(behaviour.take(), [NpcId(0)], "the newcomer is jumping in");
        let mut coasted = 0;
        while traffic.npcs()[1].mode != Mode::Flying {
            traffic.tick(&behaviour, &[], &mut Draws::of(&[2]));
            assert_eq!(behaviour.take(), [NpcId(0)]);
            if let Mode::Coasting { .. } = traffic.npcs()[1].mode {
                coasted += 1;
            }
        }
        // With no lead it waits 30 + Rand(30), the draw 2.
        assert_eq!(coasted, 32);
        traffic.tick(&behaviour, &[], &mut Draws::of(&[2]));
        assert_eq!(behaviour.take(), [NpcId(0), NpcId(1)]);
    }

    #[test]
    fn a_ship_coming_out_of_a_gate_never_decides_until_it_is_out() {
        let mut traffic = populated(2, 1);
        traffic.npcs[1].mode = Mode::Emerging { ticks_left: 2 };
        let firing = Trigger {
            primary: true,
            ..Trigger::default()
        };
        let behaviour = Recording {
            trigger: firing,
            target: Some(ShipRef::Player),
            ..Recording::deciding(Goal::JumpOut)
        };
        let mut ticks = 0;
        while traffic.npcs()[1].mode != Mode::Flying {
            traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
            assert_eq!(behaviour.take(), [NpcId(0)], "tick {ticks}");
            assert_eq!(traffic.npcs()[1].goal, Goal::Idle);
            assert_eq!(
                traffic.npcs()[1].trigger,
                Trigger::default(),
                "tick {ticks}"
            );
            assert_eq!(traffic.npcs()[1].target, None, "tick {ticks}");
            ticks += 1;
            assert!(ticks < 100, "out");
        }
        assert_eq!(traffic.npcs()[0].trigger, firing, "the flying ship decides");
        // Two ticks waiting; top speed 6: 30 down to 9 is 8 ticks; then,
        // with no lead, 30 + Rand(30) holding its course, the draw its
        // last, 29.
        assert_eq!(ticks, 2 + 8 + 59);
        traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
        assert_eq!(behaviour.take(), [NpcId(0), NpcId(1)]);
        assert_eq!(traffic.npcs()[1].trigger, firing, "its first decision");
    }

    /// A planet at the centre and a hypergate at (300, -400) heading ships
    /// out on 90°.
    fn gated() -> [LandingSite; 2] {
        [
            planet(128, 0.0, 0.0),
            LandingSite {
                flags2: crate::gate::HYPERGATE,
                exit_angle: 90,
                ..planet(129, 300.0, -400.0)
            },
        ]
    }

    #[test]
    fn an_arrival_among_a_hypergate_comes_out_of_it_and_holds_its_course_once_out() {
        let mut traffic = populated(1, 1);
        traffic.table.avg_ships = 2;
        let sites = gated();
        // An arrival: 0, not a person, not a fleet, düde 128, ship 200, at
        // angle 0, then the gate.
        traffic.tick(&Keep, &sites, &mut Draws::of(&[0, 6, 6, 0, 0, 0, 1]));
        let arrived = &traffic.npcs()[1];
        assert_eq!(arrived.mode, Mode::Emerging { ticks_left: 59 });
        assert_eq!(arrived.emerges_from, Some(StellarId(129)));
        assert_eq!(arrived.trigger, Trigger::default(), "it holds no trigger");
        assert_eq!(arrived.target, None);
        assert_eq!(arrived.state.position, Vec2::new(300.0, -400.0));
        assert_eq!(arrived.state.heading, 90.0);
        for _ in 1..emerge::EMERGE_TICKS {
            traffic.tick(&Keep, &sites, &mut Draws::of(&[2]));
        }
        assert_eq!(traffic.npcs()[1].mode, Mode::LeavingGate { tick: 0 });
        assert_eq!(traffic.npcs()[1].state.position, Vec2::new(300.0, -400.0));
        assert_eq!(traffic.npcs()[1].emerges_from, Some(StellarId(129)));
        for _ in 0..8 {
            traffic.tick(&Keep, &sites, &mut Draws::of(&[2]));
        }
        let out = &traffic.npcs()[1];
        assert_eq!(
            out.mode,
            Mode::Coasting { ticks_left: 32 },
            "top speed 6: 30 down to 9, then with no lead 30 + Rand(30), the draw 2"
        );
        assert_eq!(out.emerges_from, None, "the burst over");
        assert!((out.state.position - Vec2::new(456.0, -400.0)).length() < 1e-3);
    }

    #[test]
    fn a_linksyst_fleet_entering_among_a_hypergate_may_come_out_of_it() {
        let mut fleet_table = table(1);
        fleet_table.link_fleets.insert(FleetId(141));
        fleet_table.fleets.insert(
            FleetId(141),
            FleetRecord {
                id: FleetId(141),
                lead: Some(ShipId(200)),
                escorts: vec![EscortRecord {
                    ship: ShipId(200),
                    min: 1,
                    max: 1,
                }],
                govt: None,
                link_syst: -1,
                appear_on: Test::default(),
                quote: None,
                flags: 0,
            },
        );
        let sites = gated();
        let mut traffic = Traffic::new();
        // Not a person, a fleet, slot 13, the gate, the lead's aggression,
        // its escort's offsets and extra wait.
        traffic.enter_in(
            fleet_table,
            World::new(&sites),
            &mut Draws::of(&[6, 0, 13, 1, 0, 0, 0, 3]),
        );
        assert_eq!(
            traffic
                .npcs()
                .iter()
                .map(|npc| npc.mode)
                .collect::<Vec<_>>(),
            [
                Mode::Emerging { ticks_left: 60 },
                Mode::Emerging { ticks_left: 68 }
            ]
        );
        assert!(
            traffic
                .npcs()
                .iter()
                .all(|npc| npc.emerges_from == Some(StellarId(129)))
        );
    }

    #[test]
    fn decisions_set_goals_that_the_autopilot_flies() {
        let mut traffic = populated(1, 1);
        let lander = Recording::deciding(Goal::Land(StellarId(140)));
        let sites = [planet(140, 0.0, -300.0)];
        traffic.tick(&lander, &sites, &mut Draws::of(&[]));
        let npc = &traffic.npcs()[0];
        assert_eq!(npc.goal, Goal::Land(StellarId(140)));
        assert_ne!(npc.state, ShipState::default(), "it moved off at once");
    }

    #[test]
    fn a_ship_that_lands_or_jumps_out_leaves() {
        // NPC 0 over planet 140, at rest; NPC 1 out at its jump distance.
        let mut traffic = populated(2, 1);
        traffic.npcs[1].state.position = Vec2::new(0.0, -1000.0);
        traffic.npcs[0].goal = Goal::Land(StellarId(140));
        traffic.npcs[1].goal = Goal::JumpOut;
        let sites = [planet(140, 0.0, 0.0)];
        traffic.tick(&Keep, &sites, &mut Draws::of(&[]));
        assert_eq!(traffic.npcs(), []);
        assert_eq!(
            traffic.departed(),
            [
                (NpcId(0), Outcome::Landed(StellarId(140))),
                (NpcId(1), Outcome::JumpedOut)
            ]
        );
        traffic.tick(&Keep, &sites, &mut Draws::of(&[]));
        assert_eq!(traffic.departed(), [], "only the last tick's");
    }

    #[test]
    fn the_fire_command_decided_is_held_on_the_ai_timer() {
        let mut traffic = populated(2, 2);
        let behaviour = Recording {
            trigger: Trigger {
                primary: true,
                secondary: Some(WeaponId(138)),
                only: None,
                turrets_only: false,
                bays: false,
                point_defence: false,
            },
            ..Recording::deciding(Goal::Idle)
        };
        traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
        assert_eq!(behaviour.take(), [NpcId(0)]);
        assert_eq!(traffic.npcs()[0].trigger, behaviour.trigger);
        assert_eq!(
            traffic.npcs()[1].trigger,
            Trigger::default(),
            "not its turn"
        );
        traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[1].trigger, behaviour.trigger);
    }

    #[test]
    fn the_target_decided_is_held_on_the_ai_timer() {
        let mut traffic = populated(2, 2);
        assert_eq!(traffic.npcs()[0].target, None, "none to start with");
        let behaviour = Recording {
            target: Some(ShipRef::Player),
            ..Recording::deciding(Goal::Idle)
        };
        traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].target, Some(ShipRef::Player));
        assert_eq!(traffic.npcs()[1].target, None, "not its turn");
        traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[1].target, Some(ShipRef::Player));
        let letting_go = Recording::deciding(Goal::Idle);
        traffic.tick(&letting_go, &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].target, None);
        assert_eq!(traffic.npcs()[1].target, Some(ShipRef::Player));
    }

    #[test]
    fn an_npc_not_intact_decides_nothing_drifts_and_never_lands_or_jumps() {
        // NPC 0 over planet 140 bent on landing; NPC 1 out at its jump
        // distance bent on jumping; both drifting right.
        let mut traffic = populated(2, 1);
        traffic.npcs[1].state.position = Vec2::new(0.0, -1000.0);
        traffic.npcs[0].goal = Goal::Land(StellarId(140));
        traffic.npcs[1].goal = Goal::JumpOut;
        for npc in traffic.npcs_mut() {
            npc.state.velocity = Vec2::new(0.25, 0.0);
            npc.condition = Condition::Disabled;
        }
        let behaviour = Recording::deciding(Goal::Idle);
        let sites = [planet(140, 0.0, 0.0)];
        for _ in 0..4 {
            traffic.tick(&behaviour, &sites, &mut Draws::of(&[]));
        }
        assert_eq!(behaviour.take(), [], "no decisions");
        assert_eq!(traffic.departed(), []);
        assert_eq!(traffic.npcs().len(), 2);
        assert_eq!(traffic.npcs()[0].state.position, Vec2::new(1.0, 0.0));
        assert_eq!(traffic.npcs()[1].goal, Goal::JumpOut);
        traffic.npcs_mut()[0].condition = Condition::Dying { ticks_left: 2 };
        traffic.tick(&behaviour, &sites, &mut Draws::of(&[]));
        assert_eq!(behaviour.take(), []);
    }

    #[test]
    fn an_npc_is_taken_out_by_its_number() {
        let mut traffic = populated(3, 1);
        traffic.remove(NpcId(1));
        assert_eq!(
            traffic.npcs().iter().map(|npc| npc.id).collect::<Vec<_>>(),
            [NpcId(0), NpcId(2)]
        );
        traffic.remove(NpcId(9));
        assert_eq!(traffic.npcs().len(), 2, "not there");
    }

    #[test]
    fn arrivals_come_on_their_roll_up_to_avg_ships() {
        let mut traffic = Traffic::new();
        traffic.enter(table(1), &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs(), [], "a person");
        let behaviour = Recording::deciding(Goal::Idle);
        let mut chance = Draws::of(&[2]);
        traffic.tick(&behaviour, &[], &mut chance);
        assert_eq!((traffic.npcs().len(), chance.asked.len()), (0, 1));
        let mut chance = Draws::of(&[0, 6, 6, 0, 0, 90]);
        traffic.tick(&behaviour, &[], &mut chance);
        assert_eq!(traffic.npcs().len(), 1);
        let mut chance = Draws::of(&[0]);
        traffic.tick(&behaviour, &[], &mut chance);
        assert!(chance.asked.is_empty(), "at AvgShips, no roll");
    }

    /// The player's escort duty on slot `slot`.
    fn duty(slot: u8) -> crate::escort::EscortDuty {
        crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot,
                ships: 3,
                spacing: 30.0,
            },
            order: None,
        }
    }

    /// The number of NPCs that are not the player's.
    fn strangers(traffic: &Traffic) -> usize {
        traffic
            .npcs()
            .iter()
            .filter(|npc| npc.escort.is_none())
            .count()
    }

    /// Ticks once on the draws that bring ship 200 in on its roll; what
    /// was asked.
    fn tick_arrival(traffic: &mut Traffic) -> Vec<u32> {
        let mut chance = Draws::of(&[0, 6, 6, 0, 0, 90]);
        traffic.tick(&Recording::deciding(Goal::Idle), &[], &mut chance);
        chance.asked
    }

    #[test]
    fn escorts_do_not_count_toward_avg_ships() {
        // `_EnterMoreShips` (0x43b14) leaves out ships whose parent
        // (+0x9a) is the player.
        let mut traffic = Traffic::new();
        traffic.enter(table(1), &mut Draws::of(&[0]));
        traffic.add_npc(Npc {
            escort: Some(duty(2)),
            ..crate::testkit::npc(0, ShipStats::default())
        });
        let asked = tick_arrival(&mut traffic);
        assert_eq!(asked.first(), Some(&spawn::ARRIVAL_ODDS), "rolled");
        assert_eq!(strangers(&traffic), 1);
    }

    #[test]
    fn a_mission_escort_does_not_count_toward_avg_ships() {
        // Its leader (+0x9a) is the player too (`_SetupShipsInSystem`
        // @0x4309f-0x430ef), so `_EnterMoreShips` leaves it out as well.
        let mut traffic = Traffic::new();
        traffic.enter(table(1), &mut Draws::of(&[0]));
        traffic.add_npc(Npc {
            mission: Some(npc::MissionMark {
                order: None,
                slot: 0,
                behav: ShipBehav::EscortPlayer,
                formation: Some(duty(2).formation),
            }),
            ..crate::testkit::npc(0, ShipStats::default())
        });
        let asked = tick_arrival(&mut traffic);
        assert_eq!(asked.first(), Some(&spawn::ARRIVAL_ODDS), "rolled");
        assert_eq!(traffic.npcs().len(), 2);
    }

    #[test]
    fn a_system_fills_to_avg_ships_beside_the_players_escorts() {
        let mut traffic = Traffic::new();
        traffic.enter(table(2), &mut Draws::of(&[0, 0]));
        for slot in [2, 3] {
            traffic.add_npc(Npc {
                escort: Some(duty(slot)),
                ..crate::testkit::npc(0, ShipStats::default())
            });
        }
        for _ in 0..2 {
            assert_eq!(tick_arrival(&mut traffic).first(), Some(&500));
        }
        assert_eq!((strangers(&traffic), traffic.npcs().len()), (2, 4));
        assert!(
            tick_arrival(&mut traffic).is_empty(),
            "at AvgShips, no roll"
        );
    }

    /// The carrier of a fighter launched from `ship`'s bay.
    fn launched_from(ship: ShipRef) -> crate::bay::Carrier {
        crate::bay::Carrier {
            ship,
            window: 100.0,
            reach: 30.0,
        }
    }

    /// The player's fighters are not counted: `_LaunchFighter` (0x3d870)
    /// gives a fighter its carrier's index as its parent (+0x9a), and the
    /// player's launches (`_FirePlayerWeapon` 0x62c61) come from ship 0,
    /// the player, which `_EnterMoreShips` (0x43b14) leaves out.
    #[test]
    fn the_players_fighters_do_not_count_toward_avg_ships() {
        let mut traffic = Traffic::new();
        traffic.enter(table(1), &mut Draws::of(&[0]));
        traffic.add_npc(Npc {
            escort: Some(duty(2)),
            carrier: Some(launched_from(ShipRef::Player)),
            ..crate::testkit::npc(0, ShipStats::default())
        });
        assert_eq!(tick_arrival(&mut traffic).first(), Some(&500), "rolled");
        assert_eq!(strangers(&traffic), 1);
    }

    /// An NPC carrier's fighters are counted: `_LaunchFighter` (0x3d870)
    /// gives a fighter its carrier's index as its parent (+0x9a), and an
    /// NPC carrier's launches (`_AILaunchFighter` 0x8141c) come from a
    /// ship of index 1 or more, which `_EnterMoreShips` (0x43b14) counts,
    /// unlike the player's (`_FirePlayerWeapon` 0x62c61).
    #[test]
    fn an_npc_carriers_fighters_count_toward_avg_ships() {
        let mut traffic = Traffic::new();
        traffic.enter(table(2), &mut Draws::of(&[0, 0]));
        let carrier = traffic.add_npc(crate::testkit::npc(0, ShipStats::default()));
        let fighter = traffic.add_npc(Npc {
            leader: Some(carrier),
            carrier: Some(launched_from(ShipRef::Npc(carrier))),
            ..crate::testkit::npc(0, ShipStats::default())
        });
        assert!(
            tick_arrival(&mut traffic).is_empty(),
            "at AvgShips, no roll"
        );
        traffic.remove(fighter);
        assert_eq!(tick_arrival(&mut traffic).first(), Some(&500), "rolled");
    }

    /// A mark of a `ShipBehav` 1 mission ship, joined to the player's
    /// formation or not.
    fn escorting(joined: bool) -> npc::MissionMark {
        npc::MissionMark {
            order: None,
            slot: 0,
            behav: ShipBehav::EscortPlayer,
            formation: joined.then(|| duty(2).formation),
        }
    }

    #[test]
    fn the_player_leads_its_escorts_fighters_and_joined_mission_escorts_in_ship_table_order() {
        let mut traffic = Traffic::new();
        let ship = |id| crate::testkit::npc(id, ShipStats::default());
        traffic.add_npc(ship(0));
        let mission = traffic.add_npc(Npc {
            mission: Some(escorting(true)),
            ..ship(0)
        });
        traffic.add_npc(Npc {
            mission: Some(escorting(false)),
            ..ship(0)
        });
        let hired = traffic.add_npc(Npc {
            escort: Some(duty(2)),
            ..ship(0)
        });
        let carrier = traffic.add_npc(ship(0));
        traffic.add_npc(Npc {
            leader: Some(carrier),
            carrier: Some(launched_from(ShipRef::Npc(carrier))),
            ..ship(0)
        });
        let fighter = traffic.add_npc(Npc {
            escort: Some(duty(3)),
            carrier: Some(launched_from(ShipRef::Player)),
            ..ship(0)
        });
        let led: Vec<NpcId> = traffic.player_led().map(|npc| npc.id).collect();
        assert_eq!(led, [mission, hired, fighter]);
        assert_eq!(traffic.strangers(), 4, "the rest");
    }

    /// The berths of the system's NPCs, in `Vec` order.
    fn berths(traffic: &Traffic) -> Vec<u32> {
        traffic.npcs().iter().map(|npc| npc.berth).collect()
    }

    #[test]
    fn each_npc_takes_the_first_free_berth_from_1() {
        let mut traffic = Traffic::new();
        let ship = || crate::testkit::npc(0, ShipStats::default());
        traffic.add_npc(ship());
        let second = traffic.add_npc(ship());
        traffic.add_npc(ship());
        assert_eq!(berths(&traffic), [1, 2, 3]);
        traffic.remove(second);
        traffic.add_npc(ship());
        traffic.add_npc(ship());
        assert_eq!(berths(&traffic), [1, 3, 2, 4]);
        traffic.remove(NpcId(0));
        traffic.table = table(0);
        traffic.bring_in_within(
            vec![mission_ship(-1, 0.0), mission_ship(-1, 1.0)],
            SPAWN_RESERVE,
        );
        assert_eq!(berths(&traffic), [3, 2, 4, 1, 5], "brought in alike");
    }

    #[test]
    fn a_systems_ships_take_berths_from_1_as_they_are_set_up() {
        let mut traffic = Traffic::new();
        traffic.add_npc(crate::testkit::npc(0, ShipStats::default()));
        traffic.enter_with(
            table(1),
            World::new(&[]),
            vec![mission_ship(-1, 5.0), mission_ship(-1, 6.0)],
            &mut Draws::of(&placed(750, 750)),
        );
        assert_eq!(berths(&traffic), [1, 2, 3]);
    }

    #[test]
    fn a_reserved_berth_is_skipped_until_an_npc_is_placed_at_it() {
        let mut traffic = Traffic::new();
        let ship = || crate::testkit::npc(0, ShipStats::default());
        traffic.reserve(BTreeSet::from([1, 3]));
        traffic.add_npc(ship());
        traffic.add_npc(ship());
        assert_eq!(berths(&traffic), [2, 4]);
        let placed_at = traffic.add_npc_at(ship(), 1);
        assert_eq!(traffic.npcs()[2].id, placed_at);
        assert_eq!(berths(&traffic), [2, 4, 1]);
        traffic.add_npc(ship());
        assert_eq!(
            berths(&traffic),
            [2, 4, 1, 5],
            "1 is held, 3 still reserved"
        );
        traffic.reserve(BTreeSet::from([2]));
        traffic.enter_with(
            table(0),
            World::new(&[]),
            vec![mission_ship(-1, 0.0), mission_ship(-1, 1.0)],
            &mut Draws::of(&[]),
        );
        assert_eq!(
            berths(&traffic),
            [1, 3],
            "the reservation outlives the system"
        );
    }

    #[test]
    fn player_led_follows_berth_order() {
        let mut traffic = Traffic::new();
        let ship = |id| crate::testkit::npc(id, ShipStats::default());
        traffic.reserve(BTreeSet::from([1]));
        let hired = traffic.add_npc(Npc {
            escort: Some(duty(2)),
            ..ship(0)
        });
        let mission = traffic.add_npc_at(
            Npc {
                mission: Some(escorting(true)),
                ..ship(0)
            },
            1,
        );
        let led: Vec<NpcId> = traffic.player_led().map(|npc| npc.id).collect();
        assert_eq!(led, [mission, hired]);
    }

    #[test]
    fn ships_led_at_one_berth_keep_the_order_they_appeared_in() {
        let mut traffic = Traffic::new();
        let ship = |id| crate::testkit::npc(id, ShipStats::default());
        let first = traffic.add_npc_at(
            Npc {
                escort: Some(duty(2)),
                ..ship(0)
            },
            7,
        );
        let second = traffic.add_npc_at(
            Npc {
                mission: Some(escorting(true)),
                ..ship(0)
            },
            7,
        );
        let led: Vec<NpcId> = traffic.player_led().map(|npc| npc.id).collect();
        assert_eq!(led, [first, second]);
    }

    #[test]
    fn an_escort_heads_for_its_lead() {
        // NPC 1 escorts NPC 0, from 500 to its right.
        let mut traffic = populated(2, 1);
        traffic.npcs[1].state.position = Vec2::new(500.0, 0.0);
        traffic.npcs[1].leader = Some(NpcId(0));
        let sites = [planet(140, 0.0, -5000.0)];
        traffic.tick(&crate::ai::Peaceful, &sites, &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs()[1].goal, Goal::Follow(NpcId(0)));
        for _ in 0..60 {
            traffic.tick(&crate::ai::Peaceful, &sites, &mut Draws::of(&[]));
        }
        let escort = &traffic.npcs()[1];
        assert!(escort.state.position.x < 450.0, "{:?}", escort.state);
    }

    #[test]
    fn each_npc_keeps_the_aggression_it_drew() {
        let mut traffic = Traffic::new();
        traffic.enter(table(1), &mut Draws::of(&[6, 6, 0, 0, 0, 0, 0, 1]));
        assert_eq!(traffic.npcs()[0].aggression, 3);
        assert_eq!(traffic.npcs()[0].provoked, 0.0);
        assert_eq!(traffic.npcs()[0].inspected, None);
    }

    // Fights.

    use crate::ai::{PlayerSide, Reaction};
    use crate::combat::Strike;

    /// Records the order of its reactions and decisions; reacts to a
    /// strike on an NPC by taking its attacker, provoked by the damage;
    /// decides `goal`; and fires and targets as the goal it was given
    /// says.
    #[derive(Debug)]
    struct Fighting {
        goal: Goal,
        log: RefCell<Vec<String>>,
    }

    impl Fighting {
        fn deciding(goal: Goal) -> Self {
            Self {
                goal,
                log: RefCell::default(),
            }
        }
    }

    impl Behaviour for Fighting {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.log.borrow_mut().push(format!(
                "decide {} {:?} {}",
                npc.id.0, npc.target, npc.provoked
            ));
            self.goal
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            self.log
                .borrow_mut()
                .push(format!("trigger {} {:?}", npc.id.0, npc.goal));
            Trigger::default()
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.goal.quarry()
        }

        fn react(&self, npc: &Npc, strike: &Strike, _around: &Surroundings) -> Reaction {
            self.log
                .borrow_mut()
                .push(format!("react {} {:?}", npc.id.0, strike.ship));
            if strike.ship == ShipRef::Npc(npc.id) {
                Reaction {
                    target: Some(strike.by),
                    provoked: strike.damage,
                    goal: None,
                }
            } else {
                Reaction::default()
            }
        }
    }

    fn struck(ship: u32, damage: f32) -> Strike {
        Strike {
            ship: ShipRef::Npc(NpcId(ship)),
            by: ShipRef::Player,
            damage,
            downed: None,
        }
    }

    #[test]
    fn strikes_are_answered_before_the_decisions_and_goals_before_the_triggers() {
        let mut traffic = populated(2, 1);
        let fighting = Fighting::deciding(Goal::Attack(ShipRef::Player));
        traffic.tick_in(
            &fighting,
            World::new(&[]),
            &[struck(1, 4.0)],
            &mut Draws::of(&[]),
        );
        assert_eq!(
            *fighting.log.borrow(),
            [
                "react 0 Npc(NpcId(1))",
                "react 1 Npc(NpcId(1))",
                "decide 0 None 0",
                "decide 1 Some(Player) 4",
                "trigger 0 Attack(Player)",
                "trigger 1 Attack(Player)",
            ]
        );
        assert_eq!(traffic.npcs()[1].target, Some(ShipRef::Player));
        assert_eq!(traffic.npcs()[1].provoked, 4.0, "kept while it fights");
    }

    #[test]
    fn an_assisting_npc_unprovoked_is_never_due_and_keeps_its_help() {
        let mut traffic = populated(2, 1);
        traffic.npcs[0].goal = Goal::Assist(Help::Refuel);
        traffic.npcs[0].target = Some(ShipRef::Player);
        let fighting = Fighting::deciding(Goal::Idle);
        for _ in 0..30 {
            traffic.tick_in(&fighting, World::new(&[]), &[], &mut Draws::of(&[]));
        }
        let decided: Vec<String> = fighting
            .log
            .borrow()
            .iter()
            .filter(|line| line.starts_with("decide") || line.starts_with("trigger"))
            .cloned()
            .collect();
        assert!(
            decided.iter().all(|line| !line.contains(" 0 ")),
            "{decided:?}"
        );
        assert!(decided.iter().any(|line| line.starts_with("decide 1")));
        let helper = &traffic.npcs()[0];
        assert_eq!(helper.goal, Goal::Assist(Help::Refuel));
        assert_eq!(helper.target, Some(ShipRef::Player));
        assert_eq!(helper.trigger, Trigger::default());
    }

    #[test]
    fn a_strike_that_provokes_an_assisting_npc_ends_its_help() {
        let mut traffic = populated(1, 1);
        traffic.npcs[0].goal = Goal::Assist(Help::Repair);
        let fleeing = Fighting::deciding(Goal::Flee(ShipRef::Player));
        traffic.tick_in(
            &fleeing,
            World::new(&[]),
            &[struck(0, 0.0)],
            &mut Draws::of(&[]),
        );
        assert_eq!(
            traffic.npcs()[0].goal,
            Goal::Assist(Help::Repair),
            "a strike of no damage provokes nothing"
        );
        traffic.tick_in(
            &fleeing,
            World::new(&[]),
            &[struck(0, 2.0)],
            &mut Draws::of(&[]),
        );
        assert_eq!(traffic.npcs()[0].goal, Goal::Flee(ShipRef::Player));
    }

    #[test]
    fn an_assisting_npc_flies_to_the_player() {
        let mut traffic = populated(1, 1);
        traffic.npcs[0].goal = Goal::Assist(Help::Refuel);
        traffic.npcs[0].state.position = Vec2::new(-600.0, 0.0);
        let player = PlayerSide {
            state: ShipState::default(),
            condition: Condition::Intact,
            reserves: crate::reserves::Reserves::full(1.0, 1.0, 1.0),
            hull: HullSpec::default(),
            handling: ShipStats::new(FAST, &[]).handling,
            rating: 0,
            yardstick: 0,
            untargetable: false,
            blessing: crate::rank::Blessing::NONE,
        };
        let world = World {
            player: Some(player),
            ..World::new(&[])
        };
        for _ in 0..60 {
            traffic.tick_in(&Keep, world, &[], &mut Draws::of(&[]));
        }
        assert!(
            traffic.npcs()[0].state.position.x > -500.0,
            "{:?}",
            traffic.npcs()[0].state
        );
    }

    #[test]
    fn an_npc_added_is_numbered_after_every_other() {
        let mut traffic = populated(2, 1);
        let added = traffic.add_npc(crate::testkit::npc(77, ShipStats::default()));
        assert_eq!(added, NpcId(2));
        assert_eq!(traffic.npcs()[2].id, NpcId(2));
        traffic.enter(table(1), &mut Draws::of(&placed(0, 0)));
        assert_eq!(traffic.npcs()[0].id, NpcId(3), "never reused");
        assert_eq!(
            traffic.add_npc(crate::testkit::npc(0, ShipStats::default())),
            NpcId(4)
        );
    }

    #[test]
    fn an_escort_keeps_its_slot_beside_the_player() {
        let mut traffic = populated(1, 1);
        traffic.npcs[0].goal = Goal::Formation { guard: None };
        traffic.npcs[0].escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order: None,
        });
        // The escort at the centre, on slot 2, behind and left of the
        // player.
        let mut player = player_at(30.0, -30.0);
        player.state.velocity = Vec2::new(1.0, 0.0);
        let world = World {
            player: Some(player),
            ..World::new(&[])
        };
        traffic.tick_in(&Keep, world, &[], &mut Draws::of(&[]));
        let escort = &traffic.npcs()[0];
        assert_eq!(escort.state.velocity, player.state.velocity, "in formation");
    }

    /// Hypergate 300 at the centre, 100 wide: a reach of 25.
    fn gate_at_centre() -> LandingSite {
        LandingSite {
            flags2: crate::gate::HYPERGATE,
            ..planet(300, 0.0, 0.0)
        }
    }

    const GATE: StellarId = StellarId(300);

    #[test]
    fn a_lead_reaching_its_gates_box_releases_its_followers_to_enter_on_their_own() {
        // NPC 0, an interceptor, leads 1 and its fighter 2; NPC 3 is led
        // by another ship.
        let mut traffic = populated(4, 1);
        let lead = NpcId(0);
        traffic.npcs[0].ai_type = AiType::Interceptor;
        traffic.npcs[0].goal = Goal::EnterGate(GATE);
        for follower in &mut traffic.npcs[1..3] {
            follower.leader = Some(lead);
            follower.goal = Goal::Follow(lead);
            follower.target = Some(ShipRef::Player);
            follower.provoked = 5.0;
        }
        traffic.npcs[2].carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Npc(lead),
            window: 100.0,
            reach: 30.0,
        });
        traffic.npcs[3].leader = Some(NpcId(9));
        traffic.npcs[3].goal = Goal::Follow(NpcId(9));
        let untouched = traffic.npcs[3].clone();
        let sites = [gate_at_centre()];
        traffic.tick(&Keep, &sites, &mut Draws::of(&[]));
        assert!(matches!(traffic.npcs[0].mode, Mode::EnteringGate { .. }));
        for follower in &traffic.npcs[1..3] {
            assert_eq!(
                (follower.leader, follower.carrier, follower.target),
                (None, None, None),
                "{:?}",
                follower.id
            );
            assert_eq!(follower.provoked, 0.0);
            assert_eq!(follower.goal, Goal::EnterGate(GATE));
            assert_eq!(follower.ai_type, AiType::Warship, "an interceptor's");
            assert_eq!(follower.mode, Mode::Flying, "it enters on its own");
        }
        assert_eq!(traffic.npcs[3].goal, untouched.goal);
        assert_eq!(traffic.npcs[3].leader, untouched.leader);
        let mut left = Vec::new();
        for _ in 0..40 {
            traffic.tick(&Keep, &sites, &mut Draws::of(&[]));
            left.extend(traffic.departed().iter().copied());
        }
        assert_eq!(
            left,
            [
                (NpcId(0), Outcome::EnteredGate(GATE)),
                (NpcId(1), Outcome::EnteredGate(GATE)),
                (NpcId(2), Outcome::EnteredGate(GATE))
            ]
        );
        assert_eq!(traffic.npcs().len(), 1, "the other lead's");
    }

    #[test]
    fn a_trader_leads_followers_that_keep_its_type_and_a_lead_already_entering_releases_none() {
        let mut traffic = populated(2, 1);
        traffic.npcs[0].ai_type = AiType::BraveTrader;
        traffic.npcs[0].goal = Goal::EnterGate(GATE);
        traffic.npcs[1].leader = Some(NpcId(0));
        let sites = [gate_at_centre()];
        traffic.tick(&Keep, &sites, &mut Draws::of(&[]));
        assert_eq!(traffic.npcs[1].ai_type, AiType::BraveTrader);
        assert_eq!(traffic.npcs[1].leader, None);
        // A ship that joins its lead once the lead is entering is not
        // released: the release runs on the tick the lead reaches the box.
        traffic.npcs[1].leader = Some(NpcId(0));
        traffic.npcs[1].goal = Goal::Follow(NpcId(0));
        traffic.tick(&Keep, &sites, &mut Draws::of(&[]));
        assert_eq!(traffic.npcs[1].leader, Some(NpcId(0)));
        assert_eq!(traffic.npcs[1].goal, Goal::Follow(NpcId(0)));
    }

    /// Answers every hit on itself by attacking the player.
    #[derive(Debug)]
    struct Turning;

    impl Behaviour for Turning {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn react(&self, npc: &Npc, strike: &Strike, _around: &Surroundings) -> Reaction {
            if strike.ship == ShipRef::Npc(npc.id) {
                Reaction {
                    target: Some(strike.by),
                    provoked: strike.damage,
                    goal: Some(Goal::Attack(ShipRef::Player)),
                }
            } else {
                Reaction::default()
            }
        }
    }

    #[test]
    fn a_ship_entering_a_gate_decides_nothing_but_is_live_and_answers_a_hit() {
        let mut traffic = populated(1, 1);
        traffic.npcs[0].goal = Goal::EnterGate(GATE);
        traffic.npcs[0].mode = Mode::EnteringGate {
            gate: GATE,
            ticks_left: 4,
        };
        let sites = [gate_at_centre()];
        let recording = Recording::deciding(Goal::JumpOut);
        traffic.tick(&recording, &sites, &mut Draws::of(&[]));
        assert_eq!(recording.take(), [], "never asked to decide");
        let around = World::new(&sites).around(traffic.npcs());
        assert!(around.live(ShipRef::Npc(NpcId(0))));
        traffic.tick_in(
            &Turning,
            World::new(&sites),
            &[struck(0, 3.0)],
            &mut Draws::of(&[]),
        );
        let npc = &traffic.npcs()[0];
        assert_eq!(npc.target, Some(ShipRef::Player));
        assert_eq!(npc.provoked, 3.0);
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
        traffic.tick(&Turning, &sites, &mut Draws::of(&[]));
        traffic.tick(&Turning, &sites, &mut Draws::of(&[]));
        assert_eq!(traffic.departed(), [(NpcId(0), Outcome::EnteredGate(GATE))]);
    }

    #[test]
    fn a_fighter_docks_with_its_carrier_and_leaves_the_system() {
        let mut traffic = populated(3, 1);
        let carrier = traffic.npcs[1].id;
        traffic.npcs[1].state.position = Vec2::new(500.0, 0.0);
        let fighter = &mut traffic.npcs[0];
        let docker = fighter.id;
        fighter.state.position = Vec2::new(480.0, 10.0);
        fighter.goal = Goal::Dock(ShipRef::Npc(carrier));
        fighter.carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Npc(carrier),
            window: 100.0,
            reach: 30.0,
        });
        // The player's fighter, its carrier the player, far off.
        let player = traffic.npcs[2].id;
        traffic.npcs[2].state.position = Vec2::new(-900.0, 0.0);
        traffic.npcs[2].goal = Goal::Dock(ShipRef::Player);
        traffic.npcs[2].carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Player,
            window: 100.0,
            reach: 30.0,
        });
        let world = World {
            player: Some(player_at(0.0, 0.0)),
            ..World::new(&[])
        };
        traffic.tick_in(&Keep, world, &[], &mut Draws::of(&[]));
        assert_eq!(
            traffic.departed(),
            [(
                docker,
                Outcome::Docked {
                    carrier: ShipRef::Npc(carrier),
                    ship: ShipId(200)
                }
            )]
        );
        let left: Vec<NpcId> = traffic.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(left, [carrier, player]);
        let returning = &traffic.npcs()[1];
        assert!(
            returning.state.heading > 0.0,
            "turning to fly at the player: {:?}",
            returning.state
        );
    }

    #[test]
    fn a_reaction_may_set_a_goal_at_once() {
        /// Attacks the player as soon as anything is hit.
        #[derive(Debug)]
        struct Samaritan;
        impl Behaviour for Samaritan {
            fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
                npc.goal
            }
            fn react(&self, _: &Npc, _: &Strike, _: &Surroundings) -> Reaction {
                Reaction {
                    target: Some(ShipRef::Player),
                    provoked: 0.0,
                    goal: Some(Goal::Attack(ShipRef::Player)),
                }
            }
        }
        let mut traffic = populated(1, 4);
        traffic.ticks = 1;
        traffic.tick_in(
            &Samaritan,
            World::new(&[]),
            &[struck(5, 1.0)],
            &mut Draws::of(&[]),
        );
        assert_eq!(
            traffic.npcs()[0].goal,
            Goal::Attack(ShipRef::Player),
            "not its turn"
        );
        assert_eq!(traffic.npcs()[0].target, Some(ShipRef::Player));
    }

    /// Who reacts to a strike on NPC 0, by `arrival_hits`, among a flying
    /// ship, a disabled one, and ships jumping in, holding their course,
    /// waiting in a gate, held at it and bursting out of it.
    fn reacting(arrival_hits: RuleSource) -> Vec<String> {
        let mut traffic = populated(7, 1);
        traffic.npcs[1].condition = Condition::Disabled;
        traffic.npcs[2].mode = Mode::JumpingIn { tick: 0 };
        traffic.npcs[3].mode = Mode::Coasting { ticks_left: 30 };
        traffic.npcs[4].mode = Mode::Emerging { ticks_left: 30 };
        traffic.npcs[5].mode = Mode::HeldAtGate { ticks_left: 30 };
        traffic.npcs[6].mode = Mode::LeavingGate { tick: 0 };
        let fighting = Fighting::deciding(Goal::Idle);
        let world = World {
            arrival_hits,
            ..World::new(&[])
        };
        traffic.tick_in(&fighting, world, &[struck(0, 1.0)], &mut Draws::of(&[]));
        fighting
            .log
            .borrow()
            .iter()
            .filter(|line| line.starts_with("react"))
            .cloned()
            .collect()
    }

    #[test]
    fn every_intact_npc_answers_a_strike_by_the_engine() {
        // `_DamageShip` drops a reaction only while +0x50 is above 0, a
        // ship jumping out (@0x3b4b0-0x3b4c2).
        assert_eq!(World::new(&[]).arrival_hits, RuleSource::Engine);
        assert_eq!(
            reacting(RuleSource::Engine),
            [0, 2, 3, 4, 5, 6].map(|id| format!("react {id} Npc(NpcId(0))"))
        );
    }

    #[test]
    fn only_an_arrived_intact_npc_answers_a_strike_by_the_other_reading() {
        assert_eq!(
            reacting(RuleSource::Bible),
            [0, 3, 5].map(|id| format!("react {id} Npc(NpcId(0))"))
        );
    }

    /// A strike of `damage` on NPC `ship` by NPC `by`.
    fn struck_by(ship: u32, by: u32, damage: f32) -> Strike {
        Strike {
            by: ShipRef::Npc(NpcId(by)),
            ..struck(ship, damage)
        }
    }

    #[test]
    fn a_ship_an_npc_hits_in_its_gate_takes_it_as_target_and_comes_out_early_still_hidden() {
        // `_DamageShip` sets the target (+0x70, @0x3bcf6) and provocation
        // (+0x96, @0x3bcc0-0x3bcf0) and cuts the wait (@0x3be6e-0x3be7c),
        // its goal 0x15 kept.
        let mut traffic = populated(2, 1);
        traffic.npcs[0].mode = Mode::Emerging { ticks_left: 60 };
        traffic.npcs[0].emerges_from = Some(GATE);
        let fighting = Fighting::deciding(Goal::Idle);
        traffic.tick_in(
            &fighting,
            World::new(&[]),
            &[struck_by(0, 1, 5.0)],
            &mut Draws::of(&[]),
        );
        let hit = &traffic.npcs()[0];
        assert_eq!(hit.target, Some(ShipRef::Npc(NpcId(1))));
        assert_eq!(hit.provoked, 5.0);
        assert_eq!(hit.mode, Mode::Emerging { ticks_left: 19 });
        assert!(hit.emerging(), "still hidden");
        assert!(hit.holds_gate_open(GATE));
        let mut ticks = 1;
        while traffic.npcs()[0].mode != Mode::Flying {
            let npc = &traffic.npcs()[0];
            assert_eq!(npc.target, Some(ShipRef::Npc(NpcId(1))), "{:?}", npc.mode);
            assert_eq!(npc.provoked, 5.0, "{:?}", npc.mode);
            assert!(!matches!(npc.mode, Mode::HeldAtGate { .. }));
            traffic.tick_in(&fighting, World::new(&[]), &[], &mut Draws::of(&[]));
            ticks += 1;
            assert!(ticks < 200, "flies");
        }
        assert!(ticks > 20, "it burst out after its wait");
    }

    /// Keeps its goal and fires its primaries whatever it does, targeting
    /// its goal's quarry, and answers a hit on itself as [`Fighting`]
    /// does, recording who decided. Waiting in its gate it holds `waits`,
    /// recording who was asked.
    #[derive(Debug, Default)]
    struct Firing {
        decided: RefCell<Vec<NpcId>>,
        waits: GateWait,
        waited: RefCell<Vec<NpcId>>,
    }

    const PRIMARY: Trigger = Trigger {
        primary: true,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
        point_defence: false,
    };

    const POINT_DEFENCE: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
        point_defence: true,
    };

    impl Behaviour for Firing {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.decided.borrow_mut().push(npc.id);
            npc.goal
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            PRIMARY
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.goal.quarry()
        }

        fn gate_wait(&self, npc: &Npc, _around: &Surroundings) -> GateWait {
            self.waited.borrow_mut().push(npc.id);
            self.waits
        }

        fn react(&self, npc: &Npc, strike: &Strike, _around: &Surroundings) -> Reaction {
            Reaction {
                target: (strike.ship == ShipRef::Npc(npc.id)).then_some(strike.by),
                provoked: if strike.ship == ShipRef::Npc(npc.id) {
                    strike.damage
                } else {
                    0.0
                },
                goal: None,
            }
        }
    }

    #[test]
    fn a_ship_the_player_hits_in_its_gate_is_held_there_seen_and_then_flies_at_the_player() {
        // `_AIMakeShipAttackPlayer` (@0x3be66-0x3be69): goal 4, travel
        // stellar -1 and target the player (@0x89d3e-0x89d4d). Its cut
        // wait runs on with no steering or fire (@0x34a54-0x34a72), its
        // per-type AI running (@0x8ff26-0x90016).
        let mut traffic = populated(1, 1);
        traffic.npcs[0].mode = Mode::Emerging { ticks_left: 60 };
        traffic.npcs[0].emerges_from = Some(GATE);
        let at = traffic.npcs[0].state.position;
        let world = World {
            player: Some(player_at(0.0, -300.0)),
            ..World::new(&[])
        };
        let firing = Firing::default();
        traffic.tick_in(&firing, world, &[struck(0, 5.0)], &mut Draws::of(&[]));
        let held = &traffic.npcs()[0];
        assert_eq!(held.mode, Mode::HeldAtGate { ticks_left: 19 });
        assert_eq!(held.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(held.target, Some(ShipRef::Player));
        assert_eq!(held.provoked, 5.0);
        assert_eq!(held.emerges_from, None);
        assert!(!held.holds_gate_open(GATE));
        assert!(world.around(traffic.npcs()).live(ShipRef::Npc(NpcId(0))));
        assert!(crate::targeting::targetable(held, false));
        assert_eq!(firing.decided.take(), [NpcId(0)], "decides while held");
        let mut ticks = 1;
        while traffic.npcs()[0].mode != Mode::Flying {
            let npc = &traffic.npcs()[0];
            assert_eq!(npc.trigger, Trigger::default(), "fires nothing held");
            assert_eq!(npc.state.position, at, "still");
            assert_eq!(npc.state.velocity, Vec2::ZERO);
            traffic.tick_in(&firing, world, &[], &mut Draws::of(&[]));
            ticks += 1;
        }
        assert_eq!(ticks, 20, "held for its cut wait");
        assert_eq!(firing.decided.take().len(), 19, "every tick of the hold");
        traffic.tick_in(&firing, world, &[], &mut Draws::of(&[]));
        let flying = &traffic.npcs()[0];
        assert_eq!(flying.trigger, PRIMARY, "it fires once out");
        assert_eq!(flying.target, Some(ShipRef::Player));
        assert_ne!(flying.state.position, at, "it flies");
    }

    #[test]
    fn a_ship_the_player_hits_gliding_in_or_bursting_out_keeps_its_mode_and_goal() {
        // `_AIDispatch` puts goal 8 and sub-goal 0xa back every frame
        // while +0x50 < -900 (@0x8fcf5-0x8fd18): the goal 4 is undone.
        let mut traffic = populated(2, 1);
        traffic.npcs[0].mode = Mode::JumpingIn { tick: 0 };
        traffic.npcs[1].mode = Mode::LeavingGate { tick: 0 };
        traffic.npcs[1].emerges_from = Some(GATE);
        traffic.tick_in(
            &Turning,
            World::new(&[]),
            &[struck(0, 3.0), struck(1, 4.0)],
            &mut Draws::of(&[]),
        );
        for (npc, (mode, provoked)) in traffic.npcs().iter().zip([
            (Mode::JumpingIn { tick: 1 }, 3.0),
            (Mode::LeavingGate { tick: 1 }, 4.0),
        ]) {
            assert_eq!(npc.target, Some(ShipRef::Player), "{:?}", npc.id);
            assert_eq!(npc.provoked, provoked, "{:?}", npc.id);
            assert_eq!(npc.goal, Goal::Idle, "{:?}", npc.id);
            assert_eq!(npc.mode, mode, "{:?}", npc.id);
        }
    }

    #[test]
    fn a_good_samaritan_waiting_in_its_gate_is_held_there_and_one_gliding_in_keeps_its_goal() {
        // `_DoGoodSamaritan` sets goal 4 and target the player
        // (@0x82b82-0x82b97) whatever +0x50 or goal 0x15 say; the wait of
        // a ship not hit is not cut.
        /// Attacks the player as soon as another is hit, targeting its
        /// goal's quarry.
        #[derive(Debug)]
        struct Samaritan;
        impl Behaviour for Samaritan {
            fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
                npc.goal
            }
            fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
                npc.goal.quarry()
            }
            fn react(&self, npc: &Npc, strike: &Strike, _: &Surroundings) -> Reaction {
                if strike.ship == ShipRef::Npc(npc.id) {
                    return Reaction::default();
                }
                Reaction {
                    target: Some(ShipRef::Player),
                    provoked: 0.0,
                    goal: Some(Goal::Attack(ShipRef::Player)),
                }
            }
        }
        let mut traffic = populated(3, 1);
        traffic.npcs[0].mode = Mode::Emerging { ticks_left: 60 };
        traffic.npcs[0].emerges_from = Some(GATE);
        traffic.npcs[1].mode = Mode::JumpingIn { tick: 0 };
        traffic.tick_in(
            &Samaritan,
            World::new(&[]),
            &[struck(2, 1.0)],
            &mut Draws::of(&[]),
        );
        let waiting = &traffic.npcs()[0];
        assert_eq!(waiting.mode, Mode::HeldAtGate { ticks_left: 59 });
        assert_eq!(waiting.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(waiting.target, Some(ShipRef::Player));
        assert_eq!(waiting.emerges_from, None);
        let gliding = &traffic.npcs()[1];
        assert_eq!(gliding.mode, Mode::JumpingIn { tick: 1 });
        assert_eq!(gliding.goal, Goal::Idle);
        assert_eq!(gliding.target, Some(ShipRef::Player));
    }

    /// Has ship 0 answer a strike on ship 1, its escort, as a strike on
    /// itself, and ship 1 answer its own.
    #[derive(Debug)]
    struct Guarding;

    impl Behaviour for Guarding {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }
        fn react(&self, npc: &Npc, strike: &Strike, _: &Surroundings) -> Reaction {
            let answers = strike.ship == ShipRef::Npc(npc.id)
                || (npc.id == NpcId(0) && strike.ship == ShipRef::Npc(NpcId(1)));
            Reaction {
                target: answers.then_some(strike.by),
                provoked: 0.0,
                goal: None,
            }
        }
    }

    #[test]
    fn a_lead_answering_a_hit_on_its_escort_keeps_its_wait() {
        // `_DamageShip` cuts the wait of the ship hit only (`%edi`,
        // @0x3be6e-0x3be7c), not of the lead that answers for its escort
        // (@0x3bcfa-0x3bd79).
        let mut traffic = populated(2, 1);
        traffic.npcs[1].leader = Some(NpcId(0));
        traffic.npcs[0].mode = Mode::Coasting { ticks_left: 45 };
        traffic.npcs[1].mode = Mode::Coasting { ticks_left: 45 };
        traffic.tick_in(
            &Guarding,
            World::new(&[]),
            &[struck(1, 5.0)],
            &mut Draws::of(&[]),
        );
        let lead = &traffic.npcs()[0];
        assert_eq!(lead.target, Some(ShipRef::Player), "the lead answers");
        assert_eq!(
            lead.mode,
            Mode::Coasting { ticks_left: 44 },
            "the lead's wait only drops by the tick"
        );
        let escort = &traffic.npcs()[1];
        assert_eq!(escort.target, Some(ShipRef::Player));
        assert_eq!(
            escort.mode,
            Mode::Coasting { ticks_left: 19 },
            "the escort hit: 20, less this tick"
        );
    }

    #[test]
    fn a_lead_in_its_gate_answering_the_players_hit_on_its_escort_keeps_waiting_hidden() {
        // `_AIMakeShipAttackPlayer` is called on the victim alone
        // (@0x3be66-0x3be69), as is the wait cut (@0x3be6e-0x3be7c).
        let mut traffic = populated(2, 1);
        traffic.npcs[1].leader = Some(NpcId(0));
        for npc in &mut traffic.npcs {
            npc.mode = Mode::Emerging { ticks_left: 60 };
            npc.emerges_from = Some(GATE);
        }
        let goal = traffic.npcs[0].goal;
        traffic.tick_in(
            &Guarding,
            World::new(&[]),
            &[struck(1, 5.0)],
            &mut Draws::of(&[]),
        );
        let lead = &traffic.npcs()[0];
        assert_eq!(lead.target, Some(ShipRef::Player), "the lead answers");
        assert_eq!(lead.mode, Mode::Emerging { ticks_left: 59 }, "not held");
        assert_eq!(lead.goal, goal);
        assert!(lead.holds_gate_open(GATE));
        let escort = &traffic.npcs()[1];
        assert_eq!(escort.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(escort.mode, Mode::HeldAtGate { ticks_left: 19 });
    }

    #[test]
    fn a_good_samaritan_held_at_its_gate_and_hit_later_has_its_wait_cut_to_20() {
        // `_DamageShip` cuts the victim's +0x4c to 20 whatever its goal
        // (@0x3be6e-0x3be7c); the Samaritan's hold left it uncut.
        let mut traffic = populated(2, 1);
        traffic.npcs[0].mode = Mode::Emerging { ticks_left: 60 };
        traffic.npcs[0].emerges_from = Some(GATE);
        traffic.npcs[0].hold_at_gate();
        let firing = Firing::default();
        traffic.tick_in(&firing, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].mode, Mode::HeldAtGate { ticks_left: 59 });
        traffic.tick_in(
            &firing,
            World::new(&[]),
            &[struck_by(0, 1, 2.0)],
            &mut Draws::of(&[]),
        );
        let hit = &traffic.npcs()[0];
        assert_eq!(hit.provoked, 2.0, "it answered");
        assert_eq!(hit.mode, Mode::HeldAtGate { ticks_left: 19 });
    }

    /// The point defence a waiting ship's behaviour holds.
    const DEFENDING: GateWait = GateWait {
        trigger: POINT_DEFENCE,
        drops_target: false,
    };

    /// NPC 0 waiting 30 ticks in a gate beside NPC 1, struck by `hit`
    /// in `world`, deciding as `firing` says: its mode, and the trigger and
    /// target it holds, on each tick until it is out of its wait.
    fn waiting(
        firing: &Firing,
        world: &World,
        hit: Strike,
    ) -> Vec<(Mode, Trigger, Option<ShipRef>)> {
        let mut traffic = populated(2, 1);
        traffic.npcs[0].mode = Mode::Emerging { ticks_left: 30 };
        let mut held = Vec::new();
        let mut strikes = vec![hit];
        while matches!(
            traffic.npcs[0].mode,
            Mode::Emerging { .. } | Mode::HeldAtGate { .. }
        ) {
            traffic.tick_in(firing, *world, &strikes, &mut Draws::of(&[]));
            strikes.clear();
            let npc = &traffic.npcs[0];
            held.push((npc.mode, npc.trigger, npc.target));
        }
        held
    }

    #[test]
    fn a_ship_waiting_in_or_held_at_its_gate_holds_what_its_behaviour_says_for_the_wait() {
        // `_LowLevelAIHandler` at sub-goal 0 calls
        // `_AIEscortFireUnprovoked` every frame of the wait
        // (@0x8521e-0x85236), the per-type AI's fire unread.
        let world = World {
            player: Some(player_at(0.0, -300.0)),
            ..World::new(&[])
        };
        let firing = Firing {
            waits: DEFENDING,
            ..Firing::default()
        };
        let held = waiting(&firing, &world, struck_by(0, 1, 2.0));
        let (out, waits) = held.split_last().expect("ticks");
        assert_eq!(waits.len(), 19, "its cut wait");
        for &(mode, trigger, target) in waits {
            assert!(matches!(mode, Mode::Emerging { .. }), "{mode:?}");
            assert_eq!(trigger, POINT_DEFENCE, "{mode:?}");
            assert_eq!(target, Some(ShipRef::Npc(NpcId(1))), "{mode:?}");
        }
        assert_eq!(out.0, Mode::LeavingGate { tick: 0 });
        assert_eq!(out.1, Trigger::default(), "none in the burst");
        // Held at its gate by the player's hit, its per-type AI deciding
        // and firing primaries, it holds the wait's instead.
        let held = waiting(&firing, &world, struck(0, 2.0));
        let (out, waits) = held.split_last().expect("ticks");
        assert_eq!(waits.len(), 19);
        for &(mode, trigger, target) in waits {
            assert!(matches!(mode, Mode::HeldAtGate { .. }), "{mode:?}");
            assert_eq!(trigger, POINT_DEFENCE, "{mode:?}");
            assert_eq!(target, Some(ShipRef::Player), "{mode:?}");
        }
        assert_eq!(out.0, Mode::Flying);
    }

    #[test]
    fn a_waiting_ship_drops_its_target_when_its_behaviour_says() {
        let firing = Firing {
            waits: GateWait {
                trigger: Trigger::default(),
                drops_target: true,
            },
            ..Firing::default()
        };
        let held = waiting(&firing, &World::new(&[]), struck_by(0, 1, 2.0));
        let &(mode, trigger, target) = held.first().expect("a tick");
        assert!(matches!(mode, Mode::Emerging { .. }));
        assert_eq!((trigger, target), (Trigger::default(), None));
    }

    #[test]
    fn only_a_ship_waiting_in_or_held_at_its_gate_is_asked_for_its_wait() {
        let mut traffic = populated(6, 1);
        traffic.npcs[0].mode = Mode::Emerging { ticks_left: 30 };
        traffic.npcs[1].mode = Mode::HeldAtGate { ticks_left: 30 };
        traffic.npcs[2].mode = Mode::JumpingIn { tick: 0 };
        traffic.npcs[3].mode = Mode::Coasting { ticks_left: 30 };
        traffic.npcs[4].mode = Mode::LeavingGate { tick: 0 };
        let firing = Firing {
            waits: DEFENDING,
            ..Firing::default()
        };
        traffic.tick_in(&firing, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(firing.waited.take(), [NpcId(0), NpcId(1)]);
        assert_eq!(traffic.npcs()[5].trigger, PRIMARY, "its own, flying");
    }

    #[test]
    fn provocation_clears_on_an_idle_goal_or_when_disabled() {
        let mut traffic = populated(2, 1);
        for npc in traffic.npcs_mut() {
            npc.provoked = 9.0;
        }
        traffic.npcs[1].condition = Condition::Disabled;
        let fleeing = Fighting::deciding(Goal::Flee(ShipRef::Player));
        traffic.tick_in(&fleeing, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].provoked, 9.0, "still fighting");
        assert_eq!(traffic.npcs()[1].provoked, 0.0, "disabled");
        let calm = Fighting::deciding(Goal::Idle);
        traffic.tick_in(&calm, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].provoked, 0.0, "idle");
        traffic.npcs_mut()[0].provoked = 2.0;
        let inspecting = Fighting::deciding(Goal::Inspect(ShipRef::Player));
        traffic.tick_in(&inspecting, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].provoked, 0.0, "inspecting is idle");
    }

    #[test]
    fn an_npc_that_stops_inspecting_a_ship_remembers_it() {
        let mut traffic = populated(1, 1);
        traffic.npcs[0].goal = Goal::Inspect(ShipRef::Player);
        let still = Fighting::deciding(Goal::Inspect(ShipRef::Player));
        traffic.tick_in(&still, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].inspected, None, "still at it");
        let next = Fighting::deciding(Goal::Inspect(ShipRef::Npc(NpcId(7))));
        traffic.tick_in(&next, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].inspected, Some(ShipRef::Player));
    }

    #[test]
    fn an_npc_that_stops_inspecting_the_player_within_reach_reports_a_scan() {
        let mut traffic = populated(3, 1);
        for npc in traffic.npcs_mut() {
            npc.goal = Goal::Inspect(ShipRef::Player);
            npc.state.position = Vec2::ZERO;
        }
        traffic.npcs[1].state.position = Vec2::new(-1.0, 0.0);
        traffic.npcs[2].goal = Goal::Inspect(ShipRef::Npc(NpcId(0)));
        let world = World {
            player: Some(player_at(100.0, -100.0)),
            ..World::new(&[])
        };
        let still = Fighting::deciding(Goal::Inspect(ShipRef::Player));
        traffic.tick_in(&still, world, &[], &mut Draws::of(&[]));
        assert_eq!(traffic.take_scans(), [], "still at it");
        for npc in traffic.npcs_mut() {
            npc.state.position = Vec2::ZERO;
            npc.state.velocity = Vec2::ZERO;
        }
        traffic.npcs[1].state.position = Vec2::new(-1.0, 0.0);
        traffic.npcs[2].goal = Goal::Inspect(ShipRef::Npc(NpcId(0)));
        let done = Fighting::deciding(Goal::Idle);
        traffic.tick_in(&done, world, &[], &mut Draws::of(&[]));
        assert_eq!(
            traffic.take_scans(),
            [NpcId(0)],
            "100 off on each axis is in reach; 101 is not; an NPC is no scan"
        );
        assert_eq!(traffic.take_scans(), [], "taken");
        let mut alone = populated(1, 1);
        alone.npcs[0].goal = Goal::Inspect(ShipRef::Player);
        alone.tick_in(&done, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(alone.take_scans(), [], "no player");
    }

    /// The scans wait for the session's combat tick; a system left first
    /// takes them with it, as the original scans at once.
    #[test]
    fn entering_or_leaving_for_another_system_drops_the_scans_not_taken() {
        let scanned = || {
            let mut traffic = populated(1, 1);
            traffic.npcs[0].goal = Goal::Inspect(ShipRef::Player);
            let world = World {
                player: Some(player_at(0.0, 0.0)),
                ..World::new(&[])
            };
            let done = Fighting::deciding(Goal::Idle);
            traffic.tick_in(&done, world, &[], &mut Draws::of(&[]));
            traffic
        };
        let mut entered = scanned();
        entered.enter(SpawnTable::default(), &mut Draws::of(&[]));
        assert_eq!(entered.take_scans(), []);
        let mut left = scanned();
        let kept = left.npcs()[0].id;
        left.retarget(
            SpawnTable::default(),
            &BTreeSet::from([kept]),
            Leave::Dropped,
        );
        assert_eq!(left.take_scans(), [], "even the scanner kept");
        assert_eq!(scanned().take_scans(), [NpcId(0)], "queued");
    }

    /// The player at (`x`, `y`), at rest.
    fn player_at(x: f32, y: f32) -> PlayerSide {
        PlayerSide {
            state: ShipState {
                position: Vec2::new(x, y),
                ..ShipState::default()
            },
            condition: Condition::Intact,
            reserves: crate::reserves::Reserves::full(10.0, 10.0, 10.0),
            hull: HullSpec::default(),
            handling: ShipStats::new(FAST, &[]).handling,
            rating: 0,
            yardstick: 0,
            untargetable: false,
            blessing: crate::rank::Blessing::NONE,
        }
    }

    /// A ship of top speed `speed` (in pixels a tick), as quick to turn
    /// and thrust as [`FAST`].
    fn of_speed(id: u32, speed: f32) -> Npc {
        let fields = crate::handling::ShipFields {
            speed: (speed * 100.0) as i16,
            ..FAST
        };
        crate::testkit::npc(id, ShipStats::new(fields, &[]))
    }

    /// `npc` bursting out of a gate along +x, from its first tick.
    fn bursting(npc: Npc) -> Npc {
        Npc {
            mode: Mode::LeavingGate { tick: 0 },
            state: ShipState {
                heading: 90.0,
                ..ShipState::default()
            },
            ..npc
        }
    }

    /// Ticks `traffic` in `world` until NPC `id`'s burst ends, and gives
    /// the speed it flies on at.
    fn burst_speed(traffic: &mut Traffic, world: &World, id: NpcId) -> f32 {
        for _ in 0..20 {
            traffic.tick_in(&Keep, *world, &[], &mut Draws::of(&[]));
            let npc = traffic
                .npcs()
                .iter()
                .find(|npc| npc.id == id)
                .expect("there");
            if !matches!(npc.mode, Mode::LeavingGate { .. }) {
                return npc.state.velocity.length();
            }
        }
        panic!("never ended")
    }

    #[test]
    fn the_ships_the_player_leads_fly_at_the_led_speed_and_others_at_their_own() {
        let mut traffic = Traffic::new();
        let formation = Goal::Formation { guard: None };
        let hired = traffic.add_npc(Npc {
            escort: Some(duty(1)),
            goal: formation,
            ..of_speed(0, 3.0)
        });
        let mission = traffic.add_npc(Npc {
            mission: Some(escorting(true)),
            goal: formation,
            ..of_speed(0, 3.0)
        });
        let unled = traffic.add_npc(Npc {
            goal: Goal::Attack(ShipRef::Player),
            ..of_speed(0, 3.0)
        });
        let world = World {
            player: Some(player_at(5000.0, 0.0)),
            led_speed: 1.5,
            ..World::new(&[])
        };
        let mut fastest = BTreeMap::new();
        for _ in 0..300 {
            traffic.tick_in(&Keep, world, &[], &mut Draws::of(&[]));
            for npc in traffic.npcs() {
                let speed = npc.state.velocity.length();
                let top = fastest.entry(npc.id).or_insert(0.0_f32);
                *top = top.max(speed);
            }
        }
        assert!((fastest[&hired] - 4.5).abs() < 1e-3, "{fastest:?}");
        assert!((fastest[&mission] - 4.5).abs() < 1e-3, "{fastest:?}");
        assert!(fastest[&unled] <= 3.0 + 1e-4, "{fastest:?}");
        assert!(fastest[&unled] > 2.9, "it flew flat out: {fastest:?}");
    }

    #[test]
    fn a_fleet_escort_ends_its_burst_at_its_slower_leads_top_speed() {
        // Its own 6 would end the burst at 9 (30 - 3t, ending once the
        // next tick's is at or below the cap); its lead's 3 ends it at 6.
        let mut traffic = Traffic::new();
        let lead = traffic.add_npc(of_speed(0, 3.0));
        let escort = traffic.add_npc(bursting(Npc {
            leader: Some(lead),
            ..of_speed(0, 6.0)
        }));
        let speed = burst_speed(&mut traffic, &World::new(&[]), escort);
        assert!((speed - 6.0).abs() < 1e-4, "{speed}");
    }

    #[test]
    fn an_escort_whose_lead_is_gone_ends_its_burst_at_its_own_top_speed() {
        let mut traffic = Traffic::new();
        let escort = traffic.add_npc(bursting(Npc {
            leader: Some(NpcId(99)),
            ..of_speed(0, 6.0)
        }));
        let speed = burst_speed(&mut traffic, &World::new(&[]), escort);
        assert!((speed - 9.0).abs() < 1e-4, "{speed}");
    }

    #[test]
    fn a_led_ship_ends_its_burst_at_the_players_top_speed_times_the_led_speed() {
        // The player's 4 x 1.5 = 6 ends the burst at 9; its own 10 x 1.5
        // would end it at 18, its bare 10 at 12, and the player's bare 4
        // at 6.
        let mut traffic = Traffic::new();
        let escort = traffic.add_npc(bursting(Npc {
            escort: Some(duty(1)),
            ..of_speed(0, 10.0)
        }));
        let mut player = player_at(0.0, 0.0);
        player.handling.max_speed = 4.0;
        let world = World {
            player: Some(player),
            led_speed: 1.5,
            ..World::new(&[])
        };
        let speed = burst_speed(&mut traffic, &world, escort);
        assert!((speed - 9.0).abs() < 1e-4, "{speed}");
    }

    #[test]
    fn an_npc_fights_the_player_where_the_player_is() {
        // NPC 0 at the centre facing up, the player 500 to its right.
        let mut traffic = populated(1, 1);
        let attacking = Fighting::deciding(Goal::Attack(ShipRef::Player));
        let world = World {
            player: Some(player_at(500.0, 0.0)),
            ..World::new(&[])
        };
        for _ in 0..40 {
            traffic.tick_in(&attacking, world, &[], &mut Draws::of(&[]));
        }
        let heading = traffic.npcs()[0].state.heading;
        assert!((heading - 90.0).abs() < 3.0, "{heading}");
        let mut alone = populated(1, 1);
        alone.npcs[0].state.velocity = Vec2::new(1.0, 0.0);
        for _ in 0..100 {
            alone.tick_in(&attacking, World::new(&[]), &[], &mut Draws::of(&[]));
        }
        assert!(
            alone.npcs()[0].state.velocity.length() < 0.5,
            "no player: it brakes"
        );
    }

    #[test]
    fn an_npc_fights_another_where_it_is() {
        /// NPC 0 attacks NPC 1, which stays put.
        #[derive(Debug)]
        struct Duel;
        impl Behaviour for Duel {
            fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
                if npc.id == NpcId(0) {
                    Goal::Attack(ShipRef::Npc(NpcId(1)))
                } else {
                    Goal::Idle
                }
            }
        }
        let mut traffic = populated(2, 1);
        traffic.npcs[1].state.position = Vec2::new(-500.0, 0.0);
        for _ in 0..40 {
            traffic.tick_in(&Duel, World::new(&[]), &[], &mut Draws::of(&[]));
        }
        let heading = traffic.npcs()[0].state.heading;
        assert!((heading - 270.0).abs() < 3.0, "{heading}");
    }

    #[test]
    fn a_warship_jumping_in_with_no_enemies_heads_for_a_stellar_and_stays() {
        // npc-jump-in-turns-at-edge: under Nova's AI an arrival heads in
        // for a stellar rather than turning back out at the edge.
        let mut traffic = Traffic::new();
        let mut warships = table(1);
        warships
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ai_type = 3;
        traffic.enter(warships, &mut Draws::of(&[0]));
        let sites = [planet(140, 0.0, 0.0)];
        let ai = crate::ai::NovaAi::default();
        // An arrival: 0, not a person, not a fleet, düde 128, ship 200, at
        // angle 0, aggression 0.
        traffic.tick(&ai, &sites, &mut Draws::of(&[0, 6, 6, 0, 0, 0, 2]));
        assert_eq!(traffic.npcs()[0].ai_type, AiType::Warship);
        let mut nearest = f32::INFINITY;
        // It glides in, holds its course with no lead, then flies a few
        // ticks.
        let mut ticks = 0;
        while traffic.npcs()[0].mode != Mode::Flying {
            traffic.tick(&ai, &sites, &mut Draws::of(&[]));
            ticks += 1;
            assert!(ticks < 200, "in");
        }
        for _ in 0..10 {
            traffic.tick(&ai, &sites, &mut Draws::of(&[]));
        }
        let after_glide = traffic.npcs()[0].clone();
        assert_eq!(after_glide.mode, Mode::Flying);
        assert_eq!(after_glide.goal, Goal::Land(StellarId(140)));
        assert_eq!(traffic.departed(), []);
        for _ in 0..200 {
            traffic.tick(&ai, &sites, &mut Draws::of(&[]));
            if let Some(npc) = traffic.npcs().first() {
                nearest = nearest.min(npc.state.position.length());
            }
        }
        assert!(nearest < 500.0, "well inside 1000: {nearest}");
    }

    // Persons.

    /// [`table`] of `avg_ships` with person 510 linked here, flying ship
    /// 200 fitted with twice its shield and an extra weapon: a warship of
    /// govt 128 and `Aggress` 4, `Coward` 15, quoting 24 and 8, with
    /// `HailPict` 7800, a mission, `Flags` 0x0003, and invincible; it
    /// starts disabled with no fuel.
    fn peopled(avg_ships: u32) -> SpawnTable {
        let mut table = table(avg_ships);
        let base = table.ships[&ShipId(200)].clone();
        let mut kind = base.clone();
        kind.stats.shield *= 2.0;
        kind.armament = Armament::new([(WeaponSpec::new(&weapon(129)), 1)]);
        let mut reserves = kind.stats.full();
        reserves.fuel.now = 0.0;
        table.persons.insert(
            PersonId(510),
            SpawnPerson {
                record: PersonRecord {
                    name: "Ace".to_owned(),
                    govt: Some(GovtId(128)),
                    aggress: 4,
                    coward: 15,
                    comm_quote: 24,
                    hail_quote: 8,
                    hail_pict: Some(7800),
                    link_mission: Some(400),
                    flags: 0x0003,
                    shield_mod: -1,
                    ..crate::testkit::person(510, 200)
                },
                linked: true,
                kind,
                reserves,
                condition: Condition::Disabled,
                derelict: false,
            },
        );
        table
    }

    fn entered(world: &World, draws: &[u32]) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.enter_in(peopled(1), *world, &mut Draws::of(draws));
        traffic
    }

    #[test]
    fn a_person_entering_flies_as_its_record_and_its_fitted_ship_say() {
        let traffic = entered(&World::new(&[]), &[0, 382, 750, 750, 0]);
        let npcs = traffic.npcs();
        assert_eq!(npcs.len(), 1);
        let npc = &npcs[0];
        let fitted = &traffic.person(PersonId(510)).expect("its record").kind;
        assert_eq!(
            (npc.ship, npc.govt, npc.ai_type, npc.aggression),
            (ShipId(200), Some(GovtId(128)), AiType::Warship, 4)
        );
        assert_eq!(npc.stats, fitted.stats, "its fitted stats");
        assert_eq!(npc.armament, fitted.armament);
        assert_eq!(npc.reserves.fuel.now, 0.0, "the reserves it starts with");
        assert_eq!(npc.reserves.shield.max, 2.0 * f32::from(FAST.shield));
        assert_eq!(npc.condition, Condition::Disabled);
        assert_eq!((npc.booty, npc.info_types), (0, 0));
        assert_eq!(
            npc.person,
            Some(NpcPerson {
                id: PersonId(510),
                flags: 0x0003,
                coward: 15,
                comm_quote: 24,
                hail_quote: 8,
                mission: true,
                portrait: Some(7800),
                invincible: true,
                grudge: false,
                quoted: false,
                quoted_at: None,
            })
        );
        assert_eq!(traffic.person(PersonId(600)), None);
    }

    #[test]
    fn a_person_holding_a_grudge_enters_with_it() {
        let grudges = std::collections::BTreeSet::from([PersonId(510)]);
        let world = World {
            persons: PersonWorld {
                grudges: &grudges,
                ..PersonWorld::NONE
            },
            ..World::new(&[])
        };
        let traffic = entered(&world, &[0, 382, 750, 750, 0]);
        assert!(traffic.npcs()[0].person.expect("a person").grudge);
    }

    #[test]
    fn a_person_gone_for_good_never_enters() {
        let gone = std::collections::BTreeSet::from([PersonId(510)]);
        let world = World {
            persons: PersonWorld {
                gone: &gone,
                ..PersonWorld::NONE
            },
            ..World::new(&[])
        };
        assert_eq!(entered(&world, &[0, 382]).npcs(), []);
    }

    #[test]
    fn a_person_arrives_on_its_roll_and_only_once_by_its_name() {
        let mut traffic = Traffic::new();
        traffic.enter_in(peopled(2), World::new(&[]), &mut Draws::of(&[0, 0, 0, 0]));
        assert_eq!(traffic.npcs(), [], "two empty person rolls");
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[0, 0, 382, 0]));
        assert_eq!(
            traffic
                .npcs()
                .iter()
                .map(|npc| npc.person.map(|p| p.id))
                .collect::<Vec<_>>(),
            [Some(PersonId(510))]
        );
        assert!(traffic.npcs()[0].mode != Mode::Flying, "jumping in");
        let mut chance = Draws::of(&[0, 0]);
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut chance);
        assert_eq!(
            chance.asked,
            [500, 7],
            "Ace is here: nobody may come, no draw of 1022"
        );
        assert_eq!(traffic.npcs().len(), 1);
    }

    #[test]
    fn entering_without_a_world_brings_no_one_gone_or_grudging() {
        let mut traffic = Traffic::new();
        traffic.enter(peopled(1), &mut Draws::of(&[0, 382, 750, 750, 0]));
        assert!(!traffic.npcs()[0].person.expect("a person").grudge);
    }

    // Mission ships.

    /// A mission ship of ship 200 for slot 2, of `ShipBehav` `behav`, at
    /// (`x`, 0).
    fn mission_ship(behav: i16, x: f32) -> NewShip {
        NewShip {
            ship: ShipId(200),
            govt: Some(GovtId(131)),
            ai_type: AiType::Interceptor,
            lead: None,
            state: ShipState {
                position: Vec2::new(x, 0.0),
                ..ShipState::default()
            },
            entry: Entry::Placed,
            aggression: 3,
            booty: 2,
            info_types: 1,
            person: None,
            mission: Some(npc::MissionMark {
                order: None,
                slot: 2,
                behav: ShipBehav::from_code(behav),
                formation: None,
            }),
            derelict: false,
            cloaked: false,
            aux: None,
            quote: None,
            cargo: None,
        }
    }

    #[test]
    fn a_systems_mission_ships_come_first_and_use_up_no_setup_pass() {
        let mut traffic = Traffic::new();
        let mut chance = Draws::of(&placed(750, 750));
        traffic.enter_with(
            table(1),
            World::new(&[]),
            vec![mission_ship(-1, 5.0), mission_ship(-1, 6.0)],
            &mut chance,
        );
        let npcs = traffic.npcs();
        assert_eq!(npcs.len(), 3, "both, and the one setup pass");
        assert_eq!(
            npcs.iter()
                .map(|npc| npc.mission.map(|mark| mark.slot))
                .collect::<Vec<_>>(),
            [Some(2), Some(2), None]
        );
        assert_eq!(
            (npcs[0].govt, npcs[0].ai_type),
            (Some(GovtId(131)), AiType::Interceptor)
        );
        assert_eq!(
            (npcs[0].booty, npcs[0].info_types, npcs[0].aggression),
            (2, 1, 3)
        );
        assert_eq!(npcs[1].state.position, Vec2::new(6.0, 0.0));
        assert_eq!((npcs[0].goal, npcs[0].target), (Goal::Idle, None));
    }

    #[test]
    fn a_mission_ship_of_ship_behav_0_starts_attacking_the_player() {
        let mut traffic = Traffic::new();
        traffic.enter_with(
            table(0),
            World::new(&[]),
            vec![mission_ship(0, 0.0)],
            &mut Draws::of(&[]),
        );
        traffic.bring_in_within(
            vec![mission_ship(0, 9.0), mission_ship(1, 9.0)],
            SPAWN_RESERVE,
        );
        let npcs = traffic.npcs();
        for npc in &npcs[..2] {
            assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
            assert_eq!(npc.target, Some(ShipRef::Player));
        }
        assert_eq!((npcs[2].goal, npcs[2].target), (Goal::Idle, None));
        assert_eq!(npcs[1].id, NpcId(1), "numbered after the others");
    }

    #[test]
    fn a_derelict_mission_ship_starts_disabled_with_no_shield_and_its_armour_cut() {
        let mut tough = table(0);
        tough.ships.get_mut(&ShipId(200)).expect("200").hull.tough = true;
        for (table, share) in [(table(0), 0.33_f32), (tough, 0.1)] {
            let mut traffic = Traffic::new();
            traffic.enter_with(table, World::new(&[]), Vec::new(), &mut Draws::of(&[]));
            traffic.bring_in_within(
                vec![
                    NewShip {
                        derelict: true,
                        ..mission_ship(-1, 0.0)
                    },
                    mission_ship(-1, 1.0),
                ],
                SPAWN_RESERVE,
            );
            let npcs = traffic.npcs();
            assert_eq!(npcs[0].condition, Condition::Disabled);
            assert_eq!(npcs[0].reserves.shield.now, 0.0);
            assert_eq!(npcs[0].reserves.armor.now, 45.0_f32.mul_add(share, -1.0));
            assert_eq!(npcs[0].reserves.armor.max, 45.0);
            assert_eq!(npcs[0].reserves.fuel, npcs[1].reserves.fuel);
            assert_eq!(npcs[1].condition, Condition::Intact);
            assert_eq!(npcs[1].reserves, npcs[1].stats.full());
        }
    }

    // Cloaks.

    /// A cloak of the stock Zephyr's: it cloaks retreating, keeps its cloak
    /// pressing an attack, and decloaks when hit.
    const ZEPHYR: CloakGear = CloakGear {
        device: true,
        decloaks_when_hit: true,
        on_radar: true,
        flags2: 0x5282,
        scanner: crate::cloak::Scanner::NONE,
    };

    /// [`table`] of `count` ships whose ship 200 carries `gear`.
    fn cloaking(count: u32, gear: CloakGear) -> SpawnTable {
        let mut table = table(count);
        table.ships.get_mut(&ShipId(200)).expect("200").cloak = gear;
        table
    }

    /// Traffic in a system with one mission ship of ship 200 carrying
    /// `gear`, set up `cloaked` or not, at (`x`, 0).
    fn with_mission_ship(gear: CloakGear, cloaked: bool, x: f32) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.enter_with(
            cloaking(0, gear),
            World::new(&[]),
            vec![NewShip {
                cloaked,
                ..mission_ship(-1, x)
            }],
            &mut Draws::of(&[]),
        );
        traffic
    }

    #[test]
    fn a_ship_set_up_cloaked_starts_fading_in_carrying_its_ship_types_gear() {
        let traffic = with_mission_ship(ZEPHYR, true, 0.0);
        let npc = &traffic.npcs()[0];
        assert_eq!(
            npc.cloak,
            Cloak {
                fade: 0.0,
                direction: 1
            }
        );
        assert_eq!(npc.cloak_gear, ZEPHYR);
        let plain = with_mission_ship(CloakGear::default(), false, 0.0);
        assert_eq!(plain.npcs()[0].cloak, Cloak::default());
        assert_eq!(plain.npcs()[0].cloak_gear, CloakGear::default());
    }

    #[test]
    fn a_ship_set_up_cloaked_with_nothing_to_keep_it_uncloaks_on_its_first_think_that_finds_it_cloaked()
     {
        let mut traffic = with_mission_ship(ZEPHYR, true, 0.0);
        for _ in 0..22 {
            traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        }
        let cloak = traffic.npcs()[0].cloak;
        assert_eq!(cloak.direction, 1, "an early uncloak does nothing");
        assert!(cloak.is_cloaked(), "{cloak:?}");
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].cloak.direction, -1);
        let mut ticks = 1;
        while traffic.npcs()[0].cloak.is_cloaked() {
            traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
            ticks += 1;
        }
        assert_eq!(ticks, 12, "at 0.75 a tick, from 16.5 to 8");
    }

    #[test]
    fn a_ship_without_a_cloak_set_up_cloaked_uncloaks_all_the_same() {
        let mut traffic = with_mission_ship(CloakGear::default(), true, 0.0);
        for _ in 0..23 {
            traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        }
        assert_eq!(traffic.npcs()[0].cloak.direction, -1);
    }

    #[test]
    fn a_mission_ship_come_in_uncloaked_cloaks_on_its_think_while_it_retreats_and_uncloaks_after() {
        let mut traffic = with_mission_ship(ZEPHYR, false, 0.0);
        let fleeing = Recording::deciding(Goal::Flee(ShipRef::Player));
        traffic.tick_in(&fleeing, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(
            traffic.npcs()[0].cloak,
            Cloak {
                fade: 0.75,
                direction: 1
            },
            "thinks, then fades a step"
        );
        for _ in 0..21 {
            traffic.tick_in(&fleeing, World::new(&[]), &[], &mut Draws::of(&[]));
        }
        assert!(traffic.npcs()[0].cloak.is_cloaked());
        let calm = Recording::deciding(Goal::Idle);
        traffic.tick_in(&calm, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(
            traffic.npcs()[0].cloak.direction,
            -1,
            "no longer retreating"
        );
        let mut plain = with_mission_ship(
            CloakGear {
                flags2: 0,
                ..ZEPHYR
            },
            false,
            0.0,
        );
        plain.tick_in(&fleeing, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(plain.npcs()[0].cloak, Cloak::default(), "no bit");
    }

    #[test]
    fn a_cloaked_ship_keeps_its_cloak_attacking_the_player_from_afar_and_drops_it_close() {
        let mut traffic = with_mission_ship(ZEPHYR, false, 1000.0);
        traffic.npcs[0].cloak = Cloak {
            fade: FADE_FULL,
            direction: 0,
        };
        let attacking = Recording::deciding(Goal::Attack(ShipRef::Player));
        let far = World {
            player: Some(player_at(0.0, 400.0)),
            ..World::new(&[])
        };
        traffic.tick_in(&attacking, far, &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].cloak.direction, 0, "kept");
        let npc_at = traffic.npcs()[0].state.position;
        let near = World {
            player: Some(player_at(npc_at.x + 100.0, npc_at.y - 100.0)),
            ..World::new(&[])
        };
        traffic.tick_in(&attacking, near, &[], &mut Draws::of(&[]));
        assert_eq!(traffic.npcs()[0].cloak.direction, -1, "close in to fight");
    }

    #[test]
    fn a_cloaked_ship_attacking_another_npc_measures_from_it() {
        for (x, kept) in [(1300.0, true), (1100.0, false)] {
            let mut traffic = with_mission_ship(ZEPHYR, false, 1000.0);
            traffic.bring_in_within(vec![mission_ship(-1, x)], SPAWN_RESERVE);
            traffic.npcs[0].cloak = Cloak {
                fade: FADE_FULL,
                direction: 0,
            };
            let attacking = Recording::deciding(Goal::Attack(ShipRef::Npc(NpcId(1))));
            traffic.tick_in(&attacking, World::new(&[]), &[], &mut Draws::of(&[]));
            let direction = traffic.npcs()[0].cloak.direction;
            assert_eq!(direction == 0, kept, "{x}: {direction}");
        }
    }

    #[test]
    fn a_cloaked_ship_that_can_no_longer_cloak_uncloaks() {
        for (disabled, fuel) in [(true, 100.0), (false, 0.0)] {
            let mut traffic = with_mission_ship(ZEPHYR, false, 0.0);
            let npc = &mut traffic.npcs[0];
            npc.cloak = Cloak {
                fade: FADE_FULL,
                direction: 0,
            };
            npc.mode = Mode::JumpingIn { tick: 0 };
            npc.reserves.fuel.now = fuel;
            if disabled {
                npc.condition = Condition::Disabled;
            }
            traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
            assert_eq!(
                traffic.npcs()[0].cloak.direction,
                -1,
                "no think, still: disabled {disabled}, fuel {fuel}"
            );
        }
        for mode in [
            Mode::JumpingIn { tick: 0 },
            Mode::Coasting { ticks_left: 30 },
        ] {
            let mut able = with_mission_ship(ZEPHYR, false, 0.0);
            able.npcs[0].cloak = Cloak {
                fade: FADE_FULL,
                direction: 0,
            };
            able.npcs[0].mode = mode;
            able.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
            assert_eq!(able.npcs()[0].cloak.direction, 0, "{mode:?}, no think");
        }
    }

    // The escort-strength timer.

    /// [`table`] of `avg_ships` holding the escort-strength fleets: 382
    /// led by ship 200 alone, 383 led by ship 200 with one more of it, each
    /// of government 131.
    fn strong(avg_ships: u32) -> SpawnTable {
        let mut strong = table(avg_ships);
        for (id, escorts) in [(382, 0), (383, 1)] {
            strong.fleets.insert(
                FleetId(id),
                FleetRecord {
                    id: FleetId(id),
                    lead: Some(ShipId(200)),
                    escorts: vec![EscortRecord {
                        ship: ShipId(200),
                        min: escorts,
                        max: escorts,
                    }],
                    govt: Some(GovtId(131)),
                    link_syst: -1,
                    appear_on: Test::default(),
                    quote: None,
                    flags: 0,
                },
            );
        }
        strong
    }

    /// A world of no stellars whose player's escorts carry `holds` tons.
    fn escorted(holds: i64) -> World<'static> {
        World {
            escort_tons: holds,
            ..World::new(&[])
        }
    }

    /// Traffic in a system of [`strong`] of `avg_ships`, its timer armed
    /// to 30 + `draw`.
    fn armed(avg_ships: u32, draw: u32) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.enter(strong(avg_ships), &mut Draws::of(&[]));
        traffic.arm_strength_timer(&mut Draws::of(&[draw]));
        traffic
    }

    /// Ticks `traffic` `ticks` times in `world`, on draws of the last
    /// outcome.
    fn ticked(traffic: &mut Traffic, world: &World, ticks: u32) {
        for _ in 0..ticks {
            traffic.tick_in(&Keep, *world, &[], &mut Draws::of(&[]));
        }
    }

    /// The ships of `traffic`'s NPCs, with their governments and AI
    /// types.
    fn ships_of(traffic: &Traffic) -> Vec<(ShipId, Option<GovtId>, AiType)> {
        traffic
            .npcs()
            .iter()
            .map(|npc| (npc.ship, npc.govt, npc.ai_type))
            .collect()
    }

    #[test]
    fn arming_draws_below_30() {
        let mut traffic = Traffic::new();
        let mut chance = Draws::of(&[4]);
        traffic.arm_strength_timer(&mut chance);
        assert_eq!(chance.asked, [30]);
    }

    #[test]
    fn escorts_above_49_tons_bring_fleet_382_on_the_tick_after_the_countdown() {
        for draw in [0, 7] {
            let mut traffic = armed(0, draw);
            ticked(&mut traffic, &escorted(60), 30 + draw);
            assert_eq!(traffic.npcs(), [], "armed at {}", 30 + draw);
            ticked(&mut traffic, &escorted(60), 1);
            assert_eq!(
                ships_of(&traffic),
                [(ShipId(200), Some(GovtId(131)), AiType::Interceptor)],
                "its lead an interceptor"
            );
            assert_eq!(
                traffic.npcs()[0].mode,
                Mode::JumpingIn { tick: 1 },
                "by hyperspace, gliding from the tick it came"
            );
        }
    }

    #[test]
    fn escorts_above_199_tons_bring_fleet_383_its_escorts_their_own_ai() {
        let mut traffic = armed(0, 0);
        ticked(&mut traffic, &escorted(250), 31);
        assert_eq!(
            ships_of(&traffic),
            [
                (ShipId(200), Some(GovtId(131)), AiType::Interceptor),
                (ShipId(200), Some(GovtId(131)), AiType::Warship),
            ]
        );
        assert_eq!(traffic.npcs()[1].leader, Some(traffic.npcs()[0].id));
    }

    #[test]
    fn escorts_of_49_tons_or_less_bring_nothing_then_or_later() {
        let mut traffic = armed(0, 0);
        ticked(&mut traffic, &escorted(49), 31);
        ticked(&mut traffic, &escorted(250), 200);
        assert_eq!(traffic.npcs(), [], "fired once, and disarmed");
    }

    #[test]
    fn the_timer_fires_once() {
        let mut traffic = armed(0, 0);
        ticked(&mut traffic, &escorted(60), 31);
        ticked(&mut traffic, &escorted(60), 200);
        assert_eq!(traffic.npcs().len(), 1);
    }

    #[test]
    fn the_timer_fires_in_a_system_at_avg_ships_and_after_the_arrival_roll() {
        // An empty system of AvgShips 1, so the arrival is rolled.
        let mut traffic = armed(0, 0);
        traffic.retarget(strong(1), &BTreeSet::new(), Leave::Dropped);
        ticked(&mut traffic, &escorted(60), 30);
        let mut chance = Draws::of(&[]);
        traffic.tick_in(&Keep, escorted(60), &[], &mut chance);
        assert_eq!(
            chance.asked,
            [500, 360, 3],
            "the arrival roll, then the lead's jump-in angle and aggression"
        );
        assert_eq!(traffic.npcs().len(), 1);
        // Now at its AvgShips: no arrival roll, and the timer still runs.
        let mut traffic = armed(0, 0);
        traffic.retarget(strong(1), &BTreeSet::new(), Leave::Dropped);
        traffic.add_npc(crate::testkit::npc(0, ShipStats::new(FAST, &[])));
        ticked(&mut traffic, &escorted(60), 30);
        let mut chance = Draws::of(&[]);
        traffic.tick_in(&Keep, escorted(60), &[], &mut chance);
        assert_eq!(chance.asked, [360, 3]);
        assert_eq!(traffic.npcs().len(), 2);
    }

    #[test]
    fn a_strength_fleet_whose_appear_on_the_world_refuses_brings_nothing_then_or_later() {
        // `_HyperSpawnFleet` honours the fleet only when its AppearOn holds
        // (@0x41cbb-0x41cda): `!b7` holds for a fresh pilot, but this
        // world's control bits refuse every test of bit 7.
        let mut traffic = Traffic::new();
        let mut gated = strong(0);
        gated
            .fleets
            .get_mut(&FleetId(382))
            .expect("fleet 382")
            .appear_on = Test::parse("!b7");
        traffic.enter(gated, &mut Draws::of(&[]));
        traffic.arm_strength_timer(&mut Draws::of(&[0]));
        let refusing = World {
            persons: PersonWorld {
                control_bits: &crate::testkit::RefuseBits(&[7]),
                ..PersonWorld::NONE
            },
            ..escorted(60)
        };
        ticked(&mut traffic, &refusing, 31);
        assert_eq!(traffic.npcs(), [], "refused when the timer fired");
        ticked(&mut traffic, &escorted(60), 200);
        assert_eq!(traffic.npcs(), [], "fired once, and disarmed");
    }

    #[test]
    fn a_strength_fleet_among_a_hypergate_may_come_out_of_it() {
        let sites = gated();
        let world = World {
            escort_tons: 60,
            ..World::new(&sites)
        };
        let mut traffic = armed(0, 0);
        ticked(&mut traffic, &world, 30);
        assert_eq!(traffic.npcs(), []);
        // The gate of the two stellars, then the lead's aggression.
        let mut chance = Draws::of(&[1, 0]);
        traffic.tick_in(&Keep, world, &[], &mut chance);
        assert_eq!(chance.asked, [2, 3], "as `spawn::fleet` draws");
        let lead = &traffic.npcs()[0];
        assert_eq!(
            (lead.ship, lead.ai_type),
            (ShipId(200), AiType::Interceptor)
        );
        assert_eq!(
            lead.mode,
            Mode::Emerging {
                ticks_left: emerge::EMERGE_TICKS - 1
            },
            "out of the gate, not by hyperspace"
        );
        assert_eq!(lead.state.position, Vec2::new(300.0, -400.0));
        assert_eq!(lead.state.heading, 90.0);
    }

    #[test]
    fn the_quotes_of_the_fleets_added_are_taken_once() {
        let mut quoting = strong(0);
        quoting
            .fleets
            .get_mut(&FleetId(382))
            .expect("fleet 382")
            .quote = Some(7600);
        let mut traffic = Traffic::new();
        traffic.enter(quoting, &mut Draws::of(&[]));
        traffic.arm_strength_timer(&mut Draws::of(&[0]));
        ticked(&mut traffic, &escorted(60), 30);
        assert!(traffic.take_quotes().is_empty(), "not yet come");
        ticked(&mut traffic, &escorted(60), 1);
        assert_eq!(traffic.npcs().len(), 1);
        assert_eq!(traffic.take_quotes(), [7600], "announced");
        assert!(traffic.take_quotes().is_empty(), "taken");
    }

    #[test]
    fn an_npc_carries_the_cargo_its_ship_comes_with() {
        let mut traffic = Traffic::new();
        traffic.enter(table(0), &mut Draws::of(&[]));
        traffic.bring_in_within(
            vec![
                NewShip {
                    cargo: Some((crate::market::Good::Commodity(2), 9)),
                    ..mission_ship(0, 0.0)
                },
                mission_ship(0, 0.0),
            ],
            SPAWN_RESERVE,
        );
        assert_eq!(
            traffic
                .npcs()
                .iter()
                .map(|npc| npc.cargo)
                .collect::<Vec<_>>(),
            [Some((crate::market::Good::Commodity(2), 9)), None]
        );
    }

    #[test]
    fn a_quoted_ship_that_cannot_come_adds_no_quote() {
        let quoted = |ship| NewShip {
            ship: ShipId(ship),
            quote: Some(7600),
            ..mission_ship(0, 0.0)
        };
        let mut traffic = Traffic::new();
        traffic.enter(table(0), &mut Draws::of(&[]));
        traffic.bring_in_within(vec![quoted(999)], SPAWN_RESERVE);
        assert!(traffic.take_quotes().is_empty(), "ship 999 has no record");
        traffic.bring_in_within(vec![quoted(200)], SPAWN_RESERVE);
        assert_eq!(traffic.take_quotes(), [7600]);
    }

    #[test]
    fn entering_or_leaving_for_another_system_drops_the_quotes_not_taken() {
        let quoted = || {
            let mut traffic = Traffic::new();
            traffic.enter(table(0), &mut Draws::of(&[]));
            traffic.bring_in_within(
                vec![NewShip {
                    quote: Some(7600),
                    ..mission_ship(0, 0.0)
                }],
                SPAWN_RESERVE,
            );
            traffic
        };
        let mut entered = quoted();
        entered.enter(table(0), &mut Draws::of(&[]));
        assert!(entered.take_quotes().is_empty());
        let mut left = quoted();
        left.retarget(table(0), &BTreeSet::new(), Leave::Dropped);
        assert!(left.take_quotes().is_empty());
        assert_eq!(quoted().take_quotes(), [7600], "queued");
    }

    #[test]
    fn new_traffic_is_disarmed() {
        let mut traffic = Traffic::new();
        traffic.enter(strong(0), &mut Draws::of(&[]));
        ticked(&mut traffic, &escorted(250), 200);
        assert_eq!(traffic.npcs(), []);
    }

    #[test]
    fn entering_or_going_on_into_a_system_keeps_the_timer_counting() {
        let mut traffic = armed(0, 0);
        ticked(&mut traffic, &escorted(60), 10);
        traffic.enter(strong(0), &mut Draws::of(&[]));
        ticked(&mut traffic, &escorted(60), 20);
        traffic.retarget(strong(0), &BTreeSet::new(), Leave::Dropped);
        assert_eq!(traffic.npcs(), []);
        ticked(&mut traffic, &escorted(60), 1);
        assert_eq!(traffic.npcs().len(), 1, "on the 31st tick in all");
    }

    // The arrivals pass.

    #[test]
    fn the_arrivals_pass_brings_its_arrival_then_the_strength_fleet() {
        // An empty system of AvgShips 1, so the arrival is rolled; the
        // timer at 0.
        let mut traffic = armed(0, 0);
        traffic.retarget(strong(1), &BTreeSet::new(), Leave::Dropped);
        ticked(&mut traffic, &escorted(60), 30);
        assert_eq!(traffic.npcs(), []);
        let mut chance = Draws::of(&[0, 6, 6, 0, 0, 90]);
        traffic.enter_more_ships(escorted(60), &mut chance);
        assert_eq!(
            ships_of(&traffic),
            [
                (ShipId(200), Some(GovtId(130)), AiType::WimpyTrader),
                (ShipId(200), Some(GovtId(131)), AiType::Interceptor),
            ],
            "the arrival of düde 128, then fleet 382's lead"
        );
    }

    #[test]
    fn the_arrivals_pass_moves_no_npc_and_counts_no_tick() {
        // At its AvgShips: no arrival.
        let mut traffic = populated(1, 2);
        traffic.npcs[0].state.velocity = Vec2::new(3.0, -1.0);
        let before = traffic.npcs()[0].clone();
        traffic.enter_more_ships(World::new(&[]), &mut Draws::of(&[]));
        assert_eq!(traffic.npcs(), [before], "not moved");
        assert_eq!(traffic.departed(), []);
        // NPC 0 decides on tick 0 of every 2: still due after the pass.
        let behaviour = Recording::deciding(Goal::Idle);
        traffic.tick_in(&behaviour, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(behaviour.take(), [NpcId(0)], "no tick counted");
    }

    #[test]
    fn the_arrivals_pass_counts_the_strength_timer_down_as_a_tick_does() {
        // Armed at 30, then 29 ticks: at 1.
        let mut traffic = armed(0, 0);
        ticked(&mut traffic, &escorted(60), 29);
        traffic.enter_more_ships(escorted(60), &mut Draws::of(&[]));
        assert_eq!(traffic.npcs(), [], "at 0");
        ticked(&mut traffic, &escorted(60), 1);
        assert_eq!(
            ships_of(&traffic),
            [(ShipId(200), Some(GovtId(131)), AiType::Interceptor)],
            "fired on the tick after the pass"
        );
    }

    // The ships an in-flight move leaves behind.

    const OLD: SystemId = SystemId(130);
    const NEW: SystemId = SystemId(131);

    /// Leaves `OLD` for `NEW`.
    const AWAY: Leave = Leave::Held { from: OLD, to: NEW };

    /// Comes back to `OLD` from `NEW`.
    const BACK: Leave = Leave::Held { from: NEW, to: OLD };

    /// A ship to add.
    fn ship() -> Npc {
        crate::testkit::npc(0, ShipStats::new(FAST, &[]))
    }

    /// Traffic in `OLD` of [`table`] of 0, holding three NPCs at berths
    /// 1, 2 and 3, numbered 0, 1 and 2.
    fn three_here() -> Traffic {
        let mut traffic = Traffic::new();
        traffic.enter(table(0), &mut Draws::of(&[]));
        for _ in 0..3 {
            traffic.add_npc(ship());
        }
        traffic
    }

    /// The numbers of `traffic`'s NPCs, in `Vec` order.
    fn ids(traffic: &Traffic) -> Vec<NpcId> {
        traffic.npcs().iter().map(|npc| npc.id).collect()
    }

    #[test]
    fn a_ship_left_behind_holds_its_berth_until_the_next_tick_ends() {
        let mut traffic = three_here();
        traffic.retarget(table(0), &BTreeSet::from([NpcId(1)]), AWAY);
        assert_eq!(berths(&traffic), [2], "the kept one");
        traffic.add_npc(ship());
        assert_eq!(berths(&traffic), [2, 4], "past those left behind");
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        traffic.add_npc(ship());
        assert_eq!(berths(&traffic), [2, 4, 1], "freed by the tick's end");
    }

    #[test]
    fn an_arrival_in_the_next_tick_takes_a_berth_past_those_left_behind() {
        let mut traffic = three_here();
        traffic.retarget(table(1), &BTreeSet::new(), AWAY);
        tick_arrival(&mut traffic);
        assert_eq!(berths(&traffic), [4]);
    }

    /// [`peopled`] of `avg_ships`, its Ace listed as person 511.
    fn ace_as_511(avg_ships: u32) -> SpawnTable {
        let mut table = peopled(avg_ships);
        let mut ace = table.persons.remove(&PersonId(510)).expect("Ace");
        ace.record.id = PersonId(511);
        table.persons.insert(PersonId(511), ace);
        table
    }

    #[test]
    fn a_person_left_behind_cannot_arrive_in_the_next_pass() {
        // Ace, set up in the old system as person 510.
        let mut traffic = entered(&World::new(&[]), &[0, 382, 750, 750, 0]);
        assert_eq!(traffic.npcs().len(), 1);
        // The new system lists no 510, but an Ace as 511.
        traffic.retarget(ace_as_511(1), &BTreeSet::new(), AWAY);
        let mut chance = Draws::of(&[0, 0]);
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut chance);
        assert_eq!(
            chance.asked,
            [500, 7],
            "Ace is still in the ship table: no draw of 1022"
        );
        assert_eq!(traffic.npcs(), []);
        let mut chance = Draws::of(&[0, 0, 383]);
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut chance);
        assert_eq!(&chance.asked[..3], [500, 7, 1022], "free to come");
        assert_eq!(
            traffic.npcs()[0].person.map(|person| person.id),
            Some(PersonId(511))
        );
    }

    #[test]
    fn the_ships_left_behind_are_not_in_the_system() {
        let mut traffic = three_here();
        traffic.retarget(table(3), &BTreeSet::from([NpcId(2)]), AWAY);
        assert_eq!(ids(&traffic), [NpcId(2)]);
        assert_eq!(traffic.strangers(), 1, "the kept one alone");
    }

    #[test]
    fn a_move_back_before_the_next_tick_brings_them_back_as_they_were() {
        let mut traffic = three_here();
        traffic.npcs[0].state.velocity = Vec2::new(3.0, -1.0);
        traffic.npcs[2].goal = Goal::Attack(ShipRef::Player);
        let before = traffic.npcs().to_vec();
        traffic.retarget(table(0), &BTreeSet::from([NpcId(1)]), AWAY);
        traffic.retarget(table(0), &BTreeSet::from([NpcId(1)]), BACK);
        assert_eq!(traffic.npcs(), before, "the kept one in its place");
        assert_eq!(traffic.berths_taken(), 3, "none held twice");

        let mut traffic = three_here();
        traffic.retarget(table(0), &BTreeSet::from([NpcId(1)]), AWAY);
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        traffic.retarget(table(0), &BTreeSet::from([NpcId(1)]), BACK);
        assert_eq!(ids(&traffic), [NpcId(1)], "after a tick, none");
    }

    #[test]
    fn a_move_on_restores_only_the_ships_left_in_the_system_entered() {
        let mut traffic = three_here();
        traffic.retarget(table(0), &BTreeSet::from([NpcId(1)]), AWAY);
        traffic.retarget(
            table(0),
            &BTreeSet::new(),
            Leave::Held {
                from: NEW,
                to: SystemId(132),
            },
        );
        assert_eq!(traffic.npcs(), [], "none were left in 132");
        assert_eq!(traffic.berths_taken(), 3, "all three held");
        traffic.retarget(
            table(0),
            &BTreeSet::new(),
            Leave::Held {
                from: SystemId(132),
                to: NEW,
            },
        );
        assert_eq!(ids(&traffic), [NpcId(1)], "the one left in 131");
    }

    #[test]
    fn entering_a_system_drops_the_ships_left_behind() {
        let mut traffic = three_here();
        traffic.retarget(table(0), &BTreeSet::new(), AWAY);
        traffic.enter(table(0), &mut Draws::of(&[]));
        assert_eq!(traffic.berths_taken(), 0);
        traffic.add_npc(ship());
        assert_eq!(berths(&traffic), [1], "the berths free");
        traffic.retarget(table(0), &BTreeSet::new(), BACK);
        assert_eq!(traffic.npcs(), [], "none come back");
    }

    #[test]
    fn a_ship_dropped_at_the_move_frees_its_berth_at_once() {
        let mut traffic = three_here();
        traffic.retarget(table(0), &BTreeSet::from([NpcId(1)]), Leave::Dropped);
        assert_eq!(traffic.berths_taken(), 1);
        traffic.add_npc(ship());
        assert_eq!(berths(&traffic), [2, 1]);
        traffic.retarget(table(0), &BTreeSet::new(), BACK);
        assert_eq!(traffic.npcs(), [], "nothing held to come back");
    }

    #[test]
    fn the_berths_taken_count_the_ships_left_behind() {
        let mut traffic = three_here();
        assert_eq!(traffic.berths_taken(), 3);
        traffic.retarget(table(0), &BTreeSet::from([NpcId(0)]), AWAY);
        assert_eq!(traffic.berths_taken(), 3, "one here, two held");
        traffic.add_npc(ship());
        assert_eq!(traffic.berths_taken(), 4);
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.berths_taken(), 2, "the held ones freed");
    }

    // The slot cap: `_GenericRandomShipSpawn`'s reserve.

    /// Traffic of [`table`] of 0 holding an NPC at each of `berths`.
    fn at_berths(berths: impl IntoIterator<Item = u32>) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.enter(table(0), &mut Draws::of(&[]));
        for berth in berths {
            traffic.add_npc_at(ship(), berth);
        }
        traffic
    }

    #[test]
    fn a_spawn_takes_the_first_free_berth_below_its_reserve() {
        assert_eq!(at_berths(1..=54).free_berth(SPAWN_RESERVE), Some(55));
        assert_eq!(at_berths(1..=55).free_berth(SPAWN_RESERVE), None);
        assert_eq!(at_berths(1..=55).free_berth(0), Some(56));
        assert_eq!(at_berths(1..=55).free_berth(CAPTURE_RESERVE), Some(56));
        assert_eq!(at_berths(1..=61).free_berth(CAPTURE_RESERVE), Some(62));
        assert_eq!(at_berths(1..=62).free_berth(CAPTURE_RESERVE), None);
        assert_eq!(at_berths(1..=62).free_berth(0), Some(63));
        assert_eq!(at_berths(1..=63).free_berth(0), None);
    }

    #[test]
    fn the_slot_cap_goes_by_slot_not_by_count() {
        let gapped = at_berths((1..=50).chain(59..=63));
        assert_eq!(gapped.berths_taken(), 55);
        assert_eq!(gapped.free_berth(SPAWN_RESERVE), Some(51));
        let high = at_berths((1..=54).chain([60]));
        assert_eq!(high.free_berth(SPAWN_RESERVE), Some(55));
    }

    #[test]
    fn a_held_ship_keeps_its_slot_from_a_spawn() {
        let mut traffic = at_berths(1..=55);
        let last = traffic.npcs().last().expect("55 ships").id;
        let keep: BTreeSet<NpcId> = ids(&traffic).into_iter().filter(|&id| id != last).collect();
        traffic.retarget(table(0), &keep, AWAY);
        assert_eq!(traffic.npcs().len(), 54);
        assert_eq!(traffic.free_berth(SPAWN_RESERVE), None, "55 held");
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(traffic.free_berth(SPAWN_RESERVE), Some(55), "freed");
    }

    /// A fleet of ship 200: its lead, quoted with `STR#` 7600, then
    /// `escorts` escorts following it.
    fn quoted_fleet(escorts: usize) -> Vec<NewShip> {
        let lead = NewShip {
            mission: None,
            quote: Some(7600),
            ..mission_ship(-1, 0.0)
        };
        let escort = NewShip {
            lead: Some(0),
            quote: None,
            ..lead
        };
        let mut fleet = vec![lead];
        fleet.extend(std::iter::repeat_n(escort, escorts));
        fleet
    }

    #[test]
    fn a_fleet_with_no_slot_for_its_lead_brings_nothing_and_no_quote() {
        let mut traffic = at_berths(1..=55);
        traffic.add(quoted_fleet(2), &BTreeSet::new(), Some(SPAWN_RESERVE));
        assert_eq!(traffic.npcs().len(), 55);
        assert!(traffic.take_quotes().is_empty(), "no line, no beep");
    }

    #[test]
    fn an_escort_with_no_slot_is_skipped_and_the_lead_keeps_its_quote() {
        let mut traffic = at_berths(1..=53);
        traffic.add(quoted_fleet(2), &BTreeSet::new(), Some(SPAWN_RESERVE));
        let npcs = &traffic.npcs()[53..];
        assert_eq!(npcs.len(), 2, "the lead and one escort");
        assert_eq!((npcs[0].berth, npcs[1].berth), (54, 55));
        assert_eq!(npcs[1].leader, Some(npcs[0].id));
        assert_eq!(traffic.take_quotes(), [7600]);
    }

    #[test]
    fn a_skipped_ship_does_not_shift_the_leaders_after_it() {
        let mut traffic = at_berths([]);
        let mut ships = vec![NewShip {
            ship: ShipId(999),
            ..mission_ship(-1, 0.0)
        }];
        ships.extend(quoted_fleet(1).into_iter().map(|ship| NewShip {
            lead: ship.lead.map(|_| 1),
            ..ship
        }));
        traffic.add(ships, &BTreeSet::new(), None);
        let npcs = traffic.npcs();
        assert_eq!(npcs.len(), 2, "ship 999 has no record");
        assert_eq!(npcs[1].leader, Some(npcs[0].id));
    }

    #[test]
    fn an_escort_whose_lead_has_no_record_is_skipped() {
        let mut traffic = at_berths([]);
        let mut fleet = quoted_fleet(1);
        fleet[0].ship = ShipId(999);
        traffic.add(fleet, &BTreeSet::new(), None);
        assert_eq!(traffic.npcs(), [], "no lead, no escort");
        assert!(traffic.take_quotes().is_empty());
    }

    #[test]
    fn setup_stops_at_fifty_five_ships() {
        let draws: Vec<u32> = (0..60).flat_map(|i| placed(750 + i, 750)).collect();
        let mut traffic = Traffic::new();
        traffic.enter(table(60), &mut Draws::of(&draws));
        assert_eq!(berths(&traffic), (1..=55).collect::<Vec<_>>());
    }

    #[test]
    fn an_arrival_finds_no_slot_once_fifty_five_ships_are_in_the_system() {
        let mut traffic = at_berths(1..=55);
        traffic.table = table(60);
        tick_arrival(&mut traffic);
        assert_eq!(traffic.npcs().len(), 55);
        let mut traffic = at_berths(1..=54);
        traffic.table = table(60);
        tick_arrival(&mut traffic);
        assert_eq!(traffic.npcs().len(), 55, "the last slot taken");
    }

    #[test]
    fn free_berth_skipping_skips_the_held_berths_and_stops_below_the_reserve() {
        let traffic = at_berths(1..=53);
        assert_eq!(
            traffic.free_berth_skipping(SPAWN_RESERVE, &BTreeSet::from([54])),
            Some(55)
        );
        assert_eq!(
            traffic.free_berth_skipping(SPAWN_RESERVE, &BTreeSet::from([54, 55])),
            None
        );
        assert_eq!(
            at_berths(1..=62).free_berth_skipping(0, &BTreeSet::from([63])),
            None
        );
    }

    #[test]
    fn mission_ships_set_up_find_no_slot_once_55_berths_are_held() {
        let set_up = |spawn_reserve| {
            let mut traffic = Traffic::new();
            traffic.reserve((1..=55).collect());
            traffic.enter_with(
                table(0),
                World {
                    spawn_reserve,
                    ..World::new(&[])
                },
                vec![mission_ship(-1, 0.0)],
                &mut Draws::of(&[]),
            );
            traffic
        };
        assert_eq!(set_up(SPAWN_RESERVE).npcs(), [], "no slot");
        let bible = set_up(0);
        let npcs = bible.npcs();
        assert_eq!(npcs.len(), 1);
        assert!(npcs[0].mission.is_some());
        assert_eq!(npcs[0].berth, 56);
    }

    #[test]
    fn mission_ships_brought_in_find_no_slot_once_55_berths_are_held() {
        let mut traffic = at_berths(1..=55);
        let came = traffic.bring_in_within(vec![mission_ship(-1, 0.0)], SPAWN_RESERVE);
        assert_eq!((came, traffic.npcs().len()), (0, 55));
    }

    /// [`armed`] of 0 to fire after 30 ticks, fleet 382 quoted with
    /// `STR#` 7600, holding an NPC at each of berths 1 to `ships`.
    fn crowded_strength(ships: u32) -> Traffic {
        let mut traffic = armed(0, 0);
        traffic
            .table
            .fleets
            .get_mut(&FleetId(382))
            .expect("fleet 382")
            .quote = Some(7600);
        for berth in 1..=ships {
            traffic.add_npc_at(ship(), berth);
        }
        traffic
    }

    #[test]
    fn the_escort_strength_fleet_finds_no_slot_once_fifty_five_ships_are_in_the_system() {
        let mut traffic = crowded_strength(55);
        ticked(&mut traffic, &escorted(60), 31);
        assert_eq!(traffic.npcs().len(), 55, "no fleet");
        assert!(traffic.take_quotes().is_empty(), "no line");
        let mut traffic = crowded_strength(54);
        ticked(&mut traffic, &escorted(60), 31);
        assert_eq!(traffic.npcs().len(), 55);
        assert_eq!(traffic.take_quotes(), [7600]);
    }

    #[test]
    fn by_the_bible_a_spawn_may_take_any_slot() {
        let bible = World {
            spawn_reserve: 0,
            ..escorted(60)
        };
        let mut traffic = crowded_strength(55);
        ticked(&mut traffic, &bible, 31);
        assert_eq!(traffic.npcs().last().map(|npc| npc.berth), Some(56));
        let mut traffic = crowded_strength(62);
        ticked(&mut traffic, &bible, 31);
        assert_eq!(traffic.npcs().last().map(|npc| npc.berth), Some(63));
        let mut traffic = crowded_strength(63);
        ticked(&mut traffic, &bible, 31);
        assert_eq!(traffic.npcs().len(), 63, "the table full");
        assert_eq!(World::new(&[]).spawn_reserve, SPAWN_RESERVE);
    }
}
