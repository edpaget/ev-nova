//! The exit cursors (+0xc8fe-+0xc905) each ship-table slot keeps after its
//! ship is gone.
//!
//! Every access to the cursors is in `_ModifyShotStartPosition`
//! (@0x742c-0x74bc) or `_SpawnBeam` (@0x450ff-0x4523d): no spawn
//! (`_RandomShipSpawn`, `_GenericRandomShipSpawn`, `_MissionDudeSpawn`,
//! `_LaunchFighter`, `_SpawnEscort`, `_SpawnPerson`, …), ship change,
//! capture, eject, refit, pilot load or `_InitObjects` writes them. The
//! table is a `NewPtr(0x325000)` (`_InitArrays` @0x1b7a1-0x1b7a8) that only
//! the system's zero fill sets. So a new ship brought into a slot (a
//! [`Npc::berth`](crate::traffic::npc::Npc::berth)) fires its first shot
//! of each `ExitType` from the exit after the one the slot's last ship
//! used.
//!
//! [`BerthExits`], a [`BerthKept`], keeps those cursors for each berth,
//! written after each fight ([`Traffic::keep_berth_exits`]), and gives a
//! new ship its berth's by the reading of
//! [`RuleKey::ReusedSlotExits`](crate::RuleKey::ReusedSlotExits): the last
//! ship's by the engine, all at exit 0 by the other reading. The player's
//! own cursors are the session's to carry.
//!
//! It lives as long as the [`Traffic`](crate::traffic::Traffic) holding
//! it, across systems but not across a new traffic. The original keeps a
//! slot's cursors through a pilot load or a new game until the app quits;
//! that is recorded only (rdm phase
//! weapons-fidelity/phase-18-exit-cursors-left-out-of-phase-6).
//!
//! [`Traffic::keep_berth_exits`]: crate::traffic::Traffic::keep_berth_exits

use crate::combat::exits::ExitCursors;
use crate::rulebook::RuleSource;

use super::Traffic;
use super::berth_kept::BerthKept;

/// The exit cursors each berth's last ship left, and the reading a new
/// ship takes them by (see the module docs).
pub type BerthExits = BerthKept<ExitCursors>;

impl Traffic {
    /// Writes each NPC's exit cursors as its berth's, after a fight has
    /// moved them on: the destroyed too, as they are still in the system
    /// until taken out.
    pub fn keep_berth_exits(&mut self) {
        for npc in &self.npcs {
            let cursors = *npc.armament.exit_cursors();
            self.berth_exits.write(npc.berth, Some(cursors));
        }
    }

    /// Gives each new ship its berth's last exit cursors by `reading`
    /// ([`RuleKey::ReusedSlotExits`](crate::RuleKey::ReusedSlotExits)):
    /// by the engine, as no spawn writes +0xc8fe-+0xc905; all at exit 0
    /// by the other reading.
    pub fn set_reused_slot_exits(&mut self, reading: RuleSource) {
        self.berth_exits.set_reading(reading);
    }

    /// The reading new ships take their berth's last exit cursors by (see
    /// [`Traffic::set_reused_slot_exits`]).
    #[must_use]
    pub fn reused_slot_exits(&self) -> RuleSource {
        self.berth_exits.reading()
    }

