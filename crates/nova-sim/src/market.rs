//! The commodity exchange: what a stellar trades, at what price, how much
//! cargo a ship carries, buying and selling, and the planetary events
//! (`öops`) that move prices.
//!
//! # Commodities and prices
//!
//! The six standard commodities are named by `STR#` 4000 and priced by
//! `STR#` 4004 ([`CommodityStrings`]); one whose base price is missing or
//! is not a whole number, or that has no name, is never traded. A
//! stellar with the trade-center flag trades commodity n when its `spöb`
//! flags give it a price level, in the nibble at bit 28 - 4n (food at 28
//! down to equipment at 8): 1 low, 2 medium, 4 high, 0 not traded. A
//! nibble with more than one bit set, which stock data never has, takes
//! the highest level it names. Low is [`LOW_PERCENT`] of the base price
//! and high [`HIGH_PERCENT`], truncated; medium is the base price. The
//! community's table of stock prices (Food 60/75/93, Medical 600/750/937,
//! and so on) matches these exactly.
//!
//! A stellar sells and buys each good at one price: profit comes from
//! carrying goods from where they are cheap to where they are dear.
//!
//! # Special goods
//!
//! A `jünk` is sold at its `SoldAt` stellars, at the low price of its
//! `BasePrice`, and bought at its `BoughtAt` stellars, at the high price
//! (the community's Opals: 960 and 1500 from 1200). It is traded only
//! through a trade center, so a listed stellar without one offers nothing.
//! A stellar in both lists, which stock data never has, trades it both
//! ways at its base price. `SellOn` gates selling it and `BuyOn` buying
//! it, through [`control_bits_allow`]. Its `Flags` make it multiply or
//! decay in the hold (below).
//!
//! # Tribbles and perishables
//!
//! In flight, a held `jünk` with the Tribbles flag ([`TRIBBLES`]) gains a
//! ton, and one with the Perishable flag ([`PERISHABLE`]) loses one, on
//! each due frame, as the [`Session`](crate::Session) ticks; a good that
//! decays to none is gone.
//! There is no random draw. A frame is due when the original's frame
//! counter is a multiple of [`JUNK_STEP_FRAMES`] ([`junk_step_due`]); the
//! counter counts up to [`LAST_FRAME`] and goes back to 0
//! ([`next_frame`]), so four due frames fall 250 frames apart and the
//! next 25 frames later: 5 steps every 1025 ticks, about 34 s. Each tick
//! in flight moves it on, a jump's too, but not a landed one; a take-off
//! sets it to [`AFTER_TAKE_OFF_FRAME`], so the first step comes on the
//! 15th tick of flight, and a destroyed player's ticks do not step. The
//! original's double time, which steps a due frame twice, is not
//! modelled.
//!
//! The conditions follow
//! [`RuleKey::JunkFlags`](crate::RuleKey::JunkFlags): by the engine, on
//! the free space measured once, so tribbles goods can overfill the hold,
//! perishable goods decay only while there is space, and a good with both
//! flags decays only beside a perishable-only good; by the Bible,
//! tribbles goods grow only into free space and perishable goods always
//! decay. The free space is the cargo space the ship and its outfits give
//! ([`cargo_capacity`]) less everything held; the original counts the
//! fleet's escort holds too, which waits for rdm
//! `phase/shop-and-trade-fidelity/phase-11-exchange-fleet-holds`.
//!
//! # Events
//!
//! Each day ([`step_day`]), every active event's days left go down by
//! one, and those that reach none end; then every `öops` that is not
//! active, has a stellar to start at, a `Freq` above 0, a `Duration`
//! above 0, a standard commodity and an `ActivateOn` that holds, starts
//! with `Duration` days left if a `Freq` % [`Chance`] fires. While an
//! event is active its `PriceDelta` is added to that commodity's price at
//! its stellar; several add up, and a price never goes below 0. The
//! exchange shows the names of the events active at its stellar.
//!
//! An event starts at its stellar: the `spöb` its `Stellar` names, from
//! 128 up, or, for a `Stellar` of -1 ("any stellar"), an even pick by
//! [`Chance`], once the `Freq` roll fires, among [`event_stellars`] by
//! ID: the stellars 128 to 2175 that are not uninhabited, whether or not
//! they trade. It stays there for its run, and is saved with it. A
//! `Stellar` of -2 (news only) or 0 to 127 is never rolled. With no
//! candidate a -1 event never starts either; here we leave the original,
//! whose draw never ends and hangs the game.
//!
//! # Cargo
//!
//! A ship carries `|Holds|` tons, plus each outfit's `ModVal` of
//! [`MORE_CARGO`] for each one carried. Buying one lot ([`Lot::One`]) is
//! a ton, and the most ([`Lot::Max`]) is as much as both the free space
//! and the cash allow; selling one is a ton, and the most everything
//! held. A trade that would move nothing is refused.

use std::collections::BTreeMap;

use crate::catalog::{
    CommodityStrings, DisasterId, DisasterRecord, JunkId, JunkRecord, PilotCatalog, StellarId,
};
use crate::chance::Chance;
use crate::fuel::OutfitMod;
use crate::landing::StellarFlags;
use crate::pilot::Pilot;
use crate::rulebook::RuleSource;

