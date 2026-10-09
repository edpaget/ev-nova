//! The shipyard: which ship classes (`shïp`) a stellar lists and sells, at
//! what price, what the player's ship trades in for, and buying a new one.
//!
//! # What a stellar sells
//!
//! A ship is *for sale* at a stellar with a shipyard when its `TechLevel`
//! is allowed there and its `BuyRandom` roll holds today. Its `TechLevel`
//! is allowed at or below the stellar's, or where one of the eight
//! `SpecialTech` slots equals it exactly, each compared raw
//! (`_SetupPortAvailableShipTypes` @0xbc5c-0xbc72). By the engine a class
//! of `TechLevel` below 0 is never sold by tech (@0xbc4b-0xbc52), and one
//! of 0x7fff is sold like any other ([`wares::ship_tech_allows`]); by the
//! Bible a negative one is sold too
//! ([`RuleKey::TechLevel`],
//! [`wares::by_tech_level`]). The Bible's `shïp` gives `BuyRandom` as
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
//! bought or declined. The build settles a class's `Require` and
//! `Availability` before its roll and tests `HideHigher` after it, so a
//! class whose `Require` is unmet, or one `HideHigher` takes off sale,
//! still draws; the build loop draws, and no rule does. The original builds the list once each time the
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
//! flagged [`ShipFlags3::HIDE_HIGHER`] that is listed, greyed or not (for
//! the class flown, the cash, its `Require` or its `Availability`), takes
//! every higher-numbered ship of the same `DispWeight` off sale
//! (`_SetupPortAvailableShipTypes` @0xbdc3-0xbe2c). The ship class
//! flown is listed like any other, but cannot be bought: `_CalcShipCanBuy`
//! compares the class selected with the player's own, and does so first,
//! before the cash, `Require` and `Availability` (@0x4f6f5-0x4f6fc). It
//! reads no name, so a ship bought under any name is still its class, and
//! another class of the same name still sells. Rows go by `DispWeight`,
//! highest first, then by ID.
//!
//! # Price and trade-in
//!
//! A ship's price is its `Cost`, never below none. The player's ship trades
//! in for [`TRADE_IN_PERCENT`] of its class's `Cost`, plus half of each
//! outfit line it carries (count × price), except a persistent one, which
//! the player keeps, the total never below none ([`trade_in`]). By the
//! engine an outfit that cannot be sold counts too; by the Bible it does
//! not ([`RuleKey::TradeInOutfits`](crate::RuleKey::TradeInOutfits)). The
//! final price is the price less
//! the trade-in, and may be negative: the player is then paid. A buy is
//! refused when the ship is the class flown, when it cannot be bought
//! here, or when its final price, taken as none when it is negative, is
//! more than the cash. The comparison is signed, so a pilot in debt can
//! buy no ship, even one whose trade-in would cover the price and the
//! debt (`_CalcShipCanBuy` @0x4f7d8-0x4f7eb).
//!
//! # Buying a ship
//!
//! Buying replaces the ship class flown, and the cash changes by the
//! trade-in less the price. Every outfit that is not persistent
//! ([`OutfitFlags::PERSISTENT`], `oütf` flag 0x0004: "This item stays with
//! you when you trade ships") goes with the old ship, its value in the
//! trade-in. What the persistent ones do follows the rulebook's
//! [`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry)
//! (`purchase_carry`). By the engine, every persistent outfit carries
//! over whole, whatever its `Max` or mass, and none is sold back:
//! `_DoShipyardDialog` zeroes only the outfits whose record byte +0x378,
//! `Flags` 0x0004 alone, is clear (@0x5ee1b-0x5ee4f), so the new ship may
//! have less than no free mass. By the other reading, the port's earlier
//! carry, each carries over as far as the new ship allows: first no more
//! than its `Max`; then, by ascending ID, one at a time while the new ship
//! still has free mass for it with its own default items fitted (they are
//! fitted regardless, so their mass is not room); every one that does not
//! carry over is sold back at the outfitter's rate, priced on the old
//! ship, and the refund is credited on top (it does not count towards
//! affording the ship). A mass expansion carries onto a ship whose
//! `Holds` is negative too: the purchase never reads the class's
//! no-expansions flag (@0x5ee1b-0x5ee4f), which only the outfitter does.
//! The new ship's default
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
//! escorts the fleet's holds count ([`escort_tons`], @0xce08-0xcf0d), each
//! a negative one's size (`_LoadObjectData` negates it, @0x7a6a5-0x7a6d3),
//! with no 32000 cap; f = A/B in doubles (@0xcf27-0xcf37), so B is never
//! below A and f never above 1 (the engine's `minsd` cap at 1, @0xcf33,
//! never bites), and none
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
//! ship comes with its shield, armour and fuel full, as a new hull (the
//! Bible is silent here too), once its `OnPurchase` has run: each is filled
//! to the most of the class and outfits `OnPurchase` leaves
//! (`_DoShipyardDialog` @0x5f051-0x5f07b, after @0x5f00d), so an
//! `OnPurchase` that removes an outfit or changes the class still leaves
//! them full. Everything else about the pilot (where it is,
//! the date, the course, its legal records and the events under way)
//! stays as it was.
//!
//! # The rules
//!
//! The listing reads value rules over each class as the shipyard is built
//! (`required`, `available`, `day_roll` on
//! [`RuleKey::BuyRandom`](crate::RuleKey::BuyRandom), `price`) and over
//! the ship flown (`trade_in_value` on
//! [`RuleKey::TradeInOutfits`](crate::RuleKey::TradeInOutfits)), and
//! settles in each row's [`ShipFacts`] whether the class can be bought
//! here at all. [`Shipyard::can`] answers from the checks list `can_buy`:
//! "flown", "for sale", then "cash". A purchase is worked out as a value
//! (`purchase`, over a `Sale`, the pilot as `OnRetire` left it): `carried`
//! on [`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry) (the
//! persistent outfits carried and sold back, with the refund),
//! `cash_after`, `cargo_kept` on
//! [`RuleKey::PurchaseCargo`](crate::RuleKey::PurchaseCargo); it writes
//! nothing, and the session applies it. The reserves are not part of it:
//! once `OnPurchase` has run, the session reads `reserves_full` over the
//! stats it leaves ([`filled`]). No rule draws.
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
//! @0x7ad8d), so they take no room, and by the other reading of
//! `purchase_carry` a persistent outfit carries over only while it fits
//! with them fitted. Owned, they trade in as any other
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
use crate::chance::{Chance, NeverFires};
use crate::combat::armament::{
    Arsenal, MOD_AMMO, MOD_WEAPON, StockFit, fit_stock, fitted, holder, holders,
};
use crate::control::Gate;
use crate::handling::ShipFields;
use crate::landing::StellarFlags;
use crate::market::{EscortHolds, Good, escort_tons, fleet_holds};
use crate::outfitter::{OutfitFlags, free_mass, outfit_mods, resale, unit_mass, unit_price};
use crate::pilot::{Pilot, merged, tally};
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::rules::{Check, Checks, Context, Value};
use crate::ship_change::{OutfitCarry, carried_outfits};
use crate::stats::ShipStats;
use crate::wares::{self, DayRolls, HideBits, HideHigher, Roll, ShownTons};

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
    /// The ship is the class the player flies: `_CalcShipCanBuy` refuses
    /// it (@0x4f6f5-0x4f6fc), whatever the ship is named.
    AlreadyFlown,
    /// The ship is listed but cannot be bought here: its `Require` is not
    /// met, or its `Availability` does not hold.
    NotForSale,
    /// Its final price, at least none, is more than the cash: with the
    /// cash below 0, whatever the trade-in (`_CalcShipCanBuy`
    /// @0x4f7d8-0x4f7eb).
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
    /// What the shipyard's rules read about the class, settled when the
    /// shipyard was built.
    pub facts: ShipFacts,
}

/// What the shipyard's rules read about one ship class, settled when the
/// shipyard is built: whatever needs the gate or the day's roll is worked
/// out then, so the rules answer from these alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipFacts {
    /// Whether it can be bought here at all: for sale here today (its
    /// tech level, the day's roll and `HideHigher` as listed), its
    /// `Require` met and its `Availability` holding (see the module docs).
    pub buyable: bool,
}

/// What the shipyard's Info shows of a ship class: from its record, and
/// its Space worked out from its stock.
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
    /// The "Shipyard Info" panel's Space number, worked out when the list
    /// is built ([`space`]).
    pub space: i64,
}

impl ShipSpecs {
    /// The specs of class `ship`, its Space worked out from its stock in
    /// `arsenal` and `outfits`.
    pub(crate) fn of(ship: &ShipRecord, arsenal: &Arsenal, outfits: &[OutfitRecord]) -> Self {
        Self {
            fields: ship.fields,
            max_gun: ship.max_gun,
            max_tur: ship.max_tur,
            length: ship.length,
            crew: ship.crew,
            space: space(ship, arsenal, outfits),
        }
    }

    /// The panel's "Space:" number with its unit: "ton" at exactly 1, else
    /// "tons" (`_ShipDescDialogUpdate` @0x50366-0x5037e).
    #[must_use]
    pub fn space_shown(&self) -> ShownTons {
        ShownTons::of(self.space)
    }

    /// The panel's "Cargo:" number with its unit: |`Holds`|, as the loader
    /// negates a negative one (`_LoadObjectData` @0x7a6ad-0x7a6b8), "ton"
    /// at exactly 1, else "tons" (`_ShipDescDialogUpdate`
    /// @0x503ea-0x50446).
    #[must_use]
    pub fn cargo_shown(&self) -> ShownTons {
        ShownTons::of(i64::from(self.fields.holds).abs())
    }

    /// The panel's "Mass:" number with its unit: `Mass`, "ton" at exactly
    /// 1, else "tons" (`_ShipDescDialogUpdate` @0x50e05-0x50e63).
    #[must_use]
    pub fn mass_shown(&self) -> ShownTons {
        ShownTons::of(i64::from(self.fields.mass))
    }
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
    /// The rule set the shipyard was built by, which its questions read.
    pub rules: Rulebook,
}

impl Shipyard {
    /// `ship`'s row, if it is listed.
    #[must_use]
    pub fn row(&self, ship: ShipId) -> Option<&ShipRow> {
        self.rows.iter().find(|row| row.id == ship)
    }

    /// Whether `ship` can be bought now, or why not: a class not listed
    /// is [`ShipRefusal::NotListed`]; a listed one is refused by the first
    /// of the buy checks that refuses it (the class flown, for sale, then
    /// the cash; see the module docs).
    pub fn can(&self, ship: ShipId) -> Result<(), ShipRefusal> {
        let row = self.row(ship).ok_or(ShipRefusal::NotListed)?;
        refusal(
            &can_buy(),
            &RowShip {
                shipyard: self,
                row,
            },
        )
    }

    /// Whether `ship` can be bought now, or why not: the same as
    /// [`can`](Self::can).
    pub fn check(&self, ship: ShipId) -> Result<(), ShipRefusal> {
        self.can(ship)
    }

    /// What buying `row`'s ship costs after the trade-in: negative when
    /// the player is paid.
    #[must_use]
    pub fn final_price(&self, row: &ShipRow) -> i64 {
        row.price.saturating_sub(self.trade_in)
    }
}

/// One listed ship class, asked whether it can be bought: the facts the
/// shipyard's buy checks read, all settled when the shipyard was built.
#[derive(Clone, Copy)]
struct RowShip<'a> {
    /// The shipyard: its cash, the trade-in and the rule set.
    shipyard: &'a Shipyard,
    /// The row: its price and facts.
    row: &'a ShipRow,
}

impl Context for Shipyard {
    fn rulebook(&self) -> &Rulebook {
        &self.rules
    }
}

impl Context for RowShip<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.shipyard.rulebook()
    }
}

