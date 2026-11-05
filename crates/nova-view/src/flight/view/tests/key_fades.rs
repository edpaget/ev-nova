//! What fades the cursor, on [`FakeFlight`]: each key behind its gate
//! (`flight/view/key_fade.rs`) and when the view reads the gates. The gates
//! themselves are nova-sim's (`nova_sim::key_fade`).

use nova_sim::flight_frame::{FadeKey, KeyFades, StepReport};

use super::controls::{NOWHERE, faked_cursor_alpha};
use super::faked_flight::{at_rest, hold, ticked};
use super::faked_hud::changes;
use super::faked_screen::{braking, in_flight};
use super::*;
use crate::keymap::test_keys::MISSION_INFO_KEY;
use crate::keymap::test_keys::PLAYER_INFO_KEY;

/// The cursor whole.
const WHOLE: Option<u8> = Some(255);
/// One level gone.
const ONE: Option<u8> = Some(247);
/// Two levels gone.
const TWO: Option<u8> = Some(239);

/// A view at rest over a flight whose frame says `fades`, the cursor
/// shown.
fn showing(fades: KeyFades) -> Faked {
    let mut view = at_rest(catalog());
    changes(&mut view, |frame| frame.key_fades = fades);
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    view
}

/// The cursor's alpha two ticks after `keys` go down in turn under `fades`.
fn alpha_holding(fades: KeyFades, keys: &[Key]) -> Option<u8> {
    let mut view = showing(fades);
    for &k in keys {
        hold(&mut view, k, true);
    }
    ticked(&mut view, 2);
    faked_cursor_alpha(&view)
}

/// A key held fades under `fades` but not with `gate` refused.
fn follows(gate: FadeKey, keys: &[Key]) {
    assert_eq!(alpha_holding(KeyFades::EVERY, keys), TWO, "{keys:?}");
    assert_eq!(
        alpha_holding(KeyFades::EVERY.refusing(gate), keys),
        WHOLE,
        "{keys:?} refused"
    );
}

#[test]
fn the_helm_keys_follow_their_gate_press_included() {
    for k in [
        Key::ArrowUp,
        Key::ArrowDown,
        Key::ArrowLeft,
        Key::ArrowRight,
        AFTERBURNER_KEY,
        FIRE_KEY,
        JUMP_KEY,
        CLOAK_KEY,
    ] {
        follows(FadeKey::Helm, &[k]);
    }
}

#[test]
fn ctrl_s_l_and_i_each_follow_their_gate() {
    follows(FadeKey::Secondary, &[SECONDARY_KEY]);
    follows(FadeKey::WeaponSafety, &[DESELECT_KEY]);
    follows(FadeKey::Land, &[LAND_KEY]);
    follows(FadeKey::MissionInfo, &[MISSION_INFO_KEY]);
}

#[test]
fn the_keys_handle_player_writes_no_byte_for_never_fade() {
    for k in [
        Key::AltLeft,
        Key::ShiftLeft,
        Key::MetaLeft,
        EJECT_KEY,
        SELF_DESTRUCT_KEY,
        JETTISON_KEY,
        MENU_KEY,
        ESCORT_ATTACK_KEY,
        ESCORT_DEFEND_KEY,
        ESCORT_HOLD_KEY,
        ESCORT_RECALL_KEY,
        FRAME_RATE_KEY,
        ZOOM_IN_KEY,
        Key::KeyQ,
    ] {
        assert_eq!(alpha_holding(KeyFades::EVERY, &[k]), WHOLE, "{k:?}");
    }
}

#[test]
fn the_chords_follow_their_gates_with_command_down() {
    follows(FadeKey::Eject, &[Key::MetaLeft, EJECT_KEY]);
    follows(FadeKey::SelfDestruct, &[Key::MetaLeft, SELF_DESTRUCT_KEY]);
    assert_eq!(
        alpha_holding(KeyFades::EVERY, &[Key::MetaLeft, JETTISON_KEY]),
        TWO,
        "Command-K: no gate"
    );
}

