use super::*;
use crate::flight::sim_port::PortSource;
use crate::spaceport::OfferNames;

// The mission screens.

/// Kept real for phase 12 (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks),
/// as the `OfferNames` impl stays concrete for its `NameSource`: the
/// mission computer's cut offer names are filled by the session.
#[test]
fn the_offer_names_are_the_sessions() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![errand(130), errand(131)];
    let mut view = FlightView::new(errands);
    land_now(&mut view);
    view.settle_script();
    let munged = OfferNames::munged(&view, MissionId(130), "Run <PAY>");
    assert_eq!(
        munged,
        view.session()
            .ok()
            .and_then(|session| session.offer_text(MissionId(130), "Run <PAY>")),
        "the session's munge"
    );
    assert!(munged.is_some_and(|name| !name.contains("<PAY>")), "filled");
    assert_eq!(
        OfferNames::munged(&view, MissionId(999), "Run"),
        None,
        "not offered"
    );
    let broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(OfferNames::munged(&broken, MissionId(130), "Run"), None);
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

#[test]
fn each_jettison_message_is_worded_and_the_arrivals_are_not_among_them() {
    let view = flight();
    let session = view.session().expect("flying");
    let text = |message| sim_message_text(&message, session, RuleSource::Engine);
    assert_eq!(
        text(SimMessage::MissionFailed).as_deref(),
        Some(MISSION_FAILED)
    );
    assert_eq!(
        text(SimMessage::CargoJettisoned).as_deref(),
        Some(CARGO_JETTISONED)
    );
    assert_eq!(
        text(SimMessage::NonMissionCargoJettisoned).as_deref(),
        Some(NON_MISSION_CARGO_JETTISONED)
    );
    assert_eq!(text(SimMessage::Arrived(SystemId(130))), None);
    assert_eq!(text(SimMessage::ExitedHypergate(SystemId(130))), None);
    assert_eq!(text(SimMessage::PassedWormhole(SystemId(130))), None);
    assert_eq!(text(SimMessage::Buoy(HELLO.to_owned())), None);
    for message in [
        SimMessage::MissionFailed,
        SimMessage::CargoJettisoned,
        SimMessage::NonMissionCargoJettisoned,
        SimMessage::MissionScanned,
        SimMessage::Smuggling {
            scanner: ShipId(129),
            ship_name: None,
            fine: 0,
        },
    ] {
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}

#[test]
fn a_former_escort_rejoining_says_escort_or_fighter_repaired() {
    assert_eq!(ESCORT_REPAIRED, "Escort repaired.");
    assert_eq!(FIGHTER_REPAIRED, "Fighter repaired.");
    let view = flight();
    let session = view.session().expect("flying");
    let text = |message| sim_message_text(&message, session, RuleSource::Engine);
    assert_eq!(
        text(SimMessage::EscortRepaired).as_deref(),
        Some(ESCORT_REPAIRED)
    );
    assert_eq!(
        text(SimMessage::FighterRepaired).as_deref(),
        Some(FIGHTER_REPAIRED)
    );
    for message in [SimMessage::EscortRepaired, SimMessage::FighterRepaired] {
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}

#[test]
fn fighter_captured_says_the_originals_words() {
    // `STR#` 2002 #129.
    assert_eq!(FIGHTER_CAPTURED, "Fighter captured.");
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::FighterCaptured, session, RuleSource::Engine).as_deref(),
        Some(FIGHTER_CAPTURED)
    );
    assert_eq!(arrival_lead(&SimMessage::FighterCaptured), None);
}

/// A flight whose one outfit is called "bomb", "bombs".
fn with_a_bomb() -> FlightView<FakeCatalog> {
    FlightView::new(FakeCatalog {
        outfits: vec![OutfitRecord {
            lc_name: "bomb".to_owned(),
            lc_plural: "bombs".to_owned(),
            ..multi_jumping().outfits[0].clone()
        }],
        ..catalog()
    })
}

