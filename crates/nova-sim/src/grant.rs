//! Boarding grants: the outfits boarding a disabled person may give the
//! player, of its `përs` `GrantClass`.
//!
//! The rules are the original's (`_DoPlunderDialog` @0x9302b in the `EV
//! Nova` executable); [`NovaBoarding`](crate::NovaBoarding) applies them
//! ([`BoardingRule::grant`](crate::BoardingRule::grant)) and the session
//! grants on boarding ([`Session::board`](crate::Session::board),
//! [`Session::take_grant`](crate::Session::take_grant)).
//!
//! **When.** Boarding a person grants when the plunder dialog opens, after
//! its plunder and capture odds are rolled; boarders repelled are granted
//! nothing. Only a person whose `GrantClass`, `GrantProb` and
//! `GrantCount` are all above 0 grants anything ([`PersonGrant::of`]);
//! `GrantProb` reads as at most [`MOST_PROB`].
//!
//! **In order**, drawing nothing more once one fails:
//!
//! 1. *The odds*: `Rand(100) + 1` at most `GrantProb` grants
//!    ([`GRANT_ROLL`]).
//! 2. *The candidates* ([`candidates`]): the outfits whose `ItemClass` is
//!    the `GrantClass` and that the player owns fewer than `Max` of; with
//!    none, nothing is granted.
//! 3. *The pick*: one of k candidates in ascending ID by `Rand(k)`, drawn
//!    only when k is 2 or more. The original draws `Rand(512)` until it
//!    lands on a candidate, so each is as likely.
//! 4. *The count*, as [`RuleKey::GrantCount`](crate::RuleKey::GrantCount)
//!    says: by the engine ([`engine_count`]) trunc((50 + `Rand(51)`) x
//!    `GrantCount` / 100.0), at least 1, so `GrantCount` / 2 to
//!    `GrantCount`, as the Bible says too; by the phase ([`phase_count`])
//!    1 + `Rand(GrantCount)`.
//! 5. *`Max`*, as [`RuleKey::GrantMax`](crate::RuleKey::GrantMax) says:
//!    by the engine the count is not held to the outfit's `Max`, so a
//!    grant may pass it; otherwise it is held to `Max` less the owned.
//! 6. *Room* ([`fit_count`]): while the count times the outfit's raw
//!    `Mass` (not scaled by its `Flags` 0x0400) is above the free mass,
//!    taken as none when below none, the count drops by one. At none,
//!    nothing is granted and nothing said.
//!
//! The outfits granted are added as bought ones are, and the ship is
//! refitted with them; a grant ignores `Flags` 0x0010 ("remove after
//! purchase") and makes no save due. The flight says what was granted.
//!
//! **Deferred**: the original's `_GrantOutfitItem` (@0x44d4f), which the
//! outfitter's purchases also go through, explores the map for an outfit
//! of `ModType` 16, clears the legal record for 21 and paints the ship for
//! 43, instead of adding it. Here, as in the outfitter, they are added as
//! plain items (rdm task `outfit-mod-type-effects`); stock Dr Ralph's map
//! is one. The sound the original plays with the message is deferred too.
//!
//! The values are defaults, not a contract.

use crate::catalog::{OutfitId, PersonRecord};
use crate::chance::Chance;
use crate::rulebook::RuleSource;

/// The odds' draw: a grant when `Rand(100) + 1` is at most `GrantProb`
/// (@0x930b4-0x930d4).
pub const GRANT_ROLL: u32 = 100;
/// The most `GrantProb` reads as (`_LoadObjectData` @0x7c3f5).
pub const MOST_PROB: i16 = 100;
/// The engine's count draws `Rand(51)` (0x33 @0x931a2) ...
pub const COUNT_SPREAD: u32 = 51;
/// ... above 50 percent of `GrantCount` ...
pub const COUNT_FLOOR_PERCENT: u32 = 50;
/// ... divided by 100.0 (@0xdd0e0).
pub const COUNT_PERCENT: f64 = 100.0;

/// What a person's record grants when boarded: the `ItemClass` of its
/// outfits, the percent chance, and the most it gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonGrant {
    /// Its `GrantClass`: the `ItemClass` granted, above 0.
    pub class: i16,
    /// Its `GrantProb`, 1 to 100.
    pub prob: u8,
    /// Its `GrantCount`, at least 1.
    pub count: u16,
}

