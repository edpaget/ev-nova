//! The outfitter: which outfits (`oütf`) a stellar lists and sells, at
//! what price and mass, and buying and selling them one at a time.
//!
//! # What a stellar sells
//!
//! An outfit is *for sale* at a stellar whose `TechLevel` is at or above
//! the outfit's, or one of whose eight `SpecialTech` slots equals it
//! exactly (a slot of 0 or below is unused). An outfit flagged
//! [`OutfitFlags::HIDE_HIGHER`] that can be bought here takes every
//! higher-numbered outfit of the same `DispWeight` off sale.
//!
//! An outfit for sale can be *bought* when its `Require` bits are met and
//! its `Availability` holds. `Require` is met when each of its bits is in
//! the `Contribute` of the ship or of an outfit the player owns. Its
//! `RequireGovt` says where `Require` applies: -1, and any value outside
//! the Bible's four ranges (stock data has 127 and 0), everywhere; 128-383
//! only at that government's stellars; 1128-1383 there and at independent
//! ones; 2128-2383 everywhere but that government's; 3128-3383 everywhere
//! but that government's and independent ones. Alliances are not modelled
//! until governments' relations are, so "this government or its allies"
//! is this government alone. `Availability` is tested through the
//! [`ControlBits`](crate::ControlBits) port for the player's pilot (a
//! [`Gate`]), as `_CanBuyOutfitItem` does (@0x4eb51); one that did not
//! parse never holds. An outfit whose `Availability` does not hold is
//! listed but cannot be bought, and can still be sold.
//!
//! `BuyRandom`, the percent chance a day that an outfit is for sale, is
//! not applied yet: every outfit is for sale every day.
//!
//! # What it lists
//!
//! The outfitter lists every outfit for sale, except one flagged
//! [`OutfitFlags::HIDE_UNLESS_REQUIRED`] whose `Require` is not met, or
//! [`OutfitFlags::HIDE_UNLESS_AVAILABLE`] whose `Availability` does not
//! hold, unless the player owns one. It also lists, at every outfitter,
//! each outfit the player owns that is flagged
//! [`OutfitFlags::SELL_ANYWHERE`], so it can be sold there; such a row can
//! only be sold where the outfit is not for sale. Rows go by `DispWeight`,
//! highest first, then by ID.
//!
//! # Price, mass and free mass
//!
//! An outfit's price is its `Cost`, times the ship's `Mass` with
//! [`OutfitFlags::PRICE_BY_MASS`]; its mass is its `Mass`, or the ship's
//! `Mass` times it over 100, truncated, with [`OutfitFlags::MASS_BY_MASS`]
//! when it is positive. The ship's free mass is its `FreeMass`, plus its
//! default items' mass (`FreeMass` is space on top of them), less the
//! mass of every outfit it carries: a new ship has exactly its `FreeMass`
//! free, and an outfit of negative mass adds space.
//!
//! # Buying and selling
//!
//! Each order buys or sells one, and a refused one changes nothing. A buy
//! is refused when the outfit cannot be bought here, the player owns its
//! `Max` already, it is a fighter whose bays have no room left (their
//! [`capacity`](crate::bay::capacity), the fighters out counted against
//! it, `_CanBuyFighter` @0x5a82; refused as `Max` owned), the ship's
//! `Holds` is negative and the outfit adds mass
//! space, there is not the free mass for it, or the player cannot pay. A
//! buy pays the price, then grants one through the session's grant path,
//! as the original's `_DoOutfitDialog` calls `_GrantOutfitItem`
//! (@0x5c21d): a map explores, a clean-record outfit cleans the legal
//! record and a paint paints the ship instead of being added (see
//! [`outfit_effects`](crate::outfit_effects)), and anything else is added,
//! but taken away again with [`OutfitFlags::REMOVE_AFTER_PURCHASE`] (the
//! original takes it away as the outfitter closes, @0x5daeb-0x5dafa). A
//! sale is refused
//! when the player owns none, the outfit is flagged
//! [`OutfitFlags::CANNOT_SELL`], it is neither for sale here nor flagged
//! [`OutfitFlags::SELL_ANYWHERE`], or the ship would be left with negative
//! free mass. A sale pays [`RESALE_PERCENT`] of the price and removes one.
//! Selling cargo space below the cargo held is allowed: the exchange then
//! shows no space free until enough is sold.
//!
//! Not modelled yet: the gun and turret limits (`MaxGun`, `MaxTur`, flags
//! 0x0001 and 0x0002), `MaxAmmo` for ammunition other than fighters,
//! selling a launcher before its ammunition,
//! and `ModType` 27's raised maximums. Which outfits a ship bought in the
//! [`shipyard`](crate::shipyard) keeps is the shipyard's (flag 0x0004);
//! flag 0x0020 only concerns a mission's change of ship.

use std::collections::BTreeMap;

use crate::catalog::{GovtId, LandingSite, OutfitId, OutfitRecord};
use crate::control::Gate;
use crate::fuel::OutfitMod;
use crate::handling::ShipFields;
use crate::landing::StellarFlags;
use crate::market::Direction;
use crate::pilot::Pilot;
use crate::wares::{self, HideBits, HideHigher};

/// The `oütf` `Flags` bits the outfitter reads (the Bible).
#[derive(Clone, Copy, Debug)]
pub struct OutfitFlags;

impl OutfitFlags {
    /// Stays with the player when they trade ships (persistent): see
    /// [`shipyard`](crate::shipyard).
    pub const PERSISTENT: u16 = 0x0004;
    /// Can't be sold.
    pub const CANNOT_SELL: u16 = 0x0008;
    /// Removed after purchase: buying it only pays.
    pub const REMOVE_AFTER_PURCHASE: u16 = 0x0010;
    /// Not shown unless the player meets its `Require`, or owns one.
    pub const HIDE_UNLESS_REQUIRED: u16 = 0x0100;
    /// Its price is its `Cost` times the ship's `Mass`.
    pub const PRICE_BY_MASS: u16 = 0x0200;
    /// Its mass is the ship's `Mass` times its `Mass` over 100.
    pub const MASS_BY_MASS: u16 = 0x0400;
    /// Can be sold anywhere.
    pub const SELL_ANYWHERE: u16 = 0x0800;
    /// For sale, it takes higher-numbered outfits of equal `DispWeight`
    /// off sale.
    pub const HIDE_HIGHER: u16 = 0x1000;
    /// Not shown unless its `Availability` holds, or the player owns one.
    pub const HIDE_UNLESS_AVAILABLE: u16 = 0x4000;
}

