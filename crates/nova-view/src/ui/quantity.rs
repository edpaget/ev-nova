//! The quantity dialog (`DLOG`/`DITL` 1003): how many to buy or sell, up
//! to a maximum, which OK confirms and Cancel declines.
//!
//! The original's `_DoQuantityDialog` (@0x56d5b) asks it at the exchange
//! when Option is held on Buy or Sell (`_DoTradeDialog` @0x5e211,
//! @0x5e45c), and at the outfitter and for a race bet too. The stock
//! dialog is 172 x 72, centred: OK (item 1), the prompt (item 2), the
//! count (item 3) and Cancel (item 4). The caller titles OK and replaces
//! the prompt: at the exchange "Buy" or "Sell" (`STR#` 150 #2/#3,
//! @0x56e66-0x56ebd) and "Enter quantity:" ([`PROMPT`], `STR#` 2002 #371,
//! @0x56ec2-0x56f2c), which never names the good. The field opens holding the maximum
//! (@0x56f61-0x56f86), selected whole (@0x56e0b-0x56e28), so typing
//! replaces it. OK is the default item, so Return confirms, and there is
//! no cancel item: Escape does nothing, and only a click on Cancel
//! declines, as the count 0 (@0x56fae, @0x57138). Nothing filters what is
//! typed; the key that opened it types nothing into it
//! ([`QuantityDialog::flush_typed_key`]).
//!
//! OK checks the field ([`check`]) and, refusing it, beeps (the alert,
//! [`UiSound::Alert`], for the original's `SysBeep`), sets the field and
//! stays open. Without the interface file, [`QuantityDialog::fallback`]
//! lays out the same items itself.
//!
//! The dialog has no stock frame: like "Text Input", it is drawn over a
//! dark backdrop with a 1-unit outline.

use std::rc::Rc;
use std::time::Duration;

use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role, outline,
};
use super::prefs::{BACKDROP, BORDER};
use super::text_field::TextField;

/// The dialog's `DLOG` (and `DITL`) ID.
pub const QUANTITY_DIALOG: i16 = 1003;
/// OK's item: it confirms the count.
pub const OK_ITEM: usize = 1;
/// The prompt's item.
pub const PROMPT_ITEM: usize = 2;
/// The count's item.
pub const FIELD_ITEM: usize = 3;
/// Cancel's item.
pub const CANCEL_ITEM: usize = 4;

/// The prompt asking for a count of goods or outfits: `STR#` 2002 #371.
pub const PROMPT: &str = "Enter quantity:";

/// What OK makes of the field ([`check`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    /// This count is confirmed.
    Accept(u32),
    /// Refused: the dialog beeps, and the field becomes this text,
    /// selected whole.
    Reject(String),
}

/// What OK makes of `text` with the most it confirms `max`, in the
/// original's order (@0x56fbc-0x5712e):
///
/// 1. Empty: refused, and it stays empty.
/// 2. Any character not a digit, a '-' too: refused, keeping its digits
///    in order (which may be none).
/// 3. Digits only: read as `StringToNum` reads them, n x 10 + digit in 32
///    bits with no overflow check, so a long text wraps. From 0 to `max`
///    it is confirmed; otherwise refused, reset to "0" when it wrapped
///    below 0 and to `max` otherwise.
///
/// With `max` below 0 nothing is confirmed.
#[must_use]
pub fn check(text: &str, max: i64) -> Check {
    if text.is_empty() {
        return Check::Reject(String::new());
    }
    if !text.chars().all(|c| c.is_ascii_digit()) {
        return Check::Reject(text.chars().filter(char::is_ascii_digit).collect());
    }
    let n = text.bytes().fold(0_i32, |n, digit| {
        n.wrapping_mul(10).wrapping_add(i32::from(digit - b'0'))
    });
    match u32::try_from(n) {
        Ok(count) if i64::from(count) <= max => Check::Accept(count),
        Ok(_) => Check::Reject(max.to_string()),
        Err(_) => Check::Reject("0".to_owned()),
    }
}

