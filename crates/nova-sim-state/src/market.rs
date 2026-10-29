//! The commodity exchange: what a stellar trades, at what price, how much
//! cargo a ship carries, buying and selling, and the planetary events
//! (`öops`) that move prices.
//!
//! # Commodities and prices
//!
//! The six standard commodities are named by `STR#` 4000 and priced by
//! `STR#` 4004 ([`CommodityStrings`]). A plug-in's `'STR '` 9000 + n
//! replaces commodity n's 4000 name whenever it exists, whatever it
//! holds; an empty (zero-byte) or unreadable one names it empty
//! (`_LoadStrings` @0x71afb-0x71b55). A plug-in's `'STR '` 9300 + n
//! replaces commodity n's 4004 string whenever it exists, whatever it
//! holds (`_InitObjects` @0x1cedd-0x1cf43). An empty (zero-byte) one
//! from 9301 up takes the string commodity n - 1 was priced from. Every
//! price string is read as the Toolbox's `StringToNum` reads it, kept to
//! 16 bits ([`string_to_num`]), so a string that is no number still
//! prices its commodity. A missing 4004 string, a patch that cannot be
//! read, and an empty 9300 price it at 0; a missing name is empty
//! (`_LoadStrings` @0x71afb-0x71b55). So all six standard commodities are
//! always traded wherever a stellar sets them a level. A stellar with
//! the trade-center flag trades commodity n when its `spöb`
//! flags give it a price level, in the nibble at bit 28 - 4n (food at 28
//! down to equipment at 8): 1 low, 2 medium, 4 high, 0 not traded. A
//! nibble with more than one bit set, which stock data never has, takes
//! the lowest level it names (`_CalcPortDemand` @0x56b1-0x5794).
//!
//! Low is the base price divided by the stellar's [`Markup`] and high
//! the base price multiplied by it, in doubles, truncated toward zero;
//! medium is the base price (`_DoTradeDialog` @0x5dc0c-0x5dc48,
//! @0x5dc87-0x5dcc5). The markup is 1.25 ([`Markup::Standard`]) unless
//! the stellar has a government and the player's legal record in its
//! system is below 0, when it is 1.1 ([`Markup::Outlaw`]: 110 is 99 low,
//! 121 high), or the stellar is dominated, when it is 1.5
//! ([`Markup::Dominated`]) whatever the record. Until the port models
//! domination, a dominated stellar is one with the `Flags2` bit
//! [`DOMINATED`]. By 1.25 low is 80 % and high 125 %, and the community's
//! table of stock prices (Food 60/75/93, Medical 600/750/937, and so on)
//! matches these exactly. The price is then held in 16 bits, as the
//! engine stores it (@0x5dcc5), so a high price above 32767 wraps
//! negative ([`band_price`]). A standard commodity's price of 4 or
//! less, after the wrap, a base price of 0 or below included, is
//! [`MIN_COMMODITY_PRICE`], 5, as in the engine (`_DoTradeDialog`
//! @0x5dcc8-0x5dcce). A `jünk` price wraps the same way
//! (@0x5dddb, @0x5de2f) and has no floor, as
//! [`RuleKey::JunkPrice`](crate::RuleKey::JunkPrice) says by the engine
//! (below); by the other reading it is never below 0.
//!
//! A stellar sells and buys each good at one price: profit comes from
//! carrying goods from where they are cheap to where they are dear.
//!
//! # Special goods
//!
//! A `jünk` is listed at its `SoldAt` stellars at the low price of its
//! `BasePrice`, and at its `BoughtAt` stellars at the high price, by the
//! stellar's [`Markup`] as a commodity is (`_DoTradeDialog`
//! @0x5dd83-0x5de2f; the community's Opals: 960 and 1500 from 1200 by
//! 1.25). It is traded only
//! through a trade center, so a listed stellar without one offers nothing.
//! A stellar in both lists, which stock data never has, lists it twice,
//! as the original's rows 6 and 7 are filled independently: first the
//! `BoughtAt` row at the high price, then the `SoldAt` row at the low
//! price, both under its name and with the tons held (@0x5dd83-0x5de2f,
//! @0x4d301-0x4d36c). Its `Flags` make it multiply or decay in the hold
//! (below).
//!
//! Its two control-bit tests, asked through the
//! [`ControlBits`](crate::ControlBits) port for the player's pilot (a
//! [`Gate`]), go by the stellar's side, as the field names `SoldAt` and
//! `BoughtAt` do: `SellOn` gates the stellar selling it, its `SoldAt`
//! row, and `BuyOn` the stellar buying it, its `BoughtAt` row. That is the
//! executable's pairing: `_DoTradeDialog` tests `BuyOn` for a `BoughtAt`
//! stellar (@0x5dda8, at the high price) and `SellOn` for a `SoldAt` one
//! (@0x5de58, at the low price), the tests `_LoadObjectData` copies from
//! the record's `BuyOn` and `SellOn` (@0x7d5e5, @0x7d603). The Bible's
//! "available to be bought" and "able to be sold" read the same way from
//! the stellar's side. A row whose test does not hold is not listed, so a
//! `jünk` listed both ways keeps only its other row; a test that did not
//! parse never holds.
//!
//! Which ways a `jünk` row trades follows
//! [`RuleKey::JunkTrade`](crate::RuleKey::JunkTrade). By the engine (the
//! default), every listed row trades both ways at its own price, as a
//! commodity's does: the trade buttons ask only `_CanBuyGoods` (@0xccec:
//! cash for a ton at the row's price, and free space) and `_CanSellGoods`
//! (@0x4a94: tons held), never which list the row came from
//! (`_DrawTradeButtons` @0x2938f/0x2939d, `_TrackTradeButtons`
//! @0x2960d/0x2963e). A trade is on the selected row (`_selTradeItem`)
//! at its price, a buy charging it (@0x5e2c4-0x5e2e2) and a sale paying it
//! (@0x5e528-0x5e546), and both rows of a `jünk` move its one held count
//! (@0x5e29e-0x5e2c0, @0x5e504-0x5e524). So an [`Order`] names its
//! [`row`](Order::row), and a `jünk` in both lists is bought and sold
//! high on one row and low on the other. By the Bible, whose `SoldAt` is
//! where "the commodity is sold" and `BoughtAt` where it "is purchased",
//! the `SoldAt` row is bought only and the `BoughtAt` row sold only
//! ([`Market::row_allows`]), so the player buys it low and sells it
//! high.
//!
//! A `jünk` price of 0 or below follows
//! [`RuleKey::JunkPrice`](crate::RuleKey::JunkPrice). By the engine (the
//! default), the price is signed: a negative one is listed at that price
//! (`_TradeDialogUpdate` @0x4d545-0x4d54b); buying it with cash of 0 or
//! more gets no tons, as cash / price is 0 or less and the `jle`
//! @0x5e27d skips the buy (with cash below 0, see below); and
//! selling it takes tons x price from the cash, which can go below 0
//! (@0x5e543-0x5e546). A row priced 0 is not listed (`_TradeDialogUpdate`
//! @0x4d2a2-0x4d2ac, `_TradeFilter` @0x4e262), each way on its own, so a
//! `jünk` listed both ways at base 1 (high 1, low 0) keeps only its high
//! row, and one at base 0 is not listed at all. By the other reading, the port's
//! own, the price is never below 0, and a row priced 0 is listed and
//! bought free, limited by space alone.
//!
//! # Tribbles and perishables
//!
//! In flight, a held `jünk` with the Tribbles flag ([`TRIBBLES`]) gains a
//! ton, and one with the Perishable flag ([`PERISHABLE`]) loses one, on
//! each due frame, as the `nova_sim::Session` ticks; a good that
//! decays to none is gone.
//! There is no random draw. A frame is due when the original's frame
//! counter is a multiple of [`JUNK_STEP_FRAMES`] ([`junk_step_due`]); the
//! counter counts up to [`LAST_FRAME`] and goes back to 0
//! ([`next_frame`]), so four due frames fall 250 frames apart and the
//! next 25 frames later: 5 steps every 1025 ticks, about 34 s. Each tick
//! in flight moves it on, a jump's too, but not a landed one; a take-off
//! or a gate's exit sets it to [`AFTER_TAKE_OFF_FRAME`] (the dock
//! request's tail, which both run; see the session's `takeoff_grace`
//! module), so the first step comes on the 15th tick of flight, and a
//! destroyed player's ticks do not step. In double time a due frame steps
//! twice, once on each pass, as the original's `_HandlePlayer` runs the
//! step on both (@0x70827; see the session's `double_time` module).
//!
//! The conditions follow
//! [`RuleKey::JunkFlags`](crate::RuleKey::JunkFlags): by the engine, on
//! the free space measured once, so tribbles goods can overfill the hold,
//! perishable goods decay only while there is space, and a good with both
//! flags decays only beside a perishable-only good; by the Bible,
//! tribbles goods grow only into free space and perishable goods always
//! decay. The free space is the fleet's cargo space ([`fleet_holds`],
//! below) less every good held and the mission cargo aboard, as
//! `_TotalCargo` (@0x4a0b) counts them (@0x70863-0x70888), so mission
//! cargo filling the hold stops the growth and, by the engine, the decay.
//!
//! # Events
//!
//! Each day ([`step_day`]), every active event's days left go down by
//! one, and those that reach none end; then every `öops` that had no
//! days left that morning (an event that ends today is rolled first
//! tomorrow, as `_DisasterHandler` @0x41add-0x41ae7 has it), has a
//! stellar to start at, a `Freq` above 0, a `Duration` above 0 and an
//! `ActivateOn` that holds, starts with
//! `Duration` days left if a `Freq` % [`Chance`] fires. So an event is
//! active for `Duration` days. `ActivateOn` is asked through the
//! [`ControlBits`](crate::ControlBits) port for the player's pilot (a
//! [`Gate`]); one that did not parse never holds. The original rolls
//! `Freq` before it tests `ActivateOn` (`_DisasterHandler` @0x41b0f,
//! @0x41b2c); either order starts an event with the same chance, and
//! here an event that cannot start asks no draw, whatever stops it. An
//! event already active runs out its days when its `ActivateOn` stops
//! holding. Its commodity is not asked (`_DisasterHandler` never tests
//! it, 0x41ab9-0x41bc4): the loader reads one above 5 as 5, equipment,
//! and a negative one as -1 (`_LoadObjectData` @0x7cd56-0x7cd6f), and
//! [`event_commodity`] reads them so. A negative-commodity event is active
//! but prices nothing and is not shown; the original writes
//! `_commodityPrice[-1]`, out of bounds (`_DoTradeDialog` @0x5dd19).
//! The handler's other branch, which puts days at 2 for a slot with a
//! negative `Duration` and its +0x20d flag set (@0x41ae9-0x41afd), is
//! dead: nothing ever sets that flag, and it is not ported.
//!
//! An active event moves its commodity's price at its stellar as
//! [`RuleKey::EventPrice`](crate::RuleKey::EventPrice) says. By the
//! engine (the default; `_DoTradeDialog` @0x5dcf8-0x5dd58), the price is
//! the commodity's `BasePrice` plus the event's `PriceDelta`, whatever
//! the stellar's level; of several events on one commodity the highest
//! ID wins; and an event lists its commodity, to buy and to sell, at a
//! stellar that does not otherwise trade it. By the Bible, the event's `PriceDelta` is added
//! to the stellar's own (level) price, several add up, and an event on a
//! commodity not traded there moves nothing. Either way the price is
//! held in 16 bits (@0x5dd19-0x5dd40 adds the delta with `addw`; the Bible
//! is silent on a price's width), and then a price of 4 or less is 5.
//! Wrapping the sum once is wrapping each addition, so the order of
//! several events does not matter. What the exchange says of the events
//! active at its stellar is told by the story layer
//! (`nova_sim::exchange_events`), which [`Market::events`] holds; an
//! event of a negative commodity moves no price (the engine's reading of
//! the original's `_commodityPrice[-1]` write, @0x5dd19-0x5dd40).
//!
//! An event starts at its stellar: the `spöb` its `Stellar` names, from
//! 128 up, or, for a `Stellar` of -1 ("any stellar"), an even pick by
//! [`Chance`], once the `Freq` roll fires, among [`event_stellars`] by
//! ID: the stellars 128 to 2175 that are not uninhabited, whether or not
//! they trade. It stays there for its run, and is saved with it. A
//! `Stellar` of -2 (news only) or 0 to 127 is never rolled, nor started
//! (`_DisasterHandler` @0x41b57-0x41b59). With no
//! candidate a -1 event never starts either; here we leave the original,
//! whose draw never ends and hangs the game.
//!
//! # Cargo
//!
//! A ship carries `|Holds|` tons ([`hold_tons`]), plus each outfit's
//! `ModVal` of [`MORE_CARGO`] for each one carried ([`cargo_capacity`]).
//! The engine reads a negative `Holds` as its size everywhere:
//! `_LoadObjectData` negates it in place as it loads the class
//! (@0x7a6a5-0x7a6d3), and only the outfitter reads that it was negative
//! (`nova_sim::outfitter::OutfitRefusal::NoExpansion`).
//!
//! The exchange measures the fleet's cargo space ([`fleet_holds`],
//! `_TotalFleetHolds` @0xc24d-0xc356): the ship's own, plus the `Holds`
//! (its size) of each of the player's escorts whose ship class's
//! `InherentAI` is 2 or less (a trader, or any value below 1), at most [`MAX_FLEET_HOLDS`]
//! tons. A warship or interceptor escort adds nothing, nor does a fighter
//! launched from the player's bays, nor any escort's outfits or cargo:
//! every good the fleet holds is held on the player's ship. The free
//! space is that less everything held and the mission cargo aboard
//! (`_TotalCargo` @0x4a0b), which takes space but is never a row's held
//! tons, so it cannot be sold.
//!
//! A plain click on Buy or Sell ([`Lot::Click`]) moves up to
//! [`CLICK_TONS`], as [`RuleKey::TradeLot`](crate::RuleKey::TradeLot)
//! says. By the engine (the default), a plain buy moves
//! min(trunc(cash / price), 10, free) tons (`_DoTradeDialog`
//! @0x5e268-0x5e278) and a plain sale min(held, 10) (@0x5e48d-0x5e4fa);
//! by the other reading, the port's earlier one, a ton. The most
//! ([`Lot::Max`], [`Market::row_max`]) is what the original's Option
//! quantity dialog offers by default (@0x5e23a-0x5e25a, @0x5e44f-0x5e47f):
//! for a buy min(cash / price, free), the quotient in single floats by
//! the engine (@0x5e21d-0x5e234) or exact by the other reading
//! ([`RuleKey::TradeQuotient`](crate::RuleKey::TradeQuotient)), and for a
//! sale everything held; either way at most 32000 tons. A counted lot
//! ([`Lot::Count`]) moves exactly its count, from 1 up to that most, as
//! the dialog trades what it is given with no check against the cash, so
//! above 2^24 cash the engine's most can leave the cash below 0. A trade
//! that would move nothing is refused. The Buy button is enabled
//! as `_CanBuyGoods` has it, which a row priced below nothing can pass
//! with nothing then bought ([`Market::row_allows`]), and a buy it
//! greys, with the cash below the price (@0xcd07-0xcd0d), is refused.
//!
//! The cash a buy divides follows
//! [`RuleKey::TradeDebt`](crate::RuleKey::TradeDebt). By the engine (the
//! default), it is signed, with no clamp before the quotient or after it
//! (@0x5e21d-0x5e234). With cash below 0 Buy is enabled only on a row
//! priced at or below the cash, so only a `jünk` priced below 0 can be
//! bought, and cash / price is then in (0, 1]: exactly at the price it
//! is 1, and a click or a count of 1 buys a ton, which takes price x 1
//! from the cash (@0x5e2df-0x5e2e2) and leaves it at 0. By the other
//! reading, the port's earlier one, cash below 0 counts as none, and
//! such a buy is refused.

use std::collections::BTreeMap;

use crate::catalog::{
    CommodityStrings, DisasterId, DisasterRecord, JunkId, JunkRecord, PilotCatalog, StellarId,
    StringPatch,
};
use crate::chance::{Chance, NeverFires};
use crate::control::Gate;
use crate::fuel::OutfitMod;
use crate::landing::StellarFlags;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::rules::{Check, Checks, Context, Value};
use crate::trade_dialog::{
    CargoSummary, HeldIn, MissionLoad, PriceMarker, TradeWords, cargo_summary, commodity_marker,
    first_local_delta, held_in, junk_marker,
};

/// How many standard commodities there are.
pub const COMMODITIES: u8 = 6;

/// The `spöb` `Flags2` bit of a stellar that is always dominated by the
/// player: `_ResetPlayerRecord` sets each stellar's dominated byte (+0x46)
/// from it (@0x1dbdb-0x1dbdf), and the exchange prices such a stellar at
/// [`Markup::Dominated`].
pub const DOMINATED: u16 = 0x0020;

/// What a stellar's low and high prices are marked down and up by
/// (`_DoTradeDialog` @0x5dc0c-0x5dc48): low is the base price divided by
/// its [`factor`](Markup::factor), high the base price multiplied by it,
/// in doubles, truncated toward zero; medium is the base price.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Markup {
    /// 1.25 (the double at 0xdd5c8): low is 80 % of the base price and
    /// high 125 %.
    #[default]
    Standard,
    /// 1.1 (0xdd1b0): at a stellar with a government, while the player's
    /// legal record in its system is below 0.
    Outlaw,
    /// 1.5 (0xdd640): at a stellar the player dominates, whatever the
    /// record.
    Dominated,
}

impl Markup {
    /// The markup at a stellar that has a government (`governed`) or not,
    /// where the player's legal record in its system is `record`, and
    /// that is `dominated` or not (`_DoTradeDialog` @0x5dc0c-0x5dc48).
    ///
    /// The original reads the record from `_playerRecord`, one per
    /// system; the port's is the record with the system's government, 0
    /// in an independent system, as [`legal`](crate::legal) maps it.
    #[must_use]
    pub const fn of(governed: bool, record: i16, dominated: bool) -> Self {
        if dominated {
            Self::Dominated
        } else if governed && record < 0 {
            Self::Outlaw
        } else {
            Self::Standard
        }
    }

    /// The factor prices are divided and multiplied by.
    #[must_use]
    pub const fn factor(self) -> f64 {
        match self {
            Self::Standard => 1.25,
            Self::Outlaw => 1.1,
            Self::Dominated => 1.5,
        }
    }
}

/// The lowest price of a standard commodity: a price of 4 or less, a
/// level's or an event's, once held in 16 bits, is raised to this
/// (`_DoTradeDialog` @0x5dcc8-0x5dcce, @0x5dd39-0x5dd40). A `jünk` price
/// has no such floor.
pub const MIN_COMMODITY_PRICE: i64 = 5;

/// The `oütf` `ModType` that adds cargo space: the Bible's "more cargo
/// space", `ModVal` tons each.
pub const MORE_CARGO: i16 = 2;

/// A tradeable good: a standard commodity (0 food to 5 equipment) or a
/// special one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Good {
    /// Standard commodity n, from 0.
    Commodity(u8),
    /// A `jünk`.
    Junk(JunkId),
}

/// A stellar's price level for a good.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriceLevel {
    /// The base price divided by the stellar's [`Markup`].
    Low,
    /// The base price.
    Medium,
    /// The base price multiplied by the stellar's [`Markup`].
    High,
}

/// Which way a trade goes, from the player's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The player buys from the stellar.
    Buy,
    /// The player sells to the stellar.
    Sell,
}

/// How much one trade moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lot {
    /// A plain click or key on Buy or Sell: up to [`CLICK_TONS`] tons, or
    /// a ton by the other reading of
    /// [`RuleKey::TradeLot`](crate::RuleKey::TradeLot). A buy's cash /
    /// price is exact integer division under both readings of
    /// [`RuleKey::TradeQuotient`](crate::RuleKey::TradeQuotient): the
    /// single-float quotient differs from it only above 2^24 cash, where
    /// it is at least 512 and a click moves at most 10 either way.
    Click,
    /// The most the exchange offers on the row ([`Market::row_max`]): the
    /// default the original's Option quantity dialog opens with, and what
    /// Alt trades under the other reading of
    /// [`RuleKey::TradeCount`](crate::RuleKey::TradeCount).
    Max,
    /// Exactly this many tons, as the original's quantity dialog trades
    /// what it is given: from 1 up to the most the exchange offers
    /// ([`Market::row_max`]). None, or more than the most, is refused
    /// ([`TradeRefusal::OutOfRange`]).
    Count(u32),
}

/// The most tons a [`Lot::Click`] moves by the engine: a plain buy moves
/// min(trunc(cash / price), 10, free) (`_DoTradeDialog`
/// @0x5e268-0x5e278) and a plain sale min(held, 10) (@0x5e48d-0x5e4fa).
pub const CLICK_TONS: u32 = 10;

/// One trade the player asks for, on the row of the exchange the player
/// selected, as the original trades on `_selTradeItem` (0x3b54dc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    /// The row it trades on: its index in [`Market::rows`]. A `jünk`
    /// listed both ways has two rows at two prices, and the order is
    /// priced and limited by this one.
    pub row: usize,
    /// The good, which must be the row's.
    pub good: Good,
    /// Buying or selling.
    pub direction: Direction,
    /// How much.
    pub lot: Lot,
}

/// Why a trade moves nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeRefusal {
    /// There is no exchange: the ship is not landed, or the stellar has
    /// no trade center.
    NoMarket,
    /// The stellar does not trade the good that way.
    NotTraded,
    /// No cargo space is free.
    NoSpace,
    /// The player cannot pay for a ton.
    CannotAfford,
    /// The player holds none of the good.
    NoneHeld,
    /// A counted lot ([`Lot::Count`]) of none, which the original's
    /// callers skip (`jle` @0x5e27d, @0x5e4d8), or of more than the most
    /// the exchange offers ([`Market::row_max`]), which its quantity
    /// dialog never gives.
    OutOfRange,
}

/// A standard commodity that can be traded: its name and base price.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commodity {
    /// Its name, from `STR#` 4000 or a plug-in's `'STR '` 9000 + n.
    pub name: String,
    /// Its base price, from `STR#` 4004.
    pub base_price: i64,
}

/// The price level that `spöb` `flags` set for `commodity`, or `None`
/// when the stellar does not trade it (or there is no such commodity).
/// A nibble with several bits takes the lowest level it names, as
/// `_CalcPortDemand` tests low, then medium, then high (@0x56b1-0x5794).
#[must_use]
pub fn price_level(flags: u32, commodity: u8) -> Option<PriceLevel> {
    if commodity >= COMMODITIES {
        return None;
    }
    let nibble = (flags >> (28 - 4 * u32::from(commodity))) & 0xF;
    if nibble & 1 != 0 {
        Some(PriceLevel::Low)
    } else if nibble & 2 != 0 {
        Some(PriceLevel::Medium)
    } else if nibble & 4 != 0 {
        Some(PriceLevel::High)
    } else {
        None
    }
}

/// A good's price at `level`, from its `base` price, by `markup`: low is
/// `base / factor` and high `base × factor`, in doubles, truncated toward
/// zero as `cvttsd2si` truncates (`_DoTradeDialog` @0x5dc87-0x5dcc5),
/// then held in 16 bits ([`word`]) as the engine stores it, a commodity's
/// @0x5dcc5 and a `jünk`'s @0x5dddb (high) and @0x5de2f (low). Only a high
/// price can leave 16 bits: from a base of 26215 by 1.25, 29790 by 1.1
/// and 21846 by 1.5 it wraps negative, and from a base of -26216 by 1.25
/// positive.
#[must_use]
pub fn band_price(base: i64, level: PriceLevel, markup: Markup) -> i64 {
    let base_f = base as f64;
    word(match level {
        PriceLevel::Low => (base_f / markup.factor()) as i64,
        PriceLevel::Medium => base,
        PriceLevel::High => (base_f * markup.factor()) as i64,
    })
}

/// `price` held in 16 bits, as the engine's `_commodityPrice` (0x3b5078)
/// holds it: its low 16 bits, signed. `cvttsd2si` gives 32 bits and a
/// `movw` store keeps the low half; every price here is well within 32
/// bits, so keeping the low 16 of the `i64` is the same.
const fn word(price: i64) -> i64 {
    price as i16 as i64
}

/// The cargo space, in tons, of a ship with `holds` and these `outfits`;
/// never below none. [`ShipStats`](crate::stats::ShipStats) gives a ship's.
/// This is the ship's own space; the fleet's is [`fleet_holds`].
#[must_use]
pub fn cargo_capacity(holds: i16, outfits: &[OutfitMod]) -> u32 {
    let pods: i64 = outfits
        .iter()
        .filter(|outfit| outfit.mod_type == MORE_CARGO)
        .map(|outfit| i64::from(outfit.mod_val) * i64::from(outfit.count))
        .sum();
    let tons = i64::from(hold_tons(holds)) + pods;
    u32::try_from(tons.max(0)).unwrap_or(u32::MAX)
}

/// The tons a ship class's `Holds` gives: the size of a negative one, as
/// `_LoadObjectData` negates it in place (@0x7a6a5-0x7a6d3). The engine
/// leaves -32768, which negates to itself, negative; this reads it as
/// 32768.
#[must_use]
pub fn hold_tons(holds: i16) -> u16 {
    holds.unsigned_abs()
}

/// The most cargo space a fleet has (`_TotalFleetHolds` @0xc33e-0xc349).
/// A trade's lot has its own cap of the same 32000 tons ([`Lot::Max`],
/// [`Market::row_max`]), which shows only on a sale: tribbles can grow
/// past a full hold, but the free space never passes this.
pub const MAX_FLEET_HOLDS: u32 = 32_000;

/// The most tons one trade offers, a buy (@0x5e23a-0x5e24c) or a sale
/// (@0x5e44f-0x5e457).
const LOT_CAP: i64 = 32_000;

/// What the fleet's cargo space needs of one of the player's escorts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscortHolds {
    /// Its ship class's `Holds`, as the data gives it; a negative one
    /// counts as its size ([`escort_tons`]).
    pub holds: i16,
    /// Its ship class's `InherentAI`, raw.
    pub inherent_ai: i16,
    /// Whether it is a fighter launched from one of the player's bays
    /// (the original's AI type 5).
    pub carried: bool,
}

impl EscortHolds {
    /// Whether its `Holds` add to the fleet's cargo space
    /// (`_TotalFleetHolds`): it is not a launched fighter and its
    /// `InherentAI` is [`FREIGHTER_AI`](crate::catalog::FREIGHTER_AI) or
    /// less. A jettison drops boxes behind each such escort too.
    #[must_use]
    pub fn adds_holds(self) -> bool {
        !self.carried && self.inherent_ai <= crate::catalog::FREIGHTER_AI
    }
}

/// The summed `Holds` of the escorts [`fleet_holds`] counts
/// ([`EscortHolds::adds_holds`]), each a negative one's size
/// (@0x7a6a5-0x7a6d3), with no cap. A ship purchase divides the cargo by
/// it uncapped (`nova_sim::shipyard`).
#[must_use]
pub fn escort_tons(escorts: impl IntoIterator<Item = EscortHolds>) -> i64 {
    escorts
        .into_iter()
        .filter(|escort| escort.adds_holds())
        .map(|escort| i64::from(hold_tons(escort.holds)))
        .sum()
}

/// The fleet's cargo space, in tons (`_TotalFleetHolds` @0xc24d-0xc356):
/// the ship's own `ship` tons plus the `Holds` (each a negative one's
/// size) of each escort that is not a launched fighter and whose
/// `InherentAI` is [`FREIGHTER_AI`](crate::catalog::FREIGHTER_AI) or less
/// ([`escort_tons`]), at most [`MAX_FLEET_HOLDS`].
#[must_use]
pub fn fleet_holds(ship: u32, escorts: impl IntoIterator<Item = EscortHolds>) -> u32 {
    let tons = (i64::from(ship) + escort_tons(escorts)).min(i64::from(MAX_FLEET_HOLDS));
    u32::try_from(tons).unwrap_or(MAX_FLEET_HOLDS)
}

/// The share of the fleet's cargo a lost escort takes
/// (`_DestroyPartialFleetCargo` @0xcd32-0xcff6 for a ship other than the
/// player's): its `Holds` (`lost`, a negative one's size, [`hold_tons`];
/// its outfits never count) over the fleet's space, the player's ship's
/// own `own` tons plus the `escorts` tons ([`escort_tons`]), and its own
/// `Holds` too when `counted` (the other reading of
/// [`RuleKey::LostCargoShare`]), at most 1, in doubles with no 32000 cap
/// (@0xcf27-0xcf37).
///
/// With no holds the ship takes nothing: the original's 0/0 is NaN, which
/// `minsd` passes and `cvttsd2si` turns into `INT_MIN`, so every good loses
/// nothing. Holds against no other space take everything.
#[must_use]
pub fn lost_share(lost: i16, own: u32, escorts: i64, counted: bool) -> f64 {
    let lost = f64::from(hold_tons(lost));
    if lost == 0.0 {
        return 0.0;
    }
    let mut fleet = f64::from(own) + escorts as f64;
    if counted {
        fleet += lost;
    }
    if fleet == 0.0 {
        return 1.0;
    }
    (lost / fleet).min(1.0)
}

/// Takes `share` of every good in `cargo`, commodity or `jünk`: each loses
/// the truncated part of what it holds (@0xcf3b-0xcfe4), and a good left
/// at none is gone. `share` is in 0..=1 ([`lost_share`]). Gives the tons
/// each standard commodity lost, by number, none for one not held: what
/// the original then writes into the lost ship's own hold
/// (`movw %dx, 0x7a(ship)` @0xcf76); a `jünk` lost goes nowhere
/// (@0xcf94-0xcfe4).
pub fn destroy_share(cargo: &mut BTreeMap<Good, u32>, share: f64) -> [u32; COMMODITIES as usize] {
    let mut destroyed = [0; COMMODITIES as usize];
    for (good, tons) in cargo.iter_mut() {
        // The share is in 0..=1, so the part is in 0..=held.
        let part = (f64::from(*tons) * share) as u32;
        *tons -= part;
        if let Good::Commodity(commodity) = good
            && let Some(slot) = destroyed.get_mut(usize::from(*commodity))
        {
            *slot = part;
        }
    }
    cargo.retain(|_, tons| *tons > 0);
    destroyed
}

/// Cuts each commodity in `cargo` by `holds` over the `total` the fleet
/// carries (`_TotalCargo`) when that is more
/// (`_ResetPlayerPrecalcedValues` @0xc7d8-0xc82c), in single floats; a
/// `jünk` is not cut. A good cut to none is gone.
pub fn trim_commodities(cargo: &mut BTreeMap<Good, u32>, holds: u32, total: i64) {
    if i64::from(holds) >= total {
        return;
    }
    let ratio = holds as f32 / total as f32;
    for (good, tons) in cargo.iter_mut() {
        if let Good::Commodity(_) = good {
            *tons = (*tons as f32 * ratio) as u32;
        }
    }
    cargo.retain(|_, tons| *tons > 0);
}

/// The standard commodities, every one of them, by number, from
/// `strings`: a missing name is empty, and each base price is its string
/// read by [`string_to_num`] (`_InitObjects` @0x1cedd-0x1cf43,
/// `_LoadStrings` @0x71afb-0x71b55).
#[must_use]
pub fn commodities(strings: &CommodityStrings) -> Vec<(u8, Commodity)> {
    (0..COMMODITIES)
        .map(|n| {
            let at = usize::from(n);
            (
                n,
                Commodity {
                    name: name_string(strings, at).to_owned(),
                    base_price: i64::from(string_to_num(base_price_string(strings, at))),
                },
            )
        })
        .collect()
}

/// `text` read as a number as the Toolbox's `StringToNum` reads it, kept
/// to 16 bits as `_InitObjects` stores it (@0x1cf2f, `movw` @0x1cf37).
///
/// The rule was probed on the current macOS `CarbonCore`, as no i386-era
/// one is at hand. An optional first `-` or `+` is the sign; every other
/// byte, a second sign, a space or a NUL included, adds its low four bits
/// as a decimal digit, n = n × 10 + (byte & 0xF), unchecked. It reads
/// Mac Roman bytes, so each character is read as the byte it decoded
/// from. Wrapping in 16 bits gives the low 16 bits of the original's
/// wider sum, as multiplying and adding commute with taking it mod 2^16.
#[must_use]
pub fn string_to_num(text: &str) -> i16 {
    // A character with no Mac Roman byte never comes from decoded data;
    // its code point's low byte stands in, as only the low bits count.
    let mut bytes = text
        .chars()
        .map(|c| nova_data::wire::string::mac_roman_byte(c).unwrap_or(c as u8))
        .peekable();
    let negative = bytes.next_if_eq(&b'-').is_some();
    if !negative {
        bytes.next_if_eq(&b'+');
    }
    let n = bytes.fold(0_i16, |n, byte| {
        n.wrapping_mul(10).wrapping_add(i16::from(byte & 0xF))
    });
    if negative { n.wrapping_neg() } else { n }
}

