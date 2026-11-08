//! An answered offer's close: `_OfferOneMission`'s wait for the mission
//! info key's release (@0xa35fc-0xa3614), the key
//! [`PlayCommand::MissionInfo`](crate::keymap::PlayCommand::MissionInfo)
//! is bound to in the dialog's bindings, and what lands on
//! the release (@0xa361e-0xa3670). All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! Every end of `_OfferOneMission` reaches the wait. While I is down the
//! dialog stays as it was, drawn or hidden, and inert, and its venue takes
//! nothing; the keys typed meanwhile stay queued (the `key_wait` module).
//! I's release disposes of the dialog (`DisposeDialogNoFlash` @0xa361e),
//! fades the screen in, and flushes only mouse events, so the venue runs
//! the queue as its next tick begins, after what the answering key left
//! ([`VenueOffers::take_unclaimed`]). An answer with I up goes at once. The
//! wait plays no sound.

use super::{OfferDialog, VenueOffers};
use crate::input::Input;
use crate::keymap::SharedBindings;
use crate::ui::key_wait::KeyWait;
use crate::ui::mission_info::mission_info_key;

/// An offer dialog's close: whether the answer hid it, its disposal's
/// wait for the mission info key's release, and whether it has been
/// disposed of.
#[derive(Clone, Debug)]
pub(super) struct Close {
    hidden: bool,
    wait: KeyWait,
    disposed: bool,
}

impl Default for Close {
    fn default() -> Self {
        Self {
            hidden: false,
            wait: KeyWait::new(mission_info_key(&SharedBindings::default()), false),
            disposed: false,
        }
    }
}

impl Close {
    /// Whether the answer hid the dialog: it draws nothing.
    pub(super) fn hidden(&self) -> bool {
        self.hidden
    }

    /// Waits on the mission info key as `bindings` bind it, reading the
    /// live keys of what it queues through them.
    pub(super) fn set_bindings(&mut self, bindings: &SharedBindings) {
        self.wait.set_key(mission_info_key(bindings));
        self.wait.set_bindings(bindings.clone());
    }
}

impl OfferDialog {
    /// Hides the answered dialog: it draws nothing, and is disposed of as
    /// any answer is (an accept from the dialog with a refuse button,
    /// @0xa2df3, or a refusal that showed a `RefuseText`, @0xa2e8e).
    pub fn hide(&mut self) {
        self.close.hidden = true;
    }

    /// Disposes of the answered dialog: at once, when `key_held` says
    /// the mission info key is up (true); otherwise it waits for its
    /// release (false), drawn as it was and inert. Asked again while it
    /// waits, it goes on waiting.
    pub fn dispose(&mut self, key_held: bool) -> bool {
        if self.close.disposed {
            return true;
        }
        if self.close.wait.waiting() {
            return false;
        }
        let bindings = self.keys.bindings().clone();
        self.close.wait = KeyWait::new(mission_info_key(&bindings), key_held);
        self.close.wait.set_bindings(bindings);
        self.close.disposed = self.close.wait.close();
        self.close.disposed
    }

    /// Whether its disposal waits for the mission info key's release.
    #[must_use]
    pub fn waiting(&self) -> bool {
        self.close.wait.waiting()
    }

    /// Whether it has been disposed of.
    #[must_use]
    pub fn disposed(&self) -> bool {
        self.close.disposed
    }

    /// The keys typed during the wait, for the venue, once.
    pub fn take_replay(&mut self) -> Vec<Input> {
        self.close.wait.take_replay()
    }

    /// Gives `input` to the wait, if it waits: whether it took it. I's
    /// release disposes of the dialog.
    pub(super) fn close_input(&mut self, input: &Input) -> bool {
        if !self.close.wait.waiting() {
            return false;
        }
        if self.close.wait.input(input) {
            self.close.disposed = true;
        }
        true
    }

    /// Focus is lost: a wait ends, its queue dropped, and the dialog is
    /// disposed of.
    pub(super) fn release_close_wait(&mut self) {
        if self.close.wait.release() {
            self.close.disposed = true;
        }
    }
}

impl VenueOffers {
    /// Disposes of the offer dialog answered, if any, as
    /// [`OfferDialog::dispose`] does with I down as `key_held` says:
    /// dropped once disposed, kept while it waits. Its holder calls this
    /// once the mission dialogs over it have all closed.
    pub(crate) fn release_answered(&mut self, key_held: bool) {
        if let Some(dialog) = &mut self.answered
            && dialog.dispose(key_held)
        {
            self.answered = None;
        }
    }

    /// Hides the offer dialog answered, if any, for a refusal that showed
    /// a `RefuseText` (@0xa2e8e).
    pub(crate) fn hide_answered(&mut self) {
        if let Some(dialog) = &mut self.answered {
            dialog.hide();
        }
    }

    /// Whether the answered dialog's disposal waits for I's release.
    pub(super) fn answer_waits(&self) -> bool {
        self.answered.as_ref().is_some_and(OfferDialog::waiting)
    }

