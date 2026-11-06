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
//! So the router keeps what is down meanwhile, and what was down at
//! flight's last poll ([`LiveInput`]), and, as flight shows again with
//! nothing in front, hands both to flight once
//! ([`FlightView::take_back_live_input`]), which keeps of them what it
//! reads live and decides which close's rule applies, and which latched
//! keys it read on their press since that poll: the router passes
//! physical state only. Leaving flight any other way hands nothing back.
//!
//! - The latched keys flight gives back, pressed under a dialog no site
//!   follows and still down, are replayed to it as fresh presses, in the
//!   order it gives them ([`AppScreen::replay_live_presses`]). B, L and
//!   the escort commands F, D, V and C are never among them: no site sets
//!   their latches, so flight holds them as it holds the unlatched keys
//!   and reads them at its next pass. A press key
//!   acts at the close's entry point, where the original acts at the next
//!   pass's `_HandlePlayer`; no frame is drawn between the two, as flight
//!   steps only in a tick and the close's tick steps none, so this
//!   crosses no boundary the player sees. A held-polled key acts on the
//!   first step.
//! - The pad is noted beside the keys, as `Gamepad` state: each button
//!   with the layer it went down under and whether that layer is still
//!   held, and the stick axes, which the pad reports only when they move
//!   (`PadInputs::route`), so flight's let-go would leave a stick held
//!   still centred. Flight reads each button by its exact layer (the
//!   `pad_hand_back` module of `nova_view::flight`). The pad has no
//!   original, and one rule is its own: the press that closed the dialogs
//!   is let go, not handed back. East closes the comm, planet comm,
//!   escort and player info dialogs and the plain dialogs on its press,
//!   and is Land in flight. That press is the last fresh press since
//!   flight's input went away, if it was the pad's and is still down
//!   ([`LiveKeys::pad_taken`]); the press that took the input away (Select
//!   and West's hail) never is. A South that pressed a dialog's button did
//!   so on its release, so it is up by then.
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

use nova_view::flight::LiveKeys;
use nova_view::keymap::{Command, Gamepad, KeysDown, contexts};
use nova_view::{CommandReader, Input, MouseButton, PadButton, SharedBindings};

use super::{AppScreen, Side};

/// The keys, the primary button and the pad physically down, as live
/// input has told, the keys and the pad down at flight's last poll
/// ([`LiveKeys::down_before`], [`LiveKeys::pad_before`]), the last fresh
/// press under the dialogs if it was the pad's, and whether flight's input
/// was away when last settled.
#[derive(Clone, Debug, Default)]
pub(super) struct LiveInput {
    keys: KeysDown,
    button: bool,
    away: bool,
    down_before: KeysDown,
    /// The pad, each button with the layer it went down under, and its
    /// sticks.
    pad: Gamepad,
    /// The pad at flight's last poll.
    pad_before: Gamepad,
    /// The pad button of the last fresh press since flight's input went
    /// away, if that press was the pad's: what a dialog that closed on a
    /// press took ([`LiveKeys::pad_taken`]).
    pad_closing: Option<PadButton>,
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
    /// its release won't come. A pad button, its repeats as down, and its
    /// stick axes are noted as `Gamepad` state through `bindings`' pad
    /// table, each button with the layer it went down under and whether
    /// that layer is still held, so flight reads each one by its exact
    /// layer at the hand-back. A pad button's fresh press is noted as the
    /// press a closing dialog may take ([`LiveKeys::pad_taken`]); a key's
    /// or any pointer button's fresh press clears it, as then the last
    /// press wasn't the pad's.
    pub(super) fn note(&mut self, input: &Input, bindings: &SharedBindings) {
        match *input {
            Input::Key {
                pressed, repeat, ..
            } => {
                self.keys.follow(input);
                if pressed && !repeat {
                    self.pad_closing = None;
                }
            }
            Input::PointerButton {
                button, pressed, ..
            } => {
                if button == MouseButton::Left {
                    self.button = pressed;
                }
                if pressed {
                    self.pad_closing = None;
                }
            }
            Input::PointerLeft => self.button = false,
            Input::PadButton {
                button,
                pressed,
                repeat,
            } => {
                bindings.with(|b| self.pad.follow(input, &b.gamepad));
                if pressed && !repeat {
                    self.pad_closing = Some(button);
                }
            }
            Input::PadAxis { .. } => {
                bindings.with(|b| self.pad.follow(input, &b.gamepad));
            }
            _ => {}
        }
    }

