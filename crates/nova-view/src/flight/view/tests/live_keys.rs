//! What flight takes back as the last dialog over it closes, on
//! [`FakeFlight`] ([`FlightView::take_back_live_input`]): the unlatched
//! keys and the mouse button as they physically are, as `_HandlePlayer`
//! polls them; the latched keys latched when a `_SetAllKeyCmdFlags`
//! (@0x61e6d) site followed the dialog, and otherwise read by their
//! latches as they were when the dialog opened (the `dialog_latch`
//! module). Which dialogs hand back, and when, is the router's, pinned
//! in the `nova` crate's `app::screen::tests::live_keys`.

use super::super::dialog_latch::POLL_ORDER;
use super::super::map_wait::UNLATCHED_KEYS;
use super::controls::faked_cursor_alpha;
use super::faked_flight::{at_rest, hold, last_controls, ticked};
use super::faked_screen::in_flight;
use super::key_fades::{button_down, fades_with_no_release};
use super::*;

/// A dialog a `_SetAllKeyCmdFlags` site follows opening over `view`'s
/// flight: it lets go of the keys, whose releases will go to the dialog,
/// and marks the close to latch.
pub(super) fn dialog_opens(view: &mut Faked) {
    view.let_go_into_dialog();
}

/// A dialog no site follows opening over `view`'s flight: the router's
/// letting go, which marks nothing.
fn unlatching_dialog_opens(view: &mut Faked) {
    view.let_go_of_input();
}

/// The last dialog over `view`'s flight closing with `down` down,
/// `before` down at flight's last poll before it opened and the
/// button `button`: what comes
/// back to replay.
pub(super) fn back(
    view: &mut Faked,
    down: impl IntoIterator<Item = Key>,
    before: impl IntoIterator<Item = Key>,
    button: bool,
) -> Vec<Input> {
    view.take_back_live_input(LiveKeys {
        down: down.into_iter().collect(),
        down_before: before.into_iter().collect(),
        button,
    })
}

/// A fresh press of `k`, as the hand-back gives one to replay.
fn press(k: Key) -> Input {
    key(k, true)
}

/// Asserts that the close of the dialog a site opened on `view` latches:
/// J down at it comes back as nothing and waits for its release.
pub(super) fn latches_its_close(view: &mut Faked) {
    assert_eq!(back(view, [JUMP_KEY], [], false), [], "nothing to replay");
    assert!(view.latched_down.contains(&JUMP_KEY), "J latched");
}

#[test]
fn the_close_gives_back_an_unlatched_key_still_down_and_flight_thrusts_on_the_next_step() {
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Up, true);
    dialog_opens(&mut view);
    back(&mut view, [Key::Up], [], false);
    ticked(&mut view, 1);
    assert!(last_controls(&view).thrust, "Up down at the close");
}

#[test]
fn the_close_gives_back_an_unlatched_key_pressed_under_the_dialog() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    back(&mut view, [Key::Left], [], false);
    ticked(&mut view, 1);
    assert_eq!(last_controls(&view).turn, Turn::Left);
}

#[test]
fn the_close_drops_an_unlatched_key_no_longer_down() {
    let mut view = at_rest(catalog());
    // Left in `held` by a path that kept it: its release went elsewhere.
    hold(&mut view, Key::Up, true);
    back(&mut view, [], [], false);
    ticked(&mut view, 1);
    assert!(!last_controls(&view).thrust, "Up let go under the dialog");
}

#[test]
fn the_close_gives_back_no_latched_key() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    back(&mut view, [BOARD_KEY, JUMP_KEY, Key::Up], [], false);
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
    back(&mut view, [], [], false);
    assert!(view.held.contains(&BOARD_KEY));
}

#[test]
fn a_button_still_down_at_the_close_keeps_the_cursor_whole_on_the_next_frames() {
    let mut view = in_flight();
    button_down(&mut view);
    dialog_opens(&mut view);
    back(&mut view, [], [], true);
    for _ in 0..40 {
        view.tick(TICK);
    }
    assert_eq!(faked_cursor_alpha(&view), Some(255), "held whole");
}

#[test]
fn a_button_let_go_under_the_dialog_fades_the_cursor_after_the_close() {
    let mut view = in_flight();
    button_down(&mut view);
    back(&mut view, [], [], false);
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
    back(&mut view, [JUMP_KEY], [], false);
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
        back(&mut view, [k], [], false);
        view.input(&held(k));
        assert!(!view.held.contains(&k), "{k:?} not held");
    }
}