#[test]
fn a_sabotage_notice_names_the_outfit_for_its_count() {
    assert_eq!(
        (YOUR, HAS_EXPLODED, HAVE_EXPLODED),
        ("Your", "has exploded!", "have exploded!")
    );
    let view = with_a_bomb();
    let session = view.session().expect("flying");
    let sabotaged = |one| SimMessage::Sabotaged {
        outfit: Some((OutfitId(275), one)),
    };
    assert_eq!(
        sim_message_text(&sabotaged(true), session, RuleSource::Engine).as_deref(),
        Some("Your bomb has exploded!")
    );
    assert_eq!(
        sim_message_text(&sabotaged(false), session, RuleSource::Engine).as_deref(),
        Some("Your bombs have exploded!")
    );
    assert_eq!(arrival_lead(&sabotaged(true)), None);
}

#[test]
fn a_bomb_notice_names_the_outfit_with_no_empty_name_check() {
    let view = with_a_bomb();
    let session = view.session().expect("flying");
    let exploded = |outfit, one| SimMessage::BombExploded {
        outfit: OutfitId(outfit),
        one,
    };
    assert_eq!(
        sim_message_text(&exploded(275, true), session, RuleSource::Engine).as_deref(),
        Some("Your bomb has exploded!")
    );
    assert_eq!(
        sim_message_text(&exploded(275, false), session, RuleSource::Engine).as_deref(),
        Some("Your bombs have exploded!")
    );
    assert_eq!(
        sim_message_text(&exploded(999, true), session, RuleSource::Engine).as_deref(),
        Some("Your  has exploded!"),
        "an empty name is spelled out as it is"
    );
    assert_eq!(arrival_lead(&exploded(275, true)), None);
}

#[test]
fn the_leet_sabotage_notice_says_someone_set_up_the_bomb() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::BombSetUp, session, RuleSource::Engine).as_deref(),
        Some("Someone set up us the bomb!")
    );
    assert_eq!(arrival_lead(&SimMessage::BombSetUp), None);
}

#[test]
fn a_sabotage_notice_without_an_outfit_says_a_bomb_exploded() {
    let view = flight();
    let session = view.session().expect("flying");
    let message = SimMessage::Sabotaged { outfit: None };
    assert_eq!(
        sim_message_text(&message, session, RuleSource::Engine).as_deref(),
        Some("A bomb has exploded onboard your ship!")
    );
    assert_eq!(
        BOMB_EXPLODED_ONBOARD,
        "A bomb has exploded onboard your ship!"
    );
}

#[test]
fn a_collapsed_jump_says_the_hyperspace_field_collapsed() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::JumpCollapsed, session, RuleSource::Engine).as_deref(),
        Some("Ship disabled - hyperspace field collapsed.")
    );
    assert_eq!(
        JUMP_COLLAPSED,
        "Ship disabled - hyperspace field collapsed."
    );
    assert_eq!(arrival_lead(&SimMessage::JumpCollapsed), None);
}

#[test]
fn the_players_repair_droids_say_propulsion_systems_repaired() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::PropulsionRepaired, session, RuleSource::Engine).as_deref(),
        Some("Propulsion systems repaired.")
    );
    assert_eq!(PROPULSION_REPAIRED, "Propulsion systems repaired.");
    assert_eq!(arrival_lead(&SimMessage::PropulsionRepaired), None);
}

#[test]
fn the_escape_ship_says_escaping_in_its_long_name() {
    assert_eq!(ESCAPING_IN, "Escaping in");
    let mut catalog = shipbuying();
    catalog.ships[0].long_name = "Federation Viper".to_owned();
    let view = FlightView::new(catalog);
    let session = view.session().expect("flying");
    let escaping = |ship| SimMessage::EscapingIn { ship: ShipId(ship) };
    assert_eq!(
        sim_message_text(&escaping(129), session, RuleSource::Engine).as_deref(),
        Some("Escaping in Federation Viper.")
    );
    assert_eq!(
        sim_message_text(&escaping(999), session, RuleSource::Engine).as_deref(),
        Some("Escaping in ."),
        "a class with no record has no long name"
    );
    assert_eq!(arrival_lead(&escaping(129)), None);
}

