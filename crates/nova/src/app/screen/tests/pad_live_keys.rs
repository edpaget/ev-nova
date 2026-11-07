//! The pad held under a dialog over flight, handed back as the last one
//! closes (the `live_input` module): its buttons with the layers they
//! went down under, its sticks as they stand, and the press the closing
//! dialog took let go. Wiring through the real router and flight: what
//! flight takes back of a pad, and how, is nova-view's
//! (`flight::view::tests::pad_live`), the step's fire among it, as these
//! fixtures' ships carry no weapon.

use super::encounters::{boarding_data, hailable};
use super::*;
use nova_view::{ControllerFamily, Device, PadAxis, PadButton, Surroundings};

fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton {
        button,
        pressed,
        repeat: false,
    }
}

/// How fast the player's ship goes.
fn speed(screen: &AppScreen) -> f32 {
    ship(screen).velocity.length()
}

/// Whether `screen`'s flight holds a button or key down that acts
/// ([`FlightView::holds_keys`]).
fn holds(screen: &AppScreen) -> bool {
    flight(screen).holds_keys()
}

/// Flight in Sol with a hailable ship about and the pad in use, `held`
/// pressed on the pad first, then the comm dialog opened over it by Tab
/// and Y.
fn hailed_holding(held: &[PadButton]) -> AppScreen {
    let mut screen = hailable(with_dialogs(boarding_data()));
    fly(&mut screen);
    // F, which entered flight, is let go of: flight holds nothing.
    screen.input(&key(Key::KeyF, false));
    // The pad in use, so the comm dialog's focus shows for South.
    screen.input(&Input::Surroundings(Surroundings::of(Device::Gamepad(
        ControllerFamily::Generic,
    ))));
    for &button in held {
        screen.input(&pad(button, true));
    }
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Comm);
    screen
}

/// Closes the comm dialog with Escape.
fn escape(screen: &mut AppScreen) {
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Flight, "the channel closed");
}

#[test]
fn dpad_up_held_through_the_comm_dialog_closed_with_east_thrusts_on_the_first_step() {
    // As the up arrow held across the same dialog does, tick for tick.
    let mut pad_held = hailed_holding(&[PadButton::DPadUp]);
    pad_held.input(&pad(PadButton::East, true));
    assert_eq!(pad_held.showing(), Showing::Flight, "East closed it");
    pad_held.input(&pad(PadButton::East, false));
    let mut up_held = hailable(with_dialogs(boarding_data()));
    fly(&mut up_held);
    up_held.input(&key(Key::ArrowUp, true));
    hail(&mut up_held);
    escape(&mut up_held);
    let closed = speed(&pad_held);
    assert_eq!(ship(&up_held), ship(&pad_held));
    for tick in 1..=3 {
        pad_held.tick(TICK);
        up_held.tick(TICK);
        assert_eq!(ship(&pad_held), ship(&up_held), "tick {tick}");
    }
    assert!(speed(&pad_held) > closed, "thrust: {:?}", ship(&pad_held));
}

#[test]
fn east_closing_the_comm_dialog_doesnt_land_after_its_close() {
    // East is Land in flight, read held by its own latch: taken back, it
    // would land or ask to dock on the next pass.
    let mut screen = hailed_holding(&[]);
    screen.input(&pad(PadButton::East, true));
    assert_eq!(screen.showing(), Showing::Flight, "East closed it");
    assert!(!holds(&screen), "East let go, not held as Land");
    screen.tick(TICK * 4);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.input(&pad(PadButton::East, false));
    assert!(!holds(&screen), "its release means nothing");
}

#[test]
fn a_throttle_stick_held_still_through_the_comm_dialog_thrusts_after_its_close() {
    let mut screen = hailable(with_dialogs(boarding_data()));
    fly(&mut screen);
    screen.input(&Input::PadAxis {
        axis: PadAxis::LeftY,
        value: 0.9,
    });
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Comm);
    escape(&mut screen);
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) > closed, "thrust: {:?}", ship(&screen));
}

#[test]
fn dpad_up_held_through_the_spaceport_thrusts_after_the_take_off() {
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.input(&pad(PadButton::DPadUp, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "took off");
    screen.tick(TICK * 3);
    assert_ne!(
        ship(&screen).velocity,
        nova_sim::ShipState::default().velocity
    );
}

#[test]
fn south_pressing_the_comm_dialogs_close_doesnt_fire_after_it() {
    let mut screen = hailed_holding(&[]);
    screen.input(&pad(PadButton::South, true));
    screen.input(&pad(PadButton::South, false));
    assert_eq!(
        screen.showing(),
        Showing::Flight,
        "South's release closed it"
    );
    screen.tick(TICK * 3);
    assert!(!holds(&screen), "nothing held");
}

#[test]
fn south_held_from_flight_through_a_dialog_it_then_closes_doesnt_fire() {
    let mut screen = hailed_holding(&[PadButton::South]);
    screen.input(&pad(PadButton::South, false));
    assert_eq!(screen.showing(), Showing::Comm, "no press of the dialog's");
    screen.input(&pad(PadButton::South, true));
    screen.input(&pad(PadButton::South, false));
    assert_eq!(screen.showing(), Showing::Flight, "closed");
    screen.tick(TICK * 3);
    assert!(!holds(&screen), "nothing held");
}

#[test]
fn south_held_through_the_comm_dialog_closed_by_a_key_fires_after_its_close() {
    // The closing press was the key's: the pad's South comes back.
    let mut screen = hailed_holding(&[]);
    screen.input(&pad(PadButton::South, true));
    escape(&mut screen);
    assert!(holds(&screen), "Fire Primary held");
}

#[test]
fn select_and_south_held_through_the_comm_dialog_dont_fire_after_its_close() {
    // The comm dialog's close is a site (@0x625ee): Select and South come
    // back as Hyper Jump, waiting for South's release. With Select let
    // go, South means nothing: never Fire Primary.
    let mut screen = hailed_holding(&[]);
    screen.input(&pad(PadButton::Select, true));
    screen.input(&pad(PadButton::South, true));
    escape(&mut screen);
    screen.tick(TICK * 3);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.input(&pad(PadButton::Select, false));
    assert!(holds(&screen), "South down, meaning nothing");
    screen.input(&pad(PadButton::South, false));
    assert!(!holds(&screen), "nothing held");
}

#[test]
fn a_focus_loss_under_a_dialog_forgets_the_pads_buttons_and_sticks() {
    let mut screen = hailable(with_dialogs(boarding_data()));
    fly(&mut screen);
    screen.input(&pad(PadButton::DPadUp, true));
    screen.input(&Input::PadAxis {
        axis: PadAxis::LeftY,
        value: 0.9,
    });
    hail(&mut screen);
    screen.cancel_pointer();
    screen.release_keys();
    screen.focus_lost();
    screen.focus_gained();
    escape(&mut screen);
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) <= closed, "forgotten: {:?}", ship(&screen));
}
