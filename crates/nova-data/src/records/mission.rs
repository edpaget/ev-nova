//! `mïsn`: missions.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The mïsn
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 510), which sums to the 1970 bytes of every stock record. Words
//! the Bible does not describe stay raw under offset names. Control-bit
//! expressions are kept as text; nothing here parses them.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::{Flags16, Flags64};
use crate::wire::id::{DescId, DudeId, GovtId, StrListId, id};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A mission. The resource name is the mission's name.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct Mission {
    /// Bible `AvailStel` (offset 0x00, i16): encoded: -1 any inhabited,
    /// 128-2175 `spöb` ID, 5000+ adjacent system, 9999+... government-relative.
    pub avail_stel: i16,
    /// Offset 0x02, i16: undocumented (the template marks it unused).
    pub unknown_0x02: i16,
    /// Bible `AvailLoc` (offset 0x04, i16): 0 mission computer, 1 bar, 2
    /// ship, 3 spaceport, 4 trading, 5 shipyard, 6 outfitter.
    pub avail_loc: i16,
    /// Bible `AvailRecord` (offset 0x06, i16).
    pub avail_record: i16,
    /// Bible `AvailRating` (offset 0x08, i16).
    pub avail_rating: i16,
    /// Bible `AvailRandom` (offset 0x0A, i16): percent.
    pub avail_random: i16,
    /// Bible `TravelStel` (offset 0x0C, i16): encoded destination.
    pub travel_stel: i16,
    /// Bible `ReturnStel` (offset 0x0E, i16): encoded return point.
    pub return_stel: i16,
    /// Bible `CargoType` (offset 0x10, i16): -1 none, 0-255 cargo, 1000
    /// random.
    pub cargo_type: i16,
    /// Bible `CargoQty` (offset 0x12, i16): tons; -2 and below vary ±50%.
    pub cargo_qty: i16,
    /// Bible `PickupMode` (offset 0x14, i16).
    pub pickup_mode: i16,
    /// Bible `DropOffMode` (offset 0x16, i16).
    pub drop_off_mode: i16,
    /// Bible `ScanMask` (offset 0x18, u16).
    pub scan_mask: Flags16,
    /// Offset 0x1A, i16: undocumented (the template marks it unused).
    pub unknown_0x1a: i16,
    /// Bible `PayVal` (offset 0x1C, i32): encoded: credits, or legal-record
    /// and cash-taking ranges below zero.
    pub pay_val: i32,
    /// Bible `ShipCount` (offset 0x20, i16): special ships, or -1.
    pub ship_count: i16,
    /// Bible `ShipSyst` (offset 0x22, i16): encoded system for special
    /// ships.
    pub ship_syst: i16,
    /// Bible `ShipDude` (offset 0x24, i16): special ships' `düde` ID.
    #[br(map = id::<DudeId>)]
    pub ship_dude: Option<DudeId>,
    /// Bible `ShipGoal` (offset 0x26, i16).
    pub ship_goal: i16,
    /// Bible `ShipBehav` (offset 0x28, i16).
    pub ship_behav: i16,
    /// Bible `ShipNameID` (offset 0x2A, i16): `STR#` ID of special ship
    /// names.
    #[br(map = id::<StrListId>)]
    pub ship_name_id: Option<StrListId>,
    /// Bible `ShipStart` (offset 0x2C, i16): -1 to -16 nav default, 0-2
    /// placement modes.
    pub ship_start: i16,
    /// Bible `CompGovt` (offset 0x2E, i16): `gövt` ID whose record changes.
    #[br(map = id::<GovtId>)]
    pub comp_govt: Option<GovtId>,
    /// Bible `CompReward` (offset 0x30, i16).
    pub comp_reward: i16,
    /// Bible `ShipSubtitle` (offset 0x32, i16): `STR#` ID of special ship
    /// subtitles.
    #[br(map = id::<StrListId>)]
    pub ship_subtitle: Option<StrListId>,
    /// Bible `BriefText` (offset 0x34, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub brief_text: Option<DescId>,
    /// Bible `QuickBrief` (offset 0x36, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub quick_brief: Option<DescId>,
    /// Bible `LoadCargText` (offset 0x38, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub load_carg_text: Option<DescId>,
    /// Bible `DumpCargoText` (offset 0x3A, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub dump_cargo_text: Option<DescId>,
    /// Bible `CompText` (offset 0x3C, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub comp_text: Option<DescId>,
    /// Bible `FailText` (offset 0x3E, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub fail_text: Option<DescId>,
    /// Bible `TimeLimit` (offset 0x40, i16): days; -1 or 0 for none.
    pub time_limit: i16,
    /// Bible `CanAbort` (offset 0x42, i16): 0 or 1.
    pub can_abort: i16,
    /// Bible `ShipDoneText` (offset 0x44, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub ship_done_text: Option<DescId>,
    /// Offset 0x46, i16: undocumented (the template marks it unused).
    pub unknown_0x46: i16,
    /// Bible `AuxShipCount` (offset 0x48, i16).
    pub aux_ship_count: i16,
    /// Bible `AuxShipDude` (offset 0x4A, i16): aux ships' `düde` ID.
    #[br(map = id::<DudeId>)]
    pub aux_ship_dude: Option<DudeId>,
    /// Bible `AuxShipSyst` (offset 0x4C, i16): encoded system.
    pub aux_ship_syst: i16,
    /// Offset 0x4E, i16: undocumented (the template marks it unused).
    pub unknown_0x4e: i16,
    /// Bible `Flags` (offset 0x50, u16).
    pub flags: Flags16,
    /// Bible `Flags2` (offset 0x52, u16).
    pub flags2: Flags16,
    /// Offset 0x54, 4 bytes: undocumented (the template marks them unused).
    pub unknown_0x54: RawArray<4>,
    /// Bible `RefuseText` (offset 0x58, i16): `dësc` ID.
    #[br(map = id::<DescId>)]
    pub refuse_text: Option<DescId>,
    /// Bible `AvailShipType` (offset 0x5A, i16): encoded ship or government
    /// requirement.
    pub avail_ship_type: i16,
    /// Bible `AvailBits` (offset 0x5C, 255-byte C string): control-bit test
    /// expression.
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub avail_bits: MacString,
    /// Bible `OnAccept` (offset 0x15B, 255-byte C string).
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_accept: MacString,
    /// Bible `OnRefuse` (offset 0x25A, 255-byte C string).
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_refuse: MacString,
    /// Bible `OnSuccess` (offset 0x359, 255-byte C string).
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_success: MacString,
    /// Bible `OnFailure` (offset 0x458, 255-byte C string).
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_failure: MacString,
    /// Bible `OnAbort` (offset 0x557, 255-byte C string).
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_abort: MacString,
    /// Bible `Require` pair (offset 0x656, u64).
    pub require: Flags64,
    /// Bible `DatePostInc` (offset 0x65E, i16): days added on completion.
    pub date_post_inc: i16,
    /// Bible `OnShipDone` (offset 0x660, 255-byte C string).
    #[br(parse_with = fixed_c_string::<_, 255>)]
    pub on_ship_done: MacString,
    /// Bible `AcceptButton` (offset 0x75F, 32-byte C string).
    #[br(parse_with = fixed_c_string::<_, 32>)]
    pub accept_button: MacString,
    /// Bible `RefuseButton` (offset 0x77F, 33-byte C string).
    #[br(parse_with = fixed_c_string::<_, 33>)]
    pub refuse_button: MacString,
    /// Bible `DispWeight` (offset 0x7A0, i16): bar and BBS ordering.
    pub disp_weight: i16,
    /// Offset 0x7A2, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x7a2: RawArray<16>,
}

