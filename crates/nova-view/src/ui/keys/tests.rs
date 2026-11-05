use std::rc::Rc;
use std::time::Duration;

use nova_sim::{RuleKey, RuleSource, Rulebook};

use super::*;
use crate::draw::DrawCommand;
use crate::key_names::NoKeyNames;
use crate::key_names::fixture::FakeKeyNames;
use crate::keymap::{FlightCommand, Modifiers, PauseCommand, PlayCommand, PortCommand, UiCommand};
use crate::text::fixture::MonoMetrics;

/// The Preferences dialog's bounds, centred in 640 x 480.
fn bounds() -> Bounds {
    Bounds::at(Point::new(152.0, 101.0), 336.0, 278.0)
}

fn cmd_q() -> KeyCombo {
    KeyCombo::with(Key::KeyQ, Modifiers::COMMAND)
}

fn screen_with(bindings: Bindings, names: Rc<dyn KeyNames>, rules: &Rulebook) -> KeysScreen {
    KeysScreen::new(
        bounds(),
        bindings,
        names,
        vec![cmd_q()],
        rules,
        ButtonStyle::STOCK,
        Rc::new(MonoMetrics),
    )
}

/// The screen on the default bindings, by the engine's reading.
fn screen() -> KeysScreen {
    screen_with(
        Bindings::default(),
        Rc::new(NoKeyNames),
        &Rulebook::default(),
    )
}

/// The screen recording the held modifiers.
fn held_screen() -> KeysScreen {
    let rules = Rulebook::default().with_override(RuleKey::RecordedModifiers, RuleSource::Bible);
    screen_with(Bindings::default(), Rc::new(NoKeyNames), &rules)
}

fn key(key: Key, pressed: bool) -> Input {
    Input::Key {
        key,
        pressed,
        repeat: false,
    }
}

