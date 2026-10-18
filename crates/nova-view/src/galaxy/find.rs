//! The galaxy map's Find dialog (`DLOG`/`DITL` 3005): a prompt, the name
//! of a system to type, and Find or Cancel.
//!
//! `_DoSystemSearch` (@0x11c86-0x12102) opens it from the map's Find
//! button, or F or S (`_MapFilter` @0x144b9-0x144d6). The stock dialog is
//! 284 x 105, a plain modal box, centred: Find (item 1, the default), a
//! picture (item 2, `PICT` 130, not drawn yet), Cancel (item 3), the
//! prompt "Enter the name of a system to find:" (item 4) and the field
//! (item 5). The original centres it over the map dialog
//! (`_CenterWindow` @0x11cf1), itself centred on the game window
//! (@0x12534), so this dialog takes its own centred placement.
//!
//! The field starts empty (@0x11d72-0x11d92), and the events pending as
//! it opens are flushed (`FlushEvents` @0x11da6), so the key that opened
//! it types nothing into it ([`EditDialog::flush_typed_key`]). Find is the
//! default item (`SetDialogDefaultItem` @0x11cc6), so Return and Enter
//! give it, and there is no cancel item, so Escape does nothing. Find
//! gives the text as typed (`GetDialogItemText` @0x11e11), and Cancel
//! gives nothing ([`FindOutcome::Cancel`]); the map decides what an
//! empty text does (it does nothing). Every key and button is silent:
//! `ModalDialog` runs with a NULL filter (@0x11dd1).
//!
//! `_FrameDefaultButton` (@0x11d31) draws nothing on Mac OS X (it returns
//! when `_gSystemVersion` > 0xfff, @0x55215), so no ring is drawn round
//! Find. Like the other edit-field dialogs, it is drawn over a dark
//! backdrop with a 1-unit outline: the shared edit-field shell
//! ([`EditDialog`]) with the [`Search`] policy.

use std::rc::Rc;

use crate::geometry::{Bounds, Point};
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::{DialogTemplate, ItemSpec, ItemTemplate, Placement};
use crate::ui::edit_dialog::{Confirm, EditDialog, EditItems};
use crate::ui::text_field::TextField;

/// The dialog's `DLOG` (and `DITL`) ID.
pub const FIND_DIALOG: i16 = 3005;
/// Find's item: the default, which gives the text.
pub const FIND_ITEM: usize = 1;
/// Cancel's item.
pub const CANCEL_ITEM: usize = 3;
/// The prompt's item.
pub const PROMPT_ITEM: usize = 4;
/// The field's item.
pub const FIELD_ITEM: usize = 5;

/// The stock prompt, DITL 3005 item 4's text.
pub const PROMPT: &str = "Enter the name of a system to find:";

/// What the player chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindOutcome {
    /// Find, with this text as typed: not trimmed, and possibly empty.
    Search(String),
    /// Cancel.
    Cancel,
}

/// Find's rule: Find always gives the text, and Cancel gives nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Search;

impl Confirm for Search {
    type Outcome = FindOutcome;

    /// The text as typed, empty or not.
    fn confirm(&self, field: &mut TextField) -> Option<FindOutcome> {
        Some(FindOutcome::Search(field.text().to_owned()))
    }

    /// Cancel gives nothing to search for.
    fn cancelled(&self) -> FindOutcome {
        FindOutcome::Cancel
    }
}

/// The Find dialog.
pub type FindDialog = EditDialog<Search>;

impl std::fmt::Debug for EditDialog<Search> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FindDialog")
            .field("text", &self.field().text())
            .field("outcome", &self.outcome())
            .finish_non_exhaustive()
    }
}

/// The items the shell drives.
const ITEMS: EditItems = EditItems {
    id: FIND_DIALOG,
    ok: FIND_ITEM,
    prompt: PROMPT_ITEM,
    field: FIELD_ITEM,
    cancel: CANCEL_ITEM,
};

