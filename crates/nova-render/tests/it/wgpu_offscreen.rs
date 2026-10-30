//! The wgpu adapter draws a known frame into an offscreen texture, read
//! back and checked pixel by pixel. Needs a GPU adapter; skips, passing,
//! without one.

use nova_data::fonts::check_sfnt;
use nova_data::fonts::fixture::{SfntBuilder, block_font, block_sfnt};
use nova_data::graphics::Image;
use nova_render::fonts::{FALLBACK_FONT, FontFaces};
use nova_render::wgpu::{
    InitError, OffscreenGpu, OverlayGpu, OverlayPainter, PaintTarget, WithOverlay,
};
use nova_render::{
    Frame, Gpu, ImageError, ImageSource, LogicalSize, PixelRect, Renderer, Viewport, WorldSpace,
};
use nova_view::geometry::Bounds;
use nova_view::{Blend, Color, Colorize, DrawCommand, DrawList, Font, ImageKey, ImageKind, Point};

use super::common::fill;

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
/// patterned 2x4 sprite (`rlëD` 3), an opaque black 4x4 sprite (`rlëD` 4),
/// an opaque dark orange 4x4 sprite (`rlëD` 5), the 4x4 [`or_sheet`]
/// (`rlëD` 7), solid 4x4 sprites of [`DIM`] (`rlëD` 8) and [`LIT`]
/// (`rlëD` 9), and the nebula pictures: solid 4x4 [`TEAL`] (`PICT` 10),
/// [`VIOLET`] (`PICT` 11) and black (`PICT` 12), and a transparent white
/// 4x4 (`PICT` 13). Shared with [`super::world_layer`].
pub(crate) struct Images;

/// `PICT` 10's colour.
const TEAL: [u8; 4] = [60, 180, 90, 255];
/// `PICT` 11's colour.
const VIOLET: [u8; 4] = [120, 20, 250, 255];
/// The colour of the solid rect the max pictures are drawn over, as a
/// border cell is painted.
const BORDER: [u8; 4] = [200, 40, 90, 255];

/// `rlëD` 5's colour.
const ORANGE: [u8; 4] = [64, 32, 0, 255];

/// A 5-bit colour widened to opaque 8-bit by bit replication,
/// `(c << 3) | (c >> 2)`, as stock sprite texels are.
fn w555(rgb5: [u8; 3]) -> [u8; 4] {
    let [r, g, b] = rgb5.map(|c| (c << 3) | (c >> 2));
    [r, g, b, 255]
}

/// 5-bit (16, 10, 5): the lit colour of [`or_sheet`], and `rlëD` 9.
const LIT_555: [u8; 3] = [16, 10, 5];
/// [`LIT_555`] widened.
const LIT: [u8; 4] = [132, 82, 41, 255];
/// 5-bit (8, 6, 2): `rlëD` 8.
const DIM_555: [u8; 3] = [8, 6, 2];
/// [`DIM_555`] widened.
const DIM: [u8; 4] = [66, 49, 16, 255];

/// `rlëD` 7, a layer sheet: a transparent white column (which a layer
/// that ignored alpha would OR in), an opaque black column, then two
/// [`LIT`] columns.
fn or_sheet() -> Image {
    let row = [[255, 255, 255, 0], BLACK, LIT, LIT].concat();
    Image::from_rgba(4, 4, row.repeat(4)).expect("4x4")
}

