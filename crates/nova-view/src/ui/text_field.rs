//! A one-line text field: typed characters append to its text and
//! Backspace deletes the last, with a caret after the text. The only
//! selection is the whole text, which a field may start with
//! ([`TextField::with_text`]), or its first characters
//! ([`TextField::select_first`]): typing replaces it, and Backspace
//! clears it, the caret then where it was. There is no moving the caret
//! otherwise.

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
/// The fill behind a selected text.
pub const SELECTION: Color = Color::rgba(64, 64, 160, 255);
/// How far in from the field's left edge the text starts.
pub const INSET: f32 = 3.0;

/// A one-line text field holding at most a maximum number of characters.
#[derive(Clone, Debug, PartialEq)]
pub struct TextField {
    rect: Bounds,
    text: String,
    max_chars: usize,
    /// Where the caret is, in characters from the start.
    caret: usize,
    /// How many characters from the start are selected; none when 0.
    selection: usize,
}

impl TextField {
    /// An empty field at `rect` that holds up to `max_chars` characters.
    #[must_use]
    pub fn new(rect: Bounds, max_chars: usize) -> Self {
        Self {
            rect,
            text: String::new(),
            max_chars,
            caret: 0,
            selection: 0,
        }
    }

    /// A field at `rect` that holds up to `max_chars` characters, holding
    /// `text` (even beyond them), selected whole.
    #[must_use]
    pub fn with_text(rect: Bounds, max_chars: usize, text: &str) -> Self {
        let mut field = Self {
            text: text.to_owned(),
            ..Self::new(rect, max_chars)
        };
        field.select_first(usize::MAX);
        field
    }

    /// Selects the first `chars` characters, or the whole text when it is
    /// shorter, with the caret at the selection's end.
    pub fn select_first(&mut self, chars: usize) {
        self.selection = chars.min(self.text.chars().count());
        self.caret = self.selection;
    }

    /// Whether any of its text is selected.
    #[must_use]
    pub fn selected(&self) -> bool {
        self.selection > 0
    }

    /// How many characters from the start are selected: none when 0.
    #[must_use]
    pub fn selection(&self) -> usize {
        self.selection
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

    /// The byte offset of character `chars` of the text.
    fn offset(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(at, _)| at)
    }

    /// Takes one input and says whether the text changed: a typed
    /// character goes in at the caret while there is room for it, and a
    /// Backspace press (its repeats included) deletes the character before
    /// the caret. While some text is selected, a typed character replaces
    /// it and Backspace deletes it, and either ends the selection.
    /// Everything else is ignored.
    pub fn input(&mut self, input: &Input) -> bool {
        let backspace = matches!(
            *input,
            Input::Key {
                key: DELETE_KEY,
                pressed: true,
                ..
            }
        );
        let typed = match *input {
            Input::Text(c) => Some(c),
            _ => None,
        };
        if !backspace && typed.is_none() {
            return false;
        }
        let cleared = self.selected();
        if cleared {
            self.text.replace_range(..self.offset(self.selection), "");
            self.selection = 0;
            self.caret = 0;
        }
        if let Some(c) = typed {
            if self.text.chars().count() >= self.max_chars {
                return cleared;
            }
            let at = self.offset(self.caret);
            self.text.insert(at, c);
            self.caret += 1;
            return true;
        }
        if cleared || self.caret == 0 {
            return cleared;
        }
        self.caret -= 1;
        let at = self.offset(self.caret);
        self.text.remove(at);
        true
    }

