//! The galaxy catalog port: everything the map shows, in its own terms.

use std::collections::BTreeMap;
use std::rc::Rc;

pub use nova_data::{GovtId, NebulaId, StellarId, SystemId};

/// One stellar object in a system: its ID and display name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StellarEntry {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its display name.
    pub name: String,
}

/// One star system, as the map draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemEntry {
    /// The `sÿst`'s ID.
    pub id: SystemId,
    /// Its display name.
    pub name: String,
    /// Its map X position; x grows right.
    pub x: i16,
    /// Its map Y position; y grows down, as on screen.
    pub y: i16,
    /// Its hyperlinks, raw and in record order: they may repeat, point at
    /// itself or at a system that does not exist. The simulation's
    /// `StarMap` normalises them, and the map draws its links.
    pub links: Vec<SystemId>,
    /// The government that owns it, or `None` for an independent system.
    pub govt: Option<GovtId>,
    /// Its stellar objects, in record order.
    pub stellars: Vec<StellarEntry>,
}

/// One of a nebula's pictures: its `PICT` ID and size in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NebulaPicture {
    /// The `PICT`'s ID.
    pub id: i16,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
}

/// One nebula: the map rectangle it covers and the pictures that can fill
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NebulaEntry {
    /// The `nëbu`'s ID.
    pub id: NebulaId,
    /// Its display name.
    pub name: String,
    /// Its left edge on the map.
    pub left: i16,
    /// Its top edge on the map.
    pub top: i16,
    /// Its width at 100% map scale.
    pub width: i16,
    /// Its height at 100% map scale.
    pub height: i16,
    /// The pictures that exist for it, in the game's order. Their sizes say
    /// which map scale each suits; the order does not.
    pub pictures: Vec<NebulaPicture>,
}

/// The whole galaxy, read in one go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Galaxy {
    /// Every system that could be read, by ascending ID.
    pub systems: Vec<SystemEntry>,
    /// Every government's map colour, a 24-bit `00RRGGBB` value.
    pub govt_colors: BTreeMap<GovtId, u32>,
    /// Every nebula, by ascending ID.
    pub nebulae: Vec<NebulaEntry>,
    /// What could not be read, each a message ready to display.
    pub problems: Vec<String>,
}

/// The galaxy the map shows.
pub trait GalaxyCatalog {
    /// Every system, government colour and nebula.
    fn galaxy(&self) -> Galaxy;
}

/// A borrowed catalog is a catalog.
impl<T: GalaxyCatalog + ?Sized> GalaxyCatalog for &T {
    fn galaxy(&self) -> Galaxy {
        (**self).galaxy()
    }
}

/// A shared catalog is a catalog, so the map and the renderer can read the
/// same game data.
impl<T: GalaxyCatalog + ?Sized> GalaxyCatalog for Rc<T> {
    fn galaxy(&self) -> Galaxy {
        (**self).galaxy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A galaxy with one problem and nothing else.
    struct Broken;

    impl GalaxyCatalog for Broken {
        fn galaxy(&self) -> Galaxy {
            Galaxy {
                problems: vec!["broken".to_owned()],
                ..Galaxy::default()
            }
        }
    }

    fn problems(catalog: impl GalaxyCatalog) -> Vec<String> {
        catalog.galaxy().problems
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        assert_eq!(problems(&Broken), ["broken"]);
        assert_eq!(problems(Rc::new(Broken)), ["broken"]);
    }
}
