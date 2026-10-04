//! A canned [`PilotCatalog`] for the crate's own tests.

use std::cell::RefCell;

use crate::catalog::{
    CharacterStart, LandingSite, PilotCatalog, ShipId, StarSystem, StartDate, StartError,
    StellarId, SystemId,
};
use crate::fuel::OutfitMod;
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::landing::StellarFlags;

/// A canned first `chär`, ships and systems; records the ships asked
/// for.
pub(crate) struct FakePilotCatalog {
    pub(crate) character: Result<CharacterStart, StartError>,
    pub(crate) ships: Vec<(ShipId, Result<ShipFields, String>)>,
    pub(crate) systems: Vec<SystemId>,
    pub(crate) ships_asked: RefCell<Vec<ShipId>>,
    /// Each system's landing sites; any other has none.
    pub(crate) sites: Vec<(SystemId, Vec<LandingSite>)>,
    pub(crate) sites_asked: RefCell<Vec<SystemId>>,
    pub(crate) star_map: Vec<StarSystem>,
    pub(crate) star_map_reads: RefCell<usize>,
    /// Each ship's default outfits' mods; any other ship has none.
    pub(crate) outfits: Vec<(ShipId, Vec<OutfitMod>)>,
    pub(crate) outfits_asked: RefCell<Vec<ShipId>>,
}

pub(crate) const FAST: ShipFields = ShipFields {
    speed: 600,
    accel: 900,
    maneuver: 30,
    shield: 30,
    armor: 45,
    fuel: 300,
    fuel_regen: 0,
};

/// A landable planet at (`x`, `y`), 100 x 100 (radius 50).
pub(crate) fn planet(id: i16, x: f32, y: f32) -> LandingSite {
    LandingSite {
        id: StellarId(id),
        position: Vec2::new(x, y),
        frame_size: Some((100, 100)),
        flags: StellarFlags::CAN_LAND,
        min_status: 0,
        landing_sound: None,
    }
}

/// The first `chär` flies ship 128 from system 130 on 23 June 1177;
/// ship 128 is fast, and systems 130 and 131 exist. System 130 holds a
/// planet, 128, at (30, -40), which the ship starts over, and another,
/// 129, far away; system 131 holds one at the centre. On the map, 130
/// at (0, 0), 131 at (600, 0) and 132 at (600, 600) are linked in a
/// line, and 133 is linked to none.
pub(crate) fn catalog() -> FakePilotCatalog {
    FakePilotCatalog {
        character: Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [Some(SystemId(130)), None, None, None],
            start: START,
            ..CharacterStart::default()
        }),
        ships: vec![(ShipId(128), Ok(FAST))],
        systems: vec![SystemId(130), SystemId(131)],
        ships_asked: RefCell::default(),
        sites: vec![
            (
                SystemId(130),
                vec![planet(128, 30.0, -40.0), planet(129, 2000.0, 0.0)],
            ),
            (SystemId(131), vec![planet(140, 0.0, 0.0)]),
        ],
        sites_asked: RefCell::default(),
        star_map: vec![
            star(130, (0.0, 0.0), &[131]),
            star(131, (600.0, 0.0), &[132]),
            star(132, (600.0, 600.0), &[]),
            star(133, (-600.0, 0.0), &[]),
        ],
        star_map_reads: RefCell::default(),
        outfits: Vec::new(),
        outfits_asked: RefCell::default(),
    }
}

pub(crate) const START: StartDate = StartDate {
    day: 23,
    month: 6,
    year: 1177,
};

pub(crate) fn star(id: i16, (x, y): (f32, f32), links: &[i16]) -> StarSystem {
    StarSystem {
        id: SystemId(id),
        position: Vec2::new(x, y),
        links: links.iter().copied().map(SystemId).collect(),
    }
}

pub(crate) fn starting(systems: [Option<i16>; 4]) -> FakePilotCatalog {
    FakePilotCatalog {
        character: Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: systems.map(|slot| slot.map(SystemId)),
            start: START,
            ..CharacterStart::default()
        }),
        ..catalog()
    }
}

impl PilotCatalog for FakePilotCatalog {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        self.character.clone()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        self.ships_asked.borrow_mut().push(id);
        self.ships
            .iter()
            .find(|(ship, _)| *ship == id)
            .map_or_else(|| Err(format!("no shïp {}", id.0)), |(_, f)| f.clone())
    }

    fn default_outfits(&self, id: ShipId) -> Vec<OutfitMod> {
        self.outfits_asked.borrow_mut().push(id);
        self.outfits
            .iter()
            .find(|(ship, _)| *ship == id)
            .map_or_else(Vec::new, |(_, mods)| mods.clone())
    }

    fn system_exists(&self, id: SystemId) -> bool {
        self.systems.contains(&id)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        self.sites_asked.borrow_mut().push(system);
        self.sites
            .iter()
            .find(|(id, _)| *id == system)
            .map_or_else(Vec::new, |(_, sites)| sites.clone())
    }

    fn star_map(&self) -> Vec<StarSystem> {
        *self.star_map_reads.borrow_mut() += 1;
        self.star_map.clone()
    }
}
