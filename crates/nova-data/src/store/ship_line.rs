//! Each new-game ship's lineage: its chain of bible ids, nearest first,
//! and its resolved role.
//!
//! The new game's loader keeps it here, on the store, beside the
//! stand-ins ([`GameData::with_ship_lines`], filled by
//! `crate::new_game::load`), and [`GameData::ship_line`] answers it. The
//! opt-in stock look ([`crate::stock_look`]) reads it to pick each hull's
//! borrowed stock sprites: an override for a ship's own bible id, then for
//! its nearest base's, then the default for its role. It names no stock
//! data, so the new game's store stays unaware of the stock one.
//!
//! The stock game's store holds none: every stock ship answers `None`.

use nova_galaxy_format::fleet::Role;

use super::GameData;
use crate::wire::id::ShipId;

/// A new-game ship's lineage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipLine {
    /// The ship's own bible id, then its base's, then its base's base's,
    /// and so on.
    pub chain: Vec<String>,
    /// The nearest `role` in the chain, if any ship in it gives one.
    pub role: Option<Role>,
}

impl GameData {
    /// The store with `lines` as its ships' lineages, replacing any it had
    /// (see the module docs).
    #[must_use]
    pub fn with_ship_lines(mut self, lines: std::collections::BTreeMap<ShipId, ShipLine>) -> Self {
        self.ship_lines = lines;
        self
    }

    /// Ship type `id`'s lineage, if the store has one: only the new
    /// game's do (see the module docs).
    #[must_use]
    pub fn ship_line(&self, id: ShipId) -> Option<&ShipLine> {
        self.ship_lines.get(&id)
    }

    /// Every ship type's lineage, by ascending ID: the new game's; none
    /// in a stock store.
    pub fn ship_lines(&self) -> impl Iterator<Item = (ShipId, &ShipLine)> {
        self.ship_lines.iter().map(|(&id, line)| (id, line))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::store::fixture::store_of;

    fn line(chain: &[&str], role: Option<Role>) -> ShipLine {
        ShipLine {
            chain: chain.iter().map(|&id| id.to_owned()).collect(),
            role,
        }
    }

    #[test]
    fn a_stock_store_has_no_ship_lines() {
        let decoded = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&decoded, &empty, &loaded] {
            assert_eq!(data.ship_line(ShipId(128)), None);
            assert_eq!(data.ship_lines().count(), 0);
        }
    }

    #[test]
    fn a_store_with_ship_lines_answers_each_by_ship() {
        let hull = line(&["hauler"], Some(Role::Freighter));
        let variant = line(&["armed-hauler", "hauler"], Some(Role::Freighter));
        let data = GameData::default().with_ship_lines(BTreeMap::from([
            (ShipId(128), hull.clone()),
            (ShipId(129), variant.clone()),
        ]));
        assert_eq!(data.ship_line(ShipId(128)), Some(&hull));
        assert_eq!(data.ship_line(ShipId(129)), Some(&variant));
        assert_eq!(data.ship_line(ShipId(130)), None);
        assert_eq!(
            data.ship_lines().collect::<Vec<_>>(),
            [(ShipId(128), &hull), (ShipId(129), &variant)]
        );
    }

    #[test]
    fn ship_lines_replace_those_set_before() {
        let data = GameData::default()
            .with_ship_lines(BTreeMap::from([(ShipId(128), line(&["a"], None))]))
            .with_ship_lines(BTreeMap::from([(ShipId(129), line(&["b"], None))]));
        assert_eq!(data.ship_line(ShipId(128)), None);
        assert_eq!(data.ship_line(ShipId(129)), Some(&line(&["b"], None)));
    }
}