/// `k` pressed and let go.
fn tap(screen: &mut KeysScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// `k` pressed and let go with `modifier` held.
fn chord(screen: &mut KeysScreen, modifier: Key, k: Key) {
    screen.input(&key(modifier, true));
    tap(screen, k);
    screen.input(&key(modifier, false));
}

fn click(screen: &mut KeysScreen, at: Point) {
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// Selects `command`'s row with Down, from wherever the selection is.
fn select(screen: &mut KeysScreen, command: impl Into<Command>) {
    let row = screen.row_of(command.into()).expect("a row");
    for _ in 0..screen.row_kinds().len() {
        if screen.row_list().selected() == Some(row) {
            return;
        }
        tap(screen, Key::ArrowDown);
    }
    panic!("never reached row {row}");
}

/// Records `k` on `command`, starting with Return.
fn record(screen: &mut KeysScreen, command: impl Into<Command>, k: Key) {
    let command = command.into();
    select(screen, command);
    tap(screen, Key::Enter);
    assert_eq!(screen.recording(), Some(command));
    tap(screen, k);
}

fn combo(screen: &KeysScreen, command: impl Into<Command>) -> Option<KeyCombo> {
    screen.draft().keyboard.combo(command.into())
}

fn cell(screen: &KeysScreen, command: impl Into<Command>) -> String {
    let row = screen.row_of(command.into()).expect("a row");
    screen.row_list().cell(row).expect("a cell").to_owned()
}

fn marked(screen: &KeysScreen, command: impl Into<Command>) -> bool {
    let command = command.into();
    let row = screen.row_of(command).expect("a row");
    screen.is_marked(command) && screen.row_list().is_marked(row)
}

fn center(bounds: Bounds) -> Point {
    bounds.center()
}

#[test]
fn every_command_is_listed_under_its_context_s_heading_with_its_key() {
    let screen = screen();
    let kinds = screen.row_kinds();
    let commands: Vec<Command> = kinds
        .iter()
        .filter_map(|row| match row {
            Row::Command(command) => Some(*command),
            Row::Heading(_) => None,
        })
        .collect();
    let keyed: Vec<Command> = Command::all()
        .filter(|command| command.kind() != crate::keymap::Kind::Axis)
        .collect();
    assert_eq!(commands, keyed, "every command a key can take");
    for axis in [FlightCommand::TurnAxis, FlightCommand::ThrottleAxis] {
        assert!(!commands.contains(&axis.into()), "{axis:?} takes no key");
    }
    let mut heading = None;
    for (index, row) in kinds.iter().enumerate() {
        match *row {
            Row::Heading(context) => {
                heading = Some(context);
                assert_eq!(screen.row_list().rows()[index], context.label());
                assert!(screen.row_list().is_heading(index));
            }
            Row::Command(command) => {
                assert_eq!(Some(command.context()), heading, "{command:?}");
                assert_eq!(
                    screen.row_list().rows()[index],
                    format!("  {}", command.label())
                );
            }
        }
    }
    assert_eq!(kinds[0], Row::Heading(Context::Flight));
    assert!(
        !kinds.contains(&Row::Heading(Context::SystemView)),
        "no commands"
    );
    assert_eq!(cell(&screen, FlightCommand::Accelerate), "Up Arrow");
    assert_eq!(cell(&screen, FlightCommand::JettisonCargo), "Cmd-K");
    assert_eq!(cell(&screen, FlightCommand::TargetSelectBack), "Shift-Tab");
    assert_eq!(screen.row_list().selected(), Some(1), "Accelerate");
}

#[test]
fn an_unbound_command_reads_as_an_empty_recorder() {
    let bindings = Bindings::default().unbound(FlightCommand::Board);
    let screen = screen_with(bindings, Rc::new(NoKeyNames), &Rulebook::default());
    assert_eq!(cell(&screen, FlightCommand::Board), UNBOUND);
}

#[test]
fn the_row_bound_to_the_w_position_reads_z_on_azerty() {
    let screen = screen_with(
        Bindings::default(),
        Rc::new(FakeKeyNames::azerty()),
        &Rulebook::default(),
    );
    assert_eq!(cell(&screen, FlightCommand::SelectSecondary), "Z");
    assert_eq!(cell(&screen, FlightCommand::SelectSecondaryBack), "Alt-Z");
}

#[test]
fn return_starts_recording_and_the_row_says_so() {
    let mut screen = screen();
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.recording(), Some(FlightCommand::Accelerate.into()));
    assert_eq!(cell(&screen, FlightCommand::Accelerate), RECORDING);
    tap(&mut screen, Key::KeyT);
    assert_eq!(screen.recording(), None);
    assert_eq!(
        combo(&screen, FlightCommand::Accelerate),
        Some(KeyCombo::key(Key::KeyT))
    );
    assert_eq!(cell(&screen, FlightCommand::Accelerate), "T");
}

#[test]
fn space_and_the_keypad_s_enter_start_recording_too() {
    for start in [Key::Space, Key::NumpadEnter] {
        let mut screen = screen();
        tap(&mut screen, start);
        assert_eq!(
            screen.recording(),
            Some(FlightCommand::Accelerate.into()),
            "{start}"
        );
    }
}

#[test]
fn by_default_a_modifier_records_on_its_press_as_its_left_key() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::FireSecondary);
    tap(&mut screen, Key::Enter);
    screen.input(&key(Key::AltRight, true));
    assert_eq!(screen.recording(), None, "on the press");
    assert_eq!(
        combo(&screen, FlightCommand::FireSecondary),
        Some(KeyCombo::key(Key::AltLeft))
    );
}

#[test]
fn by_default_a_key_drops_the_modifiers_held_and_takes_its_command_s() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::JettisonCargo);
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::KeyJ);
    assert_eq!(
        combo(&screen, FlightCommand::JettisonCargo),
        Some(KeyCombo::with(Key::KeyJ, Modifiers::COMMAND))
    );
    assert_eq!(cell(&screen, FlightCommand::JettisonCargo), "Cmd-J");
}

