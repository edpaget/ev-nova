//! The 3D models the flight screen draws in place of the new game's 2D
//! stand-ins ([`WorldModels`]): which ship classes, outfits and stellars
//! have a model or a planet registered with the renderer, under which
//! key, and where each class's outfits stand.
//!
//! It is plain data, built at the program's edge once the keys are known
//! (`nova_render`'s `new_game_world`, which registers each key's mesh or
//! planet with the renderer as it hands one out) and given to the flight
//! ([`FlightView::with_world_models`](super::view::FlightView::with_world_models)).
//! Tests build one by hand. An empty table, the default, draws exactly
//! as no table: stock data has no models, so it never gets one.
//!
//! What a key's presence decides, and in what order a ship's looks are
//! tried, is the flight view's (its `ship_models` and `stellar_models`
//! modules). The mounts answer as the store's do
//! ([`Mounts::of`], `nova_data::store::GameData::mounts`): a class's own,
//! then each kit's on that class, in the order given.

use std::collections::BTreeMap;

use nova_data::store::{Mount, Mounts};
use nova_sim_frame::{OutfitId, ShipId, StellarId};

use crate::model::ModelKey;
use crate::planet::PlanetKey;

/// A ship class's hull model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipModel {
    /// The hull's model.
    pub key: ModelKey,
    /// Logical units per model unit to draw it at: a ship's model is in
    /// metres, drawn at the pixels a metre spans
    /// (`nova_data::new_game::fleet::PX_PER_METRE`).
    pub scale: f32,
}

/// What a stellar draws in 3D.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StellarModel {
    /// A model, a station's or an outpost's, in its own units.
    Model {
        /// The model.
        key: ModelKey,
        /// How far its footprint reaches from its centre, in model units,
        /// measured as its stand-in outline's reach is
        /// (`nova_data::store::Outline::reach`): the view scales it so it
        /// reaches as far as the stand-in would in the same slot.
        reach: f32,
    },
    /// A planet or moon, through the planet shader.
    Planet {
        /// The planet.
        key: PlanetKey,
    },
}

/// The models the flight draws (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldModels {
    /// Each ship class's hull model, by its `shïp`.
    pub ships: BTreeMap<ShipId, ShipModel>,
    /// Each ship class's kits as built, by its `shïp`: an NPC of the class
    /// carries them.
    pub built_kits: BTreeMap<ShipId, Vec<OutfitId>>,
    /// Every class's own mounts and every kit's on every class.
    pub mounts: Mounts,
    /// Each outfit model's key, by its asset id.
    pub outfits: BTreeMap<String, ModelKey>,
    /// Each stellar's model or planet, by its `spöb`.
    pub stellars: BTreeMap<StellarId, StellarModel>,
}

impl WorldModels {
    /// Ship class `ship`'s hull model, if it has one.
    #[must_use]
    pub fn ship(&self, ship: ShipId) -> Option<ShipModel> {
        self.ships.get(&ship).copied()
    }

    /// The mounts of a ship of class `ship` carrying `kits` (see the
    /// module docs).
    #[must_use]
    pub fn mounts(&self, ship: ShipId, kits: &[OutfitId]) -> Vec<Mount> {
        self.mounts.of(ship, kits)
    }

    /// Class `ship`'s kits as built: none for a class with none or not in
    /// the table.
    #[must_use]
    pub fn built_kits(&self, ship: ShipId) -> &[OutfitId] {
        self.built_kits.get(&ship).map_or(&[], Vec::as_slice)
    }

    /// The outfit model `asset`'s key, if it has one.
    #[must_use]
    pub fn outfit(&self, asset: &str) -> Option<ModelKey> {
        self.outfits.get(asset).copied()
    }

    /// Stellar `stellar`'s model or planet, if it has one.
    #[must_use]
    pub fn stellar(&self, stellar: StellarId) -> Option<StellarModel> {
        self.stellars.get(&stellar).copied()
    }
}

