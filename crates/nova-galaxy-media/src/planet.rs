//! A generated planet's look (`nova_galaxy_format::model::Look`) baked
//! into the textures a planet shader samples: an equirectangular surface
//! with its glow, a cloud layer, a ring profile, and the planet's spin and
//! tilt. A texture asset makes the same [`Surface`]
//! ([`Surface::from_texture`]), so one shader draws both.
//!
//! # Same seed, same bytes, everywhere
//!
//! Every value comes from the look, and every random one from its `seed`
//! through an integer hash (`SplitMix64`'s mixer), split into a stream per
//! use ([`Stream`]) so that changing one recipe doesn't shift another's
//! values. The bake uses only integer hashing and IEEE `+ − × ÷`, `sqrt`
//! and `floor`, never `sin`, `cos`, `exp`, `powf` or `ln`, whose last bits
//! differ between platforms' maths libraries. The noise is periodic in
//! longitude by wrapping its lattice, not by trigonometry. So a look bakes
//! to the same bytes on every platform; a digest test pins it.
//!
//! # The recipes
//!
//! The surface is a height field of fractal value noise, its texels
//! ranked so that each palette colour covers an exact share: `palette[0]`
//! the most, each later colour less ("most to least of the surface").
//! With fewer colours than a recipe uses, the rest are lighter and darker
//! shades of those given ([`stops`]), so a one-colour planet still has
//! relief. Every texel blends those colours, white (ice caps) and nothing
//! else.
//!
//! - rocky: a rough, high-contrast ramp, with seeded crater pits;
//! - desert: a smooth, low-contrast ramp, stretched east–west;
//! - ocean: about 70% water (`palette[0]`), then a land ramp;
//! - garden: about 50% water, a land ramp, white polar caps;
//! - ice: a ramp under large white polar caps;
//! - lava: a dark crust ramp, the last colour glowing in the low cracks.
//!   The surface's alpha is that glow: it shows on the night side. No
//!   other type glows;
//! - gas giant: latitude bands of seeded widths, turned by turbulence
//!   along the bands; no height field.
//!
//! `clouds` is the share of the globe a white, soft-edged cloud layer
//! covers ([`CLOUD_OPACITY`]); on a gas giant, which is all cloud, it is
//! the bands' turbulence instead, and no layer is baked. Rings take a
//! seeded radial profile of bands and gaps, at most [`RING_PEAK`] opaque.
//! The day length and the axial tilt come from the seed too ([`days`],
//! [`TILT`], [`GAS_GIANT_TILT`]).

use nova_galaxy_format::model::{Look, PlanetType};

use crate::texture::Texture;

/// How opaque the cloud layer is where it covers the globe.
pub const CLOUD_OPACITY: f64 = 0.85;

/// The most opaque a ring gets.
pub const RING_PEAK: f64 = 0.8;

/// The most a planet's axis rolls either way, in radians (25°).
pub const TILT: f64 = 0.436_332_312_998_582_4;

/// The most a gas giant's axis rolls either way, in radians (10°).
pub const GAS_GIANT_TILT: f64 = 0.174_532_925_199_432_95;

/// The day length of a world with a surface, in seconds of game time:
/// the shortest and the longest.
pub const SOLID_DAYS: (f64, f64) = (240.0, 480.0);

/// A gas giant's day length, in seconds of game time: they turn faster.
pub const GAS_GIANT_DAYS: (f64, f64) = (120.0, 240.0);

/// A texture planet's day: the middle of a solid world's.
pub const TEXTURE_DAY: f64 = f64::midpoint(SOLID_DAYS.0, SOLID_DAYS.1);

/// The shortest and longest day a `kind` of world may have, in seconds.
#[must_use]
pub fn days(kind: PlanetType) -> (f64, f64) {
    match kind {
        PlanetType::GasGiant => GAS_GIANT_DAYS,
        _ => SOLID_DAYS,
    }
}

/// How large a bake is: the surface's and the cloud layer's widths (each
/// twice its height, a power of two), and the ring profile's texels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sizes {
    /// The surface's width in texels.
    pub surface: u32,
    /// The cloud layer's width in texels.
    pub clouds: u32,
    /// The ring profile's texels, inner edge to outer.
    pub ring: u32,
}

impl Sizes {
    /// What the engine bakes: a 1024 by 512 surface, 512 by 256 clouds,
    /// and a 256-texel ring profile.
    pub const ENGINE: Sizes = Sizes {
        surface: 1024,
        clouds: 512,
        ring: 256,
    };
}

/// An image: 8-bit RGBA, row by row from the top (north), four bytes a
/// texel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// Its width in texels.
    pub width: u32,
    /// Its height in texels.
    pub height: u32,
    /// Its texels.
    pub rgba: Vec<u8>,
}

impl Image {
    /// The texel in column `x` of row `y`.
    #[must_use]
    pub fn texel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * self.width + x) * 4) as usize;
        [
            self.rgba[at],
            self.rgba[at + 1],
            self.rgba[at + 2],
            self.rgba[at + 3],
        ]
    }
}

/// A planet's rings, as baked.
#[derive(Clone, Debug, PartialEq)]
pub struct RingBand {
    /// Where they start, in planet radii.
    pub inner: f64,
    /// Where they end, in planet radii.
    pub outer: f64,
    /// Their colour.
    pub color: [u8; 3],
    /// Their opacity, 0 to 255, at evenly spaced radii from `inner`
    /// (the first) to `outer` (the last).
    pub profile: Vec<u8>,
}

