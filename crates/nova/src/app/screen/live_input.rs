//! What is physically down while flight's input is away, handed back to
//! flight as it gets it back. All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! Each frame `_DoPlayGameWork` reads the whole keyboard
//! (`_QuickKeyCheckHandler` @0x43e01, `GetKeys` @0x5d68) and
//! `_HandleMouse` the button (`_Button` @0x3801c-0x3803b), so after any
//! dialog over flight `_HandlePlayer` reads its keys and the button as
//! they physically are, whenever they went down; the 16 calls of
//! `_SetAllKeyCmdFlags` (@0x61e6d) after a dialog latch the latched keys,
//! and after a dialog none follows each latch is as the dialog found it.
//! The dialogs' `FlushEvents` empty only the event queue, which flight
//! never reads (rdm:phase/play-feel-8/phase-3-keys-live-across-a-dialog
//! records each site, and
//! rdm:phase/play-feel-8/phase-19-latched-keys-under-unlatching-dialogs
//! the dialogs no site follows).
//!
//! Ours has no live read: flight sees only the events it is given, and
//! while a dialog is over it, or the spaceport takes its place, the router
//! gives them to that instead, so flight lets go of its keys as one opens.
//! So the router keeps what is down meanwhile, and what was down as
//! flight's input went away ([`LiveInput`]), and, as flight shows again
//! with nothing in front, hands both to flight once
//! ([`FlightView::take_back_live_input`]), which keeps of them what it
//! reads live and decides which close's rule applies: the router passes
//! physical state only. Leaving flight any other way hands nothing back.
//!
//! - The latched keys flight gives back, pressed under a dialog no site
//!   follows and still down, are replayed to it as fresh presses, in the
//!   order it gives them ([`AppScreen::replay_live_presses`]). B and L are
//!   never among them: no site sets their latches, so flight holds them
//!   as it holds the unlatched keys and reads them at its next pass. A press key
//!   acts at the close's entry point, where the original acts at the next
//!   pass's `_HandlePlayer`; no frame is drawn between the two, as flight
//!   steps only in a tick and the close's tick steps none, so this
//!   crosses no boundary the player sees. A held-polled key acts on the
//!   first step.
//! - Only live input is noted, at [`Screen::input`](nova_view::Screen::input)
//!   past the background's gate: the router's replays (a map's wait, a
//!   movie's queue, the appear's and the menu's holds) went down or up
//!   long before, and noting them again would bring back a key let go.
//!   Repeats count as down.
//! - Only a real loss of input forgets it: the app's
//!   [`Screen::release_keys`](nova_view::Screen::release_keys) (focus lost,
//!   the developer overlay shown), as ours can't read the keys unfocused.
//!   The router's own letting go of the screen below a dialog
//!   ([`AppScreen::let_go_of_keys`]) keeps it.
//! - The hand-back runs at the end of each entry point that can close a
//!   dialog or take off: [`Screen::input`](nova_view::Screen::input),
//!   [`Screen::tick`](nova_view::Screen::tick),
//!   [`Screen::menu_command`](nova_view::Screen::menu_command) and
//!   [`Screen::keyed_click_ended`](nova_view::Screen::keyed_click_ended).
//!   Flight steps only in a tick, and a tick whose dialog closed steps
//!   none, so it lands before flight's next step: no boundary the player
//!   sees lies between.
//!
//! [`FlightView::take_back_live_input`]: nova_view::flight::FlightView::take_back_live_input

use std::collections::HashSet;

use nova_view::flight::LiveKeys;
use nova_view::{Input, Key, MouseButton};

use super::{AppScreen, Side};

/// The keys and the primary button physically down, as live input has
/// told, the keys down at flight's last poll or as its input last went
/// away ([`LiveKeys::down_before`]), and whether flight's input was away
/// when last settled.
#[derive(Clone, Debug, Default)]
pub(super) struct LiveInput {
    keys: HashSet<Key>,
    button: bool,
    away: bool,
    down_before: HashSet<Key>,
}

/// Where flight's input is, for [`LiveInput::settle`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FlightInput {
    /// Taken from flight: a dialog is in front of it, or the spaceport
    /// shows in its place (`_PlayerLandOnStellar`'s `_DoPortDialog`).
    Away,
    /// Flight's own: it shows with nothing in front.
    Back,
    /// Neither: no flight shows, nor the spaceport.
    Gone,
}

