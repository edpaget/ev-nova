//! How a model's base-colour texture is sampled ([`ModelSampler`]: its
//! wrap along each axis, its filters and the mip levels it reads), and
//! what a sample is ([`sample`]).
//!
//! A sampler belongs to a material's use of a texture, not to the
//! texture: two materials may draw one image through two samplers, and
//! then share its one decode and its one upload
//! ([`crate::model_texture`]). It reaches the GPU on each
//! [`Section`](crate::model::Section) of the mesh
//! ([`crate::Gpu::upload_mesh`]), so nothing about it is learned on a
//! frame.
//!
//! [`sample`] is the reference: the texel a sampler reads from a texture
//! at a coordinate and a level of detail, in the texel-index terms
//! OpenGL defines, which glTF's sampler names. The wgpu adapter binds a
//! GPU sampler that does the same (its `descriptor` maps each value
//! here), and the offscreen tests compare the GPU's pixels with [`sample`]
//! times [`crate::camera::fragment`].
//!
//! A new sampling property is a field of [`ModelSampler`], defined by
//! [`sample`] and mirrored by the adapter's `descriptor`.

use nova_galaxy_media::planet::Image;

use crate::model_texture::ModelTexture;

/// How a sampler treats a coordinate outside 0 to 1, along one axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wrap {
    /// The image again.
    Repeat,
    /// The image again, turned over each time.
    Mirror,
    /// The edge texel.
    Clamp,
}

/// How a sampler reads one level of a texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    /// The texel the coordinate falls in.
    Nearest,
    /// The four texels round the coordinate, blended by how near each is.
    Linear,
}

/// Which mip levels a sampler reads when a texture is drawn smaller than
/// it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mip {
    /// The whole image only, however small it is drawn.
    None,
    /// The nearest level.
    Nearest,
    /// The two nearest levels, blended.
    Linear,
}

/// How a section samples its texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModelSampler {
    /// The wrap along u.
    pub wrap_s: Wrap,
    /// The wrap along v.
    pub wrap_t: Wrap,
    /// The filter when the texture is drawn larger than it is.
    pub mag: Filter,
    /// The filter within a level when it is drawn smaller.
    pub min: Filter,
    /// The mip levels read when it is drawn smaller.
    pub mip: Mip,
}

impl ModelSampler {
    /// What a texture is sampled by when its model says nothing:
    /// repeating both ways and trilinear. A section with no texture
    /// carries it too.
    pub const DEFAULT: ModelSampler = ModelSampler {
        wrap_s: Wrap::Repeat,
        wrap_t: Wrap::Repeat,
        mag: Filter::Linear,
        min: Filter::Linear,
        mip: Mip::Linear,
    };
}

impl Default for ModelSampler {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The texel `sampler` reads from `texture` at `uv` (u across from the
/// left, v down from the top, 0 to 1 over the image) and the level of
/// detail `lod` (the base-2 logarithm of how many texels a pixel covers
/// along a side): straight RGBA, each channel 0 to 1.
///
/// - At `lod` 0 or below the texture is magnified: the first level is
///   read through [`ModelSampler::mag`].
/// - Above 0 it is minified, and read through [`ModelSampler::min`]: the
///   first level alone for [`Mip::None`]; the nearest level for
///   [`Mip::Nearest`] (a `lod` halfway between two takes the larger
///   level); the two levels either side, blended by `lod`'s fraction,
///   for [`Mip::Linear`]. A `lod` past the chain reads its last level.
/// - Within a level `width` by `height`, [`Filter::Nearest`] reads texel
///   `floor(u * width)`, `floor(v * height)`; [`Filter::Linear`] blends
///   the two texels either side of `u * width - 0.5` by its fraction,
///   and the same down.
/// - Each texel index is wrapped by its axis's [`Wrap`]: repeated,
///   mirrored, or held to the edge.
#[must_use]
pub fn sample(texture: &ModelTexture, sampler: ModelSampler, uv: [f32; 2], lod: f32) -> [f32; 4] {
    let levels = texture.levels();
    if lod <= 0.0 {
        return read(&levels[0], sampler, sampler.mag, uv);
    }
    let last = levels.len() - 1;
    let level = |at: f32| &levels[whole(at).clamp(0, last as i64) as usize];
    match sampler.mip {
        Mip::None => read(&levels[0], sampler, sampler.min, uv),
        Mip::Nearest => read(level((lod + 0.5).ceil() - 1.0), sampler, sampler.min, uv),
        Mip::Linear => {
            let below = lod.floor();
            let near = read(level(below), sampler, sampler.min, uv);
            let far = read(level(below + 1.0), sampler, sampler.min, uv);
            mix(near, far, lod - below)
        }
    }
}

/// `value`, a whole number, as an integer.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a texel index or a mip level is far inside i64, and a coordinate that isn't a \
              number reads as 0"
)]
fn whole(value: f32) -> i64 {
    value as i64
}