impl RingBand {
    /// How opaque the rings are `r` planet radii from the centre, 0 to 1:
    /// the profile read linearly between its texels, and 0 outside
    /// `[inner, outer]`.
    #[must_use]
    pub fn opacity(&self, r: f64) -> f64 {
        let last = self.profile.len().saturating_sub(1);
        if !(self.inner..=self.outer).contains(&r) || last == 0 {
            return 0.0;
        }
        let at = (r - self.inner) / (self.outer - self.inner) * last as f64;
        let below = (at.floor() as usize).min(last);
        let above = (below + 1).min(last);
        let t = at - below as f64;
        let byte = |i: usize| f64::from(self.profile[i]) / 255.0;
        byte(below) + (byte(above) - byte(below)) * t
    }
}

/// A planet's atmosphere, as drawn: a glow around the limb.
#[derive(Clone, Debug, PartialEq)]
pub struct Glow {
    /// Its colour.
    pub color: [u8; 3],
    /// Its width outside the limb, in planet radii.
    pub thickness: f64,
}

/// Everything a planet shader needs to draw one planet.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    /// The globe, equirectangular (longitude across from the date line
    /// eastward, latitude down from the north pole), twice as wide as
    /// tall. Its alpha is how much each texel glows on its own (lava's
    /// cracks); 0 is none.
    pub surface: Image,
    /// The cloud layer, laid out as the surface: white, its alpha the
    /// cover. `None` when it has no clouds, or is a gas giant.
    pub clouds: Option<Image>,
    /// Its rings, if it has any.
    pub rings: Option<RingBand>,
    /// Its atmosphere, if it has one.
    pub atmosphere: Option<Glow>,
    /// How long one turn takes, in seconds of game time.
    pub day: f64,
    /// How far its axis rolls from screen-up, in radians (positive
    /// clockwise on screen).
    pub tilt: f64,
}

impl Surface {
    /// A texture asset's pixels as a planet: its surface, glowing nowhere
    /// (its alpha is read as opaque), with no clouds, rings or
    /// atmosphere, [`TEXTURE_DAY`] long and untilted.
    #[must_use]
    pub fn from_texture(texture: Texture) -> Surface {
        let mut rgba = texture.rgba;
        for texel in rgba.chunks_mut(4) {
            texel[3] = 0;
        }
        Surface {
            surface: Image {
                width: texture.width,
                height: texture.height,
                rgba,
            },
            clouds: None,
            rings: None,
            atmosphere: None,
            day: TEXTURE_DAY,
            tilt: 0.0,
        }
    }
}

/// `look` baked at `sizes`.
///
/// # Panics
///
/// When the surface's or the clouds' width isn't a power of two of at
/// least 2, or the ring profile has fewer than 2 texels.
#[must_use]
pub fn bake(look: &Look, sizes: Sizes) -> Surface {
    for width in [sizes.surface, sizes.clouds] {
        assert!(
            width >= 2 && width.is_power_of_two(),
            "a baked layer's width is a power of two: {width}"
        );
    }
    assert!(sizes.ring >= 2, "a ring profile has two ends");
    let seed = look.seed;
    let palette: Vec<Rgb> = look.palette.iter().map(|c| from_bytes(c.rgb())).collect();
    let gas_giant = look.kind == PlanetType::GasGiant;
    let clouds = look.clouds.get();
    let (short, long) = days(look.kind);
    let most_tilt = if gas_giant { GAS_GIANT_TILT } else { TILT };
    Surface {
        surface: surface(look.kind, &palette, clouds, seed, sizes.surface),
        clouds: (!gas_giant && clouds > 0.0).then(|| cloud_layer(clouds, seed, sizes.clouds)),
        rings: look.rings.as_ref().map(|r| RingBand {
            inner: r.inner,
            outer: r.outer,
            color: r.color.rgb(),
            profile: ring_profile(seed, sizes.ring),
        }),
        atmosphere: look.atmosphere.as_ref().map(|a| Glow {
            color: a.color.rgb(),
            thickness: a.thickness.get(),
        }),
        day: short + (long - short) * Stream::new(seed, Use::Spin).unit(0, 0, 0),
        tilt: most_tilt * (Stream::new(seed, Use::Tilt).unit(0, 0, 0) * 2.0 - 1.0),
    }
}

/// A colour, each channel 0 to 1.
type Rgb = [f64; 3];

const WHITE: Rgb = [1.0; 3];
const BLACK: Rgb = [0.0; 3];

fn from_bytes(c: [u8; 3]) -> Rgb {
    c.map(|c| f64::from(c) / 255.0)
}

/// The byte for the channel `c`, 0 to 1.
fn byte(c: f64) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8
}

/// `a` blended toward `b` by `t`, 0 to 1.
fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// `c` lightened, toward white, by `by`.
fn lighter(c: Rgb, by: f64) -> Rgb {
    mix(c, WHITE, by)
}

/// `c` darkened, toward black, by `by`.
fn darker(c: Rgb, by: f64) -> Rgb {
    mix(c, BLACK, by)
}

/// `n` colours from `given`, in order: those given, then shades of them,
/// lighter and darker by turns and further each round. Bare grey when
/// nothing is given.
#[must_use]
pub fn stops(given: &[[f64; 3]], n: usize) -> Vec<[f64; 3]> {
    let given = if given.is_empty() {
        &[[0.5; 3]][..]
    } else {
        given
    };
    let len = given.len();
    (0..n)
        .map(|k| {
            if k < len {
                return given[k];
            }
            let round = (k - len) / len;
            let base = given[(k - len) % len];
            let by = 0.3 + 0.2 * (round / 2) as f64;
            if round % 2 == 0 {
                lighter(base, by)
            } else {
                darker(base, by)
            }
        })
        .collect()
}

