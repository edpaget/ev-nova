//! Hiring escorts in the spaceport bar: which ship classes are for hire
//! today, at what fee and daily wage, and paying the wages.
//!
//! These are the original `EV Nova` engine's rules, read from its
//! executable (each address below is in `EV Nova.app/Contents/MacOS/EV
//! Nova`), as defaults, not a contract. The fee and the wage sit behind
//! the [`HireTerms`] port, with Nova's [`NovaHire`] as the default, and the
//! control-bit test behind the [`ControlBits`] port, with Nova's
//! [`NovaBits`](crate::NovaBits) reading the pilot, so a plug-in's rules
//! replace them at the edge. A ship whose `Availability` did not parse is
//! never available.
//!
//! # Which ships are for hire (`_SetupPortAvailableShipTypes` @0xbbe3)
//!
//! A stellar with a bar ([`StellarFlags::BAR`](crate::landing::StellarFlags::BAR))
//! lists a ship class for hire when, in this order:
//!
//! 1. its `TechLevel` is 0 or more and allowed there, as the shipyard's
//!    is ([`wares::tech_allows`]); a negative one is never listed
//!    (@0xbc4b-0xbc72);
//! 2. its roll for the day holds: a `HireRandom` of 0 or less is never
//!    for hire, one of [`MAX_HIRE_RANDOM`] or more always (above 100
//!    counts as 100, `_LoadObjectData` @0x7a35d), and any other is a
//!    `HireRandom` % chance (@0xbc9f-0xbcdd; the registered-copy test is
//!    taken as registered);
//! 3. it is not hidden: `Flags3` 0x0200 hides it while its `Require` is
//!    not met, and 0x0100 while its `Availability` does not hold
//!    (@0xbce7-0xbd5b), the same bits as the shipyard's
//!    ([`HIDE_BITS`](crate::shipyard::HIDE_BITS));
//! 4. no lower-numbered class listed of its `DispWeight` has `Flags3`
//!    0x4000 (@0xbdc3-0xbe2c).
//!
//! The rows go in `DispWeight` order, as the shipyard's. The original
//! rolls every class afresh each day, on a new game and on load
//! (`_IncrementGameTime` @0xb7c4), and its landing passes a day. Here the
//! date moves only on a jump, so a class's roll is drawn the first time
//! the list is built after a landing, by ascending ship ID, only for a
//! class that passes its tech level with a `HireRandom` from 1 to 99, and
//! kept until the next landing; it is never saved, as the original rolls
//! again on load. Hiring a class draws its roll again
//! (`_DoShipyardDialog` @0x5f20d), so a second may no longer be on offer.
//!
//! # Who can be hired (`_CalcShipCanBuy` @0x4f848-0x4f90a)
//!
//! A class listed can be hired when its `Availability` holds (through
//! [`ControlBits`]), the fleet has room ([`MAX_ESCORTS`](crate::board::MAX_ESCORTS)
//! escorts that are not fighters, `_CanHireEscorts` @0x5795) and its fee
//! is no more than the cash. The hire branch never checks `Require`,
//! whereas buying does: the rulebook's
//! [`RuleKey::HireRequire`](crate::RuleKey::HireRequire) chooses
//! ([`Session::with_hire_require`](crate::Session::with_hire_require)).
//! One that cannot be hired shows greyed and is refused
//! ([`HireRefusal`]).
//!
//! # The fee (`_DoShipyardDialog` @0x5f14a-0x5f1d1)
//!
//! The fee shown and checked is a tenth ([`HIRE_FEE_SHARE`], @0xdd110) of
//! the hire price, cut down to whole credits; the hire price is
//! `_ApplyPriceAndTechnologyFlux` @0x4e6cd of the ship's `Cost`
//! ([`price_flux`]): none for a `Cost` of 0 or less; a low-tech discount
//! of [`TECH_DISCOUNT`] % a tech level when both tech levels are
//! [`LOW_TECH`] or less, the ship's below the stellar's, and the price
//! above 99; a multiplier, 1.0 here (the `ränk` price modifiers it takes
//! from the player's ranks, `_DoPortDialog` @0x5f970, come with ranks),
//! applied in single precision; then rounded down, above 100000 to a
//! thousand, above 10000 to a hundred and above 100 to ten, and never
//! below 1.
//!
//! What a hire takes from the cash follows the rulebook's
//! [`RuleKey::HireFee`](crate::RuleKey::HireFee): by the engine, the cash
//! becomes trunc(cash − 0.1 × price) in double precision (@0x5f168, the
//! −0.1 at @0xdd648), a credit more than the fee shown whenever the tenth
//! has a fraction (a price of 100 or less not a multiple of 10, which no
//! stock ship for hire has); otherwise exactly the fee shown.
//!
//! # The wage and paying it (`_DoEscortPayment` @0x40111)
//!
//! A hired escort's daily wage is a hundredth ([`WAGE_SHARE`], @0xdd098)
//! of its ship's `Cost`, cut down to whole credits, and none for a `Cost`
//! of 0 or less. The escort keeps the wage it was hired at
//! ([`Escort::wage`](crate::Escort)), which marks it hired and is saved;
//! the wage it is paid each day follows the rulebook's
//! [`RuleKey::EscortWage`](crate::RuleKey::EscortWage): by the engine,
//! worked out again from its ship type's record, as the original does
//! every pay day (@0x401d7); otherwise the wage kept.
//!
//! For each day paid, each hired escort in fleet order is paid its wage
//! while the cash covers it (cash equal to the wage pays it, leaving
//! none); otherwise it defects, leaving the fleet and the system at once
//! (@0x4021e-0x40266: the original deactivates it rather than flying it
//! off), and the flight is told how many defected ([`PayNote`], `STR#`
//! 2002 #302 or #303). Captured escorts and fighters are never paid. The
//! days paid are a jump's days, after the date has moved on
//! (`_HandlePlayer` @0x6c616), and one day at each take-off by the engine
//! (`_DoEscortLand` @0x40853, which in the original pays for its landing
//! day), as the rulebook's [`RuleKey::TakeOffPay`](crate::RuleKey::TakeOffPay)
//! chooses ([`Session::with_take_off_pay`](crate::Session::with_take_off_pay)).
//!
//! # Not modelled
//!
//! The original's hire dialog shows only the fee and the cash; the daily
//! pay shown beside them is the phase's. A freighter that defects would
//! destroy its share of the fleet's cargo (`_DestroyPartialFleetCargo`
//! @0x40240), which waits for pooled holds; mission escorts, never paid,
//! wait for missions; and the extra days the original passes for
//! outfitting and buying a ship are not passed.
//!
//! # Why the session keeps these ports
//!
//! Unlike [`BoardingRule`](crate::BoardingRule) or
//! [`Behaviour`](crate::Behaviour), which each call is given, the session
//! keeps its [`HireTerms`] and [`ControlBits`]
//! ([`Session::with_hire_terms`](crate::Session::with_hire_terms),
//! [`Session::with_control_bits`](crate::Session::with_control_bits)):
//! wages are paid inside every arrival and take-off, so passing them in
//! would add a parameter to every jump and landing path and their many
//! callers. This is a deliberate departure.