/// What an outfit sells back for, as a percentage of its price. The
/// Bible does not say; the community guide (evnova.miraheze.org,
/// "Nova:Making Money") sells the Scarab's Matter/Antimatter Reactor,
/// whose stock `Cost` is 5,000,000, "for 2.5 million credits".
pub const RESALE_PERCENT: i64 = 50;

/// One outfit the player asks to buy or sell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutfitOrder {
    /// The outfit.
    pub outfit: OutfitId,
    /// Buying or selling.
    pub direction: Direction,
}

/// Why an outfit cannot be bought or sold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutfitRefusal {
    /// There is no outfitter: the ship is not landed, or the stellar has
    /// none.
    NoOutfitter,
    /// The outfitter does not list the outfit.
    NotListed,
    /// The outfit is listed but cannot be bought here: it is not for sale,
    /// its `Require` is not met, or its `Availability` does not hold.
    NotForSale,
    /// The player owns its `Max` already.
    MaxOwned,
    /// Its `Max` is none.
    NoneAllowed,
    /// The ship has not the free mass for another.
    NoSpace,
    /// The ship has not the free mass for one.
    NoSpaceForAny,
    /// It adds mass space, and the ship's `Holds` forbids that.
    NoExpansion,
    /// The player cannot pay for it.
    CannotAfford,
    /// The player owns none.
    NoneOwned,
    /// It can't be sold.
    CannotSell,
    /// It is not for sale here, and cannot be sold anywhere.
    NotBoughtHere,
    /// Selling it would leave the ship with negative free mass.
    NegativeFreeMass,
}

/// One outfit in a stellar's outfitter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitRow {
    /// The outfit.
    pub id: OutfitId,
    /// Its name.
    pub name: String,
    /// Its `ShortName`, raw.
    pub short_name: String,
    /// Its price here, for one.
    pub price: i64,
    /// Its mass, for one, in tons.
    pub mass: i64,
    /// How many the player owns.
    pub owned: u16,
    /// Its `Max`.
    pub max: i16,
    /// Whether one can be bought now, or why not.
    pub buy: Result<(), OutfitRefusal>,
    /// Whether one can be sold now, or why not.
    pub sell: Result<(), OutfitRefusal>,
}

/// A stellar's outfitter, as the player sees it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outfitter {
    /// The outfits listed, highest `DispWeight` first, then by ID.
    pub rows: Vec<OutfitRow>,
    /// The player's credits.
    pub cash: i64,
    /// The ship's free mass, in tons; negative when it carries more than
    /// it has space for.
    pub free_mass: i64,
}

impl Outfitter {
    /// `outfit`'s row, if it is listed.
    #[must_use]
    pub fn row(&self, outfit: OutfitId) -> Option<&OutfitRow> {
        self.rows.iter().find(|row| row.id == outfit)
    }

    /// Whether `order` can go through now, or why not.
    pub fn check(&self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        let row = self.row(order.outfit).ok_or(OutfitRefusal::NotListed)?;
        match order.direction {
            Direction::Buy => row.buy,
            Direction::Sell => row.sell,
        }
    }
}

/// Whether `outfit` is for sale, by tech level alone, at `site`.
#[must_use]
pub fn tech_allows(outfit: &OutfitRecord, site: &LandingSite) -> bool {
    wares::tech_allows(outfit.tech_level, site)
}

/// Whether an outfit's `Require` applies at a stellar of `govt` (`None`
/// for independent), given its `RequireGovt`.
#[must_use]
pub fn requirements_apply(require_govt: i16, govt: Option<GovtId>) -> bool {
    let of = |base: i16| {
        let target = GovtId(require_govt - base);
        (govt == Some(target), govt.is_none())
    };
    match require_govt {
        128..=383 => of(0).0,
        1128..=1383 => {
            let (theirs, independent) = of(1000);
            theirs || independent
        }
        2128..=2383 => !of(2000).0,
        3128..=3383 => {
            let (theirs, independent) = of(3000);
            !(theirs || independent)
        }
        _ => true,
    }
}

/// `outfit`'s price, for one, on a ship of `ship_mass`; never below none.
#[must_use]
pub fn unit_price(outfit: &OutfitRecord, ship_mass: i16) -> i64 {
    let cost = i64::from(outfit.cost);
    let price = if outfit.flags & OutfitFlags::PRICE_BY_MASS == 0 {
        cost
    } else {
        cost.saturating_mul(i64::from(ship_mass))
    };
    price.max(0)
}

/// `outfit`'s mass, for one, in tons, on a ship of `ship_mass`.
#[must_use]
pub fn unit_mass(outfit: &OutfitRecord, ship_mass: i16) -> i64 {
    let mass = i64::from(outfit.mass);
    if outfit.flags & OutfitFlags::MASS_BY_MASS != 0 && mass.is_positive() {
        i64::from(ship_mass) * mass / 100
    } else {
        mass
    }
}

/// Whether an outfit with `flags` is hidden from a player who owns none,
/// given whether its `Require` is met (`required`) and its `Availability`
/// holds (`available`).
#[must_use]
pub fn hides(flags: u16, required: bool, available: bool) -> bool {
    wares::hidden(flags, HIDE_BITS, required, available)
}

/// The `oütf` flags that hide an outfit.
const HIDE_BITS: HideBits = HideBits {
    unless_required: OutfitFlags::HIDE_UNLESS_REQUIRED,
    unless_available: OutfitFlags::HIDE_UNLESS_AVAILABLE,
};