#[test]
fn a_fresh_press_of_a_latched_key_after_the_close_acts_though_its_release_was_missed() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    back(&mut view, [JUMP_KEY], [], false);
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
    back(&mut view, [Key::Up], [], false);
    view.input(&held(Key::Up));
    assert!(view.held.contains(&Key::Up));
    ticked(&mut view, 1);
    assert!(last_controls(&view).thrust);
}

#[test]
fn a_dialog_opening_forgets_the_latched_keys() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    back(&mut view, [JUMP_KEY], [], false);
    // The next dialog, or a focus loss, takes the releases elsewhere.
    dialog_opens(&mut view);
    view.input(&held(JUMP_KEY));
    assert!(view.held.contains(&JUMP_KEY), "a first sight, as before");
}

#[test]
fn the_close_hands_back_a_latched_key_set_whole() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    back(&mut view, [JUMP_KEY], [], false);
    // J let go under the next dialog, which hands back nothing of it.
    back(&mut view, [], [], false);
    view.input(&held(JUMP_KEY));
    assert!(view.held.contains(&JUMP_KEY), "no longer latched");
}

#[test]
fn the_close_latches_only_the_latched_flight_keys_of_what_is_down() {
    // Only a latched key waits for its release; an unlatched one's
    // repeats must go on holding it wherever `held` lost it.
    let mut view = in_flight();
    dialog_opens(&mut view);
    back(&mut view, [JUMP_KEY, Key::Up, FIRE_KEY], [], false);
    assert_eq!(view.latched_down, HashSet::from([JUMP_KEY]));
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
    back(&mut view, down, [], false);
    for k in down {
        assert!(view.held.contains(&k), "{k:?} reads down");
    }
    ticked(&mut view, 1);
    let step = view.sim.inputs().last().copied().expect("a step");
    assert!(step.weapon_safety, "S");
    assert!(step.eject, "Command-X");
    assert!(step.self_destruct, "Command--");
}

// A dialog no `_SetAllKeyCmdFlags` follows (the mission handler's texts,
// the pod rescue's, an escort's defection, Preferences, About): no latch
// is touched and `_HandlePlayer` doesn't run under it, so each latched
// key's latch is what it was as the dialog opened.

#[test]
fn a_latched_key_pressed_under_a_dialog_no_latch_follows_comes_back_as_a_press() {
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back(&mut view, [JUMP_KEY], [], false);
    assert_eq!(replay, [press(JUMP_KEY)]);
    let from = sent(&view).len();
    for input in &replay {
        view.input(input);
    }
    view.held_keys_frame();
    assert!(jumped_since(&view, from), "{:?}", &sent(&view)[from..]);
}

#[test]
fn every_latched_key_pressed_under_an_unlatching_dialog_comes_back_in_handle_players_poll_order() {
    let latched: Vec<Key> = every_key()
        .filter(|k| !UNLATCHED_KEYS.contains(k))
        .collect();
    let mut order = POLL_ORDER.to_vec();
    order.sort_by_key(|k| format!("{k:?}"));
    order.dedup();
    assert_eq!(order.len(), POLL_ORDER.len(), "no key twice");
    let mut sorted = latched.clone();
    sorted.sort_by_key(|k| format!("{k:?}"));
    sorted.dedup();
    assert_eq!(order, sorted, "the poll order covers every latched key");
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    // Handed in reverse, as a set has no order.
    let mut down: Vec<Key> = every_key().collect();
    down.reverse();
    let replay = back(&mut view, down, [], false);
    // The slot block's keys and the cloak act at the poll; the press keys
    // after the first of them wait for it.
    let first_polled = POLL_ORDER
        .iter()
        .position(|&k| k == STELLAR_SLOT_KEYS[0][0])
        .expect("the block");
    let at_poll = |k: Key| super::super::slot_block::is_block_key(k) || k == CLOAK_KEY;
    let (now, kept): (Vec<Key>, Vec<Key>) = (
        POLL_ORDER
            .into_iter()
            .enumerate()
            .filter(|&(at, k)| at < first_polled || at_poll(k))
            .map(|(_, k)| k)
            .collect(),
        POLL_ORDER
            .into_iter()
            .enumerate()
            .filter(|&(at, k)| at >= first_polled && !at_poll(k))
            .map(|(_, k)| k)
            .collect(),
    );
    let expected: Vec<Input> = now.into_iter().map(press).collect();
    assert_eq!(
        replay, expected,
        "only the latched keys, in the poll's order"
    );
    assert_eq!(view.after_poll, kept, "the rest for the poll, in order");
}

