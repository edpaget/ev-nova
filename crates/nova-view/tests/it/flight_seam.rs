//! The flight view's seam wired to a real session: the [`FlightSim`]
//! port through [`SessionFlight`], over a session a
//! `nova_sim::fixture::SessionBuilder` built.
//!
//! **What belongs here.** Wiring through a real `SessionFlight`: that a
//! command the view sends reaches the session and its answer and frame
//! come back. One to three tests per area the port carries (the
//! contract, the nav keys, and each area later work moves across). A
//! test of what the view does with a frame, an answer or an event is a
//! component test, and runs on a `FakeFlight` in the view's own tests
//! instead.
//!
//! The catalog is built here, in code, so the tests need no `NOVA_DATA`.

use nova_sim::fixture::SessionBuilder;
use nova_sim::flight_frame::{Answer, FlightCommand, FlightFrame};
use nova_sim::navigation::NavMode;
use nova_sim::{
    CharacterStart, CombatCatalog, CommodityStrings, DateAffixes, DisasterRecord, GovtRecord,
    HullRecord, JunkRecord, LandingSite, OutfitId, OutfitRecord, PilotCatalog, ShipFields, ShipId,
    StarSystem, StartDate, StartError, StellarId, StellarPick, SystemId, Vec2, WeaponRecord,
};
use nova_view::flight::sim_port::fixture::check_contract;
use nova_view::flight::{FlightSim, SessionFlight};

/// One `chär` flying from system 130, whose one stellar, 128, is in the
/// nav defaults' first slot.
struct OneStellar;

/// No asteroids anywhere.
impl nova_sim::AsteroidCatalog for OneStellar {
    fn asteroid_types(&self) -> Vec<nova_sim::AsteroidTypeRecord> {
        Vec::new()
    }

    fn asteroid_fields(&self) -> Vec<nova_sim::AsteroidFieldRecord> {
        Vec::new()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; nova_sim::BOX_GRAPHICS] {
        [None; nova_sim::BOX_GRAPHICS]
    }
}

/// Unarmed and ungoverned.
impl CombatCatalog for OneStellar {
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

impl PilotCatalog for OneStellar {
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
            speed: 450,
            accel: 600,
            maneuver: 25,
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

    fn system_exists(&self, _id: SystemId) -> bool {
        true
    }

    /// Stellar 128 alone, in every system.
    fn landing_sites(&self, _system: SystemId) -> Vec<LandingSite> {
        vec![LandingSite {
            id: StellarId(128),
            position: Vec2::new(500.0, 0.0),
            frame_size: Some((100, 100)),
            // `nova_sim::landing::StellarFlags::CAN_LAND`: it can land.
            flags: 0x01,
            min_status: 0,
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
            gravity: 0,
            animation: nova_sim::catalog::SiteAnimation::default(),
        }]
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

    /// System 130's stellar, as it lists it.
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

/// A real flight of [`OneStellar`], flying at the centre of system 130.
fn flight() -> SessionFlight<OneStellar> {
    SessionFlight::from(SessionBuilder::flying(&OneStellar).build())
}

#[test]
fn a_real_session_meets_the_contract() {
    assert_eq!(check_contract(&mut flight()), Ok(()));
}

#[test]
fn a_stellar_key_reaches_the_session_and_its_frame_shows_the_selection() {
    let mut flight = flight();
    let mut frame = FlightFrame::default();
    flight.frame(&mut frame);
    assert_eq!(frame.nav_mode, NavMode::Off);
    assert_eq!(
        flight.apply(FlightCommand::SelectStellar(StellarPick::Slot(0))),
        Answer::Stellar(StellarId(128))
    );
    flight.frame(&mut frame);
    assert_eq!(frame.nav_mode, NavMode::Stellar(Some(StellarId(128))));
}
