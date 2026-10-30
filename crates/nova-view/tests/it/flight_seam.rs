//! The flight view's seam wired to a real session: the [`FlightSim`]
//! port and the edge, through [`SessionFlight`] over a real
//! `nova_sim::Session`
//! (rdm:phase/view-sim-seam/phase-15-integration-test-set).
//!
//! **The rule for adding a test here.** One test per port surface (each
//! `FlightSim` method; each group of the screen query's asks, the maps,
//! the spaceport's, the comm's and the pilot's screens; the opening's
//! names and the frame's wording fields; `DevDesk`; the contract)
//! or per router handoff, never one per rule. Each test:
//!
//! - builds its session through a nova-sim builder
//!   (`nova_sim::fixture::SessionBuilder` and the fixture's setters) or
//!   the load path it tests (`open_flight`), never by stepping a
//!   `Session` by hand;
//! - names the surface it wires in its first doc line.
//!
//! What goes elsewhere: an adapter forward (that `SessionFlight` hands the
//! session an argument or a collaborator, or answers with the session's
//! own) is one of `sim_adapter`'s checks; what the view does with a frame,
//! an answer or an event is a component test on a `FakeFlight` in the
//! view's own tests; what the session decides is nova-sim's.
//!
//! The catalog is built here, in code, so the tests need no `NOVA_DATA`.
//! `stock_flight_seam` is the stock variant.

use std::rc::Rc;

use nova_data::records::character::Character;
use nova_data::records::fixture::ShipBuilder;
use nova_data::records::mission::Mission;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::Rulebook;
use nova_sim::fixture::SessionBuilder;
use nova_sim::flight_frame::{Answer, FlightCommand, FlightFrame};
use nova_sim::navigation::NavMode;
use nova_sim::{
    CharacterStart, CombatCatalog, CommodityStrings, DateAffixes, Direction, DisasterRecord, Good,
    GovtRecord, HullRecord, JunkRecord, LandingSite, Lot, Order, OutfitId, OutfitRecord,
    PilotCatalog, ShipFields, ShipId, StarSystem, StartDate, StartError, StellarId, StellarPick,
    SystemId, Vec2, WeaponRecord,
};
use nova_sim::{MissionId, TICK};
use nova_view::devtools::{PilotDesk, PilotEdit};
use nova_view::flight::FlightView;
use nova_view::flight::MapOpener;
use nova_view::flight::sim_port::fixture::check_contract;
use nova_view::flight::{Collaborators, FlightSim, FlightStart, SessionFlight, open_flight};
use nova_view::galaxy::SystemId as MapSystemId;
use nova_view::{Diagnostic, Input, Key, MouseButton, Point, Screen, Sound};

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
            dominated_at_run_time: false,
            graphic: 0,
            dead_graphic: None,
            dead_frame_size: None,
            type_counts_frames: true,
            dead_counts_frames: true,
            exit_angle: -1,
            tribute: 0,
            gravity: 0,
            fee: 0,
            animation: nova_sim::catalog::SiteAnimation::default(),
            defense_dude: None,
            def_count: 0,
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

    fn stellar_weapons(&self) -> Vec<(StellarId, nova_sim::WeaponId)> {
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

    fn stellar_scripts(&self) -> Vec<nova_sim::StellarScripts> {
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

/// The frame `flight` fills now.
fn frame_of<C>(flight: &SessionFlight<C>) -> FlightFrame {
    let mut frame = FlightFrame::default();
    flight.frame(&mut frame);
    frame
}

/// Checks the port's contract on a copy of `flight` in `state`, as the
/// check sends commands.
fn meets_the_contract<C>(flight: &SessionFlight<C>, state: &str) {
    assert_eq!(
        check_contract(&mut flight.clone(), &flight.opening()),
        Ok(()),
        "{state}"
    );
}

/// The contract on `SessionFlight`: a real session meets the port's
/// contract flying, with outputs pending, landed, with an escort flying,
/// holding a target and after letting it go (phase 8's review, tests-1).
#[test]
fn a_real_session_meets_the_contract_in_each_state() {
    let mut flying = flight();
    meets_the_contract(&flying, "flying");
    assert_eq!(
        flying.apply(FlightCommand::SelectStellar(StellarPick::Slot(0))),
        Answer::Stellar(StellarId(128))
    );
    meets_the_contract(&flying, "with outputs pending");
    let landed = SessionFlight::<OneStellar>::from(
        SessionBuilder::flying(&OneStellar)
            .landed_at(SystemId(130), StellarId(128))
            .build(),
    );
    assert_eq!(frame_of(&landed).landed, Some(StellarId(128)), "docked");
    meets_the_contract(&landed, "landed");
    let data = fighting_data(&[]);
    let escort = nova_sim::fleet::Escort {
        ship: ShipId(129),
        reserves: nova_sim::Reserves::full(10.0, 10.0, 100.0),
        order: None,
        carried: false,
        wage: Some(10),
        person: None,
        mark: None,
    };
    let unplaced = SessionFlight::<Rc<GameData>>::from(
        SessionBuilder::flying(&data)
            .with_escorts(vec![escort])
            .build(),
    );
    assert!(
        !frame_of(&unplaced).hud.escorts_flying,
        "not in the system before the first traffic tick"
    );
    let sim = SessionFlight::from(
        SessionBuilder::flying(&data)
            .with_escorts(vec![escort])
            .build(),
    )
    .with_collaborators(Collaborators {
        chance: drawing(&TRADER_ABOVE),
        behaviour: Rc::new(Idle),
        ..Collaborators::default()
    });
    let mut view = FlightView::from_session(Rc::clone(&data), sim, &Rulebook::default());
    // The first step sets the system up, the escort placed in it.
    view.tick(TICK);
    let escorted = view.sim();
    let frame = frame_of(escorted);
    assert!(frame.hud.escorts_flying, "the hired escort, placed");
    assert!(frame.hud.escort_rows.iter().any(|row| row.present));
    meets_the_contract(escorted, "with an escort flying");
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(data, session);
    tap(&mut view, Key::Tab);
    assert!(
        frame_of(view.sim()).target.is_some(),
        "Tab targets the trader"
    );
    meets_the_contract(view.sim(), "holding a target");
    // Option-` lets go of the target on the next frame.
    view.input(&key(Key::Alt, true));
    view.input(&key(Key::Char('`'), true));
    view.tick(TICK);
    view.input(&key(Key::Char('`'), false));
    view.input(&key(Key::Alt, false));
    assert!(frame_of(view.sim()).target.is_none(), "let go");
    meets_the_contract(view.sim(), "after the target is let go");
}

/// `FlightSim::apply` then `frame`, at the adapter: a stellar key's
/// command reaches the session and the next frame shows the selection.
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
    ShipBuilder::new()
        .shield(30)
        .accel(300)
        .speed(300)
        .maneuver(30)
        .fuel(300)
        .build()
}

/// A `sÿst` at map (`x`, 0) with these `Con` links and `NavDefs`.
fn system(x: i16, links: &[i16], nav_defs: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    put_i16s(&mut bytes, 0x24, nav_defs);
    // No government, and no message buoy.
    put_i16s(&mut bytes, 0x66, &[-1, -1]);
    bytes
}

/// [`system`] with no `NavDefs`, visible only while `visibility` (a
/// control-bit test) holds.
fn hidden_system(x: i16, links: &[i16], visibility: &str) -> Vec<u8> {
    let mut bytes = system(x, links, &[]);
    bytes[0x96..0x96 + visibility.len()].copy_from_slice(visibility.as_bytes());
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
/// both ways to Beta (129), at (100, 0), visible only while bit 1 is
/// clear (`!b1`). The first `chär` flies ship 128 from Alpha. Nothing is
/// drawn but what the course map needs.
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
        .resource(
            System::TYPE,
            129,
            Some(b"Beta"),
            &hidden_system(100, &[128], "!b1"),
        )
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &planet())
        .build()
        .bytes;
    Rc::new(one_file(fork).load())
}

