use super::*;
use crate::spaceport::OfferNames;

// The mission screens.

#[test]
fn the_mission_screens_go_through_the_session() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![errand(130), errand(131)];
    errands.mission_texts = vec![
        MissionTexts {
            brief: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        },
        MissionTexts {
            refuse: Some("Fine.".to_owned()),
            ..MissionTexts::named(MissionId(131), "Walk")
        },
    ];
    let mut view = FlightView::new(errands);
    assert_eq!(
        view.mission_computer(),
        Err(ComputerRefusal::NoneAvailable),
        "in flight"
    );
    assert_eq!(
        view.accept_mission(MissionId(130)),
        Err(AcceptError::NotOffered)
    );
    assert_eq!(view.refuse_mission(MissionId(131)), Err(NotOffered));
    land_now(&mut view);
    view.settle_script();
    let computer = view.mission_computer().expect("opens");
    let names: Vec<_> = computer.rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["Run", "Walk"]);
    assert_eq!(view.open_mission_computer(), Ok(computer.clone()));
    assert_eq!(
        view.show_computer_row(MissionId(131)),
        Some(computer.rows[1].clone()),
        "loaded through the session"
    );
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
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    assert_eq!(view.refuse_mission(MissionId(131)).map(|_| ()), Ok(()));
    let texts: Vec<_> = view
        .take_mission_texts()
        .into_iter()
        .map(|shown| match shown {
            MissionShown::Text(text) => (text.kind, text.text),
            MissionShown::Movie(movie) => panic!("an offer movie: {movie:?}"),
        })
        .collect();
    assert_eq!(
        texts,
        [
            (MissionTextKind::Brief, "Go.".to_owned()),
            (MissionTextKind::Refuse, "Fine.".to_owned()),
        ]
    );
    assert_eq!(view.take_mission_texts(), [], "taken");
    let mut broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(
        broken.mission_computer(),
        Err(ComputerRefusal::NoneAvailable)
    );
    assert_eq!(
        broken.open_mission_computer(),
        Err(ComputerRefusal::NoneAvailable)
    );
    assert_eq!(broken.show_computer_row(MissionId(130)), None);
    assert_eq!(OfferNames::munged(&broken, MissionId(130), "Run"), None);
    assert_eq!(
        broken.accept_mission(MissionId(130)),
        Err(AcceptError::NotOffered)
    );
    assert_eq!(broken.refuse_mission(MissionId(130)), Err(NotOffered));
    assert_eq!(broken.take_mission_texts(), []);
}

#[test]
fn closing_a_mission_dialog_or_its_texts_redraws_the_hud() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![errand(130), errand(131)];
    errands.mission_texts = vec![MissionTexts {
        brief: Some("Go.".to_owned()),
        ..MissionTexts::named(MissionId(130), "Run")
    }];
    let mut view = FlightView::new(errands);
    let stale = RadarScene::default();
    let fresh = view.radar_now();
    assert_ne!(fresh, stale);
    view.radar_scene = Some(stale.clone());
    let _ = view.accept_mission(MissionId(130));
    assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "accepted");
    view.radar_scene = Some(stale.clone());
    let _ = view.refuse_mission(MissionId(130));
    assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "refused");
    view.radar_scene = Some(stale.clone());
    assert_eq!(view.take_mission_texts(), []);
    assert_eq!(view.radar_scene.as_ref(), Some(&stale), "nothing shown");
    land_now(&mut view);
    view.settle_script();
    // Docked, the held gauges are redrawn too.
    let held = Reserves::default();
    assert_ne!(view.reserves(), held);
    view.reserves_scene = Some(held);
    assert_eq!(view.reserves_shown(), held, "held");
    assert_eq!(view.refuse_mission(MissionId(131)).map(|_| ()), Ok(()));
    assert_eq!(view.reserves_shown(), view.reserves(), "refused docked");
    view.reserves_scene = Some(held);
    view.radar_scene = Some(stale.clone());
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    assert_eq!(view.take_mission_texts().len(), 1);
    assert_eq!(view.reserves_shown(), held, "the brief shown over the bar");
    assert_eq!(view.radar_scene.as_ref(), Some(&stale), "the brief shown");
    view.close_mission_text();
    assert_eq!(view.radar_scene, Some(view.radar_now()), "the brief closed");
    assert_eq!(view.reserves_shown(), view.reserves(), "the brief closed");
}

#[test]
fn entering_another_venue_through_the_flight_offers_a_suppressed_offer_again() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = (130..146).map(errand).collect();
    errands.missions.push(MissionRecord {
        avail_loc: 1,
        flags: nova_sim::mission::screens::UNREFUSABLE,
        ..errand(150)
    });
    let mut view = FlightView::new(errands);
    land_now(&mut view);
    view.settle_script();
    for id in 130..146 {
        assert!(view.accept_mission(MissionId(id)).is_ok(), "{id}");
    }
    view.enter_offer_venue(1);
    let refused = Some(VenueOffer::Accepted {
        mission: MissionId(150),
        accepted: Err(AcceptError::Refused(
            nova_sim::mission::AcceptRefusal::SlotsFull,
        )),
    });
    assert_eq!(view.next_offer(1), refused);
    assert_eq!(view.next_offer(1), None, "suppressed");
    view.enter_offer_venue(3);
    view.enter_offer_venue(1);
    assert_eq!(view.next_offer(1), refused, "offered again");
}

/// A flight from [`trading`]'s Earth, its ship of 10 tons, that
/// accepted mission 130, carrying 4 tons of food loaded at accept,
/// `Flags` `flags`, and bought the other 6 tons full of food when
/// `food`, then took off.
pub(super) fn carrying(flags: u16, food: bool) -> View {
    carrying_in(trading(), Rulebook::default(), flags, food)
}

