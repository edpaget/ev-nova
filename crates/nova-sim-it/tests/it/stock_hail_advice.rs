//! Greetings' good-price and disaster advice over the stock data
//! (`_LoadAdvice` @0x9133c). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - `STR#` 2002's advice words: #60 "on", #175 "Greetings.", and #176 to
//!   #186.
//! - 26 `düde`s give good prices (`InfoTypes` 0x1000, 129 among them) and
//!   24 disasters (0x2000).
//! - 76 `përs` show disasters (`Flags` 0x8000) and quote nothing
//!   (`CommQuote` -1), 292 among them.
//! - Every stock stellar taken as active, a good price is found, each low
//!   or high, and no stellar counts a price the original's loop would
//!   never find (a nibble with its 4 bit that reads medium).
//! - Every stock `öops` moves a commodity 0-5.

use crate::common;

use nova_data::GameData;
use nova_sim::hail::advice::{self, AdvicePort, AdviceWords, DISASTERS, GOOD_PRICES};
use nova_sim::market::{COMMODITIES, PriceLevel, price_level};
use nova_sim::{Chance, CommCatalog, DudeId, PilotCatalog, TrafficCatalog};

/// Draws each outcome in turn.
struct Counting(u32);

impl Chance for Counting {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        let drawn = self.0 % n;
        self.0 += 1;
        drawn
    }
}

fn data() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

#[test]
fn the_advice_words_are_stocks_messages() {
    let Some(data) = data() else {
        return;
    };
    let messages = data.string_list(2002);
    assert_eq!(messages[59], "on");
    assert_eq!(messages[174], "Greetings.");
    let words = AdviceWords::read(&data);
    assert_eq!(
        [
            words.good_place.as_str(),
            &words.buy,
            &words.sell,
            &words.last_time,
            &words.at,
            &words.price_of,
            &words.degrees[0],
            &words.degrees[1],
            &words.degrees[2],
            &words.low,
            &words.high,
            &words.on,
        ],
        [
            "is a good place to",
            "buy",
            "sell",
            "The last time I was",
            "at",
            "the price of",
            "was very",
            "was really",
            "was pretty",
            "low",
            "high",
            "on",
        ]
    );
}

#[test]
fn stock_dudes_give_good_prices_and_disasters() {
    let Some(data) = data() else {
        return;
    };
    let dudes: Vec<_> = (128..=i16::MAX)
        .filter_map(|id| data.dude(DudeId(id)).map(|dude| (id, dude.info_types)))
        .collect();
    let giving = |kind: u16| {
        dudes
            .iter()
            .filter(|(_, info_types)| info_types & kind != 0)
            .count()
    };
    assert_eq!((giving(GOOD_PRICES), giving(DISASTERS)), (26, 24));
    assert!(
        dudes
            .iter()
            .any(|&(id, info_types)| id == 129 && info_types & GOOD_PRICES != 0)
    );
}

#[test]
fn stock_persons_showing_disasters_and_quoting_nothing_give_disaster_advice() {
    let Some(data) = data() else {
        return;
    };
    let persons = data.persons();
    let showing: Vec<i16> = persons
        .iter()
        .filter(|person| {
            person.flags & advice::SHOWS_DISASTERS != 0
                && person.comm_quote == advice::NO_COMM_QUOTE
        })
        .map(|person| person.id.0)
        .collect();
    assert_eq!(showing.len(), 76);
    assert!(showing.contains(&292));
}

#[test]
fn every_stock_stellar_active_gives_a_good_price_low_or_high() {
    let Some(data) = data() else {
        return;
    };
    let ports: Vec<AdvicePort> = data
        .listed_stellars()
        .iter()
        .map(|listed| AdvicePort {
            stellar: listed.id,
            active: true,
            flags: listed.flags,
        })
        .collect();
    let mut chance = Counting(0);
    for _ in 0..200 {
        let tip = advice::good_price(&ports, &mut chance).expect("a good price");
        assert_ne!(tip.level, PriceLevel::Medium);
        let port = ports
            .iter()
            .find(|port| port.stellar == tip.stellar)
            .expect("listed");
        assert_eq!(price_level(port.flags, tip.commodity), Some(tip.level));
    }
    // The original counts a stellar on a nibble's 1 or 4 bit and then
    // loops until it finds a low or high one: no stock nibble with its 4
    // bit reads medium, so the loop always ends.
    for port in &ports {
        for commodity in 0..COMMODITIES {
            let nibble = (port.flags >> (28 - 4 * u32::from(commodity))) & 0xF;
            if nibble & 0x4 != 0 {
                assert_ne!(
                    price_level(port.flags, commodity),
                    Some(PriceLevel::Medium),
                    "{:?} {commodity}",
                    port.stellar
                );
            }
        }
    }
}

#[test]
fn every_stock_event_moves_a_named_commodity() {
    let Some(data) = data() else {
        return;
    };
    let disasters = data.disasters();
    assert!(!disasters.is_empty());
    for event in &disasters {
        assert!((0..6).contains(&event.commodity), "{:?}", event.id);
    }
}
