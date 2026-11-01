//! A system's murk: how far the player sees before sprites fade into its
//! background, as the original's `_PlayerEffectiveMurk` (@0x6f8f-0x704b)
//! decides it, and the colour they fade toward. The view mixes each
//! sprite toward that colour by its murk (see `nova-view`'s
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
//! - The colour ([`murk_colour`], `_LoadObjectData` @0x77d85-0x77e99): the
//!   `BkgndColor` as RGB555, read only by the murk (sprite +0xb4
//!   @0x2b502, @0x2be12, @0x2c221). Each byte widens to a 16-bit channel
//!   (`byte << 8`, @0x77d8b-0x77dc5), which is masked with 0xfc00
//!   (@0x77e27-0x77e49), has 0x801 added (@0x77e68, @0x77e6e, @0x77e8c)
//!   and gives bits 15..11 (@0x77e73-0x77e97), so each channel is
//!   `(byte >> 3) + 1`: black is 0x0421, not 0. It isn't the message
//!   box's conversion, which adds nothing. A byte of 0xf8 or more wraps
//!   to 0 in the original (the `andl` @0x77e78, @0x77e7e, @0x77e94); ours
//!   stops at 31. No stock system has one.

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

/// The largest 5-bit channel of an RGB555 colour.
const CHANNEL_LIMIT: u32 = 31;

/// A system's `BkgndColor` `background`, raw `00RRGGBB`, as the RGB555
/// pixel a sprite's murk mixes toward: each byte's top 5 bits plus 1,
/// stopping at 31 (see the module docs).
#[must_use]
pub fn murk_colour(background: u32) -> u16 {
    let channel = |shift: u32| (((background >> shift) & 0xff) >> 3).saturating_add(1);
    let rounded = |shift: u32| channel(shift).min(CHANNEL_LIMIT);
    let pixel = (rounded(16) << 10) | (rounded(8) << 5) | rounded(0);
    // Three 5-bit channels: within a `u16`.
    #[allow(clippy::cast_possible_truncation)]
    let pixel = pixel as u16;
    pixel
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

    /// The 5-bit channels of an RGB555 `pixel`.
    fn channels(pixel: u16) -> (u16, u16, u16) {
        (pixel >> 10, (pixel >> 5) & 0x1f, pixel & 0x1f)
    }

    #[test]
    fn a_background_colour_is_each_byte_over_8_plus_1_at_rgb555() {
        assert_eq!(murk_colour(0), 0x0421, "black is 1, 1, 1");
        assert_eq!(murk_colour(0x0019_1900), 0x1081, "Alphara and Obatta");
        assert_eq!(murk_colour(0x0019_090f), 0x1042, "Holpa");
        assert_eq!(murk_colour(0x0000_0c07), 0x0441, "Sondurll");
        assert_eq!(channels(murk_colour(0x0007_0000)), (1, 1, 1));
        assert_eq!(channels(murk_colour(0x0008_0000)), (2, 1, 1));
        assert_eq!(channels(murk_colour(0x0000_0700)), (1, 1, 1));
        assert_eq!(channels(murk_colour(0x0000_0800)), (1, 2, 1));
        assert_eq!(channels(murk_colour(0x0000_0007)), (1, 1, 1));
        assert_eq!(channels(murk_colour(0x0000_0008)), (1, 1, 2));
    }

    #[test]
    fn a_bright_channel_stops_at_31_and_never_wraps() {
        assert_eq!(murk_colour(0x00ff_ffff), 0x7fff);
        assert_eq!(channels(murk_colour(0x00f8_0000)), (31, 1, 1));
        assert_eq!(channels(murk_colour(0x00f0_0000)), (31, 1, 1), "30 + 1");
        assert_eq!(channels(murk_colour(0x00e8_0000)), (30, 1, 1));
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
