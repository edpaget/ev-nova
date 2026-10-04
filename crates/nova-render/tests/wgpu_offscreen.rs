//! The wgpu adapter draws a known frame into an offscreen texture, read
//! back and checked pixel by pixel. Needs a GPU adapter; skips, passing,
//! without one.

use nova_data::fonts::check_sfnt;
use nova_data::fonts::fixture::{SfntBuilder, block_font, block_sfnt};
use nova_data::graphics::Image;
use nova_render::fonts::FontFaces;
use nova_render::wgpu::{
    InitError, OffscreenGpu, OverlayGpu, OverlayPainter, PaintTarget, WithOverlay,
};
use nova_render::{
    Frame, Gpu, ImageError, ImageSource, LogicalSize, PixelRect, Renderer, Viewport,
};
use nova_view::{Color, DrawList, Font, ImageKey, ImageKind, Point};

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

/// A patterned 4x2 picture (`PICT` 1), a white 4x4 sprite (`rlëD` 2), a
/// patterned 2x4 sprite (`rlëD` 3), an opaque black 4x4 sprite (`rlëD` 4)
/// and an opaque dark orange 4x4 sprite (`rlëD` 5).
struct Images;

/// `rlëD` 5's colour.
const ORANGE: [u8; 4] = [64, 32, 0, 255];

impl ImageSource for Images {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let solid = |rgba: [u8; 4]| Image::from_rgba(4, 4, rgba.repeat(16)).expect("4x4");
        match (kind, id) {
            (ImageKind::Pict, 1) => Ok(vec![patterned(4, 2)]),
            (ImageKind::Rled, 2) => Ok(vec![solid([255, 255, 255, 255])]),
            (ImageKind::Rled, 3) => Ok(vec![patterned(2, 4)]),
            (ImageKind::Rled, 4) => Ok(vec![solid([0, 0, 0, 255])]),
            (ImageKind::Rled, 5) => Ok(vec![solid(ORANGE)]),
            _ => Err(ImageError::Missing),
        }
    }
}

/// An offscreen GPU with the bundled font, or `None` (after a skip
/// message) with no adapter.
fn gpu() -> Option<OffscreenGpu> {
    gpu_with(SIZE, SIZE, &FontFaces::bundled())
}