/// What one outfit priced at `price` sells back for.
#[must_use]
pub fn resale(price: i64) -> i64 {
    price.saturating_mul(RESALE_PERCENT) / 100
}

/// Every mod of the outfits `owned`, each record's four `ModType`s and
/// `ModVal`s with how many are owned. An outfit with no record has none.
#[must_use]
pub(crate) fn outfit_mods(
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> Vec<OutfitMod> {
    records
        .iter()
        .filter_map(|record| Some((record, *owned.get(&record.id)?)))
        .flat_map(|(record, count)| {
            record.mods.map(|(mod_type, mod_val)| OutfitMod {
                mod_type,
                mod_val,
                count,
            })
        })
        .collect()
}

/// The total mass of `outfits` on a ship of `ship_mass`.
fn mass_of(outfits: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord], ship_mass: i16) -> i64 {
    records
        .iter()
        .filter_map(|record| Some((record, *outfits.get(&record.id)?)))
        .map(|(record, count)| unit_mass(record, ship_mass) * i64::from(count))
        .sum()
}

/// The free mass of a ship with `fields` and these `defaults`, carrying
/// `owned`.
#[must_use]
pub(crate) fn free_mass(
    fields: ShipFields,
    defaults: &BTreeMap<OutfitId, u16>,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> i64 {
    i64::from(fields.free_mass) + mass_of(defaults, records, fields.mass)
        - mass_of(owned, records, fields.mass)
}

/// Everything the outfitter rules read about the ship and where it is.
pub(crate) struct Shop<'a> {
    /// Every `oütf`.
    pub(crate) records: &'a [OutfitRecord],
    /// The ship's fields.
    pub(crate) fields: ShipFields,
    /// The ship's default items.
    pub(crate) defaults: &'a BTreeMap<OutfitId, u16>,
    /// The stellar landed on.
    pub(crate) site: &'a LandingSite,
    /// For every ammunition outfit of a fighter bay, the fighters the
    /// ship's bays can still take (see [`bay`](crate::bay)).
    pub(crate) fighter_room: &'a BTreeMap<OutfitId, u32>,
    /// The control-bit test of an outfit's `Availability`.
    pub(crate) gate: Gate<'a>,
}

impl Shop<'_> {
    /// The outfitter, for `pilot`; `None` when the stellar has none.
    pub(crate) fn outfitter(&self, pilot: &Pilot) -> Option<Outfitter> {
        if self.site.flags & StellarFlags::OUTFITTER == 0 {
            return None;
        }
        let contributed = wares::contributed(self.fields.contribute, &pilot.outfits, self.records);
        let free = free_mass(self.fields, self.defaults, &pilot.outfits, self.records);
        let mut sorted: Vec<&OutfitRecord> = self.records.iter().collect();
        sorted.sort_by_key(|record| record.id);
        let mut sweep = HideHigher::default();
        let mut rows = Vec::new();
        for record in sorted {
            let owned = pilot.owned(record.id);
            let required = !requirements_apply(record.require_govt, self.site.govt)
                || wares::requirement_met(record.require, contributed);
            let available = self.gate.allows(&record.availability);
            let for_sale = tech_allows(record, self.site) && sweep.on_sale(record.disp_weight);
            let buyable = for_sale && required && available;
            sweep.note(
                record.disp_weight,
                record.flags & OutfitFlags::HIDE_HIGHER != 0,
                buyable,
            );
            let hidden = owned == 0 && hides(record.flags, required, available);
            let sells_anywhere = record.flags & OutfitFlags::SELL_ANYWHERE != 0;
            let listed = (for_sale && !hidden) || (owned > 0 && sells_anywhere);
            if !listed {
                continue;
            }
            let price = unit_price(record, self.fields.mass);
            let mass = unit_mass(record, self.fields.mass);
            let buying = if !buyable {
                Err(OutfitRefusal::NotForSale)
            } else if i32::from(owned) >= i32::from(record.max) {
                Err(if record.max <= 0 {
                    OutfitRefusal::NoneAllowed
                } else {
                    OutfitRefusal::MaxOwned
                })
            } else if self.fighter_room.get(&record.id) == Some(&0) {
                Err(OutfitRefusal::MaxOwned)
            } else if mass < 0 && self.fields.holds < 0 {
                Err(OutfitRefusal::NoExpansion)
            } else if mass > free {
                Err(if owned == 0 {
                    OutfitRefusal::NoSpaceForAny
                } else {
                    OutfitRefusal::NoSpace
                })
            } else if price > pilot.cash {
                Err(OutfitRefusal::CannotAfford)
            } else {
                Ok(())
            };
            let selling = if owned == 0 {
                Err(OutfitRefusal::NoneOwned)
            } else if record.flags & OutfitFlags::CANNOT_SELL != 0 {
                Err(OutfitRefusal::CannotSell)
            } else if !for_sale && !sells_anywhere {
                Err(OutfitRefusal::NotBoughtHere)
            } else if free + mass < 0 {
                Err(OutfitRefusal::NegativeFreeMass)
            } else {
                Ok(())
            };
            rows.push((
                record.disp_weight,
                OutfitRow {
                    id: record.id,
                    name: record.name.clone(),
                    short_name: record.short_name.clone(),
                    price,
                    mass,
                    owned,
                    max: record.max,
                    buy: buying,
                    sell: selling,
                },
            ));
        }
        wares::in_display_order(&mut rows, |(weight, row)| (*weight, row.id.0));
        Some(Outfitter {
            rows: rows.into_iter().map(|(_, row)| row).collect(),
            cash: pilot.cash,
            free_mass: free,
        })
    }
}