/// A whole flight view over a real session of [`data`], flying in Alpha.
fn view() -> FlightView<Rc<GameData>> {
    let data = data();
    let session = SessionBuilder::flying(&data).build();
    FlightView::from_session(data, session.into(), &Rulebook::default())
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

/// `FlightSim::screen`, the course map: a Shift-click on the course map
/// plots a course the session holds and the map draws.
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

/// `FlightSim::apply` from the view's keys: a slot key's press reaches
/// the session at the next frame's step.
#[test]
fn a_slot_key_reaches_the_session_on_the_next_frame() {
    let mut view = view();
    tap(&mut view, Key::Char('1'));
    assert_eq!(nav_mode(&view), NavMode::Off, "the press waits for a frame");
    view.tick(TICK);
    assert_eq!(nav_mode(&view), NavMode::Stellar(Some(StellarId(128))));
}

/// The router's planet comm handoff: Y on a navigation stellar that
/// answers opens the planet comm dialog over a real session, held for the
/// router, and Close Channel closes it.
#[test]
fn a_hailed_stellar_opens_the_planet_comm_dialog_and_close_channel_closes_it() {
    use nova_sim::planet_comm::PlanetCommItem;
    let mut view = view();
    tap(&mut view, Key::Char('1'));
    view.tick(TICK);
    tap(&mut view, Key::Char('y'));
    let opened = view.take_planet_comm().expect("the planet comm dialog");
    assert_eq!(opened.stellar, StellarId(128));
    assert_eq!(opened.name, "Alpha Prime");
    assert_eq!(view.press_planet_comm(PlanetCommItem::Close), None);
    assert_eq!(
        view.press_planet_comm(PlanetCommItem::Second),
        None,
        "closed: nothing to press"
    );
}

/// `FlightSim::screen`, a dialog's map: a mission dialog's map writes the
/// session's nav, and puts back the nav it opened with as it closes.
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

// The one-shot outputs, drained through a whole `FlightView` over a real
// session.

/// Presses L until the ship lands, as the view's own tests do: the
/// request, a frame, then the landing.
fn land(view: &mut FlightView<Rc<GameData>>) {
    tap(view, Key::Char('l'));
    view.tick(TICK);
    tap(view, Key::Char('l'));
}

/// `FlightSim::drain`: a landing request's sound and line, and the
/// landing's save due, come back through the drain.
#[test]
fn a_real_landing_plays_its_sound_and_shows_its_line_through_the_drain() {
    let mut view = view();
    tap(&mut view, Key::Char('l'));
    view.tick(TICK);
    assert_eq!(
        view.message(),
        Some("Cleared to land, Courier. Commence final approach."),
        "the request's line, from the session"
    );
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(nova_sim::SimSound::TrafficControl)]
    );
    tap(&mut view, Key::Char('l'));
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    assert!(view.take_save_due(), "the landing's save");
    assert!(!view.take_save_due(), "once");
}

// The jump, the landing and the take-off, through a whole `FlightView`
// over a real session (rdm:phase/view-sim-seam/phase-9-travel-on-port).

/// `FlightSim::step`, the warp: a jump stepped through the port arrives
/// in the next system, which the map and the arrival line show.
#[test]
fn a_real_jump_arrives_in_the_next_system() {
    let mut view = view();
    tap(&mut view, Key::Char('m'));
    let beta = on_map(&view, 129);
    view.input(&key(Key::Shift, true));
    click(&mut view, beta);
    view.input(&key(Key::Shift, false));
    tap(&mut view, Key::Char('m'));
    assert_eq!(view.session().expect("flying").course(), [SystemId(129)]);
    view.input(&key(Key::Up, true));
    for _ in 0..3000 {
        let at = view.session().expect("flying").player().position;
        if at.length() >= nova_sim::hyperspace::MIN_JUMP_DISTANCE {
            break;
        }
        view.tick(TICK);
    }
    view.input(&key(Key::Up, false));
    tap(&mut view, Key::Char('j'));
    for _ in 0..3000 {
        if view.session().expect("flying").system() == SystemId(129) {
            break;
        }
        view.tick(TICK);
    }
    assert_eq!(view.session().expect("flying").system(), SystemId(129));
    assert_eq!(view.course_map().current(), Some(MapSystemId(129)));
    let line = view.message().expect("the arrival line");
    assert!(line.starts_with("Jumping into the Beta system"), "{line:?}");
    let date = view.session().expect("flying").long_date_text();
    assert!(line.contains(&date), "on the session's date: {line:?}");
}

