use super::super::tests::{key, screen, select, tap};
use super::*;
use crate::draw::DrawCommand;
use crate::input::{ControllerFamily, Key, PadButton, Surroundings};
use crate::keymap::FlightCommand;
use crate::screen::Screen;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn pad(screen: &mut KeysScreen, button: PadButton) {
    for pressed in [true, false] {
        screen.input(&Input::PadButton { button, pressed });
    }
}

fn drawn(screen: &KeysScreen) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    screen.draw(&mut list);
    list.iter().cloned().collect()
}

fn ringed(rect: Bounds, highlight: bool, screen: &KeysScreen) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    if highlight {
        pad_focus::draw_highlight(&mut list, rect, screen.style.up);
    } else {
        outline(&mut list, rect, screen.style.up);
    }
    list.iter().cloned().collect()
}

#[test]
fn the_left_stick_steps_the_list_as_the_arrows_do() {
    let mut keyed = screen();
    tap(&mut keyed, Key::ArrowDown);
    tap(&mut keyed, Key::ArrowDown);
    let mut padded = screen();
    padded.input(&told(PAD));
    pad(&mut padded, PadButton::LeftStickDown);
    pad(&mut padded, PadButton::LeftStickDown);
    assert!(padded.row_list().selected().is_some());
    assert_eq!(padded.row_list().selected(), keyed.row_list().selected());
    pad(&mut padded, PadButton::LeftStickUp);
    tap(&mut keyed, Key::ArrowUp);
    assert_eq!(padded.row_list().selected(), keyed.row_list().selected());
}

#[test]
fn south_on_the_list_records_the_selected_row() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::Accelerate);
    screen.input(&told(PAD));
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    assert_eq!(screen.recording(), Some(FlightCommand::Accelerate.into()));
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert_eq!(
        screen.recording(),
        Some(FlightCommand::Accelerate.into()),
        "its release records nothing"
    );
}

#[test]
fn east_ends_a_recording_and_records_nothing() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::Accelerate);
    screen.input(&told(PAD));
    pad(&mut screen, PadButton::South);
    let before = screen.draft().clone();
    pad(&mut screen, PadButton::East);
    assert_eq!(screen.recording(), None);
    assert_eq!(screen.draft(), &before);
    assert!(!screen.closed(), "only the recording ends");
    pad(&mut screen, PadButton::East);
    assert!(screen.closed(), "then East is Cancel");
    assert_eq!(screen.take_change(), None);
}

#[test]
fn the_pads_other_buttons_record_nothing() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::Accelerate);
    screen.input(&told(PAD));
    pad(&mut screen, PadButton::South);
    let before = screen.draft().clone();
    for button in [
        PadButton::South,
        PadButton::North,
        PadButton::West,
        PadButton::DPadDown,
        PadButton::RightShoulder,
        PadButton::LeftStickDown,
        PadButton::Start,
    ] {
        pad(&mut screen, button);
        assert_eq!(
            screen.recording(),
            Some(FlightCommand::Accelerate.into()),
            "{button:?}"
        );
    }
    assert_eq!(screen.draft(), &before);
    assert_eq!(screen.focus(), KeysFocus::List, "no focus move either");
}

#[test]
fn the_highlight_replaces_the_outline_only_while_the_pad_was_used_last() {
    let mut screen = screen();
    let list = screen.row_list().view();
    assert!(drawn(&screen).ends_with(&ringed(list, false, &screen)));
    screen.input(&told(PAD));
    assert!(drawn(&screen).ends_with(&ringed(list, true, &screen)));
    for device in [Device::Keyboard, Device::Mouse] {
        let mut left = screen.clone();
        left.input(&told(device));
        assert!(
            drawn(&left).ends_with(&ringed(list, false, &left)),
            "{device:?}"
        );
    }
}

#[test]
fn back_on_the_keyboard_the_outline_draws_where_the_pad_left_the_focus() {
    let mut screen = screen();
    screen.input(&told(PAD));
    pad(&mut screen, PadButton::DPadDown);
    let focus = screen.focus();
    assert_ne!(focus, KeysFocus::List);
    screen.input(&told(Device::Keyboard));
    assert_eq!(screen.focus(), focus);
    let rect = screen.focus_rect(focus);
    assert!(drawn(&screen).ends_with(&ringed(rect, false, &screen)));
    tap(&mut screen, Key::Tab);
    assert_eq!(screen.focus(), focus.next(), "Tab moves on from it");
}

#[test]
fn south_on_a_button_presses_it_on_its_release() {
    let mut screen = screen();
    screen.input(&told(PAD));
    while screen.focus() != KeysFocus::Button(KeysButton::Cancel) {
        pad(&mut screen, PadButton::RightShoulder);
    }
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    assert!(screen.trackers[KeysButton::Cancel.index()].pressed());
    assert!(!screen.closed());
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert!(screen.closed());
}

#[test]
fn keyboard_space_on_the_list_still_records() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::Accelerate);
    screen.input(&told(PAD));
    screen.input(&key(Key::Space, true));
    assert_eq!(screen.recording(), Some(FlightCommand::Accelerate.into()));
}

#[test]
fn it_reports_the_device_it_was_told() {
    let mut screen = screen();
    assert_eq!(screen.device(), Device::Keyboard);
    screen.input(&told(PAD));
    assert_eq!(screen.device(), PAD);
    screen.input(&told(Device::Mouse));
    assert_eq!(screen.device(), Device::Mouse);
}

#[test]
fn a_dpad_press_while_south_is_held_on_a_button_drops_the_press() {
    let mut screen = screen();
    screen.input(&told(PAD));
    while screen.focus() != KeysFocus::Button(KeysButton::Cancel) {
        pad(&mut screen, PadButton::RightShoulder);
    }
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    pad(&mut screen, PadButton::DPadRight);
    assert!(!screen.trackers[KeysButton::Cancel.index()].pressed());
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert!(!screen.closed(), "Cancel's press was dropped");
}