use std::fmt::Debug;

use crate::catalog::{LandingSite, ShipId, ShipRecord};
use crate::control::{ControlBits, Gate, PilotFacts, Test};
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::shipyard::{HIDE_BITS, ShipFlags3, ShipSpecs};
use crate::wares::{self, HideHigher};

/// The share of the hire price the fee is (@0xdd110).
pub const HIRE_FEE_SHARE: f64 = 0.1;
/// The share of a ship's `Cost` its daily wage is (@0xdd098).
pub const WAGE_SHARE: f64 = 0.01;
/// The `HireRandom` from which a class is always for hire: above it
/// counts as it (`_LoadObjectData` @0x7a35d-0x7a367).
pub const MAX_HIRE_RANDOM: i16 = 100;
/// The tech level at or below which both the ship's and the stellar's
/// must be for the low-tech discount.
pub const LOW_TECH: i16 = 5;
/// The low-tech discount, in percent a tech level of difference.
pub const TECH_DISCOUNT: i16 = 3;

/// `STR#` 150 #13: the bar's button, and the hire dialog's.
pub const HIRE_ESCORT: &str = "Hire Escort";
/// `STR#` 2002 #224: what the bar says with nothing for hire.
pub const NONE_FOR_HIRE: &str = "There are no ships available for hire.";
/// `STR#` 2002 #228.
pub const HIRING_PRICE: &str = "Hiring Price:";
/// `STR#` 2002 #217.
pub const YOU_HAVE: &str = "You Have:";
/// `STR#` 2002 #166: a hired escort's status in the comm dialog.
pub const HIRED_ESCORT: &str = "Hired Escort";
/// `STR#` 2002 #168: any other escort's status in the comm dialog.
pub const ESCORT: &str = "Escort";
/// `STR#` 2002 #297.
pub const PAY_LABEL: &str = "Pay:";
/// `STR#` 2002 #267.
pub const PER_DAY: &str = "per day";
/// `STR#` 2002 #302: one escort defected.
pub const DEFECTED_ONE: &str = "Due to lack of pay, one of your escorts has defected.";
/// `STR#` 2002 #303: more than one defected.
pub const DEFECTED_SOME: &str = "Due to lack of pay, some of your escorts have defected.";

