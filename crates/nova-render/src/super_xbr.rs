//! Super-xBR at 2x on the CPU: an [`Enlarger`] that smooths each image's
//! edges as it doubles it, once, when the image loads into the atlas.
//!
//! A port of Hyllian's Super-xBR, as the libretro/slang-shaders preset
//! `edge-smoothing/xbr/super-xbr.slangp` runs it at commit `e1d7563`: the
//! luma pass (`shaders/support/luma.slang`), then
//! `edge-smoothing/xbr/shaders/super-xbr/super-xbr-pass0.slang`,
//! `-pass1.slang` and `-pass2.slang`, with the preset's edge strengths
//! (`XBR_EDGE_STR_P0`, `_P1`, `_P2` = 3) and the shaders' defaults (`MODE`
//! 1, "Details"; `XBR_WEIGHT` 0 in pass 0, 1 in passes 1 and 2). The
//! preset's last two passes, a Jinc2 resample to the viewport and a
//! deblur, are not ported: the atlas holds the 2x image, and the quad
//! drawing it samples it nearest, as every atlas page is.
//!
//! How this port differs from the shaders:
//!
//! - It works on premultiplied RGBA, in `f32` between passes. Edges are
//!   found from the luma of the premultiplied colour, the colour over
//!   black, and alpha rides along as a fourth channel through the same
//!   weights, blends and anti-ringing clamp. (The shaders keep luma in
//!   alpha and throw the image's alpha away.) At the end each colour is
//!   clamped to at most its alpha and un-premultiplied; a texel left with
//!   no alpha has no colour.
//! - Texels outside the image are its nearest edge texel, as the preset's
//!   `clamp_to_edge` wrap gives.
//! - The shaders' sRGB framebuffers only store their values more finely,
//!   so this port's outputs may differ from the GPU's by rounding.
//!
//! The shaders carry this notice, which travels with the port:
//!
//! ```text
//! Copyright (c) 2015 Hyllian - sergiogdb@gmail.com
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
//! THE SOFTWARE.
//! ```

use std::borrow::Cow;

use nova_data::graphics::Image;
use nova_view::ImageKind;

use crate::enlarge::Enlarger;

/// Super-xBR at 2x, for every kind of image but a pattern: a `ppat` is
/// tiled, and enlarging one tile with its edges clamped would put seams
/// between the tiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuperXbr;

impl Enlarger for SuperXbr {
    /// [`enlarge`]d, or a pattern borrowed as it is.
    fn enlarge<'a>(&self, kind: ImageKind, image: &'a Image) -> Cow<'a, Image> {
        if matches!(kind, ImageKind::Pattern) {
            Cow::Borrowed(image)
        } else {
            Cow::Owned(enlarge(image))
        }
    }

    /// Every frame [`enlarge`]d, the frames spread over the machine's
    /// threads; a pattern's borrowed as they are.
    fn enlarge_frames<'a>(&self, kind: ImageKind, frames: &'a [Image]) -> Vec<Cow<'a, Image>> {
        if matches!(kind, ImageKind::Pattern) {
            return frames.iter().map(Cow::Borrowed).collect();
        }
        enlarge_all(frames, threads())
            .into_iter()
            .map(Cow::Owned)
            .collect()
    }
}

