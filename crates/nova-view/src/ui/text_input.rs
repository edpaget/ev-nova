//! The "Text Input" dialog (`DLOG`/`DITL` 3001): a prompt, and a line of
//! text to type, which OK confirms and Cancel declines.
//!
//! The original's `_EVTextInputDialog` (@0x552a3-0x5558b) asks the new
//! ship's name in the shipyard, and also christens a new pilot's ship and
//! renames a ship captured. The stock dialog is 360 x 138, centred: OK
//! (item 1), the prompt (item 3), a picture (item 4, `PICT` 130), the
//! text (item 5) and Cancel (item 6); item 2, `PICT` 129, is parked
//! below it. The prompt is set to the text asked with (@0x5537e-0x5539d)
//! and the field to the default (@0x553bf-0x553f3), selected whole
//! (@0x553f8-0x55415), so typing replaces it. OK is the default item, so
//! Return confirms (@0x5530a), and there is no cancel item: Escape does
//! nothing, and only a click on Cancel declines. The original flushes
//! pending events as it opens (@0x55422-0x55429), so the key that opened
//! it types nothing into it ([`EditDialog::flush_typed_key`]).
//!
//! The field takes text of any length, but OK refuses one longer than
//! the most asked for: it beeps (the alert,
//! [`UiSound::Alert`](crate::sound::UiSound::Alert)), selects
//! the first characters up to one less than that, and stays open
//! (@0x554a9-0x554e7). An empty text is confirmed, and the text is never
//! trimmed. Without the interface file, [`TextInputDialog::fallback`]
//! lays out the same items itself. The pictures are not drawn yet.
//!
//! The dialog has no stock frame: like the New Pilot dialog, it is drawn
//! over a dark backdrop with a 1-unit outline. It is the shared
//! edit-field shell ([`EditDialog`]) with the [`Length`] policy.

use std::rc::Rc;

use crate::geometry::{Bounds, Point};
use crate::text::TextMetrics;

use super::button::ButtonStyle;
use super::dialog::{DialogTemplate, ItemSpec, ItemTemplate, Placement};
use super::edit_dialog::{Confirm, EditDialog, EditItems};
use super::text_field::TextField;

/// The dialog's `DLOG` (and `DITL`) ID.
pub const TEXT_INPUT_DIALOG: i16 = 3001;
/// OK's item: it confirms the text.
pub const OK_ITEM: usize = 1;
/// The prompt's item.
pub const PROMPT_ITEM: usize = 3;
/// The text's item.
pub const FIELD_ITEM: usize = 5;
/// Cancel's item.
pub const CANCEL_ITEM: usize = 6;

/// What the player chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextInputOutcome {
    /// This text, as typed: not trimmed, and possibly empty.
    Confirm(String),
    /// Cancel was clicked.
    Cancel,
}

/// "Text Input"'s rule: OK confirms a text of at most `max_chars`
/// characters, and refuses a longer one selecting the first characters up
/// to one less.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Length {
    /// The longest text OK confirms, in characters.
    max_chars: usize,
}

impl Confirm for Length {
    type Outcome = TextInputOutcome;

    /// Confirms the text, or refuses one too long.
    fn confirm(&self, field: &mut TextField) -> Option<TextInputOutcome> {
        if field.text().chars().count() > self.max_chars {
            field.select_first(self.max_chars.saturating_sub(1));
            return None;
        }
        Some(TextInputOutcome::Confirm(field.text().to_owned()))
    }

    /// Cancel declines.
    fn cancelled(&self) -> TextInputOutcome {
        TextInputOutcome::Cancel
    }
}

/// "Text Input": a prompt, the text typed, and OK or Cancel.
pub type TextInputDialog = EditDialog<Length>;

impl std::fmt::Debug for EditDialog<Length> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInputDialog")
            .field("text", &self.field().text())
            .field("outcome", &self.outcome())
            .finish_non_exhaustive()
    }
}

/// The items the shell drives.
const ITEMS: EditItems = EditItems {
    id: TEXT_INPUT_DIALOG,
    ok: OK_ITEM,
    prompt: PROMPT_ITEM,
    field: FIELD_ITEM,
    cancel: CANCEL_ITEM,
};

impl EditDialog<Length> {
    /// The dialog `template` (stock `DLOG` 3001) asking `prompt`, its
    /// field holding `default`, selected whole, and confirming no more
    /// than `max_chars` characters; its buttons labelled in `style` and
    /// its text measured by `metrics`.
    ///
    /// # Errors
    ///
    /// When the template's item 5 is not edit text, or it has no item 6.
    pub fn new(
        template: &DialogTemplate,
        prompt: &str,
        default: &str,
        max_chars: usize,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        Self::laid_out(
            template,
            ITEMS,
            &[],
            prompt,
            default,
            Length { max_chars },
            style,
            metrics,
        )
    }

