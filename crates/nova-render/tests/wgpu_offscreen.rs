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

/// Eight distinct opaque colours.
const PALETTE: [[u8; 4]; 8] = [
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 255],
    [255, 255, 0, 255],
    [0, 255, 255, 255],
    [255, 0, 255, 255],
    [255, 255, 255, 255],
    [255, 128, 0, 255],
];

/// The colour of texel (`x`, `y`) in a [`patterned`] image `w` wide.
fn texel(w: u32, x: u32, y: u32) -> [u8; 4] {
    PALETTE[(y * w + x) as usize]
}

/// A `w` x `h` image (at most eight texels) whose every texel is a
/// different colour, so a flip, a transpose or a wrong row stride moves
/// some colour.
fn patterned(w: u32, h: u32) -> Image {
    let pixels = (0..h)
        .flat_map(|y| (0..w).map(move |x| texel(w, x, y)))
        .flatten()
        .collect();
    Image::from_rgba(w, h, pixels).expect("w x h")
}

/// A patterned 4x2 picture (`PICT` 1), a white 4x4 sprite (`rlëD` 2) and
/// a patterned 2x4 sprite (`rlëD` 3).
struct Images;

impl ImageSource for Images {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let solid = |rgba: [u8; 4]| Image::from_rgba(4, 4, rgba.repeat(16)).expect("4x4");
        match (kind, id) {
            (ImageKind::Pict, 1) => Ok(vec![patterned(4, 2)]),
            (ImageKind::Rled, 2) => Ok(vec![solid([255, 255, 255, 255])]),
            (ImageKind::Rled, 3) => Ok(vec![patterned(2, 4)]),
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

/// Asserts a [`patterned`] `w` x `h` image is drawn upright at scale 2
/// with its top-left corner at pixel (`left`, `top`): every pixel of each
/// texel's 2x2 block is that texel's colour.
fn assert_patterned(pixels: &[u8], (left, top): (u32, u32), (w, h): (u32, u32)) {
    for y in 0..h {
        for x in 0..w {
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let at = (left + 2 * x + dx, top + 2 * y + dy);
                assert_near(pixels, at, texel(w, x, y), 0);
            }
        }
    }
}

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
        .dot(Point::new(28.0, 4.0), 4.0, Color::rgba(255, 255, 0, 255))
        .sprite(ImageKey::sprite(3, 0), Point::new(24.0, 12.0), Color::WHITE);
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    assert_eq!(pixels.len(), (SIZE * SIZE * 4) as usize);
    // The 4x2 picture: logical (2..6, 2..4) is pixels (4..12, 12..16),
    // each texel a 2x2 block, upright and unmirrored.
    assert_patterned(&pixels, (4, 12), (4, 2));
    // The 2x4 sprite centred on (24, 12): logical (23..25, 10..14) is
    // pixels (46..50, 28..36).
    assert_patterned(&pixels, (46, 28), (2, 4));
    // The white sprite: green at half alpha over black, pixels (28..36, 16..24).
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
    assert_near(&pixels, (5, 11), BLACK, 0);
    assert_near(&pixels, (5, 16), BLACK, 0);
    assert_near(&pixels, (45, 30), BLACK, 0);
    assert_near(&pixels, (50, 30), BLACK, 0);
    assert_near(&pixels, (47, 27), BLACK, 0);
    assert_near(&pixels, (47, 36), BLACK, 0);
    assert_near(&pixels, (5, 45), BLACK, 0);
    assert_near(&pixels, (5, 50), BLACK, 0);
}

#[test]
fn batches_on_two_pages_draw_their_own_quads_in_order() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // On 4-texel pages the 4x2 picture fills page 0's only shelf and the
    // 2x4 sprite opens page 1, so the frame is four batches: page 0, page
    // 1, a solid, then page 0 again. Each covers part of the one before.
    let grey = Color::rgba(128, 128, 128, 255);
    let mut list = DrawList::new();
    list.picture(ImageKey::picture(1), Point::new(2.0, 2.0))
        .sprite(ImageKey::sprite(3, 0), Point::new(12.0, 4.0), Color::WHITE)
        .dot(Point::new(12.0, 5.0), 2.0, grey)
        .picture(ImageKey::picture(1), Point::new(10.0, 5.0));
    let mut renderer = Renderer::with_page_size(Images, 4);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // The first picture: logical (2..6, 2..4) is pixels (4..12, 12..16).
    assert_patterned(&pixels, (4, 12), (4, 2));
    // The sprite, logical (11..13, 2..6), is pixels (22..26, 12..20); the
    // dot covers its bottom half, so only its top two rows show.
    assert_patterned(&pixels, (22, 12), (2, 2));
    // The dot, logical (11..13, 4..6), is pixels (22..26, 16..20); the
    // second picture covers its bottom row.
    for at in [(22, 16), (25, 16), (22, 17), (25, 17)] {
        assert_near(&pixels, at, [128, 128, 128, 255], 2);
    }
    // The second picture: logical (10..14, 5..7) is pixels (20..28, 18..22).
    assert_patterned(&pixels, (20, 18), (4, 2));
    // Around them.
    for at in [(21, 13), (26, 13), (19, 19), (28, 19), (22, 22), (12, 13)] {
        assert_near(&pixels, at, BLACK, 0);
    }
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
fn zero_size_text_draws_nothing_and_the_frame_goes_on() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let mut list = DrawList::new();
    list.text("EV Nova", Point::new(2.0, 2.0), 0.0, None, Color::WHITE)
        .dot(Point::new(16.0, 12.0), 2.0, Color::rgba(255, 0, 0, 255));
    let mut renderer = Renderer::new(Images);

    renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    // The dot: logical (15..17, 11..13) is pixels (30..34, 30..34).
    assert_near(&pixels, (31, 31), [255, 0, 0, 255], 2);
    assert_near(&pixels, (31, 29), BLACK, 0);
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
