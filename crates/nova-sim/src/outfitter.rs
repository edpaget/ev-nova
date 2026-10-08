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
//! is this government alone. `Availability` goes through
//! [`control_bits_allow`], which always holds until control bits exist,
//! so for now every outfit gated by one is available (stock `oütf` 342,
//! "Area Map - Vell-os", shows everywhere).
//!
//! # `BuyRandom`: what is for sale today
//!
//! `BuyRandom` is the percent chance a day that an outfit is for sale.
//! The original (`EV Nova.app`) draws one roll per outfit, `_Rand(100) +
//! 1`, for the whole galaxy each day, for a new pilot and on load, and
//! never saves it (`_IncrementGameTime` @0xb7c4-0xb833, `_ResetPlayer`
//! @0x1d84a, `_LoadPilotData` @0x76407); a day always passes at take-off
//! (`_PlayerLandOnStellar` @0x63440), so each landing sees fresh rolls
//! and nothing rolls again during a stay. `_SetupPortAvailableItems`
//! (@0xbedb, @0xc05d-0xc08f) then sells, among the outfits tech allows:
//!
//! - one the player owns, with no roll consulted; its roll is set so it
//!   stays for sale for the rest of the day, even after the last is
//!   sold;
//! - any other when its `BuyRandom` is above 0 and at least the day's
//!   roll: a `BuyRandom` % chance, and 100 always.
//!
//! The loader clamps `BuyRandom` to 0-100 (`_LoadObjectData`
//! @0x78b8d-0x78ba6), so by the engine an outfit of `BuyRandom` below 1
//! is never for sale (the stock data's 51 such are mission, granted or
//! variant items) and one of 100 or more always. The Bible says instead
//! that a value below 1 or above 100 means 100; the rulebook's
//! [`RuleKey::BuyRandom`](crate::RuleKey::BuyRandom) chooses
//! (`buy_roll`).
//!
//! Here each outfit's roll is drawn on the caller's
//! [`Chance`] the first time the list is built after a
//! landing, by ascending ID and only for an outfit that tech allows,
//! that the player does not own and whose reading is a chance of 1-99,
//! and kept until the next landing; it is never saved, as the original
//! rolls again on load. The player only ever sees one stellar's list in a
//! day, and the original always passes a day between two landings, so a
//! roll a landing is the original's roll a day. Buying or selling an
//! outfit draws no roll again. An outfit off today is not listed and
//! takes no higher one off sale; one the player owns that is flagged
//! [`OutfitFlags::SELL_ANYWHERE`] is still listed, sell-only.
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
//! when it is positive. The ship's free mass is its `FreeMass`, plus the
//! mass of its standard equipment, its default items and its stock weapons
//! and their `AmmoLoad` (`FreeMass` is space on top of them: the loader
//! adds their mass to it, `_LoadObjectData` @0x7aaf7, @0x7ac61, @0x7ad8d),
//! less the mass of every outfit it carries: a new ship has exactly its
//! `FreeMass` free, and an outfit of negative mass adds space.
//!
//! # Buying and selling
//!
//! Each order buys or sells one, and a refused one changes nothing. A buy
//! is refused when the outfit cannot be bought here, the player owns its
//! `Max` already, it is a gun or a turret past the ship's limit (below),
//! it is a fighter whose bays have no room left (their
//! [`capacity`](crate::bay::capacity), the fighters out counted against
//! it, `_CanBuyFighter` @0x5a82; refused as `Max` owned), the ship's
//! `Holds` is negative and the outfit adds mass
//! space, there is not the free mass for it, or the player cannot pay. A
//! buy pays the price and adds one, or with
//! [`OutfitFlags::REMOVE_AFTER_PURCHASE`] only pays. A sale is refused
//! when the player owns none, the outfit is flagged
//! [`OutfitFlags::CANNOT_SELL`], it is neither for sale here nor flagged
//! [`OutfitFlags::SELL_ANYWHERE`], the ship would be left with negative
//! free mass, it is one whose `Max` a `ModType` 27 item raises, while it
//! holds more than that leaves (below), or it is a launcher whose
//! ammunition must be sold first (below). A sale pays [`RESALE_PERCENT`] of the price and removes one.
//! Selling cargo space below the cargo held is allowed: the exchange then
//! shows no space free until enough is sold.
//!
//! **Guns and turrets** (`_HasMaxOfItem` @0x46c8-0x4866, after `Max`).
//! The guns owned are the count of every outfit owned flagged
//! [`OutfitFlags::GUN`], stock weapons included, and the turrets those
//! flagged [`OutfitFlags::TURRET`]: the flag alone decides. A gun's limit
//! is the class's `MaxGun` plus, for each outfit owned, the `ModVal` of
//! its first [`MORE_GUNS`] mod, once per outfit whatever the count; a
//! gun is refused when the guns owned are at the limit or above, so a
//! limit of 0 or less buys none. A turret is the same with `MaxTur` and
//! [`MORE_TURRETS`], checked after the gun limit for an outfit flagged
//! both. Selling never checks them.
//!
//! **Raised maximums** (`ModType` 27, [`RAISES_MAX`]). An outfit's mod of
//! `ModType` 27 names, by its raw `ModVal`, an outfit whose `Max` it
//! raises. By the engine, the target's `Max` is multiplied by the sum,
//! over the outfits owned, of the count owned times that outfit's mods
//! naming it, or by 1 when that is none (`_HasMaxOfItem` @0x45e9-0x46c3):
//! owning one raiser leaves the `Max` as it is, two double it, and a
//! `Max` of 0 or less stays none. The buy check reads it in place of the
//! `Max`, before the gun limit; a fighter's bay room keeps the raw `Max`
//! (`_CanBuyFighter` @0x5c3d-0x5c4f). Selling one of n raisers is refused,
//! after the free mass and before the launcher, while the target held is
//! more than its raw `Max` x (n - 1), other raisers ignored and with no
//! floor, the first such mod in slot order naming the excess to be sold
//! first (`_DoOutfitDialog` @0x5c7be-0x5ca6f), so the last raiser cannot
//! be sold while any of its target is held. By the Bible, the multiplier
//! counts each item owned once, and a sale is refused only while the
//! target held is more than the maximum left after it
//! ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)). Boarding reads the
//! raised `Max` too (see [`grant`](crate::grant)).
//!
//! **A launcher before its ammunition** (`_DoOutfitDialog`
//! @0x5ca75-0x5cbe0, after the free mass). A launcher is an outfit whose
//! first `ModType` 1 names a weapon
//! ([`Magazine`](crate::combat::armament::Magazine)). Its rounds are those
//! of its ammunition held, a bay's fighters out counted. By the engine,
//! selling one of n is refused while its weapon's `MaxAmmo` is above 0 and
//! the rounds are more than `MaxAmmo` x (n - 1), the excess to be sold
//! first; in stock data only fighter bays have a `MaxAmmo`. The other
//! reading refuses it while any round is held
//! ([`RuleKey::LauncherSale`](crate::RuleKey::LauncherSale)). The
//! original shows the refusal as a text dialog with Sell left enabled
//! (`STR#` 2002 #208-212); [`Outfitter::lc_names`] carries the names its
//! words need.
//!
//! Not modelled yet: `MaxAmmo` for ammunition other than fighters. Which
//! outfits a ship bought in the [`shipyard`](crate::shipyard) keeps is
//! the shipyard's (flag 0x0004); flag 0x0020 only concerns a mission's
//! change of ship.

use std::collections::BTreeMap;

use crate::catalog::{GovtId, LandingSite, OutfitId, OutfitRecord};
use crate::chance::Chance;
use crate::fuel::OutfitMod;
use crate::handling::ShipFields;
use crate::landing::StellarFlags;
use crate::market::{Direction, control_bits_allow};
use crate::pilot::Pilot;
use crate::rulebook::RuleSource;
use crate::wares::{self, ALWAYS_RANDOM, DayRolls, HideBits, HideHigher, Roll};

/// The `oütf` `Flags` bits the outfitter reads (the Bible).
#[derive(Clone, Copy, Debug)]
pub struct OutfitFlags;