/// The router's landing, leave, land-escorts and take-off handoffs: a
/// landing hands the router the stellar, with no leave held and no escort
/// to settle, and a take-off gives it back.
#[test]
fn a_real_landing_hands_the_router_the_stellar_and_a_take_off_gives_it_back() {
    let mut view = view();
    land(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    assert!(!view.leave_due(), "no held leave");
    assert_eq!(view.land_escorts(), None, "no escort to settle");
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.session().expect("flying").landed(), None, "in flight");
    assert_eq!(view.take_off(), None, "not landed");
}

/// The systems the open course map draws.
fn drawn(view: &FlightView<Rc<GameData>>) -> Vec<SystemId> {
    view.course_map()
        .picture()
        .map(|picture| picture.systems.iter().map(|system| system.id).collect())
        .unwrap_or_default()
}

/// `FlightSim::drain` and `FlightSim::screen`'s galaxy: a galaxy change
/// the session raises redraws the open map from the galaxy the flight
/// answers.
#[test]
fn a_galaxy_change_redraws_the_open_map_through_the_event_and_the_galaxy() {
    let mut view = view();
    land(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    view.open_bar_map();
    assert!(drawn(&view).contains(&SystemId(129)), "Beta shown");
    let mut desk = view.pilot_desk().expect("flying");
    desk.edit(PilotEdit::Bit(nova_sim::Bit::new(1).expect("a bit"), true))
        .expect("set");
    assert!(drawn(&view).contains(&SystemId(129)), "until a refresh");
    // The take-off's refresh hides Beta, and tells the galaxy changed.
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert!(view.map_open(), "still open");
    assert!(
        view.course_map().model().system(MapSystemId(129)).is_none(),
        "the galaxy the flight answered"
    );
    assert!(
        !drawn(&view).contains(&SystemId(129)),
        "the open map redrawn"
    );
}

/// The edge, `open_flight`: a flight built at the edge opens on its
/// start, and its session is set up with what the edge was given after
/// the view read it: the rules, which the view shows by too.
#[test]
fn a_flight_built_at_the_edge_opens_on_its_start_and_reads_its_setup() {
    let rules = Rulebook::default()
        .with_override(nova_sim::RuleKey::PayMillions, nova_sim::RuleSource::Bible);
    let view = open_flight(
        data(),
        FlightStart::New,
        nova_sim::SessionSetup {
            rules,
            ..nova_sim::SessionSetup::default()
        },
        Collaborators::default(),
    );
    let session = view.session().expect("started");
    assert_eq!(session.system(), SystemId(128));
    assert_eq!(session.rules(), &rules, "set up");
    assert_eq!(
        view.scene().map(nova_view::system::SystemScene::id),
        Some(SystemId(128)),
        "the scene it opens on"
    );
    let at = session.player().position;
    assert_eq!(
        view.shown_position(),
        Point::new(at.x, at.y),
        "drawn at rest where it starts"
    );
}

/// The edge, `begin_game`: entering play on an edge-built flight, the
/// adapter begins the session and the view draws the ship where the
/// beginning put it.
#[test]
fn begin_on_an_edge_built_flight_places_the_ship_where_the_session_began_it() {
    let mut view = open_flight(
        data(),
        FlightStart::New,
        nova_sim::SessionSetup::default(),
        Collaborators::default(),
    );
    nova_view::flight::begin_game(&mut view);
    let player = view.session().expect("started").player().position;
    assert_eq!(player, nova_sim::Vec2::new(50.0, 50.0));
    assert_eq!(view.shown_position(), Point::new(50.0, 50.0));
}

/// `DevDesk`: the developer tools' desk on a real flight reads the
/// session, the view names the places from its course map, an edit
/// reaches the session, and a resync after it catches the view up.
#[test]
fn a_real_desk_reads_the_session_and_its_edit_reaches_it_before_a_resync() {
    let mut view = view();
    land(&mut view);
    let mut desk = view.pilot_desk().expect("flying");
    let sheet = desk.sheet().expect("a pilot");
    assert_eq!(sheet.system.name, "Alpha");
    assert_eq!(sheet.landed.map(|place| place.id), Some(StellarId(128)));
    desk.edit(PilotEdit::Credits(77)).expect("set");
    assert_eq!(desk.sheet().expect("a pilot").credits, 77);
    view.resync();
    assert_eq!(view.session().expect("flying").pilot().cash(), 77);
    assert!(view.take_save_due(), "an edit makes a save due");
}

// The fixed steps, through a whole `FlightView` over a real session.

/// Draws its script, then the last outcome, so no roll fires.
struct Script(std::collections::VecDeque<u32>);

impl nova_sim::Chance for Script {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.pop_front().unwrap_or(n - 1)
    }
}

/// A shared chance drawing `draws`, then each last outcome.
fn drawing(draws: &[u32]) -> nova_view::flight::SharedChance {
    let script: Rc<std::cell::RefCell<dyn nova_sim::Chance>> = Rc::new(std::cell::RefCell::new(
        Script(draws.iter().copied().collect()),
    ));
    nova_view::flight::SharedChance::new(script)
}

/// Every NPC idles.
#[derive(Debug)]
struct Idle;

impl nova_sim::Behaviour for Idle {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn nova_sim::Chance,
    ) -> nova_sim::Goal {
        nova_sim::Goal::Idle
    }
}

/// A `shïp` of this `Shield` and `Armor`, carrying `weapons` (one of
/// each) and `items` (each with its count), destroyed at once in
/// explosion type 0, `bööm` 128.
fn armed_ship(shield: i16, armor: i16, weapons: &[i16], items: &[(i16, i16)]) -> Vec<u8> {
    let (ids, counts): (Vec<i16>, Vec<i16>) = items.iter().copied().unzip();
    ShipBuilder::new()
        .shield(shield)
        .accel(300)
        .speed(300)
        .maneuver(30)
        .fuel(300)
        .armor(armor)
        .weap_types(&[-1; 8])
        .weap_types(weapons)
        .weap_counts(&[1; 4])
        .explode1(-1)
        .inherent_ai(1)
        .default_items(&[-1; 8])
        .default_items(&ids)
        .item_counts(&counts)
        .build()
}