/// The quantity dialog: the prompt, the count typed, and OK or Cancel.
#[derive(Clone)]
pub struct QuantityDialog {
    dialog: Dialog,
    field: TextField,
    /// The most OK confirms.
    max: i64,
    metrics: Rc<dyn TextMetrics>,
    /// The count chosen: 0 for Cancel.
    outcome: Option<u32>,
    /// Whether the next input is dropped if it is typed text.
    flushing: bool,
    sounds: Vec<Sound>,
}

impl std::fmt::Debug for QuantityDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuantityDialog")
            .field("text", &self.field.text())
            .field("max", &self.max)
            .field("outcome", &self.outcome)
            .finish_non_exhaustive()
    }
}

impl QuantityDialog {
    /// The dialog `template` (stock `DLOG` 1003) asking `prompt`, OK
    /// titled `title` (say "Buy" and [`PROMPT`]), its field holding `max`,
    /// selected whole; its buttons labelled in `style` and its text
    /// measured by `metrics`. `max` may be 0 or less.
    ///
    /// # Errors
    ///
    /// When the template's item 3 is not edit text, or it has no item 4.
    pub fn new(
        template: &DialogTemplate,
        max: i64,
        title: &str,
        prompt: &str,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        let editable = matches!(
            template.items.get(FIELD_ITEM - 1),
            Some(ItemTemplate {
                kind: ItemSpec::EditText(_),
                ..
            })
        );
        if !editable || template.items.len() < CANCEL_ITEM {
            return Err(format!(
                "DITL {QUANTITY_DIALOG} has no edit text item {FIELD_ITEM} and Cancel item \
                 {CANCEL_ITEM}"
            ));
        }
        // `SetControlTitle` on item 1: a standard button keeps its title
        // from the template, so the copy laid out is retitled.
        let mut retitled = template.clone();
        if let ItemSpec::Button(label) = &mut retitled.items[OK_ITEM - 1].kind {
            title.clone_into(label);
        }
        let roles = [(OK_ITEM, Role::Button(title.to_owned()))];
        let mut dialog = Dialog::new(&retitled, &roles, Rc::clone(&metrics))
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(OK_ITEM))
            .with_cancel(None);
        dialog.set_text(PROMPT_ITEM, prompt);
        let field_rect = dialog.item_bounds(FIELD_ITEM).expect("checked above");
        Ok(Self {
            dialog,
            field: TextField::with_text(field_rect, usize::MAX, &max.to_string()),
            max,
            metrics,
            outcome: None,
            flushing: false,
            sounds: Vec::new(),
        })
    }

    /// The dialog laid out without the interface file, as the stock one
    /// is: OK titled `title`, the prompt `prompt`, the field and Cancel,
    /// centred.
    #[must_use]
    pub fn fallback(
        max: i64,
        title: &str,
        prompt: &str,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self::new(&fallback_template(), max, title, prompt, style, metrics)
            .expect("the fallback has a field and Cancel")
    }

    /// Drops the next input if it is typed text, as the original's
    /// keyDown that opened the dialog is consumed: the key that opened it
    /// then types nothing into it. Any other input ends the flush.
    pub fn flush_typed_key(&mut self) {
        self.flushing = true;
    }

    /// The field.
    #[must_use]
    pub fn field(&self) -> &TextField {
        &self.field
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The count chosen, once: 0 when Cancel was clicked.
    pub fn take_outcome(&mut self) -> Option<u32> {
        self.outcome.take()
    }

    /// OK: confirms the count, or beeps and resets the field.
    fn confirm(&mut self) {
        match check(self.field.text(), self.max) {
            Check::Accept(count) => self.outcome = Some(count),
            Check::Reject(text) => {
                self.sounds.push(Sound::Ui(UiSound::Alert));
                self.field.set_text(&text);
            }
        }
    }
}

/// The built-in quantity dialog: 172 x 72, centred, with OK (1), the
/// prompt (2), the field (3) and Cancel (4).
fn fallback_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let item = |bounds, enabled, kind| ItemTemplate {
        bounds,
        enabled,
        kind,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 172.0, 72.0),
        placement: Placement::Center,
        items: vec![
            item(
                at(92.0, 42.0, 70.0, 20.0),
                true,
                ItemSpec::Button("OK".to_owned()),
            ),
            item(
                at(6.0, 8.0, 102.0, 16.0),
                false,
                ItemSpec::StaticText(String::new()),
            ),
            item(
                at(112.0, 8.0, 51.0, 16.0),
                true,
                ItemSpec::EditText(String::new()),
            ),
            item(
                at(10.0, 42.0, 70.0, 20.0),
                true,
                ItemSpec::Button("Cancel".to_owned()),
            ),
        ],
    }
}

