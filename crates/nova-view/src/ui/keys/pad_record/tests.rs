use std::time::Duration;

use super::super::tests::{key, screen, select, tap};
use super::super::*;
use crate::input::{ControllerFamily, PadAxis, PadButton as P, Surroundings};
use crate::keymap::{
    AxisBinding, FlightCommand as Fc, PadBinding, PadEntry, PadMap, PauseCommand, PortCommand,
    UiCommand,
};
use crate::screen::Screen;

const XBOX: Device = Device::Gamepad(ControllerFamily::Xbox);

fn told(screen: &mut KeysScreen, device: Device) {
    screen.input(&Input::Surroundings(Surroundings { device }));
}

fn down(screen: &mut KeysScreen, button: P) {
    screen.input(&Input::PadButton {
        button,
        pressed: true,
    });
}

fn up(screen: &mut KeysScreen, button: P) {
    screen.input(&Input::PadButton {
        button,
        pressed: false,
    });
}

fn pad(screen: &mut KeysScreen, button: P) {
    down(screen, button);
    up(screen, button);
}

/// The screen on its gamepad tab, told an Xbox pad was used last.
fn gamepad_tab() -> KeysScreen {
    let mut screen = screen();
    told(&mut screen, XBOX);
    tap(&mut screen, Key::ArrowRight);
    assert_eq!(screen.tab(), Column::Gamepad);
    screen
}

/// `screen`, recording `command` on its shown tab, started with Return.
fn recording(screen: &mut KeysScreen, command: impl Into<Command>) {
    let command = command.into();
    select(screen, command);
    tap(screen, Key::Enter);
    assert_eq!(screen.recording(), Some(command));
}

fn cell(screen: &KeysScreen, command: impl Into<Command>) -> String {
    let row = screen.row_of(command.into()).expect("a row");
    screen.row_list().cell(row).unwrap_or_default().to_owned()
}

fn on_pad(screen: &KeysScreen, command: impl Into<Command>) -> PadBinding {
    screen.draft().gamepad.binding(command.into())
}

fn plain(button: P) -> PadBinding {
    PadBinding::Entry(PadEntry::plain(button))
}

fn marked_on_tab(screen: &KeysScreen, command: impl Into<Command>) -> bool {
    let row = screen.row_of(command.into()).expect("a row");
    screen.row_list().is_marked(row)
}

#[test]
fn the_gamepad_tab_lists_every_command_but_the_ui_rows_with_the_axes_and_their_invert_rows() {
    let rows = rows(Column::Gamepad);
    let commands: Vec<Command> = rows
        .iter()
        .filter_map(|row| match *row {
            Row::Command(command) => Some(command),
            _ => None,
        })
        .collect();
    let expected: Vec<Command> = Command::all()
        .filter(|command| command.context() != Context::Ui)
        .collect();
    assert_eq!(commands, expected);
    assert!(!rows.contains(&Row::Heading(Context::Ui)));
    for axis in [Fc::TurnAxis, Fc::ThrottleAxis] {
        let at = rows
            .iter()
            .position(|&row| row == Row::Command(axis.into()))
            .expect("listed");
        assert_eq!(rows[at + 1], Row::Invert(axis.into()), "{axis:?}");
    }
    assert_eq!(
        rows.iter()
            .filter(|row| matches!(row, Row::Invert(_)))
            .count(),
        2
    );
    let keyboard = super::super::rows(Column::Keyboard);
    assert!(keyboard.contains(&Row::Command(UiCommand::Back.into())));
    assert!(!keyboard.contains(&Row::Command(Fc::TurnAxis.into())));
    let screen = gamepad_tab();
    assert_eq!(screen.row_kinds(), rows.as_slice());
    let invert = screen
        .row_kinds()
        .iter()
        .position(|&row| row == Row::Invert(Fc::TurnAxis.into()))
        .expect("an invert row");
    assert_eq!(screen.row_list().rows()[invert], "    Invert");
    assert_eq!(screen.row_list().cell(invert), Some("Off"));
}

