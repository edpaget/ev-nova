//! The berths whose last ship left the system with its slot's record
//! intact, as the original's ship table keeps an inactive slot's record in
//! the `EV Nova` executable, for the one reader that skips the active test.
//!
//! `_AIFightEnemyThreatToPlayer` (@0x82c80-0x82dba), the pick of rank
//! battle help and of a paid helper, counts and draws among slots 1-63
//! with no test of the active flag (+0xb8): only not the helper's slot,
//! not slot 0, not `_IsDisabled`, not `_IsDying`, +0x70 == 0, AI state
//! +0xc8c8 3 or 4 and +0x74 the helper's system. Every other way a slot
//! goes inactive fails one of those tests: a jump out and a system wipe
//! write +0x74 = -1 (@0x859bf, `_DeleteAllComputerShips` @0x3bf5f), a
//! destroyed ship is dying, a captured one disabled, a gate entry is in
//! the gate state, a docked fighter in the return state (5), and the
//! spawn undos leave state 0.
//!
//! One route passes: a person swapped for its mission ship on the hail
//! (`_HandlePlayerCommunication` @0x62512-0x62520) clears only its slot's
//! +0xb8, with no hostility test on the hail. A hostile person attacking
//! the player keeps its state 4, its target the player, its system and its
//! armour, and stays a candidate until a ship takes its slot again (every
//! spawn rewrites +0x70 and +0xc8c8) or the next system wipe.
//! [`Traffic::vacate`] keeps such a ship's record here, at its berth, and
//! the battle-help pick reads it
//! ([`RuleKey::BattleHelpGoneSlots`](crate::RuleKey::BattleHelpGoneSlots)).
//!
//! The docked fighter and the debug "remove target" key (`_HandlePlayer`
//! @0x6e855-0x6e871) also clear only +0xb8. A docked fighter never passes,
//! and the engine has no debug keys, so neither is built. An `M`/`N` move
//! (`_EvalSetExp` @0x15807, @0x1590b) leaves the old system's inactive
//! slots on that system, so they fail the new one's test; the records are
//! dropped at [`Traffic::retarget`], and nothing visible turns on a move
//! back before the next wipe.

use crate::traffic::Traffic;
use crate::traffic::npc::{Npc, NpcId};

/// The last ship of each berth whose slot lost only its active flag, one
/// record per berth (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VacatedBerths(Vec<Npc>);

impl VacatedBerths {
    /// Records `npc` as its berth's vacated ship, replacing any record at
    /// that berth.
    pub fn record(&mut self, npc: Npc) {
        self.taken(npc.berth);
        self.0.push(npc);
    }

    /// Drops the record at `berth`, as a ship taking the slot rewrites it.
    pub fn taken(&mut self, berth: u32) {
        self.0.retain(|npc| npc.berth != berth);
    }

    /// Forgets every record, as a system wipe rewrites every slot's system.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// The records, in the order they were made.
    #[must_use]
    pub fn ships(&self) -> &[Npc] {
        &self.0
    }
}

impl Traffic {
    /// Takes NPC `id` out of the system, if it is there, keeping its record
    /// as its berth's vacated ship: a person swapped for its mission ship,
    /// whose slot loses only its active flag (@0x62520, see [the module
    /// docs](self)).
    pub fn vacate(&mut self, id: NpcId) {
        if let Some(npc) = self.npcs().iter().find(|npc| npc.id == id) {
            self.vacated.record(npc.clone());
        }
        self.remove(id);
    }