#[test]
fn by_the_other_reading_a_key_binds_with_the_modifiers_held() {
    let mut screen = held_screen();
    select(&mut screen, FlightCommand::Board);
    tap(&mut screen, Key::Enter);
    screen.input(&key(Key::ShiftRight, true));
    screen.input(&key(Key::ControlLeft, true));
    assert_eq!(screen.recording(), Some(FlightCommand::Board.into()));
    tap(&mut screen, Key::KeyJ);
    assert_eq!(
        combo(&screen, FlightCommand::Board),
        Some(KeyCombo::with(
            Key::KeyJ,
            Modifiers::SHIFT.union(Modifiers::CONTROL)
        ))
    );
    assert_eq!(screen.recording(), None);
}

#[test]
fn by_the_other_reading_a_modifier_let_go_alone_records_itself() {
    let mut screen = held_screen();
    select(&mut screen, FlightCommand::Board);
    tap(&mut screen, Key::Enter);
    screen.input(&key(Key::ControlRight, true));
    assert_eq!(screen.recording(), Some(FlightCommand::Board.into()));
    screen.input(&key(Key::ControlRight, false));
    assert_eq!(
        combo(&screen, FlightCommand::Board),
        Some(KeyCombo::key(Key::ControlRight))
    );
}

#[test]
fn repeats_and_fn_record_nothing() {
    let mut screen = screen();
    tap(&mut screen, Key::Enter);
    screen.input(&Input::Key {
        key: Key::KeyT,
        pressed: true,
        repeat: true,
    });
    tap(&mut screen, Key::Fn);
    assert_eq!(screen.recording(), Some(FlightCommand::Accelerate.into()));
    assert_eq!(
        combo(&screen, FlightCommand::Accelerate),
        Some(KeyCombo::key(Key::ArrowUp))
    );
}

#[test]
fn recording_closest_target_on_w_unbinds_and_marks_select_secondary_and_its_variant() {
    let mut screen = screen();
    record(&mut screen, FlightCommand::ClosestTarget, Key::KeyW);
    assert_eq!(combo(&screen, FlightCommand::SelectSecondary), None);
    assert_eq!(combo(&screen, FlightCommand::SelectSecondaryBack), None);
    assert!(marked(&screen, FlightCommand::SelectSecondary));
    assert!(marked(&screen, FlightCommand::SelectSecondaryBack));
    assert_eq!(cell(&screen, FlightCommand::SelectSecondary), UNBOUND);
    assert!(!marked(&screen, FlightCommand::ClosestTarget));
    assert_eq!(cell(&screen, FlightCommand::ClosestShip), "Alt-W");
}

#[test]
fn a_mark_stays_until_its_command_is_bound_again() {
    let mut screen = screen();
    record(&mut screen, FlightCommand::ClosestTarget, Key::KeyW);
    record(&mut screen, FlightCommand::SelectSecondary, Key::KeyQ);
    assert!(!marked(&screen, FlightCommand::SelectSecondary));
    assert!(
        !marked(&screen, FlightCommand::SelectSecondaryBack),
        "it follows its base onto Alt-Q"
    );
}

#[test]
fn a_flight_key_recorded_on_a_spaceport_command_leaves_both() {
    let mut screen = screen();
    record(&mut screen, PortCommand::Bar, Key::KeyL);
    assert_eq!(
        combo(&screen, PortCommand::Bar),
        Some(KeyCombo::key(Key::KeyL))
    );
    assert_eq!(
        combo(&screen, FlightCommand::LandDock),
        Some(KeyCombo::key(Key::KeyL))
    );
    assert!(!marked(&screen, FlightCommand::LandDock));
}

#[test]
fn escape_recorded_on_a_spaceport_command_unbinds_back() {
    let mut screen = screen();
    record(&mut screen, PortCommand::Bar, Key::Escape);
    assert!(!screen.closed(), "Escape records while recording");
    assert_eq!(
        combo(&screen, PortCommand::Bar),
        Some(KeyCombo::key(Key::Escape))
    );
    assert_eq!(combo(&screen, UiCommand::Back), None);
    assert!(marked(&screen, UiCommand::Back));
    assert_eq!(
        combo(&screen, PauseCommand::Pause),
        Some(KeyCombo::key(Key::Escape)),
        "never active with the spaceport"
    );
}

