//! What fades the cursor, on [`FakeFlight`]: each key behind its gate
//! (`flight/view/key_fade.rs`) and when the view reads the gates. The gates
//! themselves are nova-sim's (`nova_sim::key_fade`).

use nova_sim::flight_frame::{FadeKey, KeyFades, StepReport};

use super::controls::{NOWHERE, faked_cursor_alpha};
use super::faked_flight::{at_rest, hold, ticked};
use super::faked_hud::changes;
use super::faked_screen::{braking, in_flight};
use super::*;
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;

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
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
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
        Key::Alt,
        Key::Shift,
        Key::Command,
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
        Key::Char('q'),
    ] {
        assert_eq!(alpha_holding(KeyFades::EVERY, &[k]), WHOLE, "{k:?}");
    }
}

#[test]
fn the_chords_follow_their_gates_with_command_down() {
    follows(FadeKey::Eject, &[Key::Command, EJECT_KEY]);
    follows(FadeKey::SelfDestruct, &[Key::Command, SELF_DESTRUCT_KEY]);
    assert_eq!(
        alpha_holding(KeyFades::EVERY, &[Key::Command, JETTISON_KEY]),
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
    assert_eq!(alpha_holding(unbidden, &[Key::Up]), TWO, "Up held");
}

#[test]
fn with_nothing_fading_no_key_fades_a_slot_key_included() {
    assert_eq!(
        alpha_holding(KeyFades::NONE, &[Key::Up, STELLAR_SLOT_KEYS[1][1]]),
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
    view.input(&key(Key::Up, true));
    let refused = frame_of(&view, KeyFades::EVERY.refusing(FadeKey::Helm));
    view.sim
        .queue_report_showing(StepReport::default(), refused);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "the pass began allowed");
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "this one began refused");

    let mut view = in_flight();
    changes(&mut view, |frame| {
        frame.key_fades = KeyFades::EVERY.refusing(FadeKey::Helm);
    });
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&key(Key::Up, true));
    let allowed = frame_of(&view, KeyFades::EVERY);
    view.sim
        .queue_report_showing(StepReport::default(), allowed);
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), WHOLE, "the pass began refused");
    view.tick(TICK);
    assert_eq!(faked_cursor_alpha(&view), ONE, "this one began allowed");
}

#[test]
fn a_key_typed_into_the_map_latches_no_fade_for_after_it_closes() {
    let mut view = in_flight();
    view.input(&Input::PointerMoved(NOWHERE));
    view.open_map();
    tap(&mut view, NEAREST_KEY);
    view.close_map();
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
