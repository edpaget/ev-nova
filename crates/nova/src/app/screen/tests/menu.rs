use super::*;

// The main menu and pilots.

/// The router with the main menu over `store`, and dialogs.
fn menu_with_dialogs(store: &MemoryPilots) -> AppScreen {
    with_dialogs(data())
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
}

#[test]
fn with_pilots_the_app_opens_on_the_main_menu() {
    let store = MemoryPilots::new();
    let screen = menu(&store);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(screen.now_showing(), Some(Showing::MainMenu));
    assert!(screen.flight_view().is_none());
    let mut list = DrawList::new();
    screen.main_menu().expect("a main menu").draw(&mut list);
    let mut all = drawn(&screen);
    assert_eq!(
        all.iter().take(list.len()).collect::<Vec<_>>(),
        list.iter().collect::<Vec<_>>()
    );
    assert!(matches!(
        all.iter().last(),
        Some(DrawCommand::Text { text, .. }) if text == MENU_HINT_WITHOUT_DIALOGS
    ));
    all = drawn(&menu_with_dialogs(&store));
    assert!(matches!(
        all.iter().last(),
        Some(DrawCommand::Text { text, .. }) if text == MENU_HINT
    ));
}

#[test]
fn quit_waits_out_the_quit_delay() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    ui_sounds(&mut screen);
    assert_eq!(choose(&mut screen, MenuChoice::Quit), ScreenAction::None);
    assert_eq!(
        ui_sounds(&mut screen),
        [UiSound::ButtonDown, UiSound::ButtonUp],
        "601 already played, on the release"
    );
    assert!(!screen.take_quit());
    screen.tick(QUIT_DELAY.saturating_sub(Duration::from_millis(1)));
    assert!(!screen.take_quit(), "still waiting");
    screen.tick(Duration::from_millis(1));
    assert!(screen.take_quit(), "the delay is over");
    assert!(!screen.take_quit(), "once");
}

#[test]
fn escape_on_the_main_menu_does_nothing() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    ui_sounds(&mut screen);
    for input in [
        key(Key::Escape, true),
        held(Key::Escape),
        key(Key::Escape, false),
    ] {
        assert_eq!(screen.input(&input), ScreenAction::None, "{input:?}");
    }
    assert_eq!(ui_sounds(&mut screen), [], "silent");
    assert!(!screen.take_quit());
    screen.tick(QUIT_DELAY * 2);
    assert!(!screen.take_quit(), "no quit is under way");
    // Nothing is held: a click on New Pilot opens its dialog at once.
    choose(&mut screen, MenuChoice::NewPilot);
    assert_eq!(screen.showing(), Showing::NewPilot);
}

#[test]
fn quits_delay_is_fifteen_ticks() {
    assert_eq!(QUIT_DELAY, nova_sim::mission::screens::mac_ticks(15));
}

#[test]
fn input_during_the_quit_delay_does_nothing() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    choose(&mut screen, MenuChoice::Quit);
    ui_sounds(&mut screen);
    let new_pilot = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::NewPilot);
    click_at(&mut screen, new_pilot);
    screen.input(&Input::Text('o'));
    screen.input(&key(Key::Char('p'), true));
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(ui_sounds(&mut screen), [], "nothing clicked");
    screen.tick(QUIT_DELAY);
    assert!(screen.take_quit());
}

#[test]
fn keyed_q_quits_only_after_its_click_and_the_quit_delay() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    ui_sounds(&mut screen);
    assert_eq!(screen.input(&Input::Text('q')), ScreenAction::None);
    assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed]);
    screen.tick(QUIT_DELAY * 2);
    assert!(!screen.take_quit(), "the click still plays");
    screen.keyed_click_ended();
    assert!(!screen.take_quit(), "601 has only started");
    screen.tick(QUIT_DELAY.saturating_sub(Duration::from_millis(1)));
    assert!(!screen.take_quit());
    screen.tick(Duration::from_millis(1));
    assert!(screen.take_quit());
    assert!(!screen.take_quit(), "once");
}

#[test]
fn a_command_q_typed_during_a_keyed_click_quits_once_it_is_replayed() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    // E is refused with no pilot: the replay reaches the menu.
    screen.input(&Input::Text('e'));
    assert_eq!(screen.input(&key(Key::Command, true)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Char('q'), true)), ScreenAction::None);
    assert!(!screen.take_quit(), "held for the click");
    screen.keyed_click_ended();
    assert!(screen.take_quit(), "the replayed Command-Q quit");
    assert!(!screen.take_quit(), "once");
}

#[test]
fn input_while_a_replayed_quit_is_untaken_does_nothing() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    // E is refused with no pilot: the replay reaches the menu.
    screen.input(&Input::Text('e'));
    screen.input(&key(Key::Command, true));
    screen.input(&key(Key::Char('q'), true));
    screen.keyed_click_ended();
    screen.input(&key(Key::Command, false));
    ui_sounds(&mut screen);
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.input(&Input::Text('q')), ScreenAction::None);
    assert_eq!(ui_sounds(&mut screen), [], "nothing clicked");
    assert!(screen.take_quit());
}

/// The interface sounds `screen` reports.
fn ui_sounds(screen: &mut AppScreen) -> Vec<UiSound> {
    screen
        .take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect()
}

#[test]
fn keyed_n_o_p_a_and_x_click_then_open_their_screen_once_the_click_ends() {
    let store = MemoryPilots::new();
    for (press, opens) in [
        (Input::Text('n'), Showing::NewPilot),
        (Input::Text('O'), Showing::OpenPilot),
        (key(Key::Char('p'), true), Showing::Preferences),
        (key(Key::Char('a'), true), Showing::About),
        (key(Key::Char('x'), true), Showing::Note),
    ] {
        let mut screen = menu_with_dialogs(&store);
        ui_sounds(&mut screen);
        assert_eq!(screen.input(&press), ScreenAction::None);
        assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed], "{press:?}");
        screen.tick(Duration::from_secs(1));
        assert_eq!(screen.showing(), Showing::MainMenu, "{press:?} waits");
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), opens, "{press:?}");
        screen.keyed_click_ended();
        assert_eq!(ui_sounds(&mut screen), [], "{press:?} once");
    }
}

