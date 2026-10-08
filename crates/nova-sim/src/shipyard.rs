//! The shipyard: which ship classes (`shïp`) a stellar lists and sells, at
//! what price, what the player's ship trades in for, and buying a new one.
//!
//! # What a stellar sells
//!
//! A ship is *for sale* at a stellar with a shipyard when its `BuyRandom`
//! is above 0 and its `TechLevel` is allowed there, as an outfit's is
//! ([`wares::tech_allows`]). The Bible's `shïp` gives `BuyRandom` as "the
//! percent chance that a ship of this type will be available for purchase
//! on a given day. A `BuyRandom` of 0 means this ship will never be made
//! available for purchase": unlike an outfit's, 0 or below is never for
//! sale (stock data has 200 such NPC-only ships). A value from 1 up is not
//! rolled yet: every such ship is for sale every day, and a value above
//! 100 would count as 100.
//!
//! A ship for sale can be *bought* when its `Require` bits are met, by the
//! `Contribute` of the ship flown and of the outfits the player owns (a
//! `shïp` has no `RequireGovt`, so `Require` applies everywhere), and its
//! `Availability` holds. `Availability` is tested through the
//! [`ControlBits`](crate::ControlBits) port for the player's pilot (a
//! [`Gate`]), as `_SetupPortAvailableShipTypes` (@0xbd52) and
//! `_CalcShipCanBuy` (@0x4f901) do; one that did not parse never holds.
//!
//! # What it lists
//!
//! The shipyard lists every ship for sale, except one flagged
//! [`ShipFlags3::HIDE_UNLESS_AVAILABLE`] whose `Availability` does not
//! hold, or [`ShipFlags3::HIDE_UNLESS_REQUIRED`] whose `Require` is not
//! met; otherwise one that cannot be bought shows greyed (the Bible: it
//! "might appear in the shipyard but not be able to be purchased"). A ship
//! flagged [`ShipFlags3::HIDE_HIGHER`] that can be bought takes every
//! higher-numbered ship of the same `DispWeight` off sale. The ship class
//! flown is listed like any other, and can be bought again. Rows go by
//! `DispWeight`, highest first, then by ID.
//!
//! # Price and trade-in
//!
//! A ship's price is its `Cost`, never below none. The player's ship trades
//! in for [`TRADE_IN_PERCENT`] of its class's `Cost`, plus what each
//! outfit it carries would sell back for at the outfitter
//! ([`resale`]), except a persistent one, which the player
//! keeps, and one that cannot be sold. The final price is the price less
//! the trade-in, and may be negative: the player is then paid. A buy is
//! refused when the ship cannot be bought here, or when the cash and the
//! trade-in together do not cover the price.
//!
//! # Buying a ship
//!
//! Buying replaces the ship class flown, and the cash changes by the
//! trade-in less the price. Every outfit that is not persistent
//! ([`OutfitFlags::PERSISTENT`], `oütf` flag 0x0004: "This item stays with
//! you when you trade ships") goes with the old ship, its value in the
//! trade-in. Each persistent one carries over as far as the new ship
//! allows: first no more than its `Max`; then, by ascending ID, one at a
//! time while the new ship still has free mass for it with its own default
//! items fitted (they are fitted regardless, so their mass is not room),
//! and never a mass expansion onto a ship whose `Holds` is negative. Every
//! persistent one that does not carry over is sold back at the outfitter's
//! rate, priced on the old ship, and the refund is credited on top (it
//! does not count towards affording the ship). The new ship's default
//! items are then added on top of those carried over, even beyond their
//! `Max`, as the original fits them regardless.
//!
//! The cargo that fits in the new ship's cargo space is kept, goods in
//! [`Good`] order (commodities by number, then `jünk` by ID) until the
//! hold is full; the rest is left behind, unpaid, as the phase's "keeps
//! cargo that fits" says (the Bible and the forums are silent). The new
//! ship comes with its shield, armour and fuel full, as a new hull: the
//! Bible is silent here too. Everything else about the pilot (where it is,
//! the date, the course, its legal records and the events under way)
//! stays as it was.
//!
//! The evidence for the trade-in: the Bible's `Cost` says "the cost of the
//! new ship minus 25% of the original cost of your current ship and
//! upgrades"; an Ambrosia forum answer (Forum26 #004247, 2002) gives "25%
//! their original cost, plus 50% the cost of all upgrades"; and the
//! original engine's own debug log, reproduced from stock data, reads
//! "Striker (262) has a trade-in value of 1163000", which is 25 % of its
//! 1,000,000 plus 50 % of its outfits' 1,825,000. The forums add that the
//! trade-in leaves persistent outfits out, and that a trade-in worth more
//! than the new ship pays the difference.
//!
//! Not modelled yet: a ship's stock weapons (`WeapType`/`WeapCount`),
//! which are neither fitted nor counted in the trade-in; the gun and turret
//! limits (`MaxGun`, `MaxTur`); `OnPurchase` and `OnRetire`; naming the new
//! ship and its `Long Name` message; `MovieFile`; and escorts' `UpgradeTo`.
//! Nor does the shipyard roll `BuyRandom` or price a ship through the
//! original's tech-level flux, as hiring in the bar does with its own
//! `HireRandom` roll and price rule ([`hire`](crate::hire)).

use std::collections::BTreeMap;

use crate::catalog::{LandingSite, OutfitId, OutfitRecord, ShipId, ShipRecord};
use crate::control::Gate;
use crate::handling::ShipFields;
use crate::landing::StellarFlags;
use crate::market::Good;
use crate::outfitter::{OutfitFlags, free_mass, outfit_mods, resale, unit_mass, unit_price};
use crate::pilot::{Pilot, tally};
use crate::stats::ShipStats;
use crate::wares::{self, HideBits, HideHigher};

/// The `shïp` `Flags3` bits the shipyard reads (the Bible): not the same
/// values as the `oütf` flags of the same meaning.
#[derive(Clone, Copy, Debug)]
pub struct ShipFlags3;

