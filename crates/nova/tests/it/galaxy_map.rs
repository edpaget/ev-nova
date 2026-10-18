//! The app switching to the galaxy map over synthetic game data, wired to
//! the renderer and the recording Gpu and driven by key and mouse events.
//!
//! This is the developer path: the viewer map Tab reaches, which keeps its
//! "Enter system" button and Return. Play plots courses on the map opened
//! from flight with M, which has neither (`hyperspace.rs`).

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::PICT;
use nova_data::graphics::fixture::{DirectBits, PictBuilder};
use nova_data::records::govt::Govt;
use nova_data::records::nebula::Nebula;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record, SystemId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, Rect, SolidQuad};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_view::flight::radar::IRect;
use nova_view::galaxy::NEUTRAL;
use nova_view::galaxy::borders::draw_cells;
use nova_view::galaxy::map::{BUTTON, ENTER_BUTTON, SYSTEM_INSET};
use nova_view::galaxy::nebulae::{NebulaFrame, nebula_rect};
use nova_view::oval::{oval_fill_pixels, oval_frame_pixels, runs};
use nova_view::{Blend, Key, MouseButton, Point};

const BLUE: u32 = 0x002C_2CAF;

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

/// A `sÿst` at (`x`, `y`) owned by `govt` (-1 for none), with these
/// hyperlinks and stellars; every other slot is -1.
fn system(x: i16, y: i16, govt: i16, links: &[i16], stellars: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, y]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    put_i16s(&mut bytes, 0x24, stellars);
    put_i16s(&mut bytes, 0x66, &[govt]);
    bytes
}

/// A grey `size` x `size` `PICT`.
fn picture(size: i16) -> Vec<u8> {
    let bounds = [0, 0, size, size];
    let pixels = vec![0x4210; (size * size) as usize];
    PictBuilder::new(bounds)
        .direct_bits(&DirectBits::rgb555(bounds, &pixels))
        .end()
        .build()
}

