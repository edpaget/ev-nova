//! Fixtures the galaxy map's test modules share: a canned
//! [`GalaxyCatalog`] and its small galaxy.

use std::cell::Cell;
use std::collections::BTreeMap;

use super::catalog::{
    Galaxy, GalaxyCatalog, GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, StellarId,
    SystemEntry, SystemId,
};

/// A canned galaxy; counts how often it is read.
pub(crate) struct FakeCatalog {
    galaxy: Galaxy,
    pub(crate) reads: Cell<usize>,
}

impl FakeCatalog {
    pub(crate) fn new(galaxy: Galaxy) -> Self {
        Self {
            galaxy,
            reads: Cell::new(0),
        }
    }
}

impl GalaxyCatalog for FakeCatalog {
    fn galaxy(&self) -> Galaxy {
        self.reads.set(self.reads.get() + 1);
        self.galaxy.clone()
    }
}

/// System `id`, "System `id`", at (`x`, `y`), with no links, government or
/// stellars.
pub(crate) fn system(id: i16, x: i16, y: i16) -> SystemEntry {
    SystemEntry {
        id: SystemId(id),
        name: format!("System {id}"),
        x,
        y,
        links: Vec::new(),
        govt: None,
        stellars: Vec::new(),
    }
}

/// The Federation's colour in [`galaxy`].
pub(crate) const BLUE: u32 = 0x002C_2CAF;

/// Alpha (128) at (0, 0), Federation, with two stellars; Beta (129) at
/// (600, 0), independent; Gamma (130) at (0, 300), Federation; and the
/// alternate Alpha (131) at Alpha's position. Alpha and Beta link both
/// ways and Gamma links one way to Alpha. A nebula covers Alpha.
pub(crate) fn galaxy() -> Galaxy {
    let alpha = SystemEntry {
        name: "Alpha".to_owned(),
        links: vec![SystemId(129)],
        govt: Some(GovtId(128)),
        stellars: vec![
            StellarEntry {
                id: StellarId(128),
                name: "Alpha Prime".to_owned(),
            },
            StellarEntry {
                id: StellarId(129),
                name: "Alpha Station".to_owned(),
            },
        ],
        ..system(128, 0, 0)
    };
    let beta = SystemEntry {
        name: "Beta".to_owned(),
        links: vec![SystemId(128)],
        ..system(129, 600, 0)
    };
    let gamma = SystemEntry {
        name: "Gamma".to_owned(),
        links: vec![SystemId(128)],
        govt: Some(GovtId(128)),
        ..system(130, 0, 300)
    };
    let alternate = SystemEntry {
        name: "Alpha Reborn".to_owned(),
        ..system(131, 0, 0)
    };
    Galaxy {
        systems: vec![alpha, beta, gamma, alternate],
        govt_colors: BTreeMap::from([(GovtId(128), BLUE)]),
        nebulae: vec![NebulaEntry {
            id: NebulaId(128),
            name: "Holpa Nebula".to_owned(),
            left: -20,
            top: -20,
            width: 60,
            height: 60,
            pictures: vec![
                NebulaPicture {
                    id: 9500,
                    slot: 0,
                    width: 30,
                    height: 30,
                },
                NebulaPicture {
                    id: 9502,
                    slot: 2,
                    width: 60,
                    height: 60,
                },
            ],
        }],
        problems: Vec::new(),
    }
}