#[test]
fn a_mission_ship_boarded_on_the_assist_path_says_its_line() {
    assert_eq!(TARGET_BOARDED, "Target ship has been boarded.");
    assert_eq!(
        refuelled_message("Freighter", "Kestrel"),
        "Freighter:  Energy transfer complete, Kestrel."
    );
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        mission_board_message(MissionBoardLine::Boarded, session),
        TARGET_BOARDED
    );
}

#[test]
fn a_pickup_ships_line_reads_the_cargo_name_from_str_4001() {
    let strings = FakeCatalog {
        strings: vec![(4001, vec!["Food".to_owned(), "*Passengers".to_owned()])],
        ..catalog()
    };
    let view = flight().with_strings(Rc::new(strings));
    let session = view.session().expect("flying");
    let said = |cargo| mission_board_message(MissionBoardLine::Retrieved { cargo }, session);
    assert_eq!(said(Some(0)), "You retrieved  the Food from this ship.");
    assert_eq!(said(Some(1)), "You retrieved  Passengers from this ship.");
    assert_eq!(said(Some(2)), "You retrieved  the  from this ship.");
    assert_eq!(said(None), "You retrieved  the  from this ship.");
    assert_eq!(said(Some(256)), "You retrieved  the  from this ship.");
}

#[test]
fn a_mission_ship_lost_says_str_2002_284() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::MissionShipLost, session, RuleSource::Engine).as_deref(),
        Some(MISSION_FAILED)
    );
    assert_eq!(arrival_lead(&SimMessage::MissionShipLost), None);
}

#[test]
fn a_fleets_arrival_quote_says_its_line() {
    let view = flight();
    let session = view.session().expect("flying");
    let quote = SimMessage::FleetArrivalQuote("Hail 9".to_owned());
    assert_eq!(
        sim_message_text(&quote, session, RuleSource::Engine).as_deref(),
        Some("Hail 9")
    );
    assert_eq!(arrival_lead(&quote), None);
}

#[test]
fn a_distress_call_a_pre_scan_hail_reinforcements_and_wait_for_me_say_their_text_as_given() {
    let view = flight();
    let session = view.session().expect("flying");
    let wait = "Viper:  Wait for me!";
    let help = "Ship 129:  Help!";
    let hail = "Ship 129:  Kestrel, halt.";
    let coming = "Sensors detect Federation reinforcement fleet approaching.";
    for (message, text) in [
        (SimMessage::CallForHelp(help.to_owned()), help),
        (SimMessage::ScanHail(hail.to_owned()), hail),
        (SimMessage::WaitForMe(wait.to_owned()), wait),
        (
            SimMessage::ReinforcementsApproaching(coming.to_owned()),
            coming,
        ),
    ] {
        assert_eq!(
            sim_message_text(&message, session, RuleSource::Engine).as_deref(),
            Some(text)
        );
        assert_eq!(arrival_lead(&message), None);
    }
}

#[test]
fn the_players_downed_lines_are_str_2002_287_288_and_284() {
    let view = flight();
    let session = view.session().expect("flying");
    for (message, text) in [
        (SimMessage::ShipDisabled, "Ship disabled."),
        (SimMessage::ShipDestroyed, "Ship destroyed."),
        (SimMessage::DownedMissionFailed, "Mission failed."),
    ] {
        assert_eq!(
            sim_message_text(&message, session, RuleSource::Engine).as_deref(),
            Some(text),
            "{message:?}"
        );
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}

#[test]
fn the_no_active_missions_message_is_str_2002_354() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::NoActiveMissions, session, RuleSource::Engine).as_deref(),
        Some(NO_ACTIVE_MISSIONS)
    );
    assert_eq!(arrival_lead(&SimMessage::NoActiveMissions), None);
}

#[test]
fn the_deadline_message_is_str_2002_285() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::DeadlineMissed, session, RuleSource::Engine).as_deref(),
        Some(TIME_LIMIT_EXCEEDED)
    );
    assert_eq!(arrival_lead(&SimMessage::DeadlineMissed), None);
    assert_eq!(TIME_LIMIT_EXCEEDED, "Time limit exceeded - mission failed.");
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

