//! `bööm`: explosion type behaviour.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The bööm
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 500), which sums to the 6 bytes of every stock record.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;

/// How one of the 64 explosion types animates and sounds.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Boom {
    /// Bible `FrameAdvance` (offset 0x00, i16): animation rate; 100 shows
    /// each frame for one game frame.
    pub frame_advance: i16,
    /// Bible `SoundIndex` (offset 0x02, i16): explosion sound index 0-63
    /// (`snd ` 300-363), or -1 for silent. An index, not a resource ID.
    pub sound_index: i16,
    /// Bible `GraphicIndex` (offset 0x04, i16): explosion graphic index 0-63
    /// (`spïn` 400-463). An index, not a resource ID.
    pub graphic_index: i16,
}

impl Record for Boom {
    const TYPE: ResType = ResType::new([b'b', 0x9A, 0x9A, b'm']);
    const SIZE: Option<usize> = Some(6);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let boom: Boom = buf::<Boom>().i16(0, 100).i16(2, -1).i16(4, 63).decode();
        assert_eq!(
            boom,
            Boom {
                frame_advance: 100,
                sound_index: -1,
                graphic_index: 63,
            }
        );
    }
}
