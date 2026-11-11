//! Finding an outfit's description.

use crate::wire::id::OutfitId;

/// The first outfit ID and its description's `dësc` ID.
const FIRST_OUTFIT: i16 = 128;
const FIRST_OUTFIT_DESC: i16 = 3000;
/// The last outfit ID the Bible allows.
const LAST_OUTFIT: i16 = 639;

/// The `dësc` ID describing `outfit`: the Bible gives outfit descriptions
/// as `dësc` 3000 to 3511 for `oütf` 128 to 639, so `3000 + (ID - 128)`,
/// as the outfitter reads it (`_OutfitFilter` @0x58465, @0x588cc). `None`
/// for an ID outside that range.
#[must_use]
pub const fn outfit_desc_id(outfit: OutfitId) -> Option<i16> {
    match outfit.0 {
        id @ FIRST_OUTFIT..=LAST_OUTFIT => Some(FIRST_OUTFIT_DESC + (id - FIRST_OUTFIT)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outfit_ids_map_onto_dësc_3000_onwards() {
        assert_eq!(outfit_desc_id(OutfitId(128)), Some(3000));
        assert_eq!(outfit_desc_id(OutfitId(130)), Some(3002));
        assert_eq!(outfit_desc_id(OutfitId(639)), Some(3511));
    }

    #[test]
    fn ids_outside_the_outfit_range_have_no_description_id() {
        for id in [127, 640, 0, -1, i16::MIN, i16::MAX] {
            assert_eq!(outfit_desc_id(OutfitId(id)), None, "{id}");
        }
    }
}
