//! A static text box, as the Toolbox's `TETextBox` draws one.
//!
//! `TETextBox` wraps its text to the box's width and draws every line
//! whose top is inside the box, clipped to the box: a line that straddles
//! the bottom edge shows its top part, and the text never scrolls. The
//! news dialog draws its commercial and story this way
//! (`_NewsDialogUpdate` @0x47d7b and @0x47e2c in the `EV Nova`
//! executable). [`ScrollText`](super::scroll_text::ScrollText) differs:
//! it shows only whole lines and scrolls.

use crate::color::Color;
use crate::draw::{DrawCommand, DrawList};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::text::{TextMetrics, wrap};

/// Text wrapped to a box's width once, when it is made, and drawn from
/// the box's top: whole lines plainly, and a last line that straddles the
/// bottom edge clipped to the box.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBox {
    lines: Vec<String>,
    font: Font,
    size: f32,
    color: Color,
    rect: Bounds,
    line_height: f32,
}

impl TextBox {
    /// `text` in `font` at `size` and `color`, wrapped by `metrics` to the
    /// width of `rect`.
    #[must_use]
    pub fn new(
        text: &str,
        font: Font,
        size: f32,
        color: Color,
        rect: Bounds,
        metrics: &impl TextMetrics,
    ) -> Self {
        Self {
            lines: wrap(metrics, font, size, text, rect.width()),
            font,
            size,
            color,
            rect,
            line_height: metrics.line_height(font, size),
        }
    }

    /// Every wrapped line.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The box.
    #[must_use]
    pub fn rect(&self) -> Bounds {
        self.rect
    }

    /// Draws each line whose top is inside the box, one text run each, a
    /// line height apart from the box's top-left corner. A line whose
    /// bottom is past the box's bottom is drawn clipped to the box. Blank
    /// lines draw nothing but keep their row.
    pub fn draw(&self, list: &mut DrawList) {
        let bottom = self.rect.max.y;
        for (row, line) in self.lines.iter().enumerate() {
            let top = self.rect.min.y + row as f32 * self.line_height;
            // A NaN bottom compares as no ordering: such a box shows nothing.
            if top.partial_cmp(&bottom) != Some(std::cmp::Ordering::Less) {
                break;
            }
            if line.is_empty() {
                continue;
            }
            let run = DrawCommand::Text {
                text: line.clone(),
                font: self.font,
                origin: Point::new(self.rect.min.x, top),
                size: self.size,
                wrap_width: None,
                color: self.color,
            };
            if top + self.line_height > bottom {
                list.clipped(self.rect, run);
            } else {
                list.push(run);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::text::fixture::MonoMetrics;

    const GREEN: Color = Color::rgba(0, 255, 0, 255);
    const AT: Point = Point::new(100.0, 50.0);

    /// Geneva 10 (5 wide, 12 high a line) in a 20 x `height` box at
    /// (100, 50): four characters a line.
    fn boxed(text: &str, height: f32) -> TextBox {
        let rect = Bounds::at(AT, 20.0, height);
        TextBox::new(text, Font::Geneva, 10.0, GREEN, rect, &MonoMetrics)
    }

    /// Lines "l0" to "l{n-1}".
    fn numbered(n: usize) -> String {
        let text: Vec<String> = (0..n).map(|n| format!("l{n}")).collect();
        text.join("\r")
    }

    fn run(text: &str, row: f32) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: Point::new(AT.x, AT.y + row * 12.0),
            size: 10.0,
            wrap_width: None,
            color: GREEN,
        }
    }

    fn clipped(text: &TextBox, command: DrawCommand) -> DrawCommand {
        DrawCommand::Clipped {
            clip: text.rect(),
            command: Box::new(command),
        }
    }

    fn drawn(text: &TextBox) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        text.draw(&mut list);
        list.iter().cloned().collect()
    }

    #[test]
    fn text_wraps_to_the_box_width() {
        let text = boxed("abcd efgh ij", 100.0);
        assert_eq!(text.lines(), ["abcd", "efgh", "ij"]);
        assert_eq!(text.rect(), Bounds::at(AT, 20.0, 100.0));
    }

    #[test]
    fn whole_lines_draw_a_line_height_apart_as_plain_text() {
        let text = boxed(&numbered(3), 36.0);
        assert_eq!(
            drawn(&text),
            [run("l0", 0.0), run("l1", 1.0), run("l2", 2.0)]
        );
    }

    #[test]
    fn a_line_straddling_the_bottom_draws_clipped_to_the_box() {
        let text = boxed(&numbered(6), 40.0);
        assert_eq!(
            drawn(&text),
            [
                run("l0", 0.0),
                run("l1", 1.0),
                run("l2", 2.0),
                clipped(&text, run("l3", 3.0)),
            ]
        );
    }

    #[test]
    fn a_line_starting_on_the_bottom_edge_is_not_drawn() {
        let text = boxed(&numbered(6), 36.0);
        assert_eq!(
            drawn(&text),
            [run("l0", 0.0), run("l1", 1.0), run("l2", 2.0)]
        );
    }

    #[test]
    fn a_line_ending_on_the_bottom_edge_is_drawn_plainly() {
        let text = boxed(&numbered(2), 24.0);
        assert_eq!(drawn(&text), [run("l0", 0.0), run("l1", 1.0)]);
    }

    #[test]
    fn a_box_shorter_than_one_line_draws_its_first_line_clipped() {
        let text = boxed(&numbered(3), 8.0);
        assert_eq!(drawn(&text), [clipped(&text, run("l0", 0.0))]);
    }

    #[test]
    fn a_box_with_no_height_draws_nothing() {
        for height in [0.0, -5.0, f32::NAN] {
            let rect = Bounds {
                min: AT,
                max: Point::new(AT.x + 20.0, AT.y + height),
            };
            let text = TextBox::new(&numbered(3), Font::Geneva, 10.0, GREEN, rect, &MonoMetrics);
            assert_eq!(drawn(&text), [], "height {height}");
        }
    }

    #[test]
    fn blank_lines_draw_nothing_but_keep_their_row() {
        let text = boxed("l0\r\rl2\r\rl4", 40.0);
        assert_eq!(text.lines(), ["l0", "", "l2", "", "l4"]);
        assert_eq!(drawn(&text), [run("l0", 0.0), run("l2", 2.0)]);
        let partial = boxed("l0\r\rl2\r\rl4", 50.0);
        assert_eq!(
            drawn(&partial),
            [
                run("l0", 0.0),
                run("l2", 2.0),
                clipped(&partial, run("l4", 4.0)),
            ]
        );
    }

    #[test]
    fn empty_text_draws_nothing() {
        assert_eq!(drawn(&boxed("", 40.0)), []);
    }
}
