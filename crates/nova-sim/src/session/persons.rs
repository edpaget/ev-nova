//! Persons in flight: the session's side of [`person`](crate::person).
//!
//! **Appearing.** A system is populated with its persons as the session's
//! [`PersonRules`] say ([`Session::with_person_rules`], Nova's by
//! default), the pilot's persons gone for good kept away and each
//! person's `ActiveOn` tested through the session's
//! [`ControlBits`](crate::hire::ControlBits). A person NPC is named by
//! its record, with its subtitle ([`Session::npc_name`],
//! [`Session::npc_subtitle`]).
//!
//! **In a fight.** A person whose `Flags` has 0x0001 holds a grudge
//! against the player once a shot of the player's hits it, kept on the
//! pilot, so every later appearance starts with it. A person destroyed
//! without an escape pod (`Flags` 0x0002), self-destructed or shot
//! down, is gone for good, and so is one captured, escape pod or not;
//! its grudge goes with it. One that lands or jumps out stays alive.
//! Neither a grudge nor a death makes a save due: the next save keeps
//! them. An invincible person (`ShieldMod` below 0) ends each fight tick
//! with its shield and armour full, and is never disabled nor broken up
//! (the original refills them every frame; its order against a shot's
//! damage is not traced).

use std::rc::Rc;

