//! FSR 1 on the CPU, the reference the `fsr1` tests compare the GPU's
//! frame with: AMD's `FsrEasuF` and `FsrRcasF` (`ffx_fsr1.h`, `FidelityFX`
//! FSR 1.0, MIT, Copyright (c) 2021 Advanced Micro Devices, Inc.; the
//! full notice is at the top of `nova-render`'s `fsr1.wgsl`), ported from
//! the C source in `f32` with the shader's modifications: exact
//! reciprocals and square roots, texels clamped to the edge, a zero edge
//! length where AMD multiplies a zero by its approximate reciprocal of
//! zero, and a limiter with a zero denominator left out of RCAS's `max`.
//!
//! Images are RGBA8, row by row, as the offscreen GPU reads them back.
//! The taps keep AMD's one-letter names (`b`, `c`, `e` to `l`, `n`, `o`),
//! so the port reads against the source line by line.

#![allow(clippy::many_single_char_names)]

/// An RGBA8 image and its size.
pub(crate) struct Rgba<'a> {
    pub(crate) pixels: &'a [u8],
    pub(crate) size: (u32, u32),
}

impl Rgba<'_> {
    /// The texel at (`x`, `y`) clamped to the edge, each channel 0 to 1.
    fn at(&self, x: i64, y: i64) -> [f32; 3] {
        let x = x.clamp(0, i64::from(self.size.0) - 1) as usize;
        let y = y.clamp(0, i64::from(self.size.1) - 1) as usize;
        let i = (y * self.size.0 as usize + x) * 4;
        [0, 1, 2].map(|c| f32::from(self.pixels[i + c]) / 255.0)
    }
}

/// Luma times 2: `B x 0.5 + (R x 0.5 + G)`.
fn luma2([r, g, b]: [f32; 3]) -> f32 {
    b * 0.5 + (r * 0.5 + g)
}

/// The 4 texels nearest the sample point of EASU's output pixel
/// (`ix`, `iy`) for an `input`-sized world drawn onto `output`, as 'f',
/// 'g', 'j' and 'k' (`FsrEasuF`'s names), and `pp`, the sample point's
/// place between them.
pub(crate) fn nearest_four(
    world: &Rgba<'_>,
    output: (u32, u32),
    (ix, iy): (u32, u32),
) -> ([[f32; 3]; 4], (f32, f32)) {
    let (fx, fy, ppx, ppy) = sample_point(world.size, output, (ix, iy));
    (
        [
            world.at(fx, fy),
            world.at(fx + 1, fy),
            world.at(fx, fy + 1),
            world.at(fx + 1, fy + 1),
        ],
        (ppx, ppy),
    )
}

/// `FsrEasuF`'s 'f' texel and `pp` for output pixel `ip`:
/// `ip x in / out + (0.5 x in / out - 0.5)`, split into its floor and
/// fraction.
fn sample_point(
    input: (u32, u32),
    output: (u32, u32),
    (ix, iy): (u32, u32),
) -> (i64, i64, f32, f32) {
    let con = |i: u32, o: u32| {
        let s = i as f32 / o as f32;
        (s, 0.5 * s - 0.5)
    };
    let ((sx, ox), (sy, oy)) = (con(input.0, output.0), con(input.1, output.1));
    let (px, py) = (ix as f32 * sx + ox, iy as f32 * sy + oy);
    let (fx, fy) = (px.floor(), py.floor());
    (fx as i64, fy as i64, px - fx, py - fy)
}

