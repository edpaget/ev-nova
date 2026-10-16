//! The outfitter: which outfits (`oütf`) a stellar lists and sells, at
//! what price and mass, and buying and selling them one at a time.
//!
//! # What a stellar sells
//!
//! An outfit is *for sale* at a stellar whose `TechLevel` is at or above
//! the outfit's, or one of whose eight `SpecialTech` slots equals it
//! exactly, each slot compared raw, so a slot of 0 sells an outfit of
//! `TechLevel` 0. By the engine an outfit of `TechLevel` below 0 or of
//! exactly 0x7fff is never sold by tech (`_SetupPortAvailableItems`
//! @0xbf88-0xbfbd, [`wares::outfit_tech_allows`]); by the Bible it is
//! sold like any other ([`RuleKey::TechLevel`],
//! [`wares::by_tech_level`]). An outfit flagged
//! [`OutfitFlags::HIDE_HIGHER`] that is listed here takes every
//! higher-numbered outfit of the same `DispWeight` off the list
//! (`_SetupPortAvailableItems` @0xc11f-0xc175). That includes one listed
//! but greyed for its `Require` or `Availability`, an owned one, and an
//! owned `SELL_ANYWHERE` or `Flags2` row that tech does not sell.
//!
//! A listed outfit can be *bought* when `_CanBuyOutfitItem`'s own tech
//! test passes it (@0x4e7ff-0x4e83d), its `Require` bits are met and its
//! `Availability` holds. By the engine that tech test is not the
//! listing's: an outfit of `TechLevel` -1 always passes, and any other
//! passes at or below the stellar's tech or by a `SpecialTech` slot, with
//! no sign or 0x7fff test ([`wares::outfit_buy_tech_allows`]); by the
//! Bible it is the listing's own test ([`RuleKey::TechLevel`]). So
//! wherever tech lists an outfit, Buy's tech test passes it too. `Require` is met when each of its
//! bits is in the `Contribute` of the ship or of an outfit the player
//! owns. Its
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
//! Here, by the engine reading of
//! [`RuleKey::DayRolls`](crate::RuleKey::DayRolls), the daily step draws
//! each outfit's roll whose reading is a chance of 1-99, after the ship
//! classes', and a landing keeps it, so an owned outfit's hold lasts the
//! rest of the day too; a roll not drawn since a start or a load is drawn
//! on the caller's [`Chance`] the first time the list asks for it. By the
//! other reading, each roll is drawn the first time the list is built
//! after a landing, by ascending ID and only for an outfit that tech
//! allows and the player does not own, and kept until the next landing.
//! A roll is never saved, as the original rolls again on load. Buying or
//! selling an outfit draws no roll again. An outfit off today is not listed and
//! takes no higher one off sale; one the player owns that is flagged
//! [`OutfitFlags::SELL_ANYWHERE`] is still listed, and can be bought
//! only as `_CanBuyOutfitItem`'s tech test allows (below), unless a lower
//! hider takes it off.
//!
//! # What it lists
//!
//! The outfitter lists every outfit for sale, except one flagged
//! [`OutfitFlags::HIDE_UNLESS_REQUIRED`] whose `Require` is not met, or
//! [`OutfitFlags::HIDE_UNLESS_AVAILABLE`] whose `Availability` does not
//! hold, unless the player owns one. It also lists, at every outfitter,
//! each outfit the player owns that is flagged
//! [`OutfitFlags::SELL_ANYWHERE`], so it can be sold there, unless a
//! lower-numbered [`OutfitFlags::HIDE_HIGHER`] outfit of its `DispWeight`
//! that is listed takes it off. The sweep runs after the `SELL_ANYWHERE`
//! step (@0xc096-0xc0ba, then @0xc11f-0xc175), so such an outfit is not
//! listed and cannot be sold there. Where tech does not sell it, such a
//! row is sell-only unless `_CanBuyOutfitItem`'s own tech test passes it
//! (see Buying and selling). Rows go by `DispWeight`,
//! highest first, then by ID.
//!
//! At a stellar whose `Flags2` has [`BUYS_OWNED_OUTFITS`] (0x0400), every
//! outfit the player owns that is not [`OutfitFlags::CANNOT_SELL`] is for
//! sale, whatever its tech (`_SetupPortAvailableItems` @0xbf51-0xbf75,
//! before its tech test; the stellar's `Flags2` from `_LoadObjectData`
//! @0x77374-0x77378). Owned, it draws no roll and is never hidden by its
//! `Require` or `Availability`; a lower-numbered
//! [`OutfitFlags::HIDE_HIGHER`] outfit of its `DispWeight` still takes it
//! off sale (@0xc158); listed, it hides higher ones of its weight when it
//! is [`OutfitFlags::HIDE_HIGHER`] itself, and it goes by `DispWeight`
//! like any other row.
//! Such a row is sell-only here unless `_CanBuyOutfitItem`'s own tech
//! test passes it (@0x4e7ff-0x4e83d): by the engine, when its `TechLevel`
//! is -1, another negative one at or below the stellar's, or 0x7fff with
//! a slot of 0x7fff or at a stellar of 0x7fff; by the Bible, never, as the
//! listing's test already refused it. The
//! Bible says the stellar "can buy any nonpermanent outfits the player
//! owns"; "nonpermanent" names no `oütf` flag, and the engine reads it as
//! [`OutfitFlags::CANNOT_SELL`], the one bit that forbids a sale.
//!
//! # Price, mass and free mass
//!
//! An outfit's price is its `Cost`, or with [`OutfitFlags::PRICE_BY_MASS`]
//! max(`Cost`, `Cost` × the ship's `Mass`), and 0 for a `Cost` of 0 or
//! less, as [`RuleKey::MassPrice`] reads it (see [`unit_price`]). The
//! original's outfitter never reads the ranks' `_priceFluxMult`, so by the
//! engine's reading of [`RuleKey::RankPriceScope`] buying one costs that
//! price; by the other, the Bible's "items and ships", it costs the price
//! scaled by `_priceFluxMult[1]` alone ([`rank::scaled`](crate::rank::scaled),
//! [`OutfitRow::buy_price`]), which the cash check, the quantity dialog's
//! count, the buy itself and the info box's "Item Price:" read. The
//! resale, the trade-in's outfit lines and every half refund keep the
//! price. Its
//! mass is its `Mass`, or with [`OutfitFlags::MASS_BY_MASS`] and a
//! positive `Mass`, max(`Mass`, the ship's `Mass` times it over 100,
//! truncated), as [`RuleKey::ScaledMass`] reads it (see [`unit_mass`]).
//! The ship's free mass is its `FreeMass`, plus the
//! mass of its standard equipment, its default items and its stock weapons
//! and their `AmmoLoad` (`FreeMass` is space on top of them: the loader
//! adds their mass to it, `_LoadObjectData` @0x7aaf7, @0x7ac61, @0x7ad8d),
//! less the mass of every outfit it carries: a new ship has exactly its
//! `FreeMass` free, and an outfit of negative mass adds space.
//!
//! # Buying and selling
//!
//! Each order buys or sells one, and a refused one changes nothing. Buy
//! is asked of the selected outfit, listed or not: `_CanBuyOutfitItem`
//! reads no list, and after a relisting the selection is `oütf` 128
//! whether or not it is listed (see
//! [`RuleKey::RelistedSelection`]). A
//! buy is refused, checked in `_CanBuyOutfitItem`'s order
//! (@0x4e7c4-0x4ebbc), when `_CanBuyOutfitItem`'s own tech test refuses
//! it (@0x4e7ff-0x4e83d), the player owns its
//! ammunition cap (below) or its `Max` already, it is a gun or a turret
//! past the ship's limit (below), there is not the free mass for it (an
//! outfit of mass 0 or less always fits, even when the free mass is
//! negative, @0x4e88a-0x4e892), it is a fighter whose bays have no room
//! left (their [`capacity`](crate::bay::capacity), the fighters out
//! counted against it, `_CanBuyFighter` @0x5a82; refused as
//! [`OutfitRefusal::BaysFull`]), it is a mass expansion (the last of its
//! mods of [`MORE_CARGO`] has a negative `ModVal`, whatever its `Mass`,
//! @0x4e8bc-0x4e8d2, @0x4e94f-0x4e95a) and either the ship's `Holds` is
//! negative (`_LoadObjectData` flags such a class, @0x7a6a5-0x7a6d3, and
//! only `_CanBuyOutfitItem` reads the flag, @0x4e95c-0x4e97e) or the
//! ship's own free cargo is below the size of that `ModVal` (its cargo
//! space, every owned `ModType` 2 counted, less every commodity and jünk
//! held, the escorts' holds not counted: `_ShipTotalHolds()` less
//! `_TotalCargo(player)`, @0x4e980-0x4e9a8), it is limited to once an
//! opening and was bought since the outfitter opened (below), or the
//! player cannot pay. An
//! outfit whose first mod is `ModType` 3 is never tested as a mass
//! expansion (@0x4e8f6). `Require` (@0x4e9cb-0x4eb37) and `Availability`
//! (@0x4eb51-0x4eb58) come after the once-an-opening check and before
//! the cash, each refused as [`OutfitRefusal::NotForSale`]. The original
//! returns only whether the outfit can be bought, which enables its Buy
//! button, and never shows or tells apart a refusal. A
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
//! [`OutfitFlags::SELL_ANYWHERE`], it lacks the free mass (by the engine,
//! only an outfit of negative mass, when the free mass clamped at 0 plus
//! its mass is below 0, so one of mass 0 or more sells even while the
//! free mass is below 0; by the rulebook's other reading
//! ([`RuleKey::SaleMass`](crate::RuleKey::SaleMass)), whenever the sale
//! would leave the free mass below 0), it is one whose `Max` a
//! `ModType` 27 item raises, while it
//! holds more than that leaves (below), or it is a launcher whose
//! ammunition must be sold first (below). A sale removes one and pays
//! its refund ([`Outfitter::refund`], [`refund`]): by the engine, what
//! was paid for an outfit bought since the outfitter opened (its count
//! owned above the [visit's](OutfitVisit) count owned on entry), and
//! [`RESALE_PERCENT`] of the price for any other; by the rulebook's other
//! reading ([`RuleKey::OutfitRefund`](crate::RuleKey::OutfitRefund)),
//! always [`RESALE_PERCENT`].
//! Selling cargo space below the cargo held is allowed: `_DoOutfitDialog`
//! checks no cargo before a sale. By the engine reading of
//! [`RuleKey::OutfitSaleCargo`](crate::RuleKey::OutfitSaleCargo) the
//! sale's reset then trims the commodities to the holds left (see the
//! session's `precalced` module); by the other, the cargo stays past the
//! hold, the exchange showing no space free, until the next reset.
//!
//! **The info box's words** (`_OutfitDialogUpdate` @0x57dba-0x57e6f,
//! `STR#` 2002 #219-#222) are worked out for each row on their own
//! ([`Outfitter::words`]); the original never asks `_CanBuyOutfitItem`
//! for them, so whatever refuses the buy first plays no part. When
//! `_HasMaxOfItem(item, 0, 0)` holds (@0x57dd6: the ammunition cap, the
//! raised `Max`, the gun and turret limits), its refusal; otherwise, when
//! the outfit's mass is above `_ShipFreeMass`, the free mass clamped at 0
//! ([`Outfitter::free_mass_clamped`]; @0x57e33-0x57e45, @0xb506-0xb50a;
//! so an outfit of mass 0 or less never lacks it), the mass words by the
//! count owned; otherwise none.
//!
//! **Once an opening** (`_boughtMap` and `_boughtID`, [`Bought`], kept
//! on the [`OutfitVisit`] the session opens and hands the outfitter). By
//! the engine, an outfit with a mod of [`EXPLORES_MAP`] (16) in any slot, or
//! failing that of [`CLEAN_RECORD`] (21), is refused while its flag is
//! set, unless its first mod is `ModType` 3 or it has any [`MORE_CARGO`]
//! mod (`_CanBuyOutfitItem` @0x4e8bc-0x4e9c5, after the expansion tests
//! and before the cash). Every buy sets the flags afresh
//! (`Bought::after_buying`, `_GrantOutfitItem` @0x44d67-0x44f8b): a map
//! sets the map flag, a clean record without a [`PAINT`] mod the record
//! flag, and any other outfit clears both, which lifts the limit. Both
//! are cleared each time the outfitter opens (`_DoOutfitDialog`
//! @0x5bb37-0x5bb44), which opens a new visit, so a landing lifts it
//! too; a sale leaves them alone. Only the Buy button shows it: the info
//! box never reads the flags, so its words are unchanged. The other reading has no limit
//! ([`RuleKey::OutfitLimit`](crate::RuleKey::OutfitLimit)). Not modelled:
//! an outfit's `OnPurchase` granting another outfit inside the dialog,
//! which would move the flags in the original.
//!
//! **Counted orders** (`_DoOutfitDialog` @0x5bacf). A plain click buys
//! or sells one (@0x5c1b8, @0x5c69f). With Option held, the quantity
//! dialog asks for a count, opening at the maximum
//! [`Outfitter::count_max`] works out (none when that is 1 or less, and
//! the click moves one), and the count is bought or sold one at a time
//! through the order loop ([`order_loop`](crate::order_loop),
//! [`Session::outfit_counted`](crate::Session::outfit_counted)), a plain
//! click being the loop with a count of 1, when the
//! [`RuleKey::OutfitCount`](crate::RuleKey::OutfitCount) rule asks
//! ([`Outfitter::count_asked`]). The buy loop (@0x5c1bd-0x5c2ff) re-runs
//! `_CanBuyOutfitItem` after each grant and the sell loop
//! (@0x5c6ee-0x5d1b1) checks each sale as a single sale, so each pass is
//! refused where a single order would be: cash, `Max`, free mass, a gun
//! limit or once an opening for a buy, and the free mass, a raised
//! `Max`, a launcher's ammunition or none left for a sale. A counted buy
//! stops at its first refusal; a counted sale runs on past it to the
//! count (@0x5d19c-0x5d1b1). A sale that changes the listing
//! makes the passes left check and sell `oütf` 128 ([`FIRST_OUTFIT`]),
//! listed or not, from index 1 (or 513 when nothing is listed), as
//! [`RuleKey::RelistedSale`](crate::RuleKey::RelistedSale) says. An
//! order that changes the listing, a buy or a sale, leaves the dialog at
//! `oütf` 128 with no cell highlighted, or closes it when nothing is
//! listed ([`Reselection`]), as
//! [`RuleKey::RelistedSelection`]
//! says.
//!
//! **Ammunition cap** (`_HasMaxOfItem` @0x457e-0x45e4, before `Max`). An
//! outfit whose **first** mod is `ModType` 3 naming a weapon of `MaxAmmo`
//! above 0 is capped at that `MaxAmmo` times the player's count of the
//! weapon, stock weapons included; a `ModType` 3 in a later slot is not
//! capped. The product is 16-bit (`imulw` @0x45aa), compared signed, and
//! the buy is refused, as `Max` owned, when the count owned is at least
//! the cap: with no launcher the cap is 0 and none can be bought.
//! `ModType` 27 never raises it. `_CanBuyOutfitItem` asks it (@0x4e87d)
//! before `_CanBuyFighter` (@0x4e938), so a fighter whose bay weapon has a
//! `MaxAmmo` is refused as `Max` owned once the fighters aboard reach
//! `MaxAmmo` x bays, and for want of room only while some are out. In
//! stock data only fighter bays have a `MaxAmmo`. The session works out
//! the caps from the player's armament.
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
//! ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)). In a counted sale
//! each pass checks the whole order, as
//! [`RuleKey::CountedSale`](crate::RuleKey::CountedSale) says. Boarding
//! reads the raised `Max` too (see [`grant`](crate::grant)).
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
//! ([`RuleKey::LauncherSale`](crate::RuleKey::LauncherSale)). In a counted
//! sale each pass checks the whole order, as
//! [`RuleKey::CountedSale`](crate::RuleKey::CountedSale) says.
//! [`Outfitter::lc_names`] gives the names its words need.
//!
//! **Sell's enable and a refusal in the sell loop**
//! ([`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal)). By the engine,
//! the Sell flag alone enables Sell (some owned and not
//! [`OutfitFlags::CANNOT_SELL`], @0x5c563-0x5c595, `_OutfitFilter`
//! @0x5808c-0x580b1; [`Outfitter::can`]): the free mass, a raised `Max`
//! and a launcher's ammunition refuse a sale only inside the sell loop
//! ([`Outfitter::check`]), whose first refusal of an order shows its
//! `STR#` 2002 #207-#212 words once in `DLOG` 3003 (flag -0x63e,
//! @0x5c6ca; `_ShowTextDialog` @0x5a8cf; [`Outfitter::sale_alert`]), at
//! whatever pass it comes, and nothing is sold at that pass. A counted
//! sale runs on past it (@0x5d19c-0x5d1b1), each later pass refused
//! silently, and the sales before it stand. The info box never words them
//! ([`Outfitter::sale_words`]). By the other reading, the port's earlier
//! behaviour, they grey Sell too, and the info box words the first, with
//! no dialog.
//!
//! **The reported cap** ([`Outfitter::cap`]). `_HasMaxOfItem` also
//! reports out a cap, which only the quantity dialog reads: the `Max`,
//! lowered to a smaller ammunition cap, replaced by the raised `Max`, and
//! lowered to the ship's whole gun or turret limit.
//!
//! Which outfits a ship bought in the [`shipyard`](crate::shipyard) keeps is
//! the shipyard's (flag 0x0004); flag 0x0020 only concerns a mission's
//! change of ship, the `H` set operator (see
//! [`ship_change`](crate::ship_change)).

use std::collections::BTreeMap;

use crate::catalog::{
    CronRecord, GovtId, LandingSite, OutfitId, OutfitRecord, RankRecord, WeaponId,
};
use crate::chance::{Chance, NeverFires};
use crate::combat::armament::MOD_AMMO;
use crate::control::Gate;
use crate::fuel::OutfitMod;
use crate::handling::ShipFields;
use crate::landing::StellarFlags;
use crate::market::{Direction, MORE_CARGO};
use crate::order_loop::{OrderRun, Pass};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::rules::{Check, Checks, Context, Value};
use crate::wares::{self, ALWAYS_RANDOM, DayRolls, HideBits, HideHigher, Roll, ShownTons};

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
    /// Stays with the player when a mission's set operator changes its
    /// ship (`H`): see [`ship_change`](crate::ship_change).
    pub const MISSION_PERSISTENT: u16 = 0x0020;
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
    /// Listed among the ranks on the player info's Honors tab, not among
    /// its Extras: see [`player_info`](crate::player_info).
    pub const HONOR: u16 = 0x2000;
    /// Not shown unless its `Availability` holds, or the player owns one.
    pub const HIDE_UNLESS_AVAILABLE: u16 = 0x4000;
}

/// The `spöb` `Flags2` bit of a stellar whose outfitter buys back every
/// outfit the player owns that is not [`OutfitFlags::CANNOT_SELL`],
/// whatever its tech: `_SetupPortAvailableItems` tests it at stellar
/// +0x35 (@0xbf54), the high byte of the `Flags2` word `_LoadObjectData`
/// copies from `spöb` +0x20 (@0x77374-0x77378).
pub const BUYS_OWNED_OUTFITS: u16 = 0x0400;

/// The `oütf` `ModType` that changes the ship's `MaxGun` by its `ModVal`.
pub const MORE_GUNS: i16 = 45;
/// The `oütf` `ModType` that changes the ship's `MaxTur` by its `ModVal`.
pub const MORE_TURRETS: i16 = 46;
/// The `oütf` `ModType` that multiplies the `Max` of the outfit its
/// `ModVal` names.
pub const RAISES_MAX: i16 = 27;

/// The `oütf` `ModType` that explores a map: the outfitter sells one only
/// once an opening ([`Bought`]).
pub const EXPLORES_MAP: i16 = 16;
/// The `oütf` `ModType` that clears the player's legal record: the
/// outfitter sells one only once an opening ([`Bought`]).
pub const CLEAN_RECORD: i16 = 21;
/// The `oütf` `ModType` that paints the ship: an outfit with one never
/// sets the clean-record flag (`Bought::after_buying`).
pub const PAINT: i16 = 43;

/// What an outfit owned when the outfitter opened sells back for, as a
/// percentage of its price: the engine's 0.5 (@0xdd128). One bought since
/// sells back in full by the engine ([`refund`]). The Bible does not say;
/// the community guide (evnova.miraheze.org, "Nova:Making Money") sells
/// the Scarab's Matter/Antimatter Reactor, whose stock `Cost` is
/// 5,000,000, "for 2.5 million credits".
pub const RESALE_PERCENT: i64 = 50;

/// One outfit the player asks to buy or sell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutfitOrder {
    /// The outfit.
    pub outfit: OutfitId,
    /// Buying or selling.
    pub direction: Direction,
}

/// Why an outfit cannot be bought or sold. A buy gives the first refusal
/// in `_CanBuyOutfitItem`'s order (see the module docs). The original
/// names none: it only disables its Buy button (0xded08, @0x5c1c4-0x5c1cc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutfitRefusal {
    /// There is no outfitter: the ship is not landed, or the stellar has
    /// none.
    NoOutfitter,
    /// The outfitter has no row for the outfit: it is neither listed nor
    /// the relisting's [`Outfitter::unlisted`] row, which is checked like
    /// a listed one (see
    /// [`RuleKey::RelistedSelection`]).
    NotListed,
    /// The outfit is listed but cannot be bought here:
    /// `_CanBuyOutfitItem`'s own tech test refuses it, or its `Require` is
    /// not met, or its `Availability` does not hold.
    NotForSale,
    /// The player owns its `Max` already.
    MaxOwned,
    /// Its `Max` is none.
    NoneAllowed,
    /// It is a fighter, and the ship's bays have no room for another
    /// (`_CanBuyFighter` @0x5a82). The original reaches that check only
    /// from `_CanBuyOutfitItem` (@0x4e938), not from `_HasMaxOfItem`, so
    /// its info box gives this none of the #219/#220 words the other
    /// `Max` refusals get; it still shows the mass words #221/#222 when
    /// the fighter also lacks the mass ([`Outfitter::words`]).
    BaysFull,
    /// The ship has not the free mass for another.
    NoSpace,
    /// The ship has not the free mass for one.
    NoSpaceForAny,
    /// It is a mass expansion (the last of its mods of [`MORE_CARGO`] has
    /// a negative `ModVal`), and the ship's `Holds` is negative, which
    /// forbids that (`_CanBuyOutfitItem` @0x4e94f-0x4e97e).
    NoExpansion,
    /// It is a mass expansion, and the ship's own free cargo (its cargo
    /// space less every ton held) is below the size of its `ModVal`
    /// (`_CanBuyOutfitItem` @0x4e980-0x4e9a8).
    NoCargoRoom,
    /// It explores a map ([`EXPLORES_MAP`]) and a map was the last outfit
    /// bought since the outfitter opened (`_boughtMap`), or it clears the
    /// legal record ([`CLEAN_RECORD`]) and a clean record was
    /// (`_boughtID`; `_CanBuyOutfitItem` @0x4e9aa-0x4e9c5). See
    /// [`Bought`].
    BoughtThisOpening,
    /// The player cannot pay for it.
    CannotAfford,
    /// The player owns none.
    NoneOwned,
    /// It can't be sold.
    CannotSell,
    /// It is not for sale here, and cannot be sold anywhere.
    NotBoughtHere,
    /// The ship lacks the free mass to sell it: by the engine, it is of
    /// negative mass and the free mass clamped at 0 plus its mass is
    /// below 0; by the other reading, selling it would leave the free
    /// mass below 0 (see [`RuleKey::SaleMass`](crate::RuleKey::SaleMass)).
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

/// The refusal text a sale refused inside the sell loop shows (`DLOG`
/// 3003, `_DoOutfitDialog` @0x5c735-0x5ce6c; see
/// [`Outfitter::sale_alert`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaleAlert {
    /// The outfit sold.
    pub outfit: OutfitId,
    /// Why: [`OutfitRefusal::NegativeFreeMass`],
    /// [`OutfitRefusal::RaisedFirst`] or [`OutfitRefusal::AmmunitionFirst`].
    pub refusal: OutfitRefusal,
    /// The names the words need, as [`Outfitter::lc_names`] gives them:
    /// the sold outfit's, and the ammunition's or the target's.
    pub names: BTreeMap<OutfitId, LcNames>,
    /// Whether the sold outfit is named in `LCPlural`: passes remain after
    /// the refused one (pass < count - 1, @0x5c9ab-0x5c9dd, @0x5cd53).
    pub plural: bool,
}

/// What an outfit order did: the order loop's run, the alert its
/// refused sale shows, if any ([`Outfitter::sale_alert`]), and what it
/// leaves the outfitter's selection at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitRun {
    /// The passes that went through, and the first refusal.
    pub run: OrderRun<OutfitRefusal>,
    /// The alert the first refused pass shows, if any.
    pub alert: Option<SaleAlert>,
    /// What the order leaves the outfitter's selection at (see
    /// [`RuleKey::RelistedSelection`]).
    pub after: Reselection,
    /// Whether the order's Buy or Sell button passed its own check
    /// ([`Outfitter::can`]), so `_DoOutfitDialog` falls through its loop
    /// to the buy's reset and draw (@0x5c2fc-0x5c2ff, @0x5c305, @0x5c5a1)
    /// or the sale's draw (@0x5d328), however many passes went through.
    pub button_passed: bool,
}

/// What an order leaves the outfitter's selection at, as
/// [`RuleKey::RelistedSelection`]
/// reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Reselection {
    /// The selection is kept: the order left the listing as it was, or
    /// the rule follows its other reading.
    #[default]
    Kept,
    /// The listing changed (`_SetupPortAvailableItems` true,
    /// @0xc216-0xc23d, after the buy loop @0x5c314 or a sale @0x5d00a),
    /// so this outfit, index 0 (`*0xded58` = 0, @0x5c336, @0x5d01c), is
    /// selected, listed or not, with no cell highlighted and the grid at
    /// its top (`*0xded5c` = -1, `*0xdeda8` = 0, @0x5c32c-0x5c33b,
    /// @0x5d021-0x5d02b).
    Reset(OutfitId),
    /// The listing changed and nothing is listed: the leave flag -0x63d
    /// is set (@0x5c40c, @0x5d128), and the outfitter closes once the
    /// click is handled (@0x5da33).
    Leave,
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
    /// Its price here, for one: what its resale, the trade-in and a
    /// refund at half read.
    pub price: i64,
    /// What buying one costs: [`price`](Self::price), scaled by the ranks'
    /// factor by the other reading of
    /// [`RuleKey::RankPriceScope`](crate::RuleKey::RankPriceScope)
    /// ([`rank::scaled`](crate::rank::scaled)).
    pub buy_price: i64,
    /// Its mass, for one, in tons.
    pub mass: i64,
    /// How many the player owns.
    pub owned: u16,
    /// Its `Max`.
    pub max: i16,
    /// What the outfitter's rules read about the outfit, settled when the
    /// outfitter was built.
    pub facts: OutfitFacts,
}

/// What the outfitter's rules read about one outfit, settled when the
/// outfitter is built: whatever needs the gate, the day's rolls or every
/// record is worked out then, so the rules answer from these alone.
// Each flag is its own fact, settled at build and read by its own check.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutfitFacts {
    /// Its own `LCName` and `LCPlural`.
    pub names: LcNames,
    /// Its `Flags` ([`OutfitFlags`]): the rules read
    /// [`GUN`](OutfitFlags::GUN), [`TURRET`](OutfitFlags::TURRET) and
    /// [`CANNOT_SELL`](OutfitFlags::CANNOT_SELL).
    pub flags: u16,
    /// Whether it is in the list: false only for
    /// [`Outfitter::unlisted`].
    pub listed: bool,
    /// Whether `_CanBuyOutfitItem`'s own tech test passes it
    /// (@0x4e7ff-0x4e83d), as [`RuleKey::TechLevel`] reads it.
    pub buy_tech: bool,
    /// Whether its `Require` is met where its `RequireGovt` applies it
    /// (`_CanBuyOutfitItem` @0x4e9cb-0x4eb37).
    pub required: bool,
    /// Whether its `Availability` holds (`_CanBuyOutfitItem`
    /// @0x4eb51-0x4eb58).
    pub available: bool,
    /// Whether it can be sold here at all: for sale here today, by tech or
    /// as an owned outfit at a stellar of `Flags2`
    /// [`BUYS_OWNED_OUTFITS`], or flagged [`OutfitFlags::SELL_ANYWHERE`].
    pub sold_here: bool,
    /// Its ammunition cap, when it has one (`_HasMaxOfItem`
    /// @0x457e-0x45e4; see the module docs), a 16-bit product.
    pub ammo_cap: Option<i16>,
    /// Its `Max` as the [`RAISES_MAX`] outfits owned raise it, when the
    /// multiplier is above 1; `None` when it is its `Max`.
    pub raised: Option<i64>,
    /// When it is a fighter's ammunition, the fighters the ship's bays can
    /// still take.
    pub fighter_room: Option<u32>,
    /// When it is a mass expansion, the cargo space it takes
    /// (`mass_expansion`).
    pub expansion: Option<u16>,
    /// Which once-an-opening flag limits it, if any (`opening_limit`).
    pub opening_limit: Option<OpeningLimit>,
    /// When the player owns it as a launcher, that launcher and the
    /// ammunition it holds.
    pub launcher: Option<Launcher>,
    /// The names of the outfit that names its launcher's ammunition, when
    /// that has a record.
    pub ammo_names: Option<LcNames>,
    /// One for each of its [`RAISES_MAX`] mods, in slot order.
    pub raises: Vec<Raise>,
    /// Its `Mass`, raw: the info box draws its Item Mass and Available
    /// lines only above none (`_OutfitDialogUpdate` @0x57c72).
    pub mass_field: i16,
}

/// One [`RAISES_MAX`] mod of an outfit: the outfit whose `Max` it raises,
/// and what a sale of the raiser reads of it (`_DoOutfitDialog`
/// @0x5c7be-0x5ca6f; see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raise {
    /// The outfit its raw `ModVal` names.
    pub target: OutfitId,
    /// How many of the target the player owns.
    pub held: u16,
    /// The target's `Max`; `None` when it has no record.
    pub target_max: Option<i16>,
    /// The target's raised `Max` by the Bible once one of the raiser is
    /// sold; none when it has no record.
    pub left_by_the_bible: i64,
    /// The target's names, when it has a record.
    pub target_names: Option<LcNames>,
}

/// A stellar's outfitter, as the player sees it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outfitter {
    /// The outfits listed, highest `DispWeight` first, then by ID.
    pub rows: Vec<OutfitRow>,
    /// The row of the outfit asked for when it is not listed: in no
    /// cell, and found only through [`row`](Self::row). It is the outfit
    /// a relisting selects, `oütf` 128 ([`FIRST_OUTFIT`]), which Buy and
    /// Sell answer for listed or not (see
    /// [`RuleKey::RelistedSelection`]).
    pub unlisted: Option<OutfitRow>,
    /// The player's credits.
    pub cash: i64,
    /// The ship's free mass, in tons; negative when it carries more than
    /// it has space for.
    pub free_mass: i64,
    /// The ship's guns and turrets, and its limits for each.
    pub armed: Armed,
    /// The ship class's `Holds`: below none forbids a mass expansion.
    pub holds: i16,
    /// The ship's own free cargo: its cargo space less every ton held
    /// (`_ShipTotalHolds() - _TotalCargo(player)`, @0x4e980-0x4e9a8).
    pub free_cargo: i64,
    /// The visit it was built in: the once-an-opening flags as they
    /// stand, and the counts owned on entry ([`OutfitVisit`]).
    pub visit: OutfitVisit,
    /// The rule set the outfitter was built by, which its questions read.
    pub rules: Rulebook,
}