#[test]
fn a_and_x_show_the_about_text_and_desc_32766_both_munged_before_any_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    screen.input(&key(Key::Char('a'), true));
    screen.keyed_click_ended();
    assert_eq!(about(&screen).lines()[..2], ["line 0", "line 1"]);
    assert_eq!(
        about(&screen).lines()[2],
        "line 2  [Error]",
        "no pilot's name, no mission, as _DoAboutBox munges with index -1"
    );
    screen.input(&key(Key::Char('a'), false));
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(screen.showing(), Showing::MainMenu);

    screen.input(&key(Key::Char('x'), true));
    screen.keyed_click_ended();
    assert_eq!(
        screen.showing(),
        Showing::Note,
        "a text over the menu, not the About box"
    );
    assert_eq!(
        about(&screen).lines(),
        ["x 0", "x 1 First [Error]"],
        "no pilot's name, ship 128's class, no mission"
    );
    screen.input(&key(Key::Char('x'), false));
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
}

#[test]
fn input_during_a_keyed_click_is_replayed_in_order_once_it_ends() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    screen.input(&Input::Text('n'));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&Input::Text('b'));
    screen.input(&key(Key::Char('b'), false));
    assert_eq!(screen.showing(), Showing::MainMenu);
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert_eq!(screen.new_pilot().expect("open").name(), "b");
}

#[test]
fn letting_go_during_a_keyed_click_happens_in_its_place_in_the_replay() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    ui_sounds(&mut screen);
    // E with the E key not held (another layout) clicks at once.
    screen.input(&Input::Text('e'));
    screen.input(&key(Key::Command, true));
    screen.release_keys();
    screen.input(&Input::Text('n'));
    assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed]);
    screen.keyed_click_ended();
    assert_eq!(
        ui_sounds(&mut screen),
        [UiSound::EnterRefused, UiSound::ButtonKeyed],
        "Command let go before the N, which holds again"
    );
    assert_eq!(screen.showing(), Showing::MainMenu, "N's click plays");
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::NewPilot);
}

#[test]
fn a_pointer_cancelled_during_a_keyed_click_is_cancelled_in_its_place() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    let at = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::NewPilot);
    let button = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };
    screen.input(&Input::Text('e'));
    screen.input(&button(true));
    screen.cancel_pointer();
    screen.input(&button(false));
    ui_sounds(&mut screen);
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::MainMenu, "cancelled, no click");
    assert_eq!(
        ui_sounds(&mut screen),
        [UiSound::EnterRefused, UiSound::ButtonDown]
    );
}

// E.

#[test]
fn e_held_waits_for_its_release_to_click_then_refuses_with_no_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    ui_sounds(&mut screen);
    screen.input(&key(Key::Char('e'), true));
    screen.input(&Input::Text('e'));
    assert_eq!(ui_sounds(&mut screen), [], "no click while E is held");
    screen.keyed_click_ended();
    assert_eq!(ui_sounds(&mut screen), [], "no click hold yet");
    screen.input(&key(Key::Char('e'), false));
    assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed], "on release");
    screen.keyed_click_ended();
    assert_eq!(ui_sounds(&mut screen), [UiSound::EnterRefused]);
    assert_eq!(screen.showing(), Showing::MainMenu);
}

#[test]
fn typed_e_without_the_e_key_held_clicks_at_once() {
    let store = MemoryPilots::new();
    for typed in ['e', 'E'] {
        let mut screen = menu(&store);
        ui_sounds(&mut screen);
        screen.input(&Input::Text(typed));
        assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed], "{typed}");
        screen.keyed_click_ended();
        assert_eq!(ui_sounds(&mut screen), [UiSound::EnterRefused], "{typed}");
    }
}

#[test]
fn while_e_is_held_its_repeats_are_dropped_and_the_rest_waits() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    screen.input(&key(Key::Char('e'), true));
    screen.input(&Input::Text('e'));
    for _ in 0..3 {
        screen.input(&held(Key::Char('e')));
        screen.input(&Input::Text('e'));
    }
    screen.input(&Input::Text('n'));
    ui_sounds(&mut screen);
    screen.release_keys();
    assert_eq!(
        ui_sounds(&mut screen),
        [UiSound::ButtonKeyed],
        "letting go of the keys releases E"
    );
    assert_eq!(screen.showing(), Showing::MainMenu, "N waits");
    screen.keyed_click_ended();
    assert_eq!(
        ui_sounds(&mut screen),
        [UiSound::EnterRefused, UiSound::ButtonKeyed],
        "one refusal, then N clicks"
    );
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::NewPilot);
}

#[test]
fn command_e_does_nothing() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    ui_sounds(&mut screen);
    screen.input(&key(Key::Command, true));
    screen.input(&key(Key::Char('e'), true));
    screen.input(&Input::Text('e'));
    screen.input(&key(Key::Char('e'), false));
    screen.keyed_click_ended();
    assert_eq!(ui_sounds(&mut screen), []);
}

// O's guard.

#[test]
fn o_does_nothing_while_s_or_l_is_held() {
    let store = MemoryPilots::new();
    for guard in [Key::Char('s'), Key::Char('l')] {
        let mut screen = menu_with_dialogs(&store);
        ui_sounds(&mut screen);
        screen.input(&key(guard, true));
        screen.input(&Input::Text('o'));
        screen.keyed_click_ended();
        assert_eq!(ui_sounds(&mut screen), [], "{guard:?}");
        assert_eq!(screen.showing(), Showing::MainMenu, "{guard:?}");
        screen.input(&key(guard, false));
        screen.input(&Input::Text('o'));
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::OpenPilot, "{guard:?} let go");

        let mut screen = menu_with_dialogs(&store);
        screen.input(&key(guard, true));
        screen.release_keys();
        screen.input(&Input::Text('O'));
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::OpenPilot, "{guard:?} released");
    }
}

#[test]
fn command_n_o_and_q_on_the_main_menu_neither_click_nor_choose() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    ui_sounds(&mut screen);
    screen.input(&key(Key::Command, true));
    for typed in ['n', 'o', 'q', 'N'] {
        assert_eq!(screen.input(&Input::Text(typed)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::MainMenu, "Command {typed}");
        assert_eq!(ui_sounds(&mut screen), [], "Command {typed}");
    }
    screen.input(&key(Key::Command, false));
    screen.input(&Input::Text('n'));
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::NewPilot, "without Command");
}

