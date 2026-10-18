use super::*;

// The pilot.

// Pilot edits.

#[test]
fn the_intro_screen_seen_reads_and_marks_the_sessions_pilot() {
    let mut view = landed_view();
    assert!(!view.seen_intro());
    view.mark_intro_seen();
    assert!(view.seen_intro());
    assert!(view.session().expect("flying").seen_intro());
    let mut broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    broken.mark_intro_seen();
    assert!(!broken.seen_intro());
}

#[test]
fn a_session_that_failed_has_no_pilot_desk() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert!(view.pilot_desk().is_none());
    view.resync();
    assert!(view.scene().is_none());
}

#[test]
fn the_pilot_panel_is_the_sessions_and_none_for_a_session_that_failed() {
    let view = landed_view();
    let session = view.session().expect("flying");
    assert_eq!(
        view.pilot_panel(),
        Some(session.pilot_panel(view.catalog()))
    );
    assert_eq!(
        view.pilot_panel().map(|panel| panel.pilot_name),
        Some("Ada".to_owned())
    );
    let broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(broken.pilot_panel(), None);
}

#[test]
fn the_pilot_desk_reads_the_session_and_names_from_the_map() {
    let mut view = landed_view();
    let desk = view.pilot_desk().expect("flying");
    let sheet = desk.sheet().expect("a pilot");
    assert_eq!(sheet.name, "Ada");
    assert_eq!(sheet.system.name, "Sol");
    assert_eq!(desk.systems().len(), 3);
}

#[test]
fn after_a_move_resync_lays_out_the_new_system() {
    let mut view = landed_view();
    move_to(&mut view, 131, 140);
    view.resync();
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.course_map().route(), []);
    assert!(
        view.session()
            .expect("flying")
            .pilot()
            .has_explored(SystemId(131))
    );
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
    assert_eq!(view.message(), None, "no arrival message");
    view.resync();
    assert_eq!(
        *view.catalog().systems_read.borrow(),
        [SystemId(131)],
        "nothing moved, nothing read"
    );
}

#[test]
fn after_a_move_within_the_system_resync_drops_the_course_and_reads_nothing() {
    let mut view = landed_view();
    plot(&mut view, 131);
    assert_eq!(view.course_map().route(), [SystemId(131)]);
    view.tick(TICK / 2);
    assert!(view.alpha() > 0.0, "between steps");
    move_to(&mut view, 130, 129);
    view.resync();
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    assert_eq!(view.course_map().route(), []);
    assert_eq!(*view.catalog().systems_read.borrow(), []);
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), Point::new(300.0, -200.0));
}

#[test]
fn a_resync_with_nothing_moved_reads_nothing() {
    let mut view = landed_view();
    view.resync();
    assert_eq!(*view.catalog().systems_read.borrow(), []);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
}

#[test]
fn a_pilot_saved_at_a_stellar_resumes_in_flight_over_it_silently() {
    let pilot = docked_pilot("Ada");
    assert_eq!(pilot.stellar(), Some(StellarId(128)));
    let mut view = FlightView::with_pilot(catalog(), pilot.clone());
    assert_eq!(view.pilot(), Some(&pilot));
    assert_eq!(view.take_landing(), None, "nothing lands");
    assert_eq!(view.session().map(Session::landed), Ok(None));
    assert_eq!(player(&view).position, Vec2::new(0.0, -600.0));
    assert_eq!(view.shown_position(), Point::new(0.0, -600.0));
    assert_eq!(view.take_sounds(), [], "no landing sound");
    assert!(!view.take_save_due());
}

#[test]
fn a_new_pilots_ship_is_first_shown_at_50_50() {
    let mut view = flight();
    view.begin();
    assert_eq!(player(&view).position, Vec2::new(50.0, 50.0));
    assert_eq!(
        view.shown_position(),
        Point::new(50.0, 50.0),
        "before any tick, not on its way from the centre"
    );
    view.tick(TICK / 2);
    assert_eq!(view.shown_position(), Point::new(50.0, 50.0), "at rest");
}

#[test]
fn a_pilot_in_flight_resumes_at_the_centre() {
    let pilot = Pilot::new(&catalog(), "Bob").expect("starts");
    let mut view = FlightView::with_pilot(catalog(), pilot);
    assert_eq!(view.take_landing(), None);
    assert_eq!(player(&view), start());
    assert_eq!(view.pilot().map(Pilot::name), Some("Bob"));
}