impl Outfitter {
    /// `outfit`'s row, if it is listed or is the [`unlisted`](Self::unlisted)
    /// row asked for.
    #[must_use]
    pub fn row(&self, outfit: OutfitId) -> Option<&OutfitRow> {
        self.rows
            .iter()
            .chain(self.unlisted.as_ref())
            .find(|row| row.id == outfit)
    }

    /// The ship's free mass as `_ShipFreeMass` reads it, clamped at none
    /// (@0xb506-0xb50a): what the buy's free mass check, the info box's
    /// mass words, its "Available:" line ([`Outfitter::free_mass_shown`])
    /// and the engine's reading of a sale read.
    #[must_use]
    pub fn free_mass_clamped(&self) -> i64 {
        self.free_mass.max(0)
    }

    /// The info box's "Available:" line: the free mass clamped at none,
    /// with "ton" at exactly 1 and "tons" otherwise.
    #[must_use]
    pub fn free_mass_shown(&self) -> ShownTons {
        free_mass_shown().of(self)
    }

    /// The info box's "Item Mass:" line for `outfit`: none for an outfit
    /// of `Mass` 0 or less, which shows no Item Mass or Available line;
    /// otherwise its mass for one, unclamped, with "ton" at exactly 1 and
    /// "tons" otherwise. None for an outfit with no row (neither listed
    /// nor [`unlisted`](Self::unlisted)).
    #[must_use]
    pub fn item_mass_shown(&self, outfit: OutfitId) -> Option<ShownTons> {
        item_mass_shown().of(&self.asked_at(outfit, Pass::SINGLE)?)
    }

    /// `outfit`'s row as the outfitter's rules ask about it at `pass` of
    /// an order, if it has one: listed, or the
    /// [`unlisted`](Self::unlisted) row. The button, the info box and the cap
    /// are not inside the order loop, so they ask at [`Pass::SINGLE`].
    fn asked_at(&self, outfit: OutfitId, pass: Pass) -> Option<RowOutfit<'_>> {
        self.row(outfit).map(|row| RowOutfit {
            outfitter: self,
            row,
            pass,
        })
    }

    /// Whether `order`'s button is enabled, or why not: an outfit that
    /// is neither listed nor the relisting's [`unlisted`](Self::unlisted)
    /// row is [`OutfitRefusal::NotListed`], and the `unlisted` row is
    /// checked like a listed one (see
    /// [`RuleKey::RelistedSelection`]);
    /// a buy is refused by the
    /// first of `_CanBuyOutfitItem`'s checks that refuses it, in the
    /// port's order (see the module docs); a sale by the Sell flag's
    /// checks (some owned, sellable, sold here) and then, as
    /// [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal) says, by the
    /// engine nothing more, so a sale only the sell loop refuses leaves
    /// Sell enabled, or by the other reading the sell loop's checks too
    /// (the free mass, a raised `Max`, a launcher's ammunition).
    pub fn can(&self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        let asked = self
            .asked_at(order.outfit, Pass::SINGLE)
            .ok_or(OutfitRefusal::NotListed)?;
        match order.direction {
            Direction::Buy => refusal(&can_buy(), &asked),
            Direction::Sell => {
                refusal(&sell_flag(), &asked)?;
                if sale_checks_on_the_button().of(&asked) {
                    refusal(&sale_checks(), &asked)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Whether `order` goes through when clicked, or why not, whatever
    /// the reading: [`check_at`](Self::check_at) at a single pass
    /// ([`Pass::SINGLE`]).
    pub fn check(&self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        self.check_at(order, Pass::SINGLE)
    }

    /// Whether `order` goes through at `pass` of the order loop, or why
    /// not, whatever the [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal)
    /// reading: an outfit with no row (neither listed nor
    /// [`unlisted`](Self::unlisted)) is [`OutfitRefusal::NotListed`], and
    /// the `unlisted` row is checked like a listed one; a buy as [`can`](Self::can) says, whatever the pass; a
    /// sale by the Sell flag's checks, then the sell loop's (the free
    /// mass, a raised `Max`, a launcher's ammunition), as each pass of
    /// `_DoOutfitDialog`'s sell loop checks it (@0x5c735-0x5ce6c), a
    /// raised `Max` and a launcher's ammunition reading the pass as
    /// [`RuleKey::CountedSale`](crate::RuleKey::CountedSale) says.
    pub fn check_at(&self, order: OutfitOrder, pass: Pass) -> Result<(), OutfitRefusal> {
        let asked = self
            .asked_at(order.outfit, pass)
            .ok_or(OutfitRefusal::NotListed)?;
        match order.direction {
            Direction::Buy => refusal(&can_buy(), &asked),
            Direction::Sell => {
                refusal(&sell_flag(), &asked)?;
                refusal(&sale_checks(), &asked)
            }
        }
    }

    /// Which sale refusal the info box words for `outfit` (`STR#` 2002
    /// #207-#212), as [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal)
    /// says: by the engine none, as `_OutfitDialogUpdate`'s only words are
    /// #213-#222 (@0x57a12-0x57e68); by the other reading the first of
    /// the sell loop's checks that refuses it, when the Sell flag is set.
    /// `None` too for an outfit with no row (neither listed nor
    /// [`unlisted`](Self::unlisted)).
    #[must_use]
    pub fn sale_words(&self, outfit: OutfitId) -> Option<OutfitRefusal> {
        self.asked_at(outfit, Pass::SINGLE)
            .and_then(|asked| sale_words().of(&asked))
    }

    /// The alert a sale refused inside the sell loop shows at `pass`, as
    /// [`RuleKey::SaleRefusal`](crate::RuleKey::SaleRefusal) says: by the
    /// engine, once an order at its first refused pass, the sell loop's
    /// refusal of a sale whose Sell flag is set, with the names its words need and
    /// the sold outfit named in the plural while passes remain
    /// (`_DoOutfitDialog` @0x5c735-0x5ce6c, @0x5c9ab-0x5c9dd, shown once
    /// an order by the flag -0x63e @0x5c6ca, in `DLOG` 3003 by
    /// `_ShowTextDialog` @0x5a8cf); by the other reading none. `None` for
    /// a buy, an outfit with no row (neither listed nor
    /// [`unlisted`](Self::unlisted)), a Sell-flag refusal, a sale that goes
    /// through, and a pass after the order's first refused one
    /// ([`Pass::alerted`]).
    #[must_use]
    pub fn sale_alert(&self, order: OutfitOrder, pass: Pass) -> Option<SaleAlert> {
        let asked = self.asked_at(order.outfit, pass)?;
        sale_alert_value().of(&SaleAsk {
            asked,
            direction: order.direction,
        })
    }

    /// Which of the info box's words (`STR#` 2002 #219-#222) `outfit`
    /// gets, whatever [`can`](Self::can) says of a buy: the refusal
    /// `_HasMaxOfItem` makes, if any (a `Max`, none allowed, the
    /// ammunition cap, a gun or turret limit); otherwise
    /// [`OutfitRefusal::NoSpaceForAny`] with none owned or
    /// [`OutfitRefusal::NoSpace`] with some, when it lacks the mass;
    /// otherwise none (`_OutfitDialogUpdate` @0x57dba-0x57e6f, which never
    /// consults `_CanBuyOutfitItem`; see the module docs). `None` too when
    /// it has no row (neither listed nor [`unlisted`](Self::unlisted)).
    #[must_use]
    pub fn words(&self, outfit: OutfitId) -> Option<OutfitRefusal> {
        self.asked_at(outfit, Pass::SINGLE)
            .and_then(|asked| info_words().of(&asked))
    }

    /// The `Max` that `_HasMaxOfItem(item, 0, &cap)` reports out for
    /// `outfit` (@0x4512), which the quantity dialog's buy maximum reads:
    /// its `Max`, as the ammunition cap, a raised `Max` and the gun and
    /// turret limits change it (see [`count_max`](Self::count_max));
    /// `None` when it has no row (neither listed nor
    /// [`unlisted`](Self::unlisted)).
    #[must_use]
    pub fn cap(&self, outfit: OutfitId) -> Option<i16> {
        self.asked_at(outfit, Pass::SINGLE)
            .map(|asked| reported_cap().of(&asked))
    }

    /// What selling one of `outfit` refunds, as
    /// [`RuleKey::OutfitRefund`](crate::RuleKey::OutfitRefund) says: by
    /// the engine, the full price for one bought since the visit opened
    /// (its count owned above the [visit's](OutfitVisit) count on entry),
    /// otherwise half; by the other reading, always half (see
    /// [`refund`]). `None` when it has no row (neither listed nor
    /// [`unlisted`](Self::unlisted)).
    #[must_use]
    pub fn refund(&self, outfit: OutfitId) -> Option<i64> {
        self.asked_at(outfit, Pass::SINGLE)
            .map(|asked| refund_value().of(&asked))
    }

    /// The `LCName` and `LCPlural` of each outfit the words of
    /// [`check`](Self::check)'s refusal of `order` name (the click's,
    /// whatever the reading), worked out from that
    /// refusal: a launcher refused for its ammunition, and the outfit
    /// naming that ammunition when it has a record; a raiser refused for
    /// its target, and that target when it has a record. None for any
    /// other answer.
    #[must_use]
    pub fn lc_names(&self, order: OutfitOrder) -> BTreeMap<OutfitId, LcNames> {
        let Some(row) = self.row(order.outfit) else {
            return BTreeMap::new();
        };
        names_for(row, self.check(order))
    }

    /// The count the quantity dialog opens at when `outfit` is bought or
    /// sold with Option held, as `_DoOutfitDialog` works it out; `None`
    /// when it opens none and the click moves one, as it does at a count
    /// of 1 or less (see the module docs' "Counted orders").
    ///
    /// A buy opens only while the Buy flag is set
    /// ([`can`](Self::can) allows it). Its count is the cash over the
    /// price, truncated, at most [`COUNT_CAP`] (that cap when the price
    /// is nothing or less, @0x5c0ca-0x5c11f); at most `Max` less those
    /// owned when `Max` is above those owned (@0x5c124-0x5c156); and at
    /// most the [reported cap](Self::cap) and that cap less those
    /// owned (@0x5c170-0x5c19d). Those owned read as 16 bits, signed.
    /// The engine also skips the dialog when the count is 1 or less
    /// before the cap is read (@0x5c159), which the cap could only lower.
    ///
    /// A sale opens while the Sell flag is set: some owned and not
    /// [`OutfitFlags::CANNOT_SELL`] (@0x5c563-0x5c595, and each time the
    /// list is redrawn, `_OutfitFilter` @0x5808c-0x580b1). Free mass, a
    /// raised `Max` and a launcher's ammunition refuse a sale only inside
    /// the sell loop, so a sale they refuse still opens the dialog. Its
    /// count is those owned, read as 16 bits, signed, at most
    /// [`COUNT_CAP`] (@0x5c5ed-0x5c617).
    #[must_use]
    pub fn count_max(&self, outfit: OutfitId, direction: Direction) -> Option<u32> {
        let row = self.row(outfit)?;
        let owned = i64::from(row.owned.cast_signed());
        let most = match direction {
            Direction::Buy => {
                self.can(OutfitOrder { outfit, direction }).ok()?;
                let mut most = if row.buy_price > 0 {
                    (self.cash / row.buy_price).min(COUNT_CAP)
                } else {
                    COUNT_CAP
                };
                let max = i64::from(row.max);
                if max > owned {
                    most = most.min(max - owned);
                }
                let cap = i64::from(self.cap(outfit)?);
                most.min(cap).min(cap - owned)
            }
            Direction::Sell => {
                let asked = self.asked_at(outfit, Pass::SINGLE)?;
                refusal(&sell_flag(), &asked).ok()?;
                owned.min(COUNT_CAP)
            }
        };
        u32::try_from(most).ok().filter(|&most| most > 1)
    }

    /// The count Option (Alt here) asks for when `outfit` is bought or
    /// sold: by the engine's reading of
    /// [`RuleKey::OutfitCount`](crate::RuleKey::OutfitCount), the quantity
    /// dialog opens at [`count_max`](Self::count_max), and with `None` the
    /// click moves one; by the other reading it never opens.
    #[must_use]
    pub fn count_asked(&self, outfit: OutfitId, direction: Direction) -> Option<u32> {
        if option_asks_count().of(self) {
            self.count_max(outfit, direction)
        } else {
            None
        }
    }
}

/// The `LCName` and `LCPlural` of each outfit the words of `refused`,
/// `row`'s refusal, name (see [`Outfitter::lc_names`]): a launcher
/// refused for its ammunition and the outfit naming that ammunition, or
/// a raiser refused for its target and that target, each when it has a
/// record. None for any other answer.
fn names_for(row: &OutfitRow, refused: Result<(), OutfitRefusal>) -> BTreeMap<OutfitId, LcNames> {
    let facts = &row.facts;
    let named = match refused {
        Err(OutfitRefusal::AmmunitionFirst { ammo, .. }) => ammo.zip(facts.ammo_names.clone()),
        Err(OutfitRefusal::RaisedFirst { target, .. }) => facts
            .raises
            .iter()
            .find(|raise| raise.target == target)
            .and_then(|raise| raise.target_names.clone())
            .map(|names| (target, names)),
        _ => return BTreeMap::new(),
    };
    [(row.id, facts.names.clone())]
        .into_iter()
        .chain(named)
        .collect()
}

/// The free mass the info box's "Available:" line shows
/// ([`Outfitter::free_mass_shown`]).
fn free_mass_shown() -> Value<Outfitter, ShownTons> {
    Value::plain(
        "free mass shown",
        "`_OutfitDialogUpdate` @0x57d66-0x57d9d: `_ShipFreeMass` read as `free_mass_clamped` \
         (`cmovs` @0x57d74), `_tonString` (`STR#` 2002 #1) at exactly 1 @0x57d8b-0x57d95, else \
         `_tonsString` (#2)",
        clamped_tons,
    )
}

/// The mass the info box's "Item Mass:" line shows
/// ([`Outfitter::item_mass_shown`]).
fn item_mass_shown<'a>() -> Value<RowOutfit<'a>, Option<ShownTons>> {
    Value::plain(
        "item mass shown",
        "`_OutfitDialogUpdate` @0x57c67-0x57d19: no Item Mass or Available line for an outfit of \
         `Mass` 0 or less (`jle` @0x57c78 to @0x57da2); otherwise `_AdjustedItemMass` unclamped \
         (@0x57cec), `_tonString` (`STR#` 2002 #1) at exactly 1 @0x57d07-0x57d19, else \
         `_tonsString` (#2)",
        row_tons,
    )
}

/// The row's mass with its unit, when its outfit's `Mass` is above none.
fn row_tons(asked: &RowOutfit) -> Option<ShownTons> {
    (asked.row.facts.mass_field > 0).then(|| ShownTons::of(asked.row.mass))
}

/// The clamped free mass, with its unit.
fn clamped_tons(outfitter: &Outfitter) -> ShownTons {
    ShownTons::of(outfitter.free_mass_clamped())
}

/// Whether Option (Alt here) on Buy or Sell asks for a count, as
/// [`RuleKey::OutfitCount`](crate::RuleKey::OutfitCount) says: by the
/// engine it opens the quantity dialog at [`Outfitter::count_max`]; by
/// the other reading each click moves one.
fn option_asks_count() -> Value<Outfitter, bool> {
    Value::disputed(
        "option asks count",
        "`_DoOutfitDialog` @0x5c0c0-0x5c1aa, @0x5c5e3-0x5c628",
        RuleKey::OutfitCount,
        asks_a_count,
        moves_one,
    )
}

/// The engine's Option: a count.
fn asks_a_count(_: &Outfitter) -> bool {
    true
}

/// The other reading's Option: one a click.
fn moves_one(_: &Outfitter) -> bool {
    false
}

/// The most the quantity dialog offers, bought or sold (`0x7d00`,
/// `_DoOutfitDialog` @0x5c116, @0x5c606).
pub const COUNT_CAP: i64 = 32_000;

/// One listed outfit, asked whether it can be bought or sold, its words
/// and its cap: the facts the outfitter's checks and value rules read,
/// all settled when the outfitter was built.
#[derive(Clone, Copy)]
struct RowOutfit<'a> {
    /// The outfitter: its cash, free mass, guns, holds, free cargo, the
    /// once-an-opening flags and the rule set.
    outfitter: &'a Outfitter,
    /// The row: its price, mass, count owned, `Max` and facts.
    row: &'a OutfitRow,
    /// The pass of the order it is asked at: [`Pass::SINGLE`] outside
    /// the order loop.
    pass: Pass,
}

impl Context for Outfitter {
    fn rulebook(&self) -> &Rulebook {
        &self.rules
    }
}

impl Context for RowOutfit<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.outfitter.rulebook()
    }
}

/// One listed outfit asked about one pass of an order (its row's
/// [`pass`](RowOutfit::pass)): what the sale alert reads
/// ([`Outfitter::sale_alert`]).
#[derive(Clone, Copy)]
struct SaleAsk<'a> {
    /// The outfit, as its row is asked.
    asked: RowOutfit<'a>,
    /// Buying or selling.
    direction: Direction,
}

impl Context for SaleAsk<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.asked.rulebook()
    }
}

/// The first of `checks` that refuses `facts`, if any.
fn refusal<C: Context>(checks: &Checks<C, OutfitRefusal>, facts: &C) -> Result<(), OutfitRefusal> {
    // No outfitter check draws (`no_outfitter_check_draws`), so no chance
    // is ever asked.
    checks.first_refusal(facts, &mut NeverFires)
}

/// `Ok` when `holds`, else `refusal`.
fn holds(holds: bool, refusal: OutfitRefusal) -> Result<(), OutfitRefusal> {
    if holds { Ok(()) } else { Err(refusal) }
}

/// Whether Buy is enabled for an outfit, as `_CanBuyOutfitItem` has it
/// (@0x4e7c4-0x4ebbc), asking no list: its own tech test, [`has_max_checks`], the free mass, the bays' room, the two
/// mass-expansion tests, the once-an-opening flags, `Require`,
/// `Availability` and the cash.
fn can_buy<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    let [ammunition, raised, guns, turrets] = has_max_checks();
    [
        Check::plain(
            "tech",
            "`_CanBuyOutfitItem` @0x4e7ff-0x4e83d, as `buy tech` reads it at build \
             (`TechLevel` -1 skips it @0x4e802)",
            tech_buys,
        ),
        ammunition,
        raised,
        guns,
        turrets,
        Check::plain(
            "free mass",
            "`_CanBuyOutfitItem` @0x4e88a-0x4e897, reads `free_mass_clamped` \
             (`_ShipFreeMass` @0x4e7e1)",
            mass_fits,
        ),
        Check::plain(
            "bay room",
            "`_CanBuyOutfitItem` @0x4e8f6-0x4e93f, `_CanBuyFighter` @0x5a82",
            bay_room,
        ),
        Check::plain(
            "expansion allowed",
            "`_CanBuyOutfitItem` @0x4e94f-0x4e97e",
            expansion_allowed,
        ),
        Check::plain(
            "cargo room",
            "`_CanBuyOutfitItem` @0x4e980-0x4e9a8",
            cargo_room,
        ),
        Check::disputed(
            "once an opening",
            "`_CanBuyOutfitItem` @0x4e9aa-0x4e9c5",
            RuleKey::OutfitLimit,
            not_bought_this_opening,
            never_limited,
        ),
        Check::plain(
            "required",
            "`_CanBuyOutfitItem` @0x4e9cb-0x4eb37",
            requirement_holds,
        ),
        Check::plain(
            "available",
            "`_CanBuyOutfitItem` @0x4eb51-0x4eb58",
            availability_holds,
        ),
        Check::plain(
            "cash",
            "`_CanBuyOutfitItem` @0x4eb95-0x4ebad",
            cash_for_the_price,
        ),
    ]
    .into()
}

/// `_HasMaxOfItem`'s checks (@0x4512), as a list of their own: the
/// refusal the info box's words read first ([`info_words`]).
fn has_max<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    has_max_checks().into()
}

/// `_HasMaxOfItem(item, 0, 0)`'s checks, in its order: the ammunition
/// cap, the raised `Max`, then the gun and turret limits (see the module
/// docs).
fn has_max_checks<'a>() -> [Check<RowOutfit<'a>, OutfitRefusal>; 4] {
    [
        Check::plain(
            "ammunition cap",
            "`_HasMaxOfItem` @0x457e-0x45e4, via `_CanBuyOutfitItem` @0x4e87d",
            under_ammo_cap,
        ),
        Check::plain(
            "raised max",
            "`_HasMaxOfItem` @0x45e9-0x46c3, the multiplier settled at build",
            under_raised_max,
        ),
        Check::plain(
            "gun limit",
            "`_HasMaxOfItem` @0x46c8-0x47b7",
            under_gun_limit,
        ),
        Check::plain(
            "turret limit",
            "`_HasMaxOfItem` @0x47c7-0x4866",
            under_turret_limit,
        ),
    ]
}

/// Whether the Sell flag is set for an outfit, as `_DoOutfitDialog`
/// (@0x5c563-0x5c595) and `_OutfitFilter` (@0x5808c-0x580b1) set it: some
/// owned and not [`OutfitFlags::CANNOT_SELL`]; and, the port's own, the
/// outfit sold here.
fn sell_flag<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    [
        Check::plain(
            "owned",
            "`_DoOutfitDialog` @0x5c563-0x5c570; `_OutfitFilter` @0x5808c-0x58096",
            owned,
        ),
        Check::plain(
            "can be sold",
            "`_DoOutfitDialog` @0x5c572-0x5c595; `_OutfitFilter` @0x58098-0x580b1",
            sellable,
        ),
        Check::plain(
            "sold here",
            "port-only: the row as listed (`_SetupPortAvailableItems` @0xc0a8-0xc0ba \
             lists an owned `SELL_ANYWHERE` outfit). The original's Sell flag asks no list",
            sold_here,
        ),
    ]
    .into()
}

/// The checks of `_DoOutfitDialog`'s sell loop, made on each pass and
/// not before the quantity dialog opens, in its order: the free mass,
/// a raised `Max`, then a launcher's ammunition (see the module docs).
fn sale_checks<'a>() -> Checks<RowOutfit<'a>, OutfitRefusal> {
    [
        Check::disputed(
            "free mass",
            "`_DoOutfitDialog` @0x5c735-0x5c7ac. The engine reads `free_mass_clamped` \
             (`_ShipFreeMass` @0x5c740), and the other reading reads `free_mass`",
            RuleKey::SaleMass,
            clamped_mass_left,
            mass_left,
        ),
        Check::disputed(
            "raised first",
            "`_DoOutfitDialog` @0x5c7be-0x5ca6f. The engine's left is \
             `target_max x raisers_left`, and the other's is `left_by_the_bible`",
            RuleKey::RaisedMax,
            engine_raised_first,
            bible_raised_first,
        ),
        Check::disputed(
            "ammunition first",
            "`_DoOutfitDialog` @0x5ca75-0x5cbe0, through \
             `Launcher::excess_left(launchers_left, Engine / Bible)`",
            RuleKey::LauncherSale,
            engine_ammunition_first,
            bible_ammunition_first,
        ),
    ]
    .into()
}

/// Whether the sell loop's checks ([`sale_checks`]) also decide Sell's
/// enable, as [`RuleKey::SaleRefusal`] says: by the engine they run only
/// inside the sell loop, so the Sell flag alone enables Sell; by the
/// other reading they grey it too. Each check keeps its own descriptor
/// and key: this value only says whether the list runs.
fn sale_checks_on_the_button<'a>() -> Value<RowOutfit<'a>, bool> {
    Value::disputed(
        "sale checks on the button",
        "`_DoOutfitDialog` @0x5c563-0x5c595 and `_OutfitFilter` @0x5808c-0x580b1 set \
         the Sell flag by the count owned and `CANNOT_SELL` alone; the sale checks run \
         only in the sell loop, @0x5c735-0x5ce6c",
        RuleKey::SaleRefusal,
        loop_only,
        before_the_loop,
    )
}

/// The engine's Sell enable: the sale checks run in the loop only.
fn loop_only(_: &RowOutfit) -> bool {
    false
}

/// The other reading's Sell enable: the sale checks grey Sell too.
fn before_the_loop(_: &RowOutfit) -> bool {
    true
}

/// The info box's sale words for an outfit ([`Outfitter::sale_words`]).
fn sale_words<'a>() -> Value<RowOutfit<'a>, Option<OutfitRefusal>> {
    Value::disputed(
        "sale words",
        "`_OutfitDialogUpdate` @0x57a12-0x57e68: only #213-#222",
        RuleKey::SaleRefusal,
        no_sale_words,
        first_sale_refusal,
    )
}

/// The engine's info box: no sale words.
fn no_sale_words(_: &RowOutfit) -> Option<OutfitRefusal> {
    None
}

/// The other reading's info box: the first sale check's refusal, when
/// the Sell flag is set.
fn first_sale_refusal(asked: &RowOutfit) -> Option<OutfitRefusal> {
    refusal(&sell_flag(), asked).ok()?;
    refusal(&sale_checks(), asked).err()
}

/// The alert a refused sale shows ([`Outfitter::sale_alert`]).
fn sale_alert_value<'a>() -> Value<SaleAsk<'a>, Option<SaleAlert>> {
    Value::disputed(
        "sale alert",
        "`_DoOutfitDialog` @0x5c735-0x5ce6c, -0x63e @0x5c6ca; `_ShowTextDialog` @0x5a8cf",
        RuleKey::SaleRefusal,
        alert_at_first_refusal,
        never_alerts,
    )
}

/// The engine's alert: at the order's first refused pass, whatever its
/// index, the sell loop's refusal of a sale whose Sell flag is set. A
/// pass after it ([`Pass::alerted`], flag -0x63e) is refused silently.
fn alert_at_first_refusal(ask: &SaleAsk) -> Option<SaleAlert> {
    let asked = &ask.asked;
    if ask.direction != Direction::Sell || asked.pass.alerted {
        return None;
    }
    refusal(&sell_flag(), asked).ok()?;
    let refused = refusal(&sale_checks(), asked).err()?;
    Some(SaleAlert {
        outfit: asked.row.id,
        refusal: refused,
        names: names_for(asked.row, Err(refused)),
        plural: passes_remain(asked.pass),
    })
}

/// The other reading's alert: none.
fn never_alerts(_: &SaleAsk) -> Option<SaleAlert> {
    None
}

/// Whether passes remain after `pass`: pass < count - 1
/// (@0x5c9ab-0x5c9dd), none past the count.
fn passes_remain(pass: Pass) -> bool {
    pass.count.saturating_sub(pass.index) > 1
}

/// `_CanBuyOutfitItem`'s own tech test passes the outfit; refused as
/// [`OutfitRefusal::NotForSale`].
fn tech_buys(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.buy_tech, OutfitRefusal::NotForSale)
}

/// The outfit's `Require` is met where it applies; refused as
/// [`OutfitRefusal::NotForSale`].
fn requirement_holds(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.required, OutfitRefusal::NotForSale)
}

/// The outfit's `Availability` holds; refused as
/// [`OutfitRefusal::NotForSale`].
fn availability_holds(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.available, OutfitRefusal::NotForSale)
}

/// The count owned is below the ammunition cap, if it has one, compared
/// signed in 16 bits (`_HasMaxOfItem` @0x45cd-0x45d1); refused as `Max`
/// owned.
fn under_ammo_cap(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let capped = asked
        .row
        .facts
        .ammo_cap
        .is_some_and(|cap| i32::from(asked.row.owned) >= i32::from(cap));
    holds(!capped, OutfitRefusal::MaxOwned)
}

/// The count owned is below the raised `Max` (its `Max` when nothing
/// raises it); refused as none allowed when the `Max` is none or less.
fn under_raised_max(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let row = asked.row;
    let max = row.facts.raised.unwrap_or(i64::from(row.max));
    if i64::from(row.owned) >= max {
        Err(if row.max <= 0 {
            OutfitRefusal::NoneAllowed
        } else {
            OutfitRefusal::MaxOwned
        })
    } else {
        Ok(())
    }
}

/// A gun fits the ship's gun limit.
fn under_gun_limit(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let armed = asked.outfitter.armed;
    let full = asked.row.facts.flags & OutfitFlags::GUN != 0 && armed.guns >= armed.gun_limit;
    holds(!full, OutfitRefusal::GunLimit)
}

/// A turret fits the ship's turret limit.
fn under_turret_limit(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let armed = asked.outfitter.armed;
    let full =
        asked.row.facts.flags & OutfitFlags::TURRET != 0 && armed.turrets >= armed.turret_limit;
    holds(!full, OutfitRefusal::TurretLimit)
}

/// The outfit fits the free mass ([`lacks_the_mass`]).
fn mass_fits(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    lacks_the_mass(asked).map_or(Ok(()), Err)
}

/// The mass words, when the outfit's mass is above the free mass clamped
/// at none ([`Outfitter::free_mass_clamped`]): so one of mass 0 or less
/// always fits, even when the free mass is negative (`_CanBuyOutfitItem`
/// @0x4e88a-0x4e892, `_OutfitDialogUpdate` @0x57e3c-0x57e45). By the
/// count owned: [`OutfitRefusal::NoSpaceForAny`] with none,
/// [`OutfitRefusal::NoSpace`] with some.
fn lacks_the_mass(asked: &RowOutfit) -> Option<OutfitRefusal> {
    (asked.row.mass > asked.outfitter.free_mass_clamped()).then_some(if asked.row.owned == 0 {
        OutfitRefusal::NoSpaceForAny
    } else {
        OutfitRefusal::NoSpace
    })
}

/// A fighter's bays have room for another.
fn bay_room(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.row.facts.fighter_room != Some(0),
        OutfitRefusal::BaysFull,
    )
}

/// A mass expansion is allowed: the ship's `Holds` is not negative.
fn expansion_allowed(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let forbidden = asked.row.facts.expansion.is_some() && asked.outfitter.holds < 0;
    holds(!forbidden, OutfitRefusal::NoExpansion)
}

/// A mass expansion has the ship's own free cargo of its size.
fn cargo_room(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let short = asked
        .row
        .facts
        .expansion
        .is_some_and(|size| asked.outfitter.free_cargo < i64::from(size));
    holds(!short, OutfitRefusal::NoCargoRoom)
}

/// The engine's limit: the once-an-opening flag that limits the outfit,
/// if any, is clear.
fn not_bought_this_opening(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let refused = asked
        .outfitter
        .visit
        .bought
        .refuses(asked.row.facts.opening_limit);
    holds(!refused, OutfitRefusal::BoughtThisOpening)
}

/// The other reading: no limit.
#[allow(clippy::unnecessary_wraps)] // A check's type, the engine's reading's.
fn never_limited(_: &RowOutfit) -> Result<(), OutfitRefusal> {
    Ok(())
}

/// The player can pay the price.
fn cash_for_the_price(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.row.buy_price <= asked.outfitter.cash,
        OutfitRefusal::CannotAfford,
    )
}

/// The player owns some.
fn owned(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.owned > 0, OutfitRefusal::NoneOwned)
}

/// The outfit is not flagged [`OutfitFlags::CANNOT_SELL`].
fn sellable(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.row.facts.flags & OutfitFlags::CANNOT_SELL == 0,
        OutfitRefusal::CannotSell,
    )
}

