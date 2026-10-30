//! The rules the outfitter and the shipyard share: what a stellar's tech
//! level sells ([`by_tech_level`], behind the outfit's and the ship's
//! own guards) and, for an outfit, what Buy's own tech test lets be bought
//! ([`outfit_buy_tech_allows`]), whether an item's `Require` is met, which hiding flag hides
//! it, the hide-higher sweep over `DispWeight`, and the order the rows go
//! in.
//!
//! The Bible gives the same rules for `oütf` and `shïp`, with different
//! flag bits: each caller passes its own ([`HideBits`]).
//!
//! It also keeps the day's rolls the shipyard, the outfitter and the bar
//! share: each item's `BuyRandom` or `HireRandom` read as never, always
//! or a percent chance, and each chance drawn once a day by the daily
//! step (`draw_day`), or by the other reading of
//! [`RuleKey::DayRolls`](crate::RuleKey::DayRolls) once a landing (see
//! [`hire`](crate::hire) and the [`story`](crate::story) module's "Built
//! by phase 6").

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{CronRecord, LandingSite, OutfitId, OutfitRecord, RankRecord, ShipId};
use crate::chance::Chance;
use crate::cron;
pub use crate::cron::requirement_met;
use crate::pilot::Pilot;
use crate::rank;

/// The `BuyRandom` or `HireRandom` from which an item is always on offer:
/// the original's loader clamps above it to it (`_LoadObjectData`).
pub const ALWAYS_RANDOM: i16 = 100;

/// The unit a shop's info box puts after a number of tons, as
/// `_LoadStrings` loads them @0x71ce0-0x71d15.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TonUnit {
    /// `_tonString` (`STR#` 2002 #1, "ton").
    Ton,
    /// `_tonsString` (`STR#` 2002 #2, "tons").
    Tons,
}

impl TonUnit {
    /// The unit after `tons`: [`TonUnit::Ton`] at exactly 1, else
    /// [`TonUnit::Tons`].
    #[must_use]
    pub fn of(tons: i64) -> Self {
        if tons == 1 { Self::Ton } else { Self::Tons }
    }
}

/// A number of tons as a shop's info box shows it, with its unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownTons {
    /// The number shown.
    pub tons: i64,
    /// The unit after it.
    pub unit: TonUnit,
}

impl ShownTons {
    /// `tons` with the unit [`TonUnit::of`] picks for it.
    #[must_use]
    pub fn of(tons: i64) -> Self {
        Self {
            tons,
            unit: TonUnit::of(tons),
        }
    }
}

/// An item's roll for the day, by its `BuyRandom` or `HireRandom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Roll {
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

/// Each item's roll for the day, keyed by `K`: a chance drawn by the day
/// ([`DayRolls::draw`]), or else the first time it is asked, and kept
/// until drawn again or cleared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayRolls<K>(BTreeMap<K, bool>);

impl<K> Default for DayRolls<K> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<K: Ord> DayRolls<K> {
    /// Whether `key`'s item, rolling `roll`, is on offer today: never and
    /// always draw nothing; a chance is the answer kept, or else drawn on
    /// `chance` and kept.
    pub fn today(&mut self, key: K, roll: Roll, chance: &mut dyn Chance) -> bool {
        match roll {
            Roll::Never => false,
            Roll::Always => true,
            Roll::Chance(percent) => *self.0.entry(key).or_insert_with(|| chance.fires(percent)),
        }
    }

    /// Holds `key`'s item on offer for the rest of the stay, whatever a
    /// chance would draw; a roll of never still never is.
    pub fn hold(&mut self, key: K) {
        self.0.insert(key, true);
    }

    /// Forgets `key`'s roll, so it is drawn again when next asked.
    pub fn redraw(&mut self, key: &K) {
        self.0.remove(key);
    }

