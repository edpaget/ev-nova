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

use std::io;
use std::path::Path;
use std::rc::Rc;

use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::fixture::SessionBuilder;
use nova_sim::flight_frame::{Answer, FlightCommand, FlightFrame};
use nova_sim::navigation::NavMode;
use nova_sim::{
    CharacterStart, CombatCatalog, CommodityStrings, DateAffixes, DisasterRecord, GovtRecord,
    HullRecord, JunkRecord, LandingSite, OutfitId, OutfitRecord, PilotCatalog, ShipFields, ShipId,
    StarSystem, StartDate, StartError, StellarId, StellarPick, SystemId, Vec2, WeaponRecord,
};
use nova_sim::{MissionId, TICK};
use nova_view::flight::FlightView;
use nova_view::flight::MapOpener;
use nova_view::flight::sim_port::fixture::check_contract;
use nova_view::flight::{FlightSim, SessionFlight};
use nova_view::galaxy::SystemId as MapSystemId;
use nova_view::{Input, Key, MouseButton, Point, Screen};

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

// The course map and the nav keys, through a whole `FlightView` over a
// real session.

/// One data file, `/data/Nova Data`, holding a fork.
struct OneFile(Vec<u8>);

impl DirLister for OneFile {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        Ok(vec![Listing {
            name: "Nova Data".into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for OneFile {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// The first `chär`: ship 128 from system 128.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// A ship that flies.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300]);
    bytes
}

/// A `sÿst` at map (`x`, 0) with these `Con` links and `NavDefs`.
fn system(x: i16, links: &[i16], nav_defs: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    put_i16s(&mut bytes, 0x24, nav_defs);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// A still planet anyone may land on, at the centre.
fn planet() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 0]);
    bytes[0x06..0x0A].copy_from_slice(&1_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x14, &[-1, 0, -1, -1]);
    bytes
}

/// Alpha (128), at the map's centre, holding Alpha Prime (128), linked
/// both ways to Beta (129), at (100, 0). The first `chär` flies ship 128
/// from Alpha. Nothing is drawn but what the course map needs.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Courier"), &ship())
        .resource(
            System::TYPE,
            128,
            Some(b"Alpha"),
            &system(0, &[129], &[128]),
        )
        .resource(System::TYPE, 129, Some(b"Beta"), &system(100, &[128], &[]))
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &planet())
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// A whole flight view over a real session of [`data`], flying in Alpha.
fn view() -> FlightView<Rc<GameData>> {
    let data = data();
    let session = SessionBuilder::flying(&data).build();
    FlightView::from_session(data, session.into())
}

fn key(key: Key, pressed: bool) -> Input {
    Input::Key {
        key,
        pressed,
        repeat: false,
    }
}

fn tap(view: &mut FlightView<Rc<GameData>>, k: Key) {
    view.input(&key(k, true));
    view.input(&key(k, false));
}

/// A left click at `at`.
fn click(view: &mut FlightView<Rc<GameData>>, at: Point) {
    for pressed in [true, false] {
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// Where the open course map draws system `id`.
fn on_map(view: &FlightView<Rc<GameData>>, id: i16) -> Point {
    let map = view.course_map();
    let system = map.model().system(MapSystemId(id)).expect("on the map");
    map.transform().world_to_screen(system.position())
}

fn nav_mode(view: &FlightView<Rc<GameData>>) -> NavMode {
    view.session().expect("flying").nav_mode()
}

#[test]
fn a_course_plotted_on_the_map_reaches_the_session_and_the_map_draws_it() {
    let mut view = view();
    tap(&mut view, Key::Char('m'));
    assert!(view.map_open());
    let beta = on_map(&view, 129);
    view.input(&key(Key::Shift, true));
    click(&mut view, beta);
    view.input(&key(Key::Shift, false));
    assert_eq!(view.session().expect("flying").course(), [SystemId(129)]);
    assert_eq!(view.course_map().route(), [MapSystemId(129)]);
}

#[test]
fn a_slot_key_reaches_the_session_on_the_next_frame() {
    let mut view = view();
    tap(&mut view, Key::Char('1'));
    assert_eq!(nav_mode(&view), NavMode::Off, "the press waits for a frame");
    view.tick(TICK);
    assert_eq!(nav_mode(&view), NavMode::Stellar(Some(StellarId(128))));
}

#[test]
fn a_mission_dialogs_map_puts_back_the_nav_it_opened_with_as_it_closes() {
    let mut view = view();
    tap(&mut view, Key::Char('1'));
    view.tick(TICK);
    let opened = nav_mode(&view);
    assert_eq!(opened, NavMode::Stellar(Some(StellarId(128))));
    view.open_dialog_map(MapOpener::Offer(MissionId(128)));
    let beta = on_map(&view, 129);
    view.port_map_input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: beta,
    });
    view.port_map_input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at: beta,
    });
    assert_ne!(nav_mode(&view), opened, "the click wrote the nav");
    view.port_map_input(&key(Key::Char('m'), true));
    assert!(!view.map_open());
    assert_eq!(nav_mode(&view), opened, "put back as the map closed");
}
