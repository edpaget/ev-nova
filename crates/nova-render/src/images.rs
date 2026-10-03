//! The image source port: decoded frames by resource.

use std::rc::Rc;

use nova_data::graphics::{GraphicsError, Image};
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
}

/// A borrowed source is a source.
impl<S: ImageSource + ?Sized> ImageSource for &S {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        (**self).frames(kind, id)
    }
}

/// A shared source is a source, so a screen and the renderer can read the
/// same game data.
impl<S: ImageSource + ?Sized> ImageSource for Rc<S> {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        (**self).frames(kind, id)
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

    #[test]
    fn a_shared_source_is_a_source() {
        let shared = Rc::new(Widths);
        assert_eq!(first_width(Rc::clone(&shared), 3), Ok(3));
        assert_eq!(first_width(shared, -1), Err(ImageError::Missing));
    }
}