    /// Draws `key`'s item, rolling `roll`, now: a chance is drawn on
    /// `chance` and kept, replacing any answer kept or held; never and
    /// always draw and keep nothing.
    pub(crate) fn draw(&mut self, key: K, roll: Roll, chance: &mut dyn Chance) {
        if let Roll::Chance(percent) = roll {
            self.0.insert(key, chance.fires(percent));
        }
    }

    /// Forgets every roll.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// The day's draw of the three day rolls, as the original's daily step
/// draws them (`_IncrementGameTime` @0xb7c4-0xb833): every roll is
/// forgotten, a held one too, then each ship class of `ships` (its ID,
/// hire roll and buy roll) by ascending ID draws its `hire` roll and then
/// its `buy` roll, and then each outfit of `items` by ascending ID its
/// `outfits` roll. Only a chance draws (see [`DayRolls::draw`]).
pub fn draw_day(
    hire: &mut DayRolls<ShipId>,
    buy: &mut DayRolls<ShipId>,
    outfits: &mut DayRolls<OutfitId>,
    ships: impl IntoIterator<Item = (ShipId, Roll, Roll)>,
    items: impl IntoIterator<Item = (OutfitId, Roll)>,
    chance: &mut dyn Chance,
) {
    hire.clear();
    buy.clear();
    outfits.clear();
    let mut ships: Vec<_> = ships.into_iter().collect();
    ships.sort_by_key(|&(id, _, _)| id);
    for (id, hire_roll, buy_roll) in ships {
        hire.draw(id, hire_roll, chance);
        buy.draw(id, buy_roll, chance);
    }
    let mut items: Vec<_> = items.into_iter().collect();
    items.sort_by_key(|&(id, _)| id);
    for (id, roll) in items {
        outfits.draw(id, roll, chance);
    }
}

/// Whether an item of `tech_level` is for sale, by tech level alone, at
/// `site`: at or below its `TechLevel`, compared signed, or exactly one
/// of its eight `SpecialTech` slots, each compared raw, so a slot of 0 or
/// below matches an item of that tech too. Outfits and ships share it
/// (`_SetupPortAvailableItems` @0xbfa7-0xbfbd,
/// `_SetupPortAvailableShipTypes` @0xbc5c-0xbc72), each behind its own
/// guard ([`outfit_tech_allows`], [`ship_tech_allows`]); it is the
/// Bible's reading for both
/// ([`RuleKey::TechLevel`](crate::RuleKey::TechLevel)).
#[must_use]
pub fn by_tech_level(tech_level: i16, site: &LandingSite) -> bool {
    tech_level <= site.tech_level || site.special_tech.contains(&tech_level)
}

/// Whether `site` has no tech, as `_DoOutfitDialog` reads it to hide the
/// outfitter's Buy (`*0xded64`, @0x5bb67-0x5bb9d): a `TechLevel` of
/// exactly 0 (a negative one keeps Buy) and no `SpecialTech` slot above 0
/// (`jg` @0x5bb88), so a slot of 0 or below counts as none.
#[must_use]
pub fn has_no_tech(site: &LandingSite) -> bool {
    site.tech_level == 0 && !site.special_tech.iter().any(|&slot| slot > 0)
}

/// The outfit `TechLevel` tech never sells: `_SetupPortAvailableItems`
/// skips the tech test for it (`cmpw $0x7fff` @0xbf93).
pub const OUTFIT_TECH_NEVER: i16 = i16::MAX;

/// Whether an outfit of `tech_level` is for sale, by tech level alone, at
/// `site`, as the engine reads it: never below 0 or at
/// [`OUTFIT_TECH_NEVER`] (`_SetupPortAvailableItems` @0xbf88-0xbf9d),
/// otherwise as [`by_tech_level`] says.
#[must_use]
pub fn outfit_tech_allows(tech_level: i16, site: &LandingSite) -> bool {
    tech_level >= 0 && tech_level != OUTFIT_TECH_NEVER && by_tech_level(tech_level, site)
}

/// The outfit `TechLevel` that `_CanBuyOutfitItem` never tests by tech:
/// Buy skips its tech test for it (`cmpw $-0x1` @0x4e802).
pub const OUTFIT_TECH_ANY: i16 = -1;

/// Whether `_CanBuyOutfitItem`'s own tech test lets an outfit of
/// `tech_level` be bought at `site` (@0x4e7ff-0x4e83d): always at
/// [`OUTFIT_TECH_ANY`], otherwise as [`by_tech_level`] says. Unlike
/// [`outfit_tech_allows`] it has no `>= 0` and no 0x7fff test, so an
/// outfit listed off tech can still pass it.
#[must_use]
pub fn outfit_buy_tech_allows(tech_level: i16, site: &LandingSite) -> bool {
    tech_level == OUTFIT_TECH_ANY || by_tech_level(tech_level, site)
}

/// Whether a ship class of `tech_level` is for sale or hire, by tech
/// level alone, at `site`, as the engine reads it: never below 0
/// (`_SetupPortAvailableShipTypes` @0xbc4b-0xbc52, with no 0x7fff test),
/// otherwise as [`by_tech_level`] says.
#[must_use]
pub fn ship_tech_allows(tech_level: i16, site: &LandingSite) -> bool {
    tech_level >= 0 && by_tech_level(tech_level, site)
}

/// Everything whose `Contribute` bits the player may hold
/// (`_GetPlayerContributeBits` @0x76b2): the ship class flown, the
/// `oütf`s, the `crön`s and the `ränk`s.
#[derive(Clone, Copy, Debug)]
pub struct Contributors<'a> {
    /// The ship class's `Contribute` bits.
    pub ship: u64,
    /// Every `oütf`.
    pub outfits: &'a [OutfitRecord],
    /// Every `crön`.
    pub crons: &'a [CronRecord],
    /// Every `ränk`.
    pub ranks: &'a [RankRecord],
}

