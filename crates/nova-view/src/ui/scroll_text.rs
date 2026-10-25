//! Text wrapped into a box and scrolled through it a whole line at a time.

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key, ScrollDelta};
use crate::text::{TextMetrics, wrap};

/// Text wrapped to a box's width once, when it is made, and shown as many
/// whole lines as fit the box's height (at least one in a box as tall as
/// the text's size), from a first line the arrow keys move, or a wheel or
/// trackpad over the box. Nothing is drawn outside the box, so the
/// renderer needs no clip.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollText {
    lines: Vec<String>,
    font: Font,
    size: f32,
    color: Color,
    rect: Bounds,
    line_height: f32,
    visible: usize,
    first: usize,
    /// What wheel and trackpad scrolls have moved short of a whole line,
    /// in logical units, toward the end for positive.
    pending: f32,
}

impl ScrollText {
    /// `text` in `font` at `size` and `color`, wrapped by `metrics` to the
    /// width of `rect` and shown inside it from the first line.
    #[must_use]
    pub fn new(
        text: &str,
        font: Font,
        size: f32,
        color: Color,
        rect: Bounds,
        metrics: &impl TextMetrics,
    ) -> Self {
        let line_height = metrics.line_height(font, size);
        // A NaN or negative quotient (no room, or no line height) is 0. A
        // box at least as tall as the text's size, short of a whole line
        // with its leading, still holds the first line's glyphs.
        let whole = (rect.height() / line_height).floor() as usize;
        let visible = if rect.height() >= size {
            whole.max(1)
        } else {
            whole
        };
        Self {
            lines: wrap(metrics, font, size, text, rect.width()),
            font,
            size,
            color,
            rect,
            line_height,
            visible,
            first: 0,
            pending: 0.0,
        }
    }

    /// `text` in this one's font, size, colour and box, wrapped by
    /// `metrics` and shown from the first line.
    #[must_use]
    pub fn rewrapped(&self, text: &str, metrics: &impl TextMetrics) -> Self {
        Self::new(text, self.font, self.size, self.color, self.rect, metrics)
    }

    /// Every wrapped line.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The first line shown.
    #[must_use]
    pub fn first(&self) -> usize {
        self.first
    }

    /// How many whole lines fit the box.
    #[must_use]
    pub fn visible(&self) -> usize {
        self.visible
    }

    /// The box.
    #[must_use]
    pub fn rect(&self) -> Bounds {
        self.rect
    }

    /// Whether there is more text than fits the box.
    #[must_use]
    pub fn overflows(&self) -> bool {
        self.lines.len() > self.visible
    }

    /// Moves the first line shown by `lines` (down for positive), no
    /// further than the start, or than where the last line fills the
    /// bottom of the box.
    pub fn scroll_by(&mut self, lines: isize) {
        let last = self.lines.len().saturating_sub(self.visible);
        self.first = self.first.saturating_add_signed(lines).min(last);
    }

    /// Scrolls one line on each Up or Down press, key repeats included,
    /// and by a wheel or trackpad scroll over the box, a whole line for
    /// each line height it builds up to. Returns whether it used `input`:
    /// a scroll over the box is used even where it can't move.
    pub fn input(&mut self, input: &Input) -> bool {
        let key = match *input {
            Input::Key {
                key, pressed: true, ..
            } => key,
            Input::Scroll { delta, at } => return self.wheel(delta, at),
            _ => return false,
        };
        match key {
            Key::Up => self.scroll_by(-1),
            Key::Down => self.scroll_by(1),
            _ => return false,
        }
        true
    }

