//! The modal host the spaceport's screens share: a dialog open over a
//! screen, and what it was opened for.
//!
//! A screen opens a [`Modal`] dialog over itself with a payload (the row
//! traded, the order counted, the ship named) and hands every input to
//! the [`ModalHost`] while it is open. The host routes the input to the
//! dialog and gathers its sounds, draws it over the screen, forwards
//! [`Screen::cancel_pointer`] and [`Screen::release_keys`], and, once the
//! dialog reports what the player chose, closes and gives the payload
//! back with it. Whatever the outcome means is the screen's own rule.

use crate::draw::DrawList;
use crate::input::Input;
use crate::screen::Screen;
use crate::sound::Sound;
use crate::ui::desc::DescDialog;
use crate::ui::edit_dialog::{Confirm, EditDialog};

/// A dialog a screen hosts modally: a [`Screen`] that reports, once, what
/// the player chose. Reporting an outcome is what closes it.
pub trait Modal: Screen {
    /// What the player chose.
    type Outcome;

    /// What the player chose, once.
    fn take_outcome(&mut self) -> Option<Self::Outcome>;
}

impl<P: Confirm> Modal for EditDialog<P> {
    type Outcome = P::Outcome;

    fn take_outcome(&mut self) -> Option<P::Outcome> {
        EditDialog::take_outcome(self)
    }
}

/// "Desc Dialog" reports Done, once, as it closes: Done, Return or
/// Escape. It has nothing to choose, so its outcome is `()`.
impl Modal for DescDialog {
    type Outcome = ();

    fn take_outcome(&mut self) -> Option<()> {
        DescDialog::take_done(self)
    }
}

/// A dialog `D` open over a screen, and the payload `T` it was opened
/// for; or nothing open.
#[derive(Clone, Debug)]
pub struct ModalHost<T, D> {
    open: Option<(T, D)>,
}

impl<T, D> Default for ModalHost<T, D> {
    /// Nothing open.
    fn default() -> Self {
        Self { open: None }
    }
}

impl<T, D: Modal> ModalHost<T, D> {
    /// Opens `dialog` for `payload`, in place of any dialog open.
    pub fn open(&mut self, payload: T, dialog: D) {
        self.open = Some((payload, dialog));
    }

    /// Whether a dialog is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The dialog open, if one is.
    #[must_use]
    pub fn dialog(&self) -> Option<&D> {
        self.open.as_ref().map(|(_, dialog)| dialog)
    }

    /// Routes `input` to the open dialog and appends its sounds to
    /// `sounds`. Once the dialog reports an outcome, the host closes and
    /// gives `(payload, outcome)`. When the host is closed it does nothing
    /// and gives `None`.
    pub fn input(&mut self, input: &Input, sounds: &mut Vec<Sound>) -> Option<(T, D::Outcome)> {
        let (_, dialog) = self.open.as_mut()?;
        dialog.input(input);
        sounds.extend(dialog.take_sounds());
        let outcome = dialog.take_outcome()?;
        let (payload, _) = self.open.take()?;
        Some((payload, outcome))
    }

    /// Draws the open dialog: call it last, over the screen.
    pub fn draw(&self, list: &mut DrawList) {
        if let Some(dialog) = self.dialog() {
            dialog.draw(list);
        }
    }

    /// Forwards to the open dialog. Returns whether one was open, so the
    /// screen cancels its own dialog's pointer only otherwise.
    pub fn cancel_pointer(&mut self) -> bool {
        let Some((_, dialog)) = &mut self.open else {
            return false;
        };
        dialog.cancel_pointer();
        true
    }

