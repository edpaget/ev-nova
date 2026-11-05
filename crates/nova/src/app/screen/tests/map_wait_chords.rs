use super::held_press_chords::{chord, command, preferences_state};
use super::map_wait_keys::{
    end_the_wait, game_data_fork_loaded, offer_map_waiting, offered_over_flight, start_the_wait,
    tap,
};
use super::*;

// The menu chords typed while a map waits for M's release act at its
// close, after the keys typed before them (the `held_chords` module's map
// hold): `_DoSystemMap`'s wait reads no events (@0x13a2e-0x13a46), so the
// chord waits in the queue for the next reader.

/// Command-comma, Command let go of after it.
fn command_comma(screen: &mut AppScreen) {
    chord(screen, PREFERENCES_KEY);
    screen.input(&command(false));
}

/// Asserts the map still waits and no Preferences are open.
fn still_waiting(screen: &AppScreen) {
    assert!(flight(screen).map_waiting(), "still waiting");
    assert!(screen.preferences().is_none(), "not at the chord");
}

#[test]
fn command_comma_during_the_spaceports_maps_wait_opens_the_preferences_at_ms_release() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    command_comma(&mut screen);
    still_waiting(&screen);
    end_the_wait(&mut screen);
    assert!(screen.preferences().is_some(), "open after the close");
}

/// Past the close, the chord reaches the offer under the map, and does
/// what Command-comma typed over it does: the offer in front takes it
/// today, which rdm:phase/front-end/phase-14-preferences-over-flight-dialogs
/// changes to open the Preferences.
#[test]
fn command_comma_during_a_dialogs_maps_wait_over_flight_goes_to_the_offer_at_ms_release() {
    let mut screen = offer_map_waiting();
    command_comma(&mut screen);
    still_waiting(&screen);
    end_the_wait(&mut screen);
    assert!(screen.offer().is_some(), "the offer still open");

    let mut typed = offered_over_flight();
    command_comma(&mut typed);
    assert_eq!(
        preferences_state(&mut screen),
        preferences_state(&mut typed),
        "as typed over the offer with no map"
    );
}

#[test]
fn command_comma_during_flights_own_maps_wait_opens_the_preferences_at_ms_release() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    start_the_wait(&mut screen);
    command_comma(&mut screen);
    still_waiting(&screen);
    end_the_wait(&mut screen);
    assert!(screen.preferences().is_some(), "open over flight");
}

#[test]
fn command_let_go_before_the_close_still_makes_the_chord() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    tap(&mut screen, MAP);
    screen.input(&command(true));
    screen.input(&key(MAP, true));
    assert!(flight(&screen).map_waiting(), "waiting");
    tap(&mut screen, PREFERENCES_KEY);
    screen.input(&command(false));
    still_waiting(&screen);
    end_the_wait(&mut screen);
    assert!(screen.preferences().is_some(), "resolved at its press");
}

/// Tab after the chord moves the Preferences' focus, keeping them open.
#[test]
fn keys_before_the_chord_go_to_the_screen_and_after_it_to_the_preferences() {
    let mut screen = offer_map_waiting();
    tap(&mut screen, Key::Char('n'));
    command_comma(&mut screen);
    tap(&mut screen, Key::Tab);
    end_the_wait(&mut screen);
    assert!(screen.offer().is_none(), "refused first");
    assert!(screen.preferences().is_some(), "then opened");

    let mut typed = offered_over_flight();
    tap(&mut typed, Key::Char('n'));
    command_comma(&mut typed);
    tap(&mut typed, Key::Tab);
    assert_eq!(
        preferences_state(&mut screen),
        preferences_state(&mut typed),
        "as typed with no wait"
    );
}

#[test]
fn a_key_before_the_chord_that_opens_a_flushing_screen_drops_it() {
    let store = MemoryPilots::new();
    let mut screen = landed_at_venues(&store, game_data_fork_loaded());
    start_the_wait(&mut screen);
    tap(&mut screen, Key::Char('b'));
    command_comma(&mut screen);
    end_the_wait(&mut screen);
    assert!(spaceport(&screen).open_bar().is_some(), "the bar opened");
    assert!(
        screen.preferences().is_none(),
        "its flush dropped the chord"
    );
}

#[test]
fn command_q_during_a_maps_wait_quits_at_ms_release() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    assert_eq!(chord(&mut screen, Key::Char('q')), [ScreenAction::None; 3]);
    screen.input(&command(false));
    assert!(!screen.take_quit(), "not at the chord");
    assert!(flight(&screen).map_waiting(), "still waiting");
    assert_eq!(screen.input(&key(MAP, false)), ScreenAction::Quit);
}

#[test]
fn the_menu_bars_preferences_during_a_maps_wait_open_at_ms_release() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    assert_eq!(
        screen.menu_command(MenuCommand::Preferences),
        ScreenAction::None
    );
    still_waiting(&screen);
    end_the_wait(&mut screen);
    assert!(screen.preferences().is_some(), "as Command-comma does");
}

#[test]
fn keys_let_go_of_after_the_chord_close_the_map_and_open_the_preferences() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    start_the_wait(&mut screen);
    command_comma(&mut screen);
    screen.release_keys();
    assert!(!flight(&screen).map_open(), "closed");
    assert!(screen.preferences().is_some(), "the held chord ran");
}
