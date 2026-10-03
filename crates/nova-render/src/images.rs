//! The image source port: decoded frames by resource.

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
