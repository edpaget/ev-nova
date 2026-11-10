//! The new game's mounts and muzzles, kept on the store: where each
//! hardpoint's outfit stands on its hull, what it shows there, and how far
//! ahead of its mount each weapon's muzzle is.
//!
//! A hardpoint gives only `[x, y]`; its height comes from the hull's
//! model, cast at load (`nova_galaxy_media::mount`, asset decision 2 of
//! game-bible's format). The loader (`crate::new_game::load`, its
//! `mounts` module) works out every ship class's own mounts and every mod
//! kit's on every class, and keeps them here ([`GameData::with_mounts`]);
//! [`GameData::mounts`] answers a ship's: its class's, then each kit's on
//! that class, in the order given. That is the order a ship's exit points
//! take its spots in (`crate::store::Fittings`), so the `n`th turret drawn
//! is the `n`th turret fired from. An NPC's kits are its class's own; the
//! player's are the kits it carries.
//!
//! Each weapon whose outfit has a model that reads has a muzzle reach,
//! in whole pixels at [`PX_PER_METRE`](crate::new_game::fleet::PX_PER_METRE)
//! ([`GameData::with_muzzle_reaches`], [`GameData::muzzle_reach`]): the
//! simulation starts its shots that far ahead of the exit point under the
//! rulebook's `mounted_muzzles` reading.
//!
//! The view draws a ship's outfits on its mounts from what
//! [`GameData::mounts`] answers (`nova_view::flight::mounted_outfits`).
//!
//! The stock game's store holds neither.

use std::collections::BTreeMap;

pub use nova_galaxy_media::mount::Footing;

use super::GameData;
use crate::wire::id::{OutfitId, ShipId, WeaponId};

/// The new game's mounts: every ship class's own, and every mod kit's on
/// every class.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mounts {
    /// Each ship class's own mounts, its `base` chain's hardpoints in
    /// order, by its `shïp`.
    pub hulls: BTreeMap<ShipId, Vec<Mount>>,
    /// Each mod kit's mounts on each ship class, its hardpoints in order,
    /// by the class's `shïp` and the kit's `oütf`: a kit's mounts stand
    /// on the hull it is fitted to.
    pub kits: BTreeMap<(ShipId, OutfitId), Vec<Mount>>,
}

/// Where a hardpoint's outfit stands on its hull, and what it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Mount {
    /// The hardpoint's `id`.
    pub hardpoint: String,
    /// Where it stands, in 3D ship space: `[starboard, nose, up]` in
    /// metres from the hull model's middle, as the loaded mesh is placed
    /// (`nova_galaxy_media::Placement::ship_point_3d`).
    pub at: [f64; 3],
    /// What it stands on.
    pub footing: Footing,
    /// Whether its outfit turns on it: a `turret` hardpoint's does.
    pub turns: bool,
    /// The outfit model it shows, if any.
    pub shows: Option<Shown>,
}

/// The outfit model a mount shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    /// The model's asset id.
    pub asset: String,
    /// How far the model's middle stands above the mount, in metres, so
    /// its base rests on it (`nova_galaxy_media::mount::Fitting::lift`).
    pub lift: f64,
}

impl GameData {
    /// The store with `mounts`, replacing any it had (see the module
    /// docs).
    #[must_use]
    pub fn with_mounts(mut self, mounts: Mounts) -> Self {
        self.mounts = mounts;
        self
    }

    /// The mounts of a ship of class `ship` carrying `kits`: the class's
    /// own, then each kit's on that class, in the order given. A kit with
    /// no mounts on the class adds none; an unknown class has none of its
    /// own. None in a stock store (see the module docs).
    #[must_use]
    pub fn mounts(&self, ship: ShipId, kits: &[OutfitId]) -> Vec<Mount> {
        let own = self.mounts.hulls.get(&ship).into_iter().flatten();
        let fitted = kits
            .iter()
            .filter_map(|&kit| self.mounts.kits.get(&(ship, kit)))
            .flatten();
        own.chain(fitted).cloned().collect()
    }

    /// The store with `reaches` as its weapons' muzzle reaches, in whole
    /// pixels, replacing any it had (see the module docs).
    #[must_use]
    pub fn with_muzzle_reaches(mut self, reaches: BTreeMap<WeaponId, i16>) -> Self {
        self.muzzle_reaches = reaches;
        self
    }

