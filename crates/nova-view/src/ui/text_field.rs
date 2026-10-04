//! A one-line text field: typed characters append to its text and
//! Backspace deletes the last, with a caret after the text. There is no
//! selection and no moving the caret.

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::text::TextMetrics;

use super::dialog::outline;

/// The key that deletes the character before the caret: Backspace
/// (Delete on a Mac keyboard).
pub const DELETE_KEY: Key = Key::Backspace;

/// The text's size, in Geneva.
pub const TEXT_SIZE: f32 = 12.0;
/// The text's colour.
pub const TEXT_COLOR: Color = Color::WHITE;
/// The field's background.
pub const BACKGROUND: Color = Color::BLACK;
/// The field's outline and caret.
pub const EDGE: Color = Color::DIM;
/// How far in from the field's left edge the text starts.
pub const INSET: f32 = 3.0;

/// A one-line text field holding at most a maximum number of characters.
#[derive(Clone, Debug, PartialEq)]
pub struct TextField {
    rect: Bounds,
    text: String,
    max_chars: usize,
}

impl TextField {
    /// An empty field at `rect` that holds up to `max_chars` characters.
    #[must_use]
    pub fn new(rect: Bounds, max_chars: usize) -> Self {
        Self {
            rect,
            text: String::new(),
            max_chars,
        }
    }

    /// Where it is.
    #[must_use]
    pub fn rect(&self) -> Bounds {
        self.rect
    }

    /// The text typed.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Takes one input and says whether the text changed: a typed
    /// character is appended while there is room for it, and a Backspace
    /// press (its repeats included) deletes the last character. Everything
    /// else is ignored.
    pub fn input(&mut self, input: &Input) -> bool {
        match *input {
            Input::Text(c) if self.text.chars().count() < self.max_chars => {
                self.text.push(c);
                true
            }
            Input::Key {
                key: DELETE_KEY,
                pressed: true,
                ..
            } => self.text.pop().is_some(),
            _ => false,
        }
    }

    /// The background, its outline, the text and the caret after it,
    /// measured by `metrics`.
    pub fn draw(&self, metrics: &dyn TextMetrics, list: &mut DrawList) {
        fill_rect(list, self.rect, BACKGROUND);
        outline(list, self.rect, EDGE);
        let height = metrics.line_height(Font::Geneva, TEXT_SIZE);
        let top = self.rect.center().y - height / 2.0;
        let left = self.rect.min.x + INSET;
        list.text(
            self.text.clone(),
            Point::new(left, top),
            TEXT_SIZE,
            None,
            TEXT_COLOR,
        );
        let caret = left + metrics.width(Font::Geneva, TEXT_SIZE, &self.text);
        list.line(
            Point::new(caret, top),
            Point::new(caret, top + height),
            1.0,
            EDGE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::text::LINE_HEIGHT;
    use crate::text::fixture::MonoMetrics;

    fn field(max_chars: usize) -> TextField {
        TextField::new(Bounds::at(Point::new(100.0, 50.0), 170.0, 16.0), max_chars)
    }

    fn typed(field: &mut TextField, text: &str) {
        for c in text.chars() {
            field.input(&Input::Text(c));
        }
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    #[test]
    fn it_starts_empty_where_it_is_put() {
        let field = field(31);
        assert_eq!(field.text(), "");
        assert_eq!(
            field.rect(),
            Bounds::at(Point::new(100.0, 50.0), 170.0, 16.0)
        );
    }

    #[test]
    fn typed_characters_append() {
        let mut field = field(31);
        assert!(field.input(&Input::Text('A')));
        typed(&mut field, "da Zoë");
        assert_eq!(field.text(), "Ada Zoë");
    }

    #[test]
    fn it_holds_no_more_than_its_maximum() {
        let mut field = field(3);
        typed(&mut field, "Ab");
        assert!(field.input(&Input::Text('c')));
        assert!(!field.input(&Input::Text('d')), "full");
        assert_eq!(field.text(), "Abc");
        let mut accented = self::field(3);
        typed(&mut accented, "éééé");
        assert_eq!(accented.text(), "ééé", "counted in characters");
    }

    #[test]
    fn backspace_deletes_the_last_character_repeats_included() {
        let mut field = field(31);
        typed(&mut field, "Adaë");
        assert!(field.input(&key(Key::Backspace, true, false)));
        assert_eq!(field.text(), "Ada");
        assert!(field.input(&key(Key::Backspace, true, true)));
        assert_eq!(field.text(), "Ad");
        assert!(
            !field.input(&key(Key::Backspace, false, false)),
            "a release"
        );
        assert_eq!(field.text(), "Ad");
        field.input(&key(Key::Backspace, true, false));
        field.input(&key(Key::Backspace, true, false));
        assert!(!field.input(&key(Key::Backspace, true, false)), "empty");
        assert_eq!(field.text(), "");
    }

    #[test]
    fn other_input_is_ignored() {
        let mut field = field(31);
        typed(&mut field, "Ada");
        for input in [
            key(Key::Char('x'), true, false),
            key(Key::Left, true, false),
            key(Key::Enter, true, false),
            Input::PointerMoved(Point::new(110.0, 55.0)),
        ] {
            assert!(!field.input(&input), "{input:?}");
        }
        assert_eq!(field.text(), "Ada");
    }

    #[test]
    fn it_draws_a_box_its_text_and_a_caret_after_it() {
        let mut field = field(31);
        typed(&mut field, "Ada");
        let mut list = DrawList::new();
        field.draw(&MonoMetrics, &mut list);
        let rect = field.rect();
        let mut expected = DrawList::new();
        fill_rect(&mut expected, rect, BACKGROUND);
        outline(&mut expected, rect, EDGE);
        let height = LINE_HEIGHT * 12.0;
        let top = 58.0 - height / 2.0;
        expected.text("Ada", Point::new(103.0, top), 12.0, None, TEXT_COLOR);
        // Three characters, each 6 wide.
        expected.line(
            Point::new(121.0, top),
            Point::new(121.0, top + height),
            1.0,
            EDGE,
        );
        assert_eq!(list, expected);
        assert!(matches!(
            list.iter().nth(5),
            Some(DrawCommand::Text { text, .. }) if text == "Ada"
        ));
    }
}
