use super::*;
use nova_view::ui::mission_info::MISSION_INFO_KEY;
use nova_view::ui::player_info::PLAYER_INFO_KEY;

// The keys typed while a map over a screen waits for M's release, replayed
// to the screen under it at the close: the wiring of the flight's queue
// (`nova_view`'s `map_wait` module) and the spaceport's layers through the
// real router.

/// Presses and lets go of `k`.
fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// Opens the map from what is shown and lets M go, then starts its wait:
/// M down again on the open map.
fn start_the_wait(screen: &mut AppScreen) {
    tap(screen, MAP);
    assert!(flight(screen).map_open(), "opened");
    screen.input(&key(MAP, true));
    assert!(flight(screen).map_waiting(), "waiting");
}

/// Lets M go, ending the wait.
fn end_the_wait(screen: &mut AppScreen) {
    screen.input(&key(MAP, false));
    assert!(!flight(screen).map_open(), "closed");
}

/// A person's offer of mission 130.
fn job() -> nova_sim::OfferPrompt {
    nova_sim::OfferPrompt {
        mission: MissionId(130),
        text: "Job.".to_owned(),
        labels: nova_sim::mission::offer_labels("", "", true),
        graphic: None,
        movie: None,
    }
}

/// Flight with a person's offer open over it, laid out.
fn offered_over_flight() -> AppScreen {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(game_data_fork_loaded())
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    screen.open_offer(job());
    assert_eq!(screen.showing(), Showing::Offer);
    screen
}

fn game_data_fork_loaded() -> Rc<GameData> {
    loaded(&game_data_fork(true, true, true, 100, b"", b"", None))
}

/// The spaceport with a person's offer open over it, laid out.
fn offered_docked() -> AppScreen {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, game_data_fork_loaded());
    screen.spaceport.as_mut().expect("landed").open_offer(job());
    assert!(spaceport(&screen).offer_dialog().is_some());
    screen
}

#[test]
fn escape_tapped_during_a_wait_on_the_spaceports_map_leaves_after_the_close() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Escape);
    assert!(flight(&screen).map_open(), "still waiting");
    assert_eq!(screen.showing(), Showing::FlightMap);
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight, "the spaceport left");
}

#[test]
fn the_press_that_starts_the_wait_never_reaches_the_screen() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.input(&key(MAP, true));
    tap(&mut screen, Key::Escape);
    assert!(flight(&screen).map_waiting());
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport, "not left");
}

#[test]
fn a_key_tapped_during_a_wait_on_the_exchanges_map_acts_on_it_after_the_close() {
    // Buy: this exchange has one row for an arrow to step from.
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    let free = |screen: &AppScreen| {
        spaceport(screen)
            .open_trade()
            .expect("trading")
            .market()
            .free
    };
    let before = free(&screen);
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Char('b'));
    assert_eq!(free(&screen), before, "not yet");
    end_the_wait(&mut screen);
    assert!(free(&screen) < before, "bought");
}

#[test]
fn return_tapped_during_a_wait_on_the_exchanges_map_closes_it_after_the_close() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).open_trade().is_none(), "closed");
    assert_eq!(screen.showing(), Showing::Spaceport);
}

#[test]
fn an_offer_over_flight_answers_a_key_tapped_during_its_maps_wait() {
    let mut screen = offered_over_flight();
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(flight(&screen).map_waiting());
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::FlightMap, "not yet");
    end_the_wait(&mut screen);
    assert_ne!(screen.showing(), Showing::Offer, "answered");
}

#[test]
fn an_offers_refuse_key_tapped_during_its_maps_wait_answers_after_the_close() {
    let mut screen = offered_docked();
    screen.input(&key(MAP, true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    tap(&mut screen, Key::Char('n'));
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).offer_dialog().is_none(), "answered");
}

#[test]
fn an_offer_docked_answers_a_key_tapped_during_its_maps_wait() {
    let mut screen = offered_docked();
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    tap(&mut screen, Key::Enter);
    assert!(spaceport(&screen).offer_dialog().is_some(), "not yet");
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).offer_dialog().is_none(), "answered");
}

#[test]
fn p_tapped_during_a_port_maps_wait_opens_nothing() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    tap(&mut screen, PLAYER_INFO_KEY);
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).player_info().is_none());
}

#[test]
fn p_still_down_at_a_port_maps_close_opens_the_player_info() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).player_info().is_some());
}

