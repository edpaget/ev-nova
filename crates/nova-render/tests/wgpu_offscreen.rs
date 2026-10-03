//! The wgpu adapter draws a known frame into an offscreen texture, read
//! back and checked pixel by pixel. Needs a GPU adapter; skips, passing,
//! without one.

use nova_data::graphics::Image;
use nova_render::wgpu::{InitError, OffscreenGpu};
use nova_render::{
    Frame, Gpu, ImageError, ImageSource, LogicalSize, PixelRect, Renderer, Viewport,
};
use nova_view::{Color, DrawList, ImageKey, ImageKind, Point};

/// Logical 32x24 in a 64x64 target: scale 2, content (0, 8, 64, 48).
const LOGICAL: LogicalSize = LogicalSize { w: 32, h: 24 };
const SIZE: u32 = 64;

/// A red 4x4 picture (`PICT` 1) and a white 4x4 sprite (`rlëD` 2).
struct Images;

impl ImageSource for Images {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let solid = |rgba: [u8; 4]| Image::from_rgba(4, 4, rgba.repeat(16)).expect("4x4");
        match (kind, id) {
            (ImageKind::Pict, 1) => Ok(vec![solid([255, 0, 0, 255])]),
            (ImageKind::Rled, 2) => Ok(vec![solid([255, 255, 255, 255])]),
            _ => Err(ImageError::Missing),
        }
    }
}

/// An offscreen GPU, or `None` (after a skip message) with no adapter.
fn gpu() -> Option<OffscreenGpu> {
    match OffscreenGpu::new(SIZE, SIZE) {
        Ok(gpu) => Some(gpu),
        Err(error @ (InitError::NoAdapter(_) | InitError::NoDevice(_))) => {
            eprintln!("skipping: no GPU adapter ({error})");
            None
        }
        Err(error) => panic!("offscreen GPU: {error}"),
    }
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    pixels[at..at + 4].try_into().expect("four bytes")
}

/// Asserts the pixel at (`x`, `y`) is `want` within `tolerance` per channel.
fn assert_near(pixels: &[u8], (x, y): (u32, u32), want: [u8; 4], tolerance: u8) {
    let got = pixel(pixels, x, y);
    let close = got
        .iter()
        .zip(want)
        .all(|(&g, w)| g.abs_diff(w) <= tolerance);
    assert!(
        close,
        "pixel ({x}, {y}) is {got:?}, want {want:?} ±{tolerance}"
    );
}

const BLACK: [u8; 4] = [0, 0, 0, 255];

#[test]
fn a_known_frame_draws_the_expected_pixels() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let mut list = DrawList::new();
    list.picture(ImageKey::picture(1), Point::new(2.0, 2.0))
        .sprite(
            ImageKey::sprite(2, 0),
            Point::new(16.0, 6.0),
            Color::rgba(0, 255, 0, 128),
        )
        .line(
            Point::new(2.0, 20.0),
            Point::new(30.0, 20.0),
            2.0,
            Color::rgba(0, 0, 255, 255),
        )
        .dot(Point::new(28.0, 4.0), 4.0, Color::rgba(255, 255, 0, 255));
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    assert_eq!(pixels.len(), (SIZE * SIZE * 4) as usize);
    // The picture: logical (2..6, 2..6) is pixels (4..12, 12..20).
    assert_near(&pixels, (5, 13), [255, 0, 0, 255], 0);
    assert_near(&pixels, (10, 18), [255, 0, 0, 255], 0);
    // The sprite: green at half alpha over black, pixels (28..36, 16..24).
    assert_near(&pixels, (29, 17), [0, 128, 0, 255], 3);
    assert_near(&pixels, (34, 22), [0, 128, 0, 255], 3);
    // The line: logical y 19..21 is pixels 46..50, x 4..60.
    assert_near(&pixels, (5, 47), [0, 0, 255, 255], 2);
    assert_near(&pixels, (58, 48), [0, 0, 255, 255], 2);
    // The dot: logical (26..30, 2..6) is pixels (52..60, 12..20).
    assert_near(&pixels, (53, 13), [255, 255, 0, 255], 2);
    assert_near(&pixels, (58, 18), [255, 255, 0, 255], 2);
    // Background inside the content, and the bars above and below it.
    assert_near(&pixels, (32, 30), BLACK, 0);
    for x in [0, 31, 63] {
        for y in [0, 7, 56, 63] {
            assert_near(&pixels, (x, y), BLACK, 0);
        }
    }
    // Just outside the shapes' edges.
    assert_near(&pixels, (3, 13), BLACK, 0);
    assert_near(&pixels, (12, 13), BLACK, 0);
    assert_near(&pixels, (5, 45), BLACK, 0);
    assert_near(&pixels, (5, 50), BLACK, 0);
}

/// Whether any pixel in rows `rows` is not black.
fn any_drawn(pixels: &[u8], rows: std::ops::Range<u32>) -> bool {
    rows.flat_map(|y| (0..SIZE).map(move |x| (x, y)))
        .any(|(x, y)| pixel(pixels, x, y) != BLACK)
}

#[test]
fn text_draws_inside_its_clip_box() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // Two text batches (a dot between them), the second running off the
    // bottom of the content into the bar.
    let mut list = DrawList::new();
    list.text("EV Nova", Point::new(2.0, 2.0), 8.0, None, Color::WHITE)
        .dot(Point::new(31.0, 12.0), 1.0, Color::rgba(255, 0, 0, 255))
        .text("Nova", Point::new(2.0, 20.0), 8.0, None, Color::WHITE);
    let mut renderer = Renderer::new(Images);

    renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    if gpu.font_faces() == 0 {
        eprintln!("skipping the text check: no fonts loaded");
        return;
    }
    // The first run, logical y 2..10, is pixels 12..28.
    assert!(any_drawn(&pixels, 8..28), "the first run is not drawn");
    // The second, from logical y 20 (pixel 48), is cut off at the bar.
    assert!(any_drawn(&pixels, 48..56), "the second run is not drawn");
    for x in 0..SIZE {
        for y in (0..8).chain(56..SIZE) {
            assert_near(&pixels, (x, y), BLACK, 0);
        }
    }
}

#[test]
fn the_font_count_is_the_systems() {
    let Some(gpu) = gpu() else {
        return;
    };
    assert_eq!(gpu.font_faces(), glyphon::FontSystem::new().db().len());
}

#[test]
fn the_clear_colour_fills_the_target() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let frame = Frame {
        target: (SIZE, SIZE),
        viewport: PixelRect {
            x: 0,
            y: 8,
            w: SIZE,
            h: 48,
        },
        logical: LOGICAL,
        clear: Color::rgba(51, 102, 204, 128),
        batches: Vec::new(),
    };

    gpu.submit(&frame);
    let pixels = gpu.read_pixels().expect("read back");

    for at in [(0, 0), (32, 32), (63, 63)] {
        assert_near(&pixels, at, [51, 102, 204, 128], 1);
    }
}
