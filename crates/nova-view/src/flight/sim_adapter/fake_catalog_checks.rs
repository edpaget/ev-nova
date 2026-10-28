//! The adapter's own checks over the view's [`FakeCatalog`]
//! (rdm:phase/view-sim-seam/phase-14-view-tests-on-port): each
//! [`SessionFlight`] forward that a fake can't stand for, as it hands the
//! session the flight's chance, catalog or collaborators, checked over a
//! session a nova-sim builder made. No `FlightView` is built and no key is
//! sent: each check drives the flight through the port and its own API,
//! and compares its answer with the session's own, or with what the
//! collaborator saw. What the session does with them is nova-sim's. The
//! checks over nova-sim's own fixture catalog are the parent's `tests`;
//! together they are the adapter's one home
//! (rdm:phase/view-sim-seam/phase-15-integration-test-set).

use std::cell::RefCell;
use std::rc::Rc;

use nova_sim::catalog::MissionRecord;
use nova_sim::fixture::SessionBuilder;
use nova_sim::flight_frame::{Answer, FlightCommand, FlightFrame, MapClick, StepInput, StepReport};
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::landing::StellarFlags;
use nova_sim::mission::screens::SPACEPORT;
use nova_sim::{
    ApplyCollaborators, Behaviour, Chance, ControlBits, DisasterId, DisasterRecord, Goal, Good,
    HailOptions, JunkRecord, LandingSite, MissionId, NeverFires, NovaBoarding, NovaLaw, OutfitId,
    Pilot, PlayerFate, RuleKey, RuleSource, Rulebook, Rules, Session, SessionSetup, ShipId,
    ShipRecord, ShipRef, SimCatalog, StellarPick, SystemId, VenueOffer,
};

use crate::flight::catalog::GovtId;
use crate::flight::fake_catalog::{
    Always, BLASTER, BUY_TANK, FakeCatalog, Firing, ROCKET, RollsOne, Still, THRUST, armed,
    boardable, catalog, comm_strings, damaging, defending, errand, hiring, holding, killed_in,
    multi_jumping, on, outfitting, placed, rolling_one, scripted, shipbuying, site, trading,
    trafficked,
};
use crate::flight::sim_port::fixture::check_contract;
use crate::flight::sim_port::{CommSource, FlightSim, MapSource, PilotSource, PortSource};
use crate::flight::{Collaborators, SessionFlight, SharedChance};
use crate::system::catalog::StellarId;

/// `catalog`, shared as the edge shares it.
fn shared(catalog: FakeCatalog) -> Rc<FakeCatalog> {
    Rc::new(catalog)
}

/// The flight of `session` over `catalog`, shared, its calls handed
/// `collaborators`.
fn flight_of(
    catalog: &Rc<FakeCatalog>,
    session: Session,
    collaborators: Collaborators,
) -> SessionFlight<FakeCatalog> {
    let mut flight = SessionFlight::from(session).with_collaborators(collaborators);
    flight.share_catalog(Rc::clone(catalog) as Rc<dyn SimCatalog>);
    flight
}

/// Nova's collaborators, rolling on `chance`.
fn rolling_on(chance: SharedChance) -> Collaborators {
    Collaborators {
        chance,
        ..Collaborators::default()
    }
}

/// `chance` shared, and a handle to read it back.
fn share<T: Chance + 'static>(chance: T) -> (Rc<RefCell<T>>, SharedChance) {
    let held = Rc::new(RefCell::new(chance));
    let shared: Rc<RefCell<dyn Chance>> = held.clone();
    (held, SharedChance::new(shared))
}

/// `catalog`'s new pilot Ada, landed at Earth (128) in Sol.
fn landed(catalog: &FakeCatalog) -> Session {
    SessionBuilder::flying(catalog)
        .named("Ada")
        .landed_at(SystemId(130), StellarId(128))
        .build()
}

/// Runs `command` on `session` by hand, handed `catalog`, `chance` and
/// Nova's law, boardings and hail options, as the adapter hands them.
fn by_hand(
    session: &mut Session,
    catalog: &FakeCatalog,
    command: FlightCommand,
    chance: &mut dyn Chance,
) -> Answer {
    let mut collab = ApplyCollaborators {
        catalog: Some(catalog),
        chance,
        law: &NovaLaw::default(),
        boarding_rule: &NovaBoarding::default(),
        hail_options: &HailOptions::default(),
    };
    session.apply(command, &mut collab)
}

/// The frame `flight` fills now.
fn frame_of(flight: &SessionFlight<FakeCatalog>) -> FlightFrame {
    let mut frame = FlightFrame::default();
    flight.frame(&mut frame);
    frame
}

/// Steps `flight` once under `input`.
fn step(flight: &mut SessionFlight<FakeCatalog>, input: &StepInput) -> StepReport {
    let mut report = StepReport::default();
    flight.step(input, 1, &mut report);
    report
}