impl ImageSource for Images {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let solid = |rgba: [u8; 4]| Image::from_rgba(4, 4, rgba.repeat(16)).expect("4x4");
        match (kind, id) {
            (ImageKind::Pict, 1) => Ok(vec![patterned(4, 2)]),
            (ImageKind::Rled, 2) => Ok(vec![solid([255, 255, 255, 255])]),
            (ImageKind::Rled, 3) => Ok(vec![patterned(2, 4)]),
            (ImageKind::Rled, 4) => Ok(vec![solid([0, 0, 0, 255])]),
            (ImageKind::Rled, 5) => Ok(vec![solid(ORANGE)]),
            (ImageKind::Rled, 7) => Ok(vec![or_sheet()]),
            (ImageKind::Rled, 8) => Ok(vec![solid(DIM)]),
            (ImageKind::Rled, 9) => Ok(vec![solid(LIT)]),
            (ImageKind::Pict, 10) => Ok(vec![solid(TEAL)]),
            (ImageKind::Pict, 11) => Ok(vec![solid(VIOLET)]),
            (ImageKind::Pict, 12) => Ok(vec![solid(BLACK)]),
            (ImageKind::Pict, 13) => Ok(vec![solid([255, 255, 255, 0])]),
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
pub(crate) fn gpu_with(width: u32, height: u32, faces: &FontFaces) -> Option<OffscreenGpu> {
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
/// `top..bottom` (scale 2, content offset 8 rows) is `want` within
/// `tolerance` per channel.
fn assert_area(
    pixels: &[u8],
    (left, top): (u32, u32),
    (right, bottom): (u32, u32),
    want: [u8; 4],
    tolerance: u8,
) {
    for y in 2 * top + 8..2 * bottom + 8 {
        for x in 2 * left..2 * right {
            assert_near(pixels, (x, y), want, tolerance);
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
    assert_area(&pixels, (4, 4), (8, 8), [64, 64, 64, 255], 2);
    // Orange adds its colour.
    assert_area(&pixels, (14, 4), (18, 8), [128, 96, 64, 255], 2);
    // Red at half alpha adds half of it.
    assert_area(&pixels, (24, 4), (28, 8), [192, 64, 64, 255], 2);
    // Over black it is its own colour.
    assert_area(&pixels, (14, 14), (18, 18), ORANGE, 2);
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
        assert_area(&pixels, at, (at.0 + 1, at.1 + 1), BLACK, 2);
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
    assert_area(&pixels, (8, 8), (10, 12), [128, 0, 0, 255], 2);
    // B added over A.
    assert_area(&pixels, (10, 10), (12, 12), [192, 32, 0, 255], 2);
    // B over the clear.
    assert_area(&pixels, (12, 10), (14, 12), ORANGE, 2);
    assert_area(&pixels, (10, 12), (12, 14), ORANGE, 2);
    // C painted over B.
    assert_area(&pixels, (12, 12), (14, 14), [0, 0, 128, 255], 2);
    // C alone.
    assert_area(&pixels, (14, 12), (16, 16), [0, 0, 128, 255], 2);
}

/// The original's 16-bit light blit at `level` out of 32, transcribed
/// from `EV Nova`'s `_BlitPixieTranslucentCopy` (0xc1110) as
/// `_BlitPixieRLETranslucent` (0xc1568) sets it up for a lights sprite
/// (`_alpha` = 32, `_alphaC` = level, no bias): per 5-bit channel,
/// `((src * level) >> 5) | dst`. At level 32 it is `src | dst`,
/// `_BlitPixieRLEAddOver` (0xc24bf).
fn original_lights_555(src: [u8; 3], dst: [u8; 3], level: u8) -> [u8; 3] {
    let level = u16::from(level);
    std::array::from_fn(|c| (((u16::from(src[c]) * level) >> 5) as u8) | dst[c])
}

fn tint(rgba: [u8; 4]) -> Color {
    Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// Five-bit (16, 12, 0): a hull colour sharing set bits with [`LIT_555`]
/// in red and green.
const HULL_555: [u8; 3] = [16, 12, 0];

#[test]
fn an_or_sprite_ors_its_colour_into_what_is_beneath_not_adds() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // Hull squares at logical (4..8, 4..8), a 5-bit colour, and (14..18,
    // 4..8), a colour that is not 5-bit; the OR sheet over each and over
    // the black clear at (24..28, 4..8). Each sheet's columns are
    // transparent (x 4..5), black (5..6) and lit (6..8), and likewise 10
    // and 20 further right. The three sheets overlap nothing of each
    // other's, so they are one batch.
    let hull = w555(HULL_555);
    assert_eq!(hull, [132, 99, 0, 255]);
    let odd = [100, 37, 250, 255];
    let spots = [6.0, 16.0, 26.0].map(|x| Point::new(x, 6.0));
    let mut list = DrawList::new();
    list.sprite(ImageKey::sprite(2, 0), spots[0], tint(hull))
        .sprite(ImageKey::sprite(2, 0), spots[1], tint(odd));
    for spot in spots {
        list.or_sprite(ImageKey::sprite(7, 0), spot, Color::WHITE);
    }
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // Transparent and black layer pixels leave what is beneath exactly.
    assert_area(&pixels, (4, 4), (6, 8), hull, 0);
    assert_area(&pixels, (14, 4), (16, 8), odd, 0);
    assert_area(&pixels, (24, 4), (26, 8), BLACK, 0);
    // Lit over the 5-bit hull: 16|16, 12|10, 0|5 is (16, 14, 5), widened;
    // not the sum.
    let ored = w555(original_lights_555(LIT_555, HULL_555, 32));
    assert_eq!(ored, [132, 115, 41, 255]);
    assert_ne!(ored, [255, 181, 41, 255]);
    assert_area(&pixels, (6, 4), (8, 8), ored, 0);
    // Lit over the other: ORed at 8 bits against it as it is.
    assert_area(&pixels, (16, 4), (18, 8), [228, 119, 251, 255], 0);
    // Lit over the clear: the layer's own colour.
    assert_area(&pixels, (26, 4), (28, 8), LIT, 0);
}

#[test]
fn an_or_fill_ors_its_5_bit_colour_into_what_is_beneath_not_adds() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // Hull squares at logical (4..8, 4..8), a 5-bit colour, and (14..18,
    // 4..8), a colour that is not 5-bit, then one OR fill over x 5..27,
    // y 4..8: across both and the black clear. Its colour's low bits are
    // dropped: (19, 77, 167) is 5-bit (2, 9, 20).
    let hull = w555(HULL_555);
    let odd = [100, 37, 250, 255];
    let fill_555 = [2, 9, 20];
    let mut list = DrawList::new();
    list.sprite(ImageKey::sprite(2, 0), Point::new(6.0, 6.0), tint(hull))
        .sprite(ImageKey::sprite(2, 0), Point::new(16.0, 6.0), tint(odd))
        .or_fill(
            Bounds {
                min: Point::new(5.0, 4.0),
                max: Point::new(27.0, 8.0),
            },
            Color::rgba(19, 77, 167, 255),
        );
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // Over the 5-bit hull: the 5-bit OR, widened, not the sum.
    // 16|2, 12|9, 0|20.
    let ored = w555([18, 13, 20]);
    assert_ne!(ored, [148, 255, 165, 255]);
    assert_area(&pixels, (5, 4), (8, 8), ored, 0);
    // Over the other: ORed at 8 bits against it as it is.
    // 100|16, 37|74, 250|165 against the fill widened to (16, 74, 165).
    assert_area(&pixels, (14, 4), (18, 8), [116, 111, 255, 255], 0);
    // Over the clear: the fill's colour, widened.
    assert_area(&pixels, (20, 4), (27, 8), w555(fill_555), 0);
    // Outside the rect: as it was.
    assert_area(&pixels, (4, 4), (5, 8), hull, 0);
    assert_area(&pixels, (27, 4), (30, 8), BLACK, 0);
    assert_area(&pixels, (5, 8), (27, 10), BLACK, 0);
}

#[test]
fn a_partial_level_light_composites_like_the_originals_translucent_blit() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // Hull squares at logical (4..8, 4..8) and (14..18, 4..8), with the OR
    // sheet over the first at level 16 and over the second at level 10
    // (the stock triangle blink's lowest), and over the black clear at
    // level 16, (24..28, 4..8). Level n is white at n 32nds of full alpha.
    let (level_16, level_10) = (
        Color::rgba(255, 255, 255, 128),
        Color::rgba(255, 255, 255, 80),
    );
    let hull = w555(HULL_555);
    let spots = [6.0, 16.0, 26.0].map(|x| Point::new(x, 6.0));
    let mut list = DrawList::new();
    list.sprite(ImageKey::sprite(2, 0), spots[0], tint(hull))
        .sprite(ImageKey::sprite(2, 0), spots[1], tint(hull))
        .or_sprite(ImageKey::sprite(7, 0), spots[0], level_16)
        .or_sprite(ImageKey::sprite(7, 0), spots[1], level_10)
        .or_sprite(ImageKey::sprite(7, 0), spots[2], level_16);
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // Transparent and black layer pixels leave the hull and the clear.
    assert_area(&pixels, (4, 4), (6, 8), hull, 0);
    assert_area(&pixels, (14, 4), (16, 8), hull, 0);
    assert_area(&pixels, (24, 4), (26, 8), BLACK, 0);
    // A lit one is scaled by level/32, floored in 5 bits, and ORed in.
    let at_16 = w555(original_lights_555(LIT_555, HULL_555, 16));
    assert_eq!(at_16, [198, 107, 16, 255]);
    assert_area(&pixels, (6, 4), (8, 8), at_16, 0);
    // In blue, 5 x 10 / 32 floors to 1 (8), where rounding would give 2.
    let at_10 = w555(original_lights_555(LIT_555, HULL_555, 10));
    assert_eq!(at_10, [173, 123, 8, 255]);
    assert_area(&pixels, (16, 4), (18, 8), at_10, 0);
    let alone = w555(original_lights_555(LIT_555, [0, 0, 0], 16));
    assert_eq!(alone, [66, 41, 16, 255]);
    assert_area(&pixels, (26, 4), (28, 8), alone, 0);
}

/// The original's 16-bit translucent copy with its channel factors at 0,
/// transcribed from `EV Nova`'s `_BlitPixieTranslucentCopy` (0xc1110) as a
/// shot's plain fade and an escape pod set it up: per 5-bit channel,
/// `((src * k) >> 5) | ((dst * (32 - k)) >> 5)`, k the source's opacity in
/// 32nds.
fn original_crossfade_555(src: [u8; 3], dst: [u8; 3], k: u8) -> [u8; 3] {
    let (k, rest) = (u16::from(k), 32 - u16::from(k));
    std::array::from_fn(|c| {
        (((u16::from(src[c]) * k) >> 5) | ((u16::from(dst[c]) * rest) >> 5)) as u8
    })
}

#[test]
fn a_translucent_sprite_crossfades_by_or_like_the_originals_translucent_copy() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // Hull squares at logical (4..8, 4..8) and (14..18, 4..8), with the OR
    // sheet crossfaded over the first translucent by 8 (k = 24) and over
    // the second by 24 (k = 8), and over the black clear by 8, (24..28,
    // 4..8). Below, a hull square at (4..8, 14..18) with the sheet over it
    // at k = 0, and two solid lit squares crossfaded at k = 16 over the
    // clear, (14..18, 14..18) then (16..20, 14..18), the second over half
    // the first. Beside them, a hull square at (24..28, 14..18) with the
    // sheet crossfaded over it by 8 at a murk of 16, its colour halved.
    // White at (32 - t) 32nds of full alpha: a shot translucent by t.
    let translucent_by = |t: u8| match t {
        8 => Color::rgba(255, 255, 255, 191),
        16 => Color::rgba(255, 255, 255, 128),
        24 => Color::rgba(255, 255, 255, 64),
        32 => Color::rgba(255, 255, 255, 0),
        _ => unreachable!("translucent by {t}"),
    };
    let hull = w555(HULL_555);
    let spots = [6.0, 16.0, 26.0].map(|x| Point::new(x, 6.0));
    let mut list = DrawList::new();
    list.sprite(ImageKey::sprite(2, 0), spots[0], tint(hull))
        .sprite(ImageKey::sprite(2, 0), spots[1], tint(hull))
        .sprite(ImageKey::sprite(2, 0), Point::new(6.0, 16.0), tint(hull))
        .sprite(ImageKey::sprite(2, 0), Point::new(26.0, 16.0), tint(hull))
        .crossfade_sprite(ImageKey::sprite(7, 0), spots[0], translucent_by(8))
        .crossfade_sprite(ImageKey::sprite(7, 0), spots[1], translucent_by(24))
        .crossfade_sprite(ImageKey::sprite(7, 0), spots[2], translucent_by(8))
        .crossfade_sprite(
            ImageKey::sprite(7, 0),
            Point::new(6.0, 16.0),
            translucent_by(32),
        )
        .crossfade_sprite(
            ImageKey::sprite(7, 0),
            Point::new(26.0, 16.0),
            // Translucent by 8, its colour halved by a murk of 16.
            Color::rgba(127, 127, 127, 191),
        )
        .crossfade_sprite(
            ImageKey::sprite(9, 0),
            Point::new(16.0, 16.0),
            translucent_by(16),
        )
        .crossfade_sprite(
            ImageKey::sprite(9, 0),
            Point::new(18.0, 16.0),
            translucent_by(16),
        );
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // A transparent pixel leaves what is beneath exactly.
    assert_area(&pixels, (4, 4), (5, 8), hull, 0);
    assert_area(&pixels, (14, 4), (15, 8), hull, 0);
    assert_area(&pixels, (24, 4), (25, 8), BLACK, 0);
    // An opaque black one darkens it: the hull × 8 >> 5, not left alone
    // as the OR leaves it.
    let black_24 = w555(original_crossfade_555([0, 0, 0], HULL_555, 24));
    assert_eq!(black_24, [33, 24, 0, 255]);
    assert_area(&pixels, (5, 4), (6, 8), black_24, 0);
    let black_8 = w555(original_crossfade_555([0, 0, 0], HULL_555, 8));
    assert_eq!(black_8, [99, 74, 0, 255]);
    assert_area(&pixels, (15, 4), (16, 8), black_8, 0);
    // A lit one is scaled by k and the hull by 32 - k, both floored in 5
    // bits, and ORed: not the normal alpha blend.
    let lit_24 = w555(original_crossfade_555(LIT_555, HULL_555, 24));
    assert_eq!(lit_24, [99, 57, 24, 255]);
    assert_ne!(lit_24, [132, 86, 31, 255], "the normal blend at 24/32");
    assert_area(&pixels, (6, 4), (8, 8), lit_24, 0);
    let lit_8 = w555(original_crossfade_555(LIT_555, HULL_555, 8));
    assert_eq!(lit_8, [99, 90, 8, 255]);
    assert_area(&pixels, (16, 4), (18, 8), lit_8, 0);
    // Over the clear.
    let alone = w555(original_crossfade_555(LIT_555, [0, 0, 0], 24));
    assert_eq!(alone, [99, 57, 24, 255]);
    assert_area(&pixels, (25, 4), (26, 8), BLACK, 0);
    assert_area(&pixels, (26, 4), (28, 8), alone, 0);
    // At k = 0 nothing changes.
    assert_area(&pixels, (4, 14), (8, 18), hull, 0);
    // The later crossfade sees the earlier where they overlap.
    let first = w555(original_crossfade_555(LIT_555, [0, 0, 0], 16));
    let both = w555(original_crossfade_555(
        LIT_555,
        original_crossfade_555(LIT_555, [0, 0, 0], 16),
        16,
    ));
    assert_eq!((first, both), ([66, 41, 16, 255], [99, 57, 24, 255]));
    assert_area(&pixels, (14, 14), (16, 18), first, 0);
    assert_area(&pixels, (16, 14), (18, 18), both, 0);
    assert_area(&pixels, (18, 14), (20, 18), first, 0);
    // A darkened tint scales the source's colour before the crossfade:
    // at a murk of 16, by 16/32.
    let murked = w555(original_crossfade_555(LIT_555.map(|c| c / 2), HULL_555, 24));
    assert_eq!(murked, [49, 24, 8, 255]);
    assert_ne!(murked, lit_24, "the crossfade ignoring the tint's colour");
    assert_area(&pixels, (26, 14), (28, 18), murked, 0);
}

/// The original's 16-bit translucent copy with its factors set apart, as
/// a fully cloaked ship's hull sets them up: per 5-bit channel,
/// `((src * n) >> 5) | ((dst * m) >> 5)`.
fn original_translucent_555(src: [u8; 3], dst: [u8; 3], n: u8, m: u8) -> [u8; 3] {
    let (n, m) = (u16::from(n), u16::from(m));
    std::array::from_fn(|c| (((u16::from(src[c]) * n) >> 5) | ((u16::from(dst[c]) * m) >> 5)) as u8)
}

#[test]
fn a_translucent_sprite_draws_like_the_originals_translucent_copy_with_its_factors_apart() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // Hull squares at logical (4..8, 4..8) and (14..18, 4..8), with the OR
    // sheet drawn over the first at n = 4, m = 30 (a fully cloaked hull)
    // and over the second at n = 16, m = 8, and over the clear at n = 4,
    // m = 30, (24..28, 4..8). Each factor is its 32nds of full: n in the
    // colour, m in the alpha.
    let cloaked = Color::rgba(32, 32, 32, 239);
    let half = Color::rgba(128, 128, 128, 64);
    let hull = w555(HULL_555);
    let spots = [6.0, 16.0, 26.0].map(|x| Point::new(x, 6.0));
    let mut list = DrawList::new();
    list.sprite(ImageKey::sprite(2, 0), spots[0], tint(hull))
        .sprite(ImageKey::sprite(2, 0), spots[1], tint(hull))
        .translucent_sprite(ImageKey::sprite(7, 0), spots[0], cloaked)
        .translucent_sprite(ImageKey::sprite(7, 0), spots[1], half)
        .translucent_sprite(ImageKey::sprite(7, 0), spots[2], cloaked);
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // A transparent pixel leaves what is beneath exactly.
    assert_area(&pixels, (4, 4), (5, 8), hull, 0);
    assert_area(&pixels, (24, 4), (25, 8), BLACK, 0);
    // An opaque black one scales the hull by m alone.
    let black = w555(original_translucent_555([0, 0, 0], HULL_555, 4, 30));
    assert_area(&pixels, (5, 4), (6, 8), black, 0);
    // A lit one: the source by n, the hull by m, ORed.
    let faint = w555(original_translucent_555(LIT_555, HULL_555, 4, 30));
    assert_ne!(
        faint,
        w555(original_crossfade_555(LIT_555, HULL_555, 4)),
        "the crossfade's 32 - n"
    );
    assert_area(&pixels, (6, 4), (8, 8), faint, 0);
    let other = w555(original_translucent_555(LIT_555, HULL_555, 16, 8));
    assert_area(&pixels, (16, 4), (18, 8), other, 0);
    let alone = w555(original_translucent_555(LIT_555, [0, 0, 0], 4, 30));
    assert_area(&pixels, (26, 4), (28, 8), alone, 0);
}

#[test]
fn a_colorized_sprite_mixes_each_5_bit_channel_like_the_originals_blit() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // `rlëD` 9, solid LIT, at logical (4..8, 4..8) colorized at level 16
    // toward a 5-bit blue, at (14..18, 4..8) at level 0, and at (24..28,
    // 4..8) at level 26 under a half alpha tint.
    let blue = Colorize {
        level: 16,
        rgb5: [0, 4, 31],
    };
    let spots = [6.0, 16.0, 26.0].map(|x| Point::new(x, 6.0));
    let solid = ImageKey::sprite(9, 0);
    let mut list = DrawList::new();
    list.colorized_sprite(solid, spots[0], Color::WHITE, Blend::Normal, blue)
        .colorized_sprite(
            solid,
            spots[1],
            Color::WHITE,
            Blend::Normal,
            Colorize { level: 0, ..blue },
        )
        .colorized_sprite(
            solid,
            spots[2],
            Color::WHITE,
            Blend::Normal,
            Colorize { level: 26, ..blue },
        );
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // (16, 10, 5) at 16 toward (0, 4, 31): (8 | 0, 5 | 2, 2 | 15).
    let mixed = w555(blue.mix(LIT_555));
    assert_eq!(mixed, w555([8, 7, 15]));
    assert_area(&pixels, (4, 4), (8, 8), mixed, 0);
    assert_area(&pixels, (14, 4), (18, 8), LIT, 0);
    let deep = w555(Colorize { level: 26, ..blue }.mix(LIT_555));
    assert_area(&pixels, (24, 4), (28, 8), deep, 0);
}

