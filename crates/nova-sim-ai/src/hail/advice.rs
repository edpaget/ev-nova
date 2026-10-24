//! Greetings' advice as `_LoadAdvice` builds it (@0x9133c-0x91b64 in the
//! `EV Nova` executable), read whole.
//!
//! The comm dialog calls it once as it opens (`_DoCommDialog` @0x95b83).
//! It writes the line into the advice buffer (0xdef44), which Greetings
//! copies to a friendly ship's reply (`strncpy` @0x966f7-0x96710).
//! Nothing changes while the modal dialog is open, so every Greetings
//! press in one hail says the same line: the session rolls it as the hail
//! opens and keeps it (`Reply::Rolled`). Ported elsewhere: the kind roll
//! (@0x91365-0x91436, [`deal`](super::deal)), specific advice
//! (@0x918ce-0x919e1), the government's hail from `STR#` 7000 + g
//! (@0x91a03-0x91aa4, [`nova`](super::nova)), the check (@0x91aa9-0x91ae2,
//! [`advice_fallback`](super::reply::advice_fallback)) and the person's
//! `STR#` 7100 quote (@0x91b46-0x91b55).
//!
//! 1. **The start** (@0x91348-0x91360): the buffer holds `STR#` 2002 #175,
//!    "Greetings.", before anything else is read.
//! 2. **The person's override** (@0x9143d-0x91466, [`kind`]): a ship flying
//!    a person (+0xc8d0 ≠ -1) whose `përs` `Flags` has
//!    [`SHOWS_DISASTERS`] (+0x14 negative) and whose `CommQuote` (+0xc) is
//!    exactly -1 goes to the disaster branch, whatever the `düde` roll was.
//!    A person's ship has no `düde` (+0x78 = -1, @0x91371), so its roll is
//!    otherwise none.
//! 3. **Good prices** ([`GOOD_PRICES`], @0x91475-0x91647, [`good_price`]).
//!    A stellar *qualifies* when it is active (+0x44), its `Flags` (+0x30)
//!    has 0x1 or 0x2 (@0x9148a), and it lacks 0x20, uninhabited
//!    (@0x9148f). Destruction isn't tested. The routine counts the
//!    qualifying stellars over all 0x800 slots (stride 0x498) whose
//!    `Flags & 0x55555500` is set, some commodity nibble with its low (1)
//!    or high (4) bit (@0x91494-0x9149d). With none, the line stays
//!    "Greetings." (@0x914ad → @0x919e6). Otherwise it draws `Rand(6)`
//!    for a commodity c and `Rand(0x800)` for a slot s, and repeats until
//!    s qualifies and `_CalcPortDemand(c, Flags)` is low or high
//!    (@0x914b3-0x91514), which [`price_level`] gives too (the lowest
//!    bit of the nibble at bit 28 - 4c). So the pick is uniform over the
//!    valid (stellar, commodity) pairs. The line ([`good_price_line`]) is
//!    the stellar's name (+0x48), " ", #176 "is a good place to", " ",
//!    #177 "buy" for low or #178 "sell" for high, " ", `_lcCargoName[c]`
//!    (0x3b55b8, `STR#` 4001), "." (@0x91516-0x91647).
//!    - *The original's bug, not built:* a stellar counts on a 0x4 bit,
//!      but a nibble of 0x6 or 0xE reads medium. If every counted stellar
//!      is like that, the loop never ends and the game hangs. Stock has no
//!      such nibble. The engine says "Greetings." when no pair is valid. A
//!      hang is a bug, so it gets no `Rulebook` key; this note is the
//!      record.
//! 4. **Disasters** ([`DISASTERS`], @0x91656-0x918c9, [`disaster`]). The
//!    routine counts the 256 `öops` slots (stride 0x210) with days left
//!    (+0xc) of 2 or more (@0x91667-0x9167b). It takes the single one, or
//!    with several draws `Rand(256)` until it hits one (@0x916a7-0x916ca),
//!    which is uniform over them. With none, the line stays "Greetings."
//!    (@0x916a1). A pick whose stellar index (+0x2) is above 0x7ff
//!    unsigned, unplaced, also says "Greetings." (@0x916f3 → @0x919e6),
//!    as `market::place` gives none. The line ([`disaster_line`]) is #179
//!    "The last time I was", " ", #180 "at" when the stellar's `Flags` has
//!    0x10 (a station) or #60 "on" otherwise (@0x91750-0x91761), " ", the
//!    name, ", ", #181 "the price of", " ", `_lcCargoName[Commodity]`
//!    (+0x4), " ", #(182 + `Rand(3)`, [`degree`]) "was very", "was
//!    really" or "was pretty", " ", #185 "low" when `PriceDelta` (+0x6) is
//!    below 0 or #186 "high" otherwise (0 is high, unlike the exchange's
//!    "lowered"), "." (@0x91729-0x918c4). It reads days left as the news
//!    does (more than one day, `_RandomizeNewsDialog` @0x482b5).
//!    - A commodity the loader made -1, or one past the name table, reads
//!      memory outside `_lcCargoName`, which can't be replicated. The
//!      engine names it empty, as the news does. Stock `öops` all have
//!      commodities 0-5 and placed stellars.
//! 5. **The government's plug-in hail** (`InfoTypes` 0x8000,
//!    @0x91a0d-0x91a7d, [`govt_hail_patch`]): before `STR#` 7000 + g,
//!    `_LoadPluginString(10010 + 10g + r)` for a trader (AI type ≤ 2) or
//!    `(10015 + 10g + r)` for any other ship, g the government index (ID -
//!    128, -1 for an independent: 10000 + r and 10005 + r) and r the
//!    variant. If the `'STR '` exists it stands, and the `STR#` isn't
//!    read. `_LoadPluginString` (@0x71a8e-0x71af4) answers "exists" for an
//!    empty one without copying, so the buffer still holds "Greetings.".
//!    It copies an unreadable one (length byte past its data) raw over
//!    that buffer, which the engine can't model: as
//!    [`StringPatch::shown`](crate::catalog::StringPatch::shown) has it,
//!    it reads "", and the check then makes it "Nice to meet you.". The
//!    line goes through the check (@0x91aa9) like any other.
//! 6. **The person's plug-in quote** (@0x91ae7-0x91b55,
//!    [`comm_quote_patch`]): a person whose `CommQuote` q is above 0 reads
//!    `_LoadPluginString(15000 + q)` first, then `STR#` 7100 #q. Both come
//!    after the check, so the line is shown as written. An empty `'STR '`
//!    leaves the buffer, which for a person (no `düde`, so no roll) is
//!    always "Greetings.". An unreadable one reads "", as above.
//! 7. **Sounds:** none. The function's calls are `GetIndString`, `_Rand`,
//!    `_CalcPortDemand`, `GetResource`, `_LoadPluginString`,
//!    `_ParseTextDescription` and string functions.
//!
//! What each draw decides is matched, not their order: the pair is
//! uniform over the valid pairs, the event over those with 2 or more days
//! left, and the degree over three. A pick among one draws nothing.
//!
//! The patch IDs are added in 16 bits by the original; here the sum is
//! checked, and one that doesn't fit names no `'STR '`.

