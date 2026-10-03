//! The ship catalog port: the ships the browser can show, in its own terms.

use std::num::NonZeroU16;
use std::rc::Rc;

pub use nova_data::ShipId;

/// A ship's key stats, raw from its `shïp` record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipStats {
    /// Purchase price in credits.
    pub cost: i32,
    /// Top speed.
    pub speed: i16,
    /// Armour strength.
    pub armor: i16,
    /// Shield strength.
    pub shield: i16,
}

/// A resolved sprite sheet: its `rlëD` ID and how many frames it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SheetInfo {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// How many frames it has; never zero.
    pub frames: NonZeroU16,
}

/// Everything the browser shows about one ship. Each part resolves or
/// fails on its own; a failure is a message ready to display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipEntry {
    /// The ship's ID.
    pub id: ShipId,
    /// Its display name.
    pub name: String,
    /// Its key stats.
    pub stats: Result<ShipStats, String>,
    /// Its description text, if it has one.
    pub description: Result<Option<String>, String>,
    /// Its base sprite sheet.
    pub sprite: Result<SheetInfo, String>,
    /// Its engine glow layer, if its `shän` defines one.
    pub glow: Option<Result<SheetInfo, String>>,
    /// Its running lights layer, if its `shän` defines one.
    pub lights: Option<Result<SheetInfo, String>>,
}

/// The ships the browser can show.
pub trait ShipCatalog {
    /// Every ship's ID, ascending.
    fn ship_ids(&self) -> Vec<ShipId>;
    /// Everything about ship `id`.
    fn ship(&self, id: ShipId) -> ShipEntry;
}

/// A borrowed catalog is a catalog.
impl<T: ShipCatalog + ?Sized> ShipCatalog for &T {
    fn ship_ids(&self) -> Vec<ShipId> {
        (**self).ship_ids()
    }

    fn ship(&self, id: ShipId) -> ShipEntry {
        (**self).ship(id)
    }
}

/// A shared catalog is a catalog, so a screen and the renderer can read
/// the same game data.
impl<T: ShipCatalog + ?Sized> ShipCatalog for Rc<T> {
    fn ship_ids(&self) -> Vec<ShipId> {
        (**self).ship_ids()
    }

    fn ship(&self, id: ShipId) -> ShipEntry {
        (**self).ship(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ships 5 and 9, each named after its ID.
    struct TwoShips;

    impl ShipCatalog for TwoShips {
        fn ship_ids(&self) -> Vec<ShipId> {
            vec![ShipId(5), ShipId(9)]
        }

        fn ship(&self, id: ShipId) -> ShipEntry {
            ShipEntry {
                id,
                name: format!("Ship {}", id.0),
                stats: Err("no stats".to_owned()),
                description: Ok(None),
                sprite: Err("no sprite".to_owned()),
                glow: None,
                lights: None,
            }
        }
    }

    /// Reads `catalog` through the trait, as the browser does.
    fn names(catalog: impl ShipCatalog) -> Vec<String> {
        catalog
            .ship_ids()
            .into_iter()
            .map(|id| catalog.ship(id).name)
            .collect()
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        assert_eq!(names(&TwoShips), ["Ship 5", "Ship 9"]);
        assert_eq!(names(Rc::new(TwoShips)), ["Ship 5", "Ship 9"]);
    }
}
