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
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.command_of(Key::Char('1')), Some(PlayerCommand::Escort));
    assert_eq!(view.command_of(GROUP_KEYS[4]), Some(PlayerCommand::Escort));
    assert_eq!(view.command_of(Key::Char('q')), None);
}

#[test]
fn shutting_the_escort_menu_closes_it() {
    let mut view = fighters_out();
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    view.shut_escort_menu();
    assert!(!view.escort_menu().is_open());
    view.shut_escort_menu();
    assert!(!view.escort_menu().is_open(), "shut stays shut");
}

#[test]
fn option_c_sends_the_fighters_home_and_c_alone_back_to_formation() {
    let mut view = fighters_out();
    option(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(orders(&view), [Some(EscortOrder::Dock); 2]);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships returning to hangar.")
    );
    tap(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(orders(&view), [None; 2], "C alone recalls");
    tap(&mut view, MENU_KEY);
    tap(&mut view, GROUP_KEYS[1]);
    option(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  Fighters returning to hangar.")
    );
}

#[test]
fn option_c_sends_an_escort_of_its_own_back_to_formation() {
    let mut view = escorted();
    tap(&mut view, ESCORT_HOLD_KEY);
    option(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(orders(&view), [None]);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships returning to formation.")
    );
}

#[test]
fn the_escort_menu_shows_return_to_hangar_for_fighters_returning() {
    let mut view = fighters_out();
    option(&mut view, ESCORT_RECALL_KEY);
    tap(&mut view, MENU_KEY);
    assert!(
        texts(&drawn(&view)).contains(&"Return to Hangar".to_owned()),
        "{:?}",
        texts(&drawn(&view))
    );
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
fn the_players_escort_is_bracketed_green() {
    let mut view = escorted();
    option(&mut view, TARGET_KEY);
    assert_eq!(target_of(&view), Some(NpcId(1)));
    assert_eq!(bracket_icons(&drawn(&view)), [10016, 10017, 10018, 10019]);
}

#[test]
fn the_players_fighter_is_bracketed_green() {
    let mut view = fighters_out();
    option(&mut view, TARGET_KEY);
    assert_eq!(target_of(&view), Some(NpcId(1)));
    assert_eq!(bracket_icons(&drawn(&view)), [10016, 10017, 10018, 10019]);
}

/// Where the target panel's label `label` is drawn, if it is: its
/// top-left.
fn label_drawn(view: &View, label: &str) -> Option<Point> {
    drawn(view).iter().find_map(|command| match command {
        DrawCommand::Text { text, origin, .. } if text == label => Some(*origin),
        _ => None,
    })
}

/// Where `view` sets a label `label` against the target panel's
/// bottom right, measured by `MonoMetrics`: 7 in from the right,
/// on the baseline 6 above the bottom.
fn label_place(view: &View, label: &str) -> Point {
    let bar = view.status_bar().expect("a status bar");
    let layout = &bar.layout;
    let area = layout.targ.offset(hud::bar_origin(bar, VIEW_SIZE.0));
    let width = crate::text::TextMetrics::width(
        &crate::text::fixture::MonoMetrics,
        layout.font,
        layout.font_size,
        label,
    );
    at(
        area.max.x - 7.0 - width,
        area.max.y - 6.0 - layout.font_size,
    )
}

#[test]
fn the_target_panel_labels_the_players_escort_and_fighter_and_omits_a_missing_code() {
    let mono = || Rc::new(crate::text::fixture::MonoMetrics);
    let mut view = escorted().with_metrics(mono());
    option(&mut view, TARGET_KEY);
    assert_eq!(target_of(&view), Some(NpcId(1)));
    assert_eq!(
        label_drawn(&view, target::ESCORT),
        Some(label_place(&view, target::ESCORT)),
        "Escort"
    );
    let mut view = fighters_out().with_metrics(mono());
    option(&mut view, TARGET_KEY);
    assert_eq!(target_of(&view), Some(NpcId(1)));
    assert_eq!(
        label_drawn(&view, target::FIGHTER),
        Some(label_place(&view, target::FIGHTER)),
        "Fighter"
    );
    assert_eq!(label_drawn(&view, target::ESCORT), None);
    // The traffic ship's government, 140, has the code "Fed.".
    let mut view = fighting(armed(1, &[]), &placed(750, 650, 180)).with_metrics(mono());
    tap(&mut view, TARGET_KEY);
    assert_eq!(
        label_drawn(&view, "Fed."),
        Some(label_place(&view, "Fed.")),
        "with a code"
    );
    let uncoded = FakeCatalog {
        codes: Vec::new(),
        ..armed(1, &[])
    };
    let mut view = fighting(uncoded, &placed(750, 650, 180)).with_metrics(mono());
    tap(&mut view, TARGET_KEY);
    assert!(target_of(&view).is_some());
    let bottom = label_place(&view, "").y;
    let at_bottom: Vec<String> = drawn(&view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if origin.y == bottom => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(at_bottom, ["Shield: ", "100%"], "no code drawn");
}

#[test]
fn f_d_v_and_c_command_every_escort_and_say_what_changed() {
    let mut view = escorted();
    tap(&mut view, TARGET_KEY);
    assert_eq!(
        view.session().expect("flying").target().map(|npc| npc.id),
        Some(NpcId(0)),
        "Tab skips the escort"
    );
    tap(&mut view, ESCORT_ATTACK_KEY);
    assert_eq!(orders(&view), [Some(EscortOrder::Attack)]);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships attacking target.")
    );
    let escort = &view.session().expect("flying").npcs()[1];
    assert_eq!(escort.target, Some(ShipRef::Npc(NpcId(0))));
    tap(&mut view, ESCORT_DEFEND_KEY);
    assert_eq!(orders(&view), [Some(EscortOrder::Defend)]);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships defending.")
    );
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(orders(&view), [Some(EscortOrder::Hold)]);
    tap(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(orders(&view), [None]);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships returning to formation.")
    );
    assert_eq!(
        [
            ESCORT_ATTACK_KEY,
            ESCORT_DEFEND_KEY,
            ESCORT_HOLD_KEY,
            ESCORT_RECALL_KEY
        ],
        ['f', 'd', 'v', 'c'].map(Key::Char)
    );
}

#[test]
fn the_escort_menus_keys_beep_as_the_original_does() {
    let mut view = escorted();
    view.take_sounds();
    tap(&mut view, MENU_KEY);
    assert_eq!(ui_sounds(&mut view), [UiSound::EscortMenuToggled], "opened");
    let rows = view.session().expect("flying").escort_menu();
    let present = |n: usize| match EscortGroup::ALL[n] {
        EscortGroup::All => true,
        EscortGroup::Class(class) => rows.iter().any(|row| row.class == class && row.present),
    };
    let absent = (1..5).find(|&n| !present(n)).expect("an absent class");
    let own = (1..5).find(|&n| present(n)).expect("the escort's class");
    tap(&mut view, GROUP_KEYS[absent]);
    assert_eq!(ui_sounds(&mut view), [], "an absent class is not selected");
    tap(&mut view, GROUP_KEYS[own]);
    assert_eq!(ui_sounds(&mut view), [UiSound::EscortGroupSelected]);
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::EscortsCommanded)],
        "_beepSnd[1]"
    );
    tap(&mut view, MENU_KEY);
    assert_eq!(ui_sounds(&mut view), [UiSound::EscortMenuToggled], "closed");
    tap(&mut view, MENU_KEY);
    view.take_sounds();
    view.tick(super::super::super::escorts::MENU_TIMEOUT + TICK);
    assert!(!view.escort_menu().is_open());
    assert_eq!(ui_sounds(&mut view), [], "it fades silently");
}