/// How many standard commodities there are.
pub const COMMODITIES: u8 = 6;

/// A low price, as a percentage of the base price.
pub const LOW_PERCENT: i64 = 80;

/// A high price, as a percentage of the base price.
pub const HIGH_PERCENT: i64 = 125;

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
    /// [`LOW_PERCENT`] of the base price.
    Low,
    /// The base price.
    Medium,
    /// [`HIGH_PERCENT`] of the base price.
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
    /// A ton.
    One,
    /// As much as possible.
    Max,
}

/// One trade the player asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    /// The good.
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
}

/// A standard commodity that can be traded: its name and base price.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commodity {
    /// Its name, from `STR#` 4000.
    pub name: String,
    /// Its base price, from `STR#` 4004.
    pub base_price: i64,
}

/// The price level that `spöb` `flags` set for `commodity`, or `None`
/// when the stellar does not trade it (or there is no such commodity).
#[must_use]
pub fn price_level(flags: u32, commodity: u8) -> Option<PriceLevel> {
    if commodity >= COMMODITIES {
        return None;
    }
    let nibble = (flags >> (28 - 4 * u32::from(commodity))) & 0xF;
    if nibble & 4 != 0 {
        Some(PriceLevel::High)
    } else if nibble & 2 != 0 {
        Some(PriceLevel::Medium)
    } else if nibble & 1 != 0 {
        Some(PriceLevel::Low)
    } else {
        None
    }
}

/// A good's price at `level`, from its `base` price.
#[must_use]
pub fn band_price(base: i64, level: PriceLevel) -> i64 {
    match level {
        PriceLevel::Low => base * LOW_PERCENT / 100,
        PriceLevel::Medium => base,
        PriceLevel::High => base * HIGH_PERCENT / 100,
    }
}

/// The cargo space, in tons, of a ship with `holds` and these `outfits`;
/// never below none. [`ShipStats`](crate::stats::ShipStats) gives a ship's.
#[must_use]
pub fn cargo_capacity(holds: i16, outfits: &[OutfitMod]) -> u32 {
    let pods: i64 = outfits
        .iter()
        .filter(|outfit| outfit.mod_type == MORE_CARGO)
        .map(|outfit| i64::from(outfit.mod_val) * i64::from(outfit.count))
        .sum();
    let tons = i64::from(holds).abs() + pods;
    u32::try_from(tons.max(0)).unwrap_or(u32::MAX)
}

/// Whether a control-bit test `expression` holds.
///
/// Always true for now: there are no control bits until rdm
/// `roadmap/missions-and-storylines` builds them, and this is the one
/// place every `öops` `ActivateOn`, `jünk` `BuyOn`/`SellOn`, and `oütf`
/// and `shïp` `Availability` is tested, so that roadmap replaces it here.
#[must_use]
pub fn control_bits_allow(_expression: &str) -> bool {
    true
}

/// The standard commodities that can be traded, by number, from
/// `strings`.
#[must_use]
pub fn commodities(strings: &CommodityStrings) -> Vec<(u8, Commodity)> {
    (0..COMMODITIES)
        .filter_map(|n| {
            let at = usize::from(n);
            let name = strings.names.get(at)?;
            let base_price = strings.base_prices.get(at)?.trim().parse::<i32>().ok()?;
            Some((
                n,
                Commodity {
                    name: name.clone(),
                    base_price: i64::from(base_price),
                },
            ))
        })
        .collect()
}

/// Everything the exchange trades and every event that can move its
/// prices, read once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Goods {
    commodities: Vec<(u8, Commodity)>,
    junk: Vec<JunkRecord>,
    disasters: Vec<DisasterRecord>,
    /// The stellars a `Stellar` -1 event may be placed at, by ascending ID.
    stellars: Vec<StellarId>,
}

impl Goods {
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
        }
    }

    /// These goods, with a `Stellar` -1 event placed among
    /// [`event_stellars`] of `stellars`, each `spöb` with its `Flags`.
    #[must_use]
    pub fn with_stellars(mut self, stellars: &[(StellarId, u32)]) -> Self {
        self.stellars = event_stellars(stellars);
        self
    }

    /// `good`'s name, as the exchange names it; none for a good that is
    /// not traded.
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
    /// Whether the stellar sells it: the player can buy it.
    pub sold_here: bool,
    /// Whether the stellar buys it: the player can sell it.
    pub bought_here: bool,
}

/// A stellar's exchange, as the player sees it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Market {
    /// The goods traded, standard commodities first by number, then
    /// `jünk` by ID.
    pub rows: Vec<MarketRow>,
    /// The names of the events active here, by `öops` ID.
    pub events: Vec<String>,
    /// The player's credits.
    pub cash: i64,
    /// The ship's cargo space, in tons.
    pub capacity: u32,
    /// The cargo space free, in tons.
    pub free: u32,
}

impl Market {
    /// `good`'s row, if it is traded here.
    #[must_use]
    pub fn row(&self, good: Good) -> Option<&MarketRow> {
        self.rows.iter().find(|row| row.good == good)
    }

