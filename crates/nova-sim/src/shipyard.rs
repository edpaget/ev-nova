//! The shipyard: which ship classes (`shïp`) a stellar lists and sells, at
//! what price, what the player's ship trades in for, and buying a new one.
//!
//! # What a stellar sells
//!
//! A ship is *for sale* at a stellar with a shipyard when its `TechLevel`
//! is allowed there, as an outfit's is ([`wares::tech_allows`]), and its
//! `BuyRandom` roll holds today. The Bible's `shïp` gives `BuyRandom` as
//! "the percent chance that a ship of this type will be available for
//! purchase on a given day. A `BuyRandom` of 0 means this ship will never
//! be made available for purchase" (stock data has 200 such NPC-only
//! ships).
//!
//! The original (`EV Nova.app`) draws one buy roll per ship class,
//! `_Rand(100) + 1`, for the whole galaxy each day, for a new pilot and on
//! load, and never saves it (`_IncrementGameTime` @0xb7c4-0xb833, as
//! [`outfitter`](crate::outfitter) says); `_SetupPortAvailableShipTypes`
//! sells a class that passes its tech level when its `BuyRandom` is not 0
//! and, compared unsigned, at least the day's roll (@0xbc88-0xbc9d). The
//! class flown is rolled like any other. The loader clamps `BuyRandom`
//! with an unsigned compare (`_LoadObjectData` @0x7a340-0x7a34a), so above
//! 100 counts as 100 and so does any negative value: by the engine, 0 is
//! never for sale, 1-99 a % chance, and 100 or more or below 0 always.
//! The Bible says only that 0 means never; by its reading, 0 or below is
//! never. The rulebook's [`RuleKey::BuyRandom`](crate::RuleKey::BuyRandom)
//! chooses (`buy_roll`). Buying a ship draws its class's roll again
//! (`_DoShipyardDialog` @0x5f0d5-0x5f0f8).
//!
//! Here each class's roll is drawn on the caller's
//! [`Chance`] the first time the list is built after a landing, by
//! ascending ID and only for a class that tech allows whose reading is a
//! chance of 1-99, and kept until the next landing or until the class is
//! bought or declined. The original builds the list once each time the
//! shipyard opens (`_SetupPortAvailableShipTypes`, called only @0x5e68f),
//! so a roll drawn again after a purchase or a decline is drawn the next
//! time the list is built, and until then the class can be bought as
//! listed ([`Session::shipyard`](crate::Session::shipyard)). A roll is
//! never saved, as the original rolls again on load. A
//! class off today is not listed and takes no higher one off sale.
//!
//! A ship for sale can be *bought* when its `Require` bits are met, by the
//! `Contribute` of the ship flown and of the outfits the player owns (a
//! `shïp` has no `RequireGovt`, so `Require` applies everywhere), and its
//! `Availability` holds. `Availability` goes through
//! [`control_bits_allow`], which always holds until control bits exist, so
//! for now every ship gated by one is available: the Vell-os ships (stock
//! 381-383, tech level 1, `Cost` 0) show, free, at every shipyard, as stock
//! `oütf` 342 does at every outfitter.
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
//! `Max`, as the original fits them regardless, and then its stock weapons
//! and their `AmmoLoad` are topped up (see Stock weapons).
//!
//! What cargo is kept follows the rulebook's
//! [`RuleKey::PurchaseCargo`](crate::RuleKey::PurchaseCargo) (`purchase_cargo`);
//! the rest is left behind, unpaid. By the engine, `_DoShipyardDialog`
//! calls `_DestroyPartialFleetCargo(0)` (@0x5ef9f, @0xcd32-0xcff6) once
//! the new class, launched fighters dropped and the outfits above are in
//! place. A is the new ship's cargo space (its `Holds` and the `ModType` 2
//! outfits now owned, @0xcd4c-0xcdcd) and B is A plus the `Holds` of the
//! escorts the fleet's holds count ([`escort_tons`], @0xce08-0xcf0d), with
//! no 32000 cap; f = min(1, A/B) in doubles (@0xcf27-0xcf37), and none
//! when A is none (the engine's 0/0 NaN acts so). Each commodity becomes
//! trunc(held x f) (for the player's slot the second write, @0xcf6c,
//! wins), and each `jünk` loses trunc(held x f), so with no such escorts
//! every `jünk` goes (@0xcf94-0xcfe4). Then `_ResetPlayerPrecalcedValues`
//! trims (@0xc7d8-0xc82c): when the fleet's holds ([`fleet_holds`]) are
//! below everything held, each commodity becomes trunc(held x holds /
//! held in all), worked in single floats; `jünk` is not cut. Mission
//! cargo, which the engine leaves alone, does not exist yet. By the other
//! reading, the phase's first wording (the Bible and the forums are
//! silent), the cargo that fits in the new ship's own cargo space is
//! kept, goods in [`Good`] order (commodities by number, then `jünk` by
//! ID) until the hold is full. The new
//! ship comes with its shield, armour and fuel full, as a new hull: the
//! Bible is silent here too. Everything else about the pilot (where it is,
//! the date, the course, its legal records and the events under way)
//! stays as it was.
//!
//! # Naming the ship
//!
//! The original asks for the new ship's name *before* it buys it: Buy
//! Ship checks that the class can be bought, then opens the "Text Input"
//! prompt (`_EVTextInputDialog`, `_DoShipyardDialog` @0x5eb6c), and makes
//! the purchase only when the prompt is confirmed (@0x5ebc2). The prompt
//! reads [`NAME_PROMPT`] (`STR#` 2002 #120), a space, the class's
//! `LongName` and ": " (@0x5ea1c-0x5eac6), so the Long Name is part of the
//! question, and no message follows the purchase. Its default is the
//! class's name, a space and three digits, each `_Rand(9) + 1`, drawn left
//! to right (@0x5eacb-0x5eb4c): "Shuttle 482" ([`ShipNaming`], [`Session::ship_naming`](crate::Session::ship_naming)). The name
//! confirmed is kept on the pilot through `_CullNameString`, which drops a
//! leading "the " and nothing else ([`cull_name`], @0x5ed5a-0x5ed74).
//! Cancelling buys nothing, but still draws the class's roll again
//! (@0x5ebc2 to @0x5f0d5-0x5f0f8), when the list is next built. The prompt refuses a name longer than
//! [`SHIP_NAME_CHARS`] (@0x554a9-0x554e7), and accepts an empty one.
//!
//! # Stock weapons
//!
//! A ship's stock weapons (`WeapType`/`WeapCount`) and their `AmmoLoad`
//! are outfits it owns, each held by the `oütf` of lowest ID naming it
//! ([`StockFit`]), as the original holds them (`_ShipStatsToSystemInfo`
//! @0xcaee-0xcce7). Buying a ship tops them up after the default items
//! (`_DoShipyardDialog` @0x5eebc-0x5ef92): each weapon to at least its
//! `WeapCount`, and its ammunition to at least its `AmmoLoad`, counting
//! what the persistent outfits carried over and the default items already
//! hold, never adding to it (`fit_stock`). Like the default items, they
//! are fitted regardless: `FreeMass` leaves out their mass, which the
//! original's loader adds to it (`_LoadObjectData` @0x7aaf7, @0x7ac61,
//! @0x7ad8d), so they take no room, and a persistent outfit carries over
//! only while it fits with them fitted. Owned, they trade in as any other
//! outfit does: `_PlayerShipTradeInPrice` @0xb079 counts the outfits owned
//! and nothing else.
//!
//! The evidence for the trade-in: the Bible's `Cost` says "the cost of the
//! new ship minus 25% of the original cost of your current ship and
//! upgrades"; an Ambrosia forum answer (Forum26 #004247, 2002) gives "25%
//! their original cost, plus 50% the cost of all upgrades"; and the
//! original engine's own debug log, reproduced from stock data, reads
//! "Striker (262) has a trade-in value of 1163000". That is the loader's
//! estimate (`_LoadObjectData` @0x7a9ef-0x7ae07, printed @0x7b3a0): 25 % of
//! its 1,000,000, plus 50 % of its stock weapons and default items'
//! 1,825,000, plus 500 for one of its 30 Wraithii, as the check counts the
//! ammunition by `WeapCount` rather than `AmmoLoad` (@0x7aca2). The trade-in
//! the player sees for a Striker owning its 30 rounds is 1,177,500, and
//! exactly 1,163,000 once 29 have been fired. The forums add that the
//! trade-in leaves persistent outfits out, and that a trade-in worth more
//! than the new ship pays the difference.
//!
//! Not modelled yet: the gun and turret limits (`MaxGun`, `MaxTur`);
//! `OnPurchase` and `OnRetire`; `MovieFile`; and escorts' `UpgradeTo`.
//! Nor does the shipyard price a ship through the original's tech-level
//! flux, as hiring in the bar does with its own price rule
//! ([`hire`](crate::hire)).