/// [`carrying`], from `errands` under `rules`.
fn carrying_in(mut errands: FakeCatalog, rules: Rulebook, flags: u16, food: bool) -> View {
    errands.missions = vec![MissionRecord {
        cargo_type: 0,
        cargo_qty: 4,
        pickup_mode: 0,
        flags,
        ..errand(130)
    }];
    let mut view = FlightView::new(errands).with_rules(rules);
    land_now(&mut view);
    view.settle_script();
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    if food {
        assert_eq!(view.trade(BUY_FOOD), Ok(6), "up to the free space");
    }
    view.take_off().expect("takes off");
    view.settle_script();
    // Past the take-off greeting.
    view.message = None;
    view
}

#[test]
fn command_k_jettisons_the_cargo_failing_the_mission_and_says_so() {
    let mut view = carrying(0, true);
    assert_eq!(cargo_aboard(&view), 10);
    command_tap(&mut view, JETTISON_KEY, false);
    assert_eq!(cargo_aboard(&view), 0);
    assert_eq!(view.pilot().expect("flying").missions().iter().count(), 0);
    assert_eq!(view.message(), Some(MISSION_FAILED));
    assert_eq!(JETTISON_KEY, Key::Char('k'));
    assert_eq!(MISSION_FAILED, "Mission failed.");
    assert!(HELP.contains("Cmd-K: jettison"), "{HELP}");
}

#[test]
fn command_option_k_keeps_the_mission_cargo() {
    let mut view = carrying(0, true);
    command_tap(&mut view, JETTISON_KEY, true);
    assert_eq!(cargo_aboard(&view), 4, "the food bought went");
    assert_eq!(view.message(), Some(NON_MISSION_CARGO_JETTISONED));
    assert_eq!(
        NON_MISSION_CARGO_JETTISONED,
        "Non-mission cargo jettisoned."
    );
    let mut view = carrying(0, false);
    command_tap(&mut view, JETTISON_KEY, true);
    assert_eq!(cargo_aboard(&view), 4, "kept");
    assert_eq!(view.message(), None, "nothing else held");
}

#[test]
fn jettison_all_empties_the_hold_and_says_so() {
    let mut view = carrying(0x0400, false);
    assert!(cargo_aboard(&view) > 0);
    view.jettison_all();
    assert_eq!(cargo_aboard(&view), 0);
    assert_eq!(view.message(), Some(CARGO_JETTISONED));
}

#[test]
fn a_hidden_missions_cargo_jettisoned_says_the_cargo_went() {
    let mut view = carrying(0x0400, false);
    command_tap(&mut view, JETTISON_KEY, false);
    assert_eq!(cargo_aboard(&view), 0);
    assert_eq!(view.message(), Some(CARGO_JETTISONED));
    assert_eq!(CARGO_JETTISONED, "Cargo jettisoned.");
}

/// The cargo boxes' sprites drawn, `rlëD` 500's, in the order drawn.
fn box_sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
    sprites(list)
        .into_iter()
        .filter(|(image, _)| image.id == 500)
        .collect()
}

#[test]
fn command_k_drops_cargo_boxes_drawn_under_the_ship_where_they_drift() {
    let mut view = carrying(0, true);
    // Turned off straight up, so the boxes drift on both axes.
    view.input(&key(Key::Right, true));
    ticks(&mut view, 10);
    view.input(&key(Key::Right, false));
    assert!(box_sprites(&drawn(&view)).is_empty());
    let _ = view.take_sounds();
    command_tap(&mut view, JETTISON_KEY, false);
    let session = view.session().expect("flying");
    let boxes = session.cargo_boxes().to_vec();
    assert_eq!(boxes.len(), 2, "10 tons");
    assert!(
        boxes[0].velocity.x.abs() > 0.1 && boxes[0].velocity.y.abs() > 0.1,
        "{boxes:?}"
    );
    // A quarter of the way from the last step to this one.
    view.alpha = 0.25;
    let camera = view.camera();
    let behind = 0.75;
    let expected: Vec<_> = boxes
        .iter()
        .map(|made| {
            (
                ImageKey::sprite(500, made.frame),
                camera.world_to_screen(Point::new(
                    made.velocity.x.mul_add(-behind, made.position.x),
                    made.velocity.y.mul_add(-behind, made.position.y),
                )),
            )
        })
        .collect();
    let list = drawn(&view);
    assert_eq!(box_sprites(&list), expected);
    let all = sprites(&list);
    let first_box = all.iter().position(|(image, _)| image.id == 500);
    let ship = all.iter().position(|(image, _)| image.id == 2000);
    assert!(first_box < ship, "under the ship: {all:?}");
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::CargoJettisoned)],
        "snd 154"
    );
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

/// The rules with `key` read as the other reading.
pub(super) fn other_reading(key: RuleKey) -> Rulebook {
    Rulebook::default().with_override(key, RuleSource::Bible)
}

fn asteroid_count(view: &View) -> usize {
    view.session().expect("flying").asteroids().count()
}

/// `view` in a view 640 wide, whose window stores each system's
/// `Asteroids` unscaled, so a test's counts are its catalog's.
fn unscaled(mut view: View) -> View {
    view.set_view_size((640.0, 768.0));
    view
}

#[test]
fn each_step_spawns_an_asteroid_at_the_edge_until_the_systems_count() {
    // Just off screen, so none is culled as it spawns.
    let mut view = unscaled(
        FlightView::new(rocky(catalog())).with_rules(other_reading(RuleKey::AsteroidRespawnRing)),
    );
    assert_eq!(asteroid_count(&view), 0);
    for count in [1, 2, 3, 3] {
        ticks(&mut view, 1);
        assert_eq!(asteroid_count(&view), count);
    }
}