#[test]
fn the_free_keys_fade_with_every_other_gate_refused() {
    let only_free = FadeKey::ALL
        .into_iter()
        .filter(|&gate| gate != FadeKey::Free)
        .fold(KeyFades::EVERY, KeyFades::refusing);
    for k in [
        TARGET_KEY,
        NEAREST_KEY,
        HYPER_SELECT_KEY,
        SELECT_KEY,
        BOARD_KEY,
        HAIL_KEY,
        MAP_KEY,
        PLAYER_INFO_KEY,
        MENU_CLOSE_KEY,
        HYPERSPACE_MODE_KEY,
        NAV_OFF_KEY,
        FACE_TARGET_KEY,
        STELLAR_SLOT_KEYS[1][0],
    ] {
        assert_eq!(alpha_holding(only_free, &[k]), TWO, "{k:?}");
        assert_eq!(
            alpha_holding(KeyFades::EVERY.refusing(FadeKey::Free), &[k]),
            WHOLE,
            "{k:?} refused"
        );
    }
}

#[test]
fn the_keys_flight_now_tracks_fade_on_each_tick_they_are_held() {
    for k in [
        TARGET_KEY,
        NEAREST_KEY,
        HAIL_KEY,
        MENU_CLOSE_KEY,
        MAP_KEY,
        PLAYER_INFO_KEY,
    ] {
        let mut view = in_flight();
        view.change_sim(|sim| {
            sim.refuse(PlayerCommand::Map);
            sim.refuse(PlayerCommand::Hail);
        });
        view.sim.queue_answer(Answer::PlayerInfoPressed(false));
        view.input(&Input::PointerMoved(NOWHERE));
        view.input(&key(k, true));
        view.tick(TICK);
        view.tick(TICK);
        assert_eq!(faked_cursor_alpha(&view), TWO, "{k:?}");
    }
}

#[test]
fn an_unbidden_frame_fades_a_level_a_step_with_nothing_held() {
    let mut view = showing(KeyFades::NONE.with_unbidden());
    ticked(&mut view, 2);
    assert_eq!(faked_cursor_alpha(&view), TWO);
}

#[test]
fn a_key_held_on_an_unbidden_frame_adds_no_fade_of_its_own() {
    // Up held in the pre-jump: the pass sets the one byte either way.
    let unbidden = KeyFades::EVERY.with_unbidden();
    assert_eq!(alpha_holding(unbidden, &[]), TWO, "nothing held");
    assert_eq!(alpha_holding(unbidden, &[Key::ArrowUp]), TWO, "Up held");
}

#[test]
fn with_nothing_fading_no_key_fades_a_slot_key_included() {
    assert_eq!(
        alpha_holding(KeyFades::NONE, &[Key::ArrowUp, STELLAR_SLOT_KEYS[1][1]]),
        WHOLE
    );
}

/// The frame `view`'s flight shows with these `fades`.
fn frame_of(view: &Faked, fades: KeyFades) -> FlightFrame {
    FlightFrame {
        key_fades: fades,
        ..view.frame.clone()
    }
}

#[test]
fn a_tick_reads_the_gates_at_the_head_of_its_pass_so_a_step_changes_the_next() {
    let mut view = in_flight();
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&key(Key::ArrowUp, true));
    // Spends the press's latch, which would fade the next tick by itself.
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "the press");
    let refused = frame_of(&view, KeyFades::EVERY.refusing(FadeKey::Helm));
    view.sim
        .queue_report_showing(StepReport::default(), refused);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), TWO, "the pass began allowed");
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), TWO, "this one began refused");

    let mut view = in_flight();
    changes(&mut view, |frame| {
        frame.key_fades = KeyFades::EVERY.refusing(FadeKey::Helm);
    });
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&key(Key::ArrowUp, true));
    let allowed = frame_of(&view, KeyFades::EVERY);
    view.sim
        .queue_report_showing(StepReport::default(), allowed);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), WHOLE, "the pass began refused");
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "this one began allowed");
}