use std::collections::BTreeMap;

use crate::catalog::{CommCatalog, DisasterId, GovtId, StellarId};
use crate::chance::Chance;
use crate::landing::StellarFlags;
use crate::market::{ActiveEvent, COMMODITIES, PriceLevel, price_level};
use crate::traffic::npc::NpcPerson;

use super::nova::FIRST_GOVT;
use super::reply::MESSAGES;

/// `InfoTypes`: good prices.
pub const GOOD_PRICES: u16 = 0x1000;
/// `InfoTypes`: disasters.
pub const DISASTERS: u16 = 0x2000;
/// `përs` `Flags`: a person quoting nothing gives disaster advice.
pub const SHOWS_DISASTERS: u16 = 0x8000;
/// The `CommQuote` of a person that quotes nothing, which alone takes
/// [`SHOWS_DISASTERS`] (@0x9145c).
pub const NO_COMM_QUOTE: i16 = -1;
/// The `'STR '` that patches a person's comm quote q is this + q.
pub const COMM_QUOTE_PATCHES: i16 = 15_000;
/// The `'STR '` that patches government g's trader hail r is this + 10g
/// + r.
pub const TRADER_HAIL_PATCHES: i16 = 10_010;
/// The `'STR '` that patches government g's other ships' hail r is this +
/// 10g + r.
pub const OTHER_HAIL_PATCHES: i16 = 10_015;
/// How far apart two governments' hail patches are.
pub const HAIL_PATCHES_PER_GOVT: i16 = 10;
/// How many days an event needs left to be advice: 2.
pub const DISASTER_DAYS: u16 = 2;
/// How many degrees a disaster line draws from.
pub const DEGREES: u32 = 3;