/// A contraband notice names the scanner by its person's name when it
/// is one (`_ScanPlayer` @0x7ebd1-0x7ec0c), else its ship class, and
/// the outfit or `jünk` by the session's record; it lasts 400 ticks.
#[test]
fn a_contraband_notice_reads_its_names_from_the_session() {
    let view = FlightView::new(FakeCatalog {
        outfits: vec![OutfitRecord {
            lc_name: "blaster".to_owned(),
            lc_plural: "blasters".to_owned(),
            ..multi_jumping().outfits[0].clone()
        }],
        junk: vec![JunkRecord {
            id: nova_sim::JunkId(150),
            name: "Bio-weapons".to_owned(),
            abbrev: "Bio".to_owned(),
            base_price: 1000,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: nova_sim::Test::default(),
            sell_on: nova_sim::Test::default(),
            flags: 0,
            scan_mask: 0x0004,
            lc_name: "bio-weapons".to_owned(),
        }],
        ..catalog()
    });
    let session = view.session().expect("flying");
    let ship = session.pilot().ship_name().unwrap_or_default();
    let class = session.ship_name(ShipId(128)).unwrap_or_default();
    let outfit = |person: Option<&str>| SimMessage::IllegalOutfit {
        scanner: ShipId(128),
        person: person.map(str::to_owned),
        outfit: OutfitId(275),
        one: true,
        fine: 500,
    };
    assert_eq!(
        sim_message_text(&outfit(Some("Ace")), session, RuleSource::Engine),
        Some(illegal_outfit_message(
            "Ace", ship, true, "blaster", "blasters", 500
        ))
    );
    assert_eq!(
        sim_message_text(&outfit(None), session, RuleSource::Engine),
        Some(illegal_outfit_message(
            class, ship, true, "blaster", "blasters", 500
        ))
    );
    let junk = |id: i16| SimMessage::IllegalJunk {
        scanner: ShipId(128),
        person: Some("Ace".to_owned()),
        junk: nova_sim::JunkId(id),
        fine: 0,
    };
    assert_eq!(
        sim_message_text(&junk(150), session, RuleSource::Engine),
        Some(illegal_junk_message("Ace", ship, "bio-weapons", 0))
    );
    assert_eq!(
        sim_message_text(&junk(151), session, RuleSource::Engine),
        Some(illegal_junk_message("Ace", ship, "", 0)),
        "no record, no name"
    );
    assert_eq!(arrival_lead(&junk(150)), None);
    assert_eq!(arrival_lead(&outfit(None)), None);
}

