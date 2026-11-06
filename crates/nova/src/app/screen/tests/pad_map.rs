//! The course map from a gamepad through the router
//! (rdm:phase/key-bindings/phase-10-gamepad-map): the router feeds flight's
//! pad reader every live input ([`FlightView::note_pad`]), so a map over
//! the spaceport closes on the pad's map chord, and its Back and the
//! surroundings reach the map wherever it opens.

use super::port::in_the_bar_by;
use super::*;
use nova_view::{ControllerFamily, Device, PadButton, Surroundings};

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings::of(device))
}

fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton {
        button,
        pressed,
        repeat: false,
    }
}

fn chord(screen: &mut AppScreen) {
    screen.input(&pad(PadButton::Start, true));
    screen.input(&pad(PadButton::RightStickUp, true));
}

fn chord_up(screen: &mut AppScreen, start_first: bool) {
    let order = if start_first {
        [PadButton::Start, PadButton::RightStickUp]
    } else {
        [PadButton::RightStickUp, PadButton::Start]
    };
    for button in order {
        screen.input(&pad(button, false));
    }
}

#[test]
fn the_bars_map_opens_and_closes_on_the_pads_chord_in_either_release_order() {
    for start_first in [true, false] {
        let mut screen = in_the_bar_by(Rulebook::default());
        chord(&mut screen);
        assert_eq!(screen.showing(), Showing::FlightMap, "{start_first}");
        chord_up(&mut screen, start_first);
        assert!(flight(&screen).map_open(), "{start_first}: still open");
        chord(&mut screen);
        assert!(flight(&screen).map_waiting(), "{start_first}: waits");
        chord_up(&mut screen, start_first);
        assert!(!flight(&screen).map_open(), "{start_first}: closed");
        assert_eq!(screen.showing(), Showing::Spaceport, "{start_first}");
        assert!(spaceport(&screen).open_bar().is_some(), "back in the bar");
    }
}

#[test]
fn the_course_map_is_told_over_flight_and_over_the_spaceport() {
    let mut screen = in_the_bar_by(Rulebook::default());
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::FlightMap);
    screen.input(&told(PAD));
    assert_eq!(flight(&screen).course_map().device(), PAD, "the bar's map");

    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert!(flight(&screen).map_open());
    screen.input(&told(PAD));
    assert_eq!(flight(&screen).course_map().device(), PAD, "flight's map");
}

#[test]
fn east_closes_flights_map_as_escape_does() {
    for back in [
        [key(Key::Escape, true), key(Key::Escape, false)],
        [pad(PadButton::East, true), pad(PadButton::East, false)],
    ] {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.input(&told(PAD));
        screen.input(&key(MAP, true));
        screen.input(&key(MAP, false));
        assert!(flight(&screen).map_open());
        for input in &back {
            screen.input(input);
        }
        assert!(!flight(&screen).map_open(), "{back:?}");
        assert_eq!(screen.showing(), Showing::Flight, "{back:?}");
    }
}

#[test]
fn east_closes_a_port_map_as_escape_does() {
    for back in [
        [key(Key::Escape, true), key(Key::Escape, false)],
        [pad(PadButton::East, true), pad(PadButton::East, false)],
    ] {
        let mut screen = in_the_bar_by(Rulebook::default());
        screen.input(&told(PAD));
        screen.input(&key(MAP, true));
        screen.input(&key(MAP, false));
        assert_eq!(screen.showing(), Showing::FlightMap);
        for input in &back {
            screen.input(input);
        }
        assert!(!flight(&screen).map_open(), "{back:?}");
        assert!(spaceport(&screen).open_bar().is_some(), "{back:?}");
    }
}