/// The blaster, `wëap` 128: unguided, firing every tick, 20 pixels a
/// tick for 30 ticks, doing 5 mass damage.
fn blaster() -> Vec<u8> {
    let mut bytes = vec![0; nova_data::records::weapon::Weapon::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 30, 5, 0, -1, 2000, -1, 0, 0, -1]);
    put_i16s(&mut bytes, 0x16, &[-1]);
    bytes
}

/// An `oütf` of `ModType` `mod_type` and `ModVal` `value`, one at most.
fn fitting(mod_type: i16, value: i16) -> Vec<u8> {
    let mut bytes = vec![0; nova_data::records::outfit::Outfit::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x06, &[mod_type, value, 1]);
    bytes
}

/// An independent `sÿst` with no stellars, with `düde` 128 at 100 % and
/// one ship on average.
fn trafficked_system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[-1; 16]);
    put_i16s(&mut bytes, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
    put_i16s(&mut bytes, 0x54, &[100]);
    put_i16s(&mut bytes, 0x64, &[1, -1]);
    bytes
}

/// A `düde` of wimpy traders flying ship 129, independent.
fn traders() -> Vec<u8> {
    let mut bytes = vec![0; nova_data::records::dude::Dude::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1, -1]);
    put_i16s(&mut bytes, 0x08, &[-1; 16]);
    put_i16s(&mut bytes, 0x08, &[129]);
    put_i16s(&mut bytes, 0x28, &[100]);
    bytes
}

/// `ïntf` 128, the status bar: the stock bar's areas, in plain colours,
/// in Geneva 12, over `PICT` 700, which the data doesn't hold.
fn interface() -> Vec<u8> {
    let mut bytes = vec![0; nova_data::records::interface::Interface::SIZE.expect("fixed")];
    let colour = |bytes: &mut [u8], at: usize, rgb: u32| {
        bytes[at..at + 4].copy_from_slice(&rgb.to_be_bytes());
    };
    for (at, rgb) in [
        (0x00, 0x00FF_FFFF),
        (0x04, 0x0080_8080),
        (0x10, 0x0000_FF00),
        (0x14, 0x0000_8000),
        (0x20, 0x0000_00FF),
        (0x2C, 0x00FF_0000),
        (0x38, 0x00FF_FF00),
        (0x3C, 0x0080_8000),
    ] {
        colour(&mut bytes, at, rgb);
    }
    for (at, rect) in [
        (0x08, [8, 8, 184, 184]),
        (0x18, [199, 35, 206, 184]),
        (0x24, [216, 35, 223, 184]),
        (0x30, [234, 35, 241, 184]),
        (0x40, [250, 8, 330, 184]),
        (0x48, [340, 8, 360, 184]),
        (0x50, [370, 8, 450, 184]),
        (0x58, [458, 8, 552, 184]),
    ] {
        put_i16s(&mut bytes, at, &rect);
    }
    bytes[0x60..0x67].copy_from_slice(b"Geneva\0");
    put_i16s(&mut bytes, 0xA0, &[12, 10, 700]);
    bytes
}

/// The first `chär` flies the "Gunship" (ship 128, 30 shield, 45 armour,
/// carrying the [`blaster`], held by `oütf` 228, and `items`) from Alpha
/// (128), whose one `düde` flies the "Trader" (ship 129: no shield, 10
/// armour, no speed, a crew of 3). The escape pod is ship 895; `oütf` 188 is an
/// escape pod (`ModType` 11) and 187 an auto-eject (20). The status bar
/// is [`interface`]'s, and the mouse cursor `spïn` 650 is [`cursor`]'s.
fn fighting_data(items: &[(i16, i16)]) -> Rc<GameData> {
    let mut trader = armed_ship(0, 10, &[], &[]);
    put_i16s(&mut trader, 0x06, &[0]);
    // A crew of 3, so it can be boarded.
    put_i16s(&mut trader, 0x44, &[3]);
    let mut holder = fitting(1, 128);
    put_i16s(&mut holder, 0x0A, &[10]);
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(
            Ship::TYPE,
            128,
            Some(b"Gunship"),
            &armed_ship(30, 45, &[128], items),
        )
        .resource(Ship::TYPE, 129, Some(b"Trader;merchant"), &trader)
        .resource(Ship::TYPE, 895, Some(b"Escape Pod"), &ship())
        .resource(
            nova_data::records::weapon::Weapon::TYPE,
            128,
            Some(b"Blaster"),
            &blaster(),
        )
        .resource(nova_data::records::outfit::Outfit::TYPE, 228, None, &holder)
        .resource(
            nova_data::records::outfit::Outfit::TYPE,
            188,
            Some(b"Escape Pod"),
            &fitting(11, 0),
        )
        .resource(
            nova_data::records::outfit::Outfit::TYPE,
            187,
            Some(b"Auto-Eject"),
            &fitting(20, 0),
        )
        .resource(System::TYPE, 128, Some(b"Alpha"), &trafficked_system())
        .resource(
            nova_data::records::interface::Interface::TYPE,
            128,
            None,
            &interface(),
        )
        .resource(
            nova_data::records::dude::Dude::TYPE,
            128,
            Some(b"Traders"),
            &traders(),
        )
        .resource(
            nova_data::records::spin::Spin::TYPE,
            650,
            None,
            &cursor_spin(),
        )
        .resource(nova_data::graphics::RLED, CURSOR, None, &cursor())
        .build()
        .bytes;
    Rc::new(one_file(fork).load())
}

/// The mouse cursor's `rlëD`.
const CURSOR: i16 = 651;

/// `spïn` 650, the mouse cursor: [`cursor`]'s four frames in a row.
fn cursor_spin() -> Vec<u8> {
    let mut bytes = vec![0; nova_data::records::spin::Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[CURSOR, -1, 3, 2, 4, 1]);
    bytes
}

