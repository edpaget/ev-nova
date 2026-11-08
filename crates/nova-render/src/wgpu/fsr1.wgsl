// FidelityFX Super Resolution Sample
//
// Copyright (c) 2021 Advanced Micro Devices, Inc. All rights reserved.
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and / or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// ---------------------------------------------------------------------------
//
// AMD FidelityFX Super Resolution 1 (EASU then RCAS), ported to WGSL from
// `ffx-fsr/ffx_fsr1.h`, "AMD FidelityFX SUPER RESOLUTION [FSR 1] ::: SPATIAL
// SCALING & EXTRAS - v1.20210629", in GPUOpen-Effects/FidelityFX-FSR at
// commit a21ffb8f6c13233ba336352bdff293894c706575, with the libretro
// slang-shaders `edge-smoothing/fsr` preset as the reference for the two
// passes' structure.
//
// This file is modified from AMD's source:
// - ported to WGSL, the non-packed 32-bit paths (`FsrEasuF`, `FsrRcasF`)
//   alone: no FP16 paths;
// - EASU's 12 taps and RCAS's 5 are read with `textureLoad` at whole-texel
//   offsets, clamped to the texture's edge, in place of `textureGather`;
// - AMD's approximate reciprocal and reciprocal square root helpers
//   (`APrxLoRcpF1`, `APrxMedRcpF1`, `APrxLoRsqF1`) are exact `1.0 / x` and
//   `inverseSqrt`. Where AMD's approximation of a reciprocal of zero is a
//   large finite number that multiplies a zero, the exact port takes the
//   zero product it gives (EASU's edge length) or leaves the term out of the
//   `max` it feeds (RCAS's limiters), as IEEE `max` would drop a NaN;
// - no RCAS noise removal (`FSR_RCAS_DENOISE`), no pass-through alpha and no
//   film grain: alpha is written opaque.