/// What hiring a ship class costs: the fee shown and checked, what a hire
/// takes from the cash, and the daily wage. Nova's is [`NovaHire`].
pub trait HireTerms: Debug {
    /// The fee for hiring a ship of `ship`'s class at `site`, shown and
    /// checked against the cash.
    fn fee(&self, ship: &ShipRecord, site: &LandingSite) -> i64;

    /// What hiring a ship of `ship`'s class at `site` takes from `cash`,
    /// which is at least [`HireTerms::fee`].
    fn charge(&self, ship: &ShipRecord, site: &LandingSite, cash: i64) -> i64;

    /// The daily wage of an escort of `ship`'s class.
    fn wage(&self, ship: &ShipRecord) -> i64;
}

/// Nova's fee and wage (see the module docs), the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaHire {
    /// What a hire takes: by the engine, the cash cut down after taking a
    /// tenth of the price; otherwise the fee shown
    /// ([`RuleKey::HireFee`]).
    pub hire_fee: RuleSource,
}

impl NovaHire {
    /// The terms `rulebook` chooses: its [`RuleKey::HireFee`] entry.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            hire_fee: rulebook.source_for(RuleKey::HireFee),
        }
    }

    /// The hire price of `ship`'s class at `site`.
    fn price(ship: &ShipRecord, site: &LandingSite) -> i64 {
        price_flux(ship.cost, ship.tech_level, site.tech_level)
    }
}

impl HireTerms for NovaHire {
    fn fee(&self, ship: &ShipRecord, site: &LandingSite) -> i64 {
        (HIRE_FEE_SHARE * Self::price(ship, site) as f64) as i64
    }

    fn charge(&self, ship: &ShipRecord, site: &LandingSite, cash: i64) -> i64 {
        match self.hire_fee {
            RuleSource::Engine => {
                let left = (cash as f64 + Self::price(ship, site) as f64 * -HIRE_FEE_SHARE) as i64;
                cash - left
            }
            RuleSource::Bible => self.fee(ship, site),
        }
    }

    fn wage(&self, ship: &ShipRecord) -> i64 {
        (f64::from(ship.cost) * WAGE_SHARE).max(0.0) as i64
    }
}