/// The mouse cursor's sheet: four 3 x 2 frames.
fn cursor() -> Vec<u8> {
    (0..4)
        .fold(
            nova_data::graphics::fixture::RledBuilder::new(3, 2),
            |b, _| b.frame(|f| f.line().pixels(&[0x7C00; 3]).line().pixels(&[0x7C00; 3])),
        )
        .build()
}

/// The draws that place the trader 100 pixels above the player.
const TRADER_ABOVE: [u32; 8] = [6, 6, 0, 0, 750, 650, 180, 2];

/// A whole flight view over a real session of `data`, its chance
/// placing the trader 100 pixels above the player, idling.
fn fighting(data: Rc<GameData>, session: nova_sim::Session) -> FlightView<Rc<GameData>> {
    fighting_with(
        data,
        session,
        Collaborators {
            chance: drawing(&TRADER_ABOVE),
            behaviour: Rc::new(Idle),
            ..Collaborators::default()
        },
    )
}

/// [`fighting`], its flight handed `collaborators`, whose chance places
/// the trader.
fn fighting_with(
    data: Rc<GameData>,
    session: nova_sim::Session,
    collaborators: Collaborators,
) -> FlightView<Rc<GameData>> {
    let sim = SessionFlight::from(session).with_collaborators(collaborators);
    let mut view = FlightView::from_session(data, sim, &Rulebook::default());
    // The first step sets the system up.
    view.tick(TICK);
    assert_eq!(view.session().expect("flying").npcs().len(), 1);
    view
}

/// `FlightOpening::weapons`: the looks of a real flight's weapons are read as
/// the view opens, so the blaster's unreadable shot sheet is reported.
#[test]
fn a_real_flights_weapon_looks_are_read_as_the_view_opens() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view =
        FlightView::from_session(data, SessionFlight::from(session), &Rulebook::default());
    let diagnostics = view.take_diagnostics();
    assert!(
        diagnostics.contains(&Diagnostic::Unreadable("wëap 128: no spïn 3000".to_owned())),
        "{diagnostics:?}"
    );
}

/// `FlightSim::step`'s report: a real fight's hit comes back in the step
/// report, and the view shows its debris.
#[test]
fn a_real_fights_hit_shows_its_effect() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(data, session);
    view.input(&key(Key::Space, true));
    for _ in 0..30 {
        view.tick(TICK);
        if !view.effects().particles().is_empty() {
            assert!(
                view.session().expect("flying").npcs().is_empty(),
                "the trader destroyed"
            );
            return;
        }
    }
    panic!(
        "no debris shown: {:?}",
        view.session().expect("flying").npcs()[0].reserves
    );
}

/// `FlightSim::step`'s stop and the router's rescue desc: a rescue in the
/// pod stops the stepping, and its desc reaches the router.
#[test]
fn a_rescue_in_the_pod_stops_the_step_and_opens_its_desc() {
    let data = fighting_data(&[(188, 1), (187, 1)]);
    let mut session = SessionBuilder::flying(&data).build();
    nova_sim::fixture::set_player_condition(
        &mut session,
        nova_sim::Condition::Dying {
            ticks_left: 0,
            starting: false,
        },
    );
    let mut view = fighting(data, session);
    let mut frames = 0;
    let desc = loop {
        // A long frame, many steps: the rescue's ends them.
        view.tick(TICK * 30);
        if let Some(desc) = view.take_rescue_desc() {
            break desc;
        }
        frames += 1;
        assert!(frames <= 20, "no rescue");
    };
    assert_eq!(desc, 13999);
    let session = view.session().expect("flying");
    assert_eq!(session.ship(), ShipId(128), "out of the pod");
    assert_eq!(session.player().position, Vec2::new(50.0, 50.0), "rescued");
    assert_eq!(
        view.camera().center(),
        Point::new(50.0, 50.0),
        "drawn where the rescue put it: nothing stepped after it"
    );
}

// The world drawn from the frame (rdm:phase/view-sim-seam/phase-7-world-frame).

/// The centre of each crossed box and the centre of each sprite in
/// `list`: where the world's placeholders and sprites are drawn.
fn centres(list: &nova_view::DrawList) -> Vec<Point> {
    list.iter()
        .filter_map(|command| match *command {
            nova_view::DrawCommand::Sprite { center, .. } => Some(center),
            nova_view::DrawCommand::Line { from, to, .. } => Some(Point::new(
                f32::midpoint(from.x, to.x),
                f32::midpoint(from.y, to.y),
            )),
            _ => None,
        })
        .collect()
}

/// Whether one of `centres` is within a hundredth of `at`.
fn near(centres: &[Point], at: Point) -> bool {
    centres
        .iter()
        .any(|c| (c.x - at.x).abs() < 0.01 && (c.y - at.y).abs() < 0.01)
}

/// `FlightSim::frame`, the world: a real fight's ship and shot are drawn
/// where the frame puts them.
#[test]
fn a_real_fights_ship_and_shot_are_drawn_where_its_frame_puts_them() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(data, session);
    view.input(&key(Key::Space, true));
    let mut frame = FlightFrame::default();
    for _ in 0..30 {
        view.tick(TICK);
        view.session().expect("flying").fill_frame(&mut frame);
        if !frame.shots.is_empty() {
            break;
        }
    }
    assert!(!frame.shots.is_empty(), "a shot in flight");
    let mut list = nova_view::DrawList::new();
    view.draw(&mut list);
    let drawn = centres(&list);
    let camera = view.camera();
    let behind = 1.0 - view.alpha();
    // The trader idles where the frame has it: its crossed box there.
    let trader = frame.ships[1].position;
    assert!(
        near(
            &drawn,
            camera.world_to_screen(Point::new(trader.x, trader.y))
        ),
        "the trader at {trader:?}: {drawn:?}"
    );
    // The shot smoothed back from where the frame has it by its motion.
    let shot = frame.shots[0];
    let at = Point::new(
        shot.velocity.x.mul_add(-behind, shot.position.x),
        shot.velocity.y.mul_add(-behind, shot.position.y),
    );
    assert!(
        near(&drawn, camera.world_to_screen(at)),
        "the shot at {at:?}: {drawn:?}"
    );
}

