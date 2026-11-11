//! Each new-game ship's gate fuel, kept on the store.
//!
//! A new-game ship's `stats.gate_fuel` gives the fuel a passage through a
//! jump point's gate costs it (`nova_galaxy_format::fleet::Stats::gate_fuel`).
//! A `shïp` has no field for it, so the loader keeps it here, held within
//! `i16` as `Fuel` is ([`gate_fuel`](crate::new_game::fleet::gate_fuel),
//! filled by `crate::new_game::load`), and [`GameData::gate_fuel`] answers
//! it. The catalog serves it on the ship's fields, and a hypergate passage
//! charges it under the rulebook's `gate_fuel` reading
//! (`nova_sim::gate_fuel`); a ship with none pays one jump's worth there.
//!
//! The stock game's store holds none: every stock ship answers `None`.

use std::collections::BTreeMap;

use super::GameData;
use crate::wire::id::ShipId;

impl GameData {
    /// The store with `fuels` as its ships' gate fuel, replacing any it
    /// had (see the module docs).
    #[must_use]
    pub fn with_gate_fuels(mut self, fuels: BTreeMap<ShipId, i16>) -> Self {
        self.gate_fuels = fuels;
        self
    }

    /// The fuel a hypergate passage costs ship type `id`, if the store has
    /// it: only the new game's does (see the module docs).
    #[must_use]
    pub fn gate_fuel(&self, id: ShipId) -> Option<i16> {
        self.gate_fuels.get(&id).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::store::fixture::store_of;

    #[test]
    fn a_stock_store_has_no_gate_fuel() {
        let decoded = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&decoded, &empty, &loaded] {
            assert_eq!(data.gate_fuel(ShipId(128)), None);
        }
    }

    #[test]
    fn a_store_with_gate_fuels_answers_each_by_ship_and_new_ones_replace_them() {
        let data = GameData::default()
            .with_gate_fuels(BTreeMap::from([(ShipId(128), 150), (ShipId(129), 0)]));
        assert_eq!(data.gate_fuel(ShipId(128)), Some(150));
        assert_eq!(data.gate_fuel(ShipId(129)), Some(0), "a free passage");
        assert_eq!(data.gate_fuel(ShipId(130)), None, "unlisted");
        let data = data.with_gate_fuels(BTreeMap::from([(ShipId(130), 50)]));
        assert_eq!(data.gate_fuel(ShipId(128)), None, "replaced");
        assert_eq!(data.gate_fuel(ShipId(130)), Some(50));
    }
}