/// The string that names commodity `n`: its `'STR '` 9000 + n patch when
/// one exists, whatever it holds, or else its `STR#` 4000 string
/// (`_LoadStrings` @0x71afb-0x71b55, `_LoadPluginString` @0x71a8e).
///
/// Unlike [`base_price_string`]'s shared buffer, each name has its own
/// slot of `_cargoName`, which starts zero-filled (`__common`) and which
/// nothing else writes. An empty (zero-byte) patch copies nothing into
/// it, so it names its commodity empty, not as the slot before. A patch
/// whose length byte runs past its data names it with the data bytes
/// padded with NULs in the original; the sim keeps no bytes for it and
/// names it empty ([`StringPatch::shown`]). Stock data and editor-written
/// plug-ins never have one. A slot past the end of `STR#` 4000 is the
/// empty string, as `GetIndString` gives.
fn name_string(strings: &CommodityStrings, n: usize) -> &str {
    strings
        .name_patches
        .get(n)
        .and_then(StringPatch::shown)
        .unwrap_or_else(|| strings.names.get(n).map_or("", String::as_str))
}

/// The string that prices commodity `n`: its `'STR '` 9300 + n patch when
/// one exists, whatever it holds, or else its `STR#` 4004 string
/// (`_InitObjects` @0x1cedd-0x1cf43).
///
/// The original copies a patch into a buffer it reuses from slot to slot.
/// An empty (zero-byte) patch copies nothing, so slot n takes the string
/// slot n - 1 took. For slot 0 the buffer is uninitialised, and a patch
/// whose length byte runs past its data reads stale bytes, so the
/// original prices both at a number that depends on stale memory. The
/// sim cannot know it, and reads both as the empty string, priced 0, so
/// the commodity stays traded as in the original. A slot past the end of
/// `STR#` 4004 is the empty string too, as `GetIndString` gives. Stock
/// data and editor-written plug-ins never have such a patch.
fn base_price_string(strings: &CommodityStrings, n: usize) -> &str {
    match strings.price_patches.get(n) {
        Some(StringPatch::Text(text)) => text,
        Some(StringPatch::Empty) => n
            .checked_sub(1)
            .map_or("", |before| base_price_string(strings, before)),
        Some(StringPatch::Unreadable) => "",
        Some(StringPatch::Absent) | None => strings.base_prices.get(n).map_or("", String::as_str),
    }
}

/// Everything the exchange trades and every event that can move its
/// prices, read once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Goods {
    commodities: Vec<(u8, Commodity)>,
    junk: Vec<JunkRecord>,
    /// Every `öops` that can be read, by ascending ID, as
    /// [`PilotCatalog::disasters`] gives them: the events [`step_day`]
    /// rolls to start, in this order, and whose active days move an
    /// exchange's prices.
    pub disasters: Vec<DisasterRecord>,
    /// The stellars a `Stellar` -1 event may be placed at, by ascending ID.
    stellars: Vec<StellarId>,
    /// Every `spöb`'s `Flags`, by ID, as [`Goods::with_stellars`] got them.
    stellar_flags: BTreeMap<StellarId, u32>,
}

impl Goods {
    /// The `Flags` of `good`'s `jünk` record: none for a commodity or a
    /// `jünk` with no record.
    fn junk_flags(&self, good: Good) -> u16 {
        match good {
            Good::Junk(id) => self
                .junk
                .iter()
                .find(|junk| junk.id == id)
                .map_or(0, |junk| junk.flags),
            Good::Commodity(_) => 0,
        }
    }

    /// The goods and events `catalog` holds, and the stellars an event
    /// may be placed at.
    pub fn read(catalog: &impl PilotCatalog) -> Self {
        Self::new(
            &catalog.commodity_strings(),
            catalog.junk(),
            catalog.disasters(),
        )
        .with_stellars(&catalog.stellar_flags())
    }

    /// The goods from these commodity strings, `jünk` and `öops`.
    #[must_use]
    pub fn new(
        strings: &CommodityStrings,
        junk: Vec<JunkRecord>,
        disasters: Vec<DisasterRecord>,
    ) -> Self {
        Self {
            commodities: commodities(strings),
            junk,
            disasters,
            stellars: Vec::new(),
            stellar_flags: BTreeMap::new(),
        }
    }

    /// These goods, with a `Stellar` -1 event placed among
    /// [`event_stellars`] of `stellars`, each `spöb` with its `Flags`.
    #[must_use]
    pub fn with_stellars(mut self, stellars: &[(StellarId, u32)]) -> Self {
        self.stellars = event_stellars(stellars);
        self.stellar_flags = stellars.iter().copied().collect();
        self
    }

    /// `stellar`'s `Flags`, listed or not: 0 for a `spöb` with no
    /// record.
    #[must_use]
    pub fn stellar_flags(&self, stellar: StellarId) -> u32 {
        self.stellar_flags.get(&stellar).copied().unwrap_or(0)
    }

    /// `jünk` `id`'s `Abbrev`, as the status bar's cargo area names it;
    /// none for a `jünk` with no record.
    #[must_use]
    pub fn abbrev(&self, id: JunkId) -> Option<&str> {
        self.junk
            .iter()
            .find(|junk| junk.id == id)
            .map(|junk| junk.abbrev.as_str())
    }

    /// The `jünk` records, as the catalog gave them.
    #[must_use]
    pub fn junk(&self) -> &[JunkRecord] {
        &self.junk
    }

    /// `good`'s name, as the exchange names it, empty for a standard
    /// commodity with no name; none for a `jünk` with no record or a
    /// commodity numbered 6 or up, which is not traded.
    #[must_use]
    pub fn name(&self, good: Good) -> Option<&str> {
        match good {
            Good::Commodity(n) => self
                .commodities
                .iter()
                .find(|(number, _)| *number == n)
                .map(|(_, commodity)| commodity.name.as_str()),
            Good::Junk(id) => self
                .junk
                .iter()
                .find(|junk| junk.id == id)
                .map(|junk| junk.name.as_str()),
        }
    }
}

/// One good on a stellar's exchange.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarketRow {
    /// The good.
    pub good: Good,
    /// Its name.
    pub name: String,
    /// Its price here, a ton, the events active here included.
    pub price: i64,
    /// How many tons the player holds.
    pub held: u32,
    /// Whether the stellar sells it on this row: the player can buy it.
    /// Every commodity row, and by the engine every `jünk` row, is; by the
    /// Bible only a `jünk`'s `SoldAt` row
    /// ([`RuleKey::JunkTrade`](crate::RuleKey::JunkTrade)).
    pub sold_here: bool,
    /// Whether the stellar buys it on this row: the player can sell it.
    /// Every commodity row, and by the engine every `jünk` row, is; by the
    /// Bible only a `jünk`'s `BoughtAt` row.
    pub bought_here: bool,
    /// The original's list slot it sits in (`_TradeDialogUpdate`
    /// @0x4d103-0x4d5c5): commodity n in slot n, a `jünk`'s
    /// [`JunkSide::BoughtAt`] row in slot 6 and its [`JunkSide::SoldAt`]
    /// row in slot 7.
    pub slot: u8,
    /// The word it is marked with ([`commodity_marker`],
    /// [`junk_marker`]), if any.
    pub marker: Option<PriceMarker>,
}

/// A stellar's exchange, as the player sees it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Market {
    /// The goods traded, standard commodities first by number, then
    /// `jünk` by ID.
    pub rows: Vec<MarketRow>,
    /// What the exchange says of the events active here, one line each,
    /// drawn in item 15 (`_TradeDialogUpdate` @0x4df54-0x4dfb5).
    /// [`market`] leaves it empty: the story layer fills it through
    /// `nova_sim::exchange_events` (`nova_sim::Session::market`), since
    /// the text needs `STR#` strings this crate cannot read.
    pub events: Vec<String>,
    /// The player's credits.
    pub cash: i64,
    /// The fleet's cargo space, in tons ([`fleet_holds`]).
    pub capacity: u32,
    /// The ship's own cargo space, in tons (`_ShipTotalHolds`).
    pub ship_capacity: u32,
    /// The cargo space free, in tons.
    pub free: u32,
    /// The words the dialog is told in: [`market`] gives the stock ones,
    /// and the story layer `STR#` 2002's (`nova_sim::Session::market`).
    pub words: TradeWords,
    /// The cargo summary under the rows, item 12.
    pub summary: CargoSummary,
    /// The rule set the exchange was built by, which its questions read:
    /// how many tons a [`Lot::Click`] moves
    /// ([`RuleKey::TradeLot`](crate::RuleKey::TradeLot)), how the most a
    /// buy moves divides the cash by the price
    /// ([`RuleKey::TradeQuotient`](crate::RuleKey::TradeQuotient)),
    /// whether that cash is signed
    /// ([`RuleKey::TradeDebt`](crate::RuleKey::TradeDebt)), and whether
    /// Option asks for a count ([`Market::option_asks_count`]).
    pub rules: Rulebook,
}

impl Context for Market {
    fn rulebook(&self) -> &Rulebook {
        &self.rules
    }
}

impl Market {
    /// Where the header says the held tons are: in the fleet when
    /// escorts add holds ([`held_in`]).
    #[must_use]
    pub const fn held_in(&self) -> HeldIn {
        held_in(self.ship_capacity, self.capacity)
    }

    /// `good`'s row, if it is traded here.
    #[must_use]
    pub fn row(&self, good: Good) -> Option<&MarketRow> {
        self.rows.iter().find(|row| row.good == good)
    }

    /// The row `order` trades on: its [`row`](Order::row), when there is
    /// one and it is of the order's good.
    #[must_use]
    pub fn ordered(&self, order: Order) -> Option<&MarketRow> {
        self.rows
            .get(order.row)
            .filter(|row| row.good == order.good)
    }

    /// How many tons `order` would move on its row, or why it moves none:
    /// an order on no row, or on another good's, is not traded.
    pub fn tons(&self, order: Order) -> Result<u32, TradeRefusal> {
        let row = self.ordered(order).ok_or(TradeRefusal::NotTraded)?;
        self.row_tons(row, order.direction, order.lot)
    }

    /// How many tons `lot` of `row` would move `direction`, or why it
    /// moves none: refused by the Buy or Sell checks ([`Market::row_allows`]),
    /// then by the order's own, and otherwise sized.
    fn row_tons(
        &self,
        row: &MarketRow,
        direction: Direction,
        lot: Lot,
    ) -> Result<u32, TradeRefusal> {
        let trade = RowTrade { market: self, row };
        // `_CanBuyGoods` greys Buy while cash < price, compared signed
        // (@0xcd07-0xcd0d), so no buy reaches the quotient there.
        refusal(&can(direction), &trade)?;
        let order = RowOrder {
            trade,
            direction,
            lot,
        };
        refusal(&order_checks(direction), &order)?;
        Ok(tons().of(&order))
    }

    /// The most row `index` offers `direction` ([`Lot::Max`]), which the
    /// original's quantity dialog opens with: 0 or less where nothing can
    /// be traded, as on a row priced below nothing. `None` where there is
    /// no such row or it does not trade `direction`, so no dialog opens.
    #[must_use]
    pub fn row_max(&self, index: usize, direction: Direction) -> Option<i64> {
        self.rows
            .get(index)
            .filter(|row| row.trades(direction))
            .map(|row| most(direction).of(&RowTrade { market: self, row }))
    }

    /// Whether Option (Alt) on Buy or Sell asks for a count in the
    /// quantity dialog, by the engine, or trades the most at once, as
    /// [`RuleKey::TradeCount`](crate::RuleKey::TradeCount) says. The trade
    /// itself is the same either way; whoever shows the exchange reads it.
    #[must_use]
    pub fn option_asks_count(&self) -> bool {
        option_asks_count().of(self)
    }

    /// Whether the Buy or Sell button of row `index` is enabled now. A buy
    /// is, as `_CanBuyGoods` has it (@0xccec-0xcd31), where the row is sold
    /// here, the cash is at least its price, compared signed, and space is
    /// free: so on a row priced below nothing it is enabled, though the
    /// buy then moves nothing. A sale is, as `_CanSellGoods` has it
    /// (@0x4a94), where the row is bought here and some of it is held.
    /// The same Buy and Sell checks refuse an order in [`Market::tons`]
    /// before it is sized.
    #[must_use]
    pub fn row_allows(&self, index: usize, direction: Direction) -> bool {
        self.rows
            .get(index)
            .is_some_and(|row| refusal(&can(direction), &RowTrade { market: self, row }).is_ok())
    }
}

/// One row of an exchange, asked whether it trades and how much: the
/// facts the Buy and Sell checks and the most read.
#[derive(Clone, Copy)]
struct RowTrade<'a> {
    /// The exchange: its cash, free space and rule set.
    market: &'a Market,
    /// The row: its price, the tons held and the ways it trades.
    row: &'a MarketRow,
}

impl Context for RowTrade<'_> {
    fn rulebook(&self) -> &Rulebook {
        &self.market.rules
    }
}

/// The first of `checks` that refuses `facts`, if any.
fn refusal<C: Context>(checks: &Checks<C, TradeRefusal>, facts: &C) -> Result<(), TradeRefusal> {
    // No exchange check draws (`no_exchange_check_draws`), so no chance
    // is ever asked.
    checks.first_refusal(facts, &mut NeverFires)
}

/// Whether a row can be traded `direction`: [`can_buy`] or [`can_sell`].
fn can<'a>(direction: Direction) -> Checks<RowTrade<'a>, TradeRefusal> {
    match direction {
        Direction::Buy => can_buy(),
        Direction::Sell => can_sell(),
    }
}

/// Whether Buy is enabled on a row, as `_CanBuyGoods` has it
/// (@0xccec-0xcd31): the row is sold here, the cash is at least the
/// price, compared signed (@0xcd07-0xcd0d), and a ton of space is free
/// (@0xcd0f-0xcd26), in `_CanBuyGoods`'s order.
fn can_buy<'a>() -> Checks<RowTrade<'a>, TradeRefusal> {
    [
        Check::plain(
            "sold here",
            "the row's ways as listed (`jünk ways`); `_CanBuyGoods` @0xccec asks no list",
            sold_here,
        ),
        Check::plain(
            "cash for the price",
            "`_CanBuyGoods` @0xcd07-0xcd0d, signed",
            cash_for_the_price,
        ),
        Check::plain("space free", "`_CanBuyGoods` @0xcd0f-0xcd26", space_free),
    ]
    .into()
}

/// Whether Sell is enabled on a row, as `_CanSellGoods` has it
/// (@0x4a94): the row is bought here, and some of it is held.
fn can_sell<'a>() -> Checks<RowTrade<'a>, TradeRefusal> {
    [
        Check::plain(
            "bought here",
            "the row's ways as listed (`jünk ways`); `_CanSellGoods` @0x4a94 asks no list",
            bought_here,
        ),
        Check::plain(
            "held",
            "`_CanSellGoods` @0x4aab-0x4ab1 (a commodity), @0x4ad9 (a `jünk`)",
            held,
        ),
    ]
    .into()
}

/// `Ok` when `holds`, else `refusal`.
fn holds(holds: bool, refusal: TradeRefusal) -> Result<(), TradeRefusal> {
    if holds { Ok(()) } else { Err(refusal) }
}

/// The row is sold here: by the engine every listed row is.
fn sold_here(trade: &RowTrade) -> Result<(), TradeRefusal> {
    holds(trade.row.sold_here, TradeRefusal::NotTraded)
}

/// A ton of space is free.
fn space_free(trade: &RowTrade) -> Result<(), TradeRefusal> {
    holds(trade.market.free > 0, TradeRefusal::NoSpace)
}

/// The cash is at least the price, compared signed.
fn cash_for_the_price(trade: &RowTrade) -> Result<(), TradeRefusal> {
    holds(
        trade.market.cash >= trade.row.price,
        TradeRefusal::CannotAfford,
    )
}

/// The row is bought here: by the engine every listed row is.
fn bought_here(trade: &RowTrade) -> Result<(), TradeRefusal> {
    holds(trade.row.bought_here, TradeRefusal::NotTraded)
}

/// Some of the row's good is held.
fn held(trade: &RowTrade) -> Result<(), TradeRefusal> {
    holds(trade.row.held > 0, TradeRefusal::NoneHeld)
}

/// How many tons a [`Lot::Click`] moves at most, as
/// [`RuleKey::TradeLot`](crate::RuleKey::TradeLot) says: [`CLICK_TONS`]
/// by the engine, a ton by the other reading.
fn click_lot<'a>() -> Value<RowTrade<'a>, u32> {
    Value::disputed(
        "click lot",
        "`_DoTradeDialog` @0x5e268-0x5e278, @0x5e48d-0x5e4fa",
        RuleKey::TradeLot,
        ten_tons,
        one_ton,
    )
}

/// The engine's click: up to [`CLICK_TONS`].
fn ten_tons(_: &RowTrade) -> u32 {
    CLICK_TONS
}

/// The other reading's click: a ton.
fn one_ton(_: &RowTrade) -> u32 {
    1
}

/// The cash a buy divides by the price, as
/// [`RuleKey::TradeDebt`](crate::RuleKey::TradeDebt) says: by the engine
/// signed, so cash below nothing at a price below nothing gives a
/// positive quotient; by the other reading cash below nothing counts as
/// none, so a price below nothing gives 0 or less.
fn cash_divided<'a>() -> Value<RowTrade<'a>, i64> {
    Value::disputed(
        "cash divided",
        "`_DoTradeDialog` @0x5e21d-0x5e234",
        RuleKey::TradeDebt,
        signed_cash,
        cash_never_below_none,
    )
}

/// The engine's cash: signed.
fn signed_cash(trade: &RowTrade) -> i64 {
    trade.market.cash
}

/// The other reading's cash: none when below nothing.
fn cash_never_below_none(trade: &RowTrade) -> i64 {
    trade.market.cash.max(0)
}

/// The [`cash_divided`] divided by the row's price, as
/// [`RuleKey::TradeQuotient`](crate::RuleKey::TradeQuotient) says, or
/// `None` at a price of nothing, which the engine never lists (phase 23).
/// By the engine's division, trunc(f32(cash) / f32(price)), the cash is
/// held in the engine's 32 bits and the price in its 16, each held at the
/// least or most it can hold rather than wrapped past it; above 2^24 cash
/// it can round up to a ton more than the cash covers. By the other
/// division, exact integer division.
fn quotient<'a>() -> Value<RowTrade<'a>, Option<i64>> {
    Value::disputed(
        "quotient",
        "`_DoTradeDialog` @0x5e21d-0x5e234",
        RuleKey::TradeQuotient,
        engine_quotient,
        exact_quotient,
    )
}

/// The engine's quotient, in single floats.
fn engine_quotient(trade: &RowTrade) -> Option<i64> {
    let price = trade.row.price;
    if price == 0 {
        return None;
    }
    let cash = cash_divided().of(trade);
    let cash = i32::try_from(cash).unwrap_or(if cash < 0 { i32::MIN } else { i32::MAX });
    let price = i16::try_from(price).unwrap_or(if price < 0 { i16::MIN } else { i16::MAX });
    Some((cash as f32 / f32::from(price)) as i64)
}

/// The exact quotient, in integers.
fn exact_quotient(trade: &RowTrade) -> Option<i64> {
    let price = trade.row.price;
    (price != 0).then(|| cash_divided().of(trade) / price)
}

/// The [`cash_divided`] a plain buy divides by the row's price: exact
/// integer division whatever
/// [`RuleKey::TradeQuotient`](crate::RuleKey::TradeQuotient) says, as the
/// single-float quotient differs from it only above 2^24 cash, where it
/// is at least 512 and a click moves at most 10 either way ([`Lot::Click`]).
fn click_quotient<'a>() -> Value<RowTrade<'a>, Option<i64>> {
    Value::plain(
        "click quotient",
        "`_DoTradeDialog` @0x5e268-0x5e278",
        exact_quotient,
    )
}

/// The most a buy on a row offers, which the original's quantity dialog
/// opens with, signed, as it can be 0 or less: min([`quotient`], free),
/// and anything from 32001 up made 32000; a row priced at nothing is
/// limited by space alone.
fn buy_most<'a>() -> Value<RowTrade<'a>, i64> {
    Value::plain("buy most", "`_DoTradeDialog` @0x5e23a-0x5e25a", most_bought)
}

/// The quotient or the free space, whichever is less, at most 32000.
fn most_bought(trade: &RowTrade) -> i64 {
    let space = i64::from(trade.market.free);
    quotient()
        .of(trade)
        .map_or(space, |tons| tons.min(space))
        .min(LOT_CAP)
}

/// The most a sale on a row offers: min(held, 32000).
fn sell_most<'a>() -> Value<RowTrade<'a>, i64> {
    Value::plain("sell most", "`_DoTradeDialog` @0x5e44f-0x5e47f", most_sold)
}

/// Everything held, at most 32000.
fn most_sold(trade: &RowTrade) -> i64 {
    i64::from(trade.row.held).min(LOT_CAP)
}

/// The most a row offers `direction`: [`buy_most`] or [`sell_most`].
fn most<'a>(direction: Direction) -> Value<RowTrade<'a>, i64> {
    match direction {
        Direction::Buy => buy_most(),
        Direction::Sell => sell_most(),
    }
}

/// Whether Option (Alt) on Buy or Sell asks for a count, as
/// [`RuleKey::TradeCount`](crate::RuleKey::TradeCount) says: by the
/// engine it opens the quantity dialog at the most the row offers; by the
/// other reading it trades the most at once.
fn option_asks_count() -> Value<Market, bool> {
    Value::disputed(
        "option asks count",
        "`_DoTradeDialog` @0x5e211, @0x5e45c; `_DoQuantityDialog` @0x56d5b",
        RuleKey::TradeCount,
        asks_a_count,
        trades_the_most,
    )
}

/// The engine's Option: a count.
fn asks_a_count(_: &Market) -> bool {
    true
}

/// The other reading's Option: the most.
fn trades_the_most(_: &Market) -> bool {
    false
}

/// One order on a row: the row, the way and the lot, as the order's
/// checks and its tons read them.
#[derive(Clone, Copy)]
struct RowOrder<'a> {
    /// The row traded.
    trade: RowTrade<'a>,
    /// Buying or selling.
    direction: Direction,
    /// How much.
    lot: Lot,
}

impl Context for RowOrder<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.trade.rulebook()
    }
}

/// Whether an order the Buy or Sell checks allow moves anything. A buy
/// must move a ton: a click its tons, any other lot the most; so a good
/// priced below nothing buys none, as the engine's cash / price is then
/// negative and its `jle` @0x5e27d skips the buy. A counted lot must
/// then be from 1 up to the most ([`TradeRefusal::OutOfRange`]).
fn order_checks<'a>(direction: Direction) -> Checks<RowOrder<'a>, TradeRefusal> {
    let in_range = |reproduces| Check::plain("count in range", reproduces, count_in_range);
    match direction {
        Direction::Buy => [
            Check::plain(
                "moves a ton",
                "`_DoTradeDialog` `jle` @0x5e27d",
                moves_a_ton,
            ),
            in_range(
                "`_DoTradeDialog` `jle` @0x5e27d skips none; \
                 the quantity dialog gives no more than the most",
            ),
        ]
        .into(),
        Direction::Sell => [in_range(
            "`_DoTradeDialog` `jle` @0x5e4d8 skips none; \
             the quantity dialog gives no more than the most",
        )]
        .into(),
    }
}

/// A buy moves a ton: a click's tons, or the most, above 0.
fn moves_a_ton(order: &RowOrder) -> Result<(), TradeRefusal> {
    let tons = match order.lot {
        Lot::Click => i64::from(click_bought(&order.trade)),
        Lot::Max | Lot::Count(_) => buy_most().of(&order.trade),
    };
    holds(tons > 0, TradeRefusal::CannotAfford)
}

/// A counted lot is from 1 up to the most; any other lot passes.
fn count_in_range(order: &RowOrder) -> Result<(), TradeRefusal> {
    match order.lot {
        Lot::Count(n) => holds(
            n > 0 && i64::from(n) <= most(order.direction).of(&order.trade),
            TradeRefusal::OutOfRange,
        ),
        Lot::Click | Lot::Max => Ok(()),
    }
}

/// The tons an order the checks allow moves: a click's, the most, or
/// its count.
fn tons<'a>() -> Value<RowOrder<'a>, u32> {
    Value::plain(
        "tons",
        "`_DoTradeDialog` @0x5e268-0x5e278, @0x5e48d-0x5e4fa (a click); \
         @0x5e23a-0x5e25a, @0x5e44f-0x5e47f (the most); the quantity dialog's count",
        order_tons,
    )
}

/// A plain buy's tons: min(cash / price, [`click_lot`], free), the cash
/// divided exactly ([`click_quotient`]); at a price of nothing, the click
/// lot or the free space.
fn click_bought(trade: &RowTrade) -> u32 {
    let wanted = click_lot().of(trade).min(trade.market.free);
    click_quotient().of(trade).map_or(wanted, |tons| {
        wanted.min(u32::try_from(tons.max(0)).unwrap_or(u32::MAX))
    })
}

/// A click buys [`click_bought`] and sells min(held, [`click_lot`]);
/// [`Lot::Max`] moves the most and [`Lot::Count`] its count.
fn order_tons(order: &RowOrder) -> u32 {
    match (order.lot, order.direction) {
        (Lot::Count(n), _) => n,
        (Lot::Click, Direction::Buy) => click_bought(&order.trade),
        (Lot::Click, Direction::Sell) => order.trade.row.held.min(click_lot().of(&order.trade)),
        (Lot::Max, direction) => u32::try_from(most(direction).of(&order.trade)).unwrap_or(0),
    }
}

impl MarketRow {
    /// Whether this row trades `direction`: a buy where the stellar sells
    /// it, a sale where it buys it.
    #[must_use]
    pub fn trades(&self, direction: Direction) -> bool {
        match direction {
            Direction::Buy => self.sold_here,
            Direction::Sell => self.bought_here,
        }
    }
}

/// The pilot trading at an exchange, as [`market`] reads it and
/// [`settle`] changes it: its credits, the goods it holds, the cargo
/// aboard and the planetary events under way. The story layer's
/// `nova_sim::pilot::Pilot` implements it.
pub trait Trader {
    /// The credits.
    fn cash(&self) -> i64;
    /// The tons of `good` held; none of a good not held.
    fn held(&self, good: Good) -> u32;
    /// The tons aboard, as `_TotalCargo` (@0x4a0b) counts them: every good
    /// held, and the mission cargo aboard, which is never held as a good.
    fn total_cargo(&self) -> i64;
    /// The planetary events under way: each `öops` with the days it has
    /// left and the stellar it is at.
    fn events_under_way(&self) -> &BTreeMap<DisasterId, ActiveEvent>;
    /// Each mission slot's cargo, as the cargo summary counts it.
    fn mission_loads(&self) -> Vec<MissionLoad>;
    /// The credits, to change.
    fn cash_mut(&mut self) -> &mut i64;
    /// The tons held of each good, to change; none of a good not listed.
    fn cargo_mut(&mut self) -> &mut BTreeMap<Good, u32>;
}

/// What an exchange is built from, gathered once: the goods and events,
/// the stellar and its `spöb` `Flags`, the player's pilot and the fleet's
/// cargo space, the control-bit tests a `jünk` asks (a [`Gate`]), the
/// rule set and the stellar's [`Markup`].
#[derive(Clone, Copy)]
pub struct Exchange<'a> {
    /// Everything the exchange trades and every event that can move its
    /// prices.
    pub goods: &'a Goods,
    /// The stellar the exchange is at.
    pub stellar: StellarId,
    /// The stellar's `spöb` `Flags`: its trade center and price levels.
    pub flags: u32,
    /// The player's pilot: cash, cargo and the events under way.
    pub pilot: &'a dyn Trader,
    /// The fleet's cargo space, in tons ([`fleet_holds`]).
    pub capacity: u32,
    /// The ship's own cargo space, in tons.
    pub ship_capacity: u32,
    /// What a `jünk`'s `BuyOn` and `SellOn` tests are asked of.
    pub gate: Gate<'a>,
    /// The rule set the exchange is priced and traded by.
    pub rules: Rulebook,
    /// What the stellar's low and high prices are marked down and up by.
    pub markup: Markup,
}

/// The exchange `exchange` describes; `None` without a trade center.
///
/// Each standard commodity is listed at its [`commodity_price`], when it
/// has one, traded both ways; each `jünk` row a stellar lists, as
/// [`junk_listing`] lists it. Then the rows are ordered by good, and
/// each shows the tons held.
#[must_use]
pub fn market(exchange: &Exchange) -> Option<Market> {
    let &Exchange {
        goods,
        stellar,
        flags,
        pilot,
        capacity,
        ship_capacity,
        ref rules,
        ..
    } = exchange;
    if flags & StellarFlags::TRADE_CENTER == 0 {
        return None;
    }
    let here: Vec<&DisasterRecord> = pilot
        .events_under_way()
        .iter()
        .filter_map(|(id, active)| {
            let event = goods.disasters.iter().find(|event| event.id == *id)?;
            (place(*active, event) == Some(stellar) && event_commodity(event.commodity).is_some())
                .then_some(event)
        })
        .collect();
    let mut rows: Vec<MarketRow> = goods
        .commodities
        .iter()
        .filter_map(|(n, commodity)| {
            // By ascending ID, as `pilot.events` is ordered.
            let deltas: Vec<i64> = here
                .iter()
                .filter(|event| event_commodity(event.commodity) == Some(*n))
                .map(|event| i64::from(event.price_delta))
                .collect();
            let quote = CommodityQuote {
                exchange,
                base: commodity.base_price,
                level: price_level(flags, *n),
                deltas: &deltas,
            };
            let price = commodity_price().of(&quote)?;
            let delta = first_local_delta(pilot.events_under_way(), &goods.disasters, stellar, *n);
            Some(MarketRow {
                slot: *n,
                marker: commodity_marker(delta, price_level(flags, *n)),
                ..listed(Good::Commodity(*n), &commodity.name, price, (true, true))
            })
        })
        .collect();
    let (junk_rows, named) = junk_listing(exchange);
    rows.extend(junk_rows);
    rows.sort_by_key(|row| row.good);
    for row in &mut rows {
        row.held = pilot.held(row.good);
    }
    // `_TotalCargo`: the mission cargo aboard takes space too, though no
    // row holds it, so it cannot be sold.
    let free = (i64::from(capacity) - pilot.total_cargo()).max(0);
    // Every `jünk` held that neither row names is "other cargo"
    // (@0x4d70c-0x4d767), as the original tests by slot, not by price.
    let unlisted = goods
        .junk
        .iter()
        .filter(|junk| !named.contains(&junk.id))
        .map(|junk| i64::from(pilot.held(Good::Junk(junk.id))));
    let summary = cargo_summary(
        &pilot.mission_loads(),
        unlisted,
        ship_capacity,
        capacity,
        pilot.total_cargo(),
    );
    Some(Market {
        rows,
        events: Vec::new(),
        cash: pilot.cash(),
        capacity,
        ship_capacity,
        free: u32::try_from(free).unwrap_or(u32::MAX),
        words: TradeWords::STOCK,
        summary,
        rules: *rules,
    })
}

/// The `jünk` rows `exchange` lists. A `jünk` whose `BoughtAt` lists the
/// stellar, while the gate allows its `BuyOn`, has a row at
/// [`JunkSide::BoughtAt`]; one whose `SoldAt` lists it, while the gate
/// allows its `SellOn`, a row at [`JunkSide::SoldAt`]: the bought one
/// first, as the original's rows 6 and 7 (@0x5dd83-0x5de2f). Each is
/// priced by [`junk_price`], and not listed without a price, and traded
/// the [`junk_ways`]. Also every `jünk` a side names, priced or not,
/// which the cargo summary leaves out of its "other cargo".
fn junk_listing(exchange: &Exchange) -> (Vec<MarketRow>, Vec<JunkId>) {
    let mut rows = Vec::new();
    let mut named = Vec::new();
    for junk in &exchange.goods.junk {
        let bought =
            junk.bought_at.contains(&exchange.stellar) && exchange.gate.allows(&junk.buy_on);
        let sold = junk.sold_at.contains(&exchange.stellar) && exchange.gate.allows(&junk.sell_on);
        let sides = [
            bought.then_some(JunkSide::BoughtAt),
            sold.then_some(JunkSide::SoldAt),
        ];
        if bought || sold {
            named.push(junk.id);
        }
        for side in sides.into_iter().flatten() {
            let quote = JunkQuote {
                exchange,
                base: i64::from(junk.base_price),
                side,
            };
            if let Some(price) = junk_price().of(&quote) {
                let ways = junk_ways().of(&quote);
                rows.push(MarketRow {
                    slot: side.slot(),
                    marker: Some(junk_marker(side)),
                    ..listed(Good::Junk(junk.id), &junk.name, price, ways)
                });
            }
        }
    }
    (rows, named)
}

