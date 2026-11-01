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

/// [`offered_over_flight`] with its map open and waiting.
fn offer_map_waiting() -> AppScreen {
    let mut screen = offered_over_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(flight(&screen).map_waiting());
    screen
}

#[test]
fn a_key_that_answers_an_offer_over_flight_leaves_the_next_unread_by_flight() {
    let mut screen = offer_map_waiting();
    tap(&mut screen, Key::Char('n'));
    tap(&mut screen, LAND);
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight, "refused");
    // Two presses land: the replay must not have been the first.
    screen.tick(TICK);
    tap(&mut screen, LAND);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Flight, "L's first press");
    tap(&mut screen, LAND);
    assert_eq!(screen.showing(), Showing::Spaceport, "L's second");
}

#[test]
fn an_unlatched_key_tapped_behind_the_answer_does_nothing_in_flight() {
    let mut screen = offer_map_waiting();
    let still = ship(&screen);
    tap(&mut screen, Key::Char('n'));
    tap(&mut screen, Key::Up);
    end_the_wait(&mut screen);
    screen.tick(TICK * 3);
    assert_eq!(ship(&screen), still, "no thrust");
}

#[test]
fn an_unlatched_key_still_down_as_the_last_dialog_closes_reads_down_in_flight() {
    // `_HandlePlayer` polls it (`_QuickKeyCheck` @0xa0f5).
    let mut screen = offer_map_waiting();
    let still = ship(&screen);
    tap(&mut screen, Key::Char('n'));
    screen.input(&key(Key::Up, true));
    end_the_wait(&mut screen);
    screen.tick(TICK * 3);
    assert_ne!(ship(&screen), still, "thrust");
    screen.input(&key(Key::Up, false));
    let velocity = |screen: &AppScreen| ship(screen).velocity.length();
    let coasting = velocity(&screen);
    screen.tick(TICK * 3);
    assert!(velocity(&screen) <= coasting, "let go live");
}

#[test]
fn an_unlatched_key_replayed_to_the_offer_and_still_down_reads_down_in_flight() {
    let mut screen = offer_map_waiting();
    let still = ship(&screen);
    screen.input(&key(Key::Up, true));
    tap(&mut screen, Key::Char('n'));
    end_the_wait(&mut screen);
    screen.tick(TICK * 3);
    assert_ne!(ship(&screen), still, "thrust");
}

#[test]
fn an_unlatched_key_still_down_at_a_take_off_reads_down_in_flight() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Escape);
    screen.input(&key(Key::Up, true));
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK * 3);
    assert_ne!(
        ship(&screen).velocity,
        nova_sim::ShipState::default().velocity
    );
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

#[test]
fn a_key_that_opens_an_overlay_drops_those_behind_it() {
    let mut screen = offered_over_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    screen.input(&key(PLAYER_INFO_KEY, true));
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::PlayerInfo);
}

#[test]
fn a_key_that_opens_the_player_info_over_a_port_offer_drops_those_behind_it() {
    let mut screen = offered_docked();
    screen.input(&key(MAP, true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    screen.input(&key(PLAYER_INFO_KEY, true));
    tap(&mut screen, Key::Escape);
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).player_info().is_some(), "still open");
}

#[test]
fn a_key_that_opens_mission_info_over_an_offer_over_flight_drops_those_behind_it() {
    let (mut screen, _reel) = super::missions::holding_and_offered_in_flight();
    screen.input(&key(MAP, true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    screen.input(&key(MISSION_INFO_KEY, true));
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    assert!(screen.flight_mission_info().is_some(), "still open");
}

#[test]
fn command_down_across_a_key_that_flushes_is_let_go_with_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, game_data_fork_loaded());
    start_the_wait(&mut screen);
    screen.input(&key(Key::Command, true));
    tap(&mut screen, Key::Char('b'));
    screen.input(&key(Key::Command, false));
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).open_bar().is_some(), "the bar opened");
    assert_eq!(
        screen.input(&key(Key::Char('q'), true)),
        ScreenAction::None,
        "a plain Q, no Command-Q"
    );
}

#[test]
fn command_down_across_escape_taking_off_is_let_go_with_it() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    screen.input(&key(Key::Command, true));
    tap(&mut screen, Key::Escape);
    screen.input(&key(Key::Command, false));
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight, "took off");
    assert_eq!(
        screen.input(&key(Key::Char('q'), true)),
        ScreenAction::None,
        "a plain Q, no Command-Q"
    );
}

#[test]
fn a_press_dropped_by_a_flush_leaves_no_key_down_and_its_key_still_lets_go_live() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, game_data_fork_loaded());
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Char('b'));
    screen.input(&key(Key::Command, true));
    end_the_wait(&mut screen);
    assert_eq!(
        screen.input(&key(Key::Char('q'), true)),
        ScreenAction::Quit,
        "Command is physically down, so Command-Q stands"
    );
    screen.input(&key(Key::Char('q'), false));
    screen.input(&key(Key::Command, false));
    assert_eq!(screen.input(&key(Key::Char('q'), true)), ScreenAction::None);
}

#[test]
fn alt_down_across_a_refused_sale_is_let_go_with_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_outfitter_over(&store, pod_data());
    click_port_item(&mut screen, 8);
    select_first_outfit(&mut screen);
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Right, true));
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Left, true));
    start_the_wait(&mut screen);
    screen.input(&key(Key::Alt, true));
    tap(&mut screen, Key::Char('s'));
    screen.input(&key(Key::Alt, false));
    end_the_wait(&mut screen);
    let open = spaceport(&screen).open_outfitter().expect("outfitting");
    assert!(open.alert_dialog().is_some(), "the sale refused");
    assert!(!open.alt_held(), "Alt let go");
}

