#![allow(clippy::float_cmp)]

use super::*;
use crate::draw::DrawCommand;
use crate::geometry::Point;
use crate::input::{ControllerFamily, PadButton};
use crate::keymap::{CommandInput, FlightCommand, UiCommand};

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
    Bounds::at(Point::new(x, y), w, h)
}

fn target(id: u8, x: f32, y: f32) -> Target<u8> {
    Target::at(id, rect(x, y, 10.0, 10.0))
}

#[test]
fn a_direction_picks_the_nearest_target_that_way() {
    let targets = [
        target(0, 0.0, 0.0),
        target(1, 50.0, 0.0),
        target(2, 20.0, 0.0),
        target(3, 0.0, 30.0),
        target(4, 0.0, -40.0),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(2));
    assert_eq!(pick(&targets, 0, Direction::Down), Some(3));
    assert_eq!(pick(&targets, 0, Direction::Up), Some(4));
    assert_eq!(pick(&targets, 1, Direction::Left), Some(2));
}

#[test]
fn an_off_axis_target_loses_to_a_farther_one_in_line() {
    // In line at x = 30 scores 20; at (15, 20) scores 5 + 2 x 10 = 25.
    let targets = [
        target(0, 0.0, 0.0),
        target(1, 30.0, 0.0),
        target(2, 15.0, 20.0),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(1));
    // Moved in to x = 36 the in-line one scores 26 and loses.
    let targets = [
        target(0, 0.0, 0.0),
        target(1, 36.0, 0.0),
        target(2, 15.0, 20.0),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(2));
}

#[test]
fn a_target_overlapping_across_counts_as_in_line() {
    // Overlapping in y by a unit: no gap across, so it scores its 25 along
    // and beats a 30 in line... which it would lose to by 2 x 1 otherwise.
    let targets = [
        target(0, 0.0, 0.0),
        Target::at(1, rect(35.0, 9.0, 10.0, 10.0)),
        target(2, 40.0, 0.0),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(1));
    let targets = [
        target(0, 0.0, 0.0),
        Target::at(1, rect(35.0, 11.0, 10.0, 10.0)),
        Target::at(2, rect(36.0, 0.0, 10.0, 10.0)),
    ];
    assert_eq!(
        pick(&targets, 0, Direction::Right),
        Some(2),
        "a gap of 1 costs 2"
    );
}

#[test]
fn a_target_whose_centre_is_not_beyond_is_never_picked() {
    // A wide bar whose centre is level with the focus's: not to the right.
    let targets = [
        target(0, 20.0, 0.0),
        Target::at(1, rect(0.0, 20.0, 50.0, 10.0)),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), None);
    assert_eq!(pick(&targets, 0, Direction::Left), None);
    assert_eq!(pick(&targets, 0, Direction::Down), Some(1));
}

#[test]
fn nothing_that_way_keeps_the_focus() {
    let targets = [target(0, 0.0, 0.0), target(1, 20.0, 0.0)];
    assert_eq!(pick(&targets, 1, Direction::Right), None, "no wrap");
    assert_eq!(
        step(&targets, Some(1), 0, FocusMove::Toward(Direction::Right)),
        Some(1)
    );
    assert_eq!(pick(&targets, 9, Direction::Right), None, "not a target");
}

#[test]
fn ties_go_to_the_nearer_centre_across_then_to_tab_order() {
    // Both score 10 along (overlapping across): the one whose centre is
    // nearer across wins.
    let targets = [
        target(0, 0.0, 0.0),
        Target::at(1, rect(20.0, -8.0, 10.0, 10.0)),
        Target::at(2, rect(20.0, 3.0, 10.0, 10.0)),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(2));
    // Mirror images: the earlier in Tab order wins.
    let targets = [
        target(0, 0.0, 0.0),
        Target::at(1, rect(20.0, -3.0, 10.0, 10.0)),
        Target::at(2, rect(20.0, 3.0, 10.0, 10.0)),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(1));
    let targets = [
        target(0, 0.0, 0.0),
        Target::at(2, rect(20.0, 3.0, 10.0, 10.0)),
        Target::at(1, rect(20.0, -3.0, 10.0, 10.0)),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(2));
}

#[test]
fn an_explicit_neighbour_wins_over_the_geometry() {
    let targets = [
        target(0, 0.0, 0.0).with_neighbour(Direction::Right, 2),
        target(1, 20.0, 0.0),
        target(2, 0.0, 90.0),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(2));
    assert_eq!(pick(&targets, 0, Direction::Down), Some(2), "geometry");
    assert_eq!(targets[0].neighbour(Direction::Right), Some(2));
    assert_eq!(targets[0].neighbour(Direction::Left), None);
}