#[test]
fn a_click_in_the_open_escort_menu_over_a_ship_does_nothing() {
    let menu = Point::new(MENU_AT.x + MENU_WIDTH / 2.0, MENU_AT.y + MENU_HEIGHT / 2.0);
    let under = escorted().camera().screen_to_world(menu);
    let place = |at: f32| u32::try_from((at + 750.0).round() as i64).expect("placeable");
    let mut view = escorted_with_traffic(place(under.x), place(under.y));
    let traffic = NpcId(0);
    let over = npc_on_screen(&view, traffic);
    assert!(
        (over.x - menu.x).abs() < 2.0 && (over.y - menu.y).abs() < 2.0,
        "{over:?}"
    );
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    view.take_sounds();
    click(&mut view, over);
    assert_eq!(target_of(&view), None, "not targeted");
    assert_eq!(view.take_sounds(), [], "silent");
    tap(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open());
    click(&mut view, over);
    assert_eq!(target_of(&view), Some(traffic), "shut, it targets");
}

#[test]
fn e_with_no_escorts_beeps_no_escorts() {
    let catalog = boardable();
    let pilot = Pilot::new(&catalog, "Alone").expect("starts");
    let mut view = FlightView::with_pilot(catalog, pilot).with_behaviour(Rc::new(Still));
    view.tick(TICK);
    view.take_sounds();
    tap(&mut view, MENU_KEY);
    assert_eq!(view.message(), Some(NO_ESCORTS));
    assert_eq!(ui_sounds(&mut view), [UiSound::NoEscorts]);
}

