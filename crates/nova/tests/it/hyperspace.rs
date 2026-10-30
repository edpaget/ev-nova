//! The app jumping through hyperspace over synthetic game data, wired to
//! the renderer and the recording Gpu and driven only by key and mouse
//! events and redraws, as the window sends them.
//!
//! - M opens flight's map and Escape closes it, then leaves flight.
//! - Each day a jump takes rolls the planetary events on the app's source
//!   of chance, whether flight was entered with the developer's F or a
//!   pilot flies it, new from New Pilot or resumed from Open Pilot.
//! - A jump's warp sounds reach the Audio port.
//! - A pilot opened after a jump resumes where the jump left it, with the
//!   fuel it had.
//! - The Preferences dialog's Hyperspace Effects reaches flight, off and
//!   on again.
//! - L, L over a hypergate opens flight's map as the hypergate map, and a
//!   pick on it and M bring the ship into the linked system.
//!
//! How a course is plotted, when a jump is allowed, its fuel, its fades,
//! its messages and its sounds' order are nova-sim's and nova-view's,
//! pinned there. Play plots courses on the map opened from flight, which
//! has no "Enter system" button; the Tab side's map keeps it as the
//! developer's viewer (`galaxy_map.rs` and `system_view.rs` test that
//! path).

use crate::prefs_fixture;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::common::{EntryHarness, Frames};
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_audio::recording::RecordingAudio;
use nova_audio::{Audio, AudioCommand, AudioCore};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::disaster::Disaster;
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{FontFaces, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight::shortest_turn;
use nova_sim::hyperspace::{HyperlinkRule, MIN_JUMP_DISTANCE};
use nova_sim::{Chance, DisasterId, PilotKeeper, PilotStore, Session, ShipState, SystemId};
use nova_view::flight::{FlightView, SharedChance};
use nova_view::galaxy::MapMode;
use nova_view::galaxy::dialog::map_area;
use nova_view::menu::MenuChoice;
use nova_view::sound::SimSound;
use nova_view::text::fixture::MonoMetrics;
use nova_view::{Key, MouseButton, Point};

/// A 1024x768 window at scale 1: window pixels are logical units.
struct FakeWindow;

impl WindowPort for FakeWindow {
    fn size_px(&self) -> (u32, u32) {
        (1024, 768)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn request_redraw(&mut self) {}
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

fn put_u32s(bytes: &mut [u8], at: usize, values: &[u32]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 4 * i..at + 4 * i + 4].copy_from_slice(&value.to_be_bytes());
    }
}

/// A `chär` starting in ship 128 in system 128 on 23 June 1177.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// A fast `shïp`: `Accel` 1500, `Speed` 1000 and `Maneuver` 30, so 10
/// pixels a tick at most, 0.5 more a tick and 3° a tick. Its `Shield` is
/// 30, `Fuel` 300 (three jumps) and `Armor` 45.
fn ship() -> Vec<u8> {
    ShipBuilder::new()
        .shield(30)
        .accel(1500)
        .speed(1000)
        .maneuver(30)
        .fuel(300)
        .armor(45)
        .build()
}

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
fn interface() -> Vec<u8> {
    let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
    put_u32s(&mut bytes, 0x00, &[0x00FF_FFFF, 0x0080_8080]);
    put_i16s(&mut bytes, 0x08, &[8, 8, 184, 184]);
    put_u32s(&mut bytes, 0x10, &[0x0000_FF00, 0x0000_8000]);
    put_i16s(&mut bytes, 0x18, &[199, 35, 206, 184]);
    put_u32s(&mut bytes, 0x20, &[0x0000_00FF]);
    put_i16s(&mut bytes, 0x24, &[216, 35, 223, 184]);
    put_u32s(&mut bytes, 0x2C, &[0x00FF_0000]);
    put_i16s(&mut bytes, 0x30, &[234, 35, 241, 184]);
    put_u32s(&mut bytes, 0x38, &[0x00FF_FF00, 0x0080_8000]);
    put_i16s(&mut bytes, 0x40, &[254, 8, 286, 184]);
    bytes[0x60..0x66].copy_from_slice(b"Geneva");
    put_i16s(&mut bytes, 0xA0, &[12]);
    put_i16s(&mut bytes, 0xA4, &[700]);
    bytes
}

/// A grey 194 x 16 `PICT`: the status bar's background, cut short.
fn status_picture() -> Vec<u8> {
    let bounds = [0, 0, 16, 194];
    PictBuilder::new(bounds)
        .direct_bits(&DirectBits::rgb555(bounds, &[0x4210; 194 * 16]))
        .end()
        .build()
}

/// A grey `PICT`, `width` x 25: a piece of a button, which the course
/// map's Done and Find draw.
fn button_picture(width: i16) -> Vec<u8> {
    let bounds = [0, 0, 25, width];
    let pixels = vec![0x4210; usize::try_from(width).expect("positive") * 25];
    PictBuilder::new(bounds)
        .direct_bits(&DirectBits::rgb555(bounds, &pixels))
        .end()
        .build()
}

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations.
fn ship_anim() -> Vec<u8> {
    ShipAnimBuilder::new()
        .base_image_id(2000)
        .base_set_count(1)
        .frames_per(36)
        .build()
}

/// An independent `sÿst` at map (`x`, 0) with these hyperlinks and one
/// stellar, and no message buoy; every other slot is -1.
fn system(x: i16, links: &[i16], stellar: i16) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    put_i16s(&mut bytes, 0x24, &[stellar]);
    put_i16s(&mut bytes, 0x66, &[-1, -1]);
    bytes
}

