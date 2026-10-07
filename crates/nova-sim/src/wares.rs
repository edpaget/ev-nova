//! The rules the outfitter and the shipyard share: what a stellar's tech
//! level sells, whether an item's `Require` is met, which hiding flag hides
//! it, the hide-higher sweep over `DispWeight`, and the order the rows go
//! in.
//!
//! The Bible gives the same rules for `oütf` and `shïp`, with different
//! flag bits: each caller passes its own ([`HideBits`]).
//!
//! It also keeps the day's rolls the shipyard, the outfitter and the bar
//! share: each item's `BuyRandom` or `HireRandom` read as never, always
//! or a percent chance, and each chance drawn once a landing (see
//! [`hire`](crate::hire)).

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{LandingSite, OutfitId, OutfitRecord};
use crate::chance::Chance;

/// The `BuyRandom` or `HireRandom` from which an item is always on offer:
/// the original's loader clamps above it to it (`_LoadObjectData`).
pub const ALWAYS_RANDOM: i16 = 100;

/// An item's roll for the day, by its `BuyRandom` or `HireRandom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Roll {
    /// Never on offer.
    Never,
    /// Always on offer.
    Always,
    /// On offer on a chance of so many percent.
    Chance(u8),
}

impl Roll {
    /// The roll of an item of `random`, read plainly: 0 or less is never,
    /// [`ALWAYS_RANDOM`] or more always, and any other a chance.
    pub(crate) fn of(random: i16) -> Self {
        if random <= 0 {
            Self::Never
        } else if random >= ALWAYS_RANDOM {
            Self::Always
        } else {
            Self::Chance(random as u8)
        }
    }
}

/// Each item's roll since the last landing, keyed by `K`: a chance is
/// drawn the first time it is asked and kept until cleared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DayRolls<K>(BTreeMap<K, bool>);

impl<K> Default for DayRolls<K> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<K: Ord> DayRolls<K> {
    /// Whether `key`'s item, rolling `roll`, is on offer today: never and
    /// always draw nothing; a chance is the answer kept, or else drawn on
    /// `chance` and kept.
    pub(crate) fn today(&mut self, key: K, roll: Roll, chance: &mut dyn Chance) -> bool {
        match roll {
            Roll::Never => false,
            Roll::Always => true,
            Roll::Chance(percent) => *self.0.entry(key).or_insert_with(|| chance.fires(percent)),
        }
    }

    /// Forgets `key`'s roll, so it is drawn again when next asked.
    pub(crate) fn redraw(&mut self, key: &K) {
        self.0.remove(key);
    }

    /// Forgets every roll.
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}

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
    use crate::chance::NeverFires;
    use crate::testkit::{Scripted, outfit, planet};

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
    fn a_rolls_chance_is_its_random_up_to_always() {
        assert_eq!(Roll::of(-1), Roll::Never);
        assert_eq!(Roll::of(0), Roll::Never);
        assert_eq!(Roll::of(1), Roll::Chance(1));
        assert_eq!(Roll::of(50), Roll::Chance(50));
        assert_eq!(Roll::of(99), Roll::Chance(99));
        assert_eq!(Roll::of(100), Roll::Always);
        assert_eq!(Roll::of(250), Roll::Always);
    }

    #[test]
    fn a_chance_roll_is_drawn_once_and_then_kept() {
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[true, false]);
        assert!(rolls.today(7, Roll::Chance(40), &mut chance));
        assert!(rolls.today(7, Roll::Chance(40), &mut chance), "kept");
        assert_eq!(chance.asked, [40]);
        assert!(!rolls.today(8, Roll::Chance(60), &mut chance));
        assert!(!rolls.today(8, Roll::Chance(60), &mut chance), "kept");
        assert_eq!(chance.asked, [40, 60]);
    }

    #[test]
    fn never_and_always_draw_nothing() {
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[true]);
        assert!(!rolls.today(7, Roll::Never, &mut chance));
        assert!(rolls.today(8, Roll::Always, &mut NeverFires));
        assert!(chance.asked.is_empty());
        assert_eq!(rolls, DayRolls::default(), "nothing stored");
        assert!(rolls.today(7, Roll::Chance(5), &mut chance));
        assert_eq!(chance.asked, [5], "drawn afresh");
    }

    #[test]
    fn a_roll_redrawn_is_drawn_again_and_cleared_ones_all_are() {
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[true, false, true, false]);
        assert!(rolls.today(7, Roll::Chance(30), &mut chance));
        assert!(!rolls.today(8, Roll::Chance(20), &mut chance));
        rolls.redraw(&7);
        assert!(rolls.today(7, Roll::Chance(30), &mut chance));
        assert!(!rolls.today(8, Roll::Chance(20), &mut chance), "kept");
        assert_eq!(chance.asked, [30, 20, 30]);
        rolls.clear();
        assert_eq!(rolls, DayRolls::default());
        assert!(!rolls.today(7, Roll::Chance(30), &mut chance));
        assert_eq!(chance.asked, [30, 20, 30, 30]);
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