#[test]
fn the_main_menus_engine_keys_repeats_and_releases_are_silent() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    ui_sounds(&mut screen);
    for k in [Key::Char('p'), Key::Char('a'), Key::Char('x')] {
        screen.input(&held(k));
        screen.input(&key(k, false));
        assert_eq!(screen.showing(), Showing::MainMenu, "{k:?}");
        assert_eq!(ui_sounds(&mut screen), [], "{k:?}");
    }
    screen.input(&key(Key::Char('i'), true));
    assert_eq!(screen.showing(), Showing::About);
    assert_eq!(ui_sounds(&mut screen), [], "the engine's About key");
    screen.input(&key(Key::Enter, true));
    ui_sounds(&mut screen);

    // Command keys go to the original's menus, which do not click.
    screen.input(&key(Key::Command, true));
    screen.input(&key(Key::Char('p'), true));
    assert_eq!(screen.showing(), Showing::Preferences);
    assert_eq!(ui_sounds(&mut screen), [], "Command+P");
    screen.input(&key(Key::Escape, true));
    for k in [Key::Char('a'), Key::Char('x')] {
        screen.input(&key(k, true));
        assert_eq!(screen.showing(), Showing::MainMenu, "Command {k:?}");
    }
    screen.input(&key(Key::Command, false));
    assert_eq!(ui_sounds(&mut screen), []);

    // P away from the main menu is no main-screen key.
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Char('p'), true));
    assert_eq!(screen.showing(), Showing::Preferences);
    assert_eq!(ui_sounds(&mut screen), [], "the ship browser");
}

#[test]
fn without_dialogs_a_and_x_open_nothing_and_make_no_sound() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    ui_sounds(&mut screen);
    for k in [Key::Char('a'), Key::Char('x')] {
        screen.input(&key(k, true));
        assert_eq!(screen.showing(), Showing::MainMenu, "{k:?}");
    }
    assert_eq!(ui_sounds(&mut screen), []);
}

#[test]
fn tab_goes_from_the_menu_to_the_developer_sides_and_escape_comes_back() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    screen.input(&key(Key::Char('f'), true));
    assert_eq!(screen.showing(), Showing::MainMenu, "F is a developer key");
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::MainMenu);
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::MainMenu);
    screen.input(&key(Key::Tab, true));
    enter(&mut screen, 128);
    assert_eq!(screen.showing(), Showing::System);
    screen.input(&key(Key::Escape, true));
    assert_eq!(
        screen.showing(),
        Showing::GalaxyMap,
        "back to the map first"
    );
}

/// Slot history with berth 1 in the escorts' layer and holding
/// `shïp` 140's frames, as a flight that led a ship of that type there
/// leaves it.
fn led_berth_1() -> SlotSprites {
    let mut sprites = SlotSprites::default();
    sprites.layers.put(
        nova_sim::ship_layer::Slot::Berth(1),
        nova_sim::ship_layer::ShipLayer::Led,
    );
    sprites
        .frames
        .clone_frames(nova_sim::ship_layer::Slot::Berth(1), nova_sim::ShipId(140));
    sprites
}

/// Gives the flight shown `sprites` as its history, as its ticks would.
fn lay_out(screen: &mut AppScreen, sprites: SlotSprites) {
    screen.flight = screen
        .flight
        .take()
        .map(|flight| flight.with_slot_sprites(sprites));
}

#[test]
fn a_pilot_loaded_after_another_starts_from_the_layers_it_left() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    assert_eq!(
        AppScreen::new(data()).slot_sprites,
        SlotSprites::default(),
        "a run starts from the launch order"
    );
    create(&mut screen, "Ada");
    lay_out(&mut screen, led_berth_1());
    escape_to_menu(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.flight_view().is_none(), "put away");
    create(&mut screen, "Bea");
    assert_eq!(pilot(&screen).name(), "Bea");
    assert_eq!(flight(&screen).slot_sprites(), Some(&led_berth_1()));
}

#[test]
fn a_pilot_replaced_while_flying_hands_on_its_layers() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    lay_out(&mut screen, led_berth_1());
    let bea = nova_sim::Pilot::new(screen.data.as_ref(), "Bea").expect("a pilot");
    screen.start_flight(bea);
    assert_eq!(flight(&screen).slot_sprites(), Some(&led_berth_1()));
}

#[test]
fn the_developers_flight_after_a_pilot_starts_from_the_layers_it_left() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    lay_out(&mut screen, led_berth_1());
    escape_to_menu(&mut screen);
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Char('f'), true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(flight(&screen).slot_sprites(), Some(&led_berth_1()));
}

#[test]
fn new_pilot_asks_for_a_name_then_flies_and_saves_the_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).name(), "Ada");
    assert_eq!(store.keys(), ["Ada"], "saved at once");
    assert_eq!(saved(&store, "Ada"), *pilot(&screen));
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

/// One flight tick's time.
const FLIGHT_TICK: Duration = Duration::from_nanos(1_000_000_000 / 30);

#[test]
fn a_new_pilot_is_shown_the_land_hint() {
    use nova_view::flight::view::{TUTORIAL_DOCK, TUTORIAL_LAND};
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    assert_eq!(flight(&screen).message(), None, "before the first tick");
    screen.tick(FLIGHT_TICK);
    let shown = flight(&screen).message().expect("the land hint");
    assert!(
        shown.starts_with(TUTORIAL_LAND) || shown.starts_with(TUTORIAL_DOCK),
        "{shown}"
    );
    assert!(shown.contains("Hit 'L'"), "{shown}");
}

#[test]
fn an_opened_pilot_is_never_shown_the_tutorial() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::OpenPilot);
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    for _ in 0..120 {
        screen.tick(FLIGHT_TICK);
        assert_eq!(flight(&screen).message(), None);
    }
}

#[test]
fn a_new_pilot_runs_the_chärs_on_start_once_and_is_saved_with_what_it_did() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(starting_data(b"^b5 X129"))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    let bit_5 = nova_sim::Bit::new(5).expect("a bit");
    assert!(pilot(&screen).control_bit(bit_5), "toggled once");
    assert!(pilot(&screen).has_explored(nova_sim::SystemId(129)));
    assert_eq!(saved(&store, "Ada"), *pilot(&screen), "saved with them");
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
    // Opening the pilot runs no OnStart: a second toggle would clear it.
    assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
    let mut screen = AppScreen::new(starting_data(b"^b5 X129"))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    choose(&mut screen, MenuChoice::OpenPilot);
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(pilot(&screen).control_bit(bit_5));
}

#[test]
fn a_new_pilot_flies_and_is_saved_with_the_gender_chosen() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::NewPilot);
    let at = screen
        .new_pilot()
        .expect("open")
        .dialog()
        .item_bounds(nova_view::ui::new_pilot::GENDER_ITEM)
        .expect("the gender control")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    type_text(&mut screen, "Ada");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).gender(), nova_sim::Gender::Female);
    assert_eq!(saved(&store, "Ada").gender(), nova_sim::Gender::Female);
    let mut screen = menu(&store);
    create(&mut screen, "Bob");
    assert_eq!(pilot(&screen).gender(), nova_sim::Gender::Male);
}