#[test]
fn a_flight_started_in_flight_shows_its_placed_field_before_any_step() {
    let mut view = unscaled(
        FlightView::new(rocky(catalog())).with_rules(other_reading(RuleKey::AsteroidEntryField)),
    );
    view.tick(Duration::ZERO);
    assert_eq!(asteroid_count(&view), 3, "placed, no step run");
    let list = drawn(&view);
    let asteroids = sprites(&list)
        .into_iter()
        .filter(|(image, _)| image.id == 1800)
        .count();
    assert_eq!(asteroids, 3);
    let mut engine = unscaled(FlightView::new(rocky(catalog())));
    engine.tick(Duration::ZERO);
    assert_eq!(asteroid_count(&engine), 0, "by the engine");
}

#[test]
fn an_asteroid_is_its_types_frame_drawn_where_it_is_over_the_stellars_and_under_the_boxes() {
    let rules = other_reading(RuleKey::AsteroidRespawnRing);
    let mut view = carrying_in(rocky(trading()), rules, 0, true);
    ticks(&mut view, 2);
    command_tap(&mut view, JETTISON_KEY, false);
    view.alpha = 0.25;
    let session = view.session().expect("flying");
    let made: Vec<_> = session.asteroids().copied().collect();
    assert!(!made.is_empty());
    let camera = view.camera();
    let expected: Vec<_> = made
        .iter()
        .map(|asteroid| {
            (
                ImageKey::sprite(1800, asteroid.frame as u16),
                camera.world_to_screen(Point::new(
                    asteroid.velocity.x.mul_add(-0.75, asteroid.position.x),
                    asteroid.velocity.y.mul_add(-0.75, asteroid.position.y),
                )),
            )
        })
        .collect();
    let list = drawn(&view);
    let drawn_asteroids: Vec<_> = sprites(&list)
        .into_iter()
        .filter(|(image, _)| image.id == 1800)
        .collect();
    assert_eq!(drawn_asteroids, expected);
    let stellar = first(&list, sprite_of(1128));
    let asteroid = first(&list, sprite_of(1800));
    let cargo_box = first(&list, sprite_of(500));
    assert!(stellar < asteroid && asteroid < cargo_box, "{list:?}");
}

#[test]
fn a_landing_clears_the_cargo_boxes() {
    let mut view = carrying(0, true);
    command_tap(&mut view, JETTISON_KEY, false);
    assert_eq!(box_sprites(&drawn(&view)).len(), 2);
    land_now(&mut view);
    assert!(view.session().expect("flying").landed().is_some());
    assert!(view.session().expect("flying").cargo_boxes().is_empty());
    assert!(box_sprites(&drawn(&view)).is_empty());
}

#[test]
fn a_jump_clears_the_cargo_boxes() {
    let mut view = carrying(0, true);
    ticks(&mut view, nova_sim::TAKEOFF_GRACE_TICKS);
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    if let Ok(session) = &mut view.session {
        session.jettison_cargo(true, &mut nova_sim::NeverFires);
    }
    assert_eq!(view.session().expect("flying").cargo_boxes().len(), 2);
    arrive_now(&mut view);
    assert!(box_sprites(&drawn(&view)).is_empty());
}

#[test]
fn k_alone_jettisons_nothing() {
    let mut view = carrying(0, false);
    tap(&mut view, JETTISON_KEY);
    assert_eq!(cargo_aboard(&view), 4);
    assert_eq!(view.message(), None);
    view.input(&key(Key::Alt, true));
    tap(&mut view, JETTISON_KEY);
    view.input(&key(Key::Alt, false));
    assert_eq!(cargo_aboard(&view), 4, "Option alone");
    command_tap(&mut view, Key::Char('j'), false);
    assert_eq!(cargo_aboard(&view), 4, "another key with Command");
}

#[test]
fn each_jettison_message_is_worded_and_the_arrivals_are_not_among_them() {
    let view = flight();
    let session = view.session().expect("flying");
    let text = |message| sim_message_text(&message, session);
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
    let text = |message| sim_message_text(&message, session);
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
        sim_message_text(&sabotaged(true), session).as_deref(),
        Some("Your bomb has exploded!")
    );
    assert_eq!(
        sim_message_text(&sabotaged(false), session).as_deref(),
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
        sim_message_text(&exploded(275, true), session).as_deref(),
        Some("Your bomb has exploded!")
    );
    assert_eq!(
        sim_message_text(&exploded(275, false), session).as_deref(),
        Some("Your bombs have exploded!")
    );
    assert_eq!(
        sim_message_text(&exploded(999, true), session).as_deref(),
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
        sim_message_text(&SimMessage::BombSetUp, session).as_deref(),
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
        sim_message_text(&message, session).as_deref(),
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
        sim_message_text(&SimMessage::JumpCollapsed, session).as_deref(),
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
        sim_message_text(&SimMessage::PropulsionRepaired, session).as_deref(),
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
        sim_message_text(&escaping(129), session).as_deref(),
        Some("Escaping in Federation Viper.")
    );
    assert_eq!(
        sim_message_text(&escaping(999), session).as_deref(),
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
        sim_message_text(&SimMessage::MissionShipLost, session).as_deref(),
        Some(MISSION_FAILED)
    );
    assert_eq!(arrival_lead(&SimMessage::MissionShipLost), None);
}

