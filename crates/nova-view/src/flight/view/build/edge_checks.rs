//! The edge's own real checks (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge):
//! a flight built by [`open_flight`] hands its session each part of the
//! setup, after the view's opening read, and the run's history comes back
//! through the adapter's one read (`SessionFlight::carry_over`). The
//! setup's nova-sim side is `nova_sim::Session::set_up`'s tests; the
//! adapter's forward is `sim_adapter`'s. They sit beside the build, out of
//! the component tests' tree, so that tree knows only `FakeFlight`
//! (rdm:phase/view-sim-seam/phase-15-integration-test-set).

use std::rc::Rc;

use nova_sim::fixture::SessionBuilder;
use nova_sim::landing::StellarFlags;
use nova_sim::map_borders::Next;
use nova_sim::{
    CarryOver, DisableRule, NeverFires, Pilot, RuleKey, RuleSource, Rulebook, Session,
    SessionSetup, ShipId, SimMessage, StartError, SystemId, TICK, Vec2,
};

use crate::flight::fake_catalog::{Counting, FakeCatalog, MAP, catalog, key, rolling_one, tap};
use crate::flight::view::FlightView;
use crate::flight::{Collaborators, FlightStart, begin_game, enter_game, open_flight, test_flight};
use crate::system::catalog::StellarId;
use crate::text::fixture::MonoMetrics;
use crate::{Input, Key, Point, Screen};

/// A view over a real flight built at the edge, for the edge's own
/// checks: the view's tests run on [`FakeFlight`].
type View = FlightView<FakeCatalog>;

/// Ticks `view` `n` times.
fn ticks(view: &mut View, n: u32) {
    for _ in 0..n {
        view.tick(TICK);
    }
}

/// A new pilot of [`catalog`] flown bare ([`test_flight::new`]).
fn flight() -> View {
    test_flight::new(catalog())
}

/// A new pilot of `catalog` flown, set up at the edge with `rules` and
/// nothing else ([`open_flight`]).
fn ruled(catalog: FakeCatalog, rules: Rulebook) -> View {
    open_flight(
        catalog,
        FlightStart::New,
        SessionSetup {
            rules,
            ..SessionSetup::default()
        },
        Collaborators::default(),
    )
}

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
    begin_game(&mut view);
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

/// Phase 13's fold-in: the edge's beginning pairs the view's half with the
/// adapter's, so a new pilot's ship is drawn where the session began it
/// and the stars are scattered afresh; a flight that begins nothing
/// changes nothing.
#[test]
fn the_edges_beginning_places_the_ship_where_the_session_began_it() {
    let mut view = test_flight::new(catalog());
    view.stars_due = false;
    begin_game(&mut view);
    let begun = view.session().expect("flying").player().position;
    assert_eq!(begun, Vec2::new(50.0, 50.0), "the session began");
    assert_eq!(view.shown_position(), Point::new(50.0, 50.0), "drawn there");
    assert!(view.stars_due, "the stars scattered afresh");
    let mut failed = test_flight::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    failed.stars_due = false;
    begin_game(&mut failed);
    assert!(!failed.stars_due, "a flight that begins nothing");
}

/// Phase 13's fold-in: the edge's entering play pairs the view's half
/// with the adapter's; a flight that enters nothing changes nothing.
#[test]
fn the_edges_entering_play_scatters_the_stars_once_the_session_entered() {
    let mut view = test_flight::new(catalog());
    view.stars_due = false;
    enter_game(&mut view);
    assert!(view.stars_due, "the stars scattered afresh");
    let mut failed = test_flight::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    failed.stars_due = false;
    enter_game(&mut failed);
    assert!(!failed.stars_due, "a flight that enters nothing");
}

/// Phase 13's fold-in: the slot sprites the run carried reach the view
/// through the edge, which reads the sheets their cloned frames ask for
/// (`show_set_up`'s). The player's ship is hidden (`Cost` −1), so its
/// slot draws the frames it last cloned, here of a ship type no traffic
/// carries; with none carried, the view reads none.
#[test]
fn the_slot_sprites_carried_have_the_view_read_their_ships_sheets() {
    let hidden = || {
        let mut catalog = catalog();
        catalog.hulls[0].cost = -1;
        catalog
    };
    let mut sprites = nova_sim::ship_layer::SlotSprites::default();
    sprites
        .frames
        .clone_frames(nova_sim::ship_layer::Slot::Player, ShipId(131));
    let carried = set_up(
        hidden(),
        SessionSetup {
            carry: CarryOver {
                slot_sprites: Some(sprites),
                ..CarryOver::default()
            },
            ..SessionSetup::default()
        },
    );
    assert!(
        !carried.frame.traffic_ships.contains(&ShipId(131)),
        "no traffic carries it"
    );
    assert!(carried.npc_sheets.contains_key(&ShipId(131)), "read");
    let bare = set_up(hidden(), SessionSetup::default());
    assert!(!bare.npc_sheets.contains_key(&ShipId(131)), "none carried");
}

// The edge's rules reach the course map's borders.

/// `catalog` with Sol (130) the Federation's (128) and lived in, and
/// Proxima Near (134, at (40, 30)) government 129's and lived in, known
/// from the start, as Sol lists it; [`known`] explores it too.
fn bordered() -> FakeCatalog {
    let mut catalog = FakeCatalog {
        sol_navs: vec![Some((StellarFlags::CAN_LAND, 0))],
        near: true,
        governed: vec![(130, 128), (134, 129)],
        lived: vec![134],
        ..catalog()
    };
    catalog
        .place_names
        .systems
        .insert(SystemId(130), "Sol".to_owned());
    catalog
}

