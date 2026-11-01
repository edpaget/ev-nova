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

// A latched key held across the close: `_SetAllKeyCmdFlags` latches it,
// and `_HandlePlayer`, polling `GetKeys` (`_QuickKeyCheckHandler`
// @0x43e01) and dropping queued keys (`_IterateInGameEventLoop` @0x56a6b,
// @0x56cec), never sees its OS repeats: it acts again only once let go.

/// Whether `view` sent [`FlightCommand::Jump`] since `from` of its
/// commands.
fn jumped_since(view: &Faked, from: usize) -> bool {
    sent(view)[from..].contains(&FlightCommand::Jump)
}

/// Repeats of `k` on `view`, a tick after each.
fn repeats(view: &mut Faked, k: Key) {
    for _ in 0..3 {
        view.input(&held(k));
        view.tick(TICK);
    }
}

#[test]
fn a_jump_key_held_across_a_dialogs_close_ignores_its_repeats_until_let_go() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    view.take_back_live_input([JUMP_KEY], false);
    let from = sent(&view).len();
    repeats(&mut view, JUMP_KEY);
    assert!(
        !jumped_since(&view, from),
        "no jump: {:?}",
        &sent(&view)[from..]
    );
    assert!(!view.held.contains(&JUMP_KEY), "J not held");
    view.input(&key(JUMP_KEY, false));
    view.input(&key(JUMP_KEY, true));
    assert!(jumped_since(&view, from), "let go and pressed afresh");
}

#[test]
fn every_latched_key_held_across_the_close_ignores_its_repeats() {
    let latched = FLIGHT_KEYS
        .into_iter()
        .filter(|k| !super::super::map_wait::UNLATCHED_KEYS.contains(k));
    for k in latched {
        let mut view = in_flight();
        dialog_opens(&mut view);
        view.take_back_live_input([k], false);
        view.input(&held(k));
        assert!(!view.held.contains(&k), "{k:?} not held");
    }
}

#[test]
fn a_fresh_press_of_a_latched_key_after_the_close_acts_though_its_release_was_missed() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    view.take_back_live_input([JUMP_KEY], false);
    let from = sent(&view).len();
    // Its release went elsewhere: the OS sends a fresh press only after
    // one, so the latch has cleared.
    view.input(&key(JUMP_KEY, true));
    assert!(jumped_since(&view, from));
    assert!(view.held.contains(&JUMP_KEY));
}

#[test]
fn an_unlatched_key_repeated_after_the_close_stays_held() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    view.take_back_live_input([Key::Up], false);
    view.input(&held(Key::Up));
    assert!(view.held.contains(&Key::Up));
    ticked(&mut view, 1);
    assert!(last_controls(&view).thrust);
}

#[test]
fn a_dialog_opening_forgets_the_latched_keys() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    view.take_back_live_input([JUMP_KEY], false);
    // The next dialog, or a focus loss, takes the releases elsewhere.
    dialog_opens(&mut view);
    view.input(&held(JUMP_KEY));
    assert!(view.held.contains(&JUMP_KEY), "a first sight, as before");
}

#[test]
fn the_close_hands_back_a_latched_key_set_whole() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    view.take_back_live_input([JUMP_KEY], false);
    // J let go under the next dialog, which hands back nothing of it.
    view.take_back_live_input([], false);
    view.input(&held(JUMP_KEY));
    assert!(view.held.contains(&JUMP_KEY), "no longer latched");
}

#[test]
fn the_keys_no_latch_guards_read_down_after_the_close() {
    // Weapon Safety reads every pass held (@0x6adc2-0x6addf), the eject
    // and self-destruct chords run with no latch (@0x6f29e-0x6f2c1,
    // @0x6fdb4-0x6fdfc), and Command-K jettisons on every pass held
    // (@0x6fd54-0x6fda1).
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    let down = [
        DESELECT_KEY,
        EJECT_KEY,
        SELF_DESTRUCT_KEY,
        JETTISON_KEY,
        Key::Command,
    ];
    view.take_back_live_input(down, false);
    for k in down {
        assert!(view.held.contains(&k), "{k:?} reads down");
    }
    ticked(&mut view, 1);
    let step = view.sim.inputs().last().copied().expect("a step");
    assert!(step.weapon_safety, "S");
    assert!(step.eject, "Command-X");
    assert!(step.self_destruct, "Command--");
}