#[test]
fn left_and_right_and_the_shoulders_and_a_click_switch_tabs_keeping_the_selected_command() {
    let mut screen = screen();
    select(&mut screen, Fc::Accelerate);
    tap(&mut screen, Key::ArrowRight);
    assert_eq!(screen.tab(), Column::Gamepad);
    assert_eq!(
        screen.row_list().selected(),
        screen.row_of(Fc::Accelerate.into())
    );
    tap(&mut screen, Key::ArrowLeft);
    assert_eq!(screen.tab(), Column::Keyboard);
    assert_eq!(
        screen.row_list().selected(),
        screen.row_of(Fc::Accelerate.into())
    );
    told(&mut screen, XBOX);
    pad(&mut screen, P::RightShoulder);
    assert_eq!(screen.tab(), Column::Gamepad);
    assert_eq!(screen.focus(), KeysFocus::List, "no focus step");
    pad(&mut screen, P::LeftShoulder);
    assert_eq!(screen.tab(), Column::Keyboard);
    let gamepad = screen.tab_bounds(Column::Gamepad);
    let at = Point::new(
        f32::midpoint(gamepad.min.x, gamepad.max.x),
        f32::midpoint(gamepad.min.y, gamepad.max.y),
    );
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    assert_eq!(screen.tab(), Column::Gamepad);
    select(&mut screen, Fc::Accelerate);
    tap(&mut screen, Key::ArrowLeft);
    select(&mut screen, UiCommand::Back);
    tap(&mut screen, Key::ArrowRight);
    assert_eq!(screen.tab(), Column::Gamepad, "Back isn't listed here");
}

#[test]
fn the_tab_labels_sit_on_the_title_row_the_shown_one_white() {
    let screen = gamepad_tab();
    let mut list = DrawList::new();
    screen.draw(&mut list);
    let label = |name: &str| {
        list.iter().find_map(|command| match command {
            crate::draw::DrawCommand::Text {
                text,
                origin,
                color,
                ..
            } if text == name => Some((*origin, *color)),
            _ => None,
        })
    };
    let (keyboard_at, keyboard_color) = label("Keyboard").expect("drawn");
    let (gamepad_at, gamepad_color) = label("Gamepad").expect("drawn");
    assert_eq!(gamepad_color, crate::color::Color::WHITE);
    assert_eq!(keyboard_color, TAB_DIM);
    assert_eq!(keyboard_at, screen.tab_bounds(Column::Keyboard).min);
    assert_eq!(gamepad_at, screen.tab_bounds(Column::Gamepad).min);
    assert!(keyboard_at.x < gamepad_at.x);
}

#[test]
fn recording_on_the_gamepad_tab_binds_the_pressed_button() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    assert_eq!(cell(&screen, Fc::Autopilot), PAD_RECORDING);
    pad(&mut screen, P::West);
    assert_eq!(screen.recording(), None);
    assert_eq!(on_pad(&screen, Fc::Autopilot), plain(P::West));
    assert_eq!(cell(&screen, Fc::Autopilot), "X");
    assert!(
        screen
            .draft()
            .keyboard
            .combo(Fc::Autopilot.into())
            .is_some(),
        "the key kept"
    );
}

#[test]
fn a_stick_inside_the_threshold_binds_nothing() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    let before = screen.draft().clone();
    screen.input(&Input::PadAxis {
        axis: PadAxis::LeftX,
        value: 0.3,
    });
    assert_eq!(screen.recording(), Some(Fc::Autopilot.into()));
    assert_eq!(screen.draft(), &before);
}

#[test]
fn a_stick_past_the_threshold_records_its_direction() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::ZoomIn);
    screen.input(&Input::PadAxis {
        axis: PadAxis::RightY,
        value: 0.9,
    });
    assert_eq!(
        screen.recording(),
        Some(Fc::ZoomIn.into()),
        "the axis alone"
    );
    pad(&mut screen, P::RightStickUp);
    assert_eq!(on_pad(&screen, Fc::ZoomIn), plain(P::RightStickUp));
}

#[test]
fn south_and_east_record_while_recording() {
    let mut screen = gamepad_tab();
    select(&mut screen, Fc::Autopilot);
    down(&mut screen, P::South);
    assert_eq!(
        screen.recording(),
        Some(Fc::Autopilot.into()),
        "South starts"
    );
    up(&mut screen, P::South);
    assert_eq!(
        screen.recording(),
        Some(Fc::Autopilot.into()),
        "its release records nothing"
    );
    pad(&mut screen, P::East);
    assert_eq!(on_pad(&screen, Fc::Autopilot), plain(P::East));
    assert_eq!(on_pad(&screen, Fc::LandDock), PadBinding::Unbound);
    assert!(!screen.closed());
    recording(&mut screen, Fc::Board);
    pad(&mut screen, P::South);
    assert_eq!(on_pad(&screen, Fc::Board), plain(P::South));
}

