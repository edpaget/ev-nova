//! The app with its developer overlay, wired to the screen router over
//! minimal synthetic game data, the renderer and the recording Gpu: the
//! backquote key shows and hides the overlay, the game sees no input while
//! it shows, and redraws feed its frame times. Runs in the default build;
//! egui never appears here.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record, SystemId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
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
    gpu: RecordingGpu,
}

impl Harness {
    fn new(overlay: bool) -> Self {
        let data = data();
        let app = App::new(&FakeWindow, Rc::clone(&data), start_screen(data));
        Self {
            app: if overlay { app.with_dev_overlay() } else { app },
            gpu: RecordingGpu::new(),
        }
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue
        );
    }

    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.send(WindowEvent::Key {
                key,
                pressed,
                repeat: false,
            });
        }
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

    fn visible(&self) -> bool {
        self.app.dev_overlay().expect("an overlay").visible()
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    /// Redraws at `ms` and returns the frame submitted.
    fn redraw(&mut self, ms: u64) -> Frame {
        let before = self.gpu.submits().len();
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(ms),
        });
        assert_eq!(
            self.gpu.submits().len(),
            before + 1,
            "one submit per redraw"
        );
        self.gpu.submits()[before].clone()
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

#[test]
fn backquote_shows_and_hides_the_overlay_over_the_ship_browser() {
    let mut harness = Harness::new(true);
    assert!(!harness.visible(), "starts hidden");
    harness.press(Key::Char('`'));
    assert!(harness.visible());
    assert_eq!(harness.showing(), Showing::ShipBrowser);
    harness.press(Key::Char('`'));
    assert!(!harness.visible());
    assert_eq!(harness.app.take_failures(), []);
}

#[test]
fn tab_reaches_the_game_only_while_the_overlay_is_hidden() {
    let mut harness = Harness::new(true);
    harness.press(Key::Char('`'));
    harness.press(Key::Tab);
    assert_eq!(
        harness.showing(),
        Showing::ShipBrowser,
        "the overlay took Tab"
    );
    harness.press(Key::Char('`'));
    harness.press(Key::Tab);
    assert_eq!(harness.showing(), Showing::GalaxyMap);
}

#[test]
fn a_click_while_the_overlay_shows_selects_nothing_on_the_map() {
    let mut harness = Harness::new(true);
    harness.press(Key::Tab);
    harness.redraw(0);
    let map = harness.app.screen().galaxy_map();
    let system = map.model().system(SystemId(128)).expect("a system");
    let alpha = map.view().world_to_screen(system.position());

    harness.press(Key::Char('`'));
    harness.click(alpha);
    assert_eq!(harness.app.screen().galaxy_map().selected(), None);

    harness.press(Key::Char('`'));
    harness.click(alpha);
    assert_eq!(
        harness.app.screen().galaxy_map().selected(),
        Some(SystemId(128))
    );
    assert_eq!(harness.app.take_failures(), []);
}

#[test]
fn redraws_draw_the_game_as_before_and_feed_the_frame_times() {
    let mut plain = Harness::new(false);
    let mut harness = Harness::new(true);
    harness.press(Key::Char('`'));
    for ms in [0, 20, 40] {
        let expected = shape(&plain.redraw(ms));
        assert_eq!(shape(&harness.redraw(ms)), expected, "at {ms} ms");
    }
    let times = harness.app.dev_overlay().expect("an overlay").frame_times();
    assert_eq!(times.count(), 2);
    assert_eq!(times.last(), Some(Duration::from_millis(20)));
    assert_eq!(times.mean(), Some(Duration::from_millis(20)));
    assert_eq!(harness.app.take_failures(), []);
}
