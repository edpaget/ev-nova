//! The app entering a system from the galaxy map and going back, over
//! synthetic game data, wired to the renderer and the recording Gpu and
//! driven by key and mouse events.
//!
//! This is the developer path: the viewer map Tab reaches enters systems
//! with Return. In play, the map opened from flight plots a course instead
//! (`hyperspace.rs`).
//!
//! Where the stellars and stars are drawn, how fast the camera moves, the
//! stations' animation and the map's selection kept are nova-view's,
//! pinned there.

use crate::common::Frames;

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record, SystemId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_view::galaxy::map::ENTER_BUTTON;
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

/// A `spöb` at (`x`, `y`) of graphic type `graphic_type`, animating every
/// `delay` ticks.
fn stellar(x: i16, y: i16, graphic_type: i16, delay: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
    put_i16s(&mut bytes, 0x22, &[delay]);
    bytes
}

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`, each a different
/// colour.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, frame| {
            let color = 0x7C00 >> (frame * 5 % 15);
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![color; size.into()])))
        })
        .build()
}

/// Alpha (128) at map (0, 0) holds Alpha Prime (128) at (0, 0), type 0:
/// `spïn` 1000, `rlëD` 1000, one 8 x 8 frame; and Alpha Station (129) at
/// (300, -200), type 1: `spïn` 1001, `rlëD` 1001, 4 frames of 6 x 6, each
/// shown for 3 ticks. Beta (129) at map (600, 0) holds nothing.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[128, 129]))
        .resource(System::TYPE, 129, Some(b"Beta"), &system(600, &[]))
        .resource(
            Stellar::TYPE,
            128,
            Some(b"Alpha Prime"),
            &stellar(0, 0, 0, 0),
        )
        .resource(
            Stellar::TYPE,
            129,
            Some(b"Alpha Station"),
            &stellar(300, -200, 1, 3),
        )
        .resource(Spin::TYPE, 1000, None, &spin(1000))
        .resource(Spin::TYPE, 1001, None, &spin(1001))
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(RLED, 1001, None, &sheet(4, 6))
        .build()
        .bytes;
    Rc::new(one_file(fork).load())
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    clock: Duration,
}

impl Harness {
    /// The app, on the galaxy map after one frame.
    fn on_the_map() -> Self {
        let data = data();
        let app = App::new(&FakeWindow, Rc::clone(&data), start_screen(data));
        let mut harness = Self {
            app,
            gpu: RecordingGpu::new(),
            clock: Duration::ZERO,
        };
        harness.press(Key::Tab);
        harness.frame();
        assert_eq!(harness.showing(), Showing::GalaxyMap);
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

    /// Clicks on system `id`'s dot.
    fn select(&mut self, id: i16) {
        let map = self.app.screen().galaxy_map();
        let system = map.model().system(SystemId(id)).expect("a system");
        let at = map.transform().world_to_screen(system.position());
        self.click(at);
        assert_eq!(
            self.app.screen().galaxy_map().selected(),
            Some(SystemId(id))
        );
    }

    /// Sends a redraw `dt` after the last and returns its frame.
    fn frame_after(&mut self, dt: Duration) -> Frame {
        self.clock += dt;
        self.send(WindowEvent::Redraw {
            elapsed: self.clock,
        });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// Sends a redraw a frame (16 ms) after the last and returns its frame.
    fn frame(&mut self) -> Frame {
        self.frame_after(FRAME)
    }

    fn camera(&self) -> Point {
        self.app
            .screen()
            .system_view()
            .expect("a system open")
            .camera()
            .center()
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

/// A frame's time at 60 a second, as near as whole milliseconds go.
const FRAME: Duration = Duration::from_millis(16);

/// A quarter of a second of redraws.
const QUARTER: Duration = Duration::from_millis(250);

#[test]
fn return_enters_a_system_whose_frame_reaches_the_gpu_and_escape_goes_back() {
    let mut harness = Harness::on_the_map();
    harness.select(128);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::System);

    let first = harness.frame();
    let stars = first
        .batches
        .iter()
        .any(|batch| matches!(batch, Batch::Solid(quads) if !quads.is_empty()));
    let stellars = first
        .batches
        .iter()
        .any(|batch| matches!(batch, Batch::Sprites { quads, .. } if !quads.is_empty()));
    assert!(stars && stellars, "the stars and the stellars");
    assert!(
        shows(&first, "Alpha Prime"),
        "a stellar's name from the data: {:?}",
        texts(&first)
    );

    // Escape goes back to the map, and a second Escape quits.
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::GalaxyMap);
    harness.frame();
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
fn the_enter_button_enters_and_the_system_survives_a_tab_round_trip() {
    let mut harness = Harness::on_the_map();
    harness.select(128);
    harness.click(ENTER_BUTTON.center());
    assert_eq!(harness.showing(), Showing::System);
    harness.frame();
    let start = harness.camera();

    harness.key(Key::Down, true);
    harness.settle_until("the camera moving under Down", |harness| {
        harness.camera() != start
    });
    harness.key(Key::Down, false);
    harness.frame();
    let moved = harness.camera();

    harness.press(Key::Tab);
    assert_eq!(harness.showing(), Showing::ShipBrowser);
    harness.frame_after(QUARTER);
    harness.press(Key::Tab);
    assert_eq!(harness.showing(), Showing::System);
    harness.frame();
    assert_eq!(harness.camera(), moved, "the same system, as it was left");
}

#[test]
fn losing_focus_while_holding_a_key_stops_the_camera() {
    let mut harness = Harness::on_the_map();
    harness.select(128);
    harness.press(Key::Enter);
    harness.frame();
    let start = harness.camera();

    harness.key(Key::Right, true);
    harness.settle_until("the camera moving under Right", |harness| {
        harness.camera() != start
    });
    harness.send(WindowEvent::FocusLost);
    // Right's release went to another window.
    let stopped = harness.camera();
    // Play resumes, so the stop isn't the background's pause.
    harness.send(WindowEvent::FocusGained);
    harness.frame_after(QUARTER);
    assert_eq!(harness.camera(), stopped);
}

crate::common::frames!(Harness, frame);
