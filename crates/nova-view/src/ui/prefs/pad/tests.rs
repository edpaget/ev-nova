use super::super::tests::{click, drawn, key, ok, prefs};
use super::*;
use crate::geometry::Point;
use crate::input::{ControllerFamily, Input, Key, PadButton, Surroundings};
use crate::screen::Screen;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn tap(dialog: &mut PrefsDialog, button: PadButton) {
    for pressed in [true, false] {
        dialog.input(&Input::PadButton { button, pressed });
    }
}

/// The pad's d-pad taps that put its focus on `target`, from OK.
fn reach(dialog: &mut PrefsDialog, target: PrefsTarget) {
    for _ in 0..40 {
        if dialog.pad_target() == Some(target) {
            return;
        }
        tap(dialog, PadButton::RightShoulder);
    }
    panic!("never reached {target:?}");
}

#[test]
fn the_pads_first_focus_is_what_step_1_found() {
    let mut dialog = prefs();
    dialog.input(&told(PAD));
    assert_eq!(dialog.pad_target(), Some(PrefsTarget::Part(Focus::Ok)));
    assert_eq!(dialog.device(), PAD);
    tap(&mut dialog, PadButton::South);
    assert!(dialog.closed(), "South at once does what Return does");
}

#[test]
fn the_shoulders_step_the_twenty_targets_in_the_keyboards_order() {
    let mut dialog = prefs();
    dialog.input(&told(PAD));
    let mut parts = Vec::new();
    for _ in 0..20 {
        tap(&mut dialog, PadButton::RightShoulder);
        parts.push(dialog.pad_target().expect("focused"));
    }
    assert_eq!(parts[0], PrefsTarget::Part(Focus::Music), "OK wraps round");
    assert_eq!(parts[10], PrefsTarget::Arrow(Focus::Display, Arrow::Up));
    assert_eq!(parts[11], PrefsTarget::Arrow(Focus::Display, Arrow::Down));
    assert_eq!(parts[19], PrefsTarget::Part(Focus::Ok));
    let mut keyed = parts.iter().map(|target| target.part()).collect::<Vec<_>>();
    keyed.dedup();
    let mut next = Focus::Music;
    for part in keyed {
        assert_eq!(part, next);
        next = next.next();
    }
    tap(&mut dialog, PadButton::LeftShoulder);
    assert_eq!(
        dialog.pad_target(),
        Some(PrefsTarget::Part(Focus::KeySettings))
    );
}

#[test]
fn the_left_stick_steps_the_focused_stepper_as_the_arrow_keys_do() {
    let mut keyed = prefs();
    for _ in 0..13 {
        keyed.input(&key(Key::Tab));
    }
    assert_eq!(keyed.focus(), Some(Focus::EffectsVolume));
    keyed.input(&key(Key::ArrowUp));
    let mut padded = prefs();
    padded.input(&told(PAD));
    reach(
        &mut padded,
        PrefsTarget::Arrow(Focus::EffectsVolume, Arrow::Down),
    );
    tap(&mut padded, PadButton::LeftStickUp);
    assert_eq!(
        padded.effects_volume().level(),
        keyed.effects_volume().level()
    );
    assert_eq!(padded.effects_volume().level(), 5);
    tap(&mut padded, PadButton::LeftStickLeft);
    assert_eq!(padded.effects_volume().level(), 4);
    reach(&mut padded, PrefsTarget::Part(Focus::Music));
    tap(&mut padded, PadButton::LeftStickUp);
    assert_eq!(padded.prefs(), prefs().prefs(), "a toggle takes no step");
}

#[test]
fn keyboard_tab_from_an_arrow_goes_to_the_next_part() {
    let mut dialog = prefs();
    dialog.input(&told(PAD));
    reach(&mut dialog, PrefsTarget::Arrow(Focus::Display, Arrow::Up));
    dialog.input(&key(Key::Tab));
    assert_eq!(dialog.pad_target(), Some(PrefsTarget::Part(Focus::Scaling)));
    assert_eq!(dialog.focus(), Some(Focus::Scaling));
}

