//! The ship kinds the session builds for the NPCs it places itself (a
//! fighter launched, an escort placed or restocked, the eject's hulk) and
//! the system's traffic table, their default items counted as the
//! session's [`RuleKey::NpcDefaultItems`] reading says (see
//! `nova_sim::stats::ShipStats::for_npc`), and their frames as the sprite
//! load loads them (see the `sprite_load` module).

use super::{Setup, Statics};
use crate::catalog::{DudeId, ShipRecord, TrafficCatalog};
use crate::rulebook::{RuleKey, RuleSource};
use crate::traffic::table::{self, ShipKind, SpawnTable};

impl Setup {
    /// The [`RuleKey::NpcDefaultItems`] reading the session follows.
    pub(super) fn npc_items(&self) -> RuleSource {
        self.rules.source_for(RuleKey::NpcDefaultItems)
    }

    /// A ship of class `record` as an NPC flies it, its default items
    /// counted as [`Self::npc_items`] says ([`table::kind`]), its frames
    /// as the sprite load loads them (see the `sprite_load` module).
    pub(super) fn ship_kind(&self, statics: &Statics, record: &ShipRecord) -> ShipKind {
        let mut kind = table::kind(record, &statics.outfits, &statics.arsenal, self.npc_items());
        kind.hull.frames = self.frames_of(statics, record.id);
        kind
    }

    /// Adds `dudes` to `table` as [`SpawnTable::add_mission_dudes`] does,
    /// their kinds counted as [`Self::npc_items`] says, then gives the
    /// table's kinds their frames as the sprite load loads them.
    pub(super) fn add_dudes(
        &self,
        statics: &Statics,
        table: &mut SpawnTable,
        catalog: &(impl TrafficCatalog + ?Sized),
        dudes: &[DudeId],
    ) {
        table.add_mission_dudes(
            catalog,
            dudes,
            &statics.ships,
            &statics.outfits,
            &statics.arsenal,
            self.npc_items(),
        );
        self.load_frames(statics, table);
    }
}