struct Params {
    // FsrEasuCon's con0: output pixel to input position scale (xy) and
    // offset (zw).
    con0: vec4<f32>,
    // The top-left corner of the rectangle RCAS fills, in target pixels.
    origin: vec2<f32>,
    // FsrRcasCon's con.x: 2^-stops.
    sharpness: f32,
    padding: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
// EASU reads the world texture, RCAS EASU's output.
@group(0) @binding(1) var source: texture_2d<f32>;

// One triangle covering the whole viewport: clip (-1, -1), (3, -1), (-1, 3).
@vertex
fn fsr_vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

// The source's texel at `at`, clamped to its edge.
fn load(at: vec2<i32>) -> vec3<f32> {
    let last = vec2<i32>(textureDimensions(source)) - vec2<i32>(1, 1);
    return textureLoad(source, clamp(at, vec2<i32>(0, 0), last), 0).rgb;
}

// The simplest multi-channel approximate luma: luma times 2.
fn luma2(c: vec3<f32>) -> f32 {
    return c.b * 0.5 + (c.r * 0.5 + c.g);
}

// FsrEasuSetF: one bilinear corner's direction and length, weighted by `w`,
// from the '+' of lumas about it,
//    a
//  b c d
//    e
// as (direction x, direction y, length).
fn easu_set(w: f32, la: f32, lb: f32, lc: f32, ld: f32, le: f32) -> vec3<f32> {
    let dc = ld - lc;
    let cb = lc - lb;
    let most_x = max(abs(dc), abs(cb));
    let dir_x = ld - lb;
    var len_x = select(0.0, saturate(abs(dir_x) / most_x), most_x > 0.0);
    len_x = len_x * len_x;
    let ec = le - lc;
    let ca = lc - la;
    let most_y = max(abs(ec), abs(ca));
    let dir_y = le - la;
    var len_y = select(0.0, saturate(abs(dir_y) / most_y), most_y > 0.0);
    len_y = len_y * len_y;
    return vec3<f32>(dir_x, dir_y, len_x + len_y) * w;
}

// FsrEasuTapF: a tap of colour `c` at `off` from the resolve position,
// as (colour times weight, weight).
fn easu_tap(off: vec2<f32>, dir: vec2<f32>, len: vec2<f32>, lob: f32, clp: f32, c: vec3<f32>) -> vec4<f32> {
    // Rotate the offset by the direction, then the anisotropy.
    var v = vec2<f32>(off.x * dir.x + off.y * dir.y, off.x * (-dir.y) + off.y * dir.x);
    v = v * len;
    let d2 = min(v.x * v.x + v.y * v.y, clp);
    // Lanczos 2 approximated: (25/16 (2/5 x^2 - 1)^2 - (25/16 - 1)) (lob x^2 - 1)^2.
    var w_b = (2.0 / 5.0) * d2 - 1.0;
    var w_a = lob * d2 - 1.0;
    w_b = w_b * w_b;
    w_a = w_a * w_a;
    w_b = (25.0 / 16.0) * w_b - (25.0 / 16.0 - 1.0);
    let w = w_b * w_a;
    return vec4<f32>(c * w, w);
}

// EASU (FsrEasuF): the world texture upsampled into a texture the
// viewport's size.
@fragment
fn easu_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let ip = floor(position.xy);
    var pp = ip * params.con0.xy + params.con0.zw;
    let fp = floor(pp);
    pp = pp - fp;
    let f0 = vec2<i32>(fp);
    // The 12-tap kernel about 'f':
    //    b c
    //  e f g h
    //  i j k l
    //    n o
    let b = load(f0 + vec2<i32>(0, -1));
    let c = load(f0 + vec2<i32>(1, -1));
    let e = load(f0 + vec2<i32>(-1, 0));
    let f = load(f0);
    let g = load(f0 + vec2<i32>(1, 0));
    let h = load(f0 + vec2<i32>(2, 0));
    let i = load(f0 + vec2<i32>(-1, 1));
    let j = load(f0 + vec2<i32>(0, 1));
    let k = load(f0 + vec2<i32>(1, 1));
    let l = load(f0 + vec2<i32>(2, 1));
    let n = load(f0 + vec2<i32>(0, 2));
    let o = load(f0 + vec2<i32>(1, 2));
    let b_l = luma2(b);
    let c_l = luma2(c);
    let e_l = luma2(e);
    let f_l = luma2(f);
    let g_l = luma2(g);
    let h_l = luma2(h);
    let i_l = luma2(i);
    let j_l = luma2(j);
    let k_l = luma2(k);
    let l_l = luma2(l);
    let n_l = luma2(n);
    let o_l = luma2(o);
    // Accumulate for bilinear interpolation, the corners
    //  s t
    //  u v
    let corners = easu_set((1.0 - pp.x) * (1.0 - pp.y), b_l, e_l, f_l, g_l, j_l)
        + easu_set(pp.x * (1.0 - pp.y), c_l, f_l, g_l, h_l, k_l)
        + easu_set((1.0 - pp.x) * pp.y, f_l, i_l, j_l, k_l, n_l)
        + easu_set(pp.x * pp.y, g_l, j_l, k_l, l_l, o_l);
    var dir = corners.xy;
    var len = corners.z;
    // Normalise, and clean up close to zero.
    let dir2 = dir * dir;
    var dir_r = dir2.x + dir2.y;
    let zro = dir_r < 1.0 / 32768.0;
    dir_r = select(inverseSqrt(dir_r), 1.0, zro);
    dir.x = select(dir.x, 1.0, zro);
    dir = dir * dir_r;
    // From {0 to 2} to {0 to 1}, shaped with a square.
    len = len * 0.5;
    len = len * len;
    // Stretch the kernel {1.0 vertical or horizontal, to sqrt(2.0) on the diagonal}.
    let stretch = (dir.x * dir.x + dir.y * dir.y) / max(abs(dir.x), abs(dir.y));
    // The anisotropic length after rotation: x from 1 to 'stretch' on
    // edges, y from 1 to 0.5.
    let len2 = vec2<f32>(1.0 + (stretch - 1.0) * len, 1.0 - 0.5 * len);
    // The window shifts from +/-{sqrt(2.0) to slightly beyond 2.0} with the edge.
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * len;
    // The distance^2 clipping point, at the end of the adjustable window.
    let clp = 1.0 / lob;
    // The min and max of the 4 nearest, for deringing.
    let min4 = min(min(f, g), min(j, k));
    let max4 = max(max(f, g), max(j, k));
    let a = easu_tap(vec2<f32>(0.0, -1.0) - pp, dir, len2, lob, clp, b)
        + easu_tap(vec2<f32>(1.0, -1.0) - pp, dir, len2, lob, clp, c)
        + easu_tap(vec2<f32>(-1.0, 1.0) - pp, dir, len2, lob, clp, i)
        + easu_tap(vec2<f32>(0.0, 1.0) - pp, dir, len2, lob, clp, j)
        + easu_tap(vec2<f32>(0.0, 0.0) - pp, dir, len2, lob, clp, f)
        + easu_tap(vec2<f32>(-1.0, 0.0) - pp, dir, len2, lob, clp, e)
        + easu_tap(vec2<f32>(1.0, 1.0) - pp, dir, len2, lob, clp, k)
        + easu_tap(vec2<f32>(2.0, 1.0) - pp, dir, len2, lob, clp, l)
        + easu_tap(vec2<f32>(2.0, 0.0) - pp, dir, len2, lob, clp, h)
        + easu_tap(vec2<f32>(1.0, 0.0) - pp, dir, len2, lob, clp, g)
        + easu_tap(vec2<f32>(1.0, 2.0) - pp, dir, len2, lob, clp, o)
        + easu_tap(vec2<f32>(0.0, 2.0) - pp, dir, len2, lob, clp, n);
    // Normalise and dering.
    let pix = min(max4, max(min4, a.rgb / a.w));
    return vec4<f32>(pix, 1.0);
}