/// One standard commodity being priced at an exchange: the facts its
/// price rules read.
struct CommodityQuote<'a> {
    /// The exchange: its rule set and markup.
    exchange: &'a Exchange<'a>,
    /// The commodity's `BasePrice`.
    base: i64,
    /// The stellar's price level for it ([`price_level`]); `None` where
    /// it is not traded at a level.
    level: Option<PriceLevel>,
    /// The `PriceDelta` of each event active here on it, by ascending
    /// `öops` ID.
    deltas: &'a [i64],
}

impl Context for CommodityQuote<'_> {
    fn rulebook(&self) -> &Rulebook {
        &self.exchange.rules
    }
}

/// The price of a commodity at its level: its [`band_price`], held in 16
/// bits; none where it is not traded at a level.
fn level_price<'a>() -> Value<CommodityQuote<'a>, Option<i64>> {
    Value::plain(
        "level price",
        "`_DoTradeDialog` @0x5dc0c-0x5dc48, @0x5dc87-0x5dcc5, held in 16 bits @0x5dcc5",
        level_band,
    )
}

/// `quote`'s band price at its level, if it has one.
fn level_band(quote: &CommodityQuote) -> Option<i64> {
    quote
        .level
        .map(|level| band_price(quote.base, level, quote.exchange.markup))
}

/// The price of a commodity with the events active here, as
/// [`RuleKey::EventPrice`](crate::RuleKey::EventPrice) says: by the
/// engine its `BasePrice` plus the highest ID's `PriceDelta`, whatever
/// its level, so an event lists a commodity not traded here; by the
/// other reading its [`level_price`] plus every `PriceDelta`. Either way
/// held in 16 bits; with no event, the level price.
fn event_price<'a>() -> Value<CommodityQuote<'a>, Option<i64>> {
    Value::disputed(
        "event price",
        "`_DoTradeDialog` @0x5dcf8-0x5dd58, held in 16 bits by `addw` @0x5dd19-0x5dd40",
        RuleKey::EventPrice,
        last_event_price,
        summed_event_price,
    )
}

/// The engine's event price: the base plus the last (highest ID) delta,
/// held in 16 bits, or else the level price.
fn last_event_price(quote: &CommodityQuote) -> Option<i64> {
    quote.deltas.last().map_or_else(
        || level_price().of(quote),
        |delta| Some(word(quote.base + delta)),
    )
}

/// The other reading's event price: the level price plus every delta,
/// held in 16 bits. Wrapping the sum once is wrapping each addition, so
/// the deltas' order does not matter.
fn summed_event_price(quote: &CommodityQuote) -> Option<i64> {
    let delta: i64 = quote.deltas.iter().sum();
    level_price().of(quote).map(|price| word(price + delta))
}

/// A commodity's price on the exchange: its [`event_price`], but never
/// below [`MIN_COMMODITY_PRICE`]; none where it is not listed.
fn commodity_price<'a>() -> Value<CommodityQuote<'a>, Option<i64>> {
    Value::plain(
        "commodity price",
        "floor of 5: `_DoTradeDialog` @0x5dcc8-0x5dcce, @0x5dd39-0x5dd40",
        floored_commodity_price,
    )
}

/// `quote`'s event price, raised to [`MIN_COMMODITY_PRICE`].
fn floored_commodity_price(quote: &CommodityQuote) -> Option<i64> {
    event_price()
        .of(quote)
        .map(|price| price.max(MIN_COMMODITY_PRICE))
}

/// Which of a stellar's lists a `jünk` row comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JunkSide {
    /// Its `BoughtAt`: the stellar buys it, at the high price.
    BoughtAt,
    /// Its `SoldAt`: the stellar sells it, at the low price.
    SoldAt,
}

impl JunkSide {
    /// The original's list slot of the side's row: 6 for `BoughtAt`, 7
    /// for `SoldAt`.
    #[must_use]
    pub const fn slot(self) -> u8 {
        match self {
            Self::BoughtAt => 6,
            Self::SoldAt => 7,
        }
    }

    /// The level the row is priced at.
    const fn level(self) -> PriceLevel {
        match self {
            Self::BoughtAt => PriceLevel::High,
            Self::SoldAt => PriceLevel::Low,
        }
    }
}

/// One `jünk` row being priced and listed at an exchange: the facts its
/// rules read.
struct JunkQuote<'a> {
    /// The exchange: its rule set and markup.
    exchange: &'a Exchange<'a>,
    /// The `jünk`'s `BasePrice`.
    base: i64,
    /// The list the row comes from.
    side: JunkSide,
}

impl Context for JunkQuote<'_> {
    fn rulebook(&self) -> &Rulebook {
        &self.exchange.rules
    }
}

/// A `jünk` row's price, as
/// [`RuleKey::JunkPrice`](crate::RuleKey::JunkPrice) says, or `None` when
/// the row is not listed. Its band price at its side's level, held in 16
/// bits: by the engine signed, with no floor, and a row priced 0 not
/// listed; by the other reading never below 0, and a row priced 0 listed.
fn junk_price<'a>() -> Value<JunkQuote<'a>, Option<i64>> {
    Value::disputed(
        "jünk price",
        "rows 6/7 `_DoTradeDialog` @0x5dd83-0x5de2f, held in 16 bits @0x5dddb/@0x5de2f; \
         unlisted at 0 `_TradeDialogUpdate` @0x4d2a2-0x4d2ac, `_TradeFilter` @0x4e262; \
         signed `_TradeDialogUpdate` @0x4d545-0x4d54b",
        RuleKey::JunkPrice,
        signed_junk_price,
        floored_junk_price,
    )
}

/// `quote`'s band price at its side's level.
fn junk_band(quote: &JunkQuote) -> i64 {
    band_price(quote.base, quote.side.level(), quote.exchange.markup)
}

/// The engine's `jünk` price: signed, and unlisted at 0.
fn signed_junk_price(quote: &JunkQuote) -> Option<i64> {
    let band = junk_band(quote);
    (band != 0).then_some(band)
}

/// The other reading's `jünk` price: never below 0, and listed at 0.
#[allow(clippy::unnecessary_wraps)] // The engine's reading's type: a row it does not list.
fn floored_junk_price(quote: &JunkQuote) -> Option<i64> {
    Some(junk_band(quote).max(0))
}

/// The ways (sold here, bought here) a `jünk` row trades, as
/// [`RuleKey::JunkTrade`](crate::RuleKey::JunkTrade) says: by the engine
/// both ways, as `_CanBuyGoods` and `_CanSellGoods` never ask which list a
/// row came from; by the Bible, the `BoughtAt` row is sold only and the
/// `SoldAt` row bought only.
fn junk_ways<'a>() -> Value<JunkQuote<'a>, (bool, bool)> {
    Value::disputed(
        "jünk ways",
        "`_CanBuyGoods` @0xccec, `_CanSellGoods` @0x4a94, `_DrawTradeButtons` \
         @0x2938f/0x2939d, `_TrackTradeButtons` @0x2960d/0x2963e",
        RuleKey::JunkTrade,
        traded_both_ways,
        traded_by_side,
    )
}

/// The engine's ways: both.
fn traded_both_ways(_: &JunkQuote) -> (bool, bool) {
    (true, true)
}

/// The Bible's ways: a `BoughtAt` row sold here only, a `SoldAt` row
/// bought here only.
fn traded_by_side(quote: &JunkQuote) -> (bool, bool) {
    match quote.side {
        JunkSide::BoughtAt => (false, true),
        JunkSide::SoldAt => (true, false),
    }
}

/// The standard commodity an event's raw `commodity` moves, as the loader
/// reads it (`_LoadObjectData` @0x7cd56-0x7cd6f): one above 5 is 5
/// (equipment), and a negative one is none, an event that prices nothing
/// (the original writes `_commodityPrice[-1]`, @0x5dd19-0x5dd40).
#[must_use]
pub fn event_commodity(raw: i16) -> Option<u8> {
    u8::try_from(raw.min(i16::from(COMMODITIES) - 1)).ok()
}

/// A row for `good`, priced at `price`, traded these ways (sold here,
/// bought here), with none held.
fn listed(good: Good, name: &str, price: i64, (sold_here, bought_here): (bool, bool)) -> MarketRow {
    MarketRow {
        good,
        name: name.to_owned(),
        price,
        held: 0,
        sold_here,
        bought_here,
        slot: 0,
        marker: None,
    }
}

/// Moves `tons` of `order`'s good at `price` a ton between the stellar
/// and `pilot`, paying or being paid.
pub fn settle(pilot: &mut (impl Trader + ?Sized), order: Order, tons: u32, price: i64) {
    let total = i64::from(tons).saturating_mul(price);
    let held = pilot.held(order.good);
    let cash = pilot.cash_mut();
    let held = match order.direction {
        Direction::Buy => {
            *cash = cash.saturating_sub(total);
            held.saturating_add(tons)
        }
        Direction::Sell => {
            *cash = cash.saturating_add(total);
            held.saturating_sub(tons)
        }
    };
    if held == 0 {
        pilot.cargo_mut().remove(&order.good);
    } else {
        pilot.cargo_mut().insert(order.good, held);
    }
}

/// The lowest `spöb` ID: an event's `Stellar` names a stellar only from
/// here up (the original's "above 127", @0x41b3c).
const FIRST_STELLAR: i16 = 128;

/// How many `spöb` slots the original loads: IDs 128 to 2175 (@0x7721a,
/// @0x779e7).
const STELLAR_SLOTS: i16 = 2048;

/// The `Stellar` of an event at any stellar, drawn when it starts.
const ANY_STELLAR: i16 = -1;

/// The stellar a record's `Stellar` names, if it names one.
fn fixed_stellar(stellar: i16) -> Option<StellarId> {
    (stellar >= FIRST_STELLAR).then_some(StellarId(stellar))
}

/// The stellars a `Stellar` -1 event may be placed at, by ascending ID:
/// those from 128 to 2175 whose `Flags` (`stellars` gives them raw) are
/// not uninhabited, whether or not they trade.
#[must_use]
pub fn event_stellars(stellars: &[(StellarId, u32)]) -> Vec<StellarId> {
    let mut candidates: Vec<StellarId> = stellars
        .iter()
        .filter(|(id, flags)| {
            (FIRST_STELLAR..FIRST_STELLAR + STELLAR_SLOTS).contains(&id.0)
                && flags & StellarFlags::UNINHABITED == 0
        })
        .map(|&(id, _)| id)
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

/// An event under way: its days left and the stellar it is at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveEvent {
    /// The days it has left.
    pub days: u16,
    /// The stellar it is at; `None` for its record's own `Stellar`, which
    /// only a save from before version 15 holds.
    pub stellar: Option<StellarId>,
}

/// The stellar `active`, an event of `event`'s, is at: the one it
/// stored, or else its record's.
#[must_use]
pub fn place(active: ActiveEvent, event: &DisasterRecord) -> Option<StellarId> {
    active.stellar.or_else(|| fixed_stellar(event.stellar))
}

/// One day's events: the active ones age, and each that can start, its
/// `ActivateOn` asked of `gate`, is rolled once on `chance`. An event
/// that had days left this morning is not rolled today, even if it ends
/// today (`_DisasterHandler` @0x41add-0x41ae7).
pub fn step_day(
    goods: &Goods,
    events: &mut BTreeMap<DisasterId, ActiveEvent>,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) {
    let aging: Vec<DisasterId> = events
        .iter()
        .filter(|(_, active)| active.days > 0)
        .map(|(&id, _)| id)
        .collect();
    events.retain(|id, active| {
        active.days = active.days.saturating_sub(1);
        active.days > 0 && goods.disasters.iter().any(|event| event.id == *id)
    });
    for event in &goods.disasters {
        let fixed = fixed_stellar(event.stellar);
        let drawn = event.stellar == ANY_STELLAR && !goods.stellars.is_empty();
        if fixed.is_none() && !drawn {
            continue;
        }
        let can_start = !events.contains_key(&event.id)
            && !aging.contains(&event.id)
            && event.freq > 0
            && event.duration > 0
            && gate.allows(&event.activate_on);
        if !can_start {
            continue;
        }
        let percent = u8::try_from(event.freq.min(100)).unwrap_or(100);
        if !chance.fires(percent) {
            continue;
        }
        // The `Freq` roll comes first, as the original's (@0x41b08,
        // @0x41b5b).
        let Some(stellar) = fixed.or_else(|| draw(&goods.stellars, chance)) else {
            continue;
        };
        let days = u16::try_from(event.duration).unwrap_or(u16::MAX);
        let stellar = Some(stellar);
        events.insert(event.id, ActiveEvent { days, stellar });
    }
}

/// An even pick among `stellars` on `chance`; none if the roll is out of
/// range.
fn draw(stellars: &[StellarId], chance: &mut (impl Chance + ?Sized)) -> Option<StellarId> {
    // At most `STELLAR_SLOTS` of them, so the count fits.
    let sides = u16::try_from(stellars.len()).unwrap_or(u16::MAX);
    stellars.get(usize::from(chance.roll(sides))).copied()
}

/// The `jünk` `Flags` bit of a good that multiplies in the hold.
pub const TRIBBLES: u16 = 0x0001;

/// The `jünk` `Flags` bit of a good that decays in the hold.
pub const PERISHABLE: u16 = 0x0002;

/// How many frames apart the tribbles and perishables steps fall: a frame
/// is due when the counter is a multiple of this (`_HandlePlayer`
/// @0x70835-0x70861, @0x708c3-0x708f9).
pub const JUNK_STEP_FRAMES: i16 = 250;

/// The frame counter's highest value: it counts up to this, then goes back
/// to 0 (`_PlayGame` @0x45f36-0x45f52), a cycle of 1025 frames.
pub const LAST_FRAME: i16 = 1024;

/// The frame counter on the first tick after a take-off. The original
/// sets -15 inside the take-off frame (`_HandlePlayerDockRequest`
/// @0x67678), and that frame's end moves it on to this; -15 to -1 are
/// never due, so the first step comes on the 15th tick of flight.
pub const AFTER_TAKE_OFF_FRAME: i16 = -14;

/// The frame counter after `frame`: one more, or 0 once that passes
/// [`LAST_FRAME`].
#[must_use]
pub fn next_frame(frame: i16) -> i16 {
    if frame >= LAST_FRAME { 0 } else { frame + 1 }
}

/// Whether the tribbles and perishables step falls due on `frame`: when it
/// is a multiple of [`JUNK_STEP_FRAMES`]. The remainder truncates toward
/// zero, as the original's does, so the frames -15 to -1 after a take-off
/// are not due.
#[must_use]
pub fn junk_step_due(frame: i16) -> bool {
    frame % JUNK_STEP_FRAMES == 0
}

/// The `jünk` latches `_ResetPlayerPrecalcedValues` sets
/// (`_playerHasTribbles` 0x3b5160, `_playerHasPerishableJunk` 0x3b50d4)
/// and the tribbles and perishables step reads: set only at a reset, so
/// they hold until the next one, whatever the step does to the cargo
/// in between.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JunkLatches {
    /// A tribbles good was held at the last reset.
    pub tribbles: bool,
    /// A perishable good that is not tribbles was held at the last
    /// reset.
    pub perishable: bool,
}

impl JunkLatches {
    /// The latches a reset leaves with `cargo` held: both cleared
    /// (@0xc404, @0xc452), then set by each `jünk` held above none, a
    /// tribbles good setting the tribbles latch (@0xc547), and one that
    /// is perishable and not tribbles the perishable latch (@0xc556), so
    /// a good of both flags sets only the tribbles latch
    /// (@0xc524-0xc570). A `jünk` with no record sets neither.
    #[must_use]
    pub fn of(goods: &Goods, cargo: &BTreeMap<Good, u32>) -> Self {
        let mut latches = Self::default();
        for (&good, &tons) in cargo {
            if tons == 0 {
                continue;
            }
            let flags = goods.junk_flags(good);
            if flags & TRIBBLES != 0 {
                latches.tribbles = true;
            } else if flags & PERISHABLE != 0 {
                latches.perishable = true;
            }
        }
        latches
    }
}