    /// The background, its outline, the fill behind the text selected,
    /// the text, and the caret where it is when nothing is selected,
    /// measured by `metrics`.
    pub fn draw(&self, metrics: &dyn TextMetrics, list: &mut DrawList) {
        fill_rect(list, self.rect, BACKGROUND);
        outline(list, self.rect, EDGE);
        let height = metrics.line_height(Font::Geneva, TEXT_SIZE);
        let top = self.rect.center().y - height / 2.0;
        let left = self.rect.min.x + INSET;
        let before =
            |chars: usize| metrics.width(Font::Geneva, TEXT_SIZE, &self.text[..self.offset(chars)]);
        if self.selected() {
            fill_rect(
                list,
                Bounds::at(Point::new(left, top), before(self.selection), height),
                SELECTION,
            );
        }
        list.text(
            self.text.clone(),
            Point::new(left, top),
            TEXT_SIZE,
            None,
            TEXT_COLOR,
        );
        if self.selected() {
            return;
        }
        let caret = left + before(self.caret);
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

    fn selected(text: &str) -> TextField {
        TextField::with_text(Bounds::at(Point::new(100.0, 50.0), 170.0, 16.0), 64, text)
    }

    #[test]
    fn a_field_with_text_starts_selected_and_typing_replaces_it() {
        let mut field = selected("Ship 129 491");
        assert_eq!(field.text(), "Ship 129 491");
        assert!(field.selected());
        assert!(field.input(&Input::Text('K')));
        assert_eq!(field.text(), "K");
        assert!(!field.selected());
        assert!(!self::field(31).selected(), "a new field has none");
    }

    #[test]
    fn backspace_clears_a_selected_text() {
        let mut field = selected("Ship 129 491");
        assert!(field.input(&key(Key::Backspace, true, false)));
        assert_eq!(field.text(), "");
        assert!(!field.selected());
        let mut empty = selected("");
        assert!(
            !empty.input(&key(Key::Backspace, true, false)),
            "nothing to clear"
        );
    }

    #[test]
    fn a_released_backspace_or_other_input_keeps_the_selection() {
        let mut field = selected("Ship");
        for input in [
            key(Key::Backspace, false, false),
            key(Key::Left, true, false),
            Input::PointerMoved(Point::new(110.0, 55.0)),
        ] {
            assert!(!field.input(&input), "{input:?}");
            assert!(field.selected(), "{input:?}");
        }
        assert_eq!(field.text(), "Ship");
    }

    #[test]
    fn after_an_edit_the_field_appends_again() {
        let mut field = selected("Ship 129 491");
        typed(&mut field, "Kes");
        assert_eq!(field.text(), "Kes");
        field.input(&key(Key::Backspace, true, false));
        assert_eq!(field.text(), "Ke");
        typed(&mut field, "strel");
        assert_eq!(field.text(), "Kestrel");
    }

    #[test]
    fn a_selected_text_is_replaced_even_when_the_field_is_full() {
        let mut field =
            TextField::with_text(Bounds::at(Point::new(100.0, 50.0), 170.0, 16.0), 3, "Abc");
        assert!(field.input(&Input::Text('d')));
        assert_eq!(field.text(), "d");
    }

    #[test]
    fn typing_over_the_first_characters_selected_keeps_the_rest() {
        let mut field = selected("abcdef");
        field.select_first(3);
        assert_eq!(field.selection(), 3);
        assert!(field.selected());
        assert!(field.input(&Input::Text('X')));
        assert_eq!(field.text(), "Xdef");
        assert_eq!(field.selection(), 0);
        typed(&mut field, "Yé");
        assert_eq!(field.text(), "XYédef", "typed at the caret");
        assert!(field.input(&key(Key::Backspace, true, false)));
        assert_eq!(field.text(), "XYdef", "deleted before the caret");
    }

    #[test]
    fn backspace_clears_the_first_characters_selected() {
        let mut field = selected("abcdef");
        field.select_first(2);
        assert!(field.input(&key(Key::Backspace, true, false)));
        assert_eq!(field.text(), "cdef");
        assert!(
            !field.input(&key(Key::Backspace, true, false)),
            "nothing before the caret"
        );
        assert_eq!(field.text(), "cdef");
        typed(&mut field, "ab");
        assert_eq!(field.text(), "abcdef");
    }

    #[test]
    fn a_selection_is_no_longer_than_the_text() {
        let mut field = selected("abc");
        field.select_first(10);
        assert_eq!(field.selection(), 3);
        assert_eq!(selected("abc").selection(), 3);
        assert_eq!(self::field(31).selection(), 0);
    }

    #[test]
    fn the_first_characters_selected_are_filled_and_the_caret_follows_an_edit() {
        let mut field = selected("Adabc");
        field.select_first(3);
        let height = LINE_HEIGHT * 12.0;
        let top = 58.0 - height / 2.0;
        let mut list = DrawList::new();
        field.draw(&MonoMetrics, &mut list);
        let mut expected = DrawList::new();
        fill_rect(&mut expected, field.rect(), BACKGROUND);
        outline(&mut expected, field.rect(), EDGE);
        fill_rect(
            &mut expected,
            Bounds::at(Point::new(103.0, top), 18.0, height),
            SELECTION,
        );
        expected.text("Adabc", Point::new(103.0, top), 12.0, None, TEXT_COLOR);
        assert_eq!(list, expected);
        typed(&mut field, "Zo");
        let mut list = DrawList::new();
        field.draw(&MonoMetrics, &mut list);
        let mut expected = DrawList::new();
        fill_rect(&mut expected, field.rect(), BACKGROUND);
        outline(&mut expected, field.rect(), EDGE);
        expected.text("Zobc", Point::new(103.0, top), 12.0, None, TEXT_COLOR);
        // After "Zo", two characters each 6 wide.
        expected.line(
            Point::new(115.0, top),
            Point::new(115.0, top + height),
            1.0,
            EDGE,
        );
        assert_eq!(list, expected);
    }

    #[test]
    fn a_selected_text_is_drawn_with_a_fill_and_no_caret() {
        let field = selected("Ada");
        let mut list = DrawList::new();
        field.draw(&MonoMetrics, &mut list);
        let rect = field.rect();
        let mut expected = DrawList::new();
        fill_rect(&mut expected, rect, BACKGROUND);
        outline(&mut expected, rect, EDGE);
        let height = LINE_HEIGHT * 12.0;
        let top = 58.0 - height / 2.0;
        // Three characters, each 6 wide.
        fill_rect(
            &mut expected,
            Bounds::at(Point::new(103.0, top), 18.0, height),
            SELECTION,
        );
        expected.text("Ada", Point::new(103.0, top), 12.0, None, TEXT_COLOR);
        assert_eq!(list, expected);
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