#[test]
fn backspace_and_delete_record_while_recording() {
    for k in [Key::Backspace, Key::Delete] {
        let mut screen = screen();
        record(&mut screen, FlightCommand::Board, k);
        assert_eq!(
            combo(&screen, FlightCommand::Board),
            Some(KeyCombo::key(k)),
            "{k}"
        );
    }
}

#[test]
fn command_q_binds_nothing_and_recording_goes_on() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::Board);
    tap(&mut screen, Key::Enter);
    screen.input(&key(Key::MetaLeft, true));
    // By default Command's press records it: the original's.
    assert_eq!(
        combo(&screen, FlightCommand::Board),
        Some(KeyCombo::key(Key::MetaLeft))
    );
    let mut screen = held_screen();
    select(&mut screen, FlightCommand::Board);
    tap(&mut screen, Key::Enter);
    chord(&mut screen, Key::MetaLeft, Key::KeyQ);
    assert_eq!(
        combo(&screen, FlightCommand::Board),
        Some(KeyCombo::key(Key::KeyB))
    );
    assert_eq!(screen.recording(), Some(FlightCommand::Board.into()));
}

#[test]
fn command_q_held_from_before_recording_binds_nothing_by_default_either() {
    let mut screen = screen();
    select(&mut screen, FlightCommand::Board);
    screen.input(&key(Key::MetaLeft, true));
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::KeyQ);
    assert_eq!(
        combo(&screen, FlightCommand::Board),
        Some(KeyCombo::key(Key::KeyB))
    );
    assert_eq!(screen.recording(), Some(FlightCommand::Board.into()));
}

#[test]
fn q_on_jettison_cargo_binds_nothing_by_default_as_it_would_be_command_q() {
    let mut screen = screen();
    record(&mut screen, FlightCommand::JettisonCargo, Key::KeyQ);
    assert_eq!(
        combo(&screen, FlightCommand::JettisonCargo),
        Some(KeyCombo::with(Key::KeyK, Modifiers::COMMAND))
    );
    assert_eq!(
        screen.recording(),
        Some(FlightCommand::JettisonCargo.into())
    );
}

#[test]
fn delete_clears_the_selected_command_and_restore_defaults_restores_every_one() {
    let mut screen = screen();
    record(&mut screen, PortCommand::Bar, Key::Escape);
    select(&mut screen, PlayCommand::GalaxyMap);
    tap(&mut screen, Key::Delete);
    assert_eq!(combo(&screen, PlayCommand::GalaxyMap), None);
    assert_eq!(cell(&screen, PlayCommand::GalaxyMap), UNBOUND);
    select(&mut screen, PlayCommand::PlayerInfo);
    tap(&mut screen, Key::Backspace);
    assert_eq!(combo(&screen, PlayCommand::PlayerInfo), None);
    let restore = screen.button_bounds(KeysButton::RestoreDefaults);
    click(&mut screen, center(restore));
    assert_eq!(screen.draft(), &Bindings::default());
    assert!(!marked(&screen, UiCommand::Back));
    assert_eq!(cell(&screen, PlayCommand::GalaxyMap), "M");
}

#[test]
fn ok_hands_out_the_edit_once_and_only_a_change() {
    let mut edited = screen();
    record(&mut edited, FlightCommand::Accelerate, Key::KeyT);
    let at = center(edited.button_bounds(KeysButton::Ok));
    click(&mut edited, at);
    assert!(edited.closed());
    let change = edited.take_change().expect("the edit");
    assert_eq!(
        change.keyboard.combo(FlightCommand::Accelerate.into()),
        Some(KeyCombo::key(Key::KeyT))
    );
    assert_eq!(edited.take_change(), None, "once");
    let mut untouched = screen();
    let at = center(untouched.button_bounds(KeysButton::Ok));
    click(&mut untouched, at);
    assert!(untouched.closed());
    assert_eq!(untouched.take_change(), None, "no change");
}

