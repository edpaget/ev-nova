//! A sky decoration's image ([`SkyImage`]): its pixels premultiplied by
//! their alpha, with their mip chain, as the GPU is handed them
//! ([`crate::Gpu::upload_sky_image`]) and as the core samples them
//! ([`super::decoration::lit`]).
//!
//! # Premultiplied, then mipped
//!
//! An image arrives with straight alpha, as a PNG holds it.
//! [`SkyImage::new`] multiplies each texel's colour by its alpha
//! (`c · a / 255`, rounded), and only then builds the mip chain
//! ([`crate::planet::mips`], the box filter). Averaging premultiplied
//! texels is right where averaging straight ones is not: a clear texel
//! is all zero whatever colour it was written with, so it adds no colour
//! to the smaller levels, and a transparent background neither darkens
//! nor fringes an edge at any size. A texel's colour is then the light it
//! adds, and its alpha how much of what is beneath it hides.
//!
//! # One sampler
//!
//! Every sky image is read through [`SKY_SAMPLER`]: held to its edge both
//! ways, linear when drawn larger or smaller, and linear between mips.
//! [`crate::model_sampler::sample`] of that value is the reference, and
//! the wgpu adapter's sampler is its `model_samplers::descriptor` of the
//! same value.

use crate::model_sampler::{Filter, Mip, ModelSampler, Wrap};
use crate::model_texture::{ModelTexture, ModelTextureError};

/// Which sky image the GPU holds, as the renderer numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkyImageId(pub u32);

/// The most texels a sky image read from data may have on its longer
/// side, whatever its art budget allows: 5.6 MB with its mips at this
/// side squared.
pub const SKY_IMAGE_SIDE: u32 = 1024;

/// How every sky image is sampled: held to its edge both ways, and
/// trilinear.
pub const SKY_SAMPLER: ModelSampler = ModelSampler {
    wrap_s: Wrap::Clamp,
    wrap_t: Wrap::Clamp,
    mag: Filter::Linear,
    min: Filter::Linear,
    mip: Mip::Linear,
};

/// A sky decoration's image: premultiplied by its alpha, with every
/// smaller mip level (see the [module docs](self)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkyImage {
    texture: ModelTexture,
}

impl SkyImage {
    /// The image `width` by `height` texels of `rgba`: 8-bit RGBA with
    /// straight alpha, row by row from the top, four bytes a texel. Each
    /// texel is premultiplied, then the mips are built. It refuses a zero
    /// side, and bytes that are not four for each texel, as
    /// [`ModelTexture::new`] does.
    pub fn new(width: u32, height: u32, mut rgba: Vec<u8>) -> Result<Self, ModelTextureError> {
        for texel in rgba.as_chunks_mut::<4>().0 {
            let alpha = u32::from(texel[3]);
            for channel in &mut texel[..3] {
                *channel = premultiplied(*channel, alpha);
            }
        }
        Self::premultiplied(width, height, rgba)
    }

    /// The image `width` by `height` texels of `rgba`, whose texels are
    /// already premultiplied: what the engine bakes for itself. It
    /// refuses what [`SkyImage::new`] refuses.
    pub fn premultiplied(
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<Self, ModelTextureError> {
        Ok(Self {
            texture: ModelTexture::new(width, height, rgba)?,
        })
    }

    /// Its premultiplied texels and their mip chain.
    #[must_use]
    pub fn texture(&self) -> &ModelTexture {
        &self.texture
    }

    /// Its width in texels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    /// Its height in texels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.texture.height()
    }
}