/// The advice a hailed ship gives: the `rolled` `InfoTypes` bit, or
/// [`DISASTERS`] for a `person` of [`SHOWS_DISASTERS`] and `CommQuote`
/// [`NO_COMM_QUOTE`] (see the module docs).
#[must_use]
pub fn kind(rolled: Option<u16>, person: Option<&NpcPerson>) -> Option<u16> {
    match person {
        Some(person)
            if person.flags & SHOWS_DISASTERS != 0 && person.comm_quote == NO_COMM_QUOTE =>
        {
            Some(DISASTERS)
        }
        _ => rolled,
    }
}

/// A stellar as the good-price advice reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdvicePort {
    /// The stellar.
    pub stellar: StellarId,
    /// Whether it is active (+0x44): shown in some system.
    pub active: bool,
    /// Its `Flags`.
    pub flags: u32,
}

impl AdvicePort {
    /// Whether it qualifies (see the module docs): active, landable or
    /// trading, and inhabited. A destroyed one still does.
    fn qualifies(self) -> bool {
        self.active
            && self.flags & (StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER) != 0
            && self.flags & StellarFlags::UNINHABITED == 0
    }
}

/// A good price: where, what, and whether to buy (low) or sell (high).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PriceTip {
    /// The stellar.
    pub stellar: StellarId,
    /// The commodity, from 0.
    pub commodity: u8,
    /// [`PriceLevel::Low`] or [`PriceLevel::High`].
    pub level: PriceLevel,
}

/// A good price among `ports`, drawn on `chance` uniformly among the
/// pairs of a qualifying stellar and a commodity it prices low or high;
/// none with no such pair (see the module docs).
pub fn good_price(ports: &[AdvicePort], chance: &mut dyn Chance) -> Option<PriceTip> {
    let pairs: Vec<PriceTip> = ports
        .iter()
        .filter(|port| port.qualifies())
        .flat_map(|port| {
            (0..COMMODITIES).filter_map(move |commodity| {
                let level = price_level(port.flags, commodity)?;
                (level != PriceLevel::Medium).then_some(PriceTip {
                    stellar: port.stellar,
                    commodity,
                    level,
                })
            })
        })
        .collect();
    pick(&pairs, chance)
}

/// One of `among`, drawn on `chance` when there are two or more.
fn pick<T: Copy>(among: &[T], chance: &mut dyn Chance) -> Option<T> {
    match among {
        [] => None,
        [only] => Some(*only),
        _ => among
            .get(chance.below(u32::try_from(among.len()).unwrap_or(u32::MAX)) as usize)
            .copied(),
    }
}

/// An event among `events` with [`DISASTER_DAYS`] or more left, drawn on
/// `chance` uniformly among them; none with none (see the module docs).
/// Whether it is placed is the caller's to ask.
pub fn disaster(
    events: &BTreeMap<DisasterId, ActiveEvent>,
    chance: &mut dyn Chance,
) -> Option<DisasterId> {
    let lasting: Vec<DisasterId> = events
        .iter()
        .filter(|(_, active)| active.days >= DISASTER_DAYS)
        .map(|(&id, _)| id)
        .collect();
    pick(&lasting, chance)
}

