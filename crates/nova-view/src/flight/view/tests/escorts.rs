use super::*;

// Escorts.

#[test]
fn each_key_asks_the_session_for_its_command() {
    let mut view = fighters_out();
    let commands = [
        (LAND_KEY, PlayerCommand::Land),
        (BOARD_KEY, PlayerCommand::Board),
        (HAIL_KEY, PlayerCommand::Hail),
        (MAP_KEY, PlayerCommand::Map),
        (JUMP_KEY, PlayerCommand::Jump),
        (JETTISON_KEY, PlayerCommand::Jettison),
        (EJECT_KEY, PlayerCommand::Eject),
        (MISSION_INFO_KEY, PlayerCommand::MissionInfo),
        (HYPER_SELECT_KEY, PlayerCommand::SelectSystem),
        (HYPERSPACE_MODE_KEY, PlayerCommand::Nav),
        (TARGET_KEY, PlayerCommand::Target),
        (NEAREST_KEY, PlayerCommand::Target),
        (MENU_KEY, PlayerCommand::Escort),
        (MENU_CLOSE_KEY, PlayerCommand::Escort),
        (ESCORT_ATTACK_KEY, PlayerCommand::Escort),
        (ESCORT_DEFEND_KEY, PlayerCommand::Escort),
        (ESCORT_HOLD_KEY, PlayerCommand::Escort),
        (ESCORT_RECALL_KEY, PlayerCommand::Escort),
        (SELECT_KEY, PlayerCommand::SelectWeapon),
        (DESELECT_KEY, PlayerCommand::DeselectWeapon),
        (NEAREST_STELLAR_KEY, PlayerCommand::Nav),
        (NAV_OFF_KEY, PlayerCommand::Nav),
        (Key::Function(1), PlayerCommand::Nav),
        (ZOOM_OUT_KEY, PlayerCommand::Nav),
        (ZOOM_IN_KEY, PlayerCommand::Nav),
        (ZOOM_IN_KEYPAD_KEY, PlayerCommand::Nav),
    ];
    for (k, command) in commands {
        assert_eq!(view.command_of(k), Some(command), "{k:?}");
    }
    for k in [
        FIRE_KEY,
        Key::Up,
        AFTERBURNER_KEY,
        Key::Char('q'),
        Key::Other,
    ] {
        assert_eq!(view.command_of(k), None, "{k:?}");
    }
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.command_of(Key::Char('1')), Some(PlayerCommand::Nav));
    assert_eq!(view.command_of(GROUP_KEYS[4]), None, "5 is no stellar slot");
    view.input(&key(Key::Command, true));
    assert_eq!(
        view.command_of(NEAREST_STELLAR_COMMAND_KEY),
        Some(PlayerCommand::Nav),
        "Command-5"
    );
    view.input(&key(Key::Command, false));
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.command_of(Key::Char('1')), Some(PlayerCommand::Escort));
    assert_eq!(view.command_of(GROUP_KEYS[4]), Some(PlayerCommand::Escort));
    assert_eq!(view.command_of(Key::Char('q')), None);
}

/// `catalog` with `STR#` 1000 "Message Buoy: hello." and Alpha
/// Centauri's buoy `message`.
fn buoyed(mut catalog: FakeCatalog, message: i16) -> FakeCatalog {
    catalog
        .strings
        .push((nova_sim::buoy::BUOY_STRINGS, vec![HELLO.to_owned()]));
    catalog.alpha_buoy = message;
    catalog
}

/// How many ticks the message `view` shows stays shown.
fn shown_for(view: &mut View) -> u32 {
    let mut n = 0;
    while view.message().is_some() {
        view.tick(TICK);
        n += 1;
        assert!(n < 10_000, "never cleared");
    }
    n
}

#[test]
fn a_jump_into_a_buoyed_system_shows_the_buoy_for_480_ticks_in_place_of_the_arrival() {
    let mut view = FlightView::new(buoyed(catalog(), 1));
    jump_to_alpha(&mut view);
    assert_eq!(view.message(), Some(HELLO));
    let buoy = shown_for(&mut view);
    let mut plain = FlightView::new(buoyed(catalog(), -1));
    jump_to_alpha(&mut plain);
    assert_eq!(
        plain.message(),
        Some("Jumping into the Alpha Centauri system on June 24th, 1177 NC.")
    );
    // The arrival's 240 ticks, less those the jump's end took.
    let arrival = shown_for(&mut plain);
    assert_eq!(buoy, arrival + 240, "480 ticks from the arrival");
}

#[test]
fn an_empty_buoy_replaces_the_line_shown_before_the_jump() {
    let mut view = FlightView::new(buoyed(catalog(), 2));
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    view.say_for("Before the jump.", 10_000);
    arrive_now(&mut view);
    assert_eq!(view.message(), Some(""));
}