#[test]
fn a_fleets_arrival_quote_says_its_line() {
    let view = flight();
    let session = view.session().expect("flying");
    let quote = SimMessage::FleetArrivalQuote("Hail 9".to_owned());
    assert_eq!(sim_message_text(&quote, session).as_deref(), Some("Hail 9"));
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
        assert_eq!(sim_message_text(&message, session).as_deref(), Some(text));
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
            sim_message_text(&message, session).as_deref(),
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
        sim_message_text(&SimMessage::NoActiveMissions, session).as_deref(),
        Some(NO_ACTIVE_MISSIONS)
    );
    assert_eq!(arrival_lead(&SimMessage::NoActiveMissions), None);
}

#[test]
fn the_deadline_message_is_str_2002_285() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(
        sim_message_text(&SimMessage::DeadlineMissed, session).as_deref(),
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
    let pilot = Pilot::new(&catalog, "Ada").expect("a pilot");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["ship_name"] = serde_json::json!("Kestrel");
    save["missions"] = serde_json::json!([{
        "slot": 0,
        "mission": 130,
        "destination": null,
        "return": null,
        "cargo_type": 0,
        "cargo_tons": 4,
        "pay": 0,
        "days_left": null,
        "deadline": null,
        "reached": false,
        "goals_met": true,
        "aboard": true,
        "failed": null,
        "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
        "aux": {"countdown": 0, "arrived": 0, "left": 0},
    }]);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut draws = placed(750, 750, 0).to_vec();
    draws.push(0);
    let (_, chance) = scripted(&draws);
    FlightView::with_pilot(catalog, pilot)
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
        sim_message_text(&outfit(Some("Ace")), session),
        Some(illegal_outfit_message(
            "Ace", ship, true, "blaster", "blasters", 500
        ))
    );
    assert_eq!(
        sim_message_text(&outfit(None), session),
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
        sim_message_text(&junk(150), session),
        Some(illegal_junk_message("Ace", ship, "bio-weapons", 0))
    );
    assert_eq!(
        sim_message_text(&junk(151), session),
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

#[test]
fn mission_info_and_aborts_go_through_the_session() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![errand(130)];
    errands.mission_texts = vec![MissionTexts {
        quick_brief: Some("Quick.".to_owned()),
        ..MissionTexts::named(MissionId(130), "Run")
    }];
    let mut view = FlightView::new(errands);
    assert_eq!(view.mission_info().rows, []);
    assert_eq!(view.abort_mission(0), Err(AbortError::NotActive));
    land_now(&mut view);
    view.settle_script();
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    let slots: Vec<_> = view
        .mission_info()
        .rows
        .iter()
        .map(|row| row.slot)
        .collect();
    assert_eq!(slots, [0]);
    assert_eq!(view.show_info_row(0).as_deref(), Some("Quick."));
    assert_eq!(view.show_info_row(1), None, "a free slot");
    assert_eq!(view.abort_mission(0), Ok(()));
    assert_eq!(view.mission_info().rows, []);
    let mut broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(broken.mission_info().rows, []);
    assert_eq!(broken.show_info_row(0), None);
    assert_eq!(broken.abort_mission(0), Err(AbortError::NotActive));
}

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
    let mut broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    broken.enter_offer_venue(1);
    assert_eq!(broken.next_offer(1), None);
}

/// `pilot`, saved and read back with `escorts`, each a ship, whether
/// it is carried and its wage, each full.
fn with_paid_fleet(pilot: &Pilot, escorts: &[(i16, bool, Option<i64>)]) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
    let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
    save["escorts"] = escorts
        .iter()
        .map(|&(ship, carried, wage)| {
            serde_json::json!({
                "ship": ship,
                "reserves": {"shield": gauge(40.0), "armor": gauge(60.0), "fuel": gauge(250.0)},
                "order": null,
                "carried": carried,
                "wage": wage,
                "person": null
            })
        })
        .collect();
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

/// [`boardable`] with ship 130 of `Cost` 10,000 (a wage of 100), and
/// a pilot holding `cash` whose fleet is one hired ship 130, saved at
/// stellar 128 and landed there after its flight's first tick when
/// `docked`, the landing not yet taken; else its flight, its first
/// tick done.
fn paying(cash: i64, docked: bool) -> View {
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.cost = 10_000;
        }
    }
    let mut pilot = with_paid_fleet(
        &Pilot::new(&catalog, "Payer").expect("starts"),
        &[(130, false, Some(100))],
    );
    pilot.set_cash(cash);
    if docked {
        let text = nova_sim::save::encode(&pilot).replace("\"stellar\": null", "\"stellar\": 128");
        pilot = nova_sim::save::decode(&text).expect("a pilot");
    }
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::with_pilot(catalog, pilot)
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
    let pilot = with_paid_fleet(
        &Pilot::new(&catalog, "Carrier").expect("starts"),
        &[
            (130, true, None),
            (130, true, None),
            (129, false, Some(1500)),
        ],
    );
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::with_pilot(catalog, pilot)
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

/// The draws that list Ace and place it 100 above the player, then
/// the first quote draw landing on 0.
const ACE: [u32; 5] = [99, 750, 650, 0, 0];