#[test]
fn focus_keeps_naming_the_part_for_the_keyboard() {
    let mut dialog = prefs();
    dialog.input(&told(PAD));
    reach(
        &mut dialog,
        PrefsTarget::Arrow(Focus::MusicVolume, Arrow::Down),
    );
    assert_eq!(dialog.focus(), Some(Focus::MusicVolume));
    assert_eq!(PrefsTarget::Part(Focus::Ok).part(), Focus::Ok);
}

#[test]
fn the_inner_dialog_draws_no_highlight_of_its_own() {
    let mut dialog = prefs();
    dialog.input(&told(PAD));
    assert_eq!(dialog.dialog().device(), Device::Keyboard);
    assert_eq!(dialog.dialog().pad_highlight(), None);
    let ok = dialog.dialog().item_bounds(OK_ITEM).expect("OK");
    let mut ring = DrawList::new();
    pad_focus::draw_highlight(&mut ring, ok, dialog.style.up);
    let ring: Vec<_> = ring.iter().cloned().collect();
    let shown = drawn(&dialog);
    let count = shown
        .windows(ring.len())
        .filter(|w| *w == ring.as_slice())
        .count();
    assert_eq!(count, 1, "Preferences' own, once");
}

#[test]
fn the_pad_works_under_focus_ring_off_where_the_keyboard_has_no_focus() {
    let mut dialog = prefs().with_focus_ring(FocusRing::Off);
    dialog.input(&told(PAD));
    reach(&mut dialog, PrefsTarget::Part(Focus::Music));
    dialog.input(&key(Key::Space));
    assert!(!dialog.music().on(), "the keyboard has no focus");
    dialog.input(&key(Key::Tab));
    assert_eq!(dialog.focus(), Some(Focus::Music), "nor Tab");
    tap(&mut dialog, PadButton::South);
    assert!(dialog.music().on(), "the pad presses it");
    dialog.input(&told(Device::Keyboard));
    assert_eq!(dialog.focus(), None, "the pad's focus is dropped");
}

#[test]
fn south_on_key_settings_opens_the_keys_screen() {
    let mut dialog = prefs();
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::LeftShoulder);
    assert_eq!(
        dialog.pad_target(),
        Some(PrefsTarget::Part(Focus::KeySettings))
    );
    dialog.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    assert!(dialog.keys_screen().is_none(), "on the release");
    assert_eq!(dialog.dialog().pressed_item(), Some(KEY_SETTINGS_ITEM));
    dialog.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert!(dialog.keys_screen().is_some());
}

#[test]
fn a_dpad_press_while_south_is_held_drops_the_press() {
    let mut dialog = prefs();
    dialog.input(&told(PAD));
    reach(&mut dialog, PrefsTarget::Part(Focus::Music));
    dialog.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    assert!(dialog.music().pressed());
    tap(&mut dialog, PadButton::DPadDown);
    dialog.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert!(!dialog.music().on());
    assert!(!dialog.music().pressed());
}

#[test]
fn a_key_bound_to_a_focus_move_moves_the_keyboards_focus_under_tab() {
    use crate::keymap::{Bindings, KeyCombo, SharedBindings};
    let bindings = SharedBindings::new(
        Bindings::default().rebound(UiCommand::PrevFocus, KeyCombo::key(Key::KeyH)),
    );
    let mut dialog = prefs().with_bindings(bindings.clone());
    dialog.input(&key(Key::KeyH));
    assert_eq!(
        dialog.focus(),
        Some(Focus::Ok),
        "lands on the initial target"
    );
    dialog.input(&key(Key::KeyH));
    assert_eq!(dialog.focus(), Some(Focus::KeySettings));
    let mut off = prefs()
        .with_bindings(bindings)
        .with_focus_ring(FocusRing::Off);
    off.input(&key(Key::KeyH));
    assert_eq!(off.focus(), None);
}

#[test]
fn the_pads_click_on_ok_matches_the_pointers() {
    let mut clicked = prefs();
    let at: Point = ok(&clicked);
    click(&mut clicked, at);
    let mut padded = prefs();
    padded.input(&told(PAD));
    tap(&mut padded, PadButton::South);
    assert_eq!(padded.closed(), clicked.closed());
}
