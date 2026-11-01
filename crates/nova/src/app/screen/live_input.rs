//! What is physically down while flight's input is away, handed back to
//! flight as it gets it back. All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! Each frame `_DoPlayGameWork` reads the whole keyboard
//! (`_QuickKeyCheckHandler` @0x43e01, `GetKeys` @0x5d68) and
//! `_HandleMouse` the button (`_Button` @0x3801c-0x3803b), so after any
//! dialog over flight `_HandlePlayer` reads its unlatched keys and the
//! button as they physically are, whenever they went down; the 16 calls
//! of `_SetAllKeyCmdFlags` (@0x61e6d) after a dialog latch only the rest.
//! The dialogs' `FlushEvents` empty only the event queue, which flight
//! never reads (rdm:phase/play-feel-8/phase-3-keys-live-across-a-dialog
//! records each site).
//!
//! Ours has no live read: flight sees only the events it is given, and
//! while a dialog is over it, or the spaceport takes its place, the router
//! gives them to that instead, so flight lets go of its keys as one opens.
//! So the router keeps what is down meanwhile ([`LiveInput`]) and, as
//! flight shows again with nothing in front, hands it to flight once
//! ([`FlightView::take_back_live_input`]), which keeps of it what it reads
//! live. Leaving flight any other way hands nothing back.
//!
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

use nova_view::{Input, Key, MouseButton};

use super::{AppScreen, Side};

/// The keys and the primary button physically down, as live input has
/// told, and whether flight's input was away when last settled.
#[derive(Clone, Debug, Default)]
pub(super) struct LiveInput {
    keys: HashSet<Key>,
    button: bool,
    away: bool,
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

    /// Forgets everything down: the input is lost, and the releases may
    /// never come.
    pub(super) fn forget(&mut self) {
        self.keys.clear();
        self.button = false;
    }

    /// Takes where flight's input is `now`: whether it has just come back
    /// from away, so is to be handed back, once.
    pub(super) fn settle(&mut self, now: FlightInput) -> bool {
        let back = self.away && now == FlightInput::Back;
        self.away = now == FlightInput::Away;
        back
    }
}

impl AppScreen {
    /// Hands flight what is down if its input has just come back (see the
    /// module docs).
    pub(super) fn settle_live_input(&mut self) {
        let now = match self.side {
            Side::Flight if self.overlay_in_front().is_some() => FlightInput::Away,
            Side::Flight => FlightInput::Back,
            Side::Spaceport => FlightInput::Away,
            Side::MainMenu | Side::Ships | Side::Galaxy => FlightInput::Gone,
        };
        if self.live.settle(now)
            && let Some(flight) = &mut self.flight
        {
            flight.take_back_live_input(self.live.keys.iter().copied(), self.live.button);
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
        live.note(&key(Key::Up, true));
        live.note(&key(Key::Left, true));
        live.note(&key(Key::Left, false));
        live.note(&Input::Key {
            key: Key::Right,
            pressed: true,
            repeat: true,
        });
        assert_eq!(down(&live), [Key::Right, Key::Up]);
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

    #[test]
    fn forget_lets_go_of_everything_but_keeps_flights_input_away() {
        let mut live = LiveInput::default();
        live.settle(FlightInput::Away);
        live.note(&key(Key::Up, true));
        live.note(&button(true));
        live.forget();
        assert_eq!(down(&live), []);
        assert!(!live.button);
        assert!(live.settle(FlightInput::Back), "still handed back, empty");
    }
}