#[test]
fn a_person_targeted_shows_its_name_and_subtitle_on_the_panel() {
    let (_, chance) = scripted(&ACE);
    let mut view = FlightView::new(peopled())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    tap(&mut view, Key::Tab);
    let texts: Vec<String> = drawn(&view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|text| text == "Ace"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Top Gun"), "{texts:?}");
}

/// A flight from Sol after accepting mission 140, whose one special
/// ship, a rescue held disabled, is set up on Earth under the player
/// at take-off, and which carries 10 tons of food (`STR#` 4001 #1),
/// picked up by boarding it; the player's ship holds 10 tons. With
/// `full`, mission 141, of 10 tons loaded at its accept, fills them
/// first. The ship is targeted.
fn rescuing(full: bool) -> View {
    let mut hunted = trafficked(&[130], 0, 129, 3);
    hunted.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    hunted.sol_navs = vec![Some((0, 0))];
    hunted.fields.holds = 10;
    for record in &mut hunted.ships {
        record.crew = 3;
    }
    hunted.strings = vec![(4001, vec!["Food".to_owned()])];
    let cargo = |id, pickup_mode| MissionRecord {
        cargo_type: 0,
        cargo_qty: 10,
        pickup_mode,
        ..errand(id)
    };
    hunted.missions = vec![
        MissionRecord {
            ships: MissionShip {
                count: 1,
                goal: 5,
                syst: -1,
                dude: Some(nova_sim::DudeId(128)),
                start: -1,
                ..MissionShip::NONE
            },
            ..cargo(140, 2)
        },
        cargo(141, 0),
    ];
    hunted.mission_texts = vec![
        MissionTexts::named(MissionId(140), "Rescue"),
        MissionTexts::named(MissionId(141), "Haul"),
    ];
    let strings = FakeCatalog {
        strings: hunted.strings.clone(),
        ..catalog()
    };
    let mut view = FlightView::new(hunted)
        .with_strings(Rc::new(strings))
        .with_behaviour(Rc::new(Still));
    land_now(&mut view);
    view.settle_script();
    assert_eq!(view.accept_mission(MissionId(140)), Ok(0));
    if full {
        assert_eq!(view.accept_mission(MissionId(141)), Ok(1));
    }
    view.settle_script();
    assert!(view.take_off().is_some());
    view.settle_script();
    view.tick(TICK);
    let _ = view.take_mission_texts();
    tap(&mut view, TARGET_KEY);
    let session = view.session().expect("flying");
    assert!(session.target().is_some_and(|npc| npc.mission.is_some()));
    view
}

#[test]
fn b_on_a_pickup_ship_says_the_cargo_retrieved_for_250_frames() {
    let mut view = rescuing(false);
    tap(&mut view, BOARD_KEY);
    let said = "You retrieved  the Food from this ship.";
    assert_eq!(view.message(), Some(said));
    assert_eq!(view.take_boarding(), None, "no plunder");
    assert_eq!(view.take_mission_texts(), [], "no dialog");
    assert_lasts(&mut view, said, BOARD_LINE_TICKS);
}

/// The pickup path never tests whether the mission failed (@0x652ee),
/// so a failed mission's ship picks up, says its line, then goes on to
/// the plunder (@0x657e3 → @0x65f27).
#[test]
fn b_on_a_failed_missions_pickup_ship_says_the_cargo_retrieved_and_opens_the_plunder() {
    let mut view = rescuing(false);
    run_set(&mut view, "F140");
    tap(&mut view, BOARD_KEY);
    let said = "You retrieved  the Food from this ship.";
    assert_eq!(view.message(), Some(said));
    assert!(view.take_boarding().is_some(), "plundered");
    assert_lasts(&mut view, said, BOARD_LINE_TICKS);
}

/// The refused pickup's dialog (`_DoMissionCargoPickup` @0x982ff) is
/// queued as the B press returns, so the router shows it over flight
/// as it settles that input, and the comm line says #130.
#[test]
fn b_on_a_pickup_ship_without_room_shows_the_dialog_at_once_and_says_cant_board() {
    let mut view = rescuing(true);
    tap(&mut view, BOARD_KEY);
    assert_eq!(view.message(), Some(CANT_BOARD));
    let texts: Vec<(MissionTextKind, String)> = view
        .take_mission_texts()
        .into_iter()
        .map(|shown| match shown {
            MissionShown::Text(text) => (text.kind, text.text),
            MissionShown::Movie(movie) => panic!("an offer movie: {movie:?}"),
        })
        .collect();
    assert_eq!(
        texts,
        [(
            MissionTextKind::NoRoom,
            nova_sim::mission::cargo::NO_SPACE_TO_LOAD.to_owned()
        )]
    );
    let session = view.session().expect("flying");
    assert!(!session.npcs()[0].boarded);
}

#[test]
fn a_mission_ship_targeted_shows_its_missions_name_and_subtitle_and_no_tag() {
    let mut hunted = trafficked(&[130], 0, 129, 3);
    hunted.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    hunted.missions = vec![MissionRecord {
        ships: MissionShip {
            count: 1,
            syst: -1,
            dude: Some(nova_sim::DudeId(128)),
            start: 0,
            name_id: Some(7000),
            subtitle: Some(7001),
            ..MissionShip::NONE
        },
        ..errand(140)
    }];
    hunted.mission_texts = vec![MissionTexts::named(MissionId(140), "Hunt")];
    let strings = FakeCatalog {
        strings: vec![
            (7000, vec!["Rover".to_owned()]),
            (7001, vec!["Bounty Hunter".to_owned()]),
        ],
        ..catalog()
    };
    let mut view = FlightView::new(hunted)
        .with_strings(Rc::new(strings))
        .with_behaviour(Rc::new(Still));
    land_now(&mut view);
    view.settle_script();
    assert_eq!(view.accept_mission(MissionId(140)), Ok(0));
    view.settle_script();
    assert!(view.take_off().is_some());
    view.settle_script();
    view.tick(TICK);
    let session = view.session().expect("flying");
    assert!(
        session.npcs().iter().any(|npc| npc.mission.is_some()),
        "the mission ship is in the system"
    );
    tap(&mut view, Key::Tab);
    let texts: Vec<String> = drawn(&view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|text| text == "Rover"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text == "Bounty Hunter"),
        "{texts:?}"
    );
    assert!(!texts.iter().any(|text| text == "Ship 129"), "{texts:?}");
    assert!(
        !texts
            .iter()
            .any(|text| text.trim() == "Sp" || text.trim() == "Df"),
        "no cheats-only tag: {texts:?}"
    );
}