/// The first of `checks` that refuses `facts`, if any.
fn refusal<C: Context>(checks: &Checks<C, ShipRefusal>, facts: &C) -> Result<(), ShipRefusal> {
    // No shipyard rule draws (`no_shipyard_rule_draws`), so no chance is
    // ever asked.
    checks.first_refusal(facts, &mut NeverFires)
}

/// Whether Buy Ship is enabled for a class, as `_CalcShipCanBuy` has it
/// (@0x4f6a0-0x4f90a): in the original's order for the class flown
/// (first), then in the port's: it can be bought here at all, then the
/// cash. The original tests the cash before `Require` and
/// `Availability`, but only sets a flag, so no player can tell that order
/// apart.
fn can_buy<'a>() -> Checks<RowShip<'a>, ShipRefusal> {
    [
        Check::plain(
            "flown",
            "`_CalcShipCanBuy` @0x4f6f5-0x4f6fc: the class selected against the player's own \
             (+0x76), before the cash",
            not_flown,
        ),
        Check::plain(
            "for sale",
            "`_CalcShipCanBuy` `Require` @0x4f80a-0x4f840 and `Availability` \
             @0x4f8e3-0x4f90a, which the port refuses first; tech, the day's roll and \
             `HideHigher` as listed (`_SetupPortAvailableShipTypes` @0xbc4b-0xbe2c)",
            for_sale,
        ),
        Check::plain(
            "cash",
            "`_CalcShipCanBuy` @0x4f702-0x4f7eb: the price less the trade-in (@0x4f7db), \
             at least none (`cmovs` @0x4f7e2), against the signed cash (@0x4f7e5-0x4f7eb)",
            final_price_within_the_cash,
        ),
    ]
    .into()
}

/// The class is not the one the player flies.
fn not_flown(asked: &RowShip) -> Result<(), ShipRefusal> {
    if asked.row.id == asked.shipyard.current {
        Err(ShipRefusal::AlreadyFlown)
    } else {
        Ok(())
    }
}

/// The class can be bought here at all: for sale today, its `Require`
/// met and its `Availability` holding.
fn for_sale(asked: &RowShip) -> Result<(), ShipRefusal> {
    if asked.row.facts.buyable {
        Ok(())
    } else {
        Err(ShipRefusal::NotForSale)
    }
}

/// The final price, taken as none when the trade-in exceeds the price,
/// is no more than the cash, compared signed: a pilot in debt buys
/// nothing, whatever the trade-in (`_CalcShipCanBuy` @0x4f7d8-0x4f7eb).
fn final_price_within_the_cash(asked: &RowShip) -> Result<(), ShipRefusal> {
    let shipyard = asked.shipyard;
    if shipyard.final_price(asked.row).max(0) > shipyard.cash {
        Err(ShipRefusal::CannotAfford)
    } else {
        Ok(())
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

/// What a ship of class `cost` and `mass` carrying `owned` trades in for,
/// as `_PlayerShipTradeInPrice` (@0xb079-0xb144) values it.
///
/// The hull starts the total at [`TRADE_IN_PERCENT`] of its `Cost`,
/// truncated toward zero and not clamped, so a negative `Cost` starts it
/// below 0 (@0xb0b7-0xb0cc). Then, by ascending ID, each outfit owned
/// that has a record and is not persistent adds half its whole line,
/// count × its price on that ship ([`unit_price`], as `rules` reads
/// [`RuleKey::MassPrice`]; inlined
/// @0xb0e3-0xb106), the total truncated toward zero after
/// each (@0xb0da-0xb0e1, @0xb109-0xb11f). The engine also skips a count of
/// 0 (@0xb0d5-0xb0d8); here that outfit's line of 0 leaves the total as it
/// was, so it needs no skip of its own. The persistent flag is the record
/// byte `_LoadObjectData` sets from `Flags` 0x0004 (@0x78cad-0x78cb3). By
/// the engine an unsellable outfit counts; by
/// `source` [`RuleSource::Bible`] it adds nothing either
/// ([`RuleKey::TradeInOutfits`](crate::RuleKey::TradeInOutfits)). Only the
/// final total is clamped at 0 (@0xb131-0xb138).
#[must_use]
pub fn trade_in(
    cost: i32,
    mass: i16,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    source: RuleSource,
    rules: &Rulebook,
) -> i64 {
    let skipped = match source {
        RuleSource::Engine => OutfitFlags::PERSISTENT,
        RuleSource::Bible => OutfitFlags::PERSISTENT | OutfitFlags::CANNOT_SELL,
    };
    let hull = i64::from(cost) * TRADE_IN_PERCENT / 100;
    owned
        .iter()
        .filter_map(|(id, &count)| {
            let record = records.iter().find(|record| record.id == *id)?;
            (record.flags & skipped == 0).then_some((record, count))
        })
        .fold(hull, |total, (record, count)| {
            let line = unit_price(record, mass, rules).saturating_mul(i64::from(count));
            total.saturating_mul(2).saturating_add(line) / 2
        })
        .max(0)
}

/// The Shipyard Info panel's Space number from `start`, taking off each
/// of `taken` in turn and leaving none where a step would go below it
/// (`_ShipDescDialogUpdate`'s `cmovle`/`cmovge` @0x50176-0x50181,
/// @0x50205-0x50210 and @0x502b3-0x502be). With nothing taken, `start`
/// comes back as it is, even below none.
pub(crate) fn floored_space(start: i64, taken: impl IntoIterator<Item = i64>) -> i64 {
    taken
        .into_iter()
        .fold(start, |space, mass| space.saturating_sub(mass).max(0))
}

/// The "Shipyard Info" panel's Space number for class `ship`, its stock
/// in `arsenal` and `outfits`, as `_ShipDescDialogUpdate` works it out
/// (@0x500cf-0x502cb).
///
/// It starts at `FreeMass` plus the mass of the class's standard
/// equipment (its default items and its stock weapons and ammunition as
/// outfits), as the loader holds it (@0x7aaf7, @0x7ac61, @0x7ad8d). It
/// then takes off, through [`floored_space`]: each stock weapon by
/// ascending ID (the last slot naming it giving its count and
/// `AmmoLoad`, @0x7aa72, @0x7aa90), its count of the lowest-ID outfit
/// whose [`MOD_WEAPON`] mod names it, then its `AmmoLoad` of the lowest-ID
/// one whose [`MOD_AMMO`] mod names the weapon itself, not its
/// `AmmoType` (@0x501c5-0x501d0); then each default item in slot order
/// (@0x5025d-0x502c1). Each takes off only above none of it and with a
/// record. The original's 16-bit sum into the class (@0x7ab0b) is not
/// replicated.
pub(crate) fn space(ship: &ShipRecord, arsenal: &Arsenal, outfits: &[OutfitRecord]) -> i64 {
    let weighed = |outfit: OutfitId, count: u16| {
        outfits
            .iter()
            .find(|record| record.id == outfit)
            .map(|record| unit_mass(record, ship.fields.mass).saturating_mul(i64::from(count)))
    };
    let standard = merged(
        &tally(ship.defaults.iter().copied()),
        &arsenal.stock_outfits(ship.id, outfits),
    );
    let start = standard
        .iter()
        .filter_map(|(&outfit, &count)| weighed(outfit, count))
        .fold(i64::from(ship.fields.free_mass), i64::saturating_add);
    let slots: BTreeMap<_, _> = arsenal
        .stock_weapons(ship.id)
        .iter()
        .map(|stock| (stock.weapon, (stock.count, stock.ammo)))
        .collect();
    let weapon_holders = holders(outfits, MOD_WEAPON);
    let ammo_holders = holders(outfits, MOD_AMMO);
    let held = |holders: &[_], weapon, count: i16| {
        let count = u16::try_from(count).ok().filter(|&count| count > 0)?;
        weighed(holder(holders, weapon)?, count)
    };
    let weapons = slots.into_iter().flat_map(|(weapon, (count, ammo))| {
        [
            held(&weapon_holders, weapon, count),
            held(&ammo_holders, weapon, ammo),
        ]
    });
    let defaults = ship
        .defaults
        .iter()
        .filter(|&&(_, count)| count > 0)
        .map(|&(outfit, count)| weighed(outfit, count));
    floored_space(start, weapons.chain(defaults).flatten())
}

/// Everything the shipyard rules read about the ship and where it is.
pub(crate) struct Yard<'a> {
    /// Every `shïp`.
    pub(crate) ships: &'a [ShipRecord],
    /// Every `oütf`.
    pub(crate) outfits: &'a [OutfitRecord],
    /// Every ship class's stock weapons, which the Info's Space reads.
    pub(crate) arsenal: &'a Arsenal,
    /// The ship flown's fields.
    pub(crate) fields: ShipFields,
    /// The stellar landed on.
    pub(crate) site: &'a LandingSite,
    /// How each disputed rule reads ([`Rulebook::source_for`]).
    pub(crate) rules: Rulebook,
    /// The control-bit test of a ship's `Availability`.
    pub(crate) gate: Gate<'a>,
}

impl Yard<'_> {
    /// The shipyard, for `pilot`, each class's roll for the day kept in
    /// `rolls` and any not drawn yet drawn on `chance`; `None` when the
    /// stellar has none.
    ///
    /// The draw order: the classes are walked by ascending ID; for each,
    /// [`required`] and [`available`] are settled first (no draw), then
    /// its tech level; only when tech allows is `rolls` asked for the
    /// class's [`day_roll`], which draws once on `chance`, and only for a
    /// roll of 1-99 not already kept; `HideHigher` is tested after the
    /// roll. So a class `HideHigher` takes off sale, or one whose
    /// `Require` is unmet, still draws; one tech forbids does not, nor
    /// does a roll of always or never. A listed `HideHigher` class, greyed
    /// or not (for the class flown, the cash, its `Require` or its
    /// `Availability`), takes the higher-numbered classes of its
    /// `DispWeight` off sale (@0xbdc3-0xbe2c). No rule draws: the buy checks, the
    /// value rules and the purchase never ask `chance`.
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
        let trade_in = trade_in_value().of(&Flown { yard: self, pilot });
        let mut sorted: Vec<&ShipRecord> = self.ships.iter().collect();
        sorted.sort_by_key(|ship| ship.id);
        let mut sweep = HideHigher::default();
        let mut rows = Vec::new();
        for ship in sorted {
            let offer = ShipOffer {
                yard: self,
                ship,
                contributed,
            };
            let required = required().of(&offer);
            let available = available().of(&offer);
            let for_sale = tech_listed().of(&offer)
                && rolls.today(ship.id, day_roll().of(&offer), chance)
                && sweep.on_sale(ship.disp_weight);
            let buyable = for_sale && required && available;
            let listed = for_sale && !wares::hidden(ship.flags3, HIDE_BITS, required, available);
            // The sweep (@0xbdc3-0xbe2c) reads the listing byte alone,
            // never `_CalcShipCanBuy`, so a greyed hider still hides.
            sweep.note(
                ship.disp_weight,
                ship.flags3 & ShipFlags3::HIDE_HIGHER != 0,
                listed,
            );
            if !listed {
                continue;
            }
            rows.push((
                ship.disp_weight,
                ShipRow {
                    id: ship.id,
                    name: ship.name.clone(),
                    short_name: ship.short_name.clone(),
                    price: price().of(&offer),
                    specs: ShipSpecs::of(ship, self.arsenal, self.outfits),
                    facts: ShipFacts { buyable },
                },
            ));
        }
        wares::in_display_order(&mut rows, |(weight, row)| (*weight, row.id.0));
        Some(Shipyard {
            rows: rows.into_iter().map(|(_, row)| row).collect(),
            trade_in,
            cash: pilot.cash,
            current: pilot.ship,
            rules: self.rules,
        })
    }
}

/// One ship class asked about as the shipyard is built: the facts the
/// listing's value rules read.
struct ShipOffer<'a> {
    /// The yard: the site, the gate and the rule set.
    yard: &'a Yard<'a>,
    /// The class.
    ship: &'a ShipRecord,
    /// The `Contribute` bits of the ship flown and every outfit owned.
    contributed: u64,
}

