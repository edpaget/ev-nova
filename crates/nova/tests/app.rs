//! The app layer wired to its screen router showing the placeholder, the
//! renderer and the recording Gpu, driven by synthetic window events.

// Sizes here are small powers of two, exact in floating point.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

use nova::app::{
    App, AppScreen, Control, Placeholder, PlaceholderContent, WindowEvent, WindowPort,
};
use nova_data::graphics::Image;
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, ImageError, ImageSource, PixelRect};
use nova_view::{ImageKey, ImageKind, Key, MouseButton};

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

/// Canned solid frames by resource; `Missing` otherwise. Records calls.
#[derive(Default)]
struct FakeImages {
    resources: BTreeMap<(ImageKind, i16), Vec<Image>>,
    calls: RefCell<Vec<(ImageKind, i16)>>,
}

impl FakeImages {
    fn with(mut self, kind: ImageKind, id: i16, frames: usize, size: u32) -> Self {
        let frame = Image::from_rgba(size, size, vec![255; (size * size * 4) as usize])
            .expect("square frame");
        self.resources.insert((kind, id), vec![frame; frames]);
        self
    }
}

impl ImageSource for FakeImages {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        self.calls.borrow_mut().push((kind, id));
        self.resources
            .get(&(kind, id))
            .cloned()
            .ok_or(ImageError::Missing)
    }
}

const CONTENT: PlaceholderContent = PlaceholderContent {
    picture: Some(128),
    sprite: Some((200, 4)),
};

fn images() -> FakeImages {
    FakeImages::default()
        .with(ImageKind::Pict, 128, 1, 64)
        .with(ImageKind::Rled, 200, 4, 16)
}

struct Harness {
    app: App<FakeImages>,
    window: FakeWindow,
    gpu: RecordingGpu,
    clock: Duration,
}

impl Harness {
    fn new(images: FakeImages) -> Self {
        let window = FakeWindow {
            size_px: (1024, 768),
            scale_factor: 1.0,
            redraws: 0,
        };
        let screen = AppScreen::Placeholder(Placeholder::new(CONTENT));
        let app = App::new(&window, images, screen);
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
        submits[before].clone()
    }
}

fn images_called(harness: &Harness) -> std::cell::Ref<'_, Vec<(ImageKind, i16)>> {
    harness.app.images().calls.borrow()
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
fn every_redraw_submits_the_placeholders_batches() {
    let mut harness = Harness::new(images());

    let first = harness.frame();
    let second = harness.frame();

    // The picture and 300 sprites share page 0, then the label, then the
    // line and four dots.
    let expected = [("sprites", 301), ("text", 1), ("solid", 5)];
    assert_eq!(shape(&first), expected);
    assert_eq!(shape(&second), expected);
    assert_eq!(harness.window.redraws, 2);
    assert_eq!(
        first.viewport,
        PixelRect {
            x: 0,
            y: 0,
            w: 1024,
            h: 768
        }
    );
    match harness.app.screen() {
        AppScreen::Placeholder(placeholder) => assert_eq!(placeholder.count(), 300),
    }
    // Each resource decoded once, on the first frame.
    assert_eq!(
        *images_called(&harness),
        [(ImageKind::Pict, 128), (ImageKind::Rled, 200)]
    );
}

#[test]
fn a_resize_updates_the_next_frames_viewport() {
    let mut harness = Harness::new(images());
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
fn up_doubles_the_sprites_drawn() {
    let mut harness = Harness::new(images());
    assert_eq!(
        harness.send(WindowEvent::Key {
            key: Key::Up,
            pressed: true
        }),
        Control::Continue
    );
    assert_eq!(shape(&harness.frame())[0], ("sprites", 601));
}

#[test]
fn escape_and_closing_exit() {
    let mut harness = Harness::new(images());
    assert_eq!(
        harness.send(WindowEvent::Key {
            key: Key::Escape,
            pressed: true
        }),
        Control::Exit
    );
    assert_eq!(harness.send(WindowEvent::CloseRequested), Control::Exit);
}

#[test]
fn pointer_events_in_the_bars_change_nothing() {
    let mut harness = Harness::new(images());
    harness.send(WindowEvent::Resized {
        size_px: (1280, 768),
        scale_factor: 1.0,
    });
    let before = harness.frame();
    assert_eq!(
        harness.send(WindowEvent::PointerMoved { px: (10.0, 10.0) }),
        Control::Continue
    );
    assert_eq!(
        harness.send(WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true
        }),
        Control::Continue
    );
    let after = harness.frame();
    assert_eq!(shape(&after), shape(&before));
    assert_eq!(after.viewport, before.viewport);
}

#[test]
fn a_missing_sprite_is_reported_once() {
    let mut harness = Harness::new(FakeImages::default().with(ImageKind::Pict, 128, 1, 64));

    let frame = harness.frame();

    assert_eq!(shape(&frame)[0], ("sprites", 1));
    assert_eq!(
        harness.app.take_failures(),
        [(ImageKey::sprite(200, 0), ImageError::Missing)]
    );
    assert_eq!(harness.app.take_failures(), []);
    harness.frame();
    assert_eq!(harness.app.take_failures(), []);
}