#[test]
fn a_new_pilot_made_with_strict_play_ticked_plays_strict() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    create_strict(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(pilot(&screen).strict_play());
    assert!(saved(&store, "Ada").strict_play());
    let mut screen = menu_with_dialogs(&store);
    create(&mut screen, "Bob");
    assert!(!pilot(&screen).strict_play());
}

#[test]
fn the_router_passes_the_overlays_to_each_flight() {
    let on = FlightOverlays {
        title: true,
        help: true,
    };
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    assert_eq!(
        screen.flight_view().expect("flying").overlays(),
        FlightOverlays::default(),
        "off by default"
    );
    let mut screen = with_dialogs(data()).with_flight_overlays(on);
    fly(&mut screen);
    assert_eq!(screen.flight_view().expect("flying").overlays(), on);
    let store = MemoryPilots::new();
    let mut screen = menu(&store).with_flight_overlays(on);
    create(&mut screen, "Ada");
    assert_eq!(
        screen.flight_view().expect("flying").overlays(),
        on,
        "a new pilot's flight"
    );
}

fn flight_double_time(screen: &AppScreen) -> bool {
    screen.flight_view().expect("flying").double_time()
}

#[test]
fn caps_lock_presses_toggle_double_time_and_releases_and_repeats_do_not() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    assert!(!flight_double_time(&screen), "off to begin with");
    screen.input(&key(Key::CapsLock, true));
    assert!(flight_double_time(&screen), "on");
    screen.input(&held(Key::CapsLock));
    screen.input(&key(Key::CapsLock, false));
    assert!(flight_double_time(&screen), "a repeat and a release");
    screen.input(&key(Key::CapsLock, true));
    assert!(!flight_double_time(&screen), "off");
}

#[test]
fn a_toggle_under_an_overlay_reaches_the_flight_beneath() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    open_prefs(&mut screen);
    screen.input(&key(Key::CapsLock, true));
    assert!(flight_double_time(&screen));
}

#[test]
fn a_toggle_made_on_another_screen_reaches_a_flight_built_later() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    screen.input(&key(Key::CapsLock, true));
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(flight_double_time(&screen), "a new pilot's flight");
    let mut screen = with_dialogs(data());
    screen.input(&key(Key::CapsLock, true));
    fly(&mut screen);
    assert!(flight_double_time(&screen), "the developer's flight");
}

#[test]
fn a_new_pilot_flies_with_the_routers_hyperspace_effects() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store).with_prefs(Prefs {
        hyperspace_effects: false,
        ..Prefs::default()
    });
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(!screen.flight_view().expect("flying").hyperspace_effects());
}

#[test]
fn typing_a_name_never_opens_the_dialogs_under_it() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    choose(&mut screen, MenuChoice::NewPilot);
    for c in ['p', 'i', 'f'] {
        screen.input(&key(Key::Char(c), true));
        screen.input(&Input::Text(c));
    }
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert_eq!(screen.new_pilot().expect("open").name(), "pif");
    screen.input(&key(Key::Backspace, true));
    assert_eq!(screen.new_pilot().expect("open").name(), "pi");
}

#[test]
fn the_new_pilot_dialog_comes_from_the_interface_file_when_there_is_one() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    choose(&mut screen, MenuChoice::NewPilot);
    let field = screen.new_pilot().expect("open").field().rect();
    assert_eq!(field, new_pilot_template().items[7].bounds);
    let mut without = menu(&store);
    choose(&mut without, MenuChoice::NewPilot);
    assert_eq!(without.showing(), Showing::NewPilot, "the built-in one");
    let mut missing = AppScreen::new(data())
        .with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    choose(&mut missing, MenuChoice::NewPilot);
    assert_eq!(missing.showing(), Showing::NewPilot, "the built-in one");
    assert_eq!(
        missing.take_warnings(),
        ["nova: using the built-in New Pilot dialog: no DLOG 3102"]
    );
}

#[test]
fn cancelling_new_pilot_goes_back_to_the_menu() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::NewPilot);
    type_text(&mut screen, "Ada");
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.new_pilot().is_none());
    assert_eq!(store.writes(), 0);
}

#[test]
fn a_name_already_saved_is_refused_whatever_its_case() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    let writes = store.writes();
    create(&mut screen, "ada");
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert_eq!(
        screen.new_pilot().expect("open").refusal(),
        Some(NAME_TAKEN)
    );
    assert_eq!(store.writes(), writes);
}

#[test]
fn a_name_that_cannot_be_a_file_is_refused() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "..");
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert_eq!(
        screen.new_pilot().expect("open").refusal(),
        Some(UNUSABLE_NAME)
    );
    assert_eq!(store.writes(), 0);
}

#[test]
fn open_pilot_lists_the_saved_pilots_and_resumes_the_chosen_one_where_it_landed() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    let landed = pilot(&screen).clone();
    let docked_at = ship(&screen).position;
    assert_eq!(landed.stellar(), Some(nova_sim::StellarId(128)));
    assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
    let mut screen = menu(&store);
    assert_eq!(
        choose(&mut screen, MenuChoice::OpenPilot),
        ScreenAction::None
    );
    assert_eq!(screen.showing(), Showing::OpenPilot);
    assert_eq!(screen.pilot_list().expect("open").keys(), ["Ada"]);
    screen.take_sounds();
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight, "in space, no spaceport");
    assert!(screen.spaceport.is_none());
    assert!(undocked(&screen));
    assert_eq!(ship(&screen).position, docked_at, "over its stellar");
    assert_eq!(*pilot(&screen), landed);
    assert!(screen.pilot_list().is_none());
    assert_eq!(screen.take_sounds(), [], "no landing sound");
}

// The note on a load that fell back.

/// Rewrites `key`'s save in `store` with `change` over its JSON.
fn rewrite_save(
    store: &MemoryPilots,
    key: &str,
    change: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) {
    let mut save: serde_json::Value =
        serde_json::from_str(&store.text(key).expect("saved")).expect("JSON");
    let fields = save.as_object_mut().expect("an object");
    assert!(fields.contains_key("stellar") && fields.contains_key("system"));
    change(fields);
    store.put(key, &save.to_string());
}

