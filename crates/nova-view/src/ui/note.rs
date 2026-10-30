//! The note alert (`DLOG` 3000, `_EVNoteAlert`): a line of text and an OK
//! button, such as the warning that an opened pilot's data had to fall
//! back.
//!
//! `_EVNoteAlert` (@0x55710) opens `DLOG` 3000, a plain modal box
//! (`procID` 1), sets OK as the default and frames it
//! (`SetDialogDefaultItem`, `_FrameDefaultButton`), copies the text into
//! item 3 (`SetDialogItemText`) and waits in `ModalDialog` until item 1 is
//! hit. It draws none of the game's picture frames, so the note is drawn
//! frameless, over the backdrop and outline the engine's other plain
//! dialogs use, and it plays no sound of its own.

use std::rc::Rc;
use std::time::Duration;

use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, FocusRing, Role, STATIC_TEXT_COLOR, STATIC_TEXT_SIZE,
    outline,
};
use super::prefs::{BACKDROP, BORDER};

/// The note alert's `DLOG` (and `DITL`) ID.
pub const NOTE_DIALOG: i16 = 3000;

/// The OK button's item.
pub const OK_ITEM: usize = 1;

/// The text's item.
pub const TEXT_ITEM: usize = 3;

/// The OK button's label, as stock `DITL` 3000 titles it.
pub const OK_LABEL: &str = "OK";

/// `STR#` 140 #52, the note `_MainScreenChoice` shows through
/// `_EVNoteAlert` when `_OpenPilotFile` reports that the pilot's load had
/// to fall back (@0x1a817-0x1a85b). `STR#` 140 is in the interface file,
/// not the game data, so the text is kept here, checked against stock by
/// a stock test.
pub const LOAD_FALLBACK_NOTE: &str = "This pilot file appears to reference information that is \
from a plug-in or replacement data set that is not currently loaded. You can continue to use \
this pilot, but it may cause odd behavior.";

/// `STR#` 140 #34, the note `_OpenPilotFile` shows through `_EVNoteAlert`
/// when `_LoadPilotData` reports a pilot file of another version (-42,
/// @0x766c8-0x766ef), kept here as [`LOAD_FALLBACK_NOTE`] is.
pub const VERSION_NOTE: &str =
    "This pilot file was created with a different version of Nova, and can\u{2019}t be used.";

/// The note alert: `text` in item 3 and item 1 an OK button that Return,
/// Escape or a click activates, which closes it.
#[derive(Clone, Debug)]
pub struct NoteDialog {
    dialog: Dialog,
    text: String,
    closed: bool,
}

impl NoteDialog {
    /// The dialog `template` (stock `DLOG` 3000) showing `text`, its button
    /// labelled in `style`, its text measured by `metrics`. Item 3 shows
    /// `text` whether the template makes it static text, as stock does, or
    /// a user item.
    #[must_use]
    pub fn new(
        template: &DialogTemplate,
        text: &str,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let roles = [
            (OK_ITEM, Role::Button(OK_LABEL.to_owned())),
            (
                TEXT_ITEM,
                Role::ScrollText {
                    text: text.to_owned(),
                    font: Font::Geneva,
                    size: STATIC_TEXT_SIZE,
                    color: STATIC_TEXT_COLOR,
                    backing: None,
                },
            ),
        ];
        let mut dialog = Dialog::new(template, &roles, metrics)
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(OK_ITEM))
            .with_cancel(Some(OK_ITEM));
        dialog.set_text(TEXT_ITEM, text);
        Self {
            dialog,
            text: text.to_owned(),
            closed: false,
        }
    }

    /// Whether OK has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The text the note shows.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The note with its buttons taking the keyboard focus as `ring`
    /// says (the `dialog_focus_ring` rule).
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        self.dialog.set_focus_ring(ring);
        self
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }
}

impl Screen for NoteDialog {
    /// Every event goes to the dialog; activating OK closes it. It never
    /// quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.dialog.input(input) == Some(DialogEvent::Item(OK_ITEM)) {
            self.closed = true;
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The backdrop and its outline, then the dialog.
    fn draw(&self, list: &mut DrawList) {
        let bounds = self.dialog.bounds();
        fill_rect(list, bounds, BACKDROP);
        outline(list, bounds, BORDER);
        self.dialog.draw(list);
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock `DITL` 3000's shape at (0, 0): OK (1) a standard button, a
    /// parked picture (2), the text (3) static text, and an icon (4).
    fn template() -> DialogTemplate {
        let item = |bounds, enabled, kind| ItemTemplate {
            bounds,
            enabled,
            kind,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 357.0, 150.0),
            placement: Placement::Fixed,
            items: vec![
                item(
                    rect(282.0, 118.0, 70.0, 20.0),
                    true,
                    ItemSpec::Button("OK".to_owned()),
                ),
                item(
                    rect(296.0, 186.0, 68.0, 30.0),
                    false,
                    ItemSpec::Picture(1431),
                ),
                item(
                    rect(52.0, 5.0, 295.0, 99.0),
                    false,
                    ItemSpec::StaticText("Text".to_owned()),
                ),
                item(rect(7.0, 5.0, 32.0, 32.0), false, ItemSpec::Icon(0)),
            ],
        }
    }

