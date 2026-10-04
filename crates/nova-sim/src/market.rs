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
//! it, through [`control_bits_allow`]. Its tribbles and perishable flags
//! are not modelled.
//!
//! # Events
//!
//! Each day ([`step_day`]), every active event's days left go down by
//! one, and those that reach none end; then every `öops` that is not
//! active, names a stellar, has a `Freq` above 0, a `Duration` above 0, a
//! standard commodity and an `ActivateOn` that holds, starts with
//! `Duration` days left if a `Freq` % [`Chance`] fires. While an event is
//! active its `PriceDelta` is added to that commodity's price at its
//! stellar; several add up, and a price never goes below 0. The exchange
//! shows the names of the events active at its stellar.
//!
//! An `öops` whose `Stellar` is -2 (news only) never moves a price, and
//! -1 ("any stellar") is not modelled: no stock record uses it, and it
//! would need a second draw to pick the stellar. Neither is ever rolled.
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
}

impl Goods {
    /// The goods and events `catalog` holds.
    pub fn read(catalog: &impl PilotCatalog) -> Self {
        Self::new(
            &catalog.commodity_strings(),
            catalog.junk(),
            catalog.disasters(),
        )
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
        .keys()
        .filter_map(|id| goods.disasters.iter().find(|event| event.id == *id))
        .filter(|event| event.stellar == stellar.0 && standard(event.commodity).is_some())
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

/// One day's events: the active ones age, and each that can start is
/// rolled once on `chance`.
pub(crate) fn step_day(
    goods: &Goods,
    events: &mut BTreeMap<DisasterId, u16>,
    chance: &mut (impl Chance + ?Sized),
) {
    events.retain(|id, days| {
        *days = days.saturating_sub(1);
        *days > 0 && goods.disasters.iter().any(|event| event.id == *id)
    });
    for event in &goods.disasters {
        let can_start = !events.contains_key(&event.id)
            && event.stellar >= 0
            && event.freq > 0
            && event.duration > 0
            && standard(event.commodity).is_some()
            && control_bits_allow(&event.activate_on);
        if !can_start {
            continue;
        }
        let percent = u8::try_from(event.freq.min(100)).unwrap_or(100);
        if chance.fires(percent) {
            let days = u16::try_from(event.duration).unwrap_or(u16::MAX);
            events.insert(event.id, days);
        }
    }
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

    /// The exchange at `stellar` with these events active.
    fn with_events(stellar: StellarId, flags: u32, active: &[(i16, u16)]) -> Market {
        let mut pilot = pilot(0);
        pilot.events = active
            .iter()
            .map(|&(id, days)| (DisasterId(id), days))
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
        pilot.events = BTreeMap::from([(DisasterId(140), 3)]);
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
        pilot.events = BTreeMap::from([(DisasterId(140), 3)]);
        let found = market(&goods, EARTH, PORT_KANE, &pilot, 0).expect("trades");
        assert_eq!(found.events, Vec::<String>::new());
        assert_eq!(price(&found, Good::Commodity(5)), 440);
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

    fn days(events: &BTreeMap<DisasterId, u16>) -> Vec<(i16, u16)> {
        events.iter().map(|(id, &days)| (id.0, days)).collect()
    }

    #[test]
    fn an_event_starts_when_its_chance_fires_for_its_duration() {
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true, false, true]);
        step_day(&goods(), &mut events, &mut chance);
        assert_eq!(chance.asked, [35, 40, 50], "each öops by ID, at its Freq");
        assert_eq!(days(&events), [(128, 30), (130, 25)]);
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
        let mut events = BTreeMap::from([(DisasterId(128), 3)]);
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
        let mut events = BTreeMap::from([(DisasterId(999), 20), (DisasterId(128), 20)]);
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
                event(128, -1, 0, 10, 50),
                event(129, -2, 0, 10, 50),
                event(130, 140, 0, 10, 0),
                event(131, 140, 0, 10, -5),
                event(132, 140, 0, 0, 50),
                event(133, 140, 0, -3, 50),
                event(134, 140, -1, 10, 50),
                event(135, 140, 6, 10, 50),
                event(136, 140, 5, 10, 250),
            ],
        );
        let mut events = BTreeMap::new();
        let mut chance = Scripted::answering(&[true]);
        step_day(&goods, &mut events, &mut chance);
        assert_eq!(chance.asked, [100], "only the last, its Freq clamped");
        assert_eq!(days(&events), [(136, 10)]);
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
}
