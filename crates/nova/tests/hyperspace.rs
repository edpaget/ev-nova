//! The app jumping through hyperspace over synthetic game data, wired to
//! the renderer and the recording Gpu and driven only by key and mouse
//! events and redraws, as the window sends them: flight's map, opened with
//! M, plots a course two jumps long, and J jumps along it, pressed again as
//! soon as the new system fades in from white. Each day a jump
//! takes rolls the planetary events on the app's source of chance, whether
//! flight was entered with the developer's F or a pilot flies it, new from
//! New Pilot or resumed from Open Pilot.
//!
//! With the Hyperspace Effects preference turned off in the Preferences
//! dialog, a jump streaks and arrives without its white fades; turned back
//! on, they return.
//!
//! L, L over a hypergate opens flight's map as the hypergate map; a click
//! on a linked system and M, or Escape, bring the ship out of the gate
//! there.
//!
//! Play plots courses on the map opened from flight, which has no "Enter
//! system" button; the Tab side's map keeps it as the developer's viewer
//! (`galaxy_map.rs` and `system_view.rs` test that path).

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

mod prefs_fixture;

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
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame, QuadInstance, Rect, SolidQuad};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight::shortest_turn;
use nova_sim::hyperspace::{JUMP_FUEL, MIN_JUMP_DISTANCE};
use nova_sim::{
    Chance, DisasterId, PilotKeeper, PilotStore, Session, ShipState, SystemId, TICKS_PER_SECOND,
};
use nova_view::flight::hud::{NAV_HYPERSPACE, NAV_UNEXPLORED};
use nova_view::flight::jump::{ARRIVAL_FLASH_FOR, STREAK_FOR};
use nova_view::flight::view::TOO_CLOSE;
use nova_view::flight::{FlightView, SharedChance};
use nova_view::galaxy::MapMode;
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
/// 30, `Fuel` 300 (three jumps) and `Armor` 45, and it regenerates no fuel.
fn ship() -> Vec<u8> {
    ship_with_regen(0)
}

/// [`ship`], with a `FuelRegen` of `regen`: a unit of fuel every `regen`
/// ticks.
fn ship_with_regen(regen: i16) -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 1500, 1000, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    put_i16s(&mut bytes, 0x5E, &[regen]);
    bytes
}

/// [`ship`], with a `Fuel` of `fuel`.
fn ship_with_fuel(fuel: i16) -> Vec<u8> {
    let mut bytes = ship();
    put_i16s(&mut bytes, 0x02, &[30, 1500, 1000, 30, fuel]);
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
    data_with(&ship())
}