#[test]
fn cancel_and_escape_close_with_nothing_to_take() {
    let mut clicked = screen();
    record(&mut clicked, FlightCommand::Accelerate, Key::KeyT);
    let at = center(clicked.button_bounds(KeysButton::Cancel));
    click(&mut clicked, at);
    assert!(clicked.closed());
    assert_eq!(clicked.take_change(), None);
    let mut escaped = screen();
    record(&mut escaped, FlightCommand::Accelerate, Key::KeyT);
    tap(&mut escaped, Key::Escape);
    assert!(escaped.closed());
    assert_eq!(escaped.take_change(), None);
    tap(&mut escaped, Key::Enter);
    assert_eq!(escaped.recording(), None, "closed, it takes nothing");
}

#[test]
fn tab_moves_the_focus_and_return_presses_the_focused_button() {
    let mut screen = screen();
    assert_eq!(screen.focus(), KeysFocus::List);
    tap(&mut screen, Key::Tab);
    assert_eq!(
        screen.focus(),
        KeysFocus::Button(KeysButton::RestoreDefaults)
    );
    tap(&mut screen, Key::ArrowDown);
    assert_eq!(
        screen.row_list().selected(),
        Some(1),
        "the list isn't focused"
    );
    tap(&mut screen, Key::Delete);
    assert_eq!(screen.draft(), &Bindings::default(), "nor is Delete");
    tap(&mut screen, Key::Tab);
    tap(&mut screen, Key::Tab);
    assert_eq!(screen.focus(), KeysFocus::Button(KeysButton::Ok));
    tap(&mut screen, Key::Tab);
    assert_eq!(screen.focus(), KeysFocus::List);
    tap(&mut screen, Key::Tab);
    tap(&mut screen, Key::Tab);
    tap(&mut screen, Key::Enter);
    assert!(screen.closed(), "Cancel pressed");
}

#[test]
fn up_and_down_move_the_selection_over_the_headings_repeats_too() {
    let mut screen = screen();
    tap(&mut screen, Key::ArrowUp);
    let last = screen.row_kinds().len() - 1;
    assert_eq!(
        screen.row_list().selected(),
        Some(last),
        "round to the last"
    );
    screen.input(&Input::Key {
        key: Key::ArrowDown,
        pressed: true,
        repeat: true,
    });
    assert_eq!(
        screen.row_list().selected(),
        Some(1),
        "past the Flight heading"
    );
}

#[test]
fn a_click_on_a_row_records_it_and_a_click_elsewhere_cancels() {
    let mut screen = screen();
    let row = screen.row_list().row_bounds(3).expect("shown");
    click(&mut screen, center(row));
    let third = screen.row_kinds()[3];
    assert_eq!(third, Row::Command(FlightCommand::RotateRight.into()));
    assert_eq!(screen.recording(), Some(FlightCommand::RotateRight.into()));
    click(&mut screen, center(row));
    assert_eq!(
        screen.recording(),
        Some(FlightCommand::RotateRight.into()),
        "its own row"
    );
    click(
        &mut screen,
        Point::new(bounds().min.x + 5.0, bounds().min.y + 5.0),
    );
    assert_eq!(screen.recording(), None);
    assert_eq!(cell(&screen, FlightCommand::RotateRight), "Right Arrow");
    let heading = screen.row_list().row_bounds(0).expect("shown");
    click(&mut screen, center(heading));
    assert_eq!(screen.recording(), None, "a heading records nothing");
}

#[test]
fn a_click_on_ok_while_recording_ends_the_recording_and_presses_ok() {
    let mut screen = screen();
    tap(&mut screen, Key::Enter);
    let at = center(screen.button_bounds(KeysButton::Ok));
    click(&mut screen, at);
    assert_eq!(screen.recording(), None);
    assert!(screen.closed());
}