/// Plots a course to Alpha Centauri (131) through the port, as the map's
/// Shift-click and its close send it, flies out past the jump distance,
/// jumps, steps through the warp and arrives, checking the port's
/// contract at each stage of the jump on a copy of the flight.
fn jump_to_alpha(flight: &mut SessionFlight<FakeCatalog>) {
    let alpha = SystemId(131);
    let click = FlightCommand::MapClick(MapClick {
        hits: vec![alpha],
        shift: true,
        hypergate: None,
        missions: Vec::new(),
        green: None,
    });
    assert!(matches!(flight.apply(click), Answer::Clicked(_)), "plotted");
    flight.apply(FlightCommand::CueCourse);
    let thrust = StepInput {
        controls: THRUST,
        ..StepInput::default()
    };
    for _ in 0..2000 {
        if frame_of(flight).player().position.length() >= MIN_JUMP_DISTANCE {
            break;
        }
        step(flight, &thrust);
    }
    let contract = |flight: &SessionFlight<FakeCatalog>, at: &str| {
        assert_eq!(
            check_contract(&mut flight.clone(), &flight.opening()),
            Ok(()),
            "{at}"
        );
    };
    contract(flight, "ready");
    assert_eq!(flight.apply(FlightCommand::Jump), Answer::Jumped(alpha));
    let frame = frame_of(flight);
    assert!(frame.jump_under_way && !frame.warping, "preparing");
    contract(flight, "preparing");
    let mut warped = false;
    for _ in 0..3000 {
        let frame = frame_of(flight);
        if frame.warping && !warped {
            warped = true;
            contract(flight, "warping");
        }
        if frame.warp_end_due {
            break;
        }
        step(flight, &StepInput::default());
    }
    assert!(warped && frame_of(flight).warp_end_due, "the warp ended");
    contract(flight, "the warp's end due");
    assert!(matches!(
        flight.apply(FlightCommand::Arrive),
        Answer::Arrived { system, .. } if system == alpha
    ));
    assert_eq!(frame_of(flight).system, alpha, "arrived");
    contract(flight, "arrived");
}

/// A food surplus at Proxima (140), 35 % a day.
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

/// [`shipbuying`], with the fuel tank and ship 129 each for sale on a
/// `BuyRandom` of 50.
fn rolling() -> FakeCatalog {
    let mut catalog = shipbuying();
    catalog.outfits[0].buy_random = 50;
    catalog.ships[0].buy_random = 50;
    catalog
}

// The jump.

/// Phase 15's contract in states: a real flight meets the port's
/// contract through a jump, from ready to arrived, reached through the
/// port alone ([`jump_to_alpha`]).
#[test]
fn a_real_flight_meets_the_contract_through_a_jump() {
    let catalog = shared(catalog());
    let session = SessionBuilder::flying(&*catalog).build();
    let mut flight = flight_of(&catalog, session, Collaborators::default());
    jump_to_alpha(&mut flight);
}

// The day's chances.

#[test]
fn leaving_rolls_the_stays_days_on_the_flights_chance() {
    let (always, chance) = share(Always::default());
    let catalog = shared(eventful());
    let session = landed(&catalog);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    let mut own = Always::default();
    let mut session = session;
    let expected = by_hand(&mut session, &catalog, FlightCommand::TakeOff, &mut own);
    assert_eq!(flight.apply(FlightCommand::TakeOff), expected);
    assert_eq!(always.borrow().asked, own.asked, "on the flight's chance");
    assert!(own.asked.contains(&35), "the take-off's day");
    assert_eq!(flight.session(), Ok(&session));
}

#[test]
fn without_a_chance_given_nothing_random_happens() {
    // The flight's chance by default is the never-firing one.
    let catalog = shared(catalog());
    let mut flight = SessionFlight::start(&catalog);
    assert!(!flight.chance.fires(100));
    assert_eq!(flight.chance.below(256), 255);
    let mut given = flight.with_collaborators(Collaborators::default());
    assert!(!given.chance.fires(100), "Nova's collaborators' too");
}

// The spaceport's rolls.

#[test]
fn the_outfitter_and_shipyard_roll_on_the_flights_chance() {
    let (always, chance) = share(Always::default());
    let catalog = shared(rolling());
    let mut session = landed(&catalog);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    let mut own = Always::default();
    assert_eq!(flight.outfitter(), session.outfitter(&mut own));
    assert_eq!(flight.shipyard(), session.shipyard(&mut own));
    assert!(!own.asked.is_empty(), "the day's rolls");
    assert_eq!(always.borrow().asked, own.asked, "on the flight's chance");
}

#[test]
fn the_naming_is_the_sessions_on_the_flights_chance() {
    let (_, chance) = share(Always::default());
    let mut prompts = vec![String::new(); 119];
    prompts.push("Christen thy".to_owned());
    let catalog = shared(FakeCatalog {
        strings: vec![(2002, prompts)],
        ..shipbuying()
    });
    let mut session = landed(&catalog);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    let mut own = Always::default();
    let expected = session.ship_naming(ShipId(129), &*catalog, &mut own);
    assert!(expected.is_ok(), "{expected:?}");
    assert_eq!(flight.ship_naming(ShipId(129)), expected, "on its chance");
    let mut never = flight_of(&catalog, landed(&catalog), Collaborators::default());
    assert_ne!(
        never.ship_naming(ShipId(129)),
        expected,
        "the default name's digits are rolled"
    );
}