    /// Gives `input` to the answered dialog while its disposal waits:
    /// whether it took it. On I's release it is dropped, and what was
    /// typed meanwhile follows what its answer left
    /// ([`VenueOffers::take_unclaimed`]).
    pub(super) fn answer_input(&mut self, input: &Input) -> bool {
        let Some(dialog) = &mut self.answered else {
            return false;
        };
        if !dialog.close_input(input) {
            return false;
        }
        if dialog.disposed() {
            let replay = dialog.take_replay();
            self.unclaimed.extend(replay);
            self.answered = None;
        }
        true
    }

    /// Focus is lost: the answered dialog's wait ends, if it waits, and
    /// it is dropped with nothing typed meanwhile.
    pub(crate) fn release_answer_wait(&mut self) {
        if let Some(dialog) = &mut self.answered {
            dialog.release_close_wait();
            if dialog.disposed() {
                self.answered = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::super::fixture;
    use super::*;
    use crate::draw::{DrawCommand, DrawList};
    use crate::input::{Key, MouseButton};
    use crate::screen::Screen;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonStyle;

    const I: crate::input::Key = crate::keymap::test_keys::MISSION_INFO_KEY;

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    fn offer() -> OfferDialog {
        OfferDialog::new(
            Ok((fixture::template(), Rc::new(MonoMetrics))),
            fixture::prompt(),
            ButtonStyle::STOCK,
        )
    }

    fn drawn(dialog: &OfferDialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn answered() -> OfferDialog {
        let mut dialog = offer();
        dialog.input(&press(Key::Enter));
        assert!(dialog.take_choice().is_some(), "answered");
        dialog
    }

    fn click() -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: crate::geometry::Point::new(10.0, 10.0),
        }
    }

    #[test]
    fn a_hidden_dialog_draws_nothing_and_takes_nothing() {
        let mut dialog = offer();
        assert!(!drawn(&dialog).is_empty());
        dialog.hide();
        assert_eq!(drawn(&dialog), []);
        dialog.input(&press(Key::Enter));
        assert!(dialog.take_choice().is_some(), "still the dialog's own");
        assert_eq!(dialog.take_replay(), []);
    }

    #[test]
    fn disposed_with_i_up_it_goes_at_once() {
        let mut dialog = answered();
        assert!(dialog.dispose(false));
        assert!(dialog.disposed());
        assert!(!dialog.waiting());
        assert!(dialog.dispose(true), "and stays disposed");
    }

    /// The wait is on the key the mission info key's command is bound to
    /// in the bindings the dialog is given: rebound to F2, F2's release
    /// ends it and I's does not.
    #[test]
    fn the_wait_is_on_the_mission_info_key_s_binding() {
        use crate::keymap::{Bindings, KeyCombo, PlayCommand, SharedBindings};
        let bindings = SharedBindings::new(
            Bindings::default().rebound(PlayCommand::MissionInfo, KeyCombo::key(Key::F2)),
        );
        let mut dialog = offer().with_bindings(bindings);
        dialog.input(&press(Key::Enter));
        assert!(dialog.take_choice().is_some(), "answered");
        assert!(!dialog.dispose(true));
        dialog.input(&release(I));
        assert!(!dialog.disposed(), "I's release is no longer waited on");
        dialog.input(&release(Key::F2));
        assert!(dialog.disposed());
    }

    /// Bindings handed to a dialog already waiting reach its close wait:
    /// it waits on the key they bind the mission info key's command to.
    #[test]
    fn a_waiting_close_reads_bindings_handed_to_it() {
        use crate::keymap::{Bindings, KeyCombo, PlayCommand, SharedBindings};
        let mut dialog = answered();
        assert!(!dialog.dispose(true));
        let mut dialog = dialog.with_bindings(SharedBindings::new(
            Bindings::default().rebound(PlayCommand::MissionInfo, KeyCombo::key(Key::F2)),
        ));
        assert!(dialog.waiting());
        dialog.input(&release(I));
        assert!(!dialog.disposed(), "I's release is no longer waited on");
        dialog.input(&release(Key::F2));
        assert!(dialog.disposed());
    }

    #[test]
    fn disposed_with_i_down_it_waits_drawn_and_inert_for_is_release() {
        let mut dialog = answered();
        let before = drawn(&dialog);
        assert!(!dialog.dispose(true));
        assert!(dialog.waiting());
        assert!(!dialog.dispose(false), "asked again, it waits on");
        dialog.input(&press(Key::KeyZ));
        dialog.input(&click());
        dialog.input(&press(Key::Enter));
        assert_eq!(drawn(&dialog), before);
        assert!(!dialog.disposed());
        dialog.input(&release(I));
        assert!(dialog.disposed());
        assert_eq!(dialog.take_replay(), [press(Key::KeyZ), press(Key::Enter)]);
    }

    #[test]
    fn losing_focus_ends_the_wait() {
        let mut dialog = answered();
        dialog.dispose(true);
        dialog.input(&press(Key::KeyZ));
        dialog.release_keys();
        assert!(dialog.disposed());
        assert_eq!(dialog.take_replay(), []);
    }

    fn venue() -> VenueOffers {
        let mut offers = VenueOffers::asking(fixture::templates());
        offers.open(fixture::prompt(), Rc::new(MonoMetrics), ButtonStyle::STOCK);
        offers
    }

    /// [`venue`] with its offer answered by `key` and the choice taken.
    fn answered_by(key: Key) -> VenueOffers {
        let mut offers = venue();
        offers.input(&press(key), &mut Vec::new());
        if let Some(typed) = crate::input::key_letter(key) {
            offers.input(&Input::Text(typed), &mut Vec::new());
        }
        assert!(offers.take_choice().is_some());
        offers
    }

    #[test]
    fn an_answer_released_with_i_down_takes_the_venues_input_until_is_release() {
        // Return's accept is hidden, N's refusal drawn: both wait.
        for key in [Key::Enter, Key::KeyN] {
            let mut offers = answered_by(key);
            offers.release_answered(true);
            assert!(offers.answered().is_some(), "{key:?}: kept");
            assert!(offers.input(&press(Key::KeyZ), &mut Vec::new()));
            assert!(offers.input(&click(), &mut Vec::new()));
            assert_eq!(
                offers.take_unclaimed(),
                [],
                "{key:?}: nothing while it waits"
            );
            assert!(offers.input(&release(I), &mut Vec::new()));
            assert!(offers.answered().is_none(), "{key:?}: gone");
            assert!(!offers.input(&press(Key::KeyY), &mut Vec::new()));
            assert!(!offers.input(&Input::Text('y'), &mut Vec::new()));
            assert_eq!(offers.take_unclaimed(), [press(Key::KeyZ)]);
        }
    }

    #[test]
    fn a_hidden_accept_is_kept_hidden_and_waits_the_same() {
        let mut offers = venue();
        let dialog = offers.dialog().expect("open");
        let click = fixture::accept_click_holding(dialog, press(Key::KeyZ));
        for input in &click {
            offers.input(input, &mut Vec::new());
        }
        assert!(offers.take_choice().is_some());
        let hidden = offers.answered().expect("kept");
        assert_eq!(drawn(hidden), [], "hidden");
        offers.release_answered(true);
        assert_eq!(offers.take_unclaimed(), [], "the answer's Z waits too");
        assert!(offers.input(&press(Key::KeyX), &mut Vec::new()));
        offers.input(&release(I), &mut Vec::new());
        assert_eq!(
            offers.take_unclaimed(),
            [press(Key::KeyZ), press(Key::KeyX)],
            "the answering key's followers, then the wait's"
        );
    }

    #[test]
    fn an_answer_released_with_i_up_goes_at_once() {
        let mut offers = answered_by(Key::Enter);
        offers.release_answered(false);
        assert!(offers.answered().is_none());
        assert!(!offers.input(&press(Key::KeyZ), &mut Vec::new()));
    }

    #[test]
    fn hide_answered_hides_the_kept_answer() {
        let mut offers = answered_by(Key::KeyN);
        assert!(!drawn(offers.answered().expect("kept")).is_empty());
        offers.hide_answered();
        assert_eq!(drawn(offers.answered().expect("kept")), []);
    }

    #[test]
    fn losing_focus_drops_a_waiting_answer_and_its_queue() {
        let mut offers = answered_by(Key::Enter);
        offers.release_answered(true);
        offers.input(&press(Key::KeyZ), &mut Vec::new());
        offers.release_answer_wait();
        assert!(offers.answered().is_none());
        assert_eq!(offers.take_unclaimed(), []);
        let mut offers = answered_by(Key::Enter);
        offers.release_answer_wait();
        assert!(offers.answered().is_some(), "no wait, kept");
    }
}

#[cfg(test)]
mod timer_tests {
    use std::rc::Rc;
    use std::time::Duration;

    use super::super::fixture;
    use super::*;
    use crate::input::Key;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonStyle;

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    #[test]
    fn the_venue_asks_for_no_offer_while_an_answer_waits_for_i() {
        let mut offers = VenueOffers::asking(fixture::templates());
        assert!(offers.take_request());
        offers.open(fixture::prompt(), Rc::new(MonoMetrics), ButtonStyle::STOCK);
        offers.input(&key(Key::KeyN, true), &mut Vec::new());
        offers.input(&Input::Text('n'), &mut Vec::new());
        assert!(offers.take_choice().is_some());
        offers.release_answered(true);
        offers.rearm(Duration::from_millis(100));
        offers.tick(Duration::from_millis(200), false);
        assert!(!offers.take_request(), "not while it waits");
        assert!(offers.dialog().is_none());
        offers.input(
            &key(crate::keymap::test_keys::MISSION_INFO_KEY, false),
            &mut Vec::new(),
        );
        offers.tick(Duration::from_millis(1), false);
        assert!(
            offers.take_request(),
            "the deadline counted on: asked at once"
        );
    }
}