#[test]
fn a_pilot_whose_system_is_gone_flies_in_the_lowest_that_exists() {
    // In flight: a pilot last landed on a stellar resumes in its
    // stellar's system.
    let mut pilot = Pilot::new(&catalog(), "Ada").expect("starts");
    pilot.explore(SystemId(132));
    let text = nova_sim::save::encode(&pilot).replace("\"system\": 130", "\"system\": 132");
    let moved = nova_sim::save::decode(&text).expect("a pilot");
    let view = FlightView::with_pilot(catalog(), moved);
    let session = view.session().expect("flies");
    assert_eq!(session.system(), SystemId(130));
    assert!(session.load_repaired());
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
    let view = FlightView::loaded(catalog(), unlisted_pilot(), Rulebook::default());
    assert!(view.load_repaired());
    assert_eq!(view.pilot().and_then(Pilot::stellar), None);
    let docked = FlightView::loaded(catalog(), docked_pilot("Ada"), Rulebook::default());
    assert!(!docked.load_repaired());
    assert!(!FlightView::with_pilot(catalog(), docked_pilot("Ada")).load_repaired());
    let failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert!(!failed.load_repaired(), "a failed session repaired nothing");
}

#[test]
fn a_loaded_flight_reads_the_rules_it_is_given() {
    let mut pilot = unlisted_pilot();
    pilot.explore(SystemId(131));
    let text = nova_sim::save::encode(&pilot).replace("\"system\": 130", "\"system\": 131");
    let moved = nova_sim::save::decode(&text).expect("a pilot");
    let kept = FlightView::loaded(catalog(), moved.clone(), Rulebook::default());
    assert_eq!(kept.session().map(Session::system), Ok(SystemId(131)));
    let engine = Rulebook::default().with_override(RuleKey::UnlistedStellar, RuleSource::Engine);
    let lowest = FlightView::loaded(catalog(), moved, engine);
    assert_eq!(lowest.session().map(Session::system), Ok(SystemId(130)));
    assert!(lowest.load_repaired());
}

#[test]
fn landing_taking_off_and_transactions_make_a_save_due() {
    let centred = FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..catalog()
    };
    let mut view = FlightView::new(centred);
    assert!(!view.take_save_due());
    assert!(!view.transact(|pilot| pilot.set_cash(5)), "in flight");
    land_now(&mut view);
    assert!(view.take_save_due(), "landed");
    assert!(view.transact(|pilot| pilot.set_cash(5)));
    assert_eq!(view.pilot().map(Pilot::cash), Some(5));
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
    let mut view = FlightView::new(broken);
    assert_eq!(view.pilot(), None);
    assert!(!view.transact(|pilot| pilot.set_cash(5)));
    assert!(!view.take_save_due());
}

#[test]
fn arriving_explores_and_the_course_maps_panel_shows_it() {
    let mut view = flight();
    let explored = |view: &View| -> Vec<SystemId> {
        view.session().expect("flying").pilot().explored().collect()
    };
    assert_eq!(explored(&view), [SystemId(130)]);
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    arrive_now(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    assert_eq!(explored(&view), [SystemId(130), SystemId(131)]);
    tap(&mut view, MAP);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    let panel = view.course_map().panel().expect("taken as the map opens");
    assert_ne!(panel.column.body, nova_sim::map_panel::ColumnBody::Unknown);
}

#[test]
fn the_open_maps_panel_follows_a_system_explored_between_frames() {
    let centred = FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..catalog()
    };
    let mut view = FlightView::new(centred);
    land_now(&mut view);
    view.open_bar_map();
    let at = on_map(&mut view, 131);
    view.port_map_input(&Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed: true,
        at,
    });
    view.port_map_input(&Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed: false,
        at,
    });
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    let body = |view: &View| {
        view.course_map()
            .panel()
            .expect("taken")
            .column
            .body
            .clone()
    };
    assert_eq!(body(&view), nova_sim::map_panel::ColumnBody::Unknown);
    assert!(view.transact(|pilot| {
        pilot.explore(SystemId(131));
    }));
    assert_eq!(
        body(&view),
        nova_sim::map_panel::ColumnBody::Unknown,
        "not yet"
    );
    view.tick(TICK);
    assert_ne!(
        body(&view),
        nova_sim::map_panel::ColumnBody::Unknown,
        "the next frame"
    );
}