/// [`data`], with `ship` as `shïp` 128.
fn data_with(ship: &[u8]) -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Courier"), ship)
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
            app: App::new(&FakeWindow, data, screen),
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
            app: App::new(&FakeWindow, data, screen),
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

    /// Plots a course to Beta on flight's map and flies out far enough to
    /// jump.
    fn out_towards_beta(&mut self) {
        self.out_towards(129);
    }

    /// Plots a course to system `id` on flight's map and flies out far
    /// enough to jump.
    fn out_towards(&mut self, id: i16) {
        self.plot(id);
        self.fly_out();
    }

    /// Plots a course to system `id` on flight's map: M, a click on it, M.
    fn plot(&mut self, id: i16) {
        self.frame();
        self.press(Key::Char('m'));
        let system = self.on_map(id);
        self.click(system);
        self.press(Key::Char('m'));
    }

    /// Plots a course to Beta on flight's map, flies out and jumps there.
    fn jump_to_beta(&mut self) {
        self.out_towards_beta();
        self.jump_into(129);
        assert_eq!(self.session().system(), SystemId(129));
    }

    /// Presses J and sends redraws while the ship turns and slows to jump,
    /// jumps and arrives in system `id`, until the jump's effect is over,
    /// and returns the last frame.
    fn jump_into(&mut self, id: i16) -> Frame {
        self.press(Key::Char('j'));
        for _ in 0..1200 {
            let frame = self.frame();
            if self.session().system() == SystemId(id) && self.flight().jump_effect().is_none() {
                return frame;
            }
        }
        panic!("never arrived in {id}: {:?}", self.ship());
    }

    /// Presses J and sends redraws until the ship is in Beta, and returns
    /// the first frame drawn there.
    fn arrive_in_beta(&mut self) -> Frame {
        self.arrive_in(129)
    }

    /// Presses J and sends redraws until the ship is in system `id`, and
    /// returns the first frame drawn there.
    fn arrive_in(&mut self, id: i16) -> Frame {
        self.press(Key::Char('j'));
        for _ in 0..600 {
            let frame = self.frame();
            if self.session().system() == SystemId(id) {
                return frame;
            }
        }
        panic!("never arrived: {:?}", self.session().system());
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
            if self.ship().position.length() >= MIN_JUMP_DISTANCE {
                self.hold(&[]);
                return;
            }
            self.steer(true);
            self.frame();
        }
        panic!("never got out: {:?}", self.ship());
    }

    /// Flies back in towards the centre until the ship is nearer it than
    /// the minimum jump distance, then lets go: first Down, to face the
    /// centre while drifting out, then Up.
    fn fly_in(&mut self) {
        for _ in 0..600 {
            if self.ship().position.length() < MIN_JUMP_DISTANCE {
                self.hold(&[]);
                return;
            }
            self.steer(false);
            self.frame();
        }
        panic!("never got in: {:?}", self.ship());
    }

    /// Holds the keys for the next frame of flying `out` from the centre,
    /// or in towards it: Down while drifting the wrong way and not yet
    /// facing the right one, otherwise Up.
    fn steer(&mut self, out: bool) {
        let ship = self.ship();
        let outward = ship.position.x * ship.velocity.x + ship.position.y * ship.velocity.y;
        let drifting_wrong = if out { outward < 0.0 } else { outward > 0.0 };
        let facing = ship.position.length() == 0.0 || {
            let away = nova_sim::flight::heading_of(ship.position);
            let way = if out { away } else { away + 180.0 };
            shortest_turn(ship.heading, way).abs() < 1e-3
        };
        if drifting_wrong && !facing {
            self.hold(&[Key::Down]);
        } else {
            self.hold(&[Key::Up]);
        }
    }

    /// The texts the next frame draws in the nav area, stock `ïntf` 128's
    /// (8, 254)-(184, 286) on the bar, each with its colour.
    fn nav_runs(&mut self) -> Vec<(String, nova_view::Color)> {
        let frame = self.frame();
        frame
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Text(runs) => runs.clone(),
                _ => Vec::new(),
            })
            .filter(|run| {
                let (x, y) = run.origin_px;
                (838.0..=1014.0).contains(&x) && (254.0..=286.0).contains(&y)
            })
            .map(|run| (run.text, run.color))
            .collect()
    }

    /// The colour the next frame draws the nav area's destination in.
    fn destination_color(&mut self) -> nova_view::Color {
        let runs = self.nav_runs();
        assert_eq!(runs.len(), 2, "{runs:?}");
        assert_eq!(runs[0].0, NAV_HYPERSPACE, "{runs:?}");
        assert_eq!(runs[1].0, NAV_UNEXPLORED, "{runs:?}");
        runs[1].1
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
fn solids(frame: &Frame, color: nova_view::Color) -> Vec<SolidQuad> {
    let rgba = [color.r, color.g, color.b, color.a].map(|c| f32::from(c) / 255.0);
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.clone(),
            _ => Vec::new(),
        })
        .filter(|quad| quad.color == rgba)
        .collect()
}

/// How many solid quads are in `color`.
fn solids_in(frame: &Frame, color: nova_view::Color) -> usize {
    solids(frame, color).len()
}

/// The status bar's whole-jumps fuel colour, the fixture `ïntf`'s
/// `FuelFull`.
const FUEL_FULL: nova_view::Color = nova_view::Color::from_rgb24(0x00FF_FF00);

/// How wide the status bar's whole-jumps fuel bar is drawn: from the
/// leftmost to the rightmost corner of its quads, 0 when there are none.
fn fuel_bar(frame: &Frame) -> f32 {
    let xs: Vec<f32> = solids(frame, FUEL_FULL)
        .iter()
        .flat_map(|quad| quad.corners.map(|corner| corner.x))
        .collect();
    let min = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let max = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    if xs.is_empty() { 0.0 } else { max - min }
}

