//! What IFF and the galaxy map show: what the player's outfits give the
//! radar ([`RadarGear`]), and the colours of a ship ([`ShipIff`]), a
//! stellar ([`StellarIff`]) and a system ([`SystemColorCode`]), from
//! `nova_sim_port::iff`, which re-exports them and keeps the colour codes'
//! rules.

use std::collections::BTreeMap;

use nova_sim_catalog::catalog::{OutfitId, OutfitRecord, outfit_mods};

/// The `ModType` of a density scanner (`_HasDensityScanner` @0x431f).
pub const MOD_DENSITY_SCANNER: i16 = 13;

/// The `ModType` of IFF (`_HasIFF` @0x43a7).
pub const MOD_IFF: i16 = 14;

/// The `ModType` of anti-interference (`_ShipAntiInterference` @0x5f76).
pub const MOD_ANTI_INTERFERENCE: i16 = 24;

/// The most anti-interference counts for, either way (@0x5f9d-0x5fbd).
pub const ANTI_INTERFERENCE_LIMIT: i16 = 100;

/// What the player's outfits give the radar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RadarGear {
    /// Whether an owned outfit gives IFF.
    pub iff: bool,
    /// Whether an owned outfit gives a density scanner.
    pub density_scanner: bool,
    /// The anti-interference the owned outfits sum to, −100..=100.
    pub anti_interference: i16,
}

impl RadarGear {
    /// What the outfits `owned` (of `records`) give the radar: only counts
    /// above 0 count, and every one of an outfit's four mods.
    #[must_use]
    pub fn of(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Self {
        let mods = outfit_mods(owned, records);
        let mods = mods.iter().filter(|m| m.count > 0);
        let mut gear = Self::default();
        let mut anti = 0_i32;
        for m in mods {
            match m.mod_type {
                MOD_IFF => gear.iff = true,
                MOD_DENSITY_SCANNER => gear.density_scanner = true,
                MOD_ANTI_INTERFERENCE => anti += i32::from(m.count) * i32::from(m.mod_val),
                _ => {}
            }
        }
        let limit = i32::from(ANTI_INTERFERENCE_LIMIT);
        gear.anti_interference = i16::try_from(anti.clamp(-limit, limit)).unwrap_or_default();
        gear
    }
}

/// What IFF shows a ship as (`_ColorCodeShip`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipIff {
    /// Disabled or breaking up (`nova_sim_ai::traffic::npc::Npc::is_disabled`):
    /// dark grey.
    Disabled,
    /// Threatening the player: red.
    Threat,
    /// Escorting the player: green.
    Escort,
    /// Any other ship: blue.
    Other,
}

/// What IFF shows a stellar as (`_ColorCodeStellar`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StellarIff {
    /// Not landable, or uninhabited: medium grey.
    Unlandable,
    /// A wormhole: teal.
    Wormhole,
    /// Dominated: green.
    Dominated,
    /// The player may land: yellow.
    Cleared,
    /// The player may not land, with a record of 0 or more: orange.
    Denied,
    /// The player may not land, with a record below 0: red.
    Hostile,
}

/// What the galaxy map frames a system in (`_ColorCodeSystem`; see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemColorCode {
    /// No system stands for it, or its head is unexplored:
    /// `_rgbDarkGrey`.
    Unexplored,
    /// The system standing for it isn't inhabited, or none of its
    /// stellars counts: `_rgbLightGrey`.
    Uninhabited,
    /// A stellar is dominated: `_brightGreen`.
    Dominated,
    /// The player may land on a stellar: blue.
    Cleared,
    /// The player may not land, with a record of 0 or more: orange.
    Denied,
    /// The player may not land, with a record below 0: red.
    Hostile,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_catalog::fixture::outfit;

    fn owned(counts: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        counts.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn the_mod_types_are_the_originals() {
        assert_eq!(
            [MOD_DENSITY_SCANNER, MOD_IFF, MOD_ANTI_INTERFERENCE],
            [13, 14, 24]
        );
    }

    #[test]
    fn iff_and_the_scanner_are_found_in_any_of_the_four_slots() {
        for slot in 0..4 {
            let mut mods = [(0, 0); 4];
            mods[slot] = (MOD_IFF, 0);
            let iff = outfit(128, &mods);
            mods[slot] = (MOD_DENSITY_SCANNER, 0);
            let scanner = outfit(129, &mods);
            let records = [iff, scanner];
            assert_eq!(
                RadarGear::of(&owned(&[(128, 1)]), &records),
                RadarGear {
                    iff: true,
                    ..RadarGear::default()
                },
                "slot {slot}"
            );
            assert_eq!(
                RadarGear::of(&owned(&[(129, 1)]), &records),
                RadarGear {
                    density_scanner: true,
                    ..RadarGear::default()
                },
                "slot {slot}"
            );
        }
    }

    #[test]
    fn an_outfit_owned_none_of_or_not_owned_gives_nothing() {
        let records = [
            outfit(128, &[(MOD_IFF, 0)]),
            outfit(129, &[(MOD_DENSITY_SCANNER, 0)]),
            outfit(130, &[(MOD_ANTI_INTERFERENCE, 30)]),
        ];
        let none = RadarGear::default();
        assert_eq!(
            RadarGear::of(&owned(&[(128, 0), (129, 0), (130, 0)]), &records),
            none
        );
        assert_eq!(RadarGear::of(&owned(&[]), &records), none);
        assert_eq!(RadarGear::of(&owned(&[(131, 5)]), &records), none);
    }

    #[test]
    fn other_mod_types_give_nothing() {
        let records = [outfit(128, &[(12, 30), (15, 30), (23, 30), (25, 30)])];
        assert_eq!(
            RadarGear::of(&owned(&[(128, 2)]), &records),
            RadarGear::default()
        );
    }

    #[test]
    fn anti_interference_sums_count_times_mod_val() {
        let records = [
            outfit(128, &[(MOD_ANTI_INTERFERENCE, 30)]),
            outfit(129, &[(MOD_ANTI_INTERFERENCE, 15), (MOD_IFF, 0)]),
        ];
        let gear = RadarGear::of(&owned(&[(128, 2), (129, 1)]), &records);
        assert_eq!(gear.anti_interference, 75);
        assert!(gear.iff);
    }

    #[test]
    fn anti_interference_clamps_at_100_either_way() {
        let records = [
            outfit(128, &[(MOD_ANTI_INTERFERENCE, 30)]),
            outfit(129, &[(MOD_ANTI_INTERFERENCE, -30)]),
        ];
        let anti =
            |counts: &[(i16, u16)]| RadarGear::of(&owned(counts), &records).anti_interference;
        assert_eq!(anti(&[(128, 3)]), 90);
        assert_eq!(anti(&[(128, 4)]), 100);
        assert_eq!(anti(&[(128, 2000)]), 100, "no 16-bit wrap");
        assert_eq!(anti(&[(129, 3)]), -90);
        assert_eq!(anti(&[(129, 4)]), -100);
        assert_eq!(anti(&[(128, 4), (129, 1)]), 90, "clamped once, summed");
    }
}
