use super::*;

// Flight.

#[test]
fn f_enters_flight_from_each_screen_and_escape_goes_back_to_it() {
    let mut screen = AppScreen::new(data());
    assert!(screen.flight_view().is_none(), "not until entered");
    fly(&mut screen);
    let session = flight(&screen).session().expect("flying");
    assert_eq!(session.system(), SystemId(128));
    assert_eq!(session.ship(), nova_sim::ShipId(128));
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser);

    screen.input(&key(Key::Tab, true));
    fly(&mut screen);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::GalaxyMap);

    click_system(&mut screen, 128);
    screen.input(&key(Key::Enter, true));
    fly(&mut screen);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::System);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
}

#[test]
fn an_f_repeat_or_release_does_not_enter_flight() {
    let mut screen = AppScreen::new(data());
    assert_eq!(screen.input(&held(Key::Char('f'))), ScreenAction::None);
    assert_eq!(
        screen.input(&key(Key::Char('f'), false)),
        ScreenAction::None
    );
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert!(screen.flight_view().is_none());
}

#[test]
fn escape_in_flight_never_quits_and_its_repeats_and_release_do_nothing() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    // Holding Escape after leaving does not quit either.
    assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser);
}

#[test]
fn tab_in_flight_targets_ships_not_stellars_and_stays_in_flight() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    let target = |screen: &AppScreen| flight(screen).session().expect("flying").nav_target();
    assert_eq!(target(&screen), None);
    for input in [key(Key::Tab, true), held(Key::Tab), key(Key::Tab, false)] {
        assert_eq!(screen.input(&input), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight);
    }
    assert_eq!(target(&screen), None, "Tab is the original's Target Select");
}

#[test]
fn the_flight_keys_fly_the_ship_and_flight_resumes_where_it_left_off() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 10);
    let flown = ship(&screen);
    assert!(flown.position.y < 0.0, "{flown:?}");
    assert_eq!(
        screen.ship_browser().selected(),
        Some(ShipId(128)),
        "the keys went to flight"
    );
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Char('f'), true));
    assert_eq!(ship(&screen), flown, "the same flight");
}

#[test]
fn only_flight_with_nothing_over_it_hides_the_system_pointer() {
    let mut screen = with_dialogs(data());
    assert!(!screen.hides_pointer(), "the ship browser");
    fly(&mut screen);
    assert!(screen.hides_pointer(), "flight");
    open_prefs(&mut screen);
    assert!(!screen.hides_pointer(), "the preferences over it");
    screen.input(&key(Key::Escape, true));
    assert!(screen.hides_pointer(), "flight again");
}

#[test]
fn entering_and_leaving_flight_let_go_of_the_keys_held() {
    let mut screen = AppScreen::new(data());
    enter(&mut screen, 128);
    screen.input(&key(Key::Right, true));
    fly(&mut screen);
    // Right's release goes to flight.
    screen.input(&key(Key::Right, false));
    screen.input(&key(Key::Escape, true));
    screen.tick(Duration::from_millis(250));
    assert_eq!(camera(&screen), Point::new(0.0, 0.0), "no drift");

    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 5);
    screen.input(&key(Key::Escape, true));
    // Up's release goes to the system view.
    screen.input(&key(Key::Up, false));
    fly(&mut screen);
    let coasting = ship(&screen);
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen).velocity, coasting.velocity, "no thrust");
}

#[test]
fn entering_flight_cancels_a_drag_on_the_map() {
    let mut screen = AppScreen::new(data());
    screen.input(&key(Key::Tab, true));
    let view = *screen.galaxy_map().view().expect("the viewer");
    let left = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: Point::new(100.0, 100.0),
    };
    screen.input(&left(true));
    fly(&mut screen);
    screen.input(&left(false));
    screen.input(&key(Key::Escape, true));
    screen.input(&Input::PointerMoved(Point::new(150.0, 100.0)));
    assert_eq!(
        *screen.galaxy_map().view().expect("the viewer"),
        view,
        "no drag"
    );
}

