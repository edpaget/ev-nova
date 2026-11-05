//! The app's surroundings through the router: kept, told to a flight as it
//! is built, and passed on to the flight flying.

use super::*;
use nova_view::{ControllerFamily, Device, Surroundings};

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

#[test]
fn a_flight_built_after_the_surroundings_starts_reading_them() {
    let mut screen = AppScreen::new(data());
    assert_eq!(screen.input(&told(PAD)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser, "nothing else moves");
    fly(&mut screen);
    assert_eq!(flight(&screen).device(), PAD);
}

#[test]
fn the_flight_flying_follows_the_surroundings() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    assert_eq!(flight(&screen).device(), Device::Keyboard);
    assert_eq!(screen.input(&told(PAD)), ScreenAction::None);
    assert_eq!(flight(&screen).device(), PAD);
    screen.input(&told(Device::Mouse));
    assert_eq!(flight(&screen).device(), Device::Mouse);
}

#[test]
fn a_flight_left_for_another_screen_is_still_told_the_surroundings() {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    screen.input(&told(PAD));
    assert_eq!(flight(&screen).device(), PAD);
}

fn pad(button: nova_view::PadButton, pressed: bool) -> Input {
    Input::PadButton { button, pressed }
}

#[test]
fn an_open_overlay_follows_the_surroundings() {
    let mut screen = with_dialogs(data());
    screen.menu_command(MenuCommand::Preferences);
    assert_eq!(screen.showing(), Showing::Preferences);
    assert_eq!(screen.input(&told(PAD)), ScreenAction::None);
    assert_eq!(screen.preferences().expect("open").device(), PAD);
    screen.input(&told(Device::Mouse));
    assert_eq!(screen.preferences().expect("open").device(), Device::Mouse);
}

#[test]
fn an_overlay_opened_after_the_pad_was_used_is_told_so_as_it_opens() {
    let mut screen = with_dialogs(data());
    screen.input(&told(PAD));
    screen.menu_command(MenuCommand::Preferences);
    assert_eq!(screen.preferences().expect("open").device(), PAD);
}

#[test]
fn an_overlay_opened_again_is_told_again() {
    let mut screen = with_dialogs(data());
    screen.input(&told(PAD));
    screen.menu_command(MenuCommand::Preferences);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert!(screen.preferences().is_none(), "closed");
    screen.tick(TICK);
    screen.menu_command(MenuCommand::Preferences);
    assert_eq!(screen.preferences().expect("open").device(), PAD);
}

#[test]
fn a_flight_overlay_opened_by_a_pad_press_is_told_too() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&told(PAD));
    screen.input(&pad(nova_view::PadButton::Start, true));
    screen.input(&pad(nova_view::PadButton::RightStickDown, true));
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    assert_eq!(screen.flight_player_info().expect("open").device(), PAD);
}

/// An overlay opened outside any input (here mission texts queued over
/// flight, as a tick's `settle_script` queues them) is told the
/// surroundings by the tick that follows, before its first frame.
#[test]
fn an_overlay_a_tick_opens_is_told_the_surroundings() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&told(PAD));
    screen.show_over_flight(vec![nova_view::ui::mission_dialogs::DescShown::str_text(
        "Well done.",
        None,
    )]);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::MissionText);
    let text = screen.flight_mission_text().expect("shown").dialog();
    assert_eq!(text.device(), PAD);
    assert!(text.pad_highlight().is_some());
}

#[test]
fn the_spaceport_follows_the_surroundings() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    assert_eq!(spaceport(&screen).device(), Device::Keyboard);
    screen.input(&told(PAD));
    assert_eq!(spaceport(&screen).device(), PAD);
    screen.input(&told(Device::Mouse));
    assert_eq!(spaceport(&screen).device(), Device::Mouse);
}

#[test]
fn a_spaceport_landed_on_after_the_pad_was_used_is_told_as_it_opens() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&told(PAD));
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(spaceport(&screen).device(), PAD);
}

#[test]
fn a_spaceport_under_another_screen_is_still_told() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.menu_command(MenuCommand::Preferences);
    assert_eq!(screen.showing(), Showing::Preferences);
    screen.input(&told(PAD));
    assert_eq!(spaceport(&screen).device(), PAD);
}

/// South fires the primary weapon in flight. Pressed there and held
/// through the landing, its release at the spaceport takes nothing off,
/// and nor does anything replayed for it; only a fresh South on Leave,
/// where the pad's focus starts, does.
#[test]
fn a_south_held_through_the_landing_takes_off_only_when_pressed_again() {
    use nova_view::PadButton;
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&told(PAD));
    screen.input(&pad(PadButton::South, true));
    screen.tick(TICK);
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen.tick(TICK * 5);
    assert_eq!(screen.showing(), Showing::Spaceport, "held: no take-off");
    screen.input(&pad(PadButton::South, false));
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Spaceport, "released: none");
    let port = spaceport(&screen).dialog().expect("laid out");
    assert_eq!(port.pad_highlight(), Some(LEAVE_ITEM), "the focus shows");
    screen.input(&pad(PadButton::South, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "a press arms Leave");
    screen.input(&pad(PadButton::South, false));
    assert_eq!(screen.showing(), Showing::Flight, "its release takes off");
}