/// How many threads the machine runs at once, at least one.
fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// Every one of `frames` enlarged, in order, on at most `threads` threads:
/// the frames split into runs, one thread a run, each frame given its
/// share of the threads for its passes' bands. A frame's bytes don't
/// depend on the threads it is given.
fn enlarge_all(frames: &[Image], threads: usize) -> Vec<Image> {
    let workers = threads.min(frames.len()).max(1);
    let each = (threads / workers).max(1);
    if workers == 1 {
        return frames
            .iter()
            .map(|frame| enlarge_with(frame, each))
            .collect();
    }
    let run = frames.len().div_ceil(workers);
    std::thread::scope(|scope| {
        let runs: Vec<_> = frames
            .chunks(run)
            .map(|run| {
                scope.spawn(move || {
                    run.iter()
                        .map(|frame| enlarge_with(frame, each))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().expect("an enlarging thread panicked"))
            .collect()
    })
}

/// `image` at twice its width and height, by Super-xBR's three passes,
/// each split into bands of rows across the threads the machine runs at
/// once ([`std::thread::available_parallelism`]). It returns when the
/// image is done, so a renderer that loads with it draws the image enlarged
/// from the first frame.
#[must_use]
pub fn enlarge(image: &Image) -> Image {
    enlarge_with(image, threads())
}

/// [`enlarge`] on at most `threads` threads. Every texel is computed the
/// same way whichever thread computes it, so the bytes are the same for
/// any number.
fn enlarge_with(image: &Image, threads: usize) -> Image {
    let original = Plane::premultiplied(image);
    if original.texels.is_empty() {
        return Image::from_rgba(2 * image.width(), 2 * image.height(), Vec::new())
            .expect("no texels");
    }
    let diagonal = pass0(&original, threads);
    let doubled = pass1(&original, &diagonal, threads);
    pass2(&doubled, threads).straight()
}

/// The fewest texels a thread computes in a pass: smaller bands would
/// spend more starting threads than they save.
const MIN_BAND_TEXELS: usize = 16_384;

/// A premultiplied RGBA texel, each channel 0 to 1.
type Texel = [f32; 4];

/// An image as the passes work on it: premultiplied texels in `f32`, row
/// by row.
#[derive(Clone, Debug, PartialEq)]
struct Plane {
    width: usize,
    height: usize,
    texels: Vec<Texel>,
}

impl Plane {
    /// A `width` x `height` plane of `texels`, row by row.
    fn new(width: usize, height: usize, texels: Vec<Texel>) -> Self {
        debug_assert_eq!(texels.len(), width * height);
        Self {
            width,
            height,
            texels,
        }
    }

    /// A `width` x `height` plane, each texel `texel(x, y)`, its rows
    /// computed in bands on up to `threads` threads, each band at least
    /// [`MIN_BAND_TEXELS`].
    fn from_fn(
        width: usize,
        height: usize,
        threads: usize,
        texel: impl Fn(usize, usize) -> Texel + Sync,
    ) -> Self {
        let mut texels = vec![[0.0; 4]; width * height];
        let rows = height
            .div_ceil(threads.max(1))
            .max(MIN_BAND_TEXELS.div_ceil(width.max(1)));
        let band_len = (rows * width).max(1);
        let fill = |band: usize, chunk: &mut [Texel]| {
            for (n, slot) in chunk.iter_mut().enumerate() {
                let at = band * band_len + n;
                *slot = texel(at % width, at / width);
            }
        };
        if texels.len() <= band_len {
            fill(0, &mut texels);
        } else {
            std::thread::scope(|scope| {
                for (band, chunk) in texels.chunks_mut(band_len).enumerate() {
                    let fill = &fill;
                    scope.spawn(move || fill(band, chunk));
                }
            });
        }
        Self::new(width, height, texels)
    }

    /// `image`'s texels, premultiplied.
    fn premultiplied(image: &Image) -> Self {
        let unit = |byte: u8| f32::from(byte) / 255.0;
        let texels = image
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, a]| {
                let alpha = unit(a);
                [unit(r) * alpha, unit(g) * alpha, unit(b) * alpha, alpha]
            })
            .collect();
        Self::new(image.width() as usize, image.height() as usize, texels)
    }

    /// The texel at (`x`, `y`), inside the plane.
    fn texel(&self, x: usize, y: usize) -> Texel {
        self.texels[y * self.width + x]
    }

    /// The texel at (`x`, `y`), or the nearest edge texel outside.
    fn at(&self, x: isize, y: isize) -> Texel {
        let clamp = |at: isize, size: usize| at.clamp(0, size as isize - 1) as usize;
        self.texel(clamp(x, self.width), clamp(y, self.height))
    }

    /// The plane as a straight-alpha image: each colour clamped to at most
    /// its alpha and un-premultiplied, and a texel with no alpha left
    /// clear.
    fn straight(&self) -> Image {
        let byte = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
        let pixels = self
            .texels
            .iter()
            .flat_map(|&[r, g, b, a]| {
                let alpha = byte(a);
                if alpha == 0 {
                    return [0; 4];
                }
                let colour = |c: f32| byte(c.clamp(0.0, a) / a);
                [colour(r), colour(g), colour(b), alpha]
            })
            .collect();
        Image::from_rgba(self.width as u32, self.height as u32, pixels).expect("four per texel")
    }
}

/// The sixteen texels a pass reads round the point it computes, named as
/// the shaders name them: E, F, H and I are the four nearest, the point
/// between them.
///
/// ```text
///      |P0|B |C |P1|
///      |D |E |F |F4|
///      |G |H |I |I4|
///      |P2|H5|I5|P3|
/// ```
#[derive(Clone, Copy, Debug)]
struct Window<T> {
    p0: T,
    p1: T,
    p2: T,
    p3: T,
    b: T,
    c: T,
    d: T,
    e: T,
    f: T,
    g: T,
    h: T,
    i: T,
    f4: T,
    i4: T,
    h5: T,
    i5: T,
}

impl<T: Copy> Window<T> {
    /// Each texel turned by `f`.
    fn map<U>(&self, f: impl Fn(T) -> U) -> Window<U> {
        Window {
            p0: f(self.p0),
            p1: f(self.p1),
            p2: f(self.p2),
            p3: f(self.p3),
            b: f(self.b),
            c: f(self.c),
            d: f(self.d),
            e: f(self.e),
            f: f(self.f),
            g: f(self.g),
            h: f(self.h),
            i: f(self.i),
            f4: f(self.f4),
            i4: f(self.i4),
            h5: f(self.h5),
            i5: f(self.i5),
        }
    }
}

/// The window of `plane` with E at (`x`, `y`): the layout of passes 0 and
/// 2, the point computed at E's bottom-right corner.
fn square(plane: &Plane, x: isize, y: isize) -> Window<Texel> {
    let at = |dx: isize, dy: isize| plane.at(x + dx, y + dy);
    Window {
        p0: at(-1, -1),
        p1: at(2, -1),
        p2: at(-1, 2),
        p3: at(2, 2),
        b: at(0, -1),
        c: at(1, -1),
        d: at(-1, 0),
        e: at(0, 0),
        f: at(1, 0),
        g: at(-1, 1),
        h: at(0, 1),
        i: at(1, 1),
        f4: at(2, 0),
        i4: at(2, 1),
        h5: at(0, 2),
        i5: at(1, 2),
    }
}

/// One pass's settings: the edge detector's weights (`wp1` to `wp6`), the
/// two filters' (`weight1`, `weight2`) and the edge strength.
#[derive(Clone, Copy, Debug)]
struct Pass {
    wp: [f32; 6],
    weight1: f32,
    weight2: f32,
    strength: f32,
}

/// The preset's edge strength, in every pass.
const EDGE_STRENGTH: f32 = 3.0;

/// Pass 1's and pass 2's filter weights, `XBR_WEIGHT` being 1 there.
const DETAIL_WEIGHT1: f32 = 1.29633 / 10.0;
const DETAIL_WEIGHT2: f32 = 1.75068 / 10.0 / 2.0;

/// Pass 0: `XBR_WEIGHT` 0, so its filters average two and four texels.
const PASS0: Pass = Pass {
    wp: [2.0, 1.0, -1.0, 4.0, -1.0, 1.0],
    weight1: 0.0,
    weight2: 0.0,
    strength: EDGE_STRENGTH,
};

/// Pass 1, in "Details" mode.
const PASS1: Pass = Pass {
    wp: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    weight1: DETAIL_WEIGHT1,
    weight2: DETAIL_WEIGHT2,
    strength: EDGE_STRENGTH,
};

/// Pass 2, in "Details" mode.
const PASS2: Pass = Pass {
    wp: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    weight1: DETAIL_WEIGHT1,
    weight2: DETAIL_WEIGHT2,
    strength: EDGE_STRENGTH,
};

/// The shaders' luma: Rec. 709's weights.
fn luma([r, g, b, _]: Texel) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn df(a: f32, b: f32) -> f32 {
    (a - b).abs()
}

/// The shaders' `d_wd`: how strong an edge is along one diagonal.
fn d_wd(wp: [f32; 6], [b0, b1, c0, c1, c2, d0, d1, d2, d3, e1, e2, e3, f2, f3]: [f32; 14]) -> f32 {
    wp[0] * (df(c1, c2) + df(c1, c0) + df(e2, e1) + df(e2, e3))
        + wp[1] * (df(d2, d3) + df(d0, d1))
        + wp[2] * (df(d1, d3) + df(d0, d2))
        + wp[3] * df(d1, d2)
        + wp[4] * (df(c0, c2) + df(e1, e3))
        + wp[5] * (df(b0, b1) + df(f2, f3))
}

/// The shaders' `hv_wd`: how strong an edge is across one axis.
fn hv_wd(wp: [f32; 6], [i1, i2, i3, i4, e1, e2, e3, e4]: [f32; 8]) -> f32 {
    wp[3] * (df(i1, i2) + df(i3, i4))
        + wp[0] * (df(i1, e1) + df(i2, e2) + df(i3, e3) + df(i4, e4))
        + wp[2] * (df(i1, e2) + df(i3, e4) + df(e1, i2) + df(e3, i4))
}

/// GLSL's `smoothstep(0.0, edge1, x)`, the only way the passes call it.
fn smoothstep(edge1: f32, x: f32) -> f32 {
    let t = (x / edge1).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `weights` applied to four texels, channel by channel.
fn filter(weights: [f32; 4], texels: [Texel; 4]) -> Texel {
    std::array::from_fn(|c| {
        weights[0] * texels[0][c]
            + weights[1] * texels[1][c]
            + weights[2] * texels[2][c]
            + weights[3] * texels[3][c]
    })
}

fn add(a: Texel, b: Texel) -> Texel {
    std::array::from_fn(|c| a[c] + b[c])
}

/// The texel `pass` computes from `window`: the stronger diagonal's
/// two-tap blend, mixed with the stronger axis's by how weak the diagonal
/// edge is, then clamped between the four nearest texels (the shaders'
/// anti-ringing).
fn kernel(window: &Window<Texel>, pass: &Pass) -> Texel {
    let l = window.map(luma);
    let wp = pass.wp;
    let d_edge = d_wd(
        wp,
        [
            l.d, l.b, l.g, l.e, l.c, l.p2, l.h, l.f, l.p1, l.h5, l.i, l.f4, l.i5, l.i4,
        ],
    ) - d_wd(
        wp,
        [
            l.c, l.f4, l.b, l.f, l.i4, l.p0, l.e, l.i, l.p3, l.d, l.h, l.i5, l.g, l.h5,
        ],
    );
    let hv_edge = hv_wd(wp, [l.f, l.i, l.e, l.h, l.c, l.i5, l.b, l.h5])
        - hv_wd(wp, [l.e, l.f, l.h, l.i, l.d, l.f4, l.g, l.i4]);
    let edge_strength = smoothstep(pass.strength + 0.000_001, d_edge.abs());
    let (w1, w2) = (pass.weight1, pass.weight2);
    let two = [-w1, w1 + 0.5, w1 + 0.5, -w1];
    let four = [-w2, w2 + 0.25, w2 + 0.25, -w2];
    let w = window;
    let c1 = filter(two, [w.p2, w.h, w.f, w.p1]);
    let c2 = filter(two, [w.p0, w.e, w.i, w.p3]);
    let c3 = filter(
        four,
        [add(w.d, w.g), add(w.e, w.h), add(w.f, w.i), add(w.f4, w.i4)],
    );
    let c4 = filter(
        four,
        [add(w.c, w.b), add(w.f, w.e), add(w.i, w.h), add(w.i5, w.h5)],
    );
    let diagonal = if d_edge >= 0.0 { c2 } else { c1 };
    let axis = if hv_edge >= 0.0 { c4 } else { c3 };
    let mix = 1.0 - edge_strength;
    std::array::from_fn(|c| {
        let colour = diagonal[c] * (1.0 - mix) + axis[c] * mix;
        let near = [w.e[c], w.f[c], w.h[c], w.i[c]];
        let low = near.into_iter().fold(f32::INFINITY, f32::min);
        let high = near.into_iter().fold(f32::NEG_INFINITY, f32::max);
        colour.clamp(low, high)
    })
}

/// Pass 0: the texel at the centre of each square of four, the one at
/// (x, y) being between (x, y) and (x + 1, y + 1).
fn pass0(original: &Plane, threads: usize) -> Plane {
    Plane::from_fn(original.width, original.height, threads, |x, y| {
        kernel(&square(original, x as isize, y as isize), &PASS0)
    })
}

/// Pass 1: the doubled image. Of each 2x2 block, the top-left texel is
/// the original's, the bottom-right the diagonal one pass 0 computed, and
/// the other two are computed between those, on the grid turned 45°.
fn pass1(original: &Plane, diagonal: &Plane, threads: usize) -> Plane {
    Plane::from_fn(
        2 * original.width,
        2 * original.height,
        threads,
        |x2, y2| {
            let (x, y) = ((x2 / 2) as isize, (y2 / 2) as isize);
            match (x2 % 2, y2 % 2) {
                (0, 0) => original.at(x, y),
                (1, 1) => diagonal.at(x, y),
                (1, _) => kernel(
                    &orthogonal(|dx, dy| (x + dx, y + dy), original, diagonal),
                    &PASS1,
                ),
                _ => kernel(
                    &orthogonal(|dx, dy| (x + dy, y + dx), original, diagonal),
                    &PASS1,
                ),
            }
        },
    )
}

/// Pass 1's window for the texel right of original texel (x, y), its
/// offsets placed by `place`: as they are, or transposed for the texel
/// below it. Its taps alternate between the original (`o`) and the
/// diagonal texels (`d`).
fn orthogonal(
    place: impl Fn(isize, isize) -> (isize, isize),
    original: &Plane,
    diagonal: &Plane,
) -> Window<Texel> {
    let o = |dx, dy| {
        let (x, y) = place(dx, dy);
        original.at(x, y)
    };
    let d = |dx, dy| {
        let (x, y) = place(dx, dy);
        diagonal.at(x, y)
    };
    Window {
        p0: o(-1, 0),
        p1: d(0, -2),
        p2: d(0, 1),
        p3: o(2, 0),
        b: d(-1, -1),
        c: o(0, -1),
        d: d(-1, 0),
        e: o(0, 0),
        f: d(0, -1),
        g: o(0, 1),
        h: d(0, 0),
        i: o(1, 0),
        f4: o(1, -1),
        i4: d(1, -1),
        h5: o(1, 1),
        i5: d(1, 0),
    }
}

/// Pass 2: each texel of the doubled image refined from the four round
/// its top-left corner, which moves the image back by half a texel onto
/// the 2x grid.
fn pass2(doubled: &Plane, threads: usize) -> Plane {
    Plane::from_fn(doubled.width, doubled.height, threads, |x, y| {
        kernel(&square(doubled, x as isize - 1, y as isize - 1), &PASS2)
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::borrow::Cow;

    use nova_data::graphics::Image;
    use nova_view::ImageKind;

    use super::*;
    use crate::enlarge::Enlarger;

    /// A `w` x `h` image of `rgba` throughout.
    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Image {
        Image::from_rgba(w, h, rgba.repeat((w * h) as usize)).expect("w x h")
    }

    /// An image from rows of texels.
    fn image(rows: &[&[[u8; 4]]]) -> Image {
        let (w, h) = (rows[0].len() as u32, rows.len() as u32);
        Image::from_rgba(w, h, rows.concat().concat()).expect("rows")
    }

    /// An opaque plane of greys, from rows of levels 0 to 1.
    fn greys(rows: &[&[f32]]) -> Plane {
        let texels = rows.concat().iter().map(|&v| [v, v, v, 1.0]).collect();
        Plane::new(rows[0].len(), rows.len(), texels)
    }

    /// The grey level of `plane`'s texel at (`x`, `y`), its red.
    fn level(plane: &Plane, x: usize, y: usize) -> f32 {
        plane.texel(x, y)[0]
    }

    fn assert_near(got: f32, want: f32) {
        assert!((got - want).abs() < 1e-5, "{got} is not {want}");
    }

    /// The source texels within two of (`x`, `y`) in `image`, clamped to it:
    /// every texel an output texel at (2x or 2x + 1, 2y or 2y + 1) can
    /// draw on through the three passes.
    fn neighbourhood(image: &Image, x: u32, y: u32) -> Vec<[u8; 4]> {
        let near = |at: u32, size: u32| at.saturating_sub(2)..=(at + 2).min(size - 1);
        near(y, image.height())
            .flat_map(|sy| near(x, image.width()).map(move |sx| (sx, sy)))
            .map(|(sx, sy)| image.pixel(sx, sy).expect("inside"))
            .collect()
    }

    /// Every output texel of `enlarged` with the source neighbourhood in
    /// `source` it draws on.
    fn with_neighbourhoods<'a>(
        source: &'a Image,
        enlarged: &'a Image,
    ) -> impl Iterator<Item = ([u8; 4], Vec<[u8; 4]>)> + 'a {
        (0..enlarged.height())
            .flat_map(move |y| (0..enlarged.width()).map(move |x| (x, y)))
            .map(move |(x, y)| {
                let texel = enlarged.pixel(x, y).expect("inside");
                (texel, neighbourhood(source, x / 2, y / 2))
            })
    }

    /// 0 above-left of the anti-diagonal `x + y = n`, `1` on and below it.
    fn staircase(n: u32, side: u32, low: [u8; 4], high: [u8; 4]) -> Image {
        let pixels = (0..side)
            .flat_map(|y| (0..side).map(move |x| if x + y >= n { high } else { low }))
            .flatten()
            .collect();
        Image::from_rgba(side, side, pixels).expect("square")
    }

    const BLACK: [u8; 4] = [0, 0, 0, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    #[test]
    fn an_image_comes_back_twice_its_size_even_a_single_texel() {
        for (w, h) in [(1, 1), (3, 2), (2, 5), (0, 3), (4, 0)] {
            let enlarged = enlarge(&solid(w, h, [10, 20, 30, 255]));
            assert_eq!((enlarged.width(), enlarged.height()), (2 * w, 2 * h));
        }
        assert_eq!(
            enlarge(&solid(1, 1, [10, 20, 30, 40])),
            solid(2, 2, [10, 20, 30, 40]),
            "every neighbour clamped to the one texel"
        );
    }

    #[test]
    fn a_flat_image_stays_flat_in_its_colour_and_alpha() {
        for rgba in [
            [10, 20, 30, 255],
            [200, 100, 50, 128],
            [255, 255, 255, 1],
            CLEAR,
        ] {
            assert_eq!(enlarge(&solid(5, 3, rgba)), solid(10, 6, rgba), "{rgba:?}");
        }
    }

    #[test]
    fn every_output_channel_stays_within_its_source_neighbourhood() {
        // Opaque, so premultiplying changes nothing: the anti-ringing clamp
        // holds in the colours as stored.
        let pixels = (0..64_u32)
            .flat_map(|n| {
                let v = |k: u32| ((n * k + n * n * 7) % 256) as u8;
                [v(37), v(91), v(53), 255]
            })
            .collect();
        let source = Image::from_rgba(8, 8, pixels).expect("8x8");
        let enlarged = enlarge(&source);
        for (texel, near) in with_neighbourhoods(&source, &enlarged) {
            for c in 0..3 {
                let low = near.iter().map(|t| t[c]).min().expect("some");
                let high = near.iter().map(|t| t[c]).max().expect("some");
                assert!(
                    (low..=high).contains(&texel[c]),
                    "{texel:?} outside {near:?}"
                );
            }
        }
    }

    /// Pass 0 on a 4x4 staircase of greys at 0.25 (`k`), at (1, 1): the
    /// diagonal texel between E, F, H and I. Bilinear would give 0.0625;
    /// the edge pulls it toward the dark side.
    ///
    /// ```text
    /// E (1,1)=0, F (2,1)=0, H (1,2)=0, I (2,2)=k; P0 (0,0)=0, P1 (3,0)=0,
    /// P2 (0,3)=0, P3 (3,3)=k, B (1,0)=0, C (2,0)=0, D (0,1)=0, F4 (3,1)=k,
    /// G (0,2)=0, I4 (3,2)=k, H5 (1,3)=k, I5 (2,3)=k.
    /// Pass 0: wp = (2, 1, -1, 4, -1, 1); XBR_WEIGHT 0, so w1 = (0, .5, .5, 0)
    /// and w2 = (0, .25, .25, 0); edge strength 3.
    ///
    /// d_wd(d,b,g,e,c,p2,h,f,p1,h5,i,f4,i5,i4): e1 = h5, e2 = i, e3 = f4,
    ///   f2 = i5 and f3 = i4 are all k, the rest 0; every difference is 0,
    ///   so 0.
    /// d_wd(c,f4,b,f,i4,p0,e,i,p3,d,h,i5,g,h5): b1 = f4, c2 = i4, d2 = i,
    ///   d3 = p3, e3 = i5 and f3 = h5 are k, the rest 0:
    ///   2(k + 0 + 0 + k) + 1(0 + 0) - 1(k + k) + 4k - 1(k + k) + 1(k + k)
    ///   = 6k = 1.5.
    /// d_edge = 0 - 1.5 = -1.5 < 0: the diagonal colour is c1 = (H + F) / 2
    ///   = 0, and edge_strength = smoothstep(0, 3.000001, 1.5) = 0.5.
    /// c3 = c4 = (E + H + F + I) / 4 = k / 4 = 0.0625.
    /// colour = mix(0, 0.0625, 1 - 0.5) = 0.03125, inside [0, k].
    /// ```
    #[test]
    fn pass_0_blends_the_diagonal_texel_by_hyllians_formula() {
        let k = 0.25;
        let source = greys(&[
            &[0.0, 0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0, k],
            &[0.0, 0.0, k, k],
            &[0.0, k, k, k],
        ]);
        let diagonal = pass0(&source, 1);
        assert_eq!((diagonal.width, diagonal.height), (4, 4));
        assert_near(level(&diagonal, 1, 1), 0.03125);
    }

    /// Pass 1 at (3, 2), the texel right of original (1, 1), from a 3x3
    /// original O with its top row at a = 1 and the rest 0, and a 3x3
    /// diagonal plane Dg with its top row at b = 0.5 and the rest 0.
    ///
    /// ```text
    /// Taps (x = 1, y = 1, clamped to the planes): P0 = O(0,1) = 0,
    /// P1 = Dg(1,0) = b, P2 = Dg(1,2) = 0, P3 = O(2,1) = 0, B = Dg(0,0) = b,
    /// C = O(1,0) = a, D = Dg(0,1) = 0, E = O(1,1) = 0, F = Dg(1,0) = b,
    /// G = O(1,2) = 0, H = Dg(1,1) = 0, I = O(2,1) = 0, F4 = O(2,0) = a,
    /// I4 = Dg(2,0) = b, H5 = O(2,2) = 0, I5 = Dg(2,1) = 0.
    /// Pass 1 ("Details"): wp1 = 1 alone; w1 = (-w, w + .5, w + .5, -w) with
    /// w = 0.129633, w2 = (-v, v + .25, v + .25, -v) with v = 0.087534.
    ///
    /// d_edge = (|e-c| + |e-g| + |i-h5| + |i-f4|)
    ///        - (|f-i4| + |f-b| + |h-d| + |h-i5|) = (a + 0 + 0 + a) - 0 = 2 >= 0:
    ///   the diagonal colour is c2 = w1 . (P0, E, I, P3) = 0, and
    ///   edge_strength = smoothstep(0, 3.000001, 2) = 0.740741.
    /// hv_edge = (|f-c| + |i-i5| + |e-b| + |h-h5|)
    ///         - (|e-d| + |f-f4| + |h-g| + |i-i4|)
    ///         = (0.5 + 0 + 0.5 + 0) - (0 + 0.5 + 0 + 0.5) = 0 >= 0:
    ///   c4 = w2 . (C+B, F+E, I+H, I5+H5) = -v(1.5) + (v + .25)0.5
    ///      = 0.125 - v = 0.037466.
    /// colour = mix(0, 0.037466, 1 - 0.740741) = 0.009713, inside [0, b].
    /// ```
    #[test]
    fn pass_1_fills_an_orthogonal_texel_by_hyllians_formula() {
        let (a, b) = (1.0, 0.5);
        let original = greys(&[&[a, a, a], &[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0]]);
        let diagonal = greys(&[&[b, b, b], &[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0]]);
        let doubled = pass1(&original, &diagonal, 1);
        assert_eq!((doubled.width, doubled.height), (6, 6));
        assert_near(level(&doubled, 3, 2), 0.009_713);
        // The transposed taps, below original (1, 1), on the transposed
        // planes.
        let original_t = greys(&[&[a, 0.0, 0.0], &[a, 0.0, 0.0], &[a, 0.0, 0.0]]);
        let diagonal_t = greys(&[&[b, 0.0, 0.0], &[b, 0.0, 0.0], &[b, 0.0, 0.0]]);
        assert_near(level(&pass1(&original_t, &diagonal_t, 1), 2, 3), 0.009_713);
        // Even-even texels are the original's, odd-odd ones the diagonal's.
        assert_eq!(level(&doubled, 2, 0), a);
        assert_eq!(level(&doubled, 4, 2), 0.0);
        assert_eq!(level(&doubled, 1, 1), b);
        assert_eq!(level(&doubled, 5, 3), 0.0);
    }

    /// Pass 1 at (3, 2) again, on planes that vary across both axes, so
    /// the axis blend it picks (c3 or c4) shows in the colour.
    ///
    /// ```text
    /// O rows: (0, 0.6, 0.8), (0, 0.2, 0.8), (0, 0.2, 0.8); Dg rows, each
    /// (0.1, 0.5, 0.8). Taps: E = 0.2, I = 0.8, C = 0.6, G = 0.2, F4 = 0.8,
    /// H5 = 0.8, P0 = 0, P3 = 0.8; F = P1 = H = P2 = 0.5, B = D = 0.1,
    /// I4 = I5 = 0.8.
    ///
    /// d_edge = (0.4 + 0 + 0 + 0) - (0.3 + 0.4 + 0.4 + 0.3) = -1.0 < 0:
    ///   c1 = w1 . (P2, H, F, P1) = 0.5; edge_strength =
    ///   smoothstep(0, 3.000001, 1) = 0.259259.
    /// hv_edge = (|f-c| + |i-i5| + |e-b| + |h-h5|)
    ///         - (|e-d| + |f-f4| + |h-g| + |i-i4|)
    ///         = (0.1 + 0 + 0.1 + 0.3) - (0.1 + 0.3 + 0.3 + 0) = -0.2 < 0:
    ///   c3 = w2 . (D+G, E+H, F+I, F4+I4)
    ///      = -0.3v + 0.7(v + .25) + 1.3(v + .25) - 1.6v = 0.5 + 0.1v
    ///      = 0.508753 (c4 would be 0.5 - 0.3v = 0.473740).
    /// colour = mix(0.5, 0.508753, 1 - 0.259259) = 0.506484, inside
    ///   [0.2, 0.8]. With c4 it would be 0.480548.
    /// ```
    #[test]
    fn pass_1_blends_along_the_weaker_axis_by_hyllians_formula() {
        let original = greys(&[&[0.0, 0.6, 0.8], &[0.0, 0.2, 0.8], &[0.0, 0.2, 0.8]]);
        let diagonal = greys(&[&[0.1, 0.5, 0.8], &[0.1, 0.5, 0.8], &[0.1, 0.5, 0.8]]);
        let doubled = pass1(&original, &diagonal, 1);
        assert_near(level(&doubled, 3, 2), 0.506_484);
    }

    /// The shaders' `hv_wd` with pass 0's weights, on levels that are exact
    /// in binary.
    ///
    /// ```text
    /// i1 = 1, i2 = 0.5, i3 = 0.25, i4 = 0, e1 = 0.75, e2 = 0.125, e3 = 1,
    /// e4 = 0.5; wp = (2, 1, -1, 4, -1, 1).
    ///
    /// wp4 (|i1-i2| + |i3-i4|) = 4 (0.5 + 0.25) = 3
    /// wp1 (|i1-e1| + |i2-e2| + |i3-e3| + |i4-e4|)
    ///     = 2 (0.25 + 0.375 + 0.75 + 0.5) = 3.75
    /// wp3 (|i1-e2| + |i3-e4| + |e1-i2| + |e3-i4|)
    ///     = -1 (0.875 + 0.25 + 0.25 + 1) = -2.375
    /// hv_wd = 3 + 3.75 - 2.375 = 4.375
    /// ```
    #[test]
    fn hv_wd_is_hyllians_axis_edge_detector() {
        let levels = [1.0, 0.5, 0.25, 0.0, 0.75, 0.125, 1.0, 0.5];
        assert_eq!(hv_wd(PASS0.wp, levels), 4.375);
        // Pass 1's weights keep the second sum alone.
        assert_eq!(hv_wd(PASS1.wp, levels), 1.875);
    }

    /// Pass 2 at (1, 1) of a 3x3 plane, white but for its bottom-right 2x2:
    /// the point between E, F, H and I, round its top-left corner.
    /// Bilinear would give 0.75.
    ///
    /// ```text
    /// E (0,0) = 1, F (1,0) = 1, H (0,1) = 1, I (1,1) = 0; P0 = B = D =
    /// (0,0) = 1, P1 = F4 = (2,0) = 1, P2 = H5 = (0,2) = 1, G = (0,1) = 1,
    /// C = (1,0) = 1, I4 = (2,1) = 0, I5 = (1,2) = 0, P3 = (2,2) = 0.
    /// Pass 2 ("Details"): wp4 = 1 alone; w1 and w2 as pass 1's.
    ///
    /// d_edge = |h-f| - |e-i| = 0 - 1 = -1 < 0: the diagonal colour is
    ///   c1 = w1 . (P2, H, F, P1) = -w + 2(w + .5) - w = 1, and
    ///   edge_strength = smoothstep(0, 3.000001, 1) = 0.259259.
    /// hv_edge = (|f-i| + |e-h|) - (|e-f| + |h-i|) = 1 - 1 = 0 >= 0:
    ///   c4 = w2 . (C+B, F+E, I+H, I5+H5) = -2v + 2(v + .25) + (v + .25) - v
    ///      = 0.75.
    /// colour = mix(1, 0.75, 1 - 0.259259) = 0.814815.
    /// ```
    #[test]
    fn pass_2_refines_a_texel_by_hyllians_formula() {
        let plane = greys(&[&[1.0, 1.0, 1.0], &[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]]);
        let refined = pass2(&plane, 1);
        assert_eq!((refined.width, refined.height), (3, 3));
        assert_near(level(&refined, 1, 1), 0.814_815);
    }

    #[test]
    fn a_staircase_softens_along_its_edge_and_nowhere_else() {
        let source = staircase(8, 8, BLACK, WHITE);
        let enlarged = enlarge(&source);
        let mut softened = 0;
        for (texel, near) in with_neighbourhoods(&source, &enlarged) {
            if near.iter().all(|&t| t == near[0]) {
                assert_eq!(texel, near[0], "inside a flat area");
            } else if texel != BLACK && texel != WHITE {
                softened += 1;
            }
        }
        assert!(softened > 0, "the edge is smoothed");
    }

    /// A 6x6 sprite: a 2x2 opaque core of `rgba`'s colour, a ring at half
    /// alpha round it, and a transparent border.
    fn soft_sprite(rgb: [u8; 3]) -> Image {
        let [r, g, b] = rgb;
        let pixels = (0..6_u32)
            .flat_map(|y| (0..6_u32).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let ring = |at: u32| at.abs_diff(2) + at.abs_diff(3) - 1;
                match ring(x).max(ring(y)) {
                    0 => [r, g, b, 255],
                    1 => [r, g, b, 128],
                    _ => CLEAR,
                }
            })
            .collect();
        Image::from_rgba(6, 6, pixels).expect("6x6")
    }

    #[test]
    fn transparent_neighbourhoods_stay_transparent() {
        let mut source = soft_sprite([250, 200, 30]);
        // Pad it with transparency, so some neighbourhoods are all clear.
        let padded = (0..10_u32)
            .flat_map(|y| (0..10_u32).map(move |x| (x, y)))
            .flat_map(|(x, y)| match (x.checked_sub(2), y.checked_sub(2)) {
                (Some(sx), Some(sy)) if sx < 6 && sy < 6 => source.pixel(sx, sy).expect("inside"),
                _ => CLEAR,
            })
            .collect();
        source = Image::from_rgba(10, 10, padded).expect("10x10");
        let enlarged = enlarge(&source);
        let mut clear = 0;
        for (texel, near) in with_neighbourhoods(&source, &enlarged) {
            if near.iter().all(|&t| t == CLEAR) {
                assert_eq!(texel, CLEAR, "no halo");
                clear += 1;
            }
        }
        assert!(clear > 0);
    }

    /// `rgba` composited over black.
    fn over_black([r, g, b, a]: [u8; 4]) -> [f32; 3] {
        [r, g, b].map(|c| f32::from(c) * f32::from(a) / 255.0)
    }

    #[test]
    fn over_black_an_edge_is_the_enlargement_of_the_sprite_over_black() {
        let sprite = staircase(5, 6, CLEAR, [240, 120, 30, 255]);
        let composited = staircase(5, 6, BLACK, [240, 120, 30, 255]);
        let (alpha, colour) = (enlarge(&sprite), enlarge(&composited));
        for (a, c) in alpha.pixels().chunks(4).zip(colour.pixels().chunks(4)) {
            let a: [u8; 4] = a.try_into().expect("four");
            let mixed = over_black(a);
            for ch in 0..3 {
                let off = (mixed[ch] - f32::from(c[ch])).abs();
                assert!(off <= 1.0, "{a:?} over black is not {c:?}");
            }
        }
    }

    #[test]
    fn a_sprite_of_one_colour_keeps_its_colour_wherever_it_has_alpha() {
        let rgb = [100, 150, 200];
        let enlarged = enlarge(&soft_sprite(rgb));
        let mut faded = 0;
        for texel in enlarged.pixels().chunks(4) {
            match texel {
                [0, 0, 0, 0] => {}
                [r, g, b, a] => {
                    for (got, want) in [*r, *g, *b].iter().zip(rgb) {
                        assert!(got.abs_diff(want) <= 1, "{texel:?}: brighter or darker");
                    }
                    faded += usize::from(*a < 255);
                }
                _ => unreachable!("four bytes"),
            }
        }
        assert!(faded > 0, "the ring blends");
    }

    #[test]
    fn a_texel_left_with_no_alpha_has_no_colour() {
        // A barely-there texel by a clear one: some enlarged texels round
        // to alpha 0, and they must not keep a colour.
        let source = image(&[&[[255, 255, 255, 1], CLEAR], &[CLEAR, CLEAR]]);
        let original = Plane::premultiplied(&source);
        let planes = pass2(&pass1(&original, &pass0(&original, 1), 1), 1);
        assert!(
            planes
                .texels
                .iter()
                .any(|t| t[3] > 0.0 && t[3] < 0.5 / 255.0 && t[0] > 0.0),
            "some texel keeps a trace of colour and alpha that rounds to none"
        );
        let enlarged = enlarge(&source);
        for texel in enlarged.pixels().chunks(4) {
            if texel[3] == 0 {
                assert_eq!(texel, CLEAR, "{texel:?}");
            }
        }
    }

    /// A 6x6 sprite of many colours: opaque in the middle, half and a
    /// quarter alpha further out, clear in two corners.
    fn golden_source() -> Image {
        let pixels = (0..6_u32)
            .flat_map(|y| (0..6_u32).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let alpha = match x.abs_diff(2) + y.abs_diff(3) {
                    0..=2 => 255,
                    3 => 128,
                    4 => 64,
                    _ => 0,
                };
                let channel = |a: u32, b: u32| ((x * a + y * b + x * y * 13) % 256) as u8;
                [channel(40, 7), channel(11, 45), channel(30, 30), alpha]
            })
            .collect();
        Image::from_rgba(6, 6, pixels).expect("6x6")
    }

    /// The port's output for [`golden_source`], generated once from it
    /// after the hand-computed and alpha tests passed: it pins the whole
    /// port, every pass and branch, against regressions.
    #[rustfmt::skip]
    const GOLDEN: [u8; 576] = [
        0, 0, 0, 0, 0, 11, 5, 13, 17, 8, 12, 48, 55, 24, 46, 91,
        74, 24, 57, 128, 96, 36, 78, 128, 116, 45, 93, 92, 145, 44, 111, 50,
        183, 85, 153, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        16, 8, 14, 14, 32, 27, 34, 30, 34, 16, 29, 56, 58, 34, 53, 99,
        78, 35, 66, 127, 95, 43, 79, 130, 116, 49, 95, 96, 141, 53, 114, 59,
        194, 99, 170, 28, 228, 100, 192, 5, 214, 183, 210, 3, 0, 0, 0, 0,
        9, 30, 22, 51, 8, 35, 25, 55, 36, 36, 42, 82, 67, 62, 74, 139,
        96, 72, 95, 199, 113, 80, 110, 200, 131, 83, 122, 139, 157, 87, 138, 89,
        208, 99, 186, 59, 255, 117, 230, 29, 255, 46, 230, 6, 0, 0, 0, 0,
        8, 47, 32, 80, 17, 50, 39, 88, 47, 68, 66, 130, 78, 85, 92, 206,
        102, 98, 113, 255, 120, 106, 128, 232, 156, 129, 160, 171, 180, 154, 178, 130,
        172, 159, 161, 96, 147, 156, 132, 59, 151, 35, 137, 24, 124, 3, 107, 10,
        11, 63, 41, 99, 18, 82, 57, 126, 54, 102, 88, 200, 89, 118, 116, 252,
        116, 133, 140, 255, 156, 160, 176, 237, 213, 200, 229, 223, 198, 213, 208, 199,
        82, 213, 85, 136, 31, 203, 40, 90, 86, 62, 83, 59, 88, 0, 73, 45,
        26, 108, 77, 186, 37, 113, 86, 212, 63, 130, 109, 248, 102, 152, 141, 255,
        146, 182, 178, 255, 202, 225, 228, 255, 205, 202, 228, 255, 135, 156, 156, 236,
        78, 189, 93, 183, 41, 176, 53, 139, 88, 90, 100, 96, 104, 46, 111, 84,
        26, 135, 90, 255, 30, 128, 91, 249, 70, 152, 124, 255, 115, 190, 174, 255,
        171, 238, 228, 255, 196, 225, 225, 255, 106, 133, 140, 255, 34, 68, 70, 246,
        52, 78, 81, 244, 90, 120, 109, 199, 120, 113, 144, 151, 147, 112, 174, 123,
        30, 146, 97, 236, 37, 146, 103, 222, 82, 190, 155, 239, 129, 240, 221, 255,
        160, 233, 220, 255, 172, 138, 132, 255, 51, 37, 42, 255, 2, 38, 47, 254,
        59, 66, 96, 255, 104, 108, 134, 199, 150, 141, 146, 128, 182, 151, 158, 106,
        38, 182, 121, 151, 47, 181, 130, 166, 94, 241, 199, 226, 145, 243, 201, 255,
        204, 157, 131, 255, 179, 45, 32, 255, 78, 49, 44, 255, 55, 98, 101, 247,
        94, 136, 143, 196, 145, 149, 167, 136, 187, 182, 108, 93, 206, 205, 64, 84,
        41, 179, 141, 114, 54, 196, 157, 129, 110, 225, 197, 197, 156, 185, 159, 238,
        219, 94, 71, 252, 181, 51, 34, 247, 105, 105, 95, 239, 75, 142, 138, 203,
        113, 169, 168, 137, 177, 193, 161, 88, 238, 232, 59, 59, 255, 255, 0, 45,
        51, 206, 159, 81, 73, 162, 187, 95, 117, 109, 220, 127, 175, 90, 167, 175,
        219, 93, 93, 230, 208, 116, 88, 233, 115, 156, 137, 189, 88, 173, 171, 137,
        140, 104, 111, 98, 187, 64, 50, 59, 233, 101, 20, 26, 255, 248, 0, 10,
        43, 190, 156, 64, 72, 130, 205, 71, 123, 58, 243, 100, 179, 52, 197, 156,
        245, 107, 95, 243, 233, 124, 87, 248, 136, 180, 144, 159, 114, 194, 184, 105,
        159, 64, 63, 80, 240, 0, 0, 46, 250, 0, 1, 10, 0, 0, 0, 0,
    ];

    #[test]
    fn a_golden_sprite_enlarges_to_the_bytes_it_always_has() {
        let enlarged = enlarge(&golden_source());
        assert_eq!((enlarged.width(), enlarged.height()), (12, 12));
        assert_eq!(enlarged.pixels(), GOLDEN);
    }

    #[test]
    fn any_number_of_threads_enlarges_to_the_same_bytes() {
        // Tall and wide enough for several bands of rows in every pass.
        let (w, h) = (160_u32, 150_u32);
        let pixels = (0..w * h)
            .flat_map(|n| {
                let v = |k: u32| ((n * k + n / 7 * 13) % 256) as u8;
                [v(37), v(91), v(53), v(29)]
            })
            .collect();
        let source = Image::from_rgba(w, h, pixels).expect("w x h");
        let alone = enlarge_with(&source, 1);
        for threads in [2, 3, 7, 64] {
            assert!(enlarge_with(&source, threads) == alone, "{threads} threads");
        }
        assert!(enlarge(&source) == alone, "the machine's threads");
        assert_eq!(enlarge_with(&golden_source(), 5).pixels(), GOLDEN);
    }

    /// Six 24x20 frames, each a different shade of a soft-edged blob.
    fn sheet() -> Vec<Image> {
        (0..6_u32)
            .map(|n| {
                let pixels = (0..20_u32)
                    .flat_map(|y| (0..24_u32).map(move |x| (x, y)))
                    .flat_map(|(x, y)| {
                        let alpha =
                            (255_u32.saturating_sub((x.abs_diff(12) + y.abs_diff(10)) * 20)) as u8;
                        [(n * 40) as u8, (x * 10) as u8, (y * 12) as u8, alpha]
                    })
                    .collect();
                Image::from_rgba(24, 20, pixels).expect("24x20")
            })
            .collect()
    }

    #[test]
    fn a_sheets_frames_spread_over_threads_enlarge_as_each_alone() {
        let frames = sheet();
        let alone: Vec<Image> = frames.iter().map(|f| enlarge_with(f, 1)).collect();
        for threads in [1, 2, 4, 5, 64] {
            assert!(enlarge_all(&frames, threads) == alone, "{threads} threads");
        }
        assert_eq!(enlarge_all(&[], 4), Vec::<Image>::new());
        let turned = SuperXbr.enlarge_frames(ImageKind::Rled, &frames);
        assert_eq!(turned.len(), frames.len());
        assert!(turned.iter().zip(&alone).all(|(got, want)| **got == *want));
        let patterns = SuperXbr.enlarge_frames(ImageKind::Pattern, &frames);
        assert!(
            patterns.iter().zip(&frames).all(
                |(got, frame)| matches!(got, Cow::Borrowed(same) if std::ptr::eq(*same, frame))
            ),
            "patterns borrowed"
        );
    }

    #[test]
    fn patterns_are_left_alone_and_every_other_kind_is_enlarged() {
        let frame = staircase(3, 4, BLACK, WHITE);
        match SuperXbr.enlarge(ImageKind::Pattern, &frame) {
            Cow::Borrowed(same) => assert_eq!(std::ptr::from_ref(same), std::ptr::from_ref(&frame)),
            Cow::Owned(other) => panic!("a pattern was enlarged: {other:?}"),
        }
        for kind in [
            ImageKind::Rled,
            ImageKind::Pict,
            ImageKind::MaskedPict { mask: 3 },
            ImageKind::Icon,
            ImageKind::SpinSheet,
        ] {
            assert_eq!(
                SuperXbr.enlarge(kind, &frame).into_owned(),
                enlarge(&frame),
                "{kind:?}"
            );
        }
    }
}
