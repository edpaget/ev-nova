//! A system's murk: how far the player sees before sprites darken into
//! space, as the original's `_PlayerEffectiveMurk` (@0x6f8f-0x704b)
//! decides it. The view darkens each shot by it (see `nova-view`'s
//! `flight::murk`).
//!
//! Addresses are in `EV Nova.app/Contents/MacOS/EV Nova` (i386).
//!
//! - The loaded `Murk` ([`loaded_murk`], `_LoadObjectData`
//!   @0x77ece-0x77eff): above 100 is 100, 1 to 7 none by the engine (the
//!   rulebook's [`RuleKey::LowMurk`](crate::RuleKey::LowMurk)), and any
//!   negative −1.
//! - The player's effective murk ([`effective_murk`]): the loaded `Murk`
//!   floored at none, plus the sum over every outfit of count × `ModVal`
//!   for each of its four `ModType` [`MOD_MURK`] slots ([`outfit_murk`]),
//!   clamped to 0..=100. The original sums in 32 bits (`imull`); ours sums
//!   in `i32` too, but never wraps.
//! - The original caches it in `_playerEffectiveMurk` (0x3b5038), which
//!   the player's display refreshes when it is below none
//!   (`_HandleShipDisplay` @0x2bddf-0x2bdf0). `nova_sim::Session` works it
//!   out afresh at each call; when the original's goes stale is
//!   rdm:task/reset-player-precalced-left-out-branches's question.

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord, outfit_mods};
use crate::rulebook::RuleSource;

/// The `ModType` of an outfit that adds its `ModVal` to the murk
/// (`cmpw $0x1c` in `_PlayerEffectiveMurk`).
pub const MOD_MURK: i16 = 28;

/// The most murk there is, loaded or effective (@0x77ece, @0x7039).
pub const MURK_LIMIT: i16 = 100;

/// The highest `Murk` the engine loads as none (@0x77ee6-0x77ef8).
pub const LOW_MURK: i16 = 7;

/// A system's `Murk` `raw` as `_LoadObjectData` loads it
/// (@0x77ece-0x77eff): above [`MURK_LIMIT`] is 100; 1 to [`LOW_MURK`]
/// none by the engine's reading of `low`, kept by the Bible's; any
/// negative −1; the rest kept.
#[must_use]
pub fn loaded_murk(raw: i16, low: RuleSource) -> i16 {
    if raw < 0 {
        -1
    } else if raw <= LOW_MURK && low == RuleSource::Engine {
        0
    } else {
        raw.min(MURK_LIMIT)
    }
}

/// What the outfits `owned` (of `records`) add to the murk: count ×
/// `ModVal` for each of their [`MOD_MURK`] slots, summed in wide
/// integers.
#[must_use]
pub fn outfit_murk(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> i32 {
    outfit_mods(owned, records)
        .iter()
        .filter(|m| m.mod_type == MOD_MURK)
        .map(|m| i32::from(m.count) * i32::from(m.mod_val))
        .sum()
}

/// The player's effective murk in a system of `loaded` murk
/// ([`loaded_murk`]) with outfits adding `outfit_term` ([`outfit_murk`]):
/// `loaded` floored at none, plus the term, clamped to 0..=100
/// (`_PlayerEffectiveMurk` @0x6f8f-0x704b).
#[must_use]
pub fn effective_murk(loaded: i16, outfit_term: i32) -> u8 {
    let sum = i64::from(loaded.max(0)) + i64::from(outfit_term);
    u8::try_from(sum.clamp(0, i64::from(MURK_LIMIT))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_catalog::fixture::outfit;

    #[test]
    fn a_murk_loads_capped_at_100_none_below_8_by_the_engine_and_minus_1_below_none() {
        for (raw, engine, bible) in [
            (101, 100, 100),
            (i16::MAX, 100, 100),
            (100, 100, 100),
            (8, 8, 8),
            (7, 0, 7),
            (1, 0, 1),
            (0, 0, 0),
            (-5, -1, -1),
            (-1, -1, -1),
        ] {
            assert_eq!(loaded_murk(raw, RuleSource::Engine), engine, "{raw}");
            assert_eq!(loaded_murk(raw, RuleSource::Bible), bible, "{raw}");
        }
        assert_eq!((MOD_MURK, MURK_LIMIT, LOW_MURK), (28, 100, 7));
    }

    #[test]
    fn the_effective_murk_is_the_loaded_floored_at_none_plus_the_outfits_clamped() {
        assert_eq!(effective_murk(40, 0), 40);
        assert_eq!(effective_murk(40, 25), 65, "the sum");
        assert_eq!(effective_murk(40, -15), 25);
        assert_eq!(
            effective_murk(-1, 30),
            30,
            "the loaded floored at none first"
        );
        assert_eq!(effective_murk(-1, 0), 0);
        assert_eq!(effective_murk(90, 30), 100, "clamped at 100");
        assert_eq!(effective_murk(10, -30), 0, "clamped at none");
        assert_eq!(effective_murk(0, i32::MAX), 100, "never wraps");
        assert_eq!(effective_murk(100, i32::MIN), 0);
    }

    #[test]
    fn outfits_add_count_times_mod_val_over_their_murk_slots() {
        let records = [
            outfit(128, &[(MOD_MURK, 10), (1, 50), (MOD_MURK, 3)]),
            outfit(129, &[(MOD_MURK, -4)]),
            outfit(130, &[(24, 40)]),
        ];
        let owned = |pairs: &[(i16, u16)]| -> BTreeMap<OutfitId, u16> {
            pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
        };
        assert_eq!(
            outfit_murk(&owned(&[(128, 2)]), &records),
            26,
            "2 × (10 + 3)"
        );
        assert_eq!(outfit_murk(&owned(&[(128, 1), (129, 3)]), &records), 1);
        assert_eq!(
            outfit_murk(&owned(&[(130, 5)]), &records),
            0,
            "no murk slot"
        );
        assert_eq!(outfit_murk(&owned(&[]), &records), 0);
    }
}
