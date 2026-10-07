//! Persons in flight: the session's side of [`person`](crate::person).
//!
//! **Appearing.** A system is populated with its persons as the session's
//! [`PersonRules`] say ([`Session::with_person_rules`], Nova's by
//! default), the pilot's persons gone for good kept away and each
//! person's `ActiveOn` tested through the session's
//! [`ControlBits`](crate::hire::ControlBits). A person NPC is named by
//! its record, with its subtitle ([`Session::npc_name`],
//! [`Session::npc_subtitle`]).

use std::rc::Rc;

use super::{Session, hire};
use crate::person::PersonRules;
use crate::traffic::npc::Npc;

impl Session {
    /// This session with `rules` deciding how persons appear (see
    /// [`person`](crate::person)): Nova's
    /// ([`NovaPersons`](crate::NovaPersons)) by default.
    #[must_use]
    pub fn with_person_rules(mut self, rules: Rc<dyn PersonRules>) -> Self {
        self.person_rules = hire::Shared(rules);
        self
    }

    /// `npc`'s name: its person's, or its ship type's, if the session
    /// has either.
    #[must_use]
    pub fn npc_name(&self, npc: &Npc) -> Option<&str> {
        match npc.person.and_then(|person| self.traffic.person(person.id)) {
            Some(person) => Some(&person.record.name),
            None => self.ship_name(npc.ship),
        }
    }

    /// `npc`'s person's subtitle, unless it has none or it is empty.
    #[must_use]
    pub fn npc_subtitle(&self, npc: &Npc) -> Option<&str> {
        let person = self.traffic.person(npc.person?.id)?;
        Some(person.record.subtitle.as_str()).filter(|subtitle| !subtitle.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{PersonId, PersonRecord, ShipId, SystemId, SystemTraffic};
    use crate::chance::{Chance, NeverFires};
    use crate::hire::ControlBits;
    use crate::person::{PersonRoll, PersonRules};
    use crate::pilot::Pilot;
    use crate::rulebook::RuleSource;
    use crate::testkit::{FAST, FakePilotCatalog, person, ship};

    /// System 130 has no traffic of its own (`AvgShips` `avg_ships`) and
    /// one Person slot, naming "Ace" (`përs` 600, subtitle "Top Gun",
    /// `ActiveOn` "b3", flying ship 129) at 100 %. Person 601, linked
    /// anywhere, flies ship 129 too.
    fn peopled(avg_ships: i16) -> FakePilotCatalog {
        let mut slots: [(Option<PersonId>, i16); 8] = Default::default();
        slots[0] = (Some(PersonId(600)), 100);
        FakePilotCatalog {
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types: [(-1, 0); 8],
                    avg_ships,
                    persons: slots,
                },
            )],
            persons: vec![
                PersonRecord {
                    name: "Ace".to_owned(),
                    subtitle: "Top Gun".to_owned(),
                    active_on: "b3".to_owned(),
                    link_syst: 131,
                    ..person(600, 129)
                },
                person(601, 129),
            ],
            ship_records: vec![ship(129, FAST)],
            ..crate::testkit::catalog()
        }
    }

    /// `pilot`'s session over `catalog`, populated as the draws' last
    /// outcomes say.
    fn populated(catalog: &FakePilotCatalog, session: Session) -> Session {
        let mut session = session;
        session.populate(catalog, &mut NeverFires);
        session
    }

    fn persons(session: &Session) -> Vec<Option<i16>> {
        session
            .npcs()
            .iter()
            .map(|npc| npc.person.map(|person| person.id.0))
            .collect()
    }

    #[test]
    fn populating_brings_the_person_a_slot_names() {
        let catalog = peopled(0);
        let session = populated(&catalog, Session::start(&catalog).expect("starts"));
        assert_eq!(persons(&session), [Some(600)]);
    }

    #[test]
    fn a_person_gone_for_good_never_appears() {
        let catalog = peopled(0);
        let mut pilot = Pilot::new(&catalog, "Ada").expect("a pilot");
        pilot.gone_persons.insert(PersonId(600));
        let session = populated(&catalog, Session::fly(&catalog, pilot).expect("flies"));
        assert_eq!(persons(&session), []);
    }

    /// Lets no expression hold.
    #[derive(Debug)]
    struct NoneHold;

    impl ControlBits for NoneHold {
        fn allows(&self, _expression: &str) -> bool {
            false
        }
    }

    #[test]
    fn a_person_whose_active_on_the_control_bits_refuse_never_appears() {
        let catalog = peopled(0);
        let session = Session::start(&catalog)
            .expect("starts")
            .with_control_bits(Rc::new(NoneHold));
        assert_eq!(persons(&populated(&catalog, session)), []);
    }

    /// Brings the first person who may appear, and no slot's.
    #[derive(Debug)]
    struct FirstComes;

    impl PersonRules for FirstComes {
        fn roll(&self, eligible: &[PersonId], _chance: &mut dyn Chance) -> PersonRoll {
            eligible
                .first()
                .map_or(PersonRoll::Empty, |&id| PersonRoll::Person(id))
        }

        fn listed(&self, _prob: i16, _chance: &mut dyn Chance) -> bool {
            false
        }

        fn link_slip(&self) -> RuleSource {
            RuleSource::Engine
        }

        fn shield_mod(&self) -> RuleSource {
            RuleSource::Engine
        }
    }

    #[test]
    fn the_sessions_person_rules_decide_who_comes() {
        let catalog = peopled(1);
        let session = Session::start(&catalog)
            .expect("starts")
            .with_person_rules(Rc::new(FirstComes));
        assert_eq!(persons(&populated(&catalog, session)), [Some(601)]);
    }

    /// [`peopled`] with a düde ship (ship 129) in each setup pass, and
    /// person 601 nowhere; Ace's subtitle `subtitle`.
    fn mixed(subtitle: &str) -> FakePilotCatalog {
        let mut catalog = peopled(1);
        catalog.persons[0].subtitle = subtitle.to_owned();
        catalog.persons[1].link_syst = -2;
        catalog.traffic[0].1.dude_types[0] = (128, 100);
        catalog.dudes = vec![(
            crate::catalog::DudeId(128),
            crate::catalog::DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        )];
        catalog
    }

    #[test]
    fn a_person_is_named_and_subtitled_as_its_record_and_a_ship_as_its_type() {
        let catalog = mixed("Top Gun");
        let session = populated(&catalog, Session::start(&catalog).expect("starts"));
        assert_eq!(persons(&session), [None, Some(600)]);
        let [ship, ace] = session.npcs() else {
            panic!("two NPCs: {:?}", session.npcs());
        };
        assert_eq!(session.npc_name(ace), Some("Ace"));
        assert_eq!(session.npc_subtitle(ace), Some("Top Gun"));
        assert_eq!(session.npc_name(ship), Some("Ship 129"));
        assert_eq!(session.npc_subtitle(ship), None);
        let catalog = mixed("");
        let session = populated(&catalog, Session::start(&catalog).expect("starts"));
        let ace = &session.npcs()[1];
        assert_eq!(session.npc_subtitle(ace), None, "an empty subtitle is none");
    }
}
