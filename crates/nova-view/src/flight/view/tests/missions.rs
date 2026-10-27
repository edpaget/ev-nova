use super::*;
use crate::flight::sim_port::PortSource;

// The mission screens.

/// The adapter's own check
/// (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks): the flight's
/// offer texts are the session's. The `OfferNames` impl over them runs on
/// `FakeFlight` (`tests/faked_missions.rs`).
#[test]
fn the_offer_names_are_the_sessions() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![errand(130), errand(131)];
    let mut view = FlightView::new(errands);
    land_now(&mut view);
    view.settle_script();
    let munged = view.sim.offer_text(MissionId(130), "Run <PAY>");
    assert_eq!(
        munged,
        view.session()
            .ok()
            .and_then(|session| session.offer_text(MissionId(130), "Run <PAY>")),
        "the session's munge"
    );
    assert!(munged.is_some_and(|name| !name.contains("<PAY>")), "filled");
}

/// `base` with asteroid type 0, its sheet `rlëD` 1800, and system 130
/// holding 3 of it.
pub(super) fn rocky(base: FakeCatalog) -> FakeCatalog {
    FakeCatalog {
        asteroid_types: vec![nova_sim::AsteroidTypeRecord {
            kind: 0,
            strength: 50,
            spin_rate: 100,
            frames: 36,
            frame_size: Some((40, 40)),
            ..nova_sim::AsteroidTypeRecord::default()
        }],
        asteroid_fields: vec![nova_sim::AsteroidFieldRecord {
            system: SystemId(130),
            count: 3,
            types: 0x0001,
        }],
        ..base
    }
}

/// Inspects the player on its first decision; then it has looked the
/// player over, and idles.
#[derive(Debug, Default)]
struct Inspector(std::cell::Cell<bool>);

impl Behaviour for Inspector {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        if self.0.replace(true) {
            Goal::Idle
        } else {
            Goal::Inspect(ShipRef::Player)
        }
    }
}

/// A flight in Sol with 1200 credits, its ship named Kestrel, with
/// mission 130's 4 tons
/// of food (`ScanMask` 0x0004, `Flags` `flags`) aboard in slot 0, and
/// one interceptor (ship 129) of a government scanning for 0x0004 and
/// fining `fine`, placed on the ship, that inspects it, its scan
/// drawing 0.
fn smuggling(flags: u16, fine: i16) -> View {
    let mut catalog = trafficked(&[130], 1, 129, 4);
    catalog.character = Ok(CharacterStart {
        cash: 1200,
        ..catalog.character.expect("a chär")
    });
    catalog.dudes[0].1.govt = Some(GovtId(140));
    catalog.govts = vec![nova_sim::GovtRecord {
        id: GovtId(140),
        name: String::new(),
        flags: 0,
        flags2: 0,
        crime_tol: 0,
        initial_rec: 0,
        penalties: nova_sim::Penalties {
            smuggle: 5,
            ..nova_sim::Penalties::default()
        },
        max_odds: 100,
        classes: [-1; 4],
        allies: [-1; 4],
        enemies: [-1; 4],
        comm_name: String::new(),
        medium_name: String::new(),
        scan_mask: 0x0004,
        scan_fine: fine,
        news_pic: -1,
        require: 0,
        inh_jam: [0; 4],
        skill_mult: 0,
    }];
    catalog.missions = vec![MissionRecord {
        cargo_type: 0,
        cargo_qty: 4,
        pickup_mode: 0,
        scan_mask: 0x0004,
        flags,
        ..errand(130)
    }];
    // Mission 130 active in slot 0, its 4 tons of food aboard.
    let smuggled = nova_sim::mission::ActiveMission {
        mission: MissionId(130),
        destination: None,
        return_to: None,
        cargo_type: Some(0),
        cargo_tons: 4,
        pay: 0,
        days_left: None,
        deadline: None,
        reached: false,
        goals_met: true,
        aboard: true,
        failed: None,
        ships: nova_sim::mission::ships::SpecialShips::default(),
        aux: nova_sim::mission::aux_ships::AuxShips::default(),
    };
    let session = SessionBuilder::flying(&catalog)
        .named("Ada")
        .with_ship_name("Kestrel")
        .with_mission(0, smuggled)
        .build();
    let mut draws = placed(750, 750, 0).to_vec();
    draws.push(0);
    let (_, chance) = scripted(&draws);
    FlightView::from_session(catalog, session.into())
        .with_chance(chance)
        .with_behaviour(Rc::new(Inspector::default()))
}

