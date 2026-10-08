//! The edit-field modal shell shared by the "Text Input" dialog
//! ([`TextInputDialog`](super::TextInputDialog)) and the quantity dialog
//! ([`QuantityDialog`](super::QuantityDialog)): a dialog laid out from its
//! template, a prompt, one edit text field selected whole, OK and Cancel.
//!
//! The shell routes input (typed text, Space and Backspace to the field,
//! everything else to the dialog), drops the key that opened it
//! ([`EditDialog::flush_typed_key`]), draws the dialog over a dark backdrop
//! with a 1-unit outline, and collects its sounds. OK is the default item,
//! so Return presses it, and there is no cancel item, so Escape does
//! nothing. What OK and Cancel make of the field is each dialog's own
//! rule, its [`Confirm`] policy: a refusal beeps (the alert,
//! [`UiSound::Alert`]) and leaves the dialog open.

use std::rc::Rc;
use std::time::Duration;

use crate::draw::{DrawList, fill_rect};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Role, outline};
use super::prefs::{BACKDROP, BORDER};
use super::text_field::TextField;

/// What OK and Cancel make of the field: each dialog's own rule.
pub trait Confirm: Clone {
    /// What the player chose.
    type Outcome: Clone + std::fmt::Debug;

    /// OK with `field` as typed. `Some` confirms. `None` refuses: the
    /// policy has already reset or reselected the field, and the shell
    /// beeps ([`UiSound::Alert`]).
    fn confirm(&self, field: &mut TextField) -> Option<Self::Outcome>;

    /// What a click on Cancel gives.
    fn cancelled(&self) -> Self::Outcome;
}

/// The items the shell drives, and the `DLOG` ID its errors name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditItems {
    /// The dialog's `DLOG` (and `DITL`) ID.
    pub id: i16,
    /// OK's item: the default, which the policy confirms.
    pub ok: usize,
    /// The prompt's item.
    pub prompt: usize,
    /// The edit text item.
    pub field: usize,
    /// Cancel's item.
    pub cancel: usize,
}

/// A prompt, one edit text field, and OK or Cancel, with the policy `P`
/// deciding what they give.
#[derive(Clone)]
pub struct EditDialog<P: Confirm> {
    dialog: Dialog,
    field: TextField,
    metrics: Rc<dyn TextMetrics>,
    items: EditItems,
    policy: P,
    outcome: Option<P::Outcome>,
    /// Whether the next input is dropped if it is typed text.
    flushing: bool,
    sounds: Vec<Sound>,
}

impl<P: Confirm> EditDialog<P> {
    /// The dialog `template` with its user items given `roles`, asking
    /// `prompt` in `items.prompt`, its field holding `text`, selected
    /// whole, and `policy` deciding OK and Cancel; its buttons labelled in
    /// `style` and its text measured by `metrics`.
    ///
    /// # Errors
    ///
    /// When the template's `items.field` is not edit text, or it has no
    /// `items.cancel`.
    #[allow(clippy::too_many_arguments)]
    pub fn laid_out(
        template: &DialogTemplate,
        items: EditItems,
        roles: &[(usize, Role)],
        prompt: &str,
        text: &str,
        policy: P,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        let editable = matches!(
            template.items.get(items.field - 1),
            Some(ItemTemplate {
                kind: ItemSpec::EditText(_),
                ..
            })
        );
        if !editable || template.items.len() < items.cancel {
            return Err(format!(
                "DITL {} has no edit text item {} and Cancel item {}",
                items.id, items.field, items.cancel
            ));
        }
        let mut dialog = Dialog::new(template, roles, Rc::clone(&metrics))
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(items.ok))
            .with_cancel(None);
        dialog.set_text(items.prompt, prompt);
        let field_rect = dialog.item_bounds(items.field).expect("checked above");
        Ok(Self {
            dialog,
            field: TextField::with_text(field_rect, usize::MAX, text),
            metrics,
            items,
            policy,
            outcome: None,
            flushing: false,
            sounds: Vec::new(),
        })
    }

    /// Drops the next input if it is typed text, as the original consumes
    /// the keyDown that opened the dialog (or flushes the events pending
    /// as it opens): the key that opened it then types nothing into it.
    /// Any other input ends the flush.
    pub fn flush_typed_key(&mut self) {
        self.flushing = true;
    }

    /// The field.
    #[must_use]
    pub fn field(&self) -> &TextField {
        &self.field
    }

    /// The field, to change it.
    #[cfg(test)]
    pub(crate) fn field_mut(&mut self) -> &mut TextField {
        &mut self.field
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The policy deciding OK and Cancel.
    #[must_use]
    pub fn policy(&self) -> &P {
        &self.policy
    }

    /// What the player chose, once.
    pub fn take_outcome(&mut self) -> Option<P::Outcome> {
        self.outcome.take()
    }

    /// The outcome not yet taken, for Debug.
    pub(crate) fn outcome(&self) -> Option<&P::Outcome> {
        self.outcome.as_ref()
    }

    /// OK: the policy confirms, or refuses with a beep.
    fn confirm(&mut self) {
        match self.policy.confirm(&mut self.field) {
            Some(outcome) => self.outcome = Some(outcome),
            None => self.sounds.push(Sound::Ui(UiSound::Alert)),
        }
    }
}