    /// The same shape with OK and the text as user items.
    fn user_template() -> DialogTemplate {
        let mut template = template();
        template.items[0].kind = ItemSpec::User;
        template.items[2].kind = ItemSpec::User;
        template
    }

    fn note(template: &DialogTemplate) -> NoteDialog {
        NoteDialog::new(
            template,
            "Odd data.",
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click(note: &mut NoteDialog) {
        let ok = note.dialog().item_bounds(OK_ITEM).expect("OK").center();
        for pressed in [true, false] {
            note.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: ok,
            });
        }
    }

    #[test]
    fn return_escape_or_a_click_on_ok_closes_the_note() {
        for template in [template(), user_template()] {
            for input in [key(Key::Enter), key(Key::Escape)] {
                let mut note = note(&template);
                assert!(!note.closed());
                assert_eq!(note.input(&input), ScreenAction::None);
                assert!(note.closed(), "{input:?}");
            }
            let mut note = note(&template);
            click(&mut note);
            assert!(note.closed());
        }
    }

    #[test]
    fn other_input_leaves_it_open() {
        let mut note = note(&template());
        for input in [key(Key::Space), key(Key::Tab), key(Key::Char('f'))] {
            note.input(&input);
        }
        note.tick(Duration::from_secs(1));
        assert!(!note.closed());
    }

    #[test]
    fn the_note_shows_its_text_in_item_3_and_ok_in_item_1() {
        for template in [template(), user_template()] {
            let note = note(&template);
            assert_eq!(note.text(), "Odd data.");
            let shown = note.dialog().text(TEXT_ITEM).expect("the text item");
            assert_eq!(shown.lines(), ["Odd data."], "{template:?}");
            assert_eq!(shown.rect(), rect(52.0, 5.0, 295.0, 99.0));
            let mut list = DrawList::new();
            note.draw(&mut list);
            let labels: Vec<&str> = list
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(labels, [OK_LABEL, "Odd data."], "{template:?}");
        }
    }

    #[test]
    fn it_draws_frameless_over_the_backdrop() {
        let note = note(&template());
        let mut list = DrawList::new();
        note.draw(&mut list);
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        let mut backdrop = DrawList::new();
        fill_rect(&mut backdrop, note.dialog().bounds(), BACKDROP);
        outline(&mut backdrop, note.dialog().bounds(), BORDER);
        let backdrop: Vec<DrawCommand> = backdrop.iter().cloned().collect();
        assert_eq!(commands[..backdrop.len()], backdrop[..]);
        assert!(
            !commands.iter().any(|command| matches!(
                command,
                DrawCommand::StretchedPicture { image, .. }
                    if *image == crate::ui::desc::FRAME.top
            )),
            "no picture frame"
        );
    }

    /// `_TrackOkButton` @0x26d7a plays nothing under the pointer, and
    /// `_EVNoteAlert` plays nothing of its own.
    #[test]
    fn pointer_click_is_silent_as_the_original() {
        let mut note = note(&template());
        assert_eq!(note.take_sounds(), []);
        let ok = note.dialog().item_bounds(OK_ITEM).expect("OK").center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ok,
        };
        note.input(&button(true));
        assert_eq!(note.take_sounds(), []);
        note.input(&button(false));
        assert_eq!(note.take_sounds(), []);
    }

    /// A key that activates OK plays nothing in the original:
    /// `_EVNoteAlert` @0x55710 passes a NULL filter, and the Toolbox's
    /// standard filter only flashes the button.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for input in [key(Key::Enter), key(Key::Escape)] {
            let mut note = note(&template());
            assert_eq!(note.take_sounds(), []);
            note.input(&input);
            assert!(note.closed(), "{input:?}");
            assert_eq!(note.take_sounds(), [], "{input:?}");
        }
        let mut note = note(&template());
        note.input(&key(Key::Tab));
        note.input(&key(Key::Space));
        assert!(note.closed(), "Space on OK");
        assert_eq!(note.take_sounds(), [], "Space");
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_ok() {
        let mut note = note(&template());
        let ok = note.dialog().item_bounds(OK_ITEM).expect("OK").center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ok,
        };
        note.input(&button(true));
        note.cancel_pointer();
        note.input(&button(false));
        assert!(!note.closed());
    }

    #[test]
    fn the_note_is_dlog_3000() {
        assert_eq!(NOTE_DIALOG, 3000);
    }

    /// The Toolbox's standard filter `_EVNoteAlert` @0x55710 runs maps no
    /// Tab or Space to a button.
    #[test]
    fn tab_then_space_closes_it_only_with_the_focus_ring() {
        let mut ringed = note(&template());
        ringed.input(&key(Key::Tab));
        ringed.input(&key(Key::Space));
        assert!(ringed.closed(), "by default");
        let mut off = note(&template()).with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Tab));
        off.input(&key(Key::Space));
        assert!(!off.closed());
        assert_eq!(off.dialog().focus(), None);
    }
}
