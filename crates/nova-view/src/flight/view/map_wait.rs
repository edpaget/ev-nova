//! What [`FlightView`] keeps of the keys pressed while the map waits for
//! [`MAP_KEY`](super::MAP_KEY)'s release to close (`_DoSystemMap`
//! @0x13a2e-0x13a46). All addresses are in `EV Nova.app`'s `EV Nova`
//! (i386).
//!
//! The wait reads no events (`_BusyIdle` @0x5981 is empty) and plays no
//! sound up to the return (@0x13a2e-0x13c62), and the closing
//! `FlushEvents(7, 0)` (@0x13c44-0x13c53) drops only null, mouse-down and
//! mouse-up events. So a click during the wait is gone, and a key-down,
//! an auto-key and a key-up stay queued for whoever reads events next.
//! What that does depends on who that is:
//!
//! - **Flight reads no queued key.** `_IterateInGameEventLoop` drops them
//!   (@0x56a6b, @0x56cec), and `_HandlePlayer` polls every key from the
//!   `GetKeys` snapshot of `_QuickKeyCheckHandler` (@0x5d5a, read by
//!   `_QuickKeyCheck` @0xa0f5). A key tapped during the wait does nothing
//!   after the close, and one still down reads as down, whenever it went
//!   down, before the wait and before the map opened too. So over flight
//!   the view keeps [`UNLATCHED_KEYS`] held across the map, presses and
//!   releases, and everything else is let go as the map opens. The latched
//!   keys wait for their release, their OS repeats included, which the
//!   poll never sees ([`FlightView::latched_down`]): `_HandlePlayer` sets
//!   its 22 latches and the escort menu's 5 after M's map
//!   (`_SetAllKeyCmdFlags` @0x61e6d, called @0x6a898). The same
//!   poll reads them after every other dialog over flight, which takes
//!   flight's input from the router instead: the router keeps what is
//!   physically down under it and hands it back as the last one closes
//!   ([`FlightView::take_back_live_input`]), and a gate's movie, which
//!   flight plays, keeps them as the map does (`FlightView::came_through`).
//! - **A screen reads the queue.** Every screen's loop takes it through
//!   `_MyModalDialog` (@0x5999a), and no caller flushes between
//!   `_DoSystemMap` and that loop. So over a screen the view keeps what the
//!   wait takes ([`WaitKeys`]) and hands it to the router at the close
//!   ([`FlightView::take_map_replay`]), which gives it to the screen.
//!
//! Left out:
//!
//! - `MenuKey` for a queued Command key in flight (@0x56a81), whose result
//!   is ignored, as ours' Command-Q acts on its press in the router.
//! - An auto-key's live read of M, P or I (the `what == 5` tests @0x4e0c6,
//!   @0x608cd, @0x9f7da and their kin): those keys stay press-only by the
//!   user's decision.
//! - Key-up events as such: no filter tests `what == 4`, and the view
//!   replays a queued key's release with its press only so a screen isn't
//!   left holding it.
//! - Update and activate events, which also survive the flush: ours draws
//!   every frame whole and has no such input.
//! - A wait ended by losing focus: ours can't read keys unfocused, as
//!   `_KeyCheck` reads them live (@0xa19b-0xa1dc), so
//!   [`Screen::release_keys`](crate::Screen::release_keys) closes the map
//!   and drops what was queued.
//! - [`Input::Text`]: no filter under the map takes text, and each text
//!   dialog opens through a `0xff7f` flush (`_EVTextInputDialog`
//!   @0x55429).

use super::dialog_map::MapOpening;
use super::{
    AFTERBURNER_KEY, DESELECT_KEY, EJECT_KEY, FACE_TARGET_KEY, FIRE_KEY, FLIGHT_KEYS, FlightView,
    JETTISON_KEY, MAP_KEY, SECONDARY_KEY, SELF_DESTRUCT_KEY,
};
use crate::flight::sim_port::FlightSim;
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{Input, Key};

