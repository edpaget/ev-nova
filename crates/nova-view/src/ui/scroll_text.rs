//! Text wrapped into a box and scrolled through it a whole line at a time.

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::text::{TextMetrics, wrap};

/// Text wrapped to a box's width once, when it is made, and shown as many
/// whole lines as fit the box's height, from a first line the arrow keys
/// move. Nothing is drawn outside the box, so the renderer needs no clip.
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
        // A NaN or negative quotient (no room, or no line height) is 0.
        let visible = (rect.height() / line_height).floor() as usize;
        Self {
            lines: wrap(metrics, font, size, text, rect.width()),
            font,
            size,
            color,
            rect,
            line_height,
            visible,
            first: 0,
        }
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

    /// Scrolls one line on each Up or Down press, key repeats included.
    /// Returns whether it used `input`.
    pub fn input(&mut self, input: &Input) -> bool {
        let Input::Key {
            key, pressed: true, ..
        } = *input
        else {
            return false;
        };
        match key {
            Key::Up => self.scroll_by(-1),
            Key::Down => self.scroll_by(1),
            _ => return false,
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
        assert_eq!(scroll("a", 11.9).visible(), 0);
        assert_eq!(scroll("a", -5.0).visible(), 0);
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
}