#[test]
fn declining_a_ship_draws_its_roll_again_on_the_flights_chance() {
    let (always, chance) = share(Always::default());
    let catalog = shared(rolling());
    let mut session = landed(&catalog);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    let mut own = Always::default();
    flight.shipyard();
    session.shipyard(&mut own);
    let decline = FlightCommand::DeclineShip(ShipId(129));
    assert_eq!(
        flight.apply(decline.clone()),
        by_hand(&mut session, &catalog, decline, &mut own)
    );
    assert_eq!(flight.shipyard(), session.shipyard(&mut own));
    assert_eq!(always.borrow().asked, own.asked, "on the flight's chance");
}

#[test]
fn an_outfits_on_purchase_runs_on_the_flights_chance() {
    let mut catalog = outfitting();
    catalog.outfits[0].on_purchase = nova_sim::Script::parse("R(b1 b2)");
    let catalog = shared(catalog);
    let mut session = landed(&catalog);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(rolling_one()));
    let order = FlightCommand::Outfit {
        order: BUY_TANK,
        count: 1,
    };
    assert_eq!(
        flight.apply(order.clone()),
        by_hand(&mut session, &catalog, order, &mut RollsOne)
    );
    assert_eq!(flight.session(), Ok(&session), "rolled as by hand");
    let differently =
        |session: &Session| session.control_bit(nova_sim::Bit::new(1).expect("a bit"));
    let mut never = landed(&catalog);
    let order = FlightCommand::Outfit {
        order: BUY_TANK,
        count: 1,
    };
    by_hand(&mut never, &catalog, order, &mut NeverFires);
    assert_ne!(
        differently(&session),
        differently(&never),
        "the roll counts"
    );
}

#[test]
fn a_ships_on_purchase_runs_on_the_flights_chance() {
    let mut catalog = shipbuying();
    catalog.ships[0].on_purchase = nova_sim::Script::parse("R(b3 b4)");
    let catalog = shared(catalog);
    let mut session = landed(&catalog);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(rolling_one()));
    let buy = || FlightCommand::BuyShip {
        ship: ShipId(129),
        name: "Kestrel".to_owned(),
    };
    assert_eq!(
        flight.apply(buy()),
        by_hand(&mut session, &catalog, buy(), &mut RollsOne)
    );
    assert_eq!(flight.session(), Ok(&session), "rolled as by hand");
    let bit = |session: &Session| session.control_bit(nova_sim::Bit::new(3).expect("a bit"));
    let mut never = landed(&catalog);
    by_hand(&mut never, &catalog, buy(), &mut NeverFires);
    assert_ne!(bit(&session), bit(&never), "the roll counts");
}

// The bar.

#[test]
fn hiring_goes_through_the_session_on_the_flights_chance() {
    let (always, chance) = share(Always::default());
    let catalog = shared(hiring());
    let mut session = landed(&catalog);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    let mut own = Always::default();
    let list = session.escorts_for_hire(&mut own);
    assert!(list.is_some(), "a bar");
    assert_eq!(flight.escorts_for_hire(), list);
    let hire = FlightCommand::Hire(ShipId(129));
    assert_eq!(
        flight.apply(hire.clone()),
        by_hand(&mut session, &catalog, hire, &mut own)
    );
    assert_eq!(always.borrow().asked, own.asked, "on the flight's chance");
    assert_eq!(flight.session(), Ok(&session));
}

#[test]
fn the_bars_news_is_drawn_through_the_session_on_the_flights_chance() {
    let (always, chance) = share(Always::default());
    let strings = FakeCatalog {
        strings: vec![
            (8100, vec!["C1".to_owned(), "C2".to_owned()]),
            (8101, vec!["G1".to_owned(), "G2".to_owned()]),
        ],
        ..catalog()
    };
    let setup = || SessionSetup {
        strings: Some(Rc::new(FakeCatalog {
            strings: strings.strings.clone(),
            ..catalog()
        })),
        ..SessionSetup::default()
    };
    let catalog = shared(hiring());
    let mut session = landed(&catalog).set_up(setup());
    let mut flight = flight_of(&catalog, landed(&catalog), rolling_on(chance));
    flight.set_up(setup());
    let mut own = Always::default();
    let expected = by_hand(&mut session, &catalog, FlightCommand::OpenBar, &mut own);
    assert_eq!(flight.apply(FlightCommand::OpenBar), expected);
    assert!(session.news().is_some(), "news drawn");
    assert_eq!(flight.news(), session.news());
    assert_eq!(flight.news_picture(), session.news_picture());
    assert_eq!(always.borrow().drawn, own.drawn, "on the flight's chance");
}

/// Terms of a fee of 7 and a wage of 3 for every ship.
#[derive(Debug)]
struct Sevens;

