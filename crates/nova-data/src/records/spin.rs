//! `spïn`: sprite info for simple graphical objects.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The spïn
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 519), which sums to the 12 bytes of every stock record.

use binrw::BinRead;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::id::{PictId, id};
use nova_rsrc::ResType;

/// How to load an object's sprites from a `PICT`/`rlëD` grid.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Spin {
    /// Bible `SpritesID` (offset 0x00, i16): ID of the sprites' `PICT`, or of
    /// an `rlëD`/`rle8` resource, so the referenced type is not fixed.
    pub sprites_id: i16,
    /// Bible `MasksID` (offset 0x02, i16): ID of the masks' `PICT`.
    #[br(map = id::<PictId>)]
    pub masks_id: Option<PictId>,
    /// Bible `xSize` (offset 0x04, i16): width of each sprite.
    pub x_size: i16,
    /// Bible `ySize` (offset 0x06, i16): height of each sprite.
    pub y_size: i16,
    /// Bible `xTiles` (offset 0x08, i16): grid columns.
    pub x_tiles: i16,
    /// Bible `yTiles` (offset 0x0A, i16): grid rows.
    pub y_tiles: i16,
}

impl Record for Spin {
    const TYPE: ResType = ResType::new([b's', b'p', 0x95, b'n']);
    const SIZE: Option<usize> = Some(12);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let spin: Spin = buf::<Spin>()
            .i16(0x00, 1000)
            .i16(0x02, 1001)
            .i16(0x04, 48)
            .i16(0x06, 46)
            .i16(0x08, 6)
            .i16(0x0A, 7)
            .decode();
        assert_eq!(
            spin,
            Spin {
                sprites_id: 1000,
                masks_id: Some(PictId(1001)),
                x_size: 48,
                y_size: 46,
                x_tiles: 6,
                y_tiles: 7,
            }
        );
    }

    #[test]
    fn missing_mask_is_none() {
        let spin: Spin = buf::<Spin>().i16(0x02, -1).decode();
        assert_eq!(spin.masks_id, None);
    }
}