#[test]
fn a_press_with_select_held_binds_on_select_s_layer() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    down(&mut screen, P::Select);
    assert_eq!(screen.recording(), Some(Fc::Autopilot.into()));
    pad(&mut screen, P::North);
    up(&mut screen, P::Select);
    assert_eq!(
        on_pad(&screen, Fc::Autopilot),
        PadBinding::Entry(PadEntry::on(P::Select, P::North))
    );
    assert_eq!(cell(&screen, Fc::Autopilot), "View+Y");
    assert_eq!(on_pad(&screen, Fc::FaceStellar), PadBinding::Unbound);
}

#[test]
fn a_button_still_held_that_is_no_layer_records_no_layer() {
    let mut screen = gamepad_tab();
    select(&mut screen, Fc::Autopilot);
    down(&mut screen, P::South);
    assert_eq!(screen.recording(), Some(Fc::Autopilot.into()));
    pad(&mut screen, P::LeftStick);
    up(&mut screen, P::South);
    assert_eq!(
        on_pad(&screen, Fc::Autopilot),
        plain(P::LeftStick),
        "the South that started it, still down, is no layer"
    );
}

#[test]
fn select_alone_records_nothing_and_keeps_waiting() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    let before = screen.draft().clone();
    pad(&mut screen, P::Select);
    pad(&mut screen, P::Start);
    assert_eq!(screen.recording(), Some(Fc::Autopilot.into()));
    assert_eq!(screen.draft(), &before);
    pad(&mut screen, P::LeftStick);
    assert_eq!(
        on_pad(&screen, Fc::Autopilot),
        plain(P::LeftStick),
        "the layer let go"
    );
}

#[test]
fn select_then_start_records_the_chord() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    down(&mut screen, P::Select);
    down(&mut screen, P::Start);
    assert_eq!(
        on_pad(&screen, Fc::Autopilot),
        PadBinding::Entry(PadEntry::on(P::Select, P::Start))
    );
    assert_eq!(on_pad(&screen, PauseCommand::Pause), PadBinding::Unbound);
}

#[test]
fn a_flight_command_rebound_to_south_or_east_leaves_the_pad_navigating_and_closing() {
    for button in [P::South, P::East] {
        let mut screen = gamepad_tab();
        select(&mut screen, Fc::Autopilot);
        pad(&mut screen, P::South);
        pad(&mut screen, button);
        assert_eq!(on_pad(&screen, Fc::Autopilot), plain(button), "{button:?}");
        super::super::pad::tests::focus_on(&mut screen, KeysButton::Ok);
        pad(&mut screen, P::South);
        assert!(screen.closed(), "{button:?}: South presses OK");
        let change = screen.take_change().expect("a change");
        assert_eq!(change.gamepad.binding(Fc::Autopilot.into()), plain(button));
        let mut cancelled = gamepad_tab();
        select(&mut cancelled, Fc::Autopilot);
        pad(&mut cancelled, P::South);
        pad(&mut cancelled, button);
        pad(&mut cancelled, P::East);
        assert!(cancelled.closed(), "{button:?}: East is Cancel");
        assert_eq!(cancelled.take_change(), None);
    }
}

#[test]
fn recording_a_spaceport_command_onto_south_or_a_dpad_direction_is_refused_with_a_message() {
    let mut screen = gamepad_tab();
    recording(&mut screen, PortCommand::Bar);
    let before = screen.draft().clone();
    pad(&mut screen, P::South);
    assert_eq!(cell(&screen, PortCommand::Bar), "Held by Activate");
    assert_eq!(screen.recording(), Some(PortCommand::Bar.into()));
    pad(&mut screen, P::DPadDown);
    assert_eq!(cell(&screen, PortCommand::Bar), "Held by Focus Down");
    assert_eq!(screen.recording(), Some(PortCommand::Bar.into()));
    assert_eq!(screen.draft(), &before, "both keep their bindings");
    pad(&mut screen, P::North);
    assert_eq!(on_pad(&screen, PortCommand::Bar), plain(P::North));
    assert_eq!(screen.recording(), None);
}