    /// The last ships of the berths vacated and not taken again since the
    /// system was entered ([`Traffic::vacate`]).
    #[must_use]
    pub fn vacated(&self) -> &[Npc] {
        self.vacated.ships()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::ai::Goal;
    use crate::catalog::{ShipId, SystemId};
    use crate::combat::ShipRef;
    use crate::flight::ShipState;
    use crate::govt::Governments;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use crate::traffic::spawn::{Entry, NewShip};
    use crate::traffic::table::{ShipKind, SpawnTable};
    use crate::traffic::{Leave, SPAWN_RESERVE, Traffic, World};
    use nova_sim_base::fixture::Draws;

    /// An NPC attacking the player.
    fn attacker() -> Npc {
        Npc {
            goal: Goal::Attack(ShipRef::Player),
            target: Some(ShipRef::Player),
            ..npc(0, ShipStats::default())
        }
    }

    /// Traffic whose NPC at berth 1, attacking the player, was vacated,
    /// with another still in at berth 2: the traffic and the vacated NPC.
    fn vacated_at_berth_1() -> (Traffic, NpcId) {
        let mut traffic = Traffic::new();
        let gone = traffic.add_npc(attacker());
        traffic.add_npc(npc(0, ShipStats::default()));
        traffic.vacate(gone);
        (traffic, gone)
    }

    fn vacated_berths(traffic: &Traffic) -> Vec<(NpcId, u32)> {
        traffic
            .vacated()
            .iter()
            .map(|npc| (npc.id, npc.berth))
            .collect()
    }

    #[test]
    fn a_vacated_berth_keeps_its_last_ship_until_a_ship_takes_the_berth() {
        let (mut traffic, gone) = vacated_at_berth_1();
        assert!(traffic.npcs().iter().all(|npc| npc.id != gone), "not in");
        assert_eq!(vacated_berths(&traffic), [(gone, 1)]);
        assert_eq!(traffic.vacated()[0].goal, Goal::Attack(ShipRef::Player));
        assert_eq!(traffic.vacated()[0].target, Some(ShipRef::Player));
        let came = traffic.add_npc(npc(0, ShipStats::default()));
        let berth = traffic.npcs().iter().find(|npc| npc.id == came).unwrap();
        assert_eq!(berth.berth, 1, "the first free berth");
        assert_eq!(vacated_berths(&traffic), []);
    }

    #[test]
    fn a_ship_taking_another_berth_leaves_the_vacated_one() {
        let (mut traffic, gone) = vacated_at_berth_1();
        traffic.add_npc_at(npc(0, ShipStats::default()), 3);
        assert_eq!(vacated_berths(&traffic), [(gone, 1)]);
    }

    #[test]
    fn vacating_a_ship_not_in_the_system_records_nothing() {
        let (mut traffic, gone) = vacated_at_berth_1();
        traffic.vacate(NpcId(99));
        assert_eq!(vacated_berths(&traffic), [(gone, 1)]);
    }

    #[test]
    fn a_ship_brought_in_through_the_spawn_path_takes_the_berth_back() {
        let (mut traffic, _) = vacated_at_berth_1();
        *traffic.table_mut() = SpawnTable {
            ships: BTreeMap::from([(ShipId(200), ShipKind::default())]),
            ..SpawnTable::default()
        };
        let arrival = NewShip {
            ship: ShipId(200),
            govt: None,
            ai_type: AiType::WimpyTrader,
            lead: None,
            state: ShipState::default(),
            entry: Entry::Placed,
            aggression: 0,
            booty: None,
            info_types: 0,
            person: None,
            mission: None,
            aux: None,
            derelict: false,
            held: false,
            boarded: false,
            cloaked: false,
            quote: None,
            cargo: None,
        };
        let came = traffic.bring_in_within(
            vec![arrival],
            SPAWN_RESERVE,
            &Governments::empty(),
            &mut Draws::of(&[]),
        );
        assert_eq!(came, 1);
        assert!(traffic.npcs().iter().any(|npc| npc.berth == 1));
        assert_eq!(vacated_berths(&traffic), []);
    }

    #[test]
    fn vacating_a_berth_again_keeps_only_its_latest_ship() {
        let (mut traffic, _) = vacated_at_berth_1();
        let next = traffic.add_npc(attacker());
        traffic.vacate(next);
        assert_eq!(vacated_berths(&traffic), [(next, 1)]);
    }

    #[test]
    fn a_helper_sent_at_a_ship_gone_from_the_system_lets_it_go_at_its_next_think() {
        // `_HighLevelAIHandler`'s slot test clears +0x70 (@0x8d4e4-0x8d50d),
        // and state 4 with no target goes to state 0 (@0x8e263 →
        // @0x8e731). Rank battle help reaches any AI type.
        use crate::ai::NovaAi;
        use crate::ai::fixture::{PIRATES, TRADERS, player, ship};
        for (ai_type, vacated) in [
            AiType::WimpyTrader,
            AiType::BraveTrader,
            AiType::Warship,
            AiType::Interceptor,
        ]
        .into_iter()
        .flat_map(|ai_type| [(ai_type, true), (ai_type, false)])
        {
            let mut traffic = Traffic::new();
            let person = traffic.add_npc(Npc {
                goal: Goal::Attack(ShipRef::Player),
                target: Some(ShipRef::Player),
                ..ship(0, PIRATES, AiType::Warship, 300.0, 0.0)
            });
            let helper = traffic.add_npc(ship(1, TRADERS, ai_type, 0.0, 0.0));
            if vacated {
                traffic.vacate(person);
            }
            let ghost = ShipRef::Npc(person);
            let npc = traffic
                .npcs_mut()
                .iter_mut()
                .find(|npc| npc.id == helper)
                .unwrap();
            npc.goal = Goal::Attack(ghost);
            npc.target = Some(ghost);
            let world = World {
                player: Some(player(0.0, -500.0)),
                ..World::new(&[])
            };
            traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
            let after = traffic
                .npcs()
                .iter()
                .find(|npc| npc.id == helper)
                .expect("the helper stays");
            if vacated {
                assert_ne!(after.goal.attacking(), Some(ghost), "{ai_type:?}");
                assert_ne!(after.target, Some(ghost), "{ai_type:?}");
            } else if matches!(ai_type, AiType::Warship | AiType::Interceptor) {
                // The traders here leave either way; the fighters keep a
                // ship in the system.
                assert_eq!(
                    after.goal.attacking(),
                    Some(ghost),
                    "{ai_type:?}: a ship in the system is fought"
                );
            }
        }
    }

    #[test]
    fn entering_or_moving_to_a_system_forgets_the_vacated_berths() {
        let (mut traffic, _) = vacated_at_berth_1();
        traffic.enter_with(
            SpawnTable::default(),
            World::new(&[]),
            Vec::new(),
            &mut Draws::of(&[]),
        );
        assert_eq!(vacated_berths(&traffic), [], "entered");
        for leave in [
            Leave::Dropped,
            Leave::Held {
                from: SystemId(128),
                to: SystemId(129),
            },
        ] {
            let (mut traffic, _) = vacated_at_berth_1();
            traffic.retarget(SpawnTable::default(), &BTreeSet::new(), leave);
            assert_eq!(vacated_berths(&traffic), [], "{leave:?}");
        }
    }
}