/// `FlightSim::apply` through `ClickInSpace`: a click over the box the
/// view draws a ship in targets it.
#[test]
fn a_real_click_in_space_targets_the_ship_drawn_under_it() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(data, session);
    let mut frame = FlightFrame::default();
    view.session().expect("flying").fill_frame(&mut frame);
    let trader = frame.ships[1];
    let at = view
        .camera()
        .world_to_screen(Point::new(trader.position.x, trader.position.y));
    view.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    let session = view.session().expect("flying");
    assert_eq!(
        session.target().map(|npc| nova_sim::ShipRef::Npc(npc.id)),
        Some(trader.ship),
        "targeted through the frame's box"
    );
}

// The HUD and the target panel drawn from the frame
// (rdm:phase/view-sim-seam/phase-8-hud-and-target-frame).

/// The lines `view`'s status bar draws in `color`, alone, as the port's
/// bar draws it.
fn bar_lines(view: &FlightView<Rc<GameData>>, color: nova_view::Color) -> Vec<(f32, f32)> {
    let mut list = nova_view::DrawList::new();
    view.draw_status_bar(&mut list);
    list.iter()
        .filter_map(|command| match *command {
            nova_view::DrawCommand::Line {
                from, to, color: c, ..
            } if c == color => Some((from.x, to.x)),
            _ => None,
        })
        .collect()
}

/// The texts `list` draws.
fn texts(list: &nova_view::DrawList) -> Vec<String> {
    list.iter()
        .filter_map(|command| match command {
            nova_view::DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// `FlightSim::frame`, the HUD: the status bar shows the session's
/// reserves and credits.
#[test]
fn a_real_sessions_status_bar_shows_its_reserves_and_cargo() {
    let data = fighting_data(&[]);
    let full = SessionBuilder::flying(&data).with_credits(12_345).build();
    let view = FlightView::from_session(Rc::clone(&data), full.into(), &Rulebook::default());
    let layout = view.status_bar().expect("ïntf 128").layout.clone();
    for color in [layout.shield_color, layout.armor_color, layout.fuel_full] {
        assert_eq!(bar_lines(&view, color).len(), 1, "a gauge in {color:?}");
    }
    let full_fuel = bar_lines(&view, layout.fuel_full)[0];
    let mut bar = nova_view::DrawList::new();
    view.draw_status_bar(&mut bar);
    assert!(
        texts(&bar).contains(&"12,345".to_owned()),
        "the cargo area's credits: {:?}",
        texts(&bar)
    );
    let low = SessionBuilder::flying(&data)
        .with_reserve(nova_sim::Reserve::Fuel, 50.0)
        .build();
    let view = FlightView::from_session(data, low.into(), &Rulebook::default());
    let low_fuel = bar_lines(&view, layout.fuel_full);
    assert!(
        low_fuel.len() == 1 && low_fuel[0].1 < full_fuel.1,
        "less fuel drawn: {low_fuel:?} of {full_fuel:?}"
    );
}

/// `FlightSim::frame`, the target: Tab's target shows in the target panel
/// and its brackets.
#[test]
fn a_real_target_shows_in_the_panel_and_brackets() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(data, session);
    tap(&mut view, Key::Tab);
    let mut frame = FlightFrame::default();
    view.session().expect("flying").fill_frame(&mut frame);
    let target = frame.target.expect("Tab targets the trader");
    assert_eq!(nova_sim::ShipRef::Npc(target.npc), frame.ships[1].ship);
    assert_eq!(target.status, nova_sim::TargetStatus::NoShields);
    assert_eq!(
        frame.hud.target_brackets.map(|brackets| brackets.npc),
        Some(target.npc)
    );
    let mut list = nova_view::DrawList::new();
    view.draw(&mut list);
    let drawn = texts(&list);
    assert!(drawn.contains(&target.name), "{} in {drawn:?}", target.name);
    assert!(drawn.contains(&"No Shields".to_owned()), "{drawn:?}");
    let brackets = list
        .iter()
        .filter(|command| {
            matches!(command, nova_view::DrawCommand::Picture { image, .. }
                if image.kind == nova_view::ImageKind::Icon
                    && (10008..=10023).contains(&image.id))
        })
        .count();
    assert_eq!(brackets, 4, "its four corners");
}

/// `FlightFrame::gates`: M opens the map while the session takes its gate,
/// and a gate the session refuses keeps the view's key from acting, as a
/// player breaking up can't open the map.
#[test]
fn a_gate_the_session_refuses_keeps_the_views_key_from_acting() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(data, session);
    tap(&mut view, Key::Char('m'));
    assert!(
        view.map_open(),
        "control: M opens the map while the session takes the gate"
    );
    view.close_map();
    assert!(!view.map_open());
    let takes_the_map = view.change_sim(|flight| {
        let session = flight.session_mut().expect("flying");
        nova_sim::fixture::set_player_condition(
            session,
            nova_sim::Condition::Dying {
                ticks_left: 100,
                starting: false,
            },
        );
        session.takes(nova_sim::PlayerCommand::Map)
    });
    assert!(!takes_the_map, "precondition: the session refuses the map");
    tap(&mut view, Key::Char('m'));
    assert!(!view.map_open(), "the map key refused");
}

/// The frame of the mouse cursor `view` draws, if it draws one.
fn cursor_frame(view: &FlightView<Rc<GameData>>) -> Option<u16> {
    let mut list = nova_view::DrawList::new();
    view.draw(&mut list);
    list.iter().find_map(|command| {
        let mut command = command;
        while let nova_view::DrawCommand::Clipped { command: inner, .. } = command {
            command = inner;
        }
        match command {
            nova_view::DrawCommand::Sprite { image, .. } if image.id == CURSOR => Some(image.frame),
            _ => None,
        }
    })
}

/// `FlightCommand::Hover`: with the pointer over the trader the frame draws,
/// the session hovers it and the view's cursor animates; off it, the
/// cursor rests on its first frame.
#[test]
fn the_pointer_over_a_real_ship_hovers_it_and_the_cursor_shows_it() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(data, session);
    let trader = frame_of(view.sim()).ships[1].position;
    let over = view
        .camera()
        .world_to_screen(Point::new(trader.x, trader.y));
    view.input(&Input::PointerMoved(over));
    for _ in 0..4 {
        view.tick(TICK);
    }
    assert_ne!(cursor_frame(&view), Some(0), "animating over the trader");
    assert!(cursor_frame(&view).is_some(), "the cursor drawn");
    view.input(&Input::PointerMoved(Point::new(100.0, 600.0)));
    view.tick(TICK);
    assert_eq!(cursor_frame(&view), Some(0), "at rest off it");
}

