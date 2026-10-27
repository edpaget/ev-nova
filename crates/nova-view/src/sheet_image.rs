//! Which image a `spïn`-backed sheet's frames are drawn from.
//!
//! A `spïn` names its sheet by `SpritesID`: an `rlëD` when there is one,
//! otherwise a `PICT` pair the original cuts into frames (stellar-fidelity
//! phase 4). An `rlëD` is keyed by its own ID, as every sprite always was;
//! a `PICT` pair has no `rlëD` ID, so it is keyed by its `spïn`
//! ([`ImageKind::SpinSheet`]).

use nova_data::SheetKind;

use crate::image::ImageKey;
#[cfg(doc)]
use crate::image::ImageKind;

/// Where a sheet's frames come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SheetImage {
    /// `rlëD` `id`'s frames.
    Rled(i16),
    /// The frames `spïn` `id` resolves to: its `PICT` pair, cut.
    Spin(i16),
}

impl SheetImage {
    /// The image of a sheet `spïn` `spin_id` resolved to from `SpritesID`
    /// `image_id`, read as `kind` says.
    #[must_use]
    pub const fn of(kind: SheetKind, spin_id: i16, image_id: i16) -> Self {
        match kind {
            SheetKind::Rled => Self::Rled(image_id),
            SheetKind::PictPair { .. } => Self::Spin(spin_id),
        }
    }

    /// The image key of frame `frame`.
    #[must_use]
    pub const fn frame(self, frame: u16) -> ImageKey {
        match self {
            Self::Rled(id) => ImageKey::sprite(id, frame),
            Self::Spin(id) => ImageKey::spin_sheet(id, frame),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::ImageKind;

    #[test]
    fn a_sheet_is_its_rled_or_its_spin_by_its_kind() {
        assert_eq!(
            SheetImage::of(SheetKind::Rled, 1003, 2003),
            SheetImage::Rled(2003)
        );
        assert_eq!(
            SheetImage::of(SheetKind::PictPair { mask_id: 2103 }, 1003, 2003),
            SheetImage::Spin(1003)
        );
    }

    #[test]
    fn an_rled_sheet_is_keyed_by_its_rled() {
        assert_eq!(SheetImage::Rled(1930).frame(3), ImageKey::sprite(1930, 3));
    }

    #[test]
    fn a_pict_pair_sheet_is_keyed_by_its_spin() {
        let key = SheetImage::Spin(1003).frame(1);
        assert_eq!(
            key,
            ImageKey {
                kind: ImageKind::SpinSheet,
                id: 1003,
                frame: 1
            }
        );
        assert_eq!(key, ImageKey::spin_sheet(1003, 1));
        assert_ne!(key, ImageKey::sprite(1003, 1));
    }
}