#[test]
fn an_arrival_frame_keeps_its_jumps_fade_with_nothing_held() {
    let mut view = in_flight();
    changes(&mut view, |frame| {
        frame.key_fades = KeyFades::EVERY.with_unbidden();
    });
    view.input(&Input::PointerMoved(NOWHERE));
    let arrived = frame_of(&view, KeyFades::EVERY);
    view.sim
        .queue_report_showing(StepReport::default(), arrived);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "the pass began in the jump");
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "this one began out of it");
}

/// `view` with the pointer on it and the left button pressed.
pub(super) fn button_down(view: &mut Faked) {
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: NOWHERE,
    });
}

/// Past a second of ticks, the button's hold on `view` gone: the cursor
/// has faded past the one level a key's press costs. No release is sent,
/// as the dialog that opened over flight took it.
pub(super) fn fades_with_no_release(view: &mut Faked) {
    for _ in 0..40 {
        view.tick(TICK);
    }
    let alpha = faked_cursor_alpha(view).expect("the cursor drawn");
    assert!(Some(alpha) < ONE, "still held whole: {alpha}");
}

#[test]
fn p_opening_its_screen_lets_go_of_the_button() {
    let mut view = in_flight();
    button_down(&mut view);
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    view.input(&key(PLAYER_INFO_KEY, true));
    fades_with_no_release(&mut view);
}

#[test]
fn losing_the_keys_lets_go_of_the_button_as_its_release_may_never_come() {
    // Focus lost, with `_Button` read live by the original (@0x3803b):
    // ours counts it as the release, as it does the keys.
    let mut view = in_flight();
    button_down(&mut view);
    view.release_keys();
    fades_with_no_release(&mut view);
}

#[test]
fn a_key_that_opens_nothing_keeps_the_button_held() {
    let mut view = in_flight();
    button_down(&mut view);
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.input(&key(PLAYER_INFO_KEY, true));
    view.input(&key(PLAYER_INFO_KEY, false));
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "the press's level");
    for _ in 0..40 {
        view.tick(TICK);
    }
    assert_eq!(faked_cursor_alpha(&view), WHOLE, "restamped");
}

/// The pointer moves once the map has closed: the map hid the cursor with
/// a zero fade (`_HideCursorSprite` before `_DoSystemMap`, @0x6a826), so
/// only a move (`_HandleMouse` @0x3801c-0x3806d) shows it whole again, for
/// what follows to fade.
fn moved_after_the_map(view: &mut Faked) {
    view.input(&Input::PointerMoved(Point::new(60.0, 420.0)));
}

#[test]
fn a_map_opened_with_the_button_down_still_takes_its_release_for_the_cursor() {
    let mut view = in_flight();
    button_down(&mut view);
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    view.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at: NOWHERE,
    });
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open());
    moved_after_the_map(&mut view);
    fades_with_no_release(&mut view);
}

#[test]
fn a_key_typed_into_the_map_latches_no_fade_for_after_it_closes() {
    let mut view = in_flight();
    view.input(&Input::PointerMoved(NOWHERE));
    view.open_map();
    tap(&mut view, NEAREST_KEY);
    view.close_map();
    moved_after_the_map(&mut view);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), WHOLE);
}

#[test]
fn the_m_that_opens_the_map_fades_once_on_the_first_step_after_it_closes() {
    let mut view = in_flight();
    view.input(&Input::PointerMoved(NOWHERE));
    tap(&mut view, MAP_KEY);
    assert!(view.map_open(), "opened");
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open(), "closed");
    moved_after_the_map(&mut view);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "once");
}

#[test]
fn p_refused_stays_held_and_fades_until_its_release() {
    let mut view = in_flight();
    view.input(&Input::PointerMoved(NOWHERE));
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.input(&key(PLAYER_INFO_KEY, true));
    assert!(view.holds_keys(), "refused: still down");
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), TWO);
    view.input(&key(PLAYER_INFO_KEY, false));
    assert!(!view.holds_keys(), "its release");
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), TWO, "stopped");
}

