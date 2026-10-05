//! The app opening on the ship browser over synthetic game data, wired to
//! the renderer and the recording Gpu and driven by key events.

// Sprite sizes and centres here are small integers, exact in floating
// point.
#![allow(clippy::float_cmp)]

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::desc::Desc;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, QuadInstance};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_view::{Blend, Key};

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

fn put(bytes: &mut [u8], at: usize, value: &[u8]) {
    bytes[at..at + value.len()].copy_from_slice(value);
}

/// A `shïp` costing 12,000 credits with speed 300, armour 40, shields 80.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put(&mut bytes, 0x02, &80_i16.to_be_bytes());
    put(&mut bytes, 0x06, &300_i16.to_be_bytes());
    put(&mut bytes, 0x0E, &40_i16.to_be_bytes());
    put(&mut bytes, 0x30, &12_000_i32.to_be_bytes());
    bytes
}

/// A `shän` with base image `base` (one set of 4 rotations) and glow and
/// lights images `glow` and `lights`.
fn anim(base: i16, glow: i16, lights: i16) -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put(&mut bytes, 0x00, &base.to_be_bytes());
    put(&mut bytes, 0x04, &1_i16.to_be_bytes());
    put(&mut bytes, 0x16, &glow.to_be_bytes());
    put(&mut bytes, 0x1E, &lights.to_be_bytes());
    put(&mut bytes, 0x34, &4_i16.to_be_bytes());
    bytes
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

/// Ship 128, the Shuttle, with base, glow and lights sheets and a
/// description; ship 129, the Courier, whose `shän` names a missing sheet.
fn data() -> Rc<GameData> {
    let mut desc = b"A small craft.".to_vec();
    desc.extend([0, 0xFF, 0xFF]);
    desc.extend([0; 34]);
    let fork = ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship())
        .resource(Ship::TYPE, 129, Some(b"Courier"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &anim(1000, 1100, 1200))
        .resource(ShipAnim::TYPE, 129, None, &anim(1300, -1, -1))
        .resource(RLED, 1000, None, &sheet(8))
        .resource(RLED, 1100, None, &sheet(16))
        .resource(RLED, 1200, None, &sheet(16))
        .resource(Desc::TYPE, 13000, Some(b"Shuttle"), &desc)
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// Ship 128 alone, with base, glow and lights sheets, its lights
/// blinking as the Shuttle's do (`BlinkMode` 1, A=4 B=1 C=2 D=20): lit at
/// ticks 1-3 and 10-12 of every 40.
fn blinking_data() -> Rc<GameData> {
    let mut shan = anim(1000, 1100, 1200);
    for (at, value) in [(0x36, 1_i16), (0x38, 4), (0x3A, 1), (0x3C, 2), (0x3E, 20)] {
        put(&mut shan, at, &value.to_be_bytes());
    }
    let fork = ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &shan)
        .resource(RLED, 1000, None, &sheet(8))
        .resource(RLED, 1100, None, &sheet(16))
        .resource(RLED, 1200, None, &sheet(16))
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    clock: Duration,
}

impl Harness {
    fn new() -> Self {
        Self::over(data())
    }

    /// The app opening on the ship browser over `data`.
    fn over(data: Rc<GameData>) -> Self {
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
        frame
    }
}

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

fn solid_quads(frame: &Frame) -> usize {
    frame
        .batches
        .iter()
        .map(|batch| match batch {
            Batch::Solid(quads) => quads.len(),
            _ => 0,
        })
        .sum()
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
fn the_first_frame_draws_the_ship_its_layers_and_its_text() {
    let mut harness = Harness::new();
    let frame = harness.frame();

    // The base is drawn normally, then the glow and lights added over it.
    assert!(
        matches!(
            &frame.batches[0],
            Batch::Sprites { blend: Blend::Normal, quads, .. } if quads.len() == 1
        ),
        "{:?}",
        frame.batches[0]
    );
    assert!(
        matches!(
            &frame.batches[1],
            Batch::Sprites { blend: Blend::Additive, quads, .. } if quads.len() == 2
        ),
        "{:?}",
        frame.batches[1]
    );
    let quads = sprite_quads(&frame);
    let centre = |quad: &QuadInstance| {
        (
            quad.dest.x + quad.dest.w / 2.0,
            quad.dest.y + quad.dest.h / 2.0,
        )
    };
    // The 16x16 glow and lights frames sit centred on the 8x8 base frame.
    assert_eq!(quads[0].dest.w, 8.0);
    assert_eq!(quads[1].dest.w, 16.0);
    assert_eq!(centre(&quads[1]), centre(&quads[0]));
    assert_eq!(centre(&quads[2]), centre(&quads[0]));
    for expected in [
        "Shuttle",
        "Cost: 12000 credits",
        "Speed: 300",
        "Armour: 40",
        "Shields: 80",
        "A small craft.",
        "Ship 1 of 2 (shïp 128) - Left/Right to browse",
    ] {
        assert!(
            shows(&frame, expected),
            "{expected:?} in {:?}",
            texts(&frame)
        );
    }
    assert_eq!(solid_quads(&frame), 0);
}

#[test]
fn right_shows_the_next_ship_and_its_missing_sprite() {
    let mut harness = Harness::new();
    harness.frame();

    harness.press(Key::Right);
    let frame = harness.frame();

    assert!(shows(&frame, "Courier"), "{:?}", texts(&frame));
    assert!(
        texts(&frame)
            .iter()
            .any(|text| text.starts_with("Sprite unavailable: shïp 129: no rlëD 1300")),
        "{:?}",
        texts(&frame)
    );
    assert_eq!(sprite_quads(&frame), []);
    // The placeholder's box and diagonals.
    assert_eq!(solid_quads(&frame), 6);
}

#[test]
fn right_wraps_round_and_left_goes_back() {
    let mut harness = Harness::new();
    harness.press(Key::Right);
    harness.press(Key::Right);
    let frame = harness.frame();
    assert!(shows(&frame, "Shuttle"), "{:?}", texts(&frame));
    assert_eq!(sprite_quads(&frame).len(), 3);

    harness.press(Key::Left);
    let frame = harness.frame();
    assert!(shows(&frame, "Courier"), "{:?}", texts(&frame));
}

#[test]
fn later_redraws_turn_the_ship() {
    let mut harness = Harness::new();
    // 1/60 s: still frame 0; 2/60 s: frame 1.
    let first = sprite_quads(&harness.frame());
    let second = sprite_quads(&harness.frame());
    assert_eq!(first[0].dest, second[0].dest);
    assert_ne!(first[0].uv, second[0].uv, "the base shows its next frame");
    assert_ne!(first[1].uv, second[1].uv, "the glow follows it");
}

#[test]
fn the_lights_blink_by_the_shan() {
    let mut harness = Harness::over(blinking_data());
    // 1/60 s, tick 0: the lights are off; the base and glow show.
    let first = harness.frame();
    assert_eq!(sprite_quads(&first).len(), 2, "{:?}", first.batches);
    // 2/60 s, tick 1: the lights are added over the glow.
    let second = harness.frame();
    assert_eq!(sprite_quads(&second).len(), 3, "{:?}", second.batches);
    assert!(
        matches!(
            &second.batches[1],
            Batch::Sprites { blend: Blend::Additive, quads, .. } if quads.len() == 2
        ),
        "{:?}",
        second.batches[1]
    );
}