/// A `width` x `height` offscreen GPU drawing text in `faces`, or `None`
/// (after a skip message) with no adapter.
fn gpu_with(width: u32, height: u32, faces: &FontFaces) -> Option<OffscreenGpu> {
    match OffscreenGpu::new(width, height, faces) {
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

/// A picture, two sprites, a line and a dot, at known places.
fn known_list() -> DrawList {
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
    list
}

#[test]
fn a_known_frame_draws_the_expected_pixels() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let list = known_list();
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

/// Asserts every pixel of the logical rectangle `left..right` x
/// `top..bottom` (scale 2, content offset 8 rows) is `want` within 2.
fn assert_area(pixels: &[u8], (left, top): (u32, u32), (right, bottom): (u32, u32), want: [u8; 4]) {
    for y in 2 * top + 8..2 * bottom + 8 {
        for x in 2 * left..2 * right {
            assert_near(pixels, (x, y), want, 2);
        }
    }
}

#[test]
fn an_additive_sprite_adds_its_colour_scaled_by_its_alpha() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // Three grey squares, logical (4..8, 4..8), (14..18, 4..8) and
    // (24..28, 4..8), each with an additive sprite over it, and one
    // additive sprite over the black clear at (14..18, 14..18).
    let grey = Color::rgba(64, 64, 64, 255);
    let spots = [6.0, 16.0, 26.0].map(|x| Point::new(x, 6.0));
    let mut list = DrawList::new();
    for spot in spots {
        list.sprite(ImageKey::sprite(2, 0), spot, grey);
    }
    list.additive_sprite(ImageKey::sprite(4, 0), spots[0], Color::WHITE)
        .additive_sprite(ImageKey::sprite(5, 0), spots[1], Color::WHITE)
        .additive_sprite(
            ImageKey::sprite(2, 0),
            spots[2],
            Color::rgba(255, 0, 0, 128),
        )
        .additive_sprite(ImageKey::sprite(5, 0), Point::new(16.0, 16.0), Color::WHITE);
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // Black adds nothing.
    assert_area(&pixels, (4, 4), (8, 8), [64, 64, 64, 255]);
    // Orange adds its colour.
    assert_area(&pixels, (14, 4), (18, 8), [128, 96, 64, 255]);
    // Red at half alpha adds half of it.
    assert_area(&pixels, (24, 4), (28, 8), [192, 64, 64, 255]);
    // Over black it is its own colour.
    assert_area(&pixels, (14, 14), (18, 18), ORANGE);
    // Around them.
    for at in [
        (3, 5),
        (8, 5),
        (13, 5),
        (18, 5),
        (5, 3),
        (5, 8),
        (16, 13),
        (16, 18),
    ] {
        assert_area(&pixels, at, (at.0 + 1, at.1 + 1), BLACK);
    }
}

#[test]
fn an_additive_sprite_after_a_normal_one_lands_over_it_in_order() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // A normal red square A, logical (8..12, 8..12); an additive orange
    // square B, (10..14, 10..14), over A's bottom-right quarter; a normal
    // blue square C, (12..16, 12..16), over B's bottom-right quarter.
    let mut list = DrawList::new();
    list.sprite(
        ImageKey::sprite(2, 0),
        Point::new(10.0, 10.0),
        Color::rgba(128, 0, 0, 255),
    )
    .additive_sprite(ImageKey::sprite(5, 0), Point::new(12.0, 12.0), Color::WHITE)
    .sprite(
        ImageKey::sprite(2, 0),
        Point::new(14.0, 14.0),
        Color::rgba(0, 0, 128, 255),
    );
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // A alone.
    assert_area(&pixels, (8, 8), (10, 12), [128, 0, 0, 255]);
    // B added over A.
    assert_area(&pixels, (10, 10), (12, 12), [192, 32, 0, 255]);
    // B over the clear.
    assert_area(&pixels, (12, 10), (14, 12), ORANGE);
    assert_area(&pixels, (10, 12), (12, 14), ORANGE);
    // C painted over B.
    assert_area(&pixels, (12, 12), (14, 14), [0, 0, 128, 255]);
    // C alone.
    assert_area(&pixels, (14, 12), (16, 16), [0, 0, 128, 255]);
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
fn only_the_handed_fonts_load() {
    let Some(bundled) = gpu() else {
        return;
    };
    assert_eq!(bundled.font_faces(), 1);
    assert!(!bundled.charcoal_loaded());
    let faces = FontFaces::bundled().with_charcoal(block_font("Charcoal"));
    let Some(with_charcoal) = gpu_with(SIZE, SIZE, &faces) else {
        return;
    };
    assert_eq!(with_charcoal.font_faces(), 2);
    assert!(with_charcoal.charcoal_loaded());
}

#[test]
fn charcoal_bytes_that_load_no_face_are_not_loaded() {
    let faces = FontFaces::bundled().with_charcoal(&b"not a font"[..]);
    let Some(gpu) = gpu_with(SIZE, SIZE, &faces) else {
        return;
    };
    assert_eq!(gpu.font_faces(), 1);
    assert!(!gpu.charcoal_loaded());
}

#[test]
fn charcoal_loads_exactly_when_its_check_passes() {
    // What `nova_data` accepts as Charcoal is what the font stack loads, so
    // an unusable file is reported rather than silently replaced.
    let cases = [
        ("the block font", block_font("Charcoal")),
        ("zeroed tables", SfntBuilder::truetype().build()),
        (
            "an empty name table",
            block_sfnt("Charcoal").table(b"name", [0; 4]).build(),
        ),
        ("not a font", b"not a font".to_vec()),
    ];
    for (case, bytes) in cases {
        let faces = FontFaces::bundled().with_charcoal(bytes.clone());
        let Some(gpu) = gpu_with(SIZE, SIZE, &faces) else {
            return;
        };
        assert_eq!(
            gpu.charcoal_loaded(),
            check_sfnt(&bytes).is_ok(),
            "{case}: {:?}",
            check_sfnt(&bytes)
        );
    }
}

/// The logical space of the font tests: one line of text.
const LINE: LogicalSize = LogicalSize { w: 128, h: 32 };

/// What the font tests draw.
const SAMPLE: &str = "EV Nova";

/// Where the font tests' text starts, in logical units.
const TEXT_AT: (f32, f32) = (2.0, 4.0);

/// [`SAMPLE`] in `font` at `size` points, drawn with `faces` into a
/// [`LINE`]-sized window at `scale` physical pixels per unit, as RGBA8
/// pixels; `None` with no GPU adapter.
fn draw_sample(faces: &FontFaces, font: Font, size: f32, scale: u32) -> Option<Picture> {
    let (w, h) = (LINE.w * scale, LINE.h * scale);
    let mut gpu = gpu_with(w, h, faces)?;
    let mut list = DrawList::new();
    let at = Point::new(TEXT_AT.0, TEXT_AT.1);
    list.text_in(font, SAMPLE, at, size, None, Color::WHITE);
    let report = Renderer::new(Images).render(
        &list,
        &Viewport::new(LINE, (w, h), f64::from(scale)),
        &mut gpu,
    );
    assert_eq!(report.new_failures, vec![]);
    Some(Picture {
        width: w,
        pixels: gpu.read_pixels().expect("read back"),
    })
}

/// Read-back pixels and their width.
#[derive(PartialEq)]
struct Picture {
    width: u32,
    pixels: Vec<u8>,
}

impl std::fmt::Debug for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} lit pixels", self.lit().len())
    }
}