/// What each value drawn from the seed is for: each use hashes its own
/// stream, so a change to one recipe moves no other's values.
#[derive(Clone, Copy)]
enum Use {
    Surface = 1,
    Clouds = 2,
    Rings = 3,
    Bands = 4,
    Spin = 5,
    Tilt = 6,
    Craters = 7,
    Caps = 8,
}

/// `SplitMix64`'s output mixer.
fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The values a seed gives one [`Use`]: a hash of three integers.
#[derive(Clone, Copy)]
struct Stream(u64);

impl Stream {
    fn new(seed: u32, use_: Use) -> Self {
        Stream(mix64((u64::from(seed) << 8) | use_ as u64))
    }

    fn at(self, a: u32, b: u32, c: u32) -> u64 {
        let ab = mix64(self.0 ^ (u64::from(a) | (u64::from(b) << 32)));
        mix64(ab ^ u64::from(c).wrapping_mul(0xD6E8_FEB8_6659_FD93))
    }

    /// A value from 0 up to 1.
    fn unit(self, a: u32, b: u32, c: u32) -> f64 {
        (self.at(a, b, c) >> 11) as f64 / (1_u64 << 53) as f64
    }
}

/// Smoothstep of `t`, clamped to 0 to 1.
fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Fractal value noise on the globe, periodic in longitude.
#[derive(Clone, Copy)]
struct Noise {
    stream: Stream,
    /// Octaves summed.
    octaves: u32,
    /// Lattice cells around the equator at the first octave.
    period: u32,
    /// Each octave's weight over the one before.
    gain: f64,
    /// How much finer it is north–south than east–west: above 1, its
    /// features stretch east–west.
    stretch: f64,
}

impl Noise {
    /// The noise at longitude `u` (0 to 1 around) and latitude `v` (0 at
    /// the north pole, 1 at the south), from 0 to 1. Near the poles it
    /// fades to its value on the date line's meridian, the same at every
    /// longitude, so the poles don't pinch.
    fn at(&self, u: f64, v: f64) -> f64 {
        let t = 2.0 * v - 1.0;
        let pole = t * t * t * t;
        self.fbm(u, v) * (1.0 - pole) + self.fbm(0.0, v) * pole
    }

    fn fbm(&self, u: f64, v: f64) -> f64 {
        let (mut sum, mut norm, mut weight, mut period) = (0.0, 0.0, 1.0, self.period);
        for octave in 0..self.octaves {
            let p = f64::from(period);
            sum += weight * self.lattice(octave, period, u * p, v * p * 0.5 * self.stretch);
            norm += weight;
            weight *= self.gain;
            period *= 2;
        }
        sum / norm
    }

    /// The value noise of one octave at `(x, y)` lattice units, wrapping
    /// every `period` cells in x.
    fn lattice(&self, octave: u32, period: u32, x: f64, y: f64) -> f64 {
        let (fx, fy) = (x.floor(), y.floor());
        let (tx, ty) = (smooth(x - fx), smooth(y - fy));
        let x0 = (fx as i64).rem_euclid(i64::from(period)) as u32;
        let x1 = (x0 + 1) % period;
        let y0 = fy.max(0.0) as u32;
        let at = |x, y| self.stream.unit(octave, x, y);
        let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * tx;
        let bottom = at(x0, y0 + 1) + (at(x1, y0 + 1) - at(x0, y0 + 1)) * tx;
        top + (bottom - top) * ty
    }
}

/// Each of `values`' rank among them, from 0 to 1: the share of the
/// values below it. Ties go by position.
fn ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|a, b| values[*a].total_cmp(&values[*b]).then(a.cmp(b)));
    let n = values.len() as f64;
    let mut ranked = vec![0.0; values.len()];
    for (k, at) in order.into_iter().enumerate() {
        ranked[at] = (k as f64 + 0.5) / n;
    }
    ranked
}

/// The colour of rank `q` (0 to 1) on a ramp through `stops`, the first
/// covering the most and each later one less, blended across `soft` at
/// each edge.
fn ramp(stops: &[Rgb], q: f64, soft: f64) -> Rgb {
    let n = stops.len();
    let total = (n * (n + 1) / 2) as f64;
    let mut color = stops[0];
    let mut edge = 0.0;
    for (k, stop) in stops.iter().enumerate().skip(1) {
        edge += (n + 1 - k) as f64 / total;
        color = mix(color, *stop, smooth((q - edge + soft) / (2.0 * soft)));
    }
    color
}

/// Each texel's longitude and latitude (0 to 1) on a `width` by
/// `width / 2` layer, row by row.
fn texels(width: u32) -> impl Iterator<Item = (f64, f64)> {
    let height = width / 2;
    (0..height).flat_map(move |y| {
        (0..width).map(move |x| {
            (
                (f64::from(x) + 0.5) / f64::from(width),
                (f64::from(y) + 0.5) / f64::from(height),
            )
        })
    })
}

/// A recipe's height field: how rough, and how stretched.
fn heights(kind: PlanetType, seed: u32, width: u32) -> Vec<f64> {
    let (octaves, period, gain, stretch) = match kind {
        PlanetType::Rocky => (6, 6, 0.6, 1.0),
        PlanetType::Desert => (4, 4, 0.45, 2.5),
        PlanetType::Lava => (5, 6, 0.55, 1.0),
        _ => (5, 5, 0.5, 1.0),
    };
    let noise = Noise {
        stream: Stream::new(seed, Use::Surface),
        octaves,
        period,
        gain,
        stretch,
    };
    let mut field: Vec<f64> = texels(width).map(|(u, v)| noise.at(u, v)).collect();
    if kind == PlanetType::Rocky {
        craters(&mut field, seed, width);
    }
    field
}

