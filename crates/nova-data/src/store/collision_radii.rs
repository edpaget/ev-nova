//! Each new-game ship's collision radius, kept on the store.
//!
//! A new-game ship's `size` gives the radius of the circle it collides as
//! (`nova_galaxy_format::fleet::Size::collision_radius`). A `shän` has no
//! field for it, so the loader keeps it here, in whole pixels at
//! [`PX_PER_METRE`](crate::new_game::fleet::PX_PER_METRE)
//! ([`collision_radius`](crate::new_game::fleet::collision_radius), filled
//! by `crate::new_game::load`), and [`GameData::collision_radius`] answers
//! it. The catalog serves it on the ship's hull, and the fight strikes the
//! ship as that disc under the rulebook's `data_collision` reading
//! (`nova_sim::combat::data_collision`).
//!
//! The stock game's store holds none: every stock ship answers `None`.

use std::collections::BTreeMap;

use super::GameData;
use crate::wire::id::ShipId;

impl GameData {
    /// The store with `radii` as its ships' collision radii, in whole
    /// pixels, replacing any it had (see the module docs).
    #[must_use]
    pub fn with_collision_radii(mut self, radii: BTreeMap<ShipId, i16>) -> Self {
        self.collision_radii = radii;
        self
    }

    /// Ship type `id`'s collision radius in whole pixels, if the store has
    /// one: only the new game's do (see the module docs).
    #[must_use]
    pub fn collision_radius(&self, id: ShipId) -> Option<i16> {
        self.collision_radii.get(&id).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::store::fixture::store_of;

    #[test]
    fn a_stock_store_has_no_collision_radii() {
        let decoded = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&decoded, &empty, &loaded] {
            assert_eq!(data.collision_radius(ShipId(128)), None);
        }
    }

    #[test]
    fn a_store_with_collision_radii_answers_each_by_ship_and_new_ones_replace_them() {
        let data = GameData::default()
            .with_collision_radii(BTreeMap::from([(ShipId(128), 20), (ShipId(129), 8)]));
        assert_eq!(data.collision_radius(ShipId(128)), Some(20));
        assert_eq!(data.collision_radius(ShipId(129)), Some(8));
        assert_eq!(data.collision_radius(ShipId(130)), None, "unlisted");
        let data = data.with_collision_radii(BTreeMap::from([(ShipId(130), 5)]));
        assert_eq!(data.collision_radius(ShipId(128)), None, "replaced");
        assert_eq!(data.collision_radius(ShipId(130)), Some(5));
    }
}
