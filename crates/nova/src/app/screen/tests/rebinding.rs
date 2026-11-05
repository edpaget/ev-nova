//! The router reads its keys through the bindings it is given
//! ([`AppScreen::with_bindings`]): a rebound command's new key acts, and
//! its old key no longer does.

use nova_view::keymap::{AppCommand, MainMenuCommand, MainScreenCommand, PauseCommand, UiCommand};
use nova_view::{Bindings, KeyCombo, SharedBindings};

use super::menu::menu_with_dialogs;
use super::*;

/// `bindings` as the handle the router is given.
fn shared(bindings: Bindings) -> SharedBindings {
    SharedBindings::new(bindings)
}

#[test]
fn a_rebound_fly_key_enters_flight_and_f_no_longer_does() {
    let rebound = Bindings::default().rebound(MainScreenCommand::Fly, KeyCombo::key(Key::KeyT));
    let mut screen = AppScreen::new(data()).with_bindings(shared(rebound));
    screen.input(&key(Key::KeyF, true));
    screen.input(&key(Key::KeyF, false));
    assert_eq!(screen.showing(), Showing::ShipBrowser, "F");
    assert!(screen.flight_view().is_none(), "F");
    screen.input(&key(Key::KeyT, true));
    assert_eq!(screen.showing(), Showing::Flight, "T");
}

#[test]
fn a_rebound_back_key_quits_from_the_ship_browser_and_escape_no_longer_does() {
    let rebound = Bindings::default().rebound(UiCommand::Back, KeyCombo::key(Key::KeyT));
    let mut screen = AppScreen::new(data()).with_bindings(shared(rebound));
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::KeyT, true)), ScreenAction::Quit);
}

#[test]
fn a_rebound_tab_key_switches_sides_and_tab_no_longer_does() {
    let rebound = Bindings::default().rebound(UiCommand::Tab, KeyCombo::key(Key::KeyT));
    let mut screen = AppScreen::new(data()).with_bindings(shared(rebound));
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser, "Tab");
    screen.input(&key(Key::KeyT, true));
    assert_eq!(screen.showing(), Showing::GalaxyMap, "T");
}

#[test]
fn a_rebound_pause_key_leaves_flight_and_escape_no_longer_does() {
    let rebound = Bindings::default().rebound(PauseCommand::Pause, KeyCombo::key(Key::F13));
    let mut screen = AppScreen::new(data()).with_bindings(shared(rebound));
    fly(&mut screen);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Flight, "Escape");
    screen.input(&key(Key::F13, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser, "F13");
}

#[test]
fn a_rebound_double_time_key_toggles_it_and_caps_lock_no_longer_does() {
    let rebound = Bindings::default().rebound(AppCommand::DoubleTime, KeyCombo::key(Key::F13));
    let mut screen = with_dialogs(data()).with_bindings(shared(rebound));
    fly(&mut screen);
    let double_time = |screen: &AppScreen| screen.flight_view().expect("flying").double_time();
    screen.input(&key(Key::CapsLock, true));
    assert!(!double_time(&screen), "Caps Lock");
    screen.input(&key(Key::F13, true));
    assert!(double_time(&screen), "F13");
}

#[test]
fn a_rebound_open_guard_holds_open_back_and_s_no_longer_does() {
    let store = MemoryPilots::new();
    let rebound =
        Bindings::default().rebound(MainMenuCommand::OpenGuardS, KeyCombo::key(Key::KeyK));
    let mut screen = menu_with_dialogs(&store).with_bindings(shared(rebound.clone()));
    screen.input(&key(Key::KeyK, true));
    screen.input(&Input::Text('o'));
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::MainMenu, "K held");

    let mut screen = menu_with_dialogs(&store).with_bindings(shared(rebound));
    screen.input(&key(Key::KeyS, true));
    screen.input(&Input::Text('o'));
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::OpenPilot, "S held");
}

#[test]
fn a_rebound_about_text_key_opens_it_and_a_no_longer_does() {
    let store = MemoryPilots::new();
    let rebound = Bindings::default().rebound(MainMenuCommand::About, KeyCombo::key(Key::KeyK));
    let mut screen = menu_with_dialogs(&store).with_bindings(shared(rebound));
    screen.input(&key(Key::KeyA, true));
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::MainMenu, "A");
    screen.input(&key(Key::KeyA, false));
    screen.input(&key(Key::KeyK, true));
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::About, "K");
}
