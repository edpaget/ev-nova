use super::*;

// The pilot.

// Pilot edits.

/// This phase's real check of the adapter's [`PilotSource`]
/// (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks): the pilot
/// panel and the player info are the session's, over the catalog the
/// view shared. What the view does with them runs on `FakeFlight`
/// (`tests/faked_player.rs`).
///
/// [`PilotSource`]: crate::flight::sim_port::PilotSource
#[test]
fn the_pilot_panel_is_the_sessions_and_none_for_a_session_that_failed() {
    let view = landed_view();
    let session = view.session().expect("flying");
    assert_eq!(
        view.pilot_panel(),
        Some(session.pilot_panel(view.catalog()))
    );
    assert_eq!(
        view.player_info(),
        Some(session.player_info(view.catalog())),
        "the player info too"
    );
    assert_eq!(
        view.pilot_panel().map(|panel| panel.pilot_name),
        Some("Ada".to_owned())
    );
    let broken = test_flight::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(broken.pilot_panel(), None);
    assert_eq!(broken.player_info(), None);
}

/// [`docked_pilot`] Ada with her stellar rewritten to 999, which no
/// system lists.
fn unlisted_pilot() -> Pilot {
    let text = nova_sim::save::encode(&docked_pilot("Ada"))
        .replace("\"stellar\": 128", "\"stellar\": 999");
    let pilot = nova_sim::save::decode(&text).expect("a pilot");
    assert_eq!(pilot.stellar(), Some(StellarId(999)));
    pilot
}

#[test]
fn a_loaded_flight_reports_a_repaired_load() {
    let view = test_flight::loaded(catalog(), unlisted_pilot(), &Rulebook::default());
    assert!(view.sim().load_repaired());
    assert_eq!(view.sim().pilot().and_then(Pilot::stellar), None);
    let docked = test_flight::loaded(catalog(), docked_pilot("Ada"), &Rulebook::default());
    assert!(!docked.sim().load_repaired());
    assert!(
        !test_flight::with_pilot(catalog(), docked_pilot("Ada"))
            .sim()
            .load_repaired()
    );
    let failed = test_flight::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert!(
        !failed.sim().load_repaired(),
        "a failed session repaired nothing"
    );
}

#[test]
fn a_loaded_flight_reads_the_rules_it_is_given() {
    let mut pilot = unlisted_pilot();
    pilot.explore(SystemId(131));
    let text = nova_sim::save::encode(&pilot).replace("\"system\": 130", "\"system\": 131");
    let moved = nova_sim::save::decode(&text).expect("a pilot");
    let kept = test_flight::loaded(catalog(), moved.clone(), &Rulebook::default());
    assert_eq!(kept.session().map(Session::system), Ok(SystemId(131)));
    let engine = Rulebook::default().with_override(RuleKey::UnlistedStellar, RuleSource::Engine);
    let lowest = test_flight::loaded(catalog(), moved, &engine);
    assert_eq!(lowest.session().map(Session::system), Ok(SystemId(130)));
    assert!(lowest.sim().load_repaired());
}

#[test]
fn landing_taking_off_and_transactions_make_a_save_due() {
    let centred = FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..catalog()
    };
    let mut view = test_flight::new(centred);
    assert!(!view.take_save_due());
    assert!(!view.transact(|pilot| pilot.set_cash(5)), "in flight");
    land_now(&mut view);
    assert!(view.take_save_due(), "landed");
    assert!(view.transact(|pilot| pilot.set_cash(5)));
    assert_eq!(view.sim().pilot().map(Pilot::cash), Some(5));
    assert!(view.take_save_due(), "a transaction");
    assert!(!view.take_save_due(), "taken");
    view.take_off();
    assert!(view.take_save_due(), "took off");
}

#[test]
fn a_view_that_cannot_fly_has_no_pilot_and_saves_nothing() {
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    let mut view = test_flight::new(broken);
    assert_eq!(view.sim().pilot(), None);
    assert!(!view.transact(|pilot| pilot.set_cash(5)));
    assert!(!view.take_save_due());
}

// The exchange and the day's chances.

#[test]
fn a_shared_chance_draws_from_its_source() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut chance = SharedChance::new(shared);
    assert_eq!(chance.below(7), 0);
    assert_eq!(chance.below(500), 0);
    assert_eq!(always.borrow().drawn, [7, 500]);
    assert_eq!(SharedChance::default().below(256), 255, "never fires");
}

/// A food surplus at Proxima, 35 % a day.
fn eventful() -> FakeCatalog {
    FakeCatalog {
        disasters: vec![DisasterRecord {
            id: DisasterId(128),
            name: "An enormous food surplus".to_owned(),
            stellar: 140,
            commodity: 0,
            price_delta: -15,
            duration: 30,
            freq: 35,
            activate_on: nova_sim::Test::default(),
        }],
        ..catalog()
    }
}

#[test]
fn a_jump_rolls_the_days_events_on_the_chance_given() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut view = new_on(eventful(), SharedChance::new(shared));
    jump_to_alpha(&mut view);
    assert_eq!(always.borrow().asked, [35]);
    let events: Vec<_> = view.sim().pilot().expect("a pilot").events().collect();
    assert_eq!(events, [(DisasterId(128), 30)]);
    assert_eq!(format!("{:?}", SharedChance::default()), "Chance");
}