/// A still `spöb` at (0, -300) of graphic type 0: `spïn` 1000.
fn stellar() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, -300, 0]);
    bytes
}

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, _| {
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![0x0400; size.into()])))
        })
        .build()
}

/// An `öops` lowering food at Beta Prime by 15 for 30 days, with a 35 %
/// chance a day.
fn food_surplus() -> Vec<u8> {
    let mut bytes = vec![0; Disaster::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[129, 0, -15, 30, 35]);
    bytes
}

/// Alpha (128), Beta (129) and Gamma (130) in a line on the map, 100
/// apart and linked 128-129-130, each holding one stellar at (0, -300)
/// with an 8 x 8 sprite. The first `chär` flies the fast ship 128 from
/// Alpha. The status bar is `ïntf` 128, over a 194 x 16 `PICT` 700. A
/// food surplus can break out at Beta Prime.
fn data() -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Courier"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim())
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[129], 128))
        .resource(System::TYPE, 129, Some(b"Beta"), &system(100, &[130], 129))
        .resource(System::TYPE, 130, Some(b"Gamma"), &system(200, &[], 130))
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &stellar())
        .resource(Stellar::TYPE, 129, Some(b"Beta Prime"), &stellar())
        .resource(Stellar::TYPE, 130, Some(b"Gamma Prime"), &stellar())
        .resource(Spin::TYPE, 1000, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture())
        .resource(
            Disaster::TYPE,
            128,
            Some(b"A food surplus"),
            &food_surplus(),
        );
    // The map's background: any picture, stretched over the dialog.
    fork = fork.resource(PICT, 8509, Some(b"Map"), &button_picture(13));
    // The buttons' pictures, each state's caps masked.
    for state in [7500, 7503, 7506] {
        fork = fork
            .resource(PICT, state, None, &button_picture(13))
            .resource(PICT, state + 1, None, &button_picture(2))
            .resource(PICT, state + 2, None, &button_picture(13))
            .resource(PICT, state + 100, None, &button_picture(13))
            .resource(PICT, state + 102, None, &button_picture(13));
    }
    Rc::new(one_file(crate::bracket_art::with_bracket_art(fork).build().bytes).load())
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    /// Frames sent, at 60 a second.
    frames: u64,
    /// The flight keys held.
    held: Vec<Key>,
}

impl Harness {
    /// The app in flight, entered from the ship browser with F.
    fn flying() -> Self {
        Self::flying_with(SharedChance::default())
    }