// The limit of RCAS's lobe before results look unnatural.
const RCAS_LIMIT: f32 = 0.25 - 1.0 / 16.0;

// RCAS (FsrRcasF): EASU's output sharpened onto the viewport, from the
// cross about each pixel,
//    b
//  d e f
//    h
@fragment
fn rcas_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let sp = vec2<i32>(floor(position.xy - params.origin));
    let b = load(sp + vec2<i32>(0, -1));
    let d = load(sp + vec2<i32>(-1, 0));
    let e = load(sp);
    let f = load(sp + vec2<i32>(1, 0));
    let h = load(sp + vec2<i32>(0, 1));
    // The min and max of the ring.
    let mn4 = min(min(b, d), min(f, h));
    let mx4 = max(max(b, d), max(f, h));
    // The limiters, each channel: how negative the lobe may go before the
    // result would clip below 0 (`hit_min`) or above 1 (`hit_max`). A
    // limiter whose denominator is zero (a black ring, a white one)
    // can't be hit; -1 is below the lobe's own limit, so it leaves the
    // `max` to the other.
    let peak_c = vec2<f32>(1.0, -1.0 * 4.0);
    let hit_min = min(mn4, e) / (4.0 * mx4);
    let hit_max = (peak_c.x - max(mx4, e)) / (4.0 * mn4 + peak_c.y);
    let unbound = vec3<f32>(-1.0, -1.0, -1.0);
    let from_min = select(-hit_min, unbound, mx4 <= vec3<f32>(0.0, 0.0, 0.0));
    let from_max = select(hit_max, unbound, mn4 >= vec3<f32>(1.0, 1.0, 1.0));
    let lobe_c = max(from_min, from_max);
    let lobe = max(-RCAS_LIMIT, min(max(lobe_c.r, max(lobe_c.g, lobe_c.b)), 0.0)) * params.sharpness;
    // Resolve.
    let rcp_l = 1.0 / (4.0 * lobe + 1.0);
    let pix = (lobe * b + lobe * d + lobe * h + lobe * f + e) * rcp_l;
    return vec4<f32>(pix, 1.0);
}
