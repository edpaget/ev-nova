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
//!
//! The field always has the focus, so the shell is a text-entry host
//! ([`entry_keyboard`](super::entry_keyboard)): with the keyboard, the
//! mouse or touch with a soft keyboard it asks for the system's entry
//! from its first frame; under a pad it opens asking for nothing, so one
//! South on OK still takes the offered value, and South on the field
//! raises the on-screen keyboard; touch without a soft keyboard shows
//! the on-screen keyboard at once. The field is a pad target
//! (rdm:phase/key-bindings/phase-17-text-entry-port), which supersedes
//! phase 9's "text fields are no targets": a click on it, or South,
//! focuses it again, which starts the system's entry afresh.

use std::rc::Rc;
use std::time::Duration;

use crate::draw::{DrawList, fill_rect};
use crate::geometry::Bounds;
use crate::input::Input;
use crate::keymap::SharedBindings;
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Role, outline};
use super::entry_keyboard::{self, EntryHost, EntryKeyboard};
use super::prefs::{BACKDROP, BORDER};
use super::text_field::{TextField, is_text_entry};

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
    /// The on-screen keyboard, and the ask for the system's entry.
    keyboard: EntryKeyboard,
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
        dialog.set_pad_included(vec![items.field]);
        let field_rect = dialog.item_bounds(items.field).expect("checked above");
        let mut keyboard = EntryKeyboard::new();
        keyboard.focus_field();
        Ok(Self {
            dialog,
            field: TextField::with_text(field_rect, usize::MAX, text),
            metrics,
            items,
            policy,
            outcome: None,
            flushing: false,
            sounds: Vec::new(),
            keyboard,
        })
    }

    /// The dialog reading its keys (Return for OK) through `bindings`.
    #[must_use]
    pub fn with_bindings(mut self, bindings: SharedBindings) -> Self {
        self.dialog.set_bindings(bindings);
        self
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
    /// only a space): text entry, which no key map rebinds
    /// ([`is_text_entry`]). After [`flush_typed_key`](Self::flush_typed_key),
    /// the next typed character is dropped. Everything else goes to the
    /// dialog: OK (or Confirm, Return) asks the policy, and Cancel
    /// declines. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(action) = entry_keyboard::intercept(self, input) {
            return action;
        }
        // The surroundings are no player input: they leave the flush of
        // the opening key's character for the input that follows.
        if let Input::Surroundings(_) = input {
            self.dialog.input(input);
            return ScreenAction::None;
        }
        if std::mem::take(&mut self.flushing) && matches!(input, Input::Text(_)) {
            return ScreenAction::None;
        }
        if is_text_entry(input) {
            self.field.input(input);
            return ScreenAction::None;
        }
        let event = self.dialog.input(input);
        match event {
            Some(DialogEvent::Item(item)) if item == self.items.ok => self.confirm(),
            Some(DialogEvent::Item(item)) if item == self.items.cancel => {
                self.outcome = Some(self.policy.cancelled());
            }
            Some(DialogEvent::Item(item)) if item == self.items.field => {
                self.keyboard.field_pressed(self.dialog.pad_highlight());
            }
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The backdrop and its outline (unless the dialog brings its own
    /// backing), the dialog, and the field; then the on-screen keyboard,
    /// or the ask for the system's entry.
    fn draw(&self, list: &mut DrawList) {
        if self.dialog.backing().is_none() {
            let bounds = self.dialog.bounds();
            fill_rect(list, bounds, BACKDROP);
            outline(list, bounds, BORDER);
        }
        self.dialog.draw(list);
        self.field.draw(&*self.metrics, list);
        entry_keyboard::draw(self, list);
    }

    fn cancel_pointer(&mut self) {
        entry_keyboard::cancel(self);
        self.dialog.cancel_pointer();
    }

    /// Covered, it asks nothing and draws no keyboard.
    fn set_covered(&mut self, covered: bool) {
        entry_keyboard::cover(self, covered);
    }

    /// The alert as OK is refused. OK and Cancel are silent under the
    /// pointer, as `_TrackTextButtons` @0x26bb0 plays nothing.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

impl<P: Confirm> EntryHost for EditDialog<P> {
    fn keyboard(&self) -> &EntryKeyboard {
        &self.keyboard
    }

    fn keyboard_mut(&mut self) -> &mut EntryKeyboard {
        &mut self.keyboard
    }

    /// The one field, which always has the focus.
    fn entry_field(&self) -> Option<&TextField> {
        Some(&self.field)
    }

    fn clear_entry_field(&mut self) {
        self.field.clear();
    }

    fn dialog_bounds(&self) -> Bounds {
        self.dialog.bounds()
    }

    fn pad_target(&self) -> Option<usize> {
        self.dialog.pad_highlight()
    }
}

/// An edit dialog's row in the pad's every-screen table: OK, Cancel and
/// the field are its targets. The field became one in
/// rdm:phase/key-bindings/phase-17-text-entry-port, superseding phase
/// 9's "the field is none": South on it raises the on-screen keyboard,
/// and a click on it focuses it again.
#[cfg(test)]
pub(crate) struct EditProbe<P: Confirm>(pub(crate) EditDialog<P>);

#[cfg(test)]
impl<P: Confirm> super::pad_focus::every_screen::PadProbe for EditProbe<P> {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, crate::geometry::Bounds)> {
        let items = self.0.items;
        let bounds = |item| self.0.dialog.item_bounds(item).expect("an item");
        vec![
            ("OK", bounds(items.ok)),
            ("Cancel", bounds(items.cancel)),
            ("field", bounds(items.field)),
        ]
    }

    fn outcome(&mut self) -> String {
        format!("{:?} {}", self.0.take_outcome(), self.0.field().text())
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("OK")
    }

    /// The on-screen keyboard's drawing, which a safe-area change moves
    /// and touch shows, as it does a touch aid's.
    fn touch_aids_drawn(&mut self) -> Vec<crate::draw::DrawCommand> {
        keyboard_drawn(&self.0)
    }
}

/// What `host`'s on-screen keyboard draws now, if it shows.
#[cfg(test)]
pub(crate) fn keyboard_drawn(host: &impl EntryHost) -> Vec<crate::draw::DrawCommand> {
    let mut list = DrawList::new();
    if let Some(layout) = entry_keyboard::shown(host) {
        host.keyboard().draw(&layout, &mut list);
    }
    list.iter().cloned().collect()
}

/// Driving an edit dialog from pad inputs alone, for each dialog's own
/// rows of the acceptance criteria.
#[cfg(test)]
pub(crate) mod pad_entry {
    use super::*;
    use crate::input::{Device, PadButton};
    use crate::text_entry::EntryAsk;
    use crate::ui::entry_keyboard::KeyId;
    use crate::ui::pad_focus::every_screen::{PAD, pad, told};

    /// What `dialog` asks of the system's entry as it opens with the
    /// keyboard used last: its field, focused once.
    pub(crate) fn asked_at_open<P: Confirm>(dialog: &EditDialog<P>) -> Option<EntryAsk> {
        let mut keyed = dialog.clone();
        keyed.input(&told(Device::Keyboard));
        let mut list = DrawList::new();
        keyed.draw(&mut list);
        list.text_entry()
    }

    fn tap<P: Confirm>(dialog: &mut EditDialog<P>, button: PadButton) {
        dialog.input(&pad(button, true));
        dialog.input(&pad(button, false));
    }

    /// Types `keys` into `dialog` from pad inputs alone: told the pad (it
    /// asks for nothing and draws no keyboard), the right shoulder to the
    /// field, South on it (the keyboard rises), then each key by the d-pad
    /// and South.
    pub(crate) fn pad_types<P: Confirm>(dialog: &mut EditDialog<P>, keys: &[KeyId]) {
        dialog.input(&told(PAD));
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        assert_eq!(list.text_entry(), None, "under the pad it asks nothing");
        assert!(
            entry_keyboard::shown(dialog).is_none(),
            "nor shows the keyboard"
        );
        for _ in 0..8 {
            if dialog.dialog.pad_highlight() == Some(dialog.items.field) {
                break;
            }
            tap(dialog, PadButton::RightShoulder);
        }
        assert_eq!(dialog.dialog.pad_highlight(), Some(dialog.items.field));
        tap(dialog, PadButton::South);
        assert!(
            entry_keyboard::shown(dialog).is_some(),
            "South on the field raises it"
        );
        let layout = dialog.keyboard.layout(dialog.dialog.bounds());
        let from = dialog.keyboard.pad_focus().expect("the pad's focus");
        for input in layout.pad_presses(from, keys) {
            dialog.input(&input);
        }
    }

    /// Whether `screen` draws the on-screen keyboard now: its panel.
    fn keyboard_shown(screen: &dyn Screen) -> bool {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().any(|command| {
            matches!(command, crate::draw::DrawCommand::Line { color, .. }
                if *color == crate::ui::entry_keyboard::keyboard::PANEL)
        })
    }

    /// Asserts that covering `screen`, whose text-entry dialog is open,
    /// stops its entry: under the keyboard the frame asks for the system's
    /// entry only uncovered, and under touch the on-screen keyboard is
    /// drawn only uncovered. `what` names it in the messages.
    pub(crate) fn covering_stops_its_entry(screen: &mut dyn Screen, what: &str) {
        let asks = |screen: &dyn Screen| {
            let mut list = DrawList::new();
            screen.draw(&mut list);
            list.text_entry().is_some()
        };
        screen.input(&told(Device::Keyboard));
        assert!(asks(screen), "{what}: asks uncovered");
        screen.set_covered(true);
        assert!(!asks(screen), "{what}: asks nothing covered");
        screen.set_covered(false);
        assert!(asks(screen), "{what}: asks again uncovered");
        screen.input(&told(Device::Touch));
        assert!(keyboard_shown(screen), "{what}: the keyboard under touch");
        screen.set_covered(true);
        assert!(!keyboard_shown(screen), "{what}: no keyboard covered");
        screen.set_covered(false);
        assert!(
            keyboard_shown(screen),
            "{what}: the keyboard again uncovered"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::keymap::{Bindings, KeyCombo, UiCommand};
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
            backing: None,
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

    #[test]
    fn a_backed_edit_dialog_draws_its_backing_in_place_of_the_backdrop() {
        let backed = DialogTemplate {
            backing: Some(crate::ui::panel::PanelBacking::NEW_GAME),
            ..template()
        };
        let dialog = EditDialog::laid_out(
            &backed,
            ITEMS,
            &[],
            "Asked:",
            "abc",
            Scripted::default(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        let drawn: Vec<_> = list.iter().cloned().collect();
        crate::ui::panel::testing::assert_backed_first(&drawn, dialog.dialog().bounds());
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
        assert_eq!(click(&mut dialog, ITEMS.ok), []);
        assert_eq!(dialog.take_outcome(), Some("first".to_owned()));
        dialog.input(&Input::Text('d'));
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some("second".to_owned()));
        assert_eq!(*policy.asked.borrow(), ["abc", "d"]);
    }

    /// The dialog reads its keys through the key map it is given: Confirm
    /// rebound to F6 asks the policy on F6 and no longer on Return, while
    /// Space still types into the field.
    #[test]
    fn a_rebound_confirm_asks_on_its_new_key_alone() {
        let bindings = SharedBindings::new(
            Bindings::default().rebound(UiCommand::Confirm, KeyCombo::key(Key::F6)),
        );
        let policy = Scripted::answering(&[Some("ok")]);
        let mut dialog = dialog_with(policy.clone()).with_bindings(bindings);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), None, "Return no longer asks");
        dialog.input(&key(Key::F6));
        assert_eq!(dialog.take_outcome(), Some("ok".to_owned()));
        assert_eq!(*policy.asked.borrow(), ["abc"]);
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

    /// Return and Escape play nothing in the original:
    /// `_EVTextInputDialog` (`ModalDialog` @0x5544d) passes a NULL filter,
    /// and the Toolbox's standard filter only flashes the button. Space
    /// types into the field, so it activates nothing here.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        let mut dialog = dialog_with(Scripted::answering(&[Some("first")]));
        assert_eq!(dialog.take_sounds(), []);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some("first".to_owned()));
        assert_eq!(dialog.take_sounds(), [], "Return");
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_sounds(), [], "Escape");
    }

    #[test]
    fn typing_space_and_backspace_reach_the_field() {
        let mut dialog = dialog();
        dialog.input(&key(Key::KeyX));
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
    fn the_surroundings_leave_the_flush_for_the_next_typed_character() {
        use crate::input::{ControllerFamily, Device, Surroundings};
        let pad = Device::Gamepad(ControllerFamily::Generic);
        let mut dialog = dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::Surroundings(Surroundings::of(pad)));
        dialog.input(&Input::Text('b'));
        assert_eq!(dialog.field().text(), "abc", "still flushed");
        assert_eq!(dialog.dialog.device(), pad, "the dialog is told");
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
        expected.ask_text_entry(crate::text_entry::EntryAsk {
            field: dialog.field().rect(),
            focused: 1,
        });
        assert_eq!(list, expected);
    }

    fn asked(dialog: &EditDialog<Scripted>) -> Option<crate::text_entry::EntryAsk> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.text_entry()
    }

    fn told(device: crate::input::Device) -> Input {
        Input::Surroundings(crate::input::Surroundings::of(device))
    }

    #[test]
    fn opening_with_the_keyboard_asks_for_entry_at_the_field() {
        use crate::input::Device;
        for device in [Device::Keyboard, Device::Mouse] {
            let mut dialog = dialog();
            dialog.input(&told(device));
            assert_eq!(
                asked(&dialog),
                Some(crate::text_entry::EntryAsk {
                    field: rect(110.0, 8.0, 60.0, 16.0),
                    focused: 1,
                }),
                "{device:?}"
            );
        }
        assert_eq!(
            super::pad_entry::asked_at_open(&dialog()),
            asked(&dialog()),
            "untold is the keyboard"
        );
    }

    #[test]
    fn south_on_the_field_raises_the_keyboard_under_the_pad() {
        use crate::input::PadButton;
        use crate::ui::pad_focus::every_screen::{PAD, pad};
        let mut dialog = dialog();
        dialog.input(&told(PAD));
        assert_eq!(dialog.dialog().pad_highlight(), Some(ITEMS.ok), "OK first");
        assert!(super::keyboard_drawn(&dialog).is_empty(), "not at open");
        assert_eq!(asked(&dialog), None);
        for pressed in [true, false] {
            dialog.input(&pad(PadButton::RightShoulder, pressed));
        }
        assert_eq!(dialog.dialog().pad_highlight(), Some(ITEMS.field));
        for pressed in [true, false] {
            dialog.input(&pad(PadButton::South, pressed));
        }
        assert!(!super::keyboard_drawn(&dialog).is_empty(), "raised");
        assert_eq!(
            asked(&dialog),
            None,
            "the game's keyboard, not the system's"
        );
        assert_eq!(dialog.field().text(), "abc", "still selected whole");
        assert_eq!(dialog.field().selection(), 3);
    }

    #[test]
    fn a_tap_on_the_field_asks_afresh() {
        let mut dialog = dialog();
        let first = asked(&dialog).expect("asked");
        click(&mut dialog, ITEMS.field);
        let again = asked(&dialog).expect("asked");
        assert_eq!(again.field, first.field);
        assert_eq!(again.focused, first.focused + 1, "a new focus");
        let mut touched = self::dialog();
        crate::touch::testkit::tap(
            &mut touched,
            dialog
                .dialog()
                .item_bounds(ITEMS.field)
                .expect("the field")
                .center(),
        );
        assert_eq!(asked(&touched), Some(again));
    }

    #[test]
    fn a_click_on_another_item_focuses_no_field() {
        let mut template = template();
        template
            .items
            .push(item(rect(180.0, 50.0, 16.0, 16.0), true, ItemSpec::User));
        let mut dialog = EditDialog::laid_out(
            &template,
            ITEMS,
            &[],
            "Asked:",
            "abc",
            Scripted::default(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        let first = asked(&dialog).expect("asked");
        click(&mut dialog, 5);
        assert_eq!(asked(&dialog), Some(first), "no new focus");
        click(&mut dialog, ITEMS.field);
        assert_eq!(
            asked(&dialog).map(|ask| ask.focused),
            Some(first.focused + 1)
        );
    }

    #[test]
    fn covered_it_asks_nothing_and_draws_no_keyboard() {
        use crate::input::Device;
        let mut dialog = dialog();
        dialog.set_covered(true);
        assert_eq!(asked(&dialog), None);
        dialog.set_covered(false);
        assert!(asked(&dialog).is_some(), "uncovered, it asks again");
        let mut touched = self::dialog();
        touched.input(&told(Device::Touch));
        assert!(!super::keyboard_drawn(&touched).is_empty());
        touched.set_covered(true);
        assert!(super::keyboard_drawn(&touched).is_empty());
        let mut list = DrawList::new();
        touched.draw(&mut list);
        let mut bare = DrawList::new();
        self::dialog().draw(&mut bare);
        assert_eq!(
            list.iter().collect::<Vec<_>>(),
            bare.iter().collect::<Vec<_>>(),
            "nothing but the dialog drawn"
        );
    }

    #[test]
    fn the_on_screen_keyboard_types_through_the_field_as_keys_do() {
        use crate::ui::entry_keyboard::KeyId;
        let policy = Scripted::answering(&[Some("done")]);
        let mut dialog = dialog_with(policy.clone());
        super::pad_entry::pad_types(
            &mut dialog,
            &[
                KeyId::Char('q'),
                KeyId::Space,
                KeyId::Char('7'),
                KeyId::Delete,
                KeyId::Char('-'),
                KeyId::Done,
            ],
        );
        assert_eq!(dialog.take_outcome(), Some("done".to_owned()));
        assert_eq!(*policy.asked.borrow(), ["Q -"]);
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