    /// The app rolling chances on `chance`, in flight, entered from the
    /// ship browser with F.
    fn flying_with(chance: SharedChance) -> Self {
        Self::flying_over(data(), chance)
    }
    /// The app over `data`, rolling chances on `chance`, in flight, entered
    /// from the ship browser with F.
    fn flying_over(data: Rc<GameData>, chance: SharedChance) -> Self {
        let screen = start_screen(Rc::clone(&data)).with_chance(chance);
        let mut harness = Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
            held: Vec::new(),
        };
        harness.press(Key::Char('f'));
        assert_eq!(harness.showing(), Showing::Flight);
        harness
    }

    /// The app rolling chances on `chance`, on the main menu, keeping
    /// pilots in `store`.
    fn on_the_menu(chance: SharedChance, store: &MemoryPilots) -> Self {
        let data = data();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let screen = start_screen(Rc::clone(&data))
            .with_chance(chance)
            .with_pilots(Some(keeper), Rc::new(MonoMetrics));
        let harness = Self {
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
            held: Vec::new(),
        };
        assert_eq!(harness.showing(), Showing::MainMenu);
        harness
    }

    /// The app on the main menu, keeping pilots in `store`, with the
    /// Preferences dialog (and no other) from the interface file, laid out
    /// by the bundled fonts' metrics.
    fn on_the_menu_with_prefs(store: &MemoryPilots) -> Self {
        let data = data();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(prefs_fixture::interface()), metrics);
        let harness = Self {
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
            held: Vec::new(),
        };
        assert_eq!(harness.showing(), Showing::MainMenu);
        harness
    }

    /// Where the Preferences dialog's Hyperspace Effects box is.
    fn hyperspace_effects_box(&self) -> Point {
        self.app
            .screen()
            .preferences()
            .expect("the Preferences dialog is open")
            .hyperspace_effects()
            .rect()
            .center()
    }

    /// Clicks the main menu's `choice` button.
    fn choose(&mut self, choice: MenuChoice) {
        let at = self
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .press_point(choice);
        self.click(at);
    }

    /// Flies a new pilot named `name`: New Pilot, the name, Return.
    fn new_pilot(&mut self, name: &str) {
        self.choose(MenuChoice::NewPilot);
        assert_eq!(self.showing(), Showing::NewPilot);
        for c in name.chars() {
            self.send(WindowEvent::Text(c));
        }
        self.press(Key::Enter);
        // The ship's christening, confirmed with its default.
        self.press(Key::Enter);
        self.enter_ship();
        assert_eq!(self.showing(), Showing::Flight);
    }

    /// Plots a course to Beta on flight's map and flies out far enough to
    /// jump.
    fn out_towards_beta(&mut self) {
        self.plot(129);
        self.fly_out();
    }

    /// Plots a course to system `id` on flight's map: M, a Shift-click on
    /// each hop of the shortest route there, M, as `_DoSystemMap` builds a
    /// course a hop at a time; the close cues its first hop.
    fn plot(&mut self, id: i16) {
        self.frame();
        let session = self.session();
        let route = session
            .star_map()
            .route(session.system(), SystemId(id), HyperlinkRule::Engine)
            .expect("a route");
        self.press(Key::Char('m'));
        self.key(Key::Shift, true);
        for hop in &route {
            let at = self.on_map(hop.0);
            self.click(at);
        }
        self.key(Key::Shift, false);
        assert_eq!(self.session().course(), route, "plotted");
        self.press(Key::Char('m'));
    }

    /// Plots a course to Beta on flight's map, flies out and jumps there.
    fn jump_to_beta(&mut self) {
        self.out_towards_beta();
        self.jump_into(129);
        assert_eq!(self.session().system(), SystemId(129));
    }

    /// Presses J and draws frames until the ship has arrived in system `id`
    /// and the jump's effect is over.
    fn jump_into(&mut self, id: i16) {
        self.press(Key::Char('j'));
        self.settle_until("the jump's arrival", |harness| {
            harness.session().system() == SystemId(id) && harness.flight().jump_effect().is_none()
        });
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    fn key(&mut self, key: Key, pressed: bool) {
        self.send(WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        });
    }

    fn press(&mut self, key: Key) {
        self.key(key, true);
        self.key(key, false);
    }

    /// Presses Command+comma, which opens the Preferences in flight.
    fn command_comma(&mut self) {
        self.key(Key::Command, true);
        self.press(Key::Char(','));
        self.key(Key::Command, false);
    }

    /// Holds exactly `keys` of the flight keys, pressing and releasing
    /// only those that change.
    fn hold(&mut self, keys: &[Key]) {
        for key in self.held.clone() {
            if !keys.contains(&key) {
                self.key(key, false);
            }
        }
        for &key in keys {
            if !self.held.contains(&key) {
                self.key(key, true);
            }
        }
        self.held = keys.to_vec();
    }

    /// Clicks the left button at `at` (window pixels are logical units).
    fn click(&mut self, at: Point) {
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        });
        for pressed in [true, false] {
            self.send(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed,
            });
        }
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn flight(&self) -> &FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn session(&self) -> &Session {
        self.flight().session().expect("flying")
    }

    fn ship(&self) -> ShipState {
        *self.session().player()
    }

    /// Where system `id` is on flight's map, scrolled well into item 3
    /// with the arrows first: the map opens at 0.5625 on the current
    /// system and shows little of the galaxy.
    fn on_map(&mut self, id: i16) -> Point {
        let shown = map_area().grown(-12.0);
        for _ in 0..200 {
            let map = self.flight().course_map();
            let system = map.model().system(SystemId(id)).expect("on the map");
            let at = map.transform().world_to_screen(system.position());
            let key = if at.x < shown.min.x {
                Key::Left
            } else if at.x > shown.max.x {
                Key::Right
            } else if at.y < shown.min.y {
                Key::Up
            } else if at.y > shown.max.y {
                Key::Down
            } else {
                return at;
            };
            self.press(key);
        }
        panic!("system {id} never scrolled into item 3");
    }

    /// Sends the next redraw, 1/60 s after the last, and returns its frame.
    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        self.send(WindowEvent::Redraw { elapsed });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }
    /// Draws frames until the dock approach has cleared the request.
    fn approach(&mut self) {
        self.settle_until("the dock approach's clearance", |game| {
            game.session().dock_cleared()
        });
    }

    /// Flies out from the centre until the ship is at least the minimum
    /// jump distance from it, then lets go: first Down, to face away from
    /// the centre while drifting in, then Up.
    fn fly_out(&mut self) {
        // Long enough to turn round a jump's arrival at top speed inward.
        for _ in 0..1800 {
            if self.ship().position.length() >= MIN_JUMP_DISTANCE {
                self.hold(&[]);
                return;
            }
            self.steer(true);
            self.frame();
        }
        panic!("never got out: {:?}", self.ship());
    }

    /// Holds the keys for the next frame of flying `out` from the centre,
    /// or in towards it: Down while drifting the wrong way and not yet
    /// facing the right one, otherwise Up.
    fn steer(&mut self, out: bool) {
        let ship = self.ship();
        let outward = ship.position.x * ship.velocity.x + ship.position.y * ship.velocity.y;
        // A crawl, as a ship at rest off the centre can drift, isn't
        // drifting wrong: Down would hold it there, as reverse aims only
        // a ship moving fast enough on an axis.
        let drifting_wrong = (if out { outward < -1e-3 } else { outward > 1e-3 })
            && nova_sim::player_turn::reverse_aim(ship.velocity).is_some();
        // Down stops within a turn of its whole-degree aim, without
        // snapping onto it: a turn and the table's degree of slack.
        let turn = self.session().handling().turn_rate.trunc() + 1.0;
        let facing = ship.position.length() == 0.0 || {
            let away = nova_sim::flight::heading_of(ship.position);
            let way = if out { away } else { away + 180.0 };
            shortest_turn(ship.heading, way).abs() <= turn
        };
        if drifting_wrong && !facing {
            self.hold(&[Key::Down]);
        } else {
            self.hold(&[Key::Up]);
        }
    }
}

