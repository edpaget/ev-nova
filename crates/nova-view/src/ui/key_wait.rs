//! The wait a close makes for its key's release, shared by the map
//! (`_DoSystemMap` @0x13a2e-0x13a46, waiting for M), the player info
//! screen (`_DoPlayerInfoDialog` @0x5a6aa-0x5a6d4, waiting for P) and a
//! mission offer (`_OfferOneMission` @0xa35fc-0xa3614, waiting for I). All
//! addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! Each loop spins on its key's `_KeyCheck` with the dialog still drawn,
//! reading no events, so what is typed meanwhile stays queued for the
//! next reader, and the closing `FlushEvents(7, 0)` drops only null,
//! mouse-down and mouse-up events. [`WaitKeys`] is that queue, shared by
//! all three, and [`KeyWait`] the rule, which only the player info screen
//! and the offer use for now: the map keeps its own copy in `FlightView`
//! (`map_key_held`, `map_closing`, `map_replay`), until the
//! `map-close-on-key-wait` task moves it onto [`KeyWait`]. The rule: a close made with the key up completes at once,
//! one made with it down waits for its release. The close itself still
//! acts on its press: only the dialog's disposal and what follows it wait,
//! so no key acts on a hold or a repeat.

use crate::flight::view::MAP_KEY;
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{Input, Key};

/// The keys the filters read live (`_PortFilter` @0x4f171-0x4f1c5 and its
/// kin), so one already let go when a wait ends does nothing.
const LIVE_KEYS: [Key; 3] = [MAP_KEY, PLAYER_INFO_KEY, MISSION_INFO_KEY];

/// The keys typed while a close waits for `waited`'s release, for the
/// screen under it: the original's queue of key-downs, auto-keys and
/// key-ups, less what can't reach a screen.
#[derive(Clone, Debug)]
pub(crate) struct WaitKeys {
    waited: Key,
    keys: Vec<Input>,
}

impl WaitKeys {
    /// An empty queue for a wait on `waited`.
    pub(crate) fn new(waited: Key) -> Self {
        Self {
            waited,
            keys: Vec::new(),
        }
    }

    /// Takes `input` if it is a key the queue keeps, repeats included, in
    /// order. The waited key's own never queues, as it is down throughout,
    /// and the router ran Caps Lock's double time on its press. A release
    /// goes in only behind something of its key, a press or a repeat:
    /// otherwise the screen never saw that key. So what is queued says
    /// whether a key it saw is still down.
    pub(crate) fn take(&mut self, input: &Input) {
        let Input::Key { key, pressed, .. } = *input else {
            return;
        };
        if key == self.waited || key == Key::CapsLock {
            return;
        }
        if pressed || self.keys.iter().any(|queued| is_key(queued, key)) {
            self.keys.push(*input);
        }
    }

    /// Drops all that was taken.
    pub(crate) fn clear(&mut self) {
        self.keys.clear();
    }

    /// What the wait leaves for the screen under it, in order, the queue
    /// emptied. The filters read [`MAP_KEY`], [`PLAYER_INFO_KEY`] and
    /// [`MISSION_INFO_KEY`] live, so one already let go does nothing, and
    /// goes with its repeats and release; one still down stays.
    pub(crate) fn finish(&mut self) -> Vec<Input> {
        let mut keys = std::mem::take(&mut self.keys);
        for live in LIVE_KEYS.into_iter().filter(|&key| key != self.waited) {
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

/// A close that waits for `key`'s release when made with it down, and
/// what was typed meanwhile (see the module docs).
#[derive(Clone, Debug)]
pub(crate) struct KeyWait {
    key: Key,
    down: bool,
    waiting: bool,
    queue: WaitKeys,
    replay: Vec<Input>,
}

impl KeyWait {
    /// A wait on `key`, which is `down` or up as it starts.
    pub(crate) fn new(key: Key, down: bool) -> Self {
        Self {
            key,
            down,
            waiting: false,
            queue: WaitKeys::new(key),
            replay: Vec::new(),
        }
    }

    /// Follows `key` up and down: a press or a repeat sets it down, a
    /// release lets it go.
    pub(crate) fn note(&mut self, input: &Input) {
        if let Input::Key { key, pressed, .. } = *input
            && key == self.key
        {
            self.down = pressed;
        }
    }

    /// Asks for the close: true when it completes now, with `key` up.
    /// With `key` down it starts waiting and returns false.
    pub(crate) fn close(&mut self) -> bool {
        if self.down {
            self.waiting = true;
        }
        !self.waiting
    }

    /// Whether a close waits for `key`'s release.
    pub(crate) fn waiting(&self) -> bool {
        self.waiting
    }

    /// Takes `input` while waiting: `key`'s release ends the wait (true)
    /// and finishes the queue. Every other input is queued, if a key, and
    /// does nothing else.
    pub(crate) fn input(&mut self, input: &Input) -> bool {
        if !self.waiting {
            return false;
        }
        if is_release_of(input, self.key) {
            self.down = false;
            self.waiting = false;
            self.replay = self.queue.finish();
            return true;
        }
        self.queue.take(input);
        false
    }

    /// Focus is lost, so `key` counts as let go: an open wait ends, its
    /// queue dropped, and this returns true, as the map's `release_keys`
    /// does.
    pub(crate) fn release(&mut self) -> bool {
        self.down = false;
        self.queue.clear();
        std::mem::replace(&mut self.waiting, false)
    }

    /// What the finished wait left for the screen under it, once.
    pub(crate) fn take_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.replay)
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

    fn click() -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: Point::new(1.0, 2.0),
        }
    }