impl OutfitFlags {
    /// A fixed gun: counts against the ship's `MaxGun`.
    pub const GUN: u16 = 0x0001;
    /// A turret: counts against the ship's `MaxTur`.
    pub const TURRET: u16 = 0x0002;
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

/// The `oütf` `ModType` that changes the ship's `MaxGun` by its `ModVal`.
pub const MORE_GUNS: i16 = 45;
/// The `oütf` `ModType` that changes the ship's `MaxTur` by its `ModVal`.
pub const MORE_TURRETS: i16 = 46;
/// The `oütf` `ModType` that multiplies the `Max` of the outfit its
/// `ModVal` names.
pub const RAISES_MAX: i16 = 27;

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
    /// It is a fighter, and the ship's bays have no room for another
    /// (`_CanBuyFighter` @0x5a82). The original reaches that check only
    /// from `_CanBuyOutfitItem` (@0x4e938), not from `_HasMaxOfItem`, so
    /// its info box gives this none of the #219/#220 words the other
    /// `Max` refusals get. It would still show the mass words #221/#222
    /// if the fighter also lacked the mass; here the bays are checked
    /// first, so nothing shows (roadmap `shop-and-trade-fidelity`,
    /// `phase-20-outfitter-full-bays-mass-words`).
    BaysFull,
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
    /// The ship's guns are at its `MaxGun`, as `ModType` 45 changes it.
    GunLimit,
    /// The ship's turrets are at its `MaxTur`, as `ModType` 46 changes it.
    TurretLimit,
    /// It is a launcher, and `rounds` of its ammunition must be sold
    /// first.
    AmmunitionFirst {
        /// The rounds to sell first.
        rounds: u32,
        /// The outfit that names the ammunition, if any.
        ammo: Option<OutfitId>,
    },
    /// It raises the `Max` of `target` (`ModType` 27), and `count` of that
    /// must be sold first.
    RaisedFirst {
        /// How many of `target` to sell first.
        count: u32,
        /// The outfit whose `Max` it raises.
        target: OutfitId,
    },
}

/// An outfit's lower-case names, as the outfitter's words use them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LcNames {
    /// Its `LCName`.
    pub singular: String,
    /// Its `LCPlural`.
    pub plural: String,
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
    /// The `LCName` and `LCPlural` of every outfit a refusal's words name:
    /// each launcher refused for its ammunition, and the outfit naming
    /// that ammunition; each raiser refused for its target, and that
    /// target.
    pub lc_names: BTreeMap<OutfitId, LcNames>,
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

/// How an outfit of `buy_random` rolls for sale each day, read as
/// `source` says ([`RuleKey::BuyRandom`](crate::RuleKey::BuyRandom)): by
/// the engine, below 1 never and 100 or more always (`_LoadObjectData`
/// clamps it to 0-100, @0x78b8d-0x78ba6, and `_SetupPortAvailableItems`
/// sells only above 0, @0xc07a); by the Bible, below 1 or above 100
/// always. Any other is a `BuyRandom` % chance.
pub(crate) fn buy_roll(buy_random: i16, source: RuleSource) -> Roll {
    match source {
        RuleSource::Bible if !(1..=ALWAYS_RANDOM).contains(&buy_random) => Roll::Always,
        _ => Roll::of(buy_random),
    }
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

/// The free mass of a ship with `fields` and this `standard` equipment
/// (its default items and stock weapons, whose mass `FreeMass` leaves
/// out), carrying `owned`.
#[must_use]
pub(crate) fn free_mass(
    fields: ShipFields,
    standard: &BTreeMap<OutfitId, u16>,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> i64 {
    i64::from(fields.free_mass) + mass_of(standard, records, fields.mass)
        - mass_of(owned, records, fields.mass)
}

/// A ship class's room for guns and turrets: its `MaxGun` and `MaxTur`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Hardpoints {
    /// Its `MaxGun`.
    pub(crate) guns: i16,
    /// Its `MaxTur`.
    pub(crate) turrets: i16,
}

/// A launcher outfit the player owns, and the ammunition it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Launcher {
    /// Its weapon's `MaxAmmo`; none for no limit of its own.
    pub(crate) max_ammo: u32,
    /// The rounds of its ammunition held, the fighters out counted.
    pub(crate) rounds: u32,
    /// The outfit that names its ammunition, if any.
    pub(crate) ammo: Option<OutfitId>,
}

impl Launcher {
    /// The rounds that must be sold before one of `owned` launchers can
    /// be, as `source` reads the rule
    /// ([`RuleKey::LauncherSale`](crate::RuleKey::LauncherSale)); `None`
    /// when it can be sold. By the engine, the rounds the remaining
    /// launchers' `MaxAmmo` cannot hold, only when `MaxAmmo` is above 0
    /// (`_DoOutfitDialog` @0x5cad0, @0x5cb9c-0x5cbe0); by the other
    /// reading, every round held.
    pub(crate) fn excess(&self, owned: u16, source: RuleSource) -> Option<u32> {
        if self.rounds == 0 {
            return None;
        }
        match source {
            RuleSource::Engine => {
                let kept = self
                    .max_ammo
                    .saturating_mul(u32::from(owned.saturating_sub(1)));
                (self.max_ammo > 0).then_some(self.rounds.saturating_sub(kept))
            }
            RuleSource::Bible => Some(self.rounds),
        }
        .filter(|&excess| excess > 0)
    }
}

/// `base` changed by the `ModVal` of the first mod of `mod_type` of each
/// outfit owned, once per outfit whatever the count
/// (`_HasMaxOfItem` @0x472b-0x4796).
fn raised(
    base: i16,
    mod_type: i16,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
) -> i32 {
    let mods: i32 = records
        .iter()
        .filter(|record| owned.get(&record.id).is_some_and(|&count| count > 0))
        .filter_map(|record| record.mods.iter().find(|&&(kind, _)| kind == mod_type))
        .map(|&(_, mod_val)| i32::from(mod_val))
        .sum();
    i32::from(base) + mods
}

/// `record`'s `Max` as the [`RAISES_MAX`] outfits `owned` raise it, read
/// as `source` says ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)).
/// By the engine, the multiplier is the sum, over the outfits owned, of
/// the count owned times the number of its mods naming `record`; by the
/// Bible, the count owned of each outfit with any such mod. Either way it
/// is at least 1 (`_HasMaxOfItem` @0x45e9-0x46c3), so a `Max` of 0 or less
/// stays so. The engine's sums and product are 16-bit (`addw`/`imulw`)
/// and wrap; here they are wide and saturate.
pub(crate) fn raised_max(
    record: &OutfitRecord,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    source: RuleSource,
) -> i64 {
    let raisers: i64 = records
        .iter()
        .filter_map(|raiser| Some((raiser, *owned.get(&raiser.id)?)))
        .map(|(raiser, count)| {
            let mods: i64 = raiser
                .mods
                .iter()
                .filter(|&&(kind, val)| kind == RAISES_MAX && val == record.id.0)
                .map(|_| 1)
                .sum();
            let mods = match source {
                RuleSource::Engine => mods,
                RuleSource::Bible => mods.min(1),
            };
            i64::from(count) * mods
        })
        .fold(0, i64::saturating_add);
    i64::from(record.max).saturating_mul(raisers.max(1))
}