#[test]
fn escape_closes_flights_map_and_then_leaves_flight() {
    let mut harness = Harness::flying();
    harness.press(Key::Char('m'));
    assert_eq!(harness.showing(), Showing::FlightMap);
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::Flight);
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::ShipBrowser);
}

#[test]
fn a_jumps_warp_sounds_reach_the_audio_port() {
    let audio = RecordingAudio::new();
    let log = audio.log();
    let mut harness = Harness::flying();
    harness.app = harness
        .app
        .with_audio(AudioCore::new(Box::new(audio) as Box<dyn Audio>));
    harness.plot(129);
    harness.fly_out();
    log.borrow_mut().clear();
    harness.jump_into(129);
    let up = SimSound::JumpBegan { rate: 50_412 }
        .cue()
        .expect("the warp-up plays")
        .sound;
    let out = SimSound::Arrived.cue().expect("the warp-out plays").sound;
    let log = log.borrow();
    assert!(
        log.iter().any(
            |command| matches!(*command, AudioCommand::PlayAtRate { sound, .. } if sound == up)
        ),
        "the warp-up: {log:?}"
    );
    assert!(
        log.iter()
            .any(|command| matches!(*command, AudioCommand::Play { sound, .. } if sound == out)),
        "the warp-out: {log:?}"
    );
    assert!(
        log.iter()
            .any(|command| matches!(*command, AudioCommand::Halt { sound } if sound == up)),
        "the warp-up halted: {log:?}"
    );
}