    /// How many tons `order` would move, or why it moves none.
    pub fn tons(&self, order: Order) -> Result<u32, TradeRefusal> {
        let row = self.row(order.good).ok_or(TradeRefusal::NotTraded)?;
        match order.direction {
            Direction::Buy => {
                if !row.sold_here {
                    return Err(TradeRefusal::NotTraded);
                }
                if self.free == 0 {
                    return Err(TradeRefusal::NoSpace);
                }
                let wanted = match order.lot {
                    Lot::One => 1,
                    Lot::Max => self.free,
                };
                // A good priced at nothing is limited by space alone.
                let affordable = self
                    .cash
                    .max(0)
                    .checked_div(row.price)
                    .map_or(wanted, |tons| u32::try_from(tons).unwrap_or(u32::MAX));
                match wanted.min(affordable) {
                    0 => Err(TradeRefusal::CannotAfford),
                    tons => Ok(tons),
                }
            }
            Direction::Sell => {
                if !row.bought_here {
                    return Err(TradeRefusal::NotTraded);
                }
                match (row.held, order.lot) {
                    (0, _) => Err(TradeRefusal::NoneHeld),
                    (_, Lot::One) => Ok(1),
                    (held, Lot::Max) => Ok(held),
                }
            }
        }
    }

    /// Whether a ton of `good` can be traded `direction` now.
    #[must_use]
    pub fn allows(&self, good: Good, direction: Direction) -> bool {
        self.tons(Order {
            good,
            direction,
            lot: Lot::One,
        })
        .is_ok()
    }
}

/// The exchange of `stellar`, with these `flags`, for `pilot` with
/// `capacity` tons of cargo space; `None` without a trade center.
pub(crate) fn market(
    goods: &Goods,
    stellar: StellarId,
    flags: u32,
    pilot: &Pilot,
    capacity: u32,
) -> Option<Market> {
    if flags & StellarFlags::TRADE_CENTER == 0 {
        return None;
    }
    let here: Vec<&DisasterRecord> = pilot
        .events
        .iter()
        .filter_map(|(id, active)| {
            let event = goods.disasters.iter().find(|event| event.id == *id)?;
            (place(*active, event) == Some(stellar) && standard(event.commodity).is_some())
                .then_some(event)
        })
        .collect();
    let mut rows: Vec<MarketRow> = goods
        .commodities
        .iter()
        .filter_map(|(n, commodity)| {
            let level = price_level(flags, *n)?;
            let delta: i64 = here
                .iter()
                .filter(|event| standard(event.commodity) == Some(*n))
                .map(|event| i64::from(event.price_delta))
                .sum();
            let price = band_price(commodity.base_price, level) + delta;
            Some(listed(
                Good::Commodity(*n),
                &commodity.name,
                price,
                (true, true),
            ))
        })
        .collect();
    rows.extend(goods.junk.iter().filter_map(|junk| {
        let sold = junk.sold_at.contains(&stellar) && control_bits_allow(&junk.sell_on);
        let bought = junk.bought_at.contains(&stellar) && control_bits_allow(&junk.buy_on);
        let level = match (sold, bought) {
            (true, true) => PriceLevel::Medium,
            (true, false) => PriceLevel::Low,
            (false, true) => PriceLevel::High,
            (false, false) => return None,
        };
        let price = band_price(i64::from(junk.base_price), level);
        Some(listed(
            Good::Junk(junk.id),
            &junk.name,
            price,
            (sold, bought),
        ))
    }));
    rows.sort_by_key(|row| row.good);
    for row in &mut rows {
        row.held = pilot.held(row.good);
    }
    let held = pilot
        .cargo
        .values()
        .fold(0_u32, |sum, &tons| sum.saturating_add(tons));
    Some(Market {
        rows,
        events: here.iter().map(|event| event.name.clone()).collect(),
        cash: pilot.cash,
        capacity,
        free: capacity.saturating_sub(held),
    })
}

/// `commodity` as a standard commodity's number, if it is one.
fn standard(commodity: i16) -> Option<u8> {
    u8::try_from(commodity).ok().filter(|&n| n < COMMODITIES)
}

/// A row for `good`, priced at `price` (never below none), traded these
/// ways (sold here, bought here), with none held.
fn listed(good: Good, name: &str, price: i64, (sold_here, bought_here): (bool, bool)) -> MarketRow {
    MarketRow {
        good,
        name: name.to_owned(),
        price: price.max(0),
        held: 0,
        sold_here,
        bought_here,
    }
}