#[test]
fn fighters_abandoned_into_a_buoyed_system_are_not_told() {
    let mut view = fighters_out_of(buoyed(boardable(), 1));
    jump_to_alpha(&mut view);
    assert_eq!(view.message(), Some(HELLO));
    assert_eq!(
        view.pilot().expect("flying").escorts(),
        [],
        "still abandoned"
    );
}

#[test]
fn fighters_abandoned_in_a_jump_are_told_on_arrival() {
    let mut view = fighters_out();
    jump_to_alpha(&mut view);
    assert_eq!(
        view.message(),
        Some(
            "Jumping into the Alpha Centauri system on June 24th, 1177 NC.  (Two fighters abandoned)"
        )
    );
    assert_eq!(view.pilot().expect("flying").escorts(), []);
}

#[test]
fn with_the_escort_menu_open_1_to_4_pick_groups_not_stellars() {
    let mut view = escorted();
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    tap_slot(&mut view, Key::Char('2'));
    assert_eq!(nav_target(&view), None, "the menu took it");
    tap_slot(&mut view, Key::Function(2));
    assert_eq!(nav_target(&view), Some(StellarId(129)), "F2 never does");
    tap(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open());
    tap_slot(&mut view, Key::Char('1'));
    assert_eq!(nav_target(&view), Some(StellarId(128)), "shut: a stellar");
    assert_eq!(view.escort_menu().group(), EscortGroup::All);
}

#[test]
fn with_the_escort_menu_open_command_5_picks_freighters_not_a_stellar() {
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.escort_type = 3;
            record.inherent_ai = 3;
        }
    }
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = escorted_flight(catalog, "Freighted", fleet(&[130], false))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    view.take_sounds();
    command_tap(&mut view, NEAREST_STELLAR_COMMAND_KEY, false);
    ticks(&mut view, 1);
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Freighter)
    );
    assert_eq!(nav_target(&view), None, "no stellar");
    assert_eq!(ui_sounds(&mut view), [UiSound::EscortGroupSelected]);
    tap(&mut view, MENU_CLOSE_KEY);
    command_tap(&mut view, NEAREST_STELLAR_COMMAND_KEY, false);
    ticks(&mut view, 1);
    let nearest = nav_target(&view);
    assert!(nearest.is_some(), "shut: the nearest");
    tap(&mut view, NAV_OFF_KEY);
    tap_slot(&mut view, NEAREST_STELLAR_KEY);
    assert_eq!(nav_target(&view), nearest, "as F5's");
}

/// [`escorted`], its course to Alpha Centauri plotted and the ship
/// flown out far enough to jump.
fn escorted_out() -> View {
    let mut view = escorted();
    plot(&mut view, 131);
    fly_out(&mut view);
    view
}

#[test]
fn escort_keys_are_taken_while_the_jump_plays() {
    let mut view = escorted_out();
    jump_now(&mut view);
    view.tick(ms(300));
    view.take_sounds();
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(orders(&view), [Some(EscortOrder::Hold)]);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships will hold position.")
    );
    assert!(
        view.take_sounds()
            .contains(&Sound::Sim(SimSound::EscortsCommanded)),
        "_beepSnd[1]"
    );
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(ui_sounds(&mut view), [UiSound::EscortMenuToggled]);
    tap(&mut view, GROUP_KEYS[3]);
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Warship)
    );
    assert_eq!(ui_sounds(&mut view), [UiSound::EscortGroupSelected]);
    tap(&mut view, ESCORT_DEFEND_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  Warships will defend.")
    );
    tap(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open(), "Return closes it");
    assert!(view.jump_effect().is_some(), "the streak plays on");
    arrive_now(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
}

#[test]
fn escort_keys_are_taken_while_the_ship_brakes_to_jump() {
    let mut view = escorted_out();
    view.input(&key(JUMP, true));
    assert!(view.preparing_jump(), "braking");
    tap(&mut view, ESCORT_DEFEND_KEY);
    assert_eq!(orders(&view), [Some(EscortOrder::Defend)]);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships will defend.")
    );
}

#[test]
fn option_c_during_the_jump_is_return_to_hangar() {
    let mut view = escorted_out();
    tap(&mut view, ESCORT_HOLD_KEY);
    jump_now(&mut view);
    view.input(&key(Key::Alt, true));
    view.tick(ms(300));
    tap(&mut view, ESCORT_RECALL_KEY);
    view.input(&key(Key::Alt, false));
    assert_eq!(orders(&view), [None], "Return to Hangar: no fighter");
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships will return to formation.")
    );
    tap(&mut view, ESCORT_HOLD_KEY);
    tap(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(orders(&view), [None]);
    let mut fighters = fighters_out();
    plot(&mut fighters, 131);
    fly_out(&mut fighters);
    jump_now(&mut fighters);
    option(&mut fighters, ESCORT_RECALL_KEY);
    assert_eq!(orders(&fighters), [Some(EscortOrder::Dock); 2]);
    assert_eq!(
        fighters.message(),
        Some("New escort orders assigned:  All ships will return to hangar.")
    );
}

