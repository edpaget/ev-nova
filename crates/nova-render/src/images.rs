//! The image source port: decoded frames by resource.
//!
//! [`ImageSource::frames`] gives the RGBA8 frames the renderer packs and
//! draws. [`ImageSource::deep_frames`] gives the same frames with the
//! depth each was stored at, for QuickDraw's stretch
//! ([`crate::quickdraw_stretch::stretched`]), which merges a shrunk
//! picture's pixels by that depth. A source that doesn't know its depths
//! needn't say: by default its frames are 32-bit, merged on 8-bit
//! channels.

use std::rc::Rc;

use nova_data::graphics::{DeepImage, GraphicsError, Image, PixelDepth};
use nova_view::ImageKind;

use crate::atlas::PackError;

/// Why a draw command's image cannot be shown.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    /// No resource of that kind and ID exists.
    #[error("no such image resource")]
    Missing,
    /// The resource exists but does not decode.
    #[error("the image does not decode: {0}")]
    Decode(#[from] GraphicsError),
    /// The resource has fewer frames than the one asked for.
    #[error("frame {frame} is out of range: the resource has {count}")]
    NoFrame {
        /// The frame asked for.
        frame: u16,
        /// How many frames the resource has.
        count: usize,
    },
    /// The frame cannot be placed in the atlas.
    #[error(transparent)]
    Pack(#[from] PackError),
}

/// Decoded images, in the core's terms.
pub trait ImageSource {
    /// Every frame of resource (`kind`, `id`), in frame order. A picture
    /// has exactly one.
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError>;

    /// Every frame of (`kind`, `id`) with the depth it was stored at, for
    /// QuickDraw's stretch. By default, [`ImageSource::frames`] at
    /// [`PixelDepth::Direct32`], the renderer's RGBA8.
    fn deep_frames(&self, kind: ImageKind, id: i16) -> Result<Vec<DeepImage>, ImageError> {
        Ok(self
            .frames(kind, id)?
            .into_iter()
            .filter_map(|frame| DeepImage::new(frame, PixelDepth::Direct32))
            .collect())
    }
}

/// A borrowed source is a source.
impl<S: ImageSource + ?Sized> ImageSource for &S {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        (**self).frames(kind, id)
    }

    fn deep_frames(&self, kind: ImageKind, id: i16) -> Result<Vec<DeepImage>, ImageError> {
        (**self).deep_frames(kind, id)
    }
}

/// A shared source is a source, so a screen and the renderer can read the
/// same game data.
impl<S: ImageSource + ?Sized> ImageSource for Rc<S> {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        (**self).frames(kind, id)
    }

    fn deep_frames(&self, kind: ImageKind, id: i16) -> Result<Vec<DeepImage>, ImageError> {
        (**self).deep_frames(kind, id)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    /// One frame `id` pixels wide for a positive `id`; `Missing` otherwise.
    struct Widths;

    impl ImageSource for Widths {
        fn frames(&self, _kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
            let width = u32::try_from(id).map_err(|_| ImageError::Missing)?;
            let frame = Image::from_rgba(width, 1, vec![0; width as usize * 4]).expect("frame");
            Ok(vec![frame])
        }
    }

    /// Asks `source` through the trait, as the renderer does.
    fn first_width(source: impl ImageSource, id: i16) -> Result<u32, ImageError> {
        Ok(source.frames(ImageKind::Rled, id)?[0].width())
    }

    /// [`Widths`]'s frames, stored at 16 bits.
    struct Deep;

    impl ImageSource for Deep {
        fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
            Widths.frames(kind, id)
        }

        fn deep_frames(&self, kind: ImageKind, id: i16) -> Result<Vec<DeepImage>, ImageError> {
            Ok(self
                .frames(kind, id)?
                .into_iter()
                .map(|frame| DeepImage::new(frame, PixelDepth::Direct16).expect("direct"))
                .collect())
        }
    }

    /// The first frame's depth, asked through the trait.
    fn first_depth(source: impl ImageSource, id: i16) -> Result<PixelDepth, ImageError> {
        Ok(source.deep_frames(ImageKind::Pict, id)?[0].depth().clone())
    }

    #[test]
    fn a_source_without_depths_gives_its_frames_at_32_bits() {
        let deep = Widths.deep_frames(ImageKind::Pict, 3).expect("frames");
        let frames = Widths.frames(ImageKind::Pict, 3).expect("frames");
        assert_eq!(deep.len(), 1);
        assert_eq!(deep[0].depth(), &PixelDepth::Direct32);
        assert_eq!(deep[0].image(), &frames[0]);
        assert_eq!(
            Widths.deep_frames(ImageKind::Pict, -1),
            Err(ImageError::Missing)
        );
    }

    #[test]
    fn a_borrowed_or_shared_source_keeps_its_depths() {
        assert_eq!(first_depth(Deep, 2), Ok(PixelDepth::Direct16));
        assert_eq!(first_depth(&Deep, 2), Ok(PixelDepth::Direct16));
        assert_eq!(first_depth(Rc::new(Deep), 2), Ok(PixelDepth::Direct16));
    }

    #[test]
    fn a_shared_source_is_a_source() {
        let shared = Rc::new(Widths);
        assert_eq!(first_width(Rc::clone(&shared), 3), Ok(3));
        assert_eq!(first_width(shared, -1), Err(ImageError::Missing));
    }
}