#[test]
fn only_flight_ticks_while_it_is_shown() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    // The first tick is held at rest by the new session's box of none.
    screen.tick(TICK * 2);
    assert_eq!(screen.ship_browser().frame(), Some(0), "paused");
    let moving = ship(&screen);
    assert_ne!(moving, nova_sim::ShipState::default());
    screen.input(&key(Key::Escape, true));
    screen.tick(TICK * 3);
    assert_eq!(ship(&screen), moving, "paused while hidden");
    assert_eq!(screen.ship_browser().frame(), Some(3));
}

/// Taps the stellar slot key 1, which flight acts on at its next frame.
fn tap_slot_key(screen: &mut AppScreen) {
    screen.input(&key(Key::Char('1'), true));
    screen.input(&key(Key::Char('1'), false));
}

#[test]
fn a_slot_key_is_queued_input_until_flight_ticks_and_never_under_a_dialog() {
    let mut screen = with_dialogs(data());
    assert!(!screen.input_queued(), "no flight");
    fly(&mut screen);
    assert!(!screen.input_queued(), "nothing pressed");
    tap_slot_key(&mut screen);
    assert!(screen.input_queued(), "pressed");
    screen.tick(TICK);
    screen.tick(TICK);
    assert!(!screen.input_queued(), "taken");
    tap_slot_key(&mut screen);
    open_prefs(&mut screen);
    assert!(
        !screen.input_queued(),
        "flight paused under the preferences"
    );
}

#[test]
fn a_slot_key_left_in_a_hidden_flight_is_not_queued_input() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    tap_slot_key(&mut screen);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert!(!screen.input_queued());
}

#[test]
fn releasing_the_keys_reaches_flight() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.release_keys();
    screen.tick(TICK * 3);
    assert_eq!(ship(&screen), nova_sim::ShipState::default());
}

/// Enters flight and opens its map with an M press.
fn open_flight_map(screen: &mut AppScreen) {
    fly(screen);
    assert_eq!(screen.input(&key(MAP, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::FlightMap);
}

#[test]
fn the_flight_maps_beeps_come_through_the_router_escape_closing_it_too() {
    let mut screen = AppScreen::new(data());
    open_flight_map(&mut screen);
    let ui = |sounds: Vec<Sound>| -> Vec<Sound> {
        sounds
            .into_iter()
            .filter(|sound| matches!(sound, Sound::Ui(_)))
            .collect()
    };
    assert_eq!(
        ui(screen.take_sounds()),
        [Sound::Ui(UiSound::MapOpened)],
        "opened"
    );
    screen.input(&key(MAP, false));
    screen.input(&key(Key::Escape, true));
    assert!(!flight(&screen).map_open());
    assert_eq!(
        ui(screen.take_sounds()),
        [Sound::Ui(UiSound::MapClosed)],
        "Escape closed it"
    );
}

#[test]
fn m_in_flight_opens_its_map_and_escape_closes_it_back_into_flight() {
    let mut screen = AppScreen::new(data());
    open_flight_map(&mut screen);
    assert!(flight(&screen).map_open());
    assert_eq!(screen.input(&key(MAP, false)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::FlightMap);
    assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
    assert_eq!(
        screen.showing(),
        Showing::FlightMap,
        "a repeat does nothing"
    );
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(!flight(&screen).map_open());
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::Flight, "still flying");
    // Escape in flight leaves it, as before.
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser);
}

#[test]
fn m_closes_the_flight_map_too() {
    let mut screen = AppScreen::new(data());
    open_flight_map(&mut screen);
    screen.input(&key(MAP, false));
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::Flight);
}