/// `_SwitchedOut` halts 128 then 129 as the app is switched out
/// (@0x4f3a3-0x4f3e0), here mid-jump with the warp-up playing.
#[test]
fn losing_focus_halts_the_warp_ups_on_the_audio_port() {
    let audio = RecordingAudio::new();
    let log = audio.log();
    let mut harness = Harness::flying();
    harness.app = harness
        .app
        .with_audio(AudioCore::new(Box::new(audio) as Box<dyn Audio>));
    harness.plot(129);
    harness.fly_out();
    harness.press(Key::Char('j'));
    let warp_up_playing = |log: &Rc<RefCell<Vec<AudioCommand>>>| {
        log.borrow()
            .iter()
            .any(|command| matches!(command, AudioCommand::PlayAtRate { .. }))
    };
    harness.settle_until("the warp-up", |_| warp_up_playing(&log));
    assert_ne!(harness.session().system(), SystemId(129), "before arrival");
    log.borrow_mut().clear();
    harness.send(WindowEvent::FocusLost);
    let halts: Vec<_> = log
        .borrow()
        .iter()
        .filter_map(|command| match *command {
            AudioCommand::Halt { sound } => Some(sound),
            _ => None,
        })
        .collect();
    assert_eq!(halts, nova_sim::sound::WARP_UP_HALTS);
}

/// Fires every time, recording each percent it is asked.
struct Always(Rc<RefCell<Vec<u8>>>);

impl Chance for Always {
    fn fires(&mut self, percent: u8) -> bool {
        self.0.borrow_mut().push(percent);
        true
    }

    /// The first outcome.
    fn roll(&mut self, _sides: u16) -> u16 {
        0
    }

    fn below(&mut self, _n: u32) -> u32 {
        0
    }
}

/// The percents asked of an always-firing chance, and that chance, shared.
fn always() -> (Rc<RefCell<Vec<u8>>>, SharedChance) {
    let asked = Rc::new(RefCell::new(Vec::new()));
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always(Rc::clone(&asked))));
    (asked, SharedChance::new(chance))
}

#[test]
fn each_day_of_a_jump_rolls_the_events_on_the_apps_chance() {
    let (asked, chance) = always();
    let mut harness = Harness::flying_with(chance);
    harness.frame();
    harness.plot(129);
    harness.fly_out();
    assert!(
        asked.borrow().is_empty(),
        "nothing rolled before a day goes by"
    );
    harness.jump_into(129);
    assert_eq!(harness.session().system(), SystemId(129));
    assert_eq!(*asked.borrow(), [35], "the surplus, once for the day");
    assert_eq!(
        harness.session().pilot().events().collect::<Vec<_>>(),
        [(DisasterId(128), 30)]
    );
}

#[test]
fn a_new_pilots_jump_rolls_the_events_on_the_apps_chance() {
    let (asked, chance) = always();
    let mut harness = Harness::on_the_menu(chance, &MemoryPilots::new());
    harness.new_pilot("Ada");
    harness.jump_to_beta();
    assert_eq!(*asked.borrow(), [35], "the surplus, once for the day");
    assert_eq!(
        harness.session().pilot().events().collect::<Vec<_>>(),
        [(DisasterId(128), 30)]
    );
}

#[test]
fn an_opened_pilots_jump_rolls_the_events_on_the_apps_chance() {
    let store = MemoryPilots::new();
    Harness::on_the_menu(SharedChance::default(), &store).new_pilot("Ada");
    assert_eq!(store.keys(), ["Ada"]);
    let (asked, chance) = always();
    let mut harness = Harness::on_the_menu(chance, &store);
    harness.choose(MenuChoice::OpenPilot);
    assert_eq!(harness.showing(), Showing::OpenPilot);
    harness.press(Key::Enter);
    harness.enter_ship();
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.session().pilot().name(), "Ada");
    harness.jump_to_beta();
    assert_eq!(*asked.borrow(), [35], "the surplus, once for the day");
    assert_eq!(
        harness.session().pilot().events().collect::<Vec<_>>(),
        [(DisasterId(128), 30)]
    );
}

// The fuel is compared with the very value saved.
#[test]
#[allow(clippy::float_cmp)]
fn an_opened_pilot_resumes_where_its_saved_jump_left_it_with_its_fuel() {
    let store = MemoryPilots::new();
    let mut first = Harness::on_the_menu(SharedChance::default(), &store);
    first.new_pilot("Ada");
    first.jump_to_beta();
    let fuel = first.session().reserves().fuel.now;
    first.press(Key::Escape);
    assert_ne!(first.showing(), Showing::Flight, "left flight, saving");

    let mut harness = Harness::on_the_menu(SharedChance::default(), &store);
    harness.choose(MenuChoice::OpenPilot);
    harness.press(Key::Enter);
    harness.enter_ship();
    assert_eq!(harness.showing(), Showing::Flight);
    let session = harness.session();
    assert_eq!(session.system(), SystemId(129));
    assert_eq!(session.reserves().fuel.now, fuel);
}

