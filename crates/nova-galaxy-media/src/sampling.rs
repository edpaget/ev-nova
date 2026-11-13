//! How a glTF material's base colour samples its texture, as the file
//! says it ([`TextureRef`]): which image, through which sampler
//! ([`Sampler`]: wrap and filter modes), by which `TEXCOORD_n` set, and
//! under which `KHR_texture_transform` ([`Transform`]).
//!
//! The scene reader ([`scene`](crate::scene)) fills a [`TextureRef`] for
//! each material, and resolves the set and the transform as it reads each
//! primitive: [`Primitive::uvs`](crate::scene::Primitive::uvs) is the set
//! the base colour names with the transform already applied, so nobody
//! applies either again. The sampler is handed on for whoever draws.
//!
//! # What a file leaves out
//!
//! - No sampler, or a sampler with no `magFilter` or `minFilter`: glTF
//!   leaves the filter to the reader, and this one takes linear
//!   magnification, linear minification and a linear blend between mip
//!   levels ([`Sampler::DEFAULT`]).
//! - No `wrapS` or `wrapT`: repeat, glTF's default.
//! - No `texCoord`: set 0. A transform's own `texCoord` overrides the
//!   texture's.
//! - A transform's missing `offset`, `rotation` or `scale`: none, none
//!   and 1.
//!
//! A wrap or filter code that isn't one of glTF's, and a transform whose
//! JSON doesn't fit the extension's schema (an `offset` that isn't two
//! numbers, say), fail the `gltf` crate's reading of the file: the model
//! is unreadable, as with any other malformed field. Anisotropy is not in
//! core glTF's sampler and is not read.
//!
//! # The transform's rotation
//!
//! [`Transform::apply`] scales, then rotates, then offsets, as the
//! extension's `translation * rotation * scale` says. The rotation's sign
//! is taken from what the extension shows rather than from its GLSL
//! listing, which disagree (Khronos `KHR_texture_transform` README and
//! the `TextureTransformTest` sample asset, both read 2026-11-12):
//!
//! - the README's text: "Rotate the UVs by this many radians
//!   counter-clockwise around the origin. This is equivalent to a similar
//!   rotation of the image clockwise", with (0, 0) the image's upper left;
//! - its worked example: offset `[0, 1]`, rotation π/2, scale
//!   `[0.5, 0.5]` "utilizes only the lower left quadrant of the source
//!   image, rotated clockwise 90°";
//! - the sample asset's rotation quad, whose "correct" marker sits where
//!   a rotation of 0.3927 moves the arrow's tip.
//!
//! All three give `u' = cos·u + sin·v`, `v' = −sin·u + cos·v`. The
//! README's GLSL `mat3(cos, sin, 0, −sin, cos, 0, …)`, read column-major
//! as GLSL does, gives the opposite sine: it sends the worked example
//! outside the image and the sample asset's arrow to its "error" marker.
//! This module's tests pin the example's and the asset's numbers.

/// How a sampler treats a coordinate outside 0 to 1, along one axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wrap {
    /// The image again (`REPEAT`, 10497): glTF's default.
    Repeat,
    /// The image again, turned over each time (`MIRRORED_REPEAT`, 33648).
    Mirror,
    /// The edge texel (`CLAMP_TO_EDGE`, 33071).
    Clamp,
}

/// How a sampler reads a level of the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    /// The nearest texel (`NEAREST`, 9728).
    Nearest,
    /// The four nearest texels, blended (`LINEAR`, 9729).
    Linear,
}

/// Which mip levels a sampler reads when the image is drawn smaller than
/// it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mip {
    /// The whole image only (`NEAREST` or `LINEAR` minification).
    None,
    /// The nearest level (`*_MIPMAP_NEAREST`).
    Nearest,
    /// The two nearest levels, blended (`*_MIPMAP_LINEAR`).
    Linear,
}

/// A texture's sampler, as the file gives it (see the
/// [module docs](self) for what a file leaves out).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sampler {
    /// The wrap along u (`wrapS`).
    pub wrap_s: Wrap,
    /// The wrap along v (`wrapT`).
    pub wrap_t: Wrap,
    /// The filter when the image is drawn larger than it is
    /// (`magFilter`).
    pub mag: Filter,
    /// The filter within a level when it is drawn smaller
    /// (`minFilter`).
    pub min: Filter,
    /// The mip levels read when it is drawn smaller (`minFilter`).
    pub mip: Mip,
}

