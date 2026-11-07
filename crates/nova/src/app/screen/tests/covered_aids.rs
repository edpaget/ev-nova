//! What the app shows over the spaceport or flight covers it
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): the spaceport's
//! play keys hide under the course map over it and under a dialog of the
//! app's, and the course map's Plot Course toggle under a dialog of the
//! app's in front of it. What a covered aid does, and how the spaceport
//! and flight pass it on, is `nova_view`'s; this pins the wiring alone.

use super::*;
use nova_view::{Device, Surroundings};

fn touched(screen: &mut AppScreen) {
    screen.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
}

fn play_keys_shown(screen: &AppScreen) -> bool {
    screen.spaceport_view().expect("landed").play_keys().shown()
}

fn map_toggle_shown(screen: &AppScreen) -> bool {
    flight(screen).course_map().touch_toggle().shown()
}

#[test]
fn the_play_keys_hide_under_the_preferences_and_the_course_map_over_the_spaceport() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    touched(&mut screen);
    assert!(play_keys_shown(&screen), "shown");
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Preferences);
    assert!(!play_keys_shown(&screen), "under the preferences");
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(play_keys_shown(&screen), "back");
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::FlightMap);
    assert!(!play_keys_shown(&screen), "under the map");
}

#[test]
fn the_maps_toggle_hides_under_the_preferences_over_it() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    touched(&mut screen);
    screen.input(&key(MAP, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    assert!(map_toggle_shown(&screen), "shown on the map");
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Preferences, "over the map");
    assert!(!map_toggle_shown(&screen), "under the preferences");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    assert!(map_toggle_shown(&screen), "back");
}

#[test]
fn a_dialogs_map_over_flight_shows_its_toggle_over_the_dialog_it_opened_from() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    touched(&mut screen);
    super::dialog_maps::offer_job(&mut screen);
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert!(flight(&screen).dialog_map_open());
    assert!(map_toggle_shown(&screen), "in front of the offer");
    command_comma(&mut screen);
    assert!(screen.preferences().is_some());
    assert!(!map_toggle_shown(&screen), "under the preferences");
}
