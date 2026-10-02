//! `crön`: timed events driven by control bits.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The crön
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 503), which sums to the 822 bytes of every stock record. The
//! Bible describes the fields in a different order from the bytes; the
//! template's order is the byte order.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::{Flags16, Flags64};
use crate::wire::id::{GovtId, StrListId, id, ids};
use crate::wire::string::{MacString, fixed_c_string};

/// A timed event.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Cron {
    /// Bible `FirstDay` (offset 0x00, i16): 1-31; 0 or -1 matches any.
    pub first_day: i16,
    /// Bible `FirstMonth` (offset 0x02, i16).
    pub first_month: i16,
    /// Bible `FirstYear` (offset 0x04, i16).
    pub first_year: i16,
    /// Bible `LastDay` (offset 0x06, i16).
    pub last_day: i16,
    /// Bible `LastMonth` (offset 0x08, i16).
    pub last_month: i16,
    /// Bible `LastYear` (offset 0x0A, i16).
    pub last_year: i16,
    /// Bible `Random` (offset 0x0C, i16): percent chance of activation.
    pub random: i16,
    /// Bible `Duration` (offset 0x0E, i16): days active.
    pub duration: i16,
    /// Bible `PreHoldoff` (offset 0x10, i16): days before starting.
    pub pre_holdoff: i16,
    /// Bible `PostHoldoff` (offset 0x12, i16): days before deactivating.
    pub post_holdoff: i16,
    /// Bible `IndNewsStr` (offset 0x14, i16): `STR#` ID of independent news,
    /// or -1 for none.
    #[br(map = id::<StrListId>)]
    pub ind_news_str: Option<StrListId>,
    /// Bible `Flags` (offset 0x16, u16).
    pub flags: Flags16,
    /// Bible `EnableOn` (offset 0x18, 255-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub enable_on: MacString,
    /// Bible `OnStart` (offset 0x117, 255-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_start: MacString,
    /// Bible `OnEnd` (offset 0x216, 256-byte C string): control-bit set
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 256>)]
    pub on_end: MacString,
    /// Bible `Contribute` pair (offset 0x316, u64).
    pub contribute: Flags64,
    /// Bible `Require` pair (offset 0x31E, u64).
    pub require: Flags64,
    /// Bible `NewsGovt1-4` (offset 0x326, 4 x i16): `gövt` IDs with local
    /// news; -1 if unused.
    #[br(map = ids::<GovtId, 4>)]
    pub news_govt: [Option<GovtId>; 4],
    /// Bible `GovtNewsStr1-4` (offset 0x32E, 4 x i16): `STR#` IDs of each
    /// government's news; -1 if unused.
    #[br(map = ids::<StrListId, 4>)]
    pub govt_news_str: [Option<StrListId>; 4],
}

impl Record for Cron {
    const TYPE: ResType = ResType::new([b'c', b'r', 0x9A, b'n']);
    const SIZE: Option<usize> = Some(822);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let cron: Cron = buf::<Cron>()
            .i16(0x00, 1)
            .i16(0x02, 2)
            .i16(0x04, 1177)
            .i16(0x06, 31)
            .i16(0x08, 12)
            .i16(0x0A, 1200)
            .i16(0x0C, 100)
            .i16(0x0E, 90)
            .i16(0x10, 200)
            .i16(0x12, 7)
            .i16(0x14, -1)
            .u16(0x16, 0x0003)
            .bytes(0x18, b"b317 & !b1300\0")
            .bytes(0x117, b"b1300\0")
            .bytes(0x216, &[b'e'; 256])
            .u64(0x316, 4)
            .u64(0x31E, 8)
            .i16(0x326, 130)
            .i16(0x328, -1)
            .i16(0x32E, 15_000)
            .i16(0x330, -1)
            .decode();
        let dates = [
            cron.first_day,
            cron.first_month,
            cron.first_year,
            cron.last_day,
            cron.last_month,
            cron.last_year,
        ];
        assert_eq!(dates, [1, 2, 1177, 31, 12, 1200]);
        let timing = [
            cron.random,
            cron.duration,
            cron.pre_holdoff,
            cron.post_holdoff,
        ];
        assert_eq!(timing, [100, 90, 200, 7]);
        assert_eq!(cron.ind_news_str, None);
        assert_eq!(cron.flags, Flags16(0x0003));
        assert_eq!(cron.enable_on.as_str(), "b317 & !b1300");
        assert_eq!(cron.on_start.as_str(), "b1300");
        assert_eq!(cron.on_end.as_str(), "e".repeat(256));
        assert_eq!((cron.contribute, cron.require), (Flags64(4), Flags64(8)));
        assert_eq!(cron.news_govt[0], Some(GovtId(130)));
        assert_eq!(cron.news_govt[1], None);
        assert_eq!(cron.govt_news_str[0], Some(StrListId(15_000)));
        assert_eq!(cron.govt_news_str[1], None);
    }
}