#[test]
fn a_scan_in_the_warps_lead_in_is_said_under_the_streak() {
    let done = Rc::new(std::cell::Cell::new(false));
    let mut view =
        smuggling(0, 500).with_behaviour(Rc::new(InspectingUntil(Rc::clone(&done), Goal::Idle)));
    plot(&mut view, 131);
    ready_to_jump(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.jump_effect().is_some(), "streaking on J");
    assert_eq!(view.pilot().expect("flying").cash(), 1200, "not yet");
    done.set(true);
    view.tick(TICK);
    assert_eq!(view.pilot().expect("flying").cash(), 700, "convicted");
    assert_eq!(
        view.message(),
        Some(
            "Ship 129:  Kestrel, your attempt to smuggle illegal cargo through this \
             system has been detected. You have been fined 500 credits."
        ),
        "said under the streak"
    );
    assert!(view.jump_effect().is_some());
}

#[test]
fn a_scan_that_fails_a_mission_says_so_on_the_message_line() {
    let mut view = smuggling(0x0020, 500);
    for _ in 0..3 {
        view.tick(TICK);
    }
    let pilot = view.pilot().expect("flying");
    assert_eq!(pilot.missions().iter().count(), 0, "failed at once");
    assert_lasts(&mut view, MISSION_SCANNED, 400);
    assert_eq!(
        MISSION_SCANNED,
        "Your ship has been scanned - mission failed."
    );
}

#[test]
fn a_scan_that_finds_smuggling_names_the_scanner_and_the_ship_and_the_fine() {
    let mut view = smuggling(0, 500);
    assert_eq!(view.pilot().expect("flying").cash(), 1200);
    for _ in 0..3 {
        view.tick(TICK);
    }
    assert_eq!(view.pilot().expect("flying").cash(), 700);
    assert_eq!(
        view.message(),
        Some(
            "Ship 129:  Kestrel, your attempt to smuggle illegal cargo through this \
             system has been detected. You have been fined 500 credits."
        )
    );
}

/// The adapter's own check (rdm:phase/view-sim-seam/phase-11-spaceport-on-port):
/// a spaceport offer's dialog has a refuse button as the session says.
/// What the view redraws for it runs on `FakeFlight`
/// (`tests/faked_missions.rs`).
#[test]
fn an_offers_refuse_button_is_the_sessions() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![MissionRecord {
        avail_loc: SPACEPORT,
        ..errand(130)
    }];
    let mut view = FlightView::new(errands);
    land_now(&mut view);
    view.settle_script();
    view.enter_offer_venue(SPACEPORT);
    assert!(matches!(
        view.next_offer(SPACEPORT),
        Some(VenueOffer::Prompt(_))
    ));
    assert!(view.sim.refusable_offer_dialog(MissionId(130)));
    assert!(
        !view.sim.refusable_offer_dialog(MissionId(131)),
        "not offered"
    );
}

/// Kept real for phase 12 (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks):
/// the player info's Jettison says what went in the session's words
/// (`FlightView::jettison_all` over `FlightView::WORDS`).
#[test]
fn the_player_infos_jettison_says_the_sessions_words() {
    let mut view = FlightView::new(trading());
    land_now(&mut view);
    view.settle_script();
    assert_eq!(view.trade(BUY_FOOD), Ok(10));
    view.take_off().expect("takes off");
    view.settle_script();
    view.message = None;
    view.jettison_all();
    assert_eq!(view.message(), Some(CARGO_JETTISONED));
}

/// The adapter's own check (rdm:phase/view-sim-seam/phase-11-spaceport-on-port):
/// it hands the flight's chance to a command, the next offer, and to the
/// offer gap. What the view does with the answers runs on `FakeFlight`
/// (`tests/faked_missions.rs`).
#[test]
fn the_bars_offers_go_through_the_session_on_the_flights_chance() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![MissionRecord {
        avail_loc: 1,
        ..errand(141)
    }];
    let (script, chance) = scripted(&[]);
    let mut view = FlightView::new(errands).with_chance(chance);
    assert_eq!(view.next_offer(1), None, "in flight");
    land_now(&mut view);
    view.settle_script();
    script.borrow_mut().asked.clear();
    script.borrow_mut().draws.push_back(7);
    view.enter_offer_venue(1);
    let Some(VenueOffer::Prompt(prompt)) = view.next_offer(1) else {
        panic!("an offer");
    };
    assert_eq!(prompt.mission, MissionId(141));
    assert_eq!(view.offer_gap(), nova_sim::mission::screens::mac_ticks(37));
    assert_eq!(script.borrow().asked, [30]);
}

/// [`boardable`] with ship 130 of `Cost` 10,000 (a wage of 100), and
/// a pilot holding `cash` whose fleet is one hired ship 130, resumed
/// over stellar 128 and landed there after its flight's first tick when
/// `docked`, the landing not yet taken; else its flight, its first
/// tick done.
fn paying(cash: i64, docked: bool) -> View {
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.cost = 10_000;
        }
    }
    let mut builder = SessionBuilder::flying(&catalog)
        .named("Payer")
        .with_credits(cash)
        .with_escorts(vec![escort(130, false, Some(100))]);
    if docked {
        builder = builder.over(StellarId(128));
    }
    let session = builder.build();
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::from_session(catalog, session.into())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    if docked {
        land_now(&mut view);
    }
    view
}

