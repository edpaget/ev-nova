//! The About dialog through the whole app: synthetic game data (the About
//! text, Nova's button pictures and masks, and the text frame) and a
//! synthetic interface file holding "Desc Dialog", laid out by the real
//! glyphon metrics, drawn through the renderer into the recording Gpu and
//! driven only by window events.

use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, AppScreen, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::PICT;
use nova_data::graphics::fixture::{DirectBits, PictBuilder};
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::store::fixture::{one_file, one_file_at};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame, Rect, TextRun};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;
use nova_view::geometry::{Bounds, Point};
use nova_view::text::LINE_HEIGHT;
use nova_view::ui::desc::{DONE_ITEM, TEXT_ITEM};
use nova_view::{Key, MouseButton};

/// A 1024 x 768 window at `scale`.
struct FakeWindow {
    scale: f64,
}

impl WindowPort for FakeWindow {
    fn size_px(&self) -> (u32, u32) {
        let side = |logical: f64| (logical * self.scale) as u32;
        (side(1024.0), side(768.0))
    }

    fn scale_factor(&self) -> f64 {
        self.scale
    }

    fn request_redraw(&mut self) {}
}

fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
    resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes
}

/// A `width` x `height` picture of one colour.
fn pict(width: i16, height: i16, rgb: [u8; 3]) -> Vec<u8> {
    let frame = [0, 0, height, width];
    let pixels = vec![rgb; (width * height) as usize];
    PictBuilder::new(frame)
        .direct_bits(&DirectBits::rgb888(frame, &pixels))
        .end()
        .build()
}

/// The About text: sixty numbered lines, far more than its box holds.
fn about_text() -> Vec<u8> {
    let text: Vec<String> = (0..60).map(|n| format!("About line {n:02}")).collect();
    let mut bytes = text.join("\r").into_bytes();
    bytes.push(0);
    bytes.extend([0xFF; 2]);
    bytes.extend([0; 34]);
    bytes
}

/// The About text, the nine button pictures (caps 13 x 25, middles
/// 2 x 25), the six cap masks (black: wholly opaque) and the three frame
/// pictures (small: they are stretched).
fn game_data() -> Rc<GameData> {
    let mut resources = vec![(Desc::TYPE, 32767, about_text())];
    for state in [7500, 7503, 7506] {
        resources.push((PICT, state, pict(13, 25, [200, 0, 0])));
        resources.push((PICT, state + 1, pict(2, 25, [0, 200, 0])));
        resources.push((PICT, state + 2, pict(13, 25, [0, 0, 200])));
        resources.push((PICT, state + 100, pict(13, 25, [0, 0, 0])));
        resources.push((PICT, state + 102, pict(13, 25, [0, 0, 0])));
    }
    for frame in [8524, 8525, 8526] {
        resources.push((PICT, frame, pict(4, 2, [60, 60, 60])));
    }
    let file = fork(&resources);
    Rc::new(one_file(file).load())
}

fn be(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

/// One `DITL` item: bounds (top, left, bottom, right), type byte, no data.
fn user_item(bounds: [i16; 4], type_byte: u8) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    bytes.extend(be(&bounds));
    bytes.extend([type_byte, 0]);
    bytes
}

/// Stock "Desc Dialog": `DLOG` 3003, 441 x 313 and centred, and its
/// `DITL`: Done (1), two items parked outside (2, 4), the text box (3)
/// and two small user items (5, 6).
fn interface() -> InterfaceData {
    let mut dlog = be(&[24, 18, 337, 459, 2]);
    dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    dlog.extend(be(&[3003]));
    dlog.extend([0, 0, 0xA8, 0x0A]);
    let items = [
        user_item([281, 173, 306, 272], 0),
        user_item([329, 221, 359, 289], 0x80),
        user_item([10, 11, 272, 428], 0x80),
        user_item([345, 70, 370, 148], 0),
        user_item([282, 330, 305, 353], 0x80),
        user_item([282, 363, 305, 386], 0x80),
    ];
    let mut ditl = be(&[items.len() as i16 - 1]);
    ditl.extend(items.concat());
    let file = fork(&[(Dlog::TYPE, 3003, dlog), (Ditl::TYPE, 3003, ditl)]);
    let path = Path::new("/Nova-DF.rsrc");
    InterfaceData::load(&one_file_at(path, file), path).expect("loads")
}

struct Harness {
    app: App<Rc<GameData>>,
    window: FakeWindow,
    gpu: RecordingGpu,
    frames: u64,
}

impl Harness {
    fn new(scale: f64, dialogs: bool) -> Self {
        let data = game_data();
        let mut screen: AppScreen = start_screen(Rc::clone(&data));
        if dialogs {
            screen = screen.with_dialogs(
                Rc::new(interface()),
                Rc::new(GlyphonMetrics::new(&FontFaces::bundled())),
            );
        }
        let window = FakeWindow { scale };
        Self {
            app: App::new(&window, data, screen),
            window,
            gpu: RecordingGpu::new(),
            frames: 0,
        }
    }

