//! The app jumping through hyperspace over synthetic game data, wired to
//! the renderer and the recording Gpu and driven only by key and mouse
//! events and redraws, as the window sends them: flight's map, opened with
//! M, plots a course two jumps long, and J jumps along it. Each day a jump
//! takes rolls the planetary events on the app's source of chance, whether
//! flight was entered with the developer's F or a pilot flies it, new from
//! New Pilot or resumed from Open Pilot.
//!
//! Play plots courses on the map opened from flight, which has no "Enter
//! system" button; the Tab side's map keeps it as the developer's viewer
//! (`galaxy_map.rs` and `system_view.rs` test that path).

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_audio::recording::RecordingAudio;
use nova_audio::{Audio, AudioCommand, AudioCore, Volume};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::disaster::Disaster;
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, QuadInstance, Rect};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight::shortest_turn;
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::{Chance, DisasterId, PilotKeeper, PilotStore, Session, ShipState, SystemId};
use nova_view::flight::view::TOO_CLOSE;
use nova_view::flight::{FlightView, SharedChance};
use nova_view::galaxy::map::{COURSE_HELP, ENTER_LABEL, ROUTE};
use nova_view::menu::MenuChoice;
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
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 1500, 1000, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    bytes
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

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations.
fn ship_anim() -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[2000, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
    bytes
}

/// An independent `sÿst` at map (`x`, 0) with these hyperlinks and one
/// stellar; every other slot is -1.
fn system(x: i16, links: &[i16], stellar: i16) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    put_i16s(&mut bytes, 0x24, &[stellar]);
    put_i16s(&mut bytes, 0x66, &[-1]);
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
    let fork = ForkBuilder::new()
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
        )
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
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
        let data = data();
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
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
            held: Vec::new(),
        };
        assert_eq!(harness.showing(), Showing::MainMenu);
        harness
    }

    /// Clicks the main menu's `choice` button.
    fn choose(&mut self, choice: MenuChoice) {
        let at = self
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .button(choice)
            .rect
            .center();
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
        assert_eq!(self.showing(), Showing::Flight);
    }

    /// Plots a course to Beta on flight's map, flies out and jumps there.
    fn jump_to_beta(&mut self) {
        self.frame();
        self.press(Key::Char('m'));
        let beta = self.on_map(129);
        self.click(beta);
        self.press(Key::Char('m'));
        self.fly_out();
        self.press(Key::Char('j'));
        self.run(2);
        assert_eq!(self.session().system(), SystemId(129));
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

    /// Where system `id` is on flight's map.
    fn on_map(&self, id: i16) -> Point {
        let map = self.flight().course_map();
        let system = map.model().system(SystemId(id)).expect("on the map");
        map.view().world_to_screen(system.position())
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

    /// Sends `seconds` of redraws and returns the last frame.
    fn run(&mut self, seconds: u64) -> Frame {
        let mut last = None;
        for _ in 0..seconds * 60 {
            last = Some(self.frame());
        }
        last.expect("at least one frame")
    }

    /// Flies out from the centre until the ship is at least the minimum
    /// jump distance from it, then lets go: first Down, to face away from
    /// the centre while drifting in, then Up.
    fn fly_out(&mut self) {
        for _ in 0..600 {
            let ship = self.ship();
            if ship.position.length() >= MIN_JUMP_DISTANCE {
                self.hold(&[]);
                return;
            }
            let moving_in =
                ship.position.x * ship.velocity.x + ship.position.y * ship.velocity.y < 0.0;
            let facing_out = ship.position.length() == 0.0 || {
                let out = nova_sim::flight::heading_of(ship.position);
                shortest_turn(ship.heading, out).abs() < 1e-3
            };
            if moving_in && !facing_out {
                self.hold(&[Key::Down]);
            } else {
                self.hold(&[Key::Up]);
            }
            self.frame();
        }
        panic!("never got out: {:?}", self.ship());
    }
}

fn texts(frame: &Frame) -> Vec<String> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.iter().map(|run| run.text.clone()).collect(),
            _ => Vec::new(),
        })
        .collect()
}

fn shows(frame: &Frame, expected: &str) -> bool {
    texts(frame).iter().any(|text| text == expected)
}

/// The solid quads in `color`.
fn solids_in(frame: &Frame, color: nova_view::Color) -> usize {
    let rgba = [color.r, color.g, color.b, color.a].map(|c| f32::from(c) / 255.0);
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.clone(),
            _ => Vec::new(),
        })
        .filter(|quad| quad.color == rgba)
        .count()
}

fn quads(frame: &Frame) -> Vec<QuadInstance> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// A `w` x `h` rectangle centred on (`x`, `y`).
fn centred(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        x: x - w / 2.0,
        y: y - h / 2.0,
        w,
        h,
    }
}

/// Where the stellar at (0, -300) is drawn with the camera on the ship as
/// drawn.
fn stellar_drawn(harness: &Harness) -> Rect {
    let shown = harness.flight().shown_position();
    centred(512.0 - shown.x, 384.0 - 300.0 - shown.y, 8.0, 8.0)
}

fn date(harness: &Harness) -> (u8, u8, i32) {
    let date = harness.session().date();
    (date.day(), date.month(), date.year())
}

