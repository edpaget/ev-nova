//! A canned [`PilotCatalog`] for the crate's own tests.

use std::cell::RefCell;

use crate::catalog::{
    CharacterStart, CommodityStrings, DisasterRecord, JunkRecord, LandingSite, PilotCatalog,
    ShipId, StarSystem, StartDate, StartError, StellarId, SystemId,
};
use crate::chance::{Chance, NeverFires};
use crate::flight::{Controls, Turn};
use crate::fuel::OutfitMod;
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::hyperspace::MIN_JUMP_DISTANCE;
use crate::landing::StellarFlags;
use crate::session::Session;

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
    pub(crate) commodities: CommodityStrings,
    pub(crate) junk: Vec<JunkRecord>,
    pub(crate) disasters: Vec<DisasterRecord>,
    /// How many times the goods (commodities, `jünk` and `öops`) were read.
    pub(crate) goods_reads: RefCell<usize>,
}

pub(crate) const FAST: ShipFields = ShipFields {
    speed: 600,
    accel: 900,
    maneuver: 30,
    shield: 30,
    armor: 45,
    fuel: 300,
    fuel_regen: 0,
    holds: 20,
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
        commodities: CommodityStrings::default(),
        junk: Vec::new(),
        disasters: Vec::new(),
        goods_reads: RefCell::default(),
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

    fn commodity_strings(&self) -> CommodityStrings {
        *self.goods_reads.borrow_mut() += 1;
        self.commodities.clone()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        *self.goods_reads.borrow_mut() += 1;
        self.junk.clone()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        *self.goods_reads.borrow_mut() += 1;
        self.disasters.clone()
    }
}

/// Flies the ship out from the centre until it is at least
/// [`MIN_JUMP_DISTANCE`] away: it first turns to face away from the
/// centre (Down faces against its motion), then thrusts.
pub(crate) fn fly_out(session: &mut Session) {
    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };
    for _ in 0..3000 {
        let player = *session.player();
        if player.position.length() >= MIN_JUMP_DISTANCE {
            return;
        }
        let outward = if player.position.length() > 0.0 {
            crate::flight::heading_of(player.position)
        } else {
            0.0
        };
        let off = crate::flight::shortest_turn(player.heading, outward).abs();
        // Down lands exactly on the heading against the motion, which
        // is outward while the ship drifts in.
        let controls = if off < 1e-3 {
            THRUST
        } else if player.velocity.length() > crate::flight::AT_REST_SPEED {
            Controls {
                reverse: true,
                ..Controls::default()
            }
        } else {
            Controls {
                turn: Turn::Right,
                ..Controls::default()
            }
        };
        session.tick(controls);
    }
    panic!("never got out: {:?}", session.player());
}

/// Plots a course to `to`, flies out and jumps, and arrives, no chance
/// firing on the way.
pub(crate) fn jump(session: &mut Session, catalog: &FakePilotCatalog, to: i16) -> Option<SystemId> {
    jump_with(session, catalog, to, &mut NeverFires)
}

/// Plots a course to `to`, flies out and jumps, and arrives, the day's
/// chances rolled on `chance`.
pub(crate) fn jump_with(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    to: i16,
    chance: &mut impl Chance,
) -> Option<SystemId> {
    if session.course().last() != Some(&SystemId(to)) {
        session.plot_course(SystemId(to)).expect("a route");
    }
    fly_out(session);
    session.begin_jump().expect("jumps");
    session.arrive(catalog, chance)
}

/// A [`Chance`] that answers from a script (no once it runs out) and
/// records each percent it is asked.
#[derive(Debug, Default)]
pub(crate) struct Scripted {
    answers: Vec<bool>,
    pub(crate) asked: Vec<u8>,
}

impl Scripted {
    /// Answers `answers`, in order, then no.
    pub(crate) fn answering(answers: &[bool]) -> Self {
        Self {
            answers: answers.iter().rev().copied().collect(),
            asked: Vec::new(),
        }
    }
}

impl Chance for Scripted {
    fn fires(&mut self, percent: u8) -> bool {
        self.asked.push(percent);
        self.answers.pop().unwrap_or(false)
    }
}

/// A slow ship, 1 pixel a tick at most, gaining a unit a tick: it
/// arrives slow enough to land, over planet 140 at 131's edge.
pub(crate) fn edge_lander() -> FakePilotCatalog {
    FakePilotCatalog {
        ships: vec![(
            ShipId(128),
            Ok(ShipFields {
                speed: 100,
                fuel_regen: 1,
                ..FAST
            }),
        )],
        sites: vec![(SystemId(131), vec![planet(140, -1000.0, 0.0)])],
        ..catalog()
    }
}