#[test]
fn tab_cycles_the_flight_maps_selection_and_f_and_i_do_nothing() {
    let mut screen = with_dialogs(data());
    open_flight_map(&mut screen);
    // Let go of M, whose hold would close the map on I (`_MapFilter`
    // @0x144db-0x14587).
    screen.input(&key(MAP, false));
    let nav = |screen: &AppScreen| {
        flight(screen)
            .session()
            .ok()
            .map(nova_sim::Session::nav_mode)
    };
    assert_eq!(nav(&screen), Some(nova_sim::NavMode::Off));
    for k in [Key::Tab, Key::Char('f'), Key::Char('i')] {
        assert_eq!(screen.input(&key(k, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::FlightMap, "{k:?}");
    }
    let cycled = nav(&screen).expect("flying");
    assert!(
        matches!(cycled, nova_sim::NavMode::Hyperspace(_)),
        "Tab cycled: {cycled:?}"
    );
    assert_eq!(
        flight(&screen).course_map().hyper_selection(),
        cycled.hyperspace(),
        "the map shows it"
    );
    assert!(screen.about().is_none());
}

#[test]
fn the_flight_map_is_drawn_without_the_hint() {
    let mut screen = AppScreen::new(data());
    open_flight_map(&mut screen);
    assert_eq!(drawn(&screen), drawn(flight(&screen)));
}

#[test]
fn flight_is_drawn_without_the_hint() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.tick(TICK);
    assert_eq!(drawn(&screen), drawn(flight(&screen)));
    screen.input(&key(Key::Escape, true));
    let list = drawn(&screen);
    assert_eq!(list.iter().last(), Some(&hint()));
}

// Sounds.

#[test]
fn the_router_says_what_it_shows() {
    let mut screen = with_dialogs(data());
    assert_eq!(screen.now_showing(), Some(Showing::ShipBrowser));
    land(&mut screen);
    assert_eq!(screen.now_showing(), Some(Showing::Spaceport));
}

#[test]
fn flights_sounds_come_through_the_router() {
    let mut screen = with_dialogs(data());
    assert_eq!(screen.take_sounds(), []);
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 2);
    assert_eq!(screen.take_sounds(), [Sound::Sim(SimSound::ThrustStarted)]);
    screen.input(&key(Key::Up, false));
    screen.tick(TICK * 2);
    assert_eq!(screen.take_sounds(), [Sound::Sim(SimSound::ThrustStopped)]);
    assert_eq!(screen.take_sounds(), [], "taken");
}

#[test]
fn leaving_the_spaceport_by_leave_sounds_only_the_take_off() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    assert_eq!(
        screen.take_sounds(),
        [Sound::Sim(SimSound::TrafficControl)],
        "the greeting; an inhabited stellar's touchdown is silent"
    );
    let leave = spaceport(&screen)
        .dialog()
        .expect("laid out")
        .item_bounds(LEAVE_ITEM)
        .expect("Leave")
        .center();
    assert_eq!(
        press_and_release(&mut screen, leave),
        [vec![], vec![halted(), Sound::Sim(SimSound::TookOff)]],
        "the ambient sound halts with the spaceport"
    );
    assert!(screen.spaceport_view().is_none(), "gone");
}

fn ambient() -> Sound {
    Sound::Ui(UiSound::PortAmbient {
        sound: nova_sim::SoundId(AMBIENT),
        until_stopped: false,
    })
}

fn beta_ambient() -> Sound {
    Sound::Ui(UiSound::PortAmbient {
        sound: nova_sim::SoundId(BETA_AMBIENT),
        until_stopped: false,
    })
}

/// No UI puts a landed pilot away: flight's Escape and a game over
/// both come from flight. So the put-away is driven through
/// [`AppScreen::end_flight`] itself.
#[test]
fn putting_a_landed_pilot_away_halts_the_ambient_sound() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    screen.take_sounds();
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    screen.end_flight(Side::MainMenu);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.spaceport_view().is_none(), "gone");
    let sounds = screen.take_sounds();
    assert!(sounds.contains(&halted()), "{sounds:?}");
}

#[test]
fn a_pilot_desk_move_to_another_stellar_halts_the_old_sound_before_the_new_one_starts() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    screen.take_sounds();
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    edit(
        &mut screen,
        PilotEdit::MoveTo {
            system: SystemId(129),
            stellar: nova_sim::StellarId(129),
        },
    );
    assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(129));
    // The new spaceport's offer is served after its first frame
    // ([`AppScreen::tick`]), which it covers, so its sound starts on
    // the second.
    let sounds = [
        screen.take_sounds(),
        frame_sounds(&mut screen, TICK),
        frame_sounds(&mut screen, TICK),
    ]
    .concat();
    let halt = sounds.iter().position(|&sound| sound == halted());
    let started = sounds.iter().position(|&sound| sound == beta_ambient());
    assert!(
        halt.is_some() && started.is_some() && halt < started,
        "Alpha Prime's halt, then Beta Prime's sound: {sounds:?}"
    );
    assert!(!sounds.contains(&ambient()), "{sounds:?}");
}

