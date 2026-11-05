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

#[test]
fn the_keys_screen_names_keys_through_the_key_names_the_bindings_carry() {
    let names = nova_view::key_names::fixture::FakeKeyNames::azerty();
    let mut screen = with_dialogs(data())
        .with_bindings(nova_view::SharedBindings::default().with_key_names(Rc::new(names)));
    open_keys(&mut screen);
    let keys = keys(&screen).expect("open");
    let row = keys
        .row_of(FlightCommand::SelectSecondary.into())
        .expect("a row");
    assert_eq!(keys.row_list().cell(row), Some("Z"));
}

/// What `events` from the fake pads mean for the screen, through the
/// app's pad state ([`PadInputs`](crate::gamepad::inputs::PadInputs)),
/// given to `screen` in order.
fn from_pads(
    screen: &mut AppScreen,
    pads: &mut crate::gamepad::inputs::PadInputs,
    events: Vec<crate::gamepad::PadEvent>,
) {
    use crate::gamepad::Gamepads;
    let mut port = crate::gamepad::fixture::FakeGamepads::with([events]);
    let mut polled = Vec::new();
    port.poll(&mut polled);
    let mut inputs = Vec::new();
    pads.feed(&polled, &mut inputs);
    for input in inputs {
        screen.input(&input);
    }
}

/// The keys screen recording Jettison Cargo on its gamepad tab, told a
/// pad was used last.
fn recording_jettison_on_the_pad(screen: &mut AppScreen) {
    use nova_view::{ControllerFamily, Device, Surroundings};
    open_keys(screen);
    screen.input(&Input::Surroundings(Surroundings {
        device: Device::Gamepad(ControllerFamily::Generic),
    }));
    tap(screen, Key::ArrowRight);
    select_jettison(screen);
    tap(screen, Key::Enter);
    assert_eq!(
        keys(screen).and_then(KeysScreen::recording),
        Some(FlightCommand::JettisonCargo.into())
    );
}

/// The keys screen reads a stick only as the stick directions the app's
/// pad state presses past its threshold
/// ([`PRESS_THRESHOLD`](crate::gamepad::inputs::PRESS_THRESHOLD)): a
/// stick inside it records nothing, and one past it its direction.
#[test]
fn recording_on_the_gamepad_tab_a_stick_binds_its_direction_only_past_the_threshold() {
    use crate::gamepad::inputs::{PRESS_THRESHOLD, PadInputs};
    use crate::gamepad::{PadEvent, PadId};
    use nova_view::keymap::{PadBinding, PadEntry};
    use nova_view::{PadAxis, PadButton};
    const ONE: PadId = PadId(0);
    let stick = |value| PadEvent::Axis {
        pad: ONE,
        axis: PadAxis::LeftX,
        value,
    };
    let mut screen = with_dialogs(data());
    let mut pads = PadInputs::new();
    recording_jettison_on_the_pad(&mut screen);
    let before = keys(&screen).map(KeysScreen::draft).cloned();
    from_pads(
        &mut screen,
        &mut pads,
        vec![PadEvent::connected(ONE), stick(PRESS_THRESHOLD * 0.9)],
    );
    assert_eq!(
        keys(&screen).and_then(KeysScreen::recording),
        Some(FlightCommand::JettisonCargo.into()),
        "inside the threshold, still recording"
    );
    assert_eq!(keys(&screen).map(KeysScreen::draft).cloned(), before);
    from_pads(
        &mut screen,
        &mut pads,
        vec![stick(f32::midpoint(PRESS_THRESHOLD, 1.0))],
    );
    let keys = keys(&screen).expect("open");
    assert_eq!(keys.recording(), None);
    assert_eq!(
        keys.draft()
            .gamepad
            .binding(FlightCommand::JettisonCargo.into()),
        PadBinding::Entry(PadEntry::plain(PadButton::LeftStickRight))
    );
}
