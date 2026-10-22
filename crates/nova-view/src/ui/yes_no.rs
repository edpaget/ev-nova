//! The Yes/No dialog (`DLOG` 3002, `_EVYesNoDialog` @0x5558c): a question
//! and two buttons, such as the main screen's close box asking
//! [`QUIT_QUESTION`].
//!
//! `_EVYesNoDialog` opens `DLOG` 3002, makes OK (item 1) the default,
//! copies the question into item 3 and waits in `ModalDialog` with no
//! filter of its own (@0x5569e) until OK, the yes, or Cancel (item 5),
//! the no, is hit. Return answers yes; Escape does nothing, as the
//! player info screen's Jettison question reads it. It draws none of the
//! game's picture frames and plays no sound of its own.

use std::rc::Rc;
use std::time::Duration;

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, Role, STATIC_TEXT_COLOR, STATIC_TEXT_SIZE,
};

/// The Yes/No dialog's `DLOG` (and `DITL`) ID.
pub const YES_NO_DIALOG: i16 = 3002;
/// Its OK, the yes.
pub const YES_ITEM: usize = 1;
/// Its question.
pub const QUESTION_ITEM: usize = 3;
/// Its Cancel, the no.
pub const NO_ITEM: usize = 5;

/// `STR#` 140 #10, the question the main screen's close box asks
/// (`_MainScreenMouse` @0x1aec3-0x1aee9). `STR#` 140 is in the interface
/// file, not the game data, so the text is kept here, checked against
/// stock by a stock test.
pub const QUIT_QUESTION: &str = "Are you sure you want to quit?";

/// `STR#` 140 #8, the question New Pilot asks with a pilot already
/// loaded (`_MainScreenChoice` @0x1a74a-0x1a78f), kept here as
/// [`QUIT_QUESTION`] is.
pub const NEW_PILOT_QUESTION: &str =
    "There\u{2019}s already a pilot file loaded. Are you sure you want to create a new one?";

/// The Yes/No dialog asking a question, until it is answered.
#[derive(Clone, Debug)]
pub struct YesNoDialog {
    dialog: Dialog,
    question: String,
    answer: Option<bool>,
}

impl YesNoDialog {
    /// The dialog `template` (stock `DLOG` 3002) asking `question` in item
    /// 3, its buttons labelled in `style`, its text measured by `metrics`.
    #[must_use]
    pub fn new(
        template: &DialogTemplate,
        question: &str,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let roles = [(
            QUESTION_ITEM,
            Role::ScrollText {
                text: question.to_owned(),
                font: Font::Geneva,
                size: STATIC_TEXT_SIZE,
                color: STATIC_TEXT_COLOR,
            },
        )];
        let mut dialog = Dialog::new(template, &roles, metrics)
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(YES_ITEM))
            .with_cancel(None);
        dialog.set_text(QUESTION_ITEM, question);
        Self {
            dialog,
            question: question.to_owned(),
            answer: None,
        }
    }

    /// The answer, once: yes is `true`.
    pub fn take_answer(&mut self) -> Option<bool> {
        self.answer.take()
    }

    /// The question asked.
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }
}

impl Screen for YesNoDialog {
    /// Every event goes to the dialog: OK (a click or Return) answers yes,
    /// Cancel (a click) no. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match self.dialog.input(input) {
            Some(DialogEvent::Item(YES_ITEM)) => self.answer = Some(true),
            Some(DialogEvent::Item(NO_ITEM)) => self.answer = Some(false),
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The dialog over black, as the player info screen draws its
    /// question.
    fn draw(&self, list: &mut DrawList) {
        fill_rect(list, self.dialog.bounds(), Color::BLACK);
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

    /// Stock `DITL` 3002's shape at (0, 0): OK (1), a parked picture (2),
    /// the question (3), an icon (4) and Cancel (5).
    fn template() -> DialogTemplate {
        let item = |bounds, kind| ItemTemplate {
            bounds,
            enabled: true,
            kind,
        };
        DialogTemplate {
            bounds: rect(0.0, 0.0, 355.0, 109.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(263.0, 75.0, 70.0, 20.0), ItemSpec::Button("OK".into())),
                item(rect(277.0, 131.0, 68.0, 30.0), ItemSpec::Picture(129)),
                item(
                    rect(52.0, 5.0, 295.0, 50.0),
                    ItemSpec::StaticText("Text".into()),
                ),
                item(rect(7.0, 5.0, 32.0, 32.0), ItemSpec::Icon(0)),
                item(
                    rect(181.0, 75.0, 70.0, 20.0),
                    ItemSpec::Button("Cancel".into()),
                ),
            ],
        }
    }

    fn asking() -> YesNoDialog {
        YesNoDialog::new(
            &template(),
            QUIT_QUESTION,
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

    fn click(dialog: &mut YesNoDialog, item: usize) {
        let at = dialog
            .dialog()
            .item_bounds(item)
            .expect("the item")
            .center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    #[test]
    fn ok_by_a_click_or_return_answers_yes_once() {
        let mut dialog = asking();
        click(&mut dialog, YES_ITEM);
        assert_eq!(dialog.take_answer(), Some(true));
        assert_eq!(dialog.take_answer(), None, "once");
        let mut dialog = asking();
        assert_eq!(dialog.input(&key(Key::Enter)), ScreenAction::None);
        assert_eq!(dialog.take_answer(), Some(true));
    }

    #[test]
    fn cancel_by_a_click_answers_no() {
        let mut dialog = asking();
        click(&mut dialog, NO_ITEM);
        assert_eq!(dialog.take_answer(), Some(false));
    }

    #[test]
    fn escape_and_other_input_answer_nothing() {
        let mut dialog = asking();
        for input in [
            key(Key::Escape),
            key(Key::Tab),
            key(Key::Char('y')),
            Input::Text('y'),
        ] {
            dialog.input(&input);
        }
        dialog.tick(Duration::from_secs(1));
        assert_eq!(dialog.take_answer(), None);
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: dialog.dialog().item_bounds(YES_ITEM).expect("OK").center(),
        });
        dialog.cancel_pointer();
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: dialog.dialog().item_bounds(YES_ITEM).expect("OK").center(),
        });
        assert_eq!(dialog.take_answer(), None, "a cancelled click");
    }

    #[test]
    fn it_asks_its_question_in_item_3_over_black_with_no_sound() {
        let mut dialog = asking();
        assert_eq!(dialog.question(), QUIT_QUESTION);
        let shown = dialog.dialog().text(QUESTION_ITEM).expect("the question");
        assert_eq!(shown.lines(), [QUIT_QUESTION]);
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        let mut black = DrawList::new();
        fill_rect(&mut black, dialog.dialog().bounds(), Color::BLACK);
        assert_eq!(list.iter().next(), black.iter().next());
        let labels: Vec<&str> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(labels, ["OK", QUIT_QUESTION, "Cancel"]);
        click(&mut dialog, NO_ITEM);
        assert_eq!(dialog.take_sounds(), []);
    }
}