#[test]
fn the_open_maps_panel_follows_a_system_charted_between_frames() {
    // Sol, explored at level 1 by the new pilot, lists Earth with food.
    let earth = StellarFlags::CAN_LAND | 0x0002 | 0x0004 | 0x1000_0000;
    let mut view = FlightView::new(FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), earth)],
        sol_navs: vec![Some((earth, 0))],
        ..catalog()
    });
    land_now(&mut view);
    view.open_bar_map();
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    let goods = |view: &View| match &view.course_map().panel().expect("taken").column.body {
        nova_sim::map_panel::ColumnBody::Inhabited { goods, .. } => goods.clone(),
        body => panic!("{body:?}"),
    };
    assert_eq!(
        goods(&view),
        nova_sim::map_panel::Listing::Unknown,
        "level 1"
    );
    assert!(view.transact(|pilot| {
        pilot.chart(SystemId(130));
    }));
    assert_eq!(
        goods(&view),
        nova_sim::map_panel::Listing::Unknown,
        "not yet"
    );
    view.tick(TICK);
    assert!(
        matches!(goods(&view), nova_sim::map_panel::Listing::Items(_)),
        "level 2, the next frame"
    );
}

// The exchange and the day's chances.

#[test]
fn the_exchange_is_the_sessions_and_a_trade_goes_through_it() {
    let mut view = FlightView::new(trading());
    assert_eq!(view.market(), None, "in flight");
    assert_eq!(view.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
    land_now(&mut view);
    view.take_save_due();
    let market = view.market().expect("landed at a trade center");
    assert_eq!(
        market.row(Good::Commodity(0)).map(|row| row.price),
        Some(75)
    );
    assert_eq!(view.trade(BUY_FOOD), Ok(10), "a click buys up to 10");
    assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 10 * 75));
    assert!(view.take_save_due(), "a trade");
    assert_eq!(
        view.market()
            .and_then(|m| m.row(Good::Commodity(0)).map(|row| row.held)),
        Some(10)
    );
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..trading()
    };
    let mut broken = FlightView::new(broken);
    assert_eq!(broken.market(), None);
    assert_eq!(broken.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
}

#[test]
fn the_outfitter_is_the_sessions_and_an_order_goes_through_it() {
    let mut view = FlightView::new(outfitting());
    assert_eq!(view.outfitter(), None, "in flight");
    assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
    land_now(&mut view);
    view.take_save_due();
    let outfitter = view.outfitter().expect("landed at an outfitter");
    assert_eq!(
        outfitter.row(OutfitId(200)).map(|row| row.price),
        Some(1000)
    );
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(view.pilot().map(Pilot::cash), Some(0));
    assert!(view.take_save_due(), "a purchase");
    assert_eq!(
        view.outfitter()
            .and_then(|o| o.row(OutfitId(200)).map(|row| row.owned)),
        Some(1)
    );
    assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..outfitting()
    };
    let mut broken = FlightView::new(broken);
    assert_eq!(broken.outfitter(), None);
    assert_eq!(broken.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
}

#[test]
fn a_counted_order_goes_through_the_session() {
    let mut catalog = outfitting();
    catalog.character = Ok(CharacterStart {
        cash: 3000,
        ..catalog.character.expect("a chär")
    });
    let mut view = FlightView::new(catalog);
    assert_eq!(
        view.outfit_counted(BUY_TANK, 2),
        Err(OutfitRefusal::NoOutfitter),
        "in flight"
    );
    land_now(&mut view);
    view.take_save_due();
    assert_eq!(view.outfit_counted(BUY_TANK, 2), Ok(2));
    assert_eq!(view.pilot().map(Pilot::cash), Some(1000));
    assert!(view.take_save_due(), "a purchase");
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..outfitting()
    };
    let mut broken = FlightView::new(broken);
    assert_eq!(
        broken.outfit_counted(BUY_TANK, 2),
        Err(OutfitRefusal::NoOutfitter)
    );
}

