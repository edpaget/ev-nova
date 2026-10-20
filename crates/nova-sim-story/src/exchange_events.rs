//! The exchange's event line: what the commodity exchange says of the
//! `öops` price events active at the stellar docked at, as
//! `_TradeDialogUpdate` (0x4ceea-0x4e092) draws it.
//!
//! # What the original does
//!
//! After the rows and the cargo summary, `_TradeDialogUpdate` fetches
//! item 15's rect and draws nothing unless it is in the visible region
//! (@0x4dcd0-0x4dcf5). It then runs over the 256 `öops` slots. A slot
//! counts when it is loaded (+0x20e, @0x4dd2c), has days left above 0
//! (`cmpw $0,0xc; jle`, @0x4dd39) and its stellar (+2) is the docked
//! stellar (+0x6c, @0x4dd4a-0x4dd52). There is no `Stellar` -2 test, as
//! the news has (@0x482d0). The first slot that counts is drawn and the
//! loop exits (`jmp 0x4dfdb` @0x4dfba), so only the lowest-ID local event
//! shows. The news needs more than one day (@0x482b5-0x482c3), so an
//! event shows here on its last day but never makes the news.
//!
//! The line ([`event_line`], @0x4ddd7-0x4df3c) is the name (`strncpy` of
//! at most 63 bytes from +0x10d, then `PtoCstr`) + " " + `STR#` 2002
//! #[`HAS`] + " " + #[`RAISED`] for a `PriceDelta` above 0, else
//! #[`LOWERED`] (0 included, @0x4de58) + " " + #[`PRICE_OF`] + " " +
//! `_lcCargoName[commodity]` + "." (`STR#` [`CARGO_NAMES`],
//! @0x4ded3-0x4df0b). There is no "Today's top news:" and no "on <stellar>", and an
//! empty name is drawn as it is. Stock gives "An enormous food surplus
//! has lowered the price of food." for `öops` 128 at Port Kane.
//!
//! The loader turns a negative commodity into -1 (@0x7cd6f), and the
//! line then reads `_lcCargoName[-1]`, 256 bytes before the table, out
//! of bounds (@0x4dee2). Such a slot is still the first local one, so the
//! loop exits on it and a later local event is not shown. The engine's
//! reading gives it no line. It moves no price either
//! ([`market::event_commodity`]), the engine's reading of the
//! `_commodityPrice[-1]` write (`_DoTradeDialog` @0x5dd19-0x5dd40).
//!
//! The range plays no sound: its calls are drawing, strings,
//! `_TotalFleetHolds`, `_ShipTotalHolds`, `_TotalCargo` and
//! `_DrawTradeButtons`.
//!
//! # The rule
//!
//! [`exchange_events`] is disputed by [`RuleKey::ExchangeEvents`]: by
//! the engine, the first local event's line, as above; by the other
//! reading, the name of every local event that moves a commodity, by ID,
//! as the port showed them before. The session sets `Market.events` from
//! it (`nova_sim::Session::market`), since the line needs `STR#` strings
//! the state layer cannot read.
//!
//! # Left out
//!
//! - The debug flag 0x3b5144 (@0x4dd58-0x4ddd5), which puts "Disaster ID
//!   " + (slot + 128) in place of the name: the engine has no debug mode.
//! - The 63-byte name copy: a name over 62 characters would read past
//!   the copy. Stock names run to 34. A width, noted only.
//! - Where and how the line is drawn (item 15's rect, `TETextBox`'s wrap,
//!   black then `InvertRect`, @0x4df54-0x4dfb5), and the rest of the
//!   dialog the engine does not draw as the original does: the header
//!   (@0x4cf77-0x4d0b6), the row fill and edges, the price drawn only
//!   above 0 (@0x4d371-0x4d3c9), the row markers (@0x4d3ec-0x4d521) and
//!   item 12's cargo summary (@0x4d5cb-0x4dc93). These go to the
//!   crons-news-ranks-fidelity follow-up phase on the exchange's drawing.