impl Record for Mission {
    const TYPE: ResType = ResType::new([b'm', 0x95, b's', b'n']);
    const SIZE: Option<usize> = Some(1970);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn availability_cargo_and_ship_fields_sit_at_their_bible_offsets() {
        let mut b = buf::<Mission>();
        for (i, offset) in (0x00..0x1C).step_by(2).enumerate() {
            b = b.i16(offset, 1000 + i as i16);
        }
        let misn: Mission = b
            .i32(0x1C, -40_005)
            .i16(0x20, 3)
            .i16(0x22, -6)
            .i16(0x24, 150)
            .i16(0x26, 2)
            .i16(0x28, 1)
            .i16(0x2A, 7000)
            .i16(0x2C, -16)
            .i16(0x2E, 131)
            .i16(0x30, 5)
            .i16(0x32, 7001)
            .decode();
        assert_eq!(misn.avail_stel, 1000);
        assert_eq!(misn.unknown_0x02, 1001);
        let avail = [
            misn.avail_loc,
            misn.avail_record,
            misn.avail_rating,
            misn.avail_random,
        ];
        assert_eq!(avail, [1002, 1003, 1004, 1005]);
        assert_eq!((misn.travel_stel, misn.return_stel), (1006, 1007));
        let cargo = [
            misn.cargo_type,
            misn.cargo_qty,
            misn.pickup_mode,
            misn.drop_off_mode,
        ];
        assert_eq!(cargo, [1008, 1009, 1010, 1011]);
        assert_eq!(misn.scan_mask, Flags16(1012));
        assert_eq!(misn.unknown_0x1a, 1013);
        assert_eq!(misn.pay_val, -40_005);
        assert_eq!((misn.ship_count, misn.ship_syst), (3, -6));
        assert_eq!(misn.ship_dude, Some(DudeId(150)));
        assert_eq!((misn.ship_goal, misn.ship_behav), (2, 1));
        assert_eq!(misn.ship_name_id, Some(StrListId(7000)));
        assert_eq!(misn.ship_start, -16);
        assert_eq!(misn.comp_govt, Some(GovtId(131)));
        assert_eq!(misn.comp_reward, 5);
        assert_eq!(misn.ship_subtitle, Some(StrListId(7001)));
    }

