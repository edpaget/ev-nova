//! Greetings' rolled advice: the session's side of
//! [`hail::advice`](crate::hail::advice).
//!
//! As the hail opens ([`Session::hail`](super::Session::hail)), the
//! session settles the advice the ship gives ([`advice::kind`]: a person
//! showing disasters overrides the roll) and rolls a good-price or
//! disaster line once ([`rolled_line`]), as `_DoCommDialog` calls
//! `_LoadAdvice` once as it opens (@0x95b83). A
//! Greetings press then says the line kept ([`Reply::Rolled`]), checked
//! by the option's [`RuleKey::LongAdvice`](crate::RuleKey::LongAdvice)
//! reading. The flight is paused under the comm dialog, so rolling at the
//! open and saying at the press crosses no boundary the player sees.
//!
//! - **Good prices**: every stellar some system lists, active while the
//!   galaxy shows it in its holder ([`Chart::advice_ports`]), its `Flags`
//!   as read; the pick is [`advice::good_price`]'s, the stellar named by
//!   the session's names and the cargo by `STR#` 4001.
//! - **Disasters**: the pilot's events under way ([`advice::disaster`]),
//!   placed by [`market::place`]; one unplaced, or of no record, is no
//!   advice. Its stellar is a station by its `spöb` `Flags` 0x10, read for
//!   every `spöb` as the session starts, listed or not.
//!
//! The words come from the hail's string lists ([`AdviceWords`]).

use std::collections::BTreeMap;

use crate::catalog::{DisasterId, DisasterRecord, StellarId};
use crate::chance::Chance;
use crate::hail::advice::{
    self, AdvicePort, AdviceWords, DISASTERS, GOOD_PRICES, disaster_line, good_price_line,
};
use crate::landing::StellarFlags;
use crate::market::{self, ActiveEvent};
use crate::pilot::Pilot;
use crate::text_tags::TagNames;

use super::{Chart, Statics};

#[cfg(doc)]
use crate::hail::Reply;

impl Chart {
    /// Every stellar some system lists, as the good-price advice reads
    /// it: active while the galaxy shows it in its holder
    /// ([`OverlaidGalaxy::active_holder`](crate::story::OverlaidGalaxy::active_holder),
    /// +0x44), the player in `pilot`'s system, and its `Flags`.
    pub(super) fn advice_ports(&self, pilot: &Pilot, statics: &Statics) -> Vec<AdvicePort> {
        statics
            .listed_stellars
            .iter()
            .map(|listed| AdvicePort {
                stellar: listed.id,
                active: self
                    .galaxy
                    .active_holder(listed.id, &listed.systems, pilot.system)
                    .is_some(),
                flags: listed.flags,
            })
            .collect()
    }
}

/// What a rolled advice line reads: the stellars, the events under way,
/// the `öops` records, every `spöb`'s `Flags`, the words and the names.
pub(super) struct AdviceFacts<'a> {
    /// The stellars, as [`Chart::advice_ports`] gives them.
    pub(super) ports: &'a [AdvicePort],
    /// The pilot's events under way.
    pub(super) events: &'a BTreeMap<DisasterId, ActiveEvent>,
    /// Every `öops` record.
    pub(super) disasters: &'a [DisasterRecord],
    /// Every `spöb`'s `Flags`, by ID.
    pub(super) stellar_flags: &'a BTreeMap<StellarId, u32>,
    /// The words the lines are built from.
    pub(super) words: &'a AdviceWords,
    /// The stellars' and cargo's names.
    pub(super) names: &'a dyn TagNames,
}