/// Ada, created over `store` and landed on stellar 128 in system 128,
/// saved as if last landed on stellar 999 in system 129: a stellar no
/// system lists.
fn save_unlisted_ada(store: &MemoryPilots) {
    let mut screen = menu_with_dialogs(store);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(pilot(&screen).stellar(), Some(nova_sim::StellarId(128)));
    assert_eq!(screen.quit_and_reopen(store), Showing::MainMenu);
    rewrite_save(store, "Ada", |fields| {
        fields.insert("stellar".into(), 999.into());
        fields.insert("system".into(), 129.into());
    });
}

/// Opens the first saved pilot from the main menu's list.
fn open_first(screen: &mut AppScreen) {
    choose(screen, MenuChoice::OpenPilot);
    assert_eq!(screen.showing(), Showing::OpenPilot);
    screen.input(&key(Key::Enter, true));
}

fn flown_system(screen: &AppScreen) -> nova_sim::SystemId {
    flight(screen).session().expect("flying").system()
}

fn undocked(screen: &AppScreen) -> bool {
    flight(screen).session().expect("flying").landed().is_none()
}

#[test]
fn an_opened_pilot_whose_stellar_no_system_lists_shows_the_note_over_the_menu_then_flies() {
    let store = MemoryPilots::new();
    save_unlisted_ada(&store);
    let mut screen = menu_with_dialogs(&store);
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Note);
    assert_eq!(screen.now_showing(), Some(Showing::Note));
    assert!(screen.pilot_list().is_none());
    assert_eq!(
        screen.load_note().map(NoteDialog::text),
        Some(LOAD_FALLBACK_NOTE)
    );
    assert!(screen.flight_view().is_some(), "built behind the note");
    assert_eq!(flown_system(&screen), nova_sim::SystemId(129));
    assert!(undocked(&screen));
    let mut note = DrawList::new();
    screen.load_note().expect("open").draw(&mut note);
    let all = drawn(&screen);
    assert_eq!(
        all.iter().skip(all.len() - note.len()).collect::<Vec<_>>(),
        note.iter().collect::<Vec<_>>(),
        "the note is drawn last, over the menu"
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.load_note().is_none());
    assert_eq!(flown_system(&screen), nova_sim::SystemId(129));
    // Back on the menu and open again: the repaired save loads plainly.
    escape_to_menu(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight, "the note shows once");
}

#[test]
fn under_the_engines_reading_the_noted_pilot_flies_in_the_lowest_system() {
    let store = MemoryPilots::new();
    save_unlisted_ada(&store);
    let mut screen = menu_with_dialogs(&store).with_rules(
        Rulebook::default().with_override(RuleKey::UnlistedStellar, RuleSource::Engine),
    );
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Note);
    assert_eq!(flown_system(&screen), nova_sim::SystemId(128));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(flown_system(&screen), nova_sim::SystemId(128));
    assert!(undocked(&screen));
}

#[test]
fn a_normal_open_never_shows_the_note() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
    let mut screen = menu_with_dialogs(&store);
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(undocked(&screen));
    assert!(screen.load_note().is_none());
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    create(&mut screen, "Bob");
    assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
    let mut screen = menu_with_dialogs(&store);
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.load_note().is_none());
}

#[test]
fn a_pilot_whose_saved_system_is_gone_notes_and_flies_in_the_lowest_that_exists() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    create(&mut screen, "Ada");
    assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
    rewrite_save(&store, "Ada", |fields| {
        fields.insert("system".into(), 300.into());
        fields.insert("stellar".into(), serde_json::Value::Null);
    });
    let mut screen = menu_with_dialogs(&store);
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Note);
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(flown_system(&screen), nova_sim::SystemId(128));
    assert!(undocked(&screen));
}

#[test]
fn without_dialogs_a_repaired_open_warns_and_flies() {
    let store = MemoryPilots::new();
    save_unlisted_ada(&store);
    let mut screen = menu(&store);
    screen.take_warnings();
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.load_note().is_none());
    assert_eq!(screen.take_warnings(), [LOAD_FALLBACK_NOTE]);
}

#[test]
fn the_note_takes_every_event() {
    let store = MemoryPilots::new();
    save_unlisted_ada(&store);
    let mut screen = menu_with_dialogs(&store);
    open_first(&mut screen);
    for pressed in [true, false] {
        for typed in [Key::Char('f'), Key::Tab, Key::Char('i'), Key::Char('l')] {
            assert_eq!(screen.input(&key(typed, pressed)), ScreenAction::None);
        }
    }
    assert_eq!(screen.showing(), Showing::Note);
    screen.tick(TICK);
    screen.release_keys();
    screen.cancel_pointer();
    assert_eq!(screen.showing(), Showing::Note);
    let ok = screen
        .load_note()
        .expect("open")
        .dialog()
        .item_bounds(nova_view::ui::note::OK_ITEM)
        .expect("OK")
        .center();
    screen.take_sounds();
    let click = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: ok,
    };
    screen.input(&click(true));
    assert_eq!(screen.take_sounds(), []);
    screen.input(&click(false));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(screen.take_sounds(), [], "OK is silent as it closes");
}

/// Never fires, and draws the last outcome: a heading of 359.
struct LastOutcome;

impl Chance for LastOutcome {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }
}

fn last_outcome() -> SharedChance {
    let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(LastOutcome));
    SharedChance::new(shared)
}

#[test]
#[allow(clippy::float_cmp)] // A whole degree, exactly as drawn.
fn open_pilot_faces_the_pilot_on_a_drawn_heading() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
    let mut screen = menu(&store).with_chance(last_outcome());
    choose(&mut screen, MenuChoice::OpenPilot);
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(ship(&screen).heading, 359.0);
}

#[test]
#[allow(clippy::float_cmp)] // Facing up, exactly.
fn a_new_pilot_starts_facing_up_whatever_the_chance() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store).with_chance(last_outcome());
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(ship(&screen).heading, 0.0);
}

#[test]
fn cancelling_open_pilot_goes_back_to_the_menu() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::OpenPilot);
    assert_eq!(
        screen.pilot_list().expect("open").keys(),
        Vec::<String>::new()
    );
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
}

#[test]
fn a_pilot_that_cannot_be_opened_says_why_in_the_list() {
    let store = MemoryPilots::new();
    store.put("Broken", "not a save");
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::OpenPilot);
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::OpenPilot);
    let error = screen
        .pilot_list()
        .expect("open")
        .error()
        .expect("an error")
        .to_owned();
    assert!(
        error.starts_with("nova: the pilot Broken in memory: "),
        "{error}"
    );
    assert_eq!(screen.take_warnings(), [error]);
}