    /// Lets go of the keys held in the open dialog.
    pub fn release_keys(&mut self) {
        if let Some((_, dialog)) = &mut self.open {
            dialog.release_keys();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use std::time::Duration;

    use nova_sim::ShipId;

    use super::*;
    use crate::color::Color;
    use crate::draw::{DrawCommand, fill_rect};
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::screen::ScreenAction;
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonStyle;
    use crate::ui::desc::DONE_ITEM;
    use crate::ui::dialog::{DialogTemplate, ItemSpec, ItemTemplate, Placement};
    use crate::ui::quantity::{PROMPT, QuantityDialog};
    use crate::ui::text_input::TextInputOutcome;

    /// What the fake was given.
    #[derive(Debug, Default)]
    struct Log {
        inputs: Vec<Input>,
        cancels: usize,
        releases: usize,
    }

    /// A dialog answering each input from a script: the outcome it then
    /// reports (if any) and the sounds it makes. It records what it gets
    /// in a log the test keeps, and draws a marker.
    struct FakeDialog<O> {
        outcomes: VecDeque<Option<O>>,
        sounds: VecDeque<Vec<Sound>>,
        reported: Option<O>,
        pending: Vec<Sound>,
        log: Rc<RefCell<Log>>,
    }

    /// Where the fake draws its marker.
    fn marker() -> DrawCommand {
        let mut list = DrawList::new();
        fill_rect(&mut list, marker_bounds(), Color::WHITE);
        list.iter().next().expect("a rect").clone()
    }

    fn marker_bounds() -> Bounds {
        Bounds::at(Point::new(7.0, 11.0), 13.0, 17.0)
    }

    impl<O> FakeDialog<O> {
        /// Answers the inputs with `outcomes`, in order, making no sounds;
        /// gives the log too.
        fn new(outcomes: impl IntoIterator<Item = Option<O>>) -> (Self, Rc<RefCell<Log>>) {
            let log = Rc::new(RefCell::new(Log::default()));
            let fake = Self {
                outcomes: outcomes.into_iter().collect(),
                sounds: VecDeque::new(),
                reported: None,
                pending: Vec::new(),
                log: Rc::clone(&log),
            };
            (fake, log)
        }

        /// Makes `sounds` for the inputs, one list each, in order.
        fn sounding(mut self, sounds: impl IntoIterator<Item = Vec<Sound>>) -> Self {
            self.sounds = sounds.into_iter().collect();
            self
        }
    }

    impl<O> Screen for FakeDialog<O> {
        fn input(&mut self, input: &Input) -> ScreenAction {
            self.log.borrow_mut().inputs.push(*input);
            self.reported = self.outcomes.pop_front().flatten();
            self.pending
                .extend(self.sounds.pop_front().unwrap_or_default());
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {}

        fn draw(&self, list: &mut DrawList) {
            fill_rect(list, marker_bounds(), Color::WHITE);
        }

        fn cancel_pointer(&mut self) {
            self.log.borrow_mut().cancels += 1;
        }

        fn release_keys(&mut self) {
            self.log.borrow_mut().releases += 1;
        }

        fn take_sounds(&mut self) -> Vec<Sound> {
            std::mem::take(&mut self.pending)
        }
    }

    impl<O> Modal for FakeDialog<O> {
        type Outcome = O;

        fn take_outcome(&mut self) -> Option<O> {
            self.reported.take()
        }
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    const UP: Sound = Sound::Ui(UiSound::ButtonUp);
    const DOWN: Sound = Sound::Ui(UiSound::ButtonDown);
    const ALERT: Sound = Sound::Ui(UiSound::Alert);

    #[test]
    fn a_closed_host_takes_no_input_draws_nothing_and_gives_nothing() {
        let mut host = ModalHost::<&str, FakeDialog<u32>>::default();
        assert!(!host.is_open());
        assert!(host.dialog().is_none());
        let mut sounds = vec![UP];
        assert_eq!(host.input(&key(Key::Enter), &mut sounds), None);
        assert_eq!(sounds, [UP]);
        let mut list = DrawList::new();
        host.draw(&mut list);
        assert!(list.is_empty());
        assert!(!host.cancel_pointer());
        host.release_keys();
    }

    #[test]
    fn input_reaches_the_dialog_only_while_it_is_open() {
        let mut host = ModalHost::default();
        let (fake, log) = FakeDialog::new([None, Some(3_u32)]);
        host.open("row", fake);
        assert!(host.is_open());
        let mut sounds = Vec::new();
        host.input(&key(Key::Char('2')), &mut sounds);
        host.input(&key(Key::Enter), &mut sounds);
        host.input(&key(Key::Escape), &mut sounds);
        assert_eq!(log.borrow().inputs, [key(Key::Char('2')), key(Key::Enter)]);
    }

    #[test]
    fn an_input_without_an_outcome_gives_nothing_and_stays_open() {
        let mut host = ModalHost::default();
        let (fake, _log) = FakeDialog::new([None::<u32>]);
        host.open("row", fake);
        assert_eq!(host.input(&key(Key::Char('2')), &mut Vec::new()), None);
        assert!(host.is_open());
        assert!(host.dialog().is_some());
    }

    #[test]
    fn each_outcome_closes_it_and_gives_the_payload_with_it() {
        for count in [3_u32, 0] {
            let mut host = ModalHost::default();
            let (fake, _log) = FakeDialog::new([Some(count)]);
            host.open(("row", 4), fake);
            assert_eq!(
                host.input(&key(Key::Enter), &mut Vec::new()),
                Some((("row", 4), count))
            );
            assert!(!host.is_open(), "{count} closes it");
            assert!(host.dialog().is_none());
        }
    }

    #[test]
    fn a_text_outcome_confirmed_or_cancelled_gives_the_payload_with_it() {
        for outcome in [
            TextInputOutcome::Confirm("Viper".into()),
            TextInputOutcome::Cancel,
        ] {
            let mut host = ModalHost::default();
            let (fake, _log) = FakeDialog::new([Some(outcome.clone())]);
            host.open(ShipId(129), fake);
            assert_eq!(
                host.input(&key(Key::Enter), &mut Vec::new()),
                Some((ShipId(129), outcome))
            );
            assert!(!host.is_open());
        }
    }

    #[test]
    fn once_closed_it_gives_nothing_more() {
        let mut host = ModalHost::default();
        let (fake, _log) = FakeDialog::new([Some(1_u32), Some(2)]);
        host.open("row", fake);
        assert_eq!(
            host.input(&key(Key::Enter), &mut Vec::new()),
            Some(("row", 1))
        );
        let mut sounds = Vec::new();
        assert_eq!(host.input(&key(Key::Enter), &mut sounds), None);
        assert!(sounds.is_empty());
        assert!(!host.cancel_pointer());
    }

    #[test]
    fn it_draws_the_open_dialog_after_what_is_already_drawn() {
        let mut host = ModalHost::default();
        let (fake, _log) = FakeDialog::new([None::<u32>]);
        host.open("row", fake);
        let mut list = DrawList::new();
        list.text("the screen", Point::new(0.0, 0.0), 10.0, None, Color::WHITE);
        let before = list.iter().next().expect("the screen").clone();
        host.draw(&mut list);
        assert_eq!(list.iter().cloned().collect::<Vec<_>>(), [before, marker()]);
    }

    #[test]
    fn cancel_pointer_and_release_keys_reach_the_open_dialog() {
        let mut host = ModalHost::default();
        let (fake, log) = FakeDialog::new([None::<u32>]);
        host.open("row", fake);
        assert!(host.cancel_pointer());
        host.release_keys();
        host.release_keys();
        assert_eq!(log.borrow().cancels, 1);
        assert_eq!(log.borrow().releases, 2);
    }

    #[test]
    fn the_dialogs_sounds_are_appended_in_order_including_on_the_closing_input() {
        let mut host = ModalHost::default();
        let (fake, _log) = FakeDialog::new([None, Some(5_u32)]);
        host.open("row", fake.sounding([vec![DOWN], vec![ALERT, UP]]));
        let mut sounds = vec![UP];
        host.input(&key(Key::Char('9')), &mut sounds);
        assert_eq!(sounds, [UP, DOWN]);
        assert_eq!(host.input(&key(Key::Enter), &mut sounds), Some(("row", 5)));
        assert_eq!(sounds, [UP, DOWN, ALERT, UP]);
    }

    #[test]
    fn an_edit_dialog_reports_its_outcome_through_the_trait() {
        let mut host = ModalHost::default();
        let dialog =
            QuantityDialog::fallback(7, "Buy", PROMPT, ButtonStyle::STOCK, Rc::new(MonoMetrics));
        host.open("row", dialog);
        assert_eq!(
            host.input(&key(Key::Enter), &mut Vec::new()),
            Some(("row", 7))
        );
        assert!(!host.is_open());
    }

    /// "Desc Dialog"'s shape, small: Done (1) at (100, 90), a parked
    /// picture (2) and a text box (3).
    fn desc_template() -> DialogTemplate {
        let item = |bounds, enabled, kind| ItemTemplate {
            bounds,
            enabled,
            kind,
        };
        let rect = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
        DialogTemplate {
            bounds: rect(0.0, 0.0, 200.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(100.0, 90.0, 99.0, 25.0), true, ItemSpec::User),
                item(
                    rect(10.0, 150.0, 20.0, 20.0),
                    false,
                    ItemSpec::Picture(1431),
                ),
                item(rect(10.0, 10.0, 60.0, 36.0), false, ItemSpec::User),
            ],
        }
    }

    fn desc() -> DescDialog {
        DescDialog::new(
            &desc_template(),
            "words",
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
    }

    #[test]
    fn the_desc_dialog_reports_done_once_as_a_modal() {
        for input in [key(Key::Enter), key(Key::Escape)] {
            let mut dialog = desc();
            dialog.input(&key(Key::Down));
            assert_eq!(dialog.take_outcome(), None, "{input:?}");
            dialog.input(&input);
            assert_eq!(dialog.take_outcome(), Some(()), "{input:?}");
            assert_eq!(dialog.take_outcome(), None, "{input:?}");
            assert!(dialog.closed(), "{input:?}");
        }
        let mut dialog = desc();
        let done = dialog
            .dialog()
            .item_bounds(DONE_ITEM)
            .expect("Done")
            .center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: done,
            });
        }
        assert_eq!(dialog.take_outcome(), Some(()));
        assert_eq!(dialog.take_outcome(), None);
        let mut host = ModalHost::default();
        host.open((), desc());
        assert_eq!(host.input(&key(Key::Char('i')), &mut Vec::new()), None);
        assert!(host.is_open());
        assert_eq!(
            host.input(&key(Key::Enter), &mut Vec::new()),
            Some(((), ()))
        );
        assert!(!host.is_open());
    }
}
