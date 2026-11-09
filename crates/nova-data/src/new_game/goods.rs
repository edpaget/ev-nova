//! Goods (`goods/`) and a stellar's `prices`, mapped onto the stock
//! exchange: the six standard commodities and special goods (`jünk`).
//!
//! # Which goods are standard
//!
//! The ID table decides ([`super::ids`]): the goods fill the six standard
//! slots ([`COMMODITY_SLOTS`]) in id order, each good pinned in
//! `commodities` keeping its slot, and the goods left over take `jünk`
//! IDs ([`JUNK_IDS`]). The format says nothing of slots, so which goods
//! are standard is the mapping's choice, and the table keeps it.
//!
//! # Standard goods
//!
//! When any good takes a slot, the commodity lists are written, six
//! entries each in slot order, empty for a slot no good takes
//! ([`standard_strings`]):
//!
//! - `STR#` 4000 ([`NAMES`]): the names;
//! - `STR#` 4001 ([`LC_NAMES`]): each name lower-cased (a placeholder);
//! - `STR#` 4003 ([`LABELS`]): each name with a `:`, the status bar's
//!   label (a placeholder);
//! - `STR#` 4004 ([`PRICES`]): the base price, as decimal text.
//!
//! `STR#` 4002 holds mission cargo, not commodities, and isn't written.
//! A stellar sets the level bit of each standard good it prices in its
//! slot's nibble of `spöb` `Flags` ([`price_flags`],
//! [`flags::price_bits`]).
//!
//! # Special goods
//!
//! Each becomes a `jünk` named for the good ([`junk`]): `SoldAt` the
//! trade stellars that price it `low` and `BoughtAt` those that price it
//! `high`, each in `spöb` ID order and at most eight; `BasePrice` its
//! price; `LCName` its name lower-cased and `Abbrev` its name; blank
//! `BuyOn` and `SellOn`, so both always hold; no flags and no `ScanMask`.
//! A `jünk` has no medium price.
//!
//! # Problems
//!
//! Each is reported against its file, and the rest still maps:
//!
//! - A stellar's price for a good that isn't in `goods/`: that price is
//!   left out. A good whose file failed to load isn't reported again.
//! - A special good priced `medium`: that price is left out.
//! - A special good priced at one level at more than eight stellars: the
//!   ninth on, by `spöb` ID, get no price, each reported.
//! - A good priced above [`MAX_PRICE`]: it is priced at [`MAX_PRICE`], as
//!   the `jünk`'s `BasePrice` and the engine's read of `STR#` 4004 hold no
//!   more.
//! - A stellar with `prices` but no `trade` service: nothing is traded
//!   there.

use std::collections::BTreeMap;

use nova_galaxy_format::goods::{Good, PriceLevel};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::{Service, Stellar as BibleStellar};

use super::ids::{COMMODITY_SLOTS, IdTable, JUNK_IDS, after_pins, allocate_pinned};
use super::map::{absent, number};
use crate::decode::Entry;
use crate::records::junk::Junk;
use crate::records::stellar::flags;
use crate::records::string_list::StrList;
use crate::registry::AnyRecord;
use crate::wire::flags::Flags16;
use crate::wire::id::StellarId;
use crate::wire::string::MacString;

/// The `STR#` of the standard goods' names.
pub const NAMES: i16 = 4000;
/// The `STR#` of their lower-case names.
pub const LC_NAMES: i16 = 4001;
/// The `STR#` of their status bar labels.
pub const LABELS: i16 = 4003;
/// The `STR#` of their base prices, as text.
pub const PRICES: i16 = 4004;

/// The highest price a good can have: a `jünk`'s `BasePrice` is 16 bits,
/// and the engine reads `STR#` 4004 into 16 bits.
pub const MAX_PRICE: u32 = 0x7FFF;

/// The most stellars a `jünk` lists on each side, `SoldAt1-8` and
/// `BoughtAt1-8`.
pub const JUNK_STELLARS: usize = 8;

/// Each good's standard slot and each other good's `jünk` under `pins`,
/// in that order. Each good past the last `jünk` ID is reported and left
/// out.
pub(super) fn number_goods(
    galaxy: &Galaxy,
    pins: &IdTable,
    problems: &mut Vec<Problem>,
) -> (BTreeMap<Id, i16>, BTreeMap<Id, i16>) {
    let standard = galaxy
        .goods
        .keys()
        .filter(|good| !pins.junk.contains_key(*good));
    let mut free = after_pins(COMMODITY_SLOTS, pins.commodities.values());
    let (commodities, past) = allocate_pinned(standard, &pins.commodities, &mut free);
    let special = galaxy
        .goods
        .iter()
        .filter(|(good, _)| pins.junk.contains_key(*good) || past.contains(good));
    let mut free = after_pins(JUNK_IDS, pins.junk.values());
    let junk = number(special, &pins.junk, &mut free, "jünk", problems);
    (commodities, junk)
}