    /// Scrolls by `delta` when `at` is over the box, keeping what falls
    /// short of a whole line for the next scroll, unless it points past the
    /// end the first line already sits at. That drops what a move cut
    /// short at an end had left over, so a scroll back moves at once.
    fn wheel(&mut self, delta: ScrollDelta, at: Point) -> bool {
        if !self.rect.contains(at) {
            return false;
        }
        self.pending += match delta {
            ScrollDelta::Lines(lines) => lines * self.line_height,
            ScrollDelta::Units(units) => units,
        };
        let whole = (self.pending / self.line_height).trunc();
        self.pending -= whole * self.line_height;
        self.scroll_by(whole as isize);
        // The leftover has the sign of the move, so a move cut short at an
        // end leaves it pointing past that end too.
        let last = self.lines.len().saturating_sub(self.visible);
        if (self.pending < 0.0 && self.first == 0) || (self.pending > 0.0 && self.first == last) {
            self.pending = 0.0;
        }
        true
    }

    /// Draws the lines shown, one text run each, a line height apart from
    /// the box's top-left corner. Blank lines draw nothing.
    pub fn draw(&self, list: &mut DrawList) {
        let shown = self.lines.iter().skip(self.first).take(self.visible);
        for (row, line) in shown.enumerate() {
            if line.is_empty() {
                continue;
            }
            let origin = Point::new(
                self.rect.min.x,
                self.rect.min.y + row as f32 * self.line_height,
            );
            list.text_in(self.font, line, origin, self.size, None, self.color);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    const GREEN: Color = Color::rgba(0, 255, 0, 255);

    /// Geneva 10 (5 wide, 12 high a line) in a 20 x `height` box at
    /// (100, 50): four characters a line.
    fn scroll(text: &str, height: f32) -> ScrollText {
        let rect = Bounds::at(Point::new(100.0, 50.0), 20.0, height);
        ScrollText::new(text, Font::Geneva, 10.0, GREEN, rect, &MonoMetrics)
    }

    /// Ten lines, "l0" to "l9", in a box three lines high.
    fn ten_lines() -> ScrollText {
        let text: Vec<String> = (0..10).map(|n| format!("l{n}")).collect();
        scroll(&text.join("\r"), 36.0)
    }

    fn texts(text: &ScrollText) -> Vec<(String, Point)> {
        let mut list = DrawList::new();
        text.draw(&mut list);
        list.iter()
            .map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    font,
                    size,
                    wrap_width,
                    color,
                } => {
                    assert_eq!(
                        (*font, *size, *wrap_width, *color),
                        (Font::Geneva, 10.0, None, GREEN)
                    );
                    (text.clone(), *origin)
                }
                other => panic!("not text: {other:?}"),
            })
            .collect()
    }

    fn up(repeat: bool) -> Input {
        Input::Key {
            key: Key::Up,
            pressed: true,
            repeat,
        }
    }

    fn down(repeat: bool) -> Input {
        Input::Key {
            key: Key::Down,
            pressed: true,
            repeat,
        }
    }

    /// A point inside [`ten_lines`]'s box.
    const INSIDE: Point = Point::new(110.0, 60.0);

    fn wheel(delta: ScrollDelta) -> Input {
        Input::Scroll { delta, at: INSIDE }
    }

    fn lines(n: f32) -> Input {
        wheel(ScrollDelta::Lines(n))
    }

    fn units(n: f32) -> Input {
        wheel(ScrollDelta::Units(n))
    }

    #[test]
    fn wheel_lines_scroll_and_clamp_like_down_and_up() {
        let mut text = ten_lines();
        assert!(text.input(&lines(1.0)));
        assert_eq!(text.first(), 1);
        assert!(text.input(&lines(3.0)));
        assert_eq!(text.first(), 4);
        assert!(text.input(&lines(100.0)), "used even at the end");
        assert_eq!(text.first(), 7);
        assert!(text.input(&lines(-2.0)));
        assert_eq!(text.first(), 5);
        assert!(text.input(&lines(-100.0)));
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn a_wheel_outside_the_box_is_not_used() {
        let mut text = ten_lines();
        for at in [Point::new(120.5, 60.0), Point::new(110.0, 86.5)] {
            let input = Input::Scroll {
                delta: ScrollDelta::Lines(1.0),
                at,
            };
            assert!(!text.input(&input), "{at:?}");
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn a_wheel_over_text_shorter_than_the_box_is_used_but_never_moves() {
        let mut text = scroll("ab\rcd", 36.0);
        assert!(text.input(&lines(1.0)));
        assert_eq!(text.first(), 0);
        for _ in 0..3 {
            assert!(text.input(&units(5.0)));
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn sub_line_units_build_up_into_lines_from_the_top() {
        let mut text = ten_lines();
        text.input(&units(5.0));
        text.input(&units(5.0));
        assert_eq!(text.first(), 0);
        text.input(&units(5.0));
        assert_eq!(text.first(), 1, "15 units are a line, 3 over");
        text.input(&units(9.0));
        assert_eq!(text.first(), 2, "3 kept and 9 more are a line");
    }

    #[test]
    fn half_lines_build_up_into_a_line() {
        let mut text = ten_lines();
        text.input(&lines(0.5));
        assert_eq!(text.first(), 0);
        text.input(&lines(0.5));
        assert_eq!(text.first(), 1);
    }

    #[test]
    fn negative_units_build_up_from_the_end() {
        let mut text = ten_lines();
        text.scroll_by(7);
        for _ in 0..3 {
            text.input(&units(-5.0));
        }
        assert_eq!(text.first(), 6);
    }

    #[test]
    fn a_leftover_pointing_past_the_top_is_dropped() {
        let mut text = ten_lines();
        text.input(&units(-5.0));
        text.input(&units(12.0));
        assert_eq!(text.first(), 1, "exactly one line, not 7 units");
    }

    #[test]
    fn a_move_cut_short_at_the_end_drops_its_leftover() {
        let mut text = ten_lines();
        text.input(&units(1205.0));
        assert_eq!(text.first(), 7);
        text.input(&units(-12.0));
        assert_eq!(text.first(), 6, "the 5 over swallow no line");
    }

    #[test]
    fn a_move_that_reaches_the_end_drops_a_leftover_past_it() {
        let mut text = ten_lines();
        text.scroll_by(6);
        text.input(&units(17.0));
        assert_eq!(text.first(), 7);
        text.input(&units(-12.0));
        assert_eq!(text.first(), 6, "the 5 over swallow no line");
    }

    #[test]
    fn rewrapped_text_starts_with_nothing_left_over() {
        let mut text = ten_lines();
        text.input(&units(11.0));
        let mut new = text.rewrapped(
            &(0..10)
                .map(|n| format!("m{n}"))
                .collect::<Vec<_>>()
                .join("\r"),
            &MonoMetrics,
        );
        new.input(&units(11.0));
        assert_eq!(new.first(), 0);
        new.input(&units(1.0));
        assert_eq!(new.first(), 1);
    }

    #[test]
    fn no_line_height_scrolls_nothing() {
        let rect = Bounds::at(Point::new(100.0, 50.0), 20.0, 36.0);
        let text: Vec<String> = (0..10).map(|n| format!("l{n}")).collect();
        let mut text = ScrollText::new(
            &text.join("\r"),
            Font::Geneva,
            0.0,
            GREEN,
            rect,
            &MonoMetrics,
        );
        text.input(&lines(3.0));
        text.input(&units(30.0));
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn text_wraps_to_the_box_width() {
        let text = scroll("abcd efgh ij", 100.0);
        assert_eq!(text.lines(), ["abcd", "efgh", "ij"]);
        assert_eq!(
            text.rect(),
            Bounds::at(Point::new(100.0, 50.0), 20.0, 100.0)
        );
    }

    #[test]
    fn whole_lines_that_fit_the_height_are_visible() {
        assert_eq!(scroll("a", 36.0).visible(), 3);
        assert_eq!(scroll("a", 47.9).visible(), 3);
        assert_eq!(scroll("a", 48.0).visible(), 4);
        assert_eq!(scroll("a", 9.9).visible(), 0);
        assert_eq!(scroll("a", -5.0).visible(), 0);
    }

    #[test]
    fn a_box_as_tall_as_the_text_but_short_of_a_line_shows_its_first_line() {
        // A line's glyphs are the size high, its leading below them: the
        // mission computer's offer name, Times 18 (21.6 a line) in a box
        // 21 high, shows its first line as `TETextBox` does.
        for height in [10.0, 11.9] {
            let text = scroll("a b", height);
            assert_eq!(text.visible(), 1, "{height}");
            assert_eq!(texts(&text).len(), 1, "{height}");
        }
    }

    #[test]
    fn only_visible_lines_draw_a_line_height_apart() {
        assert_eq!(
            texts(&ten_lines()),
            [
                ("l0".to_owned(), Point::new(100.0, 50.0)),
                ("l1".to_owned(), Point::new(100.0, 62.0)),
                ("l2".to_owned(), Point::new(100.0, 74.0)),
            ]
        );
    }

    #[test]
    fn down_scrolls_until_the_last_line_fills_the_bottom() {
        let mut text = ten_lines();
        assert!(text.overflows());
        for expected in 1..=7 {
            assert!(text.input(&down(expected > 3)));
            assert_eq!(text.first(), expected);
        }
        assert!(text.input(&down(true)), "used even at the end");
        assert_eq!(text.first(), 7);
        let shown: Vec<String> = texts(&text).into_iter().map(|(text, _)| text).collect();
        assert_eq!(shown, ["l7", "l8", "l9"]);
        assert_eq!(texts(&text)[0].1, Point::new(100.0, 50.0));
    }

    #[test]
    fn up_scrolls_back_to_the_start() {
        let mut text = ten_lines();
        text.scroll_by(5);
        assert!(text.input(&up(false)));
        assert_eq!(text.first(), 4);
        for _ in 0..10 {
            text.input(&up(true));
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn scrolling_by_many_lines_clamps_at_both_ends() {
        let mut text = ten_lines();
        text.scroll_by(100);
        assert_eq!(text.first(), 7);
        text.scroll_by(-3);
        assert_eq!(text.first(), 4);
        text.scroll_by(-100);
        assert_eq!(text.first(), 0);
        text.scroll_by(isize::MAX);
        assert_eq!(text.first(), 7);
        text.scroll_by(isize::MIN);
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn text_shorter_than_the_box_never_scrolls() {
        let mut text = scroll("ab\rcd", 36.0);
        assert!(!text.overflows());
        text.input(&down(false));
        text.scroll_by(3);
        assert_eq!(text.first(), 0);
        assert_eq!(texts(&text).len(), 2);
        let exact = scroll("a\rb\rc", 36.0);
        assert!(!exact.overflows(), "three lines fit three rows");
    }

    #[test]
    fn other_keys_and_releases_are_not_used() {
        let mut text = ten_lines();
        let release = Input::Key {
            key: Key::Down,
            pressed: false,
            repeat: false,
        };
        let enter = Input::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        };
        for input in [release, enter, Input::PointerMoved(Point::new(110.0, 60.0))] {
            assert!(!text.input(&input), "{input:?}");
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn empty_text_draws_nothing() {
        let text = scroll("", 36.0);
        assert!(text.lines().is_empty());
        assert!(texts(&text).is_empty());
        assert!(!text.overflows());
    }

    #[test]
    fn blank_lines_draw_nothing_but_keep_their_row() {
        assert_eq!(
            texts(&scroll("a\r\rb", 36.0)),
            [
                ("a".to_owned(), Point::new(100.0, 50.0)),
                ("b".to_owned(), Point::new(100.0, 74.0)),
            ]
        );
    }

    #[test]
    fn rewrapped_text_keeps_the_box_and_style_and_starts_at_the_top() {
        let mut text = ten_lines();
        text.scroll_by(4);
        let new = text.rewrapped("abcd efgh", &MonoMetrics);
        assert_eq!(new.lines(), ["abcd", "efgh"]);
        assert_eq!((new.first(), new.visible()), (0, 3));
        assert_eq!(new.rect(), text.rect());
        assert_eq!(
            texts(&new),
            [
                ("abcd".to_owned(), Point::new(100.0, 50.0)),
                ("efgh".to_owned(), Point::new(100.0, 62.0)),
            ]
        );
    }
}
