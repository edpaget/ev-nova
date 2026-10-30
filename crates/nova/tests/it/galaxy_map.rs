//! The app switching to the galaxy map over synthetic game data, wired to
//! the renderer and the recording Gpu and driven by key and mouse events.
//!
//! This is the developer path: the viewer map Tab reaches, which keeps its
//! "Enter system" button and Return. Play plots courses on the map opened
//! from flight with M, which has neither (`hyperspace.rs`).
//!
//! What the map draws, how far a key pans or zooms it, what a click or a
//! drag selects and the panel's words are nova-view's, pinned there.

use crate::common::Frames;

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::PICT;
use nova_data::graphics::fixture::{DirectBits, PictBuilder};
use nova_data::records::govt::Govt;
use nova_data::records::nebula::Nebula;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record, SystemId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_view::galaxy::map::ENTER_BUTTON;
use nova_view::{Key, MouseButton, Point};

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
    Rc::new(one_file(fork).load())
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

#[test]
fn tab_opens_the_map_and_its_frame_reaches_the_gpu() {
    let (_, frame) = Harness::open_map();
    let nebula = frame
        .batches
        .iter()
        .any(|batch| matches!(batch, Batch::Sprites { quads, .. } if !quads.is_empty()));
    let solids = frame
        .batches
        .iter()
        .any(|batch| matches!(batch, Batch::Solid(quads) if !quads.is_empty()));
    assert!(
        nebula && solids,
        "the nebula's picture and the map's solids"
    );
    assert!(shows(&frame, "Alpha"), "a system's name from the data");
}

/// A press lost with the focus is over: the original's `Button()` goes
/// false wherever the pointer is, and the release is dropped in the
/// background, so nothing else would end it.
#[test]
fn a_press_lost_with_the_focus_drags_the_map_no_further() {
    let (mut harness, _) = Harness::open_map();
    let view = |harness: &Harness| {
        *harness
            .app
            .screen()
            .galaxy_map()
            .view()
            .expect("the viewer")
    };
    let before = view(&harness);
    harness.point(Point::new(500.0, 400.0));
    harness.button(true);
    harness.send(WindowEvent::FocusLost);
    harness.button(false);
    harness.point(Point::new(600.0, 400.0));
    harness.send(WindowEvent::FocusGained);
    assert_eq!(view(&harness), before, "the move passed on as it came back");
    harness.point(Point::new(700.0, 400.0));
    assert_eq!(view(&harness), before, "and the moves after");
}

#[test]
fn keys_reach_the_map() {
    let (mut harness, _) = Harness::open_map();
    let before = harness.system_at(128);
    harness.press(Key::Right);
    harness.settle();
    assert_ne!(harness.system_at(128), before, "Right pans the map");

    let zoom = |harness: &Harness| {
        harness
            .app
            .screen()
            .galaxy_map()
            .view()
            .expect("the viewer")
            .zoom()
    };
    let before = zoom(&harness);
    harness.press(Key::Char('='));
    harness.settle();
    assert_ne!(zoom(&harness), before, "= zooms the map");
}

#[test]
fn the_enter_button_opens_the_selected_system() {
    let (mut harness, _) = Harness::open_map();
    let beta = harness.system_at(129);
    harness.click(beta);
    harness.settle();
    harness.click(ENTER_BUTTON.center());
    assert_eq!(harness.app.screen().showing(), Showing::System);
    let view = harness.app.screen().system_view().expect("open");
    assert_eq!(view.scene().id(), SystemId(129));
}

#[test]
fn a_drag_reaches_the_map() {
    let (mut harness, _) = Harness::open_map();
    let before = harness.system_at(128);

    let start = Point::new(before.x + 200.0, before.y + 100.0);
    harness.point(start);
    harness.button(true);
    harness.point(Point::new(start.x + 100.0, start.y));
    harness.button(false);
    harness.settle();

    assert_ne!(harness.system_at(128), before, "the drag moves the map");
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

crate::common::frames!(Harness, frame);
