//! A decoded image with the depth its pixels were stored at.
//!
//! QuickDraw merges the pixels of a picture it shrinks by the picture's
//! own depth: 5-bit channels for a 16-bit picture, 8-bit ones for a
//! 32-bit picture, and the largest colour-table index for an indexed one.
//! [`Image`] holds RGBA8 colours only, so a [`DeepImage`] carries the
//! depth, and an indexed picture's indices, beside it.

use std::fmt;

use super::image::Image;

/// How a decoded picture's pixels were stored, which decides how
/// QuickDraw merges them when it shrinks the picture.
#[derive(Clone, PartialEq, Eq)]
pub enum PixelDepth {
    /// 16-bit `xRRRRRGGGGGBBBBB` (a `DirectBits` pixel map of
    /// `pixelSize` 16). Each 8-bit channel of the image widens a 5-bit
    /// one by [`widen_five`], so its 5-bit value is the channel `>> 3`.
    Direct16,
    /// 32-bit (a `DirectBits` pixel map of `pixelSize` 32), and the depth
    /// assumed for an image whose storage isn't known.
    Direct32,
    /// Indexed: a 1, 2, 4 or 8-bit `PixMap`, or a 1-bit `BitMap` (0
    /// white, 1 black). The original stores every one in an 8-bit image
    /// of raw pixel values.
    Indexed {
        /// Each pixel's colour-table index, row-major, one per pixel; 0
        /// where no opcode drew.
        indices: Vec<u8>,
    },
}

/// Names the variant and, for [`PixelDepth::Indexed`], the number of
/// indices only, so failed assertions on big images stay readable.
impl fmt::Debug for PixelDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Direct16 => f.write_str("Direct16"),
            Self::Direct32 => f.write_str("Direct32"),
            Self::Indexed { indices } => f
                .debug_struct("Indexed")
                .field("len", &indices.len())
                .finish(),
        }
    }
}

/// A decoded image and the depth it was stored at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeepImage {
    image: Image,
    depth: PixelDepth,
}

impl DeepImage {
    /// Pairs `image` with `depth`, or `None` when an
    /// [`PixelDepth::Indexed`] depth doesn't hold exactly one index per
    /// pixel.
    #[must_use]
    pub fn new(image: Image, depth: PixelDepth) -> Option<Self> {
        if let PixelDepth::Indexed { indices } = &depth {
            let pixels = u128::from(image.width()) * u128::from(image.height());
            if u128::try_from(indices.len()) != Ok(pixels) {
                return None;
            }
        }
        Some(Self { image, depth })
    }

    /// The same depth (and indices) with `image`'s colours, or `None`
    /// unless `image` is the same size. A mask sets a picture's alpha
    /// this way without changing how it was stored.
    #[must_use]
    pub fn with_image(self, image: Image) -> Option<Self> {
        let same = image.width() == self.image.width() && image.height() == self.image.height();
        same.then_some(Self {
            image,
            depth: self.depth,
        })
    }

    /// The colours.
    #[must_use]
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// The depth the pixels were stored at.
    #[must_use]
    pub fn depth(&self) -> &PixelDepth {
        &self.depth
    }

    /// The colours, by value.
    #[must_use]
    pub fn into_image(self) -> Image {
        self.image
    }
}

/// Widens a 5-bit channel to 8 bits as a 16-bit picture decodes: the five
/// bits, then their top three again (`(five << 3) | (five >> 2)`, as
/// `AmbrosiaTools`' `__PICT_HandleBits` does at 0x55b77). Bits above the
/// fifth are ignored.
#[must_use]
pub fn widen_five(five: u8) -> u8 {
    let five = five & 0x1F;
    // The two halves share no bits, so `+` is the same as `|`.
    (five << 3) + (five >> 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::color::rgb555;

    fn image(width: u32, height: u32) -> Image {
        Image::transparent(width, height)
    }

    #[test]
    fn direct_depths_fit_any_image() {
        for depth in [PixelDepth::Direct16, PixelDepth::Direct32] {
            let deep = DeepImage::new(image(3, 2), depth.clone()).expect("fits");
            assert_eq!(deep.depth(), &depth);
            assert_eq!(deep.image(), &image(3, 2));
            assert_eq!(deep.into_image(), image(3, 2));
        }
    }

    #[test]
    fn indexed_depth_holds_one_index_per_pixel() {
        let fits = PixelDepth::Indexed {
            indices: vec![1, 2, 3, 4, 5, 6],
        };
        let deep = DeepImage::new(image(3, 2), fits.clone()).expect("fits");
        assert_eq!(deep.depth(), &fits);
        for len in [0, 5, 7] {
            let wrong = PixelDepth::Indexed {
                indices: vec![0; len],
            };
            assert_eq!(DeepImage::new(image(3, 2), wrong), None, "{len} indices");
        }
    }

    #[test]
    fn with_image_keeps_the_depth_of_a_same_sized_image() {
        let depth = PixelDepth::Indexed {
            indices: vec![7, 8],
        };
        let deep = DeepImage::new(image(2, 1), depth.clone()).expect("fits");
        let opaque = Image::from_rgba(2, 1, vec![9; 8]).expect("2 x 1");
        let swapped = deep.clone().with_image(opaque.clone()).expect("same size");
        assert_eq!(swapped.image(), &opaque);
        assert_eq!(swapped.depth(), &depth);
        assert_eq!(deep.clone().with_image(image(1, 2)), None);
        assert_eq!(deep.with_image(image(2, 2)), None);
    }

    #[test]
    fn debug_counts_indices_without_listing_them() {
        let depth = PixelDepth::Indexed {
            indices: vec![200; 3],
        };
        assert_eq!(format!("{depth:?}"), "Indexed { len: 3 }");
        assert_eq!(format!("{:?}", PixelDepth::Direct16), "Direct16");
        assert_eq!(format!("{:?}", PixelDepth::Direct32), "Direct32");
    }

    #[test]
    fn widen_five_is_the_16_bit_decoders_rule() {
        for five in 0..32u16 {
            let red = rgb555(five << 10)[0];
            assert_eq!(widen_five(five as u8), red, "{five}");
        }
        assert_eq!(widen_five(0x20 | 3), widen_five(3), "bits above five");
        assert_eq!(widen_five(0xFF), 255);
    }
}