impl<P: Confirm> Screen for EditDialog<P> {
    /// Typed characters, Backspace and Space go to the field (Space types
    /// only a space); after [`flush_typed_key`](Self::flush_typed_key), the
    /// next typed character is dropped. Everything else goes to the
    /// dialog: OK (or Return) asks the policy, and Cancel declines. It
    /// never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if std::mem::take(&mut self.flushing) && matches!(input, Input::Text(_)) {
            return ScreenAction::None;
        }
        let typing = matches!(
            input,
            Input::Text(_)
                | Input::Key {
                    key: Key::Backspace | Key::Space,
                    ..
                }
        );
        if typing {
            self.field.input(input);
            return ScreenAction::None;
        }
        let event = self.dialog.input(input);
        self.sounds.extend(self.dialog.take_sound().map(Sound::Ui));
        match event {
            Some(DialogEvent::Item(item)) if item == self.items.ok => self.confirm(),
            Some(DialogEvent::Item(item)) if item == self.items.cancel => {
                self.outcome = Some(self.policy.cancelled());
            }
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The backdrop and its outline, the dialog, and the field.
    fn draw(&self, list: &mut DrawList) {
        let bounds = self.dialog.bounds();
        fill_rect(list, bounds, BACKDROP);
        outline(list, bounds, BORDER);
        self.dialog.draw(list);
        self.field.draw(&*self.metrics, list);
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }

    /// OK's and Cancel's sounds as they are clicked, and the alert as OK
    /// is refused, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;
    use crate::geometry::{Bounds, Point};
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::Placement;

    /// A policy answering OK from a script, and recording each text it
    /// was asked to confirm. A refusal sets the field to "reset".
    #[derive(Clone, Default)]
    struct Scripted {
        answers: Rc<RefCell<VecDeque<Option<String>>>>,
        asked: Rc<RefCell<Vec<String>>>,
    }

    impl Scripted {
        fn answering(answers: &[Option<&str>]) -> Self {
            let policy = Self::default();
            policy
                .answers
                .borrow_mut()
                .extend(answers.iter().map(|answer| answer.map(str::to_owned)));
            policy
        }
    }

    impl Confirm for Scripted {
        type Outcome = String;

        fn confirm(&self, field: &mut TextField) -> Option<String> {
            self.asked.borrow_mut().push(field.text().to_owned());
            let answer = self.answers.borrow_mut().pop_front().flatten();
            if answer.is_none() {
                field.set_text("reset");
            }
            answer
        }

        fn cancelled(&self) -> String {
            "cancelled".to_owned()
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    fn item(bounds: Bounds, enabled: bool, kind: ItemSpec) -> ItemTemplate {
        ItemTemplate {
            bounds,
            enabled,
            kind,
        }
    }

    /// Cancel (1), the prompt (2), OK (3) and the field (4), fixed at
    /// (0, 0): an order neither dialog uses, so the shell follows `ITEMS`.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 200.0, 80.0),
            placement: Placement::Fixed,
            items: vec![
                item(
                    rect(10.0, 50.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("Cancel".into()),
                ),
                item(
                    rect(6.0, 8.0, 100.0, 16.0),
                    false,
                    ItemSpec::StaticText(String::new()),
                ),
                item(
                    rect(110.0, 50.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("OK".into()),
                ),
                item(
                    rect(110.0, 8.0, 60.0, 16.0),
                    true,
                    ItemSpec::EditText(String::new()),
                ),
            ],
        }
    }

    const ITEMS: EditItems = EditItems {
        id: 77,
        ok: 3,
        prompt: 2,
        field: 4,
        cancel: 1,
    };

    fn dialog_with(policy: Scripted) -> EditDialog<Scripted> {
        EditDialog::laid_out(
            &template(),
            ITEMS,
            &[],
            "Asked:",
            "abc",
            policy,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    fn dialog() -> EditDialog<Scripted> {
        dialog_with(Scripted::default())
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click(dialog: &mut EditDialog<Scripted>, item: usize) -> Vec<Sound> {
        let at = dialog.dialog().item_bounds(item).expect("an item").center();
        let mut sounds = Vec::new();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
            sounds.extend(dialog.take_sounds());
        }
        sounds
    }

    const ALERT: Sound = Sound::Ui(UiSound::Alert);

    #[test]
    fn it_opens_with_the_prompt_and_the_text_selected_whole() {
        let dialog = dialog();
        assert_eq!(dialog.field().text(), "abc");
        assert_eq!(dialog.field().selection(), 3);
        assert_eq!(dialog.field().rect(), rect(110.0, 8.0, 60.0, 16.0));
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        let texts: Vec<_> = list
            .iter()
            .filter_map(|command| match command {
                crate::draw::DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["Cancel", "Asked:", "OK", "abc"]);
    }

    #[test]
    fn ok_and_return_ask_the_policy_with_the_fields_text() {
        let policy = Scripted::answering(&[Some("first"), Some("second")]);
        let mut dialog = dialog_with(policy.clone());
        assert_eq!(
            click(&mut dialog, ITEMS.ok),
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
        assert_eq!(dialog.take_outcome(), Some("first".to_owned()));
        dialog.input(&Input::Text('d'));
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some("second".to_owned()));
        assert_eq!(*policy.asked.borrow(), ["abc", "d"]);
    }

    #[test]
    fn a_refusal_beeps_once_and_leaves_no_outcome() {
        let policy = Scripted::answering(&[None]);
        let mut dialog = dialog_with(policy.clone());
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_sounds(), [ALERT]);
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(dialog.field().text(), "reset", "the policy sets the field");
    }

    #[test]
    fn cancel_gives_the_policys_cancel_without_asking_it() {
        let policy = Scripted::answering(&[Some("never")]);
        let mut dialog = dialog_with(policy.clone());
        click(&mut dialog, ITEMS.cancel);
        assert_eq!(dialog.take_outcome(), Some("cancelled".to_owned()));
        assert!(policy.asked.borrow().is_empty());
    }

    #[test]
    fn escape_and_other_items_do_nothing() {
        let policy = Scripted::answering(&[Some("never")]);
        let mut dialog = dialog_with(policy.clone());
        dialog.input(&key(Key::Escape));
        click(&mut dialog, ITEMS.prompt);
        click(&mut dialog, ITEMS.field);
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(dialog.field().text(), "abc");
        assert!(policy.asked.borrow().is_empty());
    }

    #[test]
    fn typing_space_and_backspace_reach_the_field() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Char('x')));
        dialog.input(&Input::Text('x'));
        dialog.input(&key(Key::Space));
        dialog.input(&Input::Text(' '));
        assert_eq!(dialog.field().text(), "x ");
        dialog.input(&key(Key::Backspace));
        assert_eq!(dialog.field().text(), "x");
    }

    #[test]
    fn a_flush_drops_exactly_the_next_typed_character() {
        let mut dialog = dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::Text('b'));
        assert_eq!(dialog.field().text(), "abc", "flushed");
        dialog.input(&Input::Text('K'));
        assert_eq!(dialog.field().text(), "K");
        let mut dialog = self::dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        dialog.input(&Input::Text('K'));
        assert_eq!(dialog.field().text(), "K", "the flush ended");
    }

    #[test]
    fn it_draws_a_backdrop_the_dialog_and_the_field() {
        let mut dialog = dialog();
        dialog.tick(Duration::from_secs(1));
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        let bounds = dialog.dialog().bounds();
        let mut expected = DrawList::new();
        fill_rect(&mut expected, bounds, BACKDROP);
        outline(&mut expected, bounds, BORDER);
        dialog.dialog().draw(&mut expected);
        dialog.field().draw(&MonoMetrics, &mut expected);
        assert_eq!(list, expected);
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click() {
        let policy = Scripted::answering(&[Some("never")]);
        let mut dialog = dialog_with(policy);
        let at = dialog.dialog().item_bounds(ITEMS.ok).expect("OK").center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        dialog.input(&button(true));
        dialog.cancel_pointer();
        dialog.input(&button(false));
        assert_eq!(dialog.take_outcome(), None);
    }

    #[test]
    fn a_template_without_its_field_or_cancel_is_an_error() {
        let error = Some("DITL 77 has no edit text item 4 and Cancel item 1".to_owned());
        let build = |template: &DialogTemplate, items: EditItems| {
            EditDialog::laid_out(
                template,
                items,
                &[],
                "",
                "",
                Scripted::default(),
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics),
            )
            .err()
        };
        let mut plain = template();
        plain.items[ITEMS.field - 1].kind = ItemSpec::StaticText(String::new());
        assert_eq!(build(&plain, ITEMS), error);
        let mut short = template();
        short.items.truncate(3);
        assert_eq!(build(&short, ITEMS), error);
        let past = EditItems { cancel: 5, ..ITEMS };
        assert_eq!(
            build(&template(), past),
            Some("DITL 77 has no edit text item 4 and Cancel item 5".to_owned())
        );
        let exact = EditItems { cancel: 4, ..ITEMS };
        assert_eq!(build(&template(), exact), None, "4 items reach item 4");
    }

    #[test]
    fn the_outcome_is_taken_once() {
        let mut dialog = dialog_with(Scripted::answering(&[Some("ok")]));
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.outcome(), Some(&"ok".to_owned()));
        assert_eq!(dialog.take_outcome(), Some("ok".to_owned()));
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(dialog.policy().asked.borrow().len(), 1);
    }

    #[test]
    fn the_field_can_be_changed_by_hand() {
        let mut dialog = dialog();
        dialog.field_mut().set_text("zz");
        assert_eq!(dialog.field().text(), "zz");
    }
}
