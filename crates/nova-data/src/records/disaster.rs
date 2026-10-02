//! `öops`: planetary "disasters" that move one commodity price.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The öops
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 512), which sums to the 282 bytes of every stock record.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::id::{StellarId, id};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A disaster. The resource name is shown in the commodity exchange.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Disaster {
    /// Bible `Stellar` (offset 0x00, i16): `spöb` ID, or -1 for any.
    #[br(map = id::<StellarId>)]
    pub stellar: Option<StellarId>,
    /// Bible `Commodity` (offset 0x02, i16): 0 food, 1 industrial, ...
    pub commodity: i16,
    /// Bible `PriceDelta` (offset 0x04, i16).
    pub price_delta: i16,
    /// Bible `Duration` (offset 0x06, i16): days.
    pub duration: i16,
    /// Bible `Freq` (offset 0x08, i16): percent chance per day.
    pub freq: i16,
    /// Bible `ActivateOn` (offset 0x0A, 256-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 256>)]
    pub activate_on: MacString,
    /// Offset 0x10A, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x10a: RawArray<16>,
}

impl Record for Disaster {
    const TYPE: ResType = ResType::new([0x9A, b'o', b'p', b's']);
    const SIZE: Option<usize> = Some(282);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let oops: Disaster = buf::<Disaster>()
            .i16(0x00, 130)
            .i16(0x02, 1)
            .i16(0x04, -50)
            .i16(0x06, 10)
            .i16(0x08, 2)
            .bytes(0x0A, b"b200\0")
            .bytes(0x10A, &[0x7E; 16])
            .decode();
        assert_eq!(oops.stellar, Some(StellarId(130)));
        assert_eq!(
            [oops.commodity, oops.price_delta, oops.duration, oops.freq],
            [1, -50, 10, 2]
        );
        assert_eq!(oops.activate_on.as_str(), "b200");
        assert_eq!(oops.unknown_0x10a, RawArray([0x7E; 16]));
    }

    #[test]
    fn any_stellar_is_none() {
        let oops: Disaster = buf::<Disaster>().i16(0x00, -1).decode();
        assert_eq!(oops.stellar, None);
    }
}