/// `a` and `b` blended, `t` of the way to `b`.
fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [0, 1, 2, 3].map(|c| a[c] + (b[c] - a[c]) * t)
}

/// What `sampler` reads from the one level `level` at `uv` through
/// `filter`, wrapping each texel index by its axis's mode.
fn read(level: &Image, sampler: ModelSampler, filter: Filter, [u, v]: [f32; 2]) -> [f32; 4] {
    let (width, height) = (i64::from(level.width), i64::from(level.height));
    let texel = |column: i64, row: i64| {
        let column = wrapped(column, width, sampler.wrap_s);
        let row = wrapped(row, height, sampler.wrap_t);
        let at = usize::try_from((row * width + column) * 4).expect("a wrapped index is in range");
        [0, 1, 2, 3].map(|c| f32::from(level.rgba[at + c]) / 255.0)
    };
    #[allow(
        clippy::cast_precision_loss,
        reason = "a texture's side is far inside f32's whole numbers"
    )]
    let (x, y) = (u * width as f32, v * height as f32);
    match filter {
        Filter::Nearest => texel(whole(x.floor()), whole(y.floor())),
        Filter::Linear => {
            let (left, top) = ((x - 0.5).floor(), (y - 0.5).floor());
            let (column, row) = (whole(left), whole(top));
            let (across, down) = (x - 0.5 - left, y - 0.5 - top);
            let upper = mix(texel(column, row), texel(column + 1, row), across);
            let lower = mix(texel(column, row + 1), texel(column + 1, row + 1), across);
            mix(upper, lower, down)
        }
    }
}