#[test]
fn the_fixed_keys_answer_whatever_the_bindings_hold() {
    let mut bindings = Bindings::default();
    for (command, k) in [
        (UiCommand::Up, Key::KeyA),
        (UiCommand::Down, Key::KeyB),
        (UiCommand::Confirm, Key::KeyC),
        (UiCommand::Back, Key::KeyD),
        (UiCommand::Tab, Key::KeyE),
    ] {
        bindings = bindings.rebound(command, KeyCombo::key(k));
    }
    bindings = bindings
        .rebound(FlightCommand::Board, KeyCombo::key(Key::ArrowDown))
        .rebound(FlightCommand::Eject, KeyCombo::key(Key::Enter))
        .rebound(FlightCommand::Communicate, KeyCombo::key(Key::Escape))
        .rebound(FlightCommand::Recall, KeyCombo::key(Key::Tab))
        .rebound(FlightCommand::Afterburner, KeyCombo::key(Key::Delete));
    let mut screen = screen_with(bindings.clone(), Rc::new(NoKeyNames), &Rulebook::default());
    tap(&mut screen, Key::ArrowDown);
    assert_eq!(screen.row_list().selected(), Some(2), "Down navigates");
    tap(&mut screen, Key::Delete);
    assert_eq!(
        combo(&screen, FlightCommand::ReverseCourse),
        None,
        "Delete clears"
    );
    tap(&mut screen, Key::Enter);
    assert_eq!(
        screen.recording(),
        Some(FlightCommand::ReverseCourse.into())
    );
    click(
        &mut screen,
        Point::new(bounds().min.x + 5.0, bounds().min.y + 5.0),
    );
    tap(&mut screen, Key::Tab);
    assert_eq!(
        screen.focus(),
        KeysFocus::Button(KeysButton::RestoreDefaults)
    );
    tap(&mut screen, Key::Escape);
    assert!(screen.closed(), "Escape cancels");
}

#[test]
fn the_screen_plays_no_sound() {
    let mut screen = screen();
    record(&mut screen, FlightCommand::JettisonCargo, Key::KeyQ);
    record(&mut screen, PortCommand::Bar, Key::Escape);
    let at = center(screen.button_bounds(KeysButton::Ok));
    click(&mut screen, at);
    assert_eq!(screen.take_sounds(), []);
}

#[test]
fn it_draws_its_backdrop_title_rows_and_buttons() {
    let screen = screen();
    let mut list = DrawList::new();
    screen.draw(&mut list);
    let texts: Vec<String> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&TITLE.to_owned()));
    assert!(texts.contains(&"Flight".to_owned()));
    assert!(texts.contains(&"  Accelerate".to_owned()));
    assert!(texts.contains(&"Up Arrow".to_owned()));
    for button in KeysButton::ALL {
        assert!(texts.contains(&button.label().to_owned()), "{button:?}");
    }
    assert!(
        matches!(list.iter().next(), Some(DrawCommand::Line { .. })),
        "the backdrop"
    );
}

#[test]
fn a_held_press_on_a_button_is_let_go_by_cancel_pointer() {
    let mut screen = screen();
    let ok = center(screen.button_bounds(KeysButton::Ok));
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: ok,
    });
    screen.cancel_pointer();
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at: ok,
    });
    assert!(!screen.closed());
}

#[test]
fn a_key_let_go_after_release_keys_is_not_held() {
    let mut screen = held_screen();
    screen.input(&key(Key::ShiftLeft, true));
    screen.release_keys();
    select(&mut screen, FlightCommand::Board);
    tap(&mut screen, Key::Enter);
    tap(&mut screen, Key::KeyJ);
    assert_eq!(
        combo(&screen, FlightCommand::Board),
        Some(KeyCombo::key(Key::KeyJ))
    );
}

