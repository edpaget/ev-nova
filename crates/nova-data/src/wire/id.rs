//! Typed references to other resources.
//!
//! A field that names another resource by ID is read as an `i16` and wrapped
//! in the newtype for that resource type, so cross-references are explicit
//! in the record types. Nothing is resolved here; that is left to callers.
//!
//! Nova's convention is that `-1` means "none", so `-1` maps to `None`. Every
//! other value, including `0` and other negatives, is kept as `Some`: some
//! fields also treat `0` as unused, but the engine may read those values
//! differently per field, so they are passed through untouched and noted on
//! the field.
//!
//! Records read these fields with `#[br(map = id::<ShipId>)]` (one ID) or
//! `#[br(map = ids::<WeaponId, 8>)]` (an array of IDs).

use serde::{Deserialize, Serialize};

/// An ID newtype over a raw `i16` resource ID.
pub trait NovaId: Copy {
    /// Wraps a raw ID; `-1` (none) becomes `None`.
    fn from_raw(raw: i16) -> Option<Self>;
    /// The raw resource ID.
    fn raw(self) -> i16;
}

/// The raw value Nova uses for "no reference".
const NONE: i16 = -1;

fn present(raw: i16) -> Option<i16> {
    (raw != NONE).then_some(raw)
}

macro_rules! nova_ids {
    ($($(#[doc = $doc:literal])* $name:ident;)*) => {$(
        $(#[doc = $doc])*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i16);

        impl NovaId for $name {
            fn from_raw(raw: i16) -> Option<Self> {
                present(raw).map(Self)
            }

            fn raw(self) -> i16 {
                self.0
            }
        }
    )*};
}

nova_ids! {
    /// ID of a `bööm` (explosion) resource.
    BoomId;
    /// ID of a `crön` (timed event) resource.
    CronId;
    /// ID of a `dësc` (description) resource.
    DescId;
    /// ID of a `DITL` (dialog item list) resource.
    DitlId;
    /// ID of a `düde` (ship group) resource.
    DudeId;
    /// ID of a `flët` (fleet) resource.
    FleetId;
    /// ID of a `gövt` (government) resource.
    GovtId;
    /// ID of an `ïntf` (status bar) resource.
    InterfaceId;
    /// ID of a `jünk` (special commodity) resource.
    JunkId;
    /// ID of a `mïsn` (mission) resource.
    MissionId;
    /// ID of a `nëbu` (nebula) resource.
    NebulaId;
    /// ID of an `oütf` (outfit) resource.
    OutfitId;
    /// ID of a `përs` (person) resource.
    PersonId;
    /// ID of a `PICT` (picture) resource.
    PictId;
    /// ID of a `ränk` (rank) resource.
    RankId;
    /// ID of a `röid` (asteroid type) resource.
    RoidId;
    /// ID of a `shïp` (ship class) resource.
    ShipId;
    /// ID of a `snd ` (sound) resource.
    SoundId;
    /// ID of a `spïn` (sprite info) resource.
    SpinId;
    /// ID of a `spöb` (stellar object) resource.
    StellarId;
    /// ID of a `STR#` (string list) resource.
    StrListId;
    /// ID of a `sÿst` (star system) resource.
    SystemId;
    /// ID of a `wëap` (weapon) resource.
    WeaponId;
}

/// Maps one raw ID; for `#[br(map = id::<T>)]`.
#[must_use]
pub fn id<T: NovaId>(raw: i16) -> Option<T> {
    T::from_raw(raw)
}

/// Maps an array of raw IDs; for `#[br(map = ids::<T, N>)]`.
#[must_use]
pub fn ids<T: NovaId, const N: usize>(raw: [i16; N]) -> [Option<T>; N] {
    raw.map(T::from_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use binrw::BinRead;
    use std::io::Cursor;

    #[test]
    fn minus_one_is_none() {
        assert_eq!(id::<ShipId>(-1), None);
        assert_eq!(ShipId::from_raw(-1), None);
    }

    #[test]
    fn every_other_value_is_kept() {
        for raw in [0, 128, -2, i16::MIN, i16::MAX] {
            assert_eq!(id::<GovtId>(raw), Some(GovtId(raw)));
            assert_eq!(GovtId(raw).raw(), raw);
        }
    }

    #[test]
    fn array_form_maps_each_element() {
        assert_eq!(
            ids::<WeaponId, 4>([128, -1, 0, 200]),
            [
                Some(WeaponId(128)),
                None,
                Some(WeaponId(0)),
                Some(WeaponId(200))
            ]
        );
    }

    #[derive(BinRead, Debug, PartialEq)]
    #[br(big)]
    struct Refs {
        #[br(map = id::<ShipId>)]
        ship: Option<ShipId>,
        #[br(map = ids::<SystemId, 2>)]
        links: [Option<SystemId>; 2],
    }

    #[test]
    fn reads_through_binrw_map() {
        let refs =
            Refs::read_be(&mut Cursor::new([0x00, 0x80, 0xFF, 0xFF, 0x00, 0x81])).expect("decodes");
        assert_eq!(
            refs,
            Refs {
                ship: Some(ShipId(128)),
                links: [None, Some(SystemId(129))],
            }
        );
    }

    #[test]
    fn json_is_a_number_or_null_and_round_trips() {
        let value = [Some(StellarId(128)), None, Some(StellarId(-2))];
        let json = serde_json::to_string(&value).expect("serializes");
        assert_eq!(json, "[128,null,-2]");
        let back: [Option<StellarId>; 3] = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, value);
    }
}
