//! What flight takes back as the last dialog over it closes, on
//! [`FakeFlight`] ([`FlightView::take_back_live_input`]): the unlatched
//! keys and the mouse button as they physically are, as `_HandlePlayer`
//! polls them; the latched keys latched when a `_SetAllKeyCmdFlags`
//! (@0x61e6d) site followed the dialog, and otherwise read by their
//! latches as they were when the dialog opened (the `dialog_latch`
//! module). Which dialogs hand back, and when, is the router's, pinned
//! in the `nova` crate's `app::screen::tests::live_keys`.

use std::collections::BTreeSet;

use super::super::dialog_latch::POLL_ORDER;
use super::super::map_wait::{OWN_LATCH, UNLATCHED};
use super::super::slot_block::is_block_command;
use super::controls::faked_cursor_alpha;
use super::faked_flight::{at_rest, hold, last_controls, ticked};
use super::faked_screen::in_flight;
use super::key_fades::{button_down, fades_with_no_release};
use super::*;
use crate::keymap::{Command, Context, FlightCommand as Fc};

/// The keys of the commands whose latches no site sets, Land's and
/// Board's ([`OWN_LATCH`]).
const OWN_LATCH_KEYS: [Key; 2] = [LAND_KEY, BOARD_KEY];

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
    assert!(view.latched_key(JUMP_KEY), "J latched");
}

#[test]
fn the_close_gives_back_an_unlatched_key_still_down_and_flight_thrusts_on_the_next_step() {
    let mut view = at_rest(catalog());
    hold(&mut view, Key::ArrowUp, true);
    dialog_opens(&mut view);
    back(&mut view, [Key::ArrowUp], [], false);
    ticked(&mut view, 1);
    assert!(last_controls(&view).thrust, "Up down at the close");
}

#[test]
fn the_close_gives_back_an_unlatched_key_pressed_under_the_dialog() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    back(&mut view, [Key::ArrowLeft], [], false);
    ticked(&mut view, 1);
    assert_eq!(last_controls(&view).turn, Turn::Left);
}

#[test]
fn the_close_drops_an_unlatched_key_no_longer_down() {
    let mut view = at_rest(catalog());
    // Left in `held` by a path that kept it: its release went elsewhere.
    hold(&mut view, Key::ArrowUp, true);
    back(&mut view, [], [], false);
    ticked(&mut view, 1);
    assert!(!last_controls(&view).thrust, "Up let go under the dialog");
}

#[test]
fn the_close_gives_back_no_latched_key() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    let replay = back(&mut view, [BOARD_KEY, JUMP_KEY, Key::ArrowUp], [], false);
    assert_eq!(replay, [], "nothing to replay");
    assert!(!view.holds_key(JUMP_KEY), "J waits for its release");
    assert!(view.holds_key(Key::ArrowUp));
    // B's own latch (0xdee18) is no site's: clear, B boards at the poll.
    assert!(view.holds_key(BOARD_KEY), "B held");
    let sent_before = sent(&view).len();
    view.held_keys_frame();
    assert!(
        sent(&view)[sent_before..].contains(&FlightCommand::Board),
        "B boards: {:?}",
        sent(&view)
    );
    assert!(!jumped_since(&view, sent_before), "J latched");
}

#[test]
fn the_close_keeps_a_latched_key_flight_still_holds() {
    // The latches are `_SetAllKeyCmdFlags`' to set, not the hand-back's
    // to clear (rdm:phase/play-feel-8/phase-4-latched-key-repeats). B is
    // read live, so B not physically down is let go, as Up is.
    let mut view = at_rest(catalog());
    view.hold_key(JUMP_KEY);
    view.hold_key(BOARD_KEY);
    view.hold_key(Key::ArrowUp);
    back(&mut view, [], [], false);
    assert!(view.holds_key(JUMP_KEY));
    assert!(!view.holds_key(BOARD_KEY), "B let go");
    assert!(!view.holds_key(Key::ArrowUp), "Up let go");
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
    assert!(!view.holds_key(JUMP_KEY), "J not held");
    view.input(&key(JUMP_KEY, false));
    view.input(&key(JUMP_KEY, true));
    assert!(jumped_since(&view, from), "let go and pressed afresh");
}

