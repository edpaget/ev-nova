//! Keys that wait for a held press's release on the mission screens: the
//! mission computer ([`MissionComputerScreen`]) and the mission info
//! screen ([`MissionInfoScreen`]). All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386), and line numbers in `Toolbox/ListMgr/ListMgrPACK.a`
//! (mac-rom) count its CR line ends.
//!
//! `LClick`'s loop (`clickScan`, line 948: `GetMouse` and `StillDown`)
//! and the screens' button trackers (`_TrackMissionInfoButtons`
//! @0x285d9/0x28643, `_TrackMissionButtons` @0x288bf/0x28928) read no
//! events. So a key pressed while a press is held on a list, its bar or a
//! button waits in the queue until the release, and then reaches the
//! filter. The list holds its keys as [`RowList`] does, and the buttons as
//! [`Dialog::with_held_keys`] does (the `held_keys` module). Each screen
//! runs what they held back through its own input path at the release,
//! in order, within the same input ([`run_replay`]), until one of them
//! stops it. Where the rest goes then ([`Rest`]):
//!
//! - **On the computer** (`_DoMissionDialog`):
//!   - Leave (the button or a key) disposes of the dialog with no flush
//!     (@0xa40c8), and `_DoPortDialog`'s loop (@0x60035-0x6004e) goes back
//!     to `_PortFilter`: the rest reaches the spaceport
//!     ([`MissionComputerScreen::take_unclaimed`]).
//!   - The map (@0xa3c85-0xa3d81) and player info (@0xa3fbb-0xa3ffc)
//!     flush event codes, then open, and each open flushes 0xff7f: the
//!     rest is dropped.
//!   - Mission info (item 10) flushes the same codes, then opens only when
//!     an active mission isn't hidden (@0xa3e94-0xa3ebf), flushing as
//!     `_DoMissionInfoDialog` opens (@0x9e287). With none, nothing opens
//!     and the rest runs on ([`MissionComputerScreen::set_missions_listed`]).
//!   - Accept (@0xa3b47-0xa3bb4) flushes codes 3, 5, 1 and 2, which
//!     flush only mouse events as masks, before it checks for a row
//!     selected (@0xa3ba4); then `_DoMissionAccept`. Full slots return
//!     with no text (@0xa1b44-0xa1bbe), and the rest runs on in the
//!     computer. Too few holds or too little space show a `STR#` text
//!     (@0xa1bb2), which flushes. Taken, the accept may show the briefing
//!     (@0xa1f0e, @0xa1fb5); the computer then re-enters, and its reopen
//!     flushes 0xff7f (@0xa3a1c), or, with nothing left to offer, it
//!     closes silently (@0xa380a-0xa3817), and the rest reaches the
//!     spaceport unless a text flushed it. The rest waits for the host's
//!     answer ([`MissionComputerScreen::take_after_accept`]). With no row
//!     selected, Accept skips `_DoMissionAccept`, and the rest runs on.
//! - **On mission info** (`_DoMissionInfoDialog`):
//!   - Done, Return, Enter, Escape and I close it with no flush
//!     (@0x9e744): the rest reaches what is beneath
//!     ([`MissionInfoScreen::take_unclaimed`]).
//!   - M opens the map, which flushes (@0x9e45a), unless `_missionDialog`
//!     is set (@0x9e3c6-0x9e3cd); then nothing happens and the rest runs
//!     on.
//!   - Abort calls `_AbortMission`, which shows nothing
//!     (@0x99a92-0x99bdc), and rebuilds the list (@0x9e5e6-0x9e6d8): the
//!     rest waits for the host's abort and runs on the rebuilt list
//!     ([`MissionInfoScreen::set_info`]). A refused abort skips everything
//!     (@0x9e583), and the next tick runs the rest.
//!
//! A press the router cancels keeps its keys for the next tick, which
//! runs them. Neither the trackers, `clickScan` nor the paths above play a
//! sound: only the screens' open and close sounds, which they already
//! play.

use std::collections::VecDeque;