impl PersonGrant {
    /// What `record` grants: none unless its `GrantClass`, `GrantProb`
    /// and `GrantCount` are all above 0 (`_DoPlunderDialog`
    /// @0x9308a-0x930ae), its `GrantProb` read as at most [`MOST_PROB`]
    /// (`_LoadObjectData` @0x7c38e-0x7c401).
    #[must_use]
    pub fn of(record: &PersonRecord) -> Option<Self> {
        if record.grant_class <= 0 || record.grant_prob <= 0 || record.grant_count <= 0 {
            return None;
        }
        Some(Self {
            class: record.grant_class,
            prob: record.grant_prob.min(MOST_PROB) as u8,
            count: record.grant_count as u16,
        })
    }
}

/// One outfit as a grant sees it: its `ItemClass`, its raw `Mass`, how
/// many the player owns and its `Max`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrantStock {
    /// The outfit.
    pub outfit: OutfitId,
    /// Its `ItemClass`.
    pub item_class: i16,
    /// Its `Mass`, raw: not scaled by its `Flags` 0x0400.
    pub mass: i16,
    /// How many the player owns.
    pub owned: u16,
    /// Its `Max`.
    pub max: i16,
}

/// What a grant gave: `count` of `outfit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Granted {
    /// The outfit granted.
    pub outfit: OutfitId,
    /// How many, at least 1.
    pub count: u16,
}

/// The outfits of `stock` a grant of `class` may pick: those of its
/// `ItemClass` the player owns fewer than `Max` of (`_HasMaxOfItem`
/// @0x93119), in `stock`'s order.
#[must_use]
pub fn candidates(class: i16, stock: &[GrantStock]) -> Vec<GrantStock> {
    stock
        .iter()
        .filter(|outfit| {
            outfit.item_class == class && i32::from(outfit.owned) < i32::from(outfit.max)
        })
        .copied()
        .collect()
}

/// The engine's count of a grant of `count`: trunc((50 + `Rand(51)`) x
/// `count` / 100.0), at least 1 (@0x9319b-0x931d6).
pub fn engine_count(count: u16, chance: &mut dyn Chance) -> u16 {
    let percent = COUNT_FLOOR_PERCENT + chance.below(COUNT_SPREAD);
    let count = f64::from(percent) * f64::from(count) / COUNT_PERCENT;
    (count as u16).max(1)
}

/// The phase's count of a grant of `count`: 1 + `Rand(count)`, 1 to
/// `count` evenly.
pub fn phase_count(count: u16, chance: &mut dyn Chance) -> u16 {
    1 + chance.below(u32::from(count)) as u16
}

/// `count` of an outfit of `Mass` `mass`, one less at a time while they
/// weigh more than `free_mass`, taken as none when below none
/// (@0x931f2-0x9320e, `_ShipFreeMass` @0xb506).
#[must_use]
pub fn fit_count(count: u16, mass: i16, free_mass: i64) -> u16 {
    let free = free_mass.max(0);
    let mut count = count;
    // At none the outfits weigh nothing, which the free mass (none or
    // more) always covers, so the loop ends there.
    while i64::from(mass) * i64::from(count) > free {
        count -= 1;
    }
    count
}