/// A ship's hire price from its `Cost` (`price`), its `TechLevel`
/// (`ship_tech`) and the stellar's (`stellar_tech`): the original's
/// `_ApplyPriceAndTechnologyFlux` with a multiplier of 1.0 (see the module
/// docs).
#[must_use]
pub fn price_flux(price: i32, ship_tech: i16, stellar_tech: i16) -> i64 {
    if price <= 0 {
        return 0;
    }
    let mut price = i64::from(price);
    // A price above 99: written `>= 100` so that `>` stands for the
    // rounding thresholds alone.
    if ship_tech <= LOW_TECH && stellar_tech <= LOW_TECH && ship_tech < stellar_tech && price >= 100
    {
        let percent =
            100 - i32::from(TECH_DISCOUNT) * (i32::from(stellar_tech) - i32::from(ship_tech));
        price = (price as f64 * (f64::from(percent) * 0.01)) as i64;
    }
    // The multiplier, 1.0 until ranks exist, is applied in single
    // precision, as the original does: the price goes through an f32.
    let price = price as f32 as i64;
    let step = if price > 100_000 {
        1000
    } else if price > 10_000 {
        100
    } else if price > 100 {
        10
    } else {
        1
    };
    (price / step * step).max(1)
}

/// Why a ship cannot be hired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HireRefusal {
    /// There is no bar: the ship is not landed, or the stellar has none.
    NoBar,
    /// The bar does not list the ship today.
    NotListed,
    /// It is listed but not for hire: its `Availability` does not hold,
    /// or, by the other reading of
    /// [`RuleKey::HireRequire`], its `Require` is not met.
    NotForHire,
    /// The cash does not cover its fee.
    CannotAfford,
    /// The fleet holds as many escorts as it can.
    FleetFull,
}

/// One ship class for hire in the bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HireRow {
    /// The ship class.
    pub id: ShipId,
    /// Its name.
    pub name: String,
    /// Its `ShortName`, raw.
    pub short_name: String,
    /// The fee for hiring it, shown and checked.
    pub fee: i64,
    /// Its daily wage once hired.
    pub wage: i64,
    /// What the Info panel shows of it.
    pub specs: ShipSpecs,
    /// Whether it can be hired now, or why not.
    pub hire: Result<(), HireRefusal>,
}

/// The ships for hire in the bar, as the player sees them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HireList {
    /// The ships listed, in `DispWeight` order.
    pub rows: Vec<HireRow>,
    /// The player's credits.
    pub cash: i64,
    /// Whether the fleet has room for another escort.
    pub room: bool,
}

impl HireList {
    /// `ship`'s row, if it is listed.
    #[must_use]
    pub fn row(&self, ship: ShipId) -> Option<&HireRow> {
        self.rows.iter().find(|row| row.id == ship)
    }

    /// Whether `ship` can be hired now, or why not.
    pub fn check(&self, ship: ShipId) -> Result<(), HireRefusal> {
        self.row(ship).ok_or(HireRefusal::NotListed)?.hire
    }
}

/// What hiring a ship did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hired {
    /// The ship class hired.
    pub ship: ShipId,
    /// What the hire took from the cash.
    pub fee: i64,
    /// The daily wage it was hired at.
    pub wage: i64,
}

/// What paying the escorts did, for the flight's message line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayNote {
    /// So many hired escorts defected for want of pay.
    Defected(u32),
}

/// A class's roll for the day, by its `HireRandom`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Roll {
    /// Never for hire: 0 or less.
    Never,
    /// Always for hire: [`MAX_HIRE_RANDOM`] or more.
    Always,
    /// For hire on a chance of so many percent.
    Chance(u8),
}

impl Roll {
    /// The roll of a class of `hire_random`.
    pub(crate) fn of(hire_random: i16) -> Self {
        if hire_random <= 0 {
            Self::Never
        } else if hire_random >= MAX_HIRE_RANDOM {
            Self::Always
        } else {
            Self::Chance(hire_random as u8)
        }
    }
}