    /// Whether the keys physically down, as live input has told, hold
    /// `command` of the shared play keys (`contexts::KEY_WAIT`'s list)
    /// through `bindings`: a key a close waits on.
    pub(super) fn holds(&self, command: impl Into<Command>, bindings: &SharedBindings) -> bool {
        let mut reader = CommandReader::new(contexts::KEY_WAIT).with_bindings(bindings.clone());
        for press in self.keys.presses() {
            reader.follow(&press);
        }
        reader.held(command)
    }

    /// Forgets everything down: the input is lost, and the releases may
    /// never come. What was down at flight's last poll stays, as a
    /// latched key down then read its latch set, whatever happened after.
    pub(super) fn forget(&mut self) {
        self.keys.let_go();
        self.button = false;
        self.pad.let_go();
    }

    /// Notes what is down as flight polls it: `_HandlePlayer` reads every
    /// key each pass, so a latched key down now has its latch set at the
    /// next close no site follows, until a poll sees it let go.
    pub(super) fn poll(&mut self) {
        self.down_before.clone_from(&self.keys);
        self.pad_before.clone_from(&self.pad);
    }

    /// Takes where flight's input is `now`: whether it has just come back
    /// from away, so is to be handed back, once. What is down on the step
    /// into away doesn't join what the last poll read: of the latched
    /// keys pressed since, flight knows which it read on their press
    /// (`FlightView::take_back_live_input`), and the router passes the
    /// physical state only. The step into away forgets the pad's last
    /// press: the press that took flight's input away (Select and West's
    /// hail) is no dialog's closing press.
    pub(super) fn settle(&mut self, now: FlightInput) -> bool {
        let back = self.away && now == FlightInput::Back;
        let away = now == FlightInput::Away;
        if away && !self.away {
            self.pad_closing = None;
        }
        self.away = away;
        back
    }