#[test]
fn a_latched_key_down_at_the_poll_before_an_unlatching_dialog_waits_for_its_release() {
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [JUMP_KEY], [JUMP_KEY], false), []);
    let from = sent(&view).len();
    repeats(&mut view, JUMP_KEY);
    assert!(!jumped_since(&view, from), "{:?}", &sent(&view)[from..]);
    view.input(&key(JUMP_KEY, false));
    view.input(&key(JUMP_KEY, true));
    assert!(jumped_since(&view, from), "let go and pressed afresh");
}

#[test]
fn a_latched_key_let_go_under_an_unlatching_dialog_comes_back_as_nothing() {
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [], [JUMP_KEY], false), []);
    assert!(view.latched_down.is_empty());
}

#[test]
fn an_unlatched_key_pressed_under_an_unlatching_dialog_is_held_not_replayed() {
    let mut view = at_rest(catalog());
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [Key::Up], [], false), []);
    ticked(&mut view, 1);
    assert!(last_controls(&view).thrust);
}

#[test]
fn a_latched_key_flight_still_holds_comes_back_as_no_press() {
    let mut view = in_flight();
    view.held.insert(BOARD_KEY);
    assert_eq!(back(&mut view, [BOARD_KEY], [], false), []);
}

#[test]
fn a_site_anywhere_under_the_dialogs_latches_every_key_at_the_close() {
    // Preferences over an offer: `_SetAllKeyCmdFlags` follows the offer
    // (@0x622ac), and no `_HandlePlayer` pass runs between them.
    let mut view = in_flight();
    dialog_opens(&mut view);
    unlatching_dialog_opens(&mut view);
    latches_its_close(&mut view);
}

#[test]
fn the_hand_back_takes_the_sites_mark() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    let _ = back(&mut view, [], [], false);
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [JUMP_KEY], [], false), [press(JUMP_KEY)]);
}

#[test]
fn a_site_that_opened_no_dialog_latches_the_keys_held_at_the_next_pass_and_no_later_dialog() {
    let mut view = in_flight();
    view.input(&key(JUMP_KEY, true));
    view.latch_at_close();
    view.tick(TICK);
    assert!(!view.held.contains(&JUMP_KEY), "latched at the next poll");
    assert!(view.latched_down.contains(&JUMP_KEY));
    // A later dialog no site follows reads 1 by its clear latch.
    let slot = STELLAR_SLOT_KEYS[0][0];
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [slot], [], false), [press(slot)]);
}

#[test]
fn a_dialog_closing_over_flights_map_leaves_the_latched_keys_to_the_maps_latch() {
    // Preferences over M's map: flight doesn't poll under the map, whose
    // close runs `_SetAllKeyCmdFlags` (@0x6a898).
    let mut view = in_flight();
    view.open_map();
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [JUMP_KEY, Key::Enter], [], false), []);
    assert!(view.latched_down.contains(&JUMP_KEY));
}

#[test]
fn the_fixtures_site_mark_latches_the_close_as_a_site_does() {
    // `FlightView::mark_site`, for the router's tests that open a site's
    // dialog themselves.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    view.mark_site();
    latches_its_close(&mut view);
}

// The slot block acts at the poll, not on its press, so a press key
// later in `_HandlePlayer`'s order waits for the next poll: the block
// @0x69d3f picks before P @0x702cb opens its dialog, whose
// `_SetAllKeyCmdFlags` (@0x70393) then latches the rest.

#[test]
fn a_press_key_after_a_slot_key_under_an_unlatching_dialog_acts_after_the_blocks_poll() {
    let slot = STELLAR_SLOT_KEYS[0][0];
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back(&mut view, [PLAYER_INFO_KEY, slot], [], false);
    assert_eq!(replay, [press(slot)], "P waits for the poll");
    for input in &replay {
        view.input(input);
    }
    let from = sent(&view).len();
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    view.tick(TICK);
    let sent_now = &sent(&view)[from..];
    let picked = sent_now
        .iter()
        .position(|command| matches!(command, FlightCommand::SelectStellar(_)));
    let asked = sent_now
        .iter()
        .position(|command| *command == FlightCommand::PressPlayerInfo);
    assert!(
        matches!((picked, asked), (Some(p), Some(a)) if p < a),
        "the pick, then P: {sent_now:?}"
    );
    assert!(view.take_player_info_request(), "P's dialog asked for");
}