#[test]
fn every_latched_key_held_across_the_close_ignores_its_repeats() {
    let probe = in_flight();
    let latched: Vec<Key> = every_key()
        .filter(|&k| probe.key_latches(&key(k, true)))
        .collect();
    for k in latched {
        let mut view = in_flight();
        dialog_opens(&mut view);
        back(&mut view, [k], [], false);
        view.input(&held(k));
        assert!(!view.holds_key(k), "{k:?} not held");
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
    assert!(view.holds_key(JUMP_KEY));
}

#[test]
fn an_unlatched_key_repeated_after_the_close_stays_held() {
    let mut view = at_rest(catalog());
    dialog_opens(&mut view);
    back(&mut view, [Key::ArrowUp], [], false);
    view.input(&held(Key::ArrowUp));
    assert!(view.holds_key(Key::ArrowUp));
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
    assert!(view.holds_key(JUMP_KEY), "a first sight, as before");
}

#[test]
fn the_close_hands_back_a_latched_key_set_whole() {
    let mut view = in_flight();
    dialog_opens(&mut view);
    back(&mut view, [JUMP_KEY], [], false);
    // J let go under the next dialog, which hands back nothing of it.
    back(&mut view, [], [], false);
    view.input(&held(JUMP_KEY));
    assert!(view.holds_key(JUMP_KEY), "no longer latched");
}

#[test]
fn the_close_latches_only_the_latched_flight_keys_of_what_is_down() {
    // Only a latched key waits for its release; an unlatched one's
    // repeats must go on holding it wherever `held` lost it.
    let mut view = in_flight();
    dialog_opens(&mut view);
    back(&mut view, [JUMP_KEY, Key::ArrowUp, FIRE_KEY], [], false);
    assert_eq!(view.latched_keys(), BTreeSet::from([JUMP_KEY]));
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
        Key::MetaLeft,
    ];
    back(&mut view, down, [], false);
    for k in down {
        assert!(view.holds_key(k), "{k:?} reads down");
    }
    ticked(&mut view, 1);
    let step = view.sim.inputs().last().copied().expect("a step");
    assert!(step.weapon_safety, "S");
    assert!(step.eject, "Command-X");
    assert!(step.self_destruct, "Command--");
    assert_eq!(step.jettison, Some(true), "Command-K");
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
    let flight = [Context::Flight, Context::EscortMenu, Context::Play];
    let polled: BTreeSet<Command> = flight
        .iter()
        .flat_map(|context| context.commands())
        .filter(|command| !UNLATCHED.contains(command) && *command != Command::Flight(Fc::ZoomOut))
        .collect();
    let order: BTreeSet<Command> = POLL_ORDER.into_iter().collect();
    assert_eq!(order.len(), POLL_ORDER.len(), "no command twice");
    assert_eq!(
        order, polled,
        "the poll order covers every latched and press command"
    );
    // Where each key flight reads goes in the poll's order: by what a
    // press of it means, keys of one command in key order.
    let probe = in_flight();
    let mut placed: Vec<(usize, Command, Key)> = every_key()
        .filter_map(|k| {
            let command = probe
                .pressed(k)
                .or_else(|| probe.keys.bound_to(&key(k, true)).first().copied())?;
            let at = POLL_ORDER.iter().position(|&polled| polled == command)?;
            Some((at, command, k))
        })
        .collect();
    placed.sort_by_key(|&(at, _, k)| (at, k));
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    // Handed in reverse, as a set has no order.
    let mut down: Vec<Key> = every_key().collect();
    down.reverse();
    let replay = back(&mut view, down, [], false);
    // The slot block's commands and the cloak act at the poll; the press
    // keys after the first of them wait for it.
    let first_polled = POLL_ORDER
        .iter()
        .position(|&command| command == Command::Flight(Fc::StellarSlot1))
        .expect("the block");
    // B and L are held, read by their own latches at the poll.
    let at_poll = |command: Command| {
        is_block_command(command)
            || command == Command::Flight(Fc::EngageCloak)
            || OWN_LATCH.contains(&command)
    };
    let (now, kept): (Vec<Key>, Vec<Key>) = (
        placed
            .iter()
            .filter(|&&(at, command, _)| {
                (at < first_polled || at_poll(command)) && !OWN_LATCH.contains(&command)
            })
            .map(|&(.., k)| k)
            .collect(),
        placed
            .iter()
            .filter(|&&(at, command, _)| at >= first_polled && !at_poll(command))
            .map(|&(.., k)| k)
            .collect(),
    );
    let expected: Vec<Input> = now.into_iter().map(press).collect();
    assert_eq!(
        replay, expected,
        "only the latched keys, in the poll's order"
    );
    let kept: Vec<Input> = kept.into_iter().map(press).collect();
    assert_eq!(view.after_poll, kept, "the rest for the poll, in order");
    for k in OWN_LATCH_KEYS {
        assert!(view.holds_key(k), "{k:?} held");
    }
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
    assert_eq!(back(&mut view, [Key::ArrowUp], [], false), []);
    ticked(&mut view, 1);
    assert!(last_controls(&view).thrust);
}