/// A disaster line's degree, drawn on `chance`: 0 "was very", 1 "was
/// really" or 2 "was pretty".
pub fn degree(chance: &mut dyn Chance) -> u8 {
    chance.below(DEGREES) as u8
}

/// The `STR#` [`MESSAGES`] words the advice lines are built from, each
/// empty when missing (`GetIndString` reads none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AdviceWords {
    /// #60 "on".
    pub on: String,
    /// #176 "is a good place to".
    pub good_place: String,
    /// #177 "buy".
    pub buy: String,
    /// #178 "sell".
    pub sell: String,
    /// #179 "The last time I was".
    pub last_time: String,
    /// #180 "at".
    pub at: String,
    /// #181 "the price of".
    pub price_of: String,
    /// #182-#184 "was very", "was really", "was pretty".
    pub degrees: [String; 3],
    /// #185 "low".
    pub low: String,
    /// #186 "high".
    pub high: String,
}

impl AdviceWords {
    /// The words, read from `strings`'s `STR#` [`MESSAGES`].
    #[must_use]
    pub fn read(strings: &(impl CommCatalog + ?Sized)) -> Self {
        let messages = strings.string_list(MESSAGES);
        let word = |n: usize| messages.get(n - 1).cloned().unwrap_or_default();
        Self {
            on: word(60),
            good_place: word(176),
            buy: word(177),
            sell: word(178),
            last_time: word(179),
            at: word(180),
            price_of: word(181),
            degrees: [word(182), word(183), word(184)],
            low: word(185),
            high: word(186),
        }
    }
}

/// The good-price line for stellar `name`'s `level` of `cargo`: "NAME is
/// a good place to buy|sell CARGO." (see the module docs).
#[must_use]
pub fn good_price_line(name: &str, level: PriceLevel, cargo: &str, words: &AdviceWords) -> String {
    let trade = if level == PriceLevel::Low {
        &words.buy
    } else {
        &words.sell
    };
    format!("{name} {} {trade} {cargo}.", words.good_place)
}

/// The disaster line for an event at stellar `name`, a `station` or not,
/// moving `cargo` `low` or high, said in `degree`: "The last time I was
/// at|on NAME, the price of CARGO was very|really|pretty low|high." (see
/// the module docs). A degree past the three reads empty.
#[must_use]
pub fn disaster_line(
    name: &str,
    station: bool,
    cargo: &str,
    degree: u8,
    low: bool,
    words: &AdviceWords,
) -> String {
    let at = if station { &words.at } else { &words.on };
    let degree = words
        .degrees
        .get(usize::from(degree))
        .map_or("", String::as_str);
    let moved = if low { &words.low } else { &words.high };
    format!(
        "{} {at} {name}, {} {cargo} {degree} {moved}.",
        words.last_time, words.price_of
    )
}

/// The `'STR '` that patches `govt`'s hail in `variant`, a `trader`'s or
/// another ship's (@0x91a0d-0x91a7d): none when it doesn't fit.
#[must_use]
pub fn govt_hail_patch(govt: Option<GovtId>, trader: bool, variant: u8) -> Option<i16> {
    let index = match govt {
        Some(govt) => govt.0.checked_sub(FIRST_GOVT)?,
        None => -1,
    };
    let first = if trader {
        TRADER_HAIL_PATCHES
    } else {
        OTHER_HAIL_PATCHES
    };
    index
        .checked_mul(HAIL_PATCHES_PER_GOVT)?
        .checked_add(first)?
        .checked_add(i16::from(variant))
}