/// The keys `_HandlePlayer` acts on the moment they read down, with no
/// latch to wait on, so the map keeps them held: thrust +0x2a (@0x6cb71),
/// turn +0x26 and +0x28 (@0x6c9cc), reverse +0x2c (@0x6c72f), face-target
/// +0xe (@0x6aec6), afterburner +0x30 (@0x6cadf), primary +0x4 (@0x6ab06),
/// secondary +0x6 (@0x6ab87), weapon safety +0x2 (@0x6adc2-0x6addf), the
/// eject chord +0x22 (@0x6f29e-0x6f2c1), the self-destruct chord +0x24
/// (@0x6fdb4-0x6fdfc), jettison +0x1e (@0x6fd54-0x6fda1), and raw Option,
/// Command and Shift (0x3a, 0x37, 0x38).
///
/// Every other flight key has a latch `_SetAllKeyCmdFlags` (@0x61e6d)
/// sets: `_navModeSelectFlag` \` and H (0xdee04), `_navTargetSelectFlag`
/// `\` (0xdee48), `_navDefaultSelectFlag` the stellar slot keys (0xdede0),
/// `_weapTargetSelectFlag` Tab (0xdede8), `_mapTryFlag` M (0xdedf8),
/// `_weapCloseTargFlag` R (0xdee50), `_weapSelectFlag` W (0xdee54),
/// `_jumpTryFlag` J (0xdee44), `_ackCommFlag` Return (0xdedf4),
/// `_playerInfoFlag` P (0xdedd4), `_commTryFlag` Y (0xdee0c),
/// `_missionInfoFlag` I (0xdee00) and `_cloakFlag` U (0xdedfc). B and L
/// have latches of their own that it doesn't set, `_boardTryFlag`
/// (0xdee18) and `_dockTryFlag` (0xdee34), set as they act, so one that
/// acted before a close waits for its release all the same. One first
/// pressed under the dialog, and the latched keys' reads that skip their
/// latch, are rdm:phase/play-feel-8/phase-20-latched-keys-read-past-their-latches.
pub(super) const UNLATCHED_KEYS: [Key; 15] = [
    Key::Up,
    Key::Left,
    Key::Right,
    Key::Down,
    FACE_TARGET_KEY,
    AFTERBURNER_KEY,
    FIRE_KEY,
    SECONDARY_KEY,
    DESELECT_KEY,
    EJECT_KEY,
    SELF_DESTRUCT_KEY,
    JETTISON_KEY,
    Key::Alt,
    Key::Command,
    Key::Shift,
];

/// The keys typed while a map over a screen waits, for the screen under
/// it: the original's queue of key-downs, auto-keys and key-ups, less what
/// can't reach a screen.
#[derive(Clone, Debug, Default)]
pub(super) struct WaitKeys {
    keys: Vec<Input>,
}

impl WaitKeys {
    /// Takes `input` if it is a key the queue keeps, repeats included, in
    /// order. M's own never queues, as it is down throughout, and the
    /// router ran Caps Lock's double time on its press. A release goes in
    /// only behind something of its key, a press or a repeat: otherwise
    /// the screen never saw that key. So what is queued says whether a key
    /// it saw is still down.
    pub(super) fn take(&mut self, input: &Input) {
        let Input::Key { key, pressed, .. } = *input else {
            return;
        };
        if key == MAP_KEY || key == Key::CapsLock {
            return;
        }
        if pressed || self.keys.iter().any(|queued| is_key(queued, key)) {
            self.keys.push(*input);
        }
    }

    /// Drops all that was taken.
    pub(super) fn clear(&mut self) {
        self.keys.clear();
    }

    /// What the wait leaves for the screen under the map, in order, the
    /// queue emptied. The filters read [`PLAYER_INFO_KEY`] and
    /// [`MISSION_INFO_KEY`] live (`_PortFilter` @0x4f171-0x4f1c5 and its
    /// kin), so one already let go does nothing, and goes with its repeats
    /// and release; one still down stays.
    pub(super) fn finish(&mut self) -> Vec<Input> {
        let mut keys = std::mem::take(&mut self.keys);
        for live in [PLAYER_INFO_KEY, MISSION_INFO_KEY] {
            let let_go = keys.iter().rposition(|input| is_release_of(input, live));
            if let Some(let_go) = let_go {
                keys = keys
                    .into_iter()
                    .enumerate()
                    .filter(|(at, input)| *at > let_go || !is_key(input, live))
                    .map(|(_, input)| input)
                    .collect();
            }
        }
        keys
    }
}

