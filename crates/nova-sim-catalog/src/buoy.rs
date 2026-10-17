//! A system's message buoy: the text a hyperspace arrival shows in place
//! of the arrival line.
//!
//! `_HandlePlayer`'s hyperspace arrival (@0x6c25b-0x6c27a) reads the
//! system's `sÿst` `Message` (resource +0x68, copied to the record's +0x92
//! by `_LoadObjectData` @0x77c3e-0x77c42). Any value but −1 calls
//! `_DisplayMessage(Message)` and jumps past the whole arrival line
//! (@0x6c27a → @0x6c472): its lead, the system's name and the date, the
//! " No stellar objects present." suffix (#49, @0x6c381-0x6c3bf) and the
//! abandoned-fighters suffix (@0x6c3c4-0x6c42b). The fighters are still
//! abandoned. Hypergate and wormhole exits never show a buoy.
//!
//! `_DisplayMessage` (@0x5289-0x530e) shows, for 480 ticks (@0x5300) and
//! with no sound:
//!
//! - the `STR ` of ID `Message` + 999 when one exists in any open file
//!   (`_LoadPluginString` @0x71a8e-0x71af4, read by
//!   [`patched_string`](crate::catalog::patched_string)), the base
//!   data's included;
//! - otherwise `STR#` 1000 #`Message` (`GetIndString` @0x52b3-0x52c7),
//!   which is empty for an index the list does not hold: 0, below 0, past
//!   its count, or with no `STR#` 1000 at all.
//!
//! A zero-byte `STR ` leaves `_DisplayMessage`'s buffer unwritten, and one
//! too short for its length byte is copied raw, so the original shows
//! whatever its stack held. The engine cannot model stack contents and
//! shows an empty line for both, as [`StringPatch::Empty`] and
//! [`StringPatch::Unreadable`] read elsewhere.
//!
//! The original adds 999 in 32 bits and cuts the sum to 16 (`cwtl`
//! @0x52a8), so a `Message` of 31,769 or more looks up a negative ID. The
//! engine keeps wider numbers: [`patch_id`] gives no ID on overflow, and
//! the caller then looks up no `STR ` ([`StringPatch::Absent`]).
//!
//! What an index `STR#` 1000 does not hold shows, with no `STR `, is
//! [`RuleKey::MissingBuoyString`](crate::RuleKey::MissingBuoyString)'s:
//! the original's empty line by the engine, the arrival line by the other
//! reading.

use crate::catalog::{self, StringPatch};

/// The `STR#` holding the buoy strings, "Message Buoy Strings".
pub const BUOY_STRINGS: i16 = 1000;

/// What a `STR ` patching buoy `index` is offset by: `index` + 999.
const PATCH_OFFSET: i16 = 999;

/// The ID of the `STR ` that patches buoy `index`, `index` + 999, or
/// `None` when the sum does not fit (see the module docs and
/// [`catalog::patch_id`](crate::catalog::patch_id)).
#[must_use]
pub fn patch_id(index: i16) -> Option<i16> {
    catalog::patch_id(index, PATCH_OFFSET)
}

/// The text buoy `index` shows, given the `STR ` `patch` of its
/// [`patch_id`] and `strings`, `STR#` 1000 whole: the
/// [`patched_string`](crate::catalog::patched_string), or `None` for an
/// index the list does not hold (see the module docs).
#[must_use]
pub fn buoy_text(index: i16, patch: StringPatch, strings: &[String]) -> Option<String> {
    catalog::patched_string(index, patch, strings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three() -> Vec<String> {
        ["one", "two", "three"].map(str::to_owned).to_vec()
    }

    #[test]
    fn an_index_the_list_holds_shows_that_string() {
        for (index, text) in [(1, "one"), (2, "two"), (3, "three")] {
            assert_eq!(
                buoy_text(index, StringPatch::Absent, &three()),
                Some(text.to_owned())
            );
        }
    }

    #[test]
    fn a_patchs_text_wins_over_the_list_even_past_it() {
        for index in [2, 4] {
            assert_eq!(
                buoy_text(index, StringPatch::Text("patched".into()), &three()),
                Some("patched".to_owned()),
                "{index}"
            );
        }
    }

    #[test]
    fn an_empty_or_unreadable_patch_shows_an_empty_line() {
        for patch in [StringPatch::Empty, StringPatch::Unreadable] {
            assert_eq!(
                buoy_text(2, patch.clone(), &three()),
                Some(String::new()),
                "{patch:?}"
            );
        }
    }

    #[test]
    fn an_index_the_list_does_not_hold_shows_nothing_of_its_own() {
        for index in [0, -2, -1, 4, i16::MIN, i16::MAX] {
            assert_eq!(
                buoy_text(index, StringPatch::Absent, &three()),
                None,
                "{index}"
            );
        }
        assert_eq!(buoy_text(1, StringPatch::Absent, &[]), None);
    }

    #[test]
    fn a_patch_is_999_past_its_index_and_none_past_the_word() {
        assert_eq!(patch_id(1), Some(1000));
        assert_eq!(patch_id(20_003), Some(21_002));
        assert_eq!(patch_id(-1000), Some(-1));
        assert_eq!(patch_id(i16::MAX - 999), Some(i16::MAX));
        assert_eq!(patch_id(i16::MAX - 998), None);
    }
}