/// `FsrEasuSetF`: one bilinear corner's direction and length, weighted
/// by `w`, added to `dir` and `len`.
fn easu_set(dir: &mut (f32, f32), len: &mut f32, w: f32, [a, b, c, d, e]: [f32; 5]) {
    let rcp_or_zero = |most: f32, x: f32| {
        if most > 0.0 {
            (x.abs() / most).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    let dir_x = d - b;
    let len_x = rcp_or_zero((d - c).abs().max((c - b).abs()), dir_x);
    dir.0 += dir_x * w;
    *len += len_x * len_x * w;
    let dir_y = e - a;
    let len_y = rcp_or_zero((e - c).abs().max((c - a).abs()), dir_y);
    dir.1 += dir_y * w;
    *len += len_y * len_y * w;
}

/// EASU's output pixel (`ix`, `iy`), each channel 0 to 1, before RCAS:
/// `FsrEasuF`.
pub(crate) fn easu_pixel(world: &Rgba<'_>, output: (u32, u32), (ix, iy): (u32, u32)) -> [f32; 3] {
    let (fx, fy, ppx, ppy) = sample_point(world.size, output, (ix, iy));
    let t = |dx: i64, dy: i64| world.at(fx + dx, fy + dy);
    let (b, c) = (t(0, -1), t(1, -1));
    let (e, f, g, h) = (t(-1, 0), t(0, 0), t(1, 0), t(2, 0));
    let (i, j, k, l) = (t(-1, 1), t(0, 1), t(1, 1), t(2, 1));
    let (n, o) = (t(0, 2), t(1, 2));
    let [bl, cl, el, fl, gl, hl, il, jl, kl, ll, nl, ol] =
        [b, c, e, f, g, h, i, j, k, l, n, o].map(luma2);
    let mut dir = (0.0, 0.0);
    let mut len = 0.0;
    easu_set(
        &mut dir,
        &mut len,
        (1.0 - ppx) * (1.0 - ppy),
        [bl, el, fl, gl, jl],
    );
    easu_set(&mut dir, &mut len, ppx * (1.0 - ppy), [cl, fl, gl, hl, kl]);
    easu_set(&mut dir, &mut len, (1.0 - ppx) * ppy, [fl, il, jl, kl, nl]);
    easu_set(&mut dir, &mut len, ppx * ppy, [gl, jl, kl, ll, ol]);
    let dir_r = dir.0 * dir.0 + dir.1 * dir.1;
    let (dir_r, dir_x) = if dir_r < 1.0 / 32768.0 {
        (1.0, 1.0)
    } else {
        (1.0 / dir_r.sqrt(), dir.0)
    };
    let dir = (dir_x * dir_r, dir.1 * dir_r);
    let len = (len * 0.5) * (len * 0.5);
    let stretch = (dir.0 * dir.0 + dir.1 * dir.1) / dir.0.abs().max(dir.1.abs());
    let len2 = (1.0 + (stretch - 1.0) * len, 1.0 - 0.5 * len);
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * len;
    let clp = 1.0 / lob;
    let mut colour = [0.0_f32; 3];
    let mut weight = 0.0_f32;
    let taps = [
        ((0.0, -1.0), b),
        ((1.0, -1.0), c),
        ((-1.0, 1.0), i),
        ((0.0, 1.0), j),
        ((0.0, 0.0), f),
        ((-1.0, 0.0), e),
        ((1.0, 1.0), k),
        ((2.0, 1.0), l),
        ((2.0, 0.0), h),
        ((1.0, 0.0), g),
        ((1.0, 2.0), o),
        ((0.0, 2.0), n),
    ];
    for ((ox, oy), tap) in taps {
        let (ox, oy) = (ox - ppx, oy - ppy);
        let vx = (ox * dir.0 + oy * dir.1) * len2.0;
        let vy = (ox * -dir.1 + oy * dir.0) * len2.1;
        let d2 = (vx * vx + vy * vy).min(clp);
        let w_b = (2.0 / 5.0) * d2 - 1.0;
        let w_a = lob * d2 - 1.0;
        let w = ((25.0 / 16.0) * w_b * w_b - (25.0 / 16.0 - 1.0)) * (w_a * w_a);
        for ch in 0..3 {
            colour[ch] += tap[ch] * w;
        }
        weight += w;
    }
    [0, 1, 2].map(|ch| {
        let lo = f[ch].min(g[ch]).min(j[ch].min(k[ch]));
        let hi = f[ch].max(g[ch]).max(j[ch].max(k[ch]));
        (colour[ch] / weight).max(lo).min(hi)
    })
}

/// A channel 0 to 1 stored as an 8-bit unorm, as a render target stores it.
pub(crate) fn unorm8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// `world` upsampled by EASU to `output`, stored as RGBA8 (opaque), as
/// the shader's intermediate texture holds it.
pub(crate) fn easu(world: &Rgba<'_>, output: (u32, u32)) -> Vec<u8> {
    (0..output.1)
        .flat_map(|y| (0..output.0).map(move |x| (x, y)))
        .flat_map(|xy| {
            let [r, g, b] = easu_pixel(world, output, xy).map(unorm8);
            [r, g, b, 255]
        })
        .collect()
}

/// `FsrRcasCon`'s sharpness at AMD's default 0.2 stops.
fn sharpness() -> f32 {
    (-0.2_f32).exp2()
}

/// `upsampled` sharpened by RCAS (`FsrRcasF`, no noise removal), stored
/// as RGBA8 (opaque).
pub(crate) fn rcas(upsampled: &Rgba<'_>) -> Vec<u8> {
    const LIMIT: f32 = 0.25 - 1.0 / 16.0;
    let (w, h) = upsampled.size;
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..i64::from(h) {
        for x in 0..i64::from(w) {
            let b = upsampled.at(x, y - 1);
            let d = upsampled.at(x - 1, y);
            let e = upsampled.at(x, y);
            let f = upsampled.at(x + 1, y);
            let hh = upsampled.at(x, y + 1);
            let mut lobe = f32::NEG_INFINITY;
            for ch in 0..3 {
                let mn4 = b[ch].min(d[ch]).min(f[ch]).min(hh[ch]);
                let mx4 = b[ch].max(d[ch]).max(f[ch]).max(hh[ch]);
                // A limiter with a zero denominator can't be hit.
                let from_min = if mx4 > 0.0 {
                    -(mn4.min(e[ch]) / (4.0 * mx4))
                } else {
                    f32::NEG_INFINITY
                };
                let from_max = if mn4 < 1.0 {
                    (1.0 - mx4.max(e[ch])) / (4.0 * mn4 - 4.0)
                } else {
                    f32::NEG_INFINITY
                };
                lobe = lobe.max(from_min.max(from_max));
            }
            let lobe = lobe.clamp(-LIMIT, 0.0) * sharpness();
            let rcp_l = 1.0 / (4.0 * lobe + 1.0);
            for ch in 0..3 {
                let pix =
                    (lobe * b[ch] + lobe * d[ch] + lobe * hh[ch] + lobe * f[ch] + e[ch]) * rcp_l;
                out.push(unorm8(pix));
            }
            out.push(255);
        }
    }
    out
}

/// `inner` (`onto`'s size, RGBA8) placed at `onto` in a `target`-sized
/// frame whose bars are opaque black.
pub(crate) fn placed(inner: &[u8], target: (u32, u32), onto: nova_render::PixelRect) -> Vec<u8> {
    let mut out = Vec::with_capacity((target.0 * target.1 * 4) as usize);
    for y in 0..target.1 {
        for x in 0..target.0 {
            let inside =
                (onto.x..onto.x + onto.w).contains(&x) && (onto.y..onto.y + onto.h).contains(&y);
            if inside {
                let i = (((y - onto.y) * onto.w + (x - onto.x)) * 4) as usize;
                out.extend_from_slice(&inner[i..i + 4]);
            } else {
                out.extend([0, 0, 0, 255]);
            }
        }
    }
    out
}