impl Screen for QuantityDialog {
    /// Typed characters, Backspace and Space go to the field; after
    /// [`flush_typed_key`](Self::flush_typed_key), the next typed
    /// character is dropped. Everything else goes to the dialog: OK (or
    /// Return) checks the count, and Cancel declines. It never quits.
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
            Some(DialogEvent::Item(OK_ITEM)) => self.confirm(),
            Some(DialogEvent::Item(CANCEL_ITEM)) => self.outcome = Some(0),
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

    /// OK's and Cancel's sounds as they are clicked, and the alert as a
    /// count is refused, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;

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

    /// Stock `DITL` 1003's items, the dialog fixed at (0, 0): OK (1), the
    /// prompt "^0" (2), the field "12345" (3) and Cancel (4).
    fn stock() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 172.0, 72.0),
            placement: Placement::Fixed,
            items: vec![
                item(
                    rect(92.0, 42.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("OK".into()),
                ),
                item(
                    rect(6.0, 8.0, 102.0, 16.0),
                    false,
                    ItemSpec::StaticText("^0".into()),
                ),
                item(
                    rect(112.0, 8.0, 51.0, 16.0),
                    true,
                    ItemSpec::EditText("12345".into()),
                ),
                item(
                    rect(10.0, 42.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("Cancel".into()),
                ),
            ],
        }
    }

    fn dialog_of(max: i64, title: &str) -> QuantityDialog {
        QuantityDialog::new(
            &stock(),
            max,
            title,
            PROMPT,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    fn dialog() -> QuantityDialog {
        dialog_of(12, "Buy")
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn typed(dialog: &mut QuantityDialog, text: &str) {
        for c in text.chars() {
            // As the platform sends them: the key, then its character.
            dialog.input(&key(Key::Char(c.to_ascii_lowercase())));
            dialog.input(&Input::Text(c));
        }
    }

    fn click(dialog: &mut QuantityDialog, item: usize) -> Vec<Sound> {
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

    fn texts(dialog: &QuantityDialog) -> Vec<String> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// Replaces the field with `text` and presses Return: the outcome and
    /// the sounds made.
    fn entered(dialog: &mut QuantityDialog, text: &str) -> (Option<u32>, Vec<Sound>) {
        dialog.field.set_text("");
        typed(dialog, text);
        dialog.take_sounds();
        dialog.input(&key(Key::Enter));
        (dialog.take_outcome(), dialog.take_sounds())
    }

    const ALERT: Sound = Sound::Ui(UiSound::Alert);

    #[test]
    fn the_named_values() {
        assert_eq!(QUANTITY_DIALOG, 1003);
        assert_eq!(
            (OK_ITEM, PROMPT_ITEM, FIELD_ITEM, CANCEL_ITEM),
            (1, 2, 3, 4)
        );
        assert_eq!(PROMPT, "Enter quantity:");
    }

    #[test]
    fn it_opens_with_the_maximum_selected_ok_titled_buy_and_the_prompt() {
        let dialog = dialog();
        assert_eq!(dialog.field().text(), "12");
        assert_eq!(dialog.field().selection(), 2, "selected whole");
        assert_eq!(dialog.field().rect(), rect(112.0, 8.0, 51.0, 16.0));
        let texts = texts(&dialog);
        assert_eq!(texts, ["Buy", "Enter quantity:", "Cancel", "12"]);
    }

    #[test]
    fn a_sale_titles_ok_sell() {
        let dialog = dialog_of(3, "Sell");
        assert_eq!(texts(&dialog), ["Sell", "Enter quantity:", "Cancel", "3"]);
    }

    #[test]
    fn the_caller_gives_the_title_and_prompt() {
        let dialog = QuantityDialog::new(
            &stock(),
            5,
            "Bet",
            "Amount to bet:",
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        assert_eq!(texts(&dialog), ["Bet", "Amount to bet:", "Cancel", "5"]);
        let fallback = QuantityDialog::fallback(
            5,
            "Bet",
            "Amount to bet:",
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        );
        assert_eq!(texts(&fallback), ["Bet", "Amount to bet:", "Cancel", "5"]);
    }

    #[test]
    fn a_user_item_ok_is_titled_too() {
        let mut template = stock();
        template.items[OK_ITEM - 1].kind = ItemSpec::User;
        let dialog = QuantityDialog::new(
            &template,
            3,
            "Sell",
            PROMPT,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        assert!(texts(&dialog).contains(&"Sell".to_owned()));
    }

    #[test]
    fn typing_over_the_maximum_and_ok_or_return_confirms_the_count() {
        let mut dialog = dialog();
        typed(&mut dialog, "5");
        assert_eq!(dialog.field().text(), "5");
        assert_eq!(
            click(&mut dialog, OK_ITEM),
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
        assert_eq!(dialog.take_outcome(), Some(5));
        assert_eq!(dialog.take_outcome(), None, "taken once");
        let mut dialog = self::dialog();
        typed(&mut dialog, "7");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(7));
    }

    #[test]
    fn return_confirms_the_maximum_untouched() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(12));
        assert_eq!(dialog.take_sounds(), []);
    }

    #[test]
    fn cancel_is_a_count_of_none_and_escape_does_nothing() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(dialog.field().text(), "12");
        click(&mut dialog, CANCEL_ITEM);
        assert_eq!(dialog.take_outcome(), Some(0));
    }

    #[test]
    fn an_empty_field_beeps_and_stays_empty() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Backspace));
        assert_eq!(entered(&mut dialog, ""), (None, vec![ALERT]));
        assert_eq!(dialog.field().text(), "");
    }

    #[test]
    fn a_non_digit_beeps_and_keeps_the_digits_selected() {
        let mut dialog = dialog();
        assert_eq!(entered(&mut dialog, "1a2"), (None, vec![ALERT]));
        assert_eq!(dialog.field().text(), "12");
        assert_eq!(dialog.field().selection(), 2);
        assert_eq!(entered(&mut dialog, "-3"), (None, vec![ALERT]));
        assert_eq!(dialog.field().text(), "3", "a '-' is no digit");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(3));
    }

    #[test]
    fn past_the_maximum_beeps_and_resets_to_it() {
        let mut dialog = dialog();
        assert_eq!(entered(&mut dialog, "13"), (None, vec![ALERT]));
        assert_eq!(dialog.field().text(), "12");
        assert_eq!(dialog.field().selection(), 2);
    }

    #[test]
    fn a_count_wrapped_below_nothing_beeps_and_resets_to_0() {
        let mut dialog = dialog();
        assert_eq!(entered(&mut dialog, "2147483648"), (None, vec![ALERT]));
        assert_eq!(dialog.field().text(), "0");
        assert_eq!(dialog.field().selection(), 1);
    }

    #[test]
    fn a_count_wrapped_into_range_is_confirmed() {
        let mut dialog = dialog();
        assert_eq!(entered(&mut dialog, "4294967297"), (Some(1), vec![]));
    }

    #[test]
    fn below_a_maximum_under_nothing_only_cancel_leaves() {
        let mut dialog = dialog_of(-2, "Buy");
        assert_eq!(dialog.field().text(), "-2");
        assert_eq!(entered(&mut dialog, "0"), (None, vec![ALERT]));
        assert_eq!(dialog.field().text(), "-2");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.field().text(), "2", "stripped");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.field().text(), "-2", "then reset");
        assert_eq!(dialog.take_outcome(), None);
        click(&mut dialog, CANCEL_ITEM);
        assert_eq!(dialog.take_outcome(), Some(0));
    }

    #[test]
    fn at_a_maximum_of_nothing_0_is_confirmed() {
        let mut dialog = dialog_of(0, "Sell");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), Some(0));
    }

    #[test]
    fn check_reads_the_field_as_the_original_does() {
        let reject = |text: &str| Check::Reject(text.to_owned());
        for (text, max, wanted) in [
            ("", 12, reject("")),
            ("x", 12, reject("")),
            ("1a2", 12, reject("12")),
            ("-3", 12, reject("3")),
            ("+3", 12, reject("3")),
            (" 3", 12, reject("3")),
            ("٣", 12, reject("")),
            ("0", 12, Check::Accept(0)),
            ("00012", 12, Check::Accept(12)),
            ("12", 12, Check::Accept(12)),
            ("13", 12, reject("12")),
            ("11", 12, Check::Accept(11)),
            ("2147483647", 32_000, reject("32000")),
            ("2147483648", 32_000, reject("0")),
            ("4294967295", 32_000, reject("0")),
            ("4294967296", 32_000, Check::Accept(0)),
            ("4294967297", 32_000, Check::Accept(1)),
            ("0", 0, Check::Accept(0)),
            ("1", 0, reject("0")),
            ("0", -2, reject("-2")),
            ("2147483648", -2, reject("0")),
        ] {
            assert_eq!(check(text, max), wanted, "{text:?} up to {max}");
        }
    }

    #[test]
    fn a_flush_drops_exactly_the_next_typed_character() {
        let mut dialog = dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::Text('ß'));
        assert_eq!(dialog.field().text(), "12", "flushed");
        dialog.input(&Input::Text('4'));
        assert_eq!(dialog.field().text(), "4");
        let mut dialog = self::dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        dialog.input(&Input::Text('4'));
        assert_eq!(dialog.field().text(), "4", "the flush ended");
    }

    #[test]
    fn space_and_backspace_go_to_the_field() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Space));
        dialog.input(&Input::Text(' '));
        assert_eq!(dialog.field().text(), " ");
        dialog.input(&key(Key::Backspace));
        assert_eq!(dialog.field().text(), "");
    }

    #[test]
    fn it_draws_a_backdrop_the_dialog_and_the_field() {
        let dialog = dialog();
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
    fn a_template_without_its_field_or_cancel_is_an_error() {
        let error = Some("DITL 1003 has no edit text item 3 and Cancel item 4".to_owned());
        let build = |template: &DialogTemplate| {
            QuantityDialog::new(
                template,
                12,
                "Buy",
                PROMPT,
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics),
            )
            .err()
        };
        let mut short = stock();
        short.items.truncate(3);
        assert_eq!(build(&short), error);
        let mut plain = stock();
        plain.items[FIELD_ITEM - 1].kind = ItemSpec::StaticText(String::new());
        assert_eq!(build(&plain), error);
        let mut shorter = stock();
        shorter.items.truncate(2);
        assert_eq!(build(&shorter), error);
    }

    #[test]
    fn the_fallback_lays_out_the_same_items_centred() {
        let mut dialog =
            QuantityDialog::fallback(12, "Buy", PROMPT, ButtonStyle::STOCK, Rc::new(MonoMetrics));
        let bounds = dialog.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (172.0, 72.0));
        let origin = bounds.min;
        assert_eq!(
            origin,
            Point::new((1024.0 - 172.0) / 2.0, (768.0 - 72.0) / 2.0)
        );
        assert_eq!(
            dialog.field().rect(),
            rect(112.0, 8.0, 51.0, 16.0).offset(origin)
        );
        for (item, at) in [
            (OK_ITEM, rect(92.0, 42.0, 70.0, 20.0)),
            (PROMPT_ITEM, rect(6.0, 8.0, 102.0, 16.0)),
            (CANCEL_ITEM, rect(10.0, 42.0, 70.0, 20.0)),
        ] {
            assert_eq!(dialog.dialog().item_bounds(item), Some(at.offset(origin)));
            assert!(dialog.dialog().item_shown(item), "{item}");
        }
        assert_eq!(texts(&dialog), ["Buy", "Enter quantity:", "Cancel", "12"]);
        click(&mut dialog, CANCEL_ITEM);
        assert_eq!(dialog.take_outcome(), Some(0));
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click() {
        let mut dialog = dialog();
        let at = dialog.dialog().item_bounds(OK_ITEM).expect("OK").center();
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
    fn debug_shows_the_text_the_maximum_and_the_outcome() {
        let mut dialog = dialog();
        dialog.tick(Duration::from_secs(1));
        assert_eq!(
            format!("{dialog:?}"),
            "QuantityDialog { text: \"12\", max: 12, outcome: None, .. }"
        );
    }
}
