//! Landing through a flight session, headless: the land key's first press
//! requesting clearance, the dock approach on the tick after, and the
//! second press landing, each reason a landing is refused, and a landing
//! and take-off, with the session flown by its controls alone.

use nova_sim::landing::StellarFlags;
use nova_sim::{
    CharacterStart, CombatCatalog, CommodityStrings, Controls, DateAffixes, DisasterRecord, DudeId,
    DudeRecord, FleetRecord, GovtRecord, HullRecord, JunkRecord, LandOutcome, LandPress,
    LandingRefusal, LandingSite, NeverFires, OutfitId, OutfitRecord, PersonRecord, PilotCatalog,
    Session, ShipFields, ShipId, StarSystem, StartDate, StartError, StellarId, SystemId,
    SystemTraffic, TrafficCatalog, Vec2, WeaponRecord,
};

/// One `chär` flying an average ship that turns 6° a tick from system
/// 130, which holds `sites`.
struct Pilot {
    sites: Vec<LandingSite>,
}

/// Unarmed and ungoverned.
impl CombatCatalog for Pilot {
    fn weapons(&self) -> Vec<WeaponRecord> {
        Vec::new()
    }

    fn hulls(&self) -> Vec<HullRecord> {
        Vec::new()
    }

    fn governments(&self) -> Vec<GovtRecord> {
        Vec::new()
    }
}

/// No traffic: the system takes off into is empty.
struct NoTraffic;

impl TrafficCatalog for NoTraffic {
    fn system_traffic(&self, _id: SystemId) -> Option<SystemTraffic> {
        None
    }

    fn dude(&self, _id: DudeId) -> Option<DudeRecord> {
        None
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        Vec::new()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        Vec::new()
    }
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

    fn string_patch(&self, _id: i16) -> nova_sim::catalog::StringPatch {
        nova_sim::catalog::StringPatch::Absent
    }

    fn buoy_strings(&self) -> Vec<String> {
        Vec::new()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        Vec::new()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        Vec::new()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        Vec::new()
    }

    fn date_affixes(&self) -> DateAffixes {
        DateAffixes::default()
    }

    fn gate_sites(&self) -> Vec<nova_sim::GateSite> {
        Vec::new()
    }

    fn stellar_lives(&self) -> Vec<nova_sim::StellarLife> {
        Vec::new()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        Some((100, 100))
    }

    fn missions(&self) -> Vec<nova_sim::catalog::MissionRecord> {
        Vec::new()
    }

    fn mission_texts(&self) -> Vec<nova_sim::catalog::MissionTexts> {
        Vec::new()
    }

    /// System 130's stellars, as it lists them.
    fn listed_stellars(&self) -> Vec<nova_sim::catalog::ListedStellar> {
        let sites = self.landing_sites(SystemId(130));
        nova_sim::fixture::listed_stellars([(SystemId(130), sites.as_slice())])
    }

    fn system_visibility(&self) -> Vec<nova_sim::catalog::SystemVisibility> {
        Vec::new()
    }

    fn place_names(&self) -> nova_sim::catalog::PlaceNames {
        nova_sim::catalog::PlaceNames::default()
    }

    fn stellar_marks(&self) -> Vec<nova_sim::catalog::StellarMark> {
        Vec::new()
    }

    fn crons(&self) -> Vec<nova_sim::catalog::CronRecord> {
        Vec::new()
    }

    fn ranks(&self) -> Vec<nova_sim::catalog::RankRecord> {
        Vec::new()
    }

    fn nebulae(&self) -> Vec<nova_sim::catalog::NebulaRecord> {
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
        flags2: 0,
        destroyed: false,
        graphic: 0,
        dead_graphic: None,
        dead_frame_size: None,
        type_counts_frames: true,
        exit_angle: -1,
        tribute: 0,
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

fn requested(station: bool) -> LandPress {
    LandOutcome::Requested {
        stellar: StellarId(128),
        station,
    }
    .into()
}

#[test]
fn the_first_l_requests_clearance_and_the_second_lands_after_a_tick() {
    let mut session = session(vec![at_centre(StellarFlags::CAN_LAND, 0)]);
    assert_eq!(session.land(), Ok(requested(false)));
    assert_eq!(session.nav_target(), Some(StellarId(128)));
    assert_eq!(session.landed(), None);
    session.tick(Controls::default());
    assert_eq!(
        session.land(),
        Ok(LandOutcome::Landed(StellarId(128)).into())
    );
    assert_eq!(session.landed(), Some(StellarId(128)));
    assert_eq!(session.nav_target(), Some(StellarId(128)), "kept");
}

#[test]
fn a_ship_over_no_stellar_is_too_far() {
    let far = LandingSite {
        position: Vec2::new(0.0, -500.0),
        ..at_centre(STATION, 0)
    };
    let mut session = session(vec![far]);
    assert_eq!(session.land(), Ok(requested(true)));
    session.tick(Controls::default());
    let too_far = Err(LandingRefusal::TooFar {
        stellar: StellarId(128),
        station: true,
    });
    assert_eq!(session.land(), too_far);
    assert_eq!(session.land(), too_far, "the target is kept: try again");
    assert_eq!(session.nav_target(), Some(StellarId(128)));
}

/// Untargeted, with nothing landable, L selects nothing and makes no dock
/// request (@0x6a94c).
#[test]
fn a_stellar_that_cannot_be_landed_on_refuses() {
    let mut session = session(vec![at_centre(0, 0)]);
    assert_eq!(session.land(), Err(LandingRefusal::NothingLandable));
    assert_eq!(session.nav_target(), None, "nothing selected");
}

#[test]
fn a_new_pilot_is_denied_where_a_record_is_needed() {
    let mut session = session(vec![at_centre(StellarFlags::CAN_LAND, 32767)]);
    let denied = Err(LandingRefusal::Denied {
        stellar: StellarId(128),
        station: false,
        min_status: 32767,
    });
    assert_eq!(session.land(), denied);
    session.tick(Controls::default());
    assert_eq!(session.land(), denied, "asked again");
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
    assert_eq!(session.land(), Ok(requested(false)));
    session.tick(thrust);
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
    assert_eq!(
        session.land(),
        Ok(LandOutcome::Landed(StellarId(128)).into())
    );
    assert_eq!(session.player().velocity, Vec2::ZERO);
    assert_eq!(session.player().position, Vec2::ZERO);
    assert_eq!(
        session.take_off(&NoTraffic, &mut NeverFires),
        Some(StellarId(128))
    );
    assert_eq!(session.landed(), None);
}