#[test]
fn an_explicit_neighbour_no_longer_a_target_falls_back_to_the_geometry() {
    let targets = [
        target(0, 0.0, 0.0).with_neighbour(Direction::Right, 7),
        target(1, 20.0, 0.0),
    ];
    assert_eq!(pick(&targets, 0, Direction::Right), Some(1));
}

#[test]
fn each_direction_has_its_own_neighbour() {
    let mut origin = target(0, 0.0, 0.0);
    for (direction, id) in [
        (Direction::Up, 1),
        (Direction::Down, 2),
        (Direction::Left, 3),
        (Direction::Right, 4),
    ] {
        origin = origin.with_neighbour(direction, id);
    }
    let targets = [
        origin,
        target(1, 90.0, 90.0),
        target(2, 90.0, 90.0),
        target(3, 90.0, 90.0),
        target(4, 90.0, 90.0),
    ];
    assert_eq!(pick(&targets, 0, Direction::Up), Some(1));
    assert_eq!(pick(&targets, 0, Direction::Down), Some(2));
    assert_eq!(pick(&targets, 0, Direction::Left), Some(3));
    assert_eq!(pick(&targets, 0, Direction::Right), Some(4));
}

#[test]
fn next_and_previous_follow_tab_order_and_wrap() {
    let targets = [
        target(5, 90.0, 0.0),
        target(3, 0.0, 0.0),
        target(8, 40.0, 0.0),
    ];
    assert_eq!(step(&targets, Some(5), 0, FocusMove::Next), Some(3));
    assert_eq!(step(&targets, Some(3), 0, FocusMove::Next), Some(8));
    assert_eq!(step(&targets, Some(8), 0, FocusMove::Next), Some(5));
    assert_eq!(step(&targets, Some(5), 0, FocusMove::Previous), Some(8));
    assert_eq!(step(&targets, Some(8), 0, FocusMove::Previous), Some(3));
}

#[test]
fn with_nothing_focused_any_move_lands_on_the_initial_target() {
    let targets = [
        target(0, 0.0, 0.0),
        target(1, 20.0, 0.0),
        target(2, 40.0, 0.0),
    ];
    for mv in [
        FocusMove::Next,
        FocusMove::Previous,
        FocusMove::Toward(Direction::Right),
        FocusMove::Toward(Direction::Left),
    ] {
        assert_eq!(step(&targets, None, 1, mv), Some(1), "{mv:?}");
    }
    assert_eq!(step::<u8>(&[], None, 1, FocusMove::Next), None, "no target");
}

#[test]
fn a_focus_no_longer_a_target_moves_as_if_nothing_were_focused() {
    let targets = [target(0, 0.0, 0.0), target(1, 20.0, 0.0)];
    assert_eq!(
        step(&targets, Some(7), 0, FocusMove::Toward(Direction::Right)),
        Some(0)
    );
    assert_eq!(step(&targets, Some(7), 1, FocusMove::Next), Some(1));
}

#[test]
fn focus_moves_read_from_the_ui_commands() {
    let ui = Command::Ui;
    assert_eq!(
        FocusMove::of(ui(UiCommand::FocusUp)),
        Some(FocusMove::Toward(Direction::Up))
    );
    assert_eq!(
        FocusMove::of(ui(UiCommand::FocusDown)),
        Some(FocusMove::Toward(Direction::Down))
    );
    assert_eq!(
        FocusMove::of(ui(UiCommand::FocusLeft)),
        Some(FocusMove::Toward(Direction::Left))
    );
    assert_eq!(
        FocusMove::of(ui(UiCommand::FocusRight)),
        Some(FocusMove::Toward(Direction::Right))
    );
    assert_eq!(FocusMove::of(ui(UiCommand::Tab)), Some(FocusMove::Next));
    assert_eq!(
        FocusMove::of(ui(UiCommand::PrevFocus)),
        Some(FocusMove::Previous)
    );
    for other in [UiCommand::Up, UiCommand::Activate, UiCommand::Back] {
        assert_eq!(FocusMove::of(ui(other)), None, "{other:?}");
    }
    assert_eq!(
        FocusMove::of(Command::Flight(FlightCommand::TargetSelect)),
        None
    );
}

