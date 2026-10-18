//! The enlarger port: the image the atlas holds for each decoded frame.
//!
//! This is the load-time scaling site, apart from the per-frame
//! [`Upscaler`](crate::Upscaler). The [`Renderer`](crate::Renderer) hands
//! each frame it decodes to its [`Enlarger`] and places what comes back in
//! the atlas, but draws it at the frame's own (logical) size: quads,
//! positions and clips stay in logical units, and only the texture
//! coordinates see the bigger image. [`NativeDetail`] hands every frame
//! back as it is; [`SuperXbr`](crate::SuperXbr) doubles it.

use std::borrow::Cow;

use nova_data::graphics::Image;
use nova_view::{Enlargement, ImageKind};

use crate::super_xbr::SuperXbr;

/// Turns a decoded frame into the image the atlas holds.
pub trait Enlarger: std::fmt::Debug {
    /// `image`, a frame of a `kind` resource, as the atlas holds it:
    /// enlarged, or `image` itself when this enlarger leaves that kind
    /// alone. It is drawn at `image`'s own (logical) size whatever size it
    /// comes back.
    fn enlarge<'a>(&self, kind: ImageKind, image: &'a Image) -> Cow<'a, Image>;
}

/// No enlargement: the atlas holds each frame as it decodes, as before
/// there was an enlarger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeDetail;

impl Enlarger for NativeDetail {
    /// `image` itself, borrowed, for every kind.
    fn enlarge<'a>(&self, _kind: ImageKind, image: &'a Image) -> Cow<'a, Image> {
        Cow::Borrowed(image)
    }
}

/// The enlarger that does `enlargement`: the load-time half of the
/// player's Scaling (`nova_view::Scaling::sites`).
#[must_use]
pub fn enlarger_for(enlargement: Enlargement) -> Box<dyn Enlarger> {
    match enlargement {
        Enlargement::Native => Box::new(NativeDetail),
        Enlargement::SuperXbr => Box::new(SuperXbr),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use nova_data::graphics::Image;
    use nova_view::{Enlargement, ImageKind};

    use super::*;

    /// Every kind of image resource.
    const KINDS: [ImageKind; 5] = [
        ImageKind::Rled,
        ImageKind::Pict,
        ImageKind::MaskedPict { mask: 7 },
        ImageKind::Pattern,
        ImageKind::Icon,
    ];

    #[test]
    fn each_enlargement_has_its_enlarger() {
        let image = Image::from_rgba(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8]).expect("2x1");
        for kind in KINDS {
            let native = enlarger_for(Enlargement::Native);
            assert!(matches!(native.enlarge(kind, &image), Cow::Borrowed(_)));
            assert_eq!(
                enlarger_for(Enlargement::SuperXbr).enlarge(kind, &image),
                SuperXbr.enlarge(kind, &image),
                "{kind:?}"
            );
        }
        assert_eq!(
            enlarger_for(Enlargement::SuperXbr)
                .enlarge(ImageKind::Rled, &image)
                .width(),
            4
        );
    }

    #[test]
    fn native_hands_every_kind_back_unchanged_and_borrowed() {
        let image = Image::from_rgba(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8]).expect("2x1");
        for kind in KINDS {
            match NativeDetail.enlarge(kind, &image) {
                Cow::Borrowed(same) => assert_eq!(
                    std::ptr::from_ref(same),
                    std::ptr::from_ref(&image),
                    "{kind:?}"
                ),
                Cow::Owned(other) => panic!("{kind:?} copied: {other:?}"),
            }
        }
    }
}
