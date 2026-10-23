//! The exchange's event line: the session's side of
//! [`exchange_events`](crate::exchange_events).
//!
//! [`Session::market`] puts what [`exchange_events`] gives for the
//! stellar docked at in `Market.events`: by the engine, the first local
//! `öops` event's line, told from `STR#` 2002's pieces ([`StoryWords`])
//! and the session's cargo names; by the other reading, every local
//! event's name. The state layer's `market` leaves the field empty, since
//! it cannot read the strings.

use crate::catalog::StellarId;
use crate::exchange_events::{ExchangeEventFacts, exchange_events};
use crate::news::StoryWords;

use super::Session;

impl Session {
    /// What the exchange at `docked` shows of the price events there (see
    /// the module docs).
    pub(super) fn exchange_events(&self, docked: StellarId) -> Vec<String> {
        let words = StoryWords::read(&*self.setup.strings.0);
        exchange_events().of(&ExchangeEventFacts {
            events: &self.pilot.events,
            disasters: &self.statics.goods.disasters,
            docked,
            words: &words,
            names: self,
            rules: &self.setup.rules,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use crate::catalog::{CommCatalog, DisasterId, DisasterRecord, StellarId};
    use crate::chance::NeverFires;
    use crate::market::ActiveEvent;
    use crate::news::{CARGO_NAMES, MISC_STRINGS};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::testkit::{jump_with, land_now};

    /// `STR#` 2002, with the line's #181 and #192-#194, and the cargo
    /// names.
    struct Lists;

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            let mut misc = vec!["-".to_owned(); 194];
            misc[180] = "the price of".to_owned();
            misc[191] = "has".to_owned();
            misc[192] = "raised".to_owned();
            misc[193] = "lowered".to_owned();
            match id {
                MISC_STRINGS => misc,
                CARGO_NAMES => vec!["food".to_owned(), "industrial".to_owned()],
                _ => Vec::new(),
            }
        }
    }

    /// Landed at the trade center 140, with the food surplus (128, -15)
    /// and a drought (130, +15) both under way there, under `rules`.
    fn docked(rules: Rulebook) -> Session {
        let mut catalog = crate::session::tests::surplus();
        catalog.disasters.push(DisasterRecord {
            id: DisasterId(130),
            name: "A minor drought".to_owned(),
            price_delta: 15,
            ..catalog.disasters[0].clone()
        });
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists))
            .with_rules(rules);
        jump_with(&mut session, &catalog, 131, &mut NeverFires);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        let here = Some(StellarId(140));
        session.pilot.events = BTreeMap::from([
            (
                DisasterId(130),
                ActiveEvent {
                    days: 7,
                    stellar: here,
                },
            ),
            (
                DisasterId(128),
                ActiveEvent {
                    days: 1,
                    stellar: here,
                },
            ),
        ]);
        session
    }

    #[test]
    fn the_market_shows_the_first_events_line_by_default() {
        let session = docked(Rulebook::default());
        let market = session.market().expect("an exchange");
        assert_eq!(
            market.events,
            ["An enormous food surplus has lowered the price of food."]
        );
    }

    #[test]
    fn the_market_names_every_event_by_the_bible() {
        let rules = Rulebook::default().with_override(RuleKey::ExchangeEvents, RuleSource::Bible);
        let market = docked(rules).market().expect("an exchange");
        assert_eq!(
            market.events,
            ["An enormous food surplus", "A minor drought"]
        );
    }

    #[test]
    fn the_events_are_those_of_the_stellar_docked_at() {
        let mut session = docked(Rulebook::default());
        for active in session.pilot.events.values_mut() {
            active.stellar = Some(StellarId(139));
        }
        assert_eq!(
            session.exchange_events(StellarId(140)),
            Vec::<String>::new()
        );
        assert_eq!(
            session.exchange_events(StellarId(139)),
            ["An enormous food surplus has lowered the price of food."]
        );
    }
}