impl nova_sim::HireTerms for Sevens {
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
    // The edge sets the flight up with them (`open_flight`); how the
    // session prices a hire is nova-sim's.
    let (_, chance) = share(Always::default());
    let catalog = shared(hiring());
    let setup = || SessionSetup {
        hire_terms: Rc::new(Sevens),
        control_bits: Rc::new(Nothing),
        ..SessionSetup::default()
    };
    let mut flight = flight_of(&catalog, landed(&catalog), rolling_on(chance));
    flight.set_up(setup());
    let expected = landed(&catalog)
        .set_up(setup())
        .escorts_for_hire(&mut Always::default());
    let without = landed(&catalog).escorts_for_hire(&mut Always::default());
    assert_ne!(expected, without, "precondition: the ports change the list");
    let list = flight.escorts_for_hire();
    assert_eq!(list, expected, "as set up by hand");
    let row = list
        .as_ref()
        .and_then(|list| list.row(ShipId(129)))
        .expect("listed: no Flags3 hides it");
    assert_eq!((row.fee, row.wage), (7, 3), "the terms given");
}

// The missions.

/// [`catalog`] with Earth at the centre and `missions` on offer.
fn errands(missions: Vec<MissionRecord>) -> FakeCatalog {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = missions;
    errands
}

/// `catalog`'s pilot landed at Earth with its landing settled through the
/// port, on `chance`.
fn landed_and_settled(
    catalog: &Rc<FakeCatalog>,
    chance: SharedChance,
) -> SessionFlight<FakeCatalog> {
    let mut flight = flight_of(catalog, landed(catalog), rolling_on(chance));
    flight.apply(FlightCommand::SettleScript);
    flight
}

#[test]
fn the_offer_names_are_the_sessions() {
    let catalog = shared(errands(vec![errand(130), errand(131)]));
    let flight = landed_and_settled(&catalog, SharedChance::default());
    let session = flight.session().expect("flying");
    let munged = flight.offer_text(MissionId(130), "Run <PAY>");
    assert!(munged.is_some(), "on offer");
    assert_eq!(munged, session.offer_text(MissionId(130), "Run <PAY>"));
}

#[test]
fn an_offers_refuse_button_is_the_sessions() {
    let catalog = shared(errands(vec![MissionRecord {
        avail_loc: SPACEPORT,
        ..errand(130)
    }]));
    let mut flight = landed_and_settled(&catalog, SharedChance::default());
    flight.apply(FlightCommand::EnterOfferVenue(SPACEPORT));
    assert!(matches!(
        flight.apply(FlightCommand::NextOffer(SPACEPORT)),
        Answer::Offer(Some(VenueOffer::Prompt(_)))
    ));
    let session = flight.session().expect("flying");
    for mission in [MissionId(130), MissionId(131)] {
        assert_eq!(
            flight.refusable_offer_dialog(mission),
            session.refusable_offer_dialog(mission),
            "{mission:?}"
        );
    }
    assert!(flight.refusable_offer_dialog(MissionId(130)), "offered");
}

#[test]
fn the_bars_offers_go_through_the_session_on_the_flights_chance() {
    let catalog = shared(errands(vec![MissionRecord {
        avail_loc: 1,
        ..errand(141)
    }]));
    let (script, chance) = scripted(&[]);
    let mut flight = landed_and_settled(&catalog, chance);
    let mut session = flight.session().expect("flying").clone();
    script.borrow_mut().asked.clear();
    let (own, mut own_chance) = scripted(&[]);
    for command in [
        FlightCommand::EnterOfferVenue(1),
        FlightCommand::NextOffer(1),
    ] {
        assert_eq!(
            flight.apply(command.clone()),
            by_hand(&mut session, &catalog, command, &mut own_chance)
        );
    }
    assert_eq!(
        flight.offer_gap(),
        nova_sim::mission::offer_gap(&mut own_chance)
    );
    assert!(!own.borrow().asked.is_empty(), "the offer and the gap draw");
    assert_eq!(script.borrow().asked, own.borrow().asked);
}

#[test]
fn the_flights_mission_systems_are_its_sessions() {
    let mut catalog = catalog();
    catalog.missions = vec![MissionRecord {
        travel_stel: 140,
        flags: nova_sim::MARKS_DESTINATION,
        ..errand(130)
    }];
    let catalog = shared(catalog);
    let mut session = SessionBuilder::flying(&*catalog).build();
    session.run_set(
        &nova_sim::SetExpr::parse("S130").expect("parses"),
        &mut NeverFires,
    );
    let mut flight = flight_of(&catalog, session, Collaborators::default());
    flight.apply(FlightCommand::SettleScript);
    let session = flight.session().expect("flying");
    assert!(
        !session.map_mission_systems().is_empty(),
        "a mission arrowed"
    );
    assert_eq!(flight.map_mission_systems(), session.map_mission_systems());
}

// Entering play.

#[test]
fn resuming_faces_the_heading_drawn_on_the_flights_chance() {
    let (script, chance) = scripted(&[123]);
    let catalog = shared(catalog());
    let session = SessionBuilder::flying(&*catalog)
        .named("Ada")
        .over(StellarId(128))
        .build();
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    flight.resume();
    let mut by_hand = session;
    let (own, mut own_chance) = scripted(&[123]);
    by_hand.resume(&mut own_chance);
    assert_eq!(flight.session(), Ok(&by_hand));
    assert_eq!(script.borrow().asked, own.borrow().asked, "on its chance");
    assert!(!own.borrow().asked.is_empty());
}