#[test]
fn pilots_that_cannot_be_listed_say_why() {
    let store = MemoryPilots::new();
    store.fail_reads(true);
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::OpenPilot);
    let message = "nova: cannot list the pilots in memory (the disk is unreadable)";
    assert_eq!(screen.pilot_list().expect("open").error(), Some(message));
    assert_eq!(screen.take_warnings(), [message]);
}

#[test]
fn without_a_store_new_pilot_flies_unsaved_and_open_pilot_lists_nothing() {
    let mut screen = AppScreen::new(data())
        .with_pilots(None, Rc::new(MonoMetrics))
        .appeared();
    assert_eq!(screen.showing(), Showing::MainMenu);
    choose(&mut screen, MenuChoice::OpenPilot);
    assert_eq!(
        screen.pilot_list().expect("open").keys(),
        Vec::<String>::new()
    );
    screen.input(&key(Key::Escape, true));
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    land_twice(&mut screen);
    screen.quit();
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn landing_taking_off_and_a_transaction_each_save_the_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    assert_eq!(store.writes(), 1, "created");
    land_twice(&mut screen);
    assert_eq!(store.writes(), 2, "landed");
    assert_eq!(
        saved(&store, "Ada").stellar(),
        Some(nova_sim::StellarId(128))
    );
    assert!(screen.transact(|pilot| pilot.set_cash(4_321)));
    screen.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
    assert_eq!(store.writes(), 3, "a spaceport transaction");
    assert_eq!(saved(&store, "Ada").cash(), 4_321);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(store.writes(), 4, "took off");
    screen.tick(TICK);
    screen.input(&key(Key::Up, true));
    assert_eq!(store.writes(), 4, "flying saves nothing");
}

#[test]
fn escape_to_the_menu_and_quitting_save_the_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 10);
    let writes = store.writes();
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(store.writes(), writes + 1);
    assert!(screen.flight_view().is_none(), "the pilot is put away");
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::OpenPilot);
    screen.input(&key(Key::Enter, true));
    land_twice(&mut screen);
    let writes = store.writes();
    screen.quit();
    assert_eq!(store.writes(), writes + 1, "quitting saves");
}

#[test]
fn a_failed_save_is_a_warning() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    store.fail_writes(true);
    land_twice(&mut screen);
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot save the pilot Ada in memory (the disk is full)"]
    );
    assert_eq!(screen.take_warnings(), Vec::<String>::new(), "taken");
}

#[test]
fn the_developer_flight_is_a_fresh_unnamed_pilot_never_saved() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    let ada = store.text("Ada");
    let writes = store.writes();
    screen.input(&key(Key::Tab, true));
    fly(&mut screen);
    assert_eq!(pilot(&screen).name(), "");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    screen.quit();
    assert_eq!(store.writes(), writes, "nothing saved");
    assert_eq!(store.text("Ada"), ada);
    assert_eq!(store.keys(), ["Ada"]);
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn i_and_p_work_on_the_main_menu() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    open_about(&mut screen);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    open_prefs(&mut screen);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
}

#[test]
fn the_menus_sounds_come_through_the_router() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    let at = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::OpenPilot);
    assert_eq!(press_and_release(&mut screen, at), [vec![DOWN], vec![UP]]);
    let cancel = screen
        .pilot_list()
        .expect("open")
        .cancel_button()
        .rect
        .center();
    assert_eq!(
        press_and_release(&mut screen, cancel),
        [vec![], vec![]],
        "the pilot list's Cancel is silent"
    );
    assert!(screen.pilot_list().is_none(), "closed");
    let at = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::NewPilot);
    assert_eq!(press_and_release(&mut screen, at), [vec![DOWN], vec![UP]]);
    let ok = screen
        .new_pilot()
        .expect("open")
        .dialog()
        .item_bounds(2)
        .expect("Cancel")
        .center();
    assert_eq!(
        press_and_release(&mut screen, ok),
        [vec![], vec![]],
        "the new pilot dialog's Cancel is silent"
    );
    assert!(screen.new_pilot().is_none(), "cancelled");
}

#[test]
fn the_overlays_take_pointer_cancels_and_key_releases() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    choose(&mut screen, MenuChoice::OpenPilot);
    let cancel = screen
        .pilot_list()
        .expect("open")
        .cancel_button()
        .rect
        .center();
    let button = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: cancel,
    };
    screen.input(&button(true));
    screen.cancel_pointer();
    screen.release_keys();
    screen.input(&button(false));
    assert_eq!(
        screen.showing(),
        Showing::OpenPilot,
        "the click was abandoned"
    );
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::OpenPilot);
}

#[test]
fn a_tab_repeat_or_release_on_the_menu_does_nothing() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    for input in [held(Key::Tab), key(Key::Tab, false)] {
        assert_eq!(screen.input(&input), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::MainMenu, "{input:?}");
    }
}

#[test]
fn debug_shows_the_menu_and_where_pilots_are_kept() {
    let store = MemoryPilots::new();
    let debug = format!("{:?}", menu(&store));
    assert!(
        debug.contains("menu: Some(Menu { screen: MainMenu {")
            && debug.contains(r#"pilots: Some("memory"), .. })"#),
        "{debug}"
    );
}

impl AppScreen {
    /// Quits as the app would, and says what a new router over `store`
    /// shows.
    fn quit_and_reopen(&mut self, store: &MemoryPilots) -> Showing {
        self.quit();
        menu(store).showing()
    }
}

// Pilot edits.

#[test]
fn the_pilot_desk_is_there_only_in_flight() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    assert!(screen.pilot_desk().is_none());
    create(&mut screen, "Ada");
    let desk = screen.pilot_desk().expect("flying");
    assert_eq!(desk.sheet().expect("a pilot").name, "Ada");
}

#[test]
fn a_credits_edit_is_saved_and_the_open_exchange_shows_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    let writes = store.writes();
    edit(&mut screen, PilotEdit::Credits(5000));
    assert_eq!(store.writes(), writes + 1);
    assert_eq!(saved(&store, "Ada").cash(), 5000);
    let open = spaceport(&screen).open_trade().expect("still trading");
    assert_eq!(open.market().cash, 5000);
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn an_edit_in_flight_is_saved_too() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    edit(&mut screen, PilotEdit::Credits(42));
    assert_eq!(saved(&store, "Ada").cash(), 42);
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn a_move_rebuilds_the_spaceport_for_the_new_stellar() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    edit(
        &mut screen,
        PilotEdit::MoveTo {
            system: SystemId(129),
            stellar: nova_sim::StellarId(129),
        },
    );
    assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(129));
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(
        flight(&screen)
            .scene()
            .map(nova_view::system::scene::SystemScene::id),
        Some(SystemId(129))
    );
    let saved = saved(&store, "Ada");
    assert_eq!(saved.system(), SystemId(129));
    assert_eq!(saved.stellar(), Some(nova_sim::StellarId(129)));
    assert!(
        !spaceport(&screen)
            .offered()
            .contains(&nova_sim::Service::TradeCenter),
        "Beta Prime has no exchange"
    );
}

