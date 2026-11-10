//! Reading back the small offscreen frame the model tests draw: logical
//! 32x24 in a 64x64 target at scale 2, so the content is (0, 8)-(64, 56)
//! and each logical unit two pixels.

use nova_render::LogicalSize;

/// The logical screen.
pub const LOGICAL: LogicalSize = LogicalSize { w: 32, h: 24 };
/// The target's side, in pixels.
pub const SIZE: u32 = 64;
/// The content's top row.
pub const TOP: f32 = 8.0;

/// How far a lit face's channel may be from the core's shade: the GPU
/// interpolates the normal and rounds to 8 bits.
pub const TOLERANCE: u8 = 2;

/// The target pixel holding logical (`x`, `y`).
pub fn pixel_of(x: f32, y: f32) -> (u32, u32) {
    ((x * 2.0) as u32, (TOP + y * 2.0) as u32)
}

/// The pixel at (`x`, `y`) of `pixels`, RGBA.
pub fn pixel(pixels: &[u8], (x, y): (u32, u32)) -> [u8; 4] {
    let at = ((y * SIZE + x) * 4) as usize;
    pixels[at..at + 4].try_into().expect("four bytes")
}

/// The colour at logical (`x`, `y`).
pub fn color_at(pixels: &[u8], x: f32, y: f32) -> [u8; 3] {
    rgb(pixel(pixels, pixel_of(x, y)))
}

/// A pixel's colour, its alpha dropped.
pub fn rgb([red, green, blue, _]: [u8; 4]) -> [u8; 3] {
    [red, green, blue]
}

/// Whether each channel of `a` is within [`TOLERANCE`] of `b`'s.
pub fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(&b).all(|(p, q)| p.abs_diff(*q) <= TOLERANCE)
}

/// Asserts the colour at logical `(x, y)` is [`near`] `expected`.
#[track_caller]
pub fn assert_near(pixels: &[u8], (x, y): (f32, f32), expected: [u8; 3], what: &str) {
    let got = color_at(pixels, x, y);
    assert!(
        near(got, expected),
        "{what} at ({x}, {y}): {got:?}, not {expected:?}"
    );
}

/// Every target pixel whose logical area lies inside `area`, given as
/// `(x0, y0, x1, y1)`.
pub fn pixels_inside((x0, y0, x1, y1): (f32, f32, f32, f32)) -> Vec<(u32, u32)> {
    (0..SIZE)
        .flat_map(|py| (0..SIZE).map(move |px| (px, py)))
        .filter(|&(px, py)| {
            let (x, y) = (px as f32 / 2.0, (py as f32 - TOP) / 2.0);
            x >= x0 && x + 0.5 <= x1 && y >= y0 && y + 0.5 <= y1
        })
        .collect()
}
