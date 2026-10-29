//! The app flying the first `chär`'s ship in its starting system, over
//! synthetic game data, wired to the renderer and the recording Gpu and
//! driven only by key events and redraws, as the window sends them.
//!
//! - F enters flight, and its frame (the stellars, their names and the
//!   HUD) reaches the Gpu.
//! - A held Up reaches flight, and losing focus releases it.
//! - The same keys fly the same at 30, 60 and 120 frames a second.
//! - Escape goes back to where flight was entered, and without a `chär`
//!   F still enters flight.
//!
//! What the frame draws, how the ship flies and which system it starts
//! in are nova-view's and nova-sim's, pinned there.

use crate::common::Frames;

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
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
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::ShipState;
use nova_view::Key;
use nova_view::flight::FlightView;

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

/// A `chär` starting in ship 128, its systems -1, 999 (missing), 128 and
/// -1: the first that exists is 128.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, -1, 999, 128, -1]);
    bytes
}

/// A `shïp` with `Accel` 300, `Speed` 300 and `Maneuver` 30: 3 pixels a
/// tick at most, 0.1 more a tick, 3° a tick. Its `Shield` is 30, `Fuel`
/// 300 and `Armor` 45.
fn ship() -> Vec<u8> {
    ShipBuilder::new()
        .shield(30)
        .accel(300)
        .speed(300)
        .maneuver(30)
        .fuel(300)
        .armor(45)
        .build()
}

/// Writes 24-bit colours from `at`.
fn put_u32s(bytes: &mut [u8], at: usize, values: &[u32]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 4 * i..at + 4 * i + 4].copy_from_slice(&value.to_be_bytes());
    }
}