/// The outfit can be sold here: for sale, or sold anywhere.
fn sold_here(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(asked.row.facts.sold_here, OutfitRefusal::NotBoughtHere)
}

/// The engine's sale mass: the free mass clamped at none, plus the
/// outfit's mass, is not below none (`_DoOutfitDialog` @0x5c740-0x5c747).
/// The clamped free mass is never below none, so only an outfit of
/// negative mass is refused, and the engine's sign test (@0x5c735-0x5c73e)
/// needs no test of its own.
fn clamped_mass_left(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.outfitter.free_mass_clamped() + asked.row.mass >= 0,
        OutfitRefusal::NegativeFreeMass,
    )
}

/// The other reading's sale mass: the sale leaves the unclamped free mass
/// not below none.
fn mass_left(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    holds(
        asked.outfitter.free_mass + asked.row.mass >= 0,
        OutfitRefusal::NegativeFreeMass,
    )
}

/// The engine's raised `Max` on a sale: the target held is no more than
/// its raw `Max` x the raisers left ([`raisers_left`]: owned - 1 for a
/// single sale), other raisers ignored and with no floor; none is left of
/// a target with no record.
fn engine_raised_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    let left = raisers_left().of(asked);
    raised_first_past(asked, |raise| {
        raise.target_max.map_or(0, |max| i64::from(max) * left)
    })
}

/// How many of the raiser a raised-`Max` check counts as left at a pass,
/// as [`RuleKey::CountedSale`] says: by the engine, owned now less the
/// whole count (@0x5c82d-0x5c85e); by the other reading, owned now less
/// one. The same at a count of 1.
fn raisers_left<'a>() -> Value<RowOutfit<'a>, i64> {
    Value::disputed(
        "raisers left",
        "`_DoOutfitDialog` @0x5c82d-0x5c85e: owned now - the whole count (-0x68c)",
        RuleKey::CountedSale,
        owned_less_the_count,
        owned_less_one,
    )
}

/// The engine's raisers left: owned now less the whole count, signed.
fn owned_less_the_count(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned) - i64::from(asked.pass.count)
}

/// The other reading's raisers left: owned now less one, as one sale.
fn owned_less_one(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned) - 1
}

/// The launchers whose `MaxAmmo` a launcher-sale check counts as kept at
/// a pass, as [`RuleKey::CountedSale`] says: by the engine, owned now less
/// the pass index less one, signed (@0x5cb9c-0x5cbc7); by the other
/// reading, owned now less one, at least none. The same at pass 0 for
/// some owned.
fn launchers_left<'a>() -> Value<RowOutfit<'a>, i64> {
    Value::disputed(
        "launchers left",
        "`_DoOutfitDialog` @0x5cb9c-0x5cbc7: owned now - the pass index (-0x63a) - 1",
        RuleKey::CountedSale,
        owned_less_the_pass,
        one_launcher_sold,
    )
}

/// The engine's launchers left: owned now less the pass index less one,
/// signed.
fn owned_less_the_pass(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned) - i64::from(asked.pass.index) - 1
}

/// The other reading's launchers left: owned now less one, at least none.
fn one_launcher_sold(asked: &RowOutfit) -> i64 {
    i64::from(asked.row.owned.saturating_sub(1))
}

/// The Bible's raised `Max` on a sale: the target held is no more than
/// its raised `Max` once one of the raiser is sold.
fn bible_raised_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    raised_first_past(asked, |raise| raise.left_by_the_bible)
}

/// The first of the outfit's raises, in slot order, whose target held is
/// more than `left` of it leaves, refusing with that excess to be sold
/// first.
fn raised_first_past(asked: &RowOutfit, left: impl Fn(&Raise) -> i64) -> Result<(), OutfitRefusal> {
    asked
        .row
        .facts
        .raises
        .iter()
        .find_map(|raise| {
            let excess = i64::from(raise.held) - left(raise);
            (excess > 0).then(|| OutfitRefusal::RaisedFirst {
                count: u32::try_from(excess).unwrap_or(u32::MAX),
                target: raise.target,
            })
        })
        .map_or(Ok(()), Err)
}

/// The engine's launcher sale ([`Launcher::excess_left`]).
fn engine_ammunition_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    ammunition_first(asked, RuleSource::Engine)
}

/// The other reading's launcher sale ([`Launcher::excess_left`]).
fn bible_ammunition_first(asked: &RowOutfit) -> Result<(), OutfitRefusal> {
    ammunition_first(asked, RuleSource::Bible)
}

/// A launcher sells when no rounds must be sold first, as `source` reads
/// [`Launcher::excess_left`] of the [`launchers_left`].
fn ammunition_first(asked: &RowOutfit, source: RuleSource) -> Result<(), OutfitRefusal> {
    asked
        .row
        .facts
        .launcher
        .and_then(|launcher| {
            let rounds = launcher.excess_left(launchers_left().of(asked), source)?;
            Some(OutfitRefusal::AmmunitionFirst {
                rounds,
                ammo: launcher.ammo,
            })
        })
        .map_or(Ok(()), Err)
}

/// The info box's words for an outfit ([`Outfitter::words`]).
fn info_words<'a>() -> Value<RowOutfit<'a>, Option<OutfitRefusal>> {
    Value::plain(
        "words",
        "`_OutfitDialogUpdate` @0x57dba-0x57e6f: `_HasMaxOfItem` @0x57dd6, then the mass \
         @0x57e33-0x57e45 against `free_mass_clamped`",
        has_max_or_mass,
    )
}

/// [`has_max`]'s refusal, else [`lacks_the_mass`].
fn has_max_or_mass(asked: &RowOutfit) -> Option<OutfitRefusal> {
    refusal(&has_max(), asked)
        .err()
        .or_else(|| lacks_the_mass(asked))
}

/// The cap `_HasMaxOfItem(item, 0, &cap)` reports out for an outfit
/// ([`Outfitter::cap`]).
fn reported_cap<'a>() -> Value<RowOutfit<'a>, i16> {
    Value::plain(
        "cap",
        "`_HasMaxOfItem` out-parameter @0x4512: ammunition @0x45b2-0x45c1, raised \
         @0x4681-0x46a0, guns @0x4798-0x47a7, turrets @0x482b-0x483a",
        cap_reported,
    )
}

/// What a sale of an outfit refunds ([`Outfitter::refund`]).
fn refund_value<'a>() -> Value<RowOutfit<'a>, i64> {
    Value::disputed(
        "refund",
        "`_DoOutfitDialog` @0x5ceee-0x5cf08",
        RuleKey::OutfitRefund,
        full_since_the_visit_opened,
        always_half,
    )
}

/// The engine's refund: what was paid when the count owned is above the
/// visit's count on entry, otherwise half the price.
fn full_since_the_visit_opened(asked: &RowOutfit) -> i64 {
    refund(
        asked.row.price,
        asked.row.buy_price,
        asked.row.owned,
        asked.outfitter.visit.opened_with(asked.row.id),
        RuleSource::Engine,
    )
}

/// The other reading's refund: always half.
fn always_half(asked: &RowOutfit) -> i64 {
    resale(asked.row.price)
}

/// Its `Max`, lowered to a smaller ammunition cap (@0x45b2-0x45c1);
/// replaced by the raised `Max` when more than one raiser counts
/// (@0x4681-0x46a0: the engine also replaces it when the raised `Max` is
/// the smaller, which with a multiplier of 1 it never is); then, for a
/// gun, lowered to the ship's whole gun limit (@0x4798-0x47a7) and, for a
/// turret, to its whole turret limit (@0x482b-0x483a). Each is held in 16
/// bits ([`saturate_i16`]), where the engine's `imulw` and `addw` wrap.
fn cap_reported(asked: &RowOutfit) -> i16 {
    let facts = &asked.row.facts;
    let armed = asked.outfitter.armed;
    let mut cap = asked.row.max;
    if let Some(ammo) = facts.ammo_cap {
        cap = cap.min(ammo);
    }
    if let Some(raised) = facts.raised {
        cap = saturate_i16(raised);
    }
    if facts.flags & OutfitFlags::GUN != 0 {
        cap = cap.min(saturate_i16(i64::from(armed.gun_limit)));
    }
    if facts.flags & OutfitFlags::TURRET != 0 {
        cap = cap.min(saturate_i16(i64::from(armed.turret_limit)));
    }
    cap
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

/// `outfit`'s price, for one, on a ship of `ship_mass`, as `rules` reads
/// [`RuleKey::MassPrice`]: its `Cost`, or with
/// [`OutfitFlags::PRICE_BY_MASS`] max(`Cost`, `Cost` × `ship_mass`), and
/// 0 for a `Cost` of 0 or less. Every price the port gives an outfit
/// comes from here.
#[must_use]
pub fn unit_price(outfit: &OutfitRecord, ship_mass: i16, rules: &Rulebook) -> i64 {
    outfit_price().of(&OutfitOnShip {
        record: outfit,
        ship_mass,
        rules,
    })
}

/// An outfit on a ship class: what `_AdjustedItemCost` and
/// `_AdjustedItemMass` read.
pub(crate) struct OutfitOnShip<'a> {
    /// The outfit.
    pub(crate) record: &'a OutfitRecord,
    /// The flown class's raw `Mass` (`shïp`+0x9ee).
    pub(crate) ship_mass: i16,
    /// The rule set the price and the mass follow.
    pub(crate) rules: &'a Rulebook,
}

impl Context for OutfitOnShip<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// An outfit's price, for one, as [`RuleKey::MassPrice`] says.
fn outfit_price<'a>() -> Value<OutfitOnShip<'a>, i64> {
    Value::disputed(
        "outfit price",
        "`_AdjustedItemCost` @0x5869-0x5895; inlined in `_PlayerShipTradeInPrice` \
         @0xb0e3-0xb106",
        RuleKey::MassPrice,
        at_least_the_cost,
        bare_product,
    )
}

/// The engine's price: 0 for a `Cost` of 0 or less (@0x5872-0x5878);
/// otherwise the `Cost`, or with the flag max(`Cost`, `Cost` × `Mass`)
/// (@0x5883-0x588f). The product is not wrapped to 32 bits.
fn at_least_the_cost(priced: &OutfitOnShip) -> i64 {
    let cost = i64::from(priced.record.cost);
    if !cost.is_positive() {
        return 0;
    }
    if priced.record.flags & OutfitFlags::PRICE_BY_MASS == 0 {
        return cost;
    }
    cost.max(cost.saturating_mul(i64::from(priced.ship_mass)))
}

/// The other reading's price: the `Cost`, times the `Mass` with the
/// flag, never below 0.
fn bare_product(priced: &OutfitOnShip) -> i64 {
    let cost = i64::from(priced.record.cost);
    let price = if priced.record.flags & OutfitFlags::PRICE_BY_MASS == 0 {
        cost
    } else {
        cost.saturating_mul(i64::from(priced.ship_mass))
    };
    price.max(0)
}

/// `outfit`'s mass, for one, in tons, on a ship of `ship_mass`, as
/// `rules` reads [`RuleKey::ScaledMass`]: its `Mass`, or with
/// [`OutfitFlags::MASS_BY_MASS`] and a positive `Mass`, max(`Mass`,
/// `ship_mass` × `Mass` / 100, truncated) by the engine and the bare
/// product by the other reading. Every mass the port gives an outfit
/// comes from here.
#[must_use]
pub fn unit_mass(outfit: &OutfitRecord, ship_mass: i16, rules: &Rulebook) -> i64 {
    outfit_mass().of(&OutfitOnShip {
        record: outfit,
        ship_mass,
        rules,
    })
}

/// An outfit's mass, for one, as [`RuleKey::ScaledMass`] says.
fn outfit_mass<'a>() -> Value<OutfitOnShip<'a>, i64> {
    Value::disputed(
        "outfit mass",
        "`_AdjustedItemMass` @0x2889-0x28c2; inlined in `_ShipFreeMass` @0xb4c1-0xb4ed",
        RuleKey::ScaledMass,
        at_least_the_mass,
        scaled_product,
    )
}

/// The engine's mass: the `Mass` when it is 0 or less (@0x2893-0x2895)
/// or without the flag (@0x2897-0x289b); otherwise max(`Mass`, `Mass` ×
/// the class's `Mass` / 100, truncated) (@0x28a0-0x28bc). The product is
/// not wrapped to 32 bits.
fn at_least_the_mass(weighed: &OutfitOnShip) -> i64 {
    let mass = i64::from(weighed.record.mass);
    if !mass.is_positive() || weighed.record.flags & OutfitFlags::MASS_BY_MASS == 0 {
        return mass;
    }
    mass.max(i64::from(weighed.ship_mass) * mass / 100)
}

/// The other reading's mass: the `Mass`, or with the flag and a positive
/// `Mass` the class's `Mass` times it over 100, truncated, with no floor.
fn scaled_product(weighed: &OutfitOnShip) -> i64 {
    let mass = i64::from(weighed.record.mass);
    if weighed.record.flags & OutfitFlags::MASS_BY_MASS != 0 && mass.is_positive() {
        i64::from(weighed.ship_mass) * mass / 100
    } else {
        mass
    }
}

/// The cargo space `outfit` takes, when `_CanBuyOutfitItem` tests it as
/// a mass expansion: the size of the last of its mods of [`MORE_CARGO`],
/// when that `ModVal` is negative (@0x4e8bc-0x4e8d2, @0x4e94f-0x4e95a).
/// Its `Mass` plays no part. An outfit whose first mod is `ModType` 3
/// ([`MOD_AMMO`]) is never tested as one (@0x4e8f6).
fn mass_expansion(outfit: &OutfitRecord) -> Option<u16> {
    if outfit.mods[0].0 == MOD_AMMO {
        return None;
    }
    outfit
        .mods
        .iter()
        .rev()
        .find(|&&(mod_type, _)| mod_type == MORE_CARGO)
        .filter(|&&(_, mod_val)| mod_val < 0)
        .map(|&(_, mod_val)| mod_val.unsigned_abs())
}

/// Which of the once-an-opening flags limits an outfit
/// ([`opening_limit`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningLimit {
    /// The map flag, `_boughtMap`.
    Map,
    /// The clean-record flag, `_boughtID`.
    Record,
}

/// Which flag, if any, limits `outfit` to once an opening, as
/// `_CanBuyOutfitItem`'s else-if chain reads its mods in any slot
/// (@0x4e8bc-0x4e9c5): none when its first mod is [`MOD_AMMO`] (the
/// fighter path) or it has any [`MORE_CARGO`] mod (the mass-expansion
/// path); otherwise the map flag for an [`EXPLORES_MAP`] mod; otherwise
/// the record flag for a [`CLEAN_RECORD`] mod.
pub(crate) fn opening_limit(outfit: &OutfitRecord) -> Option<OpeningLimit> {
    let has = |kind: i16| outfit.mods.iter().any(|&(mod_type, _)| mod_type == kind);
    if outfit.mods[0].0 == MOD_AMMO || has(MORE_CARGO) {
        None
    } else if has(EXPLORES_MAP) {
        Some(OpeningLimit::Map)
    } else if has(CLEAN_RECORD) {
        Some(OpeningLimit::Record)
    } else {
        None
    }
}

/// The once-an-opening flags: whether the last outfit bought since the
/// outfitter opened explored a map (`_boughtMap`) or cleared the legal
/// record (`_boughtID`). The original clears both each time the
/// outfitter dialog opens (`_DoOutfitDialog` @0x5bb37-0x5bb44) and sets
/// them on every grant (`Bought::after_buying`); see the module docs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bought {
    /// `_boughtMap`.
    pub(crate) map: bool,
    /// `_boughtID`.
    pub(crate) record: bool,
}

impl Bought {
    /// The flags after `record` is bought, whatever they were
    /// (`_GrantOutfitItem` @0x44d4f): both cleared (@0x44d67-0x44d7d);
    /// then the map flag set when any slot has an [`EXPLORES_MAP`] mod,
    /// whatever its `ModVal` (@0x44ea9-0x44eaf); otherwise the record flag
    /// set when any slot has a [`CLEAN_RECORD`] mod and none a [`PAINT`]
    /// mod (@0x44ecc-0x44f8b). Unlike [`opening_limit`], an ammunition or
    /// cargo mod plays no part.
    #[must_use]
    pub(crate) fn after_buying(record: &OutfitRecord) -> Self {
        let has = |kind: i16| record.mods.iter().any(|&(mod_type, _)| mod_type == kind);
        let map = has(EXPLORES_MAP);
        Self {
            map,
            record: !map && has(CLEAN_RECORD) && !has(PAINT),
        }
    }

    /// Whether these flags refuse a buy of an outfit that `limit` limits
    /// ([`opening_limit`]).
    #[must_use]
    pub(crate) fn refuses(self, limit: Option<OpeningLimit>) -> bool {
        match limit {
            Some(OpeningLimit::Map) => self.map,
            Some(OpeningLimit::Record) => self.record,
            None => false,
        }
    }
}

/// One visit to the outfitter: what lasts from the outfitter opening to
/// its next opening. The once-an-opening flags ([`Bought`]), which limit
/// what is sold as [`RuleKey::OutfitLimit`] reads, and how many of each
/// outfit the player owned on entry, which a sale refunds against
/// ([`refund`]). The session opens one each time the outfitter dialog
/// opens (`_DoOutfitDialog` @0x5bb37-0x5bb44), on a landing and as it
/// starts. None of it is saved: the original keeps the flags in globals
/// `_DoOutfitDialog` clears on opening, and the counts in its locals.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutfitVisit {
    /// The once-an-opening flags (`_boughtMap`, `_boughtID`).
    pub(crate) bought: Bought,
    /// How many of each outfit the player owned when the outfitter
    /// opened, as `_DoOutfitDialog` copies them on entry
    /// (@0x5badd-0x5bafa).
    pub(crate) opened_with: BTreeMap<OutfitId, u16>,
}

impl OutfitVisit {
    /// A visit opened over `owned`: the flags clear and the counts
    /// copied.
    #[must_use]
    pub(crate) fn open(owned: &BTreeMap<OutfitId, u16>) -> Self {
        Self {
            bought: Bought::default(),
            opened_with: owned.clone(),
        }
    }

    /// `record` is bought: the flags are set afresh
    /// ([`Bought::after_buying`]); the counts on entry stay.
    pub(crate) fn bought(&mut self, record: &OutfitRecord) {
        self.bought = Bought::after_buying(record);
    }

    /// How many of `outfit` the player owned when the visit opened; none
    /// when it owned none.
    #[must_use]
    pub(crate) fn opened_with(&self, outfit: OutfitId) -> u16 {
        self.opened_with.get(&outfit).copied().unwrap_or(0)
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

/// What one outfit priced at `price` sells back for at half.
#[must_use]
pub fn resale(price: i64) -> i64 {
    price.saturating_mul(RESALE_PERCENT) / 100
}

/// What selling one outfit priced at `price`, bought here for `paid`,
/// refunds, the player owning `owned` of it before the sale and having
/// owned `opened_with` when the outfitter opened, as `source` says
/// ([`RuleKey::OutfitRefund`](crate::RuleKey::OutfitRefund)).
///
/// By the engine ([`RuleSource::Engine`]), `_DoOutfitDialog` compares the
/// count owned before the sale with its copy of the counts taken on
/// opening (@0x5ceee-0x5cefa): one bought since, the count above the copy,
/// refunds in full what was paid, `paid` (the price, unless the other
/// reading of [`RuleKey::RankPriceScope`](crate::RuleKey::RankPriceScope)
/// scaled it); any other refunds [`resale`], the price times
/// 0.5 (the double at @0xdd128) truncated toward zero (@0x5cefc-0x5cf08).
/// By the other reading, the port's earlier behaviour, every sale refunds
/// [`resale`].
#[must_use]
pub fn refund(price: i64, paid: i64, owned: u16, opened_with: u16, source: RuleSource) -> i64 {
    if source == RuleSource::Engine && owned > opened_with {
        paid
    } else {
        resale(price)
    }
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

/// The total mass of `outfits` on a ship of `ship_mass`, each weighed by
/// [`unit_mass`] as `rules` says.
fn mass_of(
    outfits: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    ship_mass: i16,
    rules: &Rulebook,
) -> i64 {
    records
        .iter()
        .filter_map(|record| Some((record, *outfits.get(&record.id)?)))
        .map(|(record, count)| unit_mass(record, ship_mass, rules) * i64::from(count))
        .sum()
}

/// The free mass of a ship with `fields` and this `standard` equipment
/// (its default items and stock weapons, whose mass `FreeMass` leaves
/// out), carrying `owned`, each outfit weighed as `rules` says
/// ([`RuleKey::ScaledMass`]; `_ShipFreeMass` @0xb4c1-0xb4ed).
#[must_use]
pub(crate) fn free_mass(
    fields: ShipFields,
    standard: &BTreeMap<OutfitId, u16>,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    rules: &Rulebook,
) -> i64 {
    i64::from(fields.free_mass) + mass_of(standard, records, fields.mass, rules)
        - mass_of(owned, records, fields.mass, rules)
}

/// A ship class's room for guns and turrets: its `MaxGun` and `MaxTur`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hardpoints {
    /// Its `MaxGun`.
    pub guns: i16,
    /// Its `MaxTur`.
    pub turrets: i16,
}

/// A launcher outfit the player owns, and the ammunition it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Launcher {
    /// Its weapon's `MaxAmmo`; none for no limit of its own.
    pub max_ammo: u32,
    /// The rounds of its ammunition held, the fighters out counted.
    pub rounds: u32,
    /// The outfit that names its ammunition, if any.
    pub ammo: Option<OutfitId>,
}

impl Launcher {
    /// The rounds that must be sold before one of `owned` launchers can
    /// be, as `source` reads the rule
    /// ([`RuleKey::LauncherSale`](crate::RuleKey::LauncherSale)); `None`
    /// when it can be sold. By the engine, the rounds the remaining
    /// launchers' `MaxAmmo` cannot hold, only when `MaxAmmo` is above 0
    /// (`_DoOutfitDialog` @0x5cad0, @0x5cb9c-0x5cbe0); by the other
    /// reading, every round held.
    #[cfg(test)]
    pub(crate) fn excess(&self, owned: u16, source: RuleSource) -> Option<u32> {
        self.excess_left(i64::from(owned.saturating_sub(1)), source)
    }

    /// The rounds that must be sold before a launcher can be, `left`
    /// launchers' `MaxAmmo` kept, as `source` reads the rule
    /// ([`RuleKey::LauncherSale`](crate::RuleKey::LauncherSale)); `None`
    /// when it can be sold. By the engine, rounds - `MaxAmmo` x `left`,
    /// signed with no floor, so `left` below none names more rounds than
    /// are held (@0x5cbc7-0x5cbcf), only when `MaxAmmo` is above 0
    /// (@0x5cad0) and some rounds are held; by the other reading, every
    /// round held.
    pub(crate) fn excess_left(&self, left: i64, source: RuleSource) -> Option<u32> {
        if self.rounds == 0 {
            return None;
        }
        match source {
            RuleSource::Engine => (self.max_ammo > 0).then(|| {
                let kept = i64::from(self.max_ammo).saturating_mul(left);
                let excess = i64::from(self.rounds).saturating_sub(kept);
                u32::try_from(excess.max(0)).unwrap_or(u32::MAX)
            }),
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
    i64::from(record.max).saturating_mul(raise_multiplier(record, owned, records, source))
}

/// What [`raised_max`] multiplies `record`'s `Max` by: at least 1.
fn raise_multiplier(
    record: &OutfitRecord,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    source: RuleSource,
) -> i64 {
    records
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
        .fold(0, i64::saturating_add)
        .max(1)
}

/// `value` held in the engine's 16 bits, at the least or most they hold
/// rather than wrapped past it.
fn saturate_i16(value: i64) -> i16 {
    let held = value.clamp(i64::from(i16::MIN), i64::from(i16::MAX));
    i16::try_from(held).unwrap_or_default()
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Armed {
    /// The outfits owned flagged [`OutfitFlags::GUN`].
    pub guns: i32,
    /// The outfits owned flagged [`OutfitFlags::TURRET`].
    pub turrets: i32,
    /// `MaxGun`, as [`MORE_GUNS`] changes it.
    pub gun_limit: i32,
    /// `MaxTur`, as [`MORE_TURRETS`] changes it.
    pub turret_limit: i32,
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
}

/// What `_HasMaxOfItem` reads, beyond the outfits owned, for an outfit's
/// limit ([`item_limit`]).
#[derive(Clone, Copy)]
pub(crate) struct ItemRoom<'a> {
    /// The ship class's `MaxGun` and `MaxTur`.
    pub(crate) hardpoints: Hardpoints,
    /// How many of each weapon the ship carries, as its weapon array
    /// holds them.
    pub(crate) launchers: &'a BTreeMap<WeaponId, u32>,
    /// Each weapon's `MaxAmmo`; none for no limit of its own.
    pub(crate) max_ammo: &'a dyn Fn(WeaponId) -> u32,
    /// How the [`RAISES_MAX`] outfits raise a `Max`
    /// ([`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax)).
    pub(crate) raised_max: RuleSource,
}

/// The limit `_HasMaxOfItem` (@0x4512-0x4894) gives `record` through its
/// second argument, `owned` of `records` being owned and `room` the rest
/// it reads, by the first of these that holds:
///
/// 1. Ammunition, an outfit whose first mod is [`MOD_AMMO`] naming a
///    weapon of `MaxAmmo` above none (@0x457e-0x45e4): when the count
///    owned is at least that `MaxAmmo` times the weapon's launchers, that
///    cap, the `Max` never read.
/// 2. When the count owned is at least its `Max` as [`raised_max`] reads
///    it (@0x45e9-0x46c3), that `Max`.
/// 3. A gun ([`OutfitFlags::GUN`], @0x46c8-0x47b7): none when its limit
///    (`MaxGun`, as [`MORE_GUNS`] changes it) is none or less, and the
///    whole limit when the guns owned are at it or above.
/// 4. A turret ([`OutfitFlags::TURRET`], @0x47bd-0x4866), the same with
///    `MaxTur` and [`MORE_TURRETS`].
/// 5. Otherwise the count owned.
///
/// The original's sums and products are 16-bit and wrap; here they are
/// wide and saturate.
pub(crate) fn item_limit(
    record: &OutfitRecord,
    owned: &BTreeMap<OutfitId, u16>,
    records: &[OutfitRecord],
    room: &ItemRoom,
) -> i64 {
    let count = i64::from(owned.get(&record.id).copied().unwrap_or_default());
    if let (MOD_AMMO, weapon) = record.mods[0] {
        let weapon = WeaponId(weapon);
        let max_ammo = (room.max_ammo)(weapon);
        if max_ammo > 0 {
            let launchers = room.launchers.get(&weapon).copied().unwrap_or_default();
            let cap = i64::from(max_ammo).saturating_mul(i64::from(launchers));
            if count >= cap {
                return cap;
            }
        }
    }
    let max = raised_max(record, owned, records, room.raised_max);
    if count >= max {
        return max;
    }
    let slots = |flag: u16, base: i16, mod_type: i16| {
        if record.flags & flag == 0 {
            return None;
        }
        let limit = i64::from(raised(base, mod_type, owned, records));
        if limit <= 0 {
            return Some(0);
        }
        (i64::from(flagged(flag, owned, records)) >= limit).then_some(limit)
    };
    slots(OutfitFlags::GUN, room.hardpoints.guns, MORE_GUNS)
        .or_else(|| slots(OutfitFlags::TURRET, room.hardpoints.turrets, MORE_TURRETS))
        .unwrap_or(count)
}

/// `record`'s lower-case names.
fn lc_names(record: &OutfitRecord) -> LcNames {
    LcNames {
        singular: record.lc_name.clone(),
        plural: record.lc_plural.clone(),
    }
}

/// The outfit table's index 0, `oütf` 128: the outfit the dialog selects
/// after an order that changes the listing (`*0xded58 = 0`, @0x5c336,
/// @0x5d01c), listed or not, which the sell loop's passes left sell (see
/// [`RuleKey::RelistedSale`](crate::RuleKey::RelistedSale)) and Buy and
/// Sell answer for (see
/// [`RuleKey::RelistedSelection`]).
pub const FIRST_OUTFIT: OutfitId = OutfitId(128);

/// The length of the port's outfit list (0x3b500c, 0x200 entries,
/// @0x5d117): the index a relisting leaves the sell loop at when nothing
/// is listed (see [`RuleKey::RelistedSale`](crate::RuleKey::RelistedSale)).
pub const LIST_SLOTS: u32 = 512;

/// Everything the outfitter rules read about the ship and where it is.
pub(crate) struct Shop<'a> {
    /// Every `oütf`.
    pub(crate) records: &'a [OutfitRecord],
    /// Every `crön`, whose running ones contribute bits.
    pub(crate) crons: &'a [CronRecord],
    /// Every `ränk`, whose active ones contribute bits.
    pub(crate) ranks: &'a [RankRecord],
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
    /// For every ammunition outfit whose first mod is `ModType` 3 naming
    /// a weapon of `MaxAmmo` above 0, the most of it the player may own:
    /// that `MaxAmmo` times the player's launchers of the weapon, as a
    /// 16-bit product (`_HasMaxOfItem` @0x457e-0x45e4; see the module
    /// docs).
    pub(crate) ammo_caps: &'a BTreeMap<OutfitId, i16>,
    /// The ship class's `MaxGun` and `MaxTur`.
    pub(crate) hardpoints: Hardpoints,
    /// The ship's own free cargo: its cargo space less every ton held,
    /// below zero when more is held than it has space for
    /// (`_ShipTotalHolds() - _TotalCargo(player)`, @0x4e980-0x4e9a8).
    pub(crate) free_cargo: i64,
    /// Each launcher outfit, with the ammunition it holds.
    pub(crate) launchers: &'a BTreeMap<OutfitId, Launcher>,
    /// The visit: the once-an-opening flags as they stand, which the
    /// outfitter's `once an opening` check reads as
    /// [`RuleKey::OutfitLimit`] says, and the counts owned on entry.
    pub(crate) visit: &'a OutfitVisit,
    /// The control-bit test of an outfit's `Availability`.
    pub(crate) gate: Gate<'a>,
    /// The ranks' factor the buy price takes: 1.0 by the engine's reading
    /// of [`RuleKey::RankPriceScope`], `_priceFluxMult[1]` by the other.
    pub(crate) price_mult: f32,
    /// How each disputed rule reads ([`Rulebook::source_for`]).
    pub(crate) rules: Rulebook,
}

impl Shop<'_> {
    /// The outfitter, for `pilot`, each outfit's roll for the day kept in
    /// `rolls` and any not drawn yet drawn on `chance` (see the module
    /// docs); `None` when the stellar has none. The session builds it
    /// through [`outfitter_with`](Self::outfitter_with).
    #[cfg(test)]
    pub(crate) fn outfitter(
        &self,
        pilot: &Pilot,
        rolls: &mut DayRolls<OutfitId>,
        chance: &mut dyn Chance,
    ) -> Option<Outfitter> {
        self.outfitter_with(pilot, rolls, chance, None)
    }