/// Everything the hire list reads about the bar and the player.
pub(crate) struct Bar<'a> {
    /// Every `shïp`.
    pub(crate) ships: &'a [ShipRecord],
    /// The stellar landed on.
    pub(crate) site: &'a LandingSite,
    /// The `Contribute` bits of the player's ship and outfits.
    pub(crate) contributed: u64,
    /// The player's credits.
    pub(crate) cash: i64,
    /// Whether the fleet has room for another escort.
    pub(crate) room: bool,
    /// The fee and wage.
    pub(crate) terms: &'a dyn HireTerms,
    /// The control-bit test.
    pub(crate) control_bits: &'a dyn ControlBits,
    /// What the control-bit test reads about the player.
    pub(crate) pilot: &'a dyn PilotFacts,
    /// Whether `Require` gates hiring ([`RuleKey::HireRequire`]).
    pub(crate) hire_require: RuleSource,
}

impl Bar<'_> {
    /// Whether `test` holds: never when it did not parse.
    fn allows(&self, test: &Test) -> bool {
        Gate {
            control_bits: self.control_bits,
            pilot: self.pilot,
        }
        .allows(test)
    }

    /// The list, each class's roll of so many percent answered by
    /// `rolled`, asked by ascending ID and only for a class that passes
    /// its tech level and is neither never nor always for hire.
    pub(crate) fn list(&self, mut rolled: impl FnMut(ShipId, u8) -> bool) -> HireList {
        let mut sorted: Vec<&ShipRecord> = self.ships.iter().collect();
        sorted.sort_by_key(|ship| ship.id);
        let mut sweep = HideHigher::default();
        let mut rows = Vec::new();
        for ship in sorted {
            if ship.tech_level < 0 || !wares::tech_allows(ship.tech_level, self.site) {
                continue;
            }
            let today = match Roll::of(ship.hire_random) {
                Roll::Never => false,
                Roll::Always => true,
                Roll::Chance(percent) => rolled(ship.id, percent),
            };
            if !today {
                continue;
            }
            let required = wares::requirement_met(ship.require, self.contributed);
            let available = self.allows(&ship.availability);
            if wares::hidden(ship.flags3, HIDE_BITS, required, available)
                || !sweep.on_sale(ship.disp_weight)
            {
                continue;
            }
            sweep.note(
                ship.disp_weight,
                ship.flags3 & ShipFlags3::HIDE_HIGHER != 0,
                true,
            );
            let fee = self.terms.fee(ship, self.site);
            let gated = self.hire_require == RuleSource::Bible && !required;
            let hire = if !available || gated {
                Err(HireRefusal::NotForHire)
            } else if !self.room {
                Err(HireRefusal::FleetFull)
            } else if self.cash < fee {
                Err(HireRefusal::CannotAfford)
            } else {
                Ok(())
            };
            rows.push((
                ship.disp_weight,
                HireRow {
                    id: ship.id,
                    name: ship.name.clone(),
                    short_name: ship.short_name.clone(),
                    fee,
                    wage: self.terms.wage(ship),
                    specs: ShipSpecs {
                        fields: ship.fields,
                        max_gun: ship.max_gun,
                        max_tur: ship.max_tur,
                        length: ship.length,
                        crew: ship.crew,
                    },
                    hire,
                },
            ));
        }
        wares::in_display_order(&mut rows, |(weight, row)| (*weight, row.id.0));
        HireList {
            rows: rows.into_iter().map(|(_, row)| row).collect(),
            cash: self.cash,
            room: self.room,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{FAST, planet, ship};

    /// A site of tech level `tech`.
    fn site(tech: i16) -> LandingSite {
        LandingSite {
            tech_level: tech,
            ..planet(128, 0.0, 0.0)
        }
    }

    /// A ship of `cost` and tech level `tech`.
    fn costing(cost: i32, tech: i16) -> ShipRecord {
        ShipRecord {
            cost,
            tech_level: tech,
            ..ship(128, FAST)
        }
    }

    #[test]
    fn the_low_tech_discount_takes_3_percent_a_tech_level_below_a_low_tech_stellar() {
        assert_eq!(price_flux(10_000, 3, 4), 9700);
        assert_eq!(price_flux(10_000, 3, 7), 10_000, "a stellar above 5");
        assert_eq!(price_flux(10_000, 3, 3), 10_000, "no lower");
        assert_eq!(price_flux(10_000, 4, 3), 10_000, "the ship higher");
        assert_eq!(price_flux(10_000, 6, 5), 10_000, "the ship above 5");
        assert_eq!(price_flux(17_500, 4, 5), 16_900);
        assert_eq!(price_flux(99, 1, 5), 99, "a price of 99 or less");
        assert_eq!(price_flux(100, 1, 5), 88, "100 is above 99");
        assert_eq!(price_flux(200, 1, 5), 170);
        assert_eq!(price_flux(10_000, 5, 5), 10_000);
        assert_eq!(price_flux(10_000, 0, 5), 8500, "tech 0");
    }

    #[test]
    fn the_price_is_rounded_down_by_its_size() {
        assert_eq!(price_flux(100, 1, 7), 100);
        assert_eq!(price_flux(101, 1, 7), 100, "above 100 to ten");
        assert_eq!(price_flux(105, 1, 7), 100);
        assert_eq!(price_flux(109, 1, 7), 100);
        assert_eq!(price_flux(10_000, 1, 7), 10_000);
        assert_eq!(price_flux(10_001, 1, 7), 10_000, "above 10000 to a hundred");
        assert_eq!(price_flux(10_099, 1, 7), 10_000);
        assert_eq!(price_flux(10_109, 1, 7), 10_100);
        assert_eq!(price_flux(100_000, 1, 7), 100_000);
        assert_eq!(
            price_flux(100_099, 1, 7),
            100_000,
            "above 100000 to a thousand"
        );
        assert_eq!(price_flux(123_456, 1, 7), 123_000);
        assert_eq!(price_flux(14, 1, 7), 14);
    }

    #[test]
    fn no_cost_is_no_price_and_any_other_is_at_least_1() {
        assert_eq!(price_flux(0, 1, 7), 0);
        assert_eq!(price_flux(-5, 1, 7), 0);
        assert_eq!(price_flux(1, 1, 7), 1);
        assert_eq!(price_flux(1, 1, 5), 1);
    }

    #[test]
    fn the_price_goes_through_single_precision() {
        assert_eq!(price_flux(16_777_217, 1, 7), 16_777_000);
        assert_eq!(price_flux(i32::MAX, 1, 7), 2_147_483_000);
    }

    #[test]
    fn the_fee_is_a_tenth_of_the_hire_price() {
        let nova = NovaHire::default();
        assert_eq!(nova.fee(&costing(10_000, 3), &site(4)), 970);
        assert_eq!(nova.fee(&costing(10_000, 3), &site(7)), 1000);
        assert_eq!(nova.fee(&costing(17_500, 4), &site(5)), 1690);
        assert_eq!(nova.fee(&costing(14, 1), &site(7)), 1, "cut down");
        assert_eq!(nova.fee(&costing(0, 1), &site(7)), 0);
        assert_eq!(nova.fee(&costing(-5, 1), &site(7)), 0);
    }

    #[test]
    fn the_wage_is_a_hundredth_of_the_cost_and_never_below_none() {
        let nova = NovaHire::default();
        assert_eq!(nova.wage(&costing(10_000, 3)), 100);
        assert_eq!(nova.wage(&costing(17_500, 4)), 175);
        assert_eq!(nova.wage(&costing(199, 4)), 1);
        assert_eq!(nova.wage(&costing(99, 4)), 0);
        assert_eq!(nova.wage(&costing(-5, 4)), 0);
        assert_eq!(nova.wage(&costing(-500, 4)), 0);
    }

    #[test]
    fn by_the_engine_a_hire_takes_a_credit_more_when_the_tenth_has_a_fraction() {
        let nova = NovaHire {
            hire_fee: RuleSource::Engine,
        };
        let at = site(7);
        assert_eq!(nova.fee(&costing(14, 1), &at), 1);
        assert_eq!(nova.charge(&costing(14, 1), &at, 10), 2, "8 left");
        assert_eq!(
            nova.charge(&costing(14, 1), &at, 1),
            1,
            "none left, never below"
        );
        assert_eq!(nova.charge(&costing(20, 1), &at, 1000), 2);
        assert_eq!(nova.charge(&costing(10_000, 1), &at, 25_000), 1000);
        assert_eq!(nova.charge(&costing(10_000, 3), &site(4), 25_000), 970);
    }

    #[test]
    fn by_the_other_reading_a_hire_takes_the_fee_shown() {
        let nova = NovaHire {
            hire_fee: RuleSource::Bible,
        };
        let at = site(7);
        assert_eq!(nova.charge(&costing(14, 1), &at, 10), 1);
        assert_eq!(nova.charge(&costing(10_000, 1), &at, 25_000), 1000);
        assert_eq!(nova.charge(&costing(10_000, 3), &site(4), 25_000), 970);
    }

    #[test]
    fn nova_hire_follows_its_rulebook_entry() {
        assert_eq!(NovaHire::default().hire_fee, RuleSource::Engine);
        assert_eq!(
            NovaHire::from_rulebook(&Rulebook::default()),
            NovaHire::default()
        );
        let bible = Rulebook::default().with_override(RuleKey::HireFee, RuleSource::Bible);
        assert_eq!(NovaHire::from_rulebook(&bible).hire_fee, RuleSource::Bible);
        let others =
            Rulebook::new(RuleSource::Bible).with_override(RuleKey::HireFee, RuleSource::Engine);
        assert_eq!(
            NovaHire::from_rulebook(&others).hire_fee,
            RuleSource::Engine
        );
    }

    #[test]
    fn a_rolls_chance_is_its_hire_random_up_to_always() {
        assert_eq!(Roll::of(-1), Roll::Never);
        assert_eq!(Roll::of(0), Roll::Never);
        assert_eq!(Roll::of(1), Roll::Chance(1));
        assert_eq!(Roll::of(50), Roll::Chance(50));
        assert_eq!(Roll::of(99), Roll::Chance(99));
        assert_eq!(Roll::of(100), Roll::Always);
        assert_eq!(Roll::of(250), Roll::Always);
    }

    #[test]
    fn a_list_reads_its_rows_by_ship() {
        let row = |id| HireRow {
            id: ShipId(id),
            name: String::new(),
            short_name: String::new(),
            fee: 0,
            wage: 0,
            specs: ShipSpecs {
                fields: FAST,
                max_gun: 0,
                max_tur: 0,
                length: 0,
                crew: 0,
            },
            hire: if id == 129 {
                Err(HireRefusal::CannotAfford)
            } else {
                Ok(())
            },
        };
        let list = HireList {
            rows: vec![row(128), row(129)],
            cash: 0,
            room: true,
        };
        assert_eq!(list.row(ShipId(129)).map(|row| row.id), Some(ShipId(129)));
        assert_eq!(list.row(ShipId(130)), None);
        assert_eq!(list.check(ShipId(128)), Ok(()));
        assert_eq!(list.check(ShipId(129)), Err(HireRefusal::CannotAfford));
        assert_eq!(list.check(ShipId(130)), Err(HireRefusal::NotListed));
    }
}