    /// What is down for flight to take back ([`LiveKeys`]), the pad's
    /// last fresh press as the closing dialog's if it was the last press
    /// and is still down.
    pub(super) fn taken(&self) -> LiveKeys {
        LiveKeys {
            down: self.keys.clone(),
            down_before: self.down_before.clone(),
            button: self.button,
            pad: self.pad.clone(),
            pad_before: self.pad_before.clone(),
            pad_taken: self.pad_closing.filter(|&button| self.pad.is_down(button)),
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
    use nova_view::keymap::Gamepad;
    use nova_view::{Key, PadAxis, PadButton};

    impl LiveInput {
        /// Notes `input` through the default bindings.
        fn note_key(&mut self, input: &Input) {
            self.note(input, &SharedBindings::default());
        }

        /// Whether `key` is physically down, as live input has told.
        fn is_down(&self, key: Key) -> bool {
            self.keys.is_down(key)
        }
    }

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
        let mut keys: Vec<Key> = live.keys.keys().collect();
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
        live.note_key(&key(Key::ArrowUp, true));
        live.note_key(&key(Key::ArrowLeft, true));
        live.note_key(&key(Key::ArrowLeft, false));
        live.note_key(&Input::Key {
            key: Key::ArrowRight,
            pressed: true,
            repeat: true,
        });
        assert_eq!(down(&live), [Key::ArrowRight, Key::ArrowUp]);
    }

    #[test]
    fn it_notes_the_primary_button_and_lets_it_go_as_the_pointer_leaves() {
        let mut live = LiveInput::default();
        live.note_key(&button(true));
        assert!(live.button);
        live.note_key(&Input::PointerButton {
            button: MouseButton::Right,
            pressed: false,
            at: nova_view::Point::new(1.0, 2.0),
        });
        assert!(live.button, "another button");
        live.note_key(&button(false));
        assert!(!live.button);
        live.note_key(&button(true));
        live.note_key(&Input::PointerLeft);
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
    fn it_keeps_what_was_down_at_flights_last_poll() {
        let (j, one, two) = (Key::KeyJ, Key::Digit1, Key::Digit2);
        let mut live = LiveInput::default();
        live.note_key(&key(j, true));
        assert_eq!(down_before(&live), [] as [Key; 0], "not polled yet");
        live.poll();
        live.note_key(&key(j, false));
        live.note_key(&key(one, true));
        live.settle(FlightInput::Away);
        assert_eq!(down_before(&live), [j], "the poll's, not the step's");
        live.note_key(&key(two, true));
        live.settle(FlightInput::Away);
        assert_eq!(down_before(&live), [j], "not while away");
        assert_eq!(down(&live), [one, two]);
        live.forget();
        assert_eq!(down_before(&live), [j], "kept");
        live.settle(FlightInput::Back);
        live.poll();
        assert_eq!(down_before(&live), [] as [Key; 0], "the next poll's");
    }

    #[test]
    fn is_down_reads_what_is_down() {
        let mut live = LiveInput::default();
        assert!(!live.is_down(Key::ArrowUp));
        live.note_key(&key(Key::ArrowUp, true));
        assert!(live.is_down(Key::ArrowUp));
        assert!(!live.is_down(Key::ArrowDown));
        live.note_key(&key(Key::ArrowUp, false));
        assert!(!live.is_down(Key::ArrowUp));
    }

    fn pad(button: PadButton, pressed: bool) -> Input {
        Input::PadButton {
            button,
            pressed,
            repeat: false,
        }
    }

    fn pad_repeat(button: PadButton) -> Input {
        Input::PadButton {
            button,
            pressed: true,
            repeat: true,
        }
    }

    fn stick(value: f32) -> Input {
        Input::PadAxis {
            axis: PadAxis::LeftY,
            value,
        }
    }

    /// A pad that followed `inputs` through `bindings`, as flight's
    /// reader would.
    fn followed(inputs: &[Input], bindings: &SharedBindings) -> Gamepad {
        let mut pad = Gamepad::default();
        for input in inputs {
            bindings.with(|b| pad.follow(input, &b.gamepad));
        }
        pad
    }

    /// `live` with each of `inputs` noted through `bindings`.
    fn noted(live: &mut LiveInput, inputs: &[Input], bindings: &SharedBindings) {
        for input in inputs {
            live.note(input, bindings);
        }
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "the value noted, unchanged")]
    fn it_notes_pad_buttons_repeats_and_axes_as_the_pad_follows_them() {
        let bindings = SharedBindings::default();
        let inputs = [
            pad(PadButton::West, true),
            pad(PadButton::Select, true),
            pad(PadButton::South, true),
            pad_repeat(PadButton::DPadUp),
            stick(0.7),
            pad(PadButton::Select, false),
        ];
        let mut live = LiveInput::default();
        noted(&mut live, &inputs, &bindings);
        let pad = live.taken().pad;
        assert_eq!(pad, followed(&inputs, &bindings));
        assert!(pad.is_down(PadButton::DPadUp), "a repeat counts as down");
        assert_eq!(pad.axis(PadAxis::LeftY), 0.7);
        assert_ne!(pad, Gamepad::default());
    }

    #[test]
    fn it_keeps_the_pad_down_at_flights_last_poll() {
        let bindings = SharedBindings::default();
        let mut live = LiveInput::default();
        let south = pad(PadButton::South, true);
        live.note(&south, &bindings);
        assert_eq!(live.taken().pad_before, Gamepad::default(), "not polled");
        live.poll();
        noted(
            &mut live,
            &[pad(PadButton::South, false), pad(PadButton::West, true)],
            &bindings,
        );
        live.settle(FlightInput::Away);
        assert_eq!(live.taken().pad_before, followed(&[south], &bindings));
    }

    #[test]
    fn a_pad_press_still_down_at_the_close_is_taken() {
        let bindings = SharedBindings::default();
        let mut live = LiveInput::default();
        live.settle(FlightInput::Away);
        live.note(&pad(PadButton::East, true), &bindings);
        assert_eq!(live.taken().pad_taken, Some(PadButton::East));
        live.note(&pad_repeat(PadButton::DPadUp), &bindings);
        live.settle(FlightInput::Away);
        assert_eq!(
            live.taken().pad_taken,
            Some(PadButton::East),
            "a repeat is no press, and still away"
        );
    }

    #[test]
    fn a_pad_press_let_go_before_the_close_isnt_taken() {
        let bindings = SharedBindings::default();
        let mut live = LiveInput::default();
        live.settle(FlightInput::Away);
        noted(
            &mut live,
            &[pad(PadButton::South, true), pad(PadButton::South, false)],
            &bindings,
        );
        assert_eq!(live.taken().pad_taken, None);
    }

    #[test]
    fn the_press_that_took_flights_input_away_is_never_taken() {
        let bindings = SharedBindings::default();
        let mut live = LiveInput::default();
        // Select and West hail, opening the comm dialog.
        noted(
            &mut live,
            &[pad(PadButton::Select, true), pad(PadButton::West, true)],
            &bindings,
        );
        live.settle(FlightInput::Away);
        assert_eq!(live.taken().pad_taken, None);
    }

    #[test]
    fn a_key_or_click_after_the_pad_press_clears_the_taken_press() {
        let bindings = SharedBindings::default();
        let east = pad(PadButton::East, true);
        let mut live = LiveInput::default();
        noted(&mut live, &[east, key(Key::ArrowUp, true)], &bindings);
        assert_eq!(live.taken().pad_taken, None, "a key's press");
        let mut live = LiveInput::default();
        noted(&mut live, &[east, button(true)], &bindings);
        assert_eq!(live.taken().pad_taken, None, "a click");
        let mut live = LiveInput::default();
        let right_click = Input::PointerButton {
            button: MouseButton::Right,
            pressed: true,
            at: nova_view::Point::new(1.0, 2.0),
        };
        noted(&mut live, &[east, right_click], &bindings);
        assert_eq!(live.taken().pad_taken, None, "any button's");
        let mut live = LiveInput::default();
        let up_repeat = Input::Key {
            key: Key::ArrowUp,
            pressed: true,
            repeat: true,
        };
        noted(
            &mut live,
            &[east, key(Key::ArrowUp, false), up_repeat, button(false)],
            &bindings,
        );
        assert_eq!(
            live.taken().pad_taken,
            Some(PadButton::East),
            "no press: a release or a repeat"
        );
    }

    #[test]
    fn forget_lets_go_of_the_pad_and_its_axes() {
        let bindings = SharedBindings::default();
        let mut live = LiveInput::default();
        noted(
            &mut live,
            &[pad(PadButton::East, true), stick(1.0)],
            &bindings,
        );
        live.forget();
        let taken = live.taken();
        assert_eq!(taken.pad, Gamepad::default());
        assert_eq!(taken.pad_taken, None);
    }

    #[test]
    fn it_hands_back_no_pad_when_flight_was_left_another_way() {
        let bindings = SharedBindings::default();
        let mut live = LiveInput::default();
        live.settle(FlightInput::Away);
        live.note(&pad(PadButton::DPadUp, true), &bindings);
        assert!(!live.settle(FlightInput::Gone));
        assert!(!live.settle(FlightInput::Back), "a new flight");
    }

    #[test]
    fn forget_lets_go_of_everything_but_keeps_flights_input_away() {
        let mut live = LiveInput::default();
        live.settle(FlightInput::Away);
        live.note_key(&key(Key::ArrowUp, true));
        live.note_key(&button(true));
        live.forget();
        assert_eq!(down(&live), [] as [Key; 0]);
        assert!(!live.button);
        assert!(live.settle(FlightInput::Back), "still handed back, empty");
    }
}