/// `channel` times `alpha` of 255, rounded to the nearest.
fn premultiplied(channel: u8, alpha: u32) -> u8 {
    let product = u32::from(channel) * alpha;
    u8::try_from((product + 127) / 255).expect("at most 255")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texels(image: &SkyImage, level: usize) -> &[u8] {
        &image.texture().levels()[level].rgba
    }

    #[test]
    fn a_half_clear_texel_premultiplies_to_half_its_colour() {
        let image = SkyImage::new(1, 1, vec![255, 0, 0, 128]).expect("one texel");
        assert_eq!(texels(&image, 0), [128, 0, 0, 128]);
        let image = SkyImage::new(1, 1, vec![200, 100, 50, 51]).expect("one texel");
        // A fifth of each: 40, 20, 10.
        assert_eq!(texels(&image, 0), [40, 20, 10, 51]);
    }

    #[test]
    fn an_opaque_texel_is_unchanged_and_a_clear_one_is_all_zero() {
        let image = SkyImage::new(2, 1, vec![9, 200, 255, 255, 200, 10, 77, 0]).expect("two");
        assert_eq!(texels(&image, 0), [9, 200, 255, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn every_product_is_rounded_to_the_nearest() {
        // 255 · 1 / 255 = 1; 127 · 1 / 255 rounds to 0 and 128 · 1 / 255
        // to 1; 1 · 127 / 255 rounds to 0 and 1 · 128 / 255 to 1.
        assert_eq!(premultiplied(255, 1), 1);
        assert_eq!(premultiplied(127, 1), 0);
        assert_eq!(premultiplied(128, 1), 1);
        assert_eq!(premultiplied(1, 127), 0);
        assert_eq!(premultiplied(1, 128), 1);
        assert_eq!(premultiplied(255, 255), 255);
        assert_eq!(premultiplied(254, 254), 253);
        assert_eq!(premultiplied(0, 255), 0);
    }

    #[test]
    fn no_colour_bleeds_from_a_clear_texel_into_a_smaller_level() {
        // Opaque white beside a clear texel written in red.
        let image = SkyImage::new(2, 1, vec![255, 255, 255, 255, 255, 0, 0, 0]).expect("two");
        assert_eq!(image.texture().levels().len(), 2);
        // Half white at half alpha: no red crept in.
        assert_eq!(texels(&image, 1), [128, 128, 128, 128]);
        // Straight alpha would have averaged the red in.
        let straight = ModelTexture::new(2, 1, vec![255, 255, 255, 255, 255, 0, 0, 0]).unwrap();
        assert_eq!(straight.levels()[1].rgba, [255, 128, 128, 128]);
    }

    #[test]
    fn an_image_already_premultiplied_is_kept_as_it_is() {
        let rgba = vec![60, 30, 10, 64];
        let image = SkyImage::premultiplied(1, 1, rgba.clone()).expect("one texel");
        assert_eq!(texels(&image, 0), rgba);
        assert_ne!(
            SkyImage::new(1, 1, rgba.clone()).unwrap(),
            image,
            "new premultiplies again"
        );
    }

    #[test]
    fn it_reports_its_size() {
        let image = SkyImage::new(3, 5, vec![0; 60]).expect("3 by 5");
        assert_eq!((image.width(), image.height()), (3, 5));
        assert_eq!(image.texture().width(), 3);
    }

    #[test]
    fn a_zero_side_and_short_bytes_are_refused_as_a_model_texture_refuses_them() {
        assert_eq!(
            SkyImage::new(0, 2, Vec::new()),
            Err(ModelTextureError::Empty {
                width: 0,
                height: 2
            })
        );
        assert_eq!(
            SkyImage::new(2, 1, vec![0; 7]),
            Err(ModelTextureError::Size {
                width: 2,
                height: 1,
                bytes: 7,
            })
        );
        assert_eq!(
            SkyImage::premultiplied(2, 0, Vec::new()),
            Err(ModelTextureError::Empty {
                width: 2,
                height: 0
            })
        );
        assert_eq!(
            SkyImage::premultiplied(2, 1, vec![0; 9]),
            Err(ModelTextureError::Size {
                width: 2,
                height: 1,
                bytes: 9,
            })
        );
    }

    #[test]
    fn the_sampler_is_clamped_linear_and_trilinear() {
        assert_eq!(
            SKY_SAMPLER,
            ModelSampler {
                wrap_s: Wrap::Clamp,
                wrap_t: Wrap::Clamp,
                mag: Filter::Linear,
                min: Filter::Linear,
                mip: Mip::Linear,
            }
        );
        assert_eq!(SKY_IMAGE_SIDE, 1024);
    }
}