#[test]
fn a_move_to_the_stellar_landed_on_keeps_the_spaceport_open() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    edit(
        &mut screen,
        PilotEdit::MoveTo {
            system: SystemId(128),
            stellar: nova_sim::StellarId(128),
        },
    );
    assert!(spaceport(&screen).open_trade().is_some(), "not rebuilt");
}

#[test]
fn an_unnamed_pilots_edit_is_not_saved() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    screen.input(&key(Key::Tab, true));
    fly(&mut screen);
    edit(&mut screen, PilotEdit::Credits(42));
    assert_eq!(pilot(&screen).cash(), 42);
    assert_eq!(store.writes(), 0);
    assert_eq!(store.keys(), Vec::<String>::new());
}

/// Opens the flight's course map, zooms it out a step with `-` and
/// closes it: the zoom it is left at.
fn zoom_the_course_map_out(screen: &mut AppScreen) -> f64 {
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert!(flight(screen).map_open());
    screen.input(&key(Key::Char('-'), true));
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert!(!flight(screen).map_open());
    flight(screen).course_map_zoom().expect("an in-flight map")
}

#[test]
#[allow(clippy::float_cmp)] // Each zoom is an exact product.
fn the_course_maps_zoom_carries_into_the_next_pilot_new_or_opened() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    assert_eq!(
        flight(&screen).course_map_zoom(),
        Some(0.5625),
        "a run's first"
    );
    let zoomed = zoom_the_course_map_out(&mut screen);
    assert_eq!(zoomed, 0.5625 * 1.333_333);
    escape_to_menu(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    create(&mut screen, "Bea");
    assert_eq!(flight(&screen).course_map_zoom(), Some(zoomed), "New Pilot");
    let again = zoom_the_course_map_out(&mut screen);
    escape_to_menu(&mut screen);
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(flight(&screen).course_map_zoom(), Some(again), "Open Pilot");
}

/// Opens the flight's course map, flips its borders with B and closes it.
fn flip_the_borders(screen: &mut AppScreen) {
    for k in [MAP, Key::Char('b'), MAP] {
        screen.input(&key(k, true));
        screen.input(&key(k, false));
    }
    assert!(!flight(screen).map_open());
}

#[test]
fn the_borders_setting_reaches_each_flight_and_a_change_carries_into_the_next_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store).with_map_borders(false);
    create(&mut screen, "Ada");
    assert!(!flight(&screen).map_borders_shown(), "the setting given");
    flip_the_borders(&mut screen);
    assert_eq!(
        screen.take_map_borders_save(),
        Some(true),
        "the flight's, forwarded"
    );
    assert_eq!(screen.take_map_borders_save(), None, "taken");
    escape_to_menu(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    create(&mut screen, "Bea");
    assert!(flight(&screen).map_borders_shown(), "New Pilot");
    flip_the_borders(&mut screen);
    escape_to_menu(&mut screen);
    open_first(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(!flight(&screen).map_borders_shown(), "Open Pilot");
}

#[test]
fn the_borders_are_on_for_a_router_given_no_setting() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    assert!(flight(&screen).map_borders_shown());
    assert_eq!(screen.take_map_borders_save(), None);
}

// The pointer's Enter Ship, Preferences and About.

/// Game data with no main-screen art, only the About text: the menu
/// draws the stand-in.
fn bare_data() -> Rc<GameData> {
    loaded(&ForkBuilder::new().resource(Desc::TYPE, ABOUT_TEXT, None, &about_text()))
}

/// The router with the main menu over `data`, with dialogs or not, the
/// stand-in's warning taken.
fn menu_over(data: Rc<GameData>, dialogs: bool) -> AppScreen {
    let store = MemoryPilots::new();
    let screen = if dialogs {
        with_dialogs(data)
    } else {
        AppScreen::new(data)
    };
    let mut screen = screen
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    screen.take_warnings();
    ui_sounds(&mut screen);
    screen
}

/// The About text's lines, if it is open.
fn about_lines(screen: &AppScreen) -> Option<Vec<String>> {
    screen.about().map(|about| {
        about
            .dialog()
            .scroll_text()
            .expect("the text")
            .lines()
            .to_vec()
    })
}

#[test]
fn the_pointers_enter_ship_preferences_and_about_reach_what_e_p_and_a_reach() {
    for data in [data(), bare_data()] {
        let keyed = |press: Input| {
            let mut screen = menu_over(Rc::clone(&data), true);
            screen.input(&press);
            screen.keyed_click_ended();
            let mut sounds = ui_sounds(&mut screen);
            sounds.retain(|&sound| sound != UiSound::ButtonKeyed);
            (screen.showing(), sounds, about_lines(&screen))
        };
        let clicked = |choice: MenuChoice| {
            let mut screen = menu_over(Rc::clone(&data), true);
            assert_eq!(choose(&mut screen, choice), ScreenAction::None);
            let mut sounds = ui_sounds(&mut screen);
            assert_eq!(
                sounds.drain(..2).collect::<Vec<_>>(),
                [UiSound::ButtonDown, UiSound::ButtonUp],
                "{choice:?}'s click"
            );
            (screen.showing(), sounds, about_lines(&screen))
        };
        let enter = clicked(MenuChoice::EnterShip);
        assert_eq!(
            enter,
            (Showing::MainMenu, vec![UiSound::EnterRefused], None),
            "refused with no pilot"
        );
        assert_eq!(enter, keyed(Input::Text('e')));
        let preferences = clicked(MenuChoice::Preferences);
        assert_eq!(preferences.0, Showing::Preferences);
        assert_eq!(preferences, keyed(key(Key::Char('p'), true)));
        let about = clicked(MenuChoice::About);
        assert_eq!(about.0, Showing::About);
        assert_eq!(
            about.2.as_ref().map(|lines| lines[2].clone()),
            Some("line 2  [Error]".to_owned()),
            "munged as before any pilot"
        );
        assert_eq!(about, keyed(key(Key::Char('a'), true)));
    }
}

