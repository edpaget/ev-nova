//! Which image a `spïn`-backed or `shän`-backed sheet's frames are drawn
//! from.
//!
//! A `spïn` names its sheet by `SpritesID`, and each of a `shän`'s images
//! by its image ID: an `rlëD` when there is one, otherwise a `PICT` pair
//! the original cuts into frames (stellar-fidelity phase 4, rendering
//! phase 20). An `rlëD` is keyed by its own ID, as every sprite always was;
//! a `PICT` pair has no `rlëD` ID, so it is keyed by its `spïn`
//! ([`ImageKind::SpinSheet`]) or by its ship and image
//! ([`ImageKind::ShipSheet`]).

use nova_data::{SheetKind, ShipImage};

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
    /// The frames `shïp` `ship`'s `image` resolves to: its `PICT` pair,
    /// cut.
    Ship {
        /// The ship's ID.
        ship: i16,
        /// Which of its images.
        image: ShipImage,
    },
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

    /// The image of `shïp` `ship`'s `image`, resolved from image ID
    /// `image_id`, read as `kind` says.
    #[must_use]
    pub const fn of_ship(kind: SheetKind, ship: i16, image: ShipImage, image_id: i16) -> Self {
        match kind {
            SheetKind::Rled => Self::Rled(image_id),
            SheetKind::PictPair { .. } => Self::Ship { ship, image },
        }
    }

    /// The image key of frame `frame`.
    #[must_use]
    pub const fn frame(self, frame: u16) -> ImageKey {
        match self {
            Self::Rled(id) => ImageKey::sprite(id, frame),
            Self::Spin(id) => ImageKey::spin_sheet(id, frame),
            Self::Ship { ship, image } => ImageKey::ship_sheet(ship, image, frame),
        }
    }

    /// Its `rlëD`'s ID, when it is one: only an `rlëD` holds solid runs
    /// to overlay (see `flight::solid_runs`).
    #[must_use]
    pub const fn rled(self) -> Option<i16> {
        match self {
            Self::Rled(id) => Some(id),
            Self::Spin(_) | Self::Ship { .. } => None,
        }
    }

    /// The image key of frame `frame` as the original draws it with its
    /// box cut by `(top, left)` ([`ImageKind::RledClipped`]): only an
    /// `rlëD` has one. A `PICT` pair's cut is a 16-bit RLE with no quirk
    /// at the edge.
    #[must_use]
    pub const fn clipped_frame(self, frame: u16, cut: (u16, u16)) -> Option<ImageKey> {
        match self {
            Self::Rled(id) => Some(ImageKey::clipped_sprite(id, frame, cut)),
            Self::Spin(_) | Self::Ship { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_data::ShipImage;

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

    #[test]
    fn a_ships_sheet_is_its_rled_or_its_ship_image_by_its_kind() {
        assert_eq!(
            SheetImage::of_ship(SheetKind::Rled, 128, ShipImage::Glow, 1100),
            SheetImage::Rled(1100)
        );
        let pair = SheetImage::of_ship(
            SheetKind::PictPair { mask_id: 2100 },
            128,
            ShipImage::Glow,
            2000,
        );
        assert_eq!(
            pair,
            SheetImage::Ship {
                ship: 128,
                image: ShipImage::Glow
            }
        );
        assert_eq!(pair.frame(2), ImageKey::ship_sheet(128, ShipImage::Glow, 2));
        assert_eq!(pair.clipped_frame(2, (1, 2)), None);
    }

    #[test]
    fn only_an_rled_sheet_has_an_rled() {
        assert_eq!(SheetImage::Rled(1930).rled(), Some(1930));
        assert_eq!(SheetImage::Spin(1003).rled(), None);
        let ship = SheetImage::Ship {
            ship: 128,
            image: ShipImage::Base,
        };
        assert_eq!(ship.rled(), None);
    }

    #[test]
    fn only_an_rled_sheet_has_a_clipped_frame() {
        assert_eq!(
            SheetImage::Rled(1930).clipped_frame(3, (1, 2)),
            Some(ImageKey::clipped_sprite(1930, 3, (1, 2)))
        );
        assert_eq!(SheetImage::Spin(1003).clipped_frame(3, (1, 2)), None);
    }
}