// The comm and the boarding through the port
// (rdm:phase/view-sim-seam/phase-10-comm-escorts-boarding-on-port).

/// The hail's screen ask and the router's hail: a hail sent through the port opens
/// the comm dialog on the session's hail view, and a hang-up closes it.
#[test]
fn a_real_hail_opens_the_comm_dialog_with_the_sessions_hail_view() {
    let data = fighting_data(&[]);
    let session = SessionBuilder::flying(&data).build();
    let mut view = fighting(Rc::clone(&data), session);
    tap(&mut view, Key::Tab);
    tap(&mut view, Key::Char('y'));
    let hail = view.take_hail().expect("the trader answers");
    let session = view.session().expect("flying");
    assert_eq!(
        Some(hail),
        session.hailing(&data, &nova_sim::HailOptions::default()),
        "the session's own, with Nova's options"
    );
    assert_eq!(
        view.hailing(),
        view.session()
            .expect("flying")
            .hailing(&data, &nova_sim::HailOptions::default())
    );
    view.hang_up();
    assert_eq!(view.hailing(), None, "hung up through the port");
}

/// Records each crime, and changes nothing.
#[derive(Debug, Default)]
struct Witness {
    seen: std::cell::RefCell<Vec<nova_sim::Crime>>,
}

impl nova_sim::LegalCode for Witness {
    fn penalties(
        &self,
        crime: nova_sim::Crime,
        _victim: Option<nova_sim::GovtId>,
        _govts: &nova_sim::Governments,
    ) -> Vec<(nova_sim::GovtId, i32)> {
        self.seen.borrow_mut().push(crime);
        Vec::new()
    }

    fn system_penalties(
        &self,
        _crime: nova_sim::Crime,
        _victim: Option<nova_sim::GovtId>,
        _from: SystemId,
        _map: &dyn nova_sim::CrimeMap,
        _govts: &nova_sim::Governments,
    ) -> Vec<(SystemId, f32)> {
        Vec::new()
    }
}

/// Nova's boarding rules, noting each time it is asked whether a ship
/// repels its boarders.
#[derive(Debug, Default)]
struct Recorder {
    nova: nova_sim::NovaBoarding,
    repels: std::cell::Cell<usize>,
}

impl nova_sim::BoardingRule for Recorder {
    fn repels(&self, booty: u16) -> bool {
        self.repels.set(self.repels.get() + 1);
        self.nova.repels(booty)
    }

    fn capture_odds(
        &self,
        crew: &nova_sim::board::CaptureCrew,
        chance: &mut dyn nova_sim::Chance,
    ) -> u8 {
        self.nova.capture_odds(crew, chance)
    }

    fn captures(&self, odds: u8, chance: &mut dyn nova_sim::Chance) -> bool {
        self.nova.captures(odds, chance)
    }

    fn person_credits(&self, credits: i32, chance: &mut dyn nova_sim::Chance) -> i64 {
        self.nova.person_credits(credits, chance)
    }

    fn grant(
        &self,
        grant: &nova_sim::PersonGrant,
        stock: &[nova_sim::GrantStock],
        free_mass: i64,
        grant_max: nova_sim::RuleSource,
        chance: &mut dyn nova_sim::Chance,
    ) -> Option<nova_sim::Granted> {
        self.nova.grant(grant, stock, free_mass, grant_max, chance)
    }
}

/// The boarding's screen ask and `FlightSim::apply`'s collaborators: a board sent
/// through the port is judged by the law and boarding rule the view was
/// built with, and opens the plunder dialog on the session's boarding.
#[test]
fn a_real_board_opens_the_plunder_dialog_on_the_sessions_boarding() {
    let data = fighting_data(&[]);
    let witness = Rc::new(Witness::default());
    let recorder = Rc::new(Recorder::default());
    let mut view = fighting_with(
        Rc::clone(&data),
        SessionBuilder::flying(&data).build(),
        Collaborators {
            chance: drawing(&TRADER_ABOVE),
            law: witness.clone(),
            boarding_rule: recorder.clone(),
            behaviour: Rc::new(Idle),
            ..Collaborators::default()
        },
    );
    view.change_sim(|flight| {
        let session = flight.session_mut().expect("flying");
        let trader = session.npcs()[0].id;
        nova_sim::fixture::disable(session, trader);
        nova_sim::fixture::put_player_over(session, trader);
    });
    tap(&mut view, Key::Char('b'));
    let shown = view.take_boarding().expect("the plunder dialog opens");
    assert!(
        witness.seen.borrow().contains(&nova_sim::Crime::Board),
        "judged by the view's law: {:?}",
        witness.seen.borrow()
    );
    assert!(recorder.repels.get() > 0, "the view's boarding rule asked");
    let boarding = view.session().expect("flying").boarding();
    assert_eq!(Some(shown.view), boarding, "the session's boarding");
    assert_eq!(view.boarding(), boarding);
    assert_eq!(
        view.plunder(nova_sim::Take::Abort),
        nova_sim::Taken::Aborted
    );
    assert_eq!(
        view.session().expect("flying").boarding(),
        None,
        "let go through the port"
    );
}

// The spaceport and the missions, over a real session's screen asks and
// commands.

/// A `STR#` of `strings`.
fn str_list(strings: &[&str]) -> Vec<u8> {
    let mut bytes = i16::try_from(strings.len())
        .expect("a short list")
        .to_be_bytes()
        .to_vec();
    for string in strings {
        bytes.push(u8::try_from(string.len()).expect("a short string"));
        bytes.extend(string.as_bytes());
    }
    bytes
}