#[test]
fn a_command_is_acknowledged_in_the_escorts_voice_at_the_next_pass() {
    // `_IssueNewEscortCommand` queues the message (@0x6667a) and
    // `_HandleVoiceSound` plays it in the next advance pass (@0x44219).
    // Ship 130 has no attributes government, so it speaks set 0.
    let clip = nova_sim::SoundId(1000);
    let second = nova_sim::warp::SoundLength {
        frames: 22_050,
        rate: 22_050,
    };
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.escort_type = 2;
            record.inherent_ai = 3;
        }
    }
    catalog.voices = nova_sim::voice::Voices::read(&[(clip, Some(second))], &[]);
    let pilot = with_escorts(&Pilot::new(&catalog, "Escorted").expect("starts"), &[130]);
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::with_pilot(catalog, pilot)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    view.take_sounds();
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::EscortsCommanded)],
        "the beep at once"
    );
    view.tick(TICK);
    let voiced: Vec<Sound> = view
        .take_sounds()
        .into_iter()
        .filter(|sound| matches!(sound, Sound::Sim(SimSound::Voice { .. })))
        .collect();
    assert_eq!(voiced, [Sound::Sim(SimSound::Voice { sound: clip })]);
}

#[test]
fn a_command_that_changes_nothing_says_nothing() {
    let mut view = escorted();
    tap(&mut view, ESCORT_RECALL_KEY);
    assert_eq!(view.message(), None, "already in formation");
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_lasts(
        &mut view,
        "New escort orders assigned:  All ships holding position.",
        ESCORT_TICKS,
    );
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(view.message(), None, "held already");
}

#[test]
fn attack_with_no_target_will_attack() {
    let mut view = escorted();
    tap(&mut view, ESCORT_ATTACK_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  All ships will attack.")
    );
}

#[test]
fn with_the_escort_menu_open_1_to_4_pick_groups_not_stellars() {
    let mut view = escorted();
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    tap(&mut view, Key::Char('2'));
    assert_eq!(nav_target(&view), None, "the menu took it");
    tap(&mut view, Key::Function(2));
    assert_eq!(nav_target(&view), Some(StellarId(129)), "F2 never does");
    tap(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open());
    tap(&mut view, Key::Char('1'));
    assert_eq!(nav_target(&view), Some(StellarId(128)), "shut: a stellar");
    assert_eq!(view.escort_menu().group(), EscortGroup::All);
}

