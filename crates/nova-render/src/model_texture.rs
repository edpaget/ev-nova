//! A model's base-colour texture: decoded pixels with their mip chain
//! ([`ModelTexture`]), as a [`Mesh`](crate::model::Mesh) holds them and
//! the GPU is handed them ([`crate::Gpu::upload_model_texture`]).
//!
//! A texture is made once, when its model is loaded
//! ([`crate::model_source`]): [`ModelTexture::new`] checks the pixels and
//! builds every mip level with [`planet::mips`], the box filter the
//! planets' surfaces take, so nothing is built on a frame and a large
//! texture on a small ship doesn't shimmer. Its texels are sRGB bytes, as
//! the image file holds them, alpha straight.
//!
//! A mesh holds its textures behind shared pointers, and a model painted
//! in two liveries ([`crate::livery::painted`]) shares them between its
//! two meshes.
//!
//! # Which textures the GPU holds
//!
//! The renderer keeps a table of the textures it has uploaded, by
//! pointer (`crate::resident::Resident`, which its models hold): a
//! texture is numbered ([`ModelTextureId`]) and uploaded
//! the first time a mesh that holds its pointer is, and every later mesh
//! holding the same pointer names the same number. Two textures with the
//! same pixels behind two pointers are two uploads: nothing is hashed or
//! compared, at load or on a frame. Textures are not atlas pages, so a
//! new enlarger leaves them where they are, as it leaves meshes. A
//! texture no mesh holds any more stays on the GPU: the port has no call
//! that drops one, as it has none that drops a mesh.

use nova_galaxy_media::planet::Image;

use crate::planet;

/// Which model texture the GPU holds, as the renderer numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelTextureId(pub u32);

impl From<u32> for ModelTextureId {
    fn from(number: u32) -> Self {
        Self(number)
    }
}

/// Why [`ModelTexture::new`] refused a texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ModelTextureError {
    /// A side is zero, so there is nothing to sample.
    #[error("a {width} by {height} texture has no texels")]
    Empty {
        /// Its width in texels.
        width: u32,
        /// Its height in texels.
        height: u32,
    },
    /// The bytes are not four for each texel.
    #[error("{bytes} bytes are not four for each texel of {width} by {height}")]
    Size {
        /// Its width in texels.
        width: u32,
        /// Its height in texels.
        height: u32,
        /// How many bytes it was given.
        bytes: usize,
    },
}

/// A model's texture: its image and every smaller mip level of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelTexture {
    levels: Vec<Image>,
}

impl ModelTexture {
    /// The texture `width` by `height` texels of `rgba`: 8-bit RGBA, row
    /// by row from the top, four bytes a texel. It refuses a zero side,
    /// and bytes that are not four for each texel.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, ModelTextureError> {
        if width == 0 || height == 0 {
            return Err(ModelTextureError::Empty { width, height });
        }
        if u64::from(width) * u64::from(height) * 4 != rgba.len() as u64 {
            return Err(ModelTextureError::Size {
                width,
                height,
                bytes: rgba.len(),
            });
        }
        Ok(Self {
            levels: planet::mips(&Image {
                width,
                height,
                rgba,
            }),
        })
    }

    /// Its mip chain, the whole image first: each level half the one
    /// before on each side (to 1 at least), down to 1 by 1.
    #[must_use]
    pub fn levels(&self) -> &[Image] {
        &self.levels
    }

    /// Its width in texels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.levels[0].width
    }

    /// Its height in texels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.levels[0].height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4 by 2 image: its left half red, its right half blue, the bottom
    /// row at half the alpha.
    fn halves() -> Vec<u8> {
        let row = |alpha| {
            [
                [200, 0, 0, alpha],
                [200, 0, 0, alpha],
                [0, 0, 100, alpha],
                [0, 0, 100, alpha],
            ]
            .concat()
        };
        [row(255), row(127)].concat()
    }

    #[test]
    fn a_texture_holds_its_image_first_then_each_halved_level_down_to_one_texel() {
        let texture = ModelTexture::new(4, 2, halves()).expect("four bytes a texel");
        assert_eq!((texture.width(), texture.height()), (4, 2));
        let sizes: Vec<(u32, u32)> = texture
            .levels()
            .iter()
            .map(|level| (level.width, level.height))
            .collect();
        assert_eq!(sizes, [(4, 2), (2, 1), (1, 1)]);
        assert_eq!(texture.levels()[0].rgba, halves());
        // Each texel the average of the four above it, alpha too.
        assert_eq!(texture.levels()[1].rgba, [200, 0, 0, 191, 0, 0, 100, 191]);
        assert_eq!(texture.levels()[2].rgba, [100, 0, 50, 191]);
    }

    #[test]
    fn a_one_texel_texture_is_its_only_level() {
        let texture = ModelTexture::new(1, 1, vec![1, 2, 3, 4]).expect("one texel");
        assert_eq!(texture.levels().len(), 1);
        assert_eq!(texture.levels()[0].rgba, [1, 2, 3, 4]);
    }

    #[test]
    fn a_texture_with_a_zero_side_is_refused() {
        assert_eq!(
            ModelTexture::new(0, 2, Vec::new()),
            Err(ModelTextureError::Empty {
                width: 0,
                height: 2
            })
        );
        assert_eq!(
            ModelTexture::new(2, 0, Vec::new()),
            Err(ModelTextureError::Empty {
                width: 2,
                height: 0
            })
        );
    }

    #[test]
    fn bytes_that_are_not_four_a_texel_are_refused() {
        for bytes in [7, 9, 0, 32] {
            assert_eq!(
                ModelTexture::new(2, 1, vec![0; bytes]),
                Err(ModelTextureError::Size {
                    width: 2,
                    height: 1,
                    bytes,
                }),
                "{bytes} bytes"
            );
        }
        assert!(ModelTexture::new(2, 1, vec![0; 8]).is_ok());
        assert!(ModelTexture::new(3, 5, vec![0; 60]).is_ok());
        // Width times height, not their sum or either alone.
        assert!(ModelTexture::new(3, 5, vec![0; 32]).is_err());
    }

    #[test]
    fn a_refusal_says_what_it_was_given() {
        let empty = ModelTextureError::Empty {
            width: 0,
            height: 2,
        };
        assert_eq!(empty.to_string(), "a 0 by 2 texture has no texels");
        let size = ModelTextureError::Size {
            width: 2,
            height: 1,
            bytes: 7,
        };
        assert_eq!(
            size.to_string(),
            "7 bytes are not four for each texel of 2 by 1"
        );
    }
}
