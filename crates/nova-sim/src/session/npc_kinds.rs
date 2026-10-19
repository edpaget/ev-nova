//! The ship kinds the session builds for the NPCs it places itself (a
//! fighter launched, an escort placed or restocked, the eject's hulk) and
//! the system's traffic table, their default items counted as the
//! session's [`RuleKey::NpcDefaultItems`] reading says (see
//! `nova_sim::stats::ShipStats::for_npc`).

use super::Session;
use crate::catalog::ShipRecord;
use crate::rulebook::{RuleKey, RuleSource};
use crate::traffic::table::{self, ShipKind};

impl Session {
    /// The [`RuleKey::NpcDefaultItems`] reading the session follows.
    pub(super) fn npc_items(&self) -> RuleSource {
        self.rules.source_for(RuleKey::NpcDefaultItems)
    }

    /// A ship of class `record` as an NPC flies it, its default items
    /// counted as [`Session::npc_items`] says ([`table::kind`]).
    pub(super) fn ship_kind(&self, record: &ShipRecord) -> ShipKind {
        table::kind(
            record,
            &self.statics.outfits,
            &self.statics.arsenal,
            self.npc_items(),
        )
    }
}