#[test]
fn return_clears_the_message_and_closes_the_menu() {
    let mut view = escorted();
    view.say_for(MISSION_FAILED, 240);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.message(), Some(MISSION_FAILED), "kept by the menu");
    tap(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.message(), None, "acknowledged");
    let mut view = flight();
    view.say_for(MISSION_FAILED, 240);
    tap(&mut view, MENU_CLOSE_KEY);
    assert_eq!(view.message(), None, "with the menu shut too");
}

#[test]
fn opening_the_escort_menu_clears_no_escorts_only() {
    let mut view = escorted();
    view.say_for(NO_ESCORTS, ESCORT_TICKS);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.message(), None, "#51 cleared");
    view.say_for(NO_ESCORTS, ESCORT_TICKS);
    tap(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.message(), Some(NO_ESCORTS), "closing does not clear");
    view.say_for(MISSION_FAILED, 240);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    assert_eq!(view.message(), Some(MISSION_FAILED), "kept");
}

#[test]
fn e_opens_the_menu_and_its_keys_pick_the_group_commanded() {
    let mut view = escorted();
    tap(&mut view, GROUP_KEYS[3]);
    assert!(!view.escort_menu().is_open(), "1-5 leave a shut menu alone");
    assert_eq!(view.escort_menu().group(), EscortGroup::All);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    tap(&mut view, GROUP_KEYS[1]);
    assert_eq!(view.escort_menu().group(), EscortGroup::All, "no fighters");
    tap(&mut view, GROUP_KEYS[3]);
    assert_eq!(
        view.escort_menu().group(),
        EscortGroup::Class(EscortClass::Warship)
    );
    tap(&mut view, ESCORT_HOLD_KEY);
    assert_eq!(
        view.message(),
        Some("New escort orders assigned:  Warships holding position.")
    );
    tap(&mut view, MENU_CLOSE_KEY);
    assert!(!view.escort_menu().is_open(), "Return closes it");
    tap(&mut view, MENU_KEY);
    tap(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open(), "E again closes it");
    assert_eq!(MENU_CLOSE_KEY, Key::Enter);
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
fn the_menu_closes_8_seconds_after_its_last_command() {
    let mut view = escorted();
    tap(&mut view, MENU_KEY);
    view.tick(Duration::from_secs(6));
    tap(&mut view, ESCORT_HOLD_KEY);
    view.tick(Duration::from_secs(6));
    assert!(view.escort_menu().is_open(), "the command kept it open");
    view.tick(Duration::from_secs(3));
    assert!(!view.escort_menu().is_open());
}

#[test]
fn e_with_no_escorts_says_so() {
    let mut view = flight();
    tap(&mut view, MENU_KEY);
    assert!(!view.escort_menu().is_open());
    assert_eq!(view.message(), Some("You don't have any escorts."));
    tap(&mut view, ESCORT_ATTACK_KEY);
    assert_eq!(
        view.message(),
        Some("You don't have any escorts."),
        "no command"
    );
    assert_lasts(&mut view, NO_ESCORTS, 250);
}

#[test]
fn option_tab_targets_the_escorts() {
    let mut view = escorted();
    view.input(&key(Key::Alt, true));
    tap(&mut view, TARGET_KEY);
    view.input(&key(Key::Alt, false));
    assert_eq!(
        view.session().expect("flying").target().map(|npc| npc.id),
        Some(NpcId(1))
    );
}

#[test]
fn shift_tab_targets_back_through_the_ships_and_with_alt_back_through_the_escorts() {
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.escort_type = 2;
            record.inherent_ai = 3;
        }
    }
    catalog.traffic[0].1.avg_ships = 2;
    let pilot = with_escorts(
        &Pilot::new(&catalog, "Escorted").expect("starts"),
        &[130, 130],
    );
    let mut draws = placed(750, 650, 0).to_vec();
    draws.extend(placed(850, 650, 0));
    let (_, chance) = scripted(&draws);
    let mut view = FlightView::with_pilot(catalog, pilot)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let session = view.session().expect("flying");
    let last = |escorts: bool| {
        session
            .npcs()
            .iter()
            .filter(|npc| session.is_escort(npc.id) == escorts)
            .max_by_key(|npc| npc.berth)
            .map(|npc| npc.id)
    };
    let (ship, escort) = (last(false), last(true));
    let escort_berth = session
        .npcs()
        .iter()
        .find(|npc| Some(npc.id) == escort)
        .map(|npc| npc.berth);
    // Tab from the last escort: the first ship above its berth.
    let after_escort = session
        .npcs()
        .iter()
        .filter(|npc| !session.is_escort(npc.id) && Some(npc.berth) > escort_berth)
        .min_by_key(|npc| npc.berth)
        .map(|npc| npc.id);
    assert!(after_escort.is_some(), "{:?}", session.npcs());
    let ships = session.npcs().len();
    assert!(ships >= 4, "two of each: {:?}", session.npcs());
    let target = |view: &View| view.session().expect("flying").target().map(|npc| npc.id);
    view.input(&key(Key::Shift, true));
    tap(&mut view, TARGET_KEY);
    assert_eq!(target(&view), ship);
    view.input(&key(Key::Alt, true));
    tap(&mut view, TARGET_KEY);
    assert_eq!(target(&view), escort);
    view.input(&key(Key::Shift, false));
    view.input(&key(Key::Alt, false));
    tap(&mut view, TARGET_KEY);
    assert_eq!(target(&view), after_escort, "Tab alone walks forward again");
}