#[test]
fn a_colorized_or_layer_mixes_its_lit_texels_and_leaves_its_transparent_ones() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // The OR sheet over the black clear at (4..8, 4..8), colorized at
    // level 20 toward a 5-bit red, at full level.
    let red = Colorize {
        level: 20,
        rgb5: [31, 0, 3],
    };
    let mut list = DrawList::new();
    list.colorized_sprite(
        ImageKey::sprite(7, 0),
        Point::new(6.0, 6.0),
        Color::WHITE,
        Blend::Or,
        red,
    );
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // The transparent column leaves the clear; the opaque black one is
    // mixed toward red too, as the original mixes every drawn texel.
    assert_area(&pixels, (4, 4), (5, 8), BLACK, 0);
    assert_area(&pixels, (5, 4), (6, 8), w555(red.mix([0, 0, 0])), 0);
    assert_area(&pixels, (6, 4), (8, 8), w555(red.mix(LIT_555)), 0);
}

#[test]
fn an_or_sprite_between_normal_ones_keeps_submission_order() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // A normal red square A, logical (8..12, 8..12); an OR square B,
    // (10..14, 10..14), over A's bottom-right quarter; a normal blue
    // square C, (12..16, 12..16), over B's bottom-right quarter.
    let red = w555([16, 0, 0]);
    let blue = [0, 0, 132, 255];
    let mut list = DrawList::new();
    list.sprite(ImageKey::sprite(2, 0), Point::new(10.0, 10.0), tint(red))
        .or_sprite(ImageKey::sprite(8, 0), Point::new(12.0, 12.0), Color::WHITE)
        .sprite(ImageKey::sprite(2, 0), Point::new(14.0, 14.0), tint(blue));
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    // A alone.
    assert_area(&pixels, (8, 8), (10, 12), red, 0);
    // B ORed over A.
    let over_a = w555(original_lights_555(DIM_555, [16, 0, 0], 32));
    assert_eq!(over_a, [198, 49, 16, 255]);
    assert_area(&pixels, (10, 10), (12, 12), over_a, 0);
    // B over the clear.
    assert_area(&pixels, (12, 10), (14, 12), DIM, 0);
    assert_area(&pixels, (10, 12), (12, 14), DIM, 0);
    // C painted over B, and C alone.
    assert_area(&pixels, (12, 12), (14, 14), blue, 0);
    assert_area(&pixels, (14, 12), (16, 16), blue, 0);
}