/// Seeded pits pressed into a rocky world's `field`.
fn craters(field: &mut [f64], seed: u32, width: u32) {
    let stream = Stream::new(seed, Use::Craters);
    let count = 12 + (stream.unit(0, 0, 0) * 8.0) as u32;
    let pits: Vec<(f64, f64, f64)> = (1..=count)
        .map(|i| {
            (
                stream.unit(i, 0, 0),
                0.15 + 0.7 * stream.unit(i, 1, 0),
                0.02 + 0.04 * stream.unit(i, 2, 0),
            )
        })
        .collect();
    for ((u, v), height) in texels(width).zip(field.iter_mut()) {
        for (cu, cv, radius) in &pits {
            let du = (u - cu).abs();
            let du = du.min(1.0 - du);
            // A degree of latitude is twice a degree of longitude's share.
            let dv = (v - cv) * 0.5;
            let near = (du * du + dv * dv) / (radius * radius);
            if near < 1.0 {
                *height -= 0.25 * (1.0 - near);
            }
        }
    }
}

/// The white of polar caps starting `from` the equator (0) toward the
/// poles (1), at latitude `v`, their edge jittered by `jitter` (0 to 1).
fn cap(from: f64, v: f64, jitter: f64) -> f64 {
    let t = (2.0 * v - 1.0).abs();
    smooth((t - from + (jitter - 0.5) * 0.1) / 0.06)
}

/// The surface of a `kind` of world in `palette`, `width` texels across.
fn surface(kind: PlanetType, palette: &[Rgb], clouds: f64, seed: u32, width: u32) -> Image {
    let colors: Vec<(Rgb, f64)> = if kind == PlanetType::GasGiant {
        bands(palette, clouds, seed, width)
    } else {
        let ranked = ranks(&heights(kind, seed, width));
        let caps = Noise {
            stream: Stream::new(seed, Use::Caps),
            octaves: 3,
            period: 8,
            gain: 0.5,
            stretch: 1.0,
        };
        texels(width)
            .zip(ranked)
            .map(|((u, v), q)| ground(kind, palette, q, v, || caps.at(u, v)))
            .collect()
    };
    Image {
        width,
        height: width / 2,
        rgba: colors
            .into_iter()
            .flat_map(|(c, glow)| [byte(c[0]), byte(c[1]), byte(c[2]), byte(glow)])
            .collect(),
    }
}

/// The colour and glow of a texel of rank `q` at latitude `v` on a world
/// with a surface; `jitter` gives the caps' ragged edge.
fn ground(
    kind: PlanetType,
    palette: &[Rgb],
    q: f64,
    v: f64,
    jitter: impl Fn() -> f64,
) -> (Rgb, f64) {
    let rest = if palette.len() > 1 {
        &palette[1..]
    } else {
        palette
    };
    match kind {
        PlanetType::Rocky => (ramp(&stops(palette, 4), q, 0.02), 0.0),
        PlanetType::Desert => (ramp(&stops(palette, 3), q, 0.08), 0.0),
        PlanetType::Ocean | PlanetType::Garden => {
            let sea = if kind == PlanetType::Ocean { 0.7 } else { 0.5 };
            let water = palette.first().copied().unwrap_or([0.5; 3]);
            let color = if q < sea {
                mix(darker(water, 0.3), water, q / sea)
            } else {
                ramp(&stops(rest, 3), (q - sea) / (1.0 - sea), 0.03)
            };
            if kind == PlanetType::Garden {
                (mix(color, WHITE, cap(0.82, v, jitter())), 0.0)
            } else {
                (color, 0.0)
            }
        }
        PlanetType::Ice => {
            let color = ramp(&stops(palette, 3), q, 0.05);
            (mix(color, WHITE, cap(0.55, v, jitter())), 0.0)
        }
        PlanetType::Lava => {
            let (crust, glow) = match palette.split_last() {
                Some((last, crust)) if !crust.is_empty() => (stops(crust, 3), *last),
                _ => {
                    let only = palette.first().copied().unwrap_or([0.5; 3]);
                    (stops(&[darker(only, 0.6)], 3), only)
                }
            };
            let crack = 0.18;
            let lit = smooth((crack - q) / (crack * 0.6));
            let color = ramp(&crust, (q - crack).max(0.0) / (1.0 - crack), 0.04);
            (mix(color, glow, lit), lit)
        }
        PlanetType::GasGiant => unreachable!("a gas giant has no ground"),
    }
}

/// How far turbulence moves a gas giant's bands, in its latitude's
/// range of 2 (pole to pole), at `clouds` 1.
const TURBULENCE: f64 = 0.08;