#[test]
fn without_dialogs_the_pointers_preferences_and_about_open_nothing() {
    for data in [data(), bare_data()] {
        for choice in [MenuChoice::Preferences, MenuChoice::About] {
            let mut screen = menu_over(Rc::clone(&data), false);
            choose(&mut screen, choice);
            assert_eq!(screen.showing(), Showing::MainMenu, "{choice:?}");
            assert_eq!(
                ui_sounds(&mut screen),
                [UiSound::ButtonDown, UiSound::ButtonUp],
                "{choice:?}"
            );
        }
    }
}

#[test]
fn the_menu_draws_the_art_and_without_it_the_stand_in_with_a_warning() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    assert!(screen.main_menu().expect("a main menu").draws_art());
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
    let mut screen = AppScreen::new(bare_data())
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    assert!(!screen.main_menu().expect("a main menu").draws_art());
    assert_eq!(
        screen.take_warnings(),
        ["nova: drawing the stand-in main menu: no cölr 128"]
    );
}

/// A source whose rolls are always the last outcome.
struct LastRoll;

impl nova_sim::Chance for LastRoll {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }

    fn roll(&mut self, sides: u16) -> u16 {
        sides - 1
    }
}

fn last_roll() -> SharedChance {
    SharedChance::new(Rc::new(std::cell::RefCell::new(LastRoll)))
}

#[test]
fn the_logo_picks_on_the_effects_chance_given_before_or_after_the_menu() {
    let store = MemoryPilots::new();
    let after = AppScreen::new(data())
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared()
        .with_effects_chance(last_roll());
    let before = AppScreen::new(data())
        .with_effects_chance(last_roll())
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    for mut screen in [Box::new(after), Box::new(before)] {
        screen.tick(nova_sim::mission::screens::mac_ticks(5));
        assert_eq!(
            screen.main_menu().and_then(MainMenu::logo_frame),
            Some(6),
            "the last of the six others"
        );
    }
}

#[test]
fn the_main_screen_stands_still_behind_a_dialog_opened_over_it() {
    for open in [MenuChoice::Preferences, MenuChoice::About] {
        let mut screen = menu_over(data(), true);
        let quit = screen
            .main_menu()
            .expect("a main menu")
            .press_point(MenuChoice::Quit);
        screen.input(&Input::PointerMoved(quit));
        screen.tick(nova_sim::mission::screens::mac_ticks(4));
        let menu = screen.main_menu().expect("a main menu");
        let (logo, rollover) = (menu.logo_frame(), menu.rollover());
        assert_eq!(rollover, Some((2, 16)), "fading in over Quit");
        let mut before = DrawList::new();
        menu.draw(&mut before);
        // Opened from the keyboard, the pointer still over Quit.
        let press = if open == MenuChoice::About {
            key(Key::Char('a'), true)
        } else {
            key(Key::Char('p'), true)
        };
        screen.input(&press);
        screen.keyed_click_ended();
        assert_ne!(screen.showing(), Showing::MainMenu, "{open:?} is open");
        for _ in 0..10 {
            screen.tick(nova_sim::mission::screens::mac_ticks(1));
        }
        let menu = screen.main_menu().expect("a main menu");
        assert_eq!(
            (menu.logo_frame(), menu.rollover()),
            (logo, rollover),
            "{open:?}: frozen, as no dialog loop idles the main screen"
        );
        let all = drawn(&screen);
        assert_eq!(
            all.iter().take(before.len()).collect::<Vec<_>>(),
            before.iter().collect::<Vec<_>>(),
            "{open:?}: drawn as it stood"
        );
    }
}

/// Ticks `screen` one Mac tick at a time, `ticks` times.
fn tick_by_ticks(screen: &mut AppScreen, ticks: usize) {
    for _ in 0..ticks {
        screen.tick(nova_sim::mission::screens::mac_ticks(1));
    }
}

/// Whether `at` is clear of every main-menu button's rectangle.
fn clear_of_the_buttons(screen: &AppScreen, at: Point) -> bool {
    let menu = screen.main_menu().expect("a main menu");
    MenuChoice::ALL
        .iter()
        .all(|&choice| !menu.button_bounds(choice).contains(at))
}

#[test]
fn the_rollover_aims_at_the_live_pointer_once_a_dialog_over_the_menu_closes() {
    let mut screen = menu_over(data(), true);
    let over_about = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::About);
    screen.input(&Input::PointerMoved(over_about));
    tick_by_ticks(&mut screen, 8);
    assert_eq!(
        screen.main_menu().and_then(MainMenu::rollover),
        Some((5, 32)),
        "over About"
    );
    click_at(&mut screen, over_about);
    assert_eq!(screen.showing(), Showing::About);
    let done = about(&screen)
        .dialog()
        .item_bounds(nova_view::ui::desc::DONE_ITEM)
        .expect("Done")
        .center();
    assert!(
        clear_of_the_buttons(&screen, done),
        "Done is off the buttons"
    );
    click_at(&mut screen, done);
    assert_eq!(screen.showing(), Showing::MainMenu);
    tick_by_ticks(&mut screen, 20);
    assert_eq!(
        screen.main_menu().and_then(MainMenu::rollover).map(|r| r.0),
        Some(6),
        "the none frame, where the pointer is now"
    );
}

#[test]
fn the_rollover_aims_at_the_live_pointer_back_from_a_widened_flight() {
    let store = MemoryPilots::new();
    let mut screen = menu_with_dialogs(&store);
    let widened = (1366.0, 768.0);
    screen.set_widened_size(widened);
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    // Over Quit as the classic frame sees it, moved on into the widened
    // view.
    let quit = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::Quit);
    let offset = nova_view::view_shape::classic_offset(widened.0);
    screen.input(&Input::PointerMoved(Point::new(quit.x + offset, quit.y)));
    screen.input(&key(Key::Escape, true));
    screen.tick(nova_view::fade::DEATH_FADE_FOR);
    assert_eq!(screen.showing(), Showing::MainMenu);
    tick_by_ticks(&mut screen, 20);
    assert_eq!(
        screen.main_menu().and_then(MainMenu::rollover),
        Some((2, 32)),
        "over Quit"
    );
}

#[test]
fn the_rollover_aims_at_the_live_pointer_back_from_the_ship_browser() {
    let mut screen = menu_over(data(), true);
    let quit = screen
        .main_menu()
        .expect("a main menu")
        .press_point(MenuChoice::Quit);
    screen.input(&Input::PointerMoved(quit));
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    screen.input(&Input::PointerLeft);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    tick_by_ticks(&mut screen, 20);
    assert_eq!(
        screen.main_menu().and_then(MainMenu::rollover).map(|r| r.0),
        Some(6),
        "the pointer left the window on the ship browser"
    );
}
