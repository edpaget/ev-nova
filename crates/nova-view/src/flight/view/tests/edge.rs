//! The edge's own real checks (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge):
//! a flight built by [`open_flight`] hands its session each part of the
//! setup, after the view's opening read, and the run's history comes back
//! through the adapter's one read (`SessionFlight::carry_over`). The
//! setup's nova-sim side is `nova_sim::Session::set_up`'s tests; the
//! adapter's forward is `sim_adapter`'s.

use super::*;
use nova_sim::{CarryOver, HireTerms};

/// A new pilot of `catalog` flown, set up at the edge with `setup`.
fn set_up(catalog: FakeCatalog, setup: SessionSetup) -> View {
    open_flight(catalog, FlightStart::New, setup, Collaborators::default())
}

#[test]
fn every_rule_reaches_the_session_through_the_edge() {
    let every = RuleKey::ALL
        .into_iter()
        .fold(Rulebook::default(), |book, key| {
            book.with_override(key, RuleSource::Bible)
        });
    assert_eq!(
        flight().session().map(Session::rules),
        Ok(&Rulebook::default())
    );
    let view = ruled(catalog(), every);
    let rules = view.session().expect("flying").rules();
    assert_eq!(rules, &every);
    for key in RuleKey::ALL {
        assert_eq!(rules.source_for(key), RuleSource::Bible, "{key:?}");
    }
}

#[test]
fn the_edges_rulebook_is_the_views_wording_too() {
    let stolen = SimMessage::CargoStolen {
        tons: 2,
        credits: 1_234_567,
    };
    let mut view = flight();
    let said = view.word(vec![stolen.clone()]);
    assert_eq!(said[0].0, "2 tons of cargo and 1.23M credits stolen!");
    let bible = Rulebook::default().with_override(RuleKey::PayMillions, RuleSource::Bible);
    let mut view = ruled(catalog(), bible);
    let said = view.word(vec![stolen]);
    assert_eq!(said[0].0, "2 tons of cargo and 1,234,567 credits stolen!");
}

#[test]
fn the_disable_rule_reaches_the_session_and_the_fight() {
    let rule = Rc::new(Counting::default());
    let view = set_up(
        catalog(),
        SessionSetup {
            disable_rule: rule.clone(),
            ..SessionSetup::default()
        },
    );
    let session = view.session().expect("flying");
    assert!(
        !session
            .disable_rule()
            .disabled(nova_sim::Gauge::full(10.0), &session.hull())
    );
    assert_eq!(rule.asked.get(), 1, "the session asks the rule given");
    assert!(Rc::ptr_eq(
        &view.sim().disable_rule,
        &(rule as Rc<dyn DisableRule>)
    ));
}

#[test]
fn the_string_lists_reach_the_session() {
    let strings = FakeCatalog {
        strings: vec![(25040, vec!["Kestrel".to_owned()])],
        ..catalog()
    };
    let view = set_up(
        catalog(),
        SessionSetup {
            strings: Some(Rc::new(strings)),
            ..SessionSetup::default()
        },
    );
    assert_eq!(
        view.session().expect("flying").strings().string_list(25040),
        ["Kestrel"]
    );
}

#[test]
fn the_persons_rules_reach_the_session() {
    let rules = nova_sim::NovaPersons {
        odds: RuleSource::Bible,
        ..nova_sim::NovaPersons::default()
    };
    let view = set_up(
        catalog(),
        SessionSetup {
            person_rules: Rc::new(rules),
            ..SessionSetup::default()
        },
    );
    assert_eq!(
        format!("{:?}", view.session().expect("flying").person_rules()),
        format!("{rules:?}")
    );
}

/// Terms of a fee of 7 and a wage of 3 for every ship.
#[derive(Debug)]
struct Sevens;

impl HireTerms for Sevens {
    fn fee(&self, _ship: &ShipRecord, _site: &LandingSite, _mult: f32) -> i64 {
        7
    }

    fn charge(&self, _ship: &ShipRecord, _site: &LandingSite, _mult: f32, _cash: i64) -> i64 {
        7
    }

    fn wage(&self, _ship: &ShipRecord) -> i64 {
        3
    }
}

/// Control bits where nothing holds.
#[derive(Debug)]
struct Nothing;