fn bounds(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
    Bounds {
        min: Point::new(left, top),
        max: Point::new(right, bottom),
    }
}

/// A max picture of `PICT` `id` stretched over the logical rectangle
/// `rect`.
fn max_picture(list: &mut DrawList, id: i16, rect: Bounds) {
    list.blended_picture(
        ImageKey::picture(id),
        rect.min,
        rect.width(),
        rect.height(),
        Blend::Max,
    );
}

/// Renders `list` at scale 2 and reads it back, or `None` with no adapter.
fn render_max(list: &DrawList) -> Option<Vec<u8>> {
    let mut gpu = gpu()?;
    let mut renderer = Renderer::new(Images);
    let report = renderer.render(list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    assert_eq!(report.new_failures, vec![]);
    Some(gpu.read_pixels().expect("read back"))
}

#[test]
fn a_max_picture_takes_the_brighter_of_each_channel_over_a_solid_rect() {
    // A border cell, painted as a filled rectangle is, at logical (4..20,
    // 4..12); a teal max picture over its right half and on past it, at
    // (12..28, 4..12); a transparent max picture over its left quarter.
    let mut list = DrawList::new();
    fill(&mut list, bounds(4.0, 4.0, 20.0, 12.0), tint(BORDER));
    max_picture(&mut list, 10, bounds(12.0, 4.0, 28.0, 12.0));
    max_picture(&mut list, 13, bounds(4.0, 4.0, 8.0, 12.0));
    let Some(pixels) = render_max(&list) else {
        return;
    };

    // The cell alone, and under the transparent picture.
    assert_area(&pixels, (4, 4), (12, 12), BORDER, 1);
    // Over the cell, the brighter of each channel: red from the cell,
    // green from the picture, blue from both.
    assert_area(&pixels, (12, 4), (20, 12), [200, 180, 90, 255], 1);
    // Over the black clear, the picture as it is.
    assert_area(&pixels, (20, 4), (28, 12), TEAL, 1);
    // Around them.
    assert_area(&pixels, (3, 4), (4, 12), BLACK, 0);
    assert_area(&pixels, (4, 12), (28, 13), BLACK, 0);
}

#[test]
fn two_overlapping_max_pictures_take_the_brighter_of_each_channel() {
    // Over the black clear: teal at logical (4..12, 4..12) and violet at
    // (8..16, 8..16). Over a border cell at (18..30, 2..22): teal at
    // (18..26, 4..12) and violet at (22..30, 8..16).
    let mut list = DrawList::new();
    max_picture(&mut list, 10, bounds(4.0, 4.0, 12.0, 12.0));
    max_picture(&mut list, 11, bounds(8.0, 8.0, 16.0, 16.0));
    fill(&mut list, bounds(18.0, 2.0, 30.0, 22.0), tint(BORDER));
    max_picture(&mut list, 10, bounds(18.0, 4.0, 26.0, 12.0));
    max_picture(&mut list, 11, bounds(22.0, 8.0, 30.0, 16.0));
    let Some(pixels) = render_max(&list) else {
        return;
    };

    // Over black: each alone is its own colour, the overlap the brighter
    // of each channel of the two.
    assert_area(&pixels, (4, 4), (8, 12), TEAL, 1);
    assert_area(&pixels, (8, 4), (12, 8), TEAL, 1);
    assert_area(&pixels, (8, 8), (12, 12), [120, 180, 250, 255], 1);
    assert_area(&pixels, (12, 8), (16, 16), VIOLET, 1);
    assert_area(&pixels, (8, 12), (12, 16), VIOLET, 1);
    // Over the cell: the brighter of each channel of all that is there.
    assert_area(&pixels, (18, 2), (30, 4), BORDER, 1);
    assert_area(&pixels, (18, 4), (22, 12), [200, 180, 90, 255], 1);
    assert_area(&pixels, (22, 8), (26, 12), [200, 180, 250, 255], 1);
    assert_area(&pixels, (26, 8), (30, 16), [200, 40, 250, 255], 1);
    assert_area(&pixels, (18, 16), (30, 22), BORDER, 1);
}

#[test]
fn a_plain_picture_after_a_max_one_covers_it() {
    // The partial update's `srcCopy`: teal max at logical (4..12, 4..12),
    // then plain black at (8..12, 4..12) and plain violet at (4..8,
    // 8..12) over it.
    let mut list = DrawList::new();
    max_picture(&mut list, 10, bounds(4.0, 4.0, 12.0, 12.0));
    list.stretched_picture(ImageKey::picture(12), Point::new(8.0, 4.0), 4.0, 8.0)
        .stretched_picture(ImageKey::picture(11), Point::new(4.0, 8.0), 4.0, 4.0);
    let Some(pixels) = render_max(&list) else {
        return;
    };

    assert_area(&pixels, (4, 4), (8, 8), TEAL, 1);
    assert_area(&pixels, (8, 4), (12, 12), BLACK, 0);
    assert_area(&pixels, (4, 8), (8, 12), VIOLET, 1);
}

#[test]
fn a_clipped_max_picture_draws_only_inside_its_clip() {
    // A border cell at logical (4..20, 4..12), and violet max over all of
    // it, clipped to (8..14, 6..10).
    let mut list = DrawList::new();
    fill(&mut list, bounds(4.0, 4.0, 20.0, 12.0), tint(BORDER));
    list.clipped(
        bounds(8.0, 6.0, 14.0, 10.0),
        DrawCommand::BlendedPicture {
            image: ImageKey::picture(11),
            top_left: Point::new(4.0, 4.0),
            width: 16.0,
            height: 8.0,
            blend: Blend::Max,
        },
    );
    let Some(pixels) = render_max(&list) else {
        return;
    };

    assert_area(&pixels, (8, 6), (14, 10), [200, 40, 250, 255], 1);
    assert_area(&pixels, (4, 4), (8, 12), BORDER, 1);
    assert_area(&pixels, (14, 4), (20, 12), BORDER, 1);
    assert_area(&pixels, (8, 4), (14, 6), BORDER, 1);
    assert_area(&pixels, (8, 10), (14, 12), BORDER, 1);
}

#[test]
fn an_or_sprite_sees_the_or_sprite_before_it() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // OR square X, logical (8..12, 8..12), then OR square Y, (10..14,
    // 10..14), over X's bottom-right quarter: they overlap, so Y reads a
    // backdrop that already holds X.
    let mut list = DrawList::new();
    list.or_sprite(ImageKey::sprite(9, 0), Point::new(10.0, 10.0), Color::WHITE)
        .or_sprite(ImageKey::sprite(8, 0), Point::new(12.0, 12.0), Color::WHITE);
    let mut renderer = Renderer::new(Images);

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    assert_eq!(report.new_failures, vec![]);
    assert_area(&pixels, (8, 8), (10, 12), LIT, 0);
    // Y over X: 16|8, 10|6, 5|2, widened. Y on a stale backdrop would be
    // DIM here.
    let both = w555(original_lights_555(DIM_555, LIT_555, 32));
    assert_eq!(both, [198, 115, 57, 255]);
    assert_area(&pixels, (10, 10), (12, 12), both, 0);
    assert_area(&pixels, (12, 10), (14, 14), DIM, 0);
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
fn a_clipped_run_and_picture_draw_only_inside_their_clip() {
    let Some(mut gpu) = gpu() else {
        return;
    };
    // A run over the content's whole width, clipped to logical x 4..12;
    // the 4x2 picture at (20, 18), clipped to its right half.
    let clip = |left: f32, top: f32, right: f32, bottom: f32| Bounds {
        min: Point::new(left, top),
        max: Point::new(right, bottom),
    };
    let mut list = DrawList::new();
    list.clipped(
        clip(4.0, 0.0, 12.0, 24.0),
        DrawCommand::Text {
            text: "MMMMMMMMMMMMMMMM".to_owned(),
            font: Font::Geneva,
            origin: Point::new(0.0, 2.0),
            size: 8.0,
            wrap_width: None,
            color: Color::WHITE,
        },
    )
    .clipped(
        clip(22.0, 0.0, 32.0, 24.0),
        DrawCommand::Picture {
            image: ImageKey::picture(1),
            top_left: Point::new(20.0, 18.0),
        },
    );
    let mut renderer = Renderer::new(Images);

    renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    let pixels = gpu.read_pixels().expect("read back");

    // The run's rows, logical y 2..10, are pixels 12..28: only x 8..24 lit.
    let columns: Vec<u32> = (0..SIZE)
        .filter(|&x| (12..28).any(|y| pixel(&pixels, x, y) != BLACK))
        .collect();
    assert!(!columns.is_empty(), "the run is drawn");
    assert!(
        columns.iter().all(|x| (8..24).contains(x)),
        "lit columns {columns:?}"
    );
    // The picture's left half, logical x 20..22 (pixels 40..44), is cut;
    // its right half, texels 2 and 3, is drawn at pixels 44..48.
    for x in 40..44 {
        for y in 44..48 {
            assert_near(&pixels, (x, y), BLACK, 0);
        }
    }
    for (texel_x, x) in [(2, 44), (3, 46)] {
        for texel_y in 0..2 {
            let at = (x, 44 + 2 * texel_y);
            assert_near(&pixels, at, texel(4, texel_x, texel_y), 0);
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
    let Some(fallback) = gpu_with(SIZE, SIZE, &FontFaces::new(FALLBACK_FONT)) else {
        return;
    };
    assert_eq!(fallback.font_faces(), 1);
    assert!(!fallback.charcoal_loaded());
    let faces = FontFaces::new(FALLBACK_FONT).with_charcoal(block_font("Charcoal"));
    let Some(with_charcoal) = gpu_with(SIZE, SIZE, &faces) else {
        return;
    };
    assert_eq!(with_charcoal.font_faces(), 2);
    assert!(with_charcoal.charcoal_loaded());
}

#[test]
fn the_bundled_faces_load_the_fallback_and_the_serif() {
    let Some(bundled) = gpu() else {
        return;
    };
    assert_eq!(bundled.font_faces(), 2);
    assert!(!bundled.charcoal_loaded());
}

#[test]
fn charcoal_bytes_that_load_no_face_are_not_loaded() {
    let faces = FontFaces::new(FALLBACK_FONT).with_charcoal(&b"not a font"[..]);
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
pub(crate) const TEXT_AT: (f32, f32) = (2.0, 4.0);

/// [`SAMPLE`] in `font` at `size` points, drawn with `faces` into a
/// [`LINE`]-sized window at `scale` physical pixels per unit, as RGBA8
/// pixels; `None` with no GPU adapter.
fn draw_sample(faces: &FontFaces, font: Font, size: f32, scale: u32) -> Option<Picture> {
    let at = Point::new(TEXT_AT.0, TEXT_AT.1);
    draw_text(faces, font, SAMPLE, at, size, scale)
}

/// `text` in `font` at `size` points from `at`, drawn white on black with
/// `faces` into a [`LINE`]-sized window at `scale` physical pixels per
/// unit; `None` with no GPU adapter.
pub(crate) fn draw_text(
    faces: &FontFaces,
    font: Font,
    text: &str,
    at: Point,
    size: f32,
    scale: u32,
) -> Option<Picture> {
    let mut list = DrawList::new();
    list.text_in(font, text, at, size, None, Color::WHITE);
    draw_list(faces, &list, scale)
}

/// `list` drawn with `faces` into a [`LINE`]-sized window at `scale`
/// physical pixels per unit; `None` with no GPU adapter.
pub(crate) fn draw_list(faces: &FontFaces, list: &DrawList, scale: u32) -> Option<Picture> {
    let (w, h) = (LINE.w * scale, LINE.h * scale);
    let mut gpu = gpu_with(w, h, faces)?;
    let report = Renderer::new(Images).render(
        list,
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
pub(crate) struct Picture {
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
    pub(crate) fn lit(&self) -> Vec<(u32, u32)> {
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

    /// The coverage of each run of lit pixels along the row with the most
    /// ink, left to right: one run per stem in a row of `l`s.
    pub(crate) fn stems(&self) -> Vec<Vec<u8>> {
        self.inkiest_row()
            .split(|&c| c == 0)
            .filter(|stem| !stem.is_empty())
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// The column each run of lit pixels starts at along the row with the
    /// most ink, left to right: where each stem of a row of `l`s starts.
    pub(crate) fn stem_columns(&self) -> Vec<u32> {
        let row = self.inkiest_row();
        (0..row.len())
            .filter(|&x| row[x] > 0 && (x == 0 || row[x - 1] == 0))
            .map(|x| x as u32)
            .collect()
    }

    /// The partly covered pixels (lit, but under 250) along the row with
    /// the most ink: the soft edges of every stem in it, together.
    pub(crate) fn row_partials(&self) -> usize {
        self.inkiest_row()
            .into_iter()
            .filter(|&c| partial(c))
            .count()
    }

    /// The partly covered pixels of each of [`Picture::stems`], left to
    /// right.
    pub(crate) fn stem_partials(&self) -> Vec<usize> {
        let stems = self.stems().into_iter();
        stems
            .map(|stem| stem.into_iter().filter(|&c| partial(c)).count())
            .collect()
    }

    /// The coverage along the row with the most lit pixels.
    fn inkiest_row(&self) -> Vec<u8> {
        let mut rows: Vec<Vec<u8>> = Vec::new();
        for ((x, _), c) in self.coverage() {
            if x == 0 {
                rows.push(Vec::new());
            }
            rows.last_mut().expect("a row").push(c);
        }
        rows.into_iter()
            .max_by_key(|row| row.iter().filter(|&&c| c > 0).count())
            .unwrap_or_default()
    }
}

/// Whether `coverage` is a soft edge: lit, but short of full (250 and up).
fn partial(coverage: u8) -> bool {
    coverage > 0 && coverage < 250
}

/// Twelve `l`s: twelve identical stems in one row.
pub(crate) const STEMS: &str = "llllllllllll";

/// Every stem of [`STEMS`] in `picture` has the same coverage profile, as
/// it does when each glyph starts on a whole pixel.
pub(crate) fn assert_stems_alike(picture: &Picture, case: &str) {
    let stems = picture.stems();
    assert_eq!(stems.len(), STEMS.len(), "{case}: {stems:?}");
    let mut profiles = stems;
    profiles.sort();
    profiles.dedup();
    assert_eq!(profiles.len(), 1, "{case}: {profiles:?}");
}

#[test]
fn every_stem_in_a_row_of_ls_draws_the_same_at_1x_and_2x() {
    let faces = FontFaces::bundled();
    // At 9pt and 1x an `l` is narrower than its advance leaves room for a
    // gap beside, so its stems run together: that case has no stems to
    // tell apart.
    for (size, scale) in [(12.0, 1), (9.0, 2), (12.0, 2)] {
        let at = Point::new(TEXT_AT.0, TEXT_AT.1);
        let Some(picture) = draw_text(&faces, Font::Geneva, STEMS, at, size, scale) else {
            return;
        };
        assert_stems_alike(&picture, &format!("{size}pt at {scale}x"));
    }
}

#[test]
fn text_keeps_its_place_against_a_box_at_any_fraction_of_a_pixel() {
    let faces = FontFaces::bundled();
    let red = Color::rgba(255, 0, 0, 255);
    let green = Color::rgba(0, 255, 0, 255);
    // 1x, 2x and a fractional 1.40625x.
    for (w, h, factor) in [(128, 32, 1.0), (256, 64, 2.0), (180, 45, 1.0)] {
        let mut gaps = Vec::new();
        for y in [4.0, 4.25, 4.4, 4.5, 4.6, 4.75] {
            let Some(mut gpu) = gpu_with(w, h, &faces) else {
                return;
            };
            let mut list = DrawList::new();
            fill(&mut list, bounds(60.0, y, 100.0, y + 10.0), green);
            list.text_in(Font::Geneva, "llll", Point::new(2.0, y), 12.0, None, red);
            Renderer::new(Images).render(&list, &Viewport::new(LINE, (w, h), factor), &mut gpu);
            let pixels = gpu.read_pixels().expect("read back");
            let top = |channel: usize| {
                let pixels = pixels.as_chunks::<4>().0;
                let at = pixels.iter().position(|p| p[channel] > 0);
                at.map(|at| at as u32 / w).expect("drawn")
            };
            gaps.push((y, top(0) - top(1)));
        }
        let first = gaps[0].1;
        assert!(
            gaps.iter().all(|&(_, gap)| gap == first),
            "{w}x{h}: rows from the box's top to the ink's: {gaps:?}"
        );
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
    for font in [Font::Geneva, Font::Charcoal, Font::Times] {
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
    for font in [Font::Geneva, Font::Charcoal, Font::Times] {
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
        world_space: WorldSpace::Target,
        batches: Vec::new(),
        world: Vec::new(),
        world_clip: None,
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
        world_space: WorldSpace::Target,
        batches: Vec::new(),
        world: Vec::new(),
        world_clip: None,
    };
    gpu.submit_with(&frame, &mut painter);
    let target = ((SIZE, SIZE), wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(painter.targets, [target]);
    gpu.submit_with(&frame, &mut painter);
    assert_eq!(painter.targets, [target, target]);
    gpu.submit(&frame);
    assert_eq!(painter.targets.len(), 2, "a plain submit paints nothing");
}