impl Sampler {
    /// What a texture with no sampler takes: repeating both ways, linear
    /// both ways, blended between mip levels.
    pub const DEFAULT: Sampler = Sampler {
        wrap_s: Wrap::Repeat,
        wrap_t: Wrap::Repeat,
        mag: Filter::Linear,
        min: Filter::Linear,
        mip: Mip::Linear,
    };
}

impl Default for Sampler {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A `KHR_texture_transform`: what a texture does to the coordinates it
/// is sampled by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Added last, in units of the image's size.
    pub offset: [f32; 2],
    /// The turn about the origin (the image's upper left), in radians: it
    /// turns the image clockwise as drawn.
    pub rotation: f32,
    /// Multiplied first, u and v apart.
    pub scale: [f32; 2],
}

impl Transform {
    /// The transform that changes nothing: the extension's defaults.
    pub const IDENTITY: Transform = Transform {
        offset: [0.0; 2],
        rotation: 0.0,
        scale: [1.0; 2],
    };

    /// `uv` transformed: scaled, then turned, then offset (see the
    /// [module docs](self) for the turn's sign).
    #[must_use]
    pub fn apply(&self, [u, v]: [f32; 2]) -> [f32; 2] {
        let (x, y) = (u * self.scale[0], v * self.scale[1]);
        let (sin, cos) = self.rotation.sin_cos();
        [
            self.offset[0] + cos * x + sin * y,
            self.offset[1] - sin * x + cos * y,
        ]
    }
}

/// The texture a material's base colour takes, as read. A texture with no
/// image (no `source`) reads as none: its material has no
/// [`TextureRef`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureRef {
    /// The image, as an index into
    /// [`Scene::images`](crate::scene::Scene::images).
    pub image: usize,
    /// Its sampler.
    pub sampler: Sampler,
    /// The `TEXCOORD_n` set it is sampled by: the transform's `texCoord`
    /// if it gives one, else the texture's. Already resolved into
    /// [`Primitive::uvs`](crate::scene::Primitive::uvs).
    pub tex_coord: u32,
    /// Its `KHR_texture_transform`, if it has one. Already applied to
    /// [`Primitive::uvs`](crate::scene::Primitive::uvs).
    pub transform: Option<Transform>,
}

impl TextureRef {
    /// Image `at`, with nothing else said: the default sampler, set 0, no
    /// transform.
    #[must_use]
    pub fn image(at: usize) -> Self {
        TextureRef {
            image: at,
            sampler: Sampler::DEFAULT,
            tex_coord: 0,
            transform: None,
        }
    }
}

/// `info` as a [`TextureRef`]; `None` when its texture has no image, as
/// one that gives its image only through `KHR_texture_basisu`,
/// `EXT_texture_webp` or `MSFT_texture_dds` has none: its material then
/// draws untextured.
pub(crate) fn texture_ref(info: &gltf::texture::Info<'_>) -> Option<TextureRef> {
    use gltf::texture::{MagFilter, MinFilter, WrappingMode};
    let texture = info.texture();
    let image = texture.source()?.index();
    let sampler = texture.sampler();
    let wrap = |mode| match mode {
        WrappingMode::Repeat => Wrap::Repeat,
        WrappingMode::MirroredRepeat => Wrap::Mirror,
        WrappingMode::ClampToEdge => Wrap::Clamp,
    };
    let mag = match sampler.mag_filter() {
        Some(MagFilter::Nearest) => Filter::Nearest,
        Some(MagFilter::Linear) => Filter::Linear,
        None => Sampler::DEFAULT.mag,
    };
    let (min, mip) = match sampler.min_filter() {
        Some(MinFilter::Nearest) => (Filter::Nearest, Mip::None),
        Some(MinFilter::Linear) => (Filter::Linear, Mip::None),
        Some(MinFilter::NearestMipmapNearest) => (Filter::Nearest, Mip::Nearest),
        Some(MinFilter::LinearMipmapNearest) => (Filter::Linear, Mip::Nearest),
        Some(MinFilter::NearestMipmapLinear) => (Filter::Nearest, Mip::Linear),
        Some(MinFilter::LinearMipmapLinear) => (Filter::Linear, Mip::Linear),
        None => (Sampler::DEFAULT.min, Sampler::DEFAULT.mip),
    };
    let transform = info.texture_transform();
    Some(TextureRef {
        image,
        sampler: Sampler {
            wrap_s: wrap(sampler.wrap_s()),
            wrap_t: wrap(sampler.wrap_t()),
            mag,
            min,
            mip,
        },
        tex_coord: transform
            .as_ref()
            .and_then(gltf::texture::TextureTransform::tex_coord)
            .unwrap_or_else(|| info.tex_coord()),
        transform: transform.map(|t| Transform {
            offset: t.offset(),
            rotation: t.rotation(),
            scale: t.scale(),
        }),
    })
}

