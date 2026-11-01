//! Rewriting the display of every ship the traffic holds or can spawn,
//! when what of each ship's sprites loads changes
//! ([`SpriteLoad`](crate::combat::SpriteLoad)): each NPC's and each
//! spawnable kind's [`HullSpec::frames`](crate::combat::hull::HullSpec::frames),
//! and nothing else of its hull. Each NPC's fold and display state are
//! then brought inside its new sets
//! ([`ShipFrames::refit`](crate::combat::ship_frames::ShipFrames::refit)),
//! so a fold or alt set left from more sets neither holds its fire nor
//! draws a set its sprite lacks.
//!
//! The original loads the sprites once, at launch, so it has nothing to
//! rewrite. The engine's caller decides when a change applies; this only
//! carries it to the ships.

use crate::catalog::ShipId;
use crate::combat::ship_frames::FrameSpec;
use crate::traffic::Traffic;
use crate::traffic::table::SpawnTable;

impl SpawnTable {
    /// Gives each ship kind it holds, and each person's, the display
    /// `frames` gives its ship type, leaving the rest of the kind as it
    /// is.
    pub fn refit_frames(&mut self, frames: &dyn Fn(ShipId) -> FrameSpec) {
        for (&ship, kind) in &mut self.ships {
            kind.hull.frames = frames(ship);
        }
        for person in self.persons.values_mut() {
            if let Some(ship) = person.record.ship {
                person.kind.hull.frames = frames(ship);
            }
        }
    }
}

impl Traffic {
    /// Gives each NPC, and each kind its table can spawn, the display
    /// `frames` gives its ship type, leaving the rest of its hull as it
    /// is, and brings each NPC's fold and display state inside its new
    /// sets.
    pub fn refit_frames(&mut self, frames: &dyn Fn(ShipId) -> FrameSpec) {
        for npc in &mut self.npcs {
            npc.hull.frames = frames(npc.ship);
            npc.display
                .frames
                .refit(&npc.hull.frames, npc.armament.fold_mut());
        }
        self.table.refit_frames(frames);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_sim_catalog::fixture::person;

    use super::*;
    use crate::catalog::PersonId;
    use crate::combat::armament::Fold;
    use crate::combat::hull::Condition;
    use crate::combat::ship_frames::ANIMATED;
    use crate::reserves::Reserves;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::table::{ShipKind, SpawnPerson};
    use nova_sim_base::fixture::FAST;

    /// The display the refit gives ship type `ship`: its ID's sets.
    fn refit(ship: ShipId) -> FrameSpec {
        FrameSpec {
            flags: ANIMATED,
            base_sets: u16::try_from(ship.0).expect("positive"),
            ..FrameSpec::NONE
        }
    }

    /// A kind of six animated sets, its person's hull fields set.
    fn kind() -> ShipKind {
        let mut kind = ShipKind::default();
        kind.hull.frames = FrameSpec {
            flags: ANIMATED,
            base_sets: 6,
            ..FrameSpec::NONE
        };
        kind.hull.inherent_ai = 4;
        kind.hull.crew = 12;
        kind.hull.class_turn = 2.5;
        kind
    }

    /// `kind` with `refit`'s display for `ship`.
    fn refitted(kind: &ShipKind, ship: ShipId) -> ShipKind {
        let mut kind = kind.clone();
        kind.hull.frames = refit(ship);
        kind
    }

    fn table() -> SpawnTable {
        SpawnTable {
            ships: BTreeMap::from([(ShipId(2), kind()), (ShipId(3), kind())]),
            persons: BTreeMap::from([(
                PersonId(150),
                SpawnPerson {
                    record: person(150, 4),
                    linked: true,
                    kind: kind(),
                    reserves: Reserves::default(),
                    condition: Condition::Intact,
                    derelict: false,
                },
            )]),
            ..SpawnTable::default()
        }
    }

    #[test]
    fn a_table_refits_each_kind_and_each_persons_by_its_ship_type() {
        let mut table = table();
        table.refit_frames(&refit);
        assert_eq!(table.ships[&ShipId(2)], refitted(&kind(), ShipId(2)));
        assert_eq!(table.ships[&ShipId(3)], refitted(&kind(), ShipId(3)));
        assert_eq!(
            table.persons[&PersonId(150)].kind,
            refitted(&kind(), ShipId(4)),
            "by the person's ship, its hull fields kept"
        );
    }

    #[test]
    fn the_traffic_refits_each_npc_and_its_table() {
        let mut traffic = Traffic::with_interval(1);
        traffic.table = table();
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.ship = ShipId(5);
        ship.hull = kind().hull;
        let mut other = ship.clone();
        other.ship = ShipId(6);
        traffic.npcs = vec![ship.clone(), other.clone()];
        traffic.refit_frames(&refit);
        ship.hull.frames = refit(ShipId(5));
        other.hull.frames = refit(ShipId(6));
        assert_eq!(traffic.npcs, [ship, other], "the rest of each hull kept");
        let mut expected = table();
        expected.refit_frames(&refit);
        assert_eq!(traffic.table, expected);
    }

    #[test]
    fn the_traffic_brings_each_npcs_display_state_inside_its_new_sets() {
        let mut traffic = Traffic::with_interval(1);
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.ship = ShipId(2);
        ship.hull = kind().hull;
        *ship.armament.fold_mut() = Fold {
            set: 5,
            dir: 1,
            since_fire: 0,
        };
        ship.display.frames.alt_set = 4;
        traffic.npcs = vec![ship.clone()];
        traffic.refit_frames(&refit);
        ship.hull.frames = refit(ShipId(2));
        ship.display
            .frames
            .refit(&ship.hull.frames, ship.armament.fold_mut());
        assert_ne!(ship.armament.fold().set, 5, "a state the refit moves");
        assert_eq!(traffic.npcs, [ship]);
    }
}