#[test]
fn the_escorts_sheets_are_read_with_the_traffics() {
    let view = escorted();
    let asked = view.catalog().sheets_asked.borrow();
    assert!(asked.contains(&ShipId(130)), "{asked:?}");
}

#[test]
fn the_open_menu_is_drawn_over_the_flight_under_the_hud() {
    let mut view = escorted().with_overlays(TITLED);
    let mut shut = DrawList::new();
    view.draw(&mut shut);
    tap(&mut view, MENU_KEY);
    let mut open = DrawList::new();
    view.draw(&mut open);
    let commands: Vec<_> = open.iter().collect();
    let title = commands
        .iter()
        .position(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Escort Commands"))
        .expect("the menu");
    let frame = commands
        .iter()
        .position(|command| {
            matches!(command, DrawCommand::Line { color, .. } if *color == MENU_COLORS.border)
        })
        .expect("its border");
    let help = commands
        .iter()
        .position(|command| **command == overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM))
        .expect("the help line");
    let player = commands
        .iter()
        .position(|command| matches!(command, DrawCommand::Sprite { image, .. } if *image == ImageKey::sprite(2000, 0)))
        .expect("the player");
    assert!(
        player < frame && frame < title && title < help,
        "{player} {frame} {title} {help}"
    );
    assert_eq!(
        open.len() - shut.len(),
        1 + 4 + 1 + 5 + 1 + 1,
        "fill, border, title, rows, bar, order"
    );
    assert!(commands.iter().any(|command| matches!(
        command,
        DrawCommand::Text { text, color, .. } if text == "2) Fighters" && *color == ABSENT_ROW
    )));
    assert!(commands.iter().any(|command| matches!(
        command,
        DrawCommand::Line { from, .. } if from.x == MENU_AT.x && from.y == MENU_AT.y + MENU_HEIGHT / 2.0
    )), "filled across {MENU_WIDTH}");
}

#[test]
fn the_help_line_names_the_escort_keys() {
    assert!(
        HELP.contains("F/D/V/C: escorts   Alt-C: dock   E: escort menu"),
        "{HELP}"
    );
}

// Hiring escorts and paying them.

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
    let mut broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..hiring()
    });
    assert_eq!(broken.escorts_for_hire(), None);
    assert_eq!(broken.hire(ShipId(129)), Err(HireRefusal::NoBar));
}

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
    let mut broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..hiring()
    });
    assert!(!broken.open_bar());
    assert_eq!(broken.news(), None);
    assert_eq!(broken.news_picture(), None);
}