impl Picture {
    /// The coverage (red channel: white text on black) at each pixel.
    fn coverage(&self) -> impl Iterator<Item = ((u32, u32), u8)> + '_ {
        self.pixels
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .map(|(at, p)| ((at as u32 % self.width, at as u32 / self.width), p[0]))
    }

    /// Every pixel any glyph touches.
    fn lit(&self) -> Vec<(u32, u32)> {
        self.coverage()
            .filter(|&(_, c)| c > 0)
            .map(|(at, _)| at)
            .collect()
    }

    /// The rows from the top of the highest ink to the bottom of the
    /// lowest.
    fn ink_height(&self) -> u32 {
        let rows = self.lit().into_iter().map(|(_, y)| y);
        let (top, bottom) = rows.fold((u32::MAX, 0), |(t, b), y| (t.min(y), b.max(y)));
        bottom + 1 - top
    }

    /// The longest horizontal run of fully covered pixels.
    fn longest_solid_run(&self) -> u32 {
        let mut longest = 0;
        let mut run = 0;
        for ((x, _), c) in self.coverage() {
            if x == 0 {
                run = 0;
            }
            run = if c >= 250 { run + 1 } else { 0 };
            longest = longest.max(run);
        }
        longest
    }
}

#[test]
fn each_font_draws_in_its_own_face() {
    let faces = FontFaces::bundled().with_charcoal(block_font("Charcoal"));
    let (Some(charcoal), Some(geneva)) = (
        draw_sample(&faces, Font::Charcoal, 12.0, 2),
        draw_sample(&faces, Font::Geneva, 12.0, 2),
    ) else {
        return;
    };
    assert!(!charcoal.lit().is_empty(), "Charcoal draws nothing");
    assert!(!geneva.lit().is_empty(), "Geneva draws nothing");
    assert_ne!(charcoal, geneva);
    // 24-pixel text: each block-font glyph is a solid square 19 pixels
    // wide; no letter of the fallback has a solid run that long.
    assert!(
        charcoal.longest_solid_run() >= 16,
        "{}",
        charcoal.longest_solid_run()
    );
    assert!(
        geneva.longest_solid_run() < 16,
        "{}",
        geneva.longest_solid_run()
    );
}

#[test]
fn charcoal_falls_back_when_it_did_not_load() {
    let missing = FontFaces::bundled();
    let unusable = FontFaces::bundled().with_charcoal(&b"not a font"[..]);
    for faces in [missing, unusable] {
        let (Some(charcoal), Some(geneva)) = (
            draw_sample(&faces, Font::Charcoal, 12.0, 2),
            draw_sample(&faces, Font::Geneva, 12.0, 2),
        ) else {
            return;
        };
        assert!(!geneva.lit().is_empty(), "Geneva draws nothing");
        assert_eq!(charcoal, geneva);
    }
}

#[test]
fn a_fallback_that_loads_no_face_draws_no_text() {
    let faces = FontFaces::new(&b"not a font"[..]);
    let Some(gpu) = gpu_with(SIZE, SIZE, &faces) else {
        return;
    };
    assert_eq!(gpu.font_faces(), 0);
    assert!(!gpu.charcoal_loaded());
    for font in [Font::Geneva, Font::Charcoal] {
        let Some(picture) = draw_sample(&faces, font, 12.0, 2) else {
            return;
        };
        assert_eq!(picture.lit(), vec![], "{font:?}");
    }
}

#[test]
fn with_no_fallback_face_both_fonts_draw_in_charcoal() {
    let faces = FontFaces::new(&b"not a font"[..]).with_charcoal(block_font("Charcoal"));
    let Some(gpu) = gpu_with(SIZE, SIZE, &faces) else {
        return;
    };
    assert_eq!(gpu.font_faces(), 1);
    assert!(gpu.charcoal_loaded());
    let (Some(charcoal), Some(geneva)) = (
        draw_sample(&faces, Font::Charcoal, 12.0, 2),
        draw_sample(&faces, Font::Geneva, 12.0, 2),
    ) else {
        return;
    };
    // Charcoal draws in its own (block) face; Geneva, with no fallback
    // face, is matched to the only face there is.
    assert!(
        charcoal.longest_solid_run() >= 16,
        "{}",
        charcoal.longest_solid_run()
    );
    assert_eq!(geneva, charcoal);
}