/// Settles one of `record` at `price` as `direction` says: a buy pays,
/// the outfit then going through the session's grant path (see the
/// module docs); a sale is paid and removes one.
pub(crate) fn settle(pilot: &mut Pilot, record: &OutfitRecord, direction: Direction, price: i64) {
    match direction {
        Direction::Buy => pilot.cash = pilot.cash.saturating_sub(price),
        Direction::Sell => {
            pilot.cash = pilot.cash.saturating_add(resale(price));
            let owned = pilot.owned(record.id).saturating_sub(1);
            if owned == 0 {
                pilot.outfits.remove(&record.id);
            } else {
                pilot.outfits.insert(record.id, owned);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Test;
    use crate::stats::{MORE_FUEL, MORE_SPEED};
    use crate::testkit::{AllowAll, FAST, RefuseBits, catalog, outfit, planet};

    /// An outfitter of tech level 4 with special tech 6 and 55, of
    /// government 128.
    fn port() -> LandingSite {
        let mut special_tech = [0; 8];
        special_tech[..2].copy_from_slice(&[6, 55]);
        LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
            tech_level: 4,
            special_tech,
            govt: Some(GovtId(128)),
            ..planet(128, 0.0, 0.0)
        }
    }

    /// A new pilot with 10,000 credits and nothing owned, in the FAST
    /// ship (mass 40, 30 tons free, contributing bit 0x1).
    fn pilot() -> Pilot {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        pilot.cash = 10_000;
        pilot
    }

    fn owning(owned: &[(i16, u16)]) -> Pilot {
        let mut pilot = pilot();
        pilot.outfits = owned.iter().map(|&(id, n)| (OutfitId(id), n)).collect();
        pilot
    }

    fn tech(id: i16, tech_level: i16) -> OutfitRecord {
        OutfitRecord {
            tech_level,
            ..outfit(id, &[])
        }
    }

    fn requiring(id: i16, require: u64, require_govt: i16) -> OutfitRecord {
        OutfitRecord {
            require,
            require_govt,
            ..outfit(id, &[])
        }
    }

    const NO_DEFAULTS: BTreeMap<OutfitId, u16> = BTreeMap::new();
    const NO_FIGHTERS: BTreeMap<OutfitId, u32> = BTreeMap::new();

    fn open_at(records: &[OutfitRecord], site: &LandingSite, pilot: &Pilot) -> Option<Outfitter> {
        Shop {
            records,
            fields: FAST,
            defaults: &NO_DEFAULTS,
            site,
            fighter_room: &NO_FIGHTERS,
            gate: Gate::FRESH,
        }
        .outfitter(pilot)
    }

    fn open_gated(records: &[OutfitRecord], gate: Gate, pilot: &Pilot) -> Outfitter {
        Shop {
            records,
            fields: FAST,
            defaults: &NO_DEFAULTS,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            gate,
        }
        .outfitter(pilot)
        .expect("an outfitter")
    }

    fn open(records: &[OutfitRecord], pilot: &Pilot) -> Outfitter {
        open_at(records, &port(), pilot).expect("an outfitter")
    }

    fn listed(outfitter: &Outfitter) -> Vec<i16> {
        outfitter.rows.iter().map(|row| row.id.0).collect()
    }

    fn row(outfitter: &Outfitter, id: i16) -> &OutfitRow {
        outfitter.row(OutfitId(id)).expect("listed")
    }

    // What a stellar sells.

    #[test]
    fn a_stellar_without_the_outfitter_flag_has_none() {
        let site = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER,
            ..port()
        };
        assert_eq!(open_at(&[tech(128, 1)], &site, &pilot()), None);
    }

    #[test]
    fn it_sells_up_to_its_tech_level_and_its_special_tech_exactly() {
        let records = [
            tech(128, 3),
            tech(129, 4),
            tech(130, 5),
            tech(131, 6),
            tech(132, 55),
            tech(133, 7),
            tech(134, 0),
            tech(135, -3),
        ];
        let outfitter = open(&records, &pilot());
        assert_eq!(listed(&outfitter), [128, 129, 131, 132, 134, 135]);
        assert!(outfitter.rows.iter().all(|row| row.buy.is_ok()));
    }

    #[test]
    fn an_unused_special_tech_slot_sells_nothing() {
        let site = LandingSite {
            tech_level: -5,
            special_tech: [0, -1, 0, 0, 0, 0, 0, 0],
            ..port()
        };
        let outfitter = open_at(&[tech(128, 0), tech(129, -1)], &site, &pilot()).expect("open");
        assert_eq!(listed(&outfitter), Vec::<i16>::new());
        assert!(tech_allows(&tech(128, -5), &site));
        assert!(tech_allows(&tech(128, -6), &site));
    }

    #[test]
    fn rows_go_by_display_weight_highest_first_then_by_id() {
        let weighted = |id, disp_weight| OutfitRecord {
            disp_weight,
            ..outfit(id, &[])
        };
        let records = [
            weighted(128, 10),
            weighted(131, 50),
            weighted(129, 50),
            weighted(130, -1),
            weighted(132, 10),
        ];
        assert_eq!(listed(&open(&records, &pilot())), [129, 131, 128, 132, 130]);
    }

    // Require and RequireGovt.

    #[test]
    fn require_is_met_by_the_ship_or_an_owned_outfit() {
        let licence = OutfitRecord {
            contribute: 0x0800_0000_0000,
            ..outfit(140, &[])
        };
        let records = [
            requiring(128, 0x1, -1),
            requiring(129, 0x2, -1),
            requiring(130, 0x0800_0000_0001, -1),
            licence,
        ];
        let outfitter = open(&records, &pilot());
        assert_eq!(listed(&outfitter), [128, 129, 130, 140], "listed anyway");
        assert_eq!(row(&outfitter, 128).buy, Ok(()), "the ship's bit");
        assert_eq!(row(&outfitter, 129).buy, Err(OutfitRefusal::NotForSale));
        assert_eq!(row(&outfitter, 130).buy, Err(OutfitRefusal::NotForSale));
        let licensed = open(&records, &owning(&[(140, 1)]));
        assert_eq!(row(&licensed, 130).buy, Ok(()), "the licence's bit too");
        assert_eq!(row(&licensed, 129).buy, Err(OutfitRefusal::NotForSale));
    }

    #[test]
    fn an_owned_outfit_with_no_record_contributes_nothing_and_is_not_listed() {
        let records = [requiring(128, 0x2, -1)];
        let outfitter = open(&records, &owning(&[(999, 3)]));
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(row(&outfitter, 128).buy, Err(OutfitRefusal::NotForSale));
    }

    #[test]
    fn require_govt_says_where_require_applies() {
        let ours = Some(GovtId(128));
        for (require_govt, here, applies) in [
            (-1, ours, true),
            (127, ours, true),
            (0, None, true),
            (384, ours, true),
            (1127, ours, true),
            (128, ours, true),
            (128, Some(GovtId(129)), false),
            (128, None, false),
            (383, Some(GovtId(383)), true),
            (1128, ours, true),
            (1128, None, true),
            (1128, Some(GovtId(129)), false),
            (1383, Some(GovtId(383)), true),
            (2128, ours, false),
            (2128, None, true),
            (2128, Some(GovtId(129)), true),
            (2383, Some(GovtId(383)), false),
            (3128, ours, false),
            (3128, None, false),
            (3128, Some(GovtId(129)), true),
            (3383, Some(GovtId(383)), false),
            (3384, None, true),
        ] {
            assert_eq!(
                requirements_apply(require_govt, here),
                applies,
                "{require_govt} at {here:?}"
            );
        }
    }

    #[test]
    fn requirements_scoped_to_another_government_are_met_here() {
        let records = [requiring(128, 0x2, 129), requiring(129, 0x2, 128)];
        let outfitter = open(&records, &pilot());
        assert_eq!(row(&outfitter, 128).buy, Ok(()), "not applied here");
        assert_eq!(row(&outfitter, 129).buy, Err(OutfitRefusal::NotForSale));
    }

    // Flags that hide.

    #[test]
    fn hide_unless_required_hides_an_unmet_outfit_unless_one_is_owned() {
        let hidden = OutfitRecord {
            flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
            ..requiring(128, 0x2, -1)
        };
        let met = OutfitRecord {
            flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
            ..requiring(129, 0x1, -1)
        };
        let records = [hidden, met];
        assert_eq!(listed(&open(&records, &pilot())), [129]);
        let owned = open(&records, &owning(&[(128, 1)]));
        assert_eq!(listed(&owned), [128, 129]);
        assert_eq!(row(&owned, 128).buy, Err(OutfitRefusal::NotForSale));
        assert_eq!(row(&owned, 128).sell, Ok(()));
    }

    #[test]
    fn each_hiding_flag_hides_only_for_its_own_test() {
        let required = OutfitFlags::HIDE_UNLESS_REQUIRED;
        let available = OutfitFlags::HIDE_UNLESS_AVAILABLE;
        for (flags, met, holds, hidden) in [
            (0, false, false, false),
            (required, false, true, true),
            (required, true, false, false),
            (available, true, false, true),
            (available, false, true, false),
            (required | available, true, true, false),
            (required | available, false, true, true),
            (required | available, true, false, true),
            (!(required | available), false, false, false),
        ] {
            assert_eq!(
                hides(flags, met, holds),
                hidden,
                "{flags:#06x} {met} {holds}"
            );
        }
    }

    #[test]
    fn a_bit_both_the_ship_and_an_outfit_contribute_is_still_met() {
        let records = [
            OutfitRecord {
                contribute: 0x3,
                ..outfit(140, &[])
            },
            requiring(128, 0x3, -1),
        ];
        let outfitter = open(&records, &owning(&[(140, 1)]));
        assert_eq!(row(&outfitter, 128).buy, Ok(()));
    }

    fn gated(id: i16, flags: u16, availability: &str) -> OutfitRecord {
        OutfitRecord {
            flags,
            availability: Test::parse(availability),
            ..outfit(id, &[])
        }
    }

    #[test]
    fn an_outfit_whose_availability_is_refused_is_listed_but_not_for_sale() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let records = [gated(128, 0, "b7"), gated(129, 0, "b8")];
        let outfitter = open_gated(&records, refusing, &pilot());
        assert_eq!(listed(&outfitter), [128, 129]);
        assert_eq!(row(&outfitter, 128).buy, Err(OutfitRefusal::NotForSale));
        assert_eq!(row(&outfitter, 129).buy, Ok(()), "another bit holds");
    }

    #[test]
    fn hide_unless_available_hides_a_refused_outfit_unless_one_is_owned() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let records = [gated(128, OutfitFlags::HIDE_UNLESS_AVAILABLE, "b7")];
        assert!(listed(&open_gated(&records, refusing, &pilot())).is_empty());
        let owner = owning(&[(128, 1)]);
        let outfitter = open_gated(&records, refusing, &owner);
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(row(&outfitter, 128).buy, Err(OutfitRefusal::NotForSale));
        assert_eq!(row(&outfitter, 128).sell, Ok(()), "still sold back");
    }

    #[test]
    fn a_new_pilots_clear_bit_refuses_an_outfit_by_novas_bits() {
        let records = [
            gated(128, OutfitFlags::HIDE_UNLESS_AVAILABLE, "b7"),
            gated(129, 0, "b7"),
            gated(130, OutfitFlags::HIDE_UNLESS_AVAILABLE, "!b7"),
        ];
        let outfitter = open(&records, &pilot());
        assert_eq!(listed(&outfitter), [129, 130]);
        assert_eq!(row(&outfitter, 129).buy, Err(OutfitRefusal::NotForSale));
        assert_eq!(row(&outfitter, 130).buy, Ok(()));
    }

    #[test]
    fn a_malformed_availability_is_never_met() {
        let records = [
            gated(128, 0, "b1 &"),
            gated(129, OutfitFlags::HIDE_UNLESS_AVAILABLE, "b1 &"),
        ];
        let allowing = Gate {
            control_bits: &AllowAll,
            ..Gate::FRESH
        };
        let outfitter = open_gated(&records, allowing, &pilot());
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(row(&outfitter, 128).buy, Err(OutfitRefusal::NotForSale));
    }

    #[test]
    fn a_refused_outfit_hides_nothing_higher() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let records = [
            OutfitRecord {
                disp_weight: 5,
                ..gated(129, OutfitFlags::HIDE_HIGHER, "b7")
            },
            OutfitRecord {
                disp_weight: 5,
                ..outfit(130, &[])
            },
        ];
        assert_eq!(
            listed(&open_gated(&records, refusing, &pilot())),
            [129, 130]
        );
    }

    #[test]
    fn hide_higher_takes_higher_numbered_outfits_of_its_weight_off_sale() {
        let weighted = |id, disp_weight, flags| OutfitRecord {
            disp_weight,
            flags,
            ..outfit(id, &[])
        };
        let records = [
            weighted(128, 5, 0),
            weighted(129, 5, OutfitFlags::HIDE_HIGHER),
            weighted(130, 5, 0),
            weighted(131, 6, 0),
            weighted(132, 5, OutfitFlags::HIDE_HIGHER),
        ];
        assert_eq!(listed(&open(&records, &pilot())), [131, 128, 129]);
        // Owned, a hidden outfit is still not for sale, so not listed...
        assert_eq!(
            listed(&open(&records, &owning(&[(130, 1)]))),
            [131, 128, 129]
        );
        // ...and one that cannot be bought hides nothing.
        let unmet = [
            OutfitRecord {
                require: 0x2,
                ..weighted(129, 5, OutfitFlags::HIDE_HIGHER)
            },
            weighted(130, 5, 0),
        ];
        assert_eq!(listed(&open(&unmet, &pilot())), [129, 130]);
        let off_tech = [
            OutfitRecord {
                tech_level: 9,
                ..weighted(129, 5, OutfitFlags::HIDE_HIGHER)
            },
            weighted(130, 5, 0),
        ];
        assert_eq!(listed(&open(&off_tech, &pilot())), [130]);
    }

    // Price, mass and free mass.

    #[test]
    fn the_price_is_the_cost_or_the_cost_times_the_ships_mass() {
        let plain = OutfitRecord {
            cost: 1500,
            ..outfit(128, &[])
        };
        assert_eq!(unit_price(&plain, 40), 1500);
        let by_mass = OutfitRecord {
            flags: OutfitFlags::PRICE_BY_MASS,
            ..plain.clone()
        };
        assert_eq!(unit_price(&by_mass, 40), 60_000);
        assert_eq!(unit_price(&by_mass, -2), 0, "never below none");
        let negative = OutfitRecord { cost: -5, ..plain };
        assert_eq!(unit_price(&negative, 40), 0);
        let most = OutfitRecord {
            cost: i32::MAX,
            ..by_mass
        };
        assert_eq!(
            unit_price(&most, i16::MAX),
            i64::from(i32::MAX) * i64::from(i16::MAX)
        );
    }

    #[test]
    fn the_mass_is_the_items_or_scaled_by_the_ship_when_positive() {
        let scaled = |mass| OutfitRecord {
            mass,
            flags: OutfitFlags::MASS_BY_MASS,
            ..outfit(128, &[])
        };
        assert_eq!(unit_mass(&scaled(10), 155), 15, "truncated");
        assert_eq!(unit_mass(&scaled(-5), 155), -5, "positive masses only");
        assert_eq!(unit_mass(&scaled(0), 155), 0);
        let plain = OutfitRecord {
            mass: 7,
            ..outfit(128, &[])
        };
        assert_eq!(unit_mass(&plain, 155), 7);
    }

    #[test]
    fn free_mass_counts_the_default_items_and_takes_away_what_is_owned() {
        let records = [
            OutfitRecord {
                mass: 3,
                ..outfit(128, &[])
            },
            OutfitRecord {
                mass: -5,
                ..outfit(129, &[])
            },
            OutfitRecord {
                mass: 50,
                flags: OutfitFlags::MASS_BY_MASS,
                ..outfit(130, &[])
            },
        ];
        let map = |pairs: &[(i16, u16)]| -> BTreeMap<OutfitId, u16> {
            pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
        };
        let defaults = map(&[(128, 2)]);
        assert_eq!(free_mass(FAST, &defaults, &defaults, &records), 30);
        assert_eq!(free_mass(FAST, &defaults, &map(&[]), &records), 36);
        assert_eq!(
            free_mass(FAST, &defaults, &map(&[(128, 2), (129, 1)]), &records),
            35
        );
        assert_eq!(
            free_mass(FAST, &NO_DEFAULTS, &map(&[(130, 1)]), &records),
            10
        );
        assert_eq!(
            free_mass(FAST, &NO_DEFAULTS, &map(&[(999, 4)]), &records),
            30
        );
        let outfitter = open(&records, &owning(&[(128, 3)]));
        assert_eq!(outfitter.free_mass, 21);
        assert_eq!(outfitter.cash, 10_000);
        assert_eq!(
            (row(&outfitter, 128).price, row(&outfitter, 128).mass),
            (1000, 3)
        );
        assert_eq!(row(&outfitter, 130).mass, 20);
    }

    #[test]
    fn a_row_carries_the_outfits_names_owned_count_and_max() {
        let record = OutfitRecord {
            name: "Battery Pack".to_owned(),
            short_name: "Battery\\nPack".to_owned(),
            max: 7,
            ..outfit(128, &[(MORE_FUEL, 100)])
        };
        let outfitter = open(&[record], &owning(&[(128, 2)]));
        assert_eq!(
            outfitter.rows,
            [OutfitRow {
                id: OutfitId(128),
                name: "Battery Pack".to_owned(),
                short_name: "Battery\\nPack".to_owned(),
                price: 1000,
                mass: 1,
                owned: 2,
                max: 7,
                buy: Ok(()),
                sell: Ok(()),
            }]
        );
    }

    // Buying.

    /// A 10-ton outfit for 4000 credits, up to 2 owned.
    fn heavy() -> OutfitRecord {
        OutfitRecord {
            mass: 10,
            cost: 4000,
            max: 2,
            ..outfit(128, &[])
        }
    }

    fn buy(outfitter: &Outfitter) -> Result<(), OutfitRefusal> {
        outfitter.check(OutfitOrder {
            outfit: OutfitId(128),
            direction: Direction::Buy,
        })
    }

    fn sell(outfitter: &Outfitter) -> Result<(), OutfitRefusal> {
        outfitter.check(OutfitOrder {
            outfit: OutfitId(128),
            direction: Direction::Sell,
        })
    }

    #[test]
    fn a_buy_is_refused_by_cash_alone() {
        let mut broke = pilot();
        broke.cash = 3999;
        assert_eq!(
            buy(&open(&[heavy()], &broke)),
            Err(OutfitRefusal::CannotAfford)
        );
        broke.cash = 4000;
        assert_eq!(buy(&open(&[heavy()], &broke)), Ok(()), "exactly enough");
    }

    #[test]
    fn a_buy_is_refused_by_free_mass_alone() {
        let records = [
            heavy(),
            OutfitRecord {
                mass: 25,
                ..outfit(129, &[])
            },
        ];
        let full = open(&records, &owning(&[(129, 1)]));
        assert_eq!(full.free_mass, 5);
        assert_eq!(buy(&full), Err(OutfitRefusal::NoSpaceForAny));
        let one = open(
            &[
                OutfitRecord { max: 5, ..heavy() },
                OutfitRecord {
                    mass: 10,
                    ..outfit(129, &[])
                },
            ],
            &owning(&[(128, 1), (129, 1)]),
        );
        assert_eq!(one.free_mass, 10);
        assert_eq!(buy(&one), Ok(()), "exactly the space");
        let more = open(
            &[
                OutfitRecord { max: 5, ..heavy() },
                OutfitRecord {
                    mass: 11,
                    ..outfit(129, &[])
                },
            ],
            &owning(&[(128, 1), (129, 1)]),
        );
        assert_eq!(buy(&more), Err(OutfitRefusal::NoSpace));
    }

    #[test]
    fn a_fighter_outfit_is_refused_once_its_bays_have_no_room() {
        let fighters = OutfitRecord {
            mass: 0,
            max: 9999,
            ..heavy()
        };
        let room = |id: i16, n: u32| BTreeMap::from([(OutfitId(id), n)]);
        let open_with = |fighter_room: &BTreeMap<OutfitId, u32>, pilot: &Pilot| {
            Shop {
                records: std::slice::from_ref(&fighters),
                fields: FAST,
                defaults: &NO_DEFAULTS,
                site: &port(),
                fighter_room,
                gate: Gate::FRESH,
            }
            .outfitter(pilot)
            .expect("open")
        };
        assert_eq!(buy(&open_with(&room(128, 1), &pilot())), Ok(()));
        assert_eq!(
            buy(&open_with(&room(128, 0), &pilot())),
            Err(OutfitRefusal::MaxOwned),
            "the bays are full"
        );
        assert_eq!(
            buy(&open_with(&room(129, 0), &pilot())),
            Ok(()),
            "another outfit's room"
        );
        assert_eq!(
            buy(&open_with(&BTreeMap::new(), &owning(&[(128, 9998)]))),
            Ok(()),
            "not a fighter: its Max alone"
        );
        assert_eq!(
            buy(&open_with(&room(128, 5), &owning(&[(128, 9999)]))),
            Err(OutfitRefusal::MaxOwned),
            "its Max still holds"
        );
        assert_eq!(
            sell(&open_with(&room(128, 0), &owning(&[(128, 3)]))),
            Ok(()),
            "a full bay's fighters still sell"
        );
    }

    #[test]
    fn a_buy_is_refused_by_max_alone() {
        assert_eq!(buy(&open(&[heavy()], &owning(&[(128, 1)]))), Ok(()));
        assert_eq!(
            buy(&open(&[heavy()], &owning(&[(128, 2)]))),
            Err(OutfitRefusal::MaxOwned)
        );
        for max in [0, -1] {
            let none = OutfitRecord { max, ..heavy() };
            assert_eq!(
                buy(&open(&[none], &pilot())),
                Err(OutfitRefusal::NoneAllowed),
                "{max}"
            );
        }
    }

    #[test]
    fn a_buy_is_refused_when_the_outfit_cannot_be_bought_here() {
        let off = OutfitRecord {
            flags: OutfitFlags::SELL_ANYWHERE,
            tech_level: 9,
            ..heavy()
        };
        let outfitter = open(&[off], &owning(&[(128, 1)]));
        assert_eq!(buy(&outfitter), Err(OutfitRefusal::NotForSale));
        assert_eq!(
            outfitter.check(OutfitOrder {
                outfit: OutfitId(999),
                direction: Direction::Buy
            }),
            Err(OutfitRefusal::NotListed)
        );
    }

    #[test]
    fn a_mass_expansion_is_refused_on_negative_holds() {
        let expansion = OutfitRecord {
            mass: -5,
            ..outfit(128, &[])
        };
        assert_eq!(
            buy(&open(std::slice::from_ref(&expansion), &pilot())),
            Ok(())
        );
        let negative = Shop {
            records: std::slice::from_ref(&expansion),
            fields: ShipFields { holds: -1, ..FAST },
            defaults: &NO_DEFAULTS,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            gate: Gate::FRESH,
        }
        .outfitter(&pilot())
        .expect("open");
        assert_eq!(buy(&negative), Err(OutfitRefusal::NoExpansion));
        let massless = Shop {
            records: &[OutfitRecord {
                mass: 0,
                ..expansion.clone()
            }],
            fields: ShipFields { holds: -1, ..FAST },
            defaults: &NO_DEFAULTS,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            gate: Gate::FRESH,
        }
        .outfitter(&pilot())
        .expect("open");
        assert_eq!(buy(&massless), Ok(()));
        let empty_holds = Shop {
            records: std::slice::from_ref(&expansion),
            fields: ShipFields { holds: 0, ..FAST },
            defaults: &NO_DEFAULTS,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            gate: Gate::FRESH,
        }
        .outfitter(&pilot())
        .expect("open");
        assert_eq!(buy(&empty_holds), Ok(()), "only negative Holds forbids it");
    }

    // Selling.

    #[test]
    fn a_sale_is_refused_when_none_is_owned() {
        assert_eq!(
            sell(&open(&[heavy()], &pilot())),
            Err(OutfitRefusal::NoneOwned)
        );
        assert_eq!(sell(&open(&[heavy()], &owning(&[(128, 1)]))), Ok(()));
    }

    #[test]
    fn a_sale_is_refused_when_the_outfit_cannot_be_sold() {
        let unsellable = OutfitRecord {
            flags: OutfitFlags::CANNOT_SELL,
            ..heavy()
        };
        assert_eq!(
            sell(&open(&[unsellable], &owning(&[(128, 1)]))),
            Err(OutfitRefusal::CannotSell)
        );
    }

    #[test]
    fn a_sale_is_refused_where_the_outfit_is_not_for_sale_unless_it_sells_anywhere() {
        let off = OutfitRecord {
            tech_level: 9,
            ..heavy()
        };
        let outfitter = open(std::slice::from_ref(&off), &owning(&[(128, 1)]));
        assert_eq!(listed(&outfitter), Vec::<i16>::new(), "not listed there");
        let anywhere = OutfitRecord {
            flags: OutfitFlags::SELL_ANYWHERE,
            ..off
        };
        let outfitter = open(std::slice::from_ref(&anywhere), &owning(&[(128, 1)]));
        assert_eq!(listed(&outfitter), [128], "listed, sell-only");
        assert_eq!(buy(&outfitter), Err(OutfitRefusal::NotForSale));
        assert_eq!(sell(&outfitter), Ok(()));
        assert_eq!(listed(&open(&[anywhere], &pilot())), Vec::<i16>::new());
        // For sale here, an outfit sells back even when it cannot be
        // bought.
        let required = OutfitRecord {
            require: 0x2,
            flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
            ..heavy()
        };
        let outfitter = open(&[required], &owning(&[(128, 1)]));
        assert_eq!(sell(&outfitter), Ok(()), "for sale here, if not buyable");
    }

    #[test]
    fn a_sale_is_refused_when_it_would_leave_negative_free_mass() {
        let expansion = OutfitRecord {
            mass: -20,
            max: 5,
            ..outfit(128, &[])
        };
        let cargo = OutfitRecord {
            mass: 40,
            ..outfit(129, &[])
        };
        let records = [expansion, cargo];
        let tight = open(&records, &owning(&[(128, 1), (129, 1)]));
        assert_eq!(tight.free_mass, 10);
        assert_eq!(sell(&tight), Err(OutfitRefusal::NegativeFreeMass));
        let exact = open(&records, &owning(&[(128, 2), (129, 1)]));
        assert_eq!(exact.free_mass, 30);
        assert_eq!(sell(&exact), Ok(()), "leaves exactly none");
        let loose = open(&records, &owning(&[(128, 1)]));
        assert_eq!(sell(&loose), Ok(()));
        let to_none = [
            OutfitRecord {
                mass: -20,
                ..outfit(128, &[])
            },
            OutfitRecord {
                mass: 30,
                ..outfit(129, &[])
            },
        ];
        let zero = open(&to_none, &owning(&[(128, 1), (129, 1)]));
        assert_eq!(zero.free_mass, 20);
        assert_eq!(sell(&zero), Ok(()), "leaves none free");
    }

    // Settling.

    #[test]
    fn a_buy_only_pays_leaving_the_outfit_to_the_grant_path() {
        let mut pilot = pilot();
        settle(&mut pilot, &heavy(), Direction::Buy, 4000);
        assert_eq!((pilot.cash, pilot.owned(OutfitId(128))), (6000, 0));
        settle(&mut pilot, &heavy(), Direction::Buy, 4000);
        assert_eq!(pilot.cash, 2000);
        assert_eq!(pilot.outfits().count(), 0);
    }

    #[test]
    fn a_sale_pays_half_the_price_truncated_and_removes_one() {
        let mut pilot = owning(&[(128, 2)]);
        settle(&mut pilot, &heavy(), Direction::Sell, 4001);
        assert_eq!((pilot.cash, pilot.owned(OutfitId(128))), (12_000, 1));
        settle(&mut pilot, &heavy(), Direction::Sell, 4001);
        assert_eq!(pilot.cash, 14_000);
        assert_eq!(pilot.outfits().count(), 0, "none left is none listed");
        assert_eq!(resale(5_000_000), 2_500_000);
        assert_eq!(resale(15_001), 7_500);
        assert_eq!(resale(i64::MAX), i64::MAX / 100);
        assert_eq!(RESALE_PERCENT, 50);
    }

    // The mods of what is owned.

    #[test]
    fn the_owned_outfits_mods_are_each_records_four_with_the_count() {
        let records = [
            outfit(128, &[(MORE_SPEED, 100), (MORE_FUEL, 50)]),
            outfit(129, &[(MORE_FUEL, 10)]),
        ];
        let owned: BTreeMap<OutfitId, u16> = [(OutfitId(128), 2), (OutfitId(999), 1)]
            .into_iter()
            .collect();
        let mods = outfit_mods(&owned, &records);
        let at = |mod_type, mod_val, count| OutfitMod {
            mod_type,
            mod_val,
            count,
        };
        assert_eq!(
            mods,
            [
                at(MORE_SPEED, 100, 2),
                at(MORE_FUEL, 50, 2),
                at(0, 0, 2),
                at(0, 0, 2)
            ]
        );
    }
}