impl LiveInput {
    /// Notes what `input` puts down or lets up: a key, repeats as down,
    /// or the primary button; the pointer leaving lets the button go, as
    /// its release won't come.
    pub(super) fn note(&mut self, input: &Input) {
        match *input {
            Input::Key { key, pressed, .. } => {
                if pressed {
                    self.keys.insert(key);
                } else {
                    self.keys.remove(&key);
                }
            }
            Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                ..
            } => self.button = pressed,
            Input::PointerLeft => self.button = false,
            _ => {}
        }
    }

    /// Whether `key` is physically down, as live input has told.
    pub(super) fn is_down(&self, key: Key) -> bool {
        self.keys.contains(&key)
    }

    /// Forgets everything down: the input is lost, and the releases may
    /// never come. What was down before flight's input went away stays,
    /// as a latched key down then read its latch set, whatever happened
    /// after.
    pub(super) fn forget(&mut self) {
        self.keys.clear();
        self.button = false;
    }

    /// Notes what is down as flight polls it: `_HandlePlayer` reads every
    /// key each pass, so a latched key down now has its latch set at the
    /// next close no site follows, until a poll sees it let go.
    pub(super) fn poll(&mut self) {
        self.down_before.clone_from(&self.keys);
    }

    /// Takes where flight's input is `now`: whether it has just come back
    /// from away, so is to be handed back, once. On the step into away,
    /// what is down joins what the last poll read: ours acts on some
    /// latched keys' presses (Y, P, Tab and the rest) rather than at the
    /// poll, so one pressed since was read as the original's poll would.
    pub(super) fn settle(&mut self, now: FlightInput) -> bool {
        let back = self.away && now == FlightInput::Back;
        let away = now == FlightInput::Away;
        if away && !self.away {
            self.down_before.extend(self.keys.iter().copied());
        }
        self.away = away;
        back
    }

    /// What is down for flight to take back ([`LiveKeys`]).
    pub(super) fn taken(&self) -> LiveKeys {
        LiveKeys {
            down: self.keys.iter().copied().collect(),
            down_before: self.down_before.iter().copied().collect(),
            button: self.button,
        }
    }
}

impl AppScreen {
    /// Where flight's input is now. The pod rescue's screen counts as
    /// away, from its fade out to its closing fade's end
    /// ([`FlightView::rescuing`](nova_view::flight::FlightView::rescuing)):
    /// `_HandlePlayer` runs the whole rescue inside one pass
    /// (@0x68de3-0x692ce), so no poll reads a key between the pass the
    /// rescue began on and the next one after it, its text's close
    /// included.
    fn flight_input_now(&self) -> FlightInput {
        let rescuing = self
            .flight
            .as_ref()
            .is_some_and(nova_view::flight::FlightView::rescuing);
        match self.side {
            Side::Flight if self.overlay_in_front().is_some() || rescuing => FlightInput::Away,
            Side::Flight => FlightInput::Back,
            Side::Spaceport => FlightInput::Away,
            Side::MainMenu | Side::Ships | Side::Galaxy => FlightInput::Gone,
        }
    }

    /// Notes what is down as flight polls it, at the start of a tick in
    /// which flight's input is its own ([`LiveInput::poll`]): no input
    /// arrives within a tick, so it is what the tick's key poll reads.
    pub(super) fn note_flight_poll(&mut self) {
        if self.flight_input_now() == FlightInput::Back {
            self.live.poll();
        }
    }

    /// Hands flight what is down if its input has just come back, then
    /// replays the latched keys it gives back as fresh presses (see the
    /// module docs). The hand-back reads what is down as a poll does.
    pub(super) fn settle_live_input(&mut self) {
        if !self.live.settle(self.flight_input_now()) {
            return;
        }
        let Some(flight) = &mut self.flight else {
            return;
        };
        let presses = flight.take_back_live_input(self.live.taken());
        self.live.poll();
        self.replay_live_presses(presses);
    }

