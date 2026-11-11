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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::catalog::{OutfitId, ShipId};
    use crate::geometry::Vec2;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::stats::MORE_ARMOR;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, hull, outfit, ship};
    use crate::traffic::npc::AiType;

    /// A ship class of `FAST`'s armour (45) whose default items hold one
    /// armour plate of +150, its hull beside it.
    const PLATED: ShipId = ShipId(150);
    /// The plate.
    const PLATE: OutfitId = OutfitId(330);

    fn plated() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.outfits.push(outfit(PLATE.0, &[(MORE_ARMOR, 150)]));
        let mut record = ship(PLATED.0, FAST);
        record.defaults = vec![(PLATE, 1)];
        catalog.ship_records.push(record);
        catalog.hulls.push(hull(PLATED.0));
        catalog
    }

    #[test]
    fn a_ship_brought_in_flies_its_classs_armour_by_the_engine_and_its_plates_by_the_other() {
        let catalog = plated();
        for (items, armor) in [(RuleSource::Engine, 45.0), (RuleSource::Bible, 195.0)] {
            let pilot = Pilot::new(&catalog, "Ada").expect("starts");
            let mut session = Session::fly(&catalog, pilot)
                .expect("flies")
                .with_rules(Rulebook::default().with_override(RuleKey::NpcDefaultItems, items));
            let id = session
                .bring_in(PLATED, None, AiType::Warship, Vec2::new(300.0, 0.0))
                .expect("brought in");
            let npc = session.space.npc(id).expect("in the system");
            assert_eq!(npc.stats.armor, armor, "{items:?}");
            assert_eq!(npc.reserves.armor.max, armor, "{items:?}");
            assert_eq!(npc.reserves.armor.now, armor, "{items:?}: full");
        }
    }
}