#[test]
fn option_released_during_the_jump_lets_c_recall() {
    let mut view = fighters_out();
    plot(&mut view, 131);
    fly_out(&mut view);
    tap(&mut view, ESCORT_HOLD_KEY);
    jump_now(&mut view);
    view.input(&key(Key::Alt, true));
    view.tick(ms(300));
    view.input(&key(Key::Alt, false));
    tap(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(orders(&view), [None; 2], "C alone: formation, not Dock");
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships will return to formation.")
    );
}

#[test]
fn the_escort_menu_times_out_during_the_streak() {
    // How long the ship brakes and turns, timed on a twin.
    let mut twin = escorted_out();
    let mut braking = Duration::ZERO;
    twin.input(&key(JUMP, true));
    while twin.jump_effect().is_none() {
        twin.tick(TICK);
        braking += TICK;
    }
    // Opened so that it times out half a second into hyperspace.
    let mut view = escorted_out();
    tap(&mut view, MENU_KEY);
    let wait = super::super::super::escorts::MENU_TIMEOUT
        .checked_sub(braking + ms(500))
        .expect("the braking is shorter than the menu's timeout");
    view.tick(wait);
    jump_now(&mut view);
    assert!(view.escort_menu().is_open(), "open as the streak begins");
    let in_hyperspace = |view: &View| view.session().expect("flying").jumping().is_some();
    while view.escort_menu().is_open() {
        assert!(in_hyperspace(&view), "shut before the ship arrives");
        view.tick(TICK);
    }
    assert!(in_hyperspace(&view), "shut within the streak");
}

#[test]
fn the_help_line_names_the_escort_keys() {
    assert!(
        HELP.contains("F/D/V/C: escorts   Alt-C: dock   E: escort menu"),
        "{HELP}"
    );
}

// Hiring escorts and paying them.

/// The adapter's own check (rdm:phase/view-sim-seam/phase-11-spaceport-on-port):
/// it hands the flight's chance to a rolled read, the hire list, and to
/// a command, the hire. What the view does with the answers runs on
/// `FakeFlight` (`tests/faked_shops.rs`).
#[test]
fn hiring_goes_through_the_session_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let mut view = FlightView::new(hiring()).with_chance(SharedChance::new(shared));
    assert_eq!(view.escorts_for_hire(), None, "in flight");
    assert_eq!(view.hire(ShipId(129)), Err(HireRefusal::NoBar));
    land_now(&mut view);
    view.take_save_due();
    let list = view.escorts_for_hire().expect("a bar");
    let row = list.row(ShipId(129)).expect("for hire");
    assert_eq!((row.fee, row.wage), (90, 9));
    assert_eq!(always.borrow().asked, [50]);
    let hired = view.hire(ShipId(129)).expect("hired");
    assert_eq!((hired.fee, hired.wage), (90, 9));
    assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 90));
    assert_eq!(view.pilot().expect("flying").escorts()[0].wage, Some(9));
    assert!(view.take_save_due());
}

/// The adapter's own check: it hands the flight's chance to the bar's
/// opening, whose news it reads back.
#[test]
fn the_bars_news_is_drawn_through_the_session_on_the_flights_chance() {
    let always = Rc::new(RefCell::new(Always::default()));
    let shared: Rc<RefCell<dyn Chance>> = always.clone();
    let strings = FakeCatalog {
        strings: vec![
            (8100, vec!["C1".to_owned(), "C2".to_owned()]),
            (8101, vec!["G1".to_owned(), "G2".to_owned()]),
        ],
        ..catalog()
    };
    let mut view = FlightView::new(hiring())
        .with_chance(SharedChance::new(shared))
        .with_strings(Rc::new(strings));
    assert!(!view.open_bar(), "in flight");
    assert_eq!(view.news(), None);
    assert_eq!(view.news_picture(), None, "in flight");
    land_now(&mut view);
    assert_eq!(view.news_picture(), Some(9000), "no NewsPic");
    assert!(view.open_bar());
    assert_eq!(
        view.news(),
        Some(&nova_sim::News {
            commercial: "C2".to_owned(),
            story: "G2".to_owned(),
        }),
        "each the last string, as the flight's chance rolls"
    );
}