#[test]
fn a_hail_quote_shows_in_the_message_line_for_420_ticks() {
    let (_, chance) = scripted(&ACE);
    let mut view = FlightView::new(peopled())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    assert_eq!(
        view.message(),
        Some("Ace: Prepare to die, !"),
        "an unnamed pilot"
    );
    assert_lasts(
        &mut view,
        "Ace: Prepare to die, !",
        nova_sim::person::QUOTE_SHOWN_TICKS,
    );
}

#[test]
fn in_double_time_a_person_rolls_its_hail_quote_on_both_passes() {
    // `_HandleShip` (@0x44483) runs on both passes and its quote block
    // (@0x33f0f-0x33fba) tests no pass.
    let (script, chance) = scripted(&[99, 750, 650, 0, 1, 0]);
    let mut view = FlightView::new(peopled())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_double_time(true);
    view.tick(TICK);
    assert_eq!(view.message(), Some("Ace: Prepare to die, !"));
    let asked = &script.borrow().asked;
    assert_eq!(asked[asked.len() - 2..], [140, 140], "{asked:?}");
}

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
fn a_move_in_flight_lays_out_the_system_moved_to_once_settled() {
    let mut view = flight();
    view.catalog().systems_read.borrow_mut().clear();
    run_set(&mut view, "M131");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    view.settle_script();
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "on Proxima");
    assert_eq!(view.message(), None);
    view.settle_script();
    assert_eq!(
        *view.catalog().systems_read.borrow(),
        [SystemId(131)],
        "nothing more to settle"
    );
}

#[test]
fn an_h_from_a_set_expression_redraws_the_new_hull_once_settled() {
    let mut view = FlightView::new(shipbuying());
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    run_set(&mut view, "H129");
    assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
    view.settle_script();
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)]
    );
    let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
    assert_eq!(ids, [2001, 2201], "ship 129's sheet, and its lights");
    assert_eq!(view.frame(), Some(0), "of 72 rotations");
}

#[test]
fn an_outfits_on_purchase_h_redraws_the_new_hull() {
    let mut catalog = shipbuying();
    catalog.outfits[0].on_purchase = nova_sim::Script::parse("H129");
    let mut view = FlightView::new(catalog);
    land_now(&mut view);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
    view.settle_script();
    view.settle_script();
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)],
        "once"
    );
    view.take_off().expect("took off");
    let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
    assert_eq!(ids, [2001, 2201], "the new hull");
}

#[test]
fn settling_reads_no_sheet_when_the_class_is_unchanged() {
    let mut view = FlightView::new(shipbuying());
    view.settle_script();
    ticks(&mut view, 3);
    run_set(&mut view, "b1");
    view.settle_script();
    view.tick(TICK);
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
}

#[test]
fn a_move_within_the_system_lays_it_out_afresh() {
    let mut view = flight();
    view.catalog().systems_read.borrow_mut().clear();
    run_set(&mut view, "N130");
    view.settle_script();
    assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(130)]);
}

#[test]
fn a_move_while_landed_is_laid_out_once_the_ship_takes_off() {
    let mut view = landed_view();
    run_set(&mut view, "M131");
    view.settle_script();
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    assert_eq!(*view.catalog().systems_read.borrow(), []);
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
    assert!(
        view.session()
            .expect("flying")
            .pilot()
            .has_charted(SystemId(131)),
        "the take-off charts"
    );
}

#[test]
fn a_q_in_flight_shows_its_message() {
    let mut view = told(flight());
    run_set(&mut view, "Q25048");
    view.settle_script();
    assert!(!view.leave_due());
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ScriptMessage)]);
    assert_lasts(&mut view, "Off you go, <PSN>.", SCRIPT_MESSAGE_TICKS);
}

#[test]
fn a_landed_q_waits_for_the_take_off_which_shows_its_message() {
    let mut view = told(landed_view());
    run_set(&mut view, "M131 Q25048");
    view.settle_script();
    let session = view.session().expect("flying");
    assert_eq!(session.landed(), Some(StellarId(128)), "held");
    assert!(view.leave_due());
    assert_eq!(view.message(), None, "nothing yet");
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert!(!view.leave_due());
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
    assert_lasts(&mut view, "Off you go, <PSN>.", 500);
}