#[test]
fn recording_a_button_command_onto_a_stick_flight_reads_is_refused_naming_the_axis() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::FirePrimary);
    pad(&mut screen, P::RightStickLeft);
    assert_eq!(
        cell(&screen, Fc::FirePrimary),
        format!("Held by {}", Command::from(Fc::TurnAxis).label())
    );
    assert_eq!(screen.recording(), Some(Fc::FirePrimary.into()));
}

#[test]
fn binding_a_used_button_unbinds_and_marks_its_holder() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    pad(&mut screen, P::West);
    assert_eq!(on_pad(&screen, Fc::ClosestTarget), PadBinding::Unbound);
    assert!(marked_on_tab(&screen, Fc::ClosestTarget));
    assert_eq!(cell(&screen, Fc::ClosestTarget), PAD_UNBOUND);
    tap(&mut screen, Key::ArrowLeft);
    assert!(
        !marked_on_tab(&screen, Fc::ClosestTarget),
        "the keyboard's key is untouched"
    );
}

#[test]
fn restore_defaults_on_the_gamepad_tab_resets_only_the_pad_table_axes_included() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    pad(&mut screen, P::West);
    recording(&mut screen, Fc::TurnAxis);
    pad(&mut screen, P::LeftStickLeft);
    tap(&mut screen, Key::ArrowLeft);
    recording(&mut screen, Fc::Board);
    tap(&mut screen, Key::KeyO);
    tap(&mut screen, Key::ArrowRight);
    screen.press_button(KeysButton::RestoreDefaults);
    assert_eq!(screen.draft().gamepad, PadMap::default());
    assert_eq!(
        screen.draft().keyboard.combo(Fc::Board.into()),
        Some(crate::keymap::KeyCombo::key(Key::KeyO))
    );
    assert!(!marked_on_tab(&screen, Fc::ClosestTarget));
}

#[test]
fn restore_defaults_on_the_keyboard_tab_resets_only_the_keys() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    pad(&mut screen, P::West);
    tap(&mut screen, Key::ArrowLeft);
    recording(&mut screen, Fc::Board);
    tap(&mut screen, Key::KeyO);
    screen.press_button(KeysButton::RestoreDefaults);
    assert_eq!(screen.draft().keyboard, crate::keymap::KeyMap::default());
    assert_eq!(on_pad(&screen, Fc::Autopilot), plain(P::West));
    tap(&mut screen, Key::ArrowRight);
    assert!(
        marked_on_tab(&screen, Fc::ClosestTarget),
        "the pad's marks kept"
    );
}

#[test]
fn recording_the_axes_swaps_the_sticks() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::TurnAxis);
    assert_eq!(cell(&screen, Fc::TurnAxis), AXIS_RECORDING);
    pad(&mut screen, P::South);
    assert_eq!(screen.recording(), Some(Fc::TurnAxis.into()), "no stick");
    pad(&mut screen, P::LeftStickLeft);
    recording(&mut screen, Fc::ThrottleAxis);
    pad(&mut screen, P::RightStickUp);
    let pad_map = &screen.draft().gamepad;
    assert_eq!(
        pad_map.axis_binding(Fc::TurnAxis.into()),
        Some(AxisBinding::plain(PadAxis::LeftX))
    );
    assert_eq!(
        pad_map.axis_binding(Fc::ThrottleAxis.into()),
        Some(AxisBinding::plain(PadAxis::RightY))
    );
    assert_eq!(cell(&screen, Fc::TurnAxis), "L Stick X");
}

#[test]
fn the_invert_row_toggles_inversion() {
    let mut screen = gamepad_tab();
    let invert = screen
        .row_kinds()
        .iter()
        .position(|&row| row == Row::Invert(Fc::ThrottleAxis.into()))
        .expect("an invert row");
    while screen.row_list().selected() != Some(invert) {
        tap(&mut screen, Key::ArrowDown);
    }
    let inverted = |screen: &KeysScreen| {
        screen
            .draft()
            .gamepad
            .axis_binding(Fc::ThrottleAxis.into())
            .map(|binding| binding.inverted)
    };
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.recording(), None, "no recording");
    assert_eq!(inverted(&screen), Some(true));
    assert_eq!(screen.row_list().cell(invert), Some("On"));
    pad(&mut screen, P::South);
    assert_eq!(inverted(&screen), Some(false));
    assert_eq!(screen.row_list().cell(invert), Some("Off"));
    tap(&mut screen, Key::Enter);
    recording(&mut screen, Fc::ThrottleAxis);
    pad(&mut screen, P::RightStickDown);
    assert_eq!(
        screen.draft().gamepad.axis_binding(Fc::ThrottleAxis.into()),
        Some(AxisBinding::inverted(PadAxis::RightY)),
        "a recorded axis keeps the row's inversion"
    );
}