#[test]
fn both_fonts_draw_at_the_original_sizes_at_1x_and_2x() {
    let faces = FontFaces::bundled().with_charcoal(block_font("Charcoal"));
    // Nova's interface sizes (`cölr`, `ïntf`): Geneva 9, 10 and 12;
    // Charcoal 12.
    for font in [Font::Geneva, Font::Charcoal] {
        for size in [9.0, 10.0, 12.0] {
            let (Some(one), Some(two)) = (
                draw_sample(&faces, font, size, 1),
                draw_sample(&faces, font, size, 2),
            ) else {
                return;
            };
            for (scale, picture) in [(1, &one), (2, &two)] {
                let lit = picture.lit();
                assert!(
                    !lit.is_empty(),
                    "{font:?} {size}pt at {scale}x draws nothing"
                );
                // Inside the run's box: from its origin, one line high.
                let scale = scale as f32;
                let (left, top) = (TEXT_AT.0 * scale, TEXT_AT.1 * scale);
                let bottom = top + 1.2 * size * scale;
                for (x, y) in lit {
                    let (x, y) = (x as f32, y as f32);
                    assert!(
                        x + 1.0 >= left && y + 1.0 >= top && y <= bottom,
                        "{font:?} {size}pt at {scale}x: ({x}, {y}) is outside the run"
                    );
                }
            }
            // Drawn at the physical size, not scaled up: twice the pixels
            // per point gives twice the ink height, give or take rounding.
            let (h1, h2) = (one.ink_height(), two.ink_height());
            assert!(
                h2.abs_diff(2 * h1) <= 2,
                "{font:?} {size}pt: ink {h1} rows at 1x, {h2} at 2x"
            );
        }
    }
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

/// Opens one pass over the target that loads it with `load`, draws
/// nothing, and submits it.
fn pass(target: &PaintTarget<'_>, load: wgpu::LoadOp<wgpu::Color>) {
    let mut encoder = target
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("test overlay"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target.view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    }));
    target.queue.submit(Some(encoder.finish()));
}

/// Clears the whole target blue.
struct ClearBlue;

impl OverlayPainter for ClearBlue {
    fn paint(&mut self, target: &PaintTarget<'_>) {
        pass(target, wgpu::LoadOp::Clear(wgpu::Color::BLUE));
    }
}

/// Loads the target and draws nothing.
struct LoadOnly;

impl OverlayPainter for LoadOnly {
    fn paint(&mut self, target: &PaintTarget<'_>) {
        pass(target, wgpu::LoadOp::Load);
    }
}

/// Records each target's size and format.
#[derive(Default)]
struct Recording {
    targets: Vec<((u32, u32), wgpu::TextureFormat)>,
}

impl OverlayPainter for Recording {
    fn paint(&mut self, target: &PaintTarget<'_>) {
        self.targets.push((target.size_px, target.format));
    }
}

fn render_known(gpu: &mut impl nova_render::Gpu) {
    let report = Renderer::new(Images).render(
        &known_list(),
        &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0),
        gpu,
    );
    assert_eq!(report.new_failures, vec![]);
}

#[test]
fn an_overlay_paints_over_the_finished_frame() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    render_known(&mut WithOverlay {
        gpu: &mut gpu,
        painter: &mut ClearBlue,
    });
    let pixels = gpu.read_pixels().expect("read back");
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [0, 0, 255, 255]),
        "every pixel blue"
    );
}

#[test]
fn an_overlay_that_draws_nothing_leaves_the_frame_as_it_was() {
    let (Some(mut plain), Some(mut painted)) = (gpu(), gpu()) else {
        return;
    };
    render_known(&mut plain);
    render_known(&mut WithOverlay {
        gpu: &mut painted,
        painter: &mut LoadOnly,
    });
    let expected = plain.read_pixels().expect("read back");
    let pixels = painted.read_pixels().expect("read back");
    assert_patterned(&pixels, (4, 12), (4, 2));
    assert_eq!(pixels, expected);
}

#[test]
fn the_painter_runs_once_per_frame_on_the_offscreen_target() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    let mut painter = Recording::default();
    let frame = Frame {
        target: (SIZE, SIZE),
        viewport: PixelRect {
            x: 0,
            y: 8,
            w: SIZE,
            h: 48,
        },
        logical: LOGICAL,
        clear: Color::rgba(0, 0, 0, 255),
        batches: Vec::new(),
    };
    gpu.submit_with(&frame, &mut painter);
    let target = ((SIZE, SIZE), wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(painter.targets, [target]);
    gpu.submit_with(&frame, &mut painter);
    assert_eq!(painter.targets, [target, target]);
    gpu.submit(&frame);
    assert_eq!(painter.targets.len(), 2, "a plain submit paints nothing");
}
