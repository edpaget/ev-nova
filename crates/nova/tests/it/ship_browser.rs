//! The app opening on the ship browser over synthetic game data, wired to
//! the renderer and the recording Gpu and driven by key events.

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};

use crate::common::Frames;
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, QuadInstance};
use nova_rsrc::fixture::ForkBuilder;
use nova_view::Key;

/// A 1024x768 window at scale 1.
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

/// A `shïp` costing 12,000 credits with speed 300, armour 40, shields 80.
fn ship() -> Vec<u8> {
    ShipBuilder::new()
        .shield(80)
        .speed(300)
        .armor(40)
        .cost(12_000)
        .build()
}

/// A `shän` with base image `base` (one set of 4 rotations) and glow and
/// lights images `glow` and `lights`.
fn anim(base: i16, glow: i16, lights: i16) -> Vec<u8> {
    ShipAnimBuilder::new()
        .base_image_id(base)
        .base_set_count(1)
        .glow_image_id(glow)
        .light_image_id(lights)
        .frames_per(4)
        .build()
}

/// An `rlëD` of 4 `size` x `size` frames, each a different colour.
fn sheet(size: u16) -> Vec<u8> {
    [0x7C00, 0x03E0, 0x001F, 0x7FFF]
        .into_iter()
        .fold(RledBuilder::new(size, size), |sheet, color| {
            sheet.frame(|f| {
                (0..size).fold(f, |f, _| f.line().pixels(&vec![color; usize::from(size)]))
            })
        })
        .build()
}

/// Ship 128, the Shuttle, with base, glow and lights sheets; ship 129,
/// the Courier, whose `shän` names a missing sheet.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship())
        .resource(Ship::TYPE, 129, Some(b"Courier"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &anim(1000, 1100, 1200))
        .resource(ShipAnim::TYPE, 129, None, &anim(1300, -1, -1))
        .resource(RLED, 1000, None, &sheet(8))
        .resource(RLED, 1100, None, &sheet(16))
        .resource(RLED, 1200, None, &sheet(16))
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
    /// The app opening on the ship browser.
    fn new() -> Self {
        let data = data();
        let app = App::new(&FakeWindow, Rc::clone(&data), start_screen(data));
        Self {
            app,
            gpu: RecordingGpu::new(),
            clock: Duration::ZERO,
        }
    }

    fn send(&mut self, event: WindowEvent) -> Control {
        self.app.handle(event, &mut FakeWindow, &mut self.gpu)
    }

    fn press(&mut self, key: Key) {
        assert_eq!(
            self.send(WindowEvent::Key {
                key,
                pressed: true,
                repeat: false,
            }),
            Control::Continue
        );
    }

    /// Sends a redraw 1/60 s after the last and returns its frame.
    fn frame(&mut self) -> Frame {
        self.clock += Duration::from_micros(16_667);
        self.send(WindowEvent::Redraw {
            elapsed: self.clock,
        });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        // Only flight draws a world layer.
        assert_eq!(frame.world, [0_usize; 0], "no world");
        frame
    }

    /// The last frame drawn.
    fn last_frame(&self) -> Frame {
        (*self.gpu.submits().last().expect("a frame")).clone()
    }
}

crate::common::frames!(Harness, frame);

fn sprite_quads(frame: &Frame) -> Vec<QuadInstance> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.clone(),
            _ => Vec::new(),
        })
        .collect()
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

#[test]
fn the_app_opens_on_the_ship_browser() {
    let harness = Harness::new();
    assert_eq!(harness.app.screen().showing(), Showing::ShipBrowser);
    assert_eq!(
        harness.app.screen().ship_browser().selected(),
        Some(nova_view::ships::ShipId(128))
    );
}

#[test]
fn the_first_frame_brings_the_ships_sprites_and_name_to_the_gpu() {
    let mut harness = Harness::new();
    let frame = harness.frame();
    assert!(!sprite_quads(&frame).is_empty(), "{:?}", frame.batches);
    assert!(shows(&frame, "Shuttle"), "{:?}", texts(&frame));
}

#[test]
fn right_and_left_reach_the_browser_and_change_the_ship_drawn() {
    let mut harness = Harness::new();
    harness.press(Key::Right);
    let frame = harness.frame();
    assert!(shows(&frame, "Courier"), "{:?}", texts(&frame));

    harness.press(Key::Left);
    let frame = harness.frame();
    assert!(shows(&frame, "Shuttle"), "{:?}", texts(&frame));
}

#[test]
fn later_redraws_turn_the_ship() {
    let mut harness = Harness::new();
    let first = sprite_quads(&harness.frame());
    harness.settle_until("the ship's next frame", |harness| {
        harness.app.screen().ship_browser().frame() != Some(0)
    });
    let later = sprite_quads(&harness.last_frame());
    assert_ne!(first[0].uv, later[0].uv, "the base shows its next frame");
}