/// The four commodity lists, when any good takes a standard slot.
pub(super) fn standard_strings(
    galaxy: &Galaxy,
    ids: &IdTable,
    problems: &mut Vec<Problem>,
) -> Vec<Entry<AnyRecord>> {
    if ids.commodities.is_empty() {
        return Vec::new();
    }
    let slots = COMMODITY_SLOTS.map(|slot| {
        let good = ids.commodities.iter().find(|&(_, &held)| held == slot);
        good.map(|(id, _)| &galaxy.goods[id])
    });
    let mut lists: [Vec<MacString>; 4] = Default::default();
    for doc in slots {
        let [names, lc_names, labels, prices] = &mut lists;
        let Some(doc) = doc else {
            for list in [names, lc_names, labels, prices] {
                list.push(MacString::default());
            }
            continue;
        };
        let name = &doc.data.name;
        names.push(name.as_str().into());
        lc_names.push(name.to_lowercase().as_str().into());
        labels.push(format!("{name}:").as_str().into());
        prices.push(price(doc, problems).to_string().as_str().into());
    }
    let [names, lc_names, labels, prices] = lists;
    [
        (NAMES, "Commodity names", names),
        (LC_NAMES, "Commodity names, lower case", lc_names),
        (LABELS, "Commodity labels", labels),
        (PRICES, "Commodity prices", prices),
    ]
    .into_iter()
    .map(|(id, name, strings)| Entry {
        id,
        name: Some(name.to_owned()),
        record: AnyRecord::StrList(Box::new(StrList { strings })),
    })
    .collect()
}

/// The `jünk` of each special good.
pub(super) fn junk(
    galaxy: &Galaxy,
    ids: &IdTable,
    problems: &mut Vec<Problem>,
) -> Vec<Entry<AnyRecord>> {
    let mut by_id: Vec<(i16, &Doc<BibleStellar>)> = (ids.stellars.iter())
        .map(|(id, &nova)| (nova, &galaxy.stellars[id]))
        .filter(|(_, doc)| trades(&doc.data))
        .collect();
    by_id.sort_by_key(|&(nova, _)| nova);
    ids.junk
        .iter()
        .map(|(good, &nova)| {
            let doc = &galaxy.goods[good];
            let mut side = |level: PriceLevel| -> [Option<StellarId>; 8] {
                let at = by_id
                    .iter()
                    .filter(|(_, stellar)| stellar.data.prices.get(good) == Some(&level));
                let mut side = [None; JUNK_STELLARS];
                for (n, &(nova, stellar)) in at.enumerate() {
                    match side.get_mut(n) {
                        Some(slot) => *slot = Some(StellarId(nova)),
                        None => problems.push(Problem::new(
                            &stellar.path,
                            format!(
                                "the good `{good}` is priced {} at more than {JUNK_STELLARS} \
                                 stellars; it has no price here",
                                level.as_str()
                            ),
                        )),
                    }
                }
                side
            };
            let sold_at = side(PriceLevel::Low);
            let bought_at = side(PriceLevel::High);
            let name = &doc.data.name;
            let junk = Junk {
                sold_at,
                bought_at,
                base_price: price(doc, problems),
                flags: Flags16(0),
                scan_mask: Flags16(0),
                lc_name: name.to_lowercase().as_str().into(),
                abbrev: name.as_str().into(),
                buy_on: MacString::default(),
                sell_on: MacString::default(),
            };
            Entry {
                id: nova,
                name: Some(name.clone()),
                record: AnyRecord::Junk(Box::new(junk)),
            }
        })
        .collect()
}

/// The price-level bits of `doc`'s standard goods, for its `spöb`
/// `Flags`, and the problems of its `prices`.
pub(super) fn price_flags(
    galaxy: &Galaxy,
    ids: &IdTable,
    doc: &Doc<BibleStellar>,
    problems: &mut Vec<Problem>,
) -> u32 {
    let data = &doc.data;
    let trades = trades(data);
    if !data.prices.is_empty() && !trades {
        problems.push(Problem::new(
            &doc.path,
            "it prices goods but has no `trade` service; nothing is traded here",
        ));
    }
    let mut bits = 0;
    for (good, &level) in &data.prices {
        if let Some(&slot) = ids.commodities.get(good) {
            if trades {
                let slot = u8::try_from(slot).expect("a slot is 0 to 5");
                bits |= flags::price_bits(slot, level_bit(level));
            }
        } else if ids.junk.contains_key(good) {
            if trades && level == PriceLevel::Medium {
                problems.push(Problem::new(
                    &doc.path,
                    format!(
                        "the good `{good}` takes no standard slot, so it is priced only low or \
                         high; its medium price is left out"
                    ),
                ));
            }
        } else if absent(galaxy, &galaxy.goods, "goods", good) {
            problems.push(Problem::new(
                &doc.path,
                format!("the good `{good}` is not in goods/; its price is left out"),
            ));
        }
    }
    bits
}

/// Whether `stellar` has an exchange.
fn trades(stellar: &BibleStellar) -> bool {
    stellar.services.contains(&Service::Trade)
}

/// `level`'s bit in a commodity's nibble.
fn level_bit(level: PriceLevel) -> u32 {
    match level {
        PriceLevel::Low => flags::LOW_PRICE,
        PriceLevel::Medium => flags::MEDIUM_PRICE,
        PriceLevel::High => flags::HIGH_PRICE,
    }
}

/// `doc`'s price as a record holds it, moved down to [`MAX_PRICE`] and
/// reported when above it.
fn price(doc: &Doc<Good>, problems: &mut Vec<Problem>) -> i16 {
    let price = doc.data.price;
    if price > MAX_PRICE {
        problems.push(Problem::new(
            &doc.path,
            format!("its price {price} is above {MAX_PRICE}; it is priced at {MAX_PRICE}"),
        ));
    }
    i16::try_from(price.min(MAX_PRICE)).expect("at most i16::MAX")
}

#[cfg(test)]
mod tests;