#[test]
fn a_gamepad_surroundings_places_the_initial_focus_once() {
    let mut state = FocusState::<u8>::default();
    assert_eq!(state.device(), Device::Keyboard);
    state.surroundings(PAD, Some(4), false);
    assert_eq!(state.get(), Some(4));
    assert!(state.pad_moved());
    assert_eq!(state.device(), PAD);
    state.set_by_pad(6);
    state.surroundings(PAD, Some(4), false);
    assert_eq!(state.get(), Some(6), "not placed again");
    let mut none = FocusState::<u8>::default();
    none.surroundings(PAD, None, false);
    assert_eq!(none.get(), None);
}

#[test]
fn the_pad_highlight_shows_only_while_the_pad_was_used_last() {
    let mut state = FocusState::<u8>::default();
    state.surroundings(PAD, Some(2), true);
    assert_eq!(state.pad_highlight(), Some(2));
    assert!(state.pad_shown());
    for device in [Device::Keyboard, Device::Mouse] {
        let mut left = state;
        left.surroundings(device, Some(2), true);
        assert_eq!(left.pad_highlight(), None, "{device:?}");
        assert!(!left.pad_shown());
    }
    let mut keyed = FocusState::<u8>::default();
    keyed.set_by_keys(Some(1));
    keyed.surroundings(PAD, Some(2), false);
    assert_eq!(keyed.pad_highlight(), None, "the keyboard placed it");
    assert_eq!(keyed.get(), Some(1));
}

fn ui_input(command: UiCommand, pressed: bool) -> CommandInput {
    CommandInput {
        command: Command::Ui(command),
        pressed,
        repeat: false,
    }
}

#[test]
fn the_pads_focus_moves_and_south_are_held_back_until_the_pad_shows() {
    let focus_or_south = [
        UiCommand::FocusUp,
        UiCommand::FocusDown,
        UiCommand::FocusLeft,
        UiCommand::FocusRight,
        UiCommand::Tab,
        UiCommand::PrevFocus,
        UiCommand::Activate,
    ];
    let mut state = FocusState::<u8>::default();
    for command in focus_or_south {
        assert!(state.pad_holds_back(ui_input(command, true)), "{command:?}");
        assert!(
            !state.pad_holds_back(ui_input(command, false)),
            "a release finishes only what a press started: {command:?}"
        );
    }
    for command in [UiCommand::Back, UiCommand::Up, UiCommand::Confirm] {
        assert!(
            !state.pad_holds_back(ui_input(command, true)),
            "{command:?}"
        );
    }
    state.surroundings(PAD, Some(1), false);
    for command in focus_or_south {
        assert!(
            !state.pad_holds_back(ui_input(command, true)),
            "{command:?}"
        );
    }
    state.surroundings(Device::Mouse, Some(1), false);
    assert!(state.pad_holds_back(ui_input(UiCommand::Activate, true)));
}

#[test]
fn leaving_the_pad_drops_a_focus_it_placed_and_keeps_a_keyboard_one() {
    let mut state = FocusState::<u8>::default();
    state.surroundings(PAD, Some(2), false);
    state.surroundings(Device::Mouse, Some(2), false);
    assert_eq!(state.get(), None);
    assert!(!state.pad_moved());
    let mut keyed = FocusState::<u8>::default();
    keyed.set_by_keys(Some(1));
    keyed.surroundings(PAD, Some(2), false);
    keyed.surroundings(Device::Keyboard, Some(2), false);
    assert_eq!(keyed.get(), Some(1));
    assert_eq!(keyed.keyboard_ring(), Some(1));
}

#[test]
fn a_screen_that_keeps_its_focus_hands_it_to_the_keyboard() {
    let mut state = FocusState::<u8>::default();
    state.set_by_keys(Some(0));
    state.surroundings(PAD, Some(0), true);
    state.set_by_pad(3);
    state.surroundings(Device::Keyboard, Some(0), true);
    assert_eq!(state.get(), Some(3));
    assert_eq!(state.keyboard_ring(), Some(3));
    assert!(!state.pad_moved());
}

#[test]
fn the_keyboard_ring_never_shows_a_pad_placed_focus() {
    let mut state = FocusState::<u8>::default();
    assert_eq!(state.keyboard_ring(), None);
    state.set_by_keys(Some(1));
    assert_eq!(state.keyboard_ring(), Some(1));
    state.set_by_pad(2);
    assert_eq!(state.keyboard_ring(), None);
    state.set_by_keys(Some(2));
    assert_eq!(state.keyboard_ring(), Some(2));
}

