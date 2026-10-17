//! The app entering a system from the galaxy map and going back, over
//! synthetic game data, wired to the renderer and the recording Gpu and
//! driven by key and mouse events.
//!
//! This is the developer path: the viewer map Tab reaches enters systems
//! with Return. In play, the map opened from flight plots a course instead
//! (`hyperspace.rs`).

// Positions here are whole numbers and quarter-second moves, exact in
// floating point.
#![allow(clippy::float_cmp)]

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record, SystemId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, QuadInstance, Rect, SolidQuad};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
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
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
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
        harness.frame_after(Duration::from_millis(16));
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
        let at = map.view().world_to_screen(system.position());
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

    fn camera(&self) -> Point {
        self.app
            .screen()
            .system_view()
            .expect("a system open")
            .camera()
            .center()
    }
}

fn shape(frame: &Frame) -> Vec<(&'static str, usize)> {
    frame
        .batches
        .iter()
        .map(|batch| match batch {
            Batch::Sprites { quads, .. } => ("sprites", quads.len()),
            Batch::Solid(quads) => ("solid", quads.len()),
            Batch::Text(runs) => ("text", runs.len()),
        })
        .collect()
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

/// The stars' centres.
fn stars(frame: &Frame) -> Vec<Point> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.clone(),
            _ => Vec::new(),
        })
        .map(|quad: SolidQuad| {
            let [a, _, c, _] = quad.corners;
            Point::new(f32::midpoint(a.x, c.x), f32::midpoint(a.y, c.y))
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

/// A `w` x `h` rectangle centred on (`x`, `y`).
fn centred(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        x: x - w / 2.0,
        y: y - h / 2.0,
        w,
        h,
    }
}

/// Whether some star of `before` is at `after`'s height and `dx` to its
/// right.
fn moved_by(before: &[Point], after: Point, dx: f32) -> bool {
    before
        .iter()
        .any(|b| b.y == after.y && (b.x - dx - after.x).abs() < 1e-3)
}

const QUARTER: Duration = Duration::from_millis(250);

#[test]
fn return_enters_a_system_whose_stellars_and_stars_reach_the_gpu_and_escape_goes_back() {
    let mut harness = Harness::on_the_map();
    harness.select(128);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::System);

    let first = harness.frame_after(Duration::from_millis(16));
    let shape = shape(&first);
    assert_eq!(shape.len(), 3, "{shape:?}");
    assert_eq!(shape[0].0, "solid");
    assert!(shape[0].1 > 0, "stars");
    // The two stellars; then their names, the title, the camera and help
    // lines, and the router's hint.
    assert_eq!(&shape[1..], [("sprites", 2), ("text", 6)]);
    let start = quads(&first);
    assert_eq!(start[0].dest, centred(512.0, 384.0, 8.0, 8.0));
    assert_eq!(start[1].dest, centred(812.0, 184.0, 6.0, 6.0));
    for expected in ["Alpha (sÿst 128)", "Alpha Prime", "Alpha Station"] {
        assert!(
            shows(&first, expected),
            "{expected:?} in {:?}",
            texts(&first)
        );
    }

    // Holding Right for a quarter of a second moves the stellars 240 left,
    // and each layer of stars by its share of that.
    harness.key(Key::Right, true);
    let moved = harness.frame_after(QUARTER);
    let now = quads(&moved);
    assert_eq!(now[0].dest, centred(272.0, 384.0, 8.0, 8.0));
    assert_eq!(now[1].dest, centred(572.0, 184.0, 6.0, 6.0));
    let (before, after) = (stars(&first), stars(&moved));
    for dx in [60.0, 120.0, 180.0] {
        let count = after.iter().filter(|&&p| moved_by(&before, p, dx)).count();
        assert!(count > 0, "no star moved {dx} left");
    }

    // Releasing it stops the camera.
    harness.key(Key::Right, false);
    let stopped = harness.frame_after(QUARTER);
    assert_eq!(quads(&stopped)[0].dest, now[0].dest);
    assert_eq!(stars(&stopped), after);

    // The station animates; the planet, with one frame, does not.
    let later = harness.frame_after(Duration::from_secs(1) / 10);
    let (station, planet) = (quads(&later)[1], quads(&later)[0]);
    assert_ne!(station.uv, start[1].uv, "the station's frame changed");
    assert_ne!(station.uv, quads(&stopped)[1].uv);
    assert_eq!(planet.uv, start[0].uv);

    // Escape goes back to the map, with Alpha still selected.
    harness.key(Key::Escape, true);
    harness.key(Key::Escape, false);
    assert_eq!(harness.showing(), Showing::GalaxyMap);
    assert_eq!(
        harness.app.screen().galaxy_map().selected(),
        Some(SystemId(128))
    );
    let map = harness.frame_after(Duration::from_millis(16));
    for expected in ["Alpha (sÿst 128)", "Enter system (Return)"] {
        assert!(shows(&map, expected), "{expected:?} in {:?}", texts(&map));
    }
    assert!(!shows(&map, "Camera (240, 0)"));

    // And a second Escape quits.
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
    harness.frame_after(Duration::from_millis(16));

    harness.key(Key::Down, true);
    harness.frame_after(QUARTER);
    harness.key(Key::Down, false);
    assert_eq!(harness.camera(), Point::new(0.0, 240.0));

    harness.press(Key::Tab);
    assert_eq!(harness.showing(), Showing::ShipBrowser);
    harness.frame_after(QUARTER);
    harness.press(Key::Tab);
    assert_eq!(harness.showing(), Showing::System);
    let frame = harness.frame_after(Duration::from_millis(16));
    assert_eq!(harness.camera(), Point::new(0.0, 240.0));
    assert!(shows(&frame, "Camera (0, 240)"), "{:?}", texts(&frame));
}

#[test]
fn losing_focus_while_holding_a_key_stops_the_camera() {
    let mut harness = Harness::on_the_map();
    harness.select(128);
    harness.press(Key::Enter);
    harness.frame_after(Duration::from_millis(16));

    harness.key(Key::Right, true);
    harness.frame_after(QUARTER);
    assert_eq!(harness.camera(), Point::new(240.0, 0.0));
    harness.send(WindowEvent::FocusLost);
    // Right's release went to another window.
    harness.frame_after(QUARTER);
    assert_eq!(harness.camera(), Point::new(240.0, 0.0));
}