#[test]
fn leaving_rolls_the_stays_days_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut view = over_earth_on(eventful(), SharedChance::new(shared));
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    always.borrow_mut().asked.clear();
    let landed = view.session().expect("docked").date();
    view.take_off().expect("took off");
    assert_eq!(always.borrow().asked, [35], "the take-off's day");
    assert_eq!(view.session().expect("flying").date(), landed.next_day());
}

#[test]
fn without_a_chance_given_nothing_random_happens() {
    let mut view = test_flight::new(eventful());
    jump_to_alpha(&mut view);
    assert_eq!(view.sim().pilot().expect("a pilot").events().count(), 0);
    let mut shared = SharedChance::default();
    assert!(!shared.fires(100));
    assert_eq!(shared.roll(8), 0);
}

#[test]
fn a_shared_chance_rolls_on_its_source() {
    let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
    assert_eq!(SharedChance::new(shared).roll(360), 359);
}

/// [`shipbuying`], with the fuel tank and ship 129 each for sale on a
/// `BuyRandom` of 50.
fn rolling() -> FakeCatalog {
    let mut catalog = shipbuying();
    catalog.outfits[0].buy_random = 50;
    catalog.ships[0].buy_random = 50;
    catalog
}

#[test]
fn the_outfitter_and_shipyard_roll_on_the_flights_chance() {
    let mut view = new_on(rolling(), SharedChance::default());
    land_now(&mut view);
    let outfitter = view.outfitter().expect("an outfitter");
    assert!(outfitter.row(OutfitId(200)).is_none(), "off today");
    let shipyard = view.shipyard().expect("a shipyard");
    assert!(shipyard.row(ShipId(129)).is_none(), "off today");
    assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NotListed));
    assert_eq!(
        view.buy_ship(ShipId(129), "Kestrel"),
        Err(ShipRefusal::NotListed)
    );
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut view = new_on(rolling(), SharedChance::new(shared));
    land_now(&mut view);
    always.borrow_mut().asked.clear();
    let outfitter = view.outfitter().expect("an outfitter");
    assert!(outfitter.row(OutfitId(200)).is_some(), "on today");
    let shipyard = view.shipyard().expect("a shipyard");
    assert!(shipyard.row(ShipId(129)).is_some(), "on today");
    assert_eq!(always.borrow().asked, [50, 50]);
    assert!(view.buy_ship(ShipId(129), "Kestrel").is_ok());
}

#[test]
fn the_naming_is_the_sessions_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut prompts = vec![String::new(); 119];
    prompts.push("Christen thy".to_owned());
    let catalog = FakeCatalog {
        strings: vec![(2002, prompts)],
        ..shipbuying()
    };
    let mut view = new_on(catalog, SharedChance::new(shared));
    assert_eq!(view.ship_naming(ShipId(129)), Err(ShipRefusal::NoShipyard));
    land_now(&mut view);
    view.take_save_due();
    assert_eq!(view.ship_naming(ShipId(999)), Err(ShipRefusal::NotListed));
    let naming = view.ship_naming(ShipId(129)).expect("a naming");
    assert_eq!(naming.ship, ShipId(129));
    assert_eq!(
        naming.prompt, "Christen thy : ",
        "#120, read from the catalog"
    );
    assert_eq!(naming.default, "Fast 999", "each digit the last of nine");
    assert!(!view.take_save_due());
    let mut broken = test_flight::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..shipbuying()
    });
    assert_eq!(
        broken.ship_naming(ShipId(129)),
        Err(ShipRefusal::NoShipyard)
    );
    broken.decline_ship(ShipId(129));
}

#[test]
fn declining_a_ship_draws_its_roll_again_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut view = new_on(rolling(), SharedChance::new(shared));
    land_now(&mut view);
    view.take_save_due();
    view.shipyard().expect("a shipyard");
    always.borrow_mut().asked.clear();
    view.shipyard().expect("a shipyard");
    assert!(always.borrow().asked.is_empty(), "kept");
    view.decline_ship(ShipId(129));
    assert!(!view.take_save_due());
    view.shipyard().expect("a shipyard");
    assert_eq!(always.borrow().asked, [50], "drawn again");
}

// The set-expression hooks.

#[test]
fn an_outfits_on_purchase_runs_on_the_flights_chance() {
    let mut catalog = outfitting();
    catalog.outfits[0].on_purchase = nova_sim::Script::parse("R(b1 b2)");
    let mut view = new_on(catalog, rolling_one());
    land_now(&mut view);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert!(bit_set(&view, 1));
    assert!(!bit_set(&view, 2));
}

#[test]
fn a_ships_on_purchase_runs_on_the_flights_chance() {
    let mut catalog = shipbuying();
    catalog.ships[0].on_purchase = nova_sim::Script::parse("R(b3 b4)");
    let mut view = new_on(catalog, rolling_one());
    land_now(&mut view);
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    assert!(bit_set(&view, 3));
    assert!(!bit_set(&view, 4));
}