    /// The outfitter as `outfitter` (built for tests only) builds it, with
    /// the row of `also`, when it has a record and is not listed, as its
    /// [`unlisted`](Outfitter::unlisted) row, sold here: the outfit a
    /// relisting selects, [`FIRST_OUTFIT`], which the sell loop sells and
    /// Buy and Sell answer for, asking no list (@0x5d164-0x5d199,
    /// `_OutfitFilter` @0x5806f-0x580ca; see
    /// [`RuleKey::RelistedSale`](crate::RuleKey::RelistedSale) and
    /// [`RuleKey::RelistedSelection`]).
    pub(crate) fn outfitter_with(
        &self,
        pilot: &Pilot,
        rolls: &mut DayRolls<OutfitId>,
        chance: &mut dyn Chance,
        also: Option<OutfitId>,
    ) -> Option<Outfitter> {
        if self.site.flags & StellarFlags::OUTFITTER == 0 {
            return None;
        }
        let by = wares::Contributors {
            ship: self.fields.contribute,
            outfits: self.records,
            crons: self.crons,
            ranks: self.ranks,
        };
        let contributed = wares::contributed(&by, pilot);
        let free = free_mass(
            self.fields,
            self.standard,
            &pilot.outfits,
            self.records,
            &self.rules,
        );
        let mut sorted: Vec<&OutfitRecord> = self.records.iter().collect();
        sorted.sort_by_key(|record| record.id);
        let armed = Armed::of(self.hardpoints, &pilot.outfits, self.records);
        let buy_random = self.rules.source_for(RuleKey::BuyRandom);
        let mut sweep = HideHigher::default();
        let mut rows = Vec::new();
        let mut unlisted = None;
        for record in sorted {
            let owned = pilot.owned(record.id);
            let offer = Offer {
                shop: self,
                owned: &pilot.outfits,
                record,
                contributed,
            };
            let required = required().of(&offer);
            let available = available().of(&offer);
            let by_tech = tech_listed().of(&offer);
            let on_offer = by_tech || buys_owned().of(&offer);
            let today = on_offer
                && if owned > 0 {
                    rolls.hold(record.id);
                    true
                } else {
                    let roll = buy_roll(record.buy_random, buy_random);
                    rolls.today(record.id, roll, chance)
                };
            let on_sale = sweep.on_sale(record.disp_weight);
            let hidden = owned == 0 && hides(record.flags, required, available);
            let sells_anywhere = record.flags & OutfitFlags::SELL_ANYWHERE != 0;
            // The listing byte: the sweep (`_SetupPortAvailableItems`
            // @0xc11f-0xc175, clear @0xc158) runs after the owned
            // `SELL_ANYWHERE` step (@0xc096-0xc0ba), and its hider is any
            // outfit whose byte is set, read alone: never
            // `_CanBuyOutfitItem`, `Require`, `Availability` or the cash.
            let listed = on_sale && ((today && !hidden) || (owned > 0 && sells_anywhere));
            sweep.note(
                record.disp_weight,
                record.flags & OutfitFlags::HIDE_HIGHER != 0,
                listed,
            );
            if !listed && also != Some(record.id) {
                continue;
            }
            let price = unit_price(record, self.fields.mass, &self.rules);
            let mass = unit_mass(record, self.fields.mass, &self.rules);
            // A listed row passed the sweep, so it is for sale exactly
            // when it is on offer today.
            let sold_here = !listed || today || sells_anywhere;
            let facts = OutfitFacts {
                listed,
                required,
                available,
                sold_here,
                ..self.facts(&offer, owned)
            };
            let row = OutfitRow {
                id: record.id,
                name: record.name.clone(),
                short_name: record.short_name.clone(),
                price,
                buy_price: crate::rank::scaled(price, self.price_mult),
                mass,
                owned,
                max: record.max,
                facts,
            };
            if listed {
                rows.push((record.disp_weight, row));
            } else {
                unlisted = Some(row);
            }
        }
        wares::in_display_order(&mut rows, |(weight, row)| (*weight, row.id.0));
        Some(Outfitter {
            rows: rows.into_iter().map(|(_, row)| row).collect(),
            unlisted,
            cash: pilot.cash,
            free_mass: free,
            armed,
            holds: self.fields.holds,
            free_cargo: self.free_cargo,
            visit: self.visit.clone(),
            rules: self.rules,
        })
    }

    /// The facts the outfitter's rules read about `offer`'s outfit, the
    /// player owning `owned` of it, settled as the outfitter is built:
    /// whether Buy's own tech test passes it, and what the records, the
    /// ship and the rule set say of it. The listing's outcomes (`listed`,
    /// `required`, `available` and `sold_here`) are left false for the
    /// build to fill in.
    fn facts(&self, offer: &Offer, owned: u16) -> OutfitFacts {
        let record = offer.record;
        let multiplier = max_multiplier().of(offer);
        let launcher = self.launchers.get(&record.id).copied();
        OutfitFacts {
            names: lc_names(record),
            flags: record.flags,
            buy_tech: buy_tech().of(offer),
            ammo_cap: self.ammo_caps.get(&record.id).copied(),
            raised: (multiplier > 1).then(|| i64::from(record.max).saturating_mul(multiplier)),
            fighter_room: self.fighter_room.get(&record.id).copied(),
            expansion: mass_expansion(record),
            opening_limit: opening_limit(record),
            launcher,
            ammo_names: launcher
                .and_then(|launcher| launcher.ammo)
                .and_then(|ammo| self.records.iter().find(|named| named.id == ammo))
                .map(lc_names),
            raises: self.raises(record, owned, offer.owned),
            mass_field: record.mass,
            ..OutfitFacts::default()
        }
    }

    /// One [`Raise`] for each [`RAISES_MAX`] mod of `record`, in slot
    /// order, the player owning `owned` of it and `all` the outfits: the
    /// target, how many of it are held, its `Max`, and its [`raised_max`]
    /// by the Bible once one `record` is sold, which reads every record.
    /// The original reads only a `ModVal` of 128-639 (@0x5c819-0x5c827);
    /// one outside names no outfit and none of it is owned.
    fn raises(
        &self,
        record: &OutfitRecord,
        owned: u16,
        all: &BTreeMap<OutfitId, u16>,
    ) -> Vec<Raise> {
        let mut after = all.clone();
        match owned.saturating_sub(1) {
            0 => after.remove(&record.id),
            left => after.insert(record.id, left),
        };
        record
            .mods
            .iter()
            .filter(|&&(kind, _)| kind == RAISES_MAX)
            .map(|&(_, val)| {
                let target = OutfitId(val);
                let target_record = self.records.iter().find(|other| other.id == target);
                Raise {
                    target,
                    held: all.get(&target).copied().unwrap_or(0),
                    target_max: target_record.map(|other| other.max),
                    left_by_the_bible: target_record.map_or(0, |other| {
                        raised_max(other, &after, self.records, RuleSource::Bible)
                    }),
                    target_names: target_record.map(lc_names),
                }
            })
            .collect()
    }
}

/// One outfit asked about as the outfitter is built: the facts the build
/// rules read, which need the gate or every record, and so are settled
/// once, then kept as [`OutfitFacts`].
struct Offer<'a> {
    /// The shop: the records, the site, the gate and the rule set.
    shop: &'a Shop<'a>,
    /// Every outfit the player owns.
    owned: &'a BTreeMap<OutfitId, u16>,
    /// The outfit.
    record: &'a OutfitRecord,
    /// The `Contribute` bits of the ship and every outfit owned.
    contributed: u64,
}

impl Context for Offer<'_> {
    fn rulebook(&self) -> &Rulebook {
        &self.shop.rules
    }
}

/// Whether an outfit's `TechLevel` puts it on sale at the stellar, as
/// [`RuleKey::TechLevel`] says: by the engine,
/// [`wares::outfit_tech_allows`]; by the Bible, [`wares::by_tech_level`].
fn tech_listed<'a>() -> Value<Offer<'a>, bool> {
    Value::disputed(
        "tech",
        "`_SetupPortAvailableItems` @0xbf88-0xbfbd; `_LoadObjectData` @0x78af3",
        RuleKey::TechLevel,
        engine_tech,
        bible_tech,
    )
}

/// Whether `_CanBuyOutfitItem`'s own tech test lets the outfit be bought
/// at the stellar, as [`RuleKey::TechLevel`] says: by the engine,
/// [`wares::outfit_buy_tech_allows`] (-1 always, no sign or 0x7fff test);
/// by the Bible, [`wares::by_tech_level`], the listing's own reading.
fn buy_tech<'a>() -> Value<Offer<'a>, bool> {
    Value::disputed(
        "buy tech",
        "`_CanBuyOutfitItem` @0x4e7ff-0x4e83d; `_LoadObjectData` @0x78af3",
        RuleKey::TechLevel,
        engine_buy_tech,
        bible_tech,
    )
}

/// The engine's Buy tech test: -1 always, otherwise the stellar's tech
/// or a slot.
fn engine_buy_tech(offer: &Offer) -> bool {
    wares::outfit_buy_tech_allows(offer.record.tech_level, offer.shop.site)
}

/// The engine's tech test: never below 0 or at 0x7fff.
fn engine_tech(offer: &Offer) -> bool {
    wares::outfit_tech_allows(offer.record.tech_level, offer.shop.site)
}

/// The Bible's tech test: at or below the stellar's, or a slot exactly.
fn bible_tech(offer: &Offer) -> bool {
    wares::by_tech_level(offer.record.tech_level, offer.shop.site)
}

/// Whether the stellar puts an outfit on sale because the player owns
/// it, whatever its tech: its `Flags2` has [`BUYS_OWNED_OUTFITS`], the
/// player owns at least one, and it is not [`OutfitFlags::CANNOT_SELL`]
/// (see the module docs).
fn buys_owned<'a>() -> Value<Offer<'a>, bool> {
    Value::plain(
        "buys owned",
        "`_SetupPortAvailableItems` @0xbf51-0xbf75; `_LoadObjectData` @0x77374-0x77378",
        owned_bought_back,
    )
}

/// The stellar buys back owned outfits, one is owned, and it can be sold.
fn owned_bought_back(offer: &Offer) -> bool {
    offer.shop.site.flags2 & BUYS_OWNED_OUTFITS != 0
        && offer.owned.get(&offer.record.id).copied().unwrap_or(0) > 0
        && offer.record.flags & OutfitFlags::CANNOT_SELL == 0
}

/// Whether an outfit's `Require` is met where it applies (see the module
/// docs).
fn required<'a>() -> Value<Offer<'a>, bool> {
    Value::plain(
        "required",
        "`_CanBuyOutfitItem` @0x4e9cb-0x4eb37; `_SetupPortAvailableItems` @0xbff5-0xc056",
        requirement_met,
    )
}

/// `Require` does not apply here, or each of its bits is contributed.
fn requirement_met(offer: &Offer) -> bool {
    !requirements_apply(offer.record.require_govt, offer.shop.site.govt)
        || wares::requirement_met(offer.record.require, offer.contributed)
}

/// Whether an outfit's `Availability` holds, asked through the gate.
fn available<'a>() -> Value<Offer<'a>, bool> {
    Value::plain(
        "available",
        "`_CanBuyOutfitItem` @0x4eb51; `_SetupPortAvailableItems` @0xbfe6",
        gate_allows,
    )
}

/// The gate allows the outfit's `Availability`.
fn gate_allows(offer: &Offer) -> bool {
    offer.shop.gate.allows(&offer.record.availability)
}

/// What the [`RAISES_MAX`] outfits owned multiply an outfit's `Max` by, as
/// [`RuleKey::RaisedMax`](crate::RuleKey::RaisedMax) says (see
/// [`raised_max`]): at least 1.
fn max_multiplier<'a>() -> Value<Offer<'a>, i64> {
    Value::disputed(
        "max multiplier",
        "`_HasMaxOfItem` @0x45e9-0x46c3",
        RuleKey::RaisedMax,
        engine_multiplier,
        bible_multiplier,
    )
}

/// The engine's multiplier: each mod naming the outfit counts.
fn engine_multiplier(offer: &Offer) -> i64 {
    raise_multiplier(
        offer.record,
        offer.owned,
        offer.shop.records,
        RuleSource::Engine,
    )
}

/// The Bible's multiplier: each item with such a mod counts once.
fn bible_multiplier(offer: &Offer) -> i64 {
    raise_multiplier(
        offer.record,
        offer.owned,
        offer.shop.records,
        RuleSource::Bible,
    )
}