    /// The dialog laid out without the interface file, as the stock one
    /// is: the prompt, the field, and OK and Cancel, centred.
    #[must_use]
    pub fn fallback(
        prompt: &str,
        default: &str,
        max_chars: usize,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self::new(
            &fallback_template(),
            prompt,
            default,
            max_chars,
            style,
            metrics,
        )
        .expect("the fallback has a field and Cancel")
    }
}

/// The built-in "Text Input": 360 x 138, centred, with OK (1), the prompt
/// (3), the field (5) and Cancel (6); the pictures (2 and 4) are parked
/// outside it.
fn fallback_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let item = |bounds, enabled, kind| ItemTemplate {
        bounds,
        enabled,
        kind,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 360.0, 138.0),
        placement: Placement::Center,
        items: vec![
            item(
                at(252.0, 106.0, 70.0, 20.0),
                true,
                ItemSpec::Button("OK".to_owned()),
            ),
            item(at(0.0, 400.0, 10.0, 10.0), false, ItemSpec::Picture(129)),
            item(
                at(52.0, 5.0, 295.0, 50.0),
                false,
                ItemSpec::StaticText(String::new()),
            ),
            item(at(0.0, 400.0, 10.0, 10.0), false, ItemSpec::Picture(130)),
            item(
                at(91.0, 64.0, 200.0, 16.0),
                true,
                ItemSpec::EditText(String::new()),
            ),
            item(
                at(170.0, 106.0, 70.0, 20.0),
                true,
                ItemSpec::Button("Cancel".to_owned()),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{DrawCommand, DrawList, fill_rect};
    use crate::input::{Input, Key, MouseButton};
    use crate::screen::Screen;
    use crate::sound::{Sound, UiSound};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::outline;
    use crate::ui::prefs::{BACKDROP, BORDER};

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

    /// Stock "Text Input"'s items, the dialog fixed at (0, 0): OK (1), a
    /// picture parked below (2), the prompt (3), a picture (4), the field
    /// (5) and Cancel (6).
    fn stock() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 360.0, 138.0),
            placement: Placement::Fixed,
            items: vec![
                item(
                    rect(252.0, 106.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("OK".into()),
                ),
                item(rect(7.0, 147.0, 32.0, 32.0), false, ItemSpec::Picture(129)),
                item(
                    rect(52.0, 5.0, 295.0, 50.0),
                    false,
                    ItemSpec::StaticText("^0".into()),
                ),
                item(rect(7.0, 5.0, 32.0, 32.0), false, ItemSpec::Picture(130)),
                item(
                    rect(91.0, 64.0, 200.0, 16.0),
                    true,
                    ItemSpec::EditText(String::new()),
                ),
                item(
                    rect(170.0, 106.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("Cancel".into()),
                ),
            ],
        }
    }

    const PROMPT: &str = "Please name your new The Ship 129: ";
    const DEFAULT: &str = "Ship 129 491";

    fn dialog() -> TextInputDialog {
        TextInputDialog::new(
            &stock(),
            PROMPT,
            DEFAULT,
            64,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn typed(dialog: &mut TextInputDialog, text: &str) {
        for c in text.chars() {
            // As the platform sends them: the key, then its character.
            dialog.input(&key(Key::Char(c.to_ascii_lowercase())));
            dialog.input(&Input::Text(c));
        }
    }

    fn click(dialog: &mut TextInputDialog, item: usize) -> Vec<Sound> {
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

    fn texts(dialog: &TextInputDialog) -> Vec<String> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn it_opens_with_the_prompt_and_the_default_selected() {
        let dialog = dialog();
        assert_eq!(dialog.field().text(), DEFAULT);
        assert!(dialog.field().selected());
        assert_eq!(dialog.field().selection(), DEFAULT.chars().count());
        assert_eq!(dialog.field().rect(), rect(91.0, 64.0, 200.0, 16.0));
        let texts = texts(&dialog);
        assert!(
            texts
                .iter()
                .any(|text| text.contains("Please name your new")),
            "{texts:?}"
        );
        for shown in ["OK", "Cancel", DEFAULT] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
        assert_eq!(TEXT_INPUT_DIALOG, 3001);
        assert_eq!(
            (OK_ITEM, PROMPT_ITEM, FIELD_ITEM, CANCEL_ITEM),
            (1, 3, 5, 6)
        );
    }

    #[test]
    fn typing_replaces_the_default_and_return_confirms_it() {
        let mut dialog = dialog();
        typed(&mut dialog, "Kestrek");
        dialog.input(&key(Key::Backspace));
        typed(&mut dialog, "l");
        assert_eq!(dialog.take_outcome(), None);
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(TextInputOutcome::Confirm("Kestrel".to_owned()))
        );
        assert_eq!(dialog.take_outcome(), None, "taken once");
    }

    #[test]
    fn return_confirms_the_default_untouched() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(TextInputOutcome::Confirm(DEFAULT.to_owned()))
        );
    }

    #[test]
    fn ok_confirms_and_cancel_declines_as_clicked() {
        let mut dialog = dialog();
        typed(&mut dialog, " A b ");
        assert_eq!(
            click(&mut dialog, OK_ITEM),
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
        assert_eq!(
            dialog.take_outcome(),
            Some(TextInputOutcome::Confirm(" A b ".to_owned())),
            "not trimmed, and Space types a space"
        );
        let mut dialog = self::dialog();
        click(&mut dialog, CANCEL_ITEM);
        assert_eq!(dialog.take_outcome(), Some(TextInputOutcome::Cancel));
    }

    #[test]
    fn escape_does_nothing() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(dialog.field().text(), DEFAULT);
    }

    #[test]
    fn an_empty_text_is_confirmed() {
        let mut dialog = dialog();
        dialog.input(&key(Key::Backspace));
        assert_eq!(dialog.field().text(), "");
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(TextInputOutcome::Confirm(String::new()))
        );
    }

    #[test]
    fn a_flush_drops_exactly_the_next_typed_character() {
        let mut dialog = dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::Text('b'));
        assert_eq!(dialog.field().text(), DEFAULT, "flushed");
        dialog.input(&Input::Text('K'));
        assert_eq!(dialog.field().text(), "K");
        let mut dialog = self::dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        dialog.input(&Input::Text('K'));
        assert_eq!(dialog.field().text(), "K", "the flush ended");
    }

    #[test]
    fn the_field_takes_more_than_the_most_but_ok_refuses_it_and_selects_the_first() {
        let mut dialog = dialog();
        let long = "x".repeat(70);
        typed(&mut dialog, &long);
        assert_eq!(dialog.field().text(), long, "typing is not capped");
        dialog.take_sounds();
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_outcome(), None, "the dialog stays open");
        assert_eq!(dialog.take_sounds(), [Sound::Ui(UiSound::Alert)]);
        assert_eq!(dialog.field().selection(), 63);
        typed(&mut dialog, "K");
        assert_eq!(dialog.field().text(), format!("K{}", "x".repeat(7)));
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(TextInputOutcome::Confirm(format!("K{}", "x".repeat(7))))
        );
        assert_eq!(dialog.take_sounds(), Vec::<Sound>::new());
    }

    #[test]
    fn a_text_of_exactly_the_most_is_confirmed() {
        let mut dialog = dialog();
        let most = "é".repeat(64);
        typed(&mut dialog, &most);
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(TextInputOutcome::Confirm(most)),
            "counted in characters"
        );
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
        let error = Some("DITL 3001 has no edit text item 5 and Cancel item 6".to_owned());
        let build = |template: &DialogTemplate| {
            TextInputDialog::new(
                template,
                PROMPT,
                DEFAULT,
                64,
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics),
            )
            .err()
        };
        let mut short = stock();
        short.items.truncate(5);
        assert_eq!(build(&short), error);
        let mut plain = stock();
        plain.items[FIELD_ITEM - 1].kind = ItemSpec::StaticText(String::new());
        assert_eq!(build(&plain), error);
        let mut shorter = stock();
        shorter.items.truncate(4);
        assert_eq!(build(&shorter), error);
    }

    #[test]
    fn the_fallback_lays_out_the_same_items_centred() {
        let mut dialog = TextInputDialog::fallback(
            PROMPT,
            DEFAULT,
            64,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        );
        let bounds = dialog.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (360.0, 138.0));
        let origin = bounds.min;
        assert_eq!(
            dialog.field().rect(),
            rect(91.0, 64.0, 200.0, 16.0).offset(origin)
        );
        for (item, at) in [
            (OK_ITEM, rect(252.0, 106.0, 70.0, 20.0)),
            (PROMPT_ITEM, rect(52.0, 5.0, 295.0, 50.0)),
            (CANCEL_ITEM, rect(170.0, 106.0, 70.0, 20.0)),
        ] {
            assert_eq!(dialog.dialog().item_bounds(item), Some(at.offset(origin)));
            assert!(dialog.dialog().item_shown(item), "{item}");
        }
        let texts = texts(&dialog);
        for shown in ["OK", "Cancel", DEFAULT] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
        click(&mut dialog, CANCEL_ITEM);
        assert_eq!(dialog.take_outcome(), Some(TextInputOutcome::Cancel));
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
    fn debug_shows_the_text_and_the_outcome() {
        let dialog = dialog();
        assert_eq!(
            format!("{dialog:?}"),
            format!("TextInputDialog {{ text: {DEFAULT:?}, outcome: None, .. }}")
        );
    }
}