    /// Gives flight `presses`, in order, past the router's own handlers
    /// ([`AppScreen::rerouted_input`]), as live input they aren't noted
    /// again: they went down under the dialog. It stops at the first that
    /// takes flight's input away again, a dialog or the spaceport it
    /// opened, and settles that: `_HandlePlayer` reads the rest of the
    /// pass past the site that dialog's command runs (P's @0x70393 before
    /// Y's read @0x703e9), so the presses left latch at its close.
    fn replay_live_presses(&mut self, presses: Vec<Input>) {
        for press in presses {
            let _ = self.rerouted_input(&press);
            let now = self.flight_input_now();
            if now != FlightInput::Back {
                self.live.settle(now);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn button(pressed: bool) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: nova_view::Point::new(1.0, 2.0),
        }
    }

    fn down(live: &LiveInput) -> Vec<Key> {
        let mut keys: Vec<Key> = live.keys.iter().copied().collect();
        keys.sort_by_key(|key| format!("{key:?}"));
        keys
    }

    #[test]
    fn it_hands_back_once_as_flights_input_comes_back() {
        let mut live = LiveInput::default();
        assert!(!live.settle(FlightInput::Back), "never away");
        assert!(!live.settle(FlightInput::Away));
        assert!(!live.settle(FlightInput::Away), "still away");
        assert!(live.settle(FlightInput::Back), "back");
        assert!(!live.settle(FlightInput::Back), "once");
    }

    #[test]
    fn it_hands_back_nothing_when_flight_was_left_another_way() {
        let mut live = LiveInput::default();
        live.settle(FlightInput::Away);
        assert!(!live.settle(FlightInput::Gone));
        assert!(!live.settle(FlightInput::Back), "a new flight");
    }

    #[test]
    fn it_notes_keys_down_and_up_repeats_as_down() {
        let mut live = LiveInput::default();
        live.note(&key(Key::ArrowUp, true));
        live.note(&key(Key::ArrowLeft, true));
        live.note(&key(Key::ArrowLeft, false));
        live.note(&Input::Key {
            key: Key::ArrowRight,
            pressed: true,
            repeat: true,
        });
        assert_eq!(down(&live), [Key::ArrowRight, Key::ArrowUp]);
    }

    #[test]
    fn it_notes_the_primary_button_and_lets_it_go_as_the_pointer_leaves() {
        let mut live = LiveInput::default();
        live.note(&button(true));
        assert!(live.button);
        live.note(&Input::PointerButton {
            button: MouseButton::Right,
            pressed: false,
            at: nova_view::Point::new(1.0, 2.0),
        });
        assert!(live.button, "another button");
        live.note(&button(false));
        assert!(!live.button);
        live.note(&button(true));
        live.note(&Input::PointerLeft);
        assert!(!live.button, "its release won't come");
    }

    /// What `live` holds of `keys` as down before flight's input went
    /// away, in order.
    fn down_before(live: &LiveInput) -> Vec<Key> {
        let mut keys: Vec<Key> = live.taken().down_before.keys().collect();
        keys.sort_by_key(|key| format!("{key:?}"));
        keys
    }

    #[test]
    fn it_keeps_what_was_down_at_flights_last_poll_and_as_its_input_went_away() {
        let (j, one, two) = (Key::KeyJ, Key::Digit1, Key::Digit2);
        let mut live = LiveInput::default();
        live.note(&key(j, true));
        assert_eq!(down_before(&live), [], "not polled yet");
        live.poll();
        live.note(&key(j, false));
        live.note(&key(one, true));
        live.settle(FlightInput::Away);
        assert_eq!(down_before(&live), [one, j], "the poll's and the step's");
        live.note(&key(two, true));
        live.settle(FlightInput::Away);
        assert_eq!(down_before(&live), [one, j], "not while away");
        assert_eq!(down(&live), [one, two]);
        live.forget();
        assert_eq!(down_before(&live), [one, j], "kept");
        live.settle(FlightInput::Back);
        live.poll();
        assert_eq!(down_before(&live), [], "the next poll's");
    }

    #[test]
    fn is_down_reads_what_is_down() {
        let mut live = LiveInput::default();
        assert!(!live.is_down(Key::ArrowUp));
        live.note(&key(Key::ArrowUp, true));
        assert!(live.is_down(Key::ArrowUp));
        assert!(!live.is_down(Key::ArrowDown));
        live.note(&key(Key::ArrowUp, false));
        assert!(!live.is_down(Key::ArrowUp));
    }

    #[test]
    fn forget_lets_go_of_everything_but_keeps_flights_input_away() {
        let mut live = LiveInput::default();
        live.settle(FlightInput::Away);
        live.note(&key(Key::ArrowUp, true));
        live.note(&button(true));
        live.forget();
        assert_eq!(down(&live), []);
        assert!(!live.button);
        assert!(live.settle(FlightInput::Back), "still handed back, empty");
    }
}