#[test]
fn i_tapped_during_a_port_maps_wait_opens_nothing() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    tap(&mut screen, MISSION_INFO_KEY);
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).mission_info().is_none());
}

#[test]
fn a_click_on_a_button_during_a_port_maps_wait_does_nothing_after_the_close() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    let leave = spaceport(&screen)
        .dialog()
        .expect("laid out")
        .item_bounds(LEAVE_ITEM)
        .expect("Leave")
        .center();
    start_the_wait(&mut screen);
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: leave,
        });
    }
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport, "not left");
}

#[test]
fn a_key_that_opens_the_exchange_passes_the_next_on_to_it_as_it_flushes_nothing() {
    // `_DoTradeDialog` never calls `FlushEvents`.
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Char('c'));
    tap(&mut screen, Key::Escape);
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport, "not left");
    assert!(
        spaceport(&screen).open_trade().is_none(),
        "the exchange opened, and Escape closed it"
    );
}

#[test]
fn a_key_that_opens_a_flushing_screen_drops_those_behind_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, game_data_fork_loaded());
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Char('b'));
    tap(&mut screen, Key::Escape);
    end_the_wait(&mut screen);
    assert!(
        spaceport(&screen).open_bar().is_some(),
        "the bar opened, and Escape never reached it"
    );
}

#[test]
fn a_key_that_closes_the_exchange_passes_the_next_on_to_the_spaceport() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Escape);
    tap(&mut screen, Key::Escape);
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight, "the spaceport left");
}

#[test]
fn a_key_that_answers_an_offer_over_flight_leaves_the_next_unread_by_flight() {
    let mut screen = offered_over_flight();
    let still = ship(&screen);
    screen.input(&key(MAP, true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    tap(&mut screen, Key::Char('n'));
    screen.input(&key(Key::Up, true));
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight, "refused");
    screen.tick(TICK * 3);
    assert_eq!(ship(&screen), still, "no thrust");
}

#[test]
fn a_key_that_closes_a_text_with_another_behind_it_drops_those_behind_it() {
    // The next `_ShowTextDialog` flushes (@0x5ae67).
    let mut screen = with_dialogs_at_port_with_two_texts();
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    let shown = spaceport(&screen)
        .mission_text()
        .expect("the second text")
        .lines()
        .join(" ");
    assert_eq!(shown, "Two.");
}

/// The spaceport with two mission texts open, the first opening the map.
fn with_dialogs_at_port_with_two_texts() -> AppScreen {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, game_data_fork_loaded());
    let text = |text: &str, map| nova_sim::MissionText {
        kind: nova_sim::MissionTextKind::Comp,
        mission: MissionId(130),
        text: text.to_owned(),
        graphic: None,
        movie: None,
        map,
    };
    screen
        .spaceport
        .as_mut()
        .expect("landed")
        .show_mission_texts(vec![
            text("One.", Some(nova_sim::MapMarks::NONE)),
            text("Two.", None),
        ]);
    screen
}

#[test]
fn caps_lock_during_a_port_maps_wait_toggles_double_time_once() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    assert!(!flight(&screen).double_time());
    tap(&mut screen, Key::CapsLock);
    end_the_wait(&mut screen);
    assert!(flight(&screen).double_time(), "on, not toggled back");
}

#[test]
fn a_key_that_opens_a_flushing_screen_takes_its_release_along() {
    let store = MemoryPilots::new();
    let mut screen = landed_shipyard(&store);
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Char('s'));
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).open_shipyard().is_some(), "opened");
    assert!(!screen.keys_held.s, "S isn't left down");
}

#[test]
fn a_mission_text_over_flight_closed_by_a_replayed_key_drops_those_behind_it() {
    let mut screen = offered_over_flight();
    let text = |text: &str| DescShown {
        text: text.to_owned(),
        graphic: None,
        movie: None,
    };
    screen.show_over_flight(vec![text("One."), text("Two.")]);
    assert_eq!(screen.showing(), Showing::MissionText);
    let shown = |screen: &AppScreen| {
        screen
            .missions
            .as_ref()
            .and_then(|dialogs| dialogs.text())
            .map(|dialog| dialog.lines().join(" "))
    };
    // The texts have no map of their own, so the offer's is under them.
    let flight = screen.flight.as_mut().expect("flying");
    flight.open_dialog_map(nova_view::flight::MapOpener::Offer(MissionId(130)));
    screen.input(&key(Key::Escape, true));
    assert!(self::flight(&screen).map_waiting());
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    assert_eq!(shown(&screen).as_deref(), Some("Two."));
}
