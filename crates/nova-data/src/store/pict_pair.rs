//! Reading a sprites `PICT` and its mask and cutting them into a sheet:
//! the step a `spïn` ([`GameData::spin_sheet`]) and a `shän`'s images
//! ([`GameData::ship_sprite`]) share when they have no `rlëD`.

use super::GameData;
use crate::graphics::{
    FrameLimit, GraphicsError, Image, PICT, SheetLayout, SheetMask, SpriteSheet, cut_pict_sheet,
    decode_pict,
};

/// What a sheet was read from: a `spïn`'s, or one of a `shän`'s images.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SheetKind {
    /// The `rlëD` named by its image ID (a `spïn`'s `SpritesID`, a
    /// `shän` image's ID).
    Rled,
    /// The sprites `PICT` named by its image ID and the mask `PICT`
    /// `mask_id` (which may be the sprites themselves), cut into frames as
    /// the original does with no `rlëD` ([`cut_pict_sheet`]).
    PictPair {
        /// The mask `PICT`'s ID.
        mask_id: i16,
    },
}

/// The depth a cut `PICT` sheet's tokens are made at: the 16-bit world's,
/// as `_SWCreateRLEData` parses each frame's 16-bit pixels.
pub(super) const PICT_SHEET_DEPTH: u16 = 16;

/// How a pair is cut: each frame's size, the frames it must hold and the
/// layout the sheet is composed in.
#[derive(Clone, Copy, Debug)]
pub(super) struct PairCut {
    /// Each frame's width.
    pub x_size: i16,
    /// Each frame's height.
    pub y_size: i16,
    /// How many frames it must hold.
    pub limit: FrameLimit,
    /// The sheet's layout.
    pub layout: Option<SheetLayout>,
}

/// Why a pair could not be cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PairError {
    /// There is no mask: no ID, or no `PICT` of it.
    NoMask,
    /// A `PICT` failed to decode, or the pair to cut (reported against the
    /// sprites).
    Picture {
        /// The `PICT`'s ID.
        pict_id: i16,
        /// What went wrong.
        source: GraphicsError,
    },
}

impl GameData {
    /// Cuts sprites `PICT` `image_id` (`sprites`, its bytes) and its mask
    /// `mask_id` into a sheet as `cut` says, with the mask's ID: the mask is
    /// the sprites themselves when `mask_id` is `image_id`, else the `PICT`
    /// `mask_id`. The sprites are decoded before the mask is looked for.
    pub(super) fn cut_pict_pair(
        &self,
        image_id: i16,
        sprites: &[u8],
        mask_id: Option<i16>,
        cut: PairCut,
    ) -> Result<(i16, SpriteSheet), PairError> {
        let decode = |pict_id: i16, data: &[u8]| {
            decode_pict(data).map_err(|source| PairError::Picture { pict_id, source })
        };
        let sprites = decode(image_id, sprites)?;
        let mask_id = mask_id.ok_or(PairError::NoMask)?;
        let mask: Option<Image> = if mask_id == image_id {
            None
        } else {
            let found = self.resource(PICT, mask_id).ok_or(PairError::NoMask)?;
            Some(decode(mask_id, found.resource.data())?)
        };
        let mask = mask.as_ref().map_or(SheetMask::Itself, SheetMask::Picture);
        cut_pict_sheet(
            &sprites, mask, cut.x_size, cut.y_size, cut.limit, cut.layout,
        )
        .map(|sheet| (mask_id, sheet))
        .map_err(|source| PairError::Picture {
            pict_id: image_id,
            source,
        })
    }
}