use std::collections::BTreeMap;

use crate::catalog::{LandingSite, OutfitId, OutfitRecord, ShipId, ShipRecord};
use crate::chance::Chance;
use crate::combat::armament::{StockFit, fit_stock, fitted};
use crate::handling::ShipFields;
use crate::landing::StellarFlags;
use crate::market::{EscortHolds, Good, control_bits_allow, escort_tons, fleet_holds};
use crate::outfitter::{OutfitFlags, free_mass, outfit_mods, resale, unit_mass, unit_price};
use crate::pilot::{Pilot, merged, tally};
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::wares::{self, DayRolls, HideBits, HideHigher, Roll};

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

/// How a ship class of `buy_random` rolls for sale each day, read as
/// `source` says ([`RuleKey::BuyRandom`](crate::RuleKey::BuyRandom)): by
/// the engine, 0 never and below 0 or 100 or more always
/// (`_LoadObjectData` clamps it unsigned, @0x7a340-0x7a34a, and
/// `_SetupPortAvailableShipTypes` sells only when it is not 0,
/// @0xbc88-0xbc9d); by the Bible, 0 or below never and 100 or more
/// always. Any other is a `BuyRandom` % chance.
pub(crate) fn buy_roll(buy_random: i16, source: RuleSource) -> Roll {
    match source {
        RuleSource::Engine if buy_random < 0 => Roll::Always,
        _ => Roll::of(buy_random),
    }
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
    /// How `BuyRandom` reads ([`buy_roll`]).
    pub(crate) buy_random: RuleSource,
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

    /// The shipyard, for `pilot`, each class's roll for the day kept in
    /// `rolls` and any not drawn yet drawn on `chance` (see the module
    /// docs); `None` when the stellar has none.
    pub(crate) fn shipyard(
        &self,
        pilot: &Pilot,
        rolls: &mut DayRolls<ShipId>,
        chance: &mut dyn Chance,
    ) -> Option<Shipyard> {
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
            let available = control_bits_allow(&ship.availability);
            let for_sale = wares::tech_allows(ship.tech_level, self.site)
                && rolls.today(ship.id, buy_roll(ship.buy_random, self.buy_random), chance)
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

/// The name prompt's opening words, `STR#` 2002 #120.
pub const NAME_PROMPT: &str = "Please name your new";

/// The longest name the original's name prompt takes, in characters
/// (`_EVTextInputDialog`'s most, 0x40 @0x5eb5d, checked @0x554b3).
pub const SHIP_NAME_CHARS: usize = 64;

/// The prompt for naming a ship about to be bought.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipNaming {
    /// The ship class to buy.
    pub ship: ShipId,
    /// The prompt's text: [`NAME_PROMPT`], a space, the class's
    /// `LongName`, then ": ".
    pub prompt: String,
    /// The name the prompt opens with: the class's name, a space and three
    /// digits from 1 to 9.
    pub default: String,
}

/// The prompt for naming a new ship of class `record`, its default's
/// digits drawn on `chance`, as the module says.
pub(crate) fn naming(record: &ShipRecord, chance: &mut dyn Chance) -> ShipNaming {
    let mut default = format!("{} ", record.name);
    for _ in 0..3 {
        default.push_str(&(chance.roll(9) + 1).to_string());
    }
    ShipNaming {
        ship: record.id,
        prompt: format!("{NAME_PROMPT} {}: ", record.long_name),
        default,
    }
}

/// `name` as the original keeps a ship's name (`_CullNameString`
/// @0xdbe2-0xdd2c): a leading "the " (the letters t, h and e in any case,
/// then a space) is dropped, and nothing else changes (@0xdc54-0xdc9c).
#[must_use]
pub fn cull_name(name: &str) -> String {
    match name.get(..4) {
        Some(head) if head.eq_ignore_ascii_case("the ") => name[4..].to_owned(),
        _ => name.to_owned(),
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

/// What a purchase's cargo step reads of the fleet.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fleet<'a> {
    /// The escorts that stay with the player, launched fighters already
    /// gone.
    pub(crate) escorts: &'a [EscortHolds],
    /// What cargo the purchase keeps
    /// ([`RuleKey::PurchaseCargo`](crate::RuleKey::PurchaseCargo)).
    pub(crate) cargo: RuleSource,
}

/// Cuts `cargo` down to what a purchase keeps on a new ship of `capacity`
/// tons of its own with `escorts`, read as `source` says, and gives what
/// it left behind, each good with its tons (see the module's Buying a
/// ship).
pub(crate) fn keep_cargo(
    cargo: &mut BTreeMap<Good, u32>,
    capacity: u32,
    escorts: &[EscortHolds],
    source: RuleSource,
) -> BTreeMap<Good, u32> {
    let before = cargo.clone();
    match source {
        RuleSource::Engine => {
            let share = new_ships_share(capacity, escort_tons(escorts.iter().copied()));
            for (good, tons) in cargo.iter_mut() {
                let part = (f64::from(*tons) * share) as i64;
                // The engine sets a `jünk` to 0 when its part is above
                // what is held (@0xcf94-0xcfe4); below none, the clamp to none
                // after does the same.
                let kept = match good {
                    Good::Commodity(_) => part,
                    Good::Junk(_) => i64::from(*tons) - part,
                };
                *tons = u32::try_from(kept.max(0)).unwrap_or(u32::MAX);
            }
            trim_to(cargo, fleet_holds(capacity, escorts.iter().copied()));
        }
        RuleSource::Bible => {
            let mut room = capacity;
            for tons in cargo.values_mut() {
                let kept = (*tons).min(room);
                room -= kept;
                *tons = kept;
            }
        }
    }
    cargo.retain(|_, tons| *tons > 0);
    before
        .into_iter()
        .filter_map(|(good, was)| {
            let now = cargo.get(&good).copied().unwrap_or(0);
            (was > now).then(|| (good, was - now))
        })
        .collect()
}

/// The new ship's share of the fleet's cargo space when it buys a ship
/// (`_DestroyPartialFleetCargo` @0xcf27-0xcf37): its own `capacity` over
/// that plus the `escorts` tons, at most 1, in doubles. With no space of
/// its own the share is none: the engine's 0/0 is NaN, which converts to
/// a value that keeps nothing of a commodity and takes nothing of a
/// `jünk`, as none does. The cap is `minsd` with 1 first (@0xcf33), so
/// escorts that bring the fleet to no space give A/0 = +inf and so 1, and
/// below none give a negative share, kept as it is.
fn new_ships_share(capacity: u32, escorts: i64) -> f64 {
    if capacity == 0 {
        return 0.0;
    }
    let own = f64::from(capacity);
    (own / (own + escorts as f64)).min(1.0)
}

/// Cuts each commodity in `cargo` by `holds` over everything held when
/// that is more (`_ResetPlayerPrecalcedValues` @0xc7d8-0xc82c), in single
/// floats; a `jünk` is not cut.
fn trim_to(cargo: &mut BTreeMap<Good, u32>, holds: u32) {
    let total: u64 = cargo.values().map(|&tons| u64::from(tons)).sum();
    if u64::from(holds) >= total {
        return;
    }
    let ratio = holds as f32 / total as f32;
    for (good, tons) in cargo.iter_mut() {
        if let Good::Commodity(_) = good {
            *tons = (*tons as f32 * ratio) as u32;
        }
    }
}

/// Buys `new`, whose stock weapons and ammunition are `fits`
/// ([`Arsenal::stock_fits`](crate::combat::armament::Arsenal::stock_fits)),
/// for `pilot`, at `quote`, from a ship of `old_mass`, naming it `name`
/// ([`cull_name`]), with `fleet`, as the module says, and gives what it
/// did.
#[allow(clippy::too_many_arguments)]
pub(crate) fn purchase(
    pilot: &mut Pilot,
    old_mass: i16,
    new: &ShipRecord,
    name: &str,
    fits: &[StockFit],
    quote: Quote,
    records: &[OutfitRecord],
    fleet: Fleet,
) -> ShipPurchase {
    let defaults = tally(new.defaults.iter().copied());
    let standard = merged(&defaults, &fitted(fits));
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
                && free_mass(new.fields, &standard, &merged(&trial, &standard), records) >= 0;
            if fits {
                carried = trial;
            } else {
                *sold_back.entry(record.id).or_default() += 1;
                refund = refund.saturating_add(resale(unit_price(record, old_mass)));
            }
        }
    }
    pilot.outfits = merged(&carried, &defaults);
    fit_stock(&mut pilot.outfits, fits, records);
    pilot.ship = new.id;
    pilot.ship_name = Some(cull_name(name));
    pilot.cash = pilot
        .cash
        .saturating_sub(quote.price)
        .saturating_add(quote.trade_in)
        .saturating_add(refund);
    let stats = ShipStats::new(new.fields, &outfit_mods(&pilot.outfits, records));
    let left_behind = keep_cargo(&mut pilot.cargo, stats.capacity, fleet.escorts, fleet.cargo);
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
    use crate::catalog::{DisasterId, GovtId, JunkId, StellarId, SystemId, WeaponId};
    use crate::chance::NeverFires;
    use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use crate::market::MORE_CARGO;
    use crate::reserves::Reserves;
    use crate::stats::{MORE_FUEL, MORE_SHIELD};
    use crate::testkit::{FAST, Scripted, catalog, outfit, planet, ship};

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

    /// The shipyard of `ships` at `site`, reading `BuyRandom` by the
    /// engine.
    fn yard<'a>(
        ships: &'a [ShipRecord],
        outfits: &'a [OutfitRecord],
        site: &'a LandingSite,
    ) -> Yard<'a> {
        Yard {
            ships,
            outfits,
            fields: FAST,
            site,
            buy_random: RuleSource::Engine,
        }
    }

    /// The shipyard at `site` with no roll drawn yet, and none that fires.
    fn open_at(
        ships: &[ShipRecord],
        outfits: &[OutfitRecord],
        site: &LandingSite,
        pilot: &Pilot,
    ) -> Option<Shipyard> {
        yard(ships, outfits, site).shipyard(pilot, &mut DayRolls::default(), &mut NeverFires)
    }

    /// The shipyard of `ships` at [`port`] on `rolls`, drawing on `chance`.
    fn rolled(
        ships: &[ShipRecord],
        pilot: &Pilot,
        rolls: &mut DayRolls<ShipId>,
        chance: &mut dyn Chance,
    ) -> Shipyard {
        yard(ships, &[], &port())
            .shipyard(pilot, rolls, chance)
            .expect("a shipyard")
    }

    /// Ship `id` at 1000 credits, of `buy_random`.
    fn buying(id: i16, buy_random: i16) -> ShipRecord {
        ShipRecord {
            buy_random,
            ..cheap(id)
        }
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
    fn by_the_engine_a_ships_negative_buy_random_is_always() {
        let engine = |buy_random| buy_roll(buy_random, RuleSource::Engine);
        assert_eq!(engine(i16::MIN), Roll::Always);
        assert_eq!(engine(-1), Roll::Always);
        assert_eq!(engine(0), Roll::Never);
        assert_eq!(engine(1), Roll::Chance(1));
        assert_eq!(engine(35), Roll::Chance(35));
        assert_eq!(engine(99), Roll::Chance(99));
        assert_eq!(engine(100), Roll::Always);
        assert_eq!(engine(200), Roll::Always);
    }

    #[test]
    fn by_the_bible_a_ships_buy_random_of_0_or_below_is_never() {
        let bible = |buy_random| buy_roll(buy_random, RuleSource::Bible);
        assert_eq!(bible(i16::MIN), Roll::Never);
        assert_eq!(bible(-1), Roll::Never);
        assert_eq!(bible(0), Roll::Never);
        assert_eq!(bible(1), Roll::Chance(1));
        assert_eq!(bible(99), Roll::Chance(99));
        assert_eq!(bible(100), Roll::Always);
        assert_eq!(bible(150), Roll::Always);
    }

    #[test]
    fn a_ship_whose_roll_misses_is_not_for_sale_today() {
        let ships = [buying(129, 35), buying(130, 0), buying(131, 100)];
        let mut chance = Scripted::answering(&[false]);
        let off = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(listed(&off), [131]);
        assert_eq!(chance.asked, [35]);
        let mut chance = Scripted::answering(&[true]);
        let on = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(listed(&on), [129, 131]);
        assert_eq!(row(&on, 129).buy, Ok(()));
        assert_eq!(chance.asked, [35]);
    }

    #[test]
    fn a_ships_roll_is_kept_across_builds_of_one_stay() {
        let ships = [buying(129, 35)];
        let mut rolls = DayRolls::default();
        let mut missing = Scripted::answering(&[false]);
        assert_eq!(
            listed(&rolled(&ships, &pilot(), &mut rolls, &mut missing)),
            Vec::<i16>::new()
        );
        let mut firing = Scripted::answering(&[true]);
        assert_eq!(
            listed(&rolled(&ships, &pilot(), &mut rolls, &mut firing)),
            Vec::<i16>::new()
        );
        assert!(firing.asked.is_empty(), "no second draw");
    }

    #[test]
    fn the_class_flown_is_rolled_like_any_other() {
        let ships = [buying(128, 40), buying(129, 60)];
        let mut chance = Scripted::answering(&[false, true]);
        let shipyard = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(shipyard.current, ShipId(128));
        assert_eq!(listed(&shipyard), [129]);
        assert_eq!(chance.asked, [40, 60]);
    }

    #[test]
    fn a_class_tech_forbids_is_never_rolled() {
        let ships = [
            ShipRecord {
                tech_level: 9,
                ..buying(129, 70)
            },
            buying(131, 30),
            buying(130, 20),
        ];
        let mut chance = Scripted::answering(&[true, true]);
        let shipyard = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(chance.asked, [20, 30], "by ascending ID");
        assert_eq!(listed(&shipyard), [130, 131]);
    }

    #[test]
    fn by_the_engine_a_ship_of_negative_buy_random_is_always_for_sale() {
        let ships = [buying(129, -1), buying(130, i16::MIN), buying(131, 0)];
        let mut chance = Scripted::default();
        let shipyard = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(listed(&shipyard), [129, 130]);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn by_the_bible_it_never_is() {
        let ships = [buying(129, -1), buying(130, 0), buying(131, 100)];
        let site = port();
        let bible = Yard {
            buy_random: RuleSource::Bible,
            ..yard(&ships, &[], &site)
        };
        let shipyard = bible
            .shipyard(&pilot(), &mut DayRolls::default(), &mut NeverFires)
            .expect("a shipyard");
        assert_eq!(listed(&shipyard), [131]);
    }

    #[test]
    fn a_ship_off_today_hides_no_higher_one() {
        let ships = [
            ShipRecord {
                disp_weight: 5,
                flags3: ShipFlags3::HIDE_HIGHER,
                ..buying(129, 50)
            },
            ShipRecord {
                disp_weight: 5,
                ..cheap(130)
            },
        ];
        let off = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut NeverFires);
        assert_eq!(listed(&off), [130]);
        let mut firing = Scripted::answering(&[true]);
        let on = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut firing);
        assert_eq!(listed(&on), [129]);
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

    #[test]
    fn hide_unless_available_shows_an_available_ship() {
        // Availability always holds until control bits exist.
        let gated = ShipRecord {
            flags3: ShipFlags3::HIDE_UNLESS_AVAILABLE,
            availability: "b422".to_owned(),
            ..cheap(129)
        };
        let shipyard = open(&[gated], &pilot());
        assert_eq!(listed(&shipyard), [129]);
        assert_eq!(row(&shipyard, 129).buy, Ok(()));
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
        buy_named(pilot, new, "Kestrel", records)
    }

    /// Buys `new`, naming it `name`.
    fn buy_named(
        pilot: &mut Pilot,
        new: &ShipRecord,
        name: &str,
        records: &[OutfitRecord],
    ) -> ShipPurchase {
        purchase(pilot, FAST.mass, new, name, &[], QUOTE, records, ALONE)
    }

    /// No escorts, the cargo kept by the engine.
    const ALONE: Fleet<'static> = Fleet {
        escorts: &[],
        cargo: RuleSource::Engine,
    };

    /// Buys `new` with `fleet`.
    fn buy_by(
        pilot: &mut Pilot,
        new: &ShipRecord,
        records: &[OutfitRecord],
        fleet: Fleet,
    ) -> ShipPurchase {
        purchase(pilot, FAST.mass, new, "Kestrel", &[], QUOTE, records, fleet)
    }

    /// Buys `new`, whose stock weapons and ammunition are `fits`.
    fn buy_stocked(
        pilot: &mut Pilot,
        new: &ShipRecord,
        fits: &[StockFit],
        records: &[OutfitRecord],
    ) -> ShipPurchase {
        purchase(
            pilot, FAST.mass, new, "Kestrel", fits, QUOTE, records, ALONE,
        )
    }

    /// Two blasters (weapon 128) held by outfit 205, and 20 rockets
    /// (weapon 138's rounds) by outfit 201.
    const STOCK: [StockFit; 2] = [
        StockFit {
            outfit: OutfitId(205),
            mod_type: MOD_WEAPON,
            weapon: WeaponId(128),
            count: 2,
        },
        StockFit {
            outfit: OutfitId(201),
            mod_type: MOD_AMMO,
            weapon: WeaponId(138),
            count: 20,
        },
    ];

    /// Rockets (201), 10 credits each and massless; blasters (205 and 206),
    /// 1000 credits each and a ton.
    fn armoury() -> Vec<OutfitRecord> {
        vec![
            OutfitRecord {
                cost: 10,
                mass: 0,
                max: 100,
                ..outfit(201, &[(MOD_AMMO, 138)])
            },
            outfit(205, &[(MOD_WEAPON, 128)]),
            outfit(206, &[(MOD_WEAPON, 128)]),
        ]
    }

    #[test]
    fn a_bought_ship_owns_its_stock_weapons_and_rounds_after_its_default_items() {
        let mut pilot = pilot();
        buy_stocked(&mut pilot, &heavy(), &STOCK, &armoury());
        assert_eq!(pilot.outfits, map(&[(201, 20), (205, 2)]));
        let with_a_blaster = ShipRecord {
            defaults: vec![(OutfitId(206), 1)],
            ..heavy()
        };
        let mut pilot = self::pilot();
        buy_stocked(&mut pilot, &with_a_blaster, &STOCK, &armoury());
        assert_eq!(
            pilot.outfits,
            map(&[(201, 20), (205, 1), (206, 1)]),
            "its default blaster is one of the two"
        );
        assert_eq!(
            pilot.reserves(),
            ShipStats::new(HEAVY, &outfit_mods(&pilot.outfits, &armoury())).full()
        );
    }

    #[test]
    fn a_carried_persistent_weapon_counts_towards_the_stock_count() {
        let mut records = armoury();
        records.push(OutfitRecord {
            flags: OutfitFlags::PERSISTENT,
            ..outfit(140, &[(MOD_WEAPON, 128)])
        });
        let mut pilot = owning(&[(140, 1), (201, 30)]);
        buy_stocked(&mut pilot, &heavy(), &STOCK, &records);
        assert_eq!(
            pilot.outfits,
            map(&[(140, 1), (201, 20), (205, 1)]),
            "1 + 1, not 1 + 2; the old rockets went with the old ship"
        );
    }

    #[test]
    fn the_new_stock_weapons_count_as_owned_when_carrying_over() {
        // HEAVY has 12 tons free, and FreeMass leaves out its two 1-ton
        // blasters: a persistent 12-ton outfit still fits, a 13-ton one
        // does not.
        let fitting = |mass| {
            let mut records = armoury();
            records.push(persistent(140, mass, 1));
            let mut pilot = owning(&[(140, 1)]);
            let bought = buy_stocked(&mut pilot, &heavy(), &STOCK, &records);
            (pilot.owned(OutfitId(140)), bought.sold_back)
        };
        assert_eq!(fitting(12), (1, BTreeMap::new()));
        assert_eq!(fitting(13), (0, map(&[(140, 1)])));
    }

    #[test]
    fn the_old_ships_stock_weapons_go_in_its_trade_in() {
        assert_eq!(
            trade_in(10_000, 40, &map(&[(201, 20), (205, 2)]), &armoury()),
            2500 + 2 * 500 + 20 * 5,
            "half of each, like any outfit owned"
        );
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
            "Kestrel",
            &[],
            Quote {
                price: 1000,
                trade_in: 2500,
            },
            &[],
            ALONE,
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

    // The cargo a purchase keeps.

    const OPALS: Good = Good::Junk(JunkId(146));
    const FOOD: Good = Good::Commodity(0);
    const METAL: Good = Good::Commodity(3);

    /// A trader escort out of no bay, of `Holds` `holds`.
    fn trader(holds: i16) -> EscortHolds {
        EscortHolds {
            holds,
            inherent_ai: 1,
            carried: false,
        }
    }

    /// The cargo `held` keeps on a new ship of `capacity` tons with
    /// `escorts`, read as `source` says, and what it leaves behind.
    fn kept(
        held: &[(Good, u32)],
        capacity: u32,
        escorts: &[EscortHolds],
        source: RuleSource,
    ) -> (Vec<(Good, u32)>, BTreeMap<Good, u32>) {
        let mut cargo: BTreeMap<Good, u32> = held.iter().copied().collect();
        let left_behind = keep_cargo(&mut cargo, capacity, escorts, source);
        (cargo.into_iter().collect(), left_behind)
    }

    #[test]
    fn by_the_engine_with_no_escorts_the_commodities_stay_and_every_junk_goes() {
        assert_eq!(
            kept(
                &[(FOOD, 6), (METAL, 9), (OPALS, 4)],
                20,
                &[],
                RuleSource::Engine
            ),
            (vec![(FOOD, 6), (METAL, 9)], BTreeMap::from([(OPALS, 4)]))
        );
    }

    #[test]
    fn by_the_engine_commodities_over_the_new_hold_are_trimmed_by_its_ratio() {
        assert_eq!(
            kept(
                &[(FOOD, 30), (METAL, 10), (OPALS, 5)],
                20,
                &[],
                RuleSource::Engine
            ),
            (
                vec![(FOOD, 15), (METAL, 5)],
                BTreeMap::from([(FOOD, 15), (METAL, 5), (OPALS, 5)])
            ),
            "the jünk goes first, then 20 of 40"
        );
    }

    #[test]
    fn by_the_engine_trader_escorts_take_their_share_of_each_good() {
        // A = 20, B = 50: f = 0.4.
        assert_eq!(
            kept(
                &[(FOOD, 40), (METAL, 7), (OPALS, 5)],
                20,
                &[trader(30)],
                RuleSource::Engine
            ),
            (
                vec![(FOOD, 16), (METAL, 2), (OPALS, 3)],
                BTreeMap::from([(FOOD, 24), (METAL, 5), (OPALS, 2)])
            ),
            "the fleet has room, so no trim"
        );
    }

    #[test]
    fn by_the_engine_a_fleet_over_its_holds_trims_the_commodities_and_not_the_junk() {
        // A = 10, B = 20: f = 0.5, leaving 30 food and 5 opals in a fleet
        // of 20 tons; then 30 x 20/35.
        assert_eq!(
            kept(
                &[(FOOD, 60), (OPALS, 10)],
                10,
                &[trader(10)],
                RuleSource::Engine
            ),
            (
                vec![(FOOD, 17), (OPALS, 5)],
                BTreeMap::from([(FOOD, 43), (OPALS, 5)])
            )
        );
    }

    #[test]
    fn by_the_engine_the_escorts_share_is_not_capped_at_32000() {
        // B = 61,000: 640 x 1000/61000 = 10.49; capped at 32,000 it would
        // be 20.
        assert_eq!(
            kept(
                &[(FOOD, 640)],
                1000,
                &[trader(30_000), trader(30_000)],
                RuleSource::Engine
            )
            .0,
            [(FOOD, 10)]
        );
    }

    #[test]
    fn by_the_engine_a_new_ship_of_no_cargo_space_keeps_no_commodity_and_every_junk() {
        for escorts in [&[][..], &[trader(30)]] {
            assert_eq!(
                kept(&[(FOOD, 10), (OPALS, 4)], 0, escorts, RuleSource::Engine),
                (vec![(OPALS, 4)], BTreeMap::from([(FOOD, 10)])),
                "{escorts:?}"
            );
        }
    }

    #[test]
    fn by_the_engine_an_escort_of_negative_holds_does_not_raise_the_share_past_all() {
        // A = 20, B = 15: A/B = 1.33, capped at 1 (@0xcf27-0xcf37).
        assert_eq!(
            kept(
                &[(FOOD, 10), (OPALS, 4)],
                20,
                &[trader(-5)],
                RuleSource::Engine
            ),
            (vec![(FOOD, 10)], BTreeMap::from([(OPALS, 4)]))
        );
    }

    #[test]
    fn by_the_engine_escorts_bringing_the_fleet_to_no_space_share_all_and_trim_all() {
        // A = 10, B = 0: A/0 is +inf, and `minsd` gives 1 (@0xcf33); the
        // fleet's holds are then none, so the trim takes every commodity.
        assert_eq!(
            kept(
                &[(FOOD, 10), (OPALS, 4)],
                10,
                &[trader(-10)],
                RuleSource::Engine
            ),
            (vec![], BTreeMap::from([(FOOD, 10), (OPALS, 4)]))
        );
    }

    #[test]
    fn by_the_engine_escorts_bringing_the_fleet_below_no_space_add_to_each_junk() {
        // A = 10, B = -20: f = A/B = -0.5, which `minsd` keeps (@0xcf33).
        // Each commodity becomes trunc(10 x -0.5), clamped to none
        // (@0xcf7c-0xcf84); each `jünk` loses trunc(4 x -0.5) = -2, so
        // gains 2 (@0xcfc0-0xcfc3).
        assert_eq!(
            kept(
                &[(FOOD, 10), (OPALS, 4)],
                10,
                &[trader(-30)],
                RuleSource::Engine
            ),
            (vec![(OPALS, 6)], BTreeMap::from([(FOOD, 10)]))
        );
    }

    #[test]
    fn by_the_engine_a_warship_escort_or_a_launched_fighter_takes_no_share() {
        let warship = EscortHolds {
            inherent_ai: 3,
            ..trader(30)
        };
        let fighter = EscortHolds {
            carried: true,
            ..trader(30)
        };
        assert_eq!(
            kept(
                &[(FOOD, 10), (OPALS, 4)],
                20,
                &[warship, fighter],
                RuleSource::Engine
            )
            .0,
            [(FOOD, 10)]
        );
    }

    #[test]
    fn by_the_engine_the_trim_is_worked_in_single_floats() {
        // 22 x (13 / 22) is 13 in doubles, but 12.999999 in singles.
        assert_eq!(
            kept(&[(FOOD, 22)], 13, &[], RuleSource::Engine).0,
            [(FOOD, 12)]
        );
    }

    #[test]
    fn by_the_other_reading_trader_escorts_are_ignored() {
        assert_eq!(
            kept(
                &[(FOOD, 40), (METAL, 7), (OPALS, 5)],
                20,
                &[trader(30)],
                RuleSource::Bible
            ),
            (
                vec![(FOOD, 20)],
                BTreeMap::from([(FOOD, 20), (METAL, 7), (OPALS, 5)])
            )
        );
    }

    #[test]
    fn a_purchase_keeps_the_cargo_by_the_engine_on_the_new_ship_with_its_pods() {
        let records = [OutfitRecord {
            mass: 0,
            ..outfit(150, &[(MORE_CARGO, 3)])
        }];
        let with_pod = ShipRecord {
            defaults: vec![(OutfitId(150), 1)],
            ..heavy()
        };
        let mut pilot = pilot();
        pilot.cargo = BTreeMap::from([(FOOD, 30), (OPALS, 4)]);
        let fleet = Fleet {
            escorts: &[trader(18)],
            cargo: RuleSource::Engine,
        };
        let bought = buy_by(&mut pilot, &with_pod, &records, fleet);
        assert_eq!(
            pilot.cargo().collect::<Vec<_>>(),
            [(FOOD, 15), (OPALS, 2)],
            "18 tons, 15 and a 3-ton pod, of 36"
        );
        assert_eq!(bought.left_behind, BTreeMap::from([(FOOD, 15), (OPALS, 2)]));
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
        let buy = |pilot: &mut Pilot, new: &ShipRecord, records: &[OutfitRecord]| {
            let fleet = Fleet {
                escorts: &[],
                cargo: RuleSource::Bible,
            };
            buy_by(pilot, new, records, fleet)
        };
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
    fn the_prompt_names_the_long_name_and_the_default_is_the_class_name_and_three_digits() {
        let mut chance = Scripted::rolling(&[3, 8, 0]);
        let naming = naming(&ship(129, FAST), &mut chance);
        assert_eq!(
            naming,
            ShipNaming {
                ship: ShipId(129),
                prompt: "Please name your new The Ship 129: ".to_owned(),
                default: "Ship 129 491".to_owned(),
            }
        );
        assert_eq!(chance.sides_asked, [9, 9, 9]);
    }

    #[test]
    fn the_name_drops_a_leading_the_and_nothing_else() {
        assert_eq!(cull_name("The Raven"), "Raven");
        assert_eq!(cull_name("tHe Raven"), "Raven");
        assert_eq!(cull_name("THE Raven"), "Raven");
        for kept in [
            "Theodore",
            "the",
            "The",
            " the Raven",
            "",
            "thе Raven",
            "Raven",
        ] {
            assert_eq!(cull_name(kept), kept);
        }
        assert_eq!(cull_name("the  two"), " two");
        assert_eq!(cull_name("the "), "");
    }

    #[test]
    fn buying_names_the_ship_as_confirmed() {
        let mut pilot = pilot();
        assert_eq!(pilot.ship_name(), Some(""));
        buy_named(&mut pilot, &heavy(), "The Kestrel", &[]);
        assert_eq!(pilot.ship_name(), Some("Kestrel"));
        buy_named(&mut pilot, &heavy(), "", &[]);
        assert_eq!(pilot.ship_name(), Some(""), "an empty name is kept");
    }

    #[test]
    fn buying_a_ship_leaves_everything_else_about_the_pilot_as_it_was() {
        let mut pilot = pilot();
        pilot.stellar = Some(StellarId(128));
        pilot.course = vec![SystemId(131), SystemId(132)];
        pilot.explore(SystemId(131));
        pilot.set_legal_record(GovtId(128), 40);
        pilot.events = BTreeMap::from([(
            DisasterId(128),
            crate::market::ActiveEvent {
                days: 3,
                stellar: Some(StellarId(140)),
            },
        )]);
        pilot.name = "Ada".to_owned();
        pilot.escorts = vec![crate::pilot::Escort {
            ship: ShipId(130),
            reserves: Reserves::full(1.0, 2.0, 3.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }];
        pilot.gone_persons = std::collections::BTreeSet::from([crate::catalog::PersonId(151)]);
        pilot.grudges = std::collections::BTreeSet::from([crate::catalog::PersonId(152)]);
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
                pilot.stock_weapons_pending,
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
                before.stock_weapons_pending,
            )
        );
        assert_eq!(
            (&pilot.escorts, &pilot.gone_persons, &pilot.grudges),
            (&before.escorts, &before.gone_persons, &before.grudges)
        );
    }
}