#[test]
fn a_due_leave_takes_off_as_the_ports_leave_does() {
    let mut view = told(landed_view());
    assert_eq!(view.leave_if_due(), None, "nothing due");
    assert_eq!(view.session().expect("ok").landed(), Some(StellarId(128)));
    run_set(&mut view, "M131 Q25048");
    view.settle_script();
    assert_eq!(view.leave_if_due(), Some(StellarId(128)));
    assert_eq!(view.session().expect("ok").landed(), None, "in flight");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_lasts(&mut view, "Off you go, <PSN>.", 500);
    assert_eq!(view.leave_if_due(), None, "once");
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

/// [`peopled`] with Ace of `flags` linked to mission 130, offered
/// from a ship, its offer text `offer`, refusable unless
/// `unrefusable`.
fn offering(flags: u16, offer: Option<&str>, unrefusable: bool) -> FakeCatalog {
    let mut catalog = peopled();
    catalog.persons[0].flags = flags;
    catalog.persons[0].link_mission = Some(130);
    catalog.missions = vec![nova_sim::catalog::MissionRecord {
        id: MissionId(130),
        avail_stel: -1,
        avail_loc: 2,
        avail_record: 0,
        avail_rating: 0,
        avail_random: 100,
        avail_ship_type: 0,
        avail_bits: nova_sim::Test::default(),
        require: 0,
        flags: if unrefusable { 0x0004 } else { 0 },
        flags2: 0,
        travel_stel: -1,
        return_stel: -1,
        cargo_type: -1,
        cargo_qty: 0,
        pickup_mode: -1,
        drop_off_mode: -1,
        scan_mask: 0,
        pay_val: 0,
        time_limit: 0,
        disp_weight: 0,
        on_accept: nova_sim::Script::default(),
        on_refuse: nova_sim::Script::default(),
        on_success: nova_sim::Script::default(),
        on_failure: nova_sim::Script::default(),
        on_abort: nova_sim::Script::default(),
        on_ship_done: nova_sim::Script::default(),
        can_abort: 1,
        comp_govt: None,
        comp_reward: 0,
        date_post_inc: 0,
        ships: nova_sim::catalog::MissionShip::NONE,
        aux: nova_sim::catalog::AuxShip::NONE,
    }];
    catalog.mission_texts = vec![nova_sim::catalog::MissionTexts {
        offer: offer.map(str::to_owned),
        ..nova_sim::catalog::MissionTexts::named(MissionId(130), "Help Ace")
    }];
    catalog
}

/// `catalog`'s flight with Ace listed and placed 100 above the
/// player, targeted.
fn targeting_ace(catalog: FakeCatalog) -> View {
    let (_, chance) = scripted(&ACE[..4]);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    tap(&mut view, TARGET_KEY);
    view
}

#[test]
fn y_on_a_person_offering_its_mission_reports_the_offer_once() {
    let mut view = targeting_ace(offering(0, Some("<OSN> needs help."), false));
    assert_eq!(view.take_offer(), None);
    view.input(&key(Key::Up, true));
    tap(&mut view, HAIL_KEY);
    assert_eq!(view.take_hail(), None, "no comm dialog");
    let prompt = view.take_offer().expect("offered");
    assert_eq!(prompt.mission, MissionId(130));
    assert_eq!(prompt.text, "Ace needs help.");
    assert_eq!(view.take_offer(), None, "once");
    view.tick(TICK);
    assert_eq!(
        view.session().expect("flying").player().velocity,
        Vec2::ZERO,
        "the flight keys let go"
    );
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    assert_eq!(
        view.pilot().expect("a pilot").missions().missions(),
        [MissionId(130)]
    );
}

#[test]
fn i_with_no_mission_says_you_have_no_active_missions() {
    let mut view = flight();
    let _ = view.take_sounds();
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "_beepSnd[3], snd 153"
    );
    assert_lasts(&mut view, NO_ACTIVE_MISSIONS, 240);
}

#[test]
fn i_in_a_flight_that_never_started_says_no_active_missions_without_a_beep() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert!(view.session().is_err());
    let _ = view.take_sounds();
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS));
    assert_eq!(view.take_sounds(), [], "no session to beep");
}

#[test]
fn i_with_an_active_mission_asks_for_the_info_screen_once() {
    let mut view = targeting_ace(offering(0, Some("Help?"), false));
    tap(&mut view, HAIL_KEY);
    view.take_offer().expect("offered");
    assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
    view.input(&key(Key::Up, true));
    let _ = view.take_sounds();
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.take_sounds(), [], "no beep from flight");
    let info = view.take_mission_info_request().expect("asked");
    assert_eq!(info, view.mission_info());
    assert_eq!(view.take_mission_info_request(), None, "once");
    assert_eq!(view.message(), None);
    view.tick(TICK);
    assert_eq!(
        view.session().expect("flying").player().velocity,
        Vec2::ZERO,
        "the flight keys let go"
    );
}

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
fn l_while_disabled_selects_a_stellar_and_then_says_nothing_and_does_not_land() {
    let mut view = disabled_player();
    assert_eq!(nav_target(&view), None);
    tap(&mut view, LAND_KEY);
    let selected = nav_target(&view);
    assert!(selected.is_some(), "the nearest selected (@0x6a901)");
    assert_eq!(view.message(), None, "no clearance reply");
    tap(&mut view, LAND_KEY);
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.message(), None, "refused silently (@0x66af7)");
    assert_eq!(nav_target(&view), selected, "kept");
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

#[test]
fn an_offer_keeps_the_message() {
    let mut view = targeting_ace(offering(0, Some("Help?"), false));
    view.say_for(MISSION_FAILED, 240);
    tap(&mut view, HAIL_KEY);
    view.take_offer().expect("offered");
    assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
}

#[test]
fn a_refusal_of_the_offer_goes_through_the_session() {
    let mut view = targeting_ace(offering(0, Some("Help?"), false));
    tap(&mut view, HAIL_KEY);
    view.take_offer().expect("offered");
    assert_eq!(view.refuse_mission(MissionId(130)).map(|_| ()), Ok(()));
    assert_eq!(
        view.refuse_mission(MissionId(130)),
        Err(NotOffered),
        "answered"
    );
}

/// How a person's offer is answered over flight: accepted or refused
/// in the dialog with a refuse button, or accepted in the one-button
/// dialog.
const FLIGHT_ANSWERS: [(&str, bool, bool); 3] = [
    ("an accept", false, false),
    ("a refusal", false, true),
    ("a one-button accept", true, false),
];