#[test]
fn carrying_the_pad_over_takes_its_device_and_its_focus() {
    let mut old = FocusState::<u8>::default();
    old.surroundings(PAD, Some(1), false);
    old.set_by_pad(3);
    let mut new = FocusState::<u8>::default();
    new.carry_pad_from(&old);
    assert_eq!(new.device(), PAD);
    assert_eq!(new.pad_highlight(), Some(3));
    let mut keyed = FocusState::<u8>::default();
    keyed.set_by_keys(Some(2));
    let mut fresh = FocusState::<u8>::default();
    fresh.carry_pad_from(&keyed);
    assert_eq!(
        fresh.get(),
        None,
        "the keyboard's focus is the host's to carry"
    );
}

#[test]
fn the_highlight_is_a_two_unit_ring_outside_the_control() {
    let mut list = DrawList::new();
    let control = rect(10.0, 20.0, 30.0, 40.0);
    draw_highlight(&mut list, control, Color::WHITE);
    let lines: Vec<(Point, Point, f32)> = list
        .iter()
        .map(|command| match *command {
            DrawCommand::Line {
                from, to, width, ..
            } => (from, to, width),
            ref other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(lines.len(), 8);
    assert_eq!(
        lines[0],
        (Point::new(8.0, 18.0), Point::new(42.0, 18.0), 1.0)
    );
    assert_eq!(
        lines[2],
        (Point::new(42.0, 62.0), Point::new(8.0, 62.0), 1.0)
    );
    assert_eq!(
        lines[4],
        (Point::new(7.0, 17.0), Point::new(43.0, 17.0), 1.0)
    );
    assert_eq!(
        lines[6],
        (Point::new(43.0, 63.0), Point::new(7.0, 63.0), 1.0)
    );
}

#[test]
fn only_a_pad_button_is_from_the_pad() {
    assert!(from_pad(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    }));
    assert!(!from_pad(&Input::Key {
        key: crate::input::Key::Space,
        pressed: true,
        repeat: false,
    }));
}

#[test]
fn the_gaps_are_the_edge_gaps_along_and_across_and_the_centre_offset() {
    // `a` spans x 40..50, y 60..80; each `b` lies one way, off to a side.
    let a = rect(40.0, 60.0, 10.0, 20.0);
    // Right, above: along 57 - 50, across 60 - 52, centres 70 vs 46.
    assert_eq!(
        gaps(a, rect(57.0, 40.0, 6.0, 12.0), Direction::Right),
        (7.0, 8.0, 24.0)
    );
    // Right, below: across 83 - 80.
    assert_eq!(
        gaps(a, rect(57.0, 83.0, 6.0, 12.0), Direction::Right),
        (7.0, 3.0, 19.0)
    );
    // Left, above: along 40 - 31.
    assert_eq!(
        gaps(a, rect(25.0, 40.0, 6.0, 12.0), Direction::Left),
        (9.0, 8.0, 24.0)
    );
    // Down, left of it: along 85 - 80, across 40 - 33, centres 45 vs 30.
    assert_eq!(
        gaps(a, rect(27.0, 85.0, 6.0, 4.0), Direction::Down),
        (5.0, 7.0, 15.0)
    );
    // Up, right of it: along 60 - 52, across 54 - 50, centres 45 vs 57.
    assert_eq!(
        gaps(a, rect(54.0, 48.0, 6.0, 4.0), Direction::Up),
        (8.0, 4.0, 12.0)
    );
    // Overlapping both ways: no gap.
    assert_eq!(
        gaps(a, rect(45.0, 65.0, 10.0, 10.0), Direction::Right),
        (0.0, 0.0, 0.0)
    );
}

#[test]
fn beyond_is_strict_each_way() {
    let a = rect(40.0, 60.0, 10.0, 20.0);
    let level = rect(42.0, 62.0, 6.0, 16.0);
    for direction in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        assert!(!beyond(a, level, direction), "{direction:?}");
    }
    assert!(beyond(a, rect(60.0, 60.0, 1.0, 1.0), Direction::Right));
    assert!(beyond(a, rect(10.0, 60.0, 1.0, 1.0), Direction::Left));
    assert!(beyond(a, rect(40.0, 90.0, 1.0, 1.0), Direction::Down));
    assert!(beyond(a, rect(40.0, 10.0, 1.0, 1.0), Direction::Up));
}

#[test]
fn an_untold_screen_reads_a_pads_move_or_south_as_nothing_and_the_rest_as_their_own() {
    let state = FocusState::<u8>::default();
    for command in [UiCommand::FocusUp, UiCommand::Tab, UiCommand::Activate] {
        assert_eq!(state.read_pad(ui_input(command, true)), PadRead::Nothing);
    }
    assert_eq!(
        state.read_pad(ui_input(UiCommand::Back, true)),
        PadRead::Other
    );
    assert_eq!(
        state.read_pad(ui_input(UiCommand::Up, true)),
        PadRead::Other
    );
}