#[test]
fn the_title_list_and_buttons_sit_where_the_layout_says() {
    let screen = screen();
    let origin = bounds().min;
    let at = |(l, t, r, b): (f32, f32, f32, f32)| Bounds {
        min: Point::new(origin.x + l, origin.y + t),
        max: Point::new(origin.x + r, origin.y + b),
    };
    assert_eq!(screen.button_bounds(KeysButton::Ok), at(OK_AREA));
    assert_eq!(screen.button_bounds(KeysButton::Cancel), at(CANCEL_AREA));
    assert_eq!(
        screen.button_bounds(KeysButton::RestoreDefaults),
        at(RESTORE_AREA)
    );
    let list = at(LIST_AREA);
    assert_eq!(screen.row_list().view().min, list.min);
    let mut drawn = DrawList::new();
    screen.draw(&mut drawn);
    let title = drawn.iter().find_map(|command| match command {
        DrawCommand::Text { text, origin, .. } if text == TITLE => Some(*origin),
        _ => None,
    });
    assert_eq!(
        title,
        Some(Point::new(origin.x + TITLE_AT.x, origin.y + TITLE_AT.y))
    );
}

#[test]
fn the_buttons_are_labelled_restore_defaults_cancel_and_ok() {
    let labels: Vec<&str> = KeysButton::ALL.iter().map(|b| b.label()).collect();
    assert_eq!(labels, ["Restore Defaults", "Cancel", "OK"]);
}

#[test]
fn it_debugs_as_the_keys_screen() {
    let text = format!("{:?}", screen());
    assert!(text.starts_with("KeysScreen"), "{text}");
    assert!(text.contains("focus: List"), "{text}");
}

#[test]
fn an_untouched_command_is_not_marked() {
    let mut screen = screen();
    record(&mut screen, FlightCommand::ClosestTarget, Key::KeyW);
    assert!(!screen.is_marked(FlightCommand::ClosestTarget.into()));
    assert!(!screen.is_marked(FlightCommand::Accelerate.into()));
}

#[test]
fn a_repeat_of_a_fixed_key_but_the_arrows_does_nothing() {
    let mut screen = screen();
    for k in [Key::Enter, Key::Space, Key::Delete, Key::Tab, Key::Escape] {
        screen.input(&Input::Key {
            key: k,
            pressed: true,
            repeat: true,
        });
    }
    assert_eq!(screen.recording(), None);
    assert_eq!(screen.focus(), KeysFocus::List);
    assert!(!screen.closed());
    assert_eq!(screen.draft(), &Bindings::default());
}

#[test]
fn a_press_released_off_the_selected_row_or_from_the_bar_records_nothing() {
    let press = |pressed, at| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };
    let mut dragged = screen();
    let row = dragged.row_list().row_bounds(2).expect("shown");
    let beside = Point::new(bounds().max.x + 20.0, row.center().y);
    dragged.input(&press(true, center(row)));
    dragged.input(&Input::PointerMoved(beside));
    dragged.input(&press(false, beside));
    assert_eq!(dragged.row_list().selected(), Some(2), "still selected");
    assert_eq!(dragged.recording(), None, "released off the row");
    let mut screen = screen();
    let bar = screen.row_list().bar().expect("a bar");
    screen.input(&press(true, Point::new(bar.center().x, bar.max.y - 3.0)));
    let selected = screen
        .row_list()
        .selected()
        .and_then(|row| screen.row_list().row_bounds(row))
        .expect("shown");
    screen.input(&Input::PointerMoved(center(selected)));
    screen.input(&press(false, center(selected)));
    assert_eq!(screen.recording(), None, "a press on the bar");
}

#[test]
fn a_held_scroll_arrow_steps_on_as_time_passes() {
    let mut screen = screen();
    let bar = screen.row_list().bar().expect("a bar");
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: Point::new(bar.center().x, bar.max.y - 3.0),
    });
    let pressed = screen.row_list().first();
    screen.tick(Duration::from_secs(2));
    assert!(screen.row_list().first() > pressed);
}

#[test]
fn the_platform_s_key_names_name_the_rows() {
    let screen = screen_with(
        Bindings::default(),
        Rc::new(
            FakeKeyNames::new()
                .with(Key::ArrowUp, "nope")
                .with(Key::KeyW, "ü"),
        ),
        &Rulebook::default(),
    );
    assert_eq!(cell(&screen, FlightCommand::SelectSecondary), "Ü");
    assert_eq!(cell(&screen, FlightCommand::Accelerate), "Up Arrow");
}
