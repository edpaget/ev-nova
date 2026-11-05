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