/// Moves `tons` of `order`'s good at `price` a ton between the stellar
/// and `pilot`, paying or being paid.
pub(crate) fn settle(pilot: &mut Pilot, order: Order, tons: u32, price: i64) {
    let total = i64::from(tons).saturating_mul(price);
    let held = pilot.held(order.good);
    let held = match order.direction {
        Direction::Buy => {
            pilot.cash = pilot.cash.saturating_sub(total);
            held.saturating_add(tons)
        }
        Direction::Sell => {
            pilot.cash = pilot.cash.saturating_add(total);
            held.saturating_sub(tons)
        }
    };
    if held == 0 {
        pilot.cargo.remove(&order.good);
    } else {
        pilot.cargo.insert(order.good, held);
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
pub(crate) struct ActiveEvent {
    /// The days it has left.
    pub(crate) days: u16,
    /// The stellar it is at; `None` for its record's own `Stellar`, which
    /// only a save from before version 13 holds.
    pub(crate) stellar: Option<StellarId>,
}

/// The stellar `active`, an event of `event`'s, is at: the one it
/// stored, or else its record's.
fn place(active: ActiveEvent, event: &DisasterRecord) -> Option<StellarId> {
    active.stellar.or_else(|| fixed_stellar(event.stellar))
}

/// One day's events: the active ones age, and each that can start is
/// rolled once on `chance`.
pub(crate) fn step_day(
    goods: &Goods,
    events: &mut BTreeMap<DisasterId, ActiveEvent>,
    chance: &mut (impl Chance + ?Sized),
) {
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
            && event.freq > 0
            && event.duration > 0
            && standard(event.commodity).is_some()
            && control_bits_allow(&event.activate_on);
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

/// One tribbles and perishables step on `cargo`, in a hold of `capacity`
/// tons, read as `source` says
/// ([`RuleKey::JunkFlags`](crate::RuleKey::JunkFlags)); whether it
/// changed anything. Only held `jünk` whose record has a flag changes,
/// a ton at a time, and a good decayed to none is gone.
///
/// The free space is the capacity less everything held, never below
/// none, measured once.
///
/// - By the engine (`_ResetPlayerPrecalcedValues` @0xc52e-0xc55b,
///   `_HandlePlayer` @0x70827-0x7093b), while that space is above none,
///   every tribbles good held gains a ton, so with several of them the
///   hold can go over; then, when a good that is perishable and not
///   tribbles is held, every perishable good held loses a ton, on the
///   same space, measured before the growth. So a good with both flags
///   only grows unless a perishable-only good is aboard too, and then it
///   grows and decays. With no tribbles good aboard, the original's
///   perishable test reads a value nothing set for it, which is undefined;
///   here it reads the free space measured the same way.
/// - By the Bible, which says only that tribbles goods multiply and
///   perishable goods "gradually decay away", the tribbles goods grow by
///   ascending ID, each taking a ton of the free space, until none is
///   left, so the hold never goes over; then every perishable good loses
///   a ton, whatever the space.
pub(crate) fn step_junk(
    goods: &Goods,
    cargo: &mut BTreeMap<Good, u32>,
    capacity: u32,
    source: RuleSource,
) -> bool {
    let flags = |good: Good| match good {
        Good::Junk(id) => goods
            .junk
            .iter()
            .find(|junk| junk.id == id)
            .map_or(0, |junk| junk.flags),
        Good::Commodity(_) => 0,
    };
    let held: Vec<(Good, u16)> = cargo
        .iter()
        .filter(|&(_, &tons)| tons > 0)
        .map(|(&good, _)| (good, flags(good)))
        .collect();
    let with = |flag: u16| -> Vec<Good> {
        held.iter()
            .filter(|(_, flags)| flags & flag != 0)
            .map(|&(good, _)| good)
            .collect()
    };
    let total = cargo
        .values()
        .fold(0_u32, |sum, &tons| sum.saturating_add(tons));
    let free = capacity.saturating_sub(total);
    let (grow, decay) = match source {
        RuleSource::Engine => {
            let rotting = held
                .iter()
                .any(|(_, flags)| flags & PERISHABLE != 0 && flags & TRIBBLES == 0);
            let room = free > 0;
            let grow = if room { with(TRIBBLES) } else { Vec::new() };
            let decay = if room && rotting {
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
    use crate::testkit::{Scripted, catalog};

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
    fn a_nibble_with_several_bits_takes_its_highest_level_and_8_alone_none() {
        use PriceLevel::{High, Medium};
        assert_eq!(price_level(0x7 << 28, 0), Some(High));
        assert_eq!(price_level(0x3 << 28, 0), Some(Medium));
        assert_eq!(price_level(0x5 << 28, 0), Some(High));
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
    fn low_is_80_percent_and_high_125_percent_truncated() {
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
                [Low, Medium, High].map(|level| band_price(base, level)),
                prices,
                "{base}"
            );
        }
        assert_eq!((LOW_PERCENT, HIGH_PERCENT), (80, 125));
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

    // Control bits.

    #[test]
    fn every_control_bit_expression_holds_until_there_are_control_bits() {
        for expression in ["", "b43", "!b80", "b1 & (b2 | !b3)"] {
            assert!(control_bits_allow(expression), "{expression}");
        }
    }

    // Commodities.

    fn strings(names: &[&str], prices: &[&str]) -> CommodityStrings {
        CommodityStrings {
            names: names.iter().map(|&s| s.to_owned()).collect(),
            base_prices: prices.iter().map(|&s| s.to_owned()).collect(),
        }
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
    fn a_commodity_without_a_whole_price_or_a_name_is_not_traded() {
        let found = commodities(&strings(
            &["Food", "Industrial", "Medical", "Luxury", "Metal"],
            &["75", "lots", "", "-40", "200", "550", "99"],
        ));
        assert_eq!(
            found,
            [
                (0, commodity("Food", 75)),
                (3, commodity("Luxury", -40)),
                (4, commodity("Metal", 200)),
            ]
        );
        assert_eq!(commodities(&CommodityStrings::default()), []);
        let padded = commodities(&strings(&["Food"], &[" 75 "]));
        assert_eq!(padded, [(0, commodity("Food", 75))], "spaces are trimmed");
    }

    // The exchange.

    const EARTH: StellarId = StellarId(140);
    const MARS: StellarId = StellarId(141);

    /// A `jünk` 0 that no stellar lists, at no price.
    fn unlisted() -> JunkRecord {
        JunkRecord {
            id: JunkId(0),
            name: String::new(),
            base_price: 0,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: String::new(),
            sell_on: String::new(),
            flags: 0,
        }
    }

    /// Opals, sold at Earth and bought at Mars; water, sold at Earth.
    fn junk() -> Vec<JunkRecord> {
        vec![
            JunkRecord {
                id: JunkId(146),
                name: "Opals".to_owned(),
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
                activate_on: String::new(),
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

    fn pilot(cash: i64) -> Pilot {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        pilot.cash = cash;
        pilot
    }

    fn row(good: Good, name: &str, price: i64) -> MarketRow {
        MarketRow {
            good,
            name: name.to_owned(),
            price,
            held: 0,
            sold_here: true,
            bought_here: true,
        }
    }

    const TRADE: u32 = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER;

    #[test]
    fn without_a_trade_center_there_is_no_exchange() {
        let flags = PORT_KANE & !StellarFlags::TRADE_CENTER;
        assert_eq!(market(&goods(), EARTH, flags, &pilot(0), 10), None);
        assert_eq!(market(&goods(), MARS, 0, &pilot(0), 10), None);
    }

    #[test]
    fn the_exchange_lists_the_commodities_traded_at_their_levels() {
        let flags = PORT_KANE;
        let found = market(&goods(), StellarId(137), flags, &pilot(500), 10).expect("trades");
        assert_eq!(
            found,
            Market {
                rows: vec![
                    row(Good::Commodity(0), "Food", 93),
                    row(Good::Commodity(1), "Industrial", 350),
                    row(Good::Commodity(2), "Medical Supplies", 937),
                    row(Good::Commodity(3), "Luxury Goods", 900),
                    row(Good::Commodity(4), "Metal", 200),
                    row(Good::Commodity(5), "Equipment", 440),
                ],
                events: Vec::new(),
                cash: 500,
                capacity: 10,
                free: 10,
            }
        );
        let some = TRADE | (1 << 24) | (4 << 12);
        let found = market(&goods(), StellarId(137), some, &pilot(0), 0).expect("trades");
        assert_eq!(
            found.rows,
            [
                row(Good::Commodity(1), "Industrial", 280),
                row(Good::Commodity(4), "Metal", 250),
            ]
        );
        let none = market(&goods(), StellarId(137), TRADE, &pilot(0), 0).expect("trades");
        assert_eq!(none.rows, [], "a trade center that trades nothing");
    }

    #[test]
    fn junk_is_sold_low_where_it_is_sold_and_bought_high_where_it_is_bought() {
        let at_earth = market(&goods(), EARTH, TRADE, &pilot(0), 0).expect("trades");
        assert_eq!(
            at_earth.rows,
            [
                MarketRow {
                    bought_here: false,
                    ..row(Good::Junk(JunkId(134)), "Water", 240)
                },
                MarketRow {
                    bought_here: false,
                    ..row(Good::Junk(JunkId(146)), "Opals", 960)
                },
            ],
            "by ID"
        );
        let at_mars = market(&goods(), MARS, TRADE, &pilot(0), 0).expect("trades");
        assert_eq!(
            at_mars.rows,
            [MarketRow {
                sold_here: false,
                ..row(Good::Junk(JunkId(146)), "Opals", 1500)
            }]
        );
        let elsewhere = market(&goods(), StellarId(150), TRADE, &pilot(0), 0).expect("trades");
        assert_eq!(elsewhere.rows, []);
    }

    #[test]
    fn junk_listed_both_ways_trades_at_its_base_price() {
        let both = JunkRecord {
            id: JunkId(200),
            name: "Both".to_owned(),
            base_price: 400,
            sold_at: vec![EARTH],
            bought_at: vec![EARTH],
            ..unlisted()
        };
        let goods = Goods::new(&CommodityStrings::default(), vec![both], Vec::new());
        let found = market(&goods, EARTH, TRADE, &pilot(0), 0).expect("trades");
        assert_eq!(found.rows, [row(Good::Junk(JunkId(200)), "Both", 400)]);
    }

    #[test]
    fn junk_follows_the_commodities() {
        let flags = TRADE | (2 << 28);
        let found = market(&goods(), MARS, flags, &pilot(0), 0).expect("trades");
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
        let found = market(&goods(), EARTH, TRADE | (2 << 28), &pilot, 12).expect("trades");
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
        let over = market(&goods(), EARTH, TRADE, &pilot, 5).expect("trades");
        assert_eq!(over.free, 0, "more held than there is space");
    }

    /// An event of `record`'s, which is at `stellar`, with `days` left.
    fn at(record: &DisasterRecord, days: u16) -> (DisasterId, ActiveEvent) {
        let stellar = Some(StellarId(record.stellar));
        (record.id, ActiveEvent { days, stellar })
    }

    /// The exchange at `stellar` with these events active, each at its
    /// record's stellar.
    fn with_events(stellar: StellarId, flags: u32, active: &[(i16, u16)]) -> Market {
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
        market(&goods(), stellar, flags, &pilot, 0).expect("trades")
    }

    fn price(market: &Market, good: Good) -> i64 {
        market.row(good).expect("listed").price
    }

    #[test]
    fn an_active_event_moves_its_commoditys_price_at_its_stellar_and_names_itself() {
        let flags = PORT_KANE;
        let food = Good::Commodity(0);
        let quiet = with_events(EARTH, flags, &[]);
        assert_eq!(price(&quiet, food), 93);
        assert_eq!(quiet.events, Vec::<String>::new());
        let surplus = with_events(EARTH, flags, &[(128, 30)]);
        assert_eq!(price(&surplus, food), 78);
        assert_eq!(surplus.events, ["An enormous food surplus"]);
        assert_eq!(
            price(&surplus, Good::Commodity(1)),
            350,
            "other goods keep theirs"
        );
        let both = with_events(EARTH, flags, &[(128, 1), (130, 7)]);
        assert_eq!(price(&both, food), 93, "-15 and +15");
        assert_eq!(both.events, ["An enormous food surplus", "A minor drought"]);
        let elsewhere = with_events(MARS, flags, &[(128, 30)]);
        assert_eq!(price(&elsewhere, food), 93);
        assert_eq!(elsewhere.events, Vec::<String>::new());
        let glut = with_events(MARS, flags, &[(129, 3), (999, 3)]);
        assert_eq!(price(&glut, Good::Commodity(1)), 280);
        assert_eq!(
            glut.events,
            ["A glut on the market"],
            "a gone öops is not shown"
        );
    }

    #[test]
    fn a_price_never_goes_below_none() {
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(140),
                name: "Crash".to_owned(),
                stellar: 140,
                commodity: 0,
                price_delta: -100,
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
        let found = market(&goods, EARTH, PORT_KANE, &pilot, 0).expect("trades");
        assert_eq!(price(&found, Good::Commodity(0)), 0);
        let negative = Goods::new(&strings(&["Food"], &["-40"]), Vec::new(), Vec::new());
        let found = market(&negative, EARTH, PORT_KANE, &pilot, 0).expect("trades");
        assert_eq!(price(&found, Good::Commodity(0)), 0);
    }

    #[test]
    fn an_event_on_a_commodity_that_is_not_standard_moves_nothing_and_is_not_shown() {
        let goods = Goods::new(
            &stock(),
            Vec::new(),
            vec![DisasterRecord {
                id: DisasterId(140),
                name: "Odd".to_owned(),
                stellar: 140,
                commodity: 6,
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
        let found = market(&goods, EARTH, PORT_KANE, &pilot, 0).expect("trades");
        assert_eq!(found.events, Vec::<String>::new());
        assert_eq!(price(&found, Good::Commodity(5)), 440);
    }

    /// The exchange at `here` (Port Kane's levels) with one food event
    /// (-15) active, its record's `Stellar` `record`, stored at `stored`.
    fn one_event(record: i16, stored: Option<StellarId>, here: StellarId) -> Market {
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
        market(&goods, here, PORT_KANE, &pilot, 0).expect("trades")
    }

    /// Food's price and the events shown at `market`.
    fn food_and_events(market: &Market) -> (i64, Vec<String>) {
        (price(market, Good::Commodity(0)), market.events.clone())
    }

    #[test]
    fn an_event_moves_the_price_and_shows_at_its_stored_stellar_only() {
        let surplus = (78, vec!["Surplus".to_owned()]);
        let quiet = (93, Vec::new());
        for record in [-1, 140] {
            let mars = one_event(record, Some(MARS), MARS);
            assert_eq!(food_and_events(&mars), surplus, "{record}");
            let earth = one_event(record, Some(MARS), EARTH);
            assert_eq!(food_and_events(&earth), quiet, "{record}");
        }
    }

    #[test]
    fn an_event_from_an_older_save_is_at_its_records_stellar() {
        let surplus = (78, vec!["Surplus".to_owned()]);
        let quiet = (93, Vec::new());
        assert_eq!(food_and_events(&one_event(140, None, EARTH)), surplus);
        assert_eq!(food_and_events(&one_event(140, None, MARS)), quiet);
        for here in [EARTH, MARS, StellarId(-1)] {
            assert_eq!(food_and_events(&one_event(-1, None, here)), quiet);
        }
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
        }
    }

    fn order(good: Good, direction: Direction, lot: Lot) -> Order {
        Order {
            good,
            direction,
            lot,
        }
    }

    const FOOD: Good = Good::Commodity(0);
    const OPALS: Good = Good::Junk(JunkId(146));
    const WATER: Good = Good::Junk(JunkId(134));

    #[test]
    fn buying_one_is_a_ton_and_the_most_is_what_space_and_cash_allow() {
        let buy = |market: &Market, lot| market.tons(order(FOOD, Direction::Buy, lot));
        assert_eq!(buy(&stall(1000, 0, 50), Lot::One), Ok(1));
        assert_eq!(buy(&stall(1000, 0, 50), Lot::Max), Ok(10), "cash");
        assert_eq!(buy(&stall(1099, 0, 50), Lot::Max), Ok(10));
        assert_eq!(buy(&stall(100_000, 0, 7), Lot::Max), Ok(7), "space");
        assert_eq!(buy(&stall(700, 0, 7), Lot::Max), Ok(7), "both");
        assert_eq!(buy(&stall(100, 0, 1), Lot::One), Ok(1));
    }

    #[test]
    fn a_buy_with_no_space_or_not_enough_cash_for_a_ton_is_refused() {
        let buy = |market: &Market, lot| market.tons(order(FOOD, Direction::Buy, lot));
        for lot in [Lot::One, Lot::Max] {
            assert_eq!(buy(&stall(1000, 0, 0), lot), Err(TradeRefusal::NoSpace));
            assert_eq!(buy(&stall(99, 0, 5), lot), Err(TradeRefusal::CannotAfford));
            assert_eq!(
                buy(&stall(-500, 0, 5), lot),
                Err(TradeRefusal::CannotAfford)
            );
            assert_eq!(buy(&stall(0, 0, 0), lot), Err(TradeRefusal::NoSpace));
        }
    }

    #[test]
    fn a_good_priced_at_nothing_is_limited_only_by_space() {
        let buy = |market: &Market, lot| market.tons(order(WATER, Direction::Buy, lot));
        assert_eq!(buy(&stall(0, 0, 9), Lot::Max), Ok(9));
        assert_eq!(buy(&stall(-5, 0, 9), Lot::One), Ok(1));
        assert_eq!(buy(&stall(0, 0, 0), Lot::Max), Err(TradeRefusal::NoSpace));
    }

    #[test]
    fn selling_one_is_a_ton_and_the_most_everything_held() {
        let sell = |market: &Market, lot| market.tons(order(FOOD, Direction::Sell, lot));
        assert_eq!(sell(&stall(0, 6, 0), Lot::One), Ok(1));
        assert_eq!(sell(&stall(0, 6, 0), Lot::Max), Ok(6));
        assert_eq!(sell(&stall(0, 1, 0), Lot::Max), Ok(1));
        for lot in [Lot::One, Lot::Max] {
            assert_eq!(sell(&stall(0, 0, 9), lot), Err(TradeRefusal::NoneHeld));
        }
    }

    #[test]
    fn a_good_not_traded_that_way_or_at_all_is_refused() {
        let market = stall(10_000, 5, 5);
        for lot in [Lot::One, Lot::Max] {
            let refused = Err(TradeRefusal::NotTraded);
            assert_eq!(market.tons(order(OPALS, Direction::Buy, lot)), refused);
            assert_eq!(market.tons(order(WATER, Direction::Sell, lot)), refused);
            for direction in [Direction::Buy, Direction::Sell] {
                let metal = order(Good::Commodity(4), direction, lot);
                assert_eq!(market.tons(metal), refused);
            }
        }
        assert_eq!(market.tons(order(OPALS, Direction::Sell, Lot::Max)), Ok(5));
        assert_eq!(market.tons(order(WATER, Direction::Buy, Lot::Max)), Ok(5));
    }

    #[test]
    fn allows_is_whether_a_ton_would_go_through() {
        let market = stall(150, 1, 3);
        assert!(market.allows(FOOD, Direction::Buy));
        assert!(market.allows(FOOD, Direction::Sell));
        assert!(!market.allows(OPALS, Direction::Buy));
        assert!(market.allows(OPALS, Direction::Sell));
        assert!(!stall(99, 0, 3).allows(FOOD, Direction::Buy));
        assert!(!stall(99, 0, 3).allows(FOOD, Direction::Sell));
        assert_eq!(market.row(OPALS).map(|row| row.price), Some(500));
        assert_eq!(market.row(Good::Commodity(3)), None);
    }

    #[test]
    fn settling_a_buy_pays_and_loads_and_a_sale_unloads_and_is_paid() {
        let mut pilot = pilot(1000);
        settle(&mut pilot, order(FOOD, Direction::Buy, Lot::Max), 4, 93);
        assert_eq!(pilot.cash, 1000 - 372);
        assert_eq!(pilot.held(FOOD), 4);
        settle(&mut pilot, order(OPALS, Direction::Buy, Lot::One), 1, 0);
        assert_eq!(pilot.held(OPALS), 1);
        settle(&mut pilot, order(FOOD, Direction::Sell, Lot::One), 1, 150);
        assert_eq!(pilot.cash, 1000 - 372 + 150);
        assert_eq!(pilot.held(FOOD), 3);
        settle(&mut pilot, order(FOOD, Direction::Sell, Lot::Max), 3, 100);
        assert_eq!(pilot.cash, 1000 - 372 + 150 + 300);
        assert_eq!(pilot.held(FOOD), 0);
        assert_eq!(
            pilot.cargo().collect::<Vec<_>>(),
            [(OPALS, 1)],
            "a good sold out is gone"
        );
    }

    #[test]
    fn settling_never_overflows() {
        let mut pilot = pilot(i64::MAX - 5);
        settle(&mut pilot, order(FOOD, Direction::Sell, Lot::Max), 0, 10);
        assert_eq!(pilot.cash, i64::MAX - 5);
        pilot.cargo.insert(FOOD, u32::MAX);
        settle(
            &mut pilot,
            order(FOOD, Direction::Sell, Lot::Max),
            u32::MAX,
            i64::MAX,
        );
        assert_eq!(pilot.cash, i64::MAX);
        pilot.cargo.insert(OPALS, u32::MAX);
        settle(&mut pilot, order(OPALS, Direction::Buy, Lot::One), 5, 0);
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
        step_day(&goods(), &mut events, &mut chance);
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
            step_day(&goods(), &mut events, &mut chance);
        }
        assert_eq!(days(&events), []);
        assert_eq!(chance.asked.len(), 30);
    }

    #[test]
    fn an_active_event_ages_a_day_at_a_time_and_is_not_rolled_again() {
        let mut events = BTreeMap::from([on_earth(128, 3)]);
        let mut chance = Scripted::default();
        step_day(&goods(), &mut events, &mut chance);
        assert_eq!(days(&events), [(128, 2)]);
        assert_eq!(chance.asked, [40, 50], "not the active one");
        step_day(&goods(), &mut events, &mut chance);
        assert_eq!(days(&events), [(128, 1)]);
        step_day(&goods(), &mut events, &mut chance);
        assert_eq!(days(&events), [], "over");
        assert_eq!(
            chance.asked.len(),
            2 + 2 + 3,
            "rolled again the day it ends"
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
            step_day(&goods, &mut events, &mut once);
            active.push(events.contains_key(&DisasterId(128)));
        }
        assert_eq!(active, [true, true, false, false]);
    }

    #[test]
    fn an_event_whose_öops_is_gone_ends() {
        let mut events = BTreeMap::from([on_earth(999, 20), on_earth(128, 20)]);
        step_day(&goods(), &mut events, &mut Scripted::default());
        assert_eq!(days(&events), [(128, 19)]);
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
                event(134, 140, -1, 10, 50),
                event(135, 140, 6, 10, 50),
                event(136, 140, 5, 10, 250),
                event(137, 0, 0, 10, 50),
            ],
        )
        .with_stellars(&[(StellarId(140), TRADE)]);
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]);
        step_day(&goods, &mut events, &mut chance);
        assert_eq!(chance.asked, [100], "only 136, its Freq clamped");
        assert_eq!(days(&events), [(136, 10)]);
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
        step_day(&anywhere(), &mut events, &mut chance);
        assert_eq!(chance.asked, [50]);
        assert_eq!(chance.sides_asked, [2], "140 and 150, not the uninhabited");
        assert_eq!(events, placed(150, 10));
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]).and_rolling(&[0]);
        step_day(&anywhere(), &mut events, &mut chance);
        assert_eq!(events, placed(140, 10));
    }

    #[test]
    fn an_any_stellar_event_whose_roll_is_out_of_range_does_not_start() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]).and_rolling(&[2]);
        step_day(&anywhere(), &mut events, &mut chance);
        assert_eq!(chance.sides_asked, [2]);
        assert_eq!(events, BTreeMap::new());
    }

    #[test]
    fn an_any_stellar_event_whose_chance_does_not_fire_draws_no_stellar() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::default();
        step_day(&anywhere(), &mut events, &mut chance);
        assert_eq!(chance.asked, [50]);
        assert_eq!(chance.sides_asked, Vec::<u16>::new());
        assert_eq!(events, BTreeMap::new());
    }

    #[test]
    fn an_any_stellar_event_with_no_candidate_is_never_rolled() {
        let goods = Goods::new(&stock(), Vec::new(), anywhere().disasters);
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]);
        step_day(&goods, &mut events, &mut chance);
        assert_eq!(chance.asked, Vec::<u8>::new());
        assert_eq!(chance.sides_asked, Vec::<u16>::new());
        assert_eq!(events, BTreeMap::new());
    }

    #[test]
    fn an_any_stellar_event_keeps_its_stellar_as_it_ages() {
        let mut events = placed(150, 3);
        let mut chance = Scripted::answering(&[true, true]).and_rolling(&[0, 0]);
        step_day(&anywhere(), &mut events, &mut chance);
        assert_eq!(events, placed(150, 2));
        step_day(&anywhere(), &mut events, &mut chance);
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
        step_day(&goods, &mut events, &mut Scripted::answering(&[true]));
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
        let mut cargo: BTreeMap<Good, u32> = held.iter().copied().collect();
        let changed = step_junk(&flagged(), &mut cargo, capacity, source);
        (cargo.into_iter().collect(), changed)
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
}