#[test]
fn an_outfit_order_goes_through_the_session_with_its_run() {
    let mut catalog = outfitting();
    catalog.character = Ok(CharacterStart {
        cash: 3000,
        ..catalog.character.expect("a chär")
    });
    let refused = |refusal| OutfitRun {
        run: OrderRun {
            done: 0,
            stopped: Some(Stopped { pass: 0, refusal }),
        },
        alert: None,
        after: Reselection::Kept,
        button_passed: false,
    };
    let mut view = FlightView::new(catalog);
    assert_eq!(
        view.outfit_order(BUY_TANK, 2),
        refused(OutfitRefusal::NoOutfitter),
        "in flight"
    );
    land_now(&mut view);
    view.take_save_due();
    assert_eq!(
        view.outfit_order(BUY_TANK, 4),
        OutfitRun {
            run: OrderRun {
                done: 3,
                stopped: Some(Stopped {
                    pass: 3,
                    refusal: OutfitRefusal::CannotAfford,
                }),
            },
            alert: None,
            after: Reselection::Kept,
            button_passed: true,
        }
    );
    assert_eq!(view.pilot().map(Pilot::cash), Some(0));
    assert!(view.take_save_due(), "a purchase");
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..outfitting()
    };
    let mut broken = FlightView::new(broken);
    assert_eq!(
        broken.outfit_order(BUY_TANK, 2),
        refused(OutfitRefusal::NoOutfitter)
    );
}

/// [`outfitting`] with 10,000 credits, a -10-ton pod (201) and an
/// outfit (202) of `heavy` tons, each 100 credits.
fn pod_outfitting(heavy: i16) -> FakeCatalog {
    let mut catalog = outfitting();
    catalog.character = Ok(CharacterStart {
        cash: 10_000,
        ..catalog.character.expect("a chär")
    });
    let tank = catalog.outfits[0].clone();
    for (id, mass) in [(201, -10), (202, heavy)] {
        catalog.outfits.push(OutfitRecord {
            id: OutfitId(id),
            mass,
            cost: 100,
            lc_name: "pod".to_owned(),
            lc_plural: "pods".to_owned(),
            ..tank.clone()
        });
    }
    catalog
}

#[test]
fn a_counted_sale_refused_part_way_goes_through_the_session_with_its_alert() {
    let pod = OutfitId(201);
    let mut probe = FlightView::new(pod_outfitting(0));
    land_now(&mut probe);
    let free = probe.outfitter().expect("open").free_mass;
    // Three pods and the heavy outfit leave ten tons free: one pod
    // sold leaves none, and the next is refused at pass 1.
    let heavy = i16::try_from(free).expect("small") + 20;
    let mut view = FlightView::new(pod_outfitting(heavy));
    land_now(&mut view);
    let buy = |outfit| OutfitOrder {
        outfit,
        direction: Direction::Buy,
    };
    assert_eq!(view.outfit_order(buy(pod), 3).run.done, 3);
    assert_eq!(view.outfit_order(buy(OutfitId(202)), 1).run.done, 1);
    assert_eq!(view.outfitter().expect("open").free_mass, 10);
    view.take_save_due();
    let sell = OutfitOrder {
        outfit: pod,
        direction: Direction::Sell,
    };
    let order = view.outfit_order(sell, 3);
    assert_eq!(
        order.run,
        OrderRun {
            done: 1,
            stopped: Some(Stopped {
                pass: 1,
                refusal: OutfitRefusal::NegativeFreeMass,
            }),
        }
    );
    let alert = order.alert.expect("an alert");
    assert_eq!(alert.refusal, OutfitRefusal::NegativeFreeMass);
    assert!(alert.plural, "a pass remains");
    assert_eq!(view.pilot().map(|pilot| pilot.owned(pod)), Some(2));
    assert!(view.take_save_due(), "a sale");
}

#[test]
fn opening_the_outfitter_lifts_the_sessions_once_an_opening_limit() {
    let mut catalog = outfitting();
    catalog.character = Ok(CharacterStart {
        cash: 5000,
        ..catalog.character.expect("a chär")
    });
    catalog.outfits[0].mods[0] = (nova_sim::outfitter::EXPLORES_MAP, 5);
    let mut view = FlightView::new(catalog);
    land_now(&mut view);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::BoughtThisOpening));
    view.open_outfitter();
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..outfitting()
    };
    FlightView::new(broken).open_outfitter();
}

#[test]
fn recharging_goes_through_the_session() {
    use nova_sim::RechargeRefusal;
    let mut view = FlightView::new(outfitting());
    assert_eq!(view.recharge(), Err(RechargeRefusal::NoFuel), "in flight");
    land_now(&mut view);
    view.take_save_due();
    assert_eq!(view.recharge(), Err(RechargeRefusal::Full));
    assert!(!view.take_save_due());
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..outfitting()
    };
    let mut broken = FlightView::new(broken);
    assert_eq!(broken.recharge(), Err(RechargeRefusal::NoFuel));
}

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
    let mut view = FlightView::new(eventful()).with_chance(SharedChance::new(shared));
    jump_to_alpha(&mut view);
    assert_eq!(always.borrow().asked, [35]);
    let events: Vec<_> = view.pilot().expect("a pilot").events().collect();
    assert_eq!(events, [(DisasterId(128), 30)]);
    assert_eq!(format!("{:?}", SharedChance::default()), "Chance");
}