#[test]
fn a_failed_flight_resumes_nothing() {
    let (script, chance) = scripted(&[123]);
    let mut broken = SessionFlight::<FakeCatalog>::new(Err("no chär".to_owned()))
        .with_collaborators(rolling_on(chance));
    broken.resume();
    broken.open_on_main_screen();
    broken.open_at_launch();
    assert_eq!(broken.pilot(), None);
    assert!(script.borrow().asked.is_empty(), "nothing drawn");
}

/// A catalog of mission 130 with two auxiliary ships, and a session over
/// Earth holding it in slot 0 as its accept leaves it.
fn aux_errand() -> (Rc<FakeCatalog>, Session) {
    let catalog = shared(errands(vec![MissionRecord {
        aux: nova_sim::catalog::AuxShip {
            count: 2,
            dude: Some(nova_sim::catalog::DudeId(200)),
            syst: -1,
        },
        ..errand(130)
    }]));
    let mut flight = landed_and_settled(&catalog, SharedChance::default());
    assert_eq!(
        flight.apply(FlightCommand::AcceptMission(MissionId(130))),
        Answer::Accepted(Ok(0))
    );
    let accepted = flight
        .pilot()
        .expect("flying")
        .missions()
        .get(0)
        .expect("held")
        .clone();
    let session = SessionBuilder::flying(&*catalog)
        .over(StellarId(128))
        .with_mission(0, accepted)
        .build();
    (catalog, session)
}

#[test]
fn opening_on_the_main_screen_restarts_the_aux_countdown_on_the_flights_chance() {
    let (catalog, session) = aux_errand();
    let (script, chance) = scripted(&[9]);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    flight.open_on_main_screen();
    let mut by_hand = session;
    let (own, mut own_chance) = scripted(&[9]);
    by_hand.restart_aux_from_main_screen(&mut own_chance);
    assert_eq!(flight.session(), Ok(&by_hand));
    assert_eq!(script.borrow().asked, own.borrow().asked, "on its chance");
    assert!(!own.borrow().asked.is_empty());
}

#[test]
fn opening_at_launch_restarts_the_aux_countdown_on_the_flights_chance() {
    let (catalog, session) = aux_errand();
    let (script, chance) = scripted(&[4]);
    let mut flight = flight_of(&catalog, session.clone(), rolling_on(chance));
    flight.open_at_launch();
    let mut by_hand = session;
    let (own, mut own_chance) = scripted(&[4]);
    by_hand.open_at_launch(&mut own_chance);
    assert_eq!(flight.session(), Ok(&by_hand));
    assert_eq!(script.borrow().asked, own.borrow().asked, "on its chance");
    assert!(!own.borrow().asked.is_empty());
}

// The pilot, its load and its transactions.

#[test]
fn the_pilot_panel_is_the_sessions_and_none_for_a_session_that_failed() {
    let catalog = shared(catalog());
    let flight = flight_of(&catalog, landed(&catalog), Collaborators::default());
    let session = flight.session().expect("flying");
    assert_eq!(flight.pilot_panel(), Some(session.pilot_panel(&*catalog)));
    assert_eq!(
        flight.player_info(),
        Some(session.player_info(&*catalog)),
        "the player info too"
    );
    let broken = SessionFlight::<FakeCatalog>::new(Err("no chär".to_owned()));
    assert_eq!(broken.pilot_panel(), None);
    assert_eq!(broken.player_info(), None);
}

/// A pilot named Ada docked at Earth, with her stellar rewritten to 999,
/// which no system lists.
fn unlisted_pilot() -> Pilot {
    let pilot = landed(&catalog()).pilot().clone();
    let text = nova_sim::save::encode(&pilot).replace("\"stellar\": 128", "\"stellar\": 999");
    let pilot = nova_sim::save::decode(&text).expect("a pilot");
    assert_eq!(pilot.stellar(), Some(StellarId(999)));
    pilot
}

#[test]
fn a_loaded_flight_reports_a_repaired_load() {
    let catalog = shared(catalog());
    let docked = landed(&catalog).pilot().clone();
    // That the rules given reach the load is the edge's check
    // (`build::edge_checks::a_loaded_flight_reads_the_rules_it_is_given`).
    let engine = Rulebook::default().with_override(RuleKey::UnlistedStellar, RuleSource::Engine);
    let mut repaired = false;
    for rules in [Rulebook::default(), engine] {
        for pilot in [unlisted_pilot(), docked.clone()] {
            let flight = SessionFlight::load(&catalog, pilot.clone(), &rules);
            let session = Session::load(&*catalog, pilot, &rules).expect("loads");
            assert_eq!(flight.load_repaired(), session.load_repaired());
            assert_eq!(flight.pilot(), Some(session.pilot()), "as loaded by hand");
            repaired |= session.load_repaired();
        }
    }
    assert!(repaired, "precondition: a load the session repaired");
}

#[test]
fn a_transaction_is_the_sessions_while_landed_and_refused_in_flight() {
    let catalog = shared(catalog());
    for docked in [true, false] {
        let session = if docked {
            landed(&catalog)
        } else {
            SessionBuilder::flying(&*catalog).build()
        };
        let mut flight = flight_of(&catalog, session.clone(), Collaborators::default());
        let mut by_hand = session;
        assert_eq!(
            flight.transact(|pilot| pilot.set_cash(5)),
            by_hand.transact(|pilot| pilot.set_cash(5))
        );
        assert_eq!(flight.session(), Ok(&by_hand));
    }
}