    fn send(&mut self, event: WindowEvent) {
        let control = self.app.handle(event, &mut self.window, &mut self.gpu);
        assert_eq!(control, Control::Continue, "{event:?}");
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

    /// Moves to the logical point `at`, then presses and releases there.
    fn click(&mut self, at: Point) {
        let scale = f64::from(self.scale());
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x) * scale, f64::from(at.y) * scale),
        });
        for pressed in [true, false] {
            self.send(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed,
            });
        }
    }

    fn scale(&self) -> f32 {
        self.window.scale as f32
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(16 * self.frames),
        });
        assert_eq!(self.app.take_failures(), []);
        (*self.gpu.submits().last().expect("a frame")).clone()
    }

    /// The open About dialog's item `item`, in logical units.
    fn item_bounds(&self, item: usize) -> Bounds {
        self.app
            .screen()
            .about()
            .expect("open")
            .dialog()
            .item_bounds(item)
            .expect("an item")
    }
}

fn texts(frame: &Frame) -> Vec<TextRun> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// `bounds` scaled by `scale`: physical pixels, or logical units at 1.
fn physical(bounds: Bounds, scale: f32) -> Rect {
    Rect {
        x: bounds.min.x * scale,
        y: bounds.min.y * scale,
        w: bounds.width() * scale,
        h: bounds.height() * scale,
    }
}

#[test]
#[allow(clippy::float_cmp)] // Whole physical pixels.
fn i_opens_about_and_its_text_reaches_the_gpu_in_window_pixels() {
    for scale in [1.0, 2.0] {
        let mut harness = Harness::new(scale, true);
        harness.frame();
        harness.press(Key::Char('i'));
        assert_eq!(harness.showing(), Showing::About);
        let frame = harness.frame();
        let scale = harness.scale();
        assert_eq!(
            (frame.viewport.w, frame.viewport.h),
            ((1024.0 * scale) as u32, (768.0 * scale) as u32)
        );

        // The About text's lines, in physical pixels, inside its text box,
        // the partial last one clipped to it.
        let text_box = physical(harness.item_bounds(TEXT_ITEM), scale);
        let lines: Vec<TextRun> = texts(&frame)
            .into_iter()
            .filter(|run| run.text.starts_with("About line"))
            .collect();
        assert!(!lines.is_empty(), "at {scale}");
        for run in &lines {
            let (x, y) = run.origin_px;
            assert!(x >= text_box.x && x < text_box.x + text_box.w, "{run:?}");
            assert!(y >= text_box.y, "{run:?}");
            let bottom = text_box.y + text_box.h;
            assert!(y < bottom, "{run:?}");
            if y + LINE_HEIGHT * run.size * run.scale > bottom + 0.01 {
                assert_eq!(
                    (run.clip.y as f32, (run.clip.y + run.clip.h) as f32),
                    (text_box.y, bottom),
                    "{run:?}"
                );
            }
        }
    }
}

#[test]
fn a_click_on_done_in_window_pixels_closes_it() {
    for scale in [1.0, 2.0] {
        let mut harness = Harness::new(scale, true);
        harness.press(Key::Char('i'));
        let done = harness.item_bounds(DONE_ITEM);
        harness.click(done.center());
        assert_eq!(harness.showing(), Showing::ShipBrowser, "at {scale}");
    }
}

#[test]
fn return_and_escape_close_it_without_quitting() {
    for key in [Key::Enter, Key::Escape] {
        let mut harness = Harness::new(1.0, true);
        harness.press(Key::Char('i'));
        harness.press(key);
        assert_eq!(harness.showing(), Showing::ShipBrowser, "{key:?}");
        assert!(harness.app.screen().about().is_none());
    }
}

#[test]
fn without_dialogs_i_does_nothing() {
    let mut harness = Harness::new(1.0, false);
    harness.press(Key::Char('i'));
    assert_eq!(harness.showing(), Showing::ShipBrowser);
    let frame = harness.frame();
    assert!(
        !texts(&frame)
            .iter()
            .any(|run| run.text.starts_with("About line"))
    );
}

#[test]
fn input_does_not_reach_the_screen_below() {
    let mut harness = Harness::new(1.0, true);
    harness.press(Key::Char('i'));
    for key in [Key::Tab, Key::Char('f'), Key::Right] {
        harness.press(key);
    }
    assert_eq!(harness.showing(), Showing::About);
    assert!(harness.app.screen().flight_view().is_none());
    harness.press(Key::Escape);
    assert_eq!(
        harness.showing(),
        Showing::ShipBrowser,
        "Tab never switched"
    );
}