/// The `'STR '` that patches comm quote `quote` (@0x91ae7): none when it
/// doesn't fit.
#[must_use]
pub fn comm_quote_patch(quote: u16) -> Option<i16> {
    i16::try_from(quote).ok()?.checked_add(COMM_QUOTE_PATCHES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::PersonId;
    use crate::landing::StellarFlags;
    use nova_sim_base::fixture::Draws;

    /// A person of `flags` quoting `comm_quote`.
    fn person(flags: u16, comm_quote: i16) -> NpcPerson {
        NpcPerson {
            id: PersonId(128),
            flags,
            coward: 0,
            comm_quote,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: false,
            quoted: false,
            quoted_at: None,
            zero_fuel: false,
        }
    }

    #[test]
    fn a_disaster_showing_person_quoting_nothing_gives_disaster_advice_whatever_the_roll() {
        let shows = person(SHOWS_DISASTERS, -1);
        for rolled in [None, Some(GOOD_PRICES), Some(0x4000), Some(0x8000)] {
            assert_eq!(kind(rolled, Some(&shows)), Some(DISASTERS), "{rolled:?}");
        }
        let mut more = person(SHOWS_DISASTERS | 0x0010, -1);
        more.hail_quote = 3;
        assert_eq!(kind(None, Some(&more)), Some(DISASTERS), "other bits too");
    }

    #[test]
    fn no_other_person_or_ship_overrides_the_roll() {
        for quote in [0, 3, -2] {
            let quoting = person(SHOWS_DISASTERS, quote);
            assert_eq!(kind(Some(0x4000), Some(&quoting)), Some(0x4000), "{quote}");
            assert_eq!(kind(None, Some(&quoting)), None, "{quote}");
        }
        let plain = person(0x7FFF, -1);
        assert_eq!(kind(None, Some(&plain)), None, "no 0x8000");
        assert_eq!(kind(Some(GOOD_PRICES), Some(&plain)), Some(GOOD_PRICES));
        assert_eq!(kind(Some(DISASTERS), None), Some(DISASTERS));
        assert_eq!(kind(None, None), None);
    }

    /// Food (commodity 0) at `food`'s nibble and industrial (1) at
    /// `industrial`'s, the rest untraded.
    const fn priced(food: u32, industrial: u32) -> u32 {
        (food << 28) | (industrial << 24)
    }

    /// An active stellar `id` that can land and trades, of `prices`.
    fn port(id: i16, prices: u32) -> AdvicePort {
        AdvicePort {
            stellar: StellarId(id),
            active: true,
            flags: StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | prices,
        }
    }

    /// Every pair `good_price` can give over `ports`, by scripting each
    /// draw in turn, and the draws it asks with the first.
    fn every_tip(ports: &[AdvicePort]) -> (Vec<PriceTip>, Vec<u32>) {
        let mut first = Draws::of(&[0]);
        let Some(tip) = good_price(ports, &mut first) else {
            return (Vec::new(), first.asked);
        };
        let asked = first.asked.clone();
        let count = asked.first().copied().unwrap_or(1);
        let mut tips = vec![tip];
        for draw in 1..count {
            let tip = good_price(ports, &mut Draws::of(&[draw])).expect("a pair");
            tips.push(tip);
        }
        (tips, asked)
    }

    fn tip(id: i16, commodity: u8, level: PriceLevel) -> PriceTip {
        PriceTip {
            stellar: StellarId(id),
            commodity,
            level,
        }
    }

    #[test]
    fn a_good_price_is_picked_uniformly_among_the_low_and_high_pairs() {
        let ports = [port(128, priced(1, 4)), port(129, priced(2, 1))];
        let (tips, asked) = every_tip(&ports);
        assert_eq!(asked, [3], "one draw over the three pairs");
        assert_eq!(
            tips,
            [
                tip(128, 0, PriceLevel::Low),
                tip(128, 1, PriceLevel::High),
                tip(129, 1, PriceLevel::Low),
            ],
            "a medium price is no tip"
        );
    }

    #[test]
    fn a_single_good_price_is_given_with_no_draw() {
        let (tips, asked) = every_tip(&[port(130, priced(0, 4))]);
        assert_eq!(tips, [tip(130, 1, PriceLevel::High)]);
        assert!(asked.is_empty());
    }

    #[test]
    fn a_nibble_reads_its_lowest_level() {
        // `_CalcPortDemand` tests low, then medium, then high: 0x5 is low
        // and 0x6 and 0xE medium, no tip.
        let (tips, _) = every_tip(&[port(128, priced(5, 6)), port(129, priced(0xE, 0xC))]);
        assert_eq!(
            tips,
            [tip(128, 0, PriceLevel::Low), tip(129, 1, PriceLevel::High)]
        );
    }

    #[test]
    fn every_commodity_can_be_a_tip() {
        let all_low = 0x1111_1100 | 0x1F;
        let (tips, asked) = every_tip(&[port(128, all_low)]);
        assert_eq!(asked, [6]);
        let commodities: Vec<u8> = tips.iter().map(|tip| tip.commodity).collect();
        assert_eq!(commodities, [0, 1, 2, 3, 4, 5], "the low byte names none");
    }

    #[test]
    fn an_inactive_uninhabited_or_closed_stellar_gives_no_tip() {
        let prices = priced(1, 4);
        let inactive = AdvicePort {
            active: false,
            ..port(128, prices)
        };
        let uninhabited = AdvicePort {
            flags: port(128, prices).flags | StellarFlags::UNINHABITED,
            ..port(128, prices)
        };
        let closed = AdvicePort {
            flags: prices | StellarFlags::OUTFITTER | StellarFlags::STATION,
            ..port(128, prices)
        };
        for ports in [[inactive], [uninhabited], [closed]] {
            assert_eq!(good_price(&ports, &mut Draws::of(&[])), None, "{ports:?}");
        }
        for open in [StellarFlags::CAN_LAND, StellarFlags::TRADE_CENTER] {
            let either = AdvicePort {
                flags: prices | open,
                ..port(128, prices)
            };
            assert!(
                good_price(&[either], &mut Draws::of(&[0])).is_some(),
                "{open}"
            );
        }
    }

    #[test]
    fn with_no_low_or_high_pair_there_is_no_tip() {
        // The original's loop never ends here (see the module docs).
        let ports = [port(128, priced(2, 6)), port(129, 0)];
        assert_eq!(good_price(&ports, &mut Draws::of(&[])), None);
        assert_eq!(good_price(&[], &mut Draws::of(&[])), None);
    }

    fn events(days: &[(i16, u16)]) -> BTreeMap<DisasterId, ActiveEvent> {
        days.iter()
            .map(|&(id, days)| {
                (
                    DisasterId(id),
                    ActiveEvent {
                        days,
                        stellar: Some(StellarId(128)),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn a_disaster_is_picked_uniformly_among_those_with_two_days_or_more() {
        let under_way = events(&[(128, 2), (129, 1), (130, 0), (131, 9)]);
        let mut chance = Draws::of(&[0]);
        assert_eq!(disaster(&under_way, &mut chance), Some(DisasterId(128)));
        assert_eq!(chance.asked, [2]);
        let mut chance = Draws::of(&[1]);
        assert_eq!(disaster(&under_way, &mut chance), Some(DisasterId(131)));
    }

    #[test]
    fn a_single_disaster_is_given_with_no_draw() {
        let under_way = events(&[(129, 1), (140, 3)]);
        let mut chance = Draws::of(&[]);
        assert_eq!(disaster(&under_way, &mut chance), Some(DisasterId(140)));
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn with_no_disaster_of_two_days_there_is_none() {
        let mut chance = Draws::of(&[]);
        assert_eq!(disaster(&events(&[(128, 1), (129, 0)]), &mut chance), None);
        assert_eq!(disaster(&BTreeMap::new(), &mut chance), None);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn the_degree_is_one_of_three() {
        for draw in 0..3 {
            let mut chance = Draws::of(&[draw]);
            assert_eq!(degree(&mut chance), draw as u8);
            assert_eq!(chance.asked, [3]);
        }
    }

    /// `STR#` 2002: "m<n>" but for the advice's words.
    struct Messages;

    impl CommCatalog for Messages {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id != 2002 {
                return Vec::new();
            }
            (1..=190)
                .map(|n| {
                    match n {
                        60 => "on",
                        176 => "is a good place to",
                        177 => "buy",
                        178 => "sell",
                        179 => "The last time I was",
                        180 => "at",
                        181 => "the price of",
                        182 => "was very",
                        183 => "was really",
                        184 => "was pretty",
                        185 => "low",
                        186 => "high",
                        _ => "",
                    }
                    .to_owned()
                })
                .map(|word| {
                    if word.is_empty() {
                        "m".to_owned()
                    } else {
                        word
                    }
                })
                .collect()
        }
    }

    #[test]
    fn the_words_are_read_from_the_messages() {
        let words = AdviceWords::read(&Messages);
        assert_eq!(
            words,
            AdviceWords {
                on: "on".to_owned(),
                good_place: "is a good place to".to_owned(),
                buy: "buy".to_owned(),
                sell: "sell".to_owned(),
                last_time: "The last time I was".to_owned(),
                at: "at".to_owned(),
                price_of: "the price of".to_owned(),
                degrees: [
                    "was very".to_owned(),
                    "was really".to_owned(),
                    "was pretty".to_owned()
                ],
                low: "low".to_owned(),
                high: "high".to_owned(),
            }
        );
        assert_eq!(AdviceWords::read(&Nothing), AdviceWords::default());
    }

    /// No strings at all.
    struct Nothing;

    impl CommCatalog for Nothing {
        fn string_list(&self, _id: i16) -> Vec<String> {
            Vec::new()
        }
    }

    #[test]
    fn a_good_price_line_names_the_stellar_the_trade_and_the_cargo() {
        let words = AdviceWords::read(&Messages);
        assert_eq!(
            good_price_line("Earth", PriceLevel::Low, "food", &words),
            "Earth is a good place to buy food."
        );
        assert_eq!(
            good_price_line("Earth", PriceLevel::High, "metal", &words),
            "Earth is a good place to sell metal."
        );
    }

    #[test]
    fn a_disaster_line_says_at_a_station_and_on_a_planet() {
        let words = AdviceWords::read(&Messages);
        assert_eq!(
            disaster_line("Earth", false, "food", 0, true, &words),
            "The last time I was on Earth, the price of food was very low."
        );
        assert_eq!(
            disaster_line("Port", true, "metal", 1, false, &words),
            "The last time I was at Port, the price of metal was really high."
        );
        assert_eq!(
            disaster_line("Port", true, "", 2, false, &words),
            "The last time I was at Port, the price of  was pretty high.",
            "a cargo with no name"
        );
    }

    #[test]
    fn a_degree_past_the_three_reads_empty() {
        let words = AdviceWords::read(&Messages);
        assert_eq!(
            disaster_line("Earth", false, "food", 3, true, &words),
            "The last time I was on Earth, the price of food  low."
        );
    }

    #[test]
    fn a_government_hail_is_patched_by_its_governments_trader_or_other_strings() {
        for variant in 0..5 {
            let r = i16::from(variant);
            assert_eq!(govt_hail_patch(None, true, variant), Some(10_000 + r));
            assert_eq!(govt_hail_patch(None, false, variant), Some(10_005 + r));
            let fed = Some(GovtId(128));
            assert_eq!(govt_hail_patch(fed, true, variant), Some(10_010 + r));
            assert_eq!(govt_hail_patch(fed, false, variant), Some(10_015 + r));
            let last = Some(GovtId(383));
            assert_eq!(govt_hail_patch(last, true, variant), Some(12_560 + r));
            assert_eq!(govt_hail_patch(last, false, variant), Some(12_565 + r));
        }
        assert_eq!(govt_hail_patch(Some(GovtId(130)), true, 2), Some(10_032));
    }

    #[test]
    fn a_patch_past_the_ids_is_none() {
        assert_eq!(govt_hail_patch(Some(GovtId(i16::MAX)), true, 0), None);
        assert_eq!(govt_hail_patch(Some(GovtId(2_404)), false, 4), None);
        assert_eq!(govt_hail_patch(Some(GovtId(i16::MIN)), true, 0), None);
        assert_eq!(comm_quote_patch(1), Some(15_001));
        assert_eq!(comm_quote_patch(17_767), Some(i16::MAX));
        assert_eq!(comm_quote_patch(17_768), None);
        assert_eq!(comm_quote_patch(u16::MAX), None);
    }
}