#[test]
fn an_escort_left_unpaid_by_a_jump_defects_and_the_flight_says_so() {
    let mut view = paying(50, false);
    assert_eq!(view.pilot().expect("flying").escorts().len(), 1);
    jump_to_alpha(&mut view);
    let said = format!(
        "{}  {DEFECTED_ONE}",
        "Jumping into the Alpha Centauri system on June 24th, 1177 NC."
    );
    assert_eq!(view.message(), Some(said.as_str()), "after the arrival");
    assert_eq!(view.pilot().expect("flying").escorts(), []);
    let mut paid = paying(100, false);
    jump_to_alpha(&mut paid);
    assert_eq!(
        paid.message(),
        Some("Jumping into the Alpha Centauri system on June 24th, 1177 NC."),
        "the arrival alone"
    );
    assert_eq!(paid.pilot().map(Pilot::cash), Some(0));
}

/// The greeting from Earth on the take-off of a docked [`paying`]
/// flight, its lead the one the fixture's scripted draws leave.
const PAYING_GREETING: &str = "Departing Earth on June 24th, 1177 NC.";

#[test]
fn by_the_engine_an_escort_left_unpaid_at_take_off_defects_and_the_flight_says_so() {
    let mut view = paying(50, true);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    assert_eq!(view.take_off(), Some(StellarId(128)));
    let said = format!("{PAYING_GREETING}  {DEFECTED_ONE}");
    assert_eq!(view.message(), Some(said.as_str()), "after the greeting");
    assert_eq!(view.pilot().expect("flying").escorts(), []);
    let mut kept = paying(50, true)
        .with_rules(Rulebook::default().with_override(RuleKey::TakeOffPay, RuleSource::Bible));
    kept.take_off().expect("took off");
    assert_eq!(kept.message(), Some(PAYING_GREETING), "the greeting alone");
    assert_eq!(kept.pilot().expect("flying").escorts().len(), 1);
}

#[test]
fn fighters_abandoned_and_escorts_defected_on_one_arrival_are_said_together() {
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.escort_type = 0;
            record.fields.fuel = 0;
        }
    }
    let fleet = vec![
        escort(130, true, None),
        escort(130, true, None),
        escort(129, false, Some(1500)),
    ];
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = escorted_flight(catalog, "Carrier", fleet)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    jump_to_alpha(&mut view);
    assert_eq!(
        view.message(),
        Some(
            "Jumping into the Alpha Centauri system on June 24th, 1177 NC.  (Two fighters abandoned)  Due to lack of pay, one of your escorts has defected."
        )
    );
    assert_eq!(view.pilot().expect("flying").escorts(), []);
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

#[test]
fn the_hire_ports_reach_the_session() {
    let always: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
    let mut view = FlightView::new(hiring())
        .with_chance(SharedChance::new(always))
        .with_hire_terms(Rc::new(Sevens))
        .with_control_bits(Rc::new(Nothing));
    land_now(&mut view);
    let list = view.escorts_for_hire().expect("a bar");
    let row = list.row(ShipId(129)).expect("listed: no Flags3 hides it");
    assert_eq!((row.fee, row.wage), (7, 3));
    assert_eq!(row.hire, Err(HireRefusal::NotForHire));
}

// Persons.

#[test]
fn the_persons_rules_reach_the_session() {
    let rules = nova_sim::NovaPersons {
        odds: RuleSource::Bible,
        ..nova_sim::NovaPersons::default()
    };
    let view = flight().with_person_rules(Rc::new(rules));
    assert_eq!(
        format!("{:?}", view.session().expect("flying").person_rules()),
        format!("{rules:?}")
    );
}

#[test]
fn a_landed_q_shows_the_take_offs_pay_after_its_message() {
    let mut view = told(paying(50, true));
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    run_set(&mut view, "Q25048");
    view.settle_script();
    assert_eq!(view.take_off(), Some(StellarId(128)));
    let said = format!("Off you go, <PSN>.  {DEFECTED_ONE}");
    assert_lasts(&mut view, &said, 500);
}

/// The greeting from Earth on the day after the fixture's start, its
/// lead the first, as the flight's chance rolls.
const EARTH_GREETING: &str = "Launching from Earth on June 24th, 1177 NC.";