/// Alpha (128) at (0, 0), owned by gövt 128, with stellars Alpha Prime
/// and Alpha Station; Beta (129) at (600, 0), independent; Gamma (130) at
/// (0, 300), owned by gövt 128. Alpha and Beta link both ways; Gamma links
/// one way to Alpha. Nebula 128 covers (-20, -20) to (40, 40), with
/// `PICT`s 9500 (30 x 30) and 9502 (60 x 60) and no 9501.
fn data() -> Rc<GameData> {
    let mut govt = vec![0; Govt::SIZE.expect("fixed")];
    govt[0xA4..0xA8].copy_from_slice(&BLUE.to_be_bytes());
    let stellar = vec![0; Stellar::SIZE.expect("fixed")];
    let mut nebula = vec![0; Nebula::SIZE.expect("fixed")];
    put_i16s(&mut nebula, 0, &[-20, -20, 60, 60]);
    let fork = ForkBuilder::new()
        .resource(
            System::TYPE,
            128,
            Some(b"Alpha"),
            &system(0, 0, 128, &[129], &[128, 129]),
        )
        .resource(
            System::TYPE,
            129,
            Some(b"Beta"),
            &system(600, 0, -1, &[128], &[]),
        )
        .resource(
            System::TYPE,
            130,
            Some(b"Gamma"),
            &system(0, 300, 128, &[128], &[]),
        )
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &stellar)
        .resource(Stellar::TYPE, 129, Some(b"Alpha Station"), &stellar)
        .resource(Govt::TYPE, 128, Some(b"Federation"), &govt)
        .resource(Nebula::TYPE, 128, Some(b"Holpa Nebula"), &nebula)
        .resource(PICT, 9500, None, &picture(30))
        .resource(PICT, 9502, None, &picture(60))
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
        let data = data();
        let app = App::new(&FakeWindow, Rc::clone(&data), start_screen(data));
        Self {
            app,
            gpu: RecordingGpu::new(),
            clock: Duration::ZERO,
        }
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue
        );
    }

    fn press(&mut self, key: Key) {
        self.send(WindowEvent::Key {
            key,
            pressed: true,
            repeat: false,
        });
        self.send(WindowEvent::Key {
            key,
            pressed: false,
            repeat: false,
        });
    }

    /// Moves the pointer to `at` (window pixels are logical units here).
    fn point(&mut self, at: Point) {
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        });
    }

    fn button(&mut self, pressed: bool) {
        self.send(WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed,
        });
    }

    fn click(&mut self, at: Point) {
        self.point(at);
        self.button(true);
        self.button(false);
    }

    /// Where system `id` is on screen.
    fn system_at(&self, id: i16) -> Point {
        let map = self.app.screen().galaxy_map();
        let system = map.model().system(SystemId(id)).expect("a system");
        map.transform().world_to_screen(system.position())
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

    /// Opens the map and returns its first frame.
    fn open_map() -> (Self, Frame) {
        let mut harness = Self::new();
        assert_eq!(harness.app.screen().showing(), Showing::ShipBrowser);
        harness.frame();
        harness.press(Key::Tab);
        assert_eq!(harness.app.screen().showing(), Showing::GalaxyMap);
        let frame = harness.frame();
        (harness, frame)
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

fn solid(frame: &Frame) -> Vec<SolidQuad> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// The systems' ovals, in draw order: each run of black fill rows, as its
/// centre, with the colour of the frame drawn next.
fn fills(frame: &Frame) -> Vec<(Point, [f32; 4])> {
    let black = rgba(nova_view::Color::BLACK);
    let quads = solid(frame);
    let mut ovals = Vec::new();
    let mut at = 0;
    while at < quads.len() {
        if quads[at].color != black {
            at += 1;
            continue;
        }
        let (mut min, mut max) = (
            Point::new(f32::MAX, f32::MAX),
            Point::new(f32::MIN, f32::MIN),
        );
        while at < quads.len() && quads[at].color == black {
            for corner in quads[at].corners {
                min = Point::new(min.x.min(corner.x), min.y.min(corner.y));
                max = Point::new(max.x.max(corner.x), max.y.max(corner.y));
            }
            at += 1;
        }
        let frame = quads.get(at).map_or([0.0; 4], |quad| quad.color);
        ovals.push((
            Point::new(f32::midpoint(min.x, max.x), f32::midpoint(min.y, max.y)),
            frame,
        ));
    }
    ovals
}

/// Where the map draws system `id`'s oval: the pixel its point truncates
/// to.
fn pixel_at(harness: &Harness, id: i16) -> Point {
    let at = harness.system_at(id);
    Point::new(at.x.trunc(), at.y.trunc())
}

fn rgba(color: nova_view::Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a].map(|c| f32::from(c) / 255.0)
}

#[test]
fn tab_opens_the_map_and_its_nebula_lines_dots_and_text_reach_the_gpu() {
    let (mut harness, frame) = Harness::open_map();
    let shape: Vec<(&str, usize)> = frame
        .batches
        .iter()
        .map(|batch| match batch {
            Batch::Sprites { quads, .. } => ("sprites", quads.len()),
            Batch::Solid(quads) => ("solid", quads.len()),
            Batch::Text(runs) => ("text", runs.len()),
        })
        .collect();
    // The borders' cells, one rect each; the nebula; 2 links and each
    // system's oval, a row at a time, its black fill then its frame; the 3
    // names, twice; the panel's background and separator; the panel's 3
    // lines and the router's hint.
    let oval = IRect::new(0, 0, 2 * SYSTEM_INSET, 2 * SYSTEM_INSET);
    let rows = runs(&oval_fill_pixels(oval)).len() + runs(&oval_frame_pixels(oval, 1)).len();
    let map = harness.app.screen().galaxy_map();
    let mut cells = nova_view::DrawList::new();
    draw_cells(
        &mut cells,
        map.border_cells(),
        (0, 0),
        IRect::new(0, 0, 1024, 608),
        map.model().govt_colors(),
    );
    assert!(!cells.is_empty(), "the viewer's borders, on by default");
    assert_eq!(
        shape,
        [
            ("solid", cells.len()),
            ("sprites", 1),
            ("solid", 2 + 3 * rows),
            ("text", 6),
            ("solid", 2),
            ("text", 4)
        ]
    );
    // The rest is the map's own drawing: the borders off.
    harness.press(Key::Char('b'));
    let frame = harness.frame();

    // 600 x 300 fits at 133%; the nebula fills its scaled rectangle,
    // truncated as `_MapDialogUpdate` truncates it, with `addMax`.
    let map = harness.app.screen().galaxy_map();
    let view = *map.view().expect("the viewer");
    assert_eq!(view.zoom(), 4);
    let frame_of_view = NebulaFrame {
        origin: Point::new(0.0, 0.0),
        centre: (512, 304),
        zoom: view.transform().zoom(),
        scroll: (f64::from(view.center().x), f64::from(view.center().y)),
        area: IRect::new(0, 0, 1024, 608),
    };
    let rect = nebula_rect(&map.model().catalog_nebulae()[0], &frame_of_view);
    let corner = view.world_to_screen(Point::new(-20.0, -20.0));
    assert_eq!(
        (rect.left, rect.top),
        (corner.x.trunc() as i32, corner.y.trunc() as i32)
    );
    let Batch::Sprites { quads, blend, .. } = &frame.batches[0] else {
        panic!("{:?}", frame.batches[0]);
    };
    assert_eq!(*blend, Blend::Max);
    assert_eq!(
        quads[0].dest,
        Rect {
            x: rect.left as f32,
            y: rect.top as f32,
            w: (rect.right - rect.left) as f32,
            h: (rect.bottom - rect.top) as f32,
        }
    );

    let blue = rgba(nova_view::Color::from_rgb24(BLUE));
    let neutral = rgba(NEUTRAL);
    assert_eq!(
        fills(&frame),
        [
            (pixel_at(&harness, 128), blue),
            (pixel_at(&harness, 129), neutral),
            (pixel_at(&harness, 130), blue),
        ]
    );
    assert!(shows(&frame, "Click a system to see its stellars"));
}

#[test]
fn keys_reach_the_map() {
    let (mut harness, _) = Harness::open_map();
    // The borders off, so that the black fills are the systems' ovals.
    harness.press(Key::Char('b'));
    let before = harness.frame();

    harness.press(Key::Right);
    let after = harness.frame();
    let moved: Vec<Point> = fills(&after).into_iter().map(|(p, _)| p).collect();
    let expected: Vec<Point> = fills(&before)
        .into_iter()
        .map(|(p, _)| Point::new(p.x - 64.0, p.y))
        .collect();
    for (moved, expected) in moved.iter().zip(&expected) {
        assert!(
            (moved.x - expected.x).abs() < 1e-3,
            "{moved:?} {expected:?}"
        );
        assert_eq!(moved.y, expected.y);
    }

    let apart = |frame: &Frame| {
        let dots = fills(frame);
        dots[1].0.x - dots[0].0.x
    };
    let at_133 = apart(&after);
    harness.press(Key::Char('='));
    let zoomed = harness.frame();
    assert_eq!(
        harness
            .app
            .screen()
            .galaxy_map()
            .view()
            .expect("the viewer")
            .zoom(),
        5
    );
    // Whole pixels, each point truncated.
    assert!((apart(&zoomed) - at_133 * 4.0 / 3.0).abs() <= 1.0);
    assert!(shows(&zoomed, "Zoom 178%"), "{:?}", texts(&zoomed));
}

#[test]
fn clicks_reach_the_map_and_select_systems() {
    let (mut harness, _) = Harness::open_map();

    let beta = harness.system_at(129);
    harness.click(beta);
    let frame = harness.frame();
    assert!(shows(&frame, "Beta (sÿst 129)"), "{:?}", texts(&frame));
    assert!(shows(&frame, "No stellars"));

    let alpha = harness.system_at(128);
    harness.click(alpha);
    let frame = harness.frame();
    for expected in ["Alpha (sÿst 128)", "Alpha Prime", "Alpha Station"] {
        assert!(
            shows(&frame, expected),
            "{expected:?} in {:?}",
            texts(&frame)
        );
    }

    harness.click(Point::new(alpha.x + 200.0, alpha.y + 100.0));
    let frame = harness.frame();
    assert!(shows(&frame, "Click a system to see its stellars"));
    assert!(!shows(&frame, "Enter system (Return)"));
}

#[test]
fn a_selected_system_shows_the_enter_button_which_opens_it() {
    let (mut harness, unselected) = Harness::open_map();
    let button = rgba(BUTTON);
    let buttons = |frame: &Frame| {
        solid(frame)
            .into_iter()
            .filter(|quad| quad.color == button)
            .map(|quad| {
                // The rectangle the quad covers, whatever its corner order.
                let xs = quad.corners.map(|p| p.x);
                let ys = quad.corners.map(|p| p.y);
                let min = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
                let max = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
                (Point::new(min(xs), min(ys)), Point::new(max(xs), max(ys)))
            })
            .collect::<Vec<_>>()
    };
    assert!(buttons(&unselected).is_empty());

    let beta = harness.system_at(129);
    harness.click(beta);
    let frame = harness.frame();
    assert!(
        shows(&frame, "Enter system (Return)"),
        "{:?}",
        texts(&frame)
    );
    assert_eq!(buttons(&frame), [(ENTER_BUTTON.min, ENTER_BUTTON.max)]);

    harness.click(ENTER_BUTTON.center());
    assert_eq!(harness.app.screen().showing(), Showing::System);
    let view = harness.app.screen().system_view().expect("open");
    assert_eq!(view.scene().id(), SystemId(129));
}

#[test]
fn a_drag_reaches_the_map_and_keeps_the_selection() {
    let (mut harness, _) = Harness::open_map();
    let beta = harness.system_at(129);
    harness.click(beta);
    let before = harness.system_at(128);

    let start = Point::new(beta.x - 300.0, beta.y + 100.0);
    harness.point(start);
    harness.button(true);
    harness.point(Point::new(start.x + 100.0, start.y));
    harness.button(false);

    let after = harness.system_at(128);
    assert!((after.x - (before.x + 100.0)).abs() < 1e-3, "{after:?}");
    assert_eq!(after.y, before.y);
    let frame = harness.frame();
    assert!(shows(&frame, "Beta (sÿst 129)"), "{:?}", texts(&frame));
}

#[test]
fn the_map_keeps_its_view_and_selection_across_a_tab_round_trip() {
    let (mut harness, _) = Harness::open_map();
    harness.press(Key::Char('-'));
    let alpha = harness.system_at(128);
    harness.click(alpha);
    let map = harness.app.screen().galaxy_map();
    let (view, selected) = (*map.view().expect("the viewer"), map.selected());

    harness.press(Key::Tab);
    assert_eq!(harness.app.screen().showing(), Showing::ShipBrowser);
    let frame = harness.frame();
    assert!(!shows(&frame, "Alpha (sÿst 128)"));
    harness.press(Key::Tab);
    let frame = harness.frame();

    let map = harness.app.screen().galaxy_map();
    assert_eq!(
        (*map.view().expect("the viewer"), map.selected()),
        (view, selected)
    );
    assert_eq!(selected, Some(SystemId(128)));
    assert!(shows(&frame, "Alpha (sÿst 128)"), "{:?}", texts(&frame));
}
