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

#[test]
fn the_player_info_opened_by_its_rebound_key_waits_for_that_key_s_release() {
    use nova_view::keymap::PlayCommand;
    let rebound = Bindings::default().rebound(PlayCommand::PlayerInfo, KeyCombo::key(Key::KeyQ));
    let mut screen = with_dialogs(data()).with_bindings(shared(rebound));
    fly(&mut screen);
    screen.input(&key(Key::KeyQ, true));
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    screen.input(&key(Key::Enter, true));
    screen.input(&key(Key::Enter, false));
    assert_eq!(screen.showing(), Showing::PlayerInfo, "waiting");
    screen.input(&key(Key::KeyQ, false));
    assert_eq!(screen.showing(), Showing::Flight, "closed on Q's release");
}

#[test]
fn the_router_hands_back_the_bindings_it_was_given() {
    let rebound = shared(Bindings::default().rebound(UiCommand::Tab, KeyCombo::key(Key::KeyT)));
    let screen = AppScreen::new(data()).with_bindings(rebound.clone());
    assert_eq!(screen.bindings().get(), rebound.get());
}

#[test]
fn the_ship_browser_built_before_the_bindings_reads_them() {
    let rebound = Bindings::default().rebound(UiCommand::Right, KeyCombo::key(Key::KeyL));
    let mut screen = AppScreen::new(data()).with_bindings(shared(rebound));
    screen.input(&key(Key::ArrowRight, true));
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)), "Right");
    screen.input(&key(Key::KeyL, true));
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)), "L");
}

#[test]
fn the_galaxy_viewer_built_before_the_bindings_reads_them() {
    let rebound = Bindings::default().rebound(UiCommand::Right, KeyCombo::key(Key::KeyL));
    let mut screen = AppScreen::new(data()).with_bindings(shared(rebound));
    screen.input(&key(Key::Tab, true));
    let fitted = *screen.galaxy_map().view().expect("the viewer");
    screen.input(&key(Key::ArrowRight, true));
    assert_eq!(
        *screen.galaxy_map().view().expect("the viewer"),
        fitted,
        "Right"
    );
    screen.input(&key(Key::KeyL, true));
    assert_ne!(
        *screen.galaxy_map().view().expect("the viewer"),
        fitted,
        "L pans"
    );
}

#[test]
fn a_spaceport_built_before_the_bindings_reads_them() {
    use nova_view::keymap::PlayCommand;
    let mut screen = with_dialogs(data());
    land(&mut screen);
    let rebound = Bindings::default().rebound(PlayCommand::PlayerInfo, KeyCombo::key(Key::KeyQ));
    let mut screen = screen.with_bindings(shared(rebound));
    screen.input(&key(PLAYER_INFO_KEY, true));
    screen.input(&key(PLAYER_INFO_KEY, false));
    assert!(spaceport(&screen).player_info().is_none(), "P");
    screen.input(&key(Key::KeyQ, true));
    assert!(spaceport(&screen).player_info().is_some(), "Q");
}