#[test]
fn a_latched_key_flight_still_holds_comes_back_as_no_press() {
    let mut view = in_flight();
    view.hold_key(BOARD_KEY);
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
    assert!(!view.holds_key(JUMP_KEY), "latched at the next poll");
    assert!(view.latched_key(JUMP_KEY));
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
    assert!(view.latched_key(JUMP_KEY));
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
    // \` @0x69aa1 precedes the block, and R @0x6a9b7, which follows it,
    // has no slot key before it to wait on.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let replay = back(&mut view, [NAV_OFF_KEY, NEAREST_KEY], [], false);
    assert_eq!(replay, [press(NAV_OFF_KEY), press(NEAREST_KEY)]);
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

// A latched key held past a close reads down past its latch
// (`_QuickKeyCheck` @0xa0f5): its command waits on the latch, and a
// failed gate that clears the latch with the key down lets it act once
// the gate opens.

#[test]
fn a_latched_jump_key_held_into_a_disable_jumps_once_intact() {
    // J down while disabled clears `_jumpTryFlag` (@0x6b0f5-0x6b10d to
    // @0x6bada).
    let mut view = in_flight();
    dialog_opens(&mut view);
    let _ = back(&mut view, [JUMP_KEY], [], false);
    let from = sent(&view).len();
    view.held_keys_frame();
    assert!(!jumped_since(&view, from), "latched");
    super::faked_hud::changes(&mut view, |frame| {
        frame.ships[0].condition = nova_sim::Condition::Disabled;
    });
    view.held_keys_frame();
    assert!(!jumped_since(&view, from), "disabled");
    super::faked_hud::changes(&mut view, |frame| {
        frame.ships[0].condition = nova_sim::Condition::Intact;
    });
    view.held_keys_frame();
    assert!(jumped_since(&view, from), "{:?}", &sent(&view)[from..]);
}

#[test]
fn a_waiting_key_under_an_unlatching_dialog_reads_its_latch_set() {
    // J down at the poll before the dialog read its latch set, and no
    // pass saw it since: it reads down now, and waits.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [JUMP_KEY], [JUMP_KEY], false), []);
    let from = sent(&view).len();
    view.held_keys_frame();
    view.held_keys_frame();
    assert!(!jumped_since(&view, from), "{:?}", &sent(&view)[from..]);
    view.input(&key(JUMP_KEY, false));
    view.held_keys_frame();
    view.input(&key(JUMP_KEY, true));
    assert!(jumped_since(&view, from), "let go and pressed afresh");
}

// B's and L's latches, `_boardTryFlag` (0xdee18) and `_dockTryFlag`
// (0xdee34), are no `_SetAllKeyCmdFlags` site's: across every close each
// is held whenever physically down and read by its own latch on every
// pass.

#[test]
fn b_pressed_under_a_dialog_boards_at_its_close() {
    // The last pass before the dialog read B up (@0x6a9a3).
    for site in [true, false] {
        let mut view = in_flight();
        if site {
            dialog_opens(&mut view);
        } else {
            unlatching_dialog_opens(&mut view);
        }
        assert_eq!(back(&mut view, [BOARD_KEY], [], false), [], "no replay");
        let from = sent(&view).len();
        view.tick(TICK);
        assert!(
            sent(&view)[from..].contains(&FlightCommand::Board),
            "site {site}: {:?}",
            &sent(&view)[from..]
        );
    }
}

#[test]
fn b_held_from_before_a_dialog_waits_for_its_release() {
    // B boarded before the dialog: its latch set (@0x6504c).
    let mut view = in_flight();
    view.board_latch = true;
    dialog_opens(&mut view);
    let _ = back(&mut view, [BOARD_KEY], [BOARD_KEY], false);
    let from = sent(&view).len();
    view.tick(TICK);
    assert!(
        !sent(&view)[from..].contains(&FlightCommand::Board),
        "{:?}",
        &sent(&view)[from..]
    );
    view.input(&key(BOARD_KEY, false));
    view.tick(TICK);
    view.input(&key(BOARD_KEY, true));
    assert!(sent(&view)[from..].contains(&FlightCommand::Board));
}

#[test]
fn l_pressed_under_a_dialog_presses_the_land_key_at_its_close() {
    for site in [true, false] {
        let mut view = in_flight();
        if site {
            dialog_opens(&mut view);
        } else {
            unlatching_dialog_opens(&mut view);
        }
        assert_eq!(back(&mut view, [LAND_KEY], [], false), [], "no replay");
        let from = sent(&view).len();
        view.tick(TICK);
        assert!(
            sent(&view)[from..].contains(&FlightCommand::Land),
            "site {site}: {:?}",
            &sent(&view)[from..]
        );
    }
}

#[test]
fn own_latch_keys_never_come_back_as_presses() {
    for site in [true, false] {
        let mut view = in_flight();
        if site {
            dialog_opens(&mut view);
        } else {
            unlatching_dialog_opens(&mut view);
        }
        assert_eq!(back(&mut view, OWN_LATCH_KEYS, [], false), []);
        for k in OWN_LATCH_KEYS {
            assert!(view.holds_key(k), "{k:?} held");
            assert!(!view.latched_key(k), "{k:?} not latched");
        }
    }
}

#[test]
fn a_press_key_after_a_fresh_l_waits_for_the_poll() {
    // L's block (@0x6a8b8) reads it at the poll, before R (@0x6a9b7).
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    assert_eq!(back(&mut view, [LAND_KEY, NEAREST_KEY], [], false), []);
    assert_eq!(view.after_poll, [press(NEAREST_KEY)]);
    // With L's latch set, R comes back at once.
    let mut view = in_flight();
    super::faded_keys::requests(&mut view);
    view.press_land();
    unlatching_dialog_opens(&mut view);
    assert_eq!(
        back(&mut view, [LAND_KEY, NEAREST_KEY], [LAND_KEY], false),
        [press(NEAREST_KEY)]
    );
}

#[test]
fn l_held_from_before_a_dialog_waits_but_stays_held() {
    // L's latch set by its request before the dialog (@0x6a953): no
    // request, but L reads down for the pick, and the latch holds the
    // approach back.
    let mut view = in_flight();
    super::faded_keys::requests(&mut view);
    view.press_land();
    dialog_opens(&mut view);
    let _ = back(&mut view, [LAND_KEY], [LAND_KEY], false);
    let from = sent(&view).len();
    view.tick(TICK);
    assert!(
        !sent(&view)[from..].contains(&FlightCommand::Land),
        "{:?}",
        &sent(&view)[from..]
    );
    assert!(view.holds_key(LAND_KEY));
    let step = view.sim.inputs().last().copied().expect("a step");
    assert!(step.land_key, "L down");
    assert!(step.dock_try, "its latch set");
}

#[test]
fn every_waiting_key_under_an_unlatching_dialog_reads_its_latch_set() {
    // Each latched key down at the poll before the dialog had its latch
    // set by that poll, so it waits for its release.
    let acted = |view: &Faked, from: usize| {
        sent(view)[from..].iter().any(|command| {
            matches!(
                command,
                FlightCommand::NavModeKeys { .. }
                    | FlightCommand::SelectNextSystem
                    | FlightCommand::SelectStellar(_)
                    | FlightCommand::PressMissionInfo
                    | FlightCommand::SelectTarget(_)
            )
        })
    };
    for k in [
        HYPERSPACE_MODE_KEY,
        NAV_OFF_KEY,
        HYPER_SELECT_KEY,
        STELLAR_SLOT_KEYS[0][0],
        STELLAR_SLOT_KEYS[2][1],
        MISSION_INFO_KEY,
    ] {
        let mut view = in_flight();
        unlatching_dialog_opens(&mut view);
        assert_eq!(back(&mut view, [k], [k], false), [], "{k:?}");
        let from = sent(&view).len();
        view.tick(TICK);
        view.tick(TICK);
        assert!(!acted(&view, from), "{k:?}: {:?}", &sent(&view)[from..]);
    }
    // Tab's latch, which the button held keeps from clearing.
    let mut view = in_flight();
    unlatching_dialog_opens(&mut view);
    let _ = back(&mut view, [TARGET_KEY], [TARGET_KEY], true);
    view.input(&key(TARGET_KEY, false));
    let from = sent(&view).len();
    view.input(&key(TARGET_KEY, true));
    assert!(!acted(&view, from), "Tab: {:?}", &sent(&view)[from..]);
}

#[test]
fn i_held_across_a_sites_close_waits_for_its_release() {
    // `_SetAllKeyCmdFlags` sets `_missionInfoFlag` (0xdee00).
    let mut view = in_flight();
    dialog_opens(&mut view);
    let _ = back(&mut view, [MISSION_INFO_KEY], [], false);
    let from = sent(&view).len();
    view.tick(TICK);
    assert!(
        !sent(&view)[from..].contains(&FlightCommand::PressMissionInfo),
        "{:?}",
        &sent(&view)[from..]
    );
}

#[test]
fn a_waiting_key_sets_only_its_own_latch() {
    // R's latch is no view latch: a slot key flight still holds, its
    // latch clear, picks at the next poll.
    let slot = STELLAR_SLOT_KEYS[0][0];
    let mut view = in_flight();
    view.hold_key(slot);
    unlatching_dialog_opens(&mut view);
    view.hold_key(slot);
    let _ = back(&mut view, [slot, NEAREST_KEY], [NEAREST_KEY], false);
    let from = sent(&view).len();
    view.held_keys_frame();
    assert!(
        sent(&view)[from..]
            .iter()
            .any(|command| matches!(command, FlightCommand::SelectStellar(_))),
        "{:?}",
        &sent(&view)[from..]
    );
}