#[test]
fn a_pilot_desk_edit_that_stays_landed_keeps_the_ambient_sound_and_its_wait() {
    use nova_sim::mission::screens::mac_ticks;
    let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(FirstOutcome));
    let mut screen = with_dialogs(data()).with_chance(SharedChance::new(shared));
    land(&mut screen);
    screen.take_sounds();
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    assert_eq!(frame_sounds(&mut screen, mac_ticks(240)), []);
    edit(&mut screen, PilotEdit::Credits(5000));
    assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(128));
    let sounds = screen.take_sounds();
    assert!(!sounds.contains(&halted()), "{sounds:?}");
    assert_eq!(
        frame_sounds(&mut screen, mac_ticks(120)),
        [],
        "not started again"
    );
    assert_eq!(
        frame_sounds(&mut screen, mac_ticks(120)),
        [ambient()],
        "480 ticks after the first, the wait carried on"
    );
}

/// No UI drops a docked pilot's flight: Enter Ship, which drops the
/// developer's flight, is reached only from the menu, once the spaceport
/// is left. So the drop is driven through [`AppScreen::drop_flight`]
/// itself, and its sounds read from the router's queue, as the spaceport
/// side is left showing with no spaceport to show.
#[test]
fn dropping_a_docked_pilot_halts_the_ambient_sound() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.take_sounds();
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    screen.drop_flight();
    assert!(screen.spaceport_view().is_none(), "gone");
    let sounds = std::mem::take(&mut screen.sounds);
    assert!(sounds.contains(&halted()), "{sounds:?}");
}

/// One frame of `dt`'s sounds.
fn frame_sounds(screen: &mut AppScreen, dt: Duration) -> Vec<Sound> {
    screen.tick(dt);
    screen.take_sounds()
}

#[test]
fn the_stellars_ambient_sound_starts_on_the_spaceports_first_frame_not_at_touchdown() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    let landing = screen.take_sounds();
    assert!(!landing.contains(&ambient()), "{landing:?}");
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    assert_eq!(frame_sounds(&mut screen, TICK), []);
}

/// The landing's input settles its texts and the spaceport's offer
/// ([`AppScreen::settle_script`]) before the spaceport's first frame,
/// so nothing holds the sound back there.
#[test]
fn with_a_pilot_the_ambient_sound_starts_once_the_landings_offer_is_served() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, data());
    screen.take_sounds();
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
}

#[test]
fn the_ambient_sounds_waits_are_drawn_on_the_routers_chance() {
    let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(FirstOutcome));
    let mut screen = with_dialogs(data()).with_chance(SharedChance::new(shared));
    land(&mut screen);
    screen.take_sounds();
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    assert_eq!(
        frame_sounds(&mut screen, nova_sim::mission::screens::mac_ticks(480)),
        [ambient()],
        "480 ticks, as the first outcome draws"
    );
}

/// A source whose every draw is the first outcome.
struct FirstOutcome;

impl Chance for FirstOutcome {
    fn fires(&mut self, _percent: u8) -> bool {
        true
    }

    fn below(&mut self, _n: u32) -> u32 {
        0
    }
}

#[test]
fn the_preferences_closing_over_the_spaceport_rearms_the_ambient_sound() {
    use nova_sim::mission::screens::mac_ticks;
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.take_sounds();
    assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    assert_eq!(frame_sounds(&mut screen, mac_ticks(900)), []);
    open_prefs(&mut screen);
    assert_eq!(frame_sounds(&mut screen, mac_ticks(900)), [], "paused");
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen.take_sounds();
    assert_eq!(
        frame_sounds(&mut screen, mac_ticks(100)),
        [],
        "the wait began again"
    );
    assert_eq!(frame_sounds(&mut screen, mac_ticks(959)), [ambient()]);
}

#[test]
/// `_TrackTextButtons` @0x26bb0 plays nothing under the pointer.
fn closing_the_about_text_by_done_is_silent() {
    let mut screen = with_dialogs(data());
    open_about(&mut screen);
    let done = about(&screen)
        .dialog()
        .item_bounds(1)
        .expect("Done")
        .center();
    assert_eq!(press_and_release(&mut screen, done), [vec![], vec![]]);
    assert!(screen.about().is_none(), "closed");
}