/// Whether `a` and `b` are within a thousandth of each other.
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
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
    let arrived = harness.jump_into(129);
    assert!(
        harness.flight().jump_effect().is_none(),
        "the effect is over"
    );
    let session = harness.session();
    assert_eq!(session.system(), SystemId(129));
    assert_eq!(session.course(), [SystemId(130)]);
    assert_eq!(session.reserves().fuel.now, 200.0);
    assert_eq!(date(&harness), (24, 6, 1177));
    assert_eq!(harness.ship().position, nova_sim::Vec2::new(-1001.0, 0.0));
    assert_eq!(
        harness.session().pilot().events().count(),
        0,
        "the app's chance never fires unless it is given one"
    );
    assert!(shows(&arrived, "Beta (sÿst 129)"), "{:?}", texts(&arrived));
    assert_eq!(quads(&arrived)[0].dest, stellar_drawn(&harness));

    // It rests at the edge, so J goes straight on to Gamma, the
    // destination.
    harness.run(1);
    assert_eq!(harness.ship().position, nova_sim::Vec2::new(-1001.0, 0.0));
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
fn j_straight_after_arriving_jumps_on_to_the_next_system() {
    let mut harness = Harness::flying();
    harness.out_towards(130);
    harness.arrive_in(129);
    // Once the jump effect is over, and a second later, J still goes on.
    let mut frames = 0;
    while harness.flight().jump_effect().is_some() {
        frames += 1;
        assert!(frames <= 120, "the jump effect never ended");
        harness.frame();
    }
    harness.run(1);
    harness.press(Key::Char('j'));
    let pressed = harness.frame();
    assert!(!shows(&pressed, TOO_CLOSE), "{:?}", texts(&pressed));
    assert!(harness.flight().jump_effect().is_some(), "jumping on");
    harness.run(2);
    let session = harness.session();
    assert_eq!(session.system(), SystemId(130));
    assert_eq!(session.reserves().fuel.now, 100.0);
}

#[test]
fn j_as_the_new_system_fades_in_jumps_on_to_the_next_system() {
    let mut harness = Harness::flying();
    harness.out_towards(130);
    let first = harness.arrive_in(129);
    assert!(white(&first) || fading(&first), "fading in from white");
    harness.press(Key::Char('j'));
    let pressed = harness.frame();
    assert!(fading(&pressed), "still fading in");
    assert!(!shows(&pressed, TOO_CLOSE), "{:?}", texts(&pressed));
    let session = harness.session();
    assert!(
        session.preparing_jump() == Some(SystemId(130)) || session.jumping() == Some(SystemId(130)),
        "jumping on: {:?} {:?}",
        session.preparing_jump(),
        session.jumping()
    );
    harness.run(4);
    let session = harness.session();
    assert_eq!(session.system(), SystemId(130));
    assert_eq!(session.reserves().fuel.now, 100.0);
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
    harness.jump_into(129);
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

    /// The first outcome.
    fn roll(&mut self, _sides: u16) -> u16 {
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
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.session().pilot().name(), "Ada");
    harness.jump_to_beta();
    assert_eq!(*asked.borrow(), [35], "the surplus, once for the day");
    assert_eq!(
        harness.session().pilot().events().collect::<Vec<_>>(),
        [(DisasterId(128), 30)]
    );
}

#[test]
fn a_jump_takes_a_jumps_fuel_off_the_gauge_and_the_hud_bar_and_it_stays_off() {
    let mut harness = Harness::flying();
    harness.out_towards_beta();
    let before = harness.session().reserves().fuel;
    assert_eq!(before.now, 300.0);
    let bar_before = fuel_bar(&harness.frame());
    assert!(close(bar_before, 149.0), "the whole bar: {bar_before}");

    let arrived = harness.arrive_in_beta();
    let after = before.now - JUMP_FUEL;
    assert_eq!(harness.session().reserves().fuel.now, after);
    let bar = fuel_bar(&arrived);
    assert!(close(bar, bar_before * after / before.max), "{bar}");

    let later = harness.run(5);
    assert_eq!(harness.session().reserves().fuel.now, after);
    assert!(close(fuel_bar(&later), bar), "{}", fuel_bar(&later));
}

#[test]
fn a_regenerating_ship_arrives_a_jumps_fuel_down_and_regen_only_creeps_back() {
    // A unit every 30 ticks: a unit a second.
    const REGEN_TICKS: i16 = 30;
    let data = data_with(&ship_with_regen(REGEN_TICKS));
    let mut harness = Harness::flying_over(data, SharedChance::default());
    // A first jump drains the tank below full, so the second jump's
    // `before` sits where regen is free to show, not clamped at the top.
    harness.jump_to_beta();
    // It rests at the edge, so plotting on to Gamma needs no flying out.
    harness.run(1);
    harness.out_towards(130);
    let bar_before = fuel_bar(&harness.frame());
    let before = harness.session().reserves().fuel.now;
    assert!(before < 300.0, "below full, so regen could show: {before}");

    let arrived = harness.arrive_in(130);
    assert_eq!(
        harness.session().reserves().fuel.now,
        before - JUMP_FUEL,
        "none regained during the jump or as a lump on arrival"
    );
    assert!(fuel_bar(&arrived) < bar_before, "{}", fuel_bar(&arrived));

    let seconds: u16 = 5;
    harness.run(u64::from(seconds));
    // The normal rate over the time flown, plus a unit for the frame
    // boundaries the fixed step may fall either side of.
    let ticks = f32::from(seconds) * TICKS_PER_SECOND as f32;
    let regen_over_the_run = ticks / f32::from(REGEN_TICKS);
    let at_most = before - JUMP_FUEL + regen_over_the_run + 1.0;
    let fuel = harness.session().reserves().fuel.now;
    assert!(
        before - JUMP_FUEL < fuel && fuel <= at_most,
        "creeping back at its normal rate, at most {at_most}: {fuel}"
    );
}