#[test]
fn the_hyperspace_effects_preference_reaches_flight_off_and_on_again() {
    let mut harness = Harness::on_the_menu_with_prefs(&MemoryPilots::new());
    harness.new_pilot("Ada");
    let was = harness.flight().hyperspace_effects();
    for on in [!was, was] {
        // Command+comma, a click on Hyperspace Effects, and Return.
        harness.command_comma();
        assert_eq!(harness.showing(), Showing::Preferences);
        let at = harness.hyperspace_effects_box();
        harness.click(at);
        harness.press(Key::Enter);
        assert_eq!(harness.showing(), Showing::Flight);
        assert_eq!(harness.flight().hyperspace_effects(), on);
    }
}

/// An independent `sÿst` at map (`x`, 0) with these hyperlinks and
/// stellars; every other slot is -1.
fn system_of(x: i16, links: &[i16], stellars: &[i16]) -> Vec<u8> {
    let mut bytes = system(x, links, -1);
    put_i16s(&mut bytes, 0x24, &[-1; 16]);
    put_i16s(&mut bytes, 0x24, stellars);
    bytes
}

/// A hypergate `spöb` (a landable station, `Flags2` 0x1000) at (`x`, `y`),
/// heading ships out on `angle`, linked to `links`; every other slot -1.
fn hypergate(x: i16, y: i16, angle: i16, links: &[i16]) -> Vec<u8> {
    let mut bytes = stellar();
    put_i16s(&mut bytes, 0x00, &[x, y]);
    bytes[0x06..0x0A].copy_from_slice(&0x11_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x1A, &[angle]);
    bytes[0x20..0x22].copy_from_slice(&0x1000_u16.to_be_bytes());
    put_i16s(&mut bytes, 0x26, &[-1; 8]);
    put_i16s(&mut bytes, 0x26, links);
    bytes
}

/// [`data`]'s galaxy, with hypergate 131 at Alpha's centre, where the ship
/// starts, linked to hypergate 132 in Gamma at (50, 60), which heads ships
/// out on 90°.
fn gate_data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Courier"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim())
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(
            System::TYPE,
            128,
            Some(b"Alpha"),
            &system_of(0, &[129], &[128, 131]),
        )
        .resource(
            System::TYPE,
            129,
            Some(b"Beta"),
            &system_of(100, &[130], &[129]),
        )
        .resource(
            System::TYPE,
            130,
            Some(b"Gamma"),
            &system_of(200, &[], &[130, 132]),
        )
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &stellar())
        .resource(Stellar::TYPE, 129, Some(b"Beta Prime"), &stellar())
        .resource(Stellar::TYPE, 130, Some(b"Gamma Prime"), &stellar())
        .resource(
            Stellar::TYPE,
            131,
            Some(b"HG-Alpha"),
            &hypergate(0, 0, 120, &[132]),
        )
        .resource(
            Stellar::TYPE,
            132,
            Some(b"HG-Gamma"),
            &hypergate(50, 60, 90, &[131]),
        )
        .resource(Spin::TYPE, 1000, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture());
    Rc::new(one_file(crate::bracket_art::with_bracket_art(fork).build().bytes).load())
}

impl Harness {
    /// In flight over [`gate_data`], L pressed twice over HG-Alpha: the
    /// hypergate map is open, and Gamma clicked on it.
    fn gamma_picked() -> Self {
        let mut harness = Self::flying_over(gate_data(), SharedChance::default());
        harness.frame();
        harness.press(Key::Char('l'));
        harness.approach();
        harness.press(Key::Char('l'));
        assert_eq!(harness.showing(), Showing::FlightMap);
        assert_eq!(harness.flight().course_map().mode(), MapMode::Hypergate);
        let gamma = harness.on_map(130);
        harness.click(gamma);
        assert_eq!(harness.session().system(), SystemId(128), "not yet");
        harness
    }
}

#[test]
fn a_hypergate_picked_on_the_map_brings_the_ship_into_its_system() {
    let mut harness = Harness::gamma_picked();
    harness.press(Key::Char('m'));
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.session().system(), SystemId(130));
}

crate::common::entry_harness!(Harness, click, frame);