#[test]
fn a_press_key_kept_for_the_poll_and_let_go_before_it_does_nothing() {
    let slot = STELLAR_SLOT_KEYS[0][0];
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back(&mut view, [slot, PLAYER_INFO_KEY], [], false);
    for input in &replay {
        view.input(input);
    }
    view.input(&key(PLAYER_INFO_KEY, false));
    let from = sent(&view).len();
    view.tick(TICK);
    assert!(
        !sent(&view)[from..].contains(&FlightCommand::PressPlayerInfo),
        "{:?}",
        &sent(&view)[from..]
    );
}

#[test]
fn a_press_key_before_any_slot_key_comes_back_at_once() {
    // \` @0x69aa1 precedes the block, and B @0x6a98b, which follows it,
    // has no slot key before it to wait on.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back(&mut view, [NAV_OFF_KEY, BOARD_KEY], [], false);
    assert_eq!(replay, [press(NAV_OFF_KEY), press(BOARD_KEY)]);
}

#[test]
fn a_kept_press_is_dropped_when_flight_lets_go_before_the_poll() {
    let slot = STELLAR_SLOT_KEYS[0][0];
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let _ = back(&mut view, [slot, HAIL_KEY], [], false);
    unlatching_dialog_opens(&mut view);
    let from = sent(&view).len();
    view.tick(TICK);
    assert!(
        !sent(&view)[from..]
            .iter()
            .any(|command| matches!(command, FlightCommand::Hail { .. })),
        "{:?}",
        &sent(&view)[from..]
    );
}

/// The commands `view` sent from `from` on, but those the message line
/// rides along with.
fn sent_from(view: &Faked, from: usize) -> Vec<FlightCommand> {
    sent(view)[from..]
        .iter()
        .filter(|command| {
            !matches!(
                command,
                FlightCommand::CommShown(_)
                    | FlightCommand::CommAcknowledged
                    | FlightCommand::CommTicked
            )
        })
        .cloned()
        .collect()
}

#[test]
fn a_kept_press_that_opens_a_dialog_ends_the_poll_and_drops_the_rest() {
    // The block picks (@0x69d3f), B boards into the plunder dialog
    // (@0x6a98b), whose `_SetAllKeyCmdFlags` (@0x659f1) latches P.
    let slot = STELLAR_SLOT_KEYS[0][0];
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back(&mut view, [slot, BOARD_KEY, PLAYER_INFO_KEY], [], false);
    for input in &replay {
        view.input(input);
    }
    let from = sent(&view).len();
    view.sim.queue_answer(Answer::Done);
    super::faked_board::opened(&mut view);
    view.tick(TICK);
    let sent_now = sent_from(&view, from);
    let picked = sent_now
        .iter()
        .position(|command| matches!(command, FlightCommand::SelectStellar(_)));
    let boarded = sent_now
        .iter()
        .position(|command| *command == FlightCommand::Board);
    assert!(
        matches!((picked, boarded), (Some(p), Some(b)) if p < b),
        "the pick, then B: {sent_now:?}"
    );
    assert!(
        !sent_now.contains(&FlightCommand::PressPlayerInfo),
        "P dropped: {sent_now:?}"
    );
    assert!(view.after_poll.is_empty());
    assert!(view.take_boarding().is_some(), "the plunder dialog");
}

#[test]
fn a_kept_p_that_opens_its_dialog_leaves_y_unread() {
    // P's dialog runs `_SetAllKeyCmdFlags` (@0x70393) before Y's read
    // (@0x703e9).
    let slot = STELLAR_SLOT_KEYS[0][0];
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back(&mut view, [slot, PLAYER_INFO_KEY, HAIL_KEY], [], false);
    for input in &replay {
        view.input(input);
    }
    let from = sent(&view).len();
    view.sim.queue_answer(Answer::Done);
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    view.tick(TICK);
    let sent_now = sent_from(&view, from);
    assert!(
        sent_now.contains(&FlightCommand::PressPlayerInfo),
        "{sent_now:?}"
    );
    assert!(
        !sent_now
            .iter()
            .any(|command| matches!(command, FlightCommand::Hail { .. })),
        "Y unread: {sent_now:?}"
    );
    assert!(view.take_player_info_request());
}
