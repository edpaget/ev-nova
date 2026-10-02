//! `röid`: asteroid type properties.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The röid
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 516), which sums to the 40 bytes of every stock record. The
//! template marks the last 16 bytes unused; the Bible does not mention them,
//! so they are kept raw.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::id::{RoidId, id};
use crate::wire::raw::RawArray;

/// One of the 16 asteroid types.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Roid {
    /// Bible `Strength` (offset 0x00, i16): armour-equivalent strength.
    pub strength: i16,
    /// Bible `SpinRate` (offset 0x02, i16): frame advance rate; 100 is 30
    /// frames per second.
    pub spin_rate: i16,
    /// Bible `YieldType` (offset 0x04, i16): encoded: 0-5 standard cargo
    /// type, 1000-1127 `jünk` index.
    pub yield_type: i16,
    /// Bible `YieldQty` (offset 0x06, i16): average resource boxes ejected.
    pub yield_qty: i16,
    /// Bible `PartCount` (offset 0x08, i16): particles thrown off when
    /// destroyed.
    pub part_count: i16,
    /// Bible `PartColor` (offset 0x0A, u32): particle colour, `00RRGGBB`.
    pub part_color: u32,
    /// Bible `FragType1` (offset 0x0E, i16): `röid` ID of sub-asteroids, or
    /// -1 for none.
    #[br(map = id::<RoidId>)]
    pub frag_type1: Option<RoidId>,
    /// Bible `FragType2` (offset 0x10, i16): alternative sub-asteroid `röid`
    /// ID, or -1 for none.
    #[br(map = id::<RoidId>)]
    pub frag_type2: Option<RoidId>,
    /// Bible `FragCount` (offset 0x12, i16): average sub-asteroids generated.
    pub frag_count: i16,
    /// Bible `ExplodeType` (offset 0x14, i16): encoded explosion type: 0-63,
    /// +1000 for extra small explosions, -1 for none.
    pub explode_type: i16,
    /// Bible `Mass` (offset 0x16, i16): mass when hit by weapons.
    pub mass: i16,
    /// Offset 0x18, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x18: RawArray<16>,
}

impl Record for Roid {
    const TYPE: ResType = ResType::new([b'r', 0x9A, b'i', b'd']);
    const SIZE: Option<usize> = Some(40);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let roid: Roid = buf::<Roid>()
            .i16(0x00, 120)
            .i16(0x08, 3)
            .u32(0x0A, 0x00AB_CDEF)
            .i16(0x0E, 129)
            .i16(0x10, -1)
            .i16(0x12, 4)
            .i16(0x14, 1001)
            .i16(0x16, 50)
            .bytes(0x18, &[0xEE; 16])
            .decode();
        assert_eq!(roid.strength, 120);
        assert_eq!(roid.part_count, 3);
        assert_eq!(roid.part_color, 0x00AB_CDEF);
        assert_eq!(roid.frag_type1, Some(RoidId(129)));
        assert_eq!(roid.frag_type2, None);
        assert_eq!(roid.frag_count, 4);
        assert_eq!(roid.explode_type, 1001);
        assert_eq!(roid.mass, 50);
        assert_eq!(roid.unknown_0x18, RawArray([0xEE; 16]));
    }

    #[test]
    fn yields_follow_spin_rate() {
        let roid: Roid = buf::<Roid>().i16(2, 30).i16(4, 1000).i16(6, 5).decode();
        assert_eq!(
            (roid.spin_rate, roid.yield_type, roid.yield_qty),
            (30, 1000, 5)
        );
    }
}