impl Context for ShipOffer<'_> {
    fn rulebook(&self) -> &Rulebook {
        &self.yard.rules
    }
}

/// The ship flown, asked what it trades in for.
struct Flown<'a> {
    /// The yard: every `shïp` and `oütf`, the ship's fields and the rule
    /// set.
    yard: &'a Yard<'a>,
    /// The pilot: the class flown and the outfits owned.
    pilot: &'a Pilot,
}

impl Context for Flown<'_> {
    fn rulebook(&self) -> &Rulebook {
        &self.yard.rules
    }
}

/// Whether a class's `TechLevel` puts it on sale at the stellar, as
/// [`RuleKey::TechLevel`] says: by the engine,
/// [`wares::ship_tech_allows`]; by the Bible, [`wares::by_tech_level`].
fn tech_listed<'a>() -> Value<ShipOffer<'a>, bool> {
    Value::disputed(
        "tech",
        "`_SetupPortAvailableShipTypes` @0xbc4b-0xbc72; `_LoadObjectData` @0x7a161-0x7a165",
        RuleKey::TechLevel,
        engine_tech,
        bible_tech,
    )
}

/// The engine's tech test: never below 0.
fn engine_tech(offer: &ShipOffer) -> bool {
    wares::ship_tech_allows(offer.ship.tech_level, offer.yard.site)
}

/// The Bible's tech test: at or below the stellar's, or a slot exactly.
fn bible_tech(offer: &ShipOffer) -> bool {
    wares::by_tech_level(offer.ship.tech_level, offer.yard.site)
}

/// Whether a class's `Require` is met: a `shïp` has no `RequireGovt`, so
/// it applies everywhere.
fn required<'a>() -> Value<ShipOffer<'a>, bool> {
    Value::plain(
        "required",
        "`_CalcShipCanBuy` @0x4f80a-0x4f840 (`_PlayerMeetsRequirements` @0x4f833); \
         `_SetupPortAvailableShipTypes` @0xbce7-0xbd2b",
        requirement_met,
    )
}

/// Each of the class's `Require` bits is contributed.
fn requirement_met(offer: &ShipOffer) -> bool {
    wares::requirement_met(offer.ship.require, offer.contributed)
}

/// Whether a class's `Availability` holds, asked through the gate.
fn available<'a>() -> Value<ShipOffer<'a>, bool> {
    Value::plain(
        "available",
        "`_CalcShipCanBuy` @0x4f8e3-0x4f90a (`_EvalMissionBitTestString` @0x4f901); \
         `_SetupPortAvailableShipTypes` @0xbd33-0xbd5b",
        gate_allows,
    )
}

/// The gate allows the class's `Availability`.
fn gate_allows(offer: &ShipOffer) -> bool {
    offer.yard.gate.allows(&offer.ship.availability)
}

/// How a class rolls for sale today, as
/// [`RuleKey::BuyRandom`](crate::RuleKey::BuyRandom) reads its
/// `BuyRandom` ([`buy_roll`]): the odds, not a draw.
fn day_roll<'a>() -> Value<ShipOffer<'a>, Roll> {
    Value::disputed(
        "day roll",
        "`_SetupPortAvailableShipTypes` @0xbc88-0xbc9d; `_LoadObjectData` @0x7a340-0x7a34a",
        RuleKey::BuyRandom,
        engine_day_roll,
        bible_day_roll,
    )
}

/// The engine's roll: below 0 is always.
fn engine_day_roll(offer: &ShipOffer) -> Roll {
    buy_roll(offer.ship.buy_random, RuleSource::Engine)
}

/// The Bible's roll: 0 or below is never.
fn bible_day_roll(offer: &ShipOffer) -> Roll {
    buy_roll(offer.ship.buy_random, RuleSource::Bible)
}

/// A class's price ([`ship_price`]).
fn price<'a>() -> Value<ShipOffer<'a>, i64> {
    Value::plain(
        "price",
        "`shïp` `Cost` (`_CalcShipCanBuy` @0x4f74b-0x4f751), without its \
         `_ApplyPriceAndTechnologyFlux`, which is not modelled",
        cost_price,
    )
}

/// The class's `Cost`, never below none.
fn cost_price(offer: &ShipOffer) -> i64 {
    ship_price(offer.ship)
}

/// What the ship flown trades in for, as
/// [`RuleKey::TradeInOutfits`](crate::RuleKey::TradeInOutfits) says
/// ([`trade_in`]); a class with no record has no `Cost`.
fn trade_in_value<'a>() -> Value<Flown<'a>, i64> {
    Value::disputed(
        "trade-in",
        "`_PlayerShipTradeInPrice` @0xb079-0xb144, the persistent byte @0xb0da-0xb0e1",
        RuleKey::TradeInOutfits,
        engine_trade_in,
        bible_trade_in,
    )
}

/// The engine's trade-in: an unsellable outfit counts.
fn engine_trade_in(flown: &Flown) -> i64 {
    flown_trade_in(flown, RuleSource::Engine)
}

/// The Bible's trade-in: an unsellable outfit adds nothing.
fn bible_trade_in(flown: &Flown) -> i64 {
    flown_trade_in(flown, RuleSource::Bible)
}

/// What the ship flown trades in for, read as `source` says.
fn flown_trade_in(flown: &Flown, source: RuleSource) -> i64 {
    let yard = flown.yard;
    let cost = yard
        .ships
        .iter()
        .find(|ship| ship.id == flown.pilot.ship)
        .map_or(0, |ship| ship.cost);
    trade_in(
        cost,
        yard.fields.mass,
        &flown.pilot.outfits,
        yard.outfits,
        source,
        &yard.rules,
    )
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
    /// sold back, by the other reading of
    /// [`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry); empty by
    /// the engine.
    pub sold_back: BTreeMap<OutfitId, u16>,
    /// What they were sold back for, credited, by the other reading of
    /// [`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry); 0 by the
    /// engine.
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
                // The share is in 0..=1, so the part is in 0..=held.
                let part = (f64::from(*tons) * share) as u32;
                *tons = match good {
                    Good::Commodity(_) => part,
                    Good::Junk(_) => tons.saturating_sub(part),
                };
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
/// that plus the `escorts` tons, in doubles. With no space of its own the
/// share is none: the engine's 0/0 is NaN, which converts to a value that
/// keeps nothing of a commodity and takes nothing of a `jünk`, as none
/// does. The escorts' tons are never below none ([`escort_tons`]), so the
/// share is never above 1 and the engine's `minsd` cap at 1 (@0xcf33)
/// never bites.
fn new_ships_share(capacity: u32, escorts: i64) -> f64 {
    if capacity == 0 {
        return 0.0;
    }
    let own = f64::from(capacity);
    own / (own + escorts as f64)
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

/// A ship purchase asked what it does: the pilot as it stands once the
/// old class's `OnRetire` has run and the launched fighters are gone, and
/// what the new class, the records and the fleet say.
pub(crate) struct Sale<'a> {
    /// The pilot buying.
    pub(crate) pilot: &'a Pilot,
    /// The old ship's mass, which a persistent outfit sold back is priced
    /// on.
    pub(crate) old_mass: i16,
    /// The class bought.
    pub(crate) new: &'a ShipRecord,
    /// The name confirmed ([`cull_name`]).
    pub(crate) name: &'a str,
    /// The new class's stock weapons and ammunition
    /// ([`Arsenal::stock_fits`](crate::combat::armament::Arsenal::stock_fits)).
    pub(crate) fits: &'a [StockFit],
    /// Every `oütf`.
    pub(crate) records: &'a [OutfitRecord],
    /// The escorts that stay with the player, launched fighters already
    /// gone.
    pub(crate) escorts: &'a [EscortHolds],
    /// How each disputed rule reads: the cargo kept by
    /// [`RuleKey::PurchaseCargo`](crate::RuleKey::PurchaseCargo), and the
    /// price of a persistent outfit sold back, by
    /// [`RuleKey::MassPrice`](crate::RuleKey::MassPrice).
    pub(crate) rules: Rulebook,
}

impl Context for Sale<'_> {
    fn rulebook(&self) -> &Rulebook {
        &self.rules
    }
}

/// A purchase with the new class's stats worked out, from the outfits it
/// leaves the pilot.
struct Refitted<'a> {
    /// The purchase.
    sale: &'a Sale<'a>,
    /// The new class's stats with the outfits after.
    stats: ShipStats,
}

impl Context for Refitted<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.sale.rulebook()
    }
}

/// A bought ship's stats once the new class's `OnPurchase` has run: the
/// class and outfits it leaves.
struct Filled<'a> {
    /// The ship's stats after `OnPurchase`.
    stats: &'a ShipStats,
    /// The rules in force.
    rules: &'a Rulebook,
}

impl Context for Filled<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// A purchase settling at a quote, with what its sale back refunds.
struct Settled<'a> {
    /// The purchase.
    sale: &'a Sale<'a>,
    /// The price and the trade-in.
    quote: Quote,
    /// What the persistent outfits sold back refund.
    refund: i64,
}

impl Context for Settled<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.sale.rulebook()
    }
}

/// What a purchase does with the persistent outfits owned.
struct Carry {
    /// Those carried over to the new ship.
    carried: BTreeMap<OutfitId, u16>,
    /// Those that did not carry over, each with how many, sold back.
    sold_back: BTreeMap<OutfitId, u16>,
    /// What they were sold back for.
    refund: i64,
}

/// What a ship purchase does, worked out and not yet applied: the
/// session applies it through `Session::become_class`.
pub(crate) struct Purchase {
    /// What the pilot owns afterwards: the persistent outfits carried, the
    /// new class's default items on top, its stock topped up.
    pub(crate) outfits: BTreeMap<OutfitId, u16>,
    /// What the purchase writes on the pilot beside the class.
    pub(crate) settlement: Settlement,
    /// What it did, as `Session::buy_ship` reports it.
    pub(crate) report: ShipPurchase,
}

/// What a purchase writes on the pilot beside the class and the outfits.
pub(crate) struct Settlement {
    /// The cash after the price, the trade-in and the refund, if any.
    pub(crate) cash: i64,
    /// The ship's name ([`cull_name`]).
    pub(crate) name: String,
    /// The cargo kept.
    pub(crate) cargo: BTreeMap<Good, u32>,
}

impl Settlement {
    /// Writes the cash, the name and the cargo on `pilot`. The reserves
    /// are filled later, once the new class's `OnPurchase` has run
    /// ([`filled`], `_DoShipyardDialog` @0x5f051-0x5f07b).
    pub(crate) fn settle(self, pilot: &mut Pilot) {
        pilot.ship_name = Some(self.name);
        pilot.cash = self.cash;
        pilot.cargo = self.cargo;
    }
}

/// Which persistent outfits a purchase carries over, and what it sells
/// back, as [`RuleKey::PurchaseCarry`](crate::RuleKey::PurchaseCarry)
/// says (see the module's Buying a ship).
fn carried<'a>() -> Value<Sale<'a>, Carry> {
    Value::disputed(
        "carried",
        "`_DoShipyardDialog` @0x5ee1b-0x5ee4f (the byte +0x378, `_LoadObjectData` \
         @0x78cad-0x78cb3), the defaults added @0x5ee51-0x5ee97",
        RuleKey::PurchaseCarry,
        engine_carry,
        carry_persistent,
    )
}

/// The engine's carry: every persistent outfit owned, whole, nothing sold
/// back ([`carried_outfits`] keeping by 0x0004 alone, as capture does).
fn engine_carry(sale: &Sale) -> Carry {
    Carry {
        carried: carried_outfits(
            &sale.pilot.outfits,
            OutfitCarry::Persistent {
                mask: OutfitFlags::PERSISTENT,
            },
            &BTreeMap::new(),
            false,
            sale.records,
        ),
        sold_back: BTreeMap::new(),
        refund: 0,
    }
}

