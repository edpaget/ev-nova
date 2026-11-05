use super::super::tests::{click_button, drawn, on_cargo, press, screen};
use super::*;
use crate::input::{ControllerFamily, Key, PadButton};
use crate::screen::Screen;
use crate::sound::Sound;
use crate::ui::player_info::Tab;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn tap(screen: &mut PlayerInfoScreen, button: PadButton) {
    screen.input(&Input::PadButton {
        button,
        pressed: true,
    });
    screen.input(&Input::PadButton {
        button,
        pressed: false,
    });
}

#[test]
fn the_pads_focus_starts_on_done_and_the_dpad_moves_it() {
    let mut screen = on_cargo();
    screen.input(&told(PAD));
    assert_eq!(screen.pad_focus(), Some(PlayerInfoButton::Done));
    assert_eq!(screen.device(), PAD);
    let mut seen = vec![screen.pad_focus()];
    for button in [PadButton::DPadUp, PadButton::DPadLeft, PadButton::DPadRight] {
        tap(&mut screen, button);
        seen.push(screen.pad_focus());
    }
    assert!(
        seen.iter().skip(1).any(|focus| *focus != seen[0]),
        "{seen:?}"
    );
}

#[test]
fn the_left_shoulder_shows_the_previous_tab_as_shift_tab_does() {
    let mut shifted = screen();
    shifted.input(&press(Key::ShiftLeft));
    shifted.input(&press(Key::Tab));
    let mut padded = screen();
    padded.input(&told(PAD));
    tap(&mut padded, PadButton::LeftShoulder);
    assert_eq!(padded.tab(), Tab::Honors);
    assert_eq!(padded.tab(), shifted.tab());
    tap(&mut padded, PadButton::RightShoulder);
    assert_eq!(padded.tab(), Tab::General, "the right shoulder is Tab");
    assert_eq!(
        padded.pad_focus(),
        Some(PlayerInfoButton::Done),
        "no focus move"
    );
}

#[test]
fn south_presses_the_focused_button_on_its_release_silently() {
    let mut screen = screen();
    screen.input(&told(PAD));
    tap(&mut screen, PadButton::DPadUp);
    let focused = screen.pad_focus().expect("focused");
    assert!(matches!(focused, PlayerInfoButton::Tab(_)), "{focused:?}");
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    assert_eq!(screen.tab(), Tab::General, "the press chooses nothing");
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    let PlayerInfoButton::Tab(tab) = focused else {
        unreachable!()
    };
    assert_eq!(screen.tab(), tab);
    assert_eq!(screen.take_sounds(), Vec::<Sound>::new());
}

#[test]
fn a_dpad_press_while_south_is_held_drops_the_press() {
    let mut screen = on_cargo();
    screen.input(&told(PAD));
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    tap(&mut screen, PadButton::DPadUp);
    screen.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert!(!screen.closed(), "Done's press was dropped");
}

#[test]
fn the_jettison_question_takes_the_pad_and_east_does_what_escape_does() {
    let mut escaped = on_cargo();
    click_button(&mut escaped, PlayerInfoButton::Jettison);
    escaped.input(&press(Key::Escape));
    let mut padded = on_cargo();
    padded.input(&told(PAD));
    click_button(&mut padded, PlayerInfoButton::Jettison);
    let confirm = padded.confirm().expect("asking");
    assert_eq!(confirm.device(), PAD, "told as it opens");
    assert_eq!(confirm.pad_highlight(), Some(1), "on OK, its default");
    tap(&mut padded, PadButton::East);
    assert_eq!(padded.confirm_open(), escaped.confirm_open());
    assert!(padded.confirm_open(), "Escape answers nothing here");
    tap(&mut padded, PadButton::DPadLeft);
    tap(&mut padded, PadButton::South);
    assert!(!padded.confirm_open(), "Cancel answers no");
    assert!(!padded.take_jettison());
}

#[test]
fn keyboard_space_still_does_nothing() {
    let mut screen = screen();
    screen.input(&told(PAD));
    screen.input(&told(Device::Keyboard));
    screen.input(&press(Key::Space));
    assert!(!screen.closed());
    assert_eq!(screen.tab(), Tab::General);
    let mut keyed = super::super::tests::screen();
    keyed.input(&press(Key::Space));
    assert_eq!(drawn(&screen), drawn(&keyed));
}

#[test]
fn under_the_question_the_screen_draws_no_highlight_of_its_own() {
    let mut screen = on_cargo();
    screen.input(&told(PAD));
    let before = drawn(&screen);
    click_button(&mut screen, PlayerInfoButton::Jettison);
    let done = screen.button_bounds(PlayerInfoButton::Done).expect("Done");
    let mut ring = DrawList::new();
    pad_focus::draw_highlight(&mut ring, done, screen.style.up);
    let ring: Vec<_> = ring.iter().cloned().collect();
    assert!(before.windows(ring.len()).any(|w| w == ring.as_slice()));
    assert!(
        !drawn(&screen)
            .windows(ring.len())
            .any(|w| w == ring.as_slice())
    );
}
