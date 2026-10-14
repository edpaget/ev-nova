//! NPC traffic: the ships that come and go in the player's system.
//!
//! - [`table`]: the [`SpawnTable`] a system's traffic is drawn from, its
//!   `DudeTypes` and `LinkSyst`s decoded and its ship types' stats
//!   resolved when the system is entered.
//! - [`spawn`]: the rolls: the initial population, the arrivals over time,
//!   the fleets and the persons (see [`person`](crate::person)), and
//!   where each ship starts.
//! - [`emerge`]: the hypergate or wormhole an arrival comes out of, and
//!   how long it waits there.
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
//! 1. the arrival roll, while fewer NPCs than `AvgShips` that are not the
//!    player's ([`Npc::is_players`]) nor its mission escorts are in the
//!    system, each arrival coming out of one of the world's stellars
//!    ([`emerge`]) or by hyperspace, then the escort-strength timer's tick ([`strength`]):
//!    when it fires, the fleet the [`World`]'s escorts' tons call for
//!    comes, whatever `AvgShips` says, its lead an interceptor. The
//!    timer is armed by its session ([`Traffic::arm_strength_timer`]),
//!    and entering or going on into a system keeps it;
//! 2. each flying, intact NPC answers the fight's strikes since the last
//!    tick ([`Behaviour::react`]): the ship it targets, its provocation,
//!    and any goal it takes at once;
//! 3. an NPC not intact loses its provocation;
//! 4. the decisions due on the AI timer, never for a ship still jumping
//!    in or emerging from a gate (waiting there or bursting out of it),
//!    one that is not intact, or one assisting the player while
//!    unprovoked (until it is done, or provoked): first every goal, then, each NPC
//!    seeing the new goals, the fire command it holds and the ship it
//!    targets. An NPC that goes back to an idle goal (not fighting) loses
//!    its provocation, and one that stops inspecting a ship remembers it,
//!    and, the player within its reach, has scanned the player
//!    ([`Traffic::take_scans`]);
//! 5. every NPC's cloak ([`cloak`]): a flying NPC's think
//!    cloaks or uncloaks it, one cloaked that can no longer cloak
//!    uncloaks, and its fade steps;
//! 6. the autopilot and a flight step for every NPC, each flying towards
//!    the ship its goal is about, the player's included (the one it
//!    assists, or escorts in formation), a fighter docking its carrier;
//! 7. and the removal of those that landed, jumped out or docked, each
//!    in [`Traffic::departed`] with how it left, for its session to take
//!    a fighter docked aboard.
//!
//! The fight ([`combat`](crate::combat)) damages them, and its session
//! takes out each one destroyed ([`Traffic::remove`]).
//!
//! The AI timer is the original's (`_AIDispatch`): an NPC decides on the
//! ticks where `tick % interval == id % interval`, so decisions are
//! staggered across NPCs. The original's interval is 1 at full speed and
//! grows when frames run slow; the simulation's is always
//! [`DECISION_INTERVAL`]. Steering and physics run every tick.

pub mod autopilot;
pub mod emerge;
pub mod npc;
pub mod spawn;
pub mod strength;
pub mod table;

use crate::ai::interceptor::within_reach;
use crate::ai::{Behaviour, Goal, PlayerSide, Reaction, Surroundings};
use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{GovtId, LandingSite, PersonId, ShipId};
use crate::chance::Chance;
use crate::cloak::{self, Cloak};
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::control::Gate;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::mission::ships::ShipBehav;
use crate::person::{PersonWorld, derelict_armor};
use autopilot::{Outcome, TopSpeeds};
use npc::{AiType, Mode, Npc, NpcId, NpcPerson};
use spawn::{Entry, JUMP_IN_TICKS, NewShip, PersonDraw, Stellars};
use strength::StrengthTimer;
use table::{SpawnPerson, SpawnTable};

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
}