/// The first `chär`, with 25,000 credits: ship 128 from system 128.
fn rich_character() -> Vec<u8> {
    let mut bytes = character();
    bytes[0x00..0x04].copy_from_slice(&25_000_i32.to_be_bytes());
    bytes
}

/// A ship that flies, with 10 tons of cargo space.
fn hauler() -> Vec<u8> {
    let mut bytes = ship();
    put_i16s(&mut bytes, 0x00, &[10]);
    bytes
}

/// Alpha Prime: a landable planet at the centre, a trade center trading
/// food at medium and metal at low.
fn trade_center() -> Vec<u8> {
    let mut bytes = planet();
    bytes[0x06..0x0A].copy_from_slice(&0x2000_1003_u32.to_be_bytes());
    bytes
}

/// A `mïsn` offered at stellar 128's mission computer on every visit,
/// going nowhere, paying 1,000, with no text.
fn errand() -> Vec<u8> {
    let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[128, 0, 0, 0, 0, 100, -1, -1, -1, -1]);
    bytes[0x1C..0x20].copy_from_slice(&1000_i32.to_be_bytes());
    put_i16s(&mut bytes, 0x20, &[0, 0, -1, -1, 0, -1, 0, -1, 0]);
    put_i16s(&mut bytes, 0x32, &[-1, -1, -1, -1, -1, -1, -1, 0, 1, -1]);
    put_i16s(&mut bytes, 0x58, &[-1]);
    bytes
}

/// Alpha (128), holding Alpha Prime (128), a trade center offering
/// mission 128 at its computer, and the commodities' names and prices.
/// The first `chär` flies ship 128, holding 10 tons, with 25,000
/// credits.
fn port_data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &rich_character())
        .resource(Ship::TYPE, 128, Some(b"Hauler"), &hauler())
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[], &[128]))
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &trade_center())
        .resource(Mission::TYPE, 128, Some(b"Errand"), &errand())
        .resource(
            StrList::TYPE,
            4000,
            Some(b"Commodities"),
            &str_list(&[
                "Food",
                "Industrial",
                "Medical Supplies",
                "Luxury Goods",
                "Metal",
                "Equipment",
            ]),
        )
        .resource(
            StrList::TYPE,
            4004,
            Some(b"Prices"),
            &str_list(&["75", "350", "750", "900", "200", "550"]),
        )
        .build()
        .bytes;
    Rc::new(one_file(fork).load())
}

/// A whole flight view over a real session of [`port_data`], landed on
/// Alpha Prime, its landing settled.
fn docked_view() -> FlightView<Rc<GameData>> {
    let data = port_data();
    let session = SessionBuilder::flying(&data)
        .landed_at(SystemId(128), StellarId(128))
        .build();
    let mut view = FlightView::from_session(data, session.into(), &Rulebook::default());
    view.settle_script();
    view
}

/// The shops' screen ask: a trade through the port changes the market the
/// view shows to the session's.
#[test]
fn a_real_trade_changes_the_market_shown() {
    let mut view = docked_view();
    let before = view.shops().market.expect("an exchange");
    let food = before
        .rows
        .iter()
        .position(|row| row.good == Good::Commodity(0))
        .expect("food traded");
    let order = Order {
        row: food,
        good: Good::Commodity(0),
        direction: Direction::Buy,
        lot: Lot::Click,
    };
    assert_eq!(view.trade(order), Ok(10), "the hold filled");
    let after = view.shops().market.expect("still an exchange");
    assert_ne!(after, before, "the hold traded shows");
    assert_eq!(
        Some(after),
        view.session().ok().and_then(nova_sim::Session::market),
        "the session's own"
    );
}

/// The mission info's screen ask: a mission accepted through the port shows
/// in the mission info as the session lists it.
#[test]
fn a_real_mission_accept_shows_in_the_mission_info() {
    let mut view = docked_view();
    assert_eq!(view.mission_info().rows, []);
    let computer = view.open_mission_computer().expect("an offer");
    let mission = computer.rows.first().expect("listed").mission;
    assert_eq!(mission, MissionId(128));
    assert_eq!(view.accept_mission(mission), Ok(0));
    let info = view.mission_info();
    assert_eq!(
        info.rows.iter().map(|row| row.slot).collect::<Vec<_>>(),
        [0],
        "listed in its slot"
    );
    assert_eq!(
        Some(info),
        view.session().ok().map(nova_sim::Session::mission_info),
        "as the session lists it"
    );
}

// The wording's names and date and the player info, through a whole
// `FlightView` over a real session
// (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks).

/// The wording's names: the take-off's greeting names its stellar and
/// the session's long date, from the opening's names and the frame.
#[test]
fn a_real_take_off_greets_naming_its_stellar_and_the_sessions_long_date() {
    let mut view = view();
    land(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    assert_eq!(view.take_off(), Some(StellarId(128)));
    let date = view.session().expect("flying").long_date_text();
    let line = view.message().expect("the greeting");
    assert!(line.contains("Alpha Prime"), "the stellar: {line:?}");
    assert!(
        line.ends_with(&format!(" on {date}.")),
        "the date: {line:?}"
    );
}

/// The router's player-info request and the pilot panel's screen ask: P
/// opens the session's player info, and the pilot panel is the
/// session's.
#[test]
fn a_real_p_opens_the_sessions_player_info_and_the_pilot_panel_is_the_sessions() {
    let data = data();
    let session = SessionBuilder::flying(&data).build();
    let mut view = FlightView::from_session(Rc::clone(&data), session.into(), &Rulebook::default());
    tap(&mut view, nova_view::ui::player_info::PLAYER_INFO_KEY);
    assert!(
        view.take_player_info_request(),
        "the session took the press"
    );
    let session = view.session().expect("flying");
    let shown = session.player_info(&*data);
    let panel = session.pilot_panel(&*data);
    assert_eq!(view.pilot_panel(), Some(panel));
    assert_eq!(view.open_player_info(), Some(shown), "it opens");
}