#[test]
fn take_off_says_the_greeting_for_240() {
    let mut view = landed_view();
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(message(&view), Some(message_drawn(EARTH_GREETING)));
    assert_lasts(&mut view, EARTH_GREETING, 240);
    assert_eq!(
        take_off_greeting(2, "Earth", "June 24th, 1177 NC"),
        "Taking off from Earth on June 24th, 1177 NC."
    );
    let leads: Vec<String> = (0..5)
        .map(|lead| take_off_greeting(lead, "X", "D"))
        .collect();
    assert_eq!(
        leads,
        [
            "Launching from X on D.",
            "Blasting off from X on D.",
            "Taking off from X on D.",
            "Leaving X on D.",
            "Departing X on D."
        ]
    );
}

#[test]
fn the_greeting_leads_the_pay_notes() {
    let mut view = paying(50, true);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    view.take_off().expect("takes off");
    let said = format!("{PAYING_GREETING}  {DEFECTED_ONE}");
    assert_lasts(&mut view, &said, 240);
}

#[test]
fn a_q_message_takes_the_greetings_place() {
    let mut view = told(landed_view());
    run_set(&mut view, "Q25048");
    view.settle_script();
    assert_eq!(view.leave_if_due(), Some(StellarId(128)));
    assert_eq!(view.message(), Some("Off you go, <PSN>."));
    let rules = Rulebook::default().with_override(RuleKey::BlankLeave, RuleSource::Bible);
    let mut view = told(landed_view()).with_rules(rules);
    run_set(&mut view, "Q25049");
    view.settle_script();
    assert_eq!(view.leave_if_due(), Some(StellarId(128)));
    assert_eq!(view.message(), Some(EARTH_GREETING), "an empty one greets");
}

#[test]
fn a_new_pilots_take_off_says_nothing() {
    let mut view = new_pilot(landed_view());
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.message(), None);
}

#[test]
fn a_take_off_with_no_q_shows_no_leave_message() {
    let mut view = told(landed_view());
    assert!(!view.leave_due());
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.message(), Some(EARTH_GREETING), "the greeting alone");
}

#[test]
fn fighters_abandoned_in_a_move_are_told_after_a_qs_message() {
    let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
    let mut view = told(fighters_out()).with_rules(rules);
    run_set(&mut view, "Q25048 M131");
    view.settle_script();
    assert_eq!(
        view.message(),
        Some("Off you go, <PSN>.  (Two fighters abandoned)")
    );
    assert_eq!(view.pilot().expect("flying").escorts(), []);
    view.message = None;
    view.settle_script();
    assert_eq!(view.message(), None, "nothing more to tell");
}

// Persons' mission offers.

/// Kept real for phase 13 (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge):
/// `Screen::input`'s press filter routes P to the player info's press,
/// and lets no repeat through. What the press does runs on `FakeFlight`
/// (`tests/faked_player.rs`).
#[test]
fn p_through_the_screen_asks_once_and_a_repeat_does_not() {
    let mut view = flight();
    assert!(!view.take_player_info_request());
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(view.take_player_info_request());
    view.input(&Input::Key {
        key: PLAYER_INFO_KEY,
        pressed: true,
        repeat: true,
    });
    assert!(!view.take_player_info_request(), "not a repeat");
}

#[test]
fn p_during_a_jump_beeps_and_asks_nothing() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.session().expect("flying").preparing_jump().is_some());
    let _ = view.take_sounds();
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(!view.take_player_info_request());
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "turning to jump"
    );
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let _ = view.take_sounds();
    view.input(&Input::Key {
        key: PLAYER_INFO_KEY,
        pressed: true,
        repeat: true,
    });
    assert!(view.take_sounds().is_empty(), "not a repeat");
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(!view.take_player_info_request());
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "streaking"
    );
}

#[test]
fn p_once_the_player_has_exploded_or_ejected_does_nothing_and_says_nothing() {
    let mut holding = exploded();
    assert!(matches!(fate_of(&holding), PlayerFate::Holding { .. }));
    let mut pod = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
    assert!(until_ejected(&mut pod).0);
    for (why, view) in [("holding", &mut holding), ("in the pod", &mut pod)] {
        let _ = view.take_sounds();
        tap(view, PLAYER_INFO_KEY);
        assert!(!view.take_player_info_request(), "{why}");
        assert_eq!(view.take_sounds(), [], "{why}");
    }
}

#[test]
fn p_with_the_map_open_does_nothing() {
    let mut view = flight();
    tap(&mut view, MAP_KEY);
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(!view.take_player_info_request());
    assert!(view.map_open());
    assert!(HELP.contains("P: player info"), "{HELP}");
}

#[test]
fn the_help_line_names_mission_info() {
    assert!(HELP.contains("P: player info   I: mission info"), "{HELP}");
}

#[test]
fn i_with_the_map_open_does_nothing() {
    let mut view = flight();
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.message(), None);
    assert_eq!(view.take_mission_info_request(), None);
}