use super::{Session, hire};
use crate::catalog::PersonId;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::person::{ESCAPE_POD, GRUDGE, PersonRules};
use crate::traffic::npc::{Npc, NpcId};

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

    /// Person `id` is gone for good: it never appears again, and its
    /// grudge goes with it (`_LoadPilotData` @0x75e1e-0x75e2f). No save is
    /// due: the next one keeps it.
    pub(super) fn go(&mut self, id: PersonId) {
        self.pilot.gone_persons.insert(id);
        self.pilot.grudges.remove(&id);
    }

    /// NPC `id`, destroyed, is taken out of the system: a person without
    /// an escape pod is gone for good (`_HandleShipDisplay`
    /// @0x2dc4a-0x2dca1).
    pub(super) fn lose_destroyed(&mut self, id: NpcId) {
        let person = self
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .and_then(|npc| npc.person);
        if let Some(person) = person.filter(|person| person.flags & ESCAPE_POD == 0) {
            self.go(person.id);
        }
        self.traffic.remove(id);
    }

    /// NPC `id`, captured, no longer flies as its person, who is gone for
    /// good, escape pod or not (`_DoShipCapture` @0x41180).
    pub(super) fn lose_captured(&mut self, id: NpcId) {
        let npc = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
        if let Some(person) = npc.and_then(|npc| npc.person.take()) {
            self.go(person.id);
        }
    }

    /// Each person of `Flags` 0x0001 that a shot of the player's hit among
    /// `strikes` holds a grudge against it from now on, on the pilot too
    /// (`_DamageShip` @0x3be83-0x3beb3); an escort's shot does not count.
    /// No save is due.
    pub(super) fn hold_grudges(&mut self, strikes: &[Strike]) {
        for strike in strikes.iter().filter(|strike| strike.by == ShipRef::Player) {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let npc = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
            let Some(person) = npc.and_then(|npc| npc.person.as_mut()) else {
                continue;
            };
            if person.flags & GRUDGE != 0 {
                person.grudge = true;
                self.pilot.grudges.insert(person.id);
            }
        }
    }

    /// Refills each invincible person's shield and armour at the end of
    /// the fight's tick, after its shots landed, and keeps it flying: no
    /// hit among `strikes` disabled or broke it up (`_HandleShip`
    /// @0x33c0d-0x33c38 refills every frame).
    pub(super) fn refill_invincible(&mut self, strikes: &mut [Strike]) {
        for npc in self.traffic.npcs_mut() {
            if !npc.person.is_some_and(|person| person.invincible) {
                continue;
            }
            npc.reserves.shield.now = npc.reserves.shield.max;
            npc.reserves.armor.now = npc.reserves.armor.max;
            npc.condition = Condition::Intact;
            let me = ShipRef::Npc(npc.id);
            for strike in strikes.iter_mut().filter(|strike| strike.ship == me) {
                strike.downed = None;
            }
        }
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

    // Fighting persons.

    use crate::catalog::{HullRecord, StockWeapon, WeaponId, WeaponRecord};
    use crate::combat::armament::Trigger;
    use crate::combat::hull::Condition;
    use crate::combat::{Rules, ShipRef, Strike};
    use crate::geometry::Vec2;
    use crate::person::{ESCAPE_POD, GRUDGE};
    use crate::testkit::{Draws, hull, weapon};
    use crate::traffic::npc::NpcId;

    /// A blaster firing every other tick, 20 pixels a tick for 30 ticks,
    /// doing 5 mass and 10 energy damage.
    fn blaster() -> WeaponRecord {
        WeaponRecord {
            reload: 2,
            count: 30,
            speed: 2000,
            mass_dmg: 5,
            energy_dmg: 10,
            ..weapon(128)
        }
    }

    /// Ship `id` armed with a blaster, breaking up for 2 ticks.
    fn armed_hull(id: i16) -> HullRecord {
        HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(128),
                count: 1,
                ammo: 0,
            }],
            death_delay: 2,
            size: Some(30),
            ..hull(id)
        }
    }

    /// [`peopled`] with no setup passes, the player and Ace armed with a
    /// blaster, and Ace of `flags` and `ShieldMod` `shield_mod`.
    fn duel(flags: u16, shield_mod: i16) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            weapons: vec![blaster()],
            hulls: vec![armed_hull(128), armed_hull(129)],
            ..peopled(0)
        };
        catalog.persons[0].flags = flags;
        catalog.persons[0].shield_mod = shield_mod;
        catalog
    }

    /// The draws placing Ace 100 above the player, facing it.
    const ACE_ABOVE: [u32; 4] = [99, 750, 650, 180];

    /// `pilot`'s session over `catalog` with Ace placed 100 above the
    /// player, who holds its fire on it.
    fn facing_ace(catalog: &FakePilotCatalog, pilot: Pilot) -> Session {
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut Draws::of(&ACE_ABOVE));
        assert_eq!(persons(&session), [Some(600)]);
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        session.take_save_due();
        session
    }

    /// Fires on Ace until it is out of the system, at most 300 ticks.
    fn shoot_down(session: &mut Session) {
        for _ in 0..300 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            if session.npcs().is_empty() {
                return;
            }
        }
        panic!("never shot down: {:?}", session.npcs());
    }

    /// `session`'s pilot, saved and loaded again, flown once more with the
    /// same draws.
    fn flown_again(catalog: &FakePilotCatalog, session: &Session) -> Session {
        let pilot = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut Draws::of(&ACE_ABOVE));
        session
    }

    fn ada(catalog: &FakePilotCatalog) -> Pilot {
        Pilot::new(catalog, "Ada").expect("a pilot")
    }

    #[test]
    fn a_unique_person_shot_down_is_gone_for_good_through_a_save() {
        let catalog = duel(0, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        shoot_down(&mut session);
        assert!(session.pilot().gone(PersonId(600)));
        assert!(!session.take_save_due(), "no save is due");
        assert_eq!(persons(&flown_again(&catalog, &session)), []);
    }

    #[test]
    fn a_person_with_an_escape_pod_shot_down_comes_again() {
        let catalog = duel(ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        shoot_down(&mut session);
        assert!(!session.pilot().gone(PersonId(600)));
        assert_eq!(persons(&flown_again(&catalog, &session)), [Some(600)]);
    }

    #[test]
    fn a_self_destructed_person_follows_the_escape_pod_rule() {
        for (flags, gone) in [(0, true), (ESCAPE_POD, false)] {
            let catalog = duel(flags, 0);
            let mut session = facing_ace(&catalog, ada(&catalog));
            session.hold_trigger(Trigger::default());
            session.self_destruct(NpcId(0));
            shoot_down(&mut session);
            assert_eq!(session.pilot().gone(PersonId(600)), gone, "{flags}");
        }
    }

    /// Ace captured, as phase 6's boarding leaves it awaiting its
    /// assignment.
    fn captured(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_ace(catalog, ada(catalog));
        session.aboard = Some(super::super::Aboard {
            npc: NpcId(0),
            ship: ShipId(129),
            plunder: crate::board::Plunder::default(),
            captured: true,
        });
        session
    }

    #[test]
    fn a_captured_person_is_gone_for_good_escape_pod_or_not() {
        use crate::board::Assignment;
        for (flags, choice) in [
            (ESCAPE_POD, Assignment::Escort),
            (0, Assignment::Escort),
            (ESCAPE_POD, Assignment::MyShip),
        ] {
            let catalog = duel(flags, 0);
            let mut session = captured(&catalog);
            assert!(session.assign(choice, &mut NeverFires).is_some());
            assert!(session.pilot().gone(PersonId(600)), "{flags} {choice:?}");
            assert!(
                session.npcs().iter().all(|npc| npc.person.is_none()),
                "no person flies for the player: {choice:?}"
            );
        }
    }

    #[test]
    fn the_players_hit_on_a_person_of_0x0001_holds_its_grudge() {
        let catalog = duel(GRUDGE | ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert!(session.npcs()[0].reserves.shield.now < 30.0, "hit");
        assert!(session.npcs()[0].person.expect("Ace").grudge);
        assert!(session.pilot().grudge(PersonId(600)));
        assert!(!session.take_save_due(), "no save is due");
        let catalog = duel(ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert!(!session.npcs()[0].person.expect("Ace").grudge, "no 0x0001");
        assert!(!session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn an_escorts_hit_holds_no_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let hit = |by| Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by,
            damage: 1.0,
            downed: None,
        };
        session.hold_grudges(&[hit(ShipRef::Npc(NpcId(7)))]);
        assert!(!session.pilot().grudge(PersonId(600)));
        session.hold_grudges(&[hit(ShipRef::Player)]);
        assert!(session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn a_person_gone_for_good_loses_its_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut pilot = ada(&catalog);
        pilot.grudges.insert(PersonId(600));
        let mut session = facing_ace(&catalog, pilot);
        assert!(
            session.npcs()[0].person.expect("Ace").grudge,
            "it enters with it"
        );
        shoot_down(&mut session);
        assert!(session.pilot().gone(PersonId(600)));
        assert!(!session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn an_invincible_person_ends_each_fight_tick_whole_and_is_never_disabled() {
        let catalog = duel(0, -1);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let full = session.npcs()[0].reserves;
        for _ in 0..300 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let ace = &session.npcs()[0];
            assert_eq!(ace.reserves, full);
            assert_eq!(ace.condition, Condition::Intact);
        }
        let hits = session
            .strikes
            .iter()
            .filter(|strike| strike.ship == ShipRef::Npc(NpcId(0)) && strike.damage > 0.0)
            .count();
        assert!(hits > 100, "the blaster hit it: {hits}");
        assert!(
            session.strikes.iter().all(|strike| strike.downed.is_none()),
            "never downed"
        );
        assert!(!session.pilot().gone(PersonId(600)));
    }
}