impl ControlBits for Nothing {
    fn allows(&self, _test: &nova_sim::TestExpr, _pilot: &dyn nova_sim::PilotFacts) -> bool {
        false
    }
}
#[test]
fn the_hire_ports_reach_the_session() {
    let always: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
    let mut view = open_flight(
        hiring(),
        FlightStart::New,
        SessionSetup {
            hire_terms: Rc::new(Sevens),
            control_bits: Rc::new(Nothing),
            ..SessionSetup::default()
        },
        Collaborators {
            chance: SharedChance::new(always),
            ..Collaborators::default()
        },
    );
    land_now(&mut view);
    let list = view.escorts_for_hire().expect("a bar");
    let row = list.row(ShipId(129)).expect("listed: no Flags3 hides it");
    assert_eq!((row.fee, row.wage), (7, 3));
    assert_eq!(row.hire, Err(HireRefusal::NotForHire));
}

/// The run's history as a flight earlier in it left it: a slot's layer
/// and frames, a ghost in the stellar sprite pool, a berth's person, and
/// a refresh that hid Alpha Centauri (131), Sol's neighbour.
fn history() -> CarryOver {
    let mut sprites = nova_sim::ship_layer::SlotSprites::default();
    sprites.layers.put(
        nova_sim::ship_layer::Slot::Berth(2),
        nova_sim::ship_layer::ShipLayer::Led,
    );
    sprites
        .frames
        .clone_frames(nova_sim::ship_layer::Slot::Berth(2), ShipId(140));
    let mut pool = nova_sim::story::sprite_pool::StellarSpritePool::default();
    pool.rebuild((2000..2016).map(nova_sim::StellarId));
    pool.place(nova_sim::StellarId(2015), (-7, 9));
    pool.rebuild([]);
    let mut traffic = nova_sim::Traffic::new();
    traffic.set_berth_person(2, "Ace");
    CarryOver {
        slot_sprites: Some(sprites),
        stellar_sprite_pool: Some(pool),
        kept_berths: Some(traffic.kept_berths()),
        last_refresh: Some(nova_sim::LastRefresh::hiding([SystemId(131)])),
    }
}

#[test]
fn the_runs_history_reaches_the_session_and_comes_back() {
    let carried = history();
    let mut view = set_up(
        catalog(),
        SessionSetup {
            carry: carried.clone(),
            ..SessionSetup::default()
        },
    );
    let back = view.sim().carry_over().expect("flying");
    assert_eq!(back.slot_sprites, carried.slot_sprites);
    let ghost = nova_sim::story::sprite_pool::SlotSprite::Ghost { top_left: (-7, 9) };
    assert_eq!(
        back.stellar_sprite_pool
            .expect("a pool")
            .sprite_of(nova_sim::StellarId(2015), false),
        ghost
    );
    assert_eq!(back.kept_berths, carried.kept_berths);
    view.begin(SessionFlight::begin);
    assert!(
        !view
            .sim()
            .pilot()
            .expect("flying")
            .has_explored(SystemId(131)),
        "the neighbour pass read the refresh carried"
    );
    let failed = set_up(
        FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        },
        SessionSetup {
            carry: history(),
            ..SessionSetup::default()
        },
    );
    assert_eq!(failed.sim().carry_over(), None);
}

/// Phase 12's fold-in: a view built from a session a builder made with
/// its own rulebook is shown by that rulebook too, so the view and the
/// session read one ([`FlightView::from_session`]).
#[test]
fn a_view_from_a_session_words_by_the_rules_the_session_was_built_with() {
    let stolen = SimMessage::CargoStolen {
        tons: 2,
        credits: 1_234_567,
    };
    let bible = Rulebook::default().with_override(RuleKey::PayMillions, RuleSource::Bible);
    let session = SessionBuilder::flying(&catalog()).with_rules(bible).build();
    let mut view = FlightView::from_session(catalog(), session.into(), &bible);
    assert_eq!(view.session().map(Session::rules), Ok(&bible));
    let said = view.word(vec![stolen]);
    assert_eq!(said[0].0, "2 tons of cargo and 1,234,567 credits stolen!");
}

/// One chance by construction: the edge hands the flight and the view's
/// wording the collaborators' one chance, so the wording's picks draw on
/// the flight's own sequence.
#[test]
fn the_edge_hands_the_flight_and_the_wording_one_chance() {
    let chance = rolling_one();
    let view = open_flight(
        catalog(),
        FlightStart::New,
        SessionSetup::default(),
        Collaborators {
            chance: chance.clone(),
            ..Collaborators::default()
        },
    );
    assert!(Rc::ptr_eq(&view.sim().chance.0, &chance.0), "the flight's");
    assert!(Rc::ptr_eq(&view.words_chance.0, &chance.0), "the wording's");
}