#[test]
fn a_move_acts_on_its_press_only() {
    let mut state = FocusState::<u8>::default();
    state.surroundings(PAD, None, false);
    assert_eq!(
        state.read_pad(ui_input(UiCommand::FocusLeft, true)),
        PadRead::Move(FocusMove::Toward(Direction::Left))
    );
    assert_eq!(
        state.read_pad(ui_input(UiCommand::PrevFocus, true)),
        PadRead::Move(FocusMove::Previous)
    );
    assert_eq!(
        state.read_pad(ui_input(UiCommand::FocusLeft, false)),
        PadRead::Nothing
    );
}

#[test]
fn south_reads_as_press_then_release() {
    let mut state = FocusState::<u8>::default();
    state.surroundings(PAD, None, false);
    assert_eq!(
        state.read_pad(ui_input(UiCommand::Activate, true)),
        PadRead::Press
    );
    assert_eq!(
        state.read_pad(ui_input(UiCommand::Activate, false)),
        PadRead::Release
    );
}

#[test]
fn a_release_passes_while_untold() {
    let state = FocusState::<u8>::default();
    assert_eq!(
        state.read_pad(ui_input(UiCommand::Activate, false)),
        PadRead::Release
    );
    assert_eq!(
        state.read_pad(ui_input(UiCommand::Back, false)),
        PadRead::Other
    );
}

#[test]
fn moving_by_pad_places_the_pads_focus_from_the_initial_target() {
    let targets = [
        target(0, 0.0, 0.0),
        target(1, 30.0, 0.0),
        target(2, 60.0, 0.0),
    ];
    let mut state = FocusState::<u8>::default();
    state.surroundings(PAD, None, false);
    let right = FocusMove::Toward(Direction::Right);
    assert!(state.move_by_pad(&targets, 1, right));
    assert_eq!(state.get(), Some(1), "the first move lands on the initial");
    assert!(state.pad_moved());
    assert!(state.move_by_pad(&targets, 1, right));
    assert_eq!(state.get(), Some(2));
    assert_eq!(state.pad_highlight(), Some(2));
}

#[test]
fn moving_by_keys_places_the_keyboards() {
    let targets = [target(0, 0.0, 0.0), target(1, 30.0, 0.0)];
    let mut state = FocusState::<u8>::default();
    state.set_by_pad(0);
    assert!(state.move_by_keys(&targets, 0, FocusMove::Next));
    assert_eq!(state.get(), Some(1));
    assert!(!state.pad_moved());
    assert_eq!(state.keyboard_ring(), Some(1));
}

#[test]
fn a_move_with_no_target_moves_nothing() {
    let mut state = FocusState::<u8>::default();
    state.set_by_keys(Some(4));
    assert!(!state.move_by_pad(&[], 0, FocusMove::Next));
    assert!(!state.move_by_keys(&[], 0, FocusMove::Next));
    assert_eq!(state.get(), Some(4));
    assert!(!state.pad_moved());
}

#[test]
fn told_keeps_the_last_surroundings_and_nothing_else() {
    let mut told = Told::default();
    assert!(!told.keep(&Input::PointerLeft));
    assert!(!told.keep(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    }));
    assert_eq!(told, Told::default());
    assert!(told.keep(&surroundings(PAD)));
    assert!(told.keep(&surroundings(Device::Mouse)));
    assert_eq!(told.device(), Device::Mouse);
}

#[test]
fn told_tells_a_screen_what_it_kept_and_nothing_before_any() {
    let mut screen = Recorder::default();
    let mut told = Told::default();
    told.tell(&mut screen);
    assert!(screen.0.is_empty());
    told.keep(&surroundings(PAD));
    told.tell(&mut screen);
    assert_eq!(screen.0, vec![surroundings(PAD)]);
}

#[test]
fn told_reads_the_keyboard_before_any() {
    assert_eq!(Told::default().device(), Device::Keyboard);
}

fn surroundings(device: Device) -> Input {
    Input::Surroundings(crate::input::Surroundings { device })
}

/// A screen that records its inputs.
#[derive(Default)]
struct Recorder(Vec<Input>);

impl crate::screen::Screen for Recorder {
    fn input(&mut self, input: &Input) -> crate::screen::ScreenAction {
        self.0.push(*input);
        crate::screen::ScreenAction::None
    }

    fn tick(&mut self, _dt: std::time::Duration) {}

    fn draw(&self, _list: &mut DrawList) {}
}