impl ShipFlags3 {
    /// Not shown unless its `Availability` holds.
    pub const HIDE_UNLESS_AVAILABLE: u16 = 0x0100;
    /// Not shown unless the player meets its `Require`.
    pub const HIDE_UNLESS_REQUIRED: u16 = 0x0200;
    /// For sale, it takes higher-numbered ships of equal `DispWeight` off
    /// sale.
    pub const HIDE_HIGHER: u16 = 0x4000;
}

/// The `Flags3` bits that hide a ship.
pub const HIDE_BITS: HideBits = HideBits {
    unless_required: ShipFlags3::HIDE_UNLESS_REQUIRED,
    unless_available: ShipFlags3::HIDE_UNLESS_AVAILABLE,
};

/// What the player's ship class trades in for, as a percentage of its
/// `Cost`: see the module's evidence.
pub const TRADE_IN_PERCENT: i64 = 25;

/// Why a ship cannot be bought.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipRefusal {
    /// There is no shipyard: the ship is not landed, or the stellar has
    /// none.
    NoShipyard,
    /// The shipyard does not list the ship.
    NotListed,
    /// The ship is listed but cannot be bought here: its `Require` is not
    /// met, or its `Availability` does not hold.
    NotForSale,
    /// The cash and the trade-in together do not cover its price.
    CannotAfford,
}

/// One ship class in a stellar's shipyard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipRow {
    /// The ship class.
    pub id: ShipId,
    /// Its name.
    pub name: String,
    /// Its `ShortName`, raw.
    pub short_name: String,
    /// Its price.
    pub price: i64,
    /// What the shipyard's Info shows of it.
    pub specs: ShipSpecs,
    /// Whether it can be bought now, or why not.
    pub buy: Result<(), ShipRefusal>,
}

/// What the shipyard's Info shows of a ship class, raw from its record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipSpecs {
    /// Its handling, reserve, cargo and mass fields.
    pub fields: ShipFields,
    /// Its `MaxGun`.
    pub max_gun: i16,
    /// Its `MaxTur`.
    pub max_tur: i16,
    /// Its `Length`, in metres.
    pub length: i16,
    /// Its `Crew`.
    pub crew: i16,
}

/// A stellar's shipyard, as the player sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shipyard {
    /// The ships listed, highest `DispWeight` first, then by ID.
    pub rows: Vec<ShipRow>,
    /// What the player's ship trades in for, whichever is bought.
    pub trade_in: i64,
    /// The player's credits.
    pub cash: i64,
    /// The ship class flown.
    pub current: ShipId,
}

impl Shipyard {
    /// `ship`'s row, if it is listed.
    #[must_use]
    pub fn row(&self, ship: ShipId) -> Option<&ShipRow> {
        self.rows.iter().find(|row| row.id == ship)
    }

    /// Whether `ship` can be bought now, or why not.
    pub fn check(&self, ship: ShipId) -> Result<(), ShipRefusal> {
        self.row(ship).ok_or(ShipRefusal::NotListed)?.buy
    }

    /// What buying `row`'s ship costs after the trade-in: negative when
    /// the player is paid.
    #[must_use]
    pub fn final_price(&self, row: &ShipRow) -> i64 {
        row.price.saturating_sub(self.trade_in)
    }
}

/// Whether a ship with this `BuyRandom` is ever for sale: above 0.
#[must_use]
pub fn buy_random_allows(buy_random: i16) -> bool {
    buy_random > 0
}

/// `ship`'s price: its `Cost`, never below none.
#[must_use]
pub fn ship_price(ship: &ShipRecord) -> i64 {
    i64::from(ship.cost).max(0)
}

/// What a ship of class `cost` and `mass` carrying `owned` trades in for:
/// [`TRADE_IN_PERCENT`] of its `Cost` (never below none), plus the resale
/// of each outfit owned that has a record and is neither persistent nor
/// unsellable, priced on that ship.
#[must_use]
pub fn trade_in(
    cost: i32,
    mass: i16,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> i64 {
    let hull = i64::from(cost).max(0) * TRADE_IN_PERCENT / 100;
    let kept = OutfitFlags::PERSISTENT | OutfitFlags::CANNOT_SELL;
    records
        .iter()
        .filter(|record| record.flags & kept == 0)
        .filter_map(|record| Some((record, *owned.get(&record.id)?)))
        .fold(hull, |total, (record, count)| {
            let each = resale(unit_price(record, mass));
            total.saturating_add(each.saturating_mul(i64::from(count)))
        })
}

/// Everything the shipyard rules read about the ship and where it is.
pub(crate) struct Yard<'a> {
    /// Every `shïp`.
    pub(crate) ships: &'a [ShipRecord],
    /// Every `oütf`.
    pub(crate) outfits: &'a [OutfitRecord],
    /// The ship flown's fields.
    pub(crate) fields: ShipFields,
    /// The stellar landed on.
    pub(crate) site: &'a LandingSite,
    /// The control-bit test of a ship's `Availability`.
    pub(crate) gate: Gate<'a>,
}

