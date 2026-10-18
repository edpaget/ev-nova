//! The system's reinforcement fleet as the traffic keeps and brings it:
//! the fleet its table names, with the `flët`'s government
//! ([`Traffic::reinforcement_fleet`]), and its arrival
//! ([`Traffic::bring_reinforcements`]), `_HyperSpawnFleet(sys, fleet, 4,
//! 1)` (`_HandleReinforcements` @0x390c2-0x390e0) through the spawn path
//! the escort-strength fleet takes. The call and the clock are
//! [`reinforcements`](crate::ai::reinforcements)'s.

use super::{Traffic, World, spawn};
use crate::ai::reinforcements::Fleet;
use crate::chance::Chance;

impl Traffic {
    /// The system's reinforcement fleet, as the call and the clock read
    /// it: its `ReinfTime` and `ReinfIntrval`, and its `flët`'s
    /// government. None when the system names none, or the `flët` named
    /// has no record (the original would read a zeroed slot; no stock
    /// system names a missing one).
    #[must_use]
    pub fn reinforcement_fleet(&self) -> Option<Fleet> {
        let kept = self.table.reinforcements?;
        let record = self.table.fleets.get(&kept.fleet)?;
        Some(Fleet {
            govt: record.govt,
            time: kept.time,
            interval: kept.interval,
        })
    }

    /// The system's reinforcement fleet comes, rolled on `chance`, its
    /// lead an interceptor and its arrival announced (`_HyperSpawnFleet`'s
    /// arguments 4 and 1, @0x390c2-0x390e0), each ship taking a slot below
    /// `world`'s reserve, its `AppearOn` asked of `world`'s persons: a lead
    /// that finds no slot brings no fleet and no quote. Nothing comes in a
    /// system with none.
    pub fn bring_reinforcements(&mut self, world: &World, chance: &mut (impl Chance + ?Sized)) {
        let Some(kept) = self.table.reinforcements else {
            return;
        };
        self.bring_fleet(
            kept.fleet,
            spawn::FleetCall::ANNOUNCED_INTERCEPTOR,
            world,
            world.persons.gate(),
            chance,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{
        EscortRecord, FleetId, FleetRecord, GovtId, ShipId, SystemReinforcements,
    };
    use crate::control::Test;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, Mode};
    use crate::traffic::table::{ShipKind, SpawnTable};
    use nova_sim_base::fixture::{Draws, FAST};

    /// A system whose reinforcement fleet, 129 of government 128, is a
    /// warship-typed ship 200 leading one more, its arrival quote `STR#`
    /// 7600; `ReinfTime` 900, `ReinfIntrval` 2.
    fn reinforced() -> SpawnTable {
        SpawnTable {
            fleets: BTreeMap::from([(
                FleetId(129),
                FleetRecord {
                    id: FleetId(129),
                    lead: Some(ShipId(200)),
                    escorts: vec![EscortRecord {
                        ship: ShipId(200),
                        min: 1,
                        max: 1,
                    }],
                    govt: Some(GovtId(128)),
                    link_syst: 131,
                    appear_on: Test::default(),
                    quote: Some(7600),
                    flags: 0,
                },
            )]),
            ships: BTreeMap::from([(
                ShipId(200),
                ShipKind {
                    stats: ShipStats::new(FAST, &[]),
                    inherent_ai: 3,
                    ..ShipKind::default()
                },
            )]),
            reinforcements: Some(SystemReinforcements {
                fleet: FleetId(129),
                time: 900,
                interval: 2,
            }),
            ..SpawnTable::default()
        }
    }

    /// Traffic in the [`reinforced`] system with `ships` already in berths 1
    /// and up.
    fn crowded(ships: u32) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.enter(reinforced(), &mut Draws::of(&[]));
        for berth in 1..=ships {
            traffic.add_npc_at(npc(1000 + berth, ShipStats::new(FAST, &[])), berth);
        }
        traffic
    }

    #[test]
    fn the_traffic_keeps_its_systems_reinforcement_fleet_with_the_fleets_government() {
        assert_eq!(
            crowded(0).reinforcement_fleet(),
            Some(Fleet {
                govt: Some(GovtId(128)),
                time: 900,
                interval: 2,
            })
        );
        let mut bare = Traffic::new();
        bare.enter(SpawnTable::default(), &mut Draws::of(&[]));
        assert_eq!(bare.reinforcement_fleet(), None, "none in the system");
        let mut unrecorded = Traffic::new();
        unrecorded.enter(
            SpawnTable {
                fleets: BTreeMap::new(),
                ..reinforced()
            },
            &mut Draws::of(&[]),
        );
        assert_eq!(unrecorded.reinforcement_fleet(), None, "no record of it");
    }

    #[test]
    fn the_reinforcement_fleet_comes_by_hyperspace_its_lead_an_interceptor_with_its_quote() {
        let mut traffic = crowded(0);
        traffic.bring_reinforcements(&World::new(&[]), &mut Draws::of(&[0]));
        let ships: Vec<_> = traffic
            .npcs()
            .iter()
            .map(|npc| (npc.ship, npc.govt, npc.ai_type))
            .collect();
        assert_eq!(
            ships,
            [
                (ShipId(200), Some(GovtId(128)), AiType::Interceptor),
                (ShipId(200), Some(GovtId(128)), AiType::Warship),
            ],
            "the lead an interceptor (@0x390ca), its escort its own"
        );
        assert!(matches!(traffic.npcs()[0].mode, Mode::JumpingIn { .. }));
        assert_eq!(traffic.take_quotes(), [7600], "announced (@0x390c2)");
    }

    #[test]
    fn a_full_ship_table_brings_no_reinforcements_and_no_quote() {
        let mut traffic = crowded(55);
        traffic.bring_reinforcements(&World::new(&[]), &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs().len(), 55, "no fleet");
        assert!(traffic.take_quotes().is_empty(), "no line");
        let mut traffic = crowded(54);
        traffic.bring_reinforcements(&World::new(&[]), &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs().len(), 55, "the lead alone");
        assert_eq!(traffic.take_quotes(), [7600]);
    }

    #[test]
    fn a_system_without_reinforcements_brings_none() {
        let mut traffic = Traffic::new();
        traffic.enter(SpawnTable::default(), &mut Draws::of(&[]));
        traffic.bring_reinforcements(&World::new(&[]), &mut Draws::of(&[0]));
        assert!(traffic.npcs().is_empty());
    }
}