/// One tribbles and perishables step on `cargo`, in a hold of `capacity`
/// tons that also carries `mission` tons of mission cargo, read as
/// `source` says ([`RuleKey::JunkFlags`](crate::RuleKey::JunkFlags)), with
/// the `jünk` latches as the last reset left them (`latches`); whether
/// it changed anything. Only held `jünk` whose record has a flag
/// changes, a ton at a time, and a good decayed to none is gone.
///
/// The free space is the capacity less every good held and the mission
/// cargo aboard, as `_TotalCargo` (@0x4a0b) counts them and the step
/// subtracts them (@0x70863-0x70888), never below none, measured once.
///
/// - By the engine (`_HandlePlayer` @0x70827-0x7093b), the latches decide
///   whether each loop runs, and the goods each loop visits are those
///   held now. With the tribbles latch set (@0x70827), while that space
///   is above none, every tribbles good held gains a ton, so with several
///   of them the hold can go over (@0x7088f-0x708c1). Then, with the
///   perishable latch set (@0x708c3), every perishable good held loses a
///   ton, on the same space, measured before the growth
///   (@0x708fb-0x7093b). So a good with both flags only grows unless a
///   perishable-only good was aboard at the last reset, and then it grows
///   and decays until the next reset, even once that good has decayed
///   away. With the tribbles latch clear, the original's perishable test
///   reads a value nothing set for it, which is undefined; here it reads
///   the free space measured the same way.
/// - By the Bible, which says only that tribbles goods multiply and
///   perishable goods "gradually decay away", the latches are not read:
///   the tribbles goods held grow by ascending ID, each taking a ton of
///   the free space, until none is left, so the hold never goes over;
///   then every perishable good held loses a ton, whatever the space.
pub fn step_junk(
    goods: &Goods,
    cargo: &mut BTreeMap<Good, u32>,
    capacity: u32,
    mission: u32,
    source: RuleSource,
    latches: JunkLatches,
) -> bool {
    let held: Vec<(Good, u16)> = cargo
        .iter()
        .filter(|&(_, &tons)| tons > 0)
        .map(|(&good, _)| (good, goods.junk_flags(good)))
        .collect();
    let with = |flag: u16| -> Vec<Good> {
        held.iter()
            .filter(|(_, flags)| flags & flag != 0)
            .map(|&(good, _)| good)
            .collect()
    };
    let total = cargo
        .values()
        .fold(mission, |sum, &tons| sum.saturating_add(tons));
    let free = capacity.saturating_sub(total);
    let (grow, decay) = match source {
        RuleSource::Engine => {
            let room = free > 0;
            let grow = if room && latches.tribbles {
                with(TRIBBLES)
            } else {
                Vec::new()
            };
            let decay = if room && latches.perishable {
                with(PERISHABLE)
            } else {
                Vec::new()
            };
            (grow, decay)
        }
        RuleSource::Bible => {
            let room = usize::try_from(free).unwrap_or(usize::MAX);
            let grow = with(TRIBBLES).into_iter().take(room).collect();
            (grow, with(PERISHABLE))
        }
    };
    let before = cargo.clone();
    for good in grow {
        cargo
            .entry(good)
            .and_modify(|tons| *tons = tons.saturating_add(1));
    }
    for good in decay {
        if let Some(tons) = cargo.get_mut(&good) {
            *tons -= 1;
            if *tons == 0 {
                cargo.remove(&good);
            }
        }
    }
    *cargo != before
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{Gate, Test};
    use crate::rules::{Descriptor, Verdict};
    use PriceMarker::{High, Higher, Low, Lower, Medium};
    use nova_sim_base::fixture::Scripted;
    use nova_sim_catalog::fixture::{AllowAll, RefuseBits};

    // Price levels and prices.

    /// Port Kane: trade center; food high, industrial medium, medical
    /// high, luxury medium, metal medium, equipment low.
    const PORT_KANE: u32 = 0x4242_2157;

    #[test]
    fn each_commoditys_nibble_sets_its_price_level() {
        use PriceLevel::{High, Low, Medium};
        let levels: Vec<_> = (0..6).map(|n| price_level(PORT_KANE, n)).collect();
        assert_eq!(
            levels,
            [
                Some(High),
                Some(Medium),
                Some(High),
                Some(Medium),
                Some(Medium),
                Some(Low)
            ]
        );
        for n in 0..6 {
            let shift = 28 - 4 * u32::from(n);
            assert_eq!(price_level(1 << shift, n), Some(Low), "{n}");
            assert_eq!(price_level(2 << shift, n), Some(Medium), "{n}");
            assert_eq!(price_level(4 << shift, n), Some(High), "{n}");
            assert_eq!(price_level(PORT_KANE & !(0xF << shift), n), None, "{n}");
        }
    }

    #[test]
    fn a_nibble_with_several_bits_takes_its_lowest_level_and_8_alone_none() {
        use PriceLevel::{Low, Medium};
        assert_eq!(price_level(0x3 << 28, 0), Some(Low));
        assert_eq!(price_level(0x5 << 28, 0), Some(Low));
        assert_eq!(price_level(0x6 << 28, 0), Some(Medium));
        assert_eq!(price_level(0x7 << 28, 0), Some(Low));
        assert_eq!(price_level(0xE << 28, 0), Some(Medium));
        assert_eq!(price_level(0x8 << 28, 0), None);
    }

    #[test]
    fn there_are_six_commodities() {
        assert_eq!(COMMODITIES, 6);
        // The nibbles below equipment's are other flags.
        for n in [6, 7, 8, 255] {
            assert_eq!(price_level(u32::MAX, n), None, "{n}");
        }
    }

    #[test]
    fn by_the_standard_markup_low_is_80_percent_and_high_125_percent_truncated() {
        use PriceLevel::{High, Low, Medium};
        let table = [
            (75, [60, 75, 93]),
            (350, [280, 350, 437]),
            (750, [600, 750, 937]),
            (900, [720, 900, 1125]),
            (200, [160, 200, 250]),
            (550, [440, 550, 687]),
            (1200, [960, 1200, 1500]),
            (300, [240, 300, 375]),
        ];
        for (base, prices) in table {
            assert_eq!(
                [Low, Medium, High].map(|level| band_price(base, level, Markup::Standard)),
                prices,
                "{base}"
            );
        }
        assert_eq!(Markup::default(), Markup::Standard);
        assert_eq!(Markup::Standard.factor().to_bits(), 1.25_f64.to_bits());
    }

    /// `base`'s low, medium and high prices by `markup`.
    fn bands(base: i64, markup: Markup) -> [i64; 3] {
        use PriceLevel::{High, Low, Medium};
        [Low, Medium, High].map(|level| band_price(base, level, markup))
    }

    #[test]
    fn the_outlaw_markup_divides_and_multiplies_by_1_1_in_doubles() {
        assert_eq!(Markup::Outlaw.factor().to_bits(), 1.1_f64.to_bits());
        assert_eq!(
            bands(110, Markup::Outlaw),
            [99, 110, 121],
            "110 / 1.1 < 100"
        );
        assert_eq!(bands(100, Markup::Outlaw), [90, 100, 110]);
        assert_eq!(bands(33, Markup::Outlaw), [29, 33, 36]);
        assert_eq!(bands(75, Markup::Outlaw), [68, 75, 82]);
    }

    #[test]
    fn the_dominated_markup_divides_and_multiplies_by_1_5() {
        assert_eq!(Markup::Dominated.factor().to_bits(), 1.5_f64.to_bits());
        assert_eq!(bands(100, Markup::Dominated), [66, 100, 150]);
        assert_eq!(bands(75, Markup::Dominated), [50, 75, 112]);
    }

    #[test]
    fn a_negative_base_truncates_toward_zero() {
        assert_eq!(bands(-75, Markup::Standard), [-60, -75, -93]);
        assert_eq!(bands(-110, Markup::Outlaw), [-99, -110, -121]);
    }

    #[test]
    fn the_markup_is_standard_unless_the_record_with_a_government_is_negative() {
        for record in [i16::MIN, -1, 0, 1, i16::MAX] {
            assert_eq!(
                Markup::of(false, record, false),
                Markup::Standard,
                "{record}"
            );
        }
        assert_eq!(Markup::of(true, 0, false), Markup::Standard);
        assert_eq!(Markup::of(true, 1, false), Markup::Standard);
        assert_eq!(Markup::of(true, -1, false), Markup::Outlaw);
        assert_eq!(Markup::of(true, i16::MIN, false), Markup::Outlaw);
    }

    #[test]
    fn a_dominated_stellar_takes_the_dominated_markup_whatever_the_record() {
        for govt in [false, true] {
            for record in [i16::MIN, -1, 0, 1] {
                assert_eq!(
                    Markup::of(govt, record, true),
                    Markup::Dominated,
                    "{govt} {record}"
                );
            }
        }
    }

    // Cargo space.

    fn cargo(mod_val: i16, count: u16) -> OutfitMod {
        OutfitMod {
            mod_type: MORE_CARGO,
            mod_val,
            count,
        }
    }

    #[test]
    fn capacity_is_the_holds_plus_each_cargo_pod_carried() {
        assert_eq!(cargo_capacity(10, &[]), 10);
        assert_eq!(cargo_capacity(-10, &[]), 10, "the size of a negative");
        assert_eq!(cargo_capacity(0, &[cargo(5, 3)]), 15);
        assert_eq!(cargo_capacity(10, &[cargo(5, 3), cargo(2, 1)]), 27);
        assert_eq!(cargo_capacity(i16::MIN, &[]), 32_768);
        assert_eq!(MORE_CARGO, 2);
    }

    #[test]
    fn other_mods_add_no_space_and_space_never_goes_below_none() {
        let other = OutfitMod {
            mod_type: 3,
            mod_val: 100,
            count: 2,
        };
        assert_eq!(cargo_capacity(10, &[other]), 10);
        assert_eq!(cargo_capacity(10, &[cargo(-4, 2)]), 2);
        assert_eq!(cargo_capacity(10, &[cargo(-4, 3)]), 0);
        assert_eq!(
            cargo_capacity(i16::MAX, &[cargo(i16::MAX, u16::MAX); 8]),
            u32::try_from(32_767 + 8 * 32_767 * 65_535_i64).unwrap_or(u32::MAX)
        );
    }

    // The fleet's cargo space.

    /// An escort out of no bay, of `Holds` `holds` and `InherentAI`
    /// `inherent_ai`.
    fn escort(holds: i16, inherent_ai: i16) -> EscortHolds {
        EscortHolds {
            holds,
            inherent_ai,
            carried: false,
        }
    }

    #[test]
    fn a_trader_out_of_no_bay_adds_its_holds() {
        assert!(escort(15, 2).adds_holds());
        assert!(escort(15, -1).adds_holds());
        assert!(!escort(15, 3).adds_holds(), "a warship");
        let fighter = EscortHolds {
            carried: true,
            ..escort(15, 1)
        };
        assert!(!fighter.adds_holds());
    }

    #[test]
    fn the_fleets_holds_are_the_ships_with_no_escorts() {
        assert_eq!(fleet_holds(20, []), 20);
    }

    #[test]
    fn a_trader_escort_adds_its_holds() {
        assert_eq!(fleet_holds(20, [escort(15, 1), escort(30, 2)]), 65);
    }

    #[test]
    fn a_warship_or_interceptor_escort_adds_nothing() {
        assert_eq!(fleet_holds(20, [escort(15, 3), escort(30, 4)]), 20);
    }

    #[test]
    fn an_inherent_ai_below_1_counts_as_a_trader() {
        assert_eq!(fleet_holds(20, [escort(15, 0), escort(30, -1)]), 65);
    }

    #[test]
    fn a_launched_fighter_adds_nothing() {
        let fighter = EscortHolds {
            carried: true,
            ..escort(15, 1)
        };
        assert_eq!(fleet_holds(20, [fighter]), 20);
    }

    #[test]
    fn a_negative_escort_holds_adds_its_size() {
        // `_LoadObjectData` negates a negative `Holds` (@0x7a6a5-0x7a6d3).
        assert_eq!(fleet_holds(20, [escort(-5, 1)]), 25);
        assert_eq!(fleet_holds(20, [escort(-50, 1)]), 70);
    }

    #[test]
    fn the_fleets_holds_stop_at_32000() {
        assert_eq!(MAX_FLEET_HOLDS, 32_000);
        assert_eq!(fleet_holds(31_990, [escort(10, 1)]), 32_000);
        assert_eq!(fleet_holds(31_990, [escort(11, 1)]), 32_000);
        assert_eq!(fleet_holds(31_990, [escort(9, 1)]), 31_999);
        assert_eq!(fleet_holds(40_000, []), 32_000, "the ship's too");
        assert_eq!(fleet_holds(u32::MAX, [escort(1, 1)]), 32_000);
        assert_eq!(fleet_holds(0, [escort(i16::MAX, 1); 6]), 32_000);
    }

    #[test]
    fn the_escorts_tons_count_the_traders_out_of_no_bay_with_no_cap() {
        let fighter = EscortHolds {
            carried: true,
            ..escort(15, 1)
        };
        assert_eq!(escort_tons([]), 0);
        assert_eq!(
            escort_tons([escort(15, 1), escort(30, 3), fighter, escort(-5, 0)]),
            20,
            "the trader and the negative one's size, not the warship or the fighter"
        );
        assert_eq!(
            escort_tons([escort(i16::MAX, 2); 2]),
            2 * i64::from(i16::MAX),
            "past 32000"
        );
    }

    // Commodities.

    fn strings(names: &[&str], prices: &[&str]) -> CommodityStrings {
        CommodityStrings {
            names: names.iter().map(|&s| s.to_owned()).collect(),
            name_patches: Default::default(),
            base_prices: prices.iter().map(|&s| s.to_owned()).collect(),
            price_patches: Default::default(),
        }
    }

    /// [`stock`] with these `'STR '` 9300 + n patches.
    fn patched(patches: &[(usize, StringPatch)]) -> CommodityStrings {
        let mut strings = stock();
        for (n, patch) in patches {
            strings.price_patches[*n] = patch.clone();
        }
        strings
    }

    /// [`stock`] with these `'STR '` 9000 + n patches.
    fn renamed(patches: &[(usize, StringPatch)]) -> CommodityStrings {
        let mut strings = stock();
        for (n, patch) in patches {
            strings.name_patches[*n] = patch.clone();
        }
        strings
    }

    /// Each commodity's name, in order.
    fn names(strings: &CommodityStrings) -> Vec<String> {
        commodities(strings)
            .into_iter()
            .map(|(_, commodity)| commodity.name)
            .collect()
    }

    fn text(s: &str) -> StringPatch {
        StringPatch::Text(s.to_owned())
    }

    /// Each traded commodity's number and base price.
    fn prices(strings: &CommodityStrings) -> Vec<(u8, i64)> {
        commodities(strings)
            .into_iter()
            .map(|(n, commodity)| (n, commodity.base_price))
            .collect()
    }

    /// Stock `STR#` 4000's first strings and `STR#` 4004.
    fn stock() -> CommodityStrings {
        strings(
            &[
                "Food",
                "Industrial",
                "Medical Supplies",
                "Luxury Goods",
                "Metal",
                "Equipment",
                "*Passengers",
            ],
            &["75", "350", "750", "900", "200", "550"],
        )
    }

    fn commodity(name: &str, base_price: i64) -> Commodity {
        Commodity {
            name: name.to_owned(),
            base_price,
        }
    }

    #[test]
    fn string_to_num_reads_as_the_toolbox_does() {
        let table = [
            ("", 0),
            ("75", 75),
            ("-40", -40),
            ("+40", 40),
            (" 75 ", 750),
            ("lots", 13543),
            ("12a", 121),
            ("1.5", 245),
            ("--3", -133),
            ("-+3", -113),
            ("+-3", 133),
            ("3-", 43),
            ("-", 0),
            ("+", 0),
            ("70000", 4464),
            ("-70000", -4464),
            ("32768", -32768),
            ("65536", 0),
            ("99999999999", -6145),
            ("1\u{0}2", 102),
            ("\u{E9}5", 145),
        ];
        for (text, number) in table {
            assert_eq!(string_to_num(text), number, "{text:?}");
        }
        assert_eq!(string_to_num(&"9".repeat(255)), -1);
    }

    #[test]
    fn the_six_commodities_are_the_first_names_with_their_prices() {
        assert_eq!(
            commodities(&stock()),
            [
                (0, commodity("Food", 75)),
                (1, commodity("Industrial", 350)),
                (2, commodity("Medical Supplies", 750)),
                (3, commodity("Luxury Goods", 900)),
                (4, commodity("Metal", 200)),
                (5, commodity("Equipment", 550)),
            ]
        );
    }

    #[test]
    fn a_name_patch_replaces_its_commoditys_str_4000_string() {
        let strings = renamed(&[(0, text("Grain"))]);
        assert_eq!(
            names(&strings),
            [
                "Grain",
                "Industrial",
                "Medical Supplies",
                "Luxury Goods",
                "Metal",
                "Equipment"
            ]
        );
        assert_eq!(prices(&strings), prices(&stock()));
    }

    #[test]
    fn a_name_patch_wins_whatever_it_holds() {
        let found = names(&renamed(&[(1, text("")), (2, text(" 9 "))]));
        assert_eq!(found[1], "");
        assert_eq!(found[2], " 9 ");
    }

    #[test]
    fn an_empty_name_patch_names_its_commodity_empty_not_the_one_before() {
        let found = commodities(&renamed(&[(1, StringPatch::Empty)]));
        assert_eq!(found[1], (1, commodity("", 350)));
        assert_eq!(found[0], (0, commodity("Food", 75)));
    }

    #[test]
    fn an_unreadable_name_patch_names_its_commodity_empty() {
        let found = commodities(&renamed(&[(3, StringPatch::Unreadable)]));
        assert_eq!(found[3], (3, commodity("", 900)));
    }

    #[test]
    fn a_name_patch_names_a_commodity_str_4000_lacks() {
        let mut five = stock();
        five.names.truncate(5);
        five.name_patches[5] = text("Gear");
        assert_eq!(names(&five)[5], "Gear");

        let mut none = stock();
        none.names.clear();
        none.name_patches[1] = text("Ore");
        none.name_patches[4] = text("Steel");
        assert_eq!(names(&none), ["", "Ore", "", "", "Steel", ""]);
    }

    #[test]
    fn name_and_price_patches_are_separate() {
        let mut both = renamed(&[(0, text("Grain"))]);
        both.price_patches[0] = text("10");
        assert_eq!(commodities(&both)[0], (0, commodity("Grain", 10)));
        assert_eq!(
            commodities(&renamed(&[(0, text("Grain"))]))[0],
            (0, commodity("Grain", 75))
        );
        assert_eq!(
            commodities(&patched(&[(0, text("10"))]))[0],
            (0, commodity("Food", 10))
        );
    }

    #[test]
    fn each_name_slot_is_patched_on_its_own() {
        let found = names(&renamed(&[(0, text("Grain")), (5, text("Gear"))]));
        assert_eq!(
            found,
            [
                "Grain",
                "Industrial",
                "Medical Supplies",
                "Luxury Goods",
                "Metal",
                "Gear"
            ]
        );
    }

    #[test]
    fn every_commodity_is_traded_priced_as_string_to_num_reads_it() {
        let found = commodities(&strings(
            &["Food", "Industrial", "Medical", "Luxury", "Metal"],
            &["75", "lots", "", "-40", "200", "550", "99"],
        ));
        assert_eq!(
            found,
            [
                (0, commodity("Food", 75)),
                (1, commodity("Industrial", 13543)),
                (2, commodity("Medical", 0)),
                (3, commodity("Luxury", -40)),
                (4, commodity("Metal", 200)),
                (5, commodity("", 550)),
            ]
        );
    }

    #[test]
    fn odd_price_strings_are_priced_as_string_to_num_reads_them() {
        let found = prices(&strings(&[], &["", " 75 ", "lots", "70000", "200", "550"]));
        assert_eq!(
            found,
            [(0, 0), (1, 750), (2, 13543), (3, 4464), (4, 200), (5, 550)]
        );
    }

    #[test]
    fn with_no_strings_there_are_six_nameless_commodities_priced_at_0() {
        let found = commodities(&CommodityStrings::default());
        let none: Vec<_> = (0..6).map(|n| (n, commodity("", 0))).collect();
        assert_eq!(found, none);
    }

    #[test]
    fn a_commodity_str_4004_lacks_is_priced_at_0() {
        let mut short = stock();
        short.base_prices.truncate(3);
        assert_eq!(
            prices(&short),
            [(0, 75), (1, 350), (2, 750), (3, 0), (4, 0), (5, 0)]
        );
    }

    // Plug-in price patches.

    const STOCK_PRICES: [(u8, i64); 6] =
        [(0, 75), (1, 350), (2, 750), (3, 900), (4, 200), (5, 550)];

    #[test]
    fn a_price_patch_replaces_its_commoditys_str_4004_string() {
        assert_eq!(
            commodities(&patched(&[(1, text("999"))])),
            [
                (0, commodity("Food", 75)),
                (1, commodity("Industrial", 999)),
                (2, commodity("Medical Supplies", 750)),
                (3, commodity("Luxury Goods", 900)),
                (4, commodity("Metal", 200)),
                (5, commodity("Equipment", 550)),
            ]
        );
    }

    #[test]
    fn a_price_patch_wins_whatever_it_holds() {
        let found = prices(&patched(&[
            (1, text("lots")),
            (2, text("")),
            (3, text(" 120 ")),
        ]));
        assert_eq!(
            found,
            [(0, 75), (1, 13543), (2, 0), (3, 1200), (4, 200), (5, 550)]
        );
    }

    #[test]
    fn an_unreadable_price_patch_prices_its_commodity_at_0() {
        let found = prices(&patched(&[
            (0, StringPatch::Unreadable),
            (4, StringPatch::Unreadable),
        ]));
        assert_eq!(
            found,
            [(0, 0), (1, 350), (2, 750), (3, 900), (4, 0), (5, 550)]
        );
    }

    #[test]
    fn a_price_patch_prices_a_commodity_str_4004_lacks() {
        let mut five = patched(&[(5, text("550"))]);
        five.base_prices.truncate(5);
        assert_eq!(prices(&five), STOCK_PRICES);

        let mut none = patched(&[(0, text("10")), (4, text("40"))]);
        none.base_prices.clear();
        assert_eq!(
            prices(&none),
            [(0, 10), (1, 0), (2, 0), (3, 0), (4, 40), (5, 0)]
        );
    }

    #[test]
    fn each_slot_is_patched_on_its_own() {
        let found = prices(&patched(&[(0, text("1")), (5, text("6"))]));
        assert_eq!(
            found,
            [(0, 1), (1, 350), (2, 750), (3, 900), (4, 200), (5, 6)]
        );
    }

    #[test]
    fn an_empty_price_patch_takes_the_string_the_slot_before_took() {
        let after_stock = prices(&patched(&[(2, StringPatch::Empty)]));
        assert_eq!(
            after_stock,
            [(0, 75), (1, 350), (2, 350), (3, 900), (4, 200), (5, 550)]
        );

        let after_patch = prices(&patched(&[(3, text("999")), (4, StringPatch::Empty)]));
        assert_eq!(
            after_patch,
            [(0, 75), (1, 350), (2, 750), (3, 999), (4, 999), (5, 550)]
        );

        let chained = prices(&patched(&[
            (1, StringPatch::Empty),
            (2, StringPatch::Empty),
        ]));
        assert_eq!(
            chained,
            [(0, 75), (1, 75), (2, 75), (3, 900), (4, 200), (5, 550)]
        );

        let mut short = patched(&[(5, StringPatch::Empty)]);
        short.base_prices.truncate(4);
        assert_eq!(
            prices(&short),
            [(0, 75), (1, 350), (2, 750), (3, 900), (4, 0), (5, 0)],
            "the slot before has no string"
        );
    }

    #[test]
    fn an_empty_price_patch_after_an_unreadable_one_or_first_is_priced_at_0() {
        let after_unreadable = prices(&patched(&[
            (1, StringPatch::Unreadable),
            (2, StringPatch::Empty),
        ]));
        assert_eq!(
            after_unreadable,
            [(0, 75), (1, 0), (2, 0), (3, 900), (4, 200), (5, 550)]
        );

        let first = prices(&patched(&[(0, StringPatch::Empty)]));
        assert_eq!(
            first,
            [(0, 0), (1, 350), (2, 750), (3, 900), (4, 200), (5, 550)]
        );
    }

    // The exchange.

    const EARTH: StellarId = StellarId(140);
    const MARS: StellarId = StellarId(141);

    /// A `jünk` 0 that no stellar lists, at no price.
    fn unlisted() -> JunkRecord {
        JunkRecord {
            id: JunkId(0),
            name: String::new(),
            abbrev: String::new(),
            base_price: 0,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        }
    }

    /// Opals, sold at Earth and bought at Mars; water, sold at Earth.
    fn junk() -> Vec<JunkRecord> {
        vec![
            JunkRecord {
                id: JunkId(146),
                name: "Opals".to_owned(),
                abbrev: "Opl".to_owned(),
                base_price: 1200,
                sold_at: vec![StellarId(189), EARTH],
                bought_at: vec![MARS],
                ..unlisted()
            },
            JunkRecord {
                id: JunkId(134),
                name: "Water".to_owned(),
                base_price: 300,
                sold_at: vec![EARTH],
                ..unlisted()
            },
        ]
    }

    /// A food surplus at Earth (-15, 30 days, 35 %), a drought at Earth
    /// (+15 food, 25 days, 50 %), a glut of industrial goods at Mars (-70,
    /// 15 days, 40 %).
    fn disasters() -> Vec<DisasterRecord> {
        let event =
            |id, name: &str, stellar, commodity, price_delta, duration, freq| DisasterRecord {
                id: DisasterId(id),
                name: name.to_owned(),
                stellar,
                commodity,
                price_delta,
                duration,
                freq,
                activate_on: Test::default(),
            };
        vec![
            event(128, "An enormous food surplus", 140, 0, -15, 30, 35),
            event(129, "A glut on the market", 141, 1, -70, 15, 40),
            event(130, "A minor drought", 140, 0, 15, 25, 50),
        ]
    }

    fn goods() -> Goods {
        Goods::new(&stock(), junk(), disasters())
    }

    #[test]
    fn a_junks_abbrev_is_read_by_id() {
        let goods = goods();
        assert_eq!(goods.abbrev(JunkId(146)), Some("Opl"));
        assert_eq!(goods.abbrev(JunkId(134)), Some(""));
        assert_eq!(goods.abbrev(JunkId(999)), None);
    }

    #[test]
    fn a_goods_name_is_read_by_number_or_id() {
        let goods = goods();
        assert_eq!(goods.name(Good::Commodity(0)), Some("Food"));
        assert_eq!(goods.name(Good::Commodity(5)), Some("Equipment"));
        assert_eq!(goods.name(Good::Commodity(6)), None);
        assert_eq!(goods.name(Good::Junk(JunkId(146))), Some("Opals"));
        assert_eq!(goods.name(Good::Junk(JunkId(134))), Some("Water"));
        assert_eq!(goods.name(Good::Junk(JunkId(999))), None);
    }

    #[test]
    fn the_junk_records_are_kept_as_given() {
        assert_eq!(goods().junk(), junk());
    }

    /// A catalog holding the stock commodity strings, [`junk`] and
    /// [`disasters`], and these stellars' `Flags`: the [`PilotCatalog`]
    /// port's fake, answering only what [`Goods::read`] asks.
    struct Trading(Vec<(StellarId, u32)>);

    impl PilotCatalog for Trading {
        fn first_character(
            &self,
        ) -> Result<crate::catalog::CharacterStart, crate::catalog::StartError> {
            unreachable!("Goods::read reads no character")
        }
        fn ship_fields(
            &self,
            _: crate::catalog::ShipId,
        ) -> Result<crate::handling::ShipFields, String> {
            unreachable!("Goods::read reads no ship")
        }
        fn default_outfits(
            &self,
            _: crate::catalog::ShipId,
        ) -> Vec<(crate::catalog::OutfitId, u16)> {
            unreachable!("Goods::read reads no ship")
        }
        fn outfits(&self) -> Vec<crate::catalog::OutfitRecord> {
            unreachable!("Goods::read reads no outfit")
        }
        fn ships(&self) -> Vec<crate::catalog::ShipRecord> {
            unreachable!("Goods::read reads no ship")
        }
        fn system_exists(&self, _: crate::catalog::SystemId) -> bool {
            unreachable!("Goods::read reads no system")
        }
        fn landing_sites(&self, _: crate::catalog::SystemId) -> Vec<crate::catalog::LandingSite> {
            unreachable!("Goods::read reads no system")
        }
        fn star_map(&self) -> Vec<crate::catalog::StarSystem> {
            unreachable!("Goods::read reads no system")
        }
        fn commodity_strings(&self) -> CommodityStrings {
            stock()
        }
        fn string_patch(&self, _: i16) -> StringPatch {
            unreachable!("Goods::read reads its strings whole")
        }
        fn buoy_strings(&self) -> Vec<String> {
            unreachable!("Goods::read reads no buoy")
        }
        fn junk(&self) -> Vec<JunkRecord> {
            junk()
        }
        fn disasters(&self) -> Vec<DisasterRecord> {
            disasters()
        }
        fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
            self.0.clone()
        }
        fn stellar_weapons(&self) -> Vec<(StellarId, crate::catalog::WeaponId)> {
            Vec::new()
        }
        fn date_affixes(&self) -> crate::catalog::DateAffixes {
            unreachable!("Goods::read reads no date")
        }
        fn gate_sites(&self) -> Vec<crate::catalog::GateSite> {
            unreachable!("Goods::read reads no gate")
        }
        fn stellar_lives(&self) -> Vec<crate::catalog::StellarLife> {
            unreachable!("Goods::read reads no stellar's life")
        }
        fn stellar_scripts(&self) -> Vec<crate::catalog::StellarScripts> {
            unreachable!("Goods::read reads no stellar's scripts")
        }
        fn fallback_frame_size(&self) -> Option<(u32, u32)> {
            unreachable!("Goods::read reads no graphic")
        }
        fn missions(&self) -> Vec<crate::catalog::MissionRecord> {
            unreachable!("Goods::read reads no mission")
        }
        fn mission_texts(&self) -> Vec<crate::catalog::MissionTexts> {
            unreachable!("Goods::read reads no mission")
        }
        fn listed_stellars(&self) -> Vec<crate::catalog::ListedStellar> {
            unreachable!("Goods::read reads no listing")
        }
        fn system_visibility(&self) -> Vec<crate::catalog::SystemVisibility> {
            unreachable!("Goods::read reads no system")
        }
        fn place_names(&self) -> crate::catalog::PlaceNames {
            unreachable!("Goods::read reads no name")
        }
        fn stellar_marks(&self) -> Vec<crate::catalog::StellarMark> {
            unreachable!("Goods::read reads no mark")
        }
        fn crons(&self) -> Vec<crate::catalog::CronRecord> {
            unreachable!("Goods::read reads no crön")
        }
        fn ranks(&self) -> Vec<crate::catalog::RankRecord> {
            unreachable!("Goods::read reads no rank")
        }
        fn nebulae(&self) -> Vec<crate::catalog::NebulaRecord> {
            unreachable!("Goods::read reads no nebula")
        }
    }

    #[test]
    fn goods_are_read_from_the_catalogs_strings_junk_events_and_stellars() {
        let stellars = vec![
            (StellarId(141), 0),
            (StellarId(140), StellarFlags::UNINHABITED),
            (StellarId(128), 0x01),
        ];
        assert_eq!(
            Goods::read(&Trading(stellars.clone())),
            Goods::new(&stock(), junk(), disasters()).with_stellars(&stellars)
        );
    }

    #[test]
    fn goods_keep_every_stellars_flags_listed_or_not() {
        let goods = Goods::read(&Trading(vec![
            (StellarId(141), 0),
            (StellarId(140), StellarFlags::UNINHABITED),
            (StellarId(128), 0x11),
        ]));
        assert_eq!(goods.stellar_flags(StellarId(128)), 0x11);
        assert_eq!(
            goods.stellar_flags(StellarId(140)),
            StellarFlags::UNINHABITED
        );
        assert_eq!(goods.stellar_flags(StellarId(141)), 0);
        assert_eq!(goods.stellar_flags(StellarId(999)), 0);
    }

    /// A pilot at the exchange: the [`Trader`] port's fake.
    #[derive(Debug, Default)]
    struct FakePilot {
        cash: i64,
        cargo: BTreeMap<Good, u32>,
        events: BTreeMap<DisasterId, ActiveEvent>,
        /// The tons of mission cargo aboard: they take space, but no good
        /// holds them.
        mission_tons: i64,
        /// Each mission slot's cargo, as the summary counts it.
        missions: Vec<MissionLoad>,
    }

    impl Trader for FakePilot {
        fn cash(&self) -> i64 {
            self.cash
        }

        fn held(&self, good: Good) -> u32 {
            self.cargo.get(&good).copied().unwrap_or(0)
        }

        fn total_cargo(&self) -> i64 {
            self.cargo
                .values()
                .map(|&tons| i64::from(tons))
                .sum::<i64>()
                + self.mission_tons
        }

        fn events_under_way(&self) -> &BTreeMap<DisasterId, ActiveEvent> {
            &self.events
        }

        fn mission_loads(&self) -> Vec<MissionLoad> {
            self.missions.clone()
        }

        fn cash_mut(&mut self) -> &mut i64 {
            &mut self.cash
        }

        fn cargo_mut(&mut self) -> &mut BTreeMap<Good, u32> {
            &mut self.cargo
        }
    }

    /// A pilot with `cash` credits, holding nothing, with no mission cargo
    /// aboard and no events under way.
    fn pilot(cash: i64) -> FakePilot {
        FakePilot {
            cash,
            ..FakePilot::default()
        }
    }

    /// A row of `good`, traded both ways, with none held: a commodity
    /// unmarked in its own slot ([`marked`] marks it), a `jünk` in its
    /// `SoldAt` slot, marked "Low" ([`bought`] moves it to `BoughtAt`).
    fn row(good: Good, name: &str, price: i64) -> MarketRow {
        let (slot, marker) = match good {
            Good::Commodity(n) => (n, None),
            Good::Junk(_) => (JunkSide::SoldAt.slot(), Some(PriceMarker::Low)),
        };
        MarketRow {
            good,
            name: name.to_owned(),
            price,
            held: 0,
            sold_here: true,
            bought_here: true,
            slot,
            marker,
        }
    }

    /// `row` marked with `marker`.
    fn marked(marker: PriceMarker, row: MarketRow) -> MarketRow {
        MarketRow {
            marker: Some(marker),
            ..row
        }
    }

    /// The `jünk` `row` on its `BoughtAt` side: slot 6, marked "High".
    fn bought(row: MarketRow) -> MarketRow {
        MarketRow {
            slot: JunkSide::BoughtAt.slot(),
            ..marked(PriceMarker::High, row)
        }
    }

    const TRADE: u32 = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER;

    /// The price rules with events priced by `event_price`, and `jünk`
    /// by the engine.
    fn rules(event_price: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::EventPrice, event_price)
    }

    #[test]
    fn without_a_trade_center_there_is_no_exchange() {
        let flags = PORT_KANE & !StellarFlags::TRADE_CENTER;
        assert_eq!(
            market(&Exchange {
                capacity: 10,
                rules: rules(RuleSource::Engine),
                ..exchange(&goods(), EARTH, flags, &pilot(0))
            }),
            None
        );
        assert_eq!(
            market(&Exchange {
                capacity: 10,
                rules: rules(RuleSource::Engine),
                ..exchange(&goods(), MARS, 0, &pilot(0))
            }),
            None
        );
    }

    /// The exchange's facts, as [`market`] takes them: `goods` at
    /// `stellar` with these `flags`, for `pilot`, with no cargo space, the
    /// fresh pilot's tests, the default rule set and the standard markup.
    fn exchange<'a>(
        goods: &'a Goods,
        stellar: StellarId,
        flags: u32,
        pilot: &'a FakePilot,
    ) -> Exchange<'a> {
        Exchange {
            goods,
            stellar,
            flags,
            pilot,
            capacity: 0,
            ship_capacity: 0,
            gate: Gate::FRESH,
            rules: Rulebook::default(),
            markup: Markup::Standard,
        }
    }

    #[test]
    fn a_market_is_built_from_one_exchange_context() {
        let goods = goods();
        let mut holding = pilot(500);
        holding.cargo = BTreeMap::from([(Good::Commodity(0), 3)]);
        let rules = Rulebook::default().with_override(RuleKey::TradeLot, RuleSource::Bible);
        let found = market(&Exchange {
            capacity: 10,
            rules,
            markup: Markup::Dominated,
            ..exchange(&goods, StellarId(137), PORT_KANE, &holding)
        })
        .expect("trades");
        let rows: Vec<_> = found
            .rows
            .iter()
            .map(|row| (row.good, row.price, row.held))
            .collect();
        assert_eq!(
            rows,
            [
                (Good::Commodity(0), 112, 3),
                (Good::Commodity(1), 350, 0),
                (Good::Commodity(2), 1125, 0),
                (Good::Commodity(3), 900, 0),
                (Good::Commodity(4), 200, 0),
                (Good::Commodity(5), 366, 0),
            ],
            "Port Kane's levels by 1.5"
        );
        assert_eq!((found.cash, found.capacity, found.free), (500, 10, 7));
        assert_eq!(found.rules, rules);
    }

    #[test]
    fn a_market_carries_the_rule_set_it_was_built_by() {
        let goods = goods();
        let pilot = pilot(500);
        for key in [
            RuleKey::TradeLot,
            RuleKey::TradeQuotient,
            RuleKey::TradeCount,
            RuleKey::TradeDebt,
        ] {
            for source in RuleSource::ALL {
                let rules = Rulebook::default().with_override(key, source);
                let found = market(&Exchange {
                    rules,
                    ..exchange(&goods, StellarId(137), PORT_KANE, &pilot)
                })
                .expect("trades");
                assert_eq!(found.rules, rules, "{key:?} {source:?}");
            }
        }
    }

    #[test]
    fn the_exchange_trades_a_click_by_its_rule() {
        for trade_lot in RuleSource::ALL {
            let found = market(&Exchange {
                capacity: 10,
                rules: Rulebook::default().with_override(RuleKey::TradeLot, trade_lot),
                ..exchange(&goods(), StellarId(137), PORT_KANE, &pilot(500))
            })
            .expect("trades");
            assert_eq!(found.rules.source_for(RuleKey::TradeLot), trade_lot);
        }
    }

    #[test]
    fn the_exchange_divides_by_its_quotient_rule() {
        for trade_quotient in RuleSource::ALL {
            let found = market(&Exchange {
                capacity: 10,
                rules: Rulebook::default().with_override(RuleKey::TradeQuotient, trade_quotient),
                ..exchange(&goods(), StellarId(137), PORT_KANE, &pilot(500))
            })
            .expect("trades");
            assert_eq!(
                found.rules.source_for(RuleKey::TradeQuotient),
                trade_quotient
            );
        }
    }

    #[test]
    fn the_exchange_carries_its_count_rule() {
        for trade_count in RuleSource::ALL {
            let found = market(&Exchange {
                capacity: 10,
                rules: Rulebook::default().with_override(RuleKey::TradeCount, trade_count),
                ..exchange(&goods(), StellarId(137), PORT_KANE, &pilot(500))
            })
            .expect("trades");
            assert_eq!(found.rules.source_for(RuleKey::TradeCount), trade_count);
        }
    }

    #[test]
    fn the_exchange_carries_its_debt_rule() {
        for trade_debt in RuleSource::ALL {
            let found = market(&Exchange {
                capacity: 10,
                rules: Rulebook::default().with_override(RuleKey::TradeDebt, trade_debt),
                ..exchange(&goods(), StellarId(137), PORT_KANE, &pilot(500))
            })
            .expect("trades");
            assert_eq!(found.rules.source_for(RuleKey::TradeDebt), trade_debt);
        }
    }

    #[test]
    fn the_exchange_lists_the_commodities_traded_at_their_levels() {
        let flags = PORT_KANE;
        let found = market(&Exchange {
            capacity: 10,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), StellarId(137), flags, &pilot(500))
        })
        .expect("trades");
        assert_eq!(
            found,
            Market {
                rows: vec![
                    marked(High, row(Good::Commodity(0), "Food", 93)),
                    marked(Medium, row(Good::Commodity(1), "Industrial", 350)),
                    marked(High, row(Good::Commodity(2), "Medical Supplies", 937)),
                    marked(Medium, row(Good::Commodity(3), "Luxury Goods", 900)),
                    marked(Medium, row(Good::Commodity(4), "Metal", 200)),
                    marked(Low, row(Good::Commodity(5), "Equipment", 440)),
                ],
                events: Vec::new(),
                cash: 500,
                capacity: 10,
                free: 10,
                summary: cargo_summary(&[], [], 0, 10, 0),
                rules: rules(RuleSource::Engine),
                ..Market::default()
            }
        );
        let some = TRADE | (1 << 24) | (4 << 12);
        let found = market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), StellarId(137), some, &pilot(0))
        })
        .expect("trades");
        assert_eq!(
            found.rows,
            [
                marked(Low, row(Good::Commodity(1), "Industrial", 280)),
                marked(High, row(Good::Commodity(4), "Metal", 250)),
            ]
        );
        let none = market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), StellarId(137), TRADE, &pilot(0))
        })
        .expect("trades");
        assert_eq!(none.rows, [], "a trade center that trades nothing");
    }

    #[test]
    fn junk_is_sold_low_where_it_is_sold_and_bought_high_where_it_is_bought() {
        let at_earth = market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), EARTH, TRADE, &pilot(0))
        })
        .expect("trades");
        assert_eq!(
            at_earth.rows,
            [
                row(Good::Junk(JunkId(134)), "Water", 240),
                row(Good::Junk(JunkId(146)), "Opals", 960),
            ],
            "by ID, each traded both ways by the engine"
        );
        let at_mars = market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), MARS, TRADE, &pilot(0))
        })
        .expect("trades");
        assert_eq!(
            at_mars.rows,
            [bought(row(Good::Junk(JunkId(146)), "Opals", 1500))]
        );
        let elsewhere = market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), StellarId(150), TRADE, &pilot(0))
        })
        .expect("trades");
        assert_eq!(elsewhere.rows, []);
    }

    #[test]
    fn rows_sit_in_the_originals_slots() {
        let food_and_metal = TRADE | (1 << 28) | (4 << 12);
        let slots = |stellar| {
            market(&exchange(&goods(), stellar, food_and_metal, &pilot(0)))
                .expect("trades")
                .rows
                .iter()
                .map(|row| (row.good, row.slot))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            slots(MARS),
            [
                (Good::Commodity(0), 0),
                (Good::Commodity(4), 4),
                (Good::Junk(JunkId(146)), 6),
            ],
            "Mars buys opals"
        );
        assert_eq!(
            slots(EARTH),
            [
                (Good::Commodity(0), 0),
                (Good::Commodity(4), 4),
                (Good::Junk(JunkId(134)), 7),
                (Good::Junk(JunkId(146)), 7),
            ],
            "Earth sells water and opals"
        );
    }

    #[test]
    fn each_row_carries_its_marker() {
        // Food low, industrial medium, metal high at Earth.
        let flags = TRADE | (1 << 28) | (2 << 24) | (4 << 12);
        let marks = |events: &[(usize, u16)]| {
            let mut trader = pilot(0);
            trader.events = events
                .iter()
                .map(|&(n, days)| at(&disasters()[n], days))
                .collect();
            market(&exchange(&goods(), EARTH, flags, &trader))
                .expect("trades")
                .rows
                .iter()
                .map(|row| (row.good, row.marker))
                .collect::<Vec<_>>()
        };
        let with = |food| {
            vec![
                (Good::Commodity(0), food),
                (Good::Commodity(1), Some(Medium)),
                (Good::Commodity(4), Some(High)),
                (Good::Junk(JunkId(134)), Some(Low)),
                (Good::Junk(JunkId(146)), Some(Low)),
            ]
        };
        assert_eq!(marks(&[]), with(Some(Low)), "by level");
        assert_eq!(marks(&[(0, 5)]), with(Some(Lower)), "the surplus");
        assert_eq!(marks(&[(2, 5)]), with(Some(Higher)), "the drought");
        assert_eq!(
            marks(&[(0, 5), (2, 5)]),
            with(Some(Lower)),
            "the first by ID"
        );
        assert_eq!(marks(&[(0, 0), (2, 5)]), with(Some(Higher)), "no days left");
        assert_eq!(marks(&[(1, 5)]), with(Some(Low)), "at Mars");
        let mut flat = disasters();
        flat[0].price_delta = 0;
        let mut trader = pilot(0);
        trader.events = BTreeMap::from([at(&flat[0], 5)]);
        let goods = Goods::new(&stock(), junk(), flat);
        let found = market(&exchange(&goods, EARTH, flags, &trader)).expect("trades");
        assert_eq!(found.rows[0].marker, None, "a delta of 0 marks nothing");
    }

    #[test]
    fn the_exchange_carries_the_ships_own_holds() {
        let found = market(&Exchange {
            capacity: 30,
            ship_capacity: 20,
            ..exchange(&goods(), EARTH, TRADE, &pilot(0))
        })
        .expect("trades");
        assert_eq!((found.ship_capacity, found.capacity), (20, 30));
        assert_eq!(found.held_in(), HeldIn::Fleet);
        assert_eq!(
            (found.summary.ship_holds, found.summary.fleet_holds),
            (20, 30)
        );
        let alone = market(&Exchange {
            capacity: 20,
            ship_capacity: 20,
            ..exchange(&goods(), EARTH, TRADE, &pilot(0))
        })
        .expect("trades");
        assert_eq!(alone.held_in(), HeldIn::Hold);
        assert_eq!(alone.words, TradeWords::STOCK);
    }

    #[test]
    fn the_summary_counts_the_mission_cargo_and_the_junk_not_listed_here() {
        let mut trader = pilot(0);
        trader.missions = vec![
            MissionLoad {
                aboard: true,
                cargo_type: Some(2),
                tons: 4,
            },
            MissionLoad {
                aboard: true,
                cargo_type: None,
                tons: 3,
            },
        ];
        trader.mission_tons = 7;
        let spice = JunkRecord {
            id: JunkId(150),
            name: "Spice".to_owned(),
            ..unlisted()
        };
        let mut all = junk();
        all.push(spice);
        trader.cargo = BTreeMap::from([
            (Good::Junk(JunkId(150)), 2),
            (Good::Junk(JunkId(146)), 5),
            (Good::Commodity(0), 1),
        ]);
        let goods = Goods::new(&stock(), all, disasters());
        let found = market(&Exchange {
            capacity: 20,
            ship_capacity: 20,
            ..exchange(&goods, MARS, TRADE, &trader)
        })
        .expect("trades");
        // Water, which Mars does not list, none held; spice, 2 held.
        let expected = cargo_summary(&trader.missions, [0, 2], 20, 20, 15);
        assert_eq!(found.summary, expected);
        assert_eq!(
            (found.summary.mission_kinds, found.summary.misc_kinds),
            (1, 1)
        );
    }

    #[test]
    fn a_junk_named_here_is_not_other_cargo_even_at_price_zero() {
        let free = JunkRecord {
            id: JunkId(150),
            name: "Free".to_owned(),
            base_price: 0,
            sold_at: vec![EARTH],
            ..unlisted()
        };
        let mut trader = pilot(0);
        trader.cargo = BTreeMap::from([(Good::Junk(JunkId(150)), 3)]);
        let goods = Goods::new(&stock(), vec![free.clone()], Vec::new());
        let found = market(&exchange(&goods, EARTH, TRADE, &trader)).expect("trades");
        assert_eq!(found.rows, [], "priced 0, so not listed by the engine");
        assert_eq!(found.summary.misc_kinds, 0, "but named by its side");
        let elsewhere_only = Goods::new(
            &stock(),
            vec![JunkRecord {
                sold_at: vec![MARS],
                ..free
            }],
            Vec::new(),
        );
        let elsewhere = market(&exchange(&elsewhere_only, EARTH, TRADE, &trader)).expect("trades");
        assert_eq!(
            (elsewhere.summary.misc_kinds, elsewhere.summary.misc_tons),
            (1, 3)
        );
    }

    /// The price rules with `jünk` traded by `junk_trade`, and the rest
    /// by the engine.
    fn trading_by(junk_trade: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::JunkTrade, junk_trade)
    }

    #[test]
    fn by_the_bible_junk_is_only_bought_where_it_is_sold_and_sold_where_it_is_bought() {
        let at = |stellar| {
            let rules = trading_by(RuleSource::Bible);
            market(&Exchange {
                rules,
                ..exchange(&goods(), stellar, TRADE, &pilot(0))
            })
            .expect("trades")
            .rows
        };
        assert_eq!(
            at(EARTH),
            [
                MarketRow {
                    bought_here: false,
                    ..row(Good::Junk(JunkId(134)), "Water", 240)
                },
                MarketRow {
                    bought_here: false,
                    ..row(Good::Junk(JunkId(146)), "Opals", 960)
                },
            ]
        );
        assert_eq!(
            at(MARS),
            [MarketRow {
                sold_here: false,
                ..bought(row(Good::Junk(JunkId(146)), "Opals", 1500))
            }]
        );
    }

    #[test]
    fn commodities_trade_both_ways_by_either_reading() {
        for source in RuleSource::ALL {
            let found = market(&Exchange {
                rules: trading_by(source),
                ..exchange(&goods(), StellarId(137), PORT_KANE, &pilot(0))
            })
            .expect("trades");
            assert_eq!(found.rows.len(), 6, "{source:?}");
            for row in &found.rows {
                assert!(row.sold_here && row.bought_here, "{source:?} {row:?}");
            }
        }
    }

    const TWO_WAY: Good = Good::Junk(JunkId(200));

    /// The exchange at Earth, by `markup`, of a `jünk` 200 of `base`
    /// price that Earth both sells and buys, for `pilot` with `capacity`
    /// tons of space, traded as `junk_trade` says.
    fn both_ways_by(
        junk_trade: RuleSource,
        base: i16,
        markup: Markup,
        pilot: &FakePilot,
        capacity: u32,
    ) -> Market {
        let both = JunkRecord {
            id: JunkId(200),
            name: "Both".to_owned(),
            base_price: base,
            sold_at: vec![EARTH],
            bought_at: vec![EARTH],
            ..unlisted()
        };
        let goods = Goods::new(&CommodityStrings::default(), vec![both], Vec::new());
        market(&Exchange {
            capacity,
            rules: trading_by(junk_trade),
            markup,
            ..exchange(&goods, EARTH, TRADE, pilot)
        })
        .expect("trades")
    }

    /// [`both_ways_by`] the engine.
    fn both_ways(base: i16, markup: Markup, pilot: &FakePilot, capacity: u32) -> Market {
        both_ways_by(RuleSource::Engine, base, markup, pilot, capacity)
    }

    #[test]
    fn junk_listed_both_ways_is_listed_twice_bought_high_then_sold_low() {
        let found = both_ways(400, Markup::Standard, &pilot(0), 0);
        assert_eq!(
            found.rows,
            [bought(row(TWO_WAY, "Both", 500)), row(TWO_WAY, "Both", 320)],
            "400 × 1.25, then 400 / 1.25, each traded both ways"
        );
        let bible = both_ways_by(RuleSource::Bible, 400, Markup::Standard, &pilot(0), 0);
        assert_eq!(
            bible.rows,
            [
                MarketRow {
                    sold_here: false,
                    ..bought(row(TWO_WAY, "Both", 500))
                },
                MarketRow {
                    bought_here: false,
                    ..row(TWO_WAY, "Both", 320)
                },
            ],
            "by the Bible, one way each"
        );
        let outlaw = both_ways(110, Markup::Outlaw, &pilot(0), 0);
        let prices: Vec<_> = outlaw.rows.iter().map(|row| row.price).collect();
        assert_eq!(prices, [121, 99], "110 × 1.1, then 110 / 1.1, truncated");
    }

    #[test]
    fn by_the_engine_junk_listed_both_ways_is_bought_and_sold_on_each_row() {
        let on = |row| {
            move |market: &Market, direction: Direction, lot: Lot| {
                market.tons(order(row, TWO_WAY, direction, lot))
            }
        };
        let (high, low) = (on(0), on(1));
        let empty = both_ways(400, Markup::Standard, &pilot(1000), 10);
        assert_eq!(low(&empty, Direction::Buy, Lot::Max), Ok(3), "1000 at 320");
        assert_eq!(high(&empty, Direction::Buy, Lot::Max), Ok(2), "1000 at 500");
        assert_eq!(
            high(&empty, Direction::Buy, Lot::Click),
            Ok(2),
            "1000 at 500"
        );
        for row in [0, 1] {
            assert!(empty.row_allows(row, Direction::Buy), "{row}");
            assert!(!empty.row_allows(row, Direction::Sell), "{row} none held");
            assert_eq!(
                on(row)(&empty, Direction::Sell, Lot::Click),
                Err(TradeRefusal::NoneHeld)
            );
        }
        let mut holding = pilot(499);
        holding.cargo = BTreeMap::from([(TWO_WAY, 3)]);
        let holding = both_ways(400, Markup::Standard, &holding, 10);
        let held: Vec<_> = holding.rows.iter().map(|row| row.held).collect();
        assert_eq!(held, [3, 3], "both rows");
        assert_eq!(high(&holding, Direction::Sell, Lot::Max), Ok(3));
        assert_eq!(low(&holding, Direction::Sell, Lot::Max), Ok(3));
        assert_eq!(low(&holding, Direction::Buy, Lot::Max), Ok(1), "499 at 320");
        assert_eq!(
            high(&holding, Direction::Buy, Lot::Click),
            Err(TradeRefusal::CannotAfford),
            "499 at 500"
        );
        for row in [0, 1] {
            assert!(holding.row_allows(row, Direction::Sell), "{row}");
        }
        assert!(!holding.row_allows(0, Direction::Buy), "the high row: cash");
        assert!(holding.row_allows(1, Direction::Buy), "the low row");
        let full = both_ways(400, Markup::Standard, &pilot(1000), 0);
        for row in [0, 1] {
            assert_eq!(
                on(row)(&full, Direction::Buy, Lot::Click),
                Err(TradeRefusal::NoSpace)
            );
        }
    }

    #[test]
    fn by_the_bible_junk_listed_both_ways_is_bought_on_its_low_row_and_sold_on_its_high_row() {
        let mut holding = pilot(640);
        holding.cargo = BTreeMap::from([(TWO_WAY, 3)]);
        let market = both_ways_by(RuleSource::Bible, 400, Markup::Standard, &holding, 10);
        let tons = |row, direction| market.tons(order(row, TWO_WAY, direction, Lot::Max));
        assert_eq!(tons(1, Direction::Buy), Ok(2), "640 at 320");
        assert_eq!(tons(0, Direction::Sell), Ok(3));
        assert_eq!(tons(0, Direction::Buy), Err(TradeRefusal::NotTraded));
        assert_eq!(tons(1, Direction::Sell), Err(TradeRefusal::NotTraded));
        assert!(!market.row_allows(0, Direction::Buy), "the high row");
        assert!(market.row_allows(0, Direction::Sell));
        assert!(market.row_allows(1, Direction::Buy), "the low row");
        assert!(!market.row_allows(1, Direction::Sell));
        assert!(!market.row_allows(2, Direction::Buy), "no row");
        assert!(!market.row_allows(2, Direction::Sell));
    }

    #[test]
    fn an_order_on_no_row_or_another_goods_row_is_not_traded() {
        let mut holding = pilot(1000);
        holding.cargo = BTreeMap::from([(TWO_WAY, 3), (FOOD, 3)]);
        let market = both_ways(400, Markup::Standard, &holding, 10);
        for direction in [Direction::Buy, Direction::Sell] {
            for (row, good) in [(2, TWO_WAY), (usize::MAX, TWO_WAY), (0, FOOD), (1, OPALS)] {
                assert_eq!(
                    market.tons(order(row, good, direction, Lot::Click)),
                    Err(TradeRefusal::NotTraded),
                    "{row} {good:?} {direction:?}"
                );
            }
        }
    }

    /// The (good, price) rows at `stellar`, with `flags`, by `markup`.
    fn priced(stellar: StellarId, flags: u32, markup: Markup) -> Vec<(Good, i64)> {
        market(&Exchange {
            rules: rules(RuleSource::Engine),
            markup,
            ..exchange(&goods(), stellar, flags, &pilot(0))
        })
        .expect("trades")
        .rows
        .iter()
        .map(|row| (row.good, row.price))
        .collect()
    }

    #[test]
    fn the_markup_prices_the_low_and_high_commodities_and_medium_is_the_base() {
        // Food low, industrial high, medical medium.
        let flags = TRADE | (1 << 28) | (4 << 24) | (2 << 20);
        let commodities = |markup| {
            priced(StellarId(150), flags, markup)
                .into_iter()
                .map(|(_, price)| price)
                .collect::<Vec<_>>()
        };
        assert_eq!(commodities(Markup::Standard), [60, 437, 750]);
        assert_eq!(commodities(Markup::Outlaw), [68, 385, 750]);
        assert_eq!(commodities(Markup::Dominated), [50, 525, 750]);
    }

    #[test]
    fn the_markup_prices_junk_where_it_is_sold_and_where_it_is_bought() {
        let water = Good::Junk(JunkId(134));
        let opals = Good::Junk(JunkId(146));
        assert_eq!(
            priced(EARTH, TRADE, Markup::Outlaw),
            [(water, 272), (opals, 1090)]
        );
        assert_eq!(priced(MARS, TRADE, Markup::Outlaw), [(opals, 1320)]);
        assert_eq!(
            priced(EARTH, TRADE, Markup::Dominated),
            [(water, 200), (opals, 800)]
        );
        assert_eq!(priced(MARS, TRADE, Markup::Dominated), [(opals, 1800)]);
    }

    /// Gold, sold at Earth on `sell_on` and bought there on `buy_on`, both
    /// as listed in `sold_at` and `bought_at`.
    fn gold(sold_here: bool, bought_here: bool, sell_on: &str, buy_on: &str) -> Goods {
        let gold = JunkRecord {
            id: JunkId(200),
            name: "Gold".to_owned(),
            abbrev: String::new(),
            base_price: 400,
            sold_at: if sold_here { vec![EARTH] } else { Vec::new() },
            bought_at: if bought_here { vec![EARTH] } else { Vec::new() },
            sell_on: Test::parse(sell_on),
            buy_on: Test::parse(buy_on),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        };
        Goods::new(&CommodityStrings::default(), vec![gold], Vec::new())
    }

    /// The prices of the gold rows Earth lists through `gate`: its
    /// `BoughtAt` row at the high price first, then its `SoldAt` row at the
    /// low price.
    fn gold_at_earth(goods: &Goods, gate: Gate) -> Vec<i64> {
        let found = market(&Exchange {
            gate,
            rules: rules(RuleSource::Engine),
            ..exchange(goods, EARTH, TRADE, &pilot(0))
        })
        .expect("trades");
        found
            .rows
            .iter()
            .filter(|row| row.good == Good::Junk(JunkId(200)))
            .map(|row| row.price)
            .collect()
    }

    const REFUSING_7: Gate<'static> = Gate {
        control_bits: &RefuseBits(&[7]),
        ..Gate::FRESH
    };

    #[test]
    fn sell_on_gates_the_junk_a_stellar_sells_and_buy_on_what_it_buys() {
        // Its SoldAt side, where the player buys, as `_DoTradeDialog` tests
        // SellOn (@0x5de58).
        let sold = gold(true, false, "b7", "");
        assert_eq!(gold_at_earth(&sold, REFUSING_7), Vec::<i64>::new());
        let sold = gold(true, false, "", "b7");
        assert_eq!(gold_at_earth(&sold, REFUSING_7), [320]);
        // Its BoughtAt side, where the player sells, on BuyOn (@0x5dda8).
        let bought = gold(false, true, "", "b7");
        assert_eq!(gold_at_earth(&bought, REFUSING_7), Vec::<i64>::new());
        let bought = gold(false, true, "b7", "");
        assert_eq!(gold_at_earth(&bought, REFUSING_7), [500]);
    }

    #[test]
    fn junk_listed_both_ways_trades_only_the_way_its_test_allows() {
        let no_selling = gold(true, true, "b7", "");
        assert_eq!(
            gold_at_earth(&no_selling, REFUSING_7),
            [500],
            "only its BoughtAt row, at the high price"
        );
        let no_buying = gold(true, true, "", "b7");
        assert_eq!(
            gold_at_earth(&no_buying, REFUSING_7),
            [320],
            "only its SoldAt row, at the low price"
        );
        let both = gold(true, true, "b7", "b7");
        assert_eq!(gold_at_earth(&both, REFUSING_7), Vec::<i64>::new());
        assert_eq!(
            gold_at_earth(&both, Gate::FRESH),
            Vec::<i64>::new(),
            "bit 7 is clear"
        );
        let neither = gold(true, true, "!b7", "!b7");
        assert_eq!(gold_at_earth(&neither, Gate::FRESH), [500, 320]);
    }

    #[test]
    fn a_malformed_junk_test_is_never_met() {
        let allowing = Gate {
            control_bits: &AllowAll,
            ..Gate::FRESH
        };
        let sold = gold(true, true, "b1 &", "");
        assert_eq!(gold_at_earth(&sold, allowing), [500]);
        let bought = gold(true, true, "", "b1 &");
        assert_eq!(gold_at_earth(&bought, allowing), [320]);
    }

    #[test]
    fn junk_follows_the_commodities() {
        let flags = TRADE | (2 << 28);
        let found = market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), MARS, flags, &pilot(0))
        })
        .expect("trades");
        let listed: Vec<_> = found.rows.iter().map(|row| row.good).collect();
        assert_eq!(listed, [Good::Commodity(0), Good::Junk(JunkId(146))]);
    }

    #[test]
    fn each_row_shows_the_tons_held_and_the_space_free() {
        let mut pilot = pilot(0);
        pilot.cargo = BTreeMap::from([
            (Good::Commodity(0), 3),
            (Good::Junk(JunkId(146)), 2),
            (Good::Commodity(5), 4),
            (Good::Junk(JunkId(999)), 1),
        ]);
        let found = market(&Exchange {
            capacity: 12,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), EARTH, TRADE | (2 << 28), &pilot)
        })
        .expect("trades");
        let held: Vec<_> = found.rows.iter().map(|row| (row.good, row.held)).collect();
        assert_eq!(
            held,
            [
                (Good::Commodity(0), 3),
                (Good::Junk(JunkId(134)), 0),
                (Good::Junk(JunkId(146)), 2),
            ]
        );
        assert_eq!(
            (found.capacity, found.free),
            (12, 2),
            "everything held counts"
        );
        let over = market(&Exchange {
            capacity: 5,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), EARTH, TRADE, &pilot)
        })
        .expect("trades");
        assert_eq!(over.free, 0, "more held than there is space");
    }

    #[test]
    fn the_mission_cargo_aboard_takes_space_but_is_never_held_to_sell() {
        let mut pilot = pilot(0);
        pilot.cargo = BTreeMap::from([(Good::Commodity(0), 3)]);
        pilot.mission_tons = 5;
        let found = market(&Exchange {
            capacity: 12,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), EARTH, TRADE | (2 << 28), &pilot)
        })
        .expect("trades");
        assert_eq!(found.free, 4, "3 tons of food held, 5 of the mission's");
        assert_eq!(found.rows[0].held, 3, "the mission's food is not held");
        pilot.cargo.clear();
        let only = market(&Exchange {
            capacity: 12,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), EARTH, TRADE | (2 << 28), &pilot)
        })
        .expect("trades");
        assert_eq!(
            (only.rows[0].good, only.rows[0].held),
            (Good::Commodity(0), 0)
        );
        let sell = Order {
            row: 0,
            good: Good::Commodity(0),
            direction: Direction::Sell,
            lot: Lot::Max,
        };
        assert_eq!(only.tons(sell), Err(TradeRefusal::NoneHeld));
        pilot.mission_tons += 20;
        let over = market(&Exchange {
            capacity: 12,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), EARTH, TRADE | (2 << 28), &pilot)
        })
        .expect("trades");
        assert_eq!(over.free, 0, "more aboard than there is space");
    }

    /// An event of `record`'s, which is at `stellar`, with `days` left.
    fn at(record: &DisasterRecord, days: u16) -> (DisasterId, ActiveEvent) {
        let stellar = Some(StellarId(record.stellar));
        (record.id, ActiveEvent { days, stellar })
    }

    /// The exchange at `stellar` with these events active, each at its
    /// record's stellar.
    fn with_events(
        stellar: StellarId,
        flags: u32,
        active: &[(i16, u16)],
        source: RuleSource,
    ) -> Market {
        let mut pilot = pilot(0);
        let records = disasters();
        pilot.events = active
            .iter()
            .map(|&(id, days)| {
                records
                    .iter()
                    .find(|record| record.id == DisasterId(id))
                    .map_or_else(
                        || {
                            (
                                DisasterId(id),
                                ActiveEvent {
                                    days,
                                    stellar: None,
                                },
                            )
                        },
                        |record| at(record, days),
                    )
            })
            .collect();
        market(&Exchange {
            rules: rules(source),
            ..exchange(&goods(), stellar, flags, &pilot)
        })
        .expect("trades")
    }

    fn price(market: &Market, good: Good) -> i64 {
        market.row(good).expect("listed").price
    }

    #[test]
    fn an_active_event_moves_its_commoditys_price_at_its_stellar() {
        let flags = PORT_KANE;
        let food = Good::Commodity(0);
        let quiet = with_events(EARTH, flags, &[], RuleSource::Engine);
        assert_eq!(price(&quiet, food), 93);
        let surplus = with_events(EARTH, flags, &[(128, 30)], RuleSource::Engine);
        assert_eq!(price(&surplus, food), 60, "75 - 15");
        assert_eq!(
            price(&surplus, Good::Commodity(1)),
            350,
            "other goods keep theirs"
        );
        let both = with_events(EARTH, flags, &[(128, 1), (130, 7)], RuleSource::Engine);
        assert_eq!(price(&both, food), 90, "75 + 15, the higher ID");
        let elsewhere = with_events(MARS, flags, &[(128, 30)], RuleSource::Engine);
        assert_eq!(price(&elsewhere, food), 93);
        let glut = with_events(MARS, flags, &[(129, 3), (999, 3)], RuleSource::Engine);
        assert_eq!(price(&glut, Good::Commodity(1)), 280, "350 - 70");
    }

    /// The story layer tells the events (`nova_sim::exchange_events`),
    /// since their text needs `STR#` strings this crate cannot read.
    #[test]
    fn the_market_leaves_its_events_to_the_story_layer() {
        for active in [&[][..], &[(128, 30)], &[(128, 1), (130, 7)]] {
            let found = with_events(EARTH, PORT_KANE, active, RuleSource::Engine);
            assert_eq!(found.events, Vec::<String>::new(), "{active:?}");
            let found = with_events(EARTH, PORT_KANE, active, RuleSource::Bible);
            assert_eq!(found.events, Vec::<String>::new(), "{active:?}");
        }
    }

    #[test]
    fn by_the_engine_an_active_event_prices_its_commodity_from_its_base_price_whatever_the_level() {
        let food = Good::Commodity(0);
        for (flags, level) in [
            (PORT_KANE, 93),
            (TRADE | (1 << 28), 60),
            (TRADE | (2 << 28), 75),
        ] {
            let quiet = with_events(EARTH, flags, &[], RuleSource::Engine);
            assert_eq!(price(&quiet, food), level, "{flags:#x}");
            let surplus = with_events(EARTH, flags, &[(128, 30)], RuleSource::Engine);
            assert_eq!(price(&surplus, food), 60, "{flags:#x}");
        }
        let industrial = Good::Commodity(1);
        for (flags, level) in [
            (TRADE | (1 << 24), 280),
            (TRADE | (2 << 24), 350),
            (TRADE | (4 << 24), 437),
        ] {
            let quiet = with_events(MARS, flags, &[], RuleSource::Engine);
            assert_eq!(price(&quiet, industrial), level, "{flags:#x}");
            let glut = with_events(MARS, flags, &[(129, 3)], RuleSource::Engine);
            assert_eq!(price(&glut, industrial), 280, "{flags:#x}");
        }
        let surplus = with_events(EARTH, PORT_KANE, &[(128, 30)], RuleSource::Engine);
        let others: Vec<_> = (1..6)
            .map(|n| price(&surplus, Good::Commodity(n)))
            .collect();
        assert_eq!(others, [350, 937, 900, 200, 440], "other goods keep theirs");
    }

    #[test]
    fn by_the_engine_the_higher_id_of_two_events_on_one_commodity_wins() {
        let both = with_events(EARTH, PORT_KANE, &[(130, 7), (128, 1)], RuleSource::Engine);
        assert_eq!(price(&both, Good::Commodity(0)), 90, "not 93");
        let mut records = disasters();
        records.reverse();
        let goods = Goods::new(&stock(), Vec::new(), records);
        let mut pilot = pilot(0);
        pilot.events = BTreeMap::from([at(&disasters()[2], 7), at(&disasters()[0], 1)]);
        let found = market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods, EARTH, PORT_KANE, &pilot)
        })
        .expect("trades");
        assert_eq!(
            price(&found, Good::Commodity(0)),
            90,
            "by ID, not record order"
        );
    }

    /// The exchange at Earth (`flags`) with one food event at Earth
    /// moving the price by `delta`, priced as `source` says.
    fn food_event(delta: i16, flags: u32, source: RuleSource) -> Market {
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(140),
                name: "Crash".to_owned(),
                stellar: 140,
                commodity: 0,
                price_delta: delta,
                ..DisasterRecord::default()
            }],
        );
        let mut pilot = pilot(0);
        pilot.events = BTreeMap::from([(
            DisasterId(140),
            ActiveEvent {
                days: 3,
                stellar: Some(EARTH),
            },
        )]);
        market(&Exchange {
            rules: rules(source),
            ..exchange(&goods, EARTH, flags, &pilot)
        })
        .expect("trades")
    }

    #[test]
    fn by_the_engine_an_event_price_of_4_or_less_is_5() {
        let food = |delta| price(&food_event(delta, PORT_KANE, RuleSource::Engine), FOOD);
        assert_eq!(food(-100), 5, "75 - 100");
        assert_eq!(food(-71), 5, "4");
        assert_eq!(food(-70), 5, "5");
        assert_eq!(food(-69), 6, "6");
        assert_eq!(MIN_COMMODITY_PRICE, 5);
    }

    #[test]
    fn by_the_engine_an_event_lists_a_commodity_the_stellar_does_not_trade() {
        let industrial_only = TRADE | (1 << 24);
        let mut buyer = pilot(1000);
        buyer.events = BTreeMap::from([at(&disasters()[0], 30)]);
        buyer.cargo = BTreeMap::from([(FOOD, 2)]);
        let found = market(&Exchange {
            capacity: 10,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), EARTH, industrial_only, &buyer)
        })
        .expect("trades");
        assert_eq!(
            found.rows,
            [
                MarketRow {
                    held: 2,
                    ..marked(Lower, row(FOOD, "Food", 60))
                },
                marked(Low, row(Good::Commodity(1), "Industrial", 280)),
                row(Good::Junk(JunkId(134)), "Water", 240),
                row(Good::Junk(JunkId(146)), "Opals", 960),
            ]
        );
        assert_eq!(
            found.tons(order(0, FOOD, Direction::Buy, Lot::Click)),
            Ok(8),
            "8 tons free"
        );
        assert_eq!(
            found.tons(order(0, FOOD, Direction::Sell, Lot::Click)),
            Ok(2)
        );

        let priced_at_0 = patched(&[(0, StringPatch::Unreadable)]);
        let unreadable = Goods::new(&priced_at_0, Vec::new(), disasters());
        let found = market(&Exchange {
            capacity: 10,
            rules: rules(RuleSource::Engine),
            ..exchange(&unreadable, EARTH, industrial_only, &buyer)
        })
        .expect("trades");
        assert_eq!(price(&found, FOOD), 5, "a commodity priced at 0, 0 - 15");

        let found = market(&Exchange {
            capacity: 10,
            rules: rules(RuleSource::Engine),
            ..exchange(&goods(), MARS, industrial_only, &buyer)
        })
        .expect("trades");
        assert_eq!(found.row(FOOD), None, "an event at another stellar");
    }

    #[test]
    fn by_the_bible_events_add_to_the_level_price_and_several_add_up() {
        let food = Good::Commodity(0);
        let bible = RuleSource::Bible;
        let surplus = with_events(EARTH, PORT_KANE, &[(128, 30)], bible);
        assert_eq!(price(&surplus, food), 78, "93 - 15");
        let both = with_events(EARTH, PORT_KANE, &[(128, 1), (130, 7)], bible);
        assert_eq!(price(&both, food), 93, "-15 and +15");
        assert_eq!(price(&food_event(-100, PORT_KANE, bible), food), 5);
        let glut = with_events(MARS, PORT_KANE, &[(129, 3)], bible);
        assert_eq!(price(&glut, Good::Commodity(1)), 280, "350 - 70");
        let low = with_events(MARS, TRADE | (1 << 24), &[(129, 3)], bible);
        assert_eq!(price(&low, Good::Commodity(1)), 210, "280 - 70");
        let industrial_only = TRADE | (1 << 24);
        let untraded = with_events(EARTH, industrial_only, &[(128, 30)], bible);
        let listed: Vec<_> = untraded
            .rows
            .iter()
            .map(|row| (row.good, row.price))
            .collect();
        assert_eq!(
            listed,
            [
                (Good::Commodity(1), 280),
                (Good::Junk(JunkId(134)), 240),
                (Good::Junk(JunkId(146)), 960),
            ],
            "an event on a commodity not traded here lists nothing"
        );
    }

    #[test]
    fn a_commodity_price_never_goes_below_5() {
        let food_low = TRADE | (1 << 28);
        let food_high = TRADE | (4 << 28);
        for source in RuleSource::ALL {
            for (base, flags, expected) in [
                ("6", food_low, 5),
                ("7", food_low, 5),
                ("8", food_low, 6),
                ("-40", food_high, 5),
                ("", food_low, 5),
                ("", TRADE | (2 << 28), 5),
                ("", food_high, 5),
            ] {
                let goods = Goods::new(&strings(&["Food"], &[base]), Vec::new(), Vec::new());
                let found = market(&Exchange {
                    rules: rules(source),
                    ..exchange(&goods, EARTH, flags, &pilot(0))
                })
                .expect("trades");
                assert_eq!(price(&found, FOOD), expected, "{base} {source:?}");
            }
        }
    }

    #[test]
    fn junk_prices_are_not_raised_to_5() {
        let cheap = |id, base_price| JunkRecord {
            id: JunkId(id),
            name: "Cheap".to_owned(),
            base_price,
            sold_at: vec![EARTH],
            ..unlisted()
        };
        let goods = Goods::new(
            &CommodityStrings::default(),
            vec![cheap(200, 0), cheap(201, 5)],
            Vec::new(),
        );
        for (junk_price, expected) in [
            (RuleSource::Engine, &[4][..]),
            (RuleSource::Bible, &[0, 4][..]),
        ] {
            for event_price in RuleSource::ALL {
                let rules = Rulebook::default()
                    .with_override(RuleKey::EventPrice, event_price)
                    .with_override(RuleKey::JunkPrice, junk_price);
                let found = market(&Exchange {
                    rules,
                    ..exchange(&goods, EARTH, TRADE, &pilot(0))
                })
                .expect("trades");
                let prices: Vec<_> = found.rows.iter().map(|row| row.price).collect();
                assert_eq!(prices, expected, "{rules:?}");
            }
        }
    }

    #[test]
    fn a_high_price_is_held_in_16_bits() {
        use PriceLevel::High;
        assert_eq!(band_price(30000, High, Markup::Standard), -28036, "37500");
        assert_eq!(band_price(-30000, High, Markup::Standard), 28036, "-37500");
        for (markup, last, first, wrapped) in [
            (Markup::Standard, 26214, 26215, -32768),
            (Markup::Outlaw, 29789, 29790, -32767),
            (Markup::Dominated, 21845, 21846, -32767),
        ] {
            assert_eq!(band_price(last, High, markup), 32767, "{markup:?}");
            assert_eq!(band_price(first, High, markup), wrapped, "{markup:?}");
        }
    }

    #[test]
    fn low_and_medium_prices_never_wrap() {
        use PriceLevel::{Low, Medium};
        for markup in [Markup::Standard, Markup::Outlaw, Markup::Dominated] {
            for base in [32767, -32768] {
                assert_eq!(band_price(base, Medium, markup), base, "{markup:?}");
            }
        }
        assert_eq!(band_price(32767, Low, Markup::Standard), 26213);
        assert_eq!(band_price(-32768, Low, Markup::Standard), -26214);
        assert_eq!(band_price(32767, Low, Markup::Outlaw), 29788);
        assert_eq!(band_price(-32768, Low, Markup::Dominated), -21845);
    }

    /// The exchange at Earth (`flags`) with food priced from `base` and
    /// these food events (ID, delta) active at Earth.
    fn food_with_events(
        base: &str,
        flags: u32,
        events: &[(i16, i16)],
        source: RuleSource,
    ) -> Market {
        let records = events
            .iter()
            .map(|&(id, delta)| DisasterRecord {
                id: DisasterId(id),
                name: format!("Event {id}"),
                stellar: 140,
                commodity: 0,
                price_delta: delta,
                ..DisasterRecord::default()
            })
            .collect();
        let goods = Goods::new(&strings(&["Food"], &[base]), Vec::new(), records);
        let mut pilot = pilot(0);
        pilot.events = events
            .iter()
            .map(|&(id, _)| {
                (
                    DisasterId(id),
                    ActiveEvent {
                        days: 3,
                        stellar: Some(EARTH),
                    },
                )
            })
            .collect();
        market(&Exchange {
            rules: rules(source),
            ..exchange(&goods, EARTH, flags, &pilot)
        })
        .expect("trades")
    }

    #[test]
    fn by_the_engine_a_price_past_32767_wraps_before_the_floor() {
        let food_high = TRADE | (4 << 28);
        let engine = RuleSource::Engine;
        let high = food_with_events("30000", food_high, &[], engine);
        assert_eq!(price(&high, FOOD), 5, "37500 is -28036");
        let event = food_with_events("32000", food_high, &[(140, 1000)], engine);
        assert_eq!(price(&event, FOOD), 5, "33000 is -32536");
        let positive = food_with_events("-30000", food_high, &[(140, -10000)], engine);
        assert_eq!(price(&positive, FOOD), 25536, "-40000 is 25536");
    }

    #[test]
    fn by_the_bible_the_level_price_plus_the_deltas_wraps_before_the_floor() {
        let food_medium = TRADE | (2 << 28);
        let bible = RuleSource::Bible;
        let one = food_with_events("32000", food_medium, &[(140, 1000)], bible);
        assert_eq!(price(&one, FOOD), 5, "33000 is -32536");
        let forward = food_with_events("32000", food_medium, &[(140, 20000), (141, 15000)], bible);
        let backward = food_with_events("32000", food_medium, &[(140, 15000), (141, 20000)], bible);
        assert_eq!(price(&forward, FOOD), 1464, "67000 is 1464");
        assert_eq!(price(&backward, FOOD), 1464, "in either ID order");
    }

    const ODD: Good = Good::Junk(JunkId(200));

    /// The exchange at Earth, by `junk_price`, of a `jünk` 200 of `base`
    /// price that Earth buys (`bought`) and sells (`sold`), for `pilot`
    /// with 10 tons of space, traded both ways by the engine.
    fn junk_market(
        base: i16,
        ways: (bool, bool),
        junk_price: RuleSource,
        pilot: &FakePilot,
    ) -> Market {
        let rules = Rulebook::default().with_override(RuleKey::JunkPrice, junk_price);
        junk_market_by(base, ways, rules, pilot)
    }

    /// [`junk_market`] priced and traded by `rules`.
    fn junk_market_by(
        base: i16,
        (bought, sold): (bool, bool),
        rules: Rulebook,
        pilot: &FakePilot,
    ) -> Market {
        let odd = JunkRecord {
            id: JunkId(200),
            name: "Odd".to_owned(),
            base_price: base,
            bought_at: if bought { vec![EARTH] } else { Vec::new() },
            sold_at: if sold { vec![EARTH] } else { Vec::new() },
            ..unlisted()
        };
        let goods = Goods::new(&CommodityStrings::default(), vec![odd], Vec::new());
        market(&Exchange {
            capacity: 10,
            rules,
            ..exchange(&goods, EARTH, TRADE, pilot)
        })
        .expect("trades")
    }

    /// The (price, sold here, bought here) of each row of `market`.
    fn junk_rows(market: &Market) -> Vec<(i64, bool, bool)> {
        market
            .rows
            .iter()
            .map(|row| (row.price, row.sold_here, row.bought_here))
            .collect()
    }

    const ONLY_BOUGHT: (bool, bool) = (true, false);
    const ONLY_SOLD: (bool, bool) = (false, true);
    const BOTH_WAYS: (bool, bool) = (true, true);

    #[test]
    fn by_the_engine_a_junk_only_sold_here_is_also_bought_at_its_low_price() {
        let mut holding = pilot(0);
        holding.cargo = BTreeMap::from([(ODD, 3)]);
        let found = junk_market(400, ONLY_SOLD, RuleSource::Engine, &holding);
        assert_eq!(junk_rows(&found), [(320, true, true)]);
        assert_eq!(found.tons(order(0, ODD, Direction::Sell, Lot::Max)), Ok(3));
        assert!(found.row_allows(0, Direction::Sell));
        let buyer = junk_market(400, ONLY_SOLD, RuleSource::Engine, &pilot(1000));
        assert_eq!(buyer.tons(order(0, ODD, Direction::Buy, Lot::Max)), Ok(3));
        assert!(buyer.row_allows(0, Direction::Buy));
    }

    #[test]
    fn by_the_engine_a_junk_only_bought_here_is_also_sold_at_its_high_price() {
        let bought = |cash| junk_market(400, ONLY_BOUGHT, RuleSource::Engine, &pilot(cash));
        assert_eq!(junk_rows(&bought(0)), [(500, true, true)]);
        let buy = |cash, lot| bought(cash).tons(order(0, ODD, Direction::Buy, lot));
        assert_eq!(buy(1999, Lot::Max), Ok(3), "1999 at 500");
        assert_eq!(buy(100_000, Lot::Max), Ok(10), "the free space");
        assert_eq!(buy(499, Lot::Click), Err(TradeRefusal::CannotAfford));
        assert!(bought(500).row_allows(0, Direction::Buy));
        assert!(!bought(499).row_allows(0, Direction::Buy));
        let mut holding = pilot(0);
        holding.cargo = BTreeMap::from([(ODD, 3)]);
        let holding = junk_market(400, ONLY_BOUGHT, RuleSource::Engine, &holding);
        assert!(holding.row_allows(0, Direction::Sell));
    }

    #[test]
    fn by_the_bible_a_junk_listed_one_way_is_not_traded_the_other() {
        let rules = trading_by(RuleSource::Bible);
        let mut holding = pilot(100_000);
        holding.cargo = BTreeMap::from([(ODD, 3)]);
        for (ways, row, refused) in [
            (ONLY_SOLD, (320, true, false), Direction::Sell),
            (ONLY_BOUGHT, (500, false, true), Direction::Buy),
        ] {
            let found = junk_market_by(400, ways, rules, &holding);
            assert_eq!(junk_rows(&found), [row], "{ways:?}");
            assert_eq!(
                found.tons(order(0, ODD, refused, Lot::Click)),
                Err(TradeRefusal::NotTraded),
                "{ways:?}"
            );
            assert!(!found.row_allows(0, refused), "{ways:?}");
        }
    }

    #[test]
    fn a_junk_price_wraps_and_by_the_engine_is_listed_signed() {
        for (base, ways, engine, bible) in [
            (30000, ONLY_BOUGHT, -28036, 0),
            (-30000, ONLY_BOUGHT, 28036, 28036),
            (-100, ONLY_BOUGHT, -125, 0),
            (-100, ONLY_SOLD, -80, 0),
            (-1, ONLY_BOUGHT, -1, 0),
        ] {
            for (source, expected) in [(RuleSource::Engine, engine), (RuleSource::Bible, bible)] {
                let found = junk_market(base, ways, source, &pilot(0));
                assert_eq!(price(&found, ODD), expected, "{base} {ways:?} {source:?}");
            }
        }
    }

    #[test]
    fn by_the_engine_a_negative_junk_price_buys_nothing() {
        for cash in [0, 1000, -1000] {
            let found = junk_market(-100, ONLY_SOLD, RuleSource::Engine, &pilot(cash));
            for lot in [Lot::Click, Lot::Max] {
                assert_eq!(
                    found.tons(order(0, ODD, Direction::Buy, lot)),
                    Err(TradeRefusal::CannotAfford),
                    "{cash} {lot:?}"
                );
            }
            assert_eq!(
                found.row_allows(0, Direction::Buy),
                cash >= -80,
                "{cash}: enabled as `_CanBuyGoods` has it"
            );
        }
    }

    #[test]
    fn by_the_other_reading_a_negative_junk_price_is_bought_free() {
        let found = junk_market(-100, ONLY_SOLD, RuleSource::Bible, &pilot(0));
        assert_eq!(found.tons(order(0, ODD, Direction::Buy, Lot::Max)), Ok(10));
    }

    #[test]
    fn by_the_engine_a_negative_junk_price_is_sold_at_a_loss() {
        let mut holding = pilot(0);
        holding.cargo = BTreeMap::from([(ODD, 3)]);
        let found = junk_market(-100, ONLY_BOUGHT, RuleSource::Engine, &holding);
        assert_eq!(found.tons(order(0, ODD, Direction::Sell, Lot::Max)), Ok(3));
        assert!(found.row_allows(0, Direction::Sell));
    }

    #[test]
    fn by_the_engine_a_junk_row_priced_0_is_not_listed() {
        for ways in [ONLY_BOUGHT, ONLY_SOLD, BOTH_WAYS] {
            let mut holding = pilot(1000);
            holding.cargo = BTreeMap::from([(ODD, 3)]);
            let found = junk_market(0, ways, RuleSource::Engine, &holding);
            assert_eq!(junk_rows(&found), [], "{ways:?}");
            for direction in [Direction::Buy, Direction::Sell] {
                assert_eq!(
                    found.tons(order(0, ODD, direction, Lot::Click)),
                    Err(TradeRefusal::NotTraded),
                    "{ways:?} {direction:?}"
                );
            }
        }
    }

    #[test]
    fn by_the_other_reading_a_junk_row_priced_0_is_listed_and_bought_free() {
        let found = junk_market(0, ONLY_SOLD, RuleSource::Bible, &pilot(0));
        assert_eq!(junk_rows(&found), [(0, true, true)]);
        assert_eq!(found.tons(order(0, ODD, Direction::Buy, Lot::Max)), Ok(10));
        let both = junk_market(0, BOTH_WAYS, RuleSource::Bible, &pilot(0));
        assert_eq!(junk_rows(&both), [(0, true, true), (0, true, true)]);
        assert_eq!(both.tons(order(0, ODD, Direction::Buy, Lot::Max)), Ok(10));
    }

    #[test]
    fn by_the_engine_junk_listed_both_ways_loses_only_its_row_priced_0() {
        let mut holding = pilot(1000);
        holding.cargo = BTreeMap::from([(ODD, 3)]);
        let found = junk_market(1, BOTH_WAYS, RuleSource::Engine, &holding);
        assert_eq!(
            junk_rows(&found),
            [(1, true, true)],
            "1 × 1.25, not 1 / 1.25"
        );
        assert_eq!(found.tons(order(0, ODD, Direction::Sell, Lot::Max)), Ok(3));
        assert_eq!(
            found.tons(order(0, ODD, Direction::Buy, Lot::Max)),
            Ok(7),
            "the free space, at 1 a ton"
        );
        assert_eq!(
            found.tons(order(1, ODD, Direction::Buy, Lot::Click)),
            Err(TradeRefusal::NotTraded),
            "no row 1"
        );
        let floor = junk_market(1, BOTH_WAYS, RuleSource::Bible, &holding);
        assert_eq!(junk_rows(&floor), [(1, true, true), (0, true, true)]);
        let one_way = Rulebook::default()
            .with_override(RuleKey::JunkPrice, RuleSource::Engine)
            .with_override(RuleKey::JunkTrade, RuleSource::Bible);
        let one_way = junk_market_by(1, BOTH_WAYS, one_way, &holding);
        assert_eq!(junk_rows(&one_way), [(1, false, true)]);
        assert_eq!(
            one_way.tons(order(0, ODD, Direction::Buy, Lot::Click)),
            Err(TradeRefusal::NotTraded)
        );
    }

    /// The exchange at Earth with one event, `commodity` (-10), active.
    fn odd_event(commodity: i16) -> Market {
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(140),
                name: "Odd".to_owned(),
                stellar: 140,
                commodity,
                price_delta: -10,
                ..DisasterRecord::default()
            }],
        );
        let mut pilot = pilot(0);
        pilot.events = BTreeMap::from([(
            DisasterId(140),
            ActiveEvent {
                days: 3,
                stellar: Some(EARTH),
            },
        )]);
        market(&Exchange {
            rules: rules(RuleSource::Engine),
            ..exchange(&goods, EARTH, PORT_KANE, &pilot)
        })
        .expect("trades")
    }

    #[test]
    fn an_event_beyond_the_standard_commodities_moves_equipment() {
        let found = odd_event(6);
        assert_eq!(
            price(&found, Good::Commodity(5)),
            540,
            "clamped to 5: its BasePrice 550, less 10"
        );
    }

    #[test]
    fn an_event_on_a_negative_commodity_moves_nothing() {
        let found = odd_event(-1);
        let quiet = odd_event(0);
        let prices = |market: &Market| -> Vec<i64> {
            (1..6).map(|n| price(market, Good::Commodity(n))).collect()
        };
        assert_eq!(
            prices(&found),
            prices(&quiet),
            "no commodity but food moves"
        );
        assert_eq!(price(&found, Good::Commodity(5)), 440);
        assert_eq!(price(&found, Good::Commodity(0)), 93, "not food either");
    }

    /// The exchange at `here` (Port Kane's levels) with one food event
    /// (-15) active, its record's `Stellar` `record`, stored at `stored`.
    fn one_event(
        record: i16,
        stored: Option<StellarId>,
        here: StellarId,
        source: RuleSource,
    ) -> Market {
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(128),
                name: "Surplus".to_owned(),
                stellar: record,
                price_delta: -15,
                ..DisasterRecord::default()
            }],
        );
        let mut pilot = pilot(0);
        pilot.events = BTreeMap::from([(
            DisasterId(128),
            ActiveEvent {
                days: 3,
                stellar: stored,
            },
        )]);
        market(&Exchange {
            rules: rules(source),
            ..exchange(&goods, here, PORT_KANE, &pilot)
        })
        .expect("trades")
    }

    /// Food's price at `market`.
    fn food(market: &Market) -> i64 {
        price(market, Good::Commodity(0))
    }

    /// Whether the event is shown is the story layer's
    /// (`nova_sim::exchange_events`).
    #[test]
    fn an_event_moves_the_price_at_its_stored_stellar_only() {
        let (surplus, quiet) = (60, 93);
        for record in [-1, 140] {
            let mars = one_event(record, Some(MARS), MARS, RuleSource::Engine);
            assert_eq!(food(&mars), surplus, "{record}");
            let earth = one_event(record, Some(MARS), EARTH, RuleSource::Engine);
            assert_eq!(food(&earth), quiet, "{record}");
        }
    }

    #[test]
    fn an_event_from_an_older_save_is_at_its_records_stellar() {
        let (surplus, quiet) = (60, 93);
        assert_eq!(
            food(&one_event(140, None, EARTH, RuleSource::Engine)),
            surplus
        );
        assert_eq!(food(&one_event(140, None, MARS, RuleSource::Engine)), quiet);
        for here in [EARTH, MARS, StellarId(-1)] {
            assert_eq!(food(&one_event(-1, None, here, RuleSource::Engine)), quiet);
        }
    }

    // The price rules.

    /// What `ask` makes of a commodity of `base` price, at `level` by the
    /// standard markup, moved by events of these `deltas` (by ascending
    /// ID), under `rules`.
    fn quoted<T>(
        rules: Rulebook,
        base: i64,
        level: Option<PriceLevel>,
        deltas: &[i64],
        ask: impl FnOnce(&CommodityQuote) -> T,
    ) -> T {
        let goods = Goods::default();
        let pilot = pilot(0);
        let exchange = Exchange {
            rules,
            ..exchange(&goods, EARTH, TRADE, &pilot)
        };
        ask(&CommodityQuote {
            exchange: &exchange,
            base,
            level,
            deltas,
        })
    }

    #[test]
    fn level_price_is_the_band_price_held_in_16_bits() {
        for source in RuleSource::ALL {
            let at = |base, level| quoted(rules(source), base, level, &[], |q| level_price().of(q));
            assert_eq!(at(75, Some(PriceLevel::High)), Some(93), "{source:?}");
            assert_eq!(at(26_215, Some(PriceLevel::Medium)), Some(26_215));
            assert_eq!(
                at(26_215, Some(PriceLevel::High)),
                Some(-32_768),
                "32768 wraps"
            );
            assert_eq!(at(75, None), None, "not traded at a level");
            let moved = quoted(rules(source), 75, Some(PriceLevel::Low), &[-15], |q| {
                level_price().of(q)
            });
            assert_eq!(moved, Some(60), "no event moves it");
        }
    }

    #[test]
    fn event_price_by_the_engine_is_the_base_plus_the_highest_ids_delta_whatever_the_level() {
        let at = |level, deltas: &[i64]| {
            quoted(rules(RuleSource::Engine), 75, level, deltas, |q| {
                event_price().of(q)
            })
        };
        assert_eq!(at(Some(PriceLevel::High), &[]), Some(93), "no event");
        assert_eq!(at(Some(PriceLevel::High), &[-15]), Some(60));
        assert_eq!(at(Some(PriceLevel::Low), &[-15, 15]), Some(90), "the last");
        assert_eq!(at(Some(PriceLevel::High), &[15, -15]), Some(60));
        let wrapped = quoted(rules(RuleSource::Engine), 32_000, None, &[1000], |q| {
            event_price().of(q)
        });
        assert_eq!(wrapped, Some(-32_536), "33000 held in 16 bits");
    }

    #[test]
    fn event_price_by_the_other_reading_adds_every_delta_to_the_level_price() {
        let at = |level, deltas: &[i64]| {
            quoted(rules(RuleSource::Bible), 75, level, deltas, |q| {
                event_price().of(q)
            })
        };
        assert_eq!(at(Some(PriceLevel::High), &[]), Some(93), "no event");
        assert_eq!(at(Some(PriceLevel::High), &[-15]), Some(78));
        assert_eq!(at(Some(PriceLevel::High), &[-15, 15]), Some(93));
        assert_eq!(at(Some(PriceLevel::Low), &[-15, -5]), Some(40));
        let wrapped = quoted(
            rules(RuleSource::Bible),
            32_000,
            Some(PriceLevel::Medium),
            &[20_000, 15_000],
            |q| event_price().of(q),
        );
        assert_eq!(wrapped, Some(1464), "67000 held in 16 bits");
    }

    #[test]
    fn event_price_lists_an_untraded_commodity_only_by_the_engine() {
        let at = |source| quoted(rules(source), 75, None, &[-15], |q| event_price().of(q));
        assert_eq!(at(RuleSource::Engine), Some(60));
        assert_eq!(at(RuleSource::Bible), None);
    }

    #[test]
    fn commodity_price_is_never_below_5_by_either_reading() {
        for source in RuleSource::ALL {
            let at = |base, level, deltas: &[i64]| {
                quoted(rules(source), base, level, deltas, |q| {
                    commodity_price().of(q)
                })
            };
            let medium = Some(PriceLevel::Medium);
            assert_eq!(at(10, medium, &[-8]), Some(5), "{source:?}");
            assert_eq!(at(4, medium, &[]), Some(5), "{source:?}");
            assert_eq!(at(5, medium, &[]), Some(5), "{source:?}");
            assert_eq!(at(6, medium, &[]), Some(6), "{source:?}");
            assert_eq!(at(-50, medium, &[]), Some(5), "{source:?}");
            assert_eq!(at(26_215, Some(PriceLevel::High), &[]), Some(5), "wrapped");
            assert_eq!(at(75, None, &[]), None, "{source:?}");
        }
        let untraded = |source| quoted(rules(source), 6, None, &[-4], |q| commodity_price().of(q));
        assert_eq!(untraded(RuleSource::Engine), Some(5));
        assert_eq!(untraded(RuleSource::Bible), None);
    }

    /// What `ask` makes of a `jünk` of `base` price on its `side`'s row,
    /// by the standard markup, under `rules`.
    fn junk_quoted<T>(
        rules: Rulebook,
        base: i64,
        side: JunkSide,
        ask: impl FnOnce(&JunkQuote) -> T,
    ) -> T {
        let goods = Goods::default();
        let pilot = pilot(0);
        let exchange = Exchange {
            rules,
            ..exchange(&goods, EARTH, TRADE, &pilot)
        };
        ask(&JunkQuote {
            exchange: &exchange,
            base,
            side,
        })
    }

    /// The rule set with `jünk` priced by `junk_price`.
    fn junk_priced_by(junk_price: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::JunkPrice, junk_price)
    }

    #[test]
    fn junk_price_by_the_engine_is_signed_and_unlisted_at_0() {
        let at = |base, side| {
            junk_quoted(junk_priced_by(RuleSource::Engine), base, side, |q| {
                junk_price().of(q)
            })
        };
        assert_eq!(at(400, JunkSide::BoughtAt), Some(500), "high");
        assert_eq!(at(400, JunkSide::SoldAt), Some(320), "low");
        assert_eq!(at(-100, JunkSide::BoughtAt), Some(-125));
        assert_eq!(at(-100, JunkSide::SoldAt), Some(-80));
        assert_eq!(at(30_000, JunkSide::BoughtAt), Some(-28_036), "wrapped");
        assert_eq!(at(0, JunkSide::BoughtAt), None);
        assert_eq!(at(1, JunkSide::SoldAt), None, "1 / 1.25 is 0");
        assert_eq!(at(1, JunkSide::BoughtAt), Some(1));
    }

    #[test]
    fn junk_price_by_the_other_reading_is_never_below_0_and_listed_at_0() {
        let at = |base, side| {
            junk_quoted(junk_priced_by(RuleSource::Bible), base, side, |q| {
                junk_price().of(q)
            })
        };
        assert_eq!(at(400, JunkSide::BoughtAt), Some(500), "high");
        assert_eq!(at(400, JunkSide::SoldAt), Some(320), "low");
        assert_eq!(at(-100, JunkSide::BoughtAt), Some(0));
        assert_eq!(at(-30_000, JunkSide::BoughtAt), Some(28_036), "wrapped");
        assert_eq!(at(30_000, JunkSide::BoughtAt), Some(0), "wrapped");
        assert_eq!(at(0, JunkSide::BoughtAt), Some(0));
        assert_eq!(at(1, JunkSide::SoldAt), Some(0));
    }

    #[test]
    fn junk_ways_by_the_engine_trade_both_ways() {
        for side in [JunkSide::BoughtAt, JunkSide::SoldAt] {
            let ways = junk_quoted(trading_by(RuleSource::Engine), 400, side, |q| {
                junk_ways().of(q)
            });
            assert_eq!(ways, (true, true));
        }
    }

    #[test]
    fn junk_ways_by_the_other_reading_trade_one_way_by_side() {
        let at = |side| {
            junk_quoted(trading_by(RuleSource::Bible), 400, side, |q| {
                junk_ways().of(q)
            })
        };
        assert_eq!(at(JunkSide::BoughtAt), (false, true), "sold here only");
        assert_eq!(at(JunkSide::SoldAt), (true, false), "bought here only");
    }

    #[test]
    fn each_disputed_exchange_rule_names_its_key() {
        let disputed = [
            (event_price().about().reads(), RuleKey::EventPrice),
            (junk_price().about().reads(), RuleKey::JunkPrice),
            (junk_ways().about().reads(), RuleKey::JunkTrade),
            (click_lot().about().reads(), RuleKey::TradeLot),
            (cash_divided().about().reads(), RuleKey::TradeDebt),
            (quotient().about().reads(), RuleKey::TradeQuotient),
            (option_asks_count().about().reads(), RuleKey::TradeCount),
        ];
        for (reads, key) in disputed {
            assert_eq!(reads, Some(key));
        }
        for plain in [
            *level_price().about(),
            *commodity_price().about(),
            *click_quotient().about(),
            *buy_most().about(),
            *sell_most().about(),
            *tons().about(),
        ] {
            assert_eq!(plain.reads(), None, "{}", plain.name());
        }
    }

    #[test]
    fn the_price_rules_cite_the_originals_addresses() {
        let cited = [
            level_price().about().reproduces(),
            event_price().about().reproduces(),
            commodity_price().about().reproduces(),
            junk_price().about().reproduces(),
            junk_ways().about().reproduces(),
        ];
        assert_eq!(
            cited,
            [
                "`_DoTradeDialog` @0x5dc0c-0x5dc48, @0x5dc87-0x5dcc5, held in 16 bits @0x5dcc5",
                "`_DoTradeDialog` @0x5dcf8-0x5dd58, held in 16 bits by `addw` @0x5dd19-0x5dd40",
                "floor of 5: `_DoTradeDialog` @0x5dcc8-0x5dcce, @0x5dd39-0x5dd40",
                "rows 6/7 `_DoTradeDialog` @0x5dd83-0x5de2f, held in 16 bits @0x5dddb/@0x5de2f; \
                 unlisted at 0 `_TradeDialogUpdate` @0x4d2a2-0x4d2ac, `_TradeFilter` @0x4e262; \
                 signed `_TradeDialogUpdate` @0x4d545-0x4d54b",
                "`_CanBuyGoods` @0xccec, `_CanSellGoods` @0x4a94, `_DrawTradeButtons` \
                 @0x2938f/0x2939d, `_TrackTradeButtons` @0x2960d/0x2963e",
            ]
        );
    }

    // Buying and selling.

    /// An exchange selling food at 100 and buying it, buying only opals at
    /// 500, and selling only water at 0, for a player with `cash`, `held`
    /// tons of each and `free` tons free.
    fn stall(cash: i64, held: u32, free: u32) -> Market {
        let mut food = row(Good::Commodity(0), "Food", 100);
        food.held = held;
        let mut opals = row(Good::Junk(JunkId(146)), "Opals", 500);
        opals.sold_here = false;
        opals.held = held;
        let mut water = row(Good::Junk(JunkId(134)), "Water", 0);
        water.bought_here = false;
        water.held = held;
        Market {
            rows: vec![food, opals, water],
            events: Vec::new(),
            cash,
            capacity: 100,
            free,
            rules: Rulebook::default(),
            ..Market::default()
        }
    }

    /// An order for `lot` of `good`, `direction`, on row `row`.
    fn order(row: usize, good: Good, direction: Direction, lot: Lot) -> Order {
        Order {
            row,
            good,
            direction,
            lot,
        }
    }

    const FOOD: Good = Good::Commodity(0);
    const OPALS: Good = Good::Junk(JunkId(146));
    const WATER: Good = Good::Junk(JunkId(134));

    #[test]
    fn a_click_buys_up_to_10_tons_and_the_most_is_what_space_and_cash_allow() {
        let buy = |market: &Market, lot| market.tons(order(0, FOOD, Direction::Buy, lot));
        assert_eq!(buy(&stall(100_000, 0, 50), Lot::Click), Ok(10), "10");
        assert_eq!(buy(&stall(1100, 0, 50), Lot::Click), Ok(10), "cash for 11");
        assert_eq!(buy(&stall(399, 0, 50), Lot::Click), Ok(3), "cash");
        assert_eq!(buy(&stall(100_000, 0, 4), Lot::Click), Ok(4), "space");
        assert_eq!(buy(&stall(100_000, 0, 11), Lot::Click), Ok(10));
        assert_eq!(buy(&stall(100, 0, 1), Lot::Click), Ok(1));
        assert_eq!(CLICK_TONS, 10);
        assert_eq!(buy(&stall(1000, 0, 50), Lot::Max), Ok(10), "cash");
        assert_eq!(buy(&stall(1099, 0, 50), Lot::Max), Ok(10));
        assert_eq!(buy(&stall(5000, 0, 50), Lot::Max), Ok(50), "past 10");
        assert_eq!(buy(&stall(100_000, 0, 7), Lot::Max), Ok(7), "space");
        assert_eq!(buy(&stall(700, 0, 7), Lot::Max), Ok(7), "both");
    }

    /// `market` traded by `trade_lot`.
    fn lots_by(trade_lot: RuleSource, market: Market) -> Market {
        Market {
            rules: market.rules.with_override(RuleKey::TradeLot, trade_lot),
            ..market
        }
    }

    #[test]
    fn by_the_other_reading_a_click_trades_a_ton() {
        let one_ton = |market: Market| lots_by(RuleSource::Bible, market);
        let buy = |market: &Market, lot| market.tons(order(0, FOOD, Direction::Buy, lot));
        let sell = |market: &Market, lot| market.tons(order(0, FOOD, Direction::Sell, lot));
        assert_eq!(buy(&one_ton(stall(100_000, 0, 50)), Lot::Click), Ok(1));
        assert_eq!(sell(&one_ton(stall(0, 25, 0)), Lot::Click), Ok(1));
        assert_eq!(buy(&one_ton(stall(5000, 0, 50)), Lot::Max), Ok(50));
        assert_eq!(sell(&one_ton(stall(0, 25, 0)), Lot::Max), Ok(25));
        assert_eq!(
            buy(&one_ton(stall(99, 0, 50)), Lot::Click),
            Err(TradeRefusal::CannotAfford)
        );
        assert_eq!(
            sell(&one_ton(stall(0, 0, 50)), Lot::Click),
            Err(TradeRefusal::NoneHeld)
        );
        let engine = lots_by(RuleSource::Engine, stall(100_000, 25, 50));
        assert_eq!(
            stall(100_000, 25, 50).rules.source_for(RuleKey::TradeLot),
            RuleSource::Engine,
            "the engine's by default"
        );
        assert_eq!(buy(&engine, Lot::Click), Ok(10));
        assert_eq!(sell(&engine, Lot::Click), Ok(10));
    }

    #[test]
    fn a_buy_with_no_space_or_not_enough_cash_for_a_ton_is_refused() {
        let buy = |market: &Market, lot| market.tons(order(0, FOOD, Direction::Buy, lot));
        for lot in [Lot::Click, Lot::Max] {
            assert_eq!(buy(&stall(1000, 0, 0), lot), Err(TradeRefusal::NoSpace));
            assert_eq!(buy(&stall(99, 0, 5), lot), Err(TradeRefusal::CannotAfford));
            assert_eq!(
                buy(&stall(-500, 0, 5), lot),
                Err(TradeRefusal::CannotAfford)
            );
            assert_eq!(buy(&stall(0, 0, 0), lot), Err(TradeRefusal::CannotAfford));
        }
    }

    #[test]
    fn a_good_priced_at_nothing_is_limited_only_by_space() {
        let buy = |market: &Market, lot| market.tons(order(2, WATER, Direction::Buy, lot));
        assert_eq!(buy(&stall(0, 0, 9), Lot::Max), Ok(9));
        assert_eq!(buy(&stall(0, 0, 9), Lot::Click), Ok(9));
        assert_eq!(buy(&stall(0, 0, 50), Lot::Click), Ok(10));
        assert_eq!(buy(&stall(0, 0, 50), Lot::Max), Ok(50));
        assert_eq!(buy(&stall(0, 0, 0), Lot::Max), Err(TradeRefusal::NoSpace));
    }

    #[test]
    fn a_click_sells_up_to_10_tons_and_the_most_everything_held() {
        let sell = |market: &Market, lot| market.tons(order(0, FOOD, Direction::Sell, lot));
        assert_eq!(sell(&stall(0, 25, 0), Lot::Click), Ok(10));
        assert_eq!(sell(&stall(0, 11, 0), Lot::Click), Ok(10));
        assert_eq!(sell(&stall(0, 10, 0), Lot::Click), Ok(10));
        assert_eq!(sell(&stall(0, 4, 0), Lot::Click), Ok(4));
        assert_eq!(sell(&stall(0, 25, 0), Lot::Max), Ok(25));
        assert_eq!(sell(&stall(0, 6, 0), Lot::Max), Ok(6));
        assert_eq!(sell(&stall(0, 1, 0), Lot::Max), Ok(1));
        for lot in [Lot::Click, Lot::Max] {
            assert_eq!(sell(&stall(0, 0, 9), lot), Err(TradeRefusal::NoneHeld));
        }
    }

    /// The stall's opals and water rows trade one way, as the Bible's
    /// `jünk` rows do ([`RuleKey::JunkTrade`](crate::RuleKey::JunkTrade)).
    #[test]
    fn a_good_not_traded_that_way_or_at_all_is_refused() {
        let market = stall(10_000, 5, 5);
        for lot in [Lot::Click, Lot::Max] {
            let refused = Err(TradeRefusal::NotTraded);
            assert_eq!(market.tons(order(1, OPALS, Direction::Buy, lot)), refused);
            assert_eq!(market.tons(order(2, WATER, Direction::Sell, lot)), refused);
            for direction in [Direction::Buy, Direction::Sell] {
                let metal = order(0, Good::Commodity(4), direction, lot);
                assert_eq!(market.tons(metal), refused);
            }
        }
        assert_eq!(
            market.tons(order(1, OPALS, Direction::Sell, Lot::Max)),
            Ok(5)
        );
        assert_eq!(
            market.tons(order(2, WATER, Direction::Buy, Lot::Max)),
            Ok(5)
        );
    }

    #[test]
    fn row_allows_a_sale_of_anything_held_on_a_row_bought_here() {
        let market = stall(150, 1, 3);
        assert!(market.row_allows(0, Direction::Sell));
        assert!(market.row_allows(1, Direction::Sell));
        assert!(!market.row_allows(2, Direction::Sell), "not bought here");
        assert!(!stall(99, 0, 3).row_allows(0, Direction::Sell));
        assert!(!market.row_allows(3, Direction::Sell), "no such row");
        assert_eq!(market.row(OPALS).map(|row| row.price), Some(500));
        assert_eq!(market.row(Good::Commodity(3)), None);
    }

    /// As `_CanBuyGoods` (@0xccec-0xcd31): the row is sold here, cash is
    /// at least the price, compared signed, and a ton of space is free.
    #[test]
    fn row_allows_a_buy_with_cash_for_the_price_and_space_free() {
        let buy = |market: &Market, index| market.row_allows(index, Direction::Buy);
        assert!(buy(&stall(100, 0, 3), 0), "cash = price");
        assert!(!buy(&stall(99, 0, 3), 0), "cash = price - 1");
        assert!(buy(&stall(150, 1, 1), 0));
        assert!(!buy(&stall(100_000, 0, 0), 0), "no space");
        assert!(!buy(&stall(100_000, 0, 50), 1), "not sold here");
        assert!(buy(&stall(0, 0, 3), 2), "water at 0");
        assert!(!buy(&stall(100_000, 0, 50), 3), "no such row");
    }

    // Can: the Buy and Sell checks.

    /// Row `index` of `market`, as the checks read it.
    fn trade(market: &Market, index: usize) -> RowTrade<'_> {
        RowTrade {
            market,
            row: &market.rows[index],
        }
    }

    /// Each check of `checks` with its citation, in order.
    fn listed_checks<C>(checks: &Checks<C, TradeRefusal>) -> Vec<(&'static str, &'static str)> {
        checks
            .descriptors()
            .map(|about| (about.name(), about.reproduces()))
            .collect()
    }

    /// The verdict of each check of `checks` that refuses `facts`, in
    /// order.
    fn verdicts<C: Context>(
        checks: &Checks<C, TradeRefusal>,
        facts: &C,
    ) -> Vec<Verdict<TradeRefusal>> {
        checks
            .all_refusals(facts)
            .into_iter()
            .map(|(_, verdict)| verdict)
            .collect()
    }

    /// The Buy checks are `_CanBuyGoods`'s (@0xccec-0xcd31), and the row's
    /// ways, in `_CanBuyGoods`'s order: the cash (@0xcd07-0xcd0d), then the
    /// space (@0xcd0f-0xcd26). The original returns only whether both
    /// hold, but the order decides which reason a buy with neither gives
    /// (`a_buy_with_no_space_or_not_enough_cash_for_a_ton_is_refused`).
    #[test]
    fn the_buy_checks_are_canbuygoods_checks_in_order() {
        assert_eq!(
            listed_checks(&can_buy()),
            [
                (
                    "sold here",
                    "the row's ways as listed (`jünk ways`); `_CanBuyGoods` @0xccec asks no list"
                ),
                (
                    "cash for the price",
                    "`_CanBuyGoods` @0xcd07-0xcd0d, signed"
                ),
                ("space free", "`_CanBuyGoods` @0xcd0f-0xcd26"),
            ]
        );
    }

    #[test]
    fn the_sell_checks_are_cansellgoods_checks_in_order() {
        assert_eq!(
            listed_checks(&can_sell()),
            [
                (
                    "bought here",
                    "the row's ways as listed (`jünk ways`); `_CanSellGoods` @0x4a94 asks no list"
                ),
                (
                    "held",
                    "`_CanSellGoods` @0x4aab-0x4ab1 (a commodity), @0x4ad9 (a `jünk`)"
                ),
            ]
        );
    }

    #[test]
    fn the_buy_checks_refuse_in_list_order() {
        let refuses = Verdict::Refuses;
        let broke = stall(0, 0, 0);
        assert_eq!(
            verdicts(&can(Direction::Buy), &trade(&broke, 1)),
            [
                refuses(TradeRefusal::NotTraded),
                refuses(TradeRefusal::CannotAfford),
                refuses(TradeRefusal::NoSpace),
            ],
            "opals, not sold here"
        );
        assert_eq!(
            verdicts(&can(Direction::Buy), &trade(&broke, 0)),
            [
                refuses(TradeRefusal::CannotAfford),
                refuses(TradeRefusal::NoSpace),
            ]
        );
        let roomy = stall(0, 0, 5);
        assert_eq!(
            verdicts(&can(Direction::Buy), &trade(&roomy, 0)),
            [refuses(TradeRefusal::CannotAfford)]
        );
        let first = |market: &Market, index| refusal(&can(Direction::Buy), &trade(market, index));
        assert_eq!(first(&broke, 1), Err(TradeRefusal::NotTraded));
        assert_eq!(first(&broke, 0), Err(TradeRefusal::CannotAfford));
        assert_eq!(first(&roomy, 0), Err(TradeRefusal::CannotAfford));
        assert_eq!(first(&stall(100, 0, 5), 0), Ok(()));
    }

    #[test]
    fn the_sell_checks_refuse_in_list_order() {
        let refuses = Verdict::Refuses;
        let empty = stall(0, 0, 0);
        assert_eq!(
            verdicts(&can(Direction::Sell), &trade(&empty, 2)),
            [
                refuses(TradeRefusal::NotTraded),
                refuses(TradeRefusal::NoneHeld),
            ],
            "water, not bought here"
        );
        assert_eq!(
            verdicts(&can(Direction::Sell), &trade(&empty, 0)),
            [refuses(TradeRefusal::NoneHeld)]
        );
        let first = |market: &Market, index| refusal(&can(Direction::Sell), &trade(market, index));
        assert_eq!(first(&empty, 2), Err(TradeRefusal::NotTraded));
        assert_eq!(first(&empty, 0), Err(TradeRefusal::NoneHeld));
        assert_eq!(first(&stall(0, 1, 0), 0), Ok(()));
    }

    #[test]
    fn no_exchange_check_draws() {
        for direction in [Direction::Buy, Direction::Sell] {
            for about in can(direction).descriptors() {
                assert!(!about.draws(), "{}", about.name());
                assert_eq!(about.reads(), None, "{}", about.name());
            }
            for about in order_checks(direction).descriptors() {
                assert!(!about.draws(), "{}", about.name());
                assert_eq!(about.reads(), None, "{}", about.name());
            }
        }
    }

    /// A stall whose only row is a `jünk` at -200, traded both ways, for
    /// a player with `cash` and `free` tons free.
    fn negative(cash: i64, free: u32) -> Market {
        Market {
            rows: vec![row(OPALS, "Opals", -200)],
            ..stall(cash, 0, free)
        }
    }

    /// `market` reading its cash below nothing as `trade_debt` says.
    fn debt_by(trade_debt: RuleSource, market: Market) -> Market {
        Market {
            rules: market.rules.with_override(RuleKey::TradeDebt, trade_debt),
            ..market
        }
    }

    /// The engine's cash / price on the signed cash (@0x5e21d-0x5e234):
    /// -200 at -200 is 1, so a click and a counted lot buy 1 ton; -199 at
    /// -200 truncates to 0, which the `jle` @0x5e27d skips.
    #[test]
    fn by_the_engine_negative_cash_buys_a_negative_price_junk_at_its_price() {
        let buy = |market: &Market, lot| market.tons(order(0, OPALS, Direction::Buy, lot));
        let at_price = negative(-200, 3);
        assert_eq!(
            at_price.rules.source_for(RuleKey::TradeDebt),
            RuleSource::Engine,
            "by default"
        );
        assert_eq!(buy(&at_price, Lot::Click), Ok(1));
        assert_eq!(buy(&at_price, Lot::Max), Ok(1));
        assert_eq!(buy(&at_price, Lot::Count(1)), Ok(1));
        assert_eq!(buy(&at_price, Lot::Count(2)), Err(TradeRefusal::OutOfRange));
        assert_eq!(at_price.row_max(0, Direction::Buy), Some(1));
        assert!(at_price.row_allows(0, Direction::Buy));
        let short = negative(-199, 3);
        for lot in [Lot::Click, Lot::Max, Lot::Count(1)] {
            assert_eq!(buy(&short, lot), Err(TradeRefusal::CannotAfford), "{lot:?}");
        }
        assert_eq!(short.row_max(0, Direction::Buy), Some(0));
        for trade_quotient in RuleSource::ALL {
            let market = quotient_by(trade_quotient, negative(-200, 3));
            assert_eq!(buy(&market, Lot::Max), Ok(1), "{trade_quotient:?}");
        }
        let one_ton = lots_by(RuleSource::Bible, negative(-200, 3));
        assert_eq!(buy(&one_ton, Lot::Click), Ok(1));
    }

    /// `_CanBuyGoods` (@0xcd07-0xcd0d) greys Buy while cash < price, so
    /// the engine never reaches the quotient there, though -1000 at -200
    /// would give 5.
    #[test]
    fn a_buy_with_cash_below_the_price_is_refused_as_buy_is_greyed() {
        for trade_debt in RuleSource::ALL {
            for trade_quotient in RuleSource::ALL {
                let market = quotient_by(trade_quotient, debt_by(trade_debt, negative(-1000, 3)));
                assert!(!market.row_allows(0, Direction::Buy));
                for lot in [Lot::Click, Lot::Max, Lot::Count(1)] {
                    assert_eq!(
                        market.tons(order(0, OPALS, Direction::Buy, lot)),
                        Err(TradeRefusal::CannotAfford),
                        "{trade_debt:?} {trade_quotient:?} {lot:?}"
                    );
                }
            }
        }
        let food = |cash| stall(cash, 0, 50).tons(order(0, FOOD, Direction::Buy, Lot::Count(1)));
        assert_eq!(food(99), Err(TradeRefusal::CannotAfford));
        assert_eq!(food(100), Ok(1));
    }

    #[test]
    fn by_the_other_reading_negative_cash_counts_as_none() {
        let market = debt_by(RuleSource::Bible, negative(-200, 3));
        for lot in [Lot::Click, Lot::Max, Lot::Count(1)] {
            assert_eq!(
                market.tons(order(0, OPALS, Direction::Buy, lot)),
                Err(TradeRefusal::CannotAfford),
                "{lot:?}"
            );
        }
        assert_eq!(market.row_max(0, Direction::Buy), Some(0));
        let exact = quotient_by(RuleSource::Bible, market);
        assert_eq!(exact.row_max(0, Direction::Buy), Some(0));
    }

    /// By the engine, cash below what its 32 bits hold is held at the
    /// least they hold, not the most: still below 0, so a negative price
    /// divides it to a positive quotient.
    #[test]
    fn by_the_engine_cash_below_32_bits_is_held_at_their_least() {
        let market = Market {
            rows: vec![row(OPALS, "Opals", i64::from(i16::MIN))],
            ..stall(i64::from(i32::MIN) - 1, 0, 50)
        };
        assert_eq!(market.row_max(0, Direction::Buy), Some(50));
        let bible = debt_by(RuleSource::Bible, market);
        assert_eq!(bible.row_max(0, Direction::Buy), Some(0));
    }

    /// A row priced at nothing (the other reading of
    /// [`RuleKey::JunkPrice`](crate::RuleKey::JunkPrice)) is greyed with
    /// cash below 0, as `_CanBuyGoods` compares, and now refused so too.
    #[test]
    fn a_good_priced_at_nothing_is_refused_with_cash_below_nothing() {
        for lot in [Lot::Click, Lot::Max, Lot::Count(1)] {
            for trade_debt in RuleSource::ALL {
                let market = debt_by(trade_debt, stall(-5, 0, 9));
                assert!(!market.row_allows(2, Direction::Buy));
                assert_eq!(
                    market.tons(order(2, WATER, Direction::Buy, lot)),
                    Err(TradeRefusal::CannotAfford),
                    "{trade_debt:?} {lot:?}"
                );
            }
        }
    }

    #[test]
    fn row_allows_a_buy_at_a_negative_price_that_then_moves_nothing() {
        let buy = |market: &Market| market.row_allows(0, Direction::Buy);
        for cash in [0, -100, 5000] {
            let market = negative(cash, 3);
            assert!(buy(&market), "cash {cash}");
            for lot in [Lot::Click, Lot::Max] {
                assert_eq!(
                    market.tons(order(0, OPALS, Direction::Buy, lot)),
                    Err(TradeRefusal::CannotAfford),
                    "cash {cash}"
                );
            }
        }
        assert!(!buy(&negative(-500, 3)), "cash below the price");
        assert!(!buy(&negative(-201, 3)), "cash below the price");
        assert!(!buy(&negative(0, 0)), "no space");
    }

    // Counted lots and the engine's maximum.

    /// `market` dividing its cash by a price as `trade_quotient` says.
    fn quotient_by(trade_quotient: RuleSource, market: Market) -> Market {
        Market {
            rules: market
                .rules
                .with_override(RuleKey::TradeQuotient, trade_quotient),
            ..market
        }
    }

    #[test]
    fn a_counted_buy_moves_exactly_its_count_up_to_the_most() {
        let buy = |market: &Market, n| market.tons(order(0, FOOD, Direction::Buy, Lot::Count(n)));
        let market = stall(5000, 0, 70);
        assert_eq!(market.row_max(0, Direction::Buy), Some(50), "5000 at 100");
        assert_eq!(buy(&market, 7), Ok(7));
        assert_eq!(buy(&market, 1), Ok(1));
        assert_eq!(buy(&market, 50), Ok(50), "the most");
        assert_eq!(buy(&market, 51), Err(TradeRefusal::OutOfRange));
        assert_eq!(buy(&market, 0), Err(TradeRefusal::OutOfRange));
        let space = stall(100_000, 0, 7);
        assert_eq!(space.row_max(0, Direction::Buy), Some(7), "the space");
        assert_eq!(buy(&space, 7), Ok(7));
        assert_eq!(buy(&space, 8), Err(TradeRefusal::OutOfRange));
    }

    #[test]
    fn a_counted_buy_is_refused_as_any_buy_before_its_count_is_read() {
        let buy = |market: &Market, n| market.tons(order(0, FOOD, Direction::Buy, Lot::Count(n)));
        assert_eq!(buy(&stall(1000, 0, 0), 1), Err(TradeRefusal::NoSpace));
        assert_eq!(buy(&stall(1000, 0, 0), 0), Err(TradeRefusal::NoSpace));
        assert_eq!(buy(&stall(99, 0, 5), 1), Err(TradeRefusal::CannotAfford));
        assert_eq!(buy(&stall(99, 0, 5), 0), Err(TradeRefusal::CannotAfford));
        let opals = stall(10_000, 5, 5).tons(order(1, OPALS, Direction::Buy, Lot::Count(1)));
        assert_eq!(opals, Err(TradeRefusal::NotTraded));
    }

    #[test]
    fn a_counted_sale_moves_exactly_its_count_up_to_everything_held() {
        let sell = |market: &Market, n| market.tons(order(0, FOOD, Direction::Sell, Lot::Count(n)));
        let market = stall(0, 25, 0);
        assert_eq!(market.row_max(0, Direction::Sell), Some(25));
        assert_eq!(sell(&market, 7), Ok(7));
        assert_eq!(sell(&market, 25), Ok(25));
        assert_eq!(sell(&market, 26), Err(TradeRefusal::OutOfRange));
        assert_eq!(sell(&market, 0), Err(TradeRefusal::OutOfRange));
        assert_eq!(sell(&stall(0, 0, 9), 0), Err(TradeRefusal::NoneHeld));
        assert_eq!(sell(&stall(0, 0, 9), 1), Err(TradeRefusal::NoneHeld));
        let water = stall(0, 5, 5).tons(order(2, WATER, Direction::Sell, Lot::Count(1)));
        assert_eq!(water, Err(TradeRefusal::NotTraded));
    }

    /// The most a sale offers is everything held up to 32000
    /// (`_DoTradeDialog` @0x5e44f-0x5e457), which only tribbles grown
    /// past a full hold can pass.
    #[test]
    fn a_sale_moves_at_most_32000_tons() {
        let sell = |held, lot| stall(0, held, 0).tons(order(0, FOOD, Direction::Sell, lot));
        let most = |held| stall(0, held, 0).row_max(0, Direction::Sell);
        assert_eq!(most(32_001), Some(32_000));
        assert_eq!(most(40_000), Some(32_000));
        assert_eq!(most(32_000), Some(32_000));
        assert_eq!(most(31_999), Some(31_999));
        assert_eq!(sell(32_001, Lot::Max), Ok(32_000));
        assert_eq!(sell(32_000, Lot::Max), Ok(32_000));
        assert_eq!(sell(32_001, Lot::Count(32_000)), Ok(32_000));
        assert_eq!(
            sell(32_001, Lot::Count(32_001)),
            Err(TradeRefusal::OutOfRange)
        );
        assert_eq!(sell(32_001, Lot::Click), Ok(10));
    }

    /// The fleet's free space stops at [`MAX_FLEET_HOLDS`], so only a
    /// market built with more free shows the buy's own cap
    /// (@0x5e23a-0x5e24c).
    #[test]
    fn a_buy_moves_at_most_32000_tons() {
        let rich = |free| stall(100_000_000, 0, free);
        let most = |free| rich(free).row_max(0, Direction::Buy);
        assert_eq!(most(40_000), Some(32_000));
        assert_eq!(most(32_001), Some(32_000));
        assert_eq!(most(32_000), Some(32_000));
        assert_eq!(most(31_999), Some(31_999));
        let buy = |free, lot| rich(free).tons(order(0, FOOD, Direction::Buy, lot));
        assert_eq!(buy(40_000, Lot::Max), Ok(32_000));
        assert_eq!(buy(40_000, Lot::Count(32_000)), Ok(32_000));
        assert_eq!(
            buy(40_000, Lot::Count(32_001)),
            Err(TradeRefusal::OutOfRange)
        );
        let water = |free| stall(0, 0, free).row_max(2, Direction::Buy);
        assert_eq!(water(40_000), Some(32_000), "at no price, by space");
        assert_eq!(water(9), Some(9));
    }

    /// A stall whose only row is food at 32767, the most a price can be,
    /// traded both ways, for a player with `cash` and 2000 tons free.
    fn dear(cash: i64) -> Market {
        Market {
            rows: vec![row(FOOD, "Food", 32_767)],
            ..stall(cash, 0, 2000)
        }
    }

    /// By the engine the cash is divided by the price in single floats
    /// (@0x5e21d-0x5e234): 32,766,999 is 32,767,000 as an `f32`, so the
    /// most is 1000, a ton more than the cash covers.
    #[test]
    fn by_the_engine_the_most_a_buy_moves_is_the_single_float_quotient() {
        let market = dear(32_766_999);
        assert_eq!(
            market.rules.source_for(RuleKey::TradeQuotient),
            RuleSource::Engine
        );
        let buy = |lot| market.tons(order(0, FOOD, Direction::Buy, lot));
        assert_eq!(market.row_max(0, Direction::Buy), Some(1000));
        assert_eq!(buy(Lot::Max), Ok(1000));
        assert_eq!(buy(Lot::Count(1000)), Ok(1000));
        assert_eq!(buy(Lot::Count(1001)), Err(TradeRefusal::OutOfRange));
    }

    #[test]
    fn by_the_other_reading_the_most_a_buy_moves_is_the_integer_quotient() {
        let market = quotient_by(RuleSource::Bible, dear(32_766_999));
        let buy = |lot| market.tons(order(0, FOOD, Direction::Buy, lot));
        assert_eq!(market.row_max(0, Direction::Buy), Some(999));
        assert_eq!(buy(Lot::Max), Ok(999));
        assert_eq!(buy(Lot::Count(999)), Ok(999));
        assert_eq!(buy(Lot::Count(1000)), Err(TradeRefusal::OutOfRange));
    }

    /// Below 2^24 the cash converts exactly, and the quotient never rounds
    /// up to the next ton.
    #[test]
    fn below_2_24_cash_both_quotients_agree() {
        for (cash, price, most) in [
            (16_383_499, 32_767, 499),
            (16_383_500, 32_767, 500),
            (999_999, 1001, 999),
            (32_766, 32_767, 0),
            (100, 100, 1),
        ] {
            let market = Market {
                rows: vec![row(FOOD, "Food", price)],
                ..stall(cash, 0, 2000)
            };
            for source in RuleSource::ALL {
                let found = quotient_by(source, market.clone()).row_max(0, Direction::Buy);
                assert_eq!(found, Some(most), "{cash} at {price} {source:?}");
            }
        }
    }

    /// The engine holds the cash in 32 bits and the price in 16 before it
    /// divides; past those, this port holds each at the most it can hold
    /// rather than wrapping it.
    #[test]
    fn by_the_engine_the_cash_and_price_divided_are_held_at_their_widths() {
        let at = |cash, price| Market {
            rows: vec![row(FOOD, "Food", price)],
            ..stall(cash, 0, 40_000)
        };
        let most = |cash, price| at(cash, price).row_max(0, Direction::Buy);
        assert_eq!(most((1 << 32) + 100_000, 10), Some(32_000), "not 10,000");
        assert_eq!(most(32_767_000, 40_000), Some(1000), "at 32767, not 819");
        assert_eq!(most(32_767_000, -40_000), Some(-999), "at -32768");
        let other = quotient_by(RuleSource::Bible, at(32_767_000, 40_000));
        assert_eq!(other.row_max(0, Direction::Buy), Some(819));
    }

    #[test]
    fn a_click_is_unchanged_by_the_quotient() {
        for source in RuleSource::ALL {
            let market = quotient_by(source, dear(32_766_999));
            let click = market.tons(order(0, FOOD, Direction::Buy, Lot::Click));
            assert_eq!(click, Ok(10), "{source:?}");
            let poor = quotient_by(source, dear(32_767 * 3 - 1));
            let click = poor.tons(order(0, FOOD, Direction::Buy, Lot::Click));
            assert_eq!(click, Ok(2), "{source:?}");
        }
    }

    /// A negative price gives a quotient of 0 or less, so the most a buy
    /// moves is 0 or less and every buy is refused; the dialog would open
    /// on it all the same.
    #[test]
    fn by_the_engine_a_negative_junk_price_offers_a_most_of_0_or_less() {
        let at = |cash| junk_market(-100, ONLY_SOLD, RuleSource::Engine, &pilot(cash));
        assert_eq!(at(0).row_max(0, Direction::Buy), Some(0), "traded at 0");
        assert_eq!(
            at(1000).row_max(0, Direction::Buy),
            Some(-12),
            "1000 at -80"
        );
        for cash in [0, 1000] {
            for lot in [Lot::Max, Lot::Count(1), Lot::Count(0)] {
                assert_eq!(
                    at(cash).tons(order(0, ODD, Direction::Buy, lot)),
                    Err(TradeRefusal::CannotAfford),
                    "{cash} {lot:?}"
                );
            }
        }
    }

    #[test]
    fn row_max_is_none_on_no_row_or_one_not_traded_that_way() {
        let market = stall(10_000, 5, 50);
        assert_eq!(market.row_max(1, Direction::Buy), None, "opals not sold");
        assert_eq!(market.row_max(2, Direction::Sell), None, "water not bought");
        assert_eq!(market.row_max(3, Direction::Buy), None, "no row 3");
        assert_eq!(market.row_max(3, Direction::Sell), None, "no row 3");
        assert_eq!(market.row_max(1, Direction::Sell), Some(5));
        assert_eq!(market.row_max(2, Direction::Buy), Some(50));
        assert_eq!(stall(0, 0, 9).row_max(0, Direction::Sell), Some(0));
    }

    #[test]
    fn row_max_is_what_the_most_moves() {
        for market in [stall(10_000, 5, 50), stall(350, 3, 70), stall(0, 40, 9)] {
            for (index, good) in [(0, FOOD), (1, OPALS), (2, WATER)] {
                for direction in [Direction::Buy, Direction::Sell] {
                    let positive = market.row_max(index, direction).filter(|&most| most > 0);
                    let Some(most) = positive else {
                        continue;
                    };
                    let moved = market.tons(order(index, good, direction, Lot::Max));
                    assert_eq!(moved, Ok(u32::try_from(most).expect("some")), "{good:?}");
                }
            }
        }
    }

    // The tons rules.

    #[test]
    fn click_lot_is_10_by_the_engine_and_1_by_the_other_reading() {
        let market = stall(100_000, 25, 50);
        assert_eq!(click_lot().of(&trade(&market, 0)), 10);
        let one_ton = lots_by(RuleSource::Bible, market);
        assert_eq!(click_lot().of(&trade(&one_ton, 0)), 1);
    }

    #[test]
    fn cash_divided_is_signed_by_the_engine_and_never_below_0_by_the_other() {
        for (cash, engine, other) in [(-5, -5, 0), (0, 0, 0), (500, 500, 500)] {
            let signed = debt_by(RuleSource::Engine, stall(cash, 0, 9));
            assert_eq!(cash_divided().of(&trade(&signed, 0)), engine, "{cash}");
            let none = debt_by(RuleSource::Bible, stall(cash, 0, 9));
            assert_eq!(cash_divided().of(&trade(&none, 0)), other, "{cash}");
        }
    }

    #[test]
    fn quotient_is_single_float_by_the_engine_and_exact_by_the_other() {
        let at = |source, market: Market| {
            let market = quotient_by(source, market);
            quotient().of(&trade(&market, 0))
        };
        assert_eq!(at(RuleSource::Engine, dear(32_766_999)), Some(1000));
        assert_eq!(at(RuleSource::Bible, dear(32_766_999)), Some(999));
        for source in RuleSource::ALL {
            let at_nothing = quotient_by(source, stall(500, 0, 9));
            assert_eq!(quotient().of(&trade(&at_nothing, 2)), None, "{source:?}");
            assert_eq!(at(source, negative(-200, 3)), Some(1), "{source:?}");
            let none = debt_by(RuleSource::Bible, negative(-200, 3));
            assert_eq!(at(source, none), Some(0), "{source:?}");
            assert_eq!(at(source, negative(1000, 3)), Some(-5), "{source:?}");
        }
    }

    #[test]
    fn click_quotient_is_exact_whatever_the_quotient_rule() {
        for source in RuleSource::ALL {
            let at = |market: Market, index| {
                let market = quotient_by(source, market);
                click_quotient().of(&trade(&market, index))
            };
            assert_eq!(at(dear(32_766_999), 0), Some(999), "{source:?}");
            assert_eq!(at(stall(399, 0, 9), 0), Some(3), "{source:?}");
            assert_eq!(at(stall(500, 0, 9), 2), None, "{source:?}");
            assert_eq!(at(negative(-200, 3), 0), Some(1), "{source:?}");
            let none = debt_by(RuleSource::Bible, negative(-200, 3));
            assert_eq!(at(none, 0), Some(0), "{source:?}");
        }
    }

    #[test]
    fn buy_most_is_the_quotient_or_the_space_at_most_32000() {
        let at = |market: Market, index| buy_most().of(&trade(&market, index));
        assert_eq!(at(stall(5000, 0, 70), 0), 50, "the quotient");
        assert_eq!(at(stall(100_000, 0, 7), 0), 7, "the space");
        assert_eq!(at(stall(100_000_000, 0, 40_000), 0), 32_000);
        assert_eq!(at(stall(100_000_000, 0, 32_001), 0), 32_000);
        assert_eq!(at(stall(100_000_000, 0, 31_999), 0), 31_999);
        assert_eq!(at(stall(0, 0, 9), 2), 9, "at no price, the space");
        assert_eq!(at(stall(0, 0, 40_000), 2), 32_000);
        assert_eq!(at(negative(1000, 3), 0), -5, "below nothing");
        assert_eq!(at(dear(32_766_999), 0), 1000, "by the engine");
        assert_eq!(at(quotient_by(RuleSource::Bible, dear(32_766_999)), 0), 999);
    }

    #[test]
    fn sell_most_is_everything_held_at_most_32000() {
        let at = |held| sell_most().of(&trade(&stall(0, held, 0), 0));
        assert_eq!(at(25), 25);
        assert_eq!(at(0), 0);
        assert_eq!(at(31_999), 31_999);
        assert_eq!(at(32_000), 32_000);
        assert_eq!(at(40_000), 32_000);
    }

    /// `lot` of row `index` of `market`, `direction`, as the order rules
    /// read it.
    fn row_order(market: &Market, index: usize, direction: Direction, lot: Lot) -> RowOrder<'_> {
        RowOrder {
            trade: trade(market, index),
            direction,
            lot,
        }
    }

    #[test]
    fn tons_sizes_a_click_the_most_and_a_count() {
        let buy =
            |market: &Market, index, lot| tons().of(&row_order(market, index, Direction::Buy, lot));
        let sell = |market: &Market, lot| tons().of(&row_order(market, 0, Direction::Sell, lot));
        assert_eq!(buy(&stall(100_000, 0, 50), 0, Lot::Click), 10, "10");
        assert_eq!(buy(&stall(399, 0, 50), 0, Lot::Click), 3, "the cash");
        assert_eq!(buy(&stall(100_000, 0, 4), 0, Lot::Click), 4, "the space");
        assert_eq!(buy(&stall(0, 0, 9), 2, Lot::Click), 9, "at no price");
        assert_eq!(buy(&negative(-199, 3), 0, Lot::Click), 0, "0 at -200");
        let one_ton = lots_by(RuleSource::Bible, stall(100_000, 25, 50));
        assert_eq!(buy(&one_ton, 0, Lot::Click), 1);
        assert_eq!(sell(&one_ton, Lot::Click), 1);
        assert_eq!(sell(&stall(0, 25, 0), Lot::Click), 10);
        assert_eq!(sell(&stall(0, 4, 0), Lot::Click), 4);
        assert_eq!(buy(&stall(5000, 0, 70), 0, Lot::Max), 50);
        assert_eq!(sell(&stall(0, 25, 0), Lot::Max), 25);
        assert_eq!(sell(&stall(0, 40_000, 0), Lot::Max), 32_000);
        assert_eq!(buy(&stall(5000, 0, 70), 0, Lot::Count(7)), 7);
        assert_eq!(sell(&stall(0, 25, 0), Lot::Count(3)), 3);
    }

    #[test]
    fn the_order_checks_refuse_a_ton_before_the_count() {
        let named = |direction| -> Vec<&str> {
            order_checks(direction)
                .descriptors()
                .map(Descriptor::name)
                .collect()
        };
        assert_eq!(named(Direction::Buy), ["moves a ton", "count in range"]);
        assert_eq!(named(Direction::Sell), ["count in range"]);
        let refuses = Verdict::Refuses;
        let buy = |market: &Market, lot| {
            verdicts(
                &order_checks(Direction::Buy),
                &row_order(market, 0, Direction::Buy, lot),
            )
        };
        let poor = stall(99, 0, 5);
        assert_eq!(
            buy(&poor, Lot::Count(0)),
            [
                refuses(TradeRefusal::CannotAfford),
                refuses(TradeRefusal::OutOfRange),
            ]
        );
        assert_eq!(
            refusal(
                &order_checks(Direction::Buy),
                &row_order(&poor, 0, Direction::Buy, Lot::Count(0))
            ),
            Err(TradeRefusal::CannotAfford)
        );
        assert_eq!(
            buy(&poor, Lot::Count(1)),
            [
                refuses(TradeRefusal::CannotAfford),
                refuses(TradeRefusal::OutOfRange),
            ],
            "1 is past a most of 0"
        );
        assert_eq!(buy(&poor, Lot::Max), [refuses(TradeRefusal::CannotAfford)]);
        assert_eq!(
            buy(&negative(-199, 3), Lot::Click),
            [refuses(TradeRefusal::CannotAfford)]
        );
        let rich = stall(5000, 0, 70);
        assert_eq!(
            buy(&rich, Lot::Count(51)),
            [refuses(TradeRefusal::OutOfRange)]
        );
        assert_eq!(
            buy(&rich, Lot::Count(0)),
            [refuses(TradeRefusal::OutOfRange)]
        );
        for lot in [Lot::Click, Lot::Max, Lot::Count(1), Lot::Count(50)] {
            assert_eq!(buy(&rich, lot), [], "{lot:?}");
        }
        let sell = |lot| {
            let holding = stall(0, 25, 0);
            verdicts(
                &order_checks(Direction::Sell),
                &row_order(&holding, 0, Direction::Sell, lot),
            )
        };
        assert_eq!(sell(Lot::Count(26)), [refuses(TradeRefusal::OutOfRange)]);
        assert_eq!(sell(Lot::Count(0)), [refuses(TradeRefusal::OutOfRange)]);
        for lot in [Lot::Click, Lot::Max, Lot::Count(1), Lot::Count(25)] {
            assert_eq!(sell(lot), [], "{lot:?}");
        }
    }

    #[test]
    fn an_order_reads_its_rows_rule_set() {
        let market = Market {
            rules: Rulebook::new(RuleSource::Bible),
            ..stall(0, 0, 0)
        };
        let order = row_order(&market, 0, Direction::Buy, Lot::Click);
        assert_eq!(order.rulebook(), &Rulebook::new(RuleSource::Bible));
        assert_eq!(
            trade(&market, 0).rulebook(),
            &Rulebook::new(RuleSource::Bible)
        );
    }

    #[test]
    fn the_snapshot_reads_its_rules_from_its_rule_set() {
        let engine = dear(32_766_999);
        let bible = Market {
            rules: Rulebook::new(RuleSource::Bible),
            ..engine.clone()
        };
        let buy = |market: &Market, lot| market.tons(order(0, FOOD, Direction::Buy, lot));
        assert_eq!(buy(&engine, Lot::Click), Ok(10));
        assert_eq!(buy(&bible, Lot::Click), Ok(1));
        assert_eq!(buy(&engine, Lot::Max), Ok(1000));
        assert_eq!(buy(&bible, Lot::Max), Ok(999));
        assert_eq!(engine.row_max(0, Direction::Buy), Some(1000));
        assert_eq!(bible.row_max(0, Direction::Buy), Some(999));
        assert!(engine.option_asks_count());
        assert!(!bible.option_asks_count());
        let in_debt = negative(-200, 3);
        let none = Market {
            rules: Rulebook::new(RuleSource::Bible),
            ..in_debt.clone()
        };
        let opals = order(0, OPALS, Direction::Buy, Lot::Click);
        assert_eq!(in_debt.tons(opals), Ok(1));
        assert_eq!(none.tons(opals), Err(TradeRefusal::CannotAfford));
    }

    #[test]
    fn option_asks_count_by_the_engine_and_not_by_the_other_reading() {
        let market = stall(1000, 5, 5);
        assert!(option_asks_count().of(&market));
        assert!(market.option_asks_count());
        let at_once = Market {
            rules: market
                .rules
                .with_override(RuleKey::TradeCount, RuleSource::Bible),
            ..market.clone()
        };
        assert!(!option_asks_count().of(&at_once));
        assert!(!at_once.option_asks_count());
        assert_eq!(
            option_asks_count().about().reproduces(),
            "`_DoTradeDialog` @0x5e211, @0x5e45c; `_DoQuantityDialog` @0x56d5b"
        );
    }

    #[test]
    fn the_tons_rules_cite_the_originals_addresses() {
        let cited = [
            click_lot().about().reproduces(),
            cash_divided().about().reproduces(),
            quotient().about().reproduces(),
            click_quotient().about().reproduces(),
            buy_most().about().reproduces(),
            sell_most().about().reproduces(),
        ];
        assert_eq!(
            cited,
            [
                "`_DoTradeDialog` @0x5e268-0x5e278, @0x5e48d-0x5e4fa",
                "`_DoTradeDialog` @0x5e21d-0x5e234",
                "`_DoTradeDialog` @0x5e21d-0x5e234",
                "`_DoTradeDialog` @0x5e268-0x5e278",
                "`_DoTradeDialog` @0x5e23a-0x5e25a",
                "`_DoTradeDialog` @0x5e44f-0x5e47f",
            ]
        );
        let order_cited: Vec<_> = [Direction::Buy, Direction::Sell]
            .into_iter()
            .flat_map(|direction| {
                order_checks(direction)
                    .descriptors()
                    .map(Descriptor::reproduces)
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(
            order_cited,
            [
                "`_DoTradeDialog` `jle` @0x5e27d",
                "`_DoTradeDialog` `jle` @0x5e27d skips none; the quantity dialog gives no more than the most",
                "`_DoTradeDialog` `jle` @0x5e4d8 skips none; the quantity dialog gives no more than the most",
            ]
        );
    }

    #[test]
    fn settling_a_buy_pays_and_loads_and_a_sale_unloads_and_is_paid() {
        let mut pilot = pilot(1000);
        settle(&mut pilot, order(0, FOOD, Direction::Buy, Lot::Max), 4, 93);
        assert_eq!(pilot.cash, 1000 - 372);
        assert_eq!(pilot.held(FOOD), 4);
        settle(
            &mut pilot,
            order(1, OPALS, Direction::Buy, Lot::Click),
            1,
            0,
        );
        assert_eq!(pilot.held(OPALS), 1);
        settle(
            &mut pilot,
            order(0, FOOD, Direction::Sell, Lot::Click),
            1,
            150,
        );
        assert_eq!(pilot.cash, 1000 - 372 + 150);
        assert_eq!(pilot.held(FOOD), 3);
        settle(
            &mut pilot,
            order(0, FOOD, Direction::Sell, Lot::Max),
            3,
            100,
        );
        assert_eq!(pilot.cash, 1000 - 372 + 150 + 300);
        assert_eq!(pilot.held(FOOD), 0);
        assert_eq!(
            pilot.cargo,
            BTreeMap::from([(OPALS, 1)]),
            "a good sold out is gone"
        );
    }

    #[test]
    fn settling_never_overflows() {
        let mut pilot = pilot(i64::MAX - 5);
        settle(&mut pilot, order(0, FOOD, Direction::Sell, Lot::Max), 0, 10);
        assert_eq!(pilot.cash, i64::MAX - 5);
        pilot.cargo.insert(FOOD, u32::MAX);
        settle(
            &mut pilot,
            order(0, FOOD, Direction::Sell, Lot::Max),
            u32::MAX,
            i64::MAX,
        );
        assert_eq!(pilot.cash, i64::MAX);
        pilot.cargo.insert(OPALS, u32::MAX);
        settle(
            &mut pilot,
            order(1, OPALS, Direction::Buy, Lot::Click),
            5,
            0,
        );
        assert_eq!(pilot.held(OPALS), u32::MAX);
    }

    // Events.

    #[test]
    fn the_event_stellars_are_the_inhabited_stellars_128_to_2175_by_id() {
        let stellars = [
            (StellarId(2175), TRADE),
            (StellarId(150), 0),
            (StellarId(127), TRADE),
            (StellarId(141), TRADE | StellarFlags::UNINHABITED),
            (StellarId(2176), TRADE),
            (StellarId(128), StellarFlags::CAN_LAND),
            (StellarId(150), 0),
        ];
        assert_eq!(
            event_stellars(&stellars),
            [StellarId(128), StellarId(150), StellarId(2175)]
        );
        assert_eq!(event_stellars(&[]), []);
    }

    fn days(events: &BTreeMap<DisasterId, ActiveEvent>) -> Vec<(i16, u16)> {
        events
            .iter()
            .map(|(id, active)| (id.0, active.days))
            .collect()
    }

    /// Event `id`, at Earth with `days` left.
    fn on_earth(id: i16, days: u16) -> (DisasterId, ActiveEvent) {
        let stellar = Some(EARTH);
        (DisasterId(id), ActiveEvent { days, stellar })
    }

    #[test]
    fn an_event_starts_when_its_chance_fires_for_its_duration() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true, false, true]);
        step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, [35, 40, 50], "each öops by ID, at its Freq");
        assert_eq!(days(&events), [(128, 30), (130, 25)]);
        assert_eq!(
            events
                .values()
                .map(|active| active.stellar)
                .collect::<Vec<_>>(),
            [Some(StellarId(140)); 2],
            "each at its record's stellar"
        );
        assert_eq!(
            chance.sides_asked,
            Vec::<u16>::new(),
            "a fixed stellar draws nothing"
        );
    }

    #[test]
    fn an_event_whose_chance_does_not_fire_does_not_start() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::default();
        for _ in 0..10 {
            step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        }
        assert_eq!(days(&events), []);
        assert_eq!(chance.asked.len(), 30);
    }

    #[test]
    fn an_active_event_ages_a_day_at_a_time_and_is_not_rolled_again() {
        let mut events = BTreeMap::from([on_earth(128, 3)]);
        let mut chance = Scripted::default();
        step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(days(&events), [(128, 2)]);
        assert_eq!(chance.asked, [40, 50], "not the active one");
        step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(days(&events), [(128, 1)]);
        step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(days(&events), [], "over");
        assert_eq!(
            chance.asked,
            [40, 50, 40, 50, 40, 50],
            "not rolled the day it ends"
        );
    }

    #[test]
    fn an_event_with_no_days_left_is_rolled_that_day() {
        let mut events = BTreeMap::from([on_earth(128, 0)]);
        let mut chance = Scripted::answering(&[true]);
        step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, [35, 40, 50], "as the original's slot at 0");
        assert_eq!(days(&events), [(128, 30)]);
    }

    #[test]
    fn an_event_that_ends_on_a_day_is_rolled_the_next() {
        let mut events = BTreeMap::from([on_earth(128, 1)]);
        let mut chance = Scripted::answering(&[true; 3]);
        step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, [40, 50]);
        assert!(!events.contains_key(&DisasterId(128)), "over");
        step_day(&goods(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, [40, 50, 35], "rolled first the next day");
        assert_eq!(
            events.get(&DisasterId(128)).map(|active| active.days),
            Some(30)
        );
    }

    #[test]
    fn an_event_lasts_its_duration_in_days() {
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(128),
                stellar: 140,
                duration: 2,
                freq: 100,
                ..DisasterRecord::default()
            }],
        );
        let mut events = BTreeMap::new();
        let mut once = Scripted::answering(&[true]);
        let mut active = Vec::new();
        for _ in 0..4 {
            step_day(&goods, &mut events, Gate::FRESH, &mut once);
            active.push(events.contains_key(&DisasterId(128)));
        }
        assert_eq!(active, [true, true, false, false]);
    }

    #[test]
    fn an_event_whose_öops_is_gone_ends() {
        let mut events = BTreeMap::from([on_earth(999, 20), on_earth(128, 20)]);
        step_day(&goods(), &mut events, Gate::FRESH, &mut Scripted::default());
        assert_eq!(days(&events), [(128, 19)]);
    }

    /// An event at Earth on food, 10 days, `freq` %, starting on
    /// `activate_on`.
    fn activating(freq: i16, activate_on: &str) -> Goods {
        let event = DisasterRecord {
            id: DisasterId(128),
            stellar: 140,
            commodity: 0,
            duration: 10,
            freq,
            price_delta: 10,
            activate_on: Test::parse(activate_on),
            ..DisasterRecord::default()
        };
        Goods::new(&stock(), Vec::new(), vec![event])
    }

    #[test]
    fn an_event_whose_activate_on_is_refused_is_never_rolled() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]);
        step_day(&activating(100, "b7"), &mut events, REFUSING_7, &mut chance);
        assert!(events.is_empty());
        assert_eq!(chance.asked, Vec::<u8>::new(), "no draw is asked");
        step_day(
            &activating(100, "b7"),
            &mut events,
            Gate::FRESH,
            &mut chance,
        );
        assert!(events.is_empty(), "bit 7 is clear");
        step_day(
            &activating(100, "!b7"),
            &mut events,
            Gate::FRESH,
            &mut chance,
        );
        assert_eq!(days(&events), [(128, 10)]);
        assert_eq!(chance.asked, [100]);
    }

    #[test]
    fn a_malformed_activate_on_never_starts_its_event() {
        let allowing = Gate {
            control_bits: &AllowAll,
            ..Gate::FRESH
        };
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]);
        step_day(&activating(100, "b1 &"), &mut events, allowing, &mut chance);
        assert!(events.is_empty());
        assert_eq!(chance.asked, Vec::<u8>::new());
    }

    #[test]
    fn an_active_event_runs_out_its_days_once_its_activate_on_stops_holding() {
        let mut events = BTreeMap::from([on_earth(128, 3)]);
        let mut chance = Scripted::answering(&[]);
        step_day(&activating(100, "b7"), &mut events, REFUSING_7, &mut chance);
        assert_eq!(days(&events), [(128, 2)]);
    }

    #[test]
    fn events_that_cannot_start_are_never_rolled() {
        let event = |id, stellar, commodity, duration, freq| DisasterRecord {
            id: DisasterId(id),
            stellar,
            commodity,
            duration,
            freq,
            price_delta: 10,
            ..DisasterRecord::default()
        };
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![
                event(128, 127, 0, 10, 50),
                event(129, -2, 0, 10, 50),
                event(130, 140, 0, 10, 0),
                event(131, 140, 0, 10, -5),
                event(132, 140, 0, 0, 50),
                event(133, 140, 0, -3, 50),
                event(136, 140, 5, 10, 250),
                event(137, 0, 0, 10, 50),
            ],
        )
        .with_stellars(&[(StellarId(140), TRADE)]);
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]);
        step_day(&goods, &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, [100], "only 136, its Freq clamped");
        assert_eq!(days(&events), [(136, 10)]);
    }

    #[test]
    fn event_commodity_is_the_loaders_reading() {
        // _LoadObjectData @0x7cd56-0x7cd6f.
        for (raw, read) in [
            (-7, None),
            (-1, None),
            (0, Some(0)),
            (3, Some(3)),
            (5, Some(5)),
            (6, Some(5)),
            (300, Some(5)),
        ] {
            assert_eq!(event_commodity(raw), read, "{raw}");
        }
    }

    #[test]
    fn an_event_on_any_commodity_starts() {
        let event = |id, commodity| DisasterRecord {
            id: DisasterId(id),
            stellar: 140,
            commodity,
            duration: 7,
            freq: 100,
            price_delta: 10,
            ..DisasterRecord::default()
        };
        let goods = Goods::new(&stock(), Vec::new(), vec![event(128, -1), event(129, 9)])
            .with_stellars(&[(StellarId(140), TRADE)]);
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true, true]);
        step_day(&goods, &mut events, Gate::FRESH, &mut chance);
        assert_eq!(
            chance.asked,
            [100, 100],
            "_DisasterHandler never tests the commodity"
        );
        assert_eq!(days(&events), [(128, 7), (129, 7)]);
    }

    /// A food event 128 at any stellar (Freq 50, Duration 10), with
    /// candidates 150 and 140, and the uninhabited 141.
    fn anywhere() -> Goods {
        Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(128),
                stellar: -1,
                price_delta: -15,
                duration: 10,
                freq: 50,
                ..DisasterRecord::default()
            }],
        )
        .with_stellars(&[
            (StellarId(150), TRADE),
            (StellarId(140), 0),
            (StellarId(141), TRADE | StellarFlags::UNINHABITED),
        ])
    }

    /// Event 128 at `stellar` with `days` left.
    fn placed(stellar: i16, days: u16) -> BTreeMap<DisasterId, ActiveEvent> {
        let stellar = Some(StellarId(stellar));
        BTreeMap::from([(DisasterId(128), ActiveEvent { days, stellar })])
    }

    #[test]
    fn an_any_stellar_event_starts_at_the_stellar_its_roll_picks() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]).and_rolling(&[1]);
        step_day(&anywhere(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, [50]);
        assert_eq!(chance.sides_asked, [2], "140 and 150, not the uninhabited");
        assert_eq!(events, placed(150, 10));
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]).and_rolling(&[0]);
        step_day(&anywhere(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(events, placed(140, 10));
    }

    #[test]
    fn an_any_stellar_event_whose_roll_is_out_of_range_does_not_start() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]).and_rolling(&[2]);
        step_day(&anywhere(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.sides_asked, [2]);
        assert_eq!(events, BTreeMap::new());
    }

    #[test]
    fn an_any_stellar_event_whose_chance_does_not_fire_draws_no_stellar() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::default();
        step_day(&anywhere(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, [50]);
        assert_eq!(chance.sides_asked, Vec::<u16>::new());
        assert_eq!(events, BTreeMap::new());
    }

    #[test]
    fn an_any_stellar_event_with_no_candidate_is_never_rolled() {
        let goods = Goods::new(&stock(), Vec::new(), anywhere().disasters);
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]);
        step_day(&goods, &mut events, Gate::FRESH, &mut chance);
        assert_eq!(chance.asked, Vec::<u8>::new());
        assert_eq!(chance.sides_asked, Vec::<u16>::new());
        assert_eq!(events, BTreeMap::new());
    }

    #[test]
    fn an_any_stellar_event_keeps_its_stellar_as_it_ages() {
        let mut events = placed(150, 3);
        let mut chance = Scripted::answering(&[true, true]).and_rolling(&[0, 0]);
        step_day(&anywhere(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(events, placed(150, 2));
        step_day(&anywhere(), &mut events, Gate::FRESH, &mut chance);
        assert_eq!(events, placed(150, 1));
        assert_eq!(chance.sides_asked, Vec::<u16>::new(), "never drawn again");
    }

    #[test]
    fn an_event_never_lasts_longer_than_a_day_count_holds() {
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(128),
                stellar: 140,
                duration: i16::MAX,
                freq: 1,
                ..DisasterRecord::default()
            }],
        );
        let mut events = BTreeMap::new();
        step_day(
            &goods,
            &mut events,
            Gate::FRESH,
            &mut Scripted::answering(&[true]),
        );
        assert_eq!(days(&events), [(128, 32_767)]);
    }

    // Tribbles and perishables: the frame counter.

    #[test]
    fn the_frame_counter_counts_up_to_1024_and_wraps_to_0() {
        let moved: Vec<_> = [-15, -1, 0, 1, 1023, 1024].map(next_frame).to_vec();
        assert_eq!(moved, [-14, 0, 1, 2, 1024, 0]);
    }

    #[test]
    fn a_frame_is_due_when_the_counter_is_a_multiple_of_250() {
        for frame in [0, 250, 500, 750, 1000] {
            assert!(junk_step_due(frame), "{frame}");
        }
        for frame in [-15, -1, 1, 249, 251, 1024] {
            assert!(!junk_step_due(frame), "{frame}");
        }
    }

    #[test]
    fn after_a_take_off_the_steps_fall_on_ticks_15_265_515_765_1015_and_1040() {
        let mut frame = AFTER_TAKE_OFF_FRAME;
        let mut due = Vec::new();
        for tick in 1..=1100 {
            if junk_step_due(frame) {
                due.push(tick);
            }
            frame = next_frame(frame);
        }
        assert_eq!(due, [15, 265, 515, 765, 1015, 1040]);
    }

    #[test]
    fn the_frame_constants_are_the_originals() {
        assert_eq!(
            (JUNK_STEP_FRAMES, LAST_FRAME, AFTER_TAKE_OFF_FRAME),
            (250, 1024, -14)
        );
        assert_eq!((TRIBBLES, PERISHABLE), (0x0001, 0x0002));
    }

    // Tribbles and perishables: the step.

    const BREEDING: Good = Good::Junk(JunkId(128));
    const ROTTING: Good = Good::Junk(JunkId(129));
    const BOTH: Good = Good::Junk(JunkId(130));
    const PLAIN: Good = Good::Junk(JunkId(131));
    const BREEDING_TOO: Good = Good::Junk(JunkId(132));

    /// `jünk` 128 and 132 tribbles, 129 perishable, 130 both and 131
    /// neither.
    fn flagged() -> Goods {
        let junk = [(128, TRIBBLES), (129, PERISHABLE)]
            .into_iter()
            .chain([(130, TRIBBLES | PERISHABLE), (131, 0), (132, TRIBBLES)])
            .map(|(id, flags)| JunkRecord {
                id: JunkId(id),
                flags,
                ..unlisted()
            })
            .collect();
        Goods::new(&stock(), junk, Vec::new())
    }

    /// `held` after one step in a hold of 10 tons by `source`, and whether
    /// the step changed it.
    fn stepped(source: RuleSource, held: &[(Good, u32)]) -> (Vec<(Good, u32)>, bool) {
        stepped_in(10, source, held)
    }

    /// `held` after one step in a hold of `capacity` tons by `source`,
    /// and whether the step changed it.
    fn stepped_in(
        capacity: u32,
        source: RuleSource,
        held: &[(Good, u32)],
    ) -> (Vec<(Good, u32)>, bool) {
        stepped_beside(capacity, 0, source, held)
    }

    /// `held` after one step in a hold of `capacity` tons that also
    /// carries `mission` tons of mission cargo, by `source`, and whether
    /// the step changed it.
    fn stepped_beside(
        capacity: u32,
        mission: u32,
        source: RuleSource,
        held: &[(Good, u32)],
    ) -> (Vec<(Good, u32)>, bool) {
        let cargo: BTreeMap<Good, u32> = held.iter().copied().collect();
        let latches = JunkLatches::of(&flagged(), &cargo);
        stepped_latched(capacity, mission, source, latches, held)
    }

    /// `held` after one step in a hold of `capacity` tons that also
    /// carries `mission` tons of mission cargo, by `source`, with the
    /// `jünk` latches as `latches` left them, and whether the step
    /// changed it.
    fn stepped_latched(
        capacity: u32,
        mission: u32,
        source: RuleSource,
        latches: JunkLatches,
        held: &[(Good, u32)],
    ) -> (Vec<(Good, u32)>, bool) {
        let mut cargo: BTreeMap<Good, u32> = held.iter().copied().collect();
        let changed = step_junk(&flagged(), &mut cargo, capacity, mission, source, latches);
        (cargo.into_iter().collect(), changed)
    }

    const NEITHER: JunkLatches = JunkLatches {
        tribbles: false,
        perishable: false,
    };
    const TRIBBLES_ONLY: JunkLatches = JunkLatches {
        tribbles: true,
        perishable: false,
    };
    const PERISHABLE_ONLY: JunkLatches = JunkLatches {
        tribbles: false,
        perishable: true,
    };
    const BOTH_LATCHES: JunkLatches = JunkLatches {
        tribbles: true,
        perishable: true,
    };

    #[test]
    fn the_latches_are_set_from_the_junk_held_above_none() {
        let latched =
            |held: &[(Good, u32)]| JunkLatches::of(&flagged(), &held.iter().copied().collect());
        assert_eq!(latched(&[]), NEITHER);
        assert_eq!(latched(&[(FOOD, 3), (PLAIN, 2)]), NEITHER);
        assert_eq!(latched(&[(BREEDING, 1)]), TRIBBLES_ONLY, "@0xc547");
        assert_eq!(latched(&[(ROTTING, 1)]), PERISHABLE_ONLY, "@0xc556");
        assert_eq!(
            latched(&[(BOTH, 1)]),
            TRIBBLES_ONLY,
            "a good of both flags sets only the tribbles latch"
        );
        assert_eq!(
            latched(&[(BREEDING, 1), (ROTTING, 2), (BOTH, 3)]),
            BOTH_LATCHES
        );
        assert_eq!(
            latched(&[(BREEDING, 0), (ROTTING, 0)]),
            NEITHER,
            "a good entered at none is not held"
        );
        assert_eq!(
            latched(&[(Good::Junk(JunkId(999)), 4)]),
            NEITHER,
            "no record"
        );
    }

    #[test]
    fn by_the_engine_a_clear_tribbles_latch_grows_nothing() {
        assert_eq!(
            stepped_latched(10, 0, ENGINE, NEITHER, &[(BREEDING, 3), (BOTH, 2)]),
            (vec![(BREEDING, 3), (BOTH, 2)], false)
        );
        assert_eq!(
            stepped_latched(10, 0, ENGINE, PERISHABLE_ONLY, &[(BREEDING, 3), (BOTH, 2)]),
            (vec![(BREEDING, 3), (BOTH, 1)], true),
            "decayed only"
        );
    }

    #[test]
    fn by_the_engine_a_set_perishable_latch_decays_a_both_flags_good_alone() {
        assert_eq!(
            stepped_latched(10, 0, ENGINE, BOTH_LATCHES, &[(BOTH, 5)]),
            (vec![(BOTH, 5)], false),
            "grown and decayed, no perishable-only good aboard"
        );
        assert_eq!(
            stepped_latched(10, 0, ENGINE, TRIBBLES_ONLY, &[(BOTH, 5)]),
            (vec![(BOTH, 6)], true),
            "grown only"
        );
        assert_eq!(
            stepped_latched(10, 0, ENGINE, TRIBBLES_ONLY, &[(ROTTING, 5)]),
            (vec![(ROTTING, 5)], false),
            "a clear perishable latch decays nothing, a perishable-only good aboard or not"
        );
    }

    #[test]
    fn by_the_engine_the_latches_only_decide_whether_each_loop_runs() {
        assert_eq!(
            stepped_latched(10, 0, ENGINE, BOTH_LATCHES, &[(FOOD, 2)]),
            (vec![(FOOD, 2)], false),
            "the goods each loop visits are read live"
        );
        assert_eq!(
            stepped_latched(
                10,
                0,
                ENGINE,
                BOTH_LATCHES,
                &[(FOOD, 6), (BREEDING, 2), (ROTTING, 2)]
            ),
            (vec![(FOOD, 6), (BREEDING, 2), (ROTTING, 2)], false),
            "the hold full"
        );
    }

    #[test]
    fn by_the_bible_the_latches_change_nothing() {
        let held = [(BREEDING, 3), (ROTTING, 3), (BOTH, 3)];
        let live = stepped_latched(10, 0, BIBLE, BOTH_LATCHES, &held);
        for latches in [NEITHER, TRIBBLES_ONLY, PERISHABLE_ONLY] {
            assert_eq!(
                stepped_latched(10, 0, BIBLE, latches, &held),
                live,
                "{latches:?}"
            );
        }
    }

    const ENGINE: RuleSource = RuleSource::Engine;
    const BIBLE: RuleSource = RuleSource::Bible;

    #[test]
    fn by_the_engine_a_tribbles_good_held_gains_a_ton_while_there_is_space() {
        assert_eq!(
            stepped(ENGINE, &[(FOOD, 2), (BREEDING, 3)]),
            (vec![(FOOD, 2), (BREEDING, 4)], true),
            "the tribbles good not held stays absent"
        );
        assert_eq!(
            stepped(ENGINE, &[(FOOD, 7), (BREEDING, 3)]),
            (vec![(FOOD, 7), (BREEDING, 3)], false),
            "the hold full"
        );
        assert_eq!(
            stepped(ENGINE, &[(FOOD, 8), (BREEDING, 3)]),
            (vec![(FOOD, 8), (BREEDING, 3)], false),
            "the hold over"
        );
    }

    #[test]
    fn by_the_engine_the_mission_cargo_aboard_takes_the_tribbles_space() {
        assert_eq!(
            stepped_beside(10, 6, ENGINE, &[(BREEDING, 4)]),
            (vec![(BREEDING, 4)], false),
            "the mission cargo fills the hold"
        );
        assert_eq!(
            stepped_beside(10, 5, ENGINE, &[(BREEDING, 4)]),
            (vec![(BREEDING, 5)], true),
            "a ton left"
        );
        assert_eq!(
            stepped_beside(10, 20, ENGINE, &[(BREEDING, 4)]),
            (vec![(BREEDING, 4)], false),
            "the mission cargo past the hold"
        );
    }

    #[test]
    fn by_the_engine_the_mission_cargo_aboard_filling_the_hold_stops_the_decay() {
        assert_eq!(
            stepped_beside(10, 3, ENGINE, &[(BREEDING, 4), (ROTTING, 3)]),
            (vec![(BREEDING, 4), (ROTTING, 3)], false)
        );
        assert_eq!(
            stepped_beside(10, 2, ENGINE, &[(BREEDING, 4), (ROTTING, 3)]),
            (vec![(BREEDING, 5), (ROTTING, 2)], true),
            "a ton left"
        );
    }

    #[test]
    fn by_the_bible_the_mission_cargo_aboard_takes_the_tribbles_space_but_not_the_decay() {
        assert_eq!(
            stepped_beside(10, 3, BIBLE, &[(BREEDING, 3), (BREEDING_TOO, 3)]),
            (vec![(BREEDING, 4), (BREEDING_TOO, 3)], true),
            "only the lower ID"
        );
        assert_eq!(
            stepped_beside(10, 7, BIBLE, &[(ROTTING, 3)]),
            (vec![(ROTTING, 2)], true),
            "the mission cargo fills the hold"
        );
    }

    #[test]
    fn by_the_engine_every_tribbles_good_grows_on_the_space_measured_once() {
        assert_eq!(
            stepped(ENGINE, &[(BREEDING, 4), (BREEDING_TOO, 5)]),
            (vec![(BREEDING, 5), (BREEDING_TOO, 6)], true),
            "11 tons of 10"
        );
    }

    #[test]
    fn by_the_engine_a_perishable_good_held_loses_a_ton_while_there_is_space() {
        assert_eq!(stepped(ENGINE, &[(ROTTING, 3)]), (vec![(ROTTING, 2)], true));
        assert_eq!(
            stepped(ENGINE, &[(FOOD, 7), (ROTTING, 3)]),
            (vec![(FOOD, 7), (ROTTING, 3)], false),
            "the hold full"
        );
        assert_eq!(
            stepped(ENGINE, &[(FOOD, 2), (ROTTING, 1)]),
            (vec![(FOOD, 2)], true),
            "a good decayed to none is gone"
        );
    }

    #[test]
    fn by_the_engine_perishables_decay_on_the_space_measured_before_the_tribbles_grew() {
        assert_eq!(
            stepped(ENGINE, &[(BREEDING, 4), (ROTTING, 5)]),
            (vec![(BREEDING, 5), (ROTTING, 4)], true)
        );
        assert_eq!(
            stepped(ENGINE, &[(BREEDING, 5), (ROTTING, 5)]),
            (vec![(BREEDING, 5), (ROTTING, 5)], false),
            "the hold full"
        );
    }

    #[test]
    fn by_the_engine_a_good_with_both_flags_decays_only_beside_a_perishable_only_good() {
        assert_eq!(stepped(ENGINE, &[(BOTH, 3)]), (vec![(BOTH, 4)], true));
        assert_eq!(
            stepped(ENGINE, &[(ROTTING, 3), (BOTH, 3)]),
            (vec![(ROTTING, 2), (BOTH, 3)], true),
            "grown and decayed"
        );
        assert_eq!(
            stepped(ENGINE, &[(BREEDING, 3), (BOTH, 3)]),
            (vec![(BREEDING, 4), (BOTH, 4)], true),
            "beside a tribbles good"
        );
        assert_eq!(
            stepped(ENGINE, &[(FOOD, 1), (BOTH, 3), (PLAIN, 1)]),
            (vec![(FOOD, 1), (BOTH, 4), (PLAIN, 1)], true),
            "beside goods that are not perishable"
        );
    }

    #[test]
    fn a_good_entered_at_none_is_not_held_and_neither_grows_nor_decays() {
        for source in [ENGINE, BIBLE] {
            let held = vec![(FOOD, 1), (BREEDING, 0), (ROTTING, 0), (BOTH, 0)];
            assert_eq!(stepped(source, &held), (held.clone(), false), "{source:?}");
        }
    }

    #[test]
    fn commodities_unflagged_junk_and_junk_with_no_record_never_change() {
        let unknown = Good::Junk(JunkId(999));
        for source in [ENGINE, BIBLE] {
            let held = vec![(FOOD, 2), (PLAIN, 3), (unknown, 1)];
            assert_eq!(stepped(source, &held), (held.clone(), false), "{source:?}");
            assert_eq!(stepped(source, &[]), (Vec::new(), false), "{source:?}");
        }
    }

    #[test]
    fn by_the_bible_tribbles_goods_grow_by_id_only_into_free_space() {
        assert_eq!(
            stepped(BIBLE, &[(BREEDING, 4), (BREEDING_TOO, 5)]),
            (vec![(BREEDING, 5), (BREEDING_TOO, 5)], true),
            "only the lower ID"
        );
        assert_eq!(
            stepped(BIBLE, &[(BREEDING, 4), (BREEDING_TOO, 4)]),
            (vec![(BREEDING, 5), (BREEDING_TOO, 5)], true),
            "room for both"
        );
        assert_eq!(
            stepped(BIBLE, &[(FOOD, 7), (BREEDING, 3)]),
            (vec![(FOOD, 7), (BREEDING, 3)], false),
            "the hold full"
        );
        assert_eq!(
            stepped_in(3, BIBLE, &[(FOOD, 4), (BREEDING, 1)]),
            (vec![(FOOD, 4), (BREEDING, 1)], false),
            "the hold over"
        );
    }

    #[test]
    fn by_the_bible_perishables_always_decay() {
        assert_eq!(
            stepped(BIBLE, &[(FOOD, 7), (ROTTING, 3)]),
            (vec![(FOOD, 7), (ROTTING, 2)], true),
            "the hold full"
        );
        assert_eq!(
            stepped(BIBLE, &[(ROTTING, 1)]),
            (Vec::new(), true),
            "a good decayed to none is gone"
        );
    }

    #[test]
    fn by_the_bible_a_good_with_both_flags_grows_then_decays() {
        assert_eq!(
            stepped(BIBLE, &[(BOTH, 3)]),
            (vec![(BOTH, 3)], false),
            "with room"
        );
        assert_eq!(
            stepped(BIBLE, &[(FOOD, 7), (BOTH, 3)]),
            (vec![(FOOD, 7), (BOTH, 2)], true),
            "the hold full"
        );
    }

    // A lost escort's share of the fleet's cargo.

    const LOST_FOOD: Good = Good::Commodity(0);
    const LOST_OPALS: Good = Good::Junk(JunkId(146));

    #[test]
    fn a_lost_ships_share_is_its_holds_over_the_fleets() {
        assert!((lost_share(30, 60, 30, false) - 1.0 / 3.0).abs() < 1e-12);
        assert!(
            (lost_share(30, 60, 30, true) - 0.25).abs() < 1e-12,
            "counted"
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_lost_ships_share_is_at_most_all_the_cargo() {
        assert_eq!(lost_share(100, 50, 0, false), 1.0);
        assert!(
            (lost_share(100, 50, 0, true) - 2.0 / 3.0).abs() < 1e-12,
            "counted"
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_lost_ship_with_no_holds_takes_nothing() {
        assert_eq!(lost_share(0, 60, 30, false), 0.0);
        assert_eq!(lost_share(0, 60, 30, true), 0.0);
        assert_eq!(lost_share(0, 0, 0, false), 0.0, "0/0");
        assert_eq!(lost_share(0, 0, 0, true), 0.0, "0/0, counted");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_lost_ships_holds_against_no_other_space_take_everything() {
        assert_eq!(lost_share(20, 0, 0, false), 1.0);
        assert_eq!(lost_share(20, 0, 0, true), 1.0, "counted");
    }

    #[test]
    fn a_lost_ships_negative_holds_count_as_their_size() {
        assert!((lost_share(-30, 60, 30, false) - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn destroying_a_share_takes_the_truncated_part_of_every_good() {
        let mut cargo = BTreeMap::from([(LOST_FOOD, 9), (LOST_OPALS, 3)]);
        destroy_share(&mut cargo, 0.5);
        assert_eq!(cargo, BTreeMap::from([(LOST_FOOD, 5), (LOST_OPALS, 2)]));
    }

    #[test]
    fn destroy_share_gives_the_tons_it_destroyed() {
        // Each commodity's d, zeros included, as `movw %dx, 0x7a(ship)`
        // writes them (@0xcf76); a `jünk` lost gives none.
        let mut cargo = BTreeMap::from([
            (Good::Commodity(0), 9),
            (Good::Commodity(4), 3),
            (Good::Junk(JunkId(128)), 8),
        ]);
        assert_eq!(destroy_share(&mut cargo, 0.5), [4, 0, 0, 0, 1, 0]);
        assert_eq!(
            cargo,
            BTreeMap::from([
                (Good::Commodity(0), 5),
                (Good::Commodity(4), 2),
                (Good::Junk(JunkId(128)), 4),
            ])
        );
        let mut cargo = BTreeMap::from([(Good::Commodity(5), 7)]);
        assert_eq!(destroy_share(&mut cargo, 1.0), [0, 0, 0, 0, 0, 7]);
        assert_eq!(destroy_share(&mut BTreeMap::new(), 1.0), [0; 6]);
    }

    #[test]
    fn destroying_all_the_share_empties_the_hold() {
        let mut cargo = BTreeMap::from([(LOST_FOOD, 9), (LOST_OPALS, 3)]);
        destroy_share(&mut cargo, 1.0);
        assert_eq!(cargo, BTreeMap::new());
    }

    #[test]
    fn destroying_no_share_changes_nothing() {
        let mut cargo = BTreeMap::from([(LOST_FOOD, 9), (LOST_OPALS, 3)]);
        destroy_share(&mut cargo, 0.0);
        assert_eq!(cargo, BTreeMap::from([(LOST_FOOD, 9), (LOST_OPALS, 3)]));
    }

    #[test]
    fn trimming_cuts_each_commodity_by_the_holds_over_the_total() {
        let mut cargo = BTreeMap::from([(LOST_FOOD, 5), (LOST_OPALS, 3)]);
        trim_commodities(&mut cargo, 40, 55);
        assert_eq!(cargo, BTreeMap::from([(LOST_FOOD, 3), (LOST_OPALS, 3)]));
    }

    #[test]
    fn trimming_to_holds_that_fit_the_total_changes_nothing() {
        let mut cargo = BTreeMap::from([(LOST_FOOD, 5), (LOST_OPALS, 3)]);
        trim_commodities(&mut cargo, 55, 55);
        assert_eq!(cargo, BTreeMap::from([(LOST_FOOD, 5), (LOST_OPALS, 3)]));
    }

    #[test]
    fn a_commodity_trimmed_to_none_is_gone() {
        let mut cargo = BTreeMap::from([(LOST_FOOD, 5), (LOST_OPALS, 3)]);
        trim_commodities(&mut cargo, 1, 55);
        assert_eq!(cargo, BTreeMap::from([(LOST_OPALS, 3)]));
    }
}