impl EditDialog<Search> {
    /// The dialog `template` (stock `DLOG` 3005), asking its own item 4's
    /// text, with the field empty; its buttons labelled in `style` and its
    /// text measured by `metrics`.
    ///
    /// # Errors
    ///
    /// When the template's item 5 is not edit text, or it has no item 3.
    pub fn new(
        template: &DialogTemplate,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        let prompt = match template.items.get(PROMPT_ITEM - 1) {
            Some(ItemTemplate {
                kind: ItemSpec::StaticText(text),
                ..
            }) => text.clone(),
            _ => String::new(),
        };
        Self::laid_out(template, ITEMS, &[], &prompt, "", Search, style, metrics)
    }

    /// The dialog laid out without the interface file, as the stock one
    /// is.
    #[must_use]
    pub fn fallback(style: ButtonStyle, metrics: Rc<dyn TextMetrics>) -> Self {
        Self::new(&fallback(), style, metrics).expect("the fallback has a field and Cancel")
    }
}

/// The built-in Find dialog, as stock DITL 3005: 284 x 105, centred, with
/// Find (1), the picture (2), Cancel (3), the prompt (4) and the field
/// (5).
#[must_use]
pub fn fallback() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let item = |bounds, enabled, kind| ItemTemplate {
        bounds,
        enabled,
        kind,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 284.0, 105.0),
        placement: Placement::Center,
        items: vec![
            item(
                at(165.0, 72.0, 70.0, 20.0),
                true,
                ItemSpec::Button("Find".to_owned()),
            ),
            item(at(7.0, 5.0, 32.0, 32.0), false, ItemSpec::Picture(130)),
            item(
                at(83.0, 72.0, 70.0, 20.0),
                true,
                ItemSpec::Button("Cancel".to_owned()),
            ),
            item(
                at(50.0, 7.0, 243.0, 17.0),
                false,
                ItemSpec::StaticText(PROMPT.to_owned()),
            ),
            item(
                at(60.0, 38.0, 200.0, 16.0),
                true,
                ItemSpec::EditText(String::new()),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{DrawCommand, DrawList};
    use crate::input::{Input, Key, MouseButton};
    use crate::screen::Screen;
    use crate::sound::Sound;
    use crate::text::fixture::MonoMetrics;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock DITL 3005's items, the dialog fixed at (0, 0), its prompt
    /// `prompt`.
    fn stock(prompt: &str) -> DialogTemplate {
        let mut template = fallback();
        template.placement = Placement::Fixed;
        template.items[PROMPT_ITEM - 1].kind = ItemSpec::StaticText(prompt.to_owned());
        template
    }

    fn dialog() -> FindDialog {
        FindDialog::new(&stock(PROMPT), ButtonStyle::STOCK, Rc::new(MonoMetrics)).expect("builds")
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn typed(dialog: &mut FindDialog, text: &str) {
        for c in text.chars() {
            dialog.input(&key(Key::Char(c.to_ascii_lowercase())));
            dialog.input(&Input::Text(c));
        }
    }

    fn click(dialog: &mut FindDialog, item: usize) -> Vec<Sound> {
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

    fn texts(dialog: &FindDialog) -> Vec<String> {
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
    fn it_opens_empty_with_the_templates_prompt_and_find_and_cancel() {
        let dialog = FindDialog::new(
            &stock("Which system?"),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        assert_eq!(dialog.field().text(), "");
        assert_eq!(dialog.field().rect(), rect(60.0, 38.0, 200.0, 16.0));
        assert_eq!(texts(&dialog), ["Find", "Cancel", "Which system?", ""]);
        assert_eq!(FIND_DIALOG, 3005);
        assert_eq!(
            (FIND_ITEM, CANCEL_ITEM, PROMPT_ITEM, FIELD_ITEM),
            (1, 3, 4, 5)
        );
    }

    #[test]
    fn a_prompt_that_is_not_text_asks_nothing() {
        let mut template = stock(PROMPT);
        template.items[PROMPT_ITEM - 1].kind = ItemSpec::Picture(5);
        let dialog =
            FindDialog::new(&template, ButtonStyle::STOCK, Rc::new(MonoMetrics)).expect("builds");
        assert!(!texts(&dialog).contains(&PROMPT.to_owned()));
    }

    #[test]
    fn return_enter_and_find_give_the_text_as_typed() {
        let mut dialog = dialog();
        typed(&mut dialog, " Sol ");
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(FindOutcome::Search(" Sol ".to_owned())),
            "untrimmed"
        );
        assert_eq!(dialog.take_outcome(), None, "taken once");
        assert_eq!(click(&mut dialog, FIND_ITEM), []);
        assert_eq!(
            dialog.take_outcome(),
            Some(FindOutcome::Search(" Sol ".to_owned()))
        );
        let mut empty = self::dialog();
        empty.input(&key(Key::Enter));
        assert_eq!(
            empty.take_outcome(),
            Some(FindOutcome::Search(String::new()))
        );
    }

    #[test]
    fn cancel_gives_cancel_and_escape_nothing() {
        let mut dialog = dialog();
        typed(&mut dialog, "sol");
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(click(&mut dialog, CANCEL_ITEM), []);
        assert_eq!(dialog.take_outcome(), Some(FindOutcome::Cancel));
    }

    #[test]
    fn every_key_and_click_is_silent() {
        let mut dialog = dialog();
        typed(&mut dialog, "ab");
        for k in [
            Key::Backspace,
            Key::Space,
            Key::Escape,
            Key::Tab,
            Key::Enter,
        ] {
            dialog.input(&key(k));
        }
        assert_eq!(dialog.take_sounds(), []);
        click(&mut dialog, FIND_ITEM);
        click(&mut dialog, CANCEL_ITEM);
        assert_eq!(dialog.take_sounds(), []);
    }

    #[test]
    fn after_a_flush_the_next_typed_character_is_dropped() {
        let mut dialog = dialog();
        dialog.flush_typed_key();
        dialog.input(&Input::Text('f'));
        assert_eq!(dialog.field().text(), "");
        dialog.input(&Input::Text('S'));
        assert_eq!(dialog.field().text(), "S");
    }

    #[test]
    fn the_fallback_lays_out_the_stock_items_centred() {
        let mut dialog = FindDialog::fallback(ButtonStyle::STOCK, Rc::new(MonoMetrics));
        let bounds = dialog.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (284.0, 105.0));
        assert_eq!(
            bounds.min,
            Point::new(370.0, 331.0),
            "centred on the screen"
        );
        let origin = bounds.min;
        assert_eq!(
            dialog.field().rect(),
            rect(60.0, 38.0, 200.0, 16.0).offset(origin)
        );
        for (item, at) in [
            (FIND_ITEM, rect(165.0, 72.0, 70.0, 20.0)),
            (CANCEL_ITEM, rect(83.0, 72.0, 70.0, 20.0)),
            (PROMPT_ITEM, rect(50.0, 7.0, 243.0, 17.0)),
        ] {
            assert_eq!(dialog.dialog().item_bounds(item), Some(at.offset(origin)));
        }
        assert!(texts(&dialog).contains(&PROMPT.to_owned()));
        click(&mut dialog, CANCEL_ITEM);
        assert_eq!(dialog.take_outcome(), Some(FindOutcome::Cancel));
    }

    #[test]
    fn a_template_without_its_field_or_cancel_is_an_error() {
        let error = Some("DITL 3005 has no edit text item 5 and Cancel item 3".to_owned());
        let build = |template: &DialogTemplate| {
            FindDialog::new(template, ButtonStyle::STOCK, Rc::new(MonoMetrics)).err()
        };
        let mut plain = stock(PROMPT);
        plain.items[FIELD_ITEM - 1].kind = ItemSpec::StaticText(String::new());
        assert_eq!(build(&plain), error);
        let mut short = stock(PROMPT);
        short.items.truncate(2);
        assert_eq!(build(&short), error);
    }

    #[test]
    fn debug_shows_the_text_and_the_outcome() {
        let mut dialog = dialog();
        typed(&mut dialog, "sol");
        assert_eq!(
            format!("{dialog:?}"),
            "FindDialog { text: \"sol\", outcome: None, .. }"
        );
    }
}
