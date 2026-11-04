//! Wiring: the spaceport's closes wait on the keys whoever flies the ship
//! says are down ([`SpaceportView::set_close_keys`]); the waits' rules are
//! pinned in the `ui::key_wait`, player info and offer `close` modules.

use super::*;
use crate::spaceport::view::CloseKeys;

fn released(key: Key) -> Input {
    Input::Key {
        key,
        pressed: false,
        repeat: false,
    }
}

const I_DOWN: CloseKeys = CloseKeys {
    player_info: false,
    mission_info: true,
};

/// [`venues`] with the bar open and offering [`offered`].
fn bar_offering() -> SpaceportView {
    let mut view = venues();
    view.take_offer_request();
    click_item(&mut view, 11);
    serve_openings(&mut view);
    view.tick(FIRST_OFFER_DELAY);
    assert_eq!(view.take_offer_request(), Some(BAR));
    view.open_offer(offered());
    view
}

/// Whether the bar has closed, or been dropped once closed.
fn bar_left(view: &SpaceportView) -> bool {
    view.open_bar().is_none_or(BarScreen::closed)
}

/// Answers the offer open with `answer` and settles it as the router
/// does, with nothing to show.
fn answer(view: &mut SpaceportView, answer: Key) {
    view.input(&key(answer));
    assert!(view.take_offer_choice().is_some(), "answered");
    view.show_mission_texts(Vec::<MissionShown>::new());
}

#[test]
fn with_i_down_a_bar_offer_refused_stays_drawn_and_the_bar_takes_nothing_until_is_release() {
    let mut view = bar_offering();
    view.set_close_keys(I_DOWN);
    answer(&mut view, Key::KeyN);
    view.tick(ms(100));
    assert!(offer_drawn(&view), "drawn through the wait");
    view.input(&key(Key::Escape));
    assert!(!bar_left(&view), "the bar took no Escape");
    view.set_close_keys(CloseKeys::default());
    view.input(&released(MISSION_INFO_KEY));
    assert!(!offer_drawn(&view), "disposed on the release");
    view.tick(ms(1));
    assert!(bar_left(&view), "the queued Escape left the bar");
}

#[test]
fn with_i_up_a_bar_offer_refused_goes_at_once() {
    let mut view = bar_offering();
    answer(&mut view, Key::KeyN);
    assert!(!offer_drawn(&view));
    view.input(&key(Key::Escape));
    assert!(view.open_bar().is_none());
}

#[test]
fn a_hidden_accept_waits_the_same_drawing_nothing() {
    let mut view = bar_offering();
    view.set_close_keys(I_DOWN);
    answer(&mut view, Key::Enter);
    assert!(!offer_drawn(&view), "hidden");
    view.input(&key(Key::Escape));
    assert!(!bar_left(&view), "the bar took no Escape");
    view.input(&released(MISSION_INFO_KEY));
    view.tick(ms(1));
    assert!(bar_left(&view));
}

#[test]
fn with_p_down_the_player_info_closed_with_return_waits_for_ps_release() {
    let mut view = informed(missioned());
    view.set_close_keys(CloseKeys {
        player_info: true,
        mission_info: false,
    });
    view.open_player_info(pilot_info());
    view.take_status_refresh();
    view.input(&key(Key::Enter));
    assert!(view.player_info().is_some(), "waiting");
    view.input(&key(Key::KeyX));
    assert!(!view.take_status_refresh(), "not before the release");
    assert_eq!(view.take_wait_replay(), []);
    view.input(&released(PLAYER_INFO_KEY));
    assert!(view.player_info().is_none(), "closed on the release");
    assert!(view.take_status_refresh(), "the status bar's refresh");
    assert_eq!(view.take_wait_replay(), [key(Key::KeyX)]);
    assert_eq!(view.take_wait_replay(), [], "once");
}

#[test]
fn with_p_up_the_player_info_closes_at_once() {
    let mut view = informed(missioned());
    view.open_player_info(pilot_info());
    view.input(&key(Key::Enter));
    assert!(view.player_info().is_none());
}

#[test]
fn losing_focus_ends_a_wait_with_nothing_to_replay() {
    let mut view = informed(missioned());
    view.set_close_keys(CloseKeys {
        player_info: true,
        mission_info: false,
    });
    view.open_player_info(pilot_info());
    view.input(&key(Key::Enter));
    view.input(&key(Key::KeyX));
    view.release_keys();
    assert!(view.player_info().is_none());
    assert_eq!(view.take_wait_replay(), []);

    let mut view = bar_offering();
    view.set_close_keys(I_DOWN);
    answer(&mut view, Key::KeyN);
    view.input(&key(Key::Escape));
    view.release_keys();
    assert!(!offer_drawn(&view));
    view.tick(ms(1));
    assert!(!bar_left(&view), "the Escape was dropped");
}