#[test]
fn an_opened_pilot_resumes_with_the_fuel_its_saved_jump_left() {
    let store = MemoryPilots::new();
    let mut first = Harness::on_the_menu(SharedChance::default(), &store);
    first.new_pilot("Ada");
    first.jump_to_beta();
    assert_eq!(first.session().reserves().fuel.now, 300.0 - JUMP_FUEL);
    first.press(Key::Escape);
    assert_ne!(first.showing(), Showing::Flight, "left flight, saving");

    let mut harness = Harness::on_the_menu(SharedChance::default(), &store);
    harness.choose(MenuChoice::OpenPilot);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::Flight);
    let session = harness.session();
    assert_eq!(session.system(), SystemId(129));
    assert_eq!(session.reserves().fuel.now, 300.0 - JUMP_FUEL);
    let bar = fuel_bar(&harness.frame());
    assert!(close(bar, 149.0 * 2.0 / 3.0), "{bar}");
}

/// `ïntf` 128's bright text colour.
const BRIGHT: nova_view::Color = nova_view::Color::from_rgb24(0x00FF_FFFF);
/// `ïntf` 128's dim text colour.
const DIM: nova_view::Color = nova_view::Color::from_rgb24(0x0080_8080);

#[test]
fn the_hyperspace_destination_brightens_once_the_ship_is_out_far_enough_to_jump() {
    let mut game = Harness::flying();
    game.plot(129);
    assert_eq!(game.destination_color(), DIM, "plotted at the centre");
    let mut dim_frames = 0;
    loop {
        game.steer(true);
        let color = game.destination_color();
        let out = game.ship().position.length() >= MIN_JUMP_DISTANCE;
        assert_eq!(color, if out { BRIGHT } else { DIM }, "{:?}", game.ship());
        if out {
            break;
        }
        dim_frames += 1;
        assert!(dim_frames < 600, "never got out: {:?}", game.ship());
    }
    assert!(dim_frames > 0, "it was dim on the way out");
    game.hold(&[]);
    assert_eq!(game.destination_color(), BRIGHT, "still out");
    game.fly_in();
    assert_eq!(game.destination_color(), DIM, "back inside");
}

#[test]
fn a_ship_without_a_jumps_fuel_keeps_the_destination_dim_out_past_the_jump_distance() {
    let mut game = Harness::flying_over(data_with(&ship_with_fuel(99)), SharedChance::default());
    game.out_towards(129);
    assert_eq!(game.destination_color(), DIM);
    assert!(game.ship().position.length() >= MIN_JUMP_DISTANCE);
}

/// The alphas of the white quads covering the whole screen: a jump's fade.
fn screen_whites(frame: &Frame) -> Vec<f32> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.clone(),
            _ => Vec::new(),
        })
        .filter(|quad| {
            let xs = quad.corners.map(|corner| corner.x);
            let ys = quad.corners.map(|corner| corner.y);
            let spans = |values: [f32; 4], to: f32| {
                values.iter().copied().fold(f32::INFINITY, f32::min) <= 0.0
                    && values.iter().copied().fold(f32::NEG_INFINITY, f32::max) >= to
            };
            quad.color[..3] == [1.0, 1.0, 1.0] && spans(xs, 1024.0) && spans(ys, 768.0)
        })
        .map(|quad| quad.color[3])
        .collect()
}

/// Whether `frame` is partway through a fade: a translucent white over the
/// whole screen.
fn fading(frame: &Frame) -> bool {
    screen_whites(frame).iter().any(|&alpha| alpha < 1.0)
}

/// Whether `frame` is covered in solid white.
fn white(frame: &Frame) -> bool {
    screen_whites(frame).contains(&1.0)
}