/// By the other reading of `purchase_carry`, each persistent outfit owned,
/// by ascending ID, one unit at a time: carried while under its `Max` and
/// while the new ship, its standard equipment fitted, has free mass for
/// it; otherwise sold back at the outfitter's rate, priced on the old
/// ship.
fn carry_persistent(sale: &Sale) -> Carry {
    let records = sale.records;
    let defaults = tally(sale.new.defaults.iter().copied());
    let standard = merged(&defaults, &fitted(sale.fits));
    let mut carried: BTreeMap<OutfitId, u16> = BTreeMap::new();
    let mut sold_back: BTreeMap<OutfitId, u16> = BTreeMap::new();
    let mut refund: i64 = 0;
    let persistent = sale.pilot.outfits.iter().filter_map(|(id, &count)| {
        let record = records.iter().find(|record| record.id == *id)?;
        (record.flags & OutfitFlags::PERSISTENT != 0).then_some((record, count))
    });
    for (record, count) in persistent {
        let most = u16::try_from(record.max).unwrap_or(0);
        for unit in 0..count {
            let mut trial = carried.clone();
            *trial.entry(record.id).or_default() += 1;
            let fits = unit < most
                && free_mass(
                    sale.new.fields,
                    &standard,
                    &merged(&trial, &standard),
                    records,
                ) >= 0;
            if fits {
                carried = trial;
            } else {
                *sold_back.entry(record.id).or_default() += 1;
                refund =
                    refund.saturating_add(resale(unit_price(record, sale.old_mass, &sale.rules)));
            }
        }
    }
    Carry {
        carried,
        sold_back,
        refund,
    }
}

/// The cash after a purchase.
fn cash_after<'a>() -> Value<Settled<'a>, i64> {
    Value::plain(
        "cash after",
        "`_DoShipyardDialog` trade-in @0x5ec96, price @0x5ed54; no other cash write on the \
         purchase, so the refund (none by the engine) is `purchase_carry`'s other reading",
        cash_settled,
    )
}

/// The cash less the price, plus the trade-in and the refund.
fn cash_settled(settled: &Settled) -> i64 {
    settled
        .sale
        .pilot
        .cash
        .saturating_sub(settled.quote.price)
        .saturating_add(settled.quote.trade_in)
        .saturating_add(settled.refund)
}

/// The cargo a purchase keeps, and what it leaves behind, each good with
/// its tons.
type CargoKept = (BTreeMap<Good, u32>, BTreeMap<Good, u32>);

/// The cargo a purchase keeps, and what it leaves behind, as
/// [`RuleKey::PurchaseCargo`](crate::RuleKey::PurchaseCargo) says
/// ([`keep_cargo`]).
fn cargo_kept<'a>() -> Value<Refitted<'a>, CargoKept> {
    Value::disputed(
        "cargo kept",
        "`_DoShipyardDialog` @0x5ef9f, `_DestroyPartialFleetCargo` @0xcd32-0xcff6, \
         `_ResetPlayerPrecalcedValues` @0xc7d8-0xc82c",
        RuleKey::PurchaseCargo,
        engine_cargo,
        bible_cargo,
    )
}

/// The engine's cargo: the new ship's share of the fleet's, trimmed.
fn engine_cargo(refitted: &Refitted) -> CargoKept {
    cargo_kept_by(refitted, RuleSource::Engine)
}

/// The other reading's cargo: what fits in the new ship's own hold.
fn bible_cargo(refitted: &Refitted) -> CargoKept {
    cargo_kept_by(refitted, RuleSource::Bible)
}

/// The cargo kept, and what is left behind, read as `source` says.
fn cargo_kept_by(refitted: &Refitted, source: RuleSource) -> CargoKept {
    let mut cargo = refitted.sale.pilot.cargo.clone();
    let left_behind = keep_cargo(
        &mut cargo,
        refitted.stats.capacity,
        refitted.sale.escorts,
        source,
    );
    (cargo, left_behind)
}

/// The bought ship's reserves, filled once its `OnPurchase` has run.
fn reserves_full<'a>() -> Value<Filled<'a>, Reserves> {
    Value::plain(
        "reserves full",
        "`_DoShipyardDialog` @0x5f051-0x5f07b, after `OnPurchase` @0x5f00d; the capacities \
         of the class and outfits it leaves (`_EvalMissionBitSetString` @0x99dea)",
        full_reserves,
    )
}

/// The shield, armour and fuel, full to the most of the stats.
fn full_reserves(filled: &Filled) -> Reserves {
    filled.stats.full()
}

/// A bought ship's shield, armour and fuel, filled to the most of
/// `stats`, the class and outfits its `OnPurchase` leaves, as
/// `_DoShipyardDialog` fills them after it (@0x5f051-0x5f07b, after
/// @0x5f00d): what `Session::buy_ship` writes last.
pub(crate) fn filled(stats: &ShipStats, rules: &Rulebook) -> Reserves {
    reserves_full().of(&Filled { stats, rules })
}

