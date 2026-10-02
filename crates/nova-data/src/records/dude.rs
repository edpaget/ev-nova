//! `düde`: ship groups with shared AI, government and booty.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The düde
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 505), which sums to the 88 bytes of every stock record.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{GovtId, ShipId, id, ids};
use crate::wire::raw::RawArray;

/// A dude class.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Dude {
    /// Bible `AIType` (offset 0x00, i16): 1-4, or 0 for each ship's own.
    pub ai_type: i16,
    /// Bible `Govt` (offset 0x02, i16): `gövt` ID, or -1 for independent.
    #[br(map = id::<GovtId>)]
    pub govt: Option<GovtId>,
    /// Bible `Booty` (offset 0x04, u16): what boarding yields; the Bible
    /// lists these bits under `Flags`.
    pub booty: Flags16,
    /// Bible `InfoTypes` (offset 0x06, u16): hail information; `0x4xxx`
    /// carries a `STR#` offset in its low 12 bits.
    pub info_types: Flags16,
    /// Bible `ShipType` x16 (offset 0x08, 16 x i16): `shïp` IDs; 0 or -1 if
    /// unused.
    #[br(map = ids::<ShipId, 16>)]
    pub ship_type: [Option<ShipId>; 16],
    /// Bible `Probability` x16 (offset 0x28, 16 x i16): chance of each ship
    /// type.
    pub probability: [i16; 16],
    /// Offset 0x48, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x48: RawArray<16>,
}

impl Record for Dude {
    const TYPE: ResType = ResType::new([b'd', 0x9F, b'd', b'e']);
    const SIZE: Option<usize> = Some(88);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let dude: Dude = buf::<Dude>()
            .i16(0x00, 3)
            .i16(0x02, 129)
            .u16(0x04, 0x0041)
            .u16(0x06, 0x4005)
            .i16(0x08, 128)
            .i16(0x0A, -1)
            .i16(0x26, 0)
            .i16(0x28, 60)
            .i16(0x46, 5)
            .bytes(0x48, &[0x99; 16])
            .decode();
        assert_eq!(dude.ai_type, 3);
        assert_eq!(dude.govt, Some(GovtId(129)));
        assert_eq!(
            (dude.booty, dude.info_types),
            (Flags16(0x0041), Flags16(0x4005))
        );
        assert_eq!(dude.ship_type[0], Some(ShipId(128)));
        assert_eq!(dude.ship_type[1], None);
        assert_eq!(dude.ship_type[15], Some(ShipId(0)), "0 is kept");
        assert_eq!((dude.probability[0], dude.probability[15]), (60, 5));
        assert_eq!(dude.unknown_0x48, RawArray([0x99; 16]));
    }
}
