//! `jünk`: specialised commodities.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The jünk
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 509), which sums to the 676 bytes of every stock record.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{StellarId, ids};
use crate::wire::string::{MacString, fixed_c_string};

/// A special commodity.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Junk {
    /// Bible `SoldAt1-8` (offset 0x00, 8 x i16): `spöb` IDs where it is
    /// sold; 0 or -1 if unused.
    #[br(map = ids::<StellarId, 8>)]
    pub sold_at: [Option<StellarId>; 8],
    /// Bible `BoughtAt1-8` (offset 0x10, 8 x i16): `spöb` IDs where it is
    /// bought; 0 or -1 if unused.
    #[br(map = ids::<StellarId, 8>)]
    pub bought_at: [Option<StellarId>; 8],
    /// Bible `BasePrice` (offset 0x20, i16).
    pub base_price: i16,
    /// Bible `Flags` (offset 0x22, u16).
    pub flags: Flags16,
    /// Bible `ScanMask` (offset 0x24, u16).
    pub scan_mask: Flags16,
    /// Bible `LCName` (offset 0x26, 64-byte C string).
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub lc_name: MacString,
    /// Bible `Abbrev` (offset 0x66, 64-byte C string): status bar name.
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub abbrev: MacString,
    /// Bible `BuyOn` (offset 0xA6, 255-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub buy_on: MacString,
    /// Bible `SellOn` (offset 0x1A5, 255-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub sell_on: MacString,
}

impl Record for Junk {
    const TYPE: ResType = ResType::new([b'j', 0x9F, b'n', b'k']);
    const SIZE: Option<usize> = Some(676);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let junk: Junk = buf::<Junk>()
            .i16(0x00, 130)
            .i16(0x02, -1)
            .i16(0x10, 131)
            .i16(0x12, -1)
            .i16(0x20, 300)
            .u16(0x22, 0x0002)
            .u16(0x24, 0x0008)
            .bytes(0x26, b"machine parts\0")
            .bytes(0x66, b"Parts\0")
            .bytes(0xA6, b"b1\0")
            .bytes(0x1A5, &[b's'; 255])
            .decode();
        assert_eq!(junk.sold_at[0], Some(StellarId(130)));
        assert_eq!(junk.sold_at[1], None);
        assert_eq!(junk.bought_at[0], Some(StellarId(131)));
        assert_eq!(junk.bought_at[1], None);
        assert_eq!(junk.base_price, 300);
        assert_eq!(
            (junk.flags, junk.scan_mask),
            (Flags16(0x0002), Flags16(0x0008))
        );
        assert_eq!(junk.lc_name.as_str(), "machine parts");
        assert_eq!(junk.abbrev.as_str(), "Parts");
        assert_eq!(junk.buy_on.as_str(), "b1");
        assert_eq!(junk.sell_on.as_str(), "s".repeat(255));
    }
}