#[test]
fn p_opening_its_screen_or_m_the_map_leaves_no_key_held() {
    let mut view = in_flight();
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    view.input(&key(PLAYER_INFO_KEY, true));
    assert!(!view.holds_keys(), "the screen takes its release");

    let mut view = in_flight();
    view.input(&key(MAP_KEY, true));
    assert!(view.map_open());
    assert!(!view.holds_keys(), "the map takes its release");
}

#[test]
fn p_refused_in_a_jump_fades_past_the_arrival_until_its_release() {
    let mut view = braking();
    view.input(&Input::PointerMoved(NOWHERE));
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.input(&key(PLAYER_INFO_KEY, true));
    assert!(view.holds_keys(), "held through the jump");
    changes(&mut view, |frame| {
        frame.jump_under_way = false;
        frame.key_fades = KeyFades::EVERY;
    });
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), TWO, "past the jump");
    view.input(&key(PLAYER_INFO_KEY, false));
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), TWO, "its release stops it");
}

#[test]
fn p_refused_in_a_jump_with_the_map_open_is_left_to_the_map() {
    let mut view = braking();
    view.open_map();
    view.input(&key(PLAYER_INFO_KEY, true));
    assert!(!view.holds_keys());
}

// A latched key held past a site's close: `_QuickKeyCheck` (@0xa0f5)
// still reads it down, and every latched block sets `_keyPressedThisFrame`
// (*0x3b5550) before testing its latch, so it fades the cursor while its
// command waits for its release.

/// The cursor's alpha two ticks after a site's close hands back `k` held,
/// under `fades`.
fn alpha_latched(fades: KeyFades, k: Key) -> Option<u8> {
    let mut view = showing(fades);
    super::live_keys::dialog_opens(&mut view);
    let _ = super::live_keys::back(&mut view, [k], [], false);
    ticked(&mut view, 2);
    faked_cursor_alpha(&view)
}

#[test]
fn a_jump_key_held_across_an_offers_close_fades_the_cursor_without_jumping_until_pressed_again() {
    // J's block sets the byte (@0x6b113) before its latch (@0x6b86d), which
    // the offer's `_SetAllKeyCmdFlags` (@0x622ac) set.
    let mut view = showing(KeyFades::EVERY);
    view.sim
        .queue_answer(Answer::Hailed(nova_sim::Hailed::Offer(VenueOffer::Prompt(
            super::faked_hail::prompt("Help?"),
        ))));
    view.hail();
    assert!(view.take_offer().is_some(), "the offer");
    let _ = super::live_keys::back(&mut view, [JUMP_KEY], [], false);
    let from = sent(&view).len();
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), TWO, "faded by J");
    assert!(
        !sent(&view)[from..].contains(&FlightCommand::Jump),
        "{:?}",
        &sent(&view)[from..]
    );
    view.input(&key(JUMP_KEY, false));
    view.input(&key(JUMP_KEY, true));
    assert!(sent(&view)[from..].contains(&FlightCommand::Jump));
}

#[test]
fn every_latched_key_held_across_a_close_fades_by_its_gate() {
    let gates = FadeKey::ALL
        .into_iter()
        .map(|gate| KeyFades::EVERY.refusing(gate))
        .chain([KeyFades::EVERY]);
    let fades: Vec<KeyFades> = gates.collect();
    let mut checked = 0;
    for k in every_key() {
        let mut view = showing(KeyFades::EVERY);
        super::live_keys::dialog_opens(&mut view);
        let _ = super::live_keys::back(&mut view, [k], [], false);
        if !view.latched_key(k) {
            continue;
        }
        checked += 1;
        for &fade in &fades {
            assert_eq!(
                alpha_latched(fade, k),
                alpha_holding(fade, &[k]),
                "{k:?} under {fade:?}"
            );
        }
    }
    assert!(checked > 10, "the latched keys: {checked}");
    assert_eq!(alpha_latched(KeyFades::EVERY, JUMP_KEY), TWO);
    assert_eq!(
        alpha_latched(KeyFades::EVERY, STELLAR_SLOT_KEYS[1][0]),
        TWO,
        "a slot key, polled by its block"
    );
}
