//! The rules the outfitter and the shipyard share: what a stellar's tech
//! level sells, whether an item's `Require` is met, which hiding flag hides
//! it, the hide-higher sweep over `DispWeight`, and the order the rows go
//! in.
//!
//! The Bible gives the same rules for `oütf` and `shïp`, with different
//! flag bits: each caller passes its own ([`HideBits`]).

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{LandingSite, OutfitId, OutfitRecord};

/// Whether an item of `tech_level` is for sale, by tech level alone, at
/// `site`: at or below its `TechLevel`, or exactly one of its eight
/// `SpecialTech` slots (a slot of 0 or below is unused).
#[must_use]
pub fn tech_allows(tech_level: i16, site: &LandingSite) -> bool {
    tech_level <= site.tech_level
        || site
            .special_tech
            .iter()
            .any(|&slot| slot > 0 && slot == tech_level)
}

/// The `Contribute` bits of a ship contributing `ship`, carrying `owned`:
/// the ship's, and those of each outfit owned that has a record.
#[must_use]
pub fn contributed(ship: u64, owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> u64 {
    owned
        .keys()
        .filter_map(|id| records.iter().find(|record| record.id == *id))
        .fold(ship, |bits, record| bits | record.contribute)
}

/// Whether `require` is met by the bits `contributed`: each of its bits is
/// among them.
#[must_use]
pub fn requirement_met(require: u64, contributed: u64) -> bool {
    require & !contributed == 0
}

/// The flag bits an item kind hides with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HideBits {
    /// Hides it when its `Require` is not met.
    pub unless_required: u16,
    /// Hides it when its `Availability` does not hold.
    pub unless_available: u16,
}

/// Whether an item with `flags` is hidden, given its kind's hiding `bits`,
/// whether its `Require` is met (`required`) and whether its
/// `Availability` holds (`available`).
#[must_use]
pub fn hidden(flags: u16, bits: HideBits, required: bool, available: bool) -> bool {
    (flags & bits.unless_required != 0 && !required)
        || (flags & bits.unless_available != 0 && !available)
}

/// The hide-higher sweep: walked by ascending ID, an item flagged to hide
/// higher ones that can be bought takes every later item of its
/// `DispWeight` off sale.
#[derive(Clone, Debug, Default)]
pub struct HideHigher {
    taken_off: BTreeSet<i16>,
}

impl HideHigher {
    /// Whether an item of `disp_weight` is still on sale.
    #[must_use]
    pub fn on_sale(&self, disp_weight: i16) -> bool {
        !self.taken_off.contains(&disp_weight)
    }

    /// Notes an item of `disp_weight`: when it `hides_higher` and is
    /// `buyable`, later ones of its weight go off sale.
    pub fn note(&mut self, disp_weight: i16, hides_higher: bool, buyable: bool) {
        if hides_higher && buyable {
            self.taken_off.insert(disp_weight);
        }
    }
}

/// Puts `rows` in display order: highest `DispWeight` first, then by ID,
/// each row's weight and ID given by `key`.
pub fn in_display_order<T>(rows: &mut [T], key: impl Fn(&T) -> (i16, i16)) {
    rows.sort_by_key(|row| {
        let (weight, id) = key(row);
        (std::cmp::Reverse(weight), id)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{outfit, planet};

    /// Tech level 4, with special tech 6 and 55.
    fn site() -> LandingSite {
        let mut special_tech = [0; 8];
        special_tech[..2].copy_from_slice(&[6, 55]);
        LandingSite {
            tech_level: 4,
            special_tech,
            ..planet(128, 0.0, 0.0)
        }
    }

    #[test]
    fn the_tech_level_sells_up_to_it_and_special_tech_exactly() {
        let allowed: Vec<i16> = [-3, 0, 3, 4, 5, 6, 7, 55, 56]
            .into_iter()
            .filter(|&level| tech_allows(level, &site()))
            .collect();
        assert_eq!(allowed, [-3, 0, 3, 4, 6, 55]);
        let unused = LandingSite {
            tech_level: -5,
            special_tech: [0, -1, 0, 0, 0, 0, 0, 0],
            ..site()
        };
        assert!(!tech_allows(0, &unused));
        assert!(!tech_allows(-1, &unused));
        assert!(tech_allows(-5, &unused));
    }

    #[test]
    fn the_ship_and_each_owned_outfit_with_a_record_contribute() {
        let records = [
            OutfitRecord {
                contribute: 0x10,
                ..outfit(128, &[])
            },
            OutfitRecord {
                contribute: 0x0100_0000_0000,
                ..outfit(129, &[])
            },
        ];
        let owned: BTreeMap<OutfitId, u16> = [(OutfitId(128), 1), (OutfitId(999), 2)]
            .into_iter()
            .collect();
        assert_eq!(contributed(0x1, &owned, &records), 0x11);
        assert_eq!(contributed(0x1, &BTreeMap::new(), &records), 0x1);
        let both: BTreeMap<OutfitId, u16> = [(OutfitId(128), 1), (OutfitId(129), 1)]
            .into_iter()
            .collect();
        assert_eq!(contributed(0, &both, &records), 0x0100_0000_0010);
    }

    #[test]
    fn a_requirement_is_met_when_each_of_its_bits_is_contributed() {
        assert!(requirement_met(0, 0));
        assert!(requirement_met(0x3, 0x7));
        assert!(!requirement_met(0x3, 0x1));
        assert!(!requirement_met(
            0x8000_0000_0000_0000,
            0x7FFF_FFFF_FFFF_FFFF
        ));
    }

    #[test]
    fn each_kind_hides_with_its_own_bits() {
        let bits = HideBits {
            unless_required: 0x0200,
            unless_available: 0x0100,
        };
        for (flags, required, available, hides) in [
            (0, false, false, false),
            (0x0200, false, true, true),
            (0x0200, true, false, false),
            (0x0100, true, false, true),
            (0x0100, false, true, false),
            (0x0300, true, true, false),
            (0x4000, false, false, false),
            (!0x0300, false, false, false),
        ] {
            assert_eq!(
                hidden(flags, bits, required, available),
                hides,
                "{flags:#06x} {required} {available}"
            );
        }
    }

    #[test]
    fn a_buyable_hider_takes_its_weight_off_sale() {
        let mut sweep = HideHigher::default();
        assert!(sweep.on_sale(5));
        sweep.note(5, true, false);
        sweep.note(6, false, true);
        assert!(sweep.on_sale(5), "not buyable");
        assert!(sweep.on_sale(6), "not a hider");
        sweep.note(5, true, true);
        assert!(!sweep.on_sale(5));
        assert!(sweep.on_sale(6));
    }

    #[test]
    fn rows_go_by_weight_highest_first_then_by_id() {
        let mut rows = [(10, 128), (50, 131), (50, 129), (-1, 130), (10, 132)];
        in_display_order(&mut rows, |&row| row);
        assert_eq!(
            rows,
            [(50, 129), (50, 131), (10, 128), (10, 132), (-1, 130)]
        );
    }
}
