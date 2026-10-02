//! `oütf`: outfit items.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The oütf
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 513), which sums to the 1028 bytes of every stock record.
//! `ModVal` meanings depend on `ModType`, so they stay raw.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::{Flags16, Flags64};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// An outfit item. The resource name is the item name.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Outfit {
    /// Bible `DispWeight` (offset 0x00, i16): outfitter ordering.
    pub disp_weight: i16,
    /// Bible `Mass` (offset 0x02, i16): tons.
    pub mass: i16,
    /// Bible `TechLevel` (offset 0x04, i16).
    pub tech_level: i16,
    /// Bible `ModType` (offset 0x06, i16): what the item does (1 weapon, 3
    /// ammunition, ...).
    pub mod_type: i16,
    /// Bible `ModVal` (offset 0x08, i16): meaning depends on `mod_type`.
    pub mod_val: i16,
    /// Bible `Max` (offset 0x0A, i16): how many the player can own.
    pub max: i16,
    /// Bible `Flags` (offset 0x0C, u16).
    pub flags: Flags16,
    /// Bible `Cost` (offset 0x0E, i32).
    pub cost: i32,
    /// Bible `ModType2` (offset 0x12, i16).
    pub mod_type2: i16,
    /// Bible `ModVal2` (offset 0x14, i16).
    pub mod_val2: i16,
    /// Bible `ModType3` (offset 0x16, i16).
    pub mod_type3: i16,
    /// Bible `ModVal3` (offset 0x18, i16).
    pub mod_val3: i16,
    /// Bible `ModType4` (offset 0x1A, i16).
    pub mod_type4: i16,
    /// Bible `ModVal4` (offset 0x1C, i16).
    pub mod_val4: i16,
    /// Bible `Contribute` pair (offset 0x1E, u64).
    pub contribute: Flags64,
    /// Bible `Require` pair (offset 0x26, u64).
    pub require: Flags64,
    /// Bible `Availability` (offset 0x2E, 255-byte C string): control-bit
    /// test expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub availability: MacString,
    /// Bible `OnPurchase` (offset 0x12D, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_purchase: MacString,
    /// Bible `OnSell` (offset 0x22C, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_sell: MacString,
    /// Bible `ShortName` (offset 0x32B, 64-byte C string): outfitter name;
    /// a literal `\n` splits it in two lines.
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub short_name: MacString,
    /// Bible `LCName` (offset 0x36B, 64-byte C string): lower-case singular.
    #[br(parse_with = fixed_c_string::<_, 64>)]
    pub lc_name: MacString,
    /// Bible `LCPlural` (offset 0x3AB, 65-byte C string): lower-case plural.
    #[br(parse_with = fixed_c_string::<_, 65>)]
    pub lc_plural: MacString,
    /// Bible `ItemClass` (offset 0x3EC, i16): class used by `përs` grants.
    pub item_class: i16,
    /// Bible `ScanMask` (offset 0x3EE, u16).
    pub scan_mask: Flags16,
    /// Bible `BuyRandom` (offset 0x3F0, i16): percent chance for sale.
    pub buy_random: i16,
    /// Bible `RequireGovt` (offset 0x3F2, i16): encoded: -1 all, 128-383
    /// `gövt` ID, +1000/+2000/+3000 variants.
    pub require_govt: i16,
    /// Offset 0x3F4, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x3f4: RawArray<16>,
}

impl Record for Outfit {
    const TYPE: ResType = ResType::new([b'o', 0x9F, b't', b'f']);
    const SIZE: Option<usize> = Some(1028);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let outf: Outfit = buf::<Outfit>()
            .i16(0x00, 50)
            .i16(0x02, 5)
            .i16(0x04, 3)
            .i16(0x06, 1)
            .i16(0x08, 130)
            .i16(0x0A, 8)
            .u16(0x0C, 0x0005)
            .i32(0x0E, 150_000)
            .i16(0x12, 9)
            .i16(0x14, -10)
            .i16(0x16, 43)
            .i16(0x18, 0x7C00)
            .i16(0x1A, 33)
            .i16(0x1C, 20)
            .u64(0x1E, 1)
            .u64(0x26, 2)
            .bytes(0x2E, b"b1 | b2\0")
            .bytes(0x12D, b"b3\0")
            .bytes(0x22C, b"!b3\0")
            .bytes(0x32B, b"Big\\nGun\0")
            .bytes(0x36B, b"big gun\0")
            .bytes(0x3AB, &[b'g'; 65])
            .i16(0x3EC, 12)
            .u16(0x3EE, 0x0002)
            .i16(0x3F0, 75)
            .i16(0x3F2, 1128)
            .bytes(0x3F4, &[0x44; 16])
            .decode();
        assert_eq!((outf.disp_weight, outf.mass, outf.tech_level), (50, 5, 3));
        assert_eq!((outf.mod_type, outf.mod_val, outf.max), (1, 130, 8));
        assert_eq!(outf.flags, Flags16(0x0005));
        assert_eq!(outf.cost, 150_000);
        assert_eq!((outf.mod_type2, outf.mod_val2), (9, -10));
        assert_eq!((outf.mod_type3, outf.mod_val3), (43, 0x7C00));
        assert_eq!((outf.mod_type4, outf.mod_val4), (33, 20));
        assert_eq!((outf.contribute, outf.require), (Flags64(1), Flags64(2)));
        assert_eq!(outf.availability.as_str(), "b1 | b2");
        assert_eq!(outf.on_purchase.as_str(), "b3");
        assert_eq!(outf.on_sell.as_str(), "!b3");
        assert_eq!(outf.short_name.as_str(), "Big\\nGun");
        assert_eq!(outf.lc_name.as_str(), "big gun");
        assert_eq!(outf.lc_plural.as_str(), "g".repeat(65));
        assert_eq!(outf.item_class, 12);
        assert_eq!(outf.scan_mask, Flags16(0x0002));
        assert_eq!((outf.buy_random, outf.require_govt), (75, 1128));
        assert_eq!(outf.unknown_0x3f4, RawArray([0x44; 16]));
    }
}