    /// What a wait on `waited` leaves after taking `inputs`.
    fn left_waiting_for(waited: Key, inputs: &[Input]) -> Vec<Input> {
        let mut queue = WaitKeys::new(waited);
        for input in inputs {
            queue.take(input);
        }
        queue.finish()
    }

    /// What the map's wait leaves after taking `inputs`.
    fn left_by(inputs: &[Input]) -> Vec<Input> {
        left_waiting_for(MAP_KEY, inputs)
    }

    const A: Key = Key::KeyA;
    const B: Key = Key::KeyB;

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
            Input::PointerMovedOutside(at),
            click(),
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
    fn a_wait_for_p_keeps_m_i_and_other_keys() {
        let keys = [press(MAP_KEY), press(MISSION_INFO_KEY), press(A)];
        assert_eq!(left_waiting_for(PLAYER_INFO_KEY, &keys), keys);
    }

    #[test]
    fn a_wait_for_p_drops_ps_own() {
        let keys = [press(PLAYER_INFO_KEY), repeat(PLAYER_INFO_KEY), press(A)];
        assert_eq!(left_waiting_for(PLAYER_INFO_KEY, &keys), [press(A)]);
    }

    #[test]
    fn a_wait_for_p_drops_a_let_go_m_but_keeps_one_still_down() {
        let let_go = [press(MAP_KEY), release(MAP_KEY), press(A)];
        assert_eq!(left_waiting_for(PLAYER_INFO_KEY, &let_go), [press(A)]);
        let down = [press(MAP_KEY), repeat(MAP_KEY)];
        assert_eq!(left_waiting_for(PLAYER_INFO_KEY, &down), down);
    }

    #[test]
    fn a_wait_for_i_drops_a_let_go_p() {
        let keys = [press(PLAYER_INFO_KEY), release(PLAYER_INFO_KEY), press(A)];
        assert_eq!(left_waiting_for(MISSION_INFO_KEY, &keys), [press(A)]);
    }

    #[test]
    fn clear_drops_what_was_taken() {
        let mut queue = WaitKeys::new(MAP_KEY);
        queue.take(&press(A));
        queue.clear();
        assert_eq!(queue.finish(), []);
    }

    #[test]
    fn finish_leaves_it_empty() {
        let mut queue = WaitKeys::new(MAP_KEY);
        queue.take(&press(A));
        assert_eq!(queue.finish().len(), 1);
        assert_eq!(queue.finish(), []);
    }

    const P: Key = PLAYER_INFO_KEY;

    #[test]
    fn a_close_with_the_key_up_completes() {
        let mut wait = KeyWait::new(P, false);
        assert!(wait.close());
        assert!(!wait.waiting());
    }

    #[test]
    fn a_close_with_the_key_down_waits_for_its_release() {
        let mut wait = KeyWait::new(P, true);
        assert!(!wait.close());
        assert!(wait.waiting());
        assert!(!wait.input(&press(A)));
        assert!(wait.input(&release(P)));
        assert!(!wait.waiting());
    }

    #[test]
    fn note_follows_the_key() {
        let mut wait = KeyWait::new(P, false);
        wait.note(&press(P));
        assert!(!wait.close(), "down by its press");
        let mut wait = KeyWait::new(P, false);
        wait.note(&repeat(P));
        assert!(!wait.close(), "down by its repeat");
        let mut wait = KeyWait::new(P, true);
        wait.note(&release(P));
        assert!(wait.close(), "let go");
        let mut wait = KeyWait::new(P, true);
        wait.note(&release(A));
        assert!(!wait.close(), "another key's release");
    }

    #[test]
    fn a_repeat_or_press_of_the_key_while_waiting_does_nothing() {
        let mut wait = KeyWait::new(P, true);
        wait.close();
        assert!(!wait.input(&repeat(P)));
        assert!(!wait.input(&press(P)));
        assert!(wait.waiting());
        assert!(wait.input(&release(P)));
        assert_eq!(wait.take_replay(), []);
    }

    #[test]
    fn a_click_while_waiting_is_neither_queued_nor_acted_on() {
        let mut wait = KeyWait::new(P, true);
        wait.close();
        assert!(!wait.input(&click()));
        assert!(wait.waiting());
        wait.input(&release(P));
        assert_eq!(wait.take_replay(), []);
    }

    #[test]
    fn input_while_not_waiting_is_ignored() {
        let mut wait = KeyWait::new(P, true);
        assert!(!wait.input(&release(P)));
        assert!(!wait.input(&press(A)));
        assert_eq!(wait.take_replay(), []);
    }

    #[test]
    fn release_ends_a_wait_and_drops_its_queue() {
        let mut wait = KeyWait::new(P, true);
        wait.close();
        wait.input(&press(A));
        assert!(wait.release());
        assert!(!wait.waiting());
        assert_eq!(wait.take_replay(), []);
        assert!(wait.close(), "let go with focus");
    }

    #[test]
    fn release_with_no_wait_open_ends_nothing() {
        let mut wait = KeyWait::new(P, true);
        assert!(!wait.release());
    }

    #[test]
    fn the_replay_is_taken_once() {
        let mut wait = KeyWait::new(P, true);
        wait.close();
        wait.input(&press(Key::Escape));
        wait.input(&release(Key::Escape));
        wait.input(&release(P));
        assert_eq!(
            wait.take_replay(),
            [press(Key::Escape), release(Key::Escape)]
        );
        assert_eq!(wait.take_replay(), []);
    }
}
