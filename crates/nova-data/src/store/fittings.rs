//! The new game's fittings, kept on the store: where each ship class's
//! own hardpoints sit, and each mod kit's attention and hardpoints.
//!
//! A ship class's `shän` holds its exit points, four of each kind, which
//! every ship of the class fires from. The new game's hulls and mod kits
//! place hardpoints instead, a kit adding its own wherever it is fitted.
//! The loader (`crate::new_game`'s `fleet` and `kits`) keeps each
//! class's chain hardpoints ([`Fittings::hulls`]) and each kit `oütf`'s
//! ([`Fittings::kits`]) here, then [`GameData::with_fittings`] sets them
//! and [`GameData::fittings`] answers them, so the session can rebuild
//! the player's exit points from its class and the kits it carries
//! (`nova_sim::combat::spots`). A kit's attention, how much more the law
//! notices a ship that carries it, has no `oütf` field either, so it is
//! kept here too (`nova_sim::dock_scan`).
//!
//! The stock game's store holds none.

use std::collections::BTreeMap;

use super::GameData;
use crate::wire::id::{OutfitId, ShipId};

/// The new game's hardpoints and kit attention.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fittings {
    /// Each ship class's own hardpoints, its `base` chain's, by its
    /// `shïp`: its kits' are in [`kits`](Self::kits).
    pub hulls: BTreeMap<ShipId, Vec<Spot>>,
    /// Each mod kit's attention and hardpoints, by its `oütf`.
    pub kits: BTreeMap<OutfitId, Kit>,
}

impl Fittings {
    /// Whether there are none: a stock store's.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hulls.is_empty() && self.kits.is_empty()
    }
}

/// A mod kit's fittings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kit {
    /// How much more the law notices a ship carrying it.
    pub attention: u32,
    /// The hardpoints it adds, in the order it gives them.
    pub spots: Vec<Spot>,
}

/// A hardpoint where a weapon fires from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spot {
    /// Which of the `shän`'s sets of exit points it is one of.
    pub kind: SpotKind,
    /// Where it is, in whole pixels from the ship's centre with the ship
    /// facing up: +x to its right and +y ahead, as the `shän`'s exit
    /// points are.
    pub at: (i16, i16),
}

/// Which `ExitType` a hardpoint fires its shots from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpotKind {
    /// A fixed gun's: `GunPosX/Y`, `ExitType` 0.
    Gun,
    /// A turret's: `TurretPosX/Y`, `ExitType` 1.
    Turret,
    /// A launcher's: `GuidedPosX/Y`, `ExitType` 2.
    Guided,
}

impl GameData {
    /// The store with `fittings`, replacing any it had.
    #[must_use]
    pub fn with_fittings(mut self, fittings: Fittings) -> Self {
        self.fittings = fittings;
        self
    }

    /// The new game's fittings: none in a stock store (see the module
    /// docs).
    #[must_use]
    pub fn fittings(&self) -> &Fittings {
        &self.fittings
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fittings() -> Fittings {
        let spot = |kind, at| Spot { kind, at };
        Fittings {
            hulls: BTreeMap::from([(ShipId(128), vec![spot(SpotKind::Gun, (0, 8))])]),
            kits: BTreeMap::from([(
                OutfitId(130),
                Kit {
                    attention: 2,
                    spots: vec![spot(SpotKind::Turret, (0, -2))],
                },
            )]),
        }
    }

    #[test]
    fn a_stock_store_keeps_no_fittings() {
        assert_eq!(GameData::default().fittings(), &Fittings::default());
        let data = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        assert!(data.fittings().is_empty());
    }

    #[test]
    fn a_store_with_fittings_answers_them_and_new_ones_replace_them() {
        let data = GameData::default().with_fittings(fittings());
        assert_eq!(data.fittings(), &fittings());
        assert!(!data.fittings().is_empty());
        let data = data.with_fittings(Fittings::default());
        assert_eq!(data.fittings(), &Fittings::default());
    }

    #[test]
    fn hulls_alone_or_kits_alone_are_not_empty() {
        let hulls = Fittings {
            kits: BTreeMap::new(),
            ..fittings()
        };
        let kits = Fittings {
            hulls: BTreeMap::new(),
            ..fittings()
        };
        assert!(!hulls.is_empty());
        assert!(!kits.is_empty());
    }
}