#[test]
fn flights_map_plots_a_course_and_j_jumps_along_it_to_the_destination() {
    let mut harness = Harness::flying();
    harness.frame();
    assert_eq!(date(&harness), (23, 6, 1177));

    // M opens flight's map: no Enter button, and Return enters nothing.
    harness.press(Key::Char('m'));
    assert_eq!(harness.showing(), Showing::FlightMap);
    let map = harness.frame();
    assert!(shows(&map, COURSE_HELP), "{:?}", texts(&map));
    assert!(!shows(&map, ENTER_LABEL), "{:?}", texts(&map));
    assert_eq!(ENTER_LABEL, "Enter system (Return)");
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::FlightMap);

    // Clicking Gamma plots the course through Beta, drawn on the map.
    let gamma = harness.on_map(130);
    harness.click(gamma);
    assert_eq!(harness.session().course(), [SystemId(129), SystemId(130)]);
    let map = harness.frame();
    assert_eq!(solids_in(&map, ROUTE), 2, "a line for each jump");
    assert!(
        shows(&map, "Course: 2 jump(s) to Gamma"),
        "{:?}",
        texts(&map)
    );

    // Back in flight, J at the centre is too close.
    harness.press(Key::Char('m'));
    assert_eq!(harness.showing(), Showing::Flight);
    harness.press(Key::Char('j'));
    let refused = harness.frame();
    assert!(shows(&refused, TOO_CLOSE), "{:?}", texts(&refused));
    assert_eq!(harness.session().system(), SystemId(128));

    // Out past the minimum distance, J jumps to Beta.
    harness.fly_out();
    let fuel = harness.session().reserves().fuel.now;
    assert_eq!(fuel, 300.0);
    harness.press(Key::Char('j'));
    let arrived = harness.run(2);
    assert!(
        harness.flight().jump_effect().is_none(),
        "the effect is over"
    );
    let session = harness.session();
    assert_eq!(session.system(), SystemId(129));
    assert_eq!(session.course(), [SystemId(130)]);
    assert_eq!(session.reserves().fuel.now, 200.0);
    assert_eq!(date(&harness), (24, 6, 1177));
    assert_eq!(harness.ship().position, nova_sim::Vec2::new(-1000.0, 0.0));
    assert_eq!(
        harness.session().pilot().events().count(),
        0,
        "the app's chance never fires unless it is given one"
    );
    assert!(shows(&arrived, "Beta (sÿst 129)"), "{:?}", texts(&arrived));
    assert_eq!(quads(&arrived)[0].dest, stellar_drawn(&harness));

    // It drifts in from the edge; out again, and J goes on to Gamma, the
    // destination.
    harness.run(1);
    assert!(harness.ship().position.length() < MIN_JUMP_DISTANCE);
    harness.press(Key::Char('j'));
    assert!(harness.flight().jump_effect().is_none(), "too close");
    harness.fly_out();
    harness.press(Key::Char('j'));
    let last = harness.run(2);
    let session = harness.session();
    assert_eq!(session.system(), SystemId(130));
    assert_eq!(session.course(), []);
    assert_eq!(session.reserves().fuel.now, 100.0);
    assert_eq!(date(&harness), (25, 6, 1177));
    assert!(shows(&last, "Gamma (sÿst 130)"), "{:?}", texts(&last));
    assert_eq!(quads(&last)[0].dest, stellar_drawn(&harness));
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
fn a_jump_through_the_app_sounds_warp_up_then_warp_out() {
    let audio = RecordingAudio::new();
    let log = audio.log();
    let mut harness = Harness::flying();
    harness.app = harness
        .app
        .with_audio(AudioCore::new(Box::new(audio) as Box<dyn Audio>));
    harness.press(Key::Char('m'));
    let beta = harness.on_map(129);
    harness.click(beta);
    harness.press(Key::Char('m'));
    harness.fly_out();
    log.borrow_mut().clear();
    harness.press(Key::Char('j'));
    harness.run(2);
    assert_eq!(harness.session().system(), SystemId(129));
    let plays: Vec<AudioCommand> = log
        .borrow()
        .iter()
        .copied()
        .filter(|command| matches!(command, AudioCommand::Play { .. }))
        .collect();
    let play = |id| AudioCommand::Play {
        sound: nova_data::SoundId(id),
        volume: Volume::FULL,
    };
    assert_eq!(plays, [play(128), play(130)]);
}

/// Fires every time, recording each percent it is asked.
struct Always(Rc<RefCell<Vec<u8>>>);

impl Chance for Always {
    fn fires(&mut self, percent: u8) -> bool {
        self.0.borrow_mut().push(percent);
        true
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
    harness.press(Key::Char('m'));
    let beta = harness.on_map(129);
    harness.click(beta);
    harness.press(Key::Char('m'));
    harness.fly_out();
    assert!(
        asked.borrow().is_empty(),
        "nothing rolled before a day goes by"
    );
    harness.press(Key::Char('j'));
    harness.run(2);
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
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.session().pilot().name(), "Ada");
    harness.jump_to_beta();
    assert_eq!(*asked.borrow(), [35], "the surplus, once for the day");
    assert_eq!(
        harness.session().pilot().events().collect::<Vec<_>>(),
        [(DisasterId(128), 30)]
    );
}