/// [`offering`] Ace's refusable (or `unrefusable`) mission 130, with
/// a `BriefText` and a `RefuseText`, whose `OnAccept` (or, `refuse`,
/// `OnRefuse`) is `G200`, a tank: hailed and its offer taken, in
/// flight with the nearest stellar, the Moon, selected, the HUD's
/// cadence read as `cadence` says.
fn offered_in_flight(cadence: RuleSource, unrefusable: bool, refuse: bool) -> View {
    let mut catalog = offering(0, Some("Help?"), unrefusable);
    catalog.outfits = vec![shipbuying().outfits[0].clone()];
    let script = nova_sim::Script::parse("G200");
    let mission = &mut catalog.missions[0];
    if refuse {
        mission.on_refuse = script;
    } else {
        mission.on_accept = script;
    }
    let texts = &mut catalog.mission_texts[0];
    texts.brief = Some("Go.".to_owned());
    texts.refuse = Some("No.".to_owned());
    let (_, chance) = scripted(&ACE[..4]);
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, cadence);
    let mut view = FlightView::new(catalog)
        .with_rules(rules)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    tap(&mut view, NEAREST_STELLAR_KEY);
    tap(&mut view, TARGET_KEY);
    tap(&mut view, HAIL_KEY);
    view.take_offer().expect("offered");
    view
}

#[test]
fn an_offer_answered_over_flight_shows_its_text_over_the_pre_answer_bar() {
    for (what, unrefusable, refuse) in FLIGHT_ANSWERS {
        let mut view = offered_in_flight(RuleSource::Engine, unrefusable, refuse);
        let before = view.reserves();
        let nav = view.nav_shown();
        if refuse {
            assert_eq!(
                view.refuse_mission(MissionId(130)).map(|_| ()),
                Ok(()),
                "{what}"
            );
        } else {
            assert_eq!(view.accept_mission(MissionId(130)), Ok(0), "{what}");
        }
        assert_eq!(
            view.reserves().fuel.max,
            before.fuel.max + 100.0,
            "{what}: G200 ran"
        );
        // A selection the bar hasn't drawn: an `M` or `N` in the
        // answer can't stand for it, as its move lays the system out
        // afresh as it settles, which redraws the whole bar.
        view.select_stellar(StellarPick::Slot(0));
        assert_ne!(view.nav_now(), nav, "{what}: Earth selected");
        assert_eq!(view.take_mission_texts().len(), 1, "{what}: its text");
        assert_eq!(view.reserves_shown(), before, "{what}: under the text");
        assert_eq!(view.nav_shown(), nav, "{what}: the nav held too");
        view.close_mission_text();
        assert_eq!(
            view.reserves_shown(),
            view.reserves(),
            "{what}: live once it closed"
        );
        assert_eq!(view.nav_shown(), view.nav_now(), "{what}: nav live too");
        assert!(view.bar_steps.is_empty(), "{what}: let go");
    }
}

#[test]
fn under_the_bible_cadence_flights_gauges_stay_live_under_an_offers_text() {
    for (what, unrefusable, refuse) in FLIGHT_ANSWERS {
        let mut view = offered_in_flight(RuleSource::Bible, unrefusable, refuse);
        if refuse {
            assert_eq!(
                view.refuse_mission(MissionId(130)).map(|_| ()),
                Ok(()),
                "{what}"
            );
        } else {
            assert_eq!(view.accept_mission(MissionId(130)), Ok(0), "{what}");
        }
        assert_eq!(view.take_mission_texts().len(), 1, "{what}: its text");
        assert_eq!(view.reserves_shown(), view.reserves(), "{what}: live");
    }
}

#[test]
fn b_on_a_person_offering_on_boarding_reports_the_offer_and_no_plunder() {
    let mut catalog = granting(2);
    let ace = &mut catalog.persons[0];
    ace.flags = 0x0200;
    ace.link_mission = Some(130);
    catalog.missions = offering(0, None, false).missions;
    catalog.mission_texts = offering(0, Some("Aboard."), false).mission_texts;
    let (_, chance) = scripted(&[99, 750, 750, 0]);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_boarding_rule(Rc::new(Grants));
    view.tick(TICK);
    tap(&mut view, TARGET_KEY);
    tap(&mut view, BOARD_KEY);
    assert_eq!(view.take_boarding(), None, "no plunder dialog");
    assert_eq!(
        view.take_offer().map(|prompt| prompt.text),
        Some("Aboard.".to_owned())
    );
    assert_eq!(view.message(), None, "nothing granted");
}

#[test]
fn an_offer_accepted_at_once_queues_its_before_movie_first() {
    let mut catalog = offering(0, None, true);
    catalog.mission_texts[0].movies.offer = nova_sim::DescMovie::from_desc("Ace.mov", 0x0004);
    let mut view = targeting_ace(catalog);
    tap(&mut view, HAIL_KEY);
    assert_eq!(view.take_offer(), None);
    assert_eq!(
        view.take_mission_texts().first(),
        nova_sim::DescMovie::from_desc("Ace.mov", 0x0004)
            .map(nova_sim::MissionShown::Movie)
            .as_ref()
    );
}

#[test]
fn an_offer_accepted_at_once_shows_nothing() {
    let mut view = targeting_ace(offering(0, None, true));
    tap(&mut view, HAIL_KEY);
    assert_eq!(view.take_offer(), None);
    assert_eq!(view.take_hail(), None);
    assert_eq!(
        view.pilot().expect("a pilot").missions().missions(),
        [MissionId(130)]
    );
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
        sim_message_text(&stolen, session),
        Some("2 tons of cargo and 1.23M credits stolen!".to_owned())
    );
    assert_eq!(
        sim_message_text(&SimMessage::ShipStolen { fighter: true }, session),
        Some("Fighter stolen!".to_owned())
    );
    assert_eq!(arrival_lead(&stolen), None);
}

#[test]
fn the_self_destructs_lines_say_its_stock_words() {
    use nova_sim::SelfDestructLine;
    let view = flight();
    let session = view.session().expect("flying");
    let text = |message: SimMessage| sim_message_text(&message, session);
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