/// `view` with Proxima Near explored and known.
fn known(mut view: View) -> View {
    let session = view.sim.session.as_mut().expect("flying");
    session.run_set(
        &nova_sim::SetExpr::parse("X134").expect("parses"),
        &mut NeverFires,
    );
    session.mark_known([SystemId(134)]);
    view.refresh_frame();
    view
}

fn passes(view: &View) -> &nova_sim::map_borders::BorderPasses {
    view.course_map().border_passes()
}

/// The edge's rulebook reaches the course map's borders: this phase's
/// real check of the view reading its rules as it opens
/// (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge). Both
/// readings on `FakeFlight` are [`by_default_the_rules_run_the_pass_in_a_tick_and_find_runs_none`]
/// and [`by_the_other_readings_the_pass_walks_a_tick_at_a_time_and_finds_centring_runs_the_coarse_one`].
#[test]
fn the_edges_rules_reach_the_course_map() {
    let rules = Rulebook::default()
        .with_override(RuleKey::MapBorderPace, RuleSource::Bible)
        .with_override(RuleKey::MapFindBorders, RuleSource::Bible);
    let mut view = known(ruled(bordered(), rules)).with_metrics(Rc::new(MonoMetrics));
    tap(&mut view, MAP);
    ticks(&mut view, 1);
    assert_eq!(
        passes(&view).next(),
        Next::Walking { after: None },
        "one null event a tick: the clear"
    );
    ticks(&mut view, 3);
    assert_eq!(passes(&view).next(), Next::Idle);
    view.input(&key(Key::Char('f'), true));
    view.input(&Input::Text('f'));
    view.input(&key(Key::Char('f'), false));
    for c in "Sol".chars() {
        view.input(&Input::Text(c));
    }
    tap(&mut view, Key::Enter);
    assert!(!view.course_map().finding());
    assert_eq!(
        passes(&view).step(),
        8,
        "Find's centring ran the coarse pass"
    );
    assert_eq!(passes(&view).next(), Next::Unstarted);
}

#[test]
fn the_view_shares_its_catalog_with_its_flight() {
    let view = flight();
    let shared = view
        .sim
        .shared_catalog()
        .expect("shared as the view is read");
    assert!(std::ptr::addr_eq(
        Rc::as_ptr(shared),
        Rc::as_ptr(&view.catalog)
    ));
}

/// Ada, loaded from a save in Alpha Centauri (131) whose stellar, 999,
/// no system lists.
fn moved_and_unlisted() -> Pilot {
    let mut pilot = SessionBuilder::flying(&catalog())
        .named("Ada")
        .landed_at(SystemId(130), StellarId(128))
        .build()
        .pilot()
        .clone();
    pilot.explore(SystemId(131));
    let text = nova_sim::save::encode(&pilot)
        .replace("\"stellar\": 128", "\"stellar\": 999")
        .replace("\"system\": 130", "\"system\": 131");
    nova_sim::save::decode(&text).expect("a pilot")
}

#[test]
fn a_loaded_flight_reads_the_rules_it_is_given() {
    // Where each reading moves the pilot is nova-sim's
    // (`loading_an_unlisted_stellar_by_the_engines_reading_moves_to_the_lowest_system`).
    let pilot = moved_and_unlisted();
    let mut systems = Vec::new();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::UnlistedStellar, source);
        let view = open_flight(
            catalog(),
            FlightStart::Loaded(pilot.clone()),
            SessionSetup {
                rules,
                ..SessionSetup::default()
            },
            Collaborators::default(),
        );
        let expected = Session::load(&catalog(), pilot.clone(), &rules).expect("loads");
        let system = view.session().map(Session::system);
        assert_eq!(system, Ok(expected.system()), "{source:?}");
        assert_eq!(view.sim().load_repaired(), expected.load_repaired());
        systems.push(expected.system());
    }
    assert_ne!(systems[0], systems[1], "the readings differ here");
}

// The hypergate map's borders over a real flight.

/// [`bordered`] with Barnard (132), which no system the player can see
/// from Sol lists, government 129's and lived in, so its borders add once
/// it is known.
fn gated_to_barnard() -> FakeCatalog {
    FakeCatalog {
        governed: vec![(130, 128), (134, 129), (132, 129)],
        lived: vec![134, 132],
        ..bordered()
    }
}

/// The seam of the hypergate map's mark: its redraw over a real flight
/// shows the borders the session paints once the drawing's arrows are
/// marked known, the mark the view sends for the session's answer.
#[test]
fn the_hypergate_maps_borders_are_the_sessions_once_its_arrows_are_marked_known() {
    let mut view = test_flight::new(gated_to_barnard());
    view.sim.session.as_mut().expect("flying").run_set(
        &nova_sim::SetExpr::parse("X132").expect("parses"),
        &mut NeverFires,
    );
    view.refresh_frame();
    let session = view.sim.session().expect("flying");
    assert!(
        !session.known().contains(&SystemId(132)),
        "precondition: Barnard explored, not known"
    );
    let unknown = session.map_border_sources();
    view.map.offer_gates(vec![SystemId(132)]);
    view.open_map();
    let session = view.sim.session().expect("flying");
    assert!(
        session.known().contains(&SystemId(132)),
        "the arrow's system marked known"
    );
    let known = session.map_border_sources();
    assert_ne!(known, unknown, "precondition: Barnard adds once known");
    assert_eq!(view.course_map().border_sources(), &known);
}
