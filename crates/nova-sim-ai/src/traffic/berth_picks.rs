//! The pick (+0x72) each ship-table slot keeps after its ship is gone.
//!
//! No spawn writes the original's +0x72 (`_RandomShipSpawn`,
//! `_GenericRandomShipSpawn`, `_MissionDudeSpawn`, `_SpecificDudeSpawn`,
//! `_RespawnEscort`, `_LaunchFighter`, `_SpawnEscort`, `_HyperSpawnFleet`,
//! `_SpawnPerson`, `_AISetup`), and nothing that frees a slot does: only
//! `_HandleShip` clears it once a pick without `Flags` 0x0002 fires
//! (@0x35012), `_DoShipCapture` clears it (@0x41207), and `_InitObjects`
//! clears it as the game starts (@0x1c385). So a new ship brought into a
//! slot (a [`Npc::berth`](crate::traffic::npc::Npc::berth)) holds the
//! pick the slot's last ship left, and its first gun pass that finds
//! nothing fires it, if it carries that weapon (`_FireAIShipWeapon`'s
//! preamble returns for a count of 0, @0x887c3).
//!
//! [`BerthPicks`] keeps that pick for each berth, written as each ship's
//! pick changes, and gives a new ship its berth's by the reading of
//! [`RuleKey::ReusedSlotPick`](crate::RuleKey::ReusedSlotPick): the
//! last ship's by the engine, none by the other reading.
//!
//! It lives as long as the [`Traffic`](crate::traffic::Traffic) holding
//! it, as its berths' persons do: across systems, but not across a new
//! traffic. The original keeps a slot's pick through a pilot load or a new
//! game until the app quits; that is recorded only (rdm phase
//! weapons-fidelity/phase-17-fire-gates-left-out-of-phase-5).

use std::collections::BTreeMap;

use crate::catalog::WeaponId;
use crate::rulebook::RuleSource;

/// The pick each berth's last ship left, and the reading a new ship takes
/// it by (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BerthPicks {
    /// The pick of the ship last in each berth, none for a berth whose
    /// ship held none.
    picks: BTreeMap<u32, WeaponId>,
    /// Whether a new ship takes its berth's pick.
    reading: RuleSource,
}

impl BerthPicks {
    /// Writes `pick` as the pick of the ship in `berth`.
    pub fn write(&mut self, berth: u32, pick: Option<WeaponId>) {
        match pick {
            Some(pick) => self.picks.insert(berth, pick),
            None => self.picks.remove(&berth),
        };
    }

    /// The pick the ship last in `berth` left, whatever the reading.
    #[must_use]
    pub fn left(&self, berth: u32) -> Option<WeaponId> {
        self.picks.get(&berth).copied()
    }

    /// The pick a new ship brought into `berth` holds: the one its last
    /// ship left by the engine, none by the other reading.
    #[must_use]
    pub fn seed(&self, berth: u32) -> Option<WeaponId> {
        match self.reading {
            RuleSource::Engine => self.left(berth),
            RuleSource::Bible => None,
        }
    }

    /// Takes new ships' picks by `reading`.
    pub fn set_reading(&mut self, reading: RuleSource) {
        self.reading = reading;
    }

    /// The reading new ships' picks are taken by.
    #[must_use]
    pub fn reading(&self) -> RuleSource {
        self.reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_berths_pick_reads_back_until_overwritten_with_none() {
        let mut picks = BerthPicks::default();
        assert_eq!(picks.left(1), None);
        picks.write(1, Some(WeaponId(131)));
        assert_eq!(picks.left(1), Some(WeaponId(131)));
        assert_eq!(picks.left(2), None);
        picks.write(1, Some(WeaponId(128)));
        assert_eq!(picks.left(1), Some(WeaponId(128)));
        picks.write(1, None);
        assert_eq!(picks.left(1), None);
    }

    #[test]
    fn a_new_ship_takes_its_berths_pick_by_the_engine_and_none_by_the_other_reading() {
        let mut picks = BerthPicks::default();
        assert_eq!(picks.reading(), RuleSource::Engine);
        picks.write(1, Some(WeaponId(131)));
        assert_eq!(picks.seed(1), Some(WeaponId(131)));
        assert_eq!(picks.seed(2), None);
        picks.set_reading(RuleSource::Bible);
        assert_eq!(picks.reading(), RuleSource::Bible);
        assert_eq!(picks.seed(1), None);
        assert_eq!(picks.left(1), Some(WeaponId(131)), "still kept");
    }
}