/// How many of the outfits `owned` are flagged `flag`.
fn flagged(flag: u16, owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> i32 {
    records
        .iter()
        .filter(|record| record.flags & flag != 0)
        .filter_map(|record| owned.get(&record.id))
        .map(|&count| i32::from(count))
        .sum()
}

/// The guns and turrets a ship carries, and its limits for each
/// (`_HasMaxOfItem` @0x46c8-0x4866; see the module docs).
struct Armed {
    /// The outfits owned flagged [`OutfitFlags::GUN`].
    guns: i32,
    /// The outfits owned flagged [`OutfitFlags::TURRET`].
    turrets: i32,
    /// `MaxGun`, as [`MORE_GUNS`] changes it.
    gun_limit: i32,
    /// `MaxTur`, as [`MORE_TURRETS`] changes it.
    turret_limit: i32,
}

impl Armed {
    /// The guns and turrets of a ship of `hardpoints` owning `owned`.
    fn of(
        hardpoints: Hardpoints,
        owned: &BTreeMap<OutfitId, u16>,
        records: &[OutfitRecord],
    ) -> Self {
        Self {
            guns: flagged(OutfitFlags::GUN, owned, records),
            turrets: flagged(OutfitFlags::TURRET, owned, records),
            gun_limit: raised(hardpoints.guns, MORE_GUNS, owned, records),
            turret_limit: raised(hardpoints.turrets, MORE_TURRETS, owned, records),
        }
    }

    /// Why one more outfit of `flags` cannot be bought, the gun limit
    /// checked first, if either refuses it.
    fn refusal(&self, flags: u16) -> Option<OutfitRefusal> {
        if flags & OutfitFlags::GUN != 0 && self.guns >= self.gun_limit {
            Some(OutfitRefusal::GunLimit)
        } else if flags & OutfitFlags::TURRET != 0 && self.turrets >= self.turret_limit {
            Some(OutfitRefusal::TurretLimit)
        } else {
            None
        }
    }
}

/// `record`'s lower-case names.
fn lc_names(record: &OutfitRecord) -> LcNames {
    LcNames {
        singular: record.lc_name.clone(),
        plural: record.lc_plural.clone(),
    }
}

/// Everything the outfitter rules read about the ship and where it is.
pub(crate) struct Shop<'a> {
    /// Every `oütf`.
    pub(crate) records: &'a [OutfitRecord],
    /// The ship's fields.
    pub(crate) fields: ShipFields,
    /// The ship's standard equipment: its default items, and its stock
    /// weapons and their ammunition as outfits.
    pub(crate) standard: &'a BTreeMap<OutfitId, u16>,
    /// The stellar landed on.
    pub(crate) site: &'a LandingSite,
    /// For every ammunition outfit of a fighter bay, the fighters the
    /// ship's bays can still take (see [`bay`](crate::bay)).
    pub(crate) fighter_room: &'a BTreeMap<OutfitId, u32>,
    /// How `BuyRandom` reads ([`buy_roll`]).
    pub(crate) buy_random: RuleSource,
    /// The ship class's `MaxGun` and `MaxTur`.
    pub(crate) hardpoints: Hardpoints,
    /// Each launcher outfit, with the ammunition it holds.
    pub(crate) launchers: &'a BTreeMap<OutfitId, Launcher>,
    /// How a launcher's sale reads ([`Launcher::excess`]).
    pub(crate) launcher_sale: RuleSource,
    /// How a `ModType` 27 outfit raises its target's `Max` ([`raised_max`]).
    pub(crate) raised_max: RuleSource,
}

impl Shop<'_> {
    /// Why one of `owned` of launcher `record` cannot be sold for its
    /// ammunition, if it cannot, with the names its words need put in
    /// `names`: its own, and those of the outfit naming its ammunition.
    fn ammunition_first(
        &self,
        record: &OutfitRecord,
        owned: u16,
        names: &mut BTreeMap<OutfitId, LcNames>,
    ) -> Option<OutfitRefusal> {
        let launcher = self.launchers.get(&record.id)?;
        let rounds = launcher.excess(owned, self.launcher_sale)?;
        names.insert(record.id, lc_names(record));
        let ammo = launcher.ammo;
        if let Some(named) = self.records.iter().find(|named| Some(named.id) == ammo) {
            names.insert(named.id, lc_names(named));
        }
        Some(OutfitRefusal::AmmunitionFirst { rounds, ammo })
    }

    /// Why one of `owned` of `record` cannot be sold for the outfits whose
    /// `Max` it raises, if it cannot, with the names its words need put in
    /// `names`: its own and the target's (`_DoOutfitDialog`
    /// @0x5c7be-0x5ca6f). Each of its [`RAISES_MAX`] mods is checked in
    /// slot order against `all`, the outfits owned, and the first whose
    /// target is held past what is left refuses. By the engine, that is
    /// the target's raw `Max` x (`owned` - 1), other raisers ignored and
    /// with no floor; by the Bible, the target's [`raised_max`] once one
    /// `record` is sold. The original reads only a `ModVal` of 128-639
    /// (@0x5c819-0x5c827); one outside names no outfit and none of it is
    /// owned, so it never refuses here either.
    fn raised_first(
        &self,
        record: &OutfitRecord,
        owned: u16,
        all: &BTreeMap<OutfitId, u16>,
        names: &mut BTreeMap<OutfitId, LcNames>,
    ) -> Option<OutfitRefusal> {
        let after = || {
            let mut after = all.clone();
            match owned.saturating_sub(1) {
                0 => after.remove(&record.id),
                left => after.insert(record.id, left),
            };
            after
        };
        let (count, target, target_record) = record
            .mods
            .iter()
            .filter(|&&(kind, _)| kind == RAISES_MAX)
            .find_map(|&(_, val)| {
                let target = OutfitId(val);
                let held = i64::from(all.get(&target).copied().unwrap_or(0));
                let target_record = self.records.iter().find(|other| other.id == target);
                let left = match (self.raised_max, target_record) {
                    (_, None) => 0,
                    (RuleSource::Engine, Some(other)) => {
                        i64::from(other.max) * (i64::from(owned) - 1)
                    }
                    (RuleSource::Bible, Some(other)) => {
                        raised_max(other, &after(), self.records, RuleSource::Bible)
                    }
                };
                let excess = held - left;
                (excess > 0).then_some((excess, target, target_record))
            })?;
        names.insert(record.id, lc_names(record));
        if let Some(other) = target_record {
            names.insert(other.id, lc_names(other));
        }
        Some(OutfitRefusal::RaisedFirst {
            count: u32::try_from(count).unwrap_or(u32::MAX),
            target,
        })
    }

    /// Whether `pilot` can sell one of `record` here, or why not, with the
    /// names a refusal's words need put in `names`: `here` is whether it
    /// can be sold here at all (for sale, or sold anywhere), and
    /// `free_after` the ship's free mass once it is sold (see the module
    /// docs).
    fn sale(
        &self,
        record: &OutfitRecord,
        pilot: &Pilot,
        here: bool,
        free_after: i64,
        names: &mut BTreeMap<OutfitId, LcNames>,
    ) -> Result<(), OutfitRefusal> {
        let owned = pilot.owned(record.id);
        if owned == 0 {
            Err(OutfitRefusal::NoneOwned)
        } else if record.flags & OutfitFlags::CANNOT_SELL != 0 {
            Err(OutfitRefusal::CannotSell)
        } else if !here {
            Err(OutfitRefusal::NotBoughtHere)
        } else if free_after < 0 {
            Err(OutfitRefusal::NegativeFreeMass)
        } else if let Some(refusal) = self
            .raised_first(record, owned, &pilot.outfits, names)
            .or_else(|| self.ammunition_first(record, owned, names))
        {
            Err(refusal)
        } else {
            Ok(())
        }
    }

    /// The outfitter, for `pilot`, each outfit's roll for the day kept in
    /// `rolls` and any not drawn yet drawn on `chance` (see the module
    /// docs); `None` when the stellar has none.
    pub(crate) fn outfitter(
        &self,
        pilot: &Pilot,
        rolls: &mut DayRolls<OutfitId>,
        chance: &mut dyn Chance,
    ) -> Option<Outfitter> {
        if self.site.flags & StellarFlags::OUTFITTER == 0 {
            return None;
        }
        let contributed = wares::contributed(self.fields.contribute, &pilot.outfits, self.records);
        let free = free_mass(self.fields, self.standard, &pilot.outfits, self.records);
        let mut sorted: Vec<&OutfitRecord> = self.records.iter().collect();
        sorted.sort_by_key(|record| record.id);
        let armed = Armed::of(self.hardpoints, &pilot.outfits, self.records);
        let mut sweep = HideHigher::default();
        let mut rows = Vec::new();
        let mut names = BTreeMap::new();
        for record in sorted {
            let owned = pilot.owned(record.id);
            let required = !requirements_apply(record.require_govt, self.site.govt)
                || wares::requirement_met(record.require, contributed);
            let available = control_bits_allow(&record.availability);
            let today = tech_allows(record, self.site)
                && if owned > 0 {
                    rolls.hold(record.id);
                    true
                } else {
                    let roll = buy_roll(record.buy_random, self.buy_random);
                    rolls.today(record.id, roll, chance)
                };
            let for_sale = today && sweep.on_sale(record.disp_weight);
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
            } else if i64::from(owned)
                >= raised_max(record, &pilot.outfits, self.records, self.raised_max)
            {
                Err(if record.max <= 0 {
                    OutfitRefusal::NoneAllowed
                } else {
                    OutfitRefusal::MaxOwned
                })
            } else if let Some(refusal) = armed.refusal(record.flags) {
                Err(refusal)
            } else if self.fighter_room.get(&record.id) == Some(&0) {
                Err(OutfitRefusal::BaysFull)
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
            let selling = self.sale(
                record,
                pilot,
                for_sale || sells_anywhere,
                free + mass,
                &mut names,
            );
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
            lc_names: names,
        })
    }
}