// The flight's other answers.

#[test]
fn the_adapter_drains_what_the_session_raised_once() {
    let session = Session::start(&catalog()).expect("starts");
    let mut flight = SessionFlight::<FakeCatalog>::from(session);
    flight
        .session_mut()
        .expect("started")
        .select_stellar(StellarPick::Slot(0));
    let mut by_hand = flight.session().expect("started").clone();
    let mut expected = Vec::new();
    by_hand.drain_events(&mut expected);
    assert!(
        !expected.is_empty(),
        "precondition: the selection raised some"
    );
    let mut into = Vec::new();
    flight.drain(&mut into);
    assert_eq!(into, expected, "the session's");
    let mut again = Vec::new();
    flight.drain(&mut again);
    let mut expected_again = Vec::new();
    by_hand.drain_events(&mut expected_again);
    assert_eq!(again, expected_again, "drained once, as the session drains");
}

#[test]
fn the_flights_opening_names_and_its_frames_ship_name_and_date_are_its_sessions() {
    let mut data = trading();
    data.ships = shipbuying().ships;
    data.ships[0].long_name = "Federation Fast".to_owned();
    data.outfits = vec![holding(200, ROCKET)];
    data.junk = vec![JunkRecord {
        id: nova_sim::JunkId(150),
        name: "Bio-weapons".to_owned(),
        abbrev: "Bio".to_owned(),
        base_price: 1000,
        sold_at: Vec::new(),
        bought_at: Vec::new(),
        buy_on: nova_sim::Test::default(),
        sell_on: nova_sim::Test::default(),
        flags: 0,
        scan_mask: 0,
        lc_name: "bio-weapons".to_owned(),
    }];
    let session = SessionBuilder::flying(&data)
        .with_ship_name("Kestrel")
        .build();
    let flight = SessionFlight::<FakeCatalog>::from(session);
    let session = flight.session().expect("flying");
    let names = flight.opening().names;
    assert_eq!(names, session.opening().names, "the session's");
    let ship = ShipId(129);
    assert_eq!(names.ship(ship), Some("Fast"));
    assert_eq!(names.ship_long(ship), Some("Federation Fast"));
    assert_eq!(names.outfit(OutfitId(200)), Some("Gun 200"));
    assert_eq!(
        names.outfit_lc(OutfitId(200)),
        Some(("gun 200", "guns 200"))
    );
    assert_eq!(names.good(Good::Commodity(0)), Some("Food"));
    assert_eq!(names.junk_lc(nova_sim::JunkId(150)), Some("bio-weapons"));
    let mut frame = FlightFrame::default();
    flight.frame(&mut frame);
    assert_eq!(frame.player_ship_name.as_deref(), Some("Kestrel"));
    assert_eq!(frame.long_date, session.long_date_text());
    assert!(!frame.long_date.is_empty(), "the date is said");
}

/// A shared catalog shows in the flight's `Debug`.
#[test]
fn a_shared_catalog_shows_in_its_flights_debug() {
    let mut flight: SessionFlight<()> = SessionFlight::new(Err("no chär".to_owned()));
    assert!(format!("{flight:?}").contains("catalog: None"));
    flight.share_catalog(Rc::new(catalog()));
    assert!(format!("{flight:?}").contains("catalog: Some(SharedCatalog)"));
}

/// The flight's steps are its session's, handed the catalog the view
/// shared: one without it runs none.
#[test]
fn the_flights_steps_are_its_sessions_once_its_catalog_is_shared() {
    let session = Session::start(&catalog()).expect("starts");
    let mut flight: SessionFlight<()> = SessionFlight::from(session.clone());
    let mut report = StepReport::default();
    flight.step(&StepInput::default(), 2, &mut report);
    assert_eq!(report, StepReport::default(), "no catalog shared");
    assert_eq!(flight.session().expect("flying"), &session, "untouched");
    flight.share_catalog(Rc::new(catalog()));
    flight.step(&StepInput::default(), 2, &mut report);
    assert_eq!(report.steps.len(), 2);
    let mut by_hand = session;
    let behaviour = nova_sim::NovaAi::default();
    let mut chance = SharedChance::default();
    let mut collab = nova_sim::StepCollaborators {
        catalog: &catalog(),
        behaviour: &behaviour,
        rules: Rules::default(),
        chance: &mut chance,
    };
    let mut expected = StepReport::default();
    by_hand.step(&StepInput::default(), 2, &mut collab, &mut expected);
    assert_eq!(report, expected);
    assert_eq!(flight.session().expect("flying"), &by_hand);
}

/// The flight's floating-map answers are its session's, through the
/// adapter.
#[test]
fn the_flights_map_answers_are_its_sessions() {
    let catalog = shared(catalog());
    let mut flight = SessionFlight::start(&catalog);
    flight.apply(FlightCommand::CycleMapSelection);
    let session = flight.session().expect("flying");
    let preselection = session.floating_preselection();
    assert!(preselection.is_some(), "the cycle noted a slot");
    assert_eq!(flight.floating_preselection(), preselection);
    let places = session.floating_map_places();
    assert!(!places.links.is_empty());
    assert_eq!(flight.floating_map_places(), places);
}