/// Texel index `at` of a side of `size` texels, wrapped by `wrap` into 0
/// to `size - 1`.
fn wrapped(at: i64, size: i64, wrap: Wrap) -> i64 {
    match wrap {
        Wrap::Repeat => at.rem_euclid(size),
        Wrap::Clamp => at.clamp(0, size - 1),
        Wrap::Mirror => {
            let within = at.rem_euclid(2 * size);
            if within >= size {
                2 * size - 1 - within
            } else {
                within
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The texel a test texture holds for `shade`.
    fn texel(shade: u8) -> [u8; 4] {
        [shade, shade / 2, 255 - shade, 255 - shade / 4]
    }

    /// What [`sample`] gives for the texel of `shade`.
    fn shown(shade: u8) -> [f32; 4] {
        texel(shade).map(|c| f32::from(c) / 255.0)
    }

    /// A texture `width` wide of `shades`, row by row.
    fn texture(width: u32, shades: &[u8]) -> ModelTexture {
        let rgba = shades.iter().flat_map(|s| texel(*s)).collect();
        ModelTexture::new(width, shades.len() as u32 / width, rgba).expect("whole rows")
    }

    /// A row of four texels.
    const ROW: [u8; 4] = [0, 60, 120, 240];

    fn row() -> ModelTexture {
        texture(4, &ROW)
    }

    fn sampler(wrap: Wrap, filter: Filter) -> ModelSampler {
        ModelSampler {
            wrap_s: wrap,
            wrap_t: wrap,
            mag: filter,
            min: filter,
            mip: Mip::None,
        }
    }

    fn close(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(&b).all(|(p, q)| (p - q).abs() < 1e-5)
    }

    /// `a` and `b` blended, `t` of the way to `b`.
    fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
        [0, 1, 2, 3].map(|c| a[c] + (b[c] - a[c]) * t)
    }

    #[track_caller]
    fn assert_sample(got: [f32; 4], expected: [f32; 4]) {
        assert!(close(got, expected), "{got:?}, not {expected:?}");
    }

    #[test]
    fn the_default_repeats_and_is_trilinear() {
        assert_eq!(
            ModelSampler::default(),
            ModelSampler {
                wrap_s: Wrap::Repeat,
                wrap_t: Wrap::Repeat,
                mag: Filter::Linear,
                min: Filter::Linear,
                mip: Mip::Linear,
            }
        );
        assert_eq!(ModelSampler::DEFAULT, ModelSampler::default());
    }

    #[test]
    fn nearest_reads_the_texel_a_coordinate_falls_in() {
        let nearest = sampler(Wrap::Repeat, Filter::Nearest);
        for (u, shade) in [(0.0, 0), (0.24, 0), (0.25, 60), (0.6, 120), (0.99, 240)] {
            assert_sample(sample(&row(), nearest, [u, 0.5], 0.0), shown(shade));
        }
        // Down a column, by the height: 2 wide and 3 tall.
        let tall = texture(2, &[10, 20, 30, 40, 50, 60]);
        assert_sample(sample(&tall, nearest, [0.75, 0.1], 0.0), shown(20));
        assert_sample(sample(&tall, nearest, [0.25, 0.4], 0.0), shown(30));
        assert_sample(sample(&tall, nearest, [0.75, 0.9], 0.0), shown(60));
    }

    #[test]
    fn linear_is_a_texel_at_its_centre_and_the_mean_midway_between_two() {
        let linear = sampler(Wrap::Repeat, Filter::Linear);
        assert_sample(sample(&row(), linear, [0.375, 0.5], 0.0), shown(60));
        let midway = mix(shown(60), shown(120), 0.5);
        assert_sample(sample(&row(), linear, [0.5, 0.5], 0.0), midway);
        // A quarter of a texel past a centre: a quarter of the next.
        let quarter = mix(shown(60), shown(120), 0.25);
        assert_sample(sample(&row(), linear, [0.4375, 0.5], 0.0), quarter);
        // Down a column the same, by the height.
        let column = texture(1, &ROW);
        assert_sample(sample(&column, linear, [0.5, 0.375], 0.0), shown(60));
        let down = mix(shown(60), shown(120), 0.25);
        assert_sample(sample(&column, linear, [0.5, 0.4375], 0.0), down);
        // Where four texels meet, their mean.
        let four = texture(2, &[0, 80, 160, 240]);
        let top = mix(shown(0), shown(80), 0.5);
        let bottom = mix(shown(160), shown(240), 0.5);
        assert_sample(
            sample(&four, linear, [0.5, 0.5], 0.0),
            mix(top, bottom, 0.5),
        );
        // Off the middle, each axis by its own fraction.
        let top = mix(shown(0), shown(80), 0.25);
        let bottom = mix(shown(160), shown(240), 0.25);
        assert_sample(
            sample(&four, linear, [0.375, 0.625], 0.0),
            mix(top, bottom, 0.75),
        );
    }

    #[test]
    fn past_the_image_repeat_reads_it_again_clamp_its_edge_and_mirror_it_reversed() {
        let at = |wrap, u| sample(&row(), sampler(wrap, Filter::Nearest), [u, 0.5], 0.0);
        // One turn past: the first texel's place, then the last's.
        assert_sample(at(Wrap::Repeat, 1.125), shown(0));
        assert_sample(at(Wrap::Repeat, 1.875), shown(240));
        assert_sample(at(Wrap::Clamp, 1.125), shown(240));
        assert_sample(at(Wrap::Clamp, 1.875), shown(240));
        assert_sample(at(Wrap::Mirror, 1.125), shown(240));
        assert_sample(at(Wrap::Mirror, 1.375), shown(120));
        assert_sample(at(Wrap::Mirror, 1.875), shown(0));
        // Exactly at the edge, the next image starts.
        assert_sample(at(Wrap::Repeat, 1.0), shown(0));
        assert_sample(at(Wrap::Clamp, 1.0), shown(240));
        assert_sample(at(Wrap::Mirror, 1.0), shown(240));
        // The second mirror is the image the right way round.
        assert_sample(at(Wrap::Mirror, 2.125), shown(0));
        assert_sample(at(Wrap::Mirror, 2.875), shown(240));
        // Before the image.
        assert_sample(at(Wrap::Repeat, -0.125), shown(240));
        assert_sample(at(Wrap::Repeat, -0.875), shown(0));
        assert_sample(at(Wrap::Clamp, -0.125), shown(0));
        assert_sample(at(Wrap::Clamp, -3.0), shown(0));
        assert_sample(at(Wrap::Mirror, -0.125), shown(0));
        assert_sample(at(Wrap::Mirror, -0.875), shown(240));
        assert_sample(at(Wrap::Mirror, -1.125), shown(240));
        // Inside it, every wrap reads the same.
        for wrap in [Wrap::Repeat, Wrap::Clamp, Wrap::Mirror] {
            assert_sample(at(wrap, 0.375), shown(60));
        }
    }

    #[test]
    fn each_axis_wraps_by_its_own_mode() {
        // 2 wide and 3 tall; (1.75, 1.2) is column 3 and row 3, one past
        // each edge and more.
        let tall = texture(2, &[10, 20, 30, 40, 50, 60]);
        let at = |wrap_s, wrap_t| {
            let sampler = ModelSampler {
                wrap_s,
                wrap_t,
                ..sampler(Wrap::Repeat, Filter::Nearest)
            };
            sample(&tall, sampler, [1.75, 1.2], 0.0)
        };
        assert_sample(at(Wrap::Repeat, Wrap::Repeat), shown(20));
        assert_sample(at(Wrap::Clamp, Wrap::Mirror), shown(60));
        assert_sample(at(Wrap::Mirror, Wrap::Clamp), shown(50));
        assert_sample(at(Wrap::Clamp, Wrap::Repeat), shown(20));
        assert_sample(at(Wrap::Mirror, Wrap::Repeat), shown(10));
        assert_sample(at(Wrap::Repeat, Wrap::Clamp), shown(60));
    }

    #[test]
    fn linear_across_an_edge_blends_what_the_wrap_puts_there() {
        let at = |wrap, u| sample(&row(), sampler(wrap, Filter::Linear), [u, 0.5], 0.0);
        // On the seam: half the last texel and half the first, wrapped
        // round; the edge texel with itself when clamped or mirrored.
        let seam = mix(shown(240), shown(0), 0.5);
        assert_sample(at(Wrap::Repeat, 0.0), seam);
        assert_sample(at(Wrap::Repeat, 1.0), seam);
        assert_sample(at(Wrap::Clamp, 0.0), shown(0));
        assert_sample(at(Wrap::Clamp, 1.0), shown(240));
        assert_sample(at(Wrap::Mirror, 0.0), shown(0));
        assert_sample(at(Wrap::Mirror, 1.0), shown(240));
        // A two-texel texture, 1/18 of a texel past the seam: 5/9 of the
        // first texel and 4/9 of the last.
        let pair = texture(2, &[40, 200]);
        let past = sample(
            &pair,
            sampler(Wrap::Repeat, Filter::Linear),
            [1.0 / 36.0, 0.5],
            0.0,
        );
        assert_sample(past, mix(shown(200), shown(40), 5.0 / 9.0));
    }

    /// A 4 by 4 texture in columns of 0, 80, 160 and 240: its second
    /// level is columns of 40 and 200, and its last one texel of 120.
    fn columns() -> ModelTexture {
        let texture = texture(4, &[0, 80, 160, 240].repeat(4));
        let shades: Vec<Vec<u8>> = texture
            .levels()
            .iter()
            .map(|level| level.rgba.as_chunks::<4>().0.iter().map(|t| t[0]).collect())
            .collect();
        assert_eq!(shades[1], [40, 200, 40, 200]);
        assert_eq!(shades[2], [120]);
        texture
    }

    /// The first channel of `columns` sampled at the first column's
    /// middle, as a byte.
    fn red(sampler: ModelSampler, lod: f32) -> f32 {
        sample(&columns(), sampler, [0.125, 0.125], lod)[0] * 255.0
    }

    #[track_caller]
    fn assert_red(sampler: ModelSampler, lod: f32, expected: f32) {
        let got = red(sampler, lod);
        assert!(
            (got - expected).abs() < 1e-3,
            "lod {lod}: {got}, not {expected}"
        );
    }

    #[test]
    fn magnified_it_reads_by_the_mag_filter_and_minified_by_the_min() {
        let sharp_near = ModelSampler {
            mag: Filter::Nearest,
            min: Filter::Linear,
            ..sampler(Wrap::Repeat, Filter::Nearest)
        };
        let sharp_far = ModelSampler {
            mag: Filter::Linear,
            min: Filter::Nearest,
            ..sharp_near
        };
        // Midway between the first two texels of the row.
        let at = |sampler, lod| sample(&row(), sampler, [0.25, 0.5], lod);
        let blend = mix(shown(0), shown(60), 0.5);
        for lod in [-2.0, 0.0] {
            assert_sample(at(sharp_near, lod), shown(60));
            assert_sample(at(sharp_far, lod), blend);
        }
        for lod in [0.01, 0.5, 3.0] {
            assert_sample(at(sharp_near, lod), blend);
            assert_sample(at(sharp_far, lod), shown(60));
        }
    }

    #[test]
    fn without_mips_every_level_of_detail_reads_the_whole_image() {
        // The second column's middle: 80 in the whole image, 40 in the
        // level below and 120 in the last.
        let none = sampler(Wrap::Repeat, Filter::Nearest);
        for lod in [-1.0, 0.0, 0.5, 1.0, 2.0, 9.0] {
            let got = sample(&columns(), none, [0.375, 0.125], lod)[0] * 255.0;
            assert!((got - 80.0).abs() < 1e-3, "lod {lod}: {got}");
        }
    }

    #[test]
    fn the_nearest_mip_is_the_level_the_detail_rounds_to() {
        let nearest = ModelSampler {
            mip: Mip::Nearest,
            ..sampler(Wrap::Repeat, Filter::Nearest)
        };
        for (lod, expected) in [
            (0.0, 0.0),
            (0.4, 0.0),
            (0.5, 0.0),
            (0.6, 40.0),
            (1.0, 40.0),
            (1.5, 40.0),
            (1.6, 120.0),
            (2.0, 120.0),
            // Past the chain, its last level.
            (2.6, 120.0),
            (9.0, 120.0),
        ] {
            assert_red(nearest, lod, expected);
        }
    }

    #[test]
    fn linear_mips_blend_the_two_levels_either_side() {
        let linear = ModelSampler {
            mip: Mip::Linear,
            ..sampler(Wrap::Repeat, Filter::Nearest)
        };
        for (lod, expected) in [
            (0.0, 0.0),
            (0.25, 10.0),
            (0.5, 20.0),
            (1.0, 40.0),
            (1.25, 60.0),
            (1.75, 100.0),
            (2.0, 120.0),
            // Past the chain, its last level.
            (2.5, 120.0),
            (9.0, 120.0),
        ] {
            assert_red(linear, lod, expected);
        }
    }

    #[test]
    fn a_lower_level_is_read_by_its_own_size_through_the_min_filter() {
        // The second level is 2 by 2: u = 0.5 is midway between its two
        // columns, and u = 0.3 is in its first.
        let at = |filter, mip, u| {
            let sampler = ModelSampler {
                mip,
                ..sampler(Wrap::Clamp, filter)
            };
            sample(&columns(), sampler, [u, 0.5], 1.0)[0] * 255.0
        };
        assert!((at(Filter::Nearest, Mip::Nearest, 0.3) - 40.0).abs() < 1e-3);
        assert!((at(Filter::Nearest, Mip::Nearest, 0.6) - 200.0).abs() < 1e-3);
        assert!((at(Filter::Linear, Mip::Nearest, 0.5) - 120.0).abs() < 1e-3);
        assert!((at(Filter::Linear, Mip::Linear, 0.5) - 120.0).abs() < 1e-3);
        // Clamped, the level's edge texel alone.
        assert!((at(Filter::Linear, Mip::Nearest, 0.0) - 40.0).abs() < 1e-3);
    }

    #[test]
    fn a_one_texel_texture_is_that_texel_everywhere() {
        let one = texture(1, &[90]);
        for wrap in [Wrap::Repeat, Wrap::Clamp, Wrap::Mirror] {
            for filter in [Filter::Nearest, Filter::Linear] {
                for mip in [Mip::None, Mip::Nearest, Mip::Linear] {
                    let sampler = ModelSampler {
                        mip,
                        ..sampler(wrap, filter)
                    };
                    for lod in [-1.0, 0.0, 0.5, 4.0] {
                        let got = sample(&one, sampler, [-2.3, 7.9], lod);
                        assert_sample(got, shown(90));
                    }
                }
            }
        }
    }
}