/// A pilot in flight, her movies played through the
/// [`MockMoviePlayer`] given back.
fn flying_filmed() -> (AppScreen, Rc<MockMoviePlayer>) {
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(game_data_fork_loaded())
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    (screen, reel)
}

/// A mission text saying `text`, opening the map on `map`, its
/// after-movie `movie`.
fn mission_text(
    text: &str,
    map: Option<nova_sim::MapMarks>,
    movie: Option<nova_sim::DescMovie>,
) -> nova_sim::MissionText {
    nova_sim::MissionText {
        kind: nova_sim::MissionTextKind::Comp,
        mission: MissionId(130),
        text: text.to_owned(),
        graphic: None,
        movie,
        map,
    }
}

/// The after-movie the tests' filmed texts play.
fn after_movie() -> Option<nova_sim::DescMovie> {
    nova_sim::DescMovie::from_desc("After.mov", 0x0001)
}

/// The spaceport with `texts` open over it, and the
/// [`MockMoviePlayer`] that plays their movies.
fn landed_with_texts(texts: Vec<nova_sim::MissionText>) -> (AppScreen, Rc<MockMoviePlayer>) {
    let (mut screen, reel) = flying_filmed();
    land_twice(&mut screen);
    screen
        .spaceport
        .as_mut()
        .expect("landed")
        .show_mission_texts(texts);
    (screen, reel)
}

/// The spaceport with a text open over it that opens the map, its
/// after-movie next, and the [`MockMoviePlayer`] that plays it.
fn landed_with_a_filmed_text() -> (AppScreen, Rc<MockMoviePlayer>) {
    landed_with_texts(vec![mission_text(
        "Done.",
        Some(nova_sim::MapMarks::NONE),
        after_movie(),
    )])
}

#[test]
fn a_key_that_closes_a_text_onto_its_movie_doesnt_skip_it_with_its_release() {
    let (mut screen, reel) = landed_with_a_filmed_text();
    let before = reeled(&reel).len();
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    screen.tick(TICK);
    assert_eq!(
        reeled(&reel)[before..],
        [Call::Start("After.mov".into())],
        "started, and nothing skipped it"
    );
}

#[test]
fn a_key_behind_a_text_closing_onto_its_movie_waits_for_what_comes_next() {
    // `_PlayQTMovie` flushes only when skipped (@0x55e9f).
    let (mut screen, _reel) = landed_with_a_filmed_text();
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Escape);
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport, "the movie isn't over");
    assert!(spaceport(&screen).playing());
    while spaceport(&screen).playing() {
        screen.tick(TICK);
    }
    assert!(
        screen
            .spaceport_view()
            .is_none_or(nova_view::spaceport::SpaceportView::left),
        "Escape reached the spaceport"
    );
}

#[test]
fn a_key_known_only_by_its_repeats_and_let_go_isnt_held_past_the_last_dialog() {
    // The press went to the map; only its repeats and release came after.
    let mut screen = offer_map_waiting();
    let still = ship(&screen);
    screen.input(&held(Key::Up));
    screen.input(&key(Key::Up, false));
    tap(&mut screen, Key::Char('n'));
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight, "refused");
    screen.tick(TICK * 3);
    assert_eq!(ship(&screen), still, "no thrust");
}

#[test]
fn a_key_known_only_by_its_repeats_and_still_down_reads_down_past_the_last_dialog() {
    let mut screen = offer_map_waiting();
    let still = ship(&screen);
    screen.input(&held(Key::Up));
    tap(&mut screen, Key::Char('n'));
    end_the_wait(&mut screen);
    screen.tick(TICK * 3);
    assert_ne!(ship(&screen), still, "thrust");
}

#[test]
fn a_replay_stopped_facing_a_movie_doesnt_skip_it_with_a_release() {
    let (mut screen, reel) = landed_with_texts(vec![
        mission_text("One.", Some(nova_sim::MapMarks::NONE), after_movie()),
        mission_text("Two.", None, None),
    ]);
    let before = reeled(&reel).len();
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    screen.tick(TICK);
    assert_eq!(
        reeled(&reel)[before..],
        [Call::Start("After.mov".into())],
        "started, and Return's release didn't skip it"
    );
    while spaceport(&screen).playing() {
        screen.tick(TICK);
    }
    let shown = spaceport(&screen).mission_text().expect("the second text");
    assert_eq!(shown.lines().join(" "), "Two.", "after the movie");
}

#[test]
fn a_movie_over_flight_isnt_skipped_by_a_key_queued_behind_the_text_before_it() {
    let (mut screen, reel) = flying_filmed();
    screen.show_over_flight(vec![DescShown {
        text: "One.".to_owned(),
        graphic: None,
        movie: after_movie(),
    }]);
    assert_eq!(screen.showing(), Showing::MissionText);
    let before = reeled(&reel).len();
    let flying = screen.flight.as_mut().expect("flying");
    flying.open_dialog_map(nova_view::flight::MapOpener::Briefing(
        nova_sim::MapMarks::NONE,
    ));
    screen.input(&key(Key::Escape, true));
    assert!(flight(&screen).map_waiting());
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::Enter);
    end_the_wait(&mut screen);
    assert_eq!(screen.showing(), Showing::Movie);
    screen.tick(TICK);
    assert_eq!(
        reeled(&reel)[before..],
        [Call::Start("After.mov".into())],
        "started, and the second Return didn't skip it"
    );
}