#[test]
fn with_hyperspace_effects_off_a_jump_streaks_and_arrives_without_fading() {
    let mut harness = Harness::on_the_menu_with_prefs(&MemoryPilots::new());
    harness.new_pilot("Ada");

    // P, a click on Hyperspace Effects, and Return turn the effects off.
    harness.press(Key::Char('p'));
    assert_eq!(harness.showing(), Showing::Preferences);
    let at = harness.hyperspace_effects_box();
    harness.click(at);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::Flight);
    assert!(!harness.flight().hyperspace_effects());

    // J: the stars streak with no fade, the ship arrives as the streak
    // ends, the arrival shows solid white once, and the jump is over.
    harness.out_towards_beta();
    harness.press(Key::Char('j'));
    let mut streaking = None;
    let mut arrived = None;
    let mut whites = 0;
    for n in 0..600_u64 {
        let frame = harness.frame();
        assert!(!fading(&frame), "frame {n} fades");
        whites += usize::from(white(&frame));
        if streaking.is_none() && harness.flight().jump_effect().is_some() {
            streaking = Some(n);
        }
        if arrived.is_none() && harness.session().system() == SystemId(129) {
            arrived = Some(n);
            assert!(white(&frame), "the arrival frame is white");
        }
        if let Some(at) = arrived
            && harness.flight().jump_effect().is_none()
        {
            let over = n - at;
            let flash_frames = ARRIVAL_FLASH_FOR.as_millis() as u64 * 60 / 1000 + 1;
            assert!(over <= flash_frames, "over {over} frames after arriving");
            break;
        }
    }
    let streaking = streaking.expect("streaked");
    let arrived = arrived.expect("arrived in Beta");
    let streak_frames = STREAK_FOR.as_millis() as u64 * 60 / 1000;
    let took = arrived - streaking;
    assert!(
        streak_frames - 1 <= took && took <= streak_frames + 1,
        "arrived {took} frames after the streak began"
    );
    assert!(whites >= 1, "a white arrival frame");
    assert!(harness.flight().jump_effect().is_none(), "the jump is over");

    // P, Tab to the box, Space and Return turn them back on: the next jump
    // fades out and in again.
    harness.press(Key::Char('p'));
    for _ in 0..3 {
        harness.press(Key::Tab);
    }
    harness.press(Key::Space);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::Flight);
    assert!(harness.flight().hyperspace_effects());
    harness.run(1);
    harness.out_towards(130);
    harness.press(Key::Char('j'));
    let mut faded_out = false;
    let mut faded_in = false;
    for _ in 0..600 {
        let frame = harness.frame();
        let gamma = harness.session().system() == SystemId(130);
        faded_out |= !gamma && fading(&frame);
        faded_in |= gamma && fading(&frame);
        if gamma && harness.flight().jump_effect().is_none() {
            break;
        }
    }
    assert!(faded_out && faded_in, "out {faded_out}, in {faded_in}");
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
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture())
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

impl Harness {
    /// In flight over [`gate_data`], L pressed twice over HG-Alpha: the
    /// hypergate map is open, and Gamma clicked on it.
    fn gamma_picked() -> Self {
        let mut harness = Self::flying_over(gate_data(), SharedChance::default());
        harness.frame();
        harness.press(Key::Char('l'));
        harness.press(Key::Char('l'));
        assert_eq!(harness.showing(), Showing::FlightMap);
        assert_eq!(harness.flight().course_map().mode(), MapMode::Hypergate);
        let gamma = harness.on_map(130);
        harness.click(gamma);
        assert_eq!(harness.session().system(), SystemId(128), "not yet");
        harness
    }

    /// Sends redraws until the gate's effect is over, and returns the
    /// last frame.
    fn come_out(&mut self) -> Frame {
        for _ in 0..600 {
            let frame = self.frame();
            if self.flight().jump_effect().is_none() {
                return frame;
            }
        }
        panic!("the gate's effect never ended");
    }
}

#[test]
fn a_hypergate_picked_on_the_map_brings_the_ship_out_of_its_linked_gate() {
    let mut harness = Harness::gamma_picked();
    harness.press(Key::Char('m'));
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.session().system(), SystemId(130));
    assert_eq!(harness.ship().position, nova_sim::Vec2::new(50.0, 60.0));
    assert_eq!(harness.ship().heading, 90.0);
    assert_eq!(harness.session().reserves().fuel.now, 300.0, "no fuel");
    let frame = harness.come_out();
    assert!(
        shows(
            &frame,
            "Exiting hypergate in the Gamma system on June 23, 1177."
        ),
        "{:?}",
        texts(&frame)
    );
}

#[test]
fn escape_on_the_hypergate_map_enters_the_gate_too() {
    let mut harness = Harness::gamma_picked();
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.session().system(), SystemId(130));
    assert_eq!(harness.ship().position, nova_sim::Vec2::new(50.0, 60.0));
}