/// The good-price or disaster line a hail of advice `kind` rolls as it
/// opens, drawn on `chance` and read from `facts`: none for any other
/// kind, or when there is nothing to tell (see the module docs). The
/// check is the press's.
pub(super) fn rolled_line(
    kind: Option<u16>,
    facts: &AdviceFacts,
    chance: &mut dyn Chance,
) -> Option<String> {
    let names = facts.names;
    match kind? {
        GOOD_PRICES => {
            let tip = advice::good_price(facts.ports, chance)?;
            Some(good_price_line(
                &names.stellar_name(tip.stellar).unwrap_or_default(),
                tip.level,
                &names.cargo_name(tip.commodity).unwrap_or_default(),
                facts.words,
            ))
        }
        DISASTERS => {
            let id = advice::disaster(facts.events, chance)?;
            let active = *facts.events.get(&id)?;
            let record = facts.disasters.iter().find(|record| record.id == id)?;
            let stellar = market::place(active, record)?;
            let cargo = market::event_commodity(record.commodity)
                .and_then(|cargo| names.cargo_name(cargo))
                .unwrap_or_default();
            let station = facts
                .stellar_flags
                .get(&stellar)
                .is_some_and(|flags| flags & StellarFlags::STATION != 0);
            Some(disaster_line(
                &names.stellar_name(stellar).unwrap_or_default(),
                station,
                &cargo,
                advice::degree(chance),
                record.price_delta < 0,
                facts.words,
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use crate::catalog::{
        CommCatalog, DisasterId, DisasterRecord, GovtId, PersonId, StellarId, StringPatch,
    };
    use crate::hail::{HailOptions, HailRefusal, HailView};
    use crate::landing::StellarFlags;
    use crate::market::ActiveEvent;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::hail::tests::{hail_with, hailable, npc, opening, targeting};
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::npc::{AiType, NpcPerson};

    const FED: GovtId = GovtId(128);

    /// A low price's nibble for food (commodity 0).
    const FOOD_LOW: u32 = 0x1 << 28;
    /// A high price's nibble for metal (commodity 4).
    const METAL_HIGH: u32 = 0x4 << 12;

    /// [`hailable`] with the advice's words in `STR#` 2002, the cargo
    /// names in 4001, stellar 128 "Earth" trading food low (and 129 and
    /// 140 trading nothing), and "Nice to meet you." 42 characters long
    /// nowhere.
    fn advising() -> FakePilotCatalog {
        let mut catalog = hailable();
        let messages = &mut catalog.strings[1].1;
        for (n, word) in [
            (60, "on"),
            (176, "is a good place to"),
            (177, "buy"),
            (178, "sell"),
            (179, "The last time I was"),
            (180, "at"),
            (181, "the price of"),
            (182, "was very"),
            (183, "was really"),
            (184, "was pretty"),
            (185, "low"),
            (186, "high"),
        ] {
            messages[n - 1] = word.to_owned();
        }
        catalog.strings.push((
            4001,
            [
                "food",
                "industrial",
                "medical",
                "luxury",
                "metal",
                "equipment",
            ]
            .map(str::to_owned)
            .to_vec(),
        ));
        catalog
            .strings
            .push((7100, (1..=9).map(|n| format!("q{n}")).collect()));
        catalog.sites[0].1[0].flags =
            StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | FOOD_LOW;
        for (id, name) in [(128, "Earth"), (129, "Luna"), (140, "Port")] {
            catalog
                .place_names
                .stellars
                .insert(StellarId(id), name.to_owned());
        }
        catalog
    }

    /// The string lists of a catalog, owned.
    struct Lists(Vec<(i16, Vec<String>)>);

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.0
                .iter()
                .find(|(list, _)| *list == id)
                .map_or_else(Vec::new, |(_, strings)| strings.clone())
        }
    }

    /// `catalog`'s session as [`targeting`] makes it, its string lists
    /// `catalog`'s, its ship giving advice `info_types`.
    fn advised(catalog: &FakePilotCatalog, info_types: u16) -> Session {
        let mut session = targeting(catalog).with_strings(Rc::new(Lists(catalog.strings.clone())));
        npc(&mut session).info_types = info_types;
        session
    }

    /// Hails with `draws` after the conversation's in variant 0, then
    /// presses Greetings twice, drawing nothing: what it said.
    fn greeted(session: &mut Session, catalog: &FakePilotCatalog, draws: &[u32]) -> String {
        let mut all = opening(0);
        all.extend(draws);
        let (hailed, asked) = hail_with(session, catalog, &all);
        hailed.expect("answered");
        assert_eq!(asked.len(), all.len(), "drew {asked:?}");
        let mut chance = Draws::of(&[]);
        let said = press_greetings(session, catalog, &mut chance);
        assert_eq!(
            press_greetings(session, catalog, &mut chance),
            said,
            "the same line again"
        );
        assert!(chance.asked.is_empty(), "nothing drawn at the press");
        said
    }

    fn press_greetings(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        chance: &mut Draws,
    ) -> String {
        session
            .answer(0, catalog, &HailOptions::default(), chance)
            .expect("open")
            .reply
    }

    #[test]
    fn a_friendly_trader_of_good_prices_names_where_to_buy() {
        let catalog = advising();
        let mut session = advised(&catalog, 0x1000);
        assert_eq!(
            greeted(&mut session, &catalog, &[]),
            "Earth is a good place to buy food."
        );
    }

    #[test]
    fn a_good_price_is_picked_among_every_shown_stellar_and_commodity() {
        let mut catalog = advising();
        catalog.sites[0].1[1].flags = StellarFlags::TRADE_CENTER | METAL_HIGH;
        catalog.sites[1].1[0].flags = StellarFlags::CAN_LAND | FOOD_LOW | METAL_HIGH;
        let said = |draw| {
            let mut session = advised(&catalog, 0x1000);
            greeted(&mut session, &catalog, &[draw])
        };
        assert_eq!(said(0), "Earth is a good place to buy food.");
        assert_eq!(said(1), "Luna is a good place to sell metal.");
        assert_eq!(
            said(2),
            "Port is a good place to buy food.",
            "another system's"
        );
        assert_eq!(said(3), "Port is a good place to sell metal.");
    }

    #[test]
    fn a_stellar_not_shown_gives_no_good_price() {
        let mut catalog = advising();
        catalog.sites[0].1[0].flags = StellarFlags::CAN_LAND;
        catalog.sites[1].1[0].flags = StellarFlags::CAN_LAND | FOOD_LOW;
        catalog.systems.retain(|&system| system.0 != 131);
        let mut session = advised(&catalog, 0x1000);
        assert_eq!(greeted(&mut session, &catalog, &[]), "Greetings.");
    }

    #[test]
    fn with_no_good_price_anywhere_it_says_greetings() {
        let mut catalog = advising();
        catalog.sites[0].1[0].flags = StellarFlags::CAN_LAND | (0x2 << 28);
        let mut session = advised(&catalog, 0x1000);
        assert_eq!(greeted(&mut session, &catalog, &[]), "Greetings.");
    }

    #[test]
    fn a_good_price_at_a_starred_stellar_is_nice_to_meet_you() {
        let mut catalog = advising();
        catalog
            .place_names
            .stellars
            .insert(StellarId(128), "*Earth".to_owned());
        let mut session = advised(&catalog, 0x1000);
        assert_eq!(greeted(&mut session, &catalog, &[]), "Nice to meet you.");
    }

    #[test]
    fn a_good_price_line_of_42_characters_follows_long_advice() {
        let mut catalog = advising();
        // "<name> is a good place to buy food." is 42 long.
        let name = "x".repeat(42 - " is a good place to buy food.".len());
        catalog
            .place_names
            .stellars
            .insert(StellarId(128), name.clone());
        let mut session = advised(&catalog, 0x1000);
        assert_eq!(greeted(&mut session, &catalog, &[]), "Nice to meet you.");
        let options = HailOptions::nova(
            &Rulebook::default().with_override(RuleKey::LongAdvice, RuleSource::Bible),
        );
        let mut session = advised(&catalog, 0x1000);
        let mut chance = Draws::of(&opening(0));
        session
            .hail(&catalog, &options, &mut chance)
            .expect("answered");
        let view = session
            .answer(0, &catalog, &options, &mut Draws::of(&[]))
            .expect("open");
        assert_eq!(view.reply, format!("{name} is a good place to buy food."));
    }

    /// `catalog` with the food surplus (`öops` 128, at 128 "Earth", -15),
    /// Earth a planet.
    fn surplus(catalog: &mut FakePilotCatalog) {
        catalog.disasters = vec![DisasterRecord {
            id: DisasterId(128),
            name: "An enormous food surplus".to_owned(),
            stellar: 128,
            commodity: 0,
            price_delta: -15,
            duration: 10,
            freq: 0,
            ..DisasterRecord::default()
        }];
        catalog.stellars = vec![(
            StellarId(128),
            StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | FOOD_LOW,
        )];
    }

    fn under_way(session: &mut Session, days: u16, stellar: Option<i16>) {
        session.pilot.events = BTreeMap::from([(
            DisasterId(128),
            ActiveEvent {
                days,
                stellar: stellar.map(StellarId),
            },
        )]);
    }

    #[test]
    fn a_friendly_ship_of_disasters_tells_of_one_under_way() {
        let mut catalog = advising();
        surplus(&mut catalog);
        let mut session = advised(&catalog, 0x2000);
        under_way(&mut session, 3, Some(128));
        assert_eq!(
            greeted(&mut session, &catalog, &[1]),
            "The last time I was on Earth, the price of food was really low."
        );
    }

    #[test]
    fn a_disaster_at_a_station_says_at_and_a_rise_high() {
        let mut catalog = advising();
        surplus(&mut catalog);
        catalog.disasters[0].price_delta = 0;
        catalog.disasters[0].commodity = 4;
        catalog.stellars = vec![(StellarId(140), StellarFlags::STATION)];
        let mut session = advised(&catalog, 0x2000);
        under_way(&mut session, 2, Some(140));
        assert_eq!(
            greeted(&mut session, &catalog, &[2]),
            "The last time I was at Port, the price of metal was pretty high."
        );
    }

    #[test]
    fn a_disaster_with_a_day_left_or_unplaced_is_no_advice() {
        let mut catalog = advising();
        surplus(&mut catalog);
        let mut session = advised(&catalog, 0x2000);
        under_way(&mut session, 1, Some(128));
        assert_eq!(greeted(&mut session, &catalog, &[]), "Greetings.");
        catalog.disasters[0].stellar = -2;
        let mut session = advised(&catalog, 0x2000);
        under_way(&mut session, 5, None);
        assert_eq!(
            greeted(&mut session, &catalog, &[]),
            "Greetings.",
            "news only"
        );
        let mut session = advised(&catalog, 0x2000);
        assert_eq!(
            greeted(&mut session, &catalog, &[]),
            "Greetings.",
            "none at all"
        );
    }

    #[test]
    fn a_disaster_of_a_commodity_the_loader_dropped_names_no_cargo() {
        let mut catalog = advising();
        surplus(&mut catalog);
        catalog.disasters[0].commodity = -1;
        let mut session = advised(&catalog, 0x2000);
        under_way(&mut session, 4, Some(128));
        assert_eq!(
            greeted(&mut session, &catalog, &[0]),
            "The last time I was on Earth, the price of  was very low."
        );
    }

    #[test]
    fn a_hostile_ship_of_good_prices_says_stop_wasting_my_time() {
        let catalog = advising();
        let mut session = advised(&catalog, 0x1000);
        crate::session::tests::with_record(&mut session, FED, -7);
        assert_eq!(
            greeted(&mut session, &catalog, &[]),
            "Stop wasting my time."
        );
    }

    /// A person of `flags` quoting `comm_quote`, flown by NPC 0, which has
    /// no `düde`.
    fn as_person(session: &mut Session, flags: u16, comm_quote: i16) {
        let ship = npc(session);
        ship.info_types = 0;
        ship.ai_type = AiType::WimpyTrader;
        ship.person = Some(NpcPerson {
            id: PersonId(600),
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
        });
    }

    #[test]
    fn a_person_showing_disasters_and_quoting_nothing_tells_of_one() {
        let mut catalog = advising();
        surplus(&mut catalog);
        let mut session = advised(&catalog, 0);
        as_person(&mut session, 0x8000, -1);
        under_way(&mut session, 3, Some(128));
        assert_eq!(
            greeted(&mut session, &catalog, &[0]),
            "The last time I was on Earth, the price of food was very low."
        );
        let mut session = advised(&catalog, 0);
        as_person(&mut session, 0x8000, 0);
        under_way(&mut session, 3, Some(128));
        assert_eq!(
            greeted(&mut session, &catalog, &[]),
            "Greetings.",
            "quote 0"
        );
        let mut session = advised(&catalog, 0);
        as_person(&mut session, 0x0010, -1);
        under_way(&mut session, 3, Some(128));
        assert_eq!(
            greeted(&mut session, &catalog, &[]),
            "Greetings.",
            "no 0x8000"
        );
    }

    #[test]
    fn a_persons_comm_quote_reads_its_plug_in_string_first() {
        let mut catalog = advising();
        let mut session = advised(&catalog, 0);
        as_person(&mut session, 0x8000, 5);
        assert_eq!(greeted(&mut session, &catalog, &[]), "q5", "STR# 7100 #5");
        catalog
            .string_patches
            .push((15_005, StringPatch::Text("*Patched quote.".to_owned())));
        let mut session = advised(&catalog, 0);
        as_person(&mut session, 0x8000, 5);
        assert_eq!(
            greeted(&mut session, &catalog, &[]),
            "*Patched quote.",
            "as written"
        );
        catalog.string_patches = vec![(15_005, StringPatch::Empty)];
        let mut session = advised(&catalog, 0);
        as_person(&mut session, 0, 5);
        assert_eq!(greeted(&mut session, &catalog, &[]), "Greetings.");
    }

    /// The opening line of a person quoting 5 by the other reading of
    /// [`RuleKey::CommQuote`], `'STR '` 15005 `patch`.
    fn opened_by_quote(patch: StringPatch) -> Result<HailView, HailRefusal> {
        let mut catalog = advising();
        catalog.string_patches = vec![(15_005, patch)];
        let rules = Rulebook::default().with_override(RuleKey::CommQuote, RuleSource::Bible);
        let mut session = advised(&catalog, 0).with_rules(rules);
        as_person(&mut session, 0, 5);
        hail_with(&mut session, &catalog, &opening(0)).0
    }

    #[test]
    fn by_the_other_reading_the_opening_quote_reads_its_plug_in_string_and_no_greetings() {
        let reply = |patch| opened_by_quote(patch).expect("answered").reply;
        assert_eq!(reply(StringPatch::Absent), "q5");
        assert_eq!(reply(StringPatch::Text("Ahoy.".to_owned())), "Ahoy.");
        assert_eq!(reply(StringPatch::Empty), "", "no buffer kept");
        assert_eq!(reply(StringPatch::Unreadable), "");
    }

    #[test]
    fn a_governments_hail_reads_its_plug_in_string_first() {
        let mut catalog = advising();
        // The Federation's warship, variant 1: `'STR '` 10015 + 1.
        catalog
            .string_patches
            .push((10_016, StringPatch::Text("Patched hail.".to_owned())));
        let mut session = advised(&catalog, 0x8000);
        let (hailed, _) = hail_with(&mut session, &catalog, &opening(1));
        hailed.expect("answered");
        let said = press_greetings(&mut session, &catalog, &mut Draws::of(&[]));
        assert_eq!(said, "Patched hail.");
        catalog.string_patches = vec![(10_016, StringPatch::Unreadable)];
        let mut session = advised(&catalog, 0x8000);
        hail_with(&mut session, &catalog, &opening(1))
            .0
            .expect("answered");
        assert_eq!(
            press_greetings(&mut session, &catalog, &mut Draws::of(&[])),
            "c47",
            "read empty, then Nice to meet you in variant 1"
        );
    }
}