#[cfg(test)]
mod tests {
    use nova_data::store::{Footing, Shown};

    use super::*;

    const HULL: ModelKey = ModelKey(1);
    const TURRET: ModelKey = ModelKey(2);
    const HUB: ModelKey = ModelKey(3);
    const GIANT: PlanetKey = PlanetKey(1);
    const BED: OutfitId = OutfitId(130);
    const RACK: OutfitId = OutfitId(131);

    fn mount(hardpoint: &str) -> Mount {
        Mount {
            hardpoint: hardpoint.to_owned(),
            at: [0.0, 0.0, 1.0],
            footing: Footing::Surface,
            turns: true,
            shows: Some(Shown {
                asset: "turret-mount".to_owned(),
                lift: 0.2,
            }),
        }
    }

    fn table() -> WorldModels {
        WorldModels {
            ships: BTreeMap::from([(
                ShipId(128),
                ShipModel {
                    key: HULL,
                    scale: 1.0,
                },
            )]),
            built_kits: BTreeMap::from([(ShipId(129), vec![BED])]),
            mounts: Mounts {
                hulls: BTreeMap::from([(ShipId(128), vec![mount("nose")])]),
                kits: BTreeMap::from([
                    ((ShipId(128), BED), vec![mount("bed")]),
                    ((ShipId(128), RACK), vec![mount("rack")]),
                ]),
            },
            outfits: BTreeMap::from([("turret-mount".to_owned(), TURRET)]),
            stellars: BTreeMap::from([
                (
                    StellarId(128),
                    StellarModel::Model {
                        key: HUB,
                        reach: 4.0,
                    },
                ),
                (StellarId(136), StellarModel::Planet { key: GIANT }),
            ]),
        }
    }

    fn hardpoints(mounts: &[Mount]) -> Vec<&str> {
        mounts.iter().map(|m| m.hardpoint.as_str()).collect()
    }

    #[test]
    fn the_table_answers_each_key_and_scale() {
        let table = table();
        assert_eq!(
            table.ship(ShipId(128)),
            Some(ShipModel {
                key: HULL,
                scale: 1.0
            })
        );
        assert_eq!(table.ship(ShipId(129)), None);
        assert_eq!(table.outfit("turret-mount"), Some(TURRET));
        assert_eq!(table.outfit("gun"), None);
        assert_eq!(
            table.stellar(StellarId(128)),
            Some(StellarModel::Model {
                key: HUB,
                reach: 4.0
            })
        );
        assert_eq!(
            table.stellar(StellarId(136)),
            Some(StellarModel::Planet { key: GIANT })
        );
        assert_eq!(table.stellar(StellarId(129)), None);
    }

    #[test]
    fn a_ship_s_mounts_are_its_class_s_then_its_kits_in_the_order_given() {
        let table = table();
        assert_eq!(hardpoints(&table.mounts(ShipId(128), &[])), ["nose"]);
        assert_eq!(
            hardpoints(&table.mounts(ShipId(128), &[RACK, BED])),
            ["nose", "rack", "bed"]
        );
        assert_eq!(hardpoints(&table.mounts(ShipId(129), &[BED])), [""; 0]);
    }

    #[test]
    fn a_class_s_built_kits_are_its_own_or_none() {
        let table = table();
        assert_eq!(table.built_kits(ShipId(129)), [BED]);
        assert_eq!(table.built_kits(ShipId(128)), []);
    }

    #[test]
    fn an_empty_table_answers_nothing() {
        let table = WorldModels::default();
        assert_eq!(table.ship(ShipId(128)), None);
        assert_eq!(table.mounts(ShipId(128), &[BED]), []);
        assert_eq!(table.built_kits(ShipId(128)), []);
        assert_eq!(table.outfit("turret-mount"), None);
        assert_eq!(table.stellar(StellarId(128)), None);
    }
}