/// Settles one of `record` as `direction` says: a buy pays `amount`, the
/// price, the outfit then going through the session's grant path (see
/// the module docs); a sale is paid `amount`, the [`refund`], and removes
/// one.
pub(crate) fn settle(pilot: &mut Pilot, record: &OutfitRecord, direction: Direction, amount: i64) {
    match direction {
        Direction::Buy => pilot.cash = pilot.cash.saturating_sub(amount),
        Direction::Sell => {
            pilot.cash = pilot.cash.saturating_add(amount);
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
    use crate::chance::NeverFires;
    use crate::control::Test;
    use crate::order_loop::Pass;
    use crate::rules::Descriptor;
    use crate::stats::{MORE_FUEL, MORE_SPEED};
    use crate::testkit::{AllowAll, FAST, RefuseBits, Scripted, catalog, outfit, planet};
    use crate::wares::TonUnit;

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
    static NO_CAPS: BTreeMap<OutfitId, i16> = BTreeMap::new();
    static NO_LAUNCHERS: BTreeMap<OutfitId, Launcher> = BTreeMap::new();
    static NO_VISIT: OutfitVisit = OutfitVisit {
        bought: Bought {
            map: false,
            record: false,
        },
        opened_with: BTreeMap::new(),
    };

    /// A visit with the flags `bought` and nothing owned on entry.
    fn with_flags(bought: Bought) -> OutfitVisit {
        OutfitVisit {
            bought,
            ..OutfitVisit::default()
        }
    }

    /// Room for more guns and turrets than any test buys.
    const ROOMY: Hardpoints = Hardpoints {
        guns: 99,
        turrets: 99,
    };

    /// More free cargo than any mass expansion a test buys needs.
    const ROOMY_CARGO: i64 = 99_999;

    /// The outfitter of `records` at `site`, reading `BuyRandom` by the
    /// engine.
    fn shop<'a>(records: &'a [OutfitRecord], site: &'a LandingSite) -> Shop<'a> {
        Shop {
            records,
            crons: &[],
            ranks: &[],
            fields: FAST,
            free_cargo: ROOMY_CARGO,
            standard: &NO_STANDARD,
            site,
            fighter_room: &NO_FIGHTERS,
            ammo_caps: &NO_CAPS,
            hardpoints: ROOMY,
            launchers: &NO_LAUNCHERS,
            visit: &NO_VISIT,
            price_mult: 1.0,
            rules: Rulebook::default(),
            gate: Gate::FRESH,
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

    fn open_gated(records: &[OutfitRecord], gate: Gate, pilot: &Pilot) -> Outfitter {
        let site = port();
        Shop {
            gate,
            ..shop(records, &site)
        }
        .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
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

    /// Buying one of outfit `id`.
    fn buy_of(id: i16) -> OutfitOrder {
        OutfitOrder {
            outfit: OutfitId(id),
            direction: Direction::Buy,
        }
    }

    /// Selling one of outfit `id`.
    fn sell_of(id: i16) -> OutfitOrder {
        OutfitOrder {
            outfit: OutfitId(id),
            direction: Direction::Sell,
        }
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
    fn only_outfits_tech_lists_are_rolled_by_ascending_id() {
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
            rules: Rulebook::default().with_override(RuleKey::BuyRandom, RuleSource::Bible),
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

    /// The outfitter of `records` at `site`, reading `TechLevel` by
    /// `source`, with no roll drawn yet and none that fires.
    fn open_teching(records: &[OutfitRecord], site: &LandingSite, source: RuleSource) -> Vec<i16> {
        let outfitter = Shop {
            rules: Rulebook::default().with_override(RuleKey::TechLevel, source),
            ..shop(records, site)
        }
        .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter");
        listed(&outfitter)
    }

    /// Outfits 128-135 of tech 3, 4, 5, 6, 55, 7, 0 and -3.
    fn teched() -> [OutfitRecord; 8] {
        [
            tech(128, 3),
            tech(129, 4),
            tech(130, 5),
            tech(131, 6),
            tech(132, 55),
            tech(133, 7),
            tech(134, 0),
            tech(135, -3),
        ]
    }

    #[test]
    fn it_sells_up_to_its_tech_level_and_its_special_tech_exactly() {
        let outfitter = open(&teched(), &pilot());
        assert_eq!(listed(&outfitter), [128, 129, 131, 132, 134]);
        assert!(
            outfitter
                .rows
                .iter()
                .all(|row| outfitter.can(buy_of(row.id.0)).is_ok())
        );
    }

    #[test]
    fn by_the_bible_an_outfit_of_negative_tech_is_sold_at_or_above_it() {
        assert_eq!(
            open_teching(&teched(), &port(), RuleSource::Bible),
            [128, 129, 131, 132, 134, 135]
        );
        assert_eq!(
            open_teching(&teched(), &port(), RuleSource::Engine),
            [128, 129, 131, 132, 134]
        );
    }

    #[test]
    fn a_special_tech_slot_of_0_sells_an_outfit_of_tech_0() {
        let site = LandingSite {
            tech_level: -5,
            special_tech: [0, -1, 0, 0, 0, 0, 0, 0],
            ..port()
        };
        let records = [tech(128, 0), tech(129, -1)];
        assert_eq!(open_teching(&records, &site, RuleSource::Engine), [128]);
        assert_eq!(open_teching(&records, &site, RuleSource::Bible), [128, 129]);
        let below = [tech(130, -5), tech(131, -6)];
        assert_eq!(open_teching(&below, &site, RuleSource::Bible), [130, 131]);
        assert_eq!(
            open_teching(&below, &site, RuleSource::Engine),
            Vec::<i16>::new()
        );
    }

    #[test]
    fn an_outfit_of_tech_0x7fff_is_never_listed() {
        let site = LandingSite {
            tech_level: i16::MAX,
            ..port()
        };
        let records = [tech(128, i16::MAX), tech(129, i16::MAX - 1)];
        assert_eq!(open_teching(&records, &site, RuleSource::Engine), [129]);
        assert_eq!(open_teching(&records, &site, RuleSource::Bible), [128, 129]);
    }

    #[test]
    fn the_tech_rule_reads_tech_level_and_cites_the_listing() {
        let value = tech_listed();
        assert_eq!(value.about().reads(), Some(RuleKey::TechLevel));
        assert_eq!(value.about().name(), "tech");
        assert_eq!(
            value.about().reproduces(),
            "`_SetupPortAvailableItems` @0xbf88-0xbfbd; `_LoadObjectData` @0x78af3"
        );
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
        assert_eq!(outfitter.can(buy_of(128)), Ok(()), "the ship's bit");
        assert_eq!(outfitter.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
        assert_eq!(outfitter.can(buy_of(130)), Err(OutfitRefusal::NotForSale));
        let licensed = open(&records, &owning(&[(140, 1)]));
        assert_eq!(licensed.can(buy_of(130)), Ok(()), "the licence's bit too");
        assert_eq!(licensed.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
    }

    #[test]
    fn an_owned_outfit_with_no_record_contributes_nothing_and_is_not_listed() {
        let records = [requiring(128, 0x2, -1)];
        let outfitter = open(&records, &owning(&[(999, 3)]));
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(outfitter.can(buy_of(128)), Err(OutfitRefusal::NotForSale));
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
        assert_eq!(outfitter.can(buy_of(128)), Ok(()), "not applied here");
        assert_eq!(outfitter.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
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
        assert_eq!(owned.can(buy_of(128)), Err(OutfitRefusal::NotForSale));
        assert_eq!(owned.can(sell_of(128)), Ok(()));
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
        assert_eq!(outfitter.can(buy_of(128)), Ok(()));
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
        assert_eq!(outfitter.can(buy_of(128)), Err(OutfitRefusal::NotForSale));
        assert_eq!(outfitter.can(buy_of(129)), Ok(()), "another bit holds");
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
        assert_eq!(outfitter.can(buy_of(128)), Err(OutfitRefusal::NotForSale));
        assert_eq!(outfitter.can(sell_of(128)), Ok(()), "still sold back");
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
        assert_eq!(outfitter.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
        assert_eq!(outfitter.can(buy_of(130)), Ok(()));
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
        assert_eq!(outfitter.can(buy_of(128)), Err(OutfitRefusal::NotForSale));
    }

    #[test]
    fn a_hider_whose_availability_fails_still_hides_higher_ones() {
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
        let outfitter = open_gated(&records, refusing, &pilot());
        assert_eq!(listed(&outfitter), [129]);
        assert_eq!(outfitter.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
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
        // ...and one greyed for an unmet Require still hides.
        let unmet = [
            OutfitRecord {
                require: 0x2,
                ..weighted(129, 5, OutfitFlags::HIDE_HIGHER)
            },
            weighted(130, 5, 0),
        ];
        assert_eq!(listed(&open(&unmet, &pilot())), [129]);
        let off_tech = [
            OutfitRecord {
                tech_level: 9,
                ..weighted(129, 5, OutfitFlags::HIDE_HIGHER)
            },
            weighted(130, 5, 0),
        ];
        assert_eq!(listed(&open(&off_tech, &pilot())), [130]);
    }

    /// A hider at 129 shaped by `hider`, and a plain outfit at 130, both
    /// of `DispWeight` 5.
    fn hiding(hider: OutfitRecord) -> [OutfitRecord; 2] {
        [
            OutfitRecord {
                disp_weight: 5,
                flags: hider.flags | OutfitFlags::HIDE_HIGHER,
                ..hider
            },
            OutfitRecord {
                disp_weight: 5,
                ..outfit(130, &[])
            },
        ]
    }

    #[test]
    fn a_listed_hider_hides_higher_outfits_whatever_greys_its_buy() {
        // (a) Require unmet, without HIDE_UNLESS_REQUIRED.
        let unmet = open(&hiding(requiring(129, 0x2, -1)), &pilot());
        assert_eq!(listed(&unmet), [129]);
        assert_eq!(unmet.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
        // (b) owned, so its HIDE_UNLESS_REQUIRED is skipped.
        let owned_unmet = open(
            &hiding(OutfitRecord {
                flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
                ..requiring(129, 0x2, -1)
            }),
            &owning(&[(129, 1)]),
        );
        assert_eq!(listed(&owned_unmet), [129]);
        assert_eq!(owned_unmet.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
        // (c) owned SELL_ANYWHERE, off tech.
        let anywhere = open(
            &hiding(OutfitRecord {
                flags: OutfitFlags::SELL_ANYWHERE,
                ..tech(129, 9)
            }),
            &owning(&[(129, 1)]),
        );
        assert_eq!(listed(&anywhere), [129]);
        assert_eq!(anywhere.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
        assert_eq!(anywhere.can(sell_of(129)), Ok(()));
        // (d) an owned Flags2 0x0400 row, off tech.
        let bought_back = open_buying_back(&hiding(tech(129, 9)), &owning(&[(129, 1)]));
        assert_eq!(listed(&bought_back), [129]);
        assert_eq!(bought_back.can(buy_of(129)), Err(OutfitRefusal::NotForSale));
        // (e) too dear: the cash never stops a hider.
        let dear = open(
            &hiding(OutfitRecord {
                cost: 20_000,
                ..outfit(129, &[])
            }),
            &pilot(),
        );
        assert_eq!(listed(&dear), [129]);
        assert_eq!(dear.can(buy_of(129)), Err(OutfitRefusal::CannotAfford));
        // (f) owned, plain and buyable.
        let owned = open(&hiding(outfit(129, &[])), &owning(&[(129, 1)]));
        assert_eq!(listed(&owned), [129]);
        assert_eq!(owned.can(buy_of(129)), Ok(()));
    }

    #[test]
    fn a_hider_its_own_flag_hides_hides_nothing() {
        let unmet = hiding(OutfitRecord {
            flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
            ..requiring(129, 0x2, -1)
        });
        assert_eq!(listed(&open(&unmet, &pilot())), [130]);
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let refused = hiding(gated(129, OutfitFlags::HIDE_UNLESS_AVAILABLE, "b7"));
        assert_eq!(listed(&open_gated(&refused, refusing, &pilot())), [130]);
    }

    #[test]
    fn a_lower_listed_hider_takes_an_owned_sell_anywhere_outfit_off_the_list() {
        let records = |tech_level, disp_weight| {
            [
                OutfitRecord {
                    disp_weight: 5,
                    flags: OutfitFlags::HIDE_HIGHER,
                    ..outfit(128, &[])
                },
                OutfitRecord {
                    disp_weight,
                    flags: OutfitFlags::SELL_ANYWHERE,
                    ..tech(129, tech_level)
                },
            ]
        };
        let owner = owning(&[(129, 1)]);
        let on_tech = open(&records(1, 5), &owner);
        assert_eq!(listed(&on_tech), [128]);
        assert_eq!(on_tech.can(sell_of(129)), Err(OutfitRefusal::NotListed));
        assert_eq!(listed(&open(&records(9, 5), &owner)), [128]);
        let other_weight = open(&records(9, 6), &owner);
        assert_eq!(listed(&other_weight), [129, 128]);
        assert_eq!(other_weight.can(sell_of(129)), Ok(()));
    }

    #[test]
    fn an_owned_sell_anywhere_hider_off_tech_takes_another_off() {
        let records = [
            OutfitRecord {
                disp_weight: 5,
                flags: OutfitFlags::SELL_ANYWHERE | OutfitFlags::HIDE_HIGHER,
                ..tech(128, 9)
            },
            OutfitRecord {
                disp_weight: 5,
                flags: OutfitFlags::SELL_ANYWHERE,
                ..tech(129, 9)
            },
        ];
        assert_eq!(
            listed(&open(&records, &owning(&[(128, 1), (129, 1)]))),
            [128]
        );
    }

    // Flags2 0x0400: owned outfits bought back whatever their tech.

    /// [`port`] with `Flags2` 0x0400: it buys back the outfits the player
    /// owns.
    fn buying_back() -> LandingSite {
        LandingSite {
            flags2: BUYS_OWNED_OUTFITS,
            ..port()
        }
    }

    /// The outfitter of `records` at [`buying_back`] for `pilot`.
    fn open_buying_back(records: &[OutfitRecord], pilot: &Pilot) -> Outfitter {
        open_at(records, &buying_back(), pilot).expect("an outfitter")
    }

    #[test]
    fn the_flags2_0x0400_constant_is_the_spob_bit() {
        assert_eq!(BUYS_OWNED_OUTFITS, 0x0400);
    }

    #[test]
    fn the_buys_owned_rule_cites_the_listing() {
        let value = buys_owned();
        assert_eq!(value.about().reads(), None);
        assert_eq!(value.about().name(), "buys owned");
        assert_eq!(
            value.about().reproduces(),
            "`_SetupPortAvailableItems` @0xbf51-0xbf75; `_LoadObjectData` @0x77374-0x77378"
        );
    }

    #[test]
    fn a_flags2_0x0400_stellar_lists_an_owned_outfit_tech_forbids_sell_only() {
        let outfitter = open_buying_back(&[tech(128, 9)], &owning(&[(128, 1)]));
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(outfitter.can(sell_of(128)), Ok(()));
        assert_eq!(outfitter.can(buy_of(128)), Err(OutfitRefusal::NotForSale));
        let facts = &row(&outfitter, 128).facts;
        assert_eq!((buyable(facts), facts.sold_here), (false, true));
    }

    #[test]
    fn a_flags2_0x0400_stellar_lists_no_outfit_the_player_does_not_own() {
        let records = [tech(128, 9)];
        assert!(listed(&open_buying_back(&records, &pilot())).is_empty());
        assert!(listed(&open_buying_back(&records, &owning(&[(128, 0)]))).is_empty());
    }

    #[test]
    fn a_flags2_0x0400_stellar_lists_no_owned_cannot_sell_outfit() {
        let flagged = |flags| OutfitRecord {
            flags,
            ..tech(128, 9)
        };
        let owner = owning(&[(128, 1)]);
        let unsellable = open_buying_back(&[flagged(OutfitFlags::CANNOT_SELL)], &owner);
        assert!(listed(&unsellable).is_empty());
        let persistent = open_buying_back(&[flagged(OutfitFlags::PERSISTENT)], &owner);
        assert_eq!(listed(&persistent), [128]);
        assert_eq!(persistent.can(sell_of(128)), Ok(()));
    }

    #[test]
    fn only_flags2_bit_0x0400_lists_an_owned_outfit_off_tech() {
        let site = LandingSite {
            flags2: !BUYS_OWNED_OUTFITS,
            ..port()
        };
        let outfitter = open_at(&[tech(128, 9)], &site, &owning(&[(128, 1)]));
        assert!(listed(&outfitter.expect("an outfitter")).is_empty());
    }

    #[test]
    fn a_flags2_0x0400_row_skips_the_roll_and_the_hiding() {
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let records = [
            OutfitRecord {
                tech_level: 9,
                ..buying(128, 50)
            },
            OutfitRecord {
                tech_level: 9,
                ..buying(129, 0)
            },
            OutfitRecord {
                tech_level: 9,
                flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
                ..requiring(130, 0x2, -1)
            },
            OutfitRecord {
                tech_level: 9,
                ..gated(131, OutfitFlags::HIDE_UNLESS_AVAILABLE, "b7")
            },
        ];
        let site = buying_back();
        let owner = owning(&[(128, 1), (129, 1), (130, 1), (131, 1)]);
        let mut chance = Scripted::default();
        let outfitter = Shop {
            gate: refusing,
            ..shop(&records, &site)
        }
        .outfitter(&owner, &mut DayRolls::default(), &mut chance)
        .expect("an outfitter");
        assert_eq!(listed(&outfitter), [128, 129, 130, 131]);
        assert!(chance.asked.is_empty(), "no roll asked");
    }

    #[test]
    fn a_lower_hide_higher_outfit_hides_a_flags2_0x0400_row() {
        let weighted = |id, disp_weight, tech_level, flags| OutfitRecord {
            disp_weight,
            tech_level,
            flags,
            ..outfit(id, &[])
        };
        let owner = owning(&[(129, 1)]);
        let same_weight = [
            weighted(128, 5, 1, OutfitFlags::HIDE_HIGHER),
            weighted(129, 5, 9, 0),
        ];
        assert_eq!(listed(&open_buying_back(&same_weight, &owner)), [128]);
        let other_weight = [
            weighted(128, 5, 1, OutfitFlags::HIDE_HIGHER),
            weighted(129, 6, 9, 0),
        ];
        assert_eq!(listed(&open_buying_back(&other_weight, &owner)), [129, 128]);
    }

    #[test]
    fn a_flags2_0x0400_row_goes_by_display_weight() {
        let weighted = |id, disp_weight, tech_level| OutfitRecord {
            disp_weight,
            tech_level,
            ..outfit(id, &[])
        };
        let records = [
            weighted(128, 10, 1),
            weighted(129, 5, 9),
            weighted(130, 1, 1),
            weighted(131, 10, 9),
        ];
        let owner = owning(&[(129, 1), (131, 1)]);
        assert_eq!(
            listed(&open_buying_back(&records, &owner)),
            [128, 131, 129, 130]
        );
    }

    #[test]
    fn by_the_bible_a_flags2_0x0400_row_is_listed_and_not_bought_too() {
        let records = [tech(128, 9)];
        let site = buying_back();
        let outfitter = Shop {
            rules: Rulebook::default().with_override(RuleKey::TechLevel, RuleSource::Bible),
            ..shop(&records, &site)
        }
        .outfitter(
            &owning(&[(128, 1)]),
            &mut DayRolls::default(),
            &mut NeverFires,
        )
        .expect("an outfitter");
        assert_eq!(listed(&outfitter), [128]);
        assert_eq!(outfitter.can(sell_of(128)), Ok(()));
        assert_eq!(outfitter.can(buy_of(128)), Err(OutfitRefusal::NotForSale));
        let facts = &row(&outfitter, 128).facts;
        assert_eq!((buyable(facts), facts.sold_here), (false, true));
    }

    // Price, mass and free mass.

    #[test]
    fn the_price_is_the_cost_or_the_cost_times_the_ships_mass() {
        let rules = Rulebook::default().with_override(RuleKey::MassPrice, RuleSource::Bible);
        let plain = OutfitRecord {
            cost: 1500,
            ..outfit(128, &[])
        };
        assert_eq!(unit_price(&plain, 40, &rules), 1500);
        let by_mass = OutfitRecord {
            flags: OutfitFlags::PRICE_BY_MASS,
            ..plain.clone()
        };
        assert_eq!(unit_price(&by_mass, 40, &rules), 60_000);
        assert_eq!(unit_price(&by_mass, -2, &rules), 0, "never below none");
        let negative = OutfitRecord { cost: -5, ..plain };
        assert_eq!(unit_price(&negative, 40, &rules), 0);
        let most = OutfitRecord {
            cost: i32::MAX,
            ..by_mass
        };
        assert_eq!(
            unit_price(&most, i16::MAX, &rules),
            i64::from(i32::MAX) * i64::from(i16::MAX)
        );
    }

    /// An outfit of `cost`, priced by the ship's mass or not.
    fn priced(cost: i32, by_mass: bool) -> OutfitRecord {
        OutfitRecord {
            cost,
            flags: if by_mass {
                OutfitFlags::PRICE_BY_MASS
            } else {
                0
            },
            ..outfit(128, &[])
        }
    }

    #[test]
    fn the_price_by_mass_is_at_least_the_cost_by_the_engine() {
        let engine = Rulebook::default();
        let by_mass = priced(100, true);
        assert_eq!(unit_price(&by_mass, 40, &engine), 4000);
        assert_eq!(unit_price(&by_mass, 1, &engine), 100);
        assert_eq!(unit_price(&by_mass, 0, &engine), 100, "the cost on mass 0");
        assert_eq!(unit_price(&by_mass, -2, &engine), 100, "the cost below 0");
        assert_eq!(unit_price(&priced(1500, false), 0, &engine), 1500);
        assert_eq!(unit_price(&priced(0, true), 40, &engine), 0);
        assert_eq!(unit_price(&priced(-5, true), -2, &engine), 0, "no product");
        assert_eq!(unit_price(&priced(-5, false), -2, &engine), 0);
        assert_eq!(
            unit_price(&priced(i32::MAX, true), i16::MAX, &engine),
            i64::from(i32::MAX) * i64::from(i16::MAX),
            "no 32-bit wrap"
        );
    }

    #[test]
    fn the_price_by_mass_is_the_bare_product_by_the_other_reading() {
        let other = Rulebook::default().with_override(RuleKey::MassPrice, RuleSource::Bible);
        let by_mass = priced(100, true);
        assert_eq!(unit_price(&by_mass, 0, &other), 0);
        assert_eq!(unit_price(&by_mass, -2, &other), 0);
        assert_eq!(unit_price(&by_mass, 40, &other), 4000);
        assert_eq!(unit_price(&priced(-5, true), -2, &other), 10);
        assert_eq!(unit_price(&priced(1500, false), 0, &other), 1500);
    }

    #[test]
    fn the_outfitter_lists_and_refunds_an_outfit_priced_by_mass_on_a_massless_ship() {
        let records = [priced(100, true)];
        let site = port();
        let mut owner = owning(&[(128, 1)]);
        owner.cash = 500;
        let visit = OutfitVisit {
            opened_with: owner.outfits.clone(),
            ..OutfitVisit::default()
        };
        let at = |source| {
            Shop {
                fields: ShipFields { mass: 0, ..FAST },
                visit: &visit,
                rules: Rulebook::default().with_override(RuleKey::MassPrice, source),
                ..shop(&records, &site)
            }
            .outfitter(&owner, &mut DayRolls::default(), &mut NeverFires)
            .expect("an outfitter")
        };
        let engine = at(RuleSource::Engine);
        assert_eq!(row(&engine, 128).price, 100, "the cost");
        assert_eq!(
            engine.refund(OutfitId(128)),
            Some(50),
            "half, owned on entry"
        );
        assert_eq!(
            engine.count_max(OutfitId(128), Direction::Buy),
            Some(5),
            "the cash over the cost"
        );
        let other = at(RuleSource::Bible);
        assert_eq!(row(&other, 128).price, 0);
        assert_eq!(other.refund(OutfitId(128)), Some(0));
    }

    #[test]
    fn the_outfit_price_rule_reads_mass_price_and_cites_adjusted_item_cost() {
        assert_eq!(outfit_price().about().reads(), Some(RuleKey::MassPrice));
        let cited = outfit_price().about().reproduces();
        assert!(cited.contains("@0x5869-0x5895"), "{cited}");
        assert!(cited.contains("@0xb0e3-0xb106"), "{cited}");
    }

    /// An outfit of `mass`, scaled by the ship's mass or not.
    fn of_mass(mass: i16, by_mass: bool) -> OutfitRecord {
        OutfitRecord {
            mass,
            flags: if by_mass {
                OutfitFlags::MASS_BY_MASS
            } else {
                0
            },
            ..outfit(128, &[])
        }
    }

    #[test]
    fn the_scaled_mass_is_at_least_the_outfits_own_mass_by_the_engine() {
        let engine = Rulebook::default();
        let scaled = of_mass(10, true);
        assert_eq!(unit_mass(&scaled, 155, &engine), 15, "the product wins");
        assert_eq!(unit_mass(&scaled, 100, &engine), 10);
        assert_eq!(unit_mass(&scaled, 40, &engine), 10, "the mass below 100");
        assert_eq!(unit_mass(&scaled, 0, &engine), 10, "the mass on mass 0");
        assert_eq!(unit_mass(&scaled, -100, &engine), 10, "the mass below 0");
        assert_eq!(
            unit_mass(&of_mass(1, true), 15, &engine),
            1,
            "Carbon Fiber on the Shuttle"
        );
        assert_eq!(unit_mass(&of_mass(0, true), 155, &engine), 0);
        assert_eq!(
            unit_mass(&of_mass(-5, true), -200, &engine),
            -5,
            "no product"
        );
        assert_eq!(unit_mass(&of_mass(7, false), 40, &engine), 7);
        assert_eq!(unit_mass(&of_mass(7, false), 155, &engine), 7);
        assert_eq!(
            unit_mass(&of_mass(i16::MAX, true), i16::MAX, &engine),
            i64::from(i16::MAX) * i64::from(i16::MAX) / 100,
            "no 32-bit wrap"
        );
    }

    #[test]
    fn the_scaled_mass_is_the_bare_product_by_the_other_reading() {
        let other = Rulebook::default().with_override(RuleKey::ScaledMass, RuleSource::Bible);
        let scaled = of_mass(10, true);
        assert_eq!(unit_mass(&scaled, 155, &other), 15);
        assert_eq!(unit_mass(&scaled, 40, &other), 4);
        assert_eq!(unit_mass(&scaled, 0, &other), 0);
        assert_eq!(unit_mass(&scaled, -100, &other), -10);
        assert_eq!(unit_mass(&of_mass(-5, true), -200, &other), -5);
        assert_eq!(unit_mass(&of_mass(7, false), 40, &other), 7);
    }

    #[test]
    fn the_outfit_mass_rule_reads_scaled_mass_and_cites_adjusted_item_mass() {
        assert_eq!(outfit_mass().about().reads(), Some(RuleKey::ScaledMass));
        let cited = outfit_mass().about().reproduces();
        assert!(cited.contains("@0x2889-0x28c2"), "{cited}");
        assert!(cited.contains("@0xb4c1-0xb4ed"), "{cited}");
    }

    #[test]
    fn the_outfitter_weighs_an_outfit_scaled_by_mass_at_least_at_its_own_mass() {
        // 130 of 20 tons scaled by FAST's 40 is 8: 20 by the engine.
        let records = [of_mass_id(130, 20)];
        let site = port();
        let owner = owning(&[(130, 1)]);
        let at = |source| {
            Shop {
                rules: Rulebook::default().with_override(RuleKey::ScaledMass, source),
                ..shop(&records, &site)
            }
            .outfitter(&owner, &mut DayRolls::default(), &mut NeverFires)
            .expect("an outfitter")
        };
        let engine = at(RuleSource::Engine);
        assert_eq!(row(&engine, 130).mass, 20);
        assert_eq!(engine.free_mass, 10);
        assert_eq!(engine.can(buy_of(130)), Err(OutfitRefusal::NoSpace));
        let other = at(RuleSource::Bible);
        assert_eq!(row(&other, 130).mass, 8);
        assert_eq!(other.free_mass, 22);
        assert_eq!(other.can(buy_of(130)), Ok(()));
    }

    /// Outfit `id` of `mass`, scaled by the ship's mass.
    fn of_mass_id(id: i16, mass: i16) -> OutfitRecord {
        OutfitRecord {
            id: OutfitId(id),
            ..of_mass(mass, true)
        }
    }

    #[test]
    fn the_mass_is_the_items_or_scaled_by_the_ship_when_positive() {
        let scaled = |mass| OutfitRecord {
            mass,
            flags: OutfitFlags::MASS_BY_MASS,
            ..outfit(128, &[])
        };
        assert_eq!(
            unit_mass(&scaled(10), 155, &Rulebook::default()),
            15,
            "truncated"
        );
        assert_eq!(
            unit_mass(&scaled(-5), 155, &Rulebook::default()),
            -5,
            "positive masses only"
        );
        assert_eq!(unit_mass(&scaled(0), 155, &Rulebook::default()), 0);
        let plain = OutfitRecord {
            mass: 7,
            ..outfit(128, &[])
        };
        assert_eq!(unit_mass(&plain, 155, &Rulebook::default()), 7);
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
        // 130 weighs 50 x 40 / 100 = 20, below its own 50: the other
        // reading of the scaled mass.
        let other = Rulebook::default().with_override(RuleKey::ScaledMass, RuleSource::Bible);
        let defaults = map(&[(128, 2)]);
        assert_eq!(free_mass(FAST, &defaults, &defaults, &records, &other), 30);
        assert_eq!(free_mass(FAST, &defaults, &map(&[]), &records, &other), 36);
        assert_eq!(
            free_mass(
                FAST,
                &defaults,
                &map(&[(128, 2), (129, 1)]),
                &records,
                &other
            ),
            35
        );
        assert_eq!(
            free_mass(FAST, &NO_STANDARD, &map(&[(130, 1)]), &records, &other),
            10
        );
        assert_eq!(
            free_mass(FAST, &NO_STANDARD, &map(&[(999, 4)]), &records, &other),
            30
        );
        let site = port();
        let outfitter = Shop {
            rules: other,
            ..shop(&records, &site)
        }
        .outfitter(
            &owning(&[(128, 3)]),
            &mut DayRolls::default(),
            &mut NeverFires,
        )
        .expect("an outfitter");
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
                buy_price: 1000,
                mass: 1,
                owned: 2,
                max: 7,
                facts: OutfitFacts {
                    names: lc("outfit 128", "outfits 128"),
                    listed: true,
                    buy_tech: true,
                    required: true,
                    available: true,
                    sold_here: true,
                    mass_field: 1,
                    ..OutfitFacts::default()
                },
            }]
        );
        let order = |direction| OutfitOrder {
            outfit: OutfitId(128),
            direction,
        };
        assert_eq!(
            (
                outfitter.can(order(Direction::Buy)),
                outfitter.can(order(Direction::Sell)),
                outfitter.words(OutfitId(128)),
                outfitter.cap(OutfitId(128)),
            ),
            (Ok(()), Ok(()), None, Some(7))
        );
    }

    /// The names `lc_name` and `lc_plural`.
    fn lc(singular: &str, plural: &str) -> LcNames {
        LcNames {
            singular: singular.to_owned(),
            plural: plural.to_owned(),
        }
    }

    #[test]
    fn the_outfitter_holds_the_facts_its_rules_read() {
        // Rack 128: a gun that cannot be sold, raising widget 200 and
        // unknown 999, exploring a map, launching missiles 129, its own
        // `Max` raised by 202 (two owned) and its guns by 202's mod 45.
        let records = [
            OutfitRecord {
                flags: OutfitFlags::GUN | OutfitFlags::CANNOT_SELL,
                max: 3,
                lc_name: "missile rack".to_owned(),
                lc_plural: "missile racks".to_owned(),
                ..outfit(
                    128,
                    &[(RAISES_MAX, 200), (EXPLORES_MAP, 1), (RAISES_MAX, 999)],
                )
            },
            OutfitRecord {
                lc_name: "missile".to_owned(),
                lc_plural: "missiles".to_owned(),
                ..outfit(129, &[])
            },
            OutfitRecord {
                max: 4,
                lc_name: "widget".to_owned(),
                lc_plural: "widgets".to_owned(),
                ..outfit(200, &[])
            },
            outfit(202, &[(RAISES_MAX, 128), (MORE_GUNS, 1)]),
            outfit(203, &[(MORE_CARGO, -7)]),
        ];
        let site = port();
        let launchers = rack(4, 2);
        let rules = Rulebook::default().with_override(RuleKey::SaleMass, RuleSource::Bible);
        let pilot = owning(&[(128, 2), (200, 5), (202, 2)]);
        let outfitter = Shop {
            fields: ShipFields { holds: -3, ..FAST },
            free_cargo: 42,
            ammo_caps: &BTreeMap::from([(OutfitId(128), 6)]),
            fighter_room: &BTreeMap::from([(OutfitId(128), 3)]),
            hardpoints: hardpoints(5, 1),
            launchers: &launchers,
            visit: &with_flags(MAP_BOUGHT),
            rules,
            ..shop(&records, &site)
        }
        .outfitter(&pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter");
        assert_eq!(
            outfitter.armed,
            Armed {
                guns: 2,
                turrets: 0,
                gun_limit: 6,
                turret_limit: 1,
            }
        );
        assert_eq!((outfitter.holds, outfitter.free_cargo), (-3, 42));
        assert_eq!(outfitter.visit.bought, MAP_BOUGHT);
        assert_eq!(outfitter.rules, rules);
        assert_eq!((outfitter.cash, outfitter.free_mass), (10_000, 21));
        assert_eq!(
            row(&outfitter, 128).facts,
            OutfitFacts {
                names: lc("missile rack", "missile racks"),
                flags: OutfitFlags::GUN | OutfitFlags::CANNOT_SELL,
                listed: true,
                buy_tech: true,
                required: true,
                available: true,
                sold_here: true,
                ammo_cap: Some(6),
                raised: Some(6),
                fighter_room: Some(3),
                expansion: None,
                opening_limit: Some(OpeningLimit::Map),
                launcher: Some(Launcher {
                    max_ammo: 4,
                    rounds: 2,
                    ammo: Some(OutfitId(129)),
                }),
                ammo_names: Some(lc("missile", "missiles")),
                mass_field: 1,
                raises: vec![
                    Raise {
                        target: OutfitId(200),
                        held: 5,
                        target_max: Some(4),
                        left_by_the_bible: 4,
                        target_names: Some(lc("widget", "widgets")),
                    },
                    Raise {
                        target: OutfitId(999),
                        held: 0,
                        target_max: None,
                        left_by_the_bible: 0,
                        target_names: None,
                    },
                ],
            }
        );
        let expansion = &row(&outfitter, 203).facts;
        assert_eq!(expansion.expansion, Some(7));
        assert_eq!(expansion.opening_limit, None);
        assert_eq!((expansion.raised, expansion.launcher), (None, None));
    }

    #[test]
    fn a_row_off_sale_here_is_neither_buyable_nor_sold_here_unless_it_sells_anywhere() {
        let off = |flags| OutfitRecord {
            flags,
            tech_level: 9,
            ..outfit(128, &[])
        };
        let anywhere = open(&[off(OutfitFlags::SELL_ANYWHERE)], &owning(&[(128, 1)]));
        let facts = &row(&anywhere, 128).facts;
        assert_eq!((buyable(facts), facts.sold_here), (false, true));
        let unmet = open(&[requiring(128, 0x2, -1)], &pilot());
        let facts = &row(&unmet, 128).facts;
        assert_eq!((buyable(facts), facts.sold_here), (false, true), "for sale");
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let gated = open_gated(&[gated(128, 0, "b7")], refusing, &pilot());
        let facts = &row(&gated, 128).facts;
        assert_eq!((buyable(facts), facts.sold_here), (false, true));
    }

    // An outfit asked for: the row a relisting sells (see
    // `RuleKey::RelistedSale`).

    /// The outfitter at [`port`] with the row of `also` asked for, no
    /// roll drawn yet and none that fires.
    fn open_with(records: &[OutfitRecord], pilot: &Pilot, also: Option<OutfitId>) -> Outfitter {
        shop(records, &port())
            .outfitter_with(pilot, &mut DayRolls::default(), &mut NeverFires, also)
            .expect("an outfitter")
    }

    #[test]
    fn the_first_outfit_is_128_and_the_list_has_512_slots() {
        assert_eq!(FIRST_OUTFIT, OutfitId(128));
        assert_eq!(LIST_SLOTS, 512);
    }

    #[test]
    fn an_outfit_asked_for_is_found_though_in_no_cell_and_sold_here() {
        let records = [tech(128, 9), tech(129, 0)];
        let owner = owning(&[(128, 1)]);
        let asked = open_with(&records, &owner, Some(FIRST_OUTFIT));
        assert_eq!(listed(&asked), [129], "in no cell");
        let found = row(&asked, 128);
        assert_eq!(found.owned, 1);
        assert_eq!(
            (buyable(&found.facts), found.facts.sold_here),
            (false, true)
        );
        assert_eq!(sell(&asked), Ok(()));
        let plain = open(&records, &owner);
        assert_eq!(listed(&plain), [129]);
        assert_eq!(sell(&plain), Err(OutfitRefusal::NotListed));
        assert_eq!(open_with(&records, &owner, None), plain);
    }

    #[test]
    fn an_outfit_asked_for_that_is_listed_keeps_its_one_row() {
        let records = [tech(128, 0), tech(129, 9)];
        let owner = owning(&[(128, 1)]);
        let asked = open_with(&records, &owner, Some(FIRST_OUTFIT));
        assert_eq!(asked, open(&records, &owner));
        assert_eq!(listed(&asked), [128]);
        let off_sale = open_with(&records, &owner, Some(OutfitId(129)));
        assert_eq!(listed(&off_sale), [128], "an unlisted one is in no cell");
        assert_eq!(
            off_sale.unlisted.as_ref().map(|row| row.id),
            Some(OutfitId(129))
        );
    }

    #[test]
    fn an_outfit_asked_for_that_is_off_today_is_bought_as_canbuyoutfititem_says() {
        let asked = open_with(&[buying(128, 50)], &pilot(), Some(FIRST_OUTFIT));
        assert!(!listed(&asked).contains(&128));
        assert!(!row(&asked, 128).facts.listed);
        assert_eq!(buy(&asked), Ok(()));
    }

    #[test]
    fn an_unlisted_row_asked_for_is_found_by_row_and_in_no_cell() {
        let records = [tech(128, 9), tech(129, 0)];
        let owner = owning(&[(128, 2)]);
        let asked = open_with(&records, &owner, Some(FIRST_OUTFIT));
        assert_eq!(listed(&asked), [129]);
        let unlisted = asked.unlisted.as_ref().expect("the row asked for");
        assert_eq!((unlisted.id, unlisted.owned), (OutfitId(128), 2));
        assert_eq!(asked.row(FIRST_OUTFIT), Some(unlisted));
        assert_eq!(asked.row(OutfitId(129)), asked.rows.first());
        assert_eq!(asked.row(OutfitId(130)), None);
        assert_eq!(sell(&asked), Ok(()));
        assert_eq!(open(&records, &owner).unlisted, None);
    }

    #[test]
    fn buy_asks_no_list_of_the_outfit_asked_for() {
        let records = [tech(128, 0), tech(129, 0)];
        let off_today = [
            OutfitRecord {
                buy_random: 50,
                ..tech(128, 0)
            },
            tech(129, 0),
        ];
        let asked = open_with(&off_today, &pilot(), Some(FIRST_OUTFIT));
        assert_eq!(listed(&asked), [129]);
        assert_eq!(buy(&asked), Ok(()), "bought though not listed");
        let beyond = [tech(128, 9), tech(129, 0)];
        let refused = open_with(&beyond, &pilot(), Some(FIRST_OUTFIT));
        assert_eq!(listed(&refused), [129]);
        assert_eq!(
            buy(&refused),
            Err(OutfitRefusal::NotForSale),
            "at its tech step"
        );
        assert_eq!(buy(&open(&records, &pilot())), Ok(()));
        assert!(
            listed_checks(&can_buy())
                .iter()
                .all(|&(name, _)| name != "listed"),
            "no list check"
        );
    }

    #[test]
    fn the_reselections_default_is_kept() {
        assert_eq!(Reselection::default(), Reselection::Kept);
    }

    #[test]
    fn an_outfit_asked_for_with_no_record_adds_no_row() {
        let records = [tech(129, 0)];
        let owner = owning(&[(128, 1)]);
        let asked = open_with(&records, &owner, Some(FIRST_OUTFIT));
        assert_eq!(asked, open(&records, &owner));
        assert_eq!(sell(&asked), Err(OutfitRefusal::NotListed));
    }

    // Buy's own tech test: `_CanBuyOutfitItem` @0x4e7ff-0x4e83d.

    /// Outfit `id` of `tech_level`, flagged `SELL_ANYWHERE`.
    fn anywhere(id: i16, tech_level: i16) -> OutfitRecord {
        OutfitRecord {
            flags: OutfitFlags::SELL_ANYWHERE,
            ..tech(id, tech_level)
        }
    }

    /// [`port`] of tech level `tech_level`, with `slots` as its first
    /// special tech and 0 in the rest.
    fn teching_site(tech_level: i16, slots: &[i16]) -> LandingSite {
        let mut special_tech = [0; 8];
        special_tech[..slots.len()].copy_from_slice(slots);
        LandingSite {
            tech_level,
            special_tech,
            ..port()
        }
    }

    /// The outfitter of `records` at `site` for `pilot`, reading
    /// `TechLevel` by `source`, with no roll drawn yet and none that
    /// fires.
    fn open_reading(
        records: &[OutfitRecord],
        site: &LandingSite,
        pilot: &Pilot,
        source: RuleSource,
    ) -> Outfitter {
        Shop {
            rules: Rulebook::default().with_override(RuleKey::TechLevel, source),
            ..shop(records, site)
        }
        .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter")
    }

    /// Whether a row's facts let it be bought at all, before the
    /// counts, the mass and the cash: listed, past Buy's tech test, its
    /// `Require` met and its `Availability` holding.
    fn buyable(facts: &OutfitFacts) -> bool {
        facts.listed && facts.buy_tech && facts.required && facts.available
    }

    const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

    #[test]
    fn an_owned_sell_anywhere_outfit_of_tech_minus_1_passes_buys_tech_test_at_any_stellar() {
        let records = [anywhere(128, -1)];
        let owner = owning(&[(128, 1)]);
        for tech_level in [4, -5, i16::MIN] {
            let site = teching_site(tech_level, &[]);
            let engine = open_reading(&records, &site, &owner, RuleSource::Engine);
            assert_eq!(listed(&engine), [128], "at {tech_level}");
            assert_eq!(buy(&engine), Ok(()), "at {tech_level}");
            let bible = open_reading(&records, &site, &owner, RuleSource::Bible);
            assert_eq!(listed(&bible), [128], "at {tech_level}");
            let by_tech = open_reading(&records, &site, &pilot(), RuleSource::Bible);
            if tech_level == 4 {
                assert_eq!(listed(&by_tech), [128], "tech lists it");
                assert_eq!(buy(&bible), Ok(()));
            } else {
                assert_eq!(listed(&by_tech), Vec::<i16>::new(), "at {tech_level}");
                assert_eq!(
                    buy(&bible),
                    Err(OutfitRefusal::NotForSale),
                    "at {tech_level}"
                );
                assert_eq!(sell(&bible), Ok(()), "sell-only at {tech_level}");
            }
        }
    }

    #[test]
    fn an_owned_sell_anywhere_outfit_of_tech_minus_3_is_bought_at_a_stellar_of_tech_4() {
        let records = [anywhere(128, -3)];
        let owner = owning(&[(128, 1)]);
        let site = teching_site(4, &[]);
        let unowned = open_reading(&records, &site, &pilot(), RuleSource::Engine);
        assert_eq!(listed(&unowned), Vec::<i16>::new(), "tech does not list it");
        let engine = open_reading(&records, &site, &owner, RuleSource::Engine);
        assert_eq!(listed(&engine), [128]);
        assert_eq!(buy(&engine), Ok(()));
        let by_tech = open_reading(&records, &site, &pilot(), RuleSource::Bible);
        assert_eq!(listed(&by_tech), [128], "the Bible's tech lists it");
        let bible = open_reading(&records, &site, &owner, RuleSource::Bible);
        assert_eq!(buy(&bible), Ok(()));
        let low = teching_site(-5, &[]);
        for source in READINGS {
            let outfitter = open_reading(&records, &low, &owner, source);
            assert_eq!(listed(&outfitter), [128], "{source:?}");
            assert_eq!(
                buy(&outfitter),
                Err(OutfitRefusal::NotForSale),
                "{source:?}"
            );
        }
    }

    #[test]
    fn an_owned_sell_anywhere_outfit_of_tech_0x7fff_is_bought_by_a_slot_of_0x7fff() {
        let records = [anywhere(128, i16::MAX)];
        let owner = owning(&[(128, 1)]);
        let slotted = teching_site(4, &[6, 55, i16::MAX]);
        let unowned = open_reading(&records, &slotted, &pilot(), RuleSource::Engine);
        assert_eq!(listed(&unowned), Vec::<i16>::new(), "tech does not list it");
        let by_tech = open_reading(&records, &slotted, &pilot(), RuleSource::Bible);
        assert_eq!(listed(&by_tech), [128], "the Bible's tech lists it");
        let top = teching_site(i16::MAX, &[]);
        for source in READINGS {
            let outfitter = open_reading(&records, &slotted, &owner, source);
            assert_eq!(buy(&outfitter), Ok(()), "by a slot, {source:?}");
            let outfitter = open_reading(&records, &top, &owner, source);
            assert_eq!(buy(&outfitter), Ok(()), "at 0x7fff, {source:?}");
            let outfitter = open_reading(&records, &port(), &owner, source);
            assert_eq!(listed(&outfitter), [128], "{source:?}");
            assert_eq!(
                buy(&outfitter),
                Err(OutfitRefusal::NotForSale),
                "{source:?}"
            );
        }
    }

    #[test]
    fn an_owned_sell_anywhere_outfit_above_the_stellars_tech_is_still_refused() {
        let records = [anywhere(128, 9)];
        let owner = owning(&[(128, 1)]);
        for source in READINGS {
            let outfitter = open_reading(&records, &port(), &owner, source);
            assert_eq!(listed(&outfitter), [128], "{source:?}");
            assert_eq!(
                buy(&outfitter),
                Err(OutfitRefusal::NotForSale),
                "{source:?}"
            );
            assert_eq!(sell(&outfitter), Ok(()), "{source:?}");
        }
    }

    #[test]
    fn past_buys_tech_test_an_off_tech_row_meets_the_rest_of_the_checks() {
        let low = teching_site(-5, &[]);
        let owner = owning(&[(128, 1)]);
        let engine = |record: OutfitRecord, pilot: &Pilot| {
            buy(&open_reading(&[record], &low, pilot, RuleSource::Engine))
        };
        let capped = OutfitRecord {
            max: 1,
            ..anywhere(128, -1)
        };
        assert_eq!(engine(capped, &owner), Err(OutfitRefusal::MaxOwned));
        let heavy = OutfitRecord {
            mass: 31,
            ..anywhere(128, -1)
        };
        assert_eq!(engine(heavy, &owner), Err(OutfitRefusal::NoSpace));
        let mut broke = owning(&[(128, 1)]);
        broke.cash = 0;
        assert_eq!(
            engine(anywhere(128, -1), &broke),
            Err(OutfitRefusal::CannotAfford)
        );
        let unmet = OutfitRecord {
            require: 0x2,
            require_govt: -1,
            ..anywhere(128, -1)
        };
        assert_eq!(engine(unmet, &owner), Err(OutfitRefusal::NotForSale));
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let records = [OutfitRecord {
            availability: Test::parse("b7"),
            ..anywhere(128, -1)
        }];
        let gated = Shop {
            gate: refusing,
            ..shop(&records, &low)
        }
        .outfitter(&owner, &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter");
        assert_eq!(listed(&gated), [128]);
        assert_eq!(buy(&gated), Err(OutfitRefusal::NotForSale));
    }

    #[test]
    fn a_flags2_0x0400_row_of_tech_minus_1_passes_buys_tech_test() {
        let owner = owning(&[(128, 1)]);
        let records = [tech(128, -1)];
        for source in READINGS {
            let outfitter = open_reading(&records, &buying_back(), &owner, source);
            assert_eq!(buy(&outfitter), Ok(()), "at tech 4, {source:?}");
        }
        let low = LandingSite {
            tech_level: -5,
            special_tech: [0; 8],
            ..buying_back()
        };
        let engine = open_reading(&records, &low, &owner, RuleSource::Engine);
        assert_eq!(listed(&engine), [128]);
        assert_eq!(buy(&engine), Ok(()));
        let bible = open_reading(&records, &low, &owner, RuleSource::Bible);
        assert_eq!(listed(&bible), [128]);
        assert_eq!(buy(&bible), Err(OutfitRefusal::NotForSale));
        assert_eq!(sell(&bible), Ok(()));
        let top = [tech(128, i16::MAX)];
        let slotted = LandingSite {
            special_tech: [6, 55, i16::MAX, 0, 0, 0, 0, 0],
            ..buying_back()
        };
        for source in READINGS {
            let outfitter = open_reading(&top, &slotted, &owner, source);
            assert_eq!(buy(&outfitter), Ok(()), "by a slot, {source:?}");
        }
    }

    #[test]
    fn by_the_bible_buys_tech_test_is_the_listings() {
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
            teching_site(4, &[6, 55]),
            teching_site(-5, &[0, -1]),
            teching_site(i16::MAX, &[i16::MAX]),
        ];
        for site in &sites {
            for level in levels {
                let records = [tech(128, level)];
                let asked = |source| {
                    Shop {
                        rules: Rulebook::default().with_override(RuleKey::TechLevel, source),
                        ..shop(&records, site)
                    }
                    .outfitter_with(
                        &pilot(),
                        &mut DayRolls::default(),
                        &mut NeverFires,
                        Some(FIRST_OUTFIT),
                    )
                    .expect("an outfitter")
                };
                let bible = asked(RuleSource::Bible);
                let facts = &row(&bible, 128).facts;
                let at = format!("{level} at {}", site.tech_level);
                assert_eq!(facts.listed, facts.buy_tech, "{at}");
                if facts.listed {
                    assert_eq!(buy(&bible), Ok(()), "{at}");
                }
                let engine = asked(RuleSource::Engine);
                let facts = &row(&engine, 128).facts;
                if facts.listed {
                    assert!(facts.buy_tech, "{at}");
                }
            }
        }
    }

    #[test]
    fn the_buy_tech_rule_reads_tech_level_and_cites_canbuyoutfititem() {
        let value = buy_tech();
        assert_eq!(value.about().reads(), Some(RuleKey::TechLevel));
        assert_eq!(value.about().name(), "buy tech");
        assert_eq!(
            value.about().reproduces(),
            "`_CanBuyOutfitItem` @0x4e7ff-0x4e83d; `_LoadObjectData` @0x78af3"
        );
    }

    #[test]
    fn require_and_availability_refuse_after_once_an_opening_and_before_the_cash() {
        let map = OutfitRecord {
            require: 0x2,
            require_govt: -1,
            ..outfit(128, &[(EXPLORES_MAP, 5)])
        };
        let limited_map = limited(&[map], MAP_BOUGHT, &pilot());
        assert_eq!(buy(&limited_map), Err(OutfitRefusal::BoughtThisOpening));
        let mut broke = pilot();
        broke.cash = 0;
        let unmet = open(&[requiring(128, 0x2, -1)], &broke);
        assert_eq!(buy(&unmet), Err(OutfitRefusal::NotForSale));
        let refusing = Gate {
            control_bits: &RefuseBits(&[7]),
            ..Gate::FRESH
        };
        let refused = open_gated(&[gated(128, 0, "b7")], refusing, &broke);
        assert_eq!(buy(&refused), Err(OutfitRefusal::NotForSale));
        let at_max = OutfitRecord {
            max: 1,
            ..requiring(128, 0x2, -1)
        };
        let at_max = open(&[at_max], &owning(&[(128, 1)]));
        assert_eq!(buy(&at_max), Err(OutfitRefusal::MaxOwned));
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
                crons: &[],
                ranks: &[],
                fields: FAST,
                free_cargo: ROOMY_CARGO,
                standard: &NO_STANDARD,
                site: &port(),
                fighter_room,
                ammo_caps: &NO_CAPS,
                hardpoints: ROOMY,
                launchers: &NO_LAUNCHERS,
                visit: &NO_VISIT,
                price_mult: 1.0,
                rules: Rulebook::default(),
                gate: Gate::FRESH,
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
        let expansion = [(MORE_CARGO, -5)];
        assert_eq!(holds_shop(1, &expansion, -5), Ok(()));
        assert_eq!(
            holds_shop(-1, &expansion, -5),
            Err(OutfitRefusal::NoExpansion)
        );
    }

    /// The outfitter of a ship of `holds` selling outfit 128 with `mods`
    /// and `mass`, and whether it would sell one.
    fn holds_shop(holds: i16, mods: &[(i16, i16)], mass: i16) -> Result<(), OutfitRefusal> {
        buy(&holds_outfitter(holds, mods, mass))
    }

    /// The outfitter of a ship of `holds` selling outfit 128 with `mods`
    /// and `mass`.
    fn holds_outfitter(holds: i16, mods: &[(i16, i16)], mass: i16) -> Outfitter {
        cargo_outfitter(holds, mods, mass, ROOMY_CARGO, &pilot())
    }

    /// The outfitter of a ship of `holds` with `free_cargo` tons of cargo
    /// room, selling `pilot` outfit 128 with `mods` and `mass`.
    fn cargo_outfitter(
        holds: i16,
        mods: &[(i16, i16)],
        mass: i16,
        free_cargo: i64,
        pilot: &Pilot,
    ) -> Outfitter {
        let records = [OutfitRecord {
            mass,
            ..outfit(128, mods)
        }];
        Shop {
            records: &records,
            crons: &[],
            ranks: &[],
            fields: ShipFields { holds, ..FAST },
            free_cargo,
            standard: &NO_STANDARD,
            site: &port(),
            fighter_room: &NO_FIGHTERS,
            ammo_caps: &NO_CAPS,
            hardpoints: ROOMY,
            launchers: &NO_LAUNCHERS,
            visit: &NO_VISIT,
            price_mult: 1.0,
            rules: Rulebook::default(),
            gate: Gate::FRESH,
        }
        .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("open")
    }

    /// Whether `cargo_outfitter` would sell one, to a new pilot.
    fn cargo_shop(
        holds: i16,
        mods: &[(i16, i16)],
        mass: i16,
        free_cargo: i64,
    ) -> Result<(), OutfitRefusal> {
        buy(&cargo_outfitter(holds, mods, mass, free_cargo, &pilot()))
    }

    #[test]
    fn a_mass_expansion_needs_free_cargo_of_its_size() {
        // `_CanBuyOutfitItem` @0x4e980-0x4e9a8: free cargo >= |ModVal|.
        let expansion = [(MORE_CARGO, -10)];
        assert_eq!(cargo_shop(10, &expansion, 0, 10), Ok(()), "exactly");
        assert_eq!(
            cargo_shop(10, &expansion, 0, 9),
            Err(OutfitRefusal::NoCargoRoom)
        );
        assert_eq!(
            cargo_shop(10, &[(MORE_CARGO, -11)], 0, 10),
            Err(OutfitRefusal::NoCargoRoom)
        );
        assert_eq!(
            cargo_shop(10, &expansion, 0, -1),
            Err(OutfitRefusal::NoCargoRoom),
            "more held than the space"
        );
        assert_eq!(
            cargo_shop(10, &[(MORE_CARGO, i16::MIN)], 0, 32_768),
            Ok(()),
            "the largest expansion, by its size"
        );
        assert_eq!(
            cargo_shop(10, &[(MORE_CARGO, i16::MIN)], 0, 32_767),
            Err(OutfitRefusal::NoCargoRoom)
        );
        assert_eq!(
            cargo_shop(10, &[(MORE_CARGO, -11), (MORE_CARGO, -10)], 0, 10),
            Ok(()),
            "the last cargo mod's size"
        );
        assert_eq!(
            cargo_shop(10, &[(MORE_CARGO, 50)], 0, 0),
            Ok(()),
            "a cargo pod needs no room"
        );
        assert_eq!(
            cargo_shop(-10, &expansion, 0, 0),
            Err(OutfitRefusal::NoExpansion),
            "negative Holds first"
        );
    }

    #[test]
    fn an_outfit_whose_first_mod_is_ammunition_is_never_a_mass_expansion() {
        // `_CanBuyOutfitItem` @0x4e8f6: the else-if chain skips the
        // expansion tests when slot 0 is `ModType` 3.
        let ammo_first = [(MOD_AMMO, 128), (MORE_CARGO, -5)];
        assert_eq!(cargo_shop(-1, &ammo_first, 0, 0), Ok(()), "negative Holds");
        assert_eq!(cargo_shop(10, &ammo_first, 0, 0), Ok(()), "no room");
        let ammo_second = [(MORE_CARGO, -5), (MOD_AMMO, 128)];
        assert_eq!(
            cargo_shop(-1, &ammo_second, 0, 0),
            Err(OutfitRefusal::NoExpansion)
        );
        assert_eq!(
            cargo_shop(10, &ammo_second, 0, 0),
            Err(OutfitRefusal::NoCargoRoom)
        );
    }

    #[test]
    fn the_free_mass_is_checked_before_the_expansion_tests() {
        // `_CanBuyOutfitItem`: free mass @0x4e88a before @0x4e94f.
        let expansion = [(MORE_CARGO, -5)];
        let negative = cargo_outfitter(-1, &expansion, 31, ROOMY_CARGO, &pilot());
        assert_eq!(buy(&negative), Err(OutfitRefusal::NoSpaceForAny));
        let cramped = cargo_outfitter(10, &expansion, 31, 0, &pilot());
        assert_eq!(buy(&cramped), Err(OutfitRefusal::NoSpaceForAny));
        assert_eq!(words(&cramped), Some(OutfitRefusal::NoSpaceForAny));
        let more = cargo_outfitter(10, &expansion, 16, 0, &owning(&[(128, 1)]));
        assert_eq!(buy(&more), Err(OutfitRefusal::NoSpace));
        assert_eq!(words(&more), Some(OutfitRefusal::NoSpace));
        let light = cargo_outfitter(10, &expansion, 30, 0, &pilot());
        assert_eq!(buy(&light), Err(OutfitRefusal::NoCargoRoom));
        assert_eq!(words(&light), None, "the room plays no part in the words");
    }

    #[test]
    fn a_mass_expansion_is_a_cargo_mod_of_negative_value_whatever_its_mass() {
        // `_CanBuyOutfitItem` @0x4e8bc-0x4e8d2, @0x4e94f-0x4e97e.
        assert_eq!(
            holds_shop(-1, &[(MORE_CARGO, -5)], 0),
            Err(OutfitRefusal::NoExpansion),
            "massless"
        );
        assert_eq!(
            holds_shop(-1, &[(MORE_SPEED, 1), (MORE_CARGO, -5)], 1),
            Err(OutfitRefusal::NoExpansion),
            "in any slot, with mass"
        );
        assert_eq!(
            holds_shop(-1, &[], -5),
            Ok(()),
            "negative Mass without a cargo mod"
        );
        assert_eq!(holds_shop(-1, &[(MORE_CARGO, 5)], 1), Ok(()), "a cargo pod");
        assert_eq!(
            holds_shop(-1, &[(MORE_CARGO, 0)], 0),
            Ok(()),
            "a ModVal of none"
        );
        assert_eq!(
            holds_shop(-1, &[(MORE_CARGO, -5), (MORE_CARGO, 5)], 0),
            Ok(()),
            "the last cargo mod counts"
        );
        assert_eq!(
            holds_shop(-1, &[(MORE_CARGO, 5), (MORE_CARGO, -5)], 0),
            Err(OutfitRefusal::NoExpansion),
            "the last cargo mod counts"
        );
        assert_eq!(
            holds_shop(0, &[(MORE_CARGO, -5)], 0),
            Ok(()),
            "only negative Holds forbids it"
        );
    }

    // The info box's words (#219-#222).

    /// The words outfit 128's row carries in `outfitter`.
    fn words(outfitter: &Outfitter) -> Option<OutfitRefusal> {
        outfitter.words(OutfitId(128))
    }

    /// Outfit 128 of `mass` tons, up to 9999 owned: on the FAST ship's 30
    /// tons free, one of 31 tons lacks the mass.
    fn weighing(mass: i16) -> OutfitRecord {
        OutfitRecord {
            mass,
            max: 9999,
            ..outfit(128, &[])
        }
    }

    /// The outfitter at [`port`] with outfit 128's fighter bays full.
    fn bays_full(records: &[OutfitRecord], pilot: &Pilot) -> Outfitter {
        let site = port();
        let full = BTreeMap::from([(OutfitId(128), 0)]);
        Shop {
            fighter_room: &full,
            ..shop(records, &site)
        }
        .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("open")
    }

    #[test]
    fn a_fighter_refused_for_full_bays_carries_the_mass_words_when_it_lacks_the_mass() {
        // `_OutfitDialogUpdate` @0x57e33-0x57e4f: the words never consult
        // `_CanBuyOutfitItem`, so `_CanBuyFighter` plays no part in them.
        // `_CanBuyOutfitItem` checks the free mass (@0x4e88a) before
        // `_CanBuyFighter` (@0x4e938), so the buy is refused for the mass.
        let none = bays_full(&[weighing(31)], &pilot());
        assert_eq!(buy(&none), Err(OutfitRefusal::NoSpaceForAny));
        assert_eq!(words(&none), Some(OutfitRefusal::NoSpaceForAny));
        let one = bays_full(&[weighing(31)], &owning(&[(128, 1)]));
        assert_eq!(buy(&one), Err(OutfitRefusal::NoSpace));
        assert_eq!(words(&one), Some(OutfitRefusal::NoSpace));
        let fits = bays_full(&[weighing(30)], &pilot());
        assert_eq!(buy(&fits), Err(OutfitRefusal::BaysFull));
        assert_eq!(words(&fits), None, "the mass fits");
    }

    #[test]
    fn a_row_whose_require_is_unmet_carries_the_words_it_meets() {
        let unmet = |mass, max| OutfitRecord {
            mass,
            max,
            ..requiring(128, 0x2, -1)
        };
        let heavy = open(&[unmet(31, 5)], &pilot());
        assert_eq!(buy(&heavy), Err(OutfitRefusal::NoSpaceForAny));
        assert_eq!(words(&heavy), Some(OutfitRefusal::NoSpaceForAny));
        let at_max = open(&[unmet(1, 1)], &owning(&[(128, 1)]));
        assert_eq!(buy(&at_max), Err(OutfitRefusal::MaxOwned));
        assert_eq!(words(&at_max), Some(OutfitRefusal::MaxOwned));
        let none_allowed = open(&[unmet(1, 0)], &pilot());
        assert_eq!(words(&none_allowed), Some(OutfitRefusal::NoneAllowed));
        let light = open(&[unmet(1, 5)], &pilot());
        assert_eq!(buy(&light), Err(OutfitRefusal::NotForSale));
        assert_eq!(words(&light), None);
    }

    #[test]
    fn a_mass_expansion_refused_on_negative_holds_carries_the_mass_words() {
        let heavy = holds_outfitter(-1, &[(MORE_CARGO, -5)], 31);
        assert_eq!(buy(&heavy), Err(OutfitRefusal::NoSpaceForAny));
        assert_eq!(words(&heavy), Some(OutfitRefusal::NoSpaceForAny));
        let light = holds_outfitter(-1, &[(MORE_CARGO, -5)], 30);
        assert_eq!(buy(&light), Err(OutfitRefusal::NoExpansion));
        assert_eq!(words(&light), None);
    }

    #[test]
    fn the_max_words_win_over_the_mass_words() {
        // `_OutfitDialogUpdate` @0x57dd6: `_HasMaxOfItem` first.
        let at_max = open(
            &[OutfitRecord {
                max: 1,
                ..weighing(31)
            }],
            &owning(&[(128, 1)]),
        );
        assert_eq!(buy(&at_max), Err(OutfitRefusal::MaxOwned));
        assert_eq!(words(&at_max), Some(OutfitRefusal::MaxOwned));
        let none = open(
            &[OutfitRecord {
                max: 0,
                ..weighing(31)
            }],
            &pilot(),
        );
        assert_eq!(words(&none), Some(OutfitRefusal::NoneAllowed));
        let heavy_gun = OutfitRecord {
            mass: 31,
            ..gun(128)
        };
        let limited = armed(&[heavy_gun], &pilot(), hardpoints(0, 0));
        assert_eq!(words(&limited), Some(OutfitRefusal::GunLimit));
        let site = port();
        let records = [weighing(31)];
        let caps = BTreeMap::from([(OutfitId(128), 0)]);
        let capped = Shop {
            ammo_caps: &caps,
            ..shop(&records, &site)
        }
        .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
        .expect("open");
        assert_eq!(
            words(&capped),
            Some(OutfitRefusal::MaxOwned),
            "the ammo cap"
        );
    }

    #[test]
    fn a_row_whose_mass_fits_carries_no_words() {
        let buyable = open(&[weighing(30)], &pilot());
        assert_eq!(buy(&buyable), Ok(()));
        assert_eq!(words(&buyable), None);
        let mut broke = pilot();
        broke.cash = 0;
        let unpaid = open(&[weighing(30)], &broke);
        assert_eq!(buy(&unpaid), Err(OutfitRefusal::CannotAfford));
        assert_eq!(words(&unpaid), None);
    }

    #[test]
    fn an_outfit_of_mass_none_or_less_fits_on_a_ship_with_negative_free_mass() {
        // `_CanBuyOutfitItem` @0x4e88a-0x4e892 and `_OutfitDialogUpdate`
        // @0x57e3c: a mass of 0 or less always fits.
        for mass in [0, -5] {
            let records = [
                weighing(mass),
                OutfitRecord {
                    mass: 31,
                    ..outfit(129, &[])
                },
            ];
            let outfitter = open(&records, &owning(&[(129, 1)]));
            assert_eq!(outfitter.free_mass, -1);
            assert_eq!(buy(&outfitter), Ok(()), "{mass}");
            assert_eq!(words(&outfitter), None, "{mass}");
        }
    }

    #[test]
    fn the_mass_words_need_more_than_the_free_mass() {
        let exact = open(&[weighing(30)], &pilot());
        assert_eq!(exact.free_mass, 30);
        assert_eq!(words(&exact), None, "exactly the space");
        let over = open(&[weighing(31)], &pilot());
        assert_eq!(words(&over), Some(OutfitRefusal::NoSpaceForAny));
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

    /// Whether the sale's free mass check refuses one of an outfit of
    /// `mass` on a ship of `free` tons free, read as `source` says.
    fn sale_lacks_mass(mass: i64, free: i64, source: RuleSource) -> bool {
        let outfitter = Outfitter {
            rows: vec![OutfitRow {
                mass,
                ..counted(1, 1, ROOM, ROOM)
            }],
            free_mass: free,
            rules: Rulebook::default()
                .with_override(RuleKey::SaleMass, source)
                .with_override(RuleKey::SaleRefusal, RuleSource::Bible),
            ..Outfitter::default()
        };
        outfitter.can(sell_of(128)) == Err(OutfitRefusal::NegativeFreeMass)
    }

    #[test]
    fn by_the_engine_only_an_outfit_of_negative_mass_lacks_mass_to_sell_on_the_clamped_free_mass() {
        let engine = |mass, free| sale_lacks_mass(mass, free, RuleSource::Engine);
        assert!(!engine(0, -5), "mass 0 is never checked");
        assert!(!engine(10, -5), "positive mass is never checked");
        assert!(engine(-1, -5), "the free mass reads 0, and 0 - 1 < 0");
        assert!(engine(-1, 0), "0 - 1 < 0");
        assert!(engine(-20, 10));
        assert!(!engine(-20, 20), "leaves exactly none");
        assert!(!engine(-20, 30));
    }

    #[test]
    fn by_the_other_reading_a_sale_lacks_mass_when_the_unclamped_free_mass_after_is_negative() {
        let bible = |mass, free| sale_lacks_mass(mass, free, RuleSource::Bible);
        assert!(bible(0, -5), "-5 + 0 < 0");
        assert!(!bible(10, -5), "-5 + 10 = 5");
        assert!(bible(3, -5), "-5 + 3 < 0, where the engine sells");
        assert!(!bible(10, -10), "-10 + 10 = 0");
        assert!(bible(-20, 10));
        assert!(!bible(-20, 20), "leaves exactly none");
    }

    #[test]
    fn by_the_engine_an_outfit_of_mass_0_or_more_sells_while_the_free_mass_is_below_0() {
        // FAST has 30 tons free: 30 - 60 - 5 - 0 + 20 = -15.
        let records = [
            OutfitRecord {
                mass: -20,
                ..outfit(128, &[])
            },
            OutfitRecord {
                mass: 60,
                ..outfit(129, &[])
            },
            OutfitRecord {
                mass: 5,
                ..outfit(130, &[])
            },
            OutfitRecord {
                mass: 0,
                ..outfit(131, &[])
            },
        ];
        let pilot = owning(&[(128, 1), (129, 1), (130, 1), (131, 1)]);
        let site = port();
        let sell_of = |source, id| {
            let outfitter = Shop {
                rules: Rulebook::default().with_override(RuleKey::SaleMass, source),
                ..shop(&records, &site)
            }
            .outfitter(&pilot, &mut DayRolls::default(), &mut NeverFires)
            .expect("open");
            assert_eq!(outfitter.free_mass, -15);
            outfitter.check(OutfitOrder {
                outfit: OutfitId(id),
                direction: Direction::Sell,
            })
        };
        let refused = Err(OutfitRefusal::NegativeFreeMass);
        assert_eq!(sell_of(RuleSource::Engine, 131), Ok(()), "mass 0");
        assert_eq!(sell_of(RuleSource::Engine, 130), Ok(()), "mass 5");
        assert_eq!(sell_of(RuleSource::Engine, 129), Ok(()), "mass 60");
        assert_eq!(sell_of(RuleSource::Engine, 128), refused, "mass -20");
        assert_eq!(sell_of(RuleSource::Bible, 131), refused, "-15 + 0 < 0");
        assert_eq!(sell_of(RuleSource::Bible, 130), refused, "-15 + 5 < 0");
        assert_eq!(sell_of(RuleSource::Bible, 129), Ok(()), "-15 + 60 = 45");
        assert_eq!(sell_of(RuleSource::Bible, 128), refused, "-15 - 20 < 0");
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
        outfitter.can(buy_of(id))
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
            assert_eq!(full.can(sell_of(sold)), Ok(()), "selling is free");
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
            rules: Rulebook::default().with_override(RuleKey::LauncherSale, source),
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
            refused.lc_names(sell_of(128)),
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
        assert_eq!(sold.lc_names(sell_of(128)), BTreeMap::new(), "none refused");
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
            outfitter.lc_names(sell_of(128)),
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
            rules: Rulebook::default().with_override(RuleKey::RaisedMax, RuleSource::Bible),
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

    /// Whether one more of outfit 200 of [`raisers`] can be bought owning
    /// `owned`, its ammunition cap `cap` if any and its fighter room
    /// `room` if any.
    fn capped(
        owned: &[(i16, u16)],
        cap: Option<i16>,
        room: Option<u32>,
    ) -> Result<(), OutfitRefusal> {
        let records = raisers();
        let site = port();
        let ammo_caps: BTreeMap<OutfitId, i16> =
            cap.map(|cap| (OutfitId(200), cap)).into_iter().collect();
        let fighter_room: BTreeMap<OutfitId, u32> =
            room.map(|room| (OutfitId(200), room)).into_iter().collect();
        let outfitter = Shop {
            ammo_caps: &ammo_caps,
            fighter_room: &fighter_room,
            ..shop(&records, &site)
        }
        .outfitter(&owning(owned), &mut DayRolls::default(), &mut NeverFires)
        .expect("open");
        buys(&outfitter, 200)
    }

    #[test]
    fn an_ammunition_outfit_is_refused_once_its_cap_is_owned() {
        assert_eq!(
            capped(&[(200, 3)], Some(3), None),
            Err(OutfitRefusal::MaxOwned)
        );
        assert_eq!(capped(&[(200, 2)], Some(3), None), Ok(()));
        assert_eq!(
            capped(&[(200, 3)], Some(2), None),
            Err(OutfitRefusal::MaxOwned)
        );
        assert_eq!(capped(&[(200, 3)], Some(6), None), Ok(()), "two launchers");
        assert_eq!(
            capped(&[(200, 3)], None, None),
            Ok(()),
            "uncapped: its Max of 4"
        );
        assert_eq!(
            capped(&[(200, 4)], None, None),
            Err(OutfitRefusal::MaxOwned)
        );
    }

    #[test]
    fn with_no_launcher_the_cap_is_none_and_none_can_be_bought() {
        assert_eq!(capped(&[], Some(0), None), Err(OutfitRefusal::MaxOwned));
        assert_eq!(
            capped(&[], Some(-2), None),
            Err(OutfitRefusal::MaxOwned),
            "wrapped"
        );
    }

    #[test]
    fn a_raiser_does_not_lift_the_ammunition_cap() {
        assert_eq!(
            capped(&[(200, 3), (201, 2)], None, None),
            Ok(()),
            "raised to 8"
        );
        assert_eq!(
            capped(&[(200, 3), (201, 2)], Some(3), None),
            Err(OutfitRefusal::MaxOwned)
        );
    }

    #[test]
    fn the_ammunition_cap_comes_before_the_fighter_room() {
        assert_eq!(
            capped(&[(200, 3)], Some(3), Some(0)),
            Err(OutfitRefusal::MaxOwned)
        );
        assert_eq!(
            capped(&[(200, 2)], Some(3), Some(0)),
            Err(OutfitRefusal::BaysFull)
        );
    }

    #[test]
    fn the_ammunition_cap_refuses_as_max_owned_whatever_the_outfits_max() {
        let mut records = raisers();
        let site = port();
        let ammo_caps = BTreeMap::from([(OutfitId(200), 0)]);
        for max in [0, -1] {
            records[0].max = max;
            let outfitter = Shop {
                ammo_caps: &ammo_caps,
                ..shop(&records, &site)
            }
            .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
            .expect("open");
            assert_eq!(
                buys(&outfitter, 200),
                Err(OutfitRefusal::MaxOwned),
                "Max {max}"
            );
        }
        let outfitter = open(&records, &pilot());
        assert_eq!(
            buys(&outfitter, 200),
            Err(OutfitRefusal::NoneAllowed),
            "uncapped"
        );
    }

    /// The outfitter at [`port`] owning `owned` of `records`, the raised
    /// `Max` read as `source` says.
    fn raising(records: &[OutfitRecord], owned: &[(i16, u16)], source: RuleSource) -> Outfitter {
        let site = port();
        Shop {
            rules: Rulebook::default()
                .with_override(RuleKey::RaisedMax, source)
                .with_override(RuleKey::SaleRefusal, RuleSource::Bible),
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
            |owned: &[(i16, u16)]| raising(&records, owned, RuleSource::Engine).can(sell_of(201));
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
        let at =
            |owned: &[(i16, u16)]| raising(&records, owned, RuleSource::Bible).can(sell_of(201));
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
            let at = |owned: &[(i16, u16)]| raising(&records, owned, source).can(sell_of(206));
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
            assert_eq!(outfitter.can(sell_of(207)), Ok(()), "{source:?}");
            assert_eq!(outfitter.lc_names(sell_of(207)), BTreeMap::new());
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
            refused.lc_names(sell_of(201)),
            BTreeMap::from([
                (OutfitId(200), names("widget", "widgets")),
                (OutfitId(201), names("widget rack", "widget racks")),
            ])
        );
        let sold = raising(&records, &[(201, 2), (200, 1)], RuleSource::Engine);
        assert_eq!(sold.lc_names(sell_of(201)), BTreeMap::new(), "none refused");
    }

    // The outfitter's questions.

    /// Each check of `checks` with its citation, in order.
    fn listed_checks<C>(checks: &Checks<C, OutfitRefusal>) -> Vec<(&'static str, &'static str)> {
        checks
            .descriptors()
            .map(|about| (about.name(), about.reproduces()))
            .collect()
    }

    /// The disputed rule each check of `checks` reads, in order.
    fn keys<C>(checks: &Checks<C, OutfitRefusal>) -> Vec<Option<RuleKey>> {
        checks.descriptors().map(Descriptor::reads).collect()
    }

    #[test]
    fn the_buy_checks_are_canbuyoutfititems_in_its_order() {
        assert_eq!(
            listed_checks(&can_buy()),
            [
                (
                    "tech",
                    "`_CanBuyOutfitItem` @0x4e7ff-0x4e83d, as `buy tech` reads it at build \
                     (`TechLevel` -1 skips it @0x4e802)"
                ),
                (
                    "ammunition cap",
                    "`_HasMaxOfItem` @0x457e-0x45e4, via `_CanBuyOutfitItem` @0x4e87d"
                ),
                (
                    "raised max",
                    "`_HasMaxOfItem` @0x45e9-0x46c3, the multiplier settled at build"
                ),
                ("gun limit", "`_HasMaxOfItem` @0x46c8-0x47b7"),
                ("turret limit", "`_HasMaxOfItem` @0x47c7-0x4866"),
                (
                    "free mass",
                    "`_CanBuyOutfitItem` @0x4e88a-0x4e897, reads `free_mass_clamped` \
                     (`_ShipFreeMass` @0x4e7e1)"
                ),
                (
                    "bay room",
                    "`_CanBuyOutfitItem` @0x4e8f6-0x4e93f, `_CanBuyFighter` @0x5a82"
                ),
                ("expansion allowed", "`_CanBuyOutfitItem` @0x4e94f-0x4e97e"),
                ("cargo room", "`_CanBuyOutfitItem` @0x4e980-0x4e9a8"),
                ("once an opening", "`_CanBuyOutfitItem` @0x4e9aa-0x4e9c5"),
                ("required", "`_CanBuyOutfitItem` @0x4e9cb-0x4eb37"),
                ("available", "`_CanBuyOutfitItem` @0x4eb51-0x4eb58"),
                ("cash", "`_CanBuyOutfitItem` @0x4eb95-0x4ebad"),
            ]
        );
    }

    #[test]
    fn the_has_max_checks_are_hasmaxofitems_in_order() {
        let has_max = listed_checks(&has_max());
        assert_eq!(
            has_max.iter().map(|&(name, _)| name).collect::<Vec<_>>(),
            ["ammunition cap", "raised max", "gun limit", "turret limit"]
        );
        assert_eq!(has_max, listed_checks(&can_buy())[1..5]);
    }

    #[test]
    fn the_sell_flag_is_owned_and_sellable_then_sold_here() {
        assert_eq!(
            listed_checks(&sell_flag()),
            [
                (
                    "owned",
                    "`_DoOutfitDialog` @0x5c563-0x5c570; `_OutfitFilter` @0x5808c-0x58096"
                ),
                (
                    "can be sold",
                    "`_DoOutfitDialog` @0x5c572-0x5c595; `_OutfitFilter` @0x58098-0x580b1"
                ),
                (
                    "sold here",
                    "port-only: the row as listed (`_SetupPortAvailableItems` @0xc0a8-0xc0ba \
                     lists an owned `SELL_ANYWHERE` outfit). The original's Sell flag asks no list"
                ),
            ]
        );
    }

    #[test]
    fn the_sale_checks_are_the_sell_loops_in_order() {
        assert_eq!(
            listed_checks(&sale_checks()),
            [
                (
                    "free mass",
                    "`_DoOutfitDialog` @0x5c735-0x5c7ac. The engine reads `free_mass_clamped` \
                     (`_ShipFreeMass` @0x5c740), and the other reading reads `free_mass`"
                ),
                (
                    "raised first",
                    "`_DoOutfitDialog` @0x5c7be-0x5ca6f. The engine's left is \
                     `target_max x raisers_left`, and the other's is `left_by_the_bible`"
                ),
                (
                    "ammunition first",
                    "`_DoOutfitDialog` @0x5ca75-0x5cbe0, through \
                     `Launcher::excess_left(launchers_left, Engine / Bible)`"
                ),
            ]
        );
    }

    #[test]
    fn the_value_rules_cite_the_originals_addresses() {
        let cited = [
            info_words().about().reproduces(),
            reported_cap().about().reproduces(),
            required().about().reproduces(),
            available().about().reproduces(),
            max_multiplier().about().reproduces(),
        ];
        assert_eq!(
            cited,
            [
                "`_OutfitDialogUpdate` @0x57dba-0x57e6f: `_HasMaxOfItem` @0x57dd6, then the mass \
                 @0x57e33-0x57e45 against `free_mass_clamped`",
                "`_HasMaxOfItem` out-parameter @0x4512: ammunition @0x45b2-0x45c1, raised \
                 @0x4681-0x46a0, guns @0x4798-0x47a7, turrets @0x482b-0x483a",
                "`_CanBuyOutfitItem` @0x4e9cb-0x4eb37; `_SetupPortAvailableItems` @0xbff5-0xc056",
                "`_CanBuyOutfitItem` @0x4eb51; `_SetupPortAvailableItems` @0xbfe6",
                "`_HasMaxOfItem` @0x45e9-0x46c3",
            ]
        );
    }

    #[test]
    fn each_disputed_outfitter_rule_names_its_key() {
        let mut buying = vec![None; 13];
        buying[9] = Some(RuleKey::OutfitLimit);
        assert_eq!(keys(&can_buy()), buying);
        assert_eq!(keys(&has_max()), [None; 4]);
        assert_eq!(keys(&sell_flag()), [None; 3]);
        assert_eq!(
            keys(&sale_checks()),
            [
                Some(RuleKey::SaleMass),
                Some(RuleKey::RaisedMax),
                Some(RuleKey::LauncherSale),
            ]
        );
        assert_eq!(max_multiplier().about().reads(), Some(RuleKey::RaisedMax));
        assert_eq!(buy_tech().about().reads(), Some(RuleKey::TechLevel));
        for plain in [
            *info_words().about(),
            *reported_cap().about(),
            *required().about(),
            *available().about(),
        ] {
            assert_eq!(plain.reads(), None, "{}", plain.name());
        }
    }

    #[test]
    fn the_refund_and_count_values_read_their_keys() {
        assert_eq!(refund_value().about().reads(), Some(RuleKey::OutfitRefund));
        assert_eq!(
            refund_value().about().reproduces(),
            "`_DoOutfitDialog` @0x5ceee-0x5cf08"
        );
        assert_eq!(
            option_asks_count().about().reads(),
            Some(RuleKey::OutfitCount)
        );
        assert_eq!(
            option_asks_count().about().reproduces(),
            "`_DoOutfitDialog` @0x5c0c0-0x5c1aa, @0x5c5e3-0x5c628"
        );
    }

    #[test]
    fn the_free_mass_shown_is_clamped_at_none_with_a_ton_at_exactly_one() {
        for (free_mass, tons, unit) in [
            (-15, 0, TonUnit::Tons),
            (0, 0, TonUnit::Tons),
            (1, 1, TonUnit::Ton),
            (2, 2, TonUnit::Tons),
            (30, 30, TonUnit::Tons),
        ] {
            let outfitter = Outfitter {
                free_mass,
                ..Outfitter::default()
            };
            assert_eq!(
                outfitter.free_mass_shown(),
                ShownTons { tons, unit },
                "{free_mass}"
            );
        }
    }

    #[test]
    fn the_free_mass_shown_names_what_it_reproduces() {
        let value = free_mass_shown();
        let about = value.about();
        assert_eq!(about.name(), "free mass shown");
        assert!(about.reproduces().contains("@0x57d66-0x57d9d"));
        assert!(about.reproduces().contains("free_mass_clamped"));
        assert_eq!(about.reads(), None);
    }

    #[test]
    fn the_item_mass_shown_is_none_without_mass_and_unclamped_with_it() {
        let shown = |mass_field, mass| {
            Outfitter {
                rows: vec![OutfitRow {
                    mass,
                    facts: OutfitFacts {
                        mass_field,
                        ..OutfitFacts::default()
                    },
                    ..counted(0, 0, 10, 10)
                }],
                ..Outfitter::default()
            }
            .item_mass_shown(OutfitId(128))
        };
        assert_eq!(shown(0, 0), None);
        assert_eq!(shown(-5, -5), None);
        let tons = |tons, unit| Some(ShownTons { tons, unit });
        assert_eq!(shown(1, 1), tons(1, TonUnit::Ton));
        assert_eq!(shown(1, 2), tons(2, TonUnit::Tons));
        assert_eq!(
            shown(1, 0),
            tons(0, TonUnit::Tons),
            "mass-scaled on a light class"
        );
        assert_eq!(
            Outfitter::default().item_mass_shown(OutfitId(128)),
            None,
            "not listed"
        );
    }

    #[test]
    fn the_item_mass_shown_names_what_it_reproduces() {
        let value = item_mass_shown();
        let about = value.about();
        assert_eq!(about.name(), "item mass shown");
        assert!(about.reproduces().contains("@0x57c67-0x57d19"));
        assert!(about.reproduces().contains("@0x57da2"));
        assert_eq!(about.reads(), None);
    }

    #[test]
    fn no_outfitter_check_draws() {
        let lists = [can_buy(), has_max(), sell_flag(), sale_checks()];
        for about in lists.iter().flat_map(Checks::descriptors) {
            assert!(!about.draws(), "{}", about.name());
        }
        for about in [
            *info_words().about(),
            *reported_cap().about(),
            *required().about(),
            *available().about(),
            *max_multiplier().about(),
            *option_asks_count().about(),
            *free_mass_shown().about(),
            *refund_value().about(),
        ] {
            assert!(!about.draws(), "{}", about.name());
        }
    }

    #[test]
    fn the_free_mass_checks_name_the_fact_they_read() {
        let named = |checks: &Checks<RowOutfit, OutfitRefusal>, name: &str| {
            checks
                .descriptors()
                .find(|about| about.name() == name)
                .map(Descriptor::reproduces)
                .expect("listed")
        };
        assert!(named(&can_buy(), "free mass").contains("reads `free_mass_clamped`"));
        let sale = named(&sale_checks(), "free mass");
        assert!(
            sale.contains("The engine reads `free_mass_clamped`"),
            "{sale}"
        );
        assert!(
            sale.contains("the other reading reads `free_mass`"),
            "{sale}"
        );
        let words = info_words().about().reproduces();
        assert!(words.contains("against `free_mass_clamped`"), "{words}");
    }

    #[test]
    fn the_clamped_free_mass_is_never_below_none() {
        let clamped = |free_mass| {
            Outfitter {
                free_mass,
                ..Outfitter::default()
            }
            .free_mass_clamped()
        };
        assert_eq!(clamped(-15), 0);
        assert_eq!(clamped(0), 0);
        assert_eq!(clamped(7), 7);
    }

    #[test]
    fn an_unlisted_outfit_is_not_listed_has_no_words_and_no_cap() {
        let outfitter = open(&[outfit(128, &[])], &pilot());
        let unlisted = OutfitId(999);
        for direction in [Direction::Buy, Direction::Sell] {
            let order = OutfitOrder {
                outfit: unlisted,
                direction,
            };
            assert_eq!(outfitter.can(order), Err(OutfitRefusal::NotListed));
            assert_eq!(outfitter.lc_names(order), BTreeMap::new());
        }
        assert_eq!(outfitter.words(unlisted), None);
        assert_eq!(outfitter.cap(unlisted), None);
    }

    #[test]
    fn the_outfitter_answers_from_facts_settled_when_it_was_built() {
        // 128's roll fires, 129's `Availability` is refused and 130 is
        // for sale every day.
        let records = [buying(128, 50), gated(129, 0, "b7"), outfit(130, &[])];
        let outfitter = {
            let refusing = Gate {
                control_bits: &RefuseBits(&[7]),
                ..Gate::FRESH
            };
            let mut chance = Scripted::answering(&[true]);
            let mut rolls = DayRolls::default();
            let site = port();
            let outfitter = Shop {
                gate: refusing,
                ..shop(&records, &site)
            }
            .outfitter(&owning(&[(130, 2)]), &mut rolls, &mut chance)
            .expect("an outfitter");
            assert_eq!(chance.asked, [50], "the build drew 128's roll alone");
            outfitter
        };
        let order = |id, direction| OutfitOrder {
            outfit: OutfitId(id),
            direction,
        };
        let answers = |id| {
            let outfit = OutfitId(id);
            (
                outfitter.can(order(id, Direction::Buy)),
                outfitter.can(order(id, Direction::Sell)),
                outfitter.words(outfit),
                outfitter.cap(outfit),
                outfitter.count_max(outfit, Direction::Buy),
                outfitter.count_max(outfit, Direction::Sell),
                outfitter.lc_names(order(id, Direction::Sell)),
            )
        };
        let none_owned = Err(OutfitRefusal::NoneOwned);
        assert_eq!(
            answers(128),
            (
                Ok(()),
                none_owned,
                None,
                Some(10),
                Some(10),
                None,
                BTreeMap::new()
            ),
            "the roll fired"
        );
        assert_eq!(
            answers(129),
            (
                Err(OutfitRefusal::NotForSale),
                none_owned,
                None,
                Some(10),
                None,
                None,
                BTreeMap::new()
            ),
            "the gate refused it"
        );
        assert_eq!(
            answers(130),
            (
                Ok(()),
                Ok(()),
                None,
                Some(10),
                Some(8),
                Some(2),
                BTreeMap::new()
            )
        );
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
    fn a_sale_pays_the_refund_and_removes_one() {
        let mut pilot = owning(&[(128, 2)]);
        settle(&mut pilot, &heavy(), Direction::Sell, 2000);
        assert_eq!((pilot.cash, pilot.owned(OutfitId(128))), (12_000, 1));
        settle(&mut pilot, &heavy(), Direction::Sell, 4001);
        assert_eq!(pilot.cash, 16_001);
        assert_eq!(pilot.outfits().count(), 0, "none left is none listed");
    }

    #[test]
    fn an_outfit_bought_since_the_opening_refunds_in_full() {
        assert_eq!(refund(15_001, 15_001, 3, 2, RuleSource::Engine), 15_001);
        assert_eq!(refund(15_001, 15_001, 1, 0, RuleSource::Engine), 15_001);
    }

    #[test]
    fn an_outfit_owned_at_the_opening_refunds_half() {
        assert_eq!(refund(15_001, 15_001, 2, 2, RuleSource::Engine), 7_500);
        assert_eq!(refund(15_001, 15_001, 1, 2, RuleSource::Engine), 7_500);
    }

    #[test]
    fn the_bible_reading_always_refunds_half() {
        assert_eq!(refund(15_001, 15_001, 3, 2, RuleSource::Bible), 7_500);
        assert_eq!(refund(15_001, 15_001, 2, 2, RuleSource::Bible), 7_500);
    }

    /// The outfitter of one outfit 128 of `Cost` 1500 at [`port`] for a
    /// pilot of `cash`, its buy price scaled by `price_mult`.
    fn ranked_outfitter(price_mult: f32, cash: i64) -> Outfitter {
        let records = [OutfitRecord {
            cost: 1500,
            ..outfit(128, &[])
        }];
        let site = port();
        let mut pilot = pilot();
        pilot.cash = cash;
        Shop {
            price_mult,
            ..shop(&records, &site)
        }
        .outfitter(&pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter")
    }

    #[test]
    fn by_the_engine_ranks_leave_an_outfits_price_alone() {
        let outfitter = ranked_outfitter(1.0, 10_000);
        let row = row(&outfitter, 128);
        assert_eq!((row.price, row.buy_price), (1500, 1500));
    }

    #[test]
    fn by_the_other_reading_the_buy_price_takes_the_rank_factor() {
        let outfitter = ranked_outfitter(0.85, 1275);
        let row = row(&outfitter, 128);
        assert_eq!((row.price, row.buy_price), (1500, 1275), "the base kept");
        assert_eq!(outfitter.can(buy_of(128)), Ok(()), "exactly the price");
        assert_eq!(
            ranked_outfitter(0.85, 1274).can(buy_of(128)),
            Err(OutfitRefusal::CannotAfford)
        );
        assert_eq!(
            ranked_outfitter(0.85, 3 * 1275).count_max(OutfitId(128), Direction::Buy),
            Some(3),
            "the cash over the buy price"
        );
    }

    #[test]
    fn a_refund_in_full_returns_what_was_paid_and_half_the_base_price_otherwise() {
        let refunds = |opened_with, source| {
            Outfitter {
                rows: vec![OutfitRow {
                    buy_price: 1275,
                    ..counted(1500, 3, ROOM, ROOM)
                }],
                visit: OutfitVisit {
                    opened_with: BTreeMap::from([(OutfitId(128), opened_with)]),
                    ..OutfitVisit::default()
                },
                rules: Rulebook::default().with_override(RuleKey::OutfitRefund, source),
                ..Outfitter::default()
            }
            .refund(OutfitId(128))
        };
        assert_eq!(refunds(2, RuleSource::Engine), Some(1275), "bought here");
        assert_eq!(refunds(3, RuleSource::Engine), Some(750));
        assert_eq!(refunds(2, RuleSource::Bible), Some(750));
    }

    #[test]
    fn the_refund_rule_reads_the_visits_owned_on_entry_counts() {
        // Three owned, priced at 15,001: one bought since an opening with
        // two refunds in full, by the engine; none since one with three.
        let refunds = |opened_with, source| {
            let outfitter = Outfitter {
                rows: vec![counted(15_001, 3, ROOM, ROOM)],
                visit: OutfitVisit {
                    opened_with: BTreeMap::from([(OutfitId(128), opened_with)]),
                    ..OutfitVisit::default()
                },
                rules: Rulebook::default().with_override(RuleKey::OutfitRefund, source),
                ..Outfitter::default()
            };
            assert_eq!(outfitter.refund(OutfitId(999)), None, "not listed");
            outfitter.refund(OutfitId(128))
        };
        assert_eq!(refunds(2, RuleSource::Engine), Some(15_001));
        assert_eq!(refunds(3, RuleSource::Engine), Some(7_500));
        assert_eq!(refunds(2, RuleSource::Bible), Some(7_500));
        assert_eq!(refunds(3, RuleSource::Bible), Some(7_500));
    }

    #[test]
    fn half_the_price_is_truncated_toward_zero() {
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

    // Once an opening: maps and clean records.

    /// The flags as `_GrantOutfitItem` leaves them after buying an outfit
    /// with `mods`.
    fn after(mods: &[(i16, i16)]) -> Bought {
        Bought::after_buying(&outfit(128, mods))
    }

    /// The flags with only the map one set, or only the record one.
    const MAP_BOUGHT: Bought = Bought {
        map: true,
        record: false,
    };
    const RECORD_BOUGHT: Bought = Bought {
        map: false,
        record: true,
    };

    #[test]
    fn a_map_or_a_clean_record_in_any_slot_is_limited() {
        // `_CanBuyOutfitItem` @0x4e8bc-0x4e8f4 scans slots 0-3.
        for slot in 0..4 {
            let mut mods = [(0, 0); 4];
            mods[slot] = (EXPLORES_MAP, 5);
            assert_eq!(
                opening_limit(&outfit(128, &mods)),
                Some(OpeningLimit::Map),
                "map in slot {slot}"
            );
            mods[slot] = (CLEAN_RECORD, 128);
            assert_eq!(
                opening_limit(&outfit(128, &mods)),
                Some(OpeningLimit::Record),
                "record in slot {slot}"
            );
        }
        assert_eq!(opening_limit(&outfit(128, &[(MORE_FUEL, 5)])), None);
    }

    #[test]
    fn a_map_and_a_clean_record_are_limited_by_the_map_flag_only() {
        // The else-if chain tests the map before the record (@0x4e9aa).
        let both = outfit(128, &[(CLEAN_RECORD, 128), (EXPLORES_MAP, 5)]);
        assert_eq!(opening_limit(&both), Some(OpeningLimit::Map));
        assert!(!RECORD_BOUGHT.refuses(opening_limit(&both)));
        assert!(MAP_BOUGHT.refuses(opening_limit(&both)));
    }

    #[test]
    fn ammunition_first_or_any_cargo_mod_is_never_limited() {
        // Slot 0 `ModType` 3 takes the fighter path (@0x4e8f6); any
        // `ModType` 2 the mass-expansion path (@0x4e94f), whatever its
        // sign.
        let fighter = outfit(128, &[(MOD_AMMO, 200), (EXPLORES_MAP, 5)]);
        let more = outfit(128, &[(EXPLORES_MAP, 5), (MORE_CARGO, 10)]);
        let less = outfit(128, &[(CLEAN_RECORD, 128), (MORE_CARGO, -10)]);
        for record in [&fighter, &more, &less] {
            assert_eq!(opening_limit(record), None, "{:?}", record.mods);
            let all = Bought {
                map: true,
                record: true,
            };
            assert!(!all.refuses(opening_limit(record)));
        }
        // Ammunition in a later slot does not take the fighter path.
        assert_eq!(
            opening_limit(&outfit(128, &[(EXPLORES_MAP, 5), (MOD_AMMO, 200)])),
            Some(OpeningLimit::Map)
        );
    }

    #[test]
    fn buying_a_map_sets_the_map_flag_and_clears_the_record_flag() {
        // `_GrantOutfitItem` clears both (@0x44d67-0x44d7d), then sets
        // `_boughtMap` for a `ModType` 16 whatever its `ModVal`
        // (@0x44ea9-0x44eaf).
        assert_eq!(after(&[(EXPLORES_MAP, 5)]), MAP_BOUGHT);
        assert_eq!(after(&[(MORE_FUEL, 1), (EXPLORES_MAP, 0)]), MAP_BOUGHT);
        assert_eq!(after(&[(EXPLORES_MAP, -2)]), MAP_BOUGHT);
        assert_eq!(after(&[(CLEAN_RECORD, 128), (EXPLORES_MAP, 5)]), MAP_BOUGHT);
        // A cargo mod does not stop the grant setting it.
        assert_eq!(after(&[(MORE_CARGO, -10), (EXPLORES_MAP, 5)]), MAP_BOUGHT);
    }

    #[test]
    fn buying_a_clean_record_without_paint_sets_the_record_flag() {
        // @0x44ecc-0x44f8b: a `ModType` 21, with no map and no paint mod.
        assert_eq!(after(&[(CLEAN_RECORD, 128)]), RECORD_BOUGHT);
        assert_eq!(after(&[(MORE_FUEL, 1), (CLEAN_RECORD, 128)]), RECORD_BOUGHT);
        assert_eq!(after(&[(CLEAN_RECORD, 128), (PAINT, 3)]), Bought::default());
        assert_eq!(after(&[(PAINT, 3), (CLEAN_RECORD, 128)]), Bought::default());
    }

    #[test]
    fn buying_any_other_outfit_clears_both_flags() {
        assert_eq!(after(&[(MORE_FUEL, 5)]), Bought::default());
        assert_eq!(after(&[]), Bought::default());
        assert_eq!(after(&[(PAINT, 3)]), Bought::default());
    }

    #[test]
    fn the_limit_rule_reads_the_visits_flags_under_each_reading() {
        // Both flags set: by the engine each refuses its outfit, as the
        // flags stand; by the other reading neither does, as if clear.
        let all = Bought {
            map: true,
            record: true,
        };
        let limited_by = |limit, source| {
            let mut row = counted(1, 0, ROOM, ROOM);
            row.facts.opening_limit = Some(limit);
            let outfitter = Outfitter {
                rows: vec![row],
                cash: 1,
                visit: with_flags(all),
                rules: Rulebook::default().with_override(RuleKey::OutfitLimit, source),
                ..Outfitter::default()
            };
            outfitter.can(buy_of(128)) == Err(OutfitRefusal::BoughtThisOpening)
        };
        for limit in [OpeningLimit::Map, OpeningLimit::Record] {
            assert!(limited_by(limit, RuleSource::Engine), "{limit:?}");
            assert!(!limited_by(limit, RuleSource::Bible), "{limit:?}");
        }
    }

    #[test]
    fn opening_a_visit_clears_the_flags_and_copies_the_counts() {
        let owned = BTreeMap::from([(OutfitId(128), 2), (OutfitId(130), 1)]);
        let visit = OutfitVisit::open(&owned);
        assert_eq!(visit.bought, Bought::default());
        assert_eq!(visit.opened_with, owned);
        assert_eq!(visit.opened_with(OutfitId(128)), 2);
        assert_eq!(visit.opened_with(OutfitId(130)), 1);
        assert_eq!(visit.opened_with(OutfitId(129)), 0, "not owned on entry");
    }

    #[test]
    fn buying_sets_the_visits_flags_afresh_and_keeps_its_counts() {
        let owned = BTreeMap::from([(OutfitId(128), 2)]);
        let mut visit = OutfitVisit::open(&owned);
        visit.bought(&outfit(129, &[(EXPLORES_MAP, 5)]));
        assert_eq!(visit.bought, MAP_BOUGHT);
        visit.bought(&outfit(130, &[(CLEAN_RECORD, 128)]));
        assert_eq!(visit.bought, RECORD_BOUGHT);
        visit.bought(&outfit(128, &[(MORE_FUEL, 5)]));
        assert_eq!(visit.bought, Bought::default());
        assert_eq!(visit.opened_with, owned, "the counts on entry kept");
    }

    #[test]
    fn the_shop_hands_its_visit_to_the_outfitter() {
        let records = [outfit(128, &[])];
        let site = port();
        let visit = OutfitVisit {
            bought: MAP_BOUGHT,
            opened_with: BTreeMap::from([(OutfitId(128), 3)]),
        };
        let outfitter = Shop {
            visit: &visit,
            ..shop(&records, &site)
        }
        .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter");
        assert_eq!(outfitter.visit, visit);
    }

    /// The outfitter of `records` after `bought`, to `pilot`.
    fn limited(records: &[OutfitRecord], bought: Bought, pilot: &Pilot) -> Outfitter {
        let site = port();
        Shop {
            visit: &with_flags(bought),
            ..shop(records, &site)
        }
        .outfitter(pilot, &mut DayRolls::default(), &mut NeverFires)
        .expect("open")
    }

    #[test]
    fn a_map_bought_this_opening_refuses_every_map_but_not_a_record() {
        let records = [
            outfit(128, &[(EXPLORES_MAP, 5)]),
            outfit(129, &[(EXPLORES_MAP, 7)]),
            outfit(130, &[(CLEAN_RECORD, 128)]),
            outfit(131, &[(MORE_FUEL, 5)]),
        ];
        let outfitter = limited(&records, MAP_BOUGHT, &pilot());
        assert_eq!(buys(&outfitter, 128), Err(OutfitRefusal::BoughtThisOpening));
        assert_eq!(buys(&outfitter, 129), Err(OutfitRefusal::BoughtThisOpening));
        assert_eq!(buys(&outfitter, 130), Ok(()));
        assert_eq!(buys(&outfitter, 131), Ok(()));
        // The info box does not read the flags (`_OutfitDialogUpdate`).
        assert_eq!(outfitter.words(OutfitId(128)), None);
        let outfitter = limited(&records, RECORD_BOUGHT, &pilot());
        assert_eq!(buys(&outfitter, 128), Ok(()));
        assert_eq!(buys(&outfitter, 130), Err(OutfitRefusal::BoughtThisOpening));
        let outfitter = limited(&records, Bought::default(), &pilot());
        assert_eq!(buys(&outfitter, 128), Ok(()));
        assert_eq!(buys(&outfitter, 130), Ok(()));
    }

    #[test]
    fn the_opening_limit_comes_after_max_and_free_mass_and_before_cash() {
        let records = [OutfitRecord {
            max: 1,
            ..outfit(128, &[(EXPLORES_MAP, 5)])
        }];
        let outfitter = limited(&records, MAP_BOUGHT, &owning(&[(128, 1)]));
        assert_eq!(buy(&outfitter), Err(OutfitRefusal::MaxOwned));
        assert_eq!(words(&outfitter), Some(OutfitRefusal::MaxOwned));
        let mut broke = pilot();
        broke.cash = 0;
        let outfitter = limited(&records, MAP_BOUGHT, &broke);
        assert_eq!(buy(&outfitter), Err(OutfitRefusal::BoughtThisOpening));
        assert_eq!(words(&outfitter), None);
        let heavy = [OutfitRecord {
            mass: 99,
            ..outfit(128, &[(EXPLORES_MAP, 5)])
        }];
        let outfitter = limited(&heavy, MAP_BOUGHT, &pilot());
        assert_eq!(buy(&outfitter), Err(OutfitRefusal::NoSpaceForAny));
    }

    // The cap `_HasMaxOfItem` reports.

    /// The cap reported for outfit 200 of `records` owning `owned`, its
    /// ammunition cap `ammo` if any, on `hardpoints`, the raised `Max`
    /// read as `source` says.
    fn reported(
        records: &[OutfitRecord],
        owned: &[(i16, u16)],
        ammo: Option<i16>,
        hardpoints: Hardpoints,
        source: RuleSource,
    ) -> i16 {
        let site = port();
        let ammo_caps: BTreeMap<OutfitId, i16> =
            ammo.map(|cap| (OutfitId(200), cap)).into_iter().collect();
        let outfitter = Shop {
            ammo_caps: &ammo_caps,
            hardpoints,
            rules: Rulebook::default()
                .with_override(RuleKey::RaisedMax, source)
                .with_override(RuleKey::SaleRefusal, RuleSource::Bible),
            ..shop(records, &site)
        }
        .outfitter(&owning(owned), &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter");
        outfitter.cap(OutfitId(200)).expect("listed")
    }

    /// The cap reported for outfit 200 of [`raisers`] (`Max` 4), by the
    /// engine, with room for any gun.
    fn raised_cap(owned: &[(i16, u16)], ammo: Option<i16>) -> i16 {
        reported(&raisers(), owned, ammo, ROOMY, RuleSource::Engine)
    }

    #[test]
    fn the_reported_cap_is_the_max() {
        assert_eq!(raised_cap(&[], None), 4);
        assert_eq!(raised_cap(&[(200, 2)], None), 4, "whatever is owned");
    }

    #[test]
    fn the_reported_cap_is_lowered_to_a_smaller_ammunition_cap() {
        assert_eq!(raised_cap(&[], Some(3)), 3);
        assert_eq!(raised_cap(&[], Some(4)), 4);
        assert_eq!(raised_cap(&[], Some(6)), 4, "a larger one leaves the Max");
    }

    #[test]
    fn a_raised_max_replaces_the_reported_cap_even_over_a_lower_ammunition_cap() {
        assert_eq!(raised_cap(&[(201, 2)], None), 8);
        assert_eq!(raised_cap(&[(201, 2)], Some(3)), 8, "@0x4681-0x46a0");
        assert_eq!(raised_cap(&[(201, 1)], Some(3)), 3, "one raiser: x1");
    }

    #[test]
    fn the_reported_cap_reads_the_raised_max_as_its_rule_says() {
        // Outfit 203 names outfit 200 twice.
        let at = |source| reported(&raisers(), &[(203, 1)], None, ROOMY, source);
        assert_eq!(at(RuleSource::Engine), 8);
        assert_eq!(at(RuleSource::Bible), 4);
    }

    #[test]
    fn the_reported_cap_saturates_at_16_bits() {
        let mut records = raisers();
        for (max, cap) in [(30_000, i16::MAX), (-30_000, i16::MIN)] {
            records[0].max = max;
            assert_eq!(
                reported(&records, &[(201, 2)], None, ROOMY, RuleSource::Engine),
                cap,
                "Max {max}"
            );
        }
    }

    #[test]
    fn a_guns_reported_cap_is_lowered_to_the_ships_gun_limit() {
        let mut records = raisers();
        records[0].max = 10;
        records[0].flags = OutfitFlags::GUN;
        let at = |owned: &[(i16, u16)], guns| {
            reported(
                &records,
                owned,
                None,
                hardpoints(guns, 1),
                RuleSource::Engine,
            )
        };
        assert_eq!(at(&[], 3), 3);
        assert_eq!(at(&[(200, 1)], 3), 3, "the whole limit, not what is left");
        assert_eq!(at(&[], 10), 10);
        assert_eq!(at(&[], 12), 10, "a larger limit leaves the Max");
    }

    #[test]
    fn a_turrets_reported_cap_is_lowered_to_the_ships_turret_limit() {
        let mut records = raisers();
        records[0].max = 10;
        let at = |records: &[OutfitRecord], guns, turrets| {
            reported(
                records,
                &[],
                None,
                hardpoints(guns, turrets),
                RuleSource::Engine,
            )
        };
        records[0].flags = OutfitFlags::TURRET;
        assert_eq!(at(&records, 1, 3), 3);
        assert_eq!(at(&records, 1, 10), 10);
        assert_eq!(at(&records, 1, 12), 10);
        records[0].flags = OutfitFlags::GUN | OutfitFlags::TURRET;
        assert_eq!(at(&records, 3, 2), 2, "flagged both: the smaller");
        assert_eq!(at(&records, 2, 3), 2);
    }

    // The quantity dialog's maximum.

    /// Outfit 128 at `price`, massless, `owned` owned, of `max` and
    /// reported `cap`, which can be bought and sold here: a cap below its
    /// `Max` is its ammunition cap, and one above it its raised `Max`.
    fn counted(price: i64, owned: u16, max: i16, cap: i16) -> OutfitRow {
        OutfitRow {
            id: OutfitId(128),
            name: String::new(),
            short_name: String::new(),
            price,
            buy_price: price,
            mass: 0,
            owned,
            max,
            facts: OutfitFacts {
                listed: true,
                buy_tech: true,
                required: true,
                available: true,
                sold_here: true,
                ammo_cap: (cap < max).then_some(cap),
                raised: (cap > max).then_some(i64::from(cap)),
                ..OutfitFacts::default()
            },
        }
    }

    /// The count the dialog opens at to buy `row` with `cash`.
    fn most_bought(row: OutfitRow, cash: i64) -> Option<u32> {
        Outfitter {
            rows: vec![row],
            cash,
            ..Outfitter::default()
        }
        .count_max(OutfitId(128), Direction::Buy)
    }

    /// The count the dialog opens at to sell `row`.
    fn most_sold(row: OutfitRow) -> Option<u32> {
        Outfitter {
            rows: vec![row],
            ..Outfitter::default()
        }
        .count_max(OutfitId(128), Direction::Sell)
    }

    /// The most a row has room for: `Max` and cap alike.
    const ROOM: i16 = i16::MAX;

    /// What Alt asks for `direction` on `row` with `cash`,
    /// [`RuleKey::OutfitCount`] reading as `source`.
    fn asked(row: OutfitRow, cash: i64, source: RuleSource, direction: Direction) -> Option<u32> {
        Outfitter {
            rows: vec![row],
            cash,
            rules: Rulebook::default().with_override(RuleKey::OutfitCount, source),
            ..Outfitter::default()
        }
        .count_asked(OutfitId(128), direction)
    }

    #[test]
    fn by_the_engine_alt_asks_at_the_maximum() {
        let engine = RuleSource::Engine;
        assert_eq!(
            Outfitter::default().rules.source_for(RuleKey::OutfitCount),
            engine
        );
        let buy = counted(1000, 0, ROOM, ROOM);
        assert_eq!(asked(buy.clone(), 3000, engine, Direction::Buy), Some(3));
        assert_eq!(asked(buy, 1999, engine, Direction::Buy), None, "at 1");
        let sale = counted(1000, 2, ROOM, ROOM);
        assert_eq!(asked(sale, 0, engine, Direction::Sell), Some(2));
        let one = counted(1000, 1, ROOM, ROOM);
        assert_eq!(asked(one, 0, engine, Direction::Sell), None, "at 1");
    }

    #[test]
    fn by_the_other_count_reading_alt_never_asks() {
        let bible = RuleSource::Bible;
        let buy = counted(1000, 0, ROOM, ROOM);
        assert_eq!(asked(buy.clone(), 3000, bible, Direction::Buy), None);
        assert_eq!(asked(buy, 1999, bible, Direction::Buy), None);
        let sale = counted(1000, 2, ROOM, ROOM);
        assert_eq!(asked(sale, 0, bible, Direction::Sell), None);
    }

    #[test]
    fn the_shop_carries_the_count_rule_to_its_outfitter() {
        let records = [outfit(128, &[])];
        let site = port();
        for source in RuleSource::ALL {
            let outfitter = Shop {
                rules: Rulebook::default().with_override(RuleKey::OutfitCount, source),
                ..shop(&records, &site)
            }
            .outfitter(&pilot(), &mut DayRolls::default(), &mut NeverFires)
            .expect("an outfitter");
            assert_eq!(outfitter.rules.source_for(RuleKey::OutfitCount), source);
        }
    }

    #[test]
    fn an_outfit_not_listed_opens_no_dialog() {
        let outfitter = Outfitter::default();
        assert_eq!(outfitter.count_max(OutfitId(128), Direction::Buy), None);
        assert_eq!(outfitter.count_max(OutfitId(128), Direction::Sell), None);
    }

    #[test]
    fn the_buy_maximum_is_the_cash_over_the_price_truncated() {
        assert_eq!(most_bought(counted(1000, 0, ROOM, ROOM), 5999), Some(5));
        assert_eq!(most_bought(counted(1000, 0, ROOM, ROOM), 6000), Some(6));
    }

    #[test]
    fn the_buy_maximum_is_at_most_32000() {
        let at = |cash| most_bought(counted(1, 0, ROOM, ROOM), cash);
        assert_eq!(at(31_999), Some(31_999));
        assert_eq!(at(32_000), Some(32_000));
        assert_eq!(at(32_001), Some(32_000));
        assert_eq!(at(1_000_000), Some(32_000));
    }

    #[test]
    fn an_outfit_priced_at_nothing_or_less_is_bought_up_to_its_max() {
        for price in [0, -5] {
            assert_eq!(most_bought(counted(price, 0, 7, 7), 0), Some(7), "{price}");
            assert_eq!(
                most_bought(counted(price, 0, ROOM, ROOM), 0),
                Some(32_000),
                "{price}"
            );
        }
    }

    #[test]
    fn the_buy_maximum_is_at_most_the_max_less_those_owned() {
        assert_eq!(most_bought(counted(1, 4, 10, 20), 1000), Some(6));
        assert_eq!(most_bought(counted(1, 4, 10, 20), 5), Some(5), "the cash");
    }

    #[test]
    fn a_max_owned_already_leaves_the_raised_cap_less_those_owned() {
        // A raised `Max` lets more be bought than the raw one.
        assert_eq!(most_bought(counted(1, 4, 4, 8), 1000), Some(4));
        assert_eq!(most_bought(counted(1, 5, 4, 8), 1000), Some(3));
        assert_eq!(most_bought(counted(1, 5, 4, 8), 2), Some(2), "the cash");
    }

    #[test]
    fn the_buy_maximum_is_at_most_the_reported_cap() {
        assert_eq!(most_bought(counted(1, 0, 10, 3), 1000), Some(3));
    }

    #[test]
    fn the_dialog_offers_a_gun_more_than_the_gun_limit_leaves() {
        // Guns 128 and 129 owned on a ship of 3: one more can be bought,
        // and the dialog offers its cap of 3 less the 1 of 128 owned.
        let records = [gun(128), gun(129)];
        let outfitter = armed(&records, &owning(&[(128, 1), (129, 1)]), hardpoints(3, 0));
        assert_eq!(buys(&outfitter, 128), Ok(()));
        assert_eq!(outfitter.cap(OutfitId(128)), Some(3));
        assert_eq!(outfitter.count_max(OutfitId(128), Direction::Buy), Some(2));
    }

    #[test]
    fn a_buy_maximum_of_1_or_less_opens_no_dialog() {
        let at = |cash, cap| most_bought(counted(10, 3, ROOM, cap), cash);
        assert_eq!(at(19, ROOM), None, "cash for one");
        assert_eq!(at(20, ROOM), Some(2));
        assert_eq!(at(0, ROOM), None);
        assert_eq!(at(-100, ROOM), None, "cash below nothing");
        assert_eq!(at(1000, 4), None, "a cap one past those owned");
        assert_eq!(at(1000, 5), Some(2));
        assert_eq!(at(1000, 2), None, "a cap below those owned");
        assert_eq!(most_bought(counted(10, 3, 4, 8), 1000), None, "Max");
        assert_eq!(most_bought(counted(10, 3, 5, 10), 1000), Some(2));
    }

    /// Outfit 128 of `outfitter` ordered `direction`.
    fn order_128(direction: Direction) -> OutfitOrder {
        OutfitOrder {
            outfit: OutfitId(128),
            direction,
        }
    }

    #[test]
    fn a_refused_buy_opens_no_dialog() {
        // A map, after a map was bought this opening.
        let mut refused = counted(1, 0, ROOM, ROOM);
        refused.facts.opening_limit = Some(OpeningLimit::Map);
        let outfitter = Outfitter {
            rows: vec![refused],
            cash: 1000,
            visit: with_flags(MAP_BOUGHT),
            ..Outfitter::default()
        };
        assert_eq!(
            outfitter.can(order_128(Direction::Buy)),
            Err(OutfitRefusal::BoughtThisOpening)
        );
        assert_eq!(outfitter.count_max(OutfitId(128), Direction::Buy), None);
    }

    #[test]
    fn the_sale_maximum_is_the_count_owned() {
        assert_eq!(most_sold(counted(1, 2, ROOM, ROOM)), Some(2));
        assert_eq!(most_sold(counted(1, 7, 3, 3)), Some(7), "whatever the Max");
        assert_eq!(most_sold(counted(1, 1, ROOM, ROOM)), None);
        assert_eq!(most_sold(counted(1, 0, ROOM, ROOM)), None);
    }

    #[test]
    fn the_sale_maximum_is_at_most_32000_of_a_count_read_as_16_bits() {
        let at = |owned| most_sold(counted(1, owned, ROOM, ROOM));
        assert_eq!(at(32_000), Some(32_000));
        assert_eq!(at(32_001), Some(32_000));
        assert_eq!(at(32_767), Some(32_000));
        assert_eq!(at(32_768), None, "read as -32768");
        assert_eq!(at(u16::MAX), None, "read as -1");
    }

    /// The outfitter of `row` alone, with no free mass and no cash.
    fn of_row(row: OutfitRow) -> Outfitter {
        Outfitter {
            rows: vec![row],
            ..Outfitter::default()
        }
    }

    #[test]
    fn a_sale_the_sell_button_refuses_opens_no_dialog() {
        // The Sell flag is set only for some owned and not `CANNOT_SELL`
        // (@0x5c563-0x5c595); the port also refuses an outfit that can't
        // be sold here, which it never lists.
        let mut unsellable = counted(1, 3, ROOM, ROOM);
        unsellable.facts.flags = OutfitFlags::CANNOT_SELL;
        let mut elsewhere = counted(1, 3, ROOM, ROOM);
        elsewhere.facts.sold_here = false;
        for (refusal, refused) in [
            (OutfitRefusal::NoneOwned, counted(1, 0, ROOM, ROOM)),
            (OutfitRefusal::CannotSell, unsellable),
            (OutfitRefusal::NotBoughtHere, elsewhere),
        ] {
            let outfitter = of_row(refused);
            assert_eq!(
                outfitter.can(order_128(Direction::Sell)),
                Err(refusal),
                "{refusal:?}"
            );
            assert_eq!(
                outfitter.count_max(OutfitId(128), Direction::Sell),
                None,
                "{refusal:?}"
            );
        }
    }

    #[test]
    fn a_sale_refused_inside_the_engines_loop_still_opens_the_dialog() {
        // Free mass, a raised `Max` and a launcher's ammunition are
        // checked each pass of the sell loop (@0x5c735-0x5cbe0), not
        // before the dialog opens.
        let negative = OutfitRow {
            mass: -5,
            ..counted(1, 3, ROOM, ROOM)
        };
        let mut raiser = counted(1, 3, ROOM, ROOM);
        raiser.facts.raises = vec![Raise {
            target: OutfitId(200),
            held: 5,
            target_max: Some(2),
            left_by_the_bible: 0,
            target_names: None,
        }];
        let mut launcher = counted(1, 3, ROOM, ROOM);
        launcher.facts.launcher = Some(Launcher {
            max_ammo: 1,
            rounds: 4,
            ammo: Some(OutfitId(158)),
        });
        for (refusal, refused) in [
            (OutfitRefusal::NegativeFreeMass, negative),
            (
                OutfitRefusal::RaisedFirst {
                    count: 1,
                    target: OutfitId(200),
                },
                raiser,
            ),
            (
                OutfitRefusal::AmmunitionFirst {
                    rounds: 2,
                    ammo: Some(OutfitId(158)),
                },
                launcher,
            ),
        ] {
            let outfitter = Outfitter {
                rules: Rulebook::default().with_override(RuleKey::SaleRefusal, RuleSource::Bible),
                ..of_row(refused)
            };
            assert_eq!(
                outfitter.can(order_128(Direction::Sell)),
                Err(refusal),
                "{refusal:?}"
            );
            assert_eq!(
                outfitter.count_max(OutfitId(128), Direction::Sell),
                Some(3),
                "{refusal:?}"
            );
        }
    }

    // When Sell is enabled, and where a sale refused in the loop is worded
    // (`RuleKey::SaleRefusal`).

    /// The outfitter of `row` alone, `SaleRefusal` read as `source` says.
    fn of_row_reading(row: OutfitRow, source: RuleSource) -> Outfitter {
        Outfitter {
            rules: Rulebook::default().with_override(RuleKey::SaleRefusal, source),
            ..of_row(row)
        }
    }

    /// Outfit 128, named, 3 owned, refused only by each of the sell
    /// loop's checks in turn: the free mass, a raised `Max` (its target
    /// named) and a launcher's ammunition (named), each with its refusal.
    fn refused_in_the_loop() -> [(OutfitRefusal, OutfitRow); 3] {
        let mut sold = counted(1, 3, ROOM, ROOM);
        sold.facts.names = lc("rack", "racks");
        let negative = OutfitRow {
            mass: -5,
            ..sold.clone()
        };
        let mut raiser = sold.clone();
        raiser.facts.raises = vec![Raise {
            target: OutfitId(200),
            held: 5,
            target_max: Some(2),
            left_by_the_bible: 0,
            target_names: Some(lc("widget", "widgets")),
        }];
        let mut launcher = sold;
        launcher.facts.launcher = Some(Launcher {
            max_ammo: 1,
            rounds: 4,
            ammo: Some(OutfitId(158)),
        });
        launcher.facts.ammo_names = Some(lc("round", "rounds"));
        [
            (OutfitRefusal::NegativeFreeMass, negative),
            (
                OutfitRefusal::RaisedFirst {
                    count: 1,
                    target: OutfitId(200),
                },
                raiser,
            ),
            (
                OutfitRefusal::AmmunitionFirst {
                    rounds: 2,
                    ammo: Some(OutfitId(158)),
                },
                launcher,
            ),
        ]
    }

    /// Outfit 128 refused by each of the Sell flag's checks in turn, with
    /// its refusal: none owned, `CANNOT_SELL`, and not sold here.
    fn refused_by_the_sell_flag() -> [(OutfitRefusal, OutfitRow); 3] {
        let mut unsellable = counted(1, 3, ROOM, ROOM);
        unsellable.facts.flags = OutfitFlags::CANNOT_SELL;
        let mut elsewhere = counted(1, 3, ROOM, ROOM);
        elsewhere.facts.sold_here = false;
        [
            (OutfitRefusal::NoneOwned, counted(1, 0, ROOM, ROOM)),
            (OutfitRefusal::CannotSell, unsellable),
            (OutfitRefusal::NotBoughtHere, elsewhere),
        ]
    }

    #[test]
    fn by_the_engine_sell_is_enabled_by_the_sell_flag_alone() {
        for (refusal, row) in refused_in_the_loop() {
            for outfitter in [of_row(row.clone()), of_row_reading(row, RuleSource::Engine)] {
                let sale = order_128(Direction::Sell);
                assert_eq!(outfitter.can(sale), Ok(()), "{refusal:?}");
                assert_eq!(outfitter.check(sale), Err(refusal), "{refusal:?}");
            }
        }
    }

    #[test]
    fn by_the_other_reading_any_sale_check_greys_sell() {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row_reading(row, RuleSource::Bible);
            assert_eq!(
                outfitter.can(order_128(Direction::Sell)),
                Err(refusal),
                "{refusal:?}"
            );
        }
    }

    #[test]
    fn by_either_reading_a_sell_flag_refusal_greys_sell() {
        for source in RuleSource::ALL {
            for (refusal, row) in refused_by_the_sell_flag() {
                let outfitter = of_row_reading(row, source);
                let sale = order_128(Direction::Sell);
                assert_eq!(outfitter.can(sale), Err(refusal), "{source:?} {refusal:?}");
                assert_eq!(
                    outfitter.check(sale),
                    Err(refusal),
                    "{source:?} {refusal:?}"
                );
            }
        }
    }

    #[test]
    fn a_click_is_checked_by_every_sale_check_whatever_the_reading() {
        for source in RuleSource::ALL {
            for (refusal, row) in refused_in_the_loop() {
                let outfitter = of_row_reading(row, source);
                assert_eq!(
                    outfitter.check(order_128(Direction::Sell)),
                    Err(refusal),
                    "{source:?}"
                );
            }
            let sold = of_row_reading(counted(1, 3, ROOM, ROOM), source);
            assert_eq!(sold.can(order_128(Direction::Sell)), Ok(()), "{source:?}");
            assert_eq!(sold.check(order_128(Direction::Sell)), Ok(()), "{source:?}");
        }
    }

    #[test]
    fn the_buttons_sale_checks_read_the_sale_refusal_rule_and_the_checks_keep_their_own() {
        let button = sale_checks_on_the_button();
        assert_eq!(button.about().name(), "sale checks on the button");
        assert_eq!(button.about().reads(), Some(RuleKey::SaleRefusal));
        assert_eq!(
            button.about().reproduces(),
            "`_DoOutfitDialog` @0x5c563-0x5c595 and `_OutfitFilter` @0x5808c-0x580b1 set \
             the Sell flag by the count owned and `CANNOT_SELL` alone; the sale checks run \
             only in the sell loop, @0x5c735-0x5ce6c"
        );
        assert!(!button.about().draws());
        // The enable runs these two lists, each check under its own key.
        assert_eq!(keys(&sell_flag()), [None; 3]);
        assert_eq!(
            keys(&sale_checks()),
            [
                Some(RuleKey::SaleMass),
                Some(RuleKey::RaisedMax),
                Some(RuleKey::LauncherSale),
            ]
        );
        for about in [*sale_words().about(), *sale_alert_value().about()] {
            assert_eq!(
                about.reads(),
                Some(RuleKey::SaleRefusal),
                "{}",
                about.name()
            );
            assert!(!about.draws(), "{}", about.name());
        }
        assert_eq!(sale_words().about().name(), "sale words");
        assert_eq!(
            sale_words().about().reproduces(),
            "`_OutfitDialogUpdate` @0x57a12-0x57e68: only #213-#222"
        );
        assert_eq!(sale_alert_value().about().name(), "sale alert");
        assert_eq!(
            sale_alert_value().about().reproduces(),
            "`_DoOutfitDialog` @0x5c735-0x5ce6c, -0x63e @0x5c6ca; `_ShowTextDialog` @0x5a8cf"
        );
    }

    #[test]
    fn by_the_engine_the_info_box_has_no_sale_words() {
        for (refusal, row) in refused_in_the_loop() {
            for outfitter in [of_row(row.clone()), of_row_reading(row, RuleSource::Engine)] {
                assert_eq!(outfitter.sale_words(OutfitId(128)), None, "{refusal:?}");
            }
        }
    }

    #[test]
    fn by_the_other_reading_the_info_box_words_the_first_sale_check() {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row_reading(row, RuleSource::Bible);
            assert_eq!(outfitter.sale_words(OutfitId(128)), Some(refusal));
            assert_eq!(outfitter.sale_words(OutfitId(129)), None, "not listed");
        }
        for (refusal, row) in refused_by_the_sell_flag() {
            let outfitter = of_row_reading(row, RuleSource::Bible);
            assert_eq!(outfitter.sale_words(OutfitId(128)), None, "{refusal:?}");
        }
        let sold = of_row_reading(counted(1, 3, ROOM, ROOM), RuleSource::Bible);
        assert_eq!(sold.sale_words(OutfitId(128)), None, "sold");
    }

    fn pass(index: u32, count: u32) -> Pass {
        Pass {
            index,
            count,
            alerted: false,
        }
    }

    /// A pass after the order's first refused one.
    fn alerted_pass(index: u32, count: u32) -> Pass {
        Pass {
            index,
            count,
            alerted: true,
        }
    }

    #[test]
    fn by_the_engine_a_sale_refused_in_the_loop_alerts_naming_the_sold_outfit_singular_on_a_plain_click()
     {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row(row);
            let sale = order_128(Direction::Sell);
            let names = outfitter.lc_names(sale);
            assert_eq!(
                outfitter.sale_alert(sale, pass(0, 1)),
                Some(SaleAlert {
                    outfit: OutfitId(128),
                    refusal,
                    names,
                    plural: false,
                }),
                "{refusal:?}"
            );
        }
    }

    #[test]
    fn the_alert_names_the_ammunition_and_the_target_too() {
        let [_, (_, raiser), (_, launcher)] = refused_in_the_loop();
        let alert = of_row(raiser)
            .sale_alert(order_128(Direction::Sell), pass(0, 1))
            .expect("alerts");
        assert_eq!(
            alert.names,
            BTreeMap::from([
                (OutfitId(128), lc("rack", "racks")),
                (OutfitId(200), lc("widget", "widgets")),
            ])
        );
        let alert = of_row(launcher)
            .sale_alert(order_128(Direction::Sell), pass(0, 1))
            .expect("alerts");
        assert_eq!(
            alert.names,
            BTreeMap::from([
                (OutfitId(128), lc("rack", "racks")),
                (OutfitId(158), lc("round", "rounds")),
            ])
        );
    }

    #[test]
    fn a_counted_sale_refused_at_its_first_pass_alerts_in_the_plural() {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row(row);
            let plural = |count| {
                outfitter
                    .sale_alert(order_128(Direction::Sell), pass(0, count))
                    .map(|alert| alert.plural)
            };
            assert_eq!(plural(3), Some(true), "{refusal:?}");
            assert_eq!(plural(2), Some(true), "{refusal:?}");
            assert_eq!(plural(1), Some(false), "{refusal:?}");
        }
    }

    #[test]
    fn passes_remain_after_a_pass_but_the_last() {
        assert!(!passes_remain(pass(0, 1)));
        assert!(passes_remain(pass(0, 2)));
        assert!(passes_remain(pass(1, 3)));
        assert!(!passes_remain(pass(2, 3)));
        assert!(!passes_remain(pass(3, 3)), "past the count");
    }

    /// By the engine's [`RuleKey::SaleRefusal`], read with
    /// [`RuleKey::CountedSale`] by the other reading, so each pass's
    /// refusal is the single sale's.
    #[test]
    fn by_the_engine_the_first_refused_pass_alerts_at_any_index() {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row_counted(row, RuleSource::Bible);
            let sale = order_128(Direction::Sell);
            let names = outfitter.lc_names(sale);
            let alert = |plural| SaleAlert {
                outfit: OutfitId(128),
                refusal,
                names: names.clone(),
                plural,
            };
            assert_eq!(
                outfitter.sale_alert(sale, pass(1, 3)),
                Some(alert(true)),
                "{refusal:?} at 1"
            );
            assert_eq!(
                outfitter.sale_alert(sale, pass(2, 3)),
                Some(alert(false)),
                "{refusal:?} at 2"
            );
        }
    }

    #[test]
    fn by_the_engine_a_refusal_once_alerted_is_silent() {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row(row);
            for index in 0..3 {
                assert_eq!(
                    outfitter.sale_alert(order_128(Direction::Sell), alerted_pass(index, 3)),
                    None,
                    "{refusal:?} at {index}"
                );
            }
        }
    }

    #[test]
    fn by_the_other_reading_a_later_pass_does_not_alert_either() {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row_reading(row, RuleSource::Bible);
            assert_eq!(
                outfitter.sale_alert(order_128(Direction::Sell), pass(1, 3)),
                None,
                "{refusal:?}"
            );
        }
    }

    #[test]
    fn by_the_other_reading_nothing_alerts() {
        for (refusal, row) in refused_in_the_loop() {
            let outfitter = of_row_reading(row, RuleSource::Bible);
            for count in [1, 3] {
                assert_eq!(
                    outfitter.sale_alert(order_128(Direction::Sell), pass(0, count)),
                    None,
                    "{refusal:?}"
                );
            }
        }
    }

    #[test]
    fn a_buy_a_sell_flag_refusal_or_an_accepted_sale_never_alerts() {
        for (refusal, row) in refused_in_the_loop() {
            let mut outfitter = of_row(row);
            outfitter.free_mass = -1;
            assert!(outfitter.check(order_128(Direction::Buy)).is_err());
            assert_eq!(
                outfitter.sale_alert(order_128(Direction::Buy), pass(0, 1)),
                None,
                "{refusal:?}"
            );
        }
        for (refusal, row) in refused_by_the_sell_flag() {
            let outfitter = of_row(row);
            assert_eq!(
                outfitter.sale_alert(order_128(Direction::Sell), pass(0, 1)),
                None,
                "{refusal:?}"
            );
        }
        let sold = of_row(counted(1, 3, ROOM, ROOM));
        assert_eq!(
            sold.sale_alert(order_128(Direction::Sell), pass(0, 1)),
            None,
            "sold"
        );
        assert_eq!(
            sold.sale_alert(
                OutfitOrder {
                    outfit: OutfitId(129),
                    direction: Direction::Sell,
                },
                pass(0, 1)
            ),
            None,
            "not listed"
        );
    }

    // A counted sale's loop arithmetic (`RuleKey::CountedSale`).

    /// The outfitter at [`port`] owning `owned` of `records`, read by
    /// `rules`.
    fn raising_by(records: &[OutfitRecord], owned: &[(i16, u16)], rules: Rulebook) -> Outfitter {
        let site = port();
        Shop {
            rules,
            ..shop(records, &site)
        }
        .outfitter(&owning(owned), &mut DayRolls::default(), &mut NeverFires)
        .expect("an outfitter")
    }

    /// The outfitter at [`port`] owning `owned` of `records`, the raised
    /// `Max` read by the engine, [`RuleKey::CountedSale`] as `counted`
    /// says and [`RuleKey::SaleRefusal`] as `refusal` says.
    fn counted_raising(
        records: &[OutfitRecord],
        owned: &[(i16, u16)],
        counted: RuleSource,
        refusal: RuleSource,
    ) -> Outfitter {
        raising_by(
            records,
            owned,
            Rulebook::default()
                .with_override(RuleKey::RaisedMax, RuleSource::Engine)
                .with_override(RuleKey::CountedSale, counted)
                .with_override(RuleKey::SaleRefusal, refusal),
        )
    }

    /// Outfit 128, named, `owned` owned, a launcher of `max_ammo` holding
    /// `rounds` of outfit 158 (named).
    fn launcher_row(owned: u16, max_ammo: u32, rounds: u32) -> OutfitRow {
        let mut row = counted(1, owned, ROOM, ROOM);
        row.facts.names = lc("rack", "racks");
        row.facts.launcher = Some(Launcher {
            max_ammo,
            rounds,
            ammo: Some(OutfitId(158)),
        });
        row.facts.ammo_names = Some(lc("round", "rounds"));
        row
    }

    /// The outfitter of `row` alone, read by `rules`.
    fn of_row_by(row: OutfitRow, rules: Rulebook) -> Outfitter {
        Outfitter {
            rules,
            ..of_row(row)
        }
    }

    /// The outfitter of `row` alone, [`RuleKey::CountedSale`] read as
    /// `source` says.
    fn of_row_counted(row: OutfitRow, source: RuleSource) -> Outfitter {
        of_row_by(
            row,
            Rulebook::default().with_override(RuleKey::CountedSale, source),
        )
    }

    fn ammunition_refused(rounds: u32) -> Result<(), OutfitRefusal> {
        Err(OutfitRefusal::AmmunitionFirst {
            rounds,
            ammo: Some(OutfitId(158)),
        })
    }

    /// Selling one of `outfitter`'s outfit 128 at `pass`.
    fn sold_at(outfitter: &Outfitter, pass: Pass) -> Result<(), OutfitRefusal> {
        outfitter.check_at(order_128(Direction::Sell), pass)
    }

    #[test]
    fn by_the_engine_a_counted_raiser_sale_checks_the_whole_count() {
        let records = raisers();
        let sale = sell_of(201);
        let engine = raising(&records, &[(201, 3), (200, 8)], RuleSource::Engine);
        assert_eq!(
            engine.rules.source_for(RuleKey::CountedSale),
            RuleSource::Engine
        );
        assert_eq!(engine.check_at(sale, pass(0, 1)), Ok(()));
        assert_eq!(engine.check_at(sale, pass(0, 2)), raised_first(4, 200));
        assert_eq!(engine.check_at(sale, pass(0, 3)), raised_first(8, 200));
        // Single clicks sell one first.
        assert_eq!(engine.check(sale), Ok(()));
        let after = raising(&records, &[(201, 2), (200, 8)], RuleSource::Engine);
        assert_eq!(after.check(sale), raised_first(4, 200));
    }

    #[test]
    fn by_the_engine_a_counted_launcher_sale_counts_each_sale_twice() {
        let at = |owned, pass| sold_at(&of_row(launcher_row(owned, 10, 20)), pass);
        assert_eq!(at(4, pass(0, 3)), Ok(()));
        assert_eq!(at(3, pass(1, 3)), ammunition_refused(10));
        assert_eq!(at(3, pass(2, 3)), ammunition_refused(20));
        // Single sales sell two.
        assert_eq!(at(3, Pass::SINGLE), Ok(()));
        assert_eq!(at(2, Pass::SINGLE), ammunition_refused(10));
        // Room below none names more rounds than are held.
        let below = of_row(launcher_row(2, 1, 1));
        assert_eq!(sold_at(&below, pass(2, 3)), ammunition_refused(2));
    }

    #[test]
    fn by_the_other_reading_each_pass_is_checked_as_one_sale() {
        let bible = RuleSource::Bible;
        let records = raisers();
        let raiser = counted_raising(&records, &[(201, 3), (200, 8)], bible, bible);
        assert_eq!(raiser.check_at(sell_of(201), pass(0, 2)), Ok(()));
        assert_eq!(raiser.check_at(sell_of(201), pass(0, 3)), Ok(()));
        let at = |owned, pass| sold_at(&of_row_counted(launcher_row(owned, 10, 20), bible), pass);
        assert_eq!(at(3, pass(1, 3)), Ok(()));
        assert_eq!(at(2, pass(1, 3)), ammunition_refused(10));
    }

    #[test]
    fn a_count_of_1_at_pass_0_checks_as_today_by_either_reading() {
        let sale = order_128(Direction::Sell);
        let records = raisers();
        for source in RuleSource::ALL {
            let rows = refused_in_the_loop()
                .map(|(refusal, row)| (Err(refusal), row))
                .into_iter()
                .chain([(Ok(()), counted(1, 3, ROOM, ROOM))]);
            for (expected, row) in rows {
                let outfitter = of_row_counted(row, source);
                assert_eq!(
                    outfitter.check_at(sale, Pass::SINGLE),
                    expected,
                    "{source:?}"
                );
                assert_eq!(outfitter.check(sale), expected, "{source:?}");
            }
            let raiser = |owned| {
                counted_raising(&records, owned, source, RuleSource::Bible)
                    .check_at(sell_of(201), Pass::SINGLE)
            };
            assert_eq!(raiser(&[(201, 3), (200, 8)]), Ok(()), "{source:?}");
            assert_eq!(
                raiser(&[(201, 2), (200, 8)]),
                raised_first(4, 200),
                "{source:?}"
            );
            let launcher = |owned| {
                sold_at(
                    &of_row_counted(launcher_row(owned, 10, 20), source),
                    Pass::SINGLE,
                )
            };
            assert_eq!(launcher(3), Ok(()), "{source:?}");
            assert_eq!(launcher(2), ammunition_refused(10), "{source:?}");
        }
    }

    #[test]
    fn under_a_bible_raised_max_or_launcher_sale_the_counted_reading_changes_nothing() {
        let records = raisers();
        for counted_source in RuleSource::ALL {
            let raiser = raising_by(
                &records,
                &[(201, 3), (200, 8)],
                Rulebook::default()
                    .with_override(RuleKey::RaisedMax, RuleSource::Bible)
                    .with_override(RuleKey::CountedSale, counted_source),
            );
            assert_eq!(
                raiser.check_at(sell_of(201), pass(0, 3)),
                Ok(()),
                "{counted_source:?}: 8 after"
            );
            let launcher = of_row_by(
                launcher_row(3, 10, 20),
                Rulebook::default()
                    .with_override(RuleKey::LauncherSale, RuleSource::Bible)
                    .with_override(RuleKey::CountedSale, counted_source),
            );
            assert_eq!(
                sold_at(&launcher, pass(2, 3)),
                ammunition_refused(20),
                "{counted_source:?}: every round held"
            );
        }
    }

    #[test]
    fn by_the_engine_every_pass_after_a_refused_one_is_refused_too() {
        let records = raisers();
        let raiser = raising(&records, &[(201, 3), (200, 8)], RuleSource::Engine);
        for index in 0..3 {
            assert_eq!(
                raiser.check_at(sell_of(201), pass(index, 3)),
                raised_first(8, 200),
                "at {index}"
            );
        }
        let launcher = of_row(launcher_row(3, 10, 20));
        let refused: Vec<_> = (1..4)
            .map(|index| sold_at(&launcher, pass(index, 4)))
            .collect();
        assert_eq!(
            refused,
            [
                ammunition_refused(10),
                ammunition_refused(20),
                ammunition_refused(30)
            ]
        );
    }

    #[test]
    fn a_counted_alert_carries_the_counted_excess_and_names_its_target() {
        let records = raisers();
        let raiser = counted_raising(
            &records,
            &[(201, 3), (200, 8)],
            RuleSource::Engine,
            RuleSource::Engine,
        );
        assert_eq!(
            raiser.sale_alert(sell_of(201), pass(0, 2)),
            Some(SaleAlert {
                outfit: OutfitId(201),
                refusal: OutfitRefusal::RaisedFirst {
                    count: 4,
                    target: OutfitId(200),
                },
                names: BTreeMap::from([
                    (OutfitId(200), lc("widget", "widgets")),
                    (OutfitId(201), lc("widget rack", "widget racks")),
                ]),
                plural: true,
            })
        );
        assert_eq!(
            raiser.lc_names(sell_of(201)),
            BTreeMap::new(),
            "a single click goes through"
        );
        let launcher = of_row(launcher_row(3, 10, 20));
        assert_eq!(
            launcher.sale_alert(order_128(Direction::Sell), pass(1, 3)),
            Some(SaleAlert {
                outfit: OutfitId(128),
                refusal: OutfitRefusal::AmmunitionFirst {
                    rounds: 10,
                    ammo: Some(OutfitId(158)),
                },
                names: BTreeMap::from([
                    (OutfitId(128), lc("rack", "racks")),
                    (OutfitId(158), lc("round", "rounds")),
                ]),
                plural: true,
            })
        );
    }

    #[test]
    fn a_launchers_excess_from_those_left_has_no_floor() {
        let excess = |max_ammo, rounds, left, source| {
            Launcher {
                max_ammo,
                rounds,
                ammo: None,
            }
            .excess_left(left, source)
        };
        let engine = RuleSource::Engine;
        assert_eq!(excess(10, 20, 1, engine), Some(10));
        assert_eq!(excess(10, 20, 0, engine), Some(20));
        assert_eq!(excess(1, 1, -1, engine), Some(2), "room below none");
        assert_eq!(excess(10, 20, 2, engine), None);
        assert_eq!(excess(0, 5, -1, engine), None, "no MaxAmmo");
        assert_eq!(excess(10, 0, -1, engine), None, "no rounds");
        assert_eq!(excess(10, 20, 5, RuleSource::Bible), Some(20));
        assert_eq!(
            excess(u32::MAX, u32::MAX, -1, engine),
            Some(u32::MAX),
            "saturated"
        );
    }

    #[test]
    fn the_counted_sale_rules_read_the_counted_sale_key() {
        for (about, name, reproduces) in [
            (
                *raisers_left().about(),
                "raisers left",
                "`_DoOutfitDialog` @0x5c82d-0x5c85e: owned now - the whole count (-0x68c)",
            ),
            (
                *launchers_left().about(),
                "launchers left",
                "`_DoOutfitDialog` @0x5cb9c-0x5cbc7: owned now - the pass index (-0x63a) - 1",
            ),
        ] {
            assert_eq!(about.name(), name);
            assert_eq!(about.reads(), Some(RuleKey::CountedSale), "{name}");
            assert_eq!(about.reproduces(), reproduces, "{name}");
            assert!(!about.draws(), "{name}");
        }
    }
}