    /// How far ahead of its exit point weapon `id`'s muzzle is, in whole
    /// pixels, if the store has it: only the new game's weapons with a
    /// model do (see the module docs).
    #[must_use]
    pub fn muzzle_reach(&self, id: WeaponId) -> Option<i16> {
        self.muzzle_reaches.get(&id).copied()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::store::fixture::store_of;

    fn mount(hardpoint: &str, x: f64) -> Mount {
        Mount {
            hardpoint: hardpoint.to_owned(),
            at: [x, 0.0, 1.5],
            footing: Footing::Surface,
            turns: false,
            shows: None,
        }
    }

    fn mounts() -> Mounts {
        Mounts {
            hulls: BTreeMap::from([
                (ShipId(128), vec![mount("nose", 0.0), mount("tail", 1.0)]),
                (ShipId(129), vec![mount("keel", 2.0)]),
            ]),
            kits: BTreeMap::from([
                ((ShipId(128), OutfitId(140)), vec![mount("bed", 3.0)]),
                ((ShipId(128), OutfitId(141)), vec![mount("ring", 4.0)]),
                ((ShipId(129), OutfitId(140)), vec![mount("bed", 5.0)]),
            ]),
        }
    }

    fn names(mounts: &[Mount]) -> Vec<(&str, f64)> {
        mounts
            .iter()
            .map(|m| (m.hardpoint.as_str(), m.at[0]))
            .collect()
    }

    #[test]
    fn a_stock_store_has_no_mounts_and_no_reaches() {
        let decoded = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&decoded, &empty, &loaded] {
            assert_eq!(data.mounts(ShipId(128), &[OutfitId(140)]), Vec::new());
            assert_eq!(data.muzzle_reach(WeaponId(128)), None);
        }
    }

    #[test]
    fn a_ship_s_mounts_are_its_class_s_then_each_kit_s_on_it_in_the_order_given() {
        let data = GameData::default().with_mounts(mounts());
        assert_eq!(
            names(&data.mounts(ShipId(128), &[])),
            [("nose", 0.0), ("tail", 1.0)]
        );
        assert_eq!(
            names(&data.mounts(ShipId(128), &[OutfitId(141), OutfitId(140)])),
            [("nose", 0.0), ("tail", 1.0), ("ring", 4.0), ("bed", 3.0)]
        );
        assert_eq!(
            names(&data.mounts(ShipId(129), &[OutfitId(140)])),
            [("keel", 2.0), ("bed", 5.0)],
            "the kit's mount on this class"
        );
    }

    #[test]
    fn a_kit_the_class_has_no_mounts_for_adds_none() {
        let data = GameData::default().with_mounts(mounts());
        assert_eq!(
            names(&data.mounts(ShipId(129), &[OutfitId(141), OutfitId(150)])),
            [("keel", 2.0)]
        );
    }

    #[test]
    fn an_unknown_class_has_none_of_its_own_and_new_mounts_replace_old() {
        let data = GameData::default().with_mounts(mounts());
        assert_eq!(data.mounts(ShipId(130), &[]), Vec::new());
        let data = data.with_mounts(Mounts::default());
        assert_eq!(data.mounts(ShipId(128), &[OutfitId(140)]), Vec::new());
    }

    #[test]
    fn a_store_with_muzzle_reaches_answers_each_by_weapon_and_new_ones_replace_them() {
        let data = GameData::default()
            .with_muzzle_reaches(BTreeMap::from([(WeaponId(128), 3), (WeaponId(129), 1)]));
        assert_eq!(data.muzzle_reach(WeaponId(128)), Some(3));
        assert_eq!(data.muzzle_reach(WeaponId(129)), Some(1));
        assert_eq!(data.muzzle_reach(WeaponId(130)), None, "unlisted");
        let data = data.with_muzzle_reaches(BTreeMap::from([(WeaponId(130), 2)]));
        assert_eq!(data.muzzle_reach(WeaponId(128)), None, "replaced");
        assert_eq!(data.muzzle_reach(WeaponId(130)), Some(2));
    }
}