/// Whether `input` is a release of `key`.
fn is_release_of(input: &Input, key: Key) -> bool {
    matches!(*input, Input::Key { key: k, pressed: false, .. } if k == key)
}

/// Whether `input` is anything of `key`.
fn is_key(input: &Input, key: Key) -> bool {
    matches!(*input, Input::Key { key: k, .. } if k == key)
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether the map open is flight's own: M's or the hypergate's, which
    /// takes its input from flight ([`MapOpening::Plain`]).
    pub(super) fn plain_map_open(&self) -> bool {
        self.map_opened
            .is_some_and(|opened| opened.by == MapOpening::Plain)
    }

    /// Keeps of the keys held only those flight goes on reading live
    /// across the map ([`UNLATCHED_KEYS`]), as every path that calls it
    /// runs `_SetAllKeyCmdFlags`: the latched keys it lets go of wait for
    /// their release ([`FlightView::latched_down`]), repeats and all.
    pub(super) fn keep_unlatched_keys(&mut self) {
        let latched: Vec<Key> = self
            .held
            .iter()
            .copied()
            .filter(|key| !UNLATCHED_KEYS.contains(key))
            .collect();
        self.held.retain(|key| UNLATCHED_KEYS.contains(key));
        self.latch_held_keys(latched);
    }

    /// Notes `keys` down as latched, those flight reads with a latch
    /// `_SetAllKeyCmdFlags` sets: the flight keys beyond
    /// [`UNLATCHED_KEYS`].
    fn latch_held_keys(&mut self, keys: impl IntoIterator<Item = Key>) {
        let latched = keys
            .into_iter()
            .filter(|key| FLIGHT_KEYS.contains(key) && !UNLATCHED_KEYS.contains(key));
        self.latched_down.extend(latched);
    }

    /// Tracks `input`'s key as held if it is one flight goes on reading
    /// live across the map ([`UNLATCHED_KEYS`]). A latched flight key
    /// isn't held, but its fresh press is noted as latched and its release
    /// lets go of it, so the close leaves latched exactly those still down
    /// ([`FlightView::latched_down`]): a gate's movie starts with the keys
    /// flight held, which its come-out sorts (`FlightView::came_through`).
    pub(super) fn track_unlatched(&mut self, input: &Input) {
        let Input::Key {
            key,
            pressed,
            repeat,
        } = *input
        else {
            return;
        };
        if UNLATCHED_KEYS.contains(&key) || !pressed {
            self.track_held(input);
        } else if !repeat {
            self.latch_held_keys([key]);
        }
    }

    /// Holds `key` if flight would read it down at once
    /// ([`UNLATCHED_KEYS`]), for one still down as the last dialog over
    /// flight closes. Its release comes to flight live.
    pub fn hold_live_key(&mut self, key: Key) {
        if UNLATCHED_KEYS.contains(&key) {
            self.held.insert(key);
        }
    }

    /// Takes back flight's input as the last dialog over it closes, `down`
    /// the keys physically down and `button` whether the primary button
    /// is: `_HandlePlayer` polls the keys (`_QuickKeyCheck` @0xa0f5) and
    /// `_HandleMouse` reads `_Button` (@0x3801c-0x3803b) each frame, so
    /// what is down reads as down whenever it went down, and
    /// `_SetAllKeyCmdFlags` (@0x61e6d) latches the rest. The unlatched
    /// keys held become exactly those of `down` ([`UNLATCHED_KEYS`]); no
    /// latched key is taken from `down`, as its latch waits for its
    /// release, and none flight holds is let go. The latched keys of
    /// `down` become exactly those whose repeats flight drops until their
    /// release ([`FlightView::latched_down`]), as `down` is the whole of
    /// what is down. A button still down keeps the cursor whole.
    pub fn take_back_live_input(&mut self, down: impl IntoIterator<Item = Key>, button: bool) {
        self.held.retain(|key| !UNLATCHED_KEYS.contains(key));
        self.latched_down.clear();
        let down: Vec<Key> = down.into_iter().collect();
        for &key in &down {
            self.hold_live_key(key);
        }
        self.latch_held_keys(down);
        self.cursor.button(button);
    }

    /// What the last map over a screen left for the screen under it, in
    /// order, once: the keys typed during its wait (see the module docs),
    /// for the router to replay. Nothing from a map over flight, and
    /// nothing from a wait that focus ended.
    pub fn take_map_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.map_replay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MouseButton, Point};

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn repeat(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    /// What the wait leaves after taking `inputs`.
    fn left_by(inputs: &[Input]) -> Vec<Input> {
        let mut queue = WaitKeys::default();
        for input in inputs {
            queue.take(input);
        }
        queue.finish()
    }

    const A: Key = Key::Char('a');
    const B: Key = Key::Char('b');

    #[test]
    fn it_keeps_keys_and_repeats_in_order() {
        let keys = [
            press(A),
            repeat(A),
            press(B),
            release(A),
            release(B),
            press(Key::Enter),
        ];
        assert_eq!(left_by(&keys), keys);
    }

    #[test]
    fn it_takes_neither_m_nor_caps_lock() {
        // The router ran Caps Lock's double time on its press.
        for key in [MAP_KEY, Key::CapsLock] {
            let keys = [press(key), repeat(key), release(key)];
            assert_eq!(left_by(&keys), [], "{key:?}");
        }
    }

    #[test]
    fn it_takes_no_pointer_input_and_no_text() {
        let at = Point::new(1.0, 2.0);
        let inputs = [
            Input::PointerMoved(at),
            Input::PointerLeft,
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at,
            },
            Input::Text('a'),
        ];
        assert_eq!(left_by(&inputs), []);
    }

    #[test]
    fn it_drops_the_release_of_a_key_whose_press_it_never_took() {
        assert_eq!(left_by(&[release(A), press(B)]), [press(B)]);
        assert_eq!(left_by(&[press(B), release(A)]), [press(B)]);
    }

    #[test]
    fn it_keeps_a_repeat_with_no_press_as_an_auto_key_stays_queued() {
        assert_eq!(left_by(&[repeat(A)]), [repeat(A)]);
    }

    #[test]
    fn it_keeps_the_release_of_a_key_it_holds_only_repeats_of() {
        let keys = [repeat(A), release(A)];
        assert_eq!(left_by(&keys), keys);
    }

    #[test]
    fn it_drops_a_live_key_already_let_go_with_its_repeats() {
        for key in [PLAYER_INFO_KEY, MISSION_INFO_KEY] {
            let keys = [press(key), press(A), repeat(key), release(key), release(A)];
            assert_eq!(left_by(&keys), [press(A), release(A)], "{key:?}");
        }
    }

    #[test]
    fn it_keeps_a_live_key_still_down() {
        for key in [PLAYER_INFO_KEY, MISSION_INFO_KEY] {
            let keys = [press(key), repeat(key)];
            assert_eq!(left_by(&keys), keys, "{key:?}");
            // Tapped first, the tap goes and the last press stays.
            let tapped = [press(key), release(key), press(key)];
            assert_eq!(left_by(&tapped), [press(key)], "{key:?}");
        }
    }

    #[test]
    fn clear_drops_what_was_taken() {
        let mut queue = WaitKeys::default();
        queue.take(&press(A));
        queue.clear();
        assert_eq!(queue.finish(), []);
    }

    #[test]
    fn finish_leaves_it_empty() {
        let mut queue = WaitKeys::default();
        queue.take(&press(A));
        assert_eq!(queue.finish().len(), 1);
        assert_eq!(queue.finish(), []);
    }
}
