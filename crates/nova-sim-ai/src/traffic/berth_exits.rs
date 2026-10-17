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

use super::berth_kept::BerthKept;

/// The exit cursors each berth's last ship left, and the reading a new
/// ship takes them by (see the module docs).
pub type BerthExits = BerthKept<ExitCursors>;

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
}