#[test]
fn leaving_rolls_the_stays_days_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut view = FlightView::with_pilot(eventful(), docked_pilot("Ada"))
        .with_chance(SharedChance::new(shared));
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
    let mut view = FlightView::new(eventful());
    jump_to_alpha(&mut view);
    assert_eq!(view.pilot().expect("a pilot").events().count(), 0);
    let mut shared = SharedChance::default();
    assert!(!shared.fires(100));
    assert_eq!(shared.roll(8), 0);
}

#[test]
fn a_shared_chance_rolls_on_its_source() {
    let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
    assert_eq!(SharedChance::new(shared).roll(360), 359);
}

#[test]
fn the_shipyard_is_the_sessions_and_a_purchase_reloads_the_ships_sheet() {
    let mut view = FlightView::new(shipbuying());
    assert_eq!(view.shipyard(), None, "in flight");
    assert_eq!(
        view.buy_ship(ShipId(129), "Kestrel"),
        Err(ShipRefusal::NoShipyard)
    );
    land_now(&mut view);
    view.take_save_due();
    let shipyard = view.shipyard().expect("landed at a shipyard");
    assert_eq!(shipyard.row(ShipId(129)).map(|row| row.price), Some(900));
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    assert_eq!(
        view.buy_ship(ShipId(999), "Kestrel"),
        Err(ShipRefusal::NotListed),
        "refused"
    );
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    let bought = view.buy_ship(ShipId(129), "The Kestrel").expect("bought");
    assert_eq!(bought.price, 900);
    assert_eq!(
        view.pilot().and_then(Pilot::ship_name),
        Some("Kestrel"),
        "named as confirmed"
    );
    assert!(view.take_save_due(), "a purchase");
    assert_eq!(view.pilot().map(Pilot::ship), Some(ShipId(129)));
    assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 900));
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)]
    );
    assert_eq!(view.reserves().fuel.max, 350.0, "its tank");
    // Off again, the new hull is drawn, at the new speed.
    view.settle_script();
    view.take_off().expect("took off");
    view.settle_script();
    view.tick(TICK);
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)],
        "read once"
    );
    let mut list = DrawList::new();
    view.draw(&mut list);
    assert!(
        list.iter().any(|command| matches!(
            command,
            DrawCommand::Sprite { image, .. } if image.id == 2001
        )),
        "the new sheet"
    );
    let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
    assert_eq!(ids, [2001, 2201], "and its lights");
    assert_eq!(
        view.session().map(|session| session.handling().max_speed),
        Ok(6.0)
    );
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..shipbuying()
    };
    let mut broken = FlightView::new(broken);
    assert_eq!(broken.shipyard(), None);
    assert_eq!(
        broken.buy_ship(ShipId(129), "Kestrel"),
        Err(ShipRefusal::NoShipyard)
    );
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
    let mut view = FlightView::new(rolling()).with_chance(SharedChance::default());
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
    let mut view = FlightView::new(rolling()).with_chance(SharedChance::new(shared));
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
    let mut view = FlightView::new(shipbuying()).with_chance(SharedChance::new(shared));
    assert_eq!(view.ship_naming(ShipId(129)), Err(ShipRefusal::NoShipyard));
    land_now(&mut view);
    view.take_save_due();
    assert_eq!(view.ship_naming(ShipId(999)), Err(ShipRefusal::NotListed));
    let naming = view.ship_naming(ShipId(129)).expect("a naming");
    assert_eq!(naming.ship, ShipId(129));
    assert_eq!(naming.prompt, "Please name your new : ");
    assert_eq!(naming.default, "Fast 999", "each digit the last of nine");
    assert!(!view.take_save_due());
    let mut broken = FlightView::new(FakeCatalog {
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
    let mut view = FlightView::new(rolling()).with_chance(SharedChance::new(shared));
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

#[test]
fn every_rule_reaches_the_session_through_the_flight() {
    let every = RuleKey::ALL
        .into_iter()
        .fold(Rulebook::default(), |book, key| {
            book.with_override(key, RuleSource::Bible)
        });
    assert_eq!(
        flight().session().map(Session::rules),
        Ok(&Rulebook::default())
    );
    let view = flight().with_rules(every);
    let rules = view.session().expect("flying").rules();
    assert_eq!(rules, &every);
    for key in RuleKey::ALL {
        assert_eq!(rules.source_for(key), RuleSource::Bible, "{key:?}");
    }
}

// The set-expression hooks.

#[test]
fn an_outfits_on_purchase_runs_on_the_flights_chance() {
    let mut catalog = outfitting();
    catalog.outfits[0].on_purchase = nova_sim::Script::parse("R(b1 b2)");
    let mut view = FlightView::new(catalog).with_chance(rolling_one());
    land_now(&mut view);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert!(bit_set(&view, 1));
    assert!(!bit_set(&view, 2));
}

#[test]
fn a_ships_on_purchase_runs_on_the_flights_chance() {
    let mut catalog = shipbuying();
    catalog.ships[0].on_purchase = nova_sim::Script::parse("R(b3 b4)");
    let mut view = FlightView::new(catalog).with_chance(rolling_one());
    land_now(&mut view);
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    assert!(bit_set(&view, 3));
    assert!(!bit_set(&view, 4));
}

#[test]
fn a_ships_on_purchase_h_reads_only_the_final_class_sheet_once() {
    let mut catalog = shipbuying();
    catalog.ships[0].on_purchase = nova_sim::Script::parse("H130");
    let other = ShipRecord {
        id: ShipId(130),
        ..catalog.ships[0].clone()
    };
    catalog.ships.push(other);
    let mut view = FlightView::new(catalog);
    land_now(&mut view);
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    assert_eq!(view.session().map(Session::ship), Ok(ShipId(130)));
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(130)]
    );
    view.settle_script();
    view.take_off().expect("took off");
    view.settle_script();
    view.tick(TICK);
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(130)],
        "a failed read is not tried again"
    );
    assert_eq!(view.frame(), None, "no sheet for 130");
}