/// What buying `sale.new` at `quote` does, as the module says, worked out
/// and not applied: the outfits the pilot owns afterwards, what it writes
/// beside them, and what it did.
pub(crate) fn purchase(sale: &Sale, quote: Quote) -> Purchase {
    let carry = carried().of(sale);
    let defaults = tally(sale.new.defaults.iter().copied());
    let mut outfits = merged(&carry.carried, &defaults);
    fit_stock(&mut outfits, sale.fits, sale.records);
    let stats = ShipStats::new(sale.new.fields, &outfit_mods(&outfits, sale.records));
    let refitted = Refitted { sale, stats };
    let (cargo, left_behind) = cargo_kept().of(&refitted);
    let cash = cash_after().of(&Settled {
        sale,
        quote,
        refund: carry.refund,
    });
    Purchase {
        outfits,
        settlement: Settlement {
            cash,
            name: cull_name(sale.name),
            cargo,
        },
        report: ShipPurchase {
            price: quote.price,
            trade_in: quote.trade_in,
            sold_back: carry.sold_back,
            refund: carry.refund,
            left_behind,
        },
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::sync::LazyLock;

    use super::*;
    use crate::catalog::{
        DisasterId, GovtId, HullRecord, JunkId, StellarId, StockWeapon, SystemId, WeaponId,
        WeaponRecord,
    };
    use crate::chance::NeverFires;
    use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use crate::control::Test;
    use crate::market::MORE_CARGO;
    use crate::rules::Descriptor;
    use crate::stats::{MORE_FUEL, MORE_SHIELD};
    use crate::testkit::{
        AllowAll, FAST, RefuseBits, Scripted, catalog, hull, outfit, planet, ship, weapon,
    };
    use crate::wares::TonUnit;

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

    /// An arsenal with no hull, so no class has stock.
    static NO_STOCK: LazyLock<Arsenal> = LazyLock::new(Arsenal::default);

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
            arsenal: &NO_STOCK,
            fields: FAST,
            site,
            rules: Rulebook::default(),
            gate: Gate::FRESH,
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

    /// Ships 129-136 of tech 3, 4, 5, 6, 55, 7, 0 and -3.
    fn teched() -> [ShipRecord; 8] {
        [
            tech(129, 3),
            tech(130, 4),
            tech(131, 5),
            tech(132, 6),
            tech(133, 55),
            tech(134, 7),
            tech(135, 0),
            tech(136, -3),
        ]
    }

    /// The ships listed of `ships` at `site`, reading `TechLevel` by
    /// `source`, with no roll drawn yet and none that fires.
    fn listed_teching(ships: &[ShipRecord], site: &LandingSite, source: RuleSource) -> Vec<i16> {
        let shipyard = Yard {
            rules: Rulebook::default().with_override(RuleKey::TechLevel, source),
            ..yard(ships, &[], site)
        }
        .shipyard(&pilot(), &mut DayRolls::default(), &mut NeverFires)
        .expect("a shipyard");
        listed(&shipyard)
    }

    #[test]
    fn it_sells_up_to_its_tech_level_and_its_special_tech_exactly() {
        let shipyard = open(&teched(), &pilot());
        assert_eq!(listed(&shipyard), [129, 130, 132, 133, 135]);
        assert!(shipyard.rows.iter().all(|row| shipyard.can(row.id).is_ok()));
    }

    #[test]
    fn by_the_bible_a_ship_of_negative_tech_is_sold_at_or_above_it() {
        assert_eq!(
            listed_teching(&teched(), &port(), RuleSource::Bible),
            [129, 130, 132, 133, 135, 136]
        );
        assert_eq!(
            listed_teching(&teched(), &port(), RuleSource::Engine),
            [129, 130, 132, 133, 135]
        );
    }

    #[test]
    fn a_ship_of_tech_0x7fff_is_listed_where_tech_allows_it() {
        let top = LandingSite {
            tech_level: i16::MAX,
            ..port()
        };
        let mut slotted = port();
        slotted.special_tech[2] = i16::MAX;
        let ships = [tech(129, i16::MAX)];
        for source in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(listed_teching(&ships, &top, source), [129], "{source:?}");
            assert_eq!(
                listed_teching(&ships, &slotted, source),
                [129],
                "{source:?}"
            );
            assert_eq!(
                listed_teching(&ships, &port(), source),
                Vec::<i16>::new(),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_special_tech_slot_of_0_sells_a_ship_of_tech_0() {
        let site = LandingSite {
            tech_level: -5,
            special_tech: [0, -1, 0, 0, 0, 0, 0, 0],
            ..port()
        };
        let ships = [tech(129, 0), tech(130, -1), tech(131, 1)];
        assert_eq!(listed_teching(&ships, &site, RuleSource::Engine), [129]);
        assert_eq!(listed_teching(&ships, &site, RuleSource::Bible), [129, 130]);
    }

    #[test]
    fn a_class_tech_forbids_draws_no_roll() {
        let ships = [
            ShipRecord {
                tech_level: -1,
                ..buying(129, 50)
            },
            buying(130, 40),
        ];
        let mut chance = Scripted::answering(&[true]);
        let shipyard = rolled(&ships, &pilot(), &mut DayRolls::default(), &mut chance);
        assert_eq!(chance.asked, [40], "the negative class draws nothing");
        assert_eq!(listed(&shipyard), [130]);
    }

    #[test]
    fn the_tech_rule_reads_tech_level_and_cites_the_listing() {
        let value = tech_listed();
        assert_eq!(value.about().reads(), Some(RuleKey::TechLevel));
        assert_eq!(value.about().name(), "tech");
        assert_eq!(
            value.about().reproduces(),
            "`_SetupPortAvailableShipTypes` @0xbc4b-0xbc72; `_LoadObjectData` @0x7a161-0x7a165"
        );
        assert!(!value.about().draws());
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
        assert_eq!(on.can(ShipId(129)), Ok(()));
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
    fn the_listing_draws_each_classs_roll_by_ascending_id() {
        let ships = [
            buying(133, 30),
            ShipRecord {
                disp_weight: 5,
                ..buying(131, 50)
            },
            ShipRecord {
                tech_level: 9,
                ..buying(132, 70)
            },
            ShipRecord {
                require: 0x2,
                ..buying(130, 40)
            },
            buying(134, 0),
            ShipRecord {
                disp_weight: 5,
                flags3: ShipFlags3::HIDE_HIGHER,
                ..buying(129, 100)
            },
            buying(135, 100),
        ];
        let mut rolls = DayRolls::default();
        let mut chance = Scripted::answering(&[true, true, true]);
        let shipyard = rolled(&ships, &pilot(), &mut rolls, &mut chance);
        assert_eq!(
            chance.asked,
            [40, 50, 30],
            "by ascending ID: 130 though its Require is unmet, 131 though HideHigher \
             takes it off; not 132, which tech forbids, nor 129, 134 or 135, which \
             are always or never"
        );
        assert_eq!(listed(&shipyard), [129, 130, 133, 135]);
        let mut again = Scripted::default();
        rolled(&ships, &pilot(), &mut rolls, &mut again);
        assert!(again.asked.is_empty(), "a second build draws nothing");
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
            rules: Rulebook::default().with_override(RuleKey::BuyRandom, RuleSource::Bible),
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
                        space: 30,
                    },
                    facts: ShipFacts { buyable: true },
                }],
                trade_in: 0,
                cash: 50_000,
                current: ShipId(128),
                rules: Rulebook::default(),
            }
        );
    }

    #[test]
    fn the_class_flown_is_listed_but_cannot_be_bought() {
        let ships = [cheap(128), cheap(129)];
        let shipyard = open(&ships, &pilot());
        assert_eq!(listed(&shipyard), [128, 129]);
        assert_eq!(shipyard.can(ShipId(128)), Err(ShipRefusal::AlreadyFlown));
        assert_eq!(shipyard.can(ShipId(129)), Ok(()));
        assert_eq!(shipyard.current, ShipId(128));
    }

    #[test]
    fn the_class_flown_is_refused_whatever_its_name_and_a_namesake_class_sells() {
        let shuttle = |id| ShipRecord {
            name: "Shuttle".to_owned(),
            ..cheap(id)
        };
        let ships = [shuttle(128), shuttle(361)];
        let mut named = pilot();
        named.ship_name = Some("Kestrel".to_owned());
        let shipyard = open(&ships, &named);
        assert_eq!(
            shipyard.can(ShipId(128)),
            Err(ShipRefusal::AlreadyFlown),
            "named Kestrel, still the class flown"
        );
        assert_eq!(
            shipyard.can(ShipId(361)),
            Ok(()),
            "another class of the same name"
        );
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
        assert_eq!(shipyard.can(ShipId(129)), Ok(()), "the ship's bit");
        assert_eq!(shipyard.can(ShipId(130)), Err(ShipRefusal::NotForSale));
        assert_eq!(shipyard.can(ShipId(131)), Err(ShipRefusal::NotForSale));
        let licensed = open_with(&ships, &outfits, &owning(&[(140, 1)]));
        assert_eq!(licensed.can(ShipId(131)), Ok(()), "the licence's bit too");
        assert_eq!(licensed.can(ShipId(130)), Err(ShipRefusal::NotForSale));
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
        let site = port();
        Yard {
            gate,
            ..yard(ships, &[], &site)
        }
        .shipyard(pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("a shipyard")
    }

    /// A `HIDE_HIGHER` class `id` at weight 5 with `flags3` added, its
    /// `Require` `require` and `Availability` `availability`.
    fn hider(id: i16, flags3: u16, require: u64, availability: &str) -> ShipRecord {
        ShipRecord {
            require,
            disp_weight: 5,
            ..gated(id, ShipFlags3::HIDE_HIGHER | flags3, availability)
        }
    }

    fn plain_at_5(id: i16) -> ShipRecord {
        ShipRecord {
            disp_weight: 5,
            ..cheap(id)
        }
    }

    // `_SetupPortAvailableShipTypes` @0xbdc3-0xbe2c sweeps on the listing
    // byte alone, never `_CalcShipCanBuy`, so a greyed hider still hides.
    #[test]
    fn a_greyed_hider_still_hides_higher_ships_of_its_weight() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let unmet = open(&[hider(130, 0, 0x2, ""), plain_at_5(131)], &pilot());
        assert_eq!(listed(&unmet), [130], "an unmet Require");
        assert_eq!(unmet.can(ShipId(130)), Err(ShipRefusal::NotForSale));
        let unavailable = open_gated(
            &[hider(130, 0, 0, "b7"), plain_at_5(131)],
            refusing,
            &pilot(),
        );
        assert_eq!(listed(&unavailable), [130], "a false Availability");
        assert_eq!(unavailable.can(ShipId(130)), Err(ShipRefusal::NotForSale));
        let dear = ShipRecord {
            cost: 20_000,
            ..hider(130, 0, 0, "")
        };
        let unaffordable = open(&[dear, plain_at_5(131)], &pilot());
        assert_eq!(listed(&unaffordable), [130], "the cash");
        assert_eq!(
            unaffordable.can(ShipId(130)),
            Err(ShipRefusal::CannotAfford)
        );
        let flown = open(&[hider(128, 0, 0, ""), plain_at_5(129)], &pilot());
        assert_eq!(listed(&flown), [128], "the class flown");
        assert_eq!(flown.can(ShipId(128)), Err(ShipRefusal::AlreadyFlown));
    }

    #[test]
    fn a_hider_its_own_flag_hides_hides_nothing() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let unmet = [
            hider(130, ShipFlags3::HIDE_UNLESS_REQUIRED, 0x2, ""),
            plain_at_5(131),
        ];
        assert_eq!(listed(&open(&unmet, &pilot())), [131]);
        let unavailable = [
            hider(130, ShipFlags3::HIDE_UNLESS_AVAILABLE, 0, "b7"),
            plain_at_5(131),
        ];
        assert_eq!(listed(&open_gated(&unavailable, refusing, &pilot())), [131]);
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
        assert_eq!(shipyard.can(ShipId(129)), Err(ShipRefusal::NotForSale));
        assert_eq!(shipyard.can(ShipId(130)), Ok(()), "another bit holds");
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
        assert_eq!(shipyard.can(ShipId(130)), Err(ShipRefusal::NotForSale));
        assert_eq!(shipyard.can(ShipId(131)), Ok(()));
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
        assert_eq!(shipyard.can(ShipId(129)), Err(ShipRefusal::NotForSale));
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
        // One greyed for an unmet Require still hides.
        let unmet = [
            ShipRecord {
                require: 0x2,
                ..weighted(130, 5, ShipFlags3::HIDE_HIGHER)
            },
            weighted(131, 5, 0),
        ];
        assert_eq!(listed(&open(&unmet, &pilot())), [130]);
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
        let hull = |cost| {
            trade_in(
                cost,
                40,
                &BTreeMap::new(),
                &[],
                RuleSource::Engine,
                &Rulebook::default(),
            )
        };
        assert_eq!(TRADE_IN_PERCENT, 25);
        assert_eq!(hull(10_000), 2500);
        assert_eq!(hull(10_003), 2500, "truncated");
        assert_eq!(hull(i32::MAX), i64::from(i32::MAX) / 4);
    }

    #[test]
    fn a_negative_hull_is_clamped_only_after_the_outfits_are_added() {
        let records = upgrades();
        let engine = |cost, owned: &[(i16, u16)]| {
            trade_in(
                cost,
                40,
                &map(owned),
                &records,
                RuleSource::Engine,
                &Rulebook::default(),
            )
        };
        assert_eq!(engine(-400, &[]), 0);
        assert_eq!(engine(-4000, &[(128, 1)]), 0, "-1000 + 1000");
        assert_eq!(engine(-4000, &[(128, 2)]), 1000, "-1000 + 2000");
        assert_eq!(engine(-4000, &[(128, 3)]), 2000);
    }

    #[test]
    fn each_step_truncates_toward_zero_by_ascending_id() {
        // A hull of trunc(-6 / 4) = -1; then trunc(-1 + 3 / 2) = 0 and
        // trunc(0 + 1 / 2) = 0. Walked 129 first, it would come to 1.
        let records = [
            OutfitRecord {
                cost: 3,
                ..outfit(128, &[])
            },
            OutfitRecord {
                cost: 1,
                ..outfit(129, &[])
            },
        ];
        assert_eq!(
            trade_in(
                -6,
                40,
                &map(&[(128, 1), (129, 1)]),
                &records,
                RuleSource::Engine,
                &Rulebook::default()
            ),
            0
        );
        // A hull of trunc(-10 / 4) = -2; trunc(-2 + 3 / 2) = 0, not
        // floor(-0.5) = -1, so trunc(0 + 1 / 2) + ... stays at 0 and a
        // further 3-credit line gives 1.
        let more = [
            records[0].clone(),
            records[1].clone(),
            OutfitRecord {
                cost: 3,
                ..outfit(130, &[])
            },
        ];
        assert_eq!(
            trade_in(
                -10,
                40,
                &map(&[(128, 1), (129, 1), (130, 1)]),
                &more,
                RuleSource::Engine,
                &Rulebook::default()
            ),
            1
        );
    }

    #[test]
    fn an_outfits_line_is_halved_whole_not_unit_by_unit() {
        let rounds = [OutfitRecord {
            cost: 25,
            ..outfit(128, &[])
        }];
        let line = |count| {
            trade_in(
                10_000,
                40,
                &map(&[(128, count)]),
                &rounds,
                RuleSource::Engine,
                &Rulebook::default(),
            ) - 2500
        };
        assert_eq!(line(20), 250, "not 20 * 12 = 240");
        assert_eq!(line(1), 12);
        assert_eq!(line(3), 37);
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
    fn by_the_engine_the_outfits_trade_in_at_half_but_persistent_ones_add_nothing() {
        let records = upgrades();
        let engine = |owned: &[(i16, u16)]| {
            trade_in(
                10_000,
                40,
                &map(owned),
                &records,
                RuleSource::Engine,
                &Rulebook::default(),
            )
        };
        let hull = 2500;
        assert_eq!(engine(&[(128, 3)]), hull + 3000);
        assert_eq!(engine(&[(129, 1)]), hull, "persistent");
        assert_eq!(engine(&[(130, 2)]), hull + 3000, "unsellable, but counted");
        assert_eq!(engine(&[(131, 1)]), hull + 200, "priced on the old ship");
        assert_eq!(engine(&[(999, 5)]), hull, "no record");
        assert_eq!(engine(&[(128, 0)]), hull, "none owned");
        assert_eq!(
            engine(&[(128, 1), (129, 1), (130, 1), (131, 2)]),
            hull + 1000 + 1500 + 400
        );
        let dear = [OutfitRecord {
            cost: i32::MAX,
            flags: OutfitFlags::PRICE_BY_MASS,
            ..outfit(128, &[])
        }];
        let unit = i64::from(i32::MAX) * i64::from(i16::MAX);
        assert_eq!(
            trade_in(
                10_000,
                i16::MAX,
                &map(&[(128, u16::MAX)]),
                &dear,
                RuleSource::Engine,
                &Rulebook::default()
            ),
            hull + unit * i64::from(u16::MAX) / 2,
            "no overflow"
        );
    }

    #[test]
    fn by_the_bible_unsellable_ones_add_nothing_either() {
        let records = upgrades();
        let bible = |owned: &[(i16, u16)]| {
            trade_in(
                10_000,
                40,
                &map(owned),
                &records,
                RuleSource::Bible,
                &Rulebook::default(),
            )
        };
        let hull = 2500;
        assert_eq!(bible(&[(128, 3)]), hull + 3000);
        assert_eq!(bible(&[(129, 1)]), hull);
        assert_eq!(bible(&[(130, 2)]), hull);
        assert_eq!(
            bible(&[(128, 1), (129, 1), (130, 1), (131, 2)]),
            hull + 1000 + 400
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
    fn the_trade_in_value_reads_trade_in_outfits() {
        let ships = [ShipRecord {
            cost: 10_000,
            ..ship(128, FAST)
        }];
        let records = upgrades();
        let site = port();
        let pilot = owning(&[(130, 1)]);
        let trade_in = |source| {
            Yard {
                rules: Rulebook::default().with_override(RuleKey::TradeInOutfits, source),
                ..yard(&ships, &records, &site)
            }
            .shipyard(&pilot, &mut DayRolls::default(), &mut NeverFires)
            .expect("a shipyard")
            .trade_in
        };
        assert_eq!(trade_in(RuleSource::Engine), 2500 + 1500);
        assert_eq!(trade_in(RuleSource::Bible), 2500, "unsellable");
    }

    #[test]
    fn the_trade_in_prices_an_outfit_by_mass_at_least_at_its_cost() {
        // Two of #131, 10 credits a ton of ship, on a class of mass 0.
        let ships = [ShipRecord {
            cost: 10_000,
            ..ship(128, FAST)
        }];
        let records = upgrades();
        let site = port();
        let pilot = owning(&[(131, 2)]);
        let traded = |source| {
            Yard {
                fields: ShipFields { mass: 0, ..FAST },
                rules: Rulebook::default().with_override(RuleKey::MassPrice, source),
                ..yard(&ships, &records, &site)
            }
            .shipyard(&pilot, &mut DayRolls::default(), &mut NeverFires)
            .expect("a shipyard")
            .trade_in
        };
        assert_eq!(traded(RuleSource::Engine), 2500 + 10, "half of 2 x 10");
        assert_eq!(traded(RuleSource::Bible), 2500);
        assert_eq!(
            trade_in(
                10_000,
                -2,
                &map(&[(131, 2)]),
                &records,
                RuleSource::Engine,
                &Rulebook::default()
            ),
            2510,
            "the cost below mass 0"
        );
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
        assert_eq!(shipyard.can(ShipId(129)), Ok(()), "exactly enough");
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
        assert_eq!(shipyard.can(ShipId(129)), Ok(()));
    }

    // The shipyard's questions.

    /// Each check of `checks` with its citation, in order.
    fn listed_checks<C>(checks: &Checks<C, ShipRefusal>) -> Vec<(&'static str, &'static str)> {
        checks
            .descriptors()
            .map(|about| (about.name(), about.reproduces()))
            .collect()
    }

    /// The disputed rule each check of `checks` reads, in order.
    fn keys<C>(checks: &Checks<C, ShipRefusal>) -> Vec<Option<RuleKey>> {
        checks.descriptors().map(Descriptor::reads).collect()
    }

    #[test]
    fn the_buy_checks_are_flown_for_sale_then_cash() {
        assert_eq!(
            listed_checks(&can_buy()),
            [
                (
                    "flown",
                    "`_CalcShipCanBuy` @0x4f6f5-0x4f6fc: the class selected against the \
                     player's own (+0x76), before the cash"
                ),
                (
                    "for sale",
                    "`_CalcShipCanBuy` `Require` @0x4f80a-0x4f840 and `Availability` \
                     @0x4f8e3-0x4f90a, which the port refuses first; tech, the day's roll and \
                     `HideHigher` as listed (`_SetupPortAvailableShipTypes` @0xbc4b-0xbe2c)"
                ),
                (
                    "cash",
                    "`_CalcShipCanBuy` @0x4f702-0x4f7eb: the price less the trade-in \
                     (@0x4f7db), at least none (`cmovs` @0x4f7e2), against the signed cash \
                     (@0x4f7e5-0x4f7eb)"
                ),
            ]
        );
    }

    /// A shipyard listing ship 129 at 17,500, `buyable` or not, with
    /// `cash` and a 2,500 trade-in.
    fn listing(buyable: bool, cash: i64) -> Shipyard {
        Shipyard {
            rows: vec![ShipRow {
                facts: ShipFacts { buyable },
                ..row(&open(&[heavy()], &pilot()), 129).clone()
            }],
            trade_in: 2500,
            cash,
            current: ShipId(128),
            rules: Rulebook::default(),
        }
    }

    #[test]
    fn the_shipyard_answers_can_from_its_facts_and_cash() {
        let can = |shipyard: Shipyard| shipyard.can(ShipId(129));
        assert_eq!(can(listing(false, 50_000)), Err(ShipRefusal::NotForSale));
        assert_eq!(
            can(listing(false, 0)),
            Err(ShipRefusal::NotForSale),
            "for sale before the cash"
        );
        assert_eq!(can(listing(true, 15_000)), Ok(()), "exactly the price");
        assert_eq!(can(listing(true, 14_999)), Err(ShipRefusal::CannotAfford));
        assert_eq!(
            listing(true, 14_999).check(ShipId(129)),
            Err(ShipRefusal::CannotAfford)
        );
        assert_eq!(can(listing(true, 0)), Err(ShipRefusal::CannotAfford));
        assert_eq!(
            can(listing(true, i64::MAX)),
            Ok(()),
            "any cash above the final price"
        );
        assert_eq!(
            listing(true, 50_000).can(ShipId(130)),
            Err(ShipRefusal::NotListed)
        );
    }

    #[test]
    fn a_pilot_in_debt_cannot_buy_whatever_the_trade_in() {
        let ships = [
            ShipRecord {
                cost: 40_000,
                ..ship(128, FAST)
            },
            cheap(129),
        ];
        let mut debtor = pilot();
        let shipyard = open(&ships, &debtor);
        assert_eq!(shipyard.trade_in, 10_000);
        assert_eq!(shipyard.final_price(row(&shipyard, 129)), -9000);
        let mut check = |cash| {
            debtor.cash = cash;
            open(&ships, &debtor).can(ShipId(129))
        };
        assert_eq!(
            check(-8000),
            Err(ShipRefusal::CannotAfford),
            "the trade-in covers the price and the debt"
        );
        assert_eq!(
            check(-9000),
            Err(ShipRefusal::CannotAfford),
            "exactly the debt the trade-in would clear"
        );
        assert_eq!(check(-1), Err(ShipRefusal::CannotAfford));
        assert_eq!(check(0), Ok(()), "the final price is taken as none");
    }

    #[test]
    fn the_final_price_at_least_none_is_set_against_the_signed_cash() {
        let can = |shipyard: Shipyard| shipyard.can(ShipId(129));
        assert_eq!(
            can(Shipyard {
                trade_in: 20_000,
                ..listing(true, -1)
            }),
            Err(ShipRefusal::CannotAfford)
        );
        assert_eq!(
            can(Shipyard {
                trade_in: 20_000,
                ..listing(true, 0)
            }),
            Ok(())
        );
        assert_eq!(can(listing(true, -15_000)), Err(ShipRefusal::CannotAfford));
        let mut overflowing = Shipyard {
            trade_in: 1,
            ..listing(true, 0)
        };
        overflowing.rows[0].price = i64::MIN;
        assert_eq!(
            overflowing.final_price(&overflowing.rows[0]),
            i64::MIN,
            "the final price saturates"
        );
        assert_eq!(can(overflowing), Ok(()), "a saturated final price is none");
    }

    #[test]
    fn the_class_flown_is_refused_before_for_sale_and_the_cash() {
        let flying = |buyable, cash| Shipyard {
            current: ShipId(129),
            ..listing(buyable, cash)
        };
        let can = |shipyard: Shipyard| shipyard.can(ShipId(129));
        assert_eq!(
            can(flying(false, 0)),
            Err(ShipRefusal::AlreadyFlown),
            "before for sale"
        );
        assert_eq!(
            can(flying(true, 0)),
            Err(ShipRefusal::AlreadyFlown),
            "before the cash"
        );
        assert_eq!(can(flying(true, 50_000)), Err(ShipRefusal::AlreadyFlown));
    }

    #[test]
    fn each_disputed_shipyard_rule_names_its_key() {
        assert_eq!(keys(&can_buy()), [None, None, None]);
        assert_eq!(day_roll().about().reads(), Some(RuleKey::BuyRandom));
        assert_eq!(
            trade_in_value().about().reads(),
            Some(RuleKey::TradeInOutfits)
        );
        assert_eq!(cargo_kept().about().reads(), Some(RuleKey::PurchaseCargo));
        assert_eq!(carried().about().reads(), Some(RuleKey::PurchaseCarry));
        for plain in [
            *required().about(),
            *available().about(),
            *price().about(),
            *cash_after().about(),
            *reserves_full().about(),
        ] {
            assert_eq!(plain.reads(), None, "{}", plain.name());
        }
    }

    #[test]
    fn each_shipyard_context_hands_over_the_rule_set_it_was_given() {
        // Only `cargo_kept` and `carried` read these; this pins the
        // hand-over itself.
        let rules = Rulebook::default()
            .with_override(RuleKey::TradeInOutfits, RuleSource::Bible)
            .with_override(RuleKey::PurchaseCargo, RuleSource::Bible);
        assert_ne!(rules, Rulebook::default());
        let shipyard = Shipyard {
            rules,
            ..listing(true, 50_000)
        };
        assert_eq!(shipyard.rulebook(), &rules);
        let asked = RowShip {
            shipyard: &shipyard,
            row: &shipyard.rows[0],
        };
        assert_eq!(asked.rulebook(), &rules);
        let pilot = pilot();
        let new = heavy();
        let sale = Sale {
            rules,
            ..sale(&pilot, &new, "Kestrel", &[], &[])
        };
        let settled = Settled {
            sale: &sale,
            quote: QUOTE,
            refund: 0,
        };
        assert_eq!(settled.rulebook(), &rules);
        let stats = ShipStats::new(HEAVY, &[]);
        let filled = Filled {
            stats: &stats,
            rules: &rules,
        };
        assert_eq!(filled.rulebook(), &rules);
    }

    #[test]
    fn the_purchase_value_rules_cite_the_originals_addresses() {
        let cited = [
            carried().about().reproduces(),
            cash_after().about().reproduces(),
            cargo_kept().about().reproduces(),
            reserves_full().about().reproduces(),
        ];
        assert_eq!(
            cited,
            [
                "`_DoShipyardDialog` @0x5ee1b-0x5ee4f (the byte +0x378, `_LoadObjectData` \
                 @0x78cad-0x78cb3), the defaults added @0x5ee51-0x5ee97",
                "`_DoShipyardDialog` trade-in @0x5ec96, price @0x5ed54; no other cash write \
                 on the purchase, so the refund (none by the engine) is `purchase_carry`'s \
                 other reading",
                "`_DoShipyardDialog` @0x5ef9f, `_DestroyPartialFleetCargo` @0xcd32-0xcff6, \
                 `_ResetPlayerPrecalcedValues` @0xc7d8-0xc82c",
                "`_DoShipyardDialog` @0x5f051-0x5f07b, after `OnPurchase` @0x5f00d; the \
                 capacities of the class and outfits it leaves (`_EvalMissionBitSetString` \
                 @0x99dea)",
            ]
        );
    }

    #[test]
    fn the_shipyard_value_rules_cite_the_originals_addresses() {
        let cited = [
            required().about().reproduces(),
            available().about().reproduces(),
            day_roll().about().reproduces(),
            price().about().reproduces(),
            trade_in_value().about().reproduces(),
        ];
        assert_eq!(
            cited,
            [
                "`_CalcShipCanBuy` @0x4f80a-0x4f840 (`_PlayerMeetsRequirements` @0x4f833); \
                 `_SetupPortAvailableShipTypes` @0xbce7-0xbd2b",
                "`_CalcShipCanBuy` @0x4f8e3-0x4f90a (`_EvalMissionBitTestString` @0x4f901); \
                 `_SetupPortAvailableShipTypes` @0xbd33-0xbd5b",
                "`_SetupPortAvailableShipTypes` @0xbc88-0xbc9d; `_LoadObjectData` \
                 @0x7a340-0x7a34a",
                "`shïp` `Cost` (`_CalcShipCanBuy` @0x4f74b-0x4f751), without its \
                 `_ApplyPriceAndTechnologyFlux`, which is not modelled",
                "`_PlayerShipTradeInPrice` @0xb079-0xb144, the persistent byte @0xb0da-0xb0e1",
            ]
        );
    }

    #[test]
    fn no_shipyard_rule_draws() {
        for about in can_buy().descriptors() {
            assert!(!about.draws(), "{}", about.name());
        }
        for about in [
            *required().about(),
            *available().about(),
            *day_roll().about(),
            *price().about(),
            *trade_in_value().about(),
            *carried().about(),
            *cash_after().about(),
            *cargo_kept().about(),
            *reserves_full().about(),
        ] {
            assert!(!about.draws(), "{}", about.name());
        }
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

    /// `pilot`'s sale of a FAST ship for `new`, named `name`, whose stock
    /// weapons and ammunition are `fits`, with no escorts, by the default
    /// rule set.
    fn sale<'a>(
        pilot: &'a Pilot,
        new: &'a ShipRecord,
        name: &'a str,
        fits: &'a [StockFit],
        records: &'a [OutfitRecord],
    ) -> Sale<'a> {
        Sale {
            pilot,
            old_mass: FAST.mass,
            new,
            name,
            fits,
            records,
            escorts: &[],
            rules: Rulebook::default(),
        }
    }

    /// Applies `bought` to `pilot` as the session does, the class aside
    /// (`Session::become_class` writes it), and gives what it did.
    fn apply(pilot: &mut Pilot, bought: Purchase) -> ShipPurchase {
        pilot.outfits = bought.outfits;
        bought.settlement.settle(pilot);
        bought.report
    }

    fn buy(pilot: &mut Pilot, new: &ShipRecord, records: &[OutfitRecord]) -> ShipPurchase {
        buy_named(pilot, new, "Kestrel", records)
    }

    /// The default rule set with the other reading of
    /// [`RuleKey::PurchaseCarry`]: the port's earlier carry.
    fn port_carry() -> Rulebook {
        Rulebook::default().with_override(RuleKey::PurchaseCarry, RuleSource::Bible)
    }

    /// Buys `new` as [`buy`] does, carrying by [`port_carry`].
    fn buy_by_the_port(
        pilot: &mut Pilot,
        new: &ShipRecord,
        records: &[OutfitRecord],
    ) -> ShipPurchase {
        let bought = purchase(
            &Sale {
                rules: port_carry(),
                ..sale(pilot, new, "Kestrel", &[], records)
            },
            QUOTE,
        );
        apply(pilot, bought)
    }

    /// Buys `new`, naming it `name`.
    fn buy_named(
        pilot: &mut Pilot,
        new: &ShipRecord,
        name: &str,
        records: &[OutfitRecord],
    ) -> ShipPurchase {
        let bought = purchase(&sale(pilot, new, name, &[], records), QUOTE);
        apply(pilot, bought)
    }

    /// Buys `new` with `escorts`, the cargo kept as `cargo` reads
    /// [`RuleKey::PurchaseCargo`].
    fn buy_by(
        pilot: &mut Pilot,
        new: &ShipRecord,
        records: &[OutfitRecord],
        escorts: &[EscortHolds],
        cargo: RuleSource,
    ) -> ShipPurchase {
        let bought = purchase(
            &Sale {
                escorts,
                rules: Rulebook::default().with_override(RuleKey::PurchaseCargo, cargo),
                ..sale(pilot, new, "Kestrel", &[], records)
            },
            QUOTE,
        );
        apply(pilot, bought)
    }

    /// Buys `new`, whose stock weapons and ammunition are `fits`, by
    /// `rules`.
    fn buy_stocked(
        pilot: &mut Pilot,
        new: &ShipRecord,
        fits: &[StockFit],
        records: &[OutfitRecord],
        rules: Rulebook,
    ) -> ShipPurchase {
        let bought = purchase(
            &Sale {
                rules,
                ..sale(pilot, new, "Kestrel", fits, records)
            },
            QUOTE,
        );
        apply(pilot, bought)
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
        buy_stocked(
            &mut pilot,
            &heavy(),
            &STOCK,
            &armoury(),
            Rulebook::default(),
        );
        assert_eq!(pilot.outfits, map(&[(201, 20), (205, 2)]));
        let with_a_blaster = ShipRecord {
            defaults: vec![(OutfitId(206), 1)],
            ..heavy()
        };
        let mut pilot = self::pilot();
        buy_stocked(
            &mut pilot,
            &with_a_blaster,
            &STOCK,
            &armoury(),
            Rulebook::default(),
        );
        assert_eq!(
            pilot.outfits,
            map(&[(201, 20), (205, 1), (206, 1)]),
            "its default blaster is one of the two"
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
        buy_stocked(&mut pilot, &heavy(), &STOCK, &records, Rulebook::default());
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
            let bought = buy_stocked(&mut pilot, &heavy(), &STOCK, &records, port_carry());
            (pilot.owned(OutfitId(140)), bought.sold_back)
        };
        assert_eq!(fitting(12), (1, BTreeMap::new()));
        assert_eq!(fitting(13), (0, map(&[(140, 1)])));
    }

    #[test]
    fn the_old_ships_stock_weapons_go_in_its_trade_in() {
        assert_eq!(
            trade_in(
                10_000,
                40,
                &map(&[(201, 20), (205, 2)]),
                &armoury(),
                RuleSource::Engine,
                &Rulebook::default()
            ),
            2500 + 2 * 500 + 20 * 5,
            "half of each, like any outfit owned"
        );
    }

    #[test]
    fn buying_replaces_the_ship_and_moves_the_cash_by_the_trade_in_less_the_price() {
        let mut pilot = pilot();
        pilot.cash = 20_000;
        let bought = buy(&mut pilot, &heavy(), &[]);
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
        let bought = purchase(
            &Sale {
                old_mass: 40,
                ..sale(&paid, &heavy(), "Kestrel", &[], &[])
            },
            Quote {
                price: 1000,
                trade_in: 2500,
            },
        );
        apply(&mut paid, bought);
        assert_eq!(paid.cash(), 11_500, "paid the difference");
    }

    #[test]
    fn a_purchase_changes_nothing_until_applied() {
        let records = [
            OutfitRecord {
                cost: 3001,
                ..persistent(140, 5, 9)
            },
            outfit(150, &[(MORE_FUEL, 100)]),
        ];
        let with_tank = ShipRecord {
            defaults: vec![(OutfitId(150), 1)],
            ..heavy()
        };
        let mut pilot = owning(&[(140, 5)]);
        pilot.cargo = BTreeMap::from([(FOOD, 20), (OPALS, 2)]);
        pilot.reserves.fuel.now = 12.0;
        let before = pilot.clone();
        let bought = purchase(
            &Sale {
                rules: port_carry(),
                ..sale(&pilot, &with_tank, "The Kestrel", &[], &records)
            },
            QUOTE,
        );
        assert_eq!(pilot, before, "nothing written");
        assert_eq!(bought.outfits, map(&[(140, 2), (150, 1)]));
        assert_eq!(bought.settlement.cash, 10_000 - 17_500 + 2500 + 4500);
        assert_eq!(bought.settlement.name, "Kestrel");
        assert_eq!(bought.settlement.cargo, BTreeMap::from([(FOOD, 15)]));
        assert_eq!(bought.report.sold_back, map(&[(140, 3)]));
        apply(&mut pilot, bought);
        assert_eq!(pilot.ship(), before.ship(), "the class is not the sale's");
        assert_eq!(pilot.owned(OutfitId(140)), 2);
        assert_eq!(pilot.ship_name(), Some("Kestrel"));
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
    fn by_the_port_outfits_that_are_not_persistent_go_with_the_old_ship() {
        let records = upgrades();
        let mut pilot = owning(&[(128, 2), (130, 1), (131, 1), (999, 1)]);
        let bought = buy_by_the_port(&mut pilot, &heavy(), &records);
        assert_eq!(pilot.outfits().count(), 0);
        assert_eq!(bought.sold_back, BTreeMap::new(), "none sold back");
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
        let bought = buy_by_the_port(&mut pilot, &heavy(), &records);
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
        let bought = buy_by_the_port(&mut pilot, &small, &records);
        assert_eq!(pilot.outfits().collect::<Vec<_>>(), [(OutfitId(150), 1)]);
        assert_eq!(bought.sold_back, map(&[(140, 1)]));
        assert_eq!(bought.refund, 2000);
        assert!(free_mass(small.fields, &map(&[(150, 1)]), &pilot.outfits, &records) >= 0);
        // With a 5-ton one instead, it fits exactly.
        let lighter = [records[0].clone(), persistent(140, 5, 1)];
        let mut pilot = owning(&[(140, 1)]);
        buy_by_the_port(&mut pilot, &small, &lighter);
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
        let bought = buy_by_the_port(&mut pilot, &heavy(), std::slice::from_ref(&by_mass));
        assert_eq!(bought.sold_back, map(&[(140, 1)]));
        assert_eq!(bought.refund, resale(100 * i64::from(FAST.mass)));
        assert_eq!(bought.refund, 2000);
        assert_eq!(pilot.cash(), 10_000 - 17_500 + 2500 + 2000);
    }

    #[test]
    fn a_persistent_outfit_sold_back_from_a_massless_ship_refunds_half_its_cost() {
        // 100 credits a ton of ship and 20 tons: it does not fit HEAVY, and
        // sells back as fitted to the old ship, of mass 0.
        let by_mass = [OutfitRecord {
            cost: 100,
            flags: OutfitFlags::PERSISTENT | OutfitFlags::PRICE_BY_MASS,
            ..persistent(140, 20, 1)
        }];
        let pilot = owning(&[(140, 1)]);
        let new = heavy();
        let bought = |source| {
            purchase(
                &Sale {
                    old_mass: 0,
                    rules: Rulebook::default()
                        .with_override(RuleKey::MassPrice, source)
                        .with_override(RuleKey::PurchaseCarry, RuleSource::Bible),
                    ..sale(&pilot, &new, "Kestrel", &[], &by_mass)
                },
                QUOTE,
            )
        };
        let engine = apply(&mut pilot.clone(), bought(RuleSource::Engine));
        assert_eq!(engine.sold_back, map(&[(140, 1)]));
        assert_eq!(engine.refund, resale(100));
        assert_eq!(engine.refund, 50);
        let other = apply(&mut pilot.clone(), bought(RuleSource::Bible));
        assert_eq!(other.sold_back, map(&[(140, 1)]));
        assert_eq!(other.refund, 0);
    }

    #[test]
    fn a_persistent_outfit_beyond_its_max_is_capped_and_the_rest_sold_back() {
        let records = [
            persistent(140, 0, 2),
            persistent(141, 0, 0),
            persistent(142, 0, -1),
        ];
        let mut pilot = owning(&[(140, 5), (141, 1), (142, 2)]);
        let bought = buy_by_the_port(&mut pilot, &heavy(), &records);
        assert_eq!(pilot.outfits().collect::<Vec<_>>(), [(OutfitId(140), 2)]);
        assert_eq!(bought.sold_back, map(&[(140, 3), (141, 1), (142, 2)]));
        assert_eq!(bought.refund, 6 * 500);
    }

    #[test]
    fn a_persistent_mass_expansion_carries_onto_negative_holds() {
        // The purchase never reads the class's no-expansions flag
        // (@0x5ee1b-0x5ee4f).
        let expansion = OutfitRecord {
            mods: [(MORE_CARGO, -5), (0, 0), (0, 0), (0, 0)],
            ..persistent(140, -5, 1)
        };
        let records = [expansion, persistent(141, 0, 1)];
        let negative = ShipRecord {
            fields: ShipFields { holds: -1, ..HEAVY },
            ..heavy()
        };
        let mut pilot = owning(&[(140, 1), (141, 1)]);
        let bought = buy(&mut pilot, &negative, &records);
        assert_eq!(
            pilot.outfits().collect::<Vec<_>>(),
            [(OutfitId(140), 1), (OutfitId(141), 1)]
        );
        assert_eq!(bought.sold_back, map(&[]));
        assert_eq!(bought.refund, 0);
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
    fn by_the_engine_every_persistent_outfit_carries_over_past_its_max_and_the_free_mass() {
        let records = [
            OutfitRecord {
                cost: 3001,
                ..persistent(140, 5, 2)
            },
            persistent(141, 0, 0),
            persistent(142, 0, -1),
            outfit(150, &[]),
        ];
        let mut pilot = owning(&[(140, 5), (141, 1), (142, 2), (150, 1)]);
        let bought = buy(&mut pilot, &heavy(), &records);
        assert_eq!(pilot.outfits, map(&[(140, 5), (141, 1), (142, 2)]));
        assert_eq!(bought.sold_back, BTreeMap::new());
        assert_eq!(bought.refund, 0);
        assert_eq!(pilot.cash(), 10_000 - 17_500 + 2500);
        assert_eq!(
            free_mass(HEAVY, &BTreeMap::new(), &pilot.outfits, &records),
            12 - 25,
            "below none, as the original allows"
        );
    }

    #[test]
    fn by_the_engine_a_persistent_default_stacks_on_the_units_carried() {
        let with_defaults = ShipRecord {
            defaults: vec![(OutfitId(140), 2)],
            ..heavy()
        };
        let records = [persistent(140, 0, 1)];
        let mut engine = owning(&[(140, 3)]);
        let bought = buy(&mut engine, &with_defaults, &records);
        assert_eq!(engine.owned(OutfitId(140)), 3 + 2, "past the Max both ways");
        assert_eq!(bought.sold_back, BTreeMap::new());
        assert_eq!(bought.refund, 0);
        let mut other = owning(&[(140, 3)]);
        let bought = buy_by_the_port(&mut other, &with_defaults, &records);
        assert_eq!(other.owned(OutfitId(140)), 1 + 2);
        assert_eq!(bought.sold_back, map(&[(140, 2)]));
    }

    #[test]
    fn the_carry_reads_purchase_carry() {
        let records = [OutfitRecord {
            cost: 3001,
            ..persistent(140, 5, 9)
        }];
        let pilot = owning(&[(140, 5)]);
        let new = heavy();
        let bought = |source| {
            purchase(
                &Sale {
                    rules: Rulebook::default().with_override(RuleKey::PurchaseCarry, source),
                    ..sale(&pilot, &new, "Kestrel", &[], &records)
                },
                QUOTE,
            )
        };
        let engine = bought(RuleSource::Engine);
        assert_eq!(engine.outfits, map(&[(140, 5)]));
        assert_eq!(engine.report.sold_back, BTreeMap::new());
        assert_eq!(engine.report.refund, 0);
        assert_eq!(engine.settlement.cash, 10_000 - 17_500 + 2500);
        let other = bought(RuleSource::Bible);
        assert_eq!(other.outfits, map(&[(140, 2)]));
        assert_eq!(other.report.sold_back, map(&[(140, 3)]));
        assert_eq!(other.report.refund, 4500);
        assert_eq!(other.settlement.cash, 10_000 - 17_500 + 2500 + 4500);
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
    fn by_the_engine_an_escort_of_negative_holds_takes_a_share_by_its_size() {
        // `_LoadObjectData` negates a negative `Holds` (@0x7a6a5-0x7a6d3):
        // A = 20, B = 25, f = 0.8. Food keeps trunc(8.0), metal
        // trunc(5.6), and the opals lose trunc(3.2).
        assert_eq!(
            kept(
                &[(FOOD, 10), (METAL, 7), (OPALS, 4)],
                20,
                &[trader(-5)],
                RuleSource::Engine
            ),
            (
                vec![(FOOD, 8), (METAL, 5), (OPALS, 1)],
                BTreeMap::from([(FOOD, 2), (METAL, 2), (OPALS, 3)])
            )
        );
    }

    #[test]
    fn by_the_engine_escorts_of_negative_holds_beyond_the_ship_keep_each_junk_in_part() {
        // A = 10, B = 40, f = 0.25: food keeps trunc(2.5), the opals lose
        // trunc(1.0).
        assert_eq!(
            kept(
                &[(FOOD, 10), (OPALS, 4)],
                10,
                &[trader(-30)],
                RuleSource::Engine
            ),
            (
                vec![(FOOD, 2), (OPALS, 3)],
                BTreeMap::from([(FOOD, 8), (OPALS, 1)])
            )
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
        let bought = buy_by(
            &mut pilot,
            &with_pod,
            &records,
            &[trader(18)],
            RuleSource::Engine,
        );
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
            buy_by(pilot, new, records, &[], RuleSource::Bible)
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
        let records = [outfit(150, &[(MORE_FUEL, 100)])];
        let stats = ShipStats::new(HEAVY, &outfit_mods(&map(&[(150, 1)]), &records));
        assert_eq!(
            filled(&stats, &Rulebook::default()),
            Reserves::full(80.0, 45.0, 400.0)
        );
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

    // The Shipyard Info panel's Space.

    #[test]
    fn space_floors_each_step_at_none_and_keeps_an_untouched_start() {
        assert_eq!(floored_space(22, [10]), 12);
        assert_eq!(floored_space(-2, [10]), 0);
        assert_eq!(floored_space(-5, []), -5);
        assert_eq!(floored_space(2, [10, -3]), 3);
        assert_eq!(floored_space(8, [-3, 10]), 1);
    }

    /// `oütf` `id` of `mass` tons with these mods.
    fn weighing(id: i16, mass: i16, mods: &[(i16, i16)]) -> OutfitRecord {
        OutfitRecord {
            mass,
            ..outfit(id, mods)
        }
    }

    /// Ship `id` of `FreeMass` `free_mass` with these default items.
    fn freeing(id: i16, free_mass: i16, defaults: &[(i16, u16)]) -> ShipRecord {
        ShipRecord {
            defaults: defaults
                .iter()
                .map(|&(outfit, count)| (OutfitId(outfit), count))
                .collect(),
            ..ship(id, ShipFields { free_mass, ..FAST })
        }
    }

    /// An arsenal of `weapons` whose one hull, `ship`, has these stock
    /// slots, each a weapon, its count and its `AmmoLoad`.
    fn stocking(weapons: &[WeaponRecord], ship: i16, slots: &[(i16, i16, i16)]) -> Arsenal {
        Arsenal::new(
            weapons,
            vec![HullRecord {
                weapons: slots
                    .iter()
                    .map(|&(weapon, count, ammo)| StockWeapon {
                        weapon: WeaponId(weapon),
                        count,
                        ammo,
                    })
                    .collect(),
                ..hull(ship)
            }],
        )
    }

    #[test]
    fn the_space_of_a_class_with_stock_is_its_free_mass() {
        // 12 + 2 x 3 + 5 x 1 + 2 to start, then 25 - 6 - 5 - 2.
        let class = freeing(129, 12, &[(152, 1)]);
        let arsenal = stocking(&[weapon(130)], 129, &[(130, 2, 5)]);
        let outfits = [
            weighing(150, 3, &[(MOD_WEAPON, 130)]),
            weighing(151, 1, &[(MOD_AMMO, 130)]),
            weighing(152, 2, &[]),
        ];
        assert_eq!(space(&class, &arsenal, &outfits), 12);
    }

    #[test]
    fn the_space_takes_off_weapons_by_id_not_slot_order() {
        // -10 - 4 + 6 = -8 to start; 130 first leaves 0, then 140 adds 4.
        // By slot order, 140 would leave -4, floored to 0, then 130 0.
        let class = freeing(129, -10, &[]);
        let arsenal = stocking(&[], 129, &[(140, 1, 0), (130, 1, 0)]);
        let outfits = [
            weighing(150, -4, &[(MOD_WEAPON, 140)]),
            weighing(151, 6, &[(MOD_WEAPON, 130)]),
        ];
        assert_eq!(space(&class, &arsenal, &outfits), 4);
    }

    #[test]
    fn the_space_takes_off_weapons_before_default_items() {
        // -10 + 6 - 4 = -8 to start; the weapon leaves 0, the item adds 4.
        let class = freeing(129, -10, &[(152, 1)]);
        let arsenal = stocking(&[], 129, &[(130, 1, 0)]);
        let outfits = [
            weighing(150, 6, &[(MOD_WEAPON, 130)]),
            weighing(152, -4, &[]),
        ];
        assert_eq!(space(&class, &arsenal, &outfits), 4);
    }

    #[test]
    fn the_space_takes_off_ammunition_named_by_the_weapon_not_its_ammo_type() {
        // The standard equipment holds 5 of 132's rounds (2 tons each): 10
        // to start; the walk takes off 5 of 131's (a ton each).
        let class = freeing(129, 0, &[]);
        let launcher = WeaponRecord {
            ammo_type: 4,
            ..weapon(131)
        };
        let arsenal = stocking(&[launcher], 129, &[(131, 1, 5)]);
        let outfits = [
            weighing(150, 1, &[(MOD_AMMO, 131)]),
            weighing(151, 2, &[(MOD_AMMO, 132)]),
        ];
        assert_eq!(space(&class, &arsenal, &outfits), 5);
    }

    #[test]
    fn a_class_without_stock_shows_its_free_mass_as_it_is() {
        assert_eq!(space(&freeing(129, -7, &[]), &Arsenal::default(), &[]), -7);
    }

    #[test]
    fn the_last_stock_slot_naming_a_weapon_sets_its_count() {
        // Slots of 1 and then 3 of gun 130 (2 tons each): the standard
        // equipment holds 3, so 5 + 6 = 11 to start, and the walk takes
        // off the last slot's 3 guns, 6 tons, leaving 5. Summing the
        // slots would take off 8 (leaving 3), and the first slot 2
        // (leaving 9).
        let class = freeing(129, 5, &[]);
        let arsenal = stocking(&[weapon(130)], 129, &[(130, 1, 0), (130, 3, 0)]);
        let outfits = [weighing(150, 2, &[(MOD_WEAPON, 130)])];
        let shown = space(&class, &arsenal, &outfits);
        assert_eq!(shown, 5);
        assert_ne!(shown, floored_space(11, [8]), "summed");
        assert_ne!(shown, floored_space(11, [2]), "the first slot");
    }

    #[test]
    fn the_space_takes_nothing_off_for_stock_of_none() {
        // A slot of no guns and no rounds, and a default item of none,
        // take no step, so `FreeMass` -7 is not floored to 0.
        let class = freeing(129, -7, &[(152, 0)]);
        let arsenal = stocking(&[weapon(130)], 129, &[(130, 0, 0)]);
        let outfits = [
            weighing(150, 3, &[(MOD_WEAPON, 130)]),
            weighing(151, 1, &[(MOD_AMMO, 130)]),
            weighing(152, 2, &[]),
        ];
        assert_eq!(space(&class, &arsenal, &outfits), -7);
    }

    /// The specs of a class of `space`, `holds` and `mass`.
    fn specs(space: i64, holds: i16, mass: i16) -> ShipSpecs {
        ShipSpecs {
            fields: ShipFields {
                holds,
                mass,
                ..FAST
            },
            max_gun: 0,
            max_tur: 0,
            length: 0,
            crew: 0,
            space,
        }
    }

    #[test]
    fn the_spec_lines_values_take_a_ton_at_exactly_one() {
        let shown = |tons, unit| ShownTons { tons, unit };
        assert_eq!(specs(1, 0, 0).space_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, 0, 0).space_shown(), shown(0, TonUnit::Tons));
        assert_eq!(specs(-3, 0, 0).space_shown(), shown(-3, TonUnit::Tons));
        assert_eq!(specs(0, 1, 0).cargo_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, -1, 0).cargo_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, -15, 0).cargo_shown(), shown(15, TonUnit::Tons));
        assert_eq!(specs(0, 0, 1).mass_shown(), shown(1, TonUnit::Ton));
        assert_eq!(specs(0, 0, 25).mass_shown(), shown(25, TonUnit::Tons));
    }

    #[test]
    fn the_shipyard_lists_each_class_with_its_space() {
        // -10 + 6 = -4 to start; the gun leaves 0, where `FreeMass` is -10.
        let ships = [ShipRecord {
            cost: 1000,
            ..freeing(129, -10, &[])
        }];
        let outfits = [weighing(150, 6, &[(MOD_WEAPON, 130)])];
        let arsenal = stocking(&[], 129, &[(130, 1, 0)]);
        let site = port();
        let shipyard = Yard {
            arsenal: &arsenal,
            ..yard(&ships, &outfits, &site)
        }
        .shipyard(&pilot(), &mut DayRolls::default(), &mut NeverFires)
        .expect("a shipyard");
        assert_eq!(row(&shipyard, 129).specs.space, 0);
    }
}