#[test]
fn a_real_flights_coming_traffic_is_the_types_its_session_brings() {
    let catalog = shared(trafficked(&[130, 131], 3, 129, 1));
    let mut flight = SessionFlight::start(&catalog);
    let session = flight.session().expect("flying");
    let coming = session.coming_traffic_ships(&*catalog);
    assert!(!coming.is_empty(), "traffic to come");
    assert_eq!(
        flight.apply(FlightCommand::ComingTraffic),
        Answer::ComingTraffic(coming),
        "asked as a command, over the catalog shared"
    );
}

#[test]
fn a_real_flights_frame_holds_its_sessions_death_descs() {
    let mut sabotaged = multi_jumping();
    sabotaged.outfits[0].mods[0] = (nova_sim::SABOTAGE_KILL, 5000);
    let catalog = shared(sabotaged);
    let flight = SessionFlight::start(&catalog);
    let descs = flight.session().expect("flying").sabotage_death_descs();
    assert!(!descs.is_empty(), "sabotaged");
    let mut frame = FlightFrame::default();
    flight.frame(&mut frame);
    assert_eq!(frame.death_descs, descs);
}

#[test]
fn a_real_flights_weapons_are_its_sessions() {
    let catalog = shared(armed(0, &[BLASTER]));
    let flight = SessionFlight::start(&catalog);
    let ids = flight.session().expect("flying").weapon_ids();
    assert!(!ids.is_empty(), "armed");
    assert_eq!(flight.opening().weapons, ids, "the opening lists them");
    assert_eq!(flight.opening().failure, None, "started");
}

// The comm dialog.

#[test]
fn the_screens_hail_options_are_listed() {
    // The adapter hands its hail options to the hail: the dialog lists
    // those given, here none.
    let catalog = shared(FakeCatalog {
        strings: vec![(3000, comm_strings())],
        ..boardable()
    });
    let (_, chance) = scripted(&placed(750, 750, 0));
    let session = SessionBuilder::flying(&*catalog).build();
    let mut flight = flight_of(
        &catalog,
        session,
        Collaborators {
            hail_options: HailOptions::empty(),
            ..on(chance, Rc::new(Still))
        },
    );
    step(&mut flight, &StepInput::default());
    flight.apply(FlightCommand::SelectTarget(nova_sim::TargetPick::Next));
    assert!(matches!(
        flight.apply(FlightCommand::Hail { option: false }),
        Answer::Hailed(_)
    ));
    let hail = flight.hailing().expect("hailed");
    assert_eq!(hail.options, []);
}

// The fight's collaborators.

#[test]
fn by_default_npcs_fight_by_novas_ai_and_crimes_by_novas_law() {
    // How Nova's AI and law decide is nova-sim's.
    let given = Collaborators::default();
    let debug = |collab: &Collaborators| {
        [
            format!("{:?}", collab.behaviour),
            format!("{:?}", collab.defence_rule),
            format!("{:?}", collab.law),
            format!("{:?}", collab.boarding_rule),
        ]
    };
    assert_eq!(
        debug(&given),
        [
            format!("{:?}", nova_sim::NovaAi::default()),
            format!("{:?}", nova_sim::Allegiance),
            format!("{:?}", NovaLaw::default()),
            format!("{:?}", NovaBoarding::default()),
        ]
    );
    let flight = SessionFlight::<()>::new(Err("no chär".to_owned()));
    assert_eq!(
        [
            format!("{:?}", flight.behaviour),
            format!("{:?}", flight.defence_rule),
            format!("{:?}", flight.law),
            format!("{:?}", flight.boarding_rule),
        ],
        debug(&given),
        "a flight's own"
    );
}

/// Records each crime, and changes nothing.
#[derive(Debug, Default)]
struct Witness {
    seen: RefCell<Vec<nova_sim::Crime>>,
}

impl nova_sim::LegalCode for Witness {
    fn penalties(
        &self,
        crime: nova_sim::Crime,
        _victim: Option<GovtId>,
        _govts: &nova_sim::Governments,
    ) -> Vec<(GovtId, i32)> {
        self.seen.borrow_mut().push(crime);
        Vec::new()
    }

    fn system_penalties(
        &self,
        _crime: nova_sim::Crime,
        _victim: Option<GovtId>,
        _from: nova_sim::SystemId,
        _map: &dyn nova_sim::CrimeMap,
        _govts: &nova_sim::Governments,
    ) -> Vec<(nova_sim::SystemId, f32)> {
        Vec::new()
    }
}