use std::collections::BTreeMap;

use crate::catalog::{DisasterId, DisasterRecord, StellarId};
use crate::market::{self, ActiveEvent};
use crate::news::StoryWords;
#[cfg(doc)]
use crate::news::{CARGO_NAMES, HAS, LOWERED, PRICE_OF, RAISED};
use crate::rulebook::{RuleKey, Rulebook};
use crate::rules::{Context, Value};
use crate::text_tags::TagNames;

/// What the exchange's event line reads: the events under way, their
/// records, the stellar docked at, the words, the names and the rule
/// set.
pub struct ExchangeEventFacts<'a> {
    /// The events under way, by ID.
    pub events: &'a BTreeMap<DisasterId, ActiveEvent>,
    /// Every `öops`.
    pub disasters: &'a [DisasterRecord],
    /// The stellar docked at.
    pub docked: StellarId,
    /// The `STR#` 2002 pieces of the line.
    pub words: &'a StoryWords,
    /// The cargo names.
    pub names: &'a dyn TagNames,
    /// The rule set in force.
    pub rules: &'a Rulebook,
}

impl Context for ExchangeEventFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// `event`'s line: "NAME has raised|lowered the price of CARGO."
/// (`_TradeDialogUpdate` @0x4ddd7-0x4df3c). `None` for a negative
/// commodity, which the original names `_lcCargoName[-1]`, out of bounds
/// (@0x4dee2); a commodity above 5 is named as equipment, as the loader
/// clamps it. A missing word or cargo name reads empty, as
/// `StrcatIndCString` appends nothing, and an empty name is drawn as it
/// is.
#[must_use]
pub fn event_line(
    event: &DisasterRecord,
    words: &StoryWords,
    names: &dyn TagNames,
) -> Option<String> {
    let cargo = market::event_commodity(event.commodity)?;
    let moved = if event.price_delta > 0 {
        &words.raised
    } else {
        &words.lowered
    };
    Some(format!(
        "{} {} {} {} {}.",
        event.name,
        words.has,
        moved,
        words.price_of,
        names.cargo_name(cargo).unwrap_or_default(),
    ))
}

/// What the exchange shows of the local events (see the module docs).
#[must_use]
pub fn exchange_events<'a>() -> Value<ExchangeEventFacts<'a>, Vec<String>> {
    Value::disputed(
        "exchange events",
        "_TradeDialogUpdate @0x4dd12-0x4dfba",
        RuleKey::ExchangeEvents,
        |facts: &ExchangeEventFacts| {
            local(facts)
                .next()
                .and_then(|event| event_line(event, facts.words, facts.names))
                .into_iter()
                .collect()
        },
        |facts: &ExchangeEventFacts| {
            local(facts)
                .filter(|event| market::event_commodity(event.commodity).is_some())
                .map(|event| event.name.clone())
                .collect()
        },
    )
}

