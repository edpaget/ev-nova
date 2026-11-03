//! An answered offer's close: `_OfferOneMission`'s wait for
//! [`MISSION_INFO_KEY`]'s release (@0xa35fc-0xa3614), and what lands on
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
use crate::ui::key_wait::KeyWait;
use crate::ui::mission_info::MISSION_INFO_KEY;

/// An offer dialog's close: whether the answer hid it, its disposal's
/// wait for [`MISSION_INFO_KEY`]'s release, and whether it has been
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
            wait: KeyWait::new(MISSION_INFO_KEY, false),
            disposed: false,
        }
    }
}

impl Close {
    /// Whether the answer hid the dialog: it draws nothing.
    pub(super) fn hidden(&self) -> bool {
        self.hidden
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
    /// [`MISSION_INFO_KEY`] is up (true); otherwise it waits for I's
    /// release (false), drawn as it was and inert. Asked again while it
    /// waits, it goes on waiting.
    pub fn dispose(&mut self, key_held: bool) -> bool {
        if self.close.disposed {
            return true;
        }
        if self.close.wait.waiting() {
            return false;
        }
        self.close.wait = KeyWait::new(MISSION_INFO_KEY, key_held);
        self.close.disposed = self.close.wait.close();
        self.close.disposed
    }

    /// Whether its disposal waits for [`MISSION_INFO_KEY`]'s release.
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

    const I: crate::input::Key = MISSION_INFO_KEY;

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

    #[test]
    fn disposed_with_i_down_it_waits_drawn_and_inert_for_is_release() {
        let mut dialog = answered();
        let before = drawn(&dialog);
        assert!(!dialog.dispose(true));
        assert!(dialog.waiting());
        assert!(!dialog.dispose(false), "asked again, it waits on");
        dialog.input(&press(Key::Char('z')));
        dialog.input(&click());
        dialog.input(&press(Key::Enter));
        assert_eq!(drawn(&dialog), before);
        assert!(!dialog.disposed());
        dialog.input(&release(I));
        assert!(dialog.disposed());
        assert_eq!(
            dialog.take_replay(),
            [press(Key::Char('z')), press(Key::Enter)]
        );
    }

    #[test]
    fn losing_focus_ends_the_wait() {
        let mut dialog = answered();
        dialog.dispose(true);
        dialog.input(&press(Key::Char('z')));
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
        assert!(offers.take_choice().is_some());
        offers
    }

    #[test]
    fn an_answer_released_with_i_down_takes_the_venues_input_until_is_release() {
        // Return's accept is hidden, N's refusal drawn: both wait.
        for key in [Key::Enter, Key::Char('n')] {
            let mut offers = answered_by(key);
            offers.release_answered(true);
            assert!(offers.answered().is_some(), "{key:?}: kept");
            assert!(offers.input(&press(Key::Char('z')), &mut Vec::new()));
            assert!(offers.input(&click(), &mut Vec::new()));
            assert_eq!(
                offers.take_unclaimed(),
                [],
                "{key:?}: nothing while it waits"
            );
            assert!(offers.input(&release(I), &mut Vec::new()));
            assert!(offers.answered().is_none(), "{key:?}: gone");
            assert!(!offers.input(&press(Key::Char('y')), &mut Vec::new()));
            assert_eq!(offers.take_unclaimed(), [press(Key::Char('z'))]);
        }
    }

    #[test]
    fn a_hidden_accept_is_kept_hidden_and_waits_the_same() {
        let mut offers = venue();
        let dialog = offers.dialog().expect("open");
        let click = fixture::accept_click_holding(dialog, press(Key::Char('z')));
        for input in &click {
            offers.input(input, &mut Vec::new());
        }
        assert!(offers.take_choice().is_some());
        let hidden = offers.answered().expect("kept");
        assert_eq!(drawn(hidden), [], "hidden");
        offers.release_answered(true);
        assert!(offers.input(&press(Key::Char('x')), &mut Vec::new()));
        offers.input(&release(I), &mut Vec::new());
        assert_eq!(
            offers.take_unclaimed(),
            [press(Key::Char('z')), press(Key::Char('x'))],
            "the answering key's followers, then the wait's"
        );
    }

    #[test]
    fn an_answer_released_with_i_up_goes_at_once() {
        let mut offers = answered_by(Key::Enter);
        offers.release_answered(false);
        assert!(offers.answered().is_none());
        assert!(!offers.input(&press(Key::Char('z')), &mut Vec::new()));
    }

    #[test]
    fn hide_answered_hides_the_kept_answer() {
        let mut offers = answered_by(Key::Char('n'));
        assert!(!drawn(offers.answered().expect("kept")).is_empty());
        offers.hide_answered();
        assert_eq!(drawn(offers.answered().expect("kept")), []);
    }

    #[test]
    fn losing_focus_drops_a_waiting_answer_and_its_queue() {
        let mut offers = answered_by(Key::Enter);
        offers.release_answered(true);
        offers.input(&press(Key::Char('z')), &mut Vec::new());
        offers.release_answer_wait();
        assert!(offers.answered().is_none());
        assert_eq!(offers.take_unclaimed(), []);
        let mut offers = answered_by(Key::Enter);
        offers.release_answer_wait();
        assert!(offers.answered().is_some(), "no wait, kept");
    }
}