impl<'a> World<'a> {
    /// `sites` alone: no player, no governments, no record, no person
    /// gone or grudging ([`PersonWorld::NONE`]), no escorts' tons, no
    /// factor on the led ships' top speed, and no damage aimed.
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
    /// The escort-strength timer, kept across systems.
    strength_timer: StrengthTimer,
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
            strength_timer: StrengthTimer::default(),
        }
    }

    /// Arms the escort-strength timer, drawing its ticks on `chance` (see
    /// [`strength`]).
    pub fn arm_strength_timer(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.strength_timer.arm(chance);
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
    /// (@0x42dc4-0x43194), none of which they use up. The scans not yet
    /// taken are dropped: the original scans at once, so none outlives its
    /// system.
    pub fn enter_with(
        &mut self,
        table: SpawnTable,
        world: World,
        missions: Vec<NewShip>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.table = table;
        self.npcs.clear();
        self.departed.clear();
        self.scans.clear();
        self.add(missions, world.persons.grudges);
        let mut draw = PersonDraw::new(world.persons);
        let first_stellar = world.sites.first().map(|site| site.position);
        let ships = spawn::initial(
            &self.table,
            &mut draw,
            first_stellar,
            world.stellars(),
            chance,
        );
        self.add(ships, world.persons.grudges);
    }

    /// Brings `ships` into the system as they are, numbered after every
    /// other NPC: the mission ships whose appear timer ran out.
    pub fn bring_in(&mut self, ships: Vec<NewShip>) {
        self.add(ships, &BTreeSet::new());
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
    /// behind, and nothing of the new system's initial population comes,
    /// only its arrivals over time. The NPCs' numbering goes on. The
    /// scans not yet taken are dropped, as entering a system drops them.
    pub fn retarget(&mut self, table: SpawnTable, keep: &BTreeSet<NpcId>) {
        self.table = table;
        self.npcs.retain(|npc| keep.contains(&npc.id));
        self.departed.clear();
        self.scans.clear();
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
        self.add(arrivals, world.persons.grudges);
        self.escort_strength(&world, draw.world.gate(), chance);
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
                npc.mode == Mode::Flying
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
        let states: Vec<(NpcId, ShipState)> =
            self.npcs.iter().map(|npc| (npc.id, npc.state)).collect();
        let player = world.player.map(|player| player.state);
        self.cloaks(player, &states);
        let speeds = self.top_speeds(&world);
        let outcomes: Vec<Outcome> = self
            .npcs
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
                autopilot::fly(npc, world.sites, other.as_ref(), speeds)
            })
            .collect();
        self.departed = self
            .npcs
            .iter()
            .zip(&outcomes)
            .filter(|(_, outcome)| **outcome != Outcome::Flying)
            .map(|(npc, outcome)| (npc.id, *outcome))
            .collect();
        let mut flying = outcomes.iter().map(|outcome| *outcome == Outcome::Flying);
        self.npcs.retain(|_| flying.next().unwrap_or(false));
        self.ticks += 1;
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
            Some(AiType::Interceptor),
            chance,
            &mut fleet,
        );
        self.add(fleet, world.persons.grudges);
    }

    /// Each flying, intact NPC answers each of `strikes` in `world` that
    /// provokes ([`Strike::provokes`]: not a death shockwave, which skips
    /// `_DamageShip`'s reaction block @0x3b44d), as `behaviour` says, all
    /// against the NPCs as they were before.
    fn react(&mut self, behaviour: &(impl Behaviour + ?Sized), world: &World, strikes: &[Strike]) {
        let strikes: Vec<&Strike> = strikes.iter().filter(|strike| strike.provokes).collect();
        if strikes.is_empty() {
            return;
        }
        let around = world.around(&self.npcs);
        let reactions: Vec<Vec<Reaction>> = self
            .npcs
            .iter()
            .map(|npc| {
                if npc.mode != Mode::Flying || npc.condition != Condition::Intact {
                    return Vec::new();
                }
                strikes
                    .iter()
                    .map(|strike| behaviour.react(npc, strike, &around))
                    .collect()
            })
            .collect();
        for (npc, reactions) in self.npcs.iter_mut().zip(reactions) {
            for reaction in reactions {
                if let Some(target) = reaction.target {
                    npc.target = Some(target);
                }
                npc.provoked += reaction.provoked;
                if let Some(goal) = reaction.goal {
                    npc.goal = goal;
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

    /// The first berth from 1 that no NPC in the system holds, none is
    /// reserved for, and `skip` does not hold.
    pub(crate) fn next_berth_skipping(&self, skip: &BTreeSet<u32>) -> u32 {
        (1..=u32::MAX)
            .find(|berth| {
                !self.reserved.contains(berth)
                    && !skip.contains(berth)
                    && self.npcs.iter().all(|npc| npc.berth != *berth)
            })
            .unwrap_or(u32::MAX)
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

    /// The names of the persons in the system.
    fn person_names(&self) -> BTreeSet<String> {
        self.npcs
            .iter()
            .filter_map(|npc| self.person(npc.person?.id))
            .map(|person| person.record.name.clone())
            .collect()
    }

    /// Adds `ships`, numbering each and pointing each escort at its lead; a
    /// person flies its fitted ship, holding a grudge when `grudges` say;
    /// a ship set up derelict ([`NewShip::derelict`]) starts disabled, its
    /// shield gone and its armour cut ([`derelict_armor`]).
    fn add(&mut self, ships: Vec<NewShip>, grudges: &BTreeSet<PersonId>) {
        let first = self.next_id;
        for ship in ships {
            let person = ship.person.and_then(|id| self.table.persons.get(&id));
            let Some(kind) = person
                .map(|person| &person.kind)
                .or_else(|| self.table.ships.get(&ship.ship))
            else {
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
            let berth = self.next_berth();
            self.npcs.push(Npc {
                id,
                berth,
                ship: ship.ship,
                govt: ship.govt,
                ai_type: ship.ai_type,
                leader: ship.lead.map(|lead| NpcId(first + lead as u32)),
                class: kind.escort_class,
                escort: None,
                stats: kind.stats,
                reserves,
                state: ship.state,
                mode: match ship.entry {
                    Entry::Placed => Mode::Flying,
                    Entry::JumpingIn => Mode::JumpingIn {
                        ticks_left: JUMP_IN_TICKS,
                    },
                    Entry::Emerging { ticks_left } => Mode::Emerging { ticks_left },
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
                self_repair: kind.self_repair,
                former: None,
            });
        }
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
            assert_eq!(
                npc.mode,
                Mode::JumpingIn {
                    ticks_left: JUMP_IN_TICKS
                }
            );
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
    fn a_ship_jumping_in_never_decides_and_glides_until_it_is_in() {
        let mut traffic = populated(1, 1);
        traffic.table.avg_ships = 2;
        let behaviour = Recording::deciding(Goal::Idle);
        // An arrival: 0, not a person, not a fleet, düde 128, ship 200, at
        // angle 0.
        traffic.tick(&behaviour, &[], &mut Draws::of(&[0, 6, 6, 0, 0, 0]));
        assert_eq!(traffic.npcs().len(), 2);
        assert_eq!(behaviour.take(), [NpcId(0)], "the newcomer is jumping in");
        for _ in 1..JUMP_IN_TICKS {
            traffic.tick(&behaviour, &[], &mut Draws::of(&[2]));
            assert_eq!(behaviour.take(), [NpcId(0)]);
        }
        assert_eq!(traffic.npcs()[1].mode, Mode::Flying);
        traffic.tick(&behaviour, &[], &mut Draws::of(&[2]));
        assert_eq!(behaviour.take(), [NpcId(0), NpcId(1)]);
    }

    #[test]
    fn a_ship_coming_out_of_a_gate_never_decides_until_it_is_out() {
        let mut traffic = populated(2, 1);
        traffic.npcs[1].mode = Mode::Emerging { ticks_left: 2 };
        let behaviour = Recording::deciding(Goal::JumpOut);
        let mut ticks = 0;
        while traffic.npcs()[1].mode != Mode::Flying {
            traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
            assert_eq!(behaviour.take(), [NpcId(0)], "tick {ticks}");
            assert_eq!(traffic.npcs()[1].goal, Goal::Idle);
            ticks += 1;
            assert!(ticks < 20, "out");
        }
        // Two ticks waiting; top speed 6: 30 down to 9 is 8 ticks.
        assert_eq!(ticks, 10);
        traffic.tick(&behaviour, &[], &mut Draws::of(&[]));
        assert_eq!(behaviour.take(), [NpcId(0), NpcId(1)]);
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
    fn an_arrival_among_a_hypergate_comes_out_of_it_and_flies_once_out() {
        let mut traffic = populated(1, 1);
        traffic.table.avg_ships = 2;
        let sites = gated();
        // An arrival: 0, not a person, not a fleet, düde 128, ship 200, at
        // angle 0, then the gate.
        traffic.tick(&Keep, &sites, &mut Draws::of(&[0, 6, 6, 0, 0, 0, 1]));
        let arrived = &traffic.npcs()[1];
        assert_eq!(arrived.mode, Mode::Emerging { ticks_left: 59 });
        assert_eq!(arrived.state.position, Vec2::new(300.0, -400.0));
        assert_eq!(arrived.state.heading, 90.0);
        for _ in 1..emerge::EMERGE_TICKS {
            traffic.tick(&Keep, &sites, &mut Draws::of(&[2]));
        }
        assert_eq!(traffic.npcs()[1].mode, Mode::LeavingGate { tick: 0 });
        assert_eq!(traffic.npcs()[1].state.position, Vec2::new(300.0, -400.0));
        for _ in 0..8 {
            traffic.tick(&Keep, &sites, &mut Draws::of(&[2]));
        }
        let out = &traffic.npcs()[1];
        assert_eq!(out.mode, Mode::Flying, "top speed 6: 30 down to 9");
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
        // Each tick slows the drift by the drag before it moves.
        let mut expected = Vec2::ZERO;
        let mut velocity = Vec2::new(0.25, 0.0);
        for _ in 0..4 {
            velocity = velocity * crate::flight::DISABLED_DRAG;
            expected = expected + velocity;
        }
        assert_eq!(traffic.npcs()[0].state.position, expected);
        assert_eq!(traffic.npcs()[1].goal, Goal::JumpOut);
        traffic.npcs_mut()[0].condition = Condition::Dying {
            ticks_left: 2,
            starting: false,
        };
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
        traffic.bring_in(vec![mission_ship(-1, 0.0), mission_ship(-1, 1.0)]);
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
            provokes: true,
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
    fn a_strike_that_provokes_nothing_is_answered_by_no_one() {
        let mut traffic = populated(2, 1);
        let fighting = Fighting::deciding(Goal::Idle);
        let shockwave = Strike {
            provokes: false,
            ..struck(1, 4.0)
        };
        traffic.tick_in(
            &fighting,
            World::new(&[]),
            &[shockwave, struck(0, 2.0)],
            &mut Draws::of(&[]),
        );
        let reacted: Vec<String> = fighting
            .log
            .borrow()
            .iter()
            .filter(|line| line.starts_with("react"))
            .cloned()
            .collect();
        assert_eq!(
            reacted,
            ["react 0 Npc(NpcId(0))", "react 1 Npc(NpcId(0))"],
            "only the attack is answered"
        );
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

    #[test]
    fn only_a_flying_intact_npc_answers_a_strike() {
        let mut traffic = populated(3, 1);
        traffic.npcs[1].condition = Condition::Disabled;
        traffic.npcs[2].mode = Mode::JumpingIn { ticks_left: 9 };
        let fighting = Fighting::deciding(Goal::Idle);
        traffic.tick_in(
            &fighting,
            World::new(&[]),
            &[struck(0, 1.0)],
            &mut Draws::of(&[]),
        );
        let reacted: Vec<String> = fighting
            .log
            .borrow()
            .iter()
            .filter(|line| line.starts_with("react"))
            .cloned()
            .collect();
        assert_eq!(reacted, ["react 0 Npc(NpcId(0))"]);
    }

    #[test]
    fn a_disabled_npc_drags_once_per_traffic_tick() {
        let mut traffic = populated(2, 1);
        traffic.npcs[1].condition = Condition::Disabled;
        traffic.npcs[1].state.velocity = Vec2::new(1.0, -2.0);
        let idle = Fighting::deciding(Goal::Idle);
        traffic.tick_in(&idle, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(
            traffic.npcs()[1].state.velocity,
            Vec2::new(1.0, -2.0) * crate::flight::DISABLED_DRAG
        );
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
        left.retarget(SpawnTable::default(), &BTreeSet::from([kept]));
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
            if npc.mode == Mode::Flying {
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
        for _ in 0..=JUMP_IN_TICKS {
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
        traffic.bring_in(vec![mission_ship(0, 9.0), mission_ship(1, 9.0)]);
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
            traffic.bring_in(vec![
                NewShip {
                    derelict: true,
                    ..mission_ship(-1, 0.0)
                },
                mission_ship(-1, 1.0),
            ]);
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
            traffic.bring_in(vec![mission_ship(-1, x)]);
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
            npc.mode = Mode::JumpingIn { ticks_left: 9 };
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
        let mut able = with_mission_ship(ZEPHYR, false, 0.0);
        able.npcs[0].cloak = Cloak {
            fade: FADE_FULL,
            direction: 0,
        };
        able.npcs[0].mode = Mode::JumpingIn { ticks_left: 9 };
        able.tick_in(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        assert_eq!(able.npcs()[0].cloak.direction, 0, "jumping in, no think");
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
                Mode::JumpingIn {
                    ticks_left: JUMP_IN_TICKS - 1
                },
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
        traffic.retarget(strong(1), &BTreeSet::new());
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
        traffic.retarget(strong(1), &BTreeSet::new());
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
        traffic.retarget(strong(0), &BTreeSet::new());
        assert_eq!(traffic.npcs(), []);
        ticked(&mut traffic, &escorted(60), 1);
        assert_eq!(traffic.npcs().len(), 1, "on the 31st tick in all");
    }
}
