//! NPC traffic: the ships that come and go in the player's system.
//!
//! - [`table`]: the [`SpawnTable`] a system's traffic is drawn from, its
//!   `DudeTypes` and `LinkSyst`s decoded and its ship types' stats
//!   resolved when the system is entered.
//! - [`spawn`]: the rolls: the initial population, the arrivals over time
//!   and the fleets, and where each ship starts.
//! - [`npc`]: an [`Npc`]: its ship, government, AI type, stats, fuel,
//!   flight state and goal.
//! - [`autopilot`]: flying an NPC's goal each tick with the player's
//!   flight physics.
//!
//! [`Traffic`] holds a system's NPCs. Entering a system
//! ([`Traffic::enter`]) replaces them with its initial population. Each
//! tick ([`Traffic::tick`]), in order: the arrival roll; the decisions due
//! on the AI timer, never for a ship still jumping in; the autopilot and
//! a flight step for every NPC; and the removal of those that landed or
//! jumped out.
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

use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::{LandingSite, ShipId};
use crate::chance::Chance;
use crate::flight::ShipState;
use autopilot::Outcome;
use npc::{Mode, Npc, NpcId};
use spawn::{JUMP_IN_TICKS, NewShip};
use table::SpawnTable;

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
    /// replaced with its initial population, rolled on `chance`.
    pub fn enter(&mut self, table: SpawnTable, chance: &mut (impl Chance + ?Sized)) {
        self.table = table;
        self.npcs.clear();
        self.departed.clear();
        let ships = spawn::initial(&self.table, chance);
        self.add(ships);
    }

    /// Advances the traffic one tick among `sites`, NPCs deciding as
    /// `behaviour` says, rolling on `chance` (see the module docs).
    pub fn tick(
        &mut self,
        behaviour: &(impl Behaviour + ?Sized),
        sites: &[LandingSite],
        chance: &mut (impl Chance + ?Sized),
    ) {
        let arrivals = spawn::arrivals(&self.table, self.npcs.len(), chance);
        self.add(arrivals);
        let around = Surroundings {
            sites,
            npcs: &self.npcs,
        };
        let mut goals = Vec::with_capacity(self.npcs.len());
        for npc in &self.npcs {
            let due = npc.mode == Mode::Flying && decision_due(self.ticks, npc.id, self.interval);
            goals.push(due.then(|| behaviour.decide(npc, &around, &mut &mut *chance)));
        }
        for (npc, goal) in self.npcs.iter_mut().zip(goals) {
            if let Some(goal) = goal {
                npc.goal = goal;
            }
        }
        let states: Vec<(NpcId, ShipState)> =
            self.npcs.iter().map(|npc| (npc.id, npc.state)).collect();
        let outcomes: Vec<Outcome> = self
            .npcs
            .iter_mut()
            .map(|npc| {
                let lead = match npc.goal {
                    Goal::Follow(lead) => states
                        .iter()
                        .find(|(id, _)| *id == lead)
                        .map(|(_, state)| state),
                    _ => None,
                };
                autopilot::fly(npc, sites, lead)
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

    /// The NPCs, in the order they appeared.
    #[must_use]
    pub fn npcs(&self) -> &[Npc] {
        &self.npcs
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

    /// Adds `ships`, numbering each and pointing each escort at its lead.
    fn add(&mut self, ships: Vec<NewShip>) {
        let first = self.next_id;
        for ship in ships {
            let Some(kind) = self.table.ships.get(&ship.ship) else {
                continue;
            };
            let id = NpcId(self.next_id);
            self.next_id += 1;
            self.npcs.push(Npc {
                id,
                ship: ship.ship,
                govt: ship.govt,
                ai_type: ship.ai_type,
                leader: ship.lead.map(|lead| NpcId(first + lead as u32)),
                stats: kind.stats,
                fuel: kind.stats.fuel,
                state: ship.state,
                mode: if ship.jumping_in {
                    Mode::JumpingIn {
                        ticks_left: JUMP_IN_TICKS,
                    }
                } else {
                    Mode::Flying
                },
                goal: Goal::Idle,
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
    use crate::catalog::{DudeId, EscortRecord, FleetId, FleetRecord, GovtId, StellarId};
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, planet};
    use crate::traffic::npc::AiType;
    use crate::traffic::table::{ShipKind, SpawnDude};

    /// Decides `goal` for everyone, recording who decided.
    #[derive(Debug)]
    struct Recording {
        goal: Goal,
        decided: RefCell<Vec<NpcId>>,
    }

    impl Recording {
        fn deciding(goal: Goal) -> Self {
            Self {
                goal,
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
                },
            )]),
            ships: BTreeMap::from([(
                ShipId(200),
                ShipKind {
                    stats: ShipStats::new(FAST, &[]),
                    inherent_ai: 3,
                },
            )]),
            ..SpawnTable::default()
        }
    }

    /// One setup pass placing ship 200 in the system at
    /// (`x` - 750, `y` - 750), facing up.
    fn placed(x: u32, y: u32) -> [u32; 7] {
        [6, 6, 0, 0, x, y, 0]
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
        let first = npcs[0];
        assert_eq!(first.ship, ShipId(200));
        assert_eq!(first.govt, Some(GovtId(130)));
        assert_eq!(first.ai_type, AiType::WimpyTrader);
        assert_eq!(first.leader, None);
        assert_eq!(first.stats, ShipStats::new(FAST, &[]));
        assert_eq!(first.fuel, first.stats.fuel);
        assert_eq!(first.state.position, Vec2::new(0.0, 0.0));
        assert_eq!(first.mode, Mode::Flying);
        assert_eq!(first.goal, Goal::Idle);
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
        let npc = traffic.npcs()[0];
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
        let escort = traffic.npcs()[1];
        assert!(escort.state.position.x < 450.0, "{:?}", escort.state);
    }
}
