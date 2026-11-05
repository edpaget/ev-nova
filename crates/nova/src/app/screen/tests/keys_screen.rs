//! The keys screen, opened from Preferences' Key Settings through the
//! real router: its OK puts the edited bindings into every screen at once
//! and keeps them for the app to save, and Cancel leaves the bindings as
//! they were. The keys screen's own rules are nova-view's (`ui::keys`).

use nova_view::keymap::{FlightCommand, UiCommand};
use nova_view::ui::keys::{KeysButton, KeysScreen};
use nova_view::ui::prefs::KEY_SETTINGS_ITEM;
use nova_view::{Bindings, KeyCombo, Modifiers, SharedBindings};

use super::*;

/// Presses and lets go of `k`.
fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// Opens Preferences with Command-comma, then its Key Settings.
fn open_keys(screen: &mut AppScreen) {
    screen.input(&key(Key::MetaLeft, true));
    tap(screen, Key::Comma);
    screen.input(&key(Key::MetaLeft, false));
    assert_eq!(screen.showing(), Showing::Preferences);
    let at = prefs_dialog(screen)
        .dialog()
        .item_bounds(KEY_SETTINGS_ITEM)
        .expect("Key Settings")
        .center();
    click_at(screen, at);
    assert!(keys(screen).is_some(), "the keys screen is open");
}

fn keys(screen: &AppScreen) -> Option<&KeysScreen> {
    screen.preferences().and_then(PrefsDialog::keys_screen)
}

/// Clicks the keys screen's `button`.
fn press(screen: &mut AppScreen, button: KeysButton) {
    let at = keys(screen).expect("open").button_bounds(button).center();
    click_at(screen, at);
}

/// In flight, opens the keys screen, records T on Accelerate (the row it
/// opens on), presses `button` and closes Preferences with its Escape.
fn rebind_accelerate_to_t(screen: &mut AppScreen, button: KeysButton) {
    fly(screen);
    open_keys(screen);
    tap(screen, Key::Enter);
    tap(screen, Key::KeyT);
    press(screen, button);
    assert!(keys(screen).is_none());
    tap(screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
}

/// Whether holding `k` in flight for a few ticks moves the ship.
fn thrusts_on(screen: &mut AppScreen, k: Key) -> bool {
    let still = ship(screen);
    screen.input(&key(k, true));
    screen.tick(TICK * 3);
    screen.input(&key(k, false));
    ship(screen).velocity != still.velocity
}

#[test]
fn ok_rebinds_accelerate_for_flight_at_once() {
    let mut screen = with_dialogs(data());
    rebind_accelerate_to_t(&mut screen, KeysButton::Ok);
    assert!(
        !thrusts_on(&mut screen, Key::ArrowUp),
        "Up no longer thrusts"
    );
    assert!(thrusts_on(&mut screen, Key::KeyT), "T thrusts");
    assert_eq!(
        screen
            .bindings()
            .get()
            .keyboard
            .combo(FlightCommand::Accelerate.into()),
        Some(KeyCombo::key(Key::KeyT))
    );
    let change = screen.take_bindings().expect("kept for the app");
    assert_eq!(change, screen.bindings().get());
    assert_eq!(screen.take_bindings(), None, "once");
}

#[test]
fn cancel_leaves_the_bindings_as_they_were() {
    let mut screen = with_dialogs(data());
    rebind_accelerate_to_t(&mut screen, KeysButton::Cancel);
    assert_eq!(screen.bindings().get(), Bindings::default());
    assert_eq!(screen.take_bindings(), None);
    assert!(thrusts_on(&mut screen, Key::ArrowUp));
}

#[test]
fn the_keys_screen_answers_its_fixed_keys_with_the_shared_ui_keys_rebound() {
    let mut rebound = Bindings::default();
    for (command, k) in [
        (UiCommand::Up, Key::F13),
        (UiCommand::Down, Key::F14),
        (UiCommand::Confirm, Key::F15),
        (UiCommand::Back, Key::F16),
        (UiCommand::Tab, Key::F17),
    ] {
        rebound = rebound.rebound(command, KeyCombo::key(k));
    }
    let mut screen = with_dialogs(data()).with_bindings(SharedBindings::new(rebound.clone()));
    open_keys(&mut screen);
    tap(&mut screen, Key::ArrowDown);
    tap(&mut screen, Key::Enter);
    assert_eq!(
        keys(&screen).and_then(KeysScreen::recording),
        Some(FlightCommand::ReverseCourse.into()),
        "Down and Return answer, though Up, Down and Confirm are rebound"
    );
    tap(&mut screen, Key::KeyK);
    tap(&mut screen, Key::Escape);
    assert!(keys(&screen).is_none(), "Escape cancels the keys screen");
    assert_eq!(screen.take_bindings(), None);
    assert_eq!(screen.bindings().get(), rebound);
}

#[test]
fn the_keys_screen_binds_none_of_the_menu_chords() {
    let mut screen = with_dialogs(data())
        .with_reserved_combos(vec![KeyCombo::with(Key::KeyH, Modifiers::COMMAND)]);
    open_keys(&mut screen);
    for k in [Key::KeyQ, Key::KeyH] {
        select_jettison(&mut screen);
        tap(&mut screen, Key::Enter);
        tap(&mut screen, k);
        assert_eq!(
            keys(&screen).and_then(KeysScreen::recording),
            Some(FlightCommand::JettisonCargo.into()),
            "{k}: Command-{k} is a menu's"
        );
        click_at(&mut screen, Point::new(1.0, 1.0));
    }
    assert_eq!(
        keys(&screen).map(KeysScreen::draft),
        Some(&Bindings::default())
    );
}

#[test]
fn the_menu_chords_are_command_q_and_command_comma() {
    assert_eq!(
        super::super::menu_chord_combos(),
        [
            KeyCombo::with(Key::Comma, Modifiers::COMMAND),
            KeyCombo::with(Key::KeyQ, Modifiers::COMMAND),
        ]
    );
}

/// Selects Jettison Cargo's row with Down.
fn select_jettison(screen: &mut AppScreen) {
    let row = keys(screen)
        .and_then(|keys| keys.row_of(FlightCommand::JettisonCargo.into()))
        .expect("a row");
    while keys(screen).and_then(|keys| keys.row_list().selected()) != Some(row) {
        tap(screen, Key::ArrowDown);
    }
}