impl Yard<'_> {
    /// What `pilot`'s ship trades in for; a class with no record has no
    /// `Cost`.
    pub(crate) fn trade_in(&self, pilot: &Pilot) -> i64 {
        let cost = self
            .ships
            .iter()
            .find(|ship| ship.id == pilot.ship)
            .map_or(0, |ship| ship.cost);
        trade_in(cost, self.fields.mass, &pilot.outfits, self.outfits)
    }

    /// The shipyard, for `pilot`; `None` when the stellar has none.
    pub(crate) fn shipyard(&self, pilot: &Pilot) -> Option<Shipyard> {
        if self.site.flags & StellarFlags::SHIPYARD == 0 {
            return None;
        }
        let contributed = wares::contributed(self.fields.contribute, &pilot.outfits, self.outfits);
        let trade_in = self.trade_in(pilot);
        let mut sorted: Vec<&ShipRecord> = self.ships.iter().collect();
        sorted.sort_by_key(|ship| ship.id);
        let mut sweep = HideHigher::default();
        let mut rows = Vec::new();
        for ship in sorted {
            let required = wares::requirement_met(ship.require, contributed);
            let available = self.gate.allows(&ship.availability);
            let for_sale = buy_random_allows(ship.buy_random)
                && wares::tech_allows(ship.tech_level, self.site)
                && sweep.on_sale(ship.disp_weight);
            let buyable = for_sale && required && available;
            sweep.note(
                ship.disp_weight,
                ship.flags3 & ShipFlags3::HIDE_HIGHER != 0,
                buyable,
            );
            if !for_sale || wares::hidden(ship.flags3, HIDE_BITS, required, available) {
                continue;
            }
            let price = ship_price(ship);
            let buy = if !buyable {
                Err(ShipRefusal::NotForSale)
            } else if pilot.cash.saturating_add(trade_in) < price {
                Err(ShipRefusal::CannotAfford)
            } else {
                Ok(())
            };
            rows.push((
                ship.disp_weight,
                ShipRow {
                    id: ship.id,
                    name: ship.name.clone(),
                    short_name: ship.short_name.clone(),
                    price,
                    specs: ShipSpecs {
                        fields: ship.fields,
                        max_gun: ship.max_gun,
                        max_tur: ship.max_tur,
                        length: ship.length,
                        crew: ship.crew,
                    },
                    buy,
                },
            ));
        }
        wares::in_display_order(&mut rows, |(weight, row)| (*weight, row.id.0));
        Some(Shipyard {
            rows: rows.into_iter().map(|(_, row)| row).collect(),
            trade_in,
            cash: pilot.cash,
            current: pilot.ship,
        })
    }
}

/// What buying a ship did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipPurchase {
    /// The new ship's price.
    pub price: i64,
    /// What the old ship traded in for.
    pub trade_in: i64,
    /// The persistent outfits that did not carry over, each with how many,
    /// sold back.
    pub sold_back: BTreeMap<OutfitId, u16>,
    /// What they were sold back for, credited.
    pub refund: i64,
    /// The cargo that did not fit, each good with its tons, left behind.
    pub left_behind: BTreeMap<Good, u32>,
}

/// The price and trade-in a purchase settles at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Quote {
    /// The new ship's price.
    pub(crate) price: i64,
    /// What the old ship trades in for.
    pub(crate) trade_in: i64,
}

/// `a` and `b` together, saturating.
fn merged(a: &BTreeMap<OutfitId, u16>, b: &BTreeMap<OutfitId, u16>) -> BTreeMap<OutfitId, u16> {
    let mut both = a.clone();
    for (&id, &count) in b {
        let owned = both.entry(id).or_default();
        *owned = owned.saturating_add(count);
    }
    both
}

/// Keeps as much of `pilot`'s cargo as `capacity` tons hold, goods in
/// [`Good`] order until the hold is full, and gives the rest, left
/// behind, each good with its tons (see the module docs).
pub(crate) fn keep_cargo(pilot: &mut Pilot, capacity: u32) -> BTreeMap<Good, u32> {
    let mut room = capacity;
    let mut left_behind = BTreeMap::new();
    for (good, tons) in &mut pilot.cargo {
        let kept = (*tons).min(room);
        room -= kept;
        if kept < *tons {
            left_behind.insert(*good, *tons - kept);
        }
        *tons = kept;
    }
    pilot.cargo.retain(|_, tons| *tons > 0);
    left_behind
}