/// [`shipbuying`], ship 129 carrying IFF ([`radar_outfits`]' 300)
/// among its default outfits, landed.
fn landed_shipyard_with_iff() -> View {
    let mut catalog = shipbuying();
    catalog.outfits.extend(radar_outfits());
    catalog.ships[0].defaults.push((OutfitId(300), 1));
    let mut view = FlightView::new(catalog);
    land_now(&mut view);
    view
}

#[test]
fn a_ship_bought_with_iff_shows_it_at_once() {
    let mut view = landed_shipyard_with_iff();
    assert!(view.hud_waits(), "the engine's reading");
    assert!(!view.radar_shown().iff, "landing's");
    assert_eq!(
        view.buy_ship(ShipId(999), "Kestrel"),
        Err(ShipRefusal::NotListed)
    );
    assert!(!view.radar_shown().iff, "refused");
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    assert!(view.radar_shown().iff, "its default IFF");
    assert!(radar_filled(&view));
}

#[test]
fn a_ship_bought_redraws_the_nav_area_and_the_target_panel() {
    let mut view = landed_shipyard_with_iff();
    let stale_nav = NavDisplay::Stellar(Some("Stale".to_owned()));
    let stale_target = TargetPanel {
        name: "Stale".to_owned(),
        subtitle: String::new(),
        picture: None,
        status: nova_sim::TargetStatus::Hidden,
        tag: None,
    };
    let stale = |view: &mut View| {
        view.nav_scene = Some((view.nav_mode(), stale_nav.clone()));
        view.target_scene = Some((None, Some(stale_target.clone())));
    };
    stale(&mut view);
    assert_eq!(view.nav_shown(), stale_nav, "held");
    assert_eq!(*view.target_shown(), Some(stale_target.clone()), "held");
    assert!(view.buy_ship(ShipId(999), "Kestrel").is_err());
    assert_eq!(view.nav_shown(), view.nav_now(), "the prompt's close");
    stale(&mut view);
    assert_eq!(view.nav_shown(), stale_nav, "held again");
    assert_eq!(*view.target_shown(), Some(stale_target.clone()), "held");
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    assert_eq!(view.nav_shown(), view.nav_now(), "redrawn");
    assert_ne!(view.nav_shown(), stale_nav);
    assert_eq!(*view.target_shown(), None, "redrawn");
}