/// What `grant` gives from `stock`, with `free_mass` free, its count
/// and `Max` as `rules` (`grant_count`, `grant_max`) say, drawn on
/// `chance` (see the module docs).
pub fn roll(
    grant: &PersonGrant,
    stock: &[GrantStock],
    free_mass: i64,
    rules: (RuleSource, RuleSource),
    chance: &mut dyn Chance,
) -> Option<Granted> {
    if chance.below(GRANT_ROLL) + 1 > u32::from(grant.prob) {
        return None;
    }
    let candidates = candidates(grant.class, stock);
    let picked = match candidates.as_slice() {
        [] => return None,
        [only] => *only,
        many => many[chance.below(many.len() as u32) as usize],
    };
    let (count_rule, max_rule) = rules;
    let mut count = match count_rule {
        RuleSource::Engine => engine_count(grant.count, chance),
        RuleSource::Bible => phase_count(grant.count, chance),
    };
    if max_rule == RuleSource::Bible {
        let room = i32::from(picked.max) - i32::from(picked.owned);
        count = count.min(room as u16);
    }
    let count = fit_count(count, picked.mass, free_mass);
    (count > 0).then_some(Granted {
        outfit: picked.outfit,
        count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, PersonRecord};
    use crate::rulebook::RuleSource;
    use crate::testkit::{Draws, person};

    /// A person granting `class` with odds `prob` and count `count`.
    fn granting(class: i16, prob: i16, count: i16) -> PersonRecord {
        PersonRecord {
            grant_class: class,
            grant_prob: prob,
            grant_count: count,
            ..person(128, 128)
        }
    }

    #[test]
    fn a_person_of_a_class_odds_and_count_all_above_none_grants() {
        assert_eq!(
            PersonGrant::of(&granting(25, 50, 1)),
            Some(PersonGrant {
                class: 25,
                prob: 50,
                count: 1,
            })
        );
    }

    #[test]
    fn a_class_odds_or_count_of_none_or_less_grants_nothing() {
        for (class, prob, count) in [(0, 50, 1), (-1, 50, 1), (25, 0, 1), (25, 50, 0)] {
            assert_eq!(
                PersonGrant::of(&granting(class, prob, count)),
                None,
                "{class} {prob} {count}"
            );
        }
        assert_eq!(PersonGrant::of(&granting(25, -5, 1)), None);
        assert_eq!(PersonGrant::of(&granting(25, 50, -2)), None);
    }

    #[test]
    fn odds_above_a_hundred_read_as_a_hundred() {
        assert_eq!(
            PersonGrant::of(&granting(25, 150, 3)).map(|grant| grant.prob),
            Some(100)
        );
        assert_eq!(
            PersonGrant::of(&granting(25, 100, 3)).map(|grant| grant.prob),
            Some(100)
        );
        assert_eq!(MOST_PROB, 100);
    }

    /// Outfits 128-131 of classes 7, 7, 8 and 7, a ton each, none owned
    /// of a `Max` of 10, but the fourth owned at its `Max`.
    fn stock() -> Vec<GrantStock> {
        let outfit = |id, item_class, owned| GrantStock {
            outfit: OutfitId(id),
            item_class,
            mass: 1,
            owned,
            max: 10,
        };
        vec![
            outfit(128, 7, 0),
            outfit(129, 7, 0),
            outfit(130, 8, 0),
            outfit(131, 7, 10),
        ]
    }

    #[test]
    fn the_candidates_are_the_outfits_of_the_class_owned_below_their_max() {
        let stock = stock();
        assert_eq!(
            candidates(7, &stock)
                .iter()
                .map(|outfit| outfit.outfit)
                .collect::<Vec<_>>(),
            [OutfitId(128), OutfitId(129)]
        );
        assert_eq!(candidates(9, &stock).len(), 0);
        let none_allowed = GrantStock { max: 0, ..stock[0] };
        assert_eq!(candidates(7, &[none_allowed]).len(), 0, "Max 0");
        let below = GrantStock {
            owned: 9,
            ..stock[0]
        };
        assert_eq!(candidates(7, &[below]).len(), 1, "one below its Max");
    }

    /// The grant `grant` gives from `stock` with `free` mass free, by the
    /// engine's readings, drawn from `draws`, and the bounds asked.
    fn rolled(
        grant: PersonGrant,
        stock: &[GrantStock],
        free: i64,
        draws: &[u32],
    ) -> (Option<Granted>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let granted = roll(
            &grant,
            stock,
            free,
            (RuleSource::Engine, RuleSource::Engine),
            &mut chance,
        );
        (granted, chance.asked)
    }

    const ONE_OF_SEVEN: PersonGrant = PersonGrant {
        class: 7,
        prob: 100,
        count: 1,
    };

    #[test]
    fn with_one_candidate_nothing_is_drawn_for_the_pick() {
        let stock = stock();
        let one = [stock[0], stock[2]];
        assert_eq!(
            rolled(ONE_OF_SEVEN, &one, 100, &[0, 0]),
            (
                Some(Granted {
                    outfit: OutfitId(128),
                    count: 1,
                }),
                vec![100, 51]
            )
        );
    }

    #[test]
    fn with_two_candidates_a_draw_of_two_picks_one() {
        let stock = stock();
        assert_eq!(
            rolled(ONE_OF_SEVEN, &stock, 100, &[0, 1, 0]),
            (
                Some(Granted {
                    outfit: OutfitId(129),
                    count: 1,
                }),
                vec![100, 2, 51]
            )
        );
        assert_eq!(
            rolled(ONE_OF_SEVEN, &stock, 100, &[0, 0, 0]).0,
            Some(Granted {
                outfit: OutfitId(128),
                count: 1,
            })
        );
    }

    #[test]
    fn with_no_candidate_nothing_is_granted_and_only_the_odds_are_drawn() {
        let stock = stock();
        let none = PersonGrant {
            class: 9,
            ..ONE_OF_SEVEN
        };
        assert_eq!(rolled(none, &stock, 100, &[0]), (None, vec![100]));
    }
}
