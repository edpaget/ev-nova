//! The app wired to its screen router over minimal synthetic game data
//! (one ship and one system), the renderer and the recording Gpu, driven
//! by synthetic window events.

// Sizes here are small powers of two, exact in floating point.
#![allow(clippy::float_cmp)]

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, PixelRect};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_view::Key;

/// A window of settable size that counts redraw requests.
struct FakeWindow {
    size_px: (u32, u32),
    scale_factor: f64,
    redraws: usize,
}

impl WindowPort for FakeWindow {
    fn size_px(&self) -> (u32, u32) {
        self.size_px
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn request_redraw(&mut self) {
        self.redraws += 1;
    }
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

fn put_i16(bytes: &mut [u8], at: usize, value: i16) {
    bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

/// Ship 128, the Shuttle, whose `shän` names a 4-frame 8x8 `rlëD`, and
/// system 128, Alpha, alone at (0, 0) with no hyperlinks or stellars.
fn data() -> Rc<GameData> {
    let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16(&mut anim, 0x00, 1000);
    put_i16(&mut anim, 0x04, 1);
    put_i16(&mut anim, 0x34, 4);
    let sheet = (0..4)
        .fold(RledBuilder::new(8, 8), |sheet, _| {
            sheet.frame(|f| (0..8).fold(f, |f, _| f.line().pixels(&[0x7C00; 8])))
        })
        .build();
    let mut system = vec![0; System::SIZE.expect("fixed")];
    for slot in 0..32 {
        // Every hyperlink and stellar slot unused.
        put_i16(&mut system, 0x04 + 2 * slot, -1);
    }
    put_i16(&mut system, 0x66, -1);
    let fork = ForkBuilder::new()
        .resource(
            Ship::TYPE,
            128,
            Some(b"Shuttle"),
            &vec![0; Ship::SIZE.expect("fixed")],
        )
        .resource(ShipAnim::TYPE, 128, None, &anim)
        .resource(RLED, 1000, None, &sheet)
        .resource(System::TYPE, 128, Some(b"Alpha"), &system)
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

struct Harness {
    app: App<Rc<GameData>>,
    window: FakeWindow,
    gpu: RecordingGpu,
    clock: Duration,
}

impl Harness {
    fn new() -> Self {
        let window = FakeWindow {
            size_px: (1024, 768),
            scale_factor: 1.0,
            redraws: 0,
        };
        let data = data();
        let app = App::new(&window, Rc::clone(&data), start_screen(data));
        Self {
            app,
            window,
            gpu: RecordingGpu::new(),
            clock: Duration::ZERO,
        }
    }

    fn send(&mut self, event: WindowEvent) -> Control {
        self.app.handle(event, &mut self.window, &mut self.gpu)
    }

    /// Sends a redraw 1/60 s after the last and returns its frame.
    fn frame(&mut self) -> Frame {
        self.clock += Duration::from_micros(16_667);
        let before = self.gpu.submits().len();
        self.send(WindowEvent::Redraw {
            elapsed: self.clock,
        });
        let submits = self.gpu.submits();
        assert_eq!(submits.len(), before + 1, "one submit per redraw");
        let frame = submits[before].clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }
}

/// The frame's batches as (kind, length).
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

fn text_size(frame: &Frame) -> f32 {
    frame
        .batches
        .iter()
        .find_map(|batch| match batch {
            Batch::Text(runs) => Some(runs[0].size_px),
            _ => None,
        })
        .expect("a text batch")
}

#[test]
fn every_redraw_submits_one_frame_and_asks_for_the_next() {
    let mut harness = Harness::new();

    let first = harness.frame();
    let second = harness.frame();

    // The ship's sprite, then its name, four stats and the footer, and the
    // router's hint.
    let expected = [("sprites", 1), ("text", 7)];
    assert_eq!(shape(&first), expected);
    assert_eq!(shape(&second), expected);
    assert_eq!(harness.window.redraws, 2);
    assert_eq!(first.target, (1024, 768));
    assert_eq!(
        first.viewport,
        PixelRect {
            x: 0,
            y: 0,
            w: 1024,
            h: 768
        }
    );
}

#[test]
fn a_resize_updates_the_next_frames_viewport() {
    let mut harness = Harness::new();
    let normal = harness.frame();

    harness.send(WindowEvent::Resized {
        size_px: (2048, 1536),
        scale_factor: 2.0,
    });
    let retina = harness.frame();
    harness.send(WindowEvent::Resized {
        size_px: (1280, 768),
        scale_factor: 1.0,
    });
    let wide = harness.frame();

    assert_eq!(
        retina.viewport,
        PixelRect {
            x: 0,
            y: 0,
            w: 2048,
            h: 1536
        }
    );
    assert_eq!(retina.target, (2048, 1536));
    assert_eq!(text_size(&retina), 2.0 * text_size(&normal));
    assert_eq!(wide.viewport.x, 128);
    assert_eq!(wide.viewport.w, 1024);
}

#[test]
fn escape_and_closing_exit() {
    let mut harness = Harness::new();
    assert_eq!(
        harness.send(WindowEvent::Key {
            key: Key::Escape,
            pressed: true
        }),
        Control::Exit
    );
    assert_eq!(harness.send(WindowEvent::CloseRequested), Control::Exit);
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
fn tab_switches_between_the_ship_browser_and_the_galaxy_map() {
    let mut harness = Harness::new();
    let tab = |pressed| WindowEvent::Key {
        key: Key::Tab,
        pressed,
    };
    let ships = texts(&harness.frame());
    assert!(ships.contains(&"Shuttle".to_owned()), "{ships:?}");
    assert!(ships.contains(&"Tab: ships / galaxy map".to_owned()));

    assert_eq!(harness.send(tab(true)), Control::Continue);
    assert_eq!(harness.send(tab(false)), Control::Continue);
    let map = texts(&harness.frame());
    assert!(
        map.contains(&"Click a system to see its stellars".to_owned()),
        "{map:?}"
    );
    assert!(!map.contains(&"Shuttle".to_owned()));
    assert!(map.contains(&"Tab: ships / galaxy map".to_owned()));

    harness.send(tab(true));
    let back = texts(&harness.frame());
    assert!(back.contains(&"Shuttle".to_owned()), "{back:?}");
}