/// Buys or sells one of `record` at `price` as `direction` says, paying
/// or being paid.
pub(crate) fn settle(pilot: &mut Pilot, record: &OutfitRecord, direction: Direction, price: i64) {
    let owned = pilot.owned(record.id);
    let owned = match direction {
        Direction::Buy => {
            pilot.cash = pilot.cash.saturating_sub(price);
            if record.flags & OutfitFlags::REMOVE_AFTER_PURCHASE == 0 {
                owned.saturating_add(1)
            } else {
                owned
            }
        }
        Direction::Sell => {
            pilot.cash = pilot.cash.saturating_add(resale(price));
            owned.saturating_sub(1)
        }
    };
    if owned == 0 {
        pilot.outfits.remove(&record.id);
    } else {
        pilot.outfits.insert(record.id, owned);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::stats::{MORE_FUEL, MORE_SPEED};
    use crate::testkit::{FAST, Scripted, catalog, outfit, planet};

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

    static NO_STANDARD: BTreeMap<OutfitId, u16> = BTreeMap::new();
    static NO_FIGHTERS: BTreeMap<OutfitId, u32> = BTreeMap::new();
    static NO_LAUNCHERS: BTreeMap<OutfitId, Launcher> = BTreeMap::new();

    /// Room for more guns and turrets than any test buys.
    const ROOMY: Hardpoints = Hardpoints {
        guns: 99,
        turrets: 99,
    };

    /// The outfitter of `records` at `site`, reading `BuyRandom` by the
    /// engine.
    fn shop<'a>(records: &'a [OutfitRecord], site: &'a LandingSite) -> Shop<'a> {
        Shop {
            records,
            fields: FAST,
            standard: &NO_STANDARD,
            site,
            fighter_room: &NO_FIGHTERS,
            buy_random: RuleSource::Engine,
            hardpoints: ROOMY,
            launchers: &NO_LAUNCHERS,
            launcher_sale: RuleSource::Engine,
            raised_max: RuleSource::Engine,
        }
    }

    /// The outfitter at `site` with no roll drawn yet, and none that
    /// fires.
    fn open_at(records: &[OutfitRecord], site: &LandingSite, pilot: &Pilot) -> Option<Outfitter> {
        shop(records, site).outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
    }

    /// The outfitter at [`port`] on `rolls`, drawing on `chance`.
    fn rolled(
        records: &[OutfitRecord],
        pilot: &Pilot,
        rolls: &mut DayRolls<OutfitId>,
        chance: &mut dyn Chance,
    ) -> Outfitter {
        shop(records, &port())
            .outfitter(pilot, rolls, chance)
            .expect("an outfitter")
    }

    /// Outfit `id` of `buy_random`.
    fn buying(id: i16, buy_random: i16) -> OutfitRecord {
        OutfitRecord {
            buy_random,
            ..outfit(id, &[])
        }
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

    // BuyRandom.

    #[test]
    fn by_the_engine_an_outfits_buy_random_below_1_is_never_and_above_99_always() {
        let engine = |buy_random| buy_roll(buy_random, RuleSource::Engine);
        assert_eq!(engine(i16::MIN), Roll::Never);
        assert_eq!(engine(-1), Roll::Never);
        assert_eq!(engine(0), Roll::Never);
        assert_eq!(engine(1), Roll::Chance(1));
        assert_eq!(engine(99), Roll::Chance(99));
        assert_eq!(engine(100), Roll::Always);
        assert_eq!(engine(250), Roll::Always);
    }

    #[test]
    fn by_the_bible_an_outfits_buy_random_outside_1_to_100_is_always() {
        let bible = |buy_random| buy_roll(buy_random, RuleSource::Bible);
        assert_eq!(bible(-1), Roll::Always);
        assert_eq!(bible(0), Roll::Always);
        assert_eq!(bible(1), Roll::Chance(1));
        assert_eq!(bible(50), Roll::Chance(50));
        assert_eq!(bible(99), Roll::Chance(99));
        assert_eq!(bible(100), Roll::Always);
        assert_eq!(bible(101), Roll::Always);
        assert_eq!(bible(i16::MAX), Roll::Always);
    }

    #[test]
    fn an_outfit_whose_roll_misses_is_not_for_sale_today() {
        let records = [buying(128, 50)];
        let mut chance = Scripted::answering(&[false]);
        let off = rolled(&records, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(listed(&off), Vec::<i16>::new());
        assert_eq!(chance.asked, [50]);
        let mut chance = Scripted::answering(&[true]);
        let on = rolled(&records, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(listed(&on), [128]);
        assert_eq!(buy(&on), Ok(()));
        assert_eq!(chance.asked, [50]);
    }

    #[test]
    fn only_outfits_tech_allows_are_rolled_by_ascending_id() {
        let records = [
            buying(130, 40),
            buying(128, 60),
            OutfitRecord {
                tech_level: 9,
                ..buying(129, 70)
            },
            buying(131, 100),
            buying(132, 0),
        ];
        let mut chance = Scripted::answering(&[true, true]);
        let outfitter = rolled(&records, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(chance.asked, [60, 40]);
        assert_eq!(listed(&outfitter), [128, 130, 131]);
    }

    #[test]
    fn an_outfits_roll_is_kept_across_builds_of_one_stay() {
        let records = [buying(128, 50)];
        let mut rolls = DayRolls::default();
        let mut missing = Scripted::answering(&[false]);
        assert_eq!(
            listed(&rolled(&records, &pilot(), &mut rolls, &mut missing)),
            Vec::<i16>::new()
        );
        let mut firing = Scripted::answering(&[true]);
        assert_eq!(
            listed(&rolled(&records, &pilot(), &mut rolls, &mut firing)),
            Vec::<i16>::new()
        );
        assert!(firing.asked.is_empty(), "no second draw");
    }

    #[test]
    fn an_owned_outfit_is_for_sale_whatever_its_roll() {
        let mut chance = Scripted::default();
        let outfitter = rolled(
            &[buying(128, 50)],
            &owning(&[(128, 1)]),
            &mut DayRolls::default(),
            &mut chance,
        );
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(buy(&outfitter), Ok(()));
        assert!(chance.asked.is_empty(), "no roll asked");
    }

    #[test]
    fn an_outfit_owned_when_listed_stays_for_sale_after_the_last_is_sold() {
        let records = [buying(128, 50)];
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::default();
        rolled(&records, &owning(&[(128, 1)]), &mut rolls, &mut chance);
        let sold = rolled(&records, &pilot(), &mut rolls, &mut chance);
        assert_eq!(listed(&sold), [128]);
        assert_eq!(buy(&sold), Ok(()));
        assert!(chance.asked.is_empty());
        let next_landing = rolled(&records, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(listed(&next_landing), Vec::<i16>::new());
    }

    #[test]
    fn an_owned_outfit_of_buy_random_0_is_for_sale_until_none_is_owned() {
        let records = [buying(128, 0)];
        let mut rolls = DayRolls::default();
        let owned = rolled(&records, &owning(&[(128, 1)]), &mut rolls, &mut NeverFires);
        assert_eq!(listed(&owned), [128]);
        assert_eq!(buy(&owned), Ok(()));
        let sold = rolled(&records, &pilot(), &mut rolls, &mut NeverFires);
        assert_eq!(listed(&sold), Vec::<i16>::new());
    }

    #[test]
    fn an_owned_sell_anywhere_outfit_off_today_lists_sell_only() {
        let records = [OutfitRecord {
            flags: OutfitFlags::SELL_ANYWHERE,
            tech_level: 9,
            ..buying(128, 50)
        }];
        let mut chance = Scripted::default();
        let outfitter = rolled(
            &records,
            &owning(&[(128, 1)]),
            &mut DayRolls::default(),
            &mut chance,
        );
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(buy(&outfitter), Err(OutfitRefusal::NotForSale));
        assert_eq!(sell(&outfitter), Ok(()));
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn an_outfit_off_today_hides_no_higher_one() {
        let records = [
            OutfitRecord {
                disp_weight: 5,
                flags: OutfitFlags::HIDE_HIGHER,
                ..buying(129, 50)
            },
            OutfitRecord {
                disp_weight: 5,
                ..outfit(130, &[])
            },
        ];
        let off = rolled(
            &records,
            &pilot(),
            &mut DayRolls::default(),
            &mut NeverFires,
        );
        assert_eq!(listed(&off), [130]);
        let mut firing = Scripted::answering(&[true]);
        let on = rolled(&records, &pilot(), &mut DayRolls::default(), &mut firing);
        assert_eq!(listed(&on), [129]);
    }

    #[test]
    fn by_the_bible_an_outfit_of_buy_random_0_is_always_for_sale() {
        let records = [buying(128, 0), buying(129, -1)];
        let site = port();
        let bible = Shop {
            buy_random: RuleSource::Bible,
            ..shop(&records, &site)
        };
        let mut chance = Scripted::default();
        let outfitter = bible
            .outfitter(&pilot(), &mut DayRolls::default(), &mut chance)
            .expect("an outfitter");
        assert_eq!(listed(&outfitter), [128, 129]);
        assert!(chance.asked.is_empty());
        let engine = open_at(&records, &site, &pilot()).expect("an outfitter");
        assert_eq!(listed(&engine), Vec::<i16>::new());
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

    #[test]
    fn hide_unless_available_shows_an_available_outfit() {
        // Availability always holds until control bits exist.
        let gated = OutfitRecord {
            flags: OutfitFlags::HIDE_UNLESS_AVAILABLE,
            availability: "b9999".to_owned(),
            ..outfit(128, &[])
        };
        let outfitter = open(&[gated], &pilot());
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(row(&outfitter, 128).buy, Ok(()));
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
            free_mass(FAST, &NO_STANDARD, &map(&[(130, 1)]), &records),
            10
        );
        assert_eq!(
            free_mass(FAST, &NO_STANDARD, &map(&[(999, 4)]), &records),
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
                standard: &NO_STANDARD,
                site: &port(),
                fighter_room,
                buy_random: RuleSource::Engine,
                hardpoints: ROOMY,
                launchers: &NO_LAUNCHERS,
                launcher_sale: RuleSource::Engine,
                raised_max: RuleSource::Engine,
            }
            .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
            .expect("open")
        };
        assert_eq!(buy(&open_with(&room(128, 1), &pilot())), Ok(()));
        assert_eq!(
            buy(&open_with(&room(128, 0), &pilot())),
            Err(OutfitRefusal::BaysFull),
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
                buy(&open(std::slice::from_ref(&none), &pilot())),
                Err(OutfitRefusal::NoneAllowed),
                "{max}"
            );
            assert_eq!(
                buy(&open(&[none], &owning(&[(128, 1)]))),
                Err(OutfitRefusal::NoneAllowed),
                "{max}, one owned (granted)"
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
            standard: &NO_STANDARD,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            buy_random: RuleSource::Engine,
            hardpoints: ROOMY,
            launchers: &NO_LAUNCHERS,
            launcher_sale: RuleSource::Engine,
            raised_max: RuleSource::Engine,
        }
        .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
        .expect("open");
        assert_eq!(buy(&negative), Err(OutfitRefusal::NoExpansion));
        let massless = Shop {
            records: &[OutfitRecord {
                mass: 0,
                ..expansion.clone()
            }],
            fields: ShipFields { holds: -1, ..FAST },
            standard: &NO_STANDARD,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            buy_random: RuleSource::Engine,
            hardpoints: ROOMY,
            launchers: &NO_LAUNCHERS,
            launcher_sale: RuleSource::Engine,
            raised_max: RuleSource::Engine,
        }
        .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
        .expect("open");
        assert_eq!(buy(&massless), Ok(()));
        let empty_holds = Shop {
            records: std::slice::from_ref(&expansion),
            fields: ShipFields { holds: 0, ..FAST },
            standard: &NO_STANDARD,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            buy_random: RuleSource::Engine,
            hardpoints: ROOMY,
            launchers: &NO_LAUNCHERS,
            launcher_sale: RuleSource::Engine,
            raised_max: RuleSource::Engine,
        }
        .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
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

    // Guns and turrets.

    /// A massless gun outfit `id`, `Max` 10.
    fn gun(id: i16) -> OutfitRecord {
        OutfitRecord {
            flags: OutfitFlags::GUN,
            mass: 0,
            ..outfit(id, &[])
        }
    }

    /// A massless turret outfit `id`, `Max` 10.
    fn turret(id: i16) -> OutfitRecord {
        OutfitRecord {
            flags: OutfitFlags::TURRET,
            ..gun(id)
        }
    }

    fn hardpoints(guns: i16, turrets: i16) -> Hardpoints {
        Hardpoints { guns, turrets }
    }

    /// The outfitter at [`port`] on a ship of `hardpoints`.
    fn armed(records: &[OutfitRecord], pilot: &Pilot, hardpoints: Hardpoints) -> Outfitter {
        let site = port();
        Shop {
            hardpoints,
            ..shop(records, &site)
        }
        .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter")
    }

    fn buys(outfitter: &Outfitter, id: i16) -> Result<(), OutfitRefusal> {
        row(outfitter, id).buy
    }

    #[test]
    fn a_gun_is_refused_once_the_guns_owned_reach_max_gun() {
        let records = [gun(128), gun(129), outfit(130, &[])];
        let one = armed(&records, &owning(&[(128, 1)]), hardpoints(2, 0));
        assert_eq!((buys(&one, 128), buys(&one, 129)), (Ok(()), Ok(())));
        let limit = Err(OutfitRefusal::GunLimit);
        for owned in [&[(128, 1), (129, 1)][..], &[(128, 2)], &[(129, 3)]] {
            let full = armed(&records, &owning(owned), hardpoints(2, 0));
            assert_eq!((buys(&full, 128), buys(&full, 129)), (limit, limit));
            assert_eq!(buys(&full, 130), Ok(()), "not a gun");
            let sold = owned[0].0;
            assert_eq!(row(&full, sold).sell, Ok(()), "selling is free");
        }
        let turrets = armed(&[turret(128)], &owning(&[(128, 1)]), hardpoints(1, 9));
        assert_eq!(buys(&turrets, 128), Ok(()), "turrets are not guns");
    }

    #[test]
    fn a_class_whose_max_gun_is_none_or_less_buys_no_gun() {
        for max_gun in [0, -1] {
            let outfitter = armed(&[gun(128)], &pilot(), hardpoints(max_gun, 5));
            assert_eq!(buys(&outfitter, 128), Err(OutfitRefusal::GunLimit));
        }
        let outfitter = armed(&[gun(128)], &pilot(), hardpoints(1, 0));
        assert_eq!(buys(&outfitter, 128), Ok(()));
    }

    #[test]
    fn a_turret_is_refused_once_the_turrets_owned_reach_max_tur() {
        let records = [turret(128), turret(129), gun(130)];
        let one = armed(&records, &owning(&[(128, 1)]), hardpoints(0, 2));
        assert_eq!((buys(&one, 128), buys(&one, 129)), (Ok(()), Ok(())));
        let full = armed(&records, &owning(&[(128, 1), (129, 1)]), hardpoints(5, 2));
        let limit = Err(OutfitRefusal::TurretLimit);
        assert_eq!((buys(&full, 128), buys(&full, 129)), (limit, limit));
        assert_eq!(buys(&full, 130), Ok(()), "guns are not turrets");
        for max_tur in [0, -1] {
            let none = armed(&records, &pilot(), hardpoints(5, max_tur));
            assert_eq!(buys(&none, 128), limit);
        }
    }

    #[test]
    fn mod_type_45_changes_max_gun_once_per_outfit_by_its_first_mod() {
        let records = [
            gun(128),
            outfit(140, &[(MORE_GUNS, 1), (MORE_GUNS, 5)]),
            outfit(141, &[(MORE_GUNS, -1)]),
            outfit(142, &[(MORE_SPEED, 1), (MORE_GUNS, 2)]),
            outfit(143, &[(MORE_TURRETS, 7)]),
        ];
        let limit = Err(OutfitRefusal::GunLimit);
        let at = |owned: &[(i16, u16)], max_gun| {
            buys(
                &armed(&records, &owning(owned), hardpoints(max_gun, 0)),
                128,
            )
        };
        assert_eq!(at(&[(128, 1), (140, 3)], 1), Ok(()), "1 + 1");
        assert_eq!(at(&[(128, 2), (140, 3)], 1), limit, "not 1 + 3, nor + 5");
        assert_eq!(at(&[(128, 1), (141, 1)], 2), limit, "2 - 1");
        assert_eq!(at(&[], 0), limit, "an unowned outfit adds nothing");
        assert_eq!(at(&[(140, 0)], 0), limit, "nor one owned none of");
        assert_eq!(at(&[(142, 1)], 0), Ok(()), "its first mod of 45");
        assert_eq!(at(&[(128, 1), (142, 1)], 0), Ok(()), "0 + 2, 1 owned");
        assert_eq!(at(&[(128, 2), (142, 1)], 0), limit, "0 + 2, 2 owned");
        assert_eq!(at(&[(128, 2), (142, 1)], 1), Ok(()), "1 + 2");
        assert_eq!(at(&[(128, 3), (142, 1)], 1), limit);
        assert_eq!(at(&[(143, 1)], 0), limit, "46 is turrets");
    }

    #[test]
    fn mod_type_46_changes_max_tur() {
        let records = [
            turret(128),
            outfit(140, &[(MORE_TURRETS, 1), (MORE_TURRETS, 5)]),
            outfit(141, &[(MORE_TURRETS, -1)]),
            outfit(143, &[(MORE_GUNS, 7)]),
        ];
        let limit = Err(OutfitRefusal::TurretLimit);
        let at = |owned: &[(i16, u16)], max_tur| {
            buys(
                &armed(&records, &owning(owned), hardpoints(0, max_tur)),
                128,
            )
        };
        assert_eq!(at(&[(128, 1), (140, 3)], 1), Ok(()), "1 + 1");
        assert_eq!(at(&[(128, 2), (140, 3)], 1), limit, "once per outfit");
        assert_eq!(at(&[(128, 1), (141, 1)], 2), limit, "2 - 1");
        assert_eq!(at(&[(143, 1)], 0), limit, "45 is guns");
    }

    #[test]
    fn an_outfit_flagged_gun_and_turret_must_fit_both_gun_first() {
        let both = OutfitRecord {
            flags: OutfitFlags::GUN | OutfitFlags::TURRET,
            ..gun(128)
        };
        let records = [both, gun(129), turret(130)];
        let at =
            |owned: &[(i16, u16)]| buys(&armed(&records, &owning(owned), hardpoints(1, 1)), 128);
        assert_eq!(at(&[]), Ok(()));
        assert_eq!(at(&[(129, 1)]), Err(OutfitRefusal::GunLimit));
        assert_eq!(at(&[(130, 1)]), Err(OutfitRefusal::TurretLimit));
        assert_eq!(at(&[(129, 1), (130, 1)]), Err(OutfitRefusal::GunLimit));
        assert_eq!(at(&[(128, 1)]), Err(OutfitRefusal::GunLimit), "itself both");
    }

    #[test]
    fn max_comes_before_the_gun_limit_and_the_gun_limit_before_space_and_cash() {
        let one = OutfitRecord { max: 1, ..gun(128) };
        let outfitter = armed(&[one], &owning(&[(128, 1)]), hardpoints(1, 0));
        assert_eq!(buys(&outfitter, 128), Err(OutfitRefusal::MaxOwned));
        let none = OutfitRecord { max: 0, ..gun(128) };
        let outfitter = armed(&[none], &pilot(), hardpoints(0, 0));
        assert_eq!(buys(&outfitter, 128), Err(OutfitRefusal::NoneAllowed));
        let dear = OutfitRecord {
            mass: 100,
            cost: 1_000_000,
            ..gun(128)
        };
        let outfitter = armed(std::slice::from_ref(&dear), &pilot(), hardpoints(0, 0));
        assert_eq!(buys(&outfitter, 128), Err(OutfitRefusal::GunLimit));
        let outfitter = armed(&[dear], &pilot(), hardpoints(1, 0));
        assert_eq!(buys(&outfitter, 128), Err(OutfitRefusal::NoSpaceForAny));
        let turret = OutfitRecord {
            mass: 100,
            ..turret(128)
        };
        let outfitter = armed(&[turret], &pilot(), hardpoints(0, 0));
        assert_eq!(buys(&outfitter, 128), Err(OutfitRefusal::TurretLimit));
    }

    // A launcher and its ammunition.

    /// Launcher outfit 128 (massless, "missile rack"/"missile racks") and
    /// its ammunition, outfit 129 ("missile"/"missiles").
    fn racks() -> [OutfitRecord; 2] {
        [
            OutfitRecord {
                lc_name: "missile rack".to_owned(),
                lc_plural: "missile racks".to_owned(),
                ..gun(128)
            },
            OutfitRecord {
                lc_name: "missile".to_owned(),
                lc_plural: "missiles".to_owned(),
                max: 999,
                ..outfit(129, &[])
            },
        ]
    }

    fn rack(max_ammo: u32, rounds: u32) -> BTreeMap<OutfitId, Launcher> {
        BTreeMap::from([(
            OutfitId(128),
            Launcher {
                max_ammo,
                rounds,
                ammo: Some(OutfitId(129)),
            },
        )])
    }

    /// The outfitter at [`port`] with `launchers`, read as `source` says.
    fn launching(
        records: &[OutfitRecord],
        pilot: &Pilot,
        launchers: &BTreeMap<OutfitId, Launcher>,
        source: RuleSource,
    ) -> Outfitter {
        let site = port();
        Shop {
            launchers,
            launcher_sale: source,
            ..shop(records, &site)
        }
        .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter")
    }

    fn first(rounds: u32) -> Result<(), OutfitRefusal> {
        Err(OutfitRefusal::AmmunitionFirst {
            rounds,
            ammo: Some(OutfitId(129)),
        })
    }

    #[test]
    fn by_the_engine_selling_the_last_launcher_is_refused_while_its_rounds_are_held() {
        let engine = |owned, max_ammo, rounds| {
            sell(&launching(
                &racks(),
                &owning(&[(128, owned)]),
                &rack(max_ammo, rounds),
                RuleSource::Engine,
            ))
        };
        assert_eq!(engine(1, 4, 2), first(2));
        assert_eq!(engine(1, 4, 0), Ok(()));
        let outfitter = launching(
            &racks(),
            &owning(&[(128, 1)]),
            &rack(4, 2),
            RuleSource::Engine,
        );
        assert_eq!(buy(&outfitter), Ok(()), "buying is free");
    }

    #[test]
    fn by_the_engine_a_launcher_sells_while_the_others_hold_every_round() {
        let engine = |owned, rounds| {
            sell(&launching(
                &racks(),
                &owning(&[(128, owned)]),
                &rack(4, rounds),
                RuleSource::Engine,
            ))
        };
        assert_eq!(engine(2, 4), Ok(()));
        assert_eq!(engine(2, 5), first(1));
        assert_eq!(engine(3, 8), Ok(()));
        assert_eq!(engine(3, 11), first(3));
    }

    #[test]
    fn by_the_engine_a_launcher_of_max_ammo_0_always_sells() {
        let outfitter = launching(
            &racks(),
            &owning(&[(128, 1)]),
            &rack(0, 7),
            RuleSource::Engine,
        );
        assert_eq!(sell(&outfitter), Ok(()));
    }

    #[test]
    fn by_the_other_reading_any_round_held_refuses_the_sale() {
        let bible = |owned, max_ammo, rounds| {
            sell(&launching(
                &racks(),
                &owning(&[(128, owned)]),
                &rack(max_ammo, rounds),
                RuleSource::Bible,
            ))
        };
        assert_eq!(bible(2, 4, 4), first(4));
        assert_eq!(bible(1, 0, 1), first(1));
        assert_eq!(bible(1, 4, 2), first(2));
        assert_eq!(bible(1, 4, 0), Ok(()));
    }

    #[test]
    fn a_launchers_excess_is_read_as_its_rule_says() {
        let excess = |max_ammo, rounds, owned, source| {
            Launcher {
                max_ammo,
                rounds,
                ammo: None,
            }
            .excess(owned, source)
        };
        for (max_ammo, rounds, owned, engine, bible) in [
            (4, 2, 1, Some(2), Some(2)),
            (4, 0, 1, None, None),
            (4, 4, 2, None, Some(4)),
            (4, 5, 2, Some(1), Some(5)),
            (0, 1, 1, None, Some(1)),
            (0, 0, 1, None, None),
            (4, 9, 3, Some(1), Some(9)),
            (4, 8, 3, None, Some(8)),
            (4, 3, 0, Some(3), Some(3)),
            (u32::MAX, u32::MAX, u16::MAX, None, Some(u32::MAX)),
            (4, u32::MAX, 1, Some(u32::MAX), Some(u32::MAX)),
            (u32::MAX, 1, 2, None, Some(1)),
        ] {
            let case = format!("{max_ammo} {rounds} {owned}");
            assert_eq!(
                excess(max_ammo, rounds, owned, RuleSource::Engine),
                engine,
                "{case}"
            );
            assert_eq!(
                excess(max_ammo, rounds, owned, RuleSource::Bible),
                bible,
                "{case}"
            );
        }
    }

    #[test]
    fn negative_free_mass_and_cannot_sell_come_before_the_ammunition() {
        let [launcher, ammo] = racks();
        let unsellable = OutfitRecord {
            flags: OutfitFlags::CANNOT_SELL,
            ..launcher.clone()
        };
        let outfitter = launching(
            &[unsellable, ammo.clone()],
            &owning(&[(128, 1)]),
            &rack(4, 2),
            RuleSource::Engine,
        );
        assert_eq!(sell(&outfitter), Err(OutfitRefusal::CannotSell));
        let expansion = OutfitRecord {
            mass: -20,
            ..launcher
        };
        let cargo = OutfitRecord {
            mass: 40,
            ..outfit(130, &[])
        };
        let outfitter = launching(
            &[expansion, ammo, cargo],
            &owning(&[(128, 1), (130, 1)]),
            &rack(4, 2),
            RuleSource::Engine,
        );
        assert_eq!(sell(&outfitter), Err(OutfitRefusal::NegativeFreeMass));
    }

    #[test]
    fn the_outfitter_carries_the_names_a_launcher_refusal_needs() {
        let refused = launching(
            &racks(),
            &owning(&[(128, 1)]),
            &rack(4, 2),
            RuleSource::Engine,
        );
        let names = |singular: &str, plural: &str| LcNames {
            singular: singular.to_owned(),
            plural: plural.to_owned(),
        };
        assert_eq!(
            refused.lc_names,
            BTreeMap::from([
                (OutfitId(128), names("missile rack", "missile racks")),
                (OutfitId(129), names("missile", "missiles")),
            ])
        );
        let sold = launching(
            &racks(),
            &owning(&[(128, 1)]),
            &rack(4, 0),
            RuleSource::Engine,
        );
        assert_eq!(sold.lc_names, BTreeMap::new(), "none refused");
        let unnamed = BTreeMap::from([(
            OutfitId(128),
            Launcher {
                max_ammo: 4,
                rounds: 2,
                ammo: None,
            },
        )]);
        let outfitter = launching(&racks(), &owning(&[(128, 1)]), &unnamed, RuleSource::Engine);
        assert_eq!(
            sell(&outfitter),
            Err(OutfitRefusal::AmmunitionFirst {
                rounds: 2,
                ammo: None
            })
        );
        assert_eq!(
            outfitter.lc_names,
            BTreeMap::from([(OutfitId(128), names("missile rack", "missile racks"))])
        );
    }

    // ModType 27: raised maximums.

    /// Target B (200, `Max` 4, "widget"/"widgets"), raisers A (201) and
    /// C (202) each naming it once, and D (203) naming it twice; E (204,
    /// `Max` 1, "gizmo"/"gizmos") is named by no one, and F (205) raises
    /// another outfit.
    fn raisers() -> Vec<OutfitRecord> {
        vec![
            OutfitRecord {
                max: 4,
                lc_name: "widget".to_owned(),
                lc_plural: "widgets".to_owned(),
                ..outfit(200, &[])
            },
            OutfitRecord {
                lc_name: "widget rack".to_owned(),
                lc_plural: "widget racks".to_owned(),
                ..outfit(201, &[(RAISES_MAX, 200)])
            },
            outfit(202, &[(MORE_SPEED, 1), (RAISES_MAX, 200)]),
            outfit(203, &[(RAISES_MAX, 200), (RAISES_MAX, 200)]),
            OutfitRecord {
                max: 1,
                lc_name: "gizmo".to_owned(),
                lc_plural: "gizmos".to_owned(),
                ..outfit(204, &[])
            },
            outfit(205, &[(RAISES_MAX, 204)]),
        ]
    }

    fn owned_map(owned: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        owned.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    /// B's `Max` as `source` raises it, owning `owned`, among `records`.
    fn max_of(records: &[OutfitRecord], owned: &[(i16, u16)], source: RuleSource) -> i64 {
        raised_max(&records[0], &owned_map(owned), records, source)
    }

    #[test]
    fn a_target_max_is_multiplied_by_the_raisers_owned_at_least_once() {
        let records = raisers();
        for source in RuleSource::ALL {
            let at = |owned: &[(i16, u16)]| max_of(&records, owned, source);
            assert_eq!(at(&[]), 4, "no raiser: unchanged");
            assert_eq!(at(&[(201, 1)]), 4, "one raiser: x 1");
            assert_eq!(at(&[(201, 2)]), 8, "two: doubled");
            assert_eq!(at(&[(201, 3)]), 12);
            assert_eq!(at(&[(201, 1), (202, 1)]), 8, "one A and one C");
            assert_eq!(at(&[(201, 0), (202, 2)]), 8, "an A owned none of");
            assert_eq!(at(&[(205, 3)]), 4, "a raiser of another outfit");
            assert_eq!(at(&[(200, 9)]), 4, "the target itself");
        }
    }

    #[test]
    fn by_the_engine_each_mod_naming_the_target_counts_and_by_the_bible_each_item() {
        let records = raisers();
        assert_eq!(max_of(&records, &[(203, 1)], RuleSource::Engine), 8);
        assert_eq!(max_of(&records, &[(203, 1)], RuleSource::Bible), 4);
        assert_eq!(max_of(&records, &[(203, 2)], RuleSource::Engine), 16);
        assert_eq!(max_of(&records, &[(203, 2)], RuleSource::Bible), 8);
    }

    #[test]
    fn a_max_of_none_or_less_is_not_raised() {
        for max in [0, -1] {
            let mut records = raisers();
            records[0].max = max;
            for source in RuleSource::ALL {
                assert_eq!(
                    max_of(&records, &[(201, 2)], source),
                    i64::from(max) * 2,
                    "{max} {source:?}"
                );
            }
        }
    }

    #[test]
    fn a_raised_max_saturates() {
        let records = [
            OutfitRecord {
                max: i16::MAX,
                ..outfit(200, &[])
            },
            outfit(201, &[(RAISES_MAX, 200); 4]),
        ];
        let owned = &[(201, u16::MAX)];
        assert_eq!(
            max_of(&records, owned, RuleSource::Engine),
            i64::from(i16::MAX) * i64::from(u16::MAX) * 4
        );
        assert_eq!(
            max_of(&records, owned, RuleSource::Bible),
            i64::from(i16::MAX) * i64::from(u16::MAX)
        );
    }

    #[test]
    fn an_owned_mod_type_27_outfit_raises_its_targets_max_in_the_buy_check() {
        let records = raisers();
        let at = |owned: &[(i16, u16)]| buys(&open(&records, &owning(owned)), 200);
        assert_eq!(at(&[(200, 3), (201, 1)]), Ok(()));
        assert_eq!(at(&[(200, 4), (201, 1)]), Err(OutfitRefusal::MaxOwned));
        assert_eq!(at(&[(200, 4), (201, 2)]), Ok(()), "raised to 8");
        assert_eq!(at(&[(200, 7), (201, 2)]), Ok(()));
        assert_eq!(at(&[(200, 8), (201, 2)]), Err(OutfitRefusal::MaxOwned));
        assert_eq!(at(&[(200, 8), (203, 1)]), Err(OutfitRefusal::MaxOwned));
        assert_eq!(at(&[(200, 7), (203, 1)]), Ok(()), "by the engine, 8");
    }

    #[test]
    fn the_buy_check_reads_the_raised_max_as_its_rule_says() {
        let records = raisers();
        let site = port();
        let bible = Shop {
            raised_max: RuleSource::Bible,
            ..shop(&records, &site)
        };
        let outfitter = |owned: &[(i16, u16)]| {
            bible
                .outfitter(&owning(owned), &mut DayRolls::default(), &mut NeverFires)
                .expect("open")
        };
        assert_eq!(
            buys(&outfitter(&[(200, 4), (203, 1)]), 200),
            Err(OutfitRefusal::MaxOwned),
            "one item, two mods: x 1"
        );
        assert_eq!(buys(&outfitter(&[(200, 4), (203, 2)]), 200), Ok(()));
    }

    #[test]
    fn a_raised_max_still_refuses_a_target_of_max_none() {
        let mut records = raisers();
        for max in [0, -1] {
            records[0].max = max;
            let outfitter = open(&records, &owning(&[(201, 2)]));
            assert_eq!(buys(&outfitter, 200), Err(OutfitRefusal::NoneAllowed));
        }
    }

    #[test]
    fn the_raised_max_comes_before_the_gun_limit() {
        let mut records = raisers();
        records[0].flags = OutfitFlags::GUN;
        let at =
            |owned: &[(i16, u16)]| buys(&armed(&records, &owning(owned), hardpoints(7, 0)), 200);
        assert_eq!(at(&[(200, 8), (201, 2)]), Err(OutfitRefusal::MaxOwned));
        assert_eq!(at(&[(200, 7), (201, 2)]), Err(OutfitRefusal::GunLimit));
        assert_eq!(at(&[(200, 6), (201, 2)]), Ok(()));
    }

    #[test]
    fn a_fighter_room_of_none_refuses_whatever_the_raised_max() {
        let records = raisers();
        let site = port();
        let room = |n: u32| BTreeMap::from([(OutfitId(200), n)]);
        let at = |fighter_room: &BTreeMap<OutfitId, u32>| {
            let outfitter = Shop {
                fighter_room,
                ..shop(&records, &site)
            }
            .outfitter(
                &owning(&[(200, 4), (201, 2)]),
                &mut DayRolls::default(),
                &mut NeverFires,
            )
            .expect("open");
            buys(&outfitter, 200)
        };
        assert_eq!(at(&room(0)), Err(OutfitRefusal::BaysFull));
        assert_eq!(at(&room(1)), Ok(()));
    }

    /// The outfitter at [`port`] owning `owned` of `records`, the raised
    /// `Max` read as `source` says.
    fn raising(records: &[OutfitRecord], owned: &[(i16, u16)], source: RuleSource) -> Outfitter {
        let site = port();
        Shop {
            raised_max: source,
            ..shop(records, &site)
        }
        .outfitter(&owning(owned), &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter")
    }

    fn raised_first(count: u32, target: i16) -> Result<(), OutfitRefusal> {
        Err(OutfitRefusal::RaisedFirst {
            count,
            target: OutfitId(target),
        })
    }

    #[test]
    fn by_the_engine_a_raiser_cannot_be_sold_while_its_target_overfills_the_rest() {
        let records = raisers();
        let at =
            |owned: &[(i16, u16)]| row(&raising(&records, owned, RuleSource::Engine), 201).sell;
        assert_eq!(at(&[(201, 2), (200, 6)]), raised_first(2, 200));
        assert_eq!(at(&[(201, 2), (200, 4)]), Ok(()));
        assert_eq!(at(&[(201, 1), (200, 1)]), raised_first(1, 200), "no floor");
        assert_eq!(at(&[(201, 1)]), Ok(()));
        assert_eq!(
            at(&[(201, 2), (202, 1), (200, 6)]),
            raised_first(2, 200),
            "C ignored"
        );
        assert_eq!(at(&[(201, 3), (200, 9)]), raised_first(1, 200));
        assert_eq!(at(&[(201, 3), (200, 8)]), Ok(()));
    }

    #[test]
    fn by_the_bible_a_raiser_sells_while_the_targets_maximum_after_the_sale_holds_them() {
        let records = raisers();
        let at = |owned: &[(i16, u16)]| row(&raising(&records, owned, RuleSource::Bible), 201).sell;
        assert_eq!(at(&[(201, 1), (200, 3)]), Ok(()), "4 after");
        assert_eq!(at(&[(201, 1), (200, 4)]), Ok(()));
        assert_eq!(at(&[(201, 1), (200, 5)]), raised_first(1, 200));
        assert_eq!(
            at(&[(201, 2), (202, 1), (200, 6)]),
            Ok(()),
            "A and C: 8 after"
        );
        assert_eq!(at(&[(201, 2), (202, 1), (200, 9)]), raised_first(1, 200));
        assert_eq!(at(&[(201, 3), (200, 9)]), raised_first(1, 200));
        assert_eq!(at(&[(201, 3), (200, 8)]), Ok(()));
    }

    #[test]
    fn the_first_refusing_mod_names_the_target() {
        let mut records = raisers();
        records.push(outfit(206, &[(RAISES_MAX, 200), (RAISES_MAX, 204)]));
        // By the engine none is left of either; by the Bible, their `Max`.
        for (source, second, both) in [(RuleSource::Engine, 2, 6), (RuleSource::Bible, 1, 2)] {
            let at = |owned: &[(i16, u16)]| row(&raising(&records, owned, source), 206).sell;
            assert_eq!(
                at(&[(206, 1), (204, 2)]),
                raised_first(second, 204),
                "{source:?}"
            );
            assert_eq!(
                at(&[(206, 1), (200, 6), (204, 2)]),
                raised_first(both, 200),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_raiser_of_an_unknown_target_sells() {
        let records = [outfit(207, &[(RAISES_MAX, 999)])];
        for source in RuleSource::ALL {
            let outfitter = raising(&records, &[(207, 1)], source);
            assert_eq!(row(&outfitter, 207).sell, Ok(()), "{source:?}");
            assert_eq!(outfitter.lc_names, BTreeMap::new());
        }
    }

    #[test]
    fn negative_free_mass_comes_before_a_raised_max_and_it_before_the_ammunition() {
        let [launcher, ammo] = racks();
        let launcher = OutfitRecord {
            mods: [(RAISES_MAX, 200), (0, 0), (0, 0), (0, 0)],
            ..launcher
        };
        let target = raisers().swap_remove(0);
        let site = port();
        let records = [launcher.clone(), ammo.clone(), target.clone()];
        let launchers = rack(4, 2);
        let outfitter = Shop {
            launchers: &launchers,
            ..shop(&records, &site)
        }
        .outfitter(
            &owning(&[(128, 1), (200, 1)]),
            &mut DayRolls::default(),
            &mut NeverFires,
        )
        .expect("open");
        assert_eq!(sell(&outfitter), raised_first(1, 200));
        let expansion = OutfitRecord {
            mass: -20,
            ..launcher
        };
        let cargo = OutfitRecord {
            mass: 40,
            ..outfit(130, &[])
        };
        let records = [expansion, ammo, target, cargo];
        let outfitter = Shop {
            launchers: &launchers,
            ..shop(&records, &site)
        }
        .outfitter(
            &owning(&[(128, 1), (200, 1), (130, 1)]),
            &mut DayRolls::default(),
            &mut NeverFires,
        )
        .expect("open");
        assert_eq!(sell(&outfitter), Err(OutfitRefusal::NegativeFreeMass));
    }

    #[test]
    fn the_outfitter_carries_the_names_a_raised_max_refusal_needs() {
        let records = raisers();
        let names = |singular: &str, plural: &str| LcNames {
            singular: singular.to_owned(),
            plural: plural.to_owned(),
        };
        let refused = raising(&records, &[(201, 1), (200, 1)], RuleSource::Engine);
        assert_eq!(
            refused.lc_names,
            BTreeMap::from([
                (OutfitId(200), names("widget", "widgets")),
                (OutfitId(201), names("widget rack", "widget racks")),
            ])
        );
        let sold = raising(&records, &[(201, 2), (200, 1)], RuleSource::Engine);
        assert_eq!(sold.lc_names, BTreeMap::new(), "none refused");
    }

    // Settling.

    #[test]
    fn a_buy_pays_and_adds_one() {
        let mut pilot = pilot();
        settle(&mut pilot, &heavy(), Direction::Buy, 4000);
        assert_eq!((pilot.cash, pilot.owned(OutfitId(128))), (6000, 1));
        settle(&mut pilot, &heavy(), Direction::Buy, 4000);
        assert_eq!((pilot.cash, pilot.owned(OutfitId(128))), (2000, 2));
    }

    #[test]
    fn a_buy_removed_after_purchase_only_pays() {
        let permit = OutfitRecord {
            flags: OutfitFlags::REMOVE_AFTER_PURCHASE,
            ..heavy()
        };
        let mut pilot = pilot();
        settle(&mut pilot, &permit, Direction::Buy, 4000);
        assert_eq!(pilot.cash, 6000);
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
