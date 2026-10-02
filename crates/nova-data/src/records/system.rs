//! `sÿst`: star systems.
//!
//! Layout: field names and meanings from the EV Nova Bible ("The sÿst
//! resource"); byte order and widths from the ResForge Nova template
//! (`TMPB` 521), which sums to the 428 bytes of every stock record. The
//! Bible describes the fields in a different order from the bytes; the
//! template's order is the byte order.

use binrw::BinRead;
use nova_rsrc::ResType;
use serde::{Deserialize, Serialize};

use crate::decode::Record;
use crate::wire::flags::Flags16;
use crate::wire::id::{FleetId, GovtId, PersonId, StellarId, SystemId, id, ids};
use crate::wire::raw::RawArray;
use crate::wire::string::{MacString, fixed_c_string};

/// A star system.
#[derive(BinRead, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[br(big)]
pub struct System {
    /// Bible `xPos` (offset 0x00, i16): map X position.
    pub x_pos: i16,
    /// Bible `yPos` (offset 0x02, i16): map Y position.
    pub y_pos: i16,
    /// Bible `Con1-Con16` (offset 0x04, 16 x i16): hyperlinks to other
    /// `sÿst` IDs; -1 for no link.
    #[br(map = ids::<SystemId, 16>)]
    pub con: [Option<SystemId>; 16],
    /// Bible `NavDef` x16 (offset 0x24, 16 x i16): navigation default `spöb`
    /// IDs; -1 for none.
    #[br(map = ids::<StellarId, 16>)]
    pub nav_def: [Option<StellarId>; 16],
    /// Bible `DudeTypes` x8 (offset 0x44, 8 x i16): encoded: 128-639 `düde`
    /// ID, -128 to -383 negated `flët` ID, -1 unused.
    pub dude_types: [i16; 8],
    /// Bible `% Prob` x8 (offset 0x54, 8 x i16): percent probability of each
    /// `dude_types` entry.
    pub prob: [i16; 8],
    /// Bible `AvgShips` (offset 0x64, i16): average number of AI ships.
    pub avg_ships: i16,
    /// Bible `Govt` (offset 0x66, i16): owning `gövt` ID, or -1 for
    /// independent.
    #[br(map = id::<GovtId>)]
    pub govt: Option<GovtId>,
    /// Bible `Message` (offset 0x68, i16): message buoy entry in `STR#` 1000
    /// (1 and up), or -1 for none. An index, not a resource ID.
    pub message: i16,
    /// Bible `Asteroids` (offset 0x6A, i16): asteroid count, 0-16.
    pub asteroids: i16,
    /// Bible `Interference` (offset 0x6C, i16): sensor static, 0-100.
    pub interference: i16,
    /// Bible `Person` fields (offset 0x6E, 8 x i16): `përs` IDs that always
    /// appear here; -1 for none.
    #[br(map = ids::<PersonId, 8>)]
    pub person: [Option<PersonId>; 8],
    /// Offset 0x7E, 8 x i16: undocumented in the Bible (the template labels
    /// them as a probability per `person` entry).
    pub unknown_0x7e: [i16; 8],
    /// Bible `BkgndColor` (offset 0x8E, u32): background colour, `00RRGGBB`.
    pub bkgnd_color: u32,
    /// Bible `Murk` (offset 0x92, i16): murkiness, 0-100; negative also
    /// hides the starfield.
    pub murk: i16,
    /// Bible `AstTypes` (offset 0x94, u16): which `röid` types appear (bit
    /// 0x0001 is `röid` 128, and so on).
    pub ast_types: Flags16,
    /// Bible `Visibility` (offset 0x96, 256-byte C string): control-bit test
    /// expression; blank means always visible.
    #[br(parse_with = fixed_c_string::<_, 256>)]
    pub visibility: MacString,
    /// Bible `ReinfFleet` (offset 0x196, i16): reinforcement `flët` ID; 0 or
    /// -1 if unused.
    #[br(map = id::<FleetId>)]
    pub reinf_fleet: Option<FleetId>,
    /// Bible `ReinfTime` (offset 0x198, i16): reinforcement delay, 30 = one
    /// second.
    pub reinf_time: i16,
    /// Bible `ReinfIntrval` (offset 0x19A, i16): days to regenerate the
    /// reinforcement fleet.
    pub reinf_intrval: i16,
    /// Offset 0x19C, 16 bytes: undocumented (the Bible is silent; the
    /// template marks them unused).
    pub unknown_0x19c: RawArray<16>,
}

impl Record for System {
    const TYPE: ResType = ResType::new([b's', 0xD8, b's', b't']);
    const SIZE: Option<usize> = Some(428);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::buf;

    #[test]
    fn fields_sit_at_their_bible_offsets() {
        let syst: System = buf::<System>()
            .i16(0x00, -150)
            .i16(0x02, 75)
            .i16(0x04, 131)
            .i16(0x06, -1)
            .i16(0x22, 2175)
            .i16(0x24, 128)
            .i16(0x26, -1)
            .i16(0x44, -130)
            .i16(0x54, 50)
            .i16(0x64, 12)
            .i16(0x66, 128)
            .i16(0x68, 3)
            .i16(0x6A, 16)
            .i16(0x6C, 100)
            .i16(0x6E, 600)
            .i16(0x70, -1)
            .i16(0x7E, 25)
            .u32(0x8E, 0x0010_2030)
            .i16(0x92, -1)
            .u16(0x94, 0x0011)
            .bytes(0x96, b"b1 & !b2\0")
            .i16(0x196, 129)
            .i16(0x198, 90)
            .i16(0x19A, 7)
            .bytes(0x19C, &[0xAA; 16])
            .decode();
        assert_eq!((syst.x_pos, syst.y_pos), (-150, 75));
        assert_eq!(syst.con[0], Some(SystemId(131)));
        assert_eq!(syst.con[1], None);
        assert_eq!(syst.con[15], Some(SystemId(2175)));
        assert_eq!(syst.nav_def[0], Some(StellarId(128)));
        assert_eq!(syst.nav_def[1], None);
        assert_eq!(syst.dude_types[0], -130);
        assert_eq!(syst.prob[0], 50);
        assert_eq!(syst.avg_ships, 12);
        assert_eq!(syst.govt, Some(GovtId(128)));
        assert_eq!(syst.message, 3);
        assert_eq!(syst.asteroids, 16);
        assert_eq!(syst.interference, 100);
        assert_eq!(syst.person[0], Some(PersonId(600)));
        assert_eq!(syst.person[1], None);
        assert_eq!(syst.unknown_0x7e[0], 25);
        assert_eq!(syst.bkgnd_color, 0x0010_2030);
        assert_eq!(syst.murk, -1);
        assert_eq!(syst.ast_types, Flags16(0x0011));
        assert_eq!(syst.visibility.as_str(), "b1 & !b2");
        assert_eq!(syst.reinf_fleet, Some(FleetId(129)));
        assert_eq!(syst.reinf_time, 90);
        assert_eq!(syst.reinf_intrval, 7);
        assert_eq!(syst.unknown_0x19c, RawArray([0xAA; 16]));
    }

    #[test]
    fn visibility_uses_all_256_bytes_without_a_nul() {
        let syst: System = buf::<System>().bytes(0x96, &[b'x'; 256]).decode();
        assert_eq!(syst.visibility.len(), 256);
    }
}
