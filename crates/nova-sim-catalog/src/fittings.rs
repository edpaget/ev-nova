//! The new game's fittings: where each ship class's own hardpoints sit,
//! each mod kit's attention, hidden cargo and hardpoints, and each
//! law-keeping faction's own scan rate.
//!
//! The store keeps them as the loader mapped them (`nova_data::Fittings`),
//! by `shïp`, by kit `oütf` and by `gövt`, each hardpoint a [`Spot`] in
//! the `shän`'s pixels. They serve here as they are, re-exported, as
//! [`storylines`](crate::storylines) re-exports its types. The catalog
//! serves them through [`PilotCatalog::fittings`], whose default is none:
//! the stock game has no fittings, so its exit points are its `shän`s',
//! no kit draws the law's attention and no kit hides cargo. The rules
//! that read them are above: the player's exit points
//! (`nova_sim::combat::spots`), and the dock scan's odds and both scans'
//! hidden cargo (`nova_sim::dock_scan`).
//!
//! [`PilotCatalog::fittings`]: crate::catalog::PilotCatalog::fittings

pub use nova_data::{Fittings, Kit, Spot, SpotKind};

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nova_data::{GameData, GovtId, OutfitId, ShipId};

    use super::*;
    use crate::catalog::PilotCatalog;
    use crate::fixture::{FakePilotCatalog, catalog};

    fn fittings() -> Fittings {
        Fittings {
            hulls: BTreeMap::from([(
                ShipId(128),
                vec![Spot {
                    kind: SpotKind::Gun,
                    at: (0, 8),
                }],
            )]),
            kits: BTreeMap::from([(
                OutfitId(131),
                Kit {
                    attention: 2,
                    hidden_cargo: 4,
                    spots: vec![Spot {
                        kind: SpotKind::Turret,
                        at: (0, -2),
                    }],
                },
            )]),
            scan_rates: BTreeMap::from([(GovtId(129), 50)]),
        }
    }

    #[test]
    fn a_stock_store_and_the_fake_serve_none() {
        assert_eq!(
            PilotCatalog::fittings(&GameData::default()),
            Fittings::default()
        );
        assert_eq!(catalog().fittings(), Fittings::default());
    }

    #[test]
    fn game_data_serves_the_store_s_fittings() {
        let data = GameData::default().with_fittings(fittings());
        assert_eq!(PilotCatalog::fittings(&data), fittings());
    }

    #[test]
    fn the_fake_serves_what_it_was_given() {
        let fake = FakePilotCatalog {
            fittings: fittings(),
            ..catalog()
        };
        assert_eq!(fake.fittings(), fittings());
        assert_eq!(through(&fake), fittings(), "through a reference");
        assert_eq!(through(std::rc::Rc::new(fake)), fittings(), "shared");
    }

    fn through(catalog: impl PilotCatalog) -> Fittings {
        catalog.fittings()
    }
}