/// The `Contribute` bits `pilot` holds, `by` the ship flown and the
/// records: the ship's, those of each outfit owned that has a record, and
/// those of each crön running outside its holdoffs that has a record
/// ([`cron::contributes`](crate::cron::contributes)), and those of each
/// active rank that has a record, whatever its government
/// ([`rank::contributed`](crate::rank::contributed)).
#[must_use]
pub fn contributed(by: &Contributors<'_>, pilot: &Pilot) -> u64 {
    let outfits = pilot
        .outfits
        .keys()
        .filter_map(|id| by.outfits.iter().find(|record| record.id == *id))
        .map(|record| record.contribute);
    let crons = pilot
        .crons
        .iter()
        .filter(|(_, state)| cron::contributes(**state))
        .filter_map(|(id, _)| by.crons.iter().find(|record| record.id == *id))
        .map(|record| record.contribute);
    outfits.chain(crons).fold(by.ship, |bits, more| bits | more)
        | rank::contributed(by.ranks, &pilot.ranks)
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
/// higher ones that is listed takes every later item of its `DispWeight`
/// off sale. The shipyard, the bar and the outfitter each pass whether the
/// item is listed, greyed or not, as the original's sweeps read only the
/// listing byte (`_SetupPortAvailableShipTypes` @0xbdc3-0xbe2c,
/// `_SetupPortAvailableItems` @0xc11f-0xc175).
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
    /// `listed`, later ones of its weight go off sale.
    pub fn note(&mut self, disp_weight: i16, hides_higher: bool, listed: bool) {
        if hides_higher && listed {
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
    use crate::catalog::{CronId, RankId, RankRecord};
    use crate::chance::NeverFires;
    use crate::cron::CronState;
    use nova_sim_base::fixture::Scripted;
    use nova_sim_catalog::fixture::{catalog, cron, outfit, planet, rank};

    #[test]
    fn a_ton_count_takes_the_singular_unit_at_exactly_one() {
        assert_eq!(TonUnit::of(1), TonUnit::Ton);
        for tons in [0, 2, -1] {
            assert_eq!(TonUnit::of(tons), TonUnit::Tons, "{tons}");
        }
        assert_eq!(
            ShownTons::of(1),
            ShownTons {
                tons: 1,
                unit: TonUnit::Ton
            }
        );
    }

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
            .filter(|&level| by_tech_level(level, &site()))
            .collect();
        assert_eq!(allowed, [-3, 0, 3, 4, 6, 55]);
        let unused = LandingSite {
            tech_level: -5,
            special_tech: [0, -1, 0, 0, 0, 0, 0, 0],
            ..site()
        };
        assert!(by_tech_level(0, &unused), "a slot of 0 matches");
        assert!(by_tech_level(-1, &unused), "a slot of -1 matches");
        assert!(by_tech_level(-5, &unused));
    }

    /// Tech level `tech_level`, with `slots` as its first special tech.
    fn teching(tech_level: i16, slots: &[i16]) -> LandingSite {
        let mut special_tech = [0; 8];
        special_tech[..slots.len()].copy_from_slice(slots);
        LandingSite {
            tech_level,
            special_tech,
            ..site()
        }
    }

    #[test]
    fn an_outfit_below_tech_0_or_of_tech_0x7fff_is_never_sold_by_tech() {
        let low = teching(5, &[-3]);
        let high = teching(i16::MAX, &[i16::MAX]);
        for level in [-3, -1, i16::MIN, i16::MAX] {
            assert!(!outfit_tech_allows(level, &low), "{level} at 5");
            assert!(!outfit_tech_allows(level, &high), "{level} at 0x7fff");
        }
        for level in [0, 5] {
            assert!(outfit_tech_allows(level, &low), "{level} at 5");
            assert!(outfit_tech_allows(level, &high), "{level} at 0x7fff");
        }
        assert!(!outfit_tech_allows(6, &low));
        assert!(outfit_tech_allows(i16::MAX - 1, &high));
        assert_eq!(OUTFIT_TECH_NEVER, 0x7fff);
    }

    #[test]
    fn the_buy_tech_skip_is_tech_minus_1() {
        assert_eq!(OUTFIT_TECH_ANY, -1);
    }

    #[test]
    fn an_outfit_of_tech_minus_1_always_passes_buys_tech_test() {
        for tech in [4, -5, i16::MIN] {
            let at = teching(tech, &[]);
            assert!(!at.special_tech.contains(&-1));
            assert!(outfit_buy_tech_allows(-1, &at), "at {tech}");
        }
    }

    #[test]
    fn buys_tech_test_has_no_sign_or_0x7fff_test() {
        let passes = [
            (-3, teching(4, &[])),
            (-3, teching(-5, &[-3])),
            (i16::MAX, teching(i16::MAX, &[])),
            (i16::MAX, teching(4, &[i16::MAX])),
            (6, teching(4, &[6])),
        ];
        for (level, at) in &passes {
            assert!(outfit_buy_tech_allows(*level, at), "{level} at {at:?}");
        }
        let refused = [
            (-3, teching(-5, &[])),
            (-2, teching(-5, &[])),
            (i16::MAX, teching(4, &[])),
            (9, teching(4, &[])),
        ];
        for (level, at) in &refused {
            assert!(!outfit_buy_tech_allows(*level, at), "{level} at {at:?}");
        }
    }

    #[test]
    fn every_outfit_tech_lists_passes_buys_tech_test() {
        let levels = [
            i16::MIN,
            -5,
            -3,
            -1,
            0,
            3,
            4,
            5,
            6,
            9,
            55,
            i16::MAX - 1,
            i16::MAX,
        ];
        let sites = [
            teching(4, &[6, 55]),
            teching(-5, &[0, -1]),
            teching(i16::MAX, &[i16::MAX]),
        ];
        for at in &sites {
            for level in levels {
                if outfit_tech_allows(level, at) {
                    assert!(outfit_buy_tech_allows(level, at), "{level} at {at:?}");
                }
            }
        }
    }

    #[test]
    fn a_ship_below_tech_0_is_never_sold_by_tech_but_0x7fff_is() {
        let slotted = teching(5, &[-3, -1, i16::MIN]);
        for level in [-3, -1, i16::MIN] {
            assert!(!ship_tech_allows(level, &slotted), "{level}");
            assert!(by_tech_level(level, &slotted), "{level}: its slot");
        }
        assert!(ship_tech_allows(0, &slotted));
        assert!(ship_tech_allows(5, &slotted));
        assert!(!ship_tech_allows(6, &slotted));
        assert!(ship_tech_allows(i16::MAX, &teching(i16::MAX, &[])));
        assert!(ship_tech_allows(i16::MAX, &teching(5, &[i16::MAX])));
        assert!(!ship_tech_allows(i16::MAX, &teching(5, &[])));
    }

    #[test]
    fn a_special_tech_slot_of_0_sells_tech_0() {
        let site = teching(-5, &[0, -1]);
        assert!(outfit_tech_allows(0, &site));
        assert!(ship_tech_allows(0, &site));
    }

    #[test]
    fn by_tech_level_compares_every_slot_raw() {
        let site = teching(-5, &[0, -1]);
        for level in [0, -1, -5, -6] {
            assert!(by_tech_level(level, &site), "{level}");
        }
        for level in [-4, 1] {
            assert!(!by_tech_level(level, &site), "{level}");
        }
        assert!(by_tech_level(9, &teching(-5, &[0, 0, 0, 0, 0, 0, 0, 9])));
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
        let by = |ship| Contributors {
            ship,
            outfits: &records,
            crons: &[],
            ranks: &[],
        };
        let mut pilot = owning(&[(128, 1), (999, 2)]);
        assert_eq!(contributed(&by(0x1), &pilot), 0x11);
        pilot.outfits.clear();
        assert_eq!(contributed(&by(0x1), &pilot), 0x1);
        let both = owning(&[(128, 1), (129, 1)]);
        assert_eq!(contributed(&by(0), &both), 0x0100_0000_0010);
    }

    /// What `pilot` holds with `crons` set to `states`, from a ship of no
    /// bits, crön 128 contributing 0x4 and crön 129 0x8.
    fn with_crons(states: &[(i16, i16, i16)]) -> u64 {
        let crons = [
            CronRecord {
                contribute: 0x4,
                ..cron(128)
            },
            CronRecord {
                contribute: 0x8,
                ..cron(129)
            },
        ];
        let by = Contributors {
            ship: 0,
            outfits: &[],
            crons: &crons,
            ranks: &[],
        };
        let mut pilot = owning(&[]);
        pilot.crons = states
            .iter()
            .map(|&(id, days, holdoff)| (CronId(id), CronState { days, holdoff }))
            .collect();
        contributed(&by, &pilot)
    }

    #[test]
    fn a_running_cron_contributes_its_bits() {
        assert_eq!(with_crons(&[(128, 3, 0)]), 0x4);
        assert_eq!(with_crons(&[(128, 0, -1)]), 0x4, "holdoff below 0");
        assert_eq!(with_crons(&[(128, 3, 0), (129, 1, 0)]), 0xC);
    }

    #[test]
    fn bits_given_twice_are_held_once() {
        let outfits = [OutfitRecord {
            contribute: 0x5,
            ..outfit(128, &[])
        }];
        let crons = [CronRecord {
            contribute: 0x6,
            ..cron(128)
        }];
        let by = Contributors {
            ship: 0x3,
            outfits: &outfits,
            crons: &crons,
            ranks: &[],
        };
        let mut pilot = owning(&[(128, 1)]);
        pilot.crons.insert(
            CronId(128),
            CronState {
                days: 1,
                holdoff: 0,
            },
        );
        assert_eq!(contributed(&by, &pilot), 0x7);
    }

    #[test]
    fn a_cron_holding_off_contributes_nothing() {
        assert_eq!(with_crons(&[(128, 3, 2)]), 0, "in its pre-holdoff");
        assert_eq!(with_crons(&[(128, 3, 1)]), 0, "a day of pre-holdoff");
        assert_eq!(with_crons(&[(129, -1, 3)]), 0, "in its post-holdoff");
        assert_eq!(with_crons(&[(128, 3, 2), (129, 1, 0)]), 0x8);
    }

    /// What `pilot` holds with ranks `active`, from a ship of no bits,
    /// rank 200 of government 128 contributing 0x10 and rank 201,
    /// unaffiliated, 0x21, a bit the ship gives too.
    fn with_ranks(active: &[i16]) -> u64 {
        let ranks = [
            RankRecord {
                contribute: 0x10,
                affil_govt: Some(crate::catalog::GovtId(128)),
                ..rank(200)
            },
            RankRecord {
                contribute: 0x21,
                ..rank(201)
            },
        ];
        let by = Contributors {
            ship: 0x1,
            outfits: &[],
            crons: &[],
            ranks: &ranks,
        };
        let mut pilot = owning(&[]);
        pilot.ranks = active.iter().copied().map(RankId).collect();
        contributed(&by, &pilot)
    }

    #[test]
    fn an_active_rank_contributes_its_bits_whatever_its_government() {
        assert_eq!(with_ranks(&[200]), 0x11);
        assert_eq!(with_ranks(&[201]), 0x21, "the ship's bit held once");
        assert_eq!(with_ranks(&[200, 201]), 0x31);
    }

    #[test]
    fn an_inactive_rank_or_one_with_no_record_contributes_nothing() {
        assert_eq!(with_ranks(&[]), 0x1);
        assert_eq!(with_ranks(&[250]), 0x1);
    }

    #[test]
    fn a_cron_whose_record_is_gone_contributes_nothing() {
        assert_eq!(with_crons(&[(300, 3, 0)]), 0);
    }

    /// A new pilot owning `owned`, each an outfit ID with its count.
    fn owning(owned: &[(i16, u16)]) -> Pilot {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        pilot.outfits = owned.iter().map(|&(id, n)| (OutfitId(id), n)).collect();
        pilot
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
    fn a_listed_hider_takes_its_weight_off_sale() {
        let mut sweep = HideHigher::default();
        assert!(sweep.on_sale(5));
        sweep.note(5, true, false);
        sweep.note(6, false, true);
        assert!(sweep.on_sale(5), "not listed");
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
    fn a_held_roll_holds_without_a_draw_but_never_stays_never() {
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[false]);
        rolls.hold(7);
        assert!(rolls.today(7, Roll::Chance(30), &mut chance));
        assert!(!rolls.today(7, Roll::Never, &mut chance));
        assert!(chance.asked.is_empty());
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
    fn a_drawn_chance_is_drawn_now_and_kept() {
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[true]);
        rolls.draw(7, Roll::Chance(40), &mut chance);
        assert_eq!(chance.asked, [40], "drawn now");
        assert!(rolls.today(7, Roll::Chance(40), &mut chance), "kept");
        assert_eq!(chance.asked, [40]);
    }

    #[test]
    fn a_draw_replaces_a_kept_or_held_answer() {
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[true, false, false]);
        assert!(rolls.today(7, Roll::Chance(40), &mut chance));
        rolls.draw(7, Roll::Chance(40), &mut chance);
        assert!(!rolls.today(7, Roll::Chance(40), &mut chance), "replaced");
        rolls.hold(8);
        rolls.draw(8, Roll::Chance(60), &mut chance);
        assert!(!rolls.today(8, Roll::Chance(60), &mut chance), "no hold");
        assert_eq!(chance.asked, [40, 40, 60]);
    }

    #[test]
    fn a_never_or_always_draw_draws_and_stores_nothing() {
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[true]);
        rolls.draw(7, Roll::Never, &mut chance);
        rolls.draw(8, Roll::Always, &mut chance);
        assert!(chance.asked.is_empty());
        assert_eq!(rolls, DayRolls::default(), "nothing stored");
    }

    /// The three day rolls, empty.
    fn day() -> (DayRolls<ShipId>, DayRolls<ShipId>, DayRolls<OutfitId>) {
        Default::default()
    }

    /// Ships 130 (hire 50, buy 60) and 129 (hire 30, buy 40), out of
    /// order.
    fn day_ships() -> Vec<(ShipId, Roll, Roll)> {
        vec![
            (ShipId(130), Roll::Chance(50), Roll::Chance(60)),
            (ShipId(129), Roll::Chance(30), Roll::Chance(40)),
        ]
    }

    /// Outfits 129 (70) and 128 (80), out of order.
    fn day_items() -> Vec<(OutfitId, Roll)> {
        vec![
            (OutfitId(129), Roll::Chance(70)),
            (OutfitId(128), Roll::Chance(80)),
        ]
    }

    #[test]
    fn a_day_draws_each_ships_hire_then_buy_by_id_then_each_outfit() {
        let (mut hire, mut buy, mut outfits) = day();
        let mut chance = Scripted::answering(&[true, false, false, true, true, false]);
        draw_day(
            &mut hire,
            &mut buy,
            &mut outfits,
            day_ships(),
            day_items(),
            &mut chance,
        );
        assert_eq!(chance.asked, [30, 40, 50, 60, 80, 70]);
        let mut none = Scripted::default();
        assert!(hire.today(ShipId(129), Roll::Chance(30), &mut none));
        assert!(!buy.today(ShipId(129), Roll::Chance(40), &mut none));
        assert!(!hire.today(ShipId(130), Roll::Chance(50), &mut none));
        assert!(buy.today(ShipId(130), Roll::Chance(60), &mut none));
        assert!(outfits.today(OutfitId(128), Roll::Chance(80), &mut none));
        assert!(!outfits.today(OutfitId(129), Roll::Chance(70), &mut none));
        assert!(none.asked.is_empty(), "each kept");
    }

    #[test]
    fn a_day_skips_never_and_always_but_keeps_the_order() {
        let (mut hire, mut buy, mut outfits) = day();
        let mut chance = Scripted::default();
        let ships = vec![
            (ShipId(130), Roll::Never, Roll::Chance(60)),
            (ShipId(129), Roll::Chance(30), Roll::Always),
        ];
        let items = vec![
            (OutfitId(129), Roll::Always),
            (OutfitId(128), Roll::Chance(80)),
        ];
        draw_day(&mut hire, &mut buy, &mut outfits, ships, items, &mut chance);
        assert_eq!(chance.asked, [30, 60, 80]);
    }

    #[test]
    fn a_day_forgets_a_hold_and_every_roll_not_drawn() {
        let (mut hire, mut buy, mut outfits) = day();
        outfits.hold(OutfitId(128));
        outfits.hold(OutfitId(200));
        hire.hold(ShipId(200));
        buy.hold(ShipId(200));
        let mut chance = Scripted::default();
        draw_day(
            &mut hire,
            &mut buy,
            &mut outfits,
            day_ships(),
            day_items(),
            &mut chance,
        );
        let mut none = Scripted::default();
        assert!(
            !outfits.today(OutfitId(128), Roll::Chance(80), &mut none),
            "drawn again: the hold is gone"
        );
        assert!(none.asked.is_empty());
        assert!(!outfits.today(OutfitId(200), Roll::Chance(5), &mut none));
        assert!(!hire.today(ShipId(200), Roll::Chance(5), &mut none));
        assert!(!buy.today(ShipId(200), Roll::Chance(5), &mut none));
        assert_eq!(none.asked, [5, 5, 5], "forgotten, so drawn when asked");
    }

    #[test]
    fn a_second_day_draws_everything_again() {
        let (mut hire, mut buy, mut outfits) = day();
        let mut chance = Scripted::default();
        for _ in 0..2 {
            draw_day(
                &mut hire,
                &mut buy,
                &mut outfits,
                day_ships(),
                day_items(),
                &mut chance,
            );
        }
        assert_eq!(
            chance.asked,
            [30, 40, 50, 60, 80, 70, 30, 40, 50, 60, 80, 70]
        );
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