/// `uvs` as a primitive is sampled by them: under `transform`, if there
/// is one. `None` when any coordinate isn't finite, whether the file holds
/// a NaN or an infinity (at any vertex, used or not) or the transform
/// leaves one (a scale near the largest float, say): the primitive then
/// has no coordinates, and draws untextured.
pub(crate) fn resolved(uvs: Vec<[f32; 2]>, transform: Option<&Transform>) -> Option<Vec<[f32; 2]>> {
    let moved: Vec<[f32; 2]> = match transform {
        Some(transform) => uvs.iter().map(|uv| transform.apply(*uv)).collect(),
        None => uvs,
    };
    moved
        .iter()
        .flatten()
        .all(|c| c.is_finite())
        .then_some(moved)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_8};

    use super::*;

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-5 && (a[1] - b[1]).abs() < 1e-5
    }

    fn turned(rotation: f32) -> Transform {
        Transform {
            rotation,
            ..Transform::IDENTITY
        }
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "the identity changes no bit")]
    fn the_identity_changes_nothing() {
        for uv in [[0.0, 0.0], [0.25, 0.75], [-3.5, 9.0]] {
            assert_eq!(Transform::IDENTITY.apply(uv), uv);
        }
    }

    #[test]
    fn an_offset_adds_and_a_scale_multiplies_each_axis_apart() {
        let offset = Transform {
            offset: [0.5, -0.25],
            ..Transform::IDENTITY
        };
        assert!(close(offset.apply([0.25, 0.75]), [0.75, 0.5]));
        let scale = Transform {
            scale: [2.0, 3.0],
            ..Transform::IDENTITY
        };
        assert!(close(scale.apply([0.25, 0.75]), [0.5, 2.25]));
        // A zero scale is legal: every vertex samples the offset.
        let point = Transform {
            offset: [0.5, 0.25],
            scale: [0.0; 2],
            ..Transform::IDENTITY
        };
        assert!(close(point.apply([7.0, -3.0]), [0.5, 0.25]));
    }

    #[test]
    fn a_quarter_turn_takes_u_to_minus_v_and_v_to_u() {
        let quarter = turned(FRAC_PI_2);
        assert!(close(quarter.apply([1.0, 0.0]), [0.0, -1.0]));
        assert!(close(quarter.apply([0.0, 1.0]), [1.0, 0.0]));
        // And back the other way.
        let back = turned(-FRAC_PI_2);
        assert!(close(back.apply([1.0, 0.0]), [0.0, 1.0]));
        assert!(close(back.apply([0.0, 1.0]), [-1.0, 0.0]));
    }

    /// The Khronos `KHR_texture_transform` README's worked example: "This
    /// example utilizes only the lower left quadrant of the source image,
    /// rotated clockwise 90°", with (0, 0) the image's upper left. The
    /// lower left quadrant is u 0 to 0.5, v 0.5 to 1.
    #[test]
    fn the_extension_s_worked_example_reads_the_lower_left_quadrant() {
        let example = Transform {
            offset: [0.0, 1.0],
            // The README writes 1.57079632679.
            rotation: FRAC_PI_2,
            scale: [0.5, 0.5],
        };
        for (uv, expected) in [
            ([0.0, 0.0], [0.0, 1.0]),
            ([1.0, 0.0], [0.0, 0.5]),
            ([0.0, 1.0], [0.5, 1.0]),
            ([1.0, 1.0], [0.5, 0.5]),
        ] {
            let got = example.apply(uv);
            assert!(close(got, expected), "{uv:?}: {got:?}, not {expected:?}");
        }
    }

    /// The Khronos `TextureTransformTest` sample asset
    /// (glTF-Sample-Assets, `Models/TextureTransformTest`). Its quads are a
    /// unit a side with coordinate (0, 0) at their upper left, and its
    /// arrow image's tip is at (0.7778, 0.7778), where the rotation quad's
    /// "not supported" marker sits (node translation (0.2778, −0.2778)
    /// from the quad's middle, y up).
    #[test]
    fn the_khronos_sample_asset_s_arrow_points_at_its_correct_marker() {
        // A marker's coordinates on its quad, from its node's translation.
        let on_quad = |[x, y]: [f32; 2]| [0.5 + x, 0.5 - y];
        let tip = on_quad([0.277_817_46, -0.277_817_46]);
        // The rotation quad: 22.5° (the asset writes 0.39269908169872414).
        // The tip is drawn at the "correct" marker, not at the "error"
        // one, the same turn the other way.
        let rotation = turned(FRAC_PI_8);
        let correct = on_quad([-0.079_048_224, -0.516_267_5]);
        let error = on_quad([0.516_267_5, 0.079_048_224]);
        let got = rotation.apply(correct);
        assert!(close(got, tip), "{got:?}, not {tip:?}");
        assert!(!close(rotation.apply(error), tip));
        // The "all" quad: its marker, placed by hand, is within a
        // twentieth of the tip.
        let all = Transform {
            offset: [-0.2, -0.1],
            rotation: 0.3,
            scale: [1.5, 1.5],
        };
        let [u, v] = all.apply(on_quad([-0.07, -0.25]));
        assert!((u - tip[0]).abs() < 0.05 && (v - tip[1]).abs() < 0.05);
    }

    #[test]
    fn a_coordinate_is_scaled_then_turned_then_offset() {
        let all = Transform {
            offset: [10.0, 20.0],
            rotation: FRAC_PI_2,
            scale: [2.0, 3.0],
        };
        // (1, 1) scales to (2, 3), turns to (3, −2), and lands on
        // (13, 18). Turned before it is scaled it would land on (12, 17);
        // offset before it is turned, on (23, −12).
        let got = all.apply([1.0, 1.0]);
        assert!(close(got, [13.0, 18.0]), "{got:?}");
    }

    #[test]
    fn coordinates_resolve_as_they_are_without_a_transform_and_transformed_with_one() {
        let uvs = vec![[0.0, 0.0], [0.25, 0.75], [2.0, -1.0]];
        assert_eq!(resolved(uvs.clone(), None), Some(uvs.clone()));
        let transform = Transform {
            offset: [1.0, 2.0],
            rotation: 0.0,
            scale: [2.0, 4.0],
        };
        let got = resolved(uvs, Some(&transform)).expect("finite");
        assert_eq!(got.len(), 3);
        assert!(close(got[1], [1.5, 5.0]), "{:?}", got[1]);
        assert!(close(got[2], [5.0, -2.0]), "{:?}", got[2]);
    }

    #[test]
    fn a_transform_that_overflows_leaves_no_coordinates() {
        let huge = Transform {
            scale: [f32::MAX, 1.0],
            ..Transform::IDENTITY
        };
        // One coordinate over is enough, on either axis.
        assert_eq!(resolved(vec![[0.5, 0.5], [4.0, 0.5]], Some(&huge)), None);
        let tall = Transform {
            scale: [1.0, f32::MAX],
            ..Transform::IDENTITY
        };
        assert_eq!(resolved(vec![[0.5, 4.0]], Some(&tall)), None);
        assert!(resolved(vec![[0.5, 0.5]], Some(&huge)).is_some());
    }

    #[test]
    fn coordinates_that_are_not_finite_resolve_to_nothing_with_or_without_a_transform() {
        for transform in [None, Some(&Transform::IDENTITY)] {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                for uv in [[bad, 0.5], [0.5, bad]] {
                    let uvs = vec![[0.0, 0.0], uv, [1.0, 1.0]];
                    assert_eq!(resolved(uvs, transform), None, "{uv:?}, {transform:?}");
                }
            }
            let fine = vec![[0.0, 0.0], [0.25, 0.75]];
            assert_eq!(resolved(fine.clone(), transform), Some(fine));
        }
    }

    #[test]
    fn a_bare_image_reference_says_nothing_else() {
        let bare = TextureRef::image(3);
        assert_eq!(bare.image, 3);
        assert_eq!(bare.sampler, Sampler::default());
        assert_eq!(bare.tex_coord, 0);
        assert_eq!(bare.transform, None);
        assert_eq!(
            Sampler::DEFAULT,
            Sampler {
                wrap_s: Wrap::Repeat,
                wrap_t: Wrap::Repeat,
                mag: Filter::Linear,
                min: Filter::Linear,
                mip: Mip::Linear,
            }
        );
    }
}