/// A gas giant's texels: latitude bands of seeded widths in `palette`,
/// `palette[0]` every other band, bent along by `clouds`' turbulence.
fn bands(palette: &[Rgb], clouds: f64, seed: u32, width: u32) -> Vec<(Rgb, f64)> {
    let stream = Stream::new(seed, Use::Bands);
    let stops = stops(palette, 4);
    let count = 9 + (stream.unit(0, 0, 0) * 6.0) as u32;
    let widths: Vec<f64> = (1..=count).map(|i| 0.5 + stream.unit(i, 0, 0)).collect();
    let total: f64 = widths.iter().sum();
    let mut edges = Vec::with_capacity(widths.len());
    let mut at = -1.0;
    for w in &widths {
        at += 2.0 * w / total;
        edges.push(at);
    }
    // 0, 1, 0, 2, 0, 1, 0, 3, …: the first colour the most.
    let color_of = |band: usize| stops[((band + 1).trailing_zeros() as usize).min(stops.len() - 1)];
    let turbulence = Noise {
        stream: Stream::new(seed, Use::Clouds),
        octaves: 4,
        period: 4,
        gain: 0.55,
        stretch: 4.0,
    };
    texels(width)
        .map(|(u, v)| {
            let t = 2.0 * v - 1.0 + clouds * TURBULENCE * (turbulence.at(u, v) * 2.0 - 1.0);
            let mut color = color_of(0);
            for (band, edge) in edges.iter().enumerate().take(edges.len() - 1) {
                color = mix(color, color_of(band + 1), smooth((t - edge + 0.02) / 0.04));
            }
            (color, 0.0)
        })
        .collect()
}

/// The cloud layer covering `clouds` of a globe, `width` texels across.
fn cloud_layer(clouds: f64, seed: u32, width: u32) -> Image {
    let noise = Noise {
        stream: Stream::new(seed, Use::Clouds),
        octaves: 5,
        period: 6,
        gain: 0.5,
        stretch: 1.5,
    };
    let field: Vec<f64> = texels(width).map(|(u, v)| noise.at(u, v)).collect();
    let soft = 0.08;
    Image {
        width,
        height: width / 2,
        rgba: ranks(&field)
            .into_iter()
            .flat_map(|q| {
                let cover = smooth((q - (1.0 - clouds)) / soft + 0.5);
                [255, 255, 255, byte(cover * CLOUD_OPACITY)]
            })
            .collect(),
    }
}