#[test]
fn the_flight_convicts_by_the_law_it_is_given() {
    // The step hands the fight the flight's law, which hears what the
    // session's own step tells the law it is handed; the board's is
    // `Session::apply`'s (phase 15's). Which crimes the fight commits is
    // nova-sim's.
    let witness = Rc::new(Witness::default());
    // 100 above, facing away: it flies on up the line of fire.
    let draws = placed(750, 650, 0);
    let (_, chance) = scripted(&draws);
    let catalog = shared(damaging());
    let session = SessionBuilder::flying(&*catalog).build();
    let mut flight = flight_of(
        &catalog,
        session.clone(),
        Collaborators {
            law: witness.clone(),
            ..on(chance, Rc::new(Still))
        },
    );
    let heard = Witness::default();
    let (_, mut own_chance) = scripted(&draws);
    let mut by_hand = session;
    let firing = StepInput {
        fire: (true, false),
        ..StepInput::default()
    };
    let inputs = std::iter::once(StepInput::default()).chain(std::iter::repeat_n(firing, 120));
    for input in inputs {
        step(&mut flight, &input);
        let mut collab = nova_sim::StepCollaborators {
            catalog: &*catalog,
            behaviour: &Still,
            rules: Rules {
                disable: &nova_sim::NovaDisable,
                defence: &nova_sim::Allegiance,
                law: &heard,
            },
            chance: &mut own_chance,
        };
        by_hand.step(&input, 1, &mut collab, &mut StepReport::default());
        if !witness.seen.borrow().is_empty() {
            break;
        }
    }
    assert!(!witness.seen.borrow().is_empty(), "a crime heard");
    assert_eq!(*witness.seen.borrow(), *heard.seen.borrow(), "as by hand");
}

/// Says no missile is hostile, counting the times it is asked.
#[derive(Debug, Default)]
struct Unalarmed {
    asked: std::cell::Cell<usize>,
}

impl nova_sim::PointDefenceRule for Unalarmed {
    fn hostile(
        &self,
        _defender: nova_sim::combat::defence::Side,
        _firer: nova_sim::combat::defence::Side,
        _govts: &nova_sim::Governments,
    ) -> bool {
        self.asked.set(self.asked.get() + 1);
        false
    }
}

/// Every NPC idles, targets the player and holds its trigger.
#[derive(Debug)]
struct Attacking;

impl Behaviour for Attacking {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, _npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> nova_sim::Trigger {
        nova_sim::Trigger {
            primary: true,
            secondary: None,
            only: None,
            bays: false,
            bays_after: false,
            point_defence: false,
            keep: None,
        }
    }

    fn target(&self, _npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> Option<ShipRef> {
        Some(ShipRef::Player)
    }
}

#[test]
fn the_fight_asks_the_point_defence_rule_given_each_step_in_flight() {
    // Which missiles the rule is asked of, and when, is nova-sim's; the
    // reading that asks it is set explicitly.
    let rule = Rc::new(Unalarmed::default());
    let (_, chance) = scripted(&placed(750, 550, 180));
    let catalog = shared(defending());
    let session = SessionBuilder::flying(&*catalog).build();
    let mut flight = flight_of(
        &catalog,
        session,
        Collaborators {
            defence_rule: rule.clone(),
            ..on(chance, Rc::new(Attacking))
        },
    );
    flight.set_up(SessionSetup {
        rules: Rulebook::default().with_override(RuleKey::PdHostility, RuleSource::Bible),
        ..SessionSetup::default()
    });
    for _ in 0..10 {
        step(&mut flight, &StepInput::default());
        if rule.asked.get() > 0 {
            break;
        }
    }
    assert!(rule.asked.get() > 0, "the rule given, asked by the step");
}

/// The port's contract holds of `flight` at `at`, checked on a copy, as
/// the check sends commands.
fn meets_the_contract(flight: &SessionFlight<FakeCatalog>, at: &str) {
    assert_eq!(
        check_contract(&mut flight.clone(), &flight.opening()),
        Ok(()),
        "{at}"
    );
}

/// Phase 15's contract in states: a real flight meets the port's
/// contract through its player's explosion, the hold after it, the hold's
/// expiry, the fade and the end of play, reached through the port alone.
#[test]
fn a_real_flight_meets_the_contract_through_the_hold_its_expiry_and_the_fade() {
    let (_, chance) = scripted(&placed(750, 650, 180));
    let catalog = shared(killed_in(1));
    let session = SessionBuilder::flying(&*catalog).build();
    let mut flight = flight_of(&catalog, session, on(chance, Rc::new(Firing)));
    let fate = |flight: &SessionFlight<FakeCatalog>| frame_of(flight).fate;
    for _ in 0..120 {
        step(&mut flight, &StepInput::default());
        if fate(&flight) == PlayerFate::Exploded {
            break;
        }
    }
    assert_eq!(fate(&flight), PlayerFate::Exploded);
    meets_the_contract(&flight, "the explosion");
    step(&mut flight, &StepInput::default());
    assert!(matches!(fate(&flight), PlayerFate::Holding { .. }));
    meets_the_contract(&flight, "the hold");
    for _ in 0..=nova_sim::HOLD_TICKS {
        if frame_of(&flight).hold_expired {
            break;
        }
        step(&mut flight, &StepInput::default());
    }
    assert!(frame_of(&flight).hold_expired);
    assert_eq!(fate(&flight), PlayerFate::Holding { ticks_left: 0 });
    meets_the_contract(&flight, "the expiry");
    flight.apply(FlightCommand::BeginFade);
    assert_eq!(fate(&flight), PlayerFate::Fading);
    meets_the_contract(&flight, "the fade");
    flight.apply(FlightCommand::EndPlay);
    assert_eq!(fate(&flight), PlayerFate::GameOver);
    meets_the_contract(&flight, "game over");
}