    #[test]
    fn text_and_aux_fields_sit_at_their_bible_offsets() {
        let misn: Mission = buf::<Mission>()
            .i16(0x34, 5000)
            .i16(0x36, 5001)
            .i16(0x38, 5002)
            .i16(0x3A, 5003)
            .i16(0x3C, 5004)
            .i16(0x3E, -1)
            .i16(0x40, 30)
            .i16(0x42, 1)
            .i16(0x44, 5006)
            .i16(0x46, 77)
            .i16(0x48, 4)
            .i16(0x4A, 151)
            .i16(0x4C, 5300)
            .i16(0x4E, 78)
            .u16(0x50, 0x8001)
            .u16(0x52, 0x0004)
            .bytes(0x54, &[1, 2, 3, 4])
            .i16(0x58, 5007)
            .i16(0x5A, 1128)
            .decode();
        assert_eq!(misn.brief_text, Some(DescId(5000)));
        assert_eq!(misn.quick_brief, Some(DescId(5001)));
        assert_eq!(misn.load_carg_text, Some(DescId(5002)));
        assert_eq!(misn.dump_cargo_text, Some(DescId(5003)));
        assert_eq!(misn.comp_text, Some(DescId(5004)));
        assert_eq!(misn.fail_text, None);
        assert_eq!((misn.time_limit, misn.can_abort), (30, 1));
        assert_eq!(misn.ship_done_text, Some(DescId(5006)));
        assert_eq!(misn.unknown_0x46, 77);
        assert_eq!(misn.aux_ship_count, 4);
        assert_eq!(misn.aux_ship_dude, Some(DudeId(151)));
        assert_eq!(misn.aux_ship_syst, 5300);
        assert_eq!(misn.unknown_0x4e, 78);
        assert_eq!(
            (misn.flags, misn.flags2),
            (Flags16(0x8001), Flags16(0x0004))
        );
        assert_eq!(misn.unknown_0x54, RawArray([1, 2, 3, 4]));
        assert_eq!(misn.refuse_text, Some(DescId(5007)));
        assert_eq!(misn.avail_ship_type, 1128);
    }

    #[test]
    fn expression_and_button_fields_sit_at_their_bible_offsets() {
        let misn: Mission = buf::<Mission>()
            .bytes(0x5C, b"b1\0")
            .bytes(0x15B, b"b2\0")
            .bytes(0x25A, b"b3\0")
            .bytes(0x359, b"b4\0")
            .bytes(0x458, b"b5\0")
            .bytes(0x557, b"b6\0")
            .u64(0x656, 0x10)
            .i16(0x65E, 3)
            .bytes(0x660, b"b7\0")
            .bytes(0x75F, b"Take it\0")
            .bytes(0x77F, &[b'n'; 33])
            .i16(0x7A0, 9)
            .bytes(0x7A2, &[0x3C; 16])
            .decode();
        assert_eq!(misn.avail_bits.as_str(), "b1");
        assert_eq!(misn.on_accept.as_str(), "b2");
        assert_eq!(misn.on_refuse.as_str(), "b3");
        assert_eq!(misn.on_success.as_str(), "b4");
        assert_eq!(misn.on_failure.as_str(), "b5");
        assert_eq!(misn.on_abort.as_str(), "b6");
        assert_eq!(misn.require, Flags64(0x10));
        assert_eq!(misn.date_post_inc, 3);
        assert_eq!(misn.on_ship_done.as_str(), "b7");
        assert_eq!(misn.accept_button.as_str(), "Take it");
        assert_eq!(misn.refuse_button.as_str(), "n".repeat(33));
        assert_eq!(misn.disp_weight, 9);
        assert_eq!(misn.unknown_0x7a2, RawArray([0x3C; 16]));
    }
}