/// The colours of the status bar fixture's shield, armour and fuel bars and
/// radar dots.
const SHIELD: u32 = 0x0000_00FF;
const ARMOR: u32 = 0x00FF_0000;
const FUEL: u32 = 0x00FF_FF00;
const RADAR: u32 = 0x0000_FF00;
const DIM_RADAR: u32 = 0x0000_8000;

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
fn interface() -> Vec<u8> {
    let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
    put_u32s(&mut bytes, 0x00, &[0x00FF_FFFF, 0x0080_8080]);
    put_i16s(&mut bytes, 0x08, &[8, 8, 184, 184]);
    put_u32s(&mut bytes, 0x10, &[RADAR, DIM_RADAR]);
    put_i16s(&mut bytes, 0x18, &[199, 35, 206, 184]);
    put_u32s(&mut bytes, 0x20, &[SHIELD]);
    put_i16s(&mut bytes, 0x24, &[216, 35, 223, 184]);
    put_u32s(&mut bytes, 0x2C, &[ARMOR]);
    put_i16s(&mut bytes, 0x30, &[234, 35, 241, 184]);
    put_u32s(&mut bytes, 0x38, &[FUEL, 0x0080_8000]);
    put_i16s(&mut bytes, 0x40, &[254, 8, 286, 184]);
    put_i16s(&mut bytes, 0x48, &[300, 8, 315, 184]);
    put_i16s(&mut bytes, 0x50, &[330, 8, 442, 184]);
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

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations,
/// with an engine glow, `rlëD` 2100.
fn ship_anim() -> Vec<u8> {
    ShipAnimBuilder::new()
        .base_image_id(2000)
        .base_set_count(1)
        .glow_image_id(2100)
        .frames_per(36)
        .build()
}

/// An independent `sÿst` at map (`x`, 0) with these stellars; every other
/// slot is -1.
fn system(x: i16, stellars: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, stellars);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// A still `spöb` at (`x`, `y`) of graphic type `graphic_type`.
fn stellar(x: i16, y: i16, graphic_type: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
    bytes
}

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`, each its own colour.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, frame| {
            let color = 0x0400 + frame;
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![color; size.into()])))
        })
        .build()
}

/// The first `chär` (if `with_character`) flies ship 128 from Alpha (128),
/// which holds Alpha Prime (128) at (0, -600), an 8 x 8 sprite, and Alpha
/// Station (129) at (300, -200), a 6 x 6 one. The ship's sheet is 36
/// rotations of 1 x 1, its glow's 36 of 3 x 3. Beta (129) holds nothing.
/// The status bar is `ïntf` 128, over a 194 x 16 `PICT` 700.
fn data(with_character: bool) -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim())
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2100, None, &sheet(36, 3))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[128, 129]))
        .resource(System::TYPE, 129, Some(b"Beta"), &system(600, &[]))
        .resource(
            Stellar::TYPE,
            128,
            Some(b"Alpha Prime"),
            &stellar(0, -600, 0),
        )
        .resource(
            Stellar::TYPE,
            129,
            Some(b"Alpha Station"),
            &stellar(300, -200, 1),
        )
        .resource(Spin::TYPE, 1000, None, &spin(1000))
        .resource(Spin::TYPE, 1001, None, &spin(1001))
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(RLED, 1001, None, &sheet(1, 6))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture());
    if with_character {
        fork = fork.resource(Character::TYPE, 128, Some(b"Pilot"), &character());
    }
    Rc::new(one_file(fork.build().bytes).load())
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    /// Display frames a second.
    fps: u64,
    /// Frames sent since flight was entered.
    frames: u64,
    /// The flight keys held.
    held: Vec<Key>,
}

impl Harness {
    /// The app on the ship browser, before any frame.
    fn new(fps: u64, with_character: bool) -> Self {
        let data = data(with_character);
        let screen = start_screen(Rc::clone(&data));
        Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            fps,
            frames: 0,
            held: Vec::new(),
        }
    }

    /// The app in flight, entered from the ship browser with F, before any
    /// frame.
    fn flying(fps: u64) -> Self {
        let mut harness = Self::new(fps, true);
        harness.press(Key::Char('f'));
        assert_eq!(harness.showing(), Showing::Flight);
        harness
    }

    fn handle(&mut self, event: WindowEvent) -> Control {
        self.app.handle(event, &mut FakeWindow, &mut self.gpu)
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(self.handle(event), Control::Continue, "{event:?}");
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

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn flight(&self) -> &FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn ship(&self) -> ShipState {
        *self.flight().session().expect("flying").player()
    }

    /// Sends the next redraw, at `frames / fps` seconds to the nanosecond
    /// (a display clock's reading, not a sum of rounded frame times), and
    /// returns its frame.
    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / self.fps);
        self.send(WindowEvent::Redraw { elapsed });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// Sends `seconds` of redraws, checking the ship never passes its top
    /// speed, and returns the last frame.
    fn run(&mut self, seconds: f64) -> Frame {
        let count = (seconds * self.fps as f64).round() as u64;
        let max_speed = self.flight().session().expect("flying").top_speed();
        let mut last = None;
        for _ in 0..count {
            last = Some(self.frame());
            let speed = self.ship().velocity.length();
            assert!(speed <= max_speed + 1e-4, "{speed} past {max_speed}");
        }
        last.expect("at least one frame")
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

#[test]
fn f_enters_flight_and_its_frame_reaches_the_gpu() {
    let mut harness = Harness::flying(60);
    assert!(harness.flight().session().is_ok(), "a session flown");

    let first = harness.frame();
    let sprites = first
        .batches
        .iter()
        .any(|batch| matches!(batch, Batch::Sprites { .. }));
    let solids = first
        .batches
        .iter()
        .any(|batch| matches!(batch, Batch::Solid(_)));
    assert!(sprites && solids, "sprites and the HUD's solids");
    assert!(
        texts(&first).contains(&"Alpha Prime".to_owned()),
        "a stellar's name from the data: {:?}",
        texts(&first)
    );
}

/// Flies one open-loop key script, its keys changing only on half-second
/// boundaries, for three seconds at `fps`, and returns the ship.
fn scripted(fps: u64) -> ShipState {
    let mut harness = Harness::flying(fps);
    let script: [&[Key]; 6] = [
        &[Key::Up],
        &[Key::Up, Key::Left],
        &[Key::Left],
        &[Key::Down],
        &[Key::Down],
        &[],
    ];
    for keys in script {
        harness.hold(keys);
        harness.run(0.5);
    }
    harness.ship()
}

#[test]
fn the_same_keys_fly_the_same_through_the_app_at_30_60_and_120_fps() {
    let at_30 = scripted(30);
    assert!(at_30.position.y < 0.0, "{at_30:?}");
    assert!(at_30.heading > 0.0, "{at_30:?}");
    assert_eq!(scripted(60), at_30);
    assert_eq!(scripted(120), at_30);
}

#[test]
fn escape_goes_back_to_where_flight_was_entered_and_then_quits_as_before() {
    let mut harness = Harness::new(60, true);
    harness.press(Key::Tab);
    assert_eq!(harness.showing(), Showing::GalaxyMap);
    harness.press(Key::Char('f'));
    assert_eq!(harness.showing(), Showing::Flight);
    harness.frame();
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::GalaxyMap);
    let map = harness.frame();
    assert!(
        texts(&map).contains(&"Tab: ships / galaxy map   F: fly".to_owned()),
        "{:?}",
        texts(&map)
    );
    assert_eq!(
        harness.handle(WindowEvent::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false,
        }),
        Control::Exit
    );
}

#[test]
fn losing_focus_while_holding_up_stops_the_thrust() {
    let mut harness = Harness::flying(60);
    harness.key(Key::Up, true);
    harness.settle_until("Up's thrust", |harness| {
        harness.ship().velocity.length() > 0.0
    });
    harness.send(WindowEvent::FocusLost);
    // Up's release went to another window.
    let coasting = harness.ship();
    harness.run(0.5);
    assert_eq!(harness.ship().velocity, coasting.velocity);
}

#[test]
fn without_a_chär_f_still_enters_flight_and_escape_goes_back() {
    let mut harness = Harness::new(60, false);
    harness.press(Key::Char('f'));
    assert_eq!(harness.showing(), Showing::Flight);
    let frame = harness.frame();
    assert!(harness.flight().session().is_err(), "no session");
    assert!(!texts(&frame).is_empty(), "the failure drawn");
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::ShipBrowser);
}

crate::common::frames!(Harness, frame);