/// Buys `new` for `pilot`, at `quote`, from a ship of `old_mass`, as the
/// module says, and gives what it did.
pub(crate) fn purchase(
    pilot: &mut Pilot,
    old_mass: i16,
    new: &ShipRecord,
    quote: Quote,
    records: &[OutfitRecord],
) -> ShipPurchase {
    let defaults = tally(new.defaults.iter().copied());
    let mut carried: BTreeMap<OutfitId, u16> = BTreeMap::new();
    let mut sold_back: BTreeMap<OutfitId, u16> = BTreeMap::new();
    let mut refund: i64 = 0;
    let persistent = pilot.outfits.iter().filter_map(|(id, &count)| {
        let record = records.iter().find(|record| record.id == *id)?;
        (record.flags & OutfitFlags::PERSISTENT != 0).then_some((record, count))
    });
    for (record, count) in persistent {
        let most = u16::try_from(record.max).unwrap_or(0);
        let expansion_refused = unit_mass(record, new.fields.mass) < 0 && new.fields.holds < 0;
        for unit in 0..count {
            let mut trial = carried.clone();
            *trial.entry(record.id).or_default() += 1;
            let fits = unit < most
                && !expansion_refused
                && free_mass(new.fields, &defaults, &merged(&trial, &defaults), records) >= 0;
            if fits {
                carried = trial;
            } else {
                *sold_back.entry(record.id).or_default() += 1;
                refund = refund.saturating_add(resale(unit_price(record, old_mass)));
            }
        }
    }
    pilot.outfits = merged(&carried, &defaults);
    pilot.ship = new.id;
    pilot.cash = pilot
        .cash
        .saturating_sub(quote.price)
        .saturating_add(quote.trade_in)
        .saturating_add(refund);
    let stats = ShipStats::new(new.fields, &outfit_mods(&pilot.outfits, records));
    let left_behind = keep_cargo(pilot, stats.capacity);
    pilot.reserves = stats.full();
    ShipPurchase {
        price: quote.price,
        trade_in: quote.trade_in,
        sold_back,
        refund,
        left_behind,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{DisasterId, GovtId, JunkId, StellarId, SystemId};
    use crate::control::Test;
    use crate::market::MORE_CARGO;
    use crate::reserves::Reserves;
    use crate::stats::{MORE_FUEL, MORE_SHIELD};
    use crate::testkit::{AllowAll, FAST, RefuseBits, catalog, outfit, planet, ship};

    /// A shipyard of tech level 4 with special tech 6 and 55.
    fn port() -> LandingSite {
        let mut special_tech = [0; 8];
        special_tech[..2].copy_from_slice(&[6, 55]);
        LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::SHIPYARD,
            tech_level: 4,
            special_tech,
            govt: Some(GovtId(128)),
            ..planet(128, 0.0, 0.0)
        }
    }

    /// A new pilot with 10,000 credits and nothing owned, in ship 128, the
    /// FAST ship (mass 40, 30 tons free, 20 tons of cargo space,
    /// contributing bit 0x1).
    fn pilot() -> Pilot {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        pilot.cash = 10_000;
        pilot
    }

    fn owning(owned: &[(i16, u16)]) -> Pilot {
        let mut pilot = pilot();
        pilot.outfits = map(owned);
        pilot
    }

    fn map(pairs: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    fn open_at(
        ships: &[ShipRecord],
        outfits: &[OutfitRecord],
        site: &LandingSite,
        pilot: &Pilot,
    ) -> Option<Shipyard> {
        Yard {
            ships,
            outfits,
            fields: FAST,
            site,
            gate: Gate::FRESH,
        }
        .shipyard(pilot)
    }

    fn open(ships: &[ShipRecord], pilot: &Pilot) -> Shipyard {
        open_with(ships, &[], pilot)
    }

    fn open_with(ships: &[ShipRecord], outfits: &[OutfitRecord], pilot: &Pilot) -> Shipyard {
        open_at(ships, outfits, &port(), pilot).expect("a shipyard")
    }

    fn listed(shipyard: &Shipyard) -> Vec<i16> {
        shipyard.rows.iter().map(|row| row.id.0).collect()
    }

    fn row(shipyard: &Shipyard, id: i16) -> &ShipRow {
        shipyard.row(ShipId(id)).expect("listed")
    }

    /// Ship `id` at 1000 credits, `fields` FAST's.
    fn cheap(id: i16) -> ShipRecord {
        ShipRecord {
            cost: 1000,
            ..ship(id, FAST)
        }
    }

    fn tech(id: i16, tech_level: i16) -> ShipRecord {
        ShipRecord {
            tech_level,
            ..cheap(id)
        }
    }

    // What a stellar sells.

    #[test]
    fn a_stellar_without_the_shipyard_flag_has_none() {
        let site = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
            ..port()
        };
        assert_eq!(open_at(&[cheap(129)], &[], &site, &pilot()), None);
    }

    #[test]
    fn it_sells_up_to_its_tech_level_and_its_special_tech_exactly() {
        let ships = [
            tech(129, 3),
            tech(130, 4),
            tech(131, 5),
            tech(132, 6),
            tech(133, 55),
            tech(134, 7),
            tech(135, 0),
            tech(136, -3),
        ];
        let shipyard = open(&ships, &pilot());
        assert_eq!(listed(&shipyard), [129, 130, 132, 133, 135, 136]);
        assert!(shipyard.rows.iter().all(|row| row.buy.is_ok()));
    }

    #[test]
    fn buy_random_of_0_or_below_is_never_for_sale_and_any_other_always() {
        let random = |id, buy_random| ShipRecord {
            buy_random,
            ..cheap(id)
        };
        let ships = [
            random(129, 0),
            random(130, -1),
            random(131, 1),
            random(132, 100),
            random(133, 150),
            random(134, i16::MIN),
        ];
        assert_eq!(listed(&open(&ships, &pilot())), [131, 132, 133]);
        assert!(!buy_random_allows(0));
        assert!(buy_random_allows(1));
    }

    #[test]
    fn rows_go_by_display_weight_highest_first_then_by_id() {
        let weighted = |id, disp_weight| ShipRecord {
            disp_weight,
            ..cheap(id)
        };
        let ships = [
            weighted(129, 10),
            weighted(132, 50),
            weighted(130, 50),
            weighted(131, -1),
            weighted(133, 10),
        ];
        assert_eq!(listed(&open(&ships, &pilot())), [130, 132, 129, 133, 131]);
    }

    #[test]
    fn a_row_carries_the_ships_names_and_price_and_the_shipyard_the_current_ship() {
        let record = ShipRecord {
            name: "Heavy Shuttle".to_owned(),
            short_name: "Heavy\\nShuttle".to_owned(),
            cost: 17_500,
            max_gun: 3,
            max_tur: 1,
            length: 26,
            crew: 4,
            ..ship(129, FAST)
        };
        let mut rich = pilot();
        rich.cash = 50_000;
        let shipyard = open(&[record], &rich);
        assert_eq!(
            shipyard,
            Shipyard {
                rows: vec![ShipRow {
                    id: ShipId(129),
                    name: "Heavy Shuttle".to_owned(),
                    short_name: "Heavy\\nShuttle".to_owned(),
                    price: 17_500,
                    specs: ShipSpecs {
                        fields: FAST,
                        max_gun: 3,
                        max_tur: 1,
                        length: 26,
                        crew: 4,
                    },
                    buy: Ok(()),
                }],
                trade_in: 0,
                cash: 50_000,
                current: ShipId(128),
            }
        );
    }

    #[test]
    fn the_ship_flown_is_listed_and_can_be_bought_again() {
        let ships = [cheap(128), cheap(129)];
        let shipyard = open(&ships, &pilot());
        assert_eq!(listed(&shipyard), [128, 129]);
        assert_eq!(row(&shipyard, 128).buy, Ok(()));
        assert_eq!(shipyard.current, ShipId(128));
    }

    // Require, Availability and the flags that hide.

    fn requiring(id: i16, require: u64, flags3: u16) -> ShipRecord {
        ShipRecord {
            require,
            flags3,
            ..cheap(id)
        }
    }

    #[test]
    fn require_is_met_by_the_ship_or_an_owned_outfit_and_unmet_greys_the_row() {
        let licence = OutfitRecord {
            contribute: 0x0800_0000_0000,
            ..outfit(140, &[])
        };
        let ships = [
            requiring(129, 0x1, 0),
            requiring(130, 0x2, 0),
            requiring(131, 0x0800_0000_0001, 0),
        ];
        let outfits = [licence];
        let shipyard = open_with(&ships, &outfits, &pilot());
        assert_eq!(listed(&shipyard), [129, 130, 131], "listed anyway");
        assert_eq!(row(&shipyard, 129).buy, Ok(()), "the ship's bit");
        assert_eq!(row(&shipyard, 130).buy, Err(ShipRefusal::NotForSale));
        assert_eq!(row(&shipyard, 131).buy, Err(ShipRefusal::NotForSale));
        let licensed = open_with(&ships, &outfits, &owning(&[(140, 1)]));
        assert_eq!(row(&licensed, 131).buy, Ok(()), "the licence's bit too");
        assert_eq!(row(&licensed, 130).buy, Err(ShipRefusal::NotForSale));
    }

    #[test]
    fn hide_unless_required_hides_an_unmet_ship() {
        let ships = [
            requiring(129, 0x2, ShipFlags3::HIDE_UNLESS_REQUIRED),
            requiring(130, 0x1, ShipFlags3::HIDE_UNLESS_REQUIRED),
            requiring(131, 0x2, OutfitFlags::HIDE_UNLESS_REQUIRED),
        ];
        assert_eq!(
            listed(&open(&ships, &pilot())),
            [130, 131],
            "the oütf bit hides nothing on a shïp"
        );
    }

    fn gated(id: i16, flags3: u16, availability: &str) -> ShipRecord {
        ShipRecord {
            flags3,
            availability: Test::parse(availability),
            ..cheap(id)
        }
    }

    fn open_gated(ships: &[ShipRecord], gate: Gate, pilot: &Pilot) -> Shipyard {
        Yard {
            ships,
            outfits: &[],
            fields: FAST,
            site: &port(),
            gate,
        }
        .shipyard(pilot)
        .expect("a shipyard")
    }

    #[test]
    fn a_ship_whose_availability_is_refused_shows_greyed() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let ships = [gated(129, 0, "b7"), gated(130, 0, "b8")];
        let shipyard = open_gated(&ships, refusing, &pilot());
        assert_eq!(listed(&shipyard), [129, 130]);
        assert_eq!(row(&shipyard, 129).buy, Err(ShipRefusal::NotForSale));
        assert_eq!(row(&shipyard, 130).buy, Ok(()), "another bit holds");
    }

    #[test]
    fn hide_unless_available_hides_a_refused_ship() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let ships = [gated(129, ShipFlags3::HIDE_UNLESS_AVAILABLE, "b7")];
        assert!(listed(&open_gated(&ships, refusing, &pilot())).is_empty());
    }

    #[test]
    fn a_new_pilots_clear_bit_refuses_a_ship_by_novas_bits() {
        let ships = [
            gated(129, ShipFlags3::HIDE_UNLESS_AVAILABLE, "b422"),
            gated(130, 0, "b422"),
            gated(131, ShipFlags3::HIDE_UNLESS_AVAILABLE, "!b422"),
        ];
        let shipyard = open(&ships, &pilot());
        assert_eq!(listed(&shipyard), [130, 131]);
        assert_eq!(row(&shipyard, 130).buy, Err(ShipRefusal::NotForSale));
        assert_eq!(row(&shipyard, 131).buy, Ok(()));
    }

    #[test]
    fn a_malformed_ship_availability_is_never_met() {
        let allowing = Gate {
            control_bits: &AllowAll,
            ..Gate::FRESH
        };
        let ships = [
            gated(129, 0, "b1 &"),
            gated(130, ShipFlags3::HIDE_UNLESS_AVAILABLE, "b1 &"),
        ];
        let shipyard = open_gated(&ships, allowing, &pilot());
        assert_eq!(listed(&shipyard), [129]);
        assert_eq!(row(&shipyard, 129).buy, Err(ShipRefusal::NotForSale));
    }

    #[test]
    fn the_ships_hiding_bits_are_flags3s_own() {
        let available = ShipFlags3::HIDE_UNLESS_AVAILABLE;
        let required = ShipFlags3::HIDE_UNLESS_REQUIRED;
        assert_eq!((available, required), (0x0100, 0x0200));
        assert_eq!(ShipFlags3::HIDE_HIGHER, 0x4000);
        assert!(wares::hidden(available, HIDE_BITS, true, false));
        assert!(!wares::hidden(available, HIDE_BITS, false, true));
        assert!(wares::hidden(required, HIDE_BITS, false, true));
        assert!(!wares::hidden(required, HIDE_BITS, true, false));
    }

    #[test]
    fn hide_higher_takes_higher_numbered_ships_of_its_weight_off_sale() {
        let weighted = |id, disp_weight, flags3| ShipRecord {
            flags3,
            disp_weight,
            ..cheap(id)
        };
        let ships = [
            weighted(129, 5, 0),
            weighted(130, 5, ShipFlags3::HIDE_HIGHER),
            weighted(131, 5, 0),
            weighted(132, 6, 0),
            weighted(133, 5, ShipFlags3::HIDE_HIGHER),
        ];
        assert_eq!(listed(&open(&ships, &pilot())), [132, 129, 130]);
        // One that cannot be bought hides nothing.
        let unmet = [
            ShipRecord {
                require: 0x2,
                ..weighted(130, 5, ShipFlags3::HIDE_HIGHER)
            },
            weighted(131, 5, 0),
        ];
        assert_eq!(listed(&open(&unmet, &pilot())), [130, 131]);
        let off_tech = [
            ShipRecord {
                tech_level: 9,
                ..weighted(130, 5, ShipFlags3::HIDE_HIGHER)
            },
            weighted(131, 5, 0),
        ];
        assert_eq!(listed(&open(&off_tech, &pilot())), [131]);
        let oütf_bit = [
            weighted(130, 5, OutfitFlags::HIDE_HIGHER),
            weighted(131, 5, 0),
        ];
        assert_eq!(listed(&open(&oütf_bit, &pilot())), [130, 131]);
    }

    // Price and trade-in.

    #[test]
    fn the_price_is_the_cost_never_below_none() {
        assert_eq!(ship_price(&cheap(129)), 1000);
        let free = ShipRecord {
            cost: -5,
            ..cheap(129)
        };
        assert_eq!(ship_price(&free), 0);
        let dear = ShipRecord {
            cost: i32::MAX,
            ..cheap(129)
        };
        assert_eq!(ship_price(&dear), i64::from(i32::MAX));
    }

    #[test]
    fn the_hull_trades_in_for_a_quarter_of_its_cost() {
        assert_eq!(TRADE_IN_PERCENT, 25);
        assert_eq!(trade_in(10_000, 40, &BTreeMap::new(), &[]), 2500);
        assert_eq!(
            trade_in(10_003, 40, &BTreeMap::new(), &[]),
            2500,
            "truncated"
        );
        assert_eq!(trade_in(-400, 40, &BTreeMap::new(), &[]), 0);
        assert_eq!(
            trade_in(i32::MAX, 40, &BTreeMap::new(), &[]),
            i64::from(i32::MAX) / 4
        );
    }

    /// A 2000-credit tank, a persistent 1000-credit licence, an unsellable
    /// 3000-credit map, and a 10-credit-a-ton armour plate priced by the
    /// ship's mass.
    fn upgrades() -> [OutfitRecord; 4] {
        [
            OutfitRecord {
                cost: 2000,
                ..outfit(128, &[(MORE_FUEL, 100)])
            },
            OutfitRecord {
                cost: 1000,
                flags: OutfitFlags::PERSISTENT,
                ..outfit(129, &[])
            },
            OutfitRecord {
                cost: 3000,
                flags: OutfitFlags::CANNOT_SELL,
                ..outfit(130, &[])
            },
            OutfitRecord {
                cost: 10,
                flags: OutfitFlags::PRICE_BY_MASS,
                ..outfit(131, &[])
            },
        ]
    }

    #[test]
    fn the_outfits_trade_in_at_their_resale_but_persistent_or_unsellable_ones_add_nothing() {
        let records = upgrades();
        let hull = 2500;
        assert_eq!(
            trade_in(10_000, 40, &map(&[(128, 3)]), &records),
            hull + 3000
        );
        assert_eq!(trade_in(10_000, 40, &map(&[(129, 1)]), &records), hull);
        assert_eq!(trade_in(10_000, 40, &map(&[(130, 2)]), &records), hull);
        assert_eq!(
            trade_in(10_000, 40, &map(&[(131, 1)]), &records),
            hull + 200,
            "priced on the old ship"
        );
        assert_eq!(trade_in(10_000, 40, &map(&[(999, 5)]), &records), hull);
        assert_eq!(
            trade_in(10_000, 40, &map(&[(128, 1), (129, 1), (131, 2)]), &records),
            hull + 1000 + 400
        );
        let dear = [OutfitRecord {
            cost: i32::MAX,
            flags: OutfitFlags::PRICE_BY_MASS,
            ..outfit(128, &[])
        }];
        let each = i64::from(i32::MAX) * i64::from(i16::MAX) / 2;
        assert_eq!(
            trade_in(10_000, i16::MAX, &map(&[(128, u16::MAX)]), &dear),
            hull + each * i64::from(u16::MAX),
            "no overflow"
        );
    }

    #[test]
    fn the_shipyard_trades_in_the_ship_flown_with_what_it_carries_including_defaults() {
        let ships = [ShipRecord {
            cost: 10_000,
            defaults: vec![(OutfitId(128), 1)],
            ..ship(128, FAST)
        }];
        let pilot = owning(&[(128, 1), (129, 1), (131, 1)]);
        let shipyard = open_with(&ships, &upgrades(), &pilot);
        assert_eq!(shipyard.trade_in, 2500 + 1000 + 200);
        let unknown = open_with(&[cheap(130)], &upgrades(), &pilot);
        assert_eq!(unknown.trade_in, 1200, "a hull with no record has no cost");
    }

    #[test]
    fn a_buy_is_refused_when_cash_and_trade_in_do_not_cover_the_price() {
        let ships = [
            ShipRecord {
                cost: 4000,
                ..ship(128, FAST)
            },
            ShipRecord {
                cost: 11_000,
                ..cheap(129)
            },
        ];
        let mut exact = pilot();
        exact.cash = 10_000;
        let shipyard = open(&ships, &exact);
        assert_eq!(shipyard.trade_in, 1000);
        assert_eq!(row(&shipyard, 129).buy, Ok(()), "exactly enough");
        exact.cash = 9999;
        assert_eq!(
            open(&ships, &exact).check(ShipId(129)),
            Err(ShipRefusal::CannotAfford)
        );
        assert_eq!(
            open(&ships, &exact).check(ShipId(999)),
            Err(ShipRefusal::NotListed)
        );
        let unmet = [ShipRecord {
            require: 0x2,
            ..cheap(129)
        }];
        assert_eq!(
            open(&unmet, &pilot()).check(ShipId(129)),
            Err(ShipRefusal::NotForSale)
        );
        let mut broke = pilot();
        broke.cash = 0;
        assert_eq!(
            open(&unmet, &broke).check(ShipId(129)),
            Err(ShipRefusal::NotForSale),
            "not for sale before not affordable"
        );
    }

    #[test]
    fn a_trade_in_worth_more_than_the_ship_pays_the_difference() {
        let ships = [
            ShipRecord {
                cost: 40_000,
                ..ship(128, FAST)
            },
            cheap(129),
        ];
        let shipyard = open(&ships, &pilot());
        let row = row(&shipyard, 129);
        assert_eq!(shipyard.final_price(row), 1000 - 10_000);
        assert_eq!(row.buy, Ok(()));
    }

    // Buying a ship.

    /// The new ship: 15 tons of cargo space, 12 free, of mass 25, faster
    /// than FAST and with more shield.
    const HEAVY: ShipFields = ShipFields {
        speed: 900,
        shield: 80,
        holds: 15,
        free_mass: 12,
        mass: 25,
        ..FAST
    };

    fn heavy() -> ShipRecord {
        ShipRecord {
            cost: 17_500,
            ..ship(129, HEAVY)
        }
    }

    /// A persistent outfit of `mass` tons, up to `max`, for 1000 credits.
    fn persistent(id: i16, mass: i16, max: i16) -> OutfitRecord {
        OutfitRecord {
            mass,
            max,
            flags: OutfitFlags::PERSISTENT,
            ..outfit(id, &[])
        }
    }

    const QUOTE: Quote = Quote {
        price: 17_500,
        trade_in: 2500,
    };

    fn buy(pilot: &mut Pilot, new: &ShipRecord, records: &[OutfitRecord]) -> ShipPurchase {
        purchase(pilot, FAST.mass, new, QUOTE, records)
    }

    #[test]
    fn buying_replaces_the_ship_and_moves_the_cash_by_the_trade_in_less_the_price() {
        let mut pilot = pilot();
        pilot.cash = 20_000;
        let bought = buy(&mut pilot, &heavy(), &[]);
        assert_eq!(pilot.ship(), ShipId(129));
        assert_eq!(pilot.cash(), 20_000 - 17_500 + 2500);
        assert_eq!(
            bought,
            ShipPurchase {
                price: 17_500,
                trade_in: 2500,
                sold_back: BTreeMap::new(),
                refund: 0,
                left_behind: BTreeMap::new(),
            }
        );
        let mut paid = self::pilot();
        purchase(
            &mut paid,
            40,
            &heavy(),
            Quote {
                price: 1000,
                trade_in: 2500,
            },
            &[],
        );
        assert_eq!(paid.cash(), 11_500, "paid the difference");
    }

    #[test]
    fn outfits_that_are_not_persistent_go_with_the_old_ship() {
        let records = upgrades();
        let mut pilot = owning(&[(128, 2), (130, 1), (131, 1), (999, 1)]);
        let bought = buy(&mut pilot, &heavy(), &records);
        assert_eq!(pilot.outfits().count(), 0);
        assert_eq!(bought.refund, 0, "in the trade-in, not sold back");
        assert_eq!(pilot.cash(), 10_000 - 17_500 + 2500);
    }

    #[test]
    fn a_persistent_outfit_that_fits_carries_over() {
        let records = [persistent(140, 0, 1), persistent(141, 4, 3)];
        let mut pilot = owning(&[(140, 1), (141, 3)]);
        let bought = buy(&mut pilot, &heavy(), &records);
        assert_eq!(
            pilot.outfits().collect::<Vec<_>>(),
            [(OutfitId(140), 1), (OutfitId(141), 3)]
        );
        assert_eq!(bought.sold_back, BTreeMap::new());
        assert_eq!(bought.refund, 0);
    }

    #[test]
    fn a_persistent_outfit_beyond_the_new_free_mass_is_sold_back_unit_by_unit() {
        // 12 tons free: two of five 5-ton units fit, then a 1-ton one by a
        // higher ID still does.
        let records = [
            OutfitRecord {
                cost: 3001,
                ..persistent(140, 5, 9)
            },
            persistent(141, 1, 1),
        ];
        let mut pilot = owning(&[(140, 5), (141, 1)]);
        let bought = buy(&mut pilot, &heavy(), &records);
        assert_eq!(pilot.owned(OutfitId(140)), 2);
        assert_eq!(pilot.owned(OutfitId(141)), 1);
        assert_eq!(bought.sold_back, map(&[(140, 3)]));
        assert_eq!(bought.refund, 3 * 1500, "half each, truncated");
        assert_eq!(pilot.cash(), 10_000 - 17_500 + 2500 + 4500);
    }

    #[test]
    fn the_new_default_items_count_as_owned_when_carrying_over() {
        // FreeMass 5 and a 3-ton default item: crediting its mass as room
        // would keep the 6-ton outfit (5 + 3 - 6 = 2); counting it as owned
        // does not (5 + 3 - 3 - 6 = -1).
        let small = ShipRecord {
            defaults: vec![(OutfitId(150), 1)],
            ..ship(
                129,
                ShipFields {
                    free_mass: 5,
                    ..HEAVY
                },
            )
        };
        let records = [
            OutfitRecord {
                mass: 3,
                ..outfit(150, &[(MORE_SHIELD, 50)])
            },
            OutfitRecord {
                cost: 4000,
                ..persistent(140, 6, 1)
            },
        ];
        let mut pilot = owning(&[(140, 1)]);
        let bought = buy(&mut pilot, &small, &records);
        assert_eq!(pilot.outfits().collect::<Vec<_>>(), [(OutfitId(150), 1)]);
        assert_eq!(bought.sold_back, map(&[(140, 1)]));
        assert_eq!(bought.refund, 2000);
        assert!(free_mass(small.fields, &map(&[(150, 1)]), &pilot.outfits, &records) >= 0);
        // With a 5-ton one instead, it fits exactly.
        let lighter = [records[0].clone(), persistent(140, 5, 1)];
        let mut pilot = owning(&[(140, 1)]);
        buy(&mut pilot, &small, &lighter);
        assert_eq!(pilot.owned(OutfitId(140)), 1);
        assert_eq!(
            free_mass(small.fields, &map(&[(150, 1)]), &pilot.outfits, &lighter),
            0
        );
    }

    #[test]
    fn a_persistent_outfit_sold_back_is_priced_on_the_old_ships_mass() {
        // 100 credits a ton of ship and 20 tons: it fits neither HEAVY's 12
        // tons free nor anywhere else, and sells back as fitted to the old
        // ship (mass 40), not the new (25).
        let by_mass = OutfitRecord {
            cost: 100,
            flags: OutfitFlags::PERSISTENT | OutfitFlags::PRICE_BY_MASS,
            ..persistent(140, 20, 1)
        };
        assert_ne!(FAST.mass, HEAVY.mass);
        let mut pilot = owning(&[(140, 1)]);
        let bought = buy(&mut pilot, &heavy(), std::slice::from_ref(&by_mass));
        assert_eq!(bought.sold_back, map(&[(140, 1)]));
        assert_eq!(bought.refund, resale(100 * i64::from(FAST.mass)));
        assert_eq!(bought.refund, 2000);
        assert_eq!(pilot.cash(), 10_000 - 17_500 + 2500 + 2000);
    }

    #[test]
    fn a_persistent_outfit_beyond_its_max_is_capped_and_the_rest_sold_back() {
        let records = [
            persistent(140, 0, 2),
            persistent(141, 0, 0),
            persistent(142, 0, -1),
        ];
        let mut pilot = owning(&[(140, 5), (141, 1), (142, 2)]);
        let bought = buy(&mut pilot, &heavy(), &records);
        assert_eq!(pilot.outfits().collect::<Vec<_>>(), [(OutfitId(140), 2)]);
        assert_eq!(bought.sold_back, map(&[(140, 3), (141, 1), (142, 2)]));
        assert_eq!(bought.refund, 6 * 500);
    }

    #[test]
    fn a_persistent_mass_expansion_onto_negative_holds_is_sold_back() {
        let records = [persistent(140, -5, 1), persistent(141, 0, 1)];
        let negative = ShipRecord {
            fields: ShipFields { holds: -1, ..HEAVY },
            ..heavy()
        };
        let mut pilot = owning(&[(140, 1), (141, 1)]);
        let bought = buy(&mut pilot, &negative, &records);
        assert_eq!(pilot.outfits().collect::<Vec<_>>(), [(OutfitId(141), 1)]);
        assert_eq!(bought.sold_back, map(&[(140, 1)]));
        let mut pilot = owning(&[(140, 1)]);
        let empty_holds = ShipRecord {
            fields: ShipFields { holds: 0, ..HEAVY },
            ..heavy()
        };
        buy(&mut pilot, &empty_holds, &records);
        assert_eq!(
            pilot.owned(OutfitId(140)),
            1,
            "only negative Holds forbids it"
        );
    }

    #[test]
    fn the_new_default_items_are_added_on_top_of_those_carried_over() {
        let with_defaults = ShipRecord {
            defaults: vec![
                (OutfitId(140), 2),
                (OutfitId(150), 1),
                (OutfitId(150), 1),
                (OutfitId(151), 0),
            ],
            ..heavy()
        };
        let records = [persistent(140, 0, 1), outfit(150, &[])];
        let mut pilot = owning(&[(140, 1)]);
        buy(&mut pilot, &with_defaults, &records);
        assert_eq!(
            pilot.outfits().collect::<Vec<_>>(),
            [(OutfitId(140), 3), (OutfitId(150), 2)],
            "beyond the Max, as the original fits them"
        );
        let mut full = owning(&[(140, 1)]);
        let most = ShipRecord {
            defaults: vec![(OutfitId(140), u16::MAX)],
            ..heavy()
        };
        buy(&mut full, &most, &records);
        assert_eq!(full.owned(OutfitId(140)), u16::MAX, "saturating");
    }

    #[test]
    fn cargo_beyond_the_new_hold_is_cut_down_in_good_order_and_reported() {
        let records = [OutfitRecord {
            mass: 0,
            ..outfit(150, &[(MORE_CARGO, 3)])
        }];
        let with_pod = ShipRecord {
            defaults: vec![(OutfitId(150), 1)],
            ..heavy()
        };
        let opals = Good::Junk(JunkId(146));
        let mut pilot = pilot();
        pilot.cargo =
            BTreeMap::from([(Good::Commodity(0), 6), (Good::Commodity(3), 9), (opals, 4)]);
        let bought = buy(&mut pilot, &with_pod, &records);
        assert_eq!(
            pilot.cargo().collect::<Vec<_>>(),
            [(Good::Commodity(0), 6), (Good::Commodity(3), 9), (opals, 3)],
            "18 tons: 15 and a 3-ton pod"
        );
        assert_eq!(bought.left_behind, BTreeMap::from([(opals, 1)]));
        let mut pilot = self::pilot();
        pilot.cargo = BTreeMap::from([(Good::Commodity(0), 20), (opals, 2)]);
        let bought = buy(&mut pilot, &heavy(), &[]);
        assert_eq!(
            pilot.cargo().collect::<Vec<_>>(),
            [(Good::Commodity(0), 15)],
            "none of a good left is none held"
        );
        assert_eq!(
            bought.left_behind,
            BTreeMap::from([(Good::Commodity(0), 5), (opals, 2)])
        );
        let mut fits = self::pilot();
        fits.cargo = BTreeMap::from([(Good::Commodity(0), 15)]);
        let bought = buy(&mut fits, &heavy(), &[]);
        assert_eq!(fits.held(Good::Commodity(0)), 15, "exactly full");
        assert_eq!(bought.left_behind, BTreeMap::new());
    }

    #[test]
    fn the_new_ship_comes_with_its_reserves_full() {
        let with_tank = ShipRecord {
            defaults: vec![(OutfitId(150), 1)],
            ..heavy()
        };
        let records = [outfit(150, &[(MORE_FUEL, 100)])];
        let mut pilot = pilot();
        pilot.reserves.fuel.now = 12.0;
        pilot.reserves.shield.now = 1.0;
        buy(&mut pilot, &with_tank, &records);
        assert_eq!(
            pilot.reserves(),
            ShipStats::new(HEAVY, &outfit_mods(&map(&[(150, 1)]), &records)).full()
        );
        assert_eq!(pilot.reserves(), Reserves::full(80.0, 45.0, 400.0));
    }

    #[test]
    fn buying_a_ship_leaves_everything_else_about_the_pilot_as_it_was() {
        let mut pilot = pilot();
        pilot.stellar = Some(StellarId(128));
        pilot.course = vec![SystemId(131), SystemId(132)];
        pilot.explore(SystemId(131));
        pilot.set_legal_record(GovtId(128), 40);
        pilot.events = BTreeMap::from([(DisasterId(128), 3)]);
        pilot.name = "Ada".to_owned();
        let before = pilot.clone();
        buy(&mut pilot, &heavy(), &[]);
        assert_eq!(
            (
                &pilot.name,
                pilot.system,
                pilot.stellar,
                pilot.date,
                &pilot.course,
                &pilot.explored,
                &pilot.legal,
                &pilot.events,
                pilot.default_outfits_pending,
            ),
            (
                &before.name,
                before.system,
                before.stellar,
                before.date,
                &before.course,
                &before.explored,
                &before.legal,
                &before.events,
                before.default_outfits_pending,
            )
        );
    }
}