/// A ring's opacity from its inner edge to its outer, `texels` of it:
/// banded by the seed, with gaps, fading in and out at its edges.
fn ring_profile(seed: u32, texels: u32) -> Vec<u8> {
    let stream = Stream::new(seed, Use::Rings);
    let noise = Noise {
        stream,
        octaves: 4,
        period: 16,
        gain: 0.6,
        stretch: 0.0,
    };
    let gaps: Vec<(f64, f64)> = (1..=2 + (stream.unit(0, 0, 1) * 3.0) as u32)
        .map(|i| {
            (
                0.1 + 0.8 * stream.unit(i, 0, 1),
                0.01 + 0.03 * stream.unit(i, 1, 1),
            )
        })
        .collect();
    let last = f64::from(texels - 1);
    (0..texels)
        .map(|i| {
            let r = f64::from(i) / last;
            let density = 0.25 + 0.75 * noise.fbm(r, 0.0);
            let open = gaps
                .iter()
                .map(|(at, half)| smooth(((r - at).abs() - half) / 0.01))
                .product::<f64>();
            let edge = smooth(r / 0.05) * smooth((1.0 - r) / 0.05);
            byte(RING_PEAK * density * open * edge)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_galaxy_format::id::{Color, Fraction};
    use nova_galaxy_format::model::{Atmosphere, Rings};

    use super::*;

    const SMALL: Sizes = Sizes {
        surface: 64,
        clouds: 32,
        ring: 32,
    };

    fn color(hex: &str) -> Color {
        Color::try_from(hex.to_owned()).unwrap()
    }

    fn fraction(f: f64) -> Fraction {
        Fraction::try_from(f).unwrap()
    }

    /// Kestrel Reach's look, as the sample writes it.
    fn kestrel_reach() -> Look {
        Look {
            kind: PlanetType::GasGiant,
            palette: vec![color("#c9a36b"), color("#8a5a3c"), color("#e8d8b0")],
            clouds: fraction(0.6),
            rings: Some(Rings {
                inner: 1.4,
                outer: 2.3,
                color: color("#d8c8a8"),
            }),
            atmosphere: Some(Atmosphere {
                color: color("#f0e0c0"),
                thickness: fraction(0.04),
            }),
            seed: 4417,
        }
    }

    /// A plain world of `kind` in two mid colours, no extras.
    fn world(kind: PlanetType, clouds: f64, seed: u32) -> Look {
        Look {
            kind,
            palette: vec![color("#336699"), color("#887744")],
            clouds: fraction(clouds),
            rings: None,
            atmosphere: None,
            seed,
        }
    }

    const KINDS: [PlanetType; 7] = [
        PlanetType::Rocky,
        PlanetType::Desert,
        PlanetType::Ocean,
        PlanetType::Garden,
        PlanetType::Ice,
        PlanetType::Lava,
        PlanetType::GasGiant,
    ];

    /// FNV-1a over `bytes`.
    fn fnv(hash: u64, bytes: &[u8]) -> u64 {
        bytes.iter().fold(hash, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01B3)
        })
    }

    fn digest(surface: &Surface) -> u64 {
        let mut h = fnv(0xCBF2_9CE4_8422_2325, &surface.surface.rgba);
        if let Some(clouds) = &surface.clouds {
            h = fnv(h, &clouds.rgba);
        }
        if let Some(rings) = &surface.rings {
            h = fnv(h, &rings.profile);
        }
        h = fnv(h, &surface.day.to_bits().to_le_bytes());
        fnv(h, &surface.tilt.to_bits().to_le_bytes())
    }

    #[test]
    fn a_look_bakes_to_the_same_bytes_every_time_and_everywhere() {
        let look = kestrel_reach();
        assert_eq!(bake(&look, SMALL), bake(&look, SMALL));
        let other = Look {
            seed: 4418,
            ..kestrel_reach()
        };
        assert_ne!(bake(&other, SMALL).surface, bake(&look, SMALL).surface);
        // The cross-platform pin: the same on every CI OS. Change it only
        // with the recipe, on purpose.
        assert_eq!(digest(&bake(&look, SMALL)), KESTREL_REACH_DIGEST);
        let garden = Look {
            clouds: fraction(0.4),
            ..world(PlanetType::Garden, 0.4, 7)
        };
        assert_eq!(digest(&bake(&garden, SMALL)), GARDEN_DIGEST);
    }

    const KESTREL_REACH_DIGEST: u64 = 14_843_289_236_354_615_152;
    const GARDEN_DIGEST: u64 = 6_254_386_249_019_385_006;

    /// The largest difference between two texels' colours.
    fn apart(a: [u8; 4], b: [u8; 4]) -> u8 {
        (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0)
    }

    #[test]
    fn the_date_line_has_no_seam() {
        for kind in KINDS {
            let image = bake(&world(kind, 0.5, 11), SMALL).surface;
            let last = image.width - 1;
            for y in 0..image.height {
                let steepest = (0..last)
                    .map(|x| apart(image.texel(x, y), image.texel(x + 1, y)))
                    .max()
                    .unwrap_or(0);
                let seam = apart(image.texel(last, y), image.texel(0, y));
                assert!(
                    seam <= steepest.max(2),
                    "{kind:?} row {y}: the seam is {seam} apart, the steepest step {steepest}"
                );
            }
        }
    }

    /// Whether `texel` is within the box `allowed` spans, channel by
    /// channel, to a rounding.
    fn within(allowed: &[Rgb], texel: [u8; 4]) -> bool {
        (0..3).all(|i| {
            let lo = allowed.iter().map(|c| byte(c[i])).min().unwrap_or(0);
            let hi = allowed.iter().map(|c| byte(c[i])).max().unwrap_or(255);
            (lo..=hi).contains(&texel[i])
        })
    }

    #[test]
    fn every_texel_blends_the_palette_its_shades_and_white() {
        for kind in KINDS {
            let look = world(kind, 0.5, 3);
            let given: Vec<Rgb> = look.palette.iter().map(|c| from_bytes(c.rgb())).collect();
            let mut allowed = stops(&given, 8);
            allowed.extend(given.iter().map(|c| darker(*c, 0.6)));
            allowed.extend(stops(&[darker(given[0], 0.6)], 3));
            allowed.push(WHITE);
            let image = bake(&look, SMALL).surface;
            for y in 0..image.height {
                for x in 0..image.width {
                    assert!(within(&allowed, image.texel(x, y)), "{kind:?} at {x},{y}");
                }
            }
        }
        // Red alone never makes blue.
        let red = Look {
            palette: vec![color("#c03020")],
            ..world(PlanetType::Rocky, 0.0, 3)
        };
        let image = bake(&red, SMALL).surface;
        assert!(image.rgba.chunks(4).all(|t| t[2] < t[0]));
    }

    #[test]
    fn a_one_colour_planet_still_has_relief() {
        for kind in KINDS {
            let look = Look {
                palette: vec![color("#807060")],
                ..world(kind, 0.0, 5)
            };
            let image = bake(&look, SMALL).surface;
            let first = image.texel(0, 0);
            assert!(
                (0..image.height)
                    .any(|y| (0..image.width).any(|x| apart(image.texel(x, y), first) > 20)),
                "{kind:?} varies"
            );
        }
    }

    #[test]
    fn shades_lighten_then_darken_those_given() {
        let given = [[0.5, 0.5, 0.5]];
        let made = stops(&given, 4);
        assert_eq!(made[0], [0.5; 3]);
        assert!(made[1][0] > 0.5 && made[2][0] < 0.5);
        assert!(made[3][0] > made[1][0], "further each round: {made:?}");
        assert_eq!(stops(&[], 2), vec![[0.5; 3], lighter([0.5; 3], 0.3)]);
        let two = stops(&[[0.2; 3], [0.8; 3]], 4);
        assert_eq!(two[2], lighter([0.2; 3], 0.3));
        assert_eq!(two[3], lighter([0.8; 3], 0.3));
    }

    /// The share of `image`'s texels in rows less than `within` from the
    /// equator (1 at the poles) that are `water` or a blend of it with its
    /// darker shade.
    fn water(image: &Image, water: Rgb, within: f64) -> f64 {
        let deep = darker(water, 0.3);
        let is_water = |t: [u8; 4]| {
            (0..3).all(|i| {
                let (a, b) = (byte(deep[i]), byte(water[i]));
                (a.min(b)..=a.max(b)).contains(&t[i])
            })
        };
        let (mut wet, mut all) = (0, 0);
        for y in 0..image.height {
            let t = ((f64::from(y) + 0.5) / f64::from(image.height) * 2.0 - 1.0).abs();
            if t < within {
                for x in 0..image.width {
                    all += 1;
                    wet += usize::from(is_water(image.texel(x, y)));
                }
            }
        }
        wet as f64 / f64::from(all)
    }

    #[test]
    fn an_ocean_is_mostly_water_and_wetter_than_a_garden() {
        let sea = from_bytes([0x33, 0x66, 0x99]);
        let ocean = bake(&world(PlanetType::Ocean, 0.0, 9), SMALL).surface;
        let garden = bake(&world(PlanetType::Garden, 0.0, 9), SMALL).surface;
        let (ocean_all, garden_all) = (water(&ocean, sea, 1.0), water(&garden, sea, 1.0));
        assert!((0.68..=0.72).contains(&ocean_all), "{ocean_all}");
        assert!(
            ocean_all > garden_all + 0.15,
            "{ocean_all} and {garden_all}"
        );
        // Away from its caps, half a garden is water, over a few worlds.
        let garden_middle = (0..8)
            .map(|seed| {
                water(
                    &bake(&world(PlanetType::Garden, 0.0, seed), SMALL).surface,
                    sea,
                    0.7,
                )
            })
            .sum::<f64>()
            / 8.0;
        assert!((0.42..=0.58).contains(&garden_middle), "{garden_middle}");
    }

    /// Whether `t` is (nearly) white.
    fn white(t: [u8; 4]) -> bool {
        t[..3].iter().all(|c| *c >= 250)
    }

    #[test]
    fn caps_are_white_and_only_near_the_poles() {
        for (kind, from) in [(PlanetType::Garden, 0.82), (PlanetType::Ice, 0.55)] {
            let image = bake(&world(kind, 0.0, 13), SMALL).surface;
            let mut any = false;
            for y in 0..image.height {
                let t = ((f64::from(y) + 0.5) / f64::from(image.height) * 2.0 - 1.0).abs();
                for x in 0..image.width {
                    if white(image.texel(x, y)) {
                        any = true;
                        assert!(
                            t > from - 0.1,
                            "{kind:?}: white at {x},{y}, {t} from the equator"
                        );
                    }
                }
            }
            assert!(any, "{kind:?} has caps");
            assert!(white(image.texel(0, 0)), "{kind:?}: the pole is white");
        }
        for kind in [PlanetType::Rocky, PlanetType::Ocean, PlanetType::Desert] {
            let image = bake(&world(kind, 0.0, 13), SMALL).surface;
            assert!(
                !image
                    .rgba
                    .chunks(4)
                    .any(|t| white([t[0], t[1], t[2], t[3]])),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn only_lava_glows() {
        for kind in KINDS {
            let image = bake(&world(kind, 0.3, 17), SMALL).surface;
            let glowing = image.rgba.chunks(4).filter(|t| t[3] > 0).count();
            if kind == PlanetType::Lava {
                let share = glowing as f64 / (image.rgba.len() / 4) as f64;
                assert!(
                    (0.05..=0.25).contains(&share),
                    "lava glows in its cracks: {share}"
                );
                // Where it glows most, it is the last colour.
                let brightest = image.rgba.chunks(4).max_by_key(|t| t[3]).unwrap();
                assert_eq!(brightest[3], 255);
                assert_eq!(&brightest[..3], &[0x88, 0x77, 0x44]);
            } else {
                assert_eq!(glowing, 0, "{kind:?}");
            }
        }
    }

    /// The mean difference between neighbours along rows, and across.
    fn along_and_across(image: &Image) -> (f64, f64) {
        let (mut along, mut across) = (0.0, 0.0);
        for y in 0..image.height - 1 {
            for x in 0..image.width {
                along += f64::from(apart(
                    image.texel(x, y),
                    image.texel((x + 1) % image.width, y),
                ));
                across += f64::from(apart(image.texel(x, y), image.texel(x, y + 1)));
            }
        }
        (along, across)
    }

    #[test]
    fn a_gas_giant_is_banded() {
        let stormy = bake(&kestrel_reach(), SMALL).surface;
        let (along, across) = along_and_across(&stormy);
        assert!(along * 2.0 < across, "{along} along, {across} across");
        assert!(along > 0.0, "clouds 0.6 bend the bands");
        let calm = Look {
            clouds: fraction(0.0),
            ..kestrel_reach()
        };
        let image = bake(&calm, SMALL).surface;
        for y in 0..image.height {
            let first = image.texel(0, y);
            assert!(
                (0..image.width).all(|x| image.texel(x, y) == first),
                "row {y}"
            );
        }
        let (_, across) = along_and_across(&image);
        assert!(across > 0.0);
    }

    /// The share of `image`'s texels at least half covered.
    fn covered(image: &Image) -> f64 {
        let half = byte(CLOUD_OPACITY / 2.0);
        image.rgba.chunks(4).filter(|t| t[3] >= half).count() as f64 / (image.rgba.len() / 4) as f64
    }

    #[test]
    fn clouds_cover_their_share_of_the_globe_in_white() {
        for share in [0.2, 0.5, 0.8] {
            let layer = bake(&world(PlanetType::Garden, share, 19), SMALL)
                .clouds
                .unwrap();
            assert_eq!((layer.width, layer.height), (32, 16));
            let got = covered(&layer);
            assert!((got - share).abs() < 0.05, "{share}: {got}");
            assert!(layer.rgba.chunks(4).all(|t| t[..3] == [255; 3]));
            assert!(layer.rgba.chunks(4).all(|t| t[3] <= byte(CLOUD_OPACITY)));
        }
        let all = bake(&world(PlanetType::Garden, 1.0, 19), SMALL)
            .clouds
            .unwrap();
        assert!(covered(&all) > 0.95);
        assert_eq!(
            bake(&world(PlanetType::Garden, 0.0, 19), SMALL).clouds,
            None
        );
        assert_eq!(
            bake(&kestrel_reach(), SMALL).clouds,
            None,
            "a gas giant is all cloud"
        );
    }

    #[test]
    fn rings_are_banded_within_their_edges_in_their_colour() {
        let rings = bake(&kestrel_reach(), SMALL).rings.unwrap();
        assert_eq!((rings.inner, rings.outer), (1.4, 2.3));
        assert_eq!(rings.color, [0xd8, 0xc8, 0xa8]);
        assert_eq!(rings.profile.len(), 32);
        let peak = *rings.profile.iter().max().unwrap();
        assert!(peak <= byte(RING_PEAK) && peak > 0, "{peak}");
        assert_eq!(rings.profile[0], 0, "fades in");
        assert_eq!(rings.profile[31], 0, "fades out");
        assert!(rings.profile.iter().any(|o| *o < peak / 2), "has gaps");
        assert_eq!(rings.opacity(1.39), 0.0);
        assert_eq!(rings.opacity(2.31), 0.0);
        assert!(rings.opacity(1.85) > 0.0 || rings.profile[15..17].contains(&0));
        // Read between texels linearly.
        let band = RingBand {
            inner: 1.0,
            outer: 2.0,
            color: [0; 3],
            profile: vec![0, 255, 51],
        };
        assert_eq!(band.opacity(1.0), 0.0);
        assert_eq!(band.opacity(1.25), 0.5);
        assert_eq!(band.opacity(1.5), 1.0);
        assert!((band.opacity(2.0) - 0.2).abs() < 1e-12);
        assert_eq!(band.opacity(0.5), 0.0);
        assert_eq!(bake(&world(PlanetType::Rocky, 0.0, 1), SMALL).rings, None);
    }

    #[test]
    fn day_and_tilt_come_from_the_seed_within_the_type_s_range() {
        for kind in [PlanetType::Rocky, PlanetType::GasGiant] {
            let (short, long) = days(kind);
            let most = if kind == PlanetType::GasGiant {
                GAS_GIANT_TILT
            } else {
                TILT
            };
            let baked: Vec<Surface> = (0..40)
                .map(|seed| {
                    bake(
                        &world(kind, 0.0, seed),
                        Sizes {
                            surface: 2,
                            clouds: 2,
                            ring: 2,
                        },
                    )
                })
                .collect();
            for s in &baked {
                assert!((short..=long).contains(&s.day), "{kind:?}: {}", s.day);
                assert!(s.tilt.abs() <= most, "{kind:?}: {}", s.tilt);
            }
            let spread = |f: fn(&Surface) -> f64| {
                let values = baked.iter().map(f);
                values.clone().fold(f64::MIN, f64::max) - values.fold(f64::MAX, f64::min)
            };
            assert!(
                spread(|s| s.day) > 0.8 * (long - short),
                "{kind:?} days spread"
            );
            assert!(spread(|s| s.tilt) > 1.6 * most, "{kind:?} tilts spread");
        }
        const { assert!(GAS_GIANT_DAYS.1 <= SOLID_DAYS.0, "gas giants turn faster") };
        const { assert!(GAS_GIANT_TILT < TILT) };
        let a = bake(&world(PlanetType::Ice, 0.0, 77), SMALL);
        let b = bake(&world(PlanetType::Ice, 0.0, 77), SMALL);
        assert_eq!((a.day, a.tilt), (b.day, b.tilt));
    }

    #[test]
    fn the_atmosphere_is_carried_through() {
        assert_eq!(
            bake(&kestrel_reach(), SMALL).atmosphere,
            Some(Glow {
                color: [0xf0, 0xe0, 0xc0],
                thickness: 0.04,
            })
        );
        assert_eq!(
            bake(&world(PlanetType::Rocky, 0.0, 1), SMALL).atmosphere,
            None
        );
    }

    #[test]
    fn a_bake_takes_its_sizes() {
        let surface = bake(&world(PlanetType::Garden, 0.5, 1), SMALL);
        assert_eq!((surface.surface.width, surface.surface.height), (64, 32));
        assert_eq!(surface.surface.rgba.len(), 64 * 32 * 4);
        assert_eq!(surface.clouds.map(|c| c.rgba.len()), Some(32 * 16 * 4));
    }

    #[test]
    #[should_panic(expected = "a power of two")]
    fn a_bake_s_width_is_a_power_of_two() {
        let _ = bake(
            &kestrel_reach(),
            Sizes {
                surface: 48,
                ..SMALL
            },
        );
    }

    #[test]
    fn a_texture_is_a_planet_with_no_glow_and_no_extras() {
        let texture = Texture {
            file: "media/export/p/a.png".into(),
            width: 2,
            height: 1,
            disk_bytes: 9,
            rgba: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        assert_eq!(
            Surface::from_texture(texture),
            Surface {
                surface: Image {
                    width: 2,
                    height: 1,
                    rgba: vec![1, 2, 3, 0, 5, 6, 7, 0],
                },
                clouds: None,
                rings: None,
                atmosphere: None,
                day: TEXTURE_DAY,
                tilt: 0.0,
            }
        );
        assert_eq!(TEXTURE_DAY, 360.0);
    }

    #[test]
    fn ranks_spread_evenly_and_keep_order() {
        assert_eq!(ranks(&[3.0, 1.0, 2.0, 1.0]), [0.875, 0.125, 0.625, 0.375]);
    }

    #[test]
    fn a_ramp_gives_its_first_colour_the_most() {
        let stops = [[0.0; 3], [0.5; 3], [1.0; 3]];
        let n = 300;
        let counts = (0..n).fold([0; 3], |mut counts, i| {
            let c = ramp(&stops, (f64::from(i) + 0.5) / f64::from(n), 0.001);
            counts[(c[0] * 2.0 + 0.5).floor() as usize] += 1;
            counts
        });
        // Shares 3:2:1.
        assert_eq!(counts, [150, 100, 50]);
    }
}