    /// The exit cursors a ship brought into `berth` starts with (see the
    /// module docs).
    pub(super) fn berth_cursors(&self, berth: u32) -> ExitCursors {
        self.berth_exits.seed(berth).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::exits::ExitKind;
    use crate::rulebook::RuleSource;

    #[test]
    fn a_new_ship_takes_its_berths_cursors_by_the_engine_and_none_by_the_other_reading() {
        let mut cursors = ExitCursors::default();
        cursors.step(ExitKind::Gun);
        cursors.step(ExitKind::Gun);
        let mut exits = BerthExits::default();
        exits.write(1, Some(cursors));
        assert_eq!(exits.seed(1).map(|c| c.next(ExitKind::Gun)), Some(2));
        assert_eq!(exits.seed(2), None);
        exits.set_reading(RuleSource::Bible);
        assert_eq!(exits.seed(1), None);
        assert_eq!(exits.left(1), Some(cursors), "still kept");
    }

    use std::collections::BTreeMap;

    use crate::catalog::ShipId;
    use crate::combat::hull::Condition;
    use crate::flight::ShipState;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::AiType;
    use crate::traffic::spawn::{Entry, NewShip};
    use crate::traffic::table::{ShipKind, SpawnTable};
    use crate::traffic::{SPAWN_RESERVE, Traffic};

    /// The gun cursor of each NPC, by its berth, in order.
    fn guns(traffic: &Traffic) -> Vec<(u32, usize)> {
        traffic
            .npcs()
            .iter()
            .map(|npc| (npc.berth, npc.armament.exit_cursors().next(ExitKind::Gun)))
            .collect()
    }

    /// Traffic whose first NPC, at berth 1, fired two gun shots and then
    /// went out (destroyed when `destroyed`), with a second at berth 2
    /// still in, reading reused berths' cursors by `reading`.
    fn left_firing(reading: RuleSource, destroyed: bool) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.set_reused_slot_exits(reading);
        let first = traffic.add_npc(npc(0, ShipStats::default()));
        traffic.add_npc(npc(0, ShipStats::default()));
        let ship = &mut traffic.npcs_mut()[0];
        ship.armament.exit_cursors_mut().step(ExitKind::Gun);
        ship.armament.exit_cursors_mut().step(ExitKind::Gun);
        if destroyed {
            ship.condition = Condition::Destroyed;
        }
        traffic.keep_berth_exits();
        traffic.remove(first);
        traffic
    }

    #[test]
    fn a_ship_placed_in_a_reused_berth_fires_on_from_the_last_ships_cursors_by_the_engine() {
        for destroyed in [false, true] {
            let mut traffic = left_firing(RuleSource::Engine, destroyed);
            assert_eq!(traffic.reused_slot_exits(), RuleSource::Engine);
            traffic.add_npc(npc(0, ShipStats::default()));
            traffic.add_npc(npc(0, ShipStats::default()));
            assert_eq!(
                guns(&traffic),
                [(2, 0), (1, 2), (3, 0)],
                "destroyed {destroyed}: berth 1's left, berth 3 fresh"
            );
        }
    }

    #[test]
    fn by_the_other_reading_a_ship_placed_in_a_reused_berth_starts_at_exit_0() {
        let mut traffic = left_firing(RuleSource::Bible, false);
        assert_eq!(traffic.reused_slot_exits(), RuleSource::Bible);
        traffic.add_npc(npc(0, ShipStats::default()));
        assert_eq!(guns(&traffic), [(2, 0), (1, 0)]);
    }

    #[test]
    fn a_placed_ships_own_cursors_give_way_to_its_berths() {
        let mut traffic = Traffic::new();
        let mut fired = npc(0, ShipStats::default());
        fired.armament.exit_cursors_mut().step(ExitKind::Gun);
        traffic.add_npc(fired);
        assert_eq!(guns(&traffic), [(1, 0)], "no spawn writes them");
    }

    /// A placed ship of type 200.
    fn arrival() -> NewShip {
        NewShip {
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
        }
    }

    #[test]
    fn a_ship_spawned_into_a_reused_berth_fires_on_from_the_last_ships_cursors() {
        for (reading, gun) in [(RuleSource::Engine, 2), (RuleSource::Bible, 0)] {
            let mut traffic = left_firing(reading, false);
            traffic.table = SpawnTable {
                ships: BTreeMap::from([(ShipId(200), ShipKind::default())]),
                ..SpawnTable::default()
            };
            traffic.bring_in_within(
                vec![arrival()],
                SPAWN_RESERVE,
                &crate::govt::Governments::empty(),
                &mut crate::chance::NeverFires,
            );
            assert_eq!(guns(&traffic), [(2, 0), (1, gun)], "{reading:?}");
        }
    }
}
