//! Landing through a flight session, headless: the land key's first press
//! requesting clearance and its second landing, each reason a landing is
//! refused, and a landing and take-off, with the session flown by its
//! controls alone.

use nova_sim::landing::StellarFlags;
use nova_sim::{
    CharacterStart, Clearance, CommodityStrings, Controls, DisasterRecord, JunkRecord, LandOutcome,
    LandingRefusal, LandingSite, OutfitId, OutfitRecord, PilotCatalog, Session, ShipFields, ShipId,
    StarSystem, StartDate, StartError, StellarId, SystemId, Vec2,
};

/// One `chär` flying an average ship that turns 6° a tick from system
/// 130, which holds `sites`.
struct Pilot {
    sites: Vec<LandingSite>,
}

impl PilotCatalog for Pilot {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [Some(SystemId(130)), None, None, None],
            start: StartDate::default(),
            ..CharacterStart::default()
        })
    }

    fn ship_fields(&self, _id: ShipId) -> Result<ShipFields, String> {
        Ok(ShipFields {
            speed: 300,
            accel: 300,
            maneuver: 60,
            ..ShipFields::default()
        })
    }

    fn default_outfits(&self, _id: ShipId) -> Vec<(OutfitId, u16)> {
        Vec::new()
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        Vec::new()
    }

    fn ships(&self) -> Vec<nova_sim::ShipRecord> {
        Vec::new()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        id == SystemId(130)
    }

    fn landing_sites(&self, _system: SystemId) -> Vec<LandingSite> {
        self.sites.clone()
    }

    fn star_map(&self) -> Vec<StarSystem> {
        Vec::new()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        CommodityStrings::default()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        Vec::new()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        Vec::new()
    }
}

/// Stellar 128 at the centre, where the ship starts, 200 x 200, with
/// `flags` and `min_status`.
fn at_centre(flags: u32, min_status: i16) -> LandingSite {
    LandingSite {
        id: StellarId(128),
        position: Vec2::ZERO,
        frame_size: Some((200, 200)),
        flags,
        min_status,
        landing_sound: None,
        tech_level: 0,
        special_tech: [0; 8],
        govt: None,
    }
}

fn session(sites: Vec<LandingSite>) -> Session {
    Session::start(&Pilot { sites }).expect("starts")
}

const STATION: u32 = StellarFlags::CAN_LAND | StellarFlags::STATION;

#[test]
fn a_system_without_stellars_refuses() {
    assert_eq!(session(Vec::new()).land(), Err(LandingRefusal::NoStellars));
}

fn selected(station: bool, clearance: Clearance) -> LandOutcome {
    LandOutcome::Selected {
        stellar: StellarId(128),
        station,
        clearance,
    }
}

#[test]
fn the_first_l_requests_clearance_and_the_second_lands() {
    let mut session = session(vec![at_centre(StellarFlags::CAN_LAND, 0)]);
    assert_eq!(session.land(), Ok(selected(false, Clearance::Granted)));
    assert_eq!(session.nav_target(), Some(StellarId(128)));
    assert_eq!(session.landed(), None);
    assert_eq!(session.land(), Ok(LandOutcome::Landed(StellarId(128))));
    assert_eq!(session.landed(), Some(StellarId(128)));
    assert_eq!(session.nav_target(), None);
}

#[test]
fn a_ship_over_no_stellar_is_too_far() {
    let far = LandingSite {
        position: Vec2::new(0.0, -500.0),
        ..at_centre(STATION, 0)
    };
    let mut session = session(vec![far]);
    assert_eq!(session.land(), Ok(selected(true, Clearance::Granted)));
    let too_far = Err(LandingRefusal::TooFar {
        stellar: StellarId(128),
        station: true,
    });
    assert_eq!(session.land(), too_far);
    assert_eq!(session.land(), too_far, "the target is kept: try again");
    assert_eq!(session.nav_target(), Some(StellarId(128)));
}

#[test]
fn a_stellar_that_cannot_be_landed_on_refuses() {
    let mut session = session(vec![at_centre(0, 0)]);
    assert_eq!(
        session.land(),
        Err(LandingRefusal::NotLandable {
            stellar: StellarId(128),
            station: false,
        })
    );
    assert_eq!(session.nav_target(), None, "nothing selected");
}

#[test]
fn a_new_pilot_is_denied_where_a_record_is_needed() {
    let mut session = session(vec![at_centre(StellarFlags::CAN_LAND, 32767)]);
    assert_eq!(session.land(), Ok(selected(false, Clearance::Denied)));
    assert_eq!(
        session.land(),
        Err(LandingRefusal::Denied {
            stellar: StellarId(128),
            station: false,
            min_status: 32767,
        })
    );
}

#[test]
fn a_ship_flying_past_is_too_fast_and_lands_once_it_brakes() {
    let mut session = session(vec![at_centre(StellarFlags::CAN_LAND, 0)]);
    let thrust = Controls {
        thrust: true,
        ..Controls::default()
    };
    for _ in 0..15 {
        session.tick(thrust);
    }
    assert_eq!(session.land(), Ok(selected(false, Clearance::Granted)));
    let Err(LandingRefusal::TooFast {
        stellar,
        station,
        speed,
    }) = session.land()
    else {
        panic!("too fast: {:?}", session.player())
    };
    assert_eq!((stellar, station), (StellarId(128), false));
    assert!(speed > 1.0, "{speed}");

    // Turn round and brake to a stop over it.
    let reverse = Controls {
        reverse: true,
        ..Controls::default()
    };
    for _ in 0..40 {
        session.tick(reverse);
    }
    while session.player().velocity.length() > 1.0 {
        session.tick(thrust);
    }
    assert_eq!(session.land(), Ok(LandOutcome::Landed(StellarId(128))));
    assert_eq!(session.player().velocity, Vec2::ZERO);
    assert_eq!(session.player().position, Vec2::ZERO);
    assert_eq!(session.take_off(), Some(StellarId(128)));
    assert_eq!(session.landed(), None);
}
