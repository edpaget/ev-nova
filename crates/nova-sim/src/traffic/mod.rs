//! NPC traffic: the ships that come and go in the player's system.
//!
//! - [`table`]: the [`SpawnTable`] a system's traffic is drawn from, its
//!   `DudeTypes` and `LinkSyst`s decoded and its ship types' stats
//!   resolved when the system is entered.
//! - [`spawn`]: the rolls: the initial population, the arrivals over time,
//!   the fleets and the persons (see [`person`](crate::person)), and
//!   where each ship starts.
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
//! 1. the arrival roll;
//! 2. each flying, intact NPC answers the fight's strikes since the last
//!    tick ([`Behaviour::react`]): the ship it targets, its provocation,
//!    and any goal it takes at once;
//! 3. an NPC not intact loses its provocation;
//! 4. the decisions due on the AI timer, never for a ship still jumping
//!    in, one that is not intact, or one assisting the player while
//!    unprovoked (until it is done, or provoked): first every goal, then, each NPC
//!    seeing the new goals, the fire command it holds and the ship it
//!    targets. An NPC that goes back to an idle goal (not fighting) loses
//!    its provocation, and one that stops inspecting a ship remembers it;
//! 5. the autopilot and a flight step for every NPC, each flying towards
//!    the ship its goal is about, the player's included (the one it
//!    assists, or escorts in formation), a fighter docking its carrier;
//! 6. and the removal of those that landed, jumped out or docked, each
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
pub mod npc;
pub mod spawn;
pub mod table;

use crate::ai::{Behaviour, Goal, PlayerSide, Reaction, Surroundings};
use std::collections::BTreeSet;

use crate::catalog::{GovtId, LandingSite, PersonId, ShipId};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::flight::ShipState;
use crate::govt::Governments;
use crate::person::PersonWorld;
use autopilot::Outcome;
use npc::{Mode, Npc, NpcId, NpcPerson};
use spawn::{JUMP_IN_TICKS, NewShip, PersonDraw};
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
}

impl<'a> World<'a> {
    /// `sites` alone: no player, no governments, no record, and no person
    /// gone or grudging ([`PersonWorld::NONE`]).
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
    /// Ticks since the traffic began.
    ticks: u64,
    /// How many ticks apart decisions are.
    interval: u32,
    /// The NPCs that left on the last tick, and how.
    departed: Vec<(NpcId, Outcome)>,
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
            ticks: 0,
            interval,
            departed: Vec::new(),
        }
    }

    /// Enters a system whose traffic is drawn from `table`: its NPCs are
    /// replaced with its initial population, rolled on `chance`, with no
    /// person gone or grudging (see [`Traffic::enter_in`]).
    pub fn enter(&mut self, table: SpawnTable, chance: &mut (impl Chance + ?Sized)) {
        self.enter_in(table, World::new(&[]), chance);
    }

    /// Enters a system whose traffic is drawn from `table` in `world`: its
    /// NPCs are replaced with its initial population, rolled on `chance`,
    /// its persons as the world's persons say.
    pub fn enter_in(
        &mut self,
        table: SpawnTable,
        world: World,
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.table = table;
        self.npcs.clear();
        self.departed.clear();
        let mut draw = PersonDraw::new(world.persons);
        let ships = spawn::initial(&self.table, &mut draw, chance);
        self.add(ships, world.persons.grudges);
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
        let arrivals = spawn::arrivals(&self.table, self.npcs.len(), &mut draw, chance);
        self.add(arrivals, world.persons.grudges);
        self.react(behaviour, world, strikes);
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
        for (npc, goal) in self.npcs.iter_mut().zip(goals) {
            let Some(goal) = goal else {
                continue;
            };
            if let Goal::Inspect(ship) = npc.goal
                && goal != npc.goal
            {
                npc.inspected = Some(ship);
            }
            if !goal.fights() {
                npc.provoked = 0.0;
            }
            npc.goal = goal;
        }
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
        let outcomes: Vec<Outcome> = self
            .npcs
            .iter_mut()
            .map(|npc| {
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
                autopilot::fly(npc, world.sites, other.as_ref())
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

    /// Each flying, intact NPC answers each of `strikes` in `world`, as
    /// `behaviour` says, all against the NPCs as they were before.
    fn react(&mut self, behaviour: &(impl Behaviour + ?Sized), world: World, strikes: &[Strike]) {
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

    /// Adds `npc` to the system, numbered as the next NPC, and gives its
    /// number.
    pub(crate) fn add_npc(&mut self, npc: Npc) -> NpcId {
        let id = NpcId(self.next_id);
        self.next_id += 1;
        self.npcs.push(Npc { id, ..npc });
        id
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
    /// person flies its fitted ship, holding a grudge when `grudges` say.
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
            let reserves = person.map_or_else(|| kind.stats.full(), |person| person.reserves);
            let condition = person.map_or(Condition::Intact, |person| person.condition);
            let traits = person
                .map(|person| NpcPerson::of(&person.record, grudges.contains(&person.record.id)));
            let id = NpcId(self.next_id);
            self.next_id += 1;
            self.npcs.push(Npc {
                id,
                ship: ship.ship,
                govt: ship.govt,
                ai_type: ship.ai_type,
                leader: ship.lead.map(|lead| NpcId(first + lead as u32)),
                class: kind.escort_class,
                escort: None,
                stats: kind.stats,
                reserves,
                state: ship.state,
                mode: if ship.jumping_in {
                    Mode::JumpingIn {
                        ticks_left: JUMP_IN_TICKS,
                    }
                } else {
                    Mode::Flying
                },
                goal: Goal::Idle,
                condition,
                hull: kind.hull,
                armament: kind.armament.clone(),
                rounds: kind.rounds.clone(),
                trigger: Trigger::default(),
                target: None,
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
    use crate::combat::ShipRef;
    use crate::combat::armament::Armament;
    use crate::combat::hull::HullSpec;
    use crate::combat::weapon::WeaponSpec;
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
    /// at rest.
    fn populated(count: u32, interval: u32) -> Traffic {
        let draws: Vec<u32> = (0..count).flat_map(|i| placed(750 + i, 750)).collect();
        let mut traffic = Traffic::with_interval(interval);
        traffic.enter(table(count), &mut Draws::of(&draws));
        traffic
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
                appear_on: String::new(),
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
                appear_on: String::new(),
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
            slot: 2,
            ships: 2,
            spacing: 30.0,
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
        }
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

    fn entered(world: World, draws: &[u32]) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.enter_in(peopled(1), world, &mut Draws::of(draws));
        traffic
    }

    #[test]
    fn a_person_entering_flies_as_its_record_and_its_fitted_ship_say() {
        let traffic = entered(World::new(&[]), &[0, 382, 750, 750, 0]);
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
        let traffic = entered(world, &[0, 382, 750, 750, 0]);
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
        assert_eq!(entered(world, &[0, 382]).npcs(), []);
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
}