#[test]
fn recording_cancels_after_five_seconds_with_no_press() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    screen.tick(Duration::from_millis(4900));
    assert_eq!(screen.recording(), Some(Fc::Autopilot.into()));
    down(&mut screen, P::Select);
    screen.tick(Duration::from_millis(100));
    up(&mut screen, P::Select);
    screen.tick(Duration::from_millis(4800));
    assert_eq!(
        screen.recording(),
        Some(Fc::Autopilot.into()),
        "a press restarts it"
    );
    let before = screen.draft().clone();
    screen.tick(Duration::from_millis(100));
    assert_eq!(screen.recording(), None);
    assert_eq!(screen.draft(), &before);
    assert_eq!(PAD_RECORD_TIMEOUT, Duration::from_secs(5));
    let mut keyboard = screen.clone();
    tap(&mut keyboard, Key::ArrowLeft);
    recording(&mut keyboard, Fc::Autopilot);
    keyboard.tick(Duration::from_secs(60));
    assert_eq!(
        keyboard.recording(),
        Some(Fc::Autopilot.into()),
        "the keyboard tab's recorder never times out"
    );
}

#[test]
fn west_clears_the_selected_pad_binding_and_delete_too() {
    let mut screen = gamepad_tab();
    select(&mut screen, Fc::Autopilot);
    pad(&mut screen, P::West);
    assert_eq!(screen.recording(), None);
    assert_eq!(on_pad(&screen, Fc::Autopilot), PadBinding::Unbound);
    assert_eq!(cell(&screen, Fc::Autopilot), PAD_UNBOUND);
    assert!(
        !marked_on_tab(&screen, Fc::Autopilot),
        "cleared, not marked"
    );
    select(&mut screen, Fc::Board);
    tap(&mut screen, Key::Delete);
    assert_eq!(on_pad(&screen, Fc::Board), PadBinding::Unbound);
    assert!(screen.draft().keyboard.combo(Fc::Board.into()).is_some());
    tap(&mut screen, Key::ArrowLeft);
    select(&mut screen, Fc::Communicate);
    pad(&mut screen, P::West);
    assert_eq!(screen.draft().keyboard.combo(Fc::Communicate.into()), None);
    assert_ne!(on_pad(&screen, Fc::Communicate), PadBinding::Unbound);
}

#[test]
fn a_key_press_ends_a_pad_recording() {
    let mut screen = gamepad_tab();
    recording(&mut screen, Fc::Autopilot);
    let before = screen.draft().clone();
    screen.input(&key(Key::KeyA, false));
    assert_eq!(screen.recording(), Some(Fc::Autopilot.into()), "a release");
    tap(&mut screen, Key::KeyA);
    assert_eq!(screen.recording(), None);
    assert_eq!(screen.draft(), &before);
    assert_ne!(cell(&screen, Fc::Autopilot), PAD_RECORDING);
}

#[test]
fn the_gamepad_cells_name_the_family_last_told() {
    let mut screen = screen();
    tap(&mut screen, Key::ArrowRight);
    assert_eq!(
        cell(&screen, Fc::FirePrimary),
        "South",
        "generic before any"
    );
    told(&mut screen, XBOX);
    assert_eq!(cell(&screen, Fc::FirePrimary), "A");
    told(&mut screen, Device::Keyboard);
    assert_eq!(cell(&screen, Fc::FirePrimary), "A", "the pad's names kept");
    told(&mut screen, Device::Gamepad(ControllerFamily::PlayStation));
    assert_eq!(cell(&screen, Fc::FirePrimary), "Cross");
    assert_eq!(cell(&screen, Fc::ShowFramerate), "Share+L3");
    assert_eq!(cell(&screen, PortCommand::Bar), PAD_UNBOUND);
    tap(&mut screen, Key::ArrowLeft);
    assert_eq!(
        cell(&screen, Fc::FirePrimary),
        "Space",
        "the keyboard tab names keys"
    );
}