use crate::input::Input;
#[cfg(doc)]
use crate::spaceport::missions::{MissionComputerScreen, MissionInfoScreen};
#[cfg(doc)]
use crate::ui::dialog::Dialog;
#[cfg(doc)]
use crate::ui::row_list::RowList;

/// Where the rest of a replay goes once an input in it stopped it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Rest {
    /// The screen closed with no flush: the rest reaches what reads
    /// events next.
    Unclaimed,
    /// A screen opened and flushed the queue: the rest is dropped.
    Dropped,
    /// The screen asked its host for an answer (an accept or an abort):
    /// the rest waits for it.
    Answer,
}

/// What the inputs a screen ran did to its replay, until the replay reads
/// it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Stops {
    stop: Option<Rest>,
    flush_mouse: bool,
}

impl Stops {
    /// The input stopped the replay, its rest going as `rest` says. The
    /// first stop holds.
    pub(super) fn stop(&mut self, rest: Rest) {
        self.stop.get_or_insert(rest);
    }

    /// The input flushed the mouse events queued after it
    /// (`FlushEvents` passed event codes for masks).
    pub(super) fn flush_mouse(&mut self) {
        self.flush_mouse = true;
    }
}

/// Runs `replay` through `input` on `screen`, in order. Before each input,
/// and after the last, it reads what the inputs so far did
/// ([`Stops`], through `stops`): a mouse flush drops the presses left, and
/// a stop ends the replay, giving back where the rest goes and the rest.
/// The input the screen ran before the replay counts, so a click's own
/// activation stops the keys it held back.
pub(super) fn run_replay<S>(
    screen: &mut S,
    replay: Vec<Input>,
    stops: fn(&mut S) -> &mut Stops,
    input: fn(&mut S, &Input),
) -> Option<(Rest, Vec<Input>)> {
    let mut rest = VecDeque::from(replay);
    loop {
        let found = stops(screen);
        if std::mem::take(&mut found.flush_mouse) {
            rest.retain(|input| !matches!(input, Input::PointerButton { .. }));
        }
        if let Some(stop) = found.stop.take() {
            return Some((stop, rest.into()));
        }
        let next = rest.pop_front()?;
        input(screen, &next);
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::{MissionId, MissionInfo};

    use super::super::missions::tests::{
        button, computer_row_at, computer_selected, info_row_at, info_screen_of, key, mac_ticks,
        many_info, many_offers, screen_of,
    };
    use super::super::missions::{
        ABORT_ITEM, ACCEPT_ITEM, LEAVE_ITEM, MissionComputerScreen, MissionInfoScreen,
    };
    use super::*;
    use crate::flight::MapOpener;
    use crate::geometry::Point;
    use crate::input::Key;
    use crate::screen::Screen;
    use crate::sound::{Sound, UiSound};
    use crate::ui::mission_info::MISSION_INFO_KEY;
    use crate::ui::row_list::RowList;

    fn up(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    fn computer_item(screen: &MissionComputerScreen, item: usize) -> Point {
        screen
            .dialog()
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("an item")
            .center()
    }

    /// Three offers, 130 on, the texts asked for so far taken.
    fn computer() -> MissionComputerScreen {
        let mut screen = screen_of(many_offers(3));
        let _ = screen.take_text_request();
        screen
    }

    /// A press on the computer's row `row`, `keys` while it is held.
    fn held_on_row(screen: &mut MissionComputerScreen, row: f32, keys: &[Input]) {
        screen.input(&button(true, computer_row_at(row)));
        for input in keys {
            screen.input(input);
        }
    }

    #[test]
    fn a_down_held_during_a_list_press_steps_only_after_the_release() {
        let mut screen = computer();
        held_on_row(&mut screen, 0.0, &[key(Key::Down)]);
        assert_eq!(computer_selected(&screen), Some(0), "no step yet");
        assert_eq!(screen.take_text_request(), None);
        screen.input(&button(false, computer_row_at(0.0)));
        assert_eq!(computer_selected(&screen), Some(1));
        assert_eq!(screen.take_text_request(), Some(MissionId(131)));
    }

    #[test]
    fn keys_held_on_the_bar_act_after_the_release_in_order() {
        let mut screen = screen_of(many_offers(13));
        let _ = screen.take_text_request();
        let down = Point::new(470.0, 450.0);
        screen.input(&button(true, down));
        screen.input(&key(Key::Down));
        screen.input(&up(Key::Down));
        screen.input(&key(Key::Down));
        assert_eq!(computer_selected(&screen), Some(0));
        screen.input(&button(false, down));
        assert_eq!(computer_selected(&screen), Some(2));
        assert_eq!(screen.take_text_request(), Some(MissionId(132)));
    }

    #[test]
    fn keys_held_during_a_click_on_leave_follow_its_close() {
        let mut screen = computer();
        let leave = computer_item(&screen, LEAVE_ITEM);
        screen.input(&button(true, leave));
        screen.input(&key(Key::Down));
        assert!(!screen.closed());
        screen.input(&button(false, leave));
        assert!(screen.closed());
        assert_eq!(screen.take_unclaimed(), [key(Key::Down)]);
        assert_eq!(computer_selected(&screen), Some(0));
        assert_eq!(screen.take_unclaimed(), [], "once");
    }

    #[test]
    fn a_leave_key_held_during_a_list_press_closes_after_the_release_and_keeps_the_rest() {
        let mut screen = computer();
        held_on_row(&mut screen, 0.0, &[key(Key::Escape), key(Key::Down)]);
        assert!(!screen.closed(), "not before the release");
        screen.input(&button(false, computer_row_at(0.0)));
        assert!(screen.closed());
        assert_eq!(screen.take_unclaimed(), [key(Key::Down)]);
        assert_eq!(computer_selected(&screen), Some(0));
    }

    #[test]
    fn m_or_p_held_during_a_press_asks_after_the_release_and_drops_the_rest() {
        let mut map = computer();
        held_on_row(&mut map, 0.0, &[key(Key::Char('m')), key(Key::Down)]);
        assert_eq!(map.take_map_request(), None, "not before the release");
        map.input(&button(false, computer_row_at(0.0)));
        assert_eq!(
            map.take_map_request(),
            Some(MapOpener::Computer(Some(MissionId(130))))
        );
        assert_eq!(computer_selected(&map), Some(0), "the Down flushed");
        assert_eq!(map.take_unclaimed(), []);
        let mut info = computer();
        held_on_row(&mut info, 0.0, &[key(Key::Char('p')), key(Key::Down)]);
        info.input(&button(false, computer_row_at(0.0)));
        assert!(info.take_player_info_request());
        assert_eq!(computer_selected(&info), Some(0), "the Down flushed");
    }

    #[test]
    fn i_held_during_a_press_drops_the_rest_only_when_missions_are_listed() {
        let mut listed = computer();
        held_on_row(&mut listed, 0.0, &[key(MISSION_INFO_KEY), key(Key::Down)]);
        listed.input(&button(false, computer_row_at(0.0)));
        assert!(listed.take_info_request());
        assert_eq!(computer_selected(&listed), Some(0), "the Down flushed");
        let mut none = computer();
        none.set_missions_listed(false);
        held_on_row(&mut none, 0.0, &[key(MISSION_INFO_KEY), key(Key::Down)]);
        none.input(&button(false, computer_row_at(0.0)));
        assert!(
            !none.take_info_request(),
            "nothing opens (@0xa3e94-0xa3ebf)"
        );
        assert_eq!(computer_selected(&none), Some(1), "the Down ran on");
    }

    #[test]
    fn a_held_during_a_press_accepts_after_the_release_and_keeps_the_rest_for_the_answer() {
        let mut screen = computer();
        held_on_row(&mut screen, 1.0, &[key(Key::Char('a')), key(Key::Down)]);
        assert_eq!(screen.take_accept(), None, "not before the release");
        screen.input(&button(false, computer_row_at(1.0)));
        assert_eq!(
            screen.take_accept(),
            Some(MissionId(131)),
            "read back first"
        );
        assert_eq!(computer_selected(&screen), Some(1), "the Down waits");
        assert!(screen.holds_back_keys(), "until the answer");
        assert_eq!(screen.take_after_accept(), [key(Key::Down)]);
        assert!(!screen.holds_back_keys());
        assert_eq!(screen.take_after_accept(), [], "once");
    }

    #[test]
    fn a_click_on_accept_with_nothing_selected_runs_the_rest_on() {
        let mut screen = computer();
        screen.input(&button(true, computer_row_at(5.0)));
        screen.input(&button(false, computer_row_at(5.0)));
        assert_eq!(computer_selected(&screen), None);
        let accept = computer_item(&screen, ACCEPT_ITEM);
        screen.input(&button(true, accept));
        screen.input(&key(Key::Down));
        screen.input(&button(false, accept));
        assert_eq!(screen.take_accept(), None);
        assert_eq!(computer_selected(&screen), Some(0), "the Down ran on");
        assert_eq!(screen.take_after_accept(), []);
    }

    #[test]
    fn an_accept_or_mission_info_flushes_the_presses_after_it() {
        // A second press ends the first, and replays after the keys.
        let mut screen = computer();
        held_on_row(&mut screen, 0.0, &[key(Key::Char('a'))]);
        screen.input(&button(true, computer_row_at(2.0)));
        assert_eq!(screen.take_accept(), Some(MissionId(130)));
        assert_eq!(
            screen.take_after_accept(),
            [],
            "codes 3, 5, 1, 2 (@0xa3b47)"
        );
        assert!(!screen.rows().is_some_and(RowList::tracking));
        let mut none = computer();
        none.set_missions_listed(false);
        held_on_row(&mut none, 0.0, &[key(MISSION_INFO_KEY)]);
        none.input(&button(true, computer_row_at(2.0)));
        assert!(!none.rows().is_some_and(RowList::tracking), "flushed");
        let mut plain = computer();
        held_on_row(&mut plain, 0.0, &[key(Key::Down)]);
        plain.input(&button(true, computer_row_at(2.0)));
        assert!(
            plain.rows().is_some_and(RowList::tracking),
            "a press after a step starts its own"
        );
    }

    #[test]
    fn a_cancelled_press_replays_its_keys_on_the_next_tick_on_the_computer() {
        let mut screen = computer();
        held_on_row(&mut screen, 0.0, &[key(Key::Down)]);
        screen.cancel_pointer();
        assert_eq!(computer_selected(&screen), Some(0));
        screen.tick(mac_ticks(1));
        assert_eq!(computer_selected(&screen), Some(1));
    }

    #[test]
    fn the_computer_holds_back_keys_from_a_press_until_its_replay_has_run() {
        let mut screen = computer();
        assert!(!screen.holds_back_keys());
        held_on_row(&mut screen, 0.0, &[key(Key::Down)]);
        assert!(screen.holds_back_keys(), "the list's press");
        screen.input(&button(false, computer_row_at(0.0)));
        assert!(!screen.holds_back_keys(), "its replay ran");
        let leave = computer_item(&screen, LEAVE_ITEM);
        screen.input(&button(true, leave));
        assert!(screen.holds_back_keys(), "a click on Leave");
        screen.input(&key(Key::Down));
        screen.release_keys();
        assert!(screen.holds_back_keys(), "abandoned until the tick");
        screen.tick(mac_ticks(1));
        assert!(!screen.holds_back_keys());
        assert!(!screen.closed(), "abandoned, not clicked");
        assert_eq!(computer_selected(&screen), Some(2), "the Down ran");
    }

    // The mission info screen.

    /// Three missions in slots 0 to 2, the even ones abortable, the texts
    /// asked for so far taken, and its opening sound.
    fn info() -> MissionInfoScreen {
        let mut screen = info_screen_of(many_info(3));
        let _ = screen.take_text_request();
        let _ = screen.take_sounds();
        screen
    }

    fn info_selected(screen: &MissionInfoScreen) -> Option<usize> {
        screen.rows().and_then(RowList::selected)
    }

    fn abort_at(screen: &MissionInfoScreen) -> Point {
        screen
            .dialog()
            .and_then(|dialog| dialog.item_bounds(ABORT_ITEM))
            .expect("Abort")
            .center()
    }

    /// A click on Abort holding Down, slot 0 selected.
    fn abort_holding_down(screen: &mut MissionInfoScreen) {
        let at = abort_at(screen);
        screen.input(&button(true, at));
        screen.input(&key(Key::Down));
        screen.input(&button(false, at));
    }

    /// `many_info(3)` less slot 0.
    fn after_abort() -> MissionInfo {
        let mut info = many_info(3);
        info.rows.remove(0);
        info
    }

    #[test]
    fn a_down_held_during_a_list_press_steps_after_the_release() {
        let mut screen = info();
        screen.input(&button(true, info_row_at(0.0)));
        screen.input(&key(Key::Down));
        assert_eq!(info_selected(&screen), Some(0));
        screen.input(&button(false, info_row_at(0.0)));
        assert_eq!(info_selected(&screen), Some(1));
        assert_eq!(screen.take_text_request(), Some(1));
    }

    #[test]
    fn escape_or_i_held_during_a_press_closes_after_the_release_keeping_the_rest() {
        for closing in [Key::Escape, MISSION_INFO_KEY] {
            let mut screen = info();
            screen.input(&button(true, info_row_at(0.0)));
            screen.input(&key(closing));
            screen.input(&key(Key::Down));
            assert!(!screen.closed(), "{closing:?}");
            assert_eq!(screen.take_sounds(), [], "{closing:?}: no close yet");
            screen.input(&button(false, info_row_at(0.0)));
            assert!(screen.closed(), "{closing:?}");
            assert_eq!(
                screen.take_sounds(),
                [Sound::Ui(UiSound::MissionInfoClosed)],
                "{closing:?}"
            );
            assert_eq!(screen.take_unclaimed(), [key(Key::Down)], "{closing:?}");
        }
    }

    #[test]
    fn m_held_during_a_press_drops_the_rest_unless_the_map_is_refused() {
        let mut allowed = info();
        allowed.input(&button(true, info_row_at(0.0)));
        allowed.input(&key(Key::Char('m')));
        allowed.input(&key(Key::Down));
        allowed.input(&button(false, info_row_at(0.0)));
        assert_eq!(
            allowed.take_map_request(),
            Some(MapOpener::MissionInfo(Some(0)))
        );
        assert_eq!(info_selected(&allowed), Some(0), "the Down flushed");
        let mut refused = info().with_map_allowed(false);
        refused.input(&button(true, info_row_at(0.0)));
        refused.input(&key(Key::Char('m')));
        refused.input(&key(Key::Down));
        refused.input(&button(false, info_row_at(0.0)));
        assert_eq!(refused.take_map_request(), None);
        assert_eq!(info_selected(&refused), Some(1), "the Down ran on");
    }

    #[test]
    fn keys_held_during_a_click_on_abort_run_on_the_info_set_after_it() {
        let mut screen = info();
        abort_holding_down(&mut screen);
        assert_eq!(screen.take_abort(), Some(0));
        assert_eq!(info_selected(&screen), Some(0), "the Down waits");
        screen.set_info(after_abort());
        assert_eq!(
            info_selected(&screen),
            Some(0),
            "nothing selected after the abort, then Down takes the first"
        );
        assert_eq!(screen.take_text_request(), Some(1), "slot 1's");
    }

    #[test]
    fn keys_held_after_a_refused_abort_run_on_the_next_tick() {
        let mut screen = info();
        abort_holding_down(&mut screen);
        assert_eq!(screen.take_abort(), Some(0));
        screen.tick(mac_ticks(1));
        assert_eq!(info_selected(&screen), Some(1), "@0x9e583");
    }

    #[test]
    fn the_info_screen_holds_back_keys_from_a_press_until_its_replay_has_run() {
        let mut screen = info();
        assert!(!screen.holds_back_keys());
        screen.input(&button(true, info_row_at(0.0)));
        assert!(screen.holds_back_keys());
        screen.input(&button(false, info_row_at(0.0)));
        assert!(!screen.holds_back_keys());
        abort_holding_down(&mut screen);
        assert!(screen.holds_back_keys(), "until the abort's answer");
        screen.set_info(after_abort());
        assert!(!screen.holds_back_keys());
    }
}