/// The found mission's drawn ship name names the scanner when it has
/// one (`_ScanPlayer` @0x7e7ce); otherwise its ship class does.
#[test]
fn the_smuggling_notice_names_the_scanner_by_the_found_missions_ship_name() {
    let view = flight();
    let session = view.session().expect("flying");
    let ship = session.pilot().ship_name().unwrap_or_default();
    let text = |ship_name: Option<&str>| {
        sim_message_text(
            &SimMessage::Smuggling {
                scanner: ShipId(128),
                ship_name: ship_name.map(str::to_owned),
                fine: 500,
            },
            session,
            RuleSource::Engine,
        )
    };
    assert_eq!(
        text(Some("Rover")),
        Some(smuggling_message("Rover", ship, 500))
    );
    assert_eq!(
        text(None),
        Some(smuggling_message(
            session.ship_name(ShipId(128)).unwrap_or_default(),
            ship,
            500
        ))
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

#[test]
fn p_asks_for_the_player_info_once_letting_go_of_the_flight_keys() {
    let mut view = flight();
    assert!(!view.take_player_info_request());
    view.input(&key(Key::Up, true));
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(view.take_player_info_request());
    assert!(!view.take_player_info_request(), "once");
    view.input(&Input::Key {
        key: PLAYER_INFO_KEY,
        pressed: true,
        repeat: true,
    });
    assert!(!view.take_player_info_request(), "not a repeat");
    view.tick(TICK);
    assert_eq!(
        view.session().expect("flying").player().velocity,
        Vec2::ZERO,
        "the flight keys let go"
    );
    assert_eq!(
        view.player_info().map(|info| info.credits),
        Some(view.pilot().expect("flying").cash())
    );
    assert!(view.take_sounds().is_empty(), "silent");
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
fn p_while_the_player_breaks_up_beeps_and_asks_nothing() {
    let mut view = breaking_up();
    let _ = view.take_sounds();
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(!view.take_player_info_request());
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Refused)]);
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
fn p_while_disabled_asks_for_the_player_info_in_silence() {
    let mut view = disabled_player();
    let _ = view.take_sounds();
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(view.take_player_info_request());
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn opening_the_player_info_gives_the_screen_unless_in_a_jump() {
    let mut view = flight();
    assert_eq!(view.open_player_info(), view.player_info(), "flying");
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.session().expect("flying").preparing_jump().is_some());
    assert!(view.player_info().is_some(), "the getter reads it");
    assert_eq!(view.open_player_info(), None, "no opening (@0x5a02a)");
}

#[test]
fn open_player_info_carries_the_cheats_setting() {
    let mut view = flight();
    assert!(
        !view.open_player_info().expect("opens").cheats,
        "off by default"
    );
    let mut view = flight().with_overlays(FlightOverlays {
        cheats: true,
        ..FlightOverlays::default()
    });
    assert!(view.open_player_info().expect("opens").cheats);
    assert!(view.player_info().expect("reads").cheats, "the getter too");
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

/// A plunder's lines read as `_AIPlunderShipContents` builds them, the
/// numbers' millions by the session's rules, and lead no arrival.
#[test]
fn a_plunders_lines_are_its_cargo_and_its_stolen_ship() {
    let view = flight();
    let session = view.session().expect("flying");
    let stolen = SimMessage::CargoStolen {
        tons: 2,
        credits: 1_234_567,
    };
    assert_eq!(
        sim_message_text(&stolen, session, RuleSource::Engine),
        Some("2 tons of cargo and 1.23M credits stolen!".to_owned())
    );
    assert_eq!(
        sim_message_text(
            &SimMessage::ShipStolen { fighter: true },
            session,
            RuleSource::Engine
        ),
        Some("Fighter stolen!".to_owned())
    );
    assert_eq!(arrival_lead(&stolen), None);
}

#[test]
fn the_self_destructs_lines_say_its_stock_words() {
    use nova_sim::SelfDestructLine;
    let view = flight();
    let session = view.session().expect("flying");
    let text = |message: SimMessage| sim_message_text(&message, session, RuleSource::Engine);
    assert_eq!(
        text(SimMessage::SelfDestruct(SelfDestructLine::Initiated)).as_deref(),
        Some("Self-destruct sequence initiated.")
    );
    assert_eq!(
        text(SimMessage::SelfDestruct(SelfDestructLine::In {
            seconds: 4,
            singular: false
        }))
        .as_deref(),
        Some("Self-destruct in 4 seconds.")
    );
    assert_eq!(
        text(SimMessage::SelfDestruct(SelfDestructLine::In {
            seconds: 1,
            singular: false
        }))
        .as_deref(),
        Some("Self-destruct in 1 seconds."),
        "the engine always says #389"
    );
    assert_eq!(
        text(SimMessage::SelfDestruct(SelfDestructLine::In {
            seconds: 1,
            singular: true
        }))
        .as_deref(),
        Some("Self-destruct in 1 second.")
    );
    assert_eq!(
        text(SimMessage::SelfDestruct(SelfDestructLine::Cancelled)).as_deref(),
        Some("Self-destruct sequence cancelled.")
    );
    assert_eq!(
        text(SimMessage::SelfDestruct(SelfDestructLine::NiceDay)).as_deref(),
        Some("Have a nice day.")
    );
    for message in [
        SimMessage::SelfDestruct(SelfDestructLine::Initiated),
        SimMessage::SelfDestruct(SelfDestructLine::In {
            seconds: 2,
            singular: false,
        }),
        SimMessage::SelfDestruct(SelfDestructLine::Cancelled),
        SimMessage::SelfDestruct(SelfDestructLine::NiceDay),
    ] {
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}
