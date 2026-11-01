//! What flight takes back as the last dialog over it closes, on
//! [`FakeFlight`] ([`FlightView::take_back_live_input`]): the unlatched
//! keys and the mouse button as they physically are, as `_HandlePlayer`
//! polls them past `_SetAllKeyCmdFlags` (@0x61e6d), which latches only
//! the rest. Which dialogs hand back, and when, is the router's, pinned
//! in the `nova` crate's `app::screen::tests::live_keys`.

use super::controls::faked_cursor_alpha;
use super::faked_flight::{at_rest, hold, last_controls, ticked};
use super::faked_screen::in_flight;
use super::key_fades::{button_down, fades_with_no_release};
use super::*;

/// A dialog opening over `view`'s flight: it lets go of the keys, whose
/// releases will go to the dialog.
fn dialog_opens(view: &mut Faked) {
    view.let_go_of_input();
}

#[test]
fn the_close_gives_back_an_unlatched_key_still_down_and_flight_thrusts_on_the_next_step() {
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Up, true);
    dialog_opens(&mut view);
    view.take_back_live_input([Key::Up], false);
    ticked(&mut view, 1);
    assert!(last_controls(&view).thrust, "Up down at the close");
}

#[test]
fn the_close_gives_back_an_unlatched_key_pressed_under_the_dialog() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    view.take_back_live_input([Key::Left], false);
    ticked(&mut view, 1);
    assert_eq!(last_controls(&view).turn, Turn::Left);
}

#[test]
fn the_close_drops_an_unlatched_key_no_longer_down() {
    let mut view = at_rest(catalog());
    // Left in `held` by a path that kept it: its release went elsewhere.
    hold(&mut view, Key::Up, true);
    view.take_back_live_input([], false);
    ticked(&mut view, 1);
    assert!(!last_controls(&view).thrust, "Up let go under the dialog");
}

#[test]
fn the_close_gives_back_no_latched_key() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    view.take_back_live_input([BOARD_KEY, JUMP_KEY, Key::Up], false);
    assert!(!view.held.contains(&BOARD_KEY), "B waits for its release");
    assert!(!view.held.contains(&JUMP_KEY), "J waits for its release");
    assert!(view.held.contains(&Key::Up));
    let sent_before = sent(&view).len();
    view.held_keys_frame();
    assert!(
        !sent(&view)[sent_before..].contains(&FlightCommand::Board),
        "no board: {:?}",
        sent(&view)
    );
    // Let go and pressed afresh, B boards.
    view.input(&key(BOARD_KEY, false));
    view.input(&key(BOARD_KEY, true));
    assert!(sent(&view)[sent_before..].contains(&FlightCommand::Board));
}

#[test]
fn the_close_keeps_a_latched_key_flight_still_holds() {
    // The latches are `_SetAllKeyCmdFlags`' to set, not the hand-back's
    // to clear (rdm:phase/play-feel-8/phase-4-latched-key-repeats).
    let mut view = at_rest(catalog());
    view.held.insert(BOARD_KEY);
    view.take_back_live_input([], false);
    assert!(view.held.contains(&BOARD_KEY));
}

#[test]
fn a_button_still_down_at_the_close_keeps_the_cursor_whole_on_the_next_frames() {
    let mut view = in_flight();
    button_down(&mut view);
    dialog_opens(&mut view);
    view.take_back_live_input([], true);
    for _ in 0..40 {
        view.tick(TICK);
    }
    assert_eq!(faked_cursor_alpha(&view), Some(255), "held whole");
}

#[test]
fn a_button_let_go_under_the_dialog_fades_the_cursor_after_the_close() {
    let mut view = in_flight();
    button_down(&mut view);
    view.take_back_live_input([], false);
    fades_with_no_release(&mut view);
}