/// The records of the events with days left above 0 (@0x4dd39) at the
/// docked stellar (@0x4dd4a-0x4dd52), by ascending ID; an event with no
/// record is skipped.
fn local<'a>(facts: &'a ExchangeEventFacts) -> impl Iterator<Item = &'a DisasterRecord> {
    facts
        .events
        .iter()
        .filter(|(_, active)| active.days > 0)
        .filter_map(|(&id, &active)| {
            let record = facts.disasters.iter().find(|record| record.id == id)?;
            (market::place(active, record) == Some(facts.docked)).then_some(record)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::rulebook::RuleSource;

    const DOCKED: StellarId = StellarId(140);
    const ELSEWHERE: StellarId = StellarId(141);

    /// Cargo names: 0 food, 5 equipment.
    struct Names;

    impl TagNames for Names {
        fn stellar_name(&self, _stellar: StellarId) -> Option<String> {
            None
        }

        fn system_name(&self, _system: SystemId) -> Option<String> {
            None
        }

        fn stellar_system(&self, _stellar: StellarId) -> Option<SystemId> {
            None
        }

        fn cargo_name(&self, cargo: u8) -> Option<String> {
            match cargo {
                0 => Some("food".to_owned()),
                5 => Some("equipment".to_owned()),
                _ => None,
            }
        }
    }

    fn words() -> StoryWords {
        StoryWords {
            top_news: "Today's top news:".to_owned(),
            has: "has".to_owned(),
            raised: "raised".to_owned(),
            lowered: "lowered".to_owned(),
            price_of: "the price of".to_owned(),
            on: "on".to_owned(),
        }
    }

    /// A food event `id` named `name`, its record at `stellar`, moving
    /// the price by `price_delta`.
    fn oops(id: i16, name: &str, stellar: i16, price_delta: i16) -> DisasterRecord {
        DisasterRecord {
            id: DisasterId(id),
            name: name.to_owned(),
            stellar,
            price_delta,
            ..DisasterRecord::default()
        }
    }

    fn line(event: &DisasterRecord) -> Option<String> {
        event_line(event, &words(), &Names)
    }

    #[test]
    fn the_line_is_the_name_has_raised_or_lowered_the_price_of_the_cargo() {
        assert_eq!(
            line(&oops(130, "A minor drought", 140, 15)).as_deref(),
            Some("A minor drought has raised the price of food.")
        );
        assert_eq!(
            line(&oops(128, "An enormous food surplus", 140, -15)).as_deref(),
            Some("An enormous food surplus has lowered the price of food.")
        );
        assert_eq!(
            line(&oops(130, "A minor drought", 140, 1)).as_deref(),
            Some("A minor drought has raised the price of food.")
        );
    }

    #[test]
    fn a_zero_delta_is_lowered() {
        assert_eq!(
            line(&oops(130, "Calm", 140, 0)).as_deref(),
            Some("Calm has lowered the price of food."),
            "@0x4de58"
        );
    }

    #[test]
    fn a_negative_commodity_has_no_line() {
        let negative = DisasterRecord {
            commodity: -1,
            ..oops(130, "Odd", 140, -10)
        };
        assert_eq!(line(&negative), None);
    }

    #[test]
    fn a_commodity_above_5_is_named_as_equipment() {
        for commodity in [5, 6, 9] {
            let beyond = DisasterRecord {
                commodity,
                ..oops(130, "Odd", 140, -10)
            };
            assert_eq!(
                line(&beyond).as_deref(),
                Some("Odd has lowered the price of equipment."),
                "{commodity}"
            );
        }
    }

    #[test]
    fn missing_words_read_empty() {
        let unknown = DisasterRecord {
            commodity: 2,
            ..oops(130, "A drought", 140, 15)
        };
        assert_eq!(
            line(&unknown).as_deref(),
            Some("A drought has raised the price of ."),
            "no cargo name"
        );
        assert_eq!(
            event_line(
                &oops(130, "A drought", 140, 15),
                &StoryWords::default(),
                &Names
            )
            .as_deref(),
            Some("A drought    food."),
            "no words"
        );
        assert_eq!(
            line(&oops(130, "", 140, 15)).as_deref(),
            Some(" has raised the price of food."),
            "an empty name is not refused"
        );
    }

    /// Event `id` active with `days` left, stored at `at`.
    fn active(id: i16, at: Option<StellarId>, days: u16) -> (DisasterId, ActiveEvent) {
        (DisasterId(id), ActiveEvent { days, stellar: at })
    }

    /// What the exchange at `docked` shows of `events` (records
    /// `disasters`) under `source`.
    fn shown(
        events: &[(DisasterId, ActiveEvent)],
        disasters: &[DisasterRecord],
        docked: StellarId,
        source: RuleSource,
    ) -> Vec<String> {
        let events: BTreeMap<_, _> = events.iter().copied().collect();
        let rules = Rulebook::default().with_override(RuleKey::ExchangeEvents, source);
        exchange_events().of(&ExchangeEventFacts {
            events: &events,
            disasters,
            docked,
            words: &words(),
            names: &Names,
            rules: &rules,
        })
    }

    /// The stock-like events: 128 a food surplus (-15) and 130 a drought
    /// (+15) at [`DOCKED`], 129 a glut of industrial (-70) elsewhere.
    fn disasters() -> Vec<DisasterRecord> {
        vec![
            oops(128, "An enormous food surplus", 140, -15),
            DisasterRecord {
                commodity: 1,
                ..oops(129, "A glut on the market", 141, -70)
            },
            oops(130, "A minor drought", 140, 15),
        ]
    }

    #[test]
    fn the_engine_shows_only_the_first_local_events_line() {
        let both = [active(130, Some(DOCKED), 7), active(128, Some(DOCKED), 1)];
        assert_eq!(
            shown(&both, &disasters(), DOCKED, RuleSource::Engine),
            ["An enormous food surplus has lowered the price of food."]
        );
        assert_eq!(
            shown(
                &[active(130, Some(DOCKED), 7)],
                &disasters(),
                DOCKED,
                RuleSource::Engine
            ),
            ["A minor drought has raised the price of food."]
        );
        assert_eq!(
            shown(&[], &disasters(), DOCKED, RuleSource::Engine),
            Vec::<String>::new()
        );
    }

    #[test]
    fn the_engine_picks_by_id_not_record_order() {
        let mut records = disasters();
        records.reverse();
        let both = [active(130, Some(DOCKED), 7), active(128, Some(DOCKED), 1)];
        assert_eq!(
            shown(&both, &records, DOCKED, RuleSource::Engine),
            ["An enormous food surplus has lowered the price of food."]
        );
    }

    #[test]
    fn an_event_elsewhere_is_not_shown() {
        for source in RuleSource::ALL {
            let glut = [active(129, Some(ELSEWHERE), 3)];
            assert_eq!(
                shown(&glut, &disasters(), DOCKED, source),
                Vec::<String>::new(),
                "{source:?}"
            );
            let everywhere = [oops(131, "News only", -2, -15)];
            assert_eq!(
                shown(
                    &[active(131, Some(ELSEWHERE), 3)],
                    &everywhere,
                    DOCKED,
                    source
                ),
                Vec::<String>::new(),
                "no Stellar -2 test, {source:?}"
            );
            assert_eq!(
                shown(&[active(131, None, 3)], &everywhere, DOCKED, source),
                Vec::<String>::new(),
                "a -2 record names no stellar, {source:?}"
            );
        }
        let first_elsewhere = [
            active(129, Some(ELSEWHERE), 3),
            active(130, Some(DOCKED), 3),
        ];
        assert_eq!(
            shown(&first_elsewhere, &disasters(), DOCKED, RuleSource::Engine),
            ["A minor drought has raised the price of food."],
            "a lower ID elsewhere does not stop the loop"
        );
    }

    #[test]
    fn an_event_on_its_last_day_is_shown() {
        for source in RuleSource::ALL {
            let last = [active(130, Some(DOCKED), 1)];
            assert_eq!(
                shown(&last, &disasters(), DOCKED, source).len(),
                1,
                "{source:?}"
            );
            let gone = [active(130, Some(DOCKED), 0)];
            assert_eq!(
                shown(&gone, &disasters(), DOCKED, source),
                Vec::<String>::new(),
                "{source:?}"
            );
        }
        let gone_first = [active(128, Some(DOCKED), 0), active(130, Some(DOCKED), 2)];
        assert_eq!(
            shown(&gone_first, &disasters(), DOCKED, RuleSource::Engine),
            ["A minor drought has raised the price of food."],
            "a slot of 0 days does not count"
        );
    }

    #[test]
    fn an_event_with_no_record_is_skipped() {
        for source in RuleSource::ALL {
            let gone = [active(127, Some(DOCKED), 3), active(130, Some(DOCKED), 3)];
            assert_eq!(
                shown(&gone, &disasters(), DOCKED, source).len(),
                1,
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_first_event_of_commodity_minus_one_shows_nothing() {
        let mut records = disasters();
        records[0].commodity = -1;
        let both = [active(128, Some(DOCKED), 3), active(130, Some(DOCKED), 3)];
        assert_eq!(
            shown(&both, &records, DOCKED, RuleSource::Engine),
            Vec::<String>::new(),
            "not replaced by a later local event"
        );
        assert_eq!(
            shown(&both, &records, DOCKED, RuleSource::Bible),
            ["A minor drought"],
            "left out of the names"
        );
    }

    #[test]
    fn the_bible_names_every_local_event() {
        let bible = RuleSource::Bible;
        let both = [active(130, Some(DOCKED), 7), active(128, Some(DOCKED), 1)];
        assert_eq!(
            shown(&both, &disasters(), DOCKED, bible),
            ["An enormous food surplus", "A minor drought"],
            "by ID"
        );
        let mut records = disasters();
        records.reverse();
        assert_eq!(
            shown(&both, &records, DOCKED, bible),
            ["An enormous food surplus", "A minor drought"],
            "by ID, not record order"
        );
        assert_eq!(
            shown(
                &[active(128, Some(DOCKED), 30)],
                &disasters(),
                DOCKED,
                bible
            ),
            ["An enormous food surplus"]
        );
        assert_eq!(
            shown(&[], &disasters(), DOCKED, bible),
            Vec::<String>::new()
        );
    }

    #[test]
    fn an_event_above_the_standard_commodities_shows_as_equipment() {
        let records = [DisasterRecord {
            commodity: 6,
            ..oops(140, "Odd", 140, -10)
        }];
        let odd = [active(140, Some(DOCKED), 3)];
        assert_eq!(
            shown(&odd, &records, DOCKED, RuleSource::Engine),
            ["Odd has lowered the price of equipment."]
        );
        assert_eq!(shown(&odd, &records, DOCKED, RuleSource::Bible), ["Odd"]);
    }

    #[test]
    fn an_event_shows_at_its_stored_stellar_only() {
        let surplus = |source| match source {
            RuleSource::Engine => vec!["Surplus has lowered the price of food.".to_owned()],
            RuleSource::Bible => vec!["Surplus".to_owned()],
        };
        for source in RuleSource::ALL {
            for record in [-1, 140] {
                let records = [oops(128, "Surplus", record, -15)];
                let stored = [active(128, Some(ELSEWHERE), 3)];
                assert_eq!(
                    shown(&stored, &records, ELSEWHERE, source),
                    surplus(source),
                    "{record} {source:?}"
                );
                assert_eq!(
                    shown(&stored, &records, DOCKED, source),
                    Vec::<String>::new(),
                    "{record} {source:?}"
                );
            }
        }
    }

    #[test]
    fn an_event_from_an_older_save_is_at_its_records_stellar() {
        for source in RuleSource::ALL {
            let old = [active(128, None, 3)];
            let at_140 = [oops(128, "Surplus", 140, -15)];
            assert_eq!(shown(&old, &at_140, DOCKED, source).len(), 1, "{source:?}");
            assert_eq!(
                shown(&old, &at_140, ELSEWHERE, source),
                Vec::<String>::new(),
                "{source:?}"
            );
            let anywhere = [oops(128, "Surplus", -1, -15)];
            for here in [DOCKED, ELSEWHERE, StellarId(-1)] {
                assert_eq!(
                    shown(&old, &anywhere, here, source),
                    Vec::<String>::new(),
                    "{here:?} {source:?}"
                );
            }
        }
    }

    #[test]
    fn the_rule_names_its_key() {
        let about = *exchange_events().about();
        assert_eq!(about.reads(), Some(RuleKey::ExchangeEvents));
        assert!(!about.draws());
    }
}
