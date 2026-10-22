//! A list of one-line rows in a box, one of them selected: the mission
//! computer's and the mission info screen's lists.

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key, MouseButton};
use crate::text::TextMetrics;

/// The rows' size, in Geneva, as the description's.
pub const ROW_SIZE: f32 = 10.0;
/// The rows' colour.
pub const ROW_COLOR: Color = Color::WHITE;
/// The selected row's highlight.
pub const SELECTED: Color = Color::rgba(48, 64, 112, 255);

/// Rows of text a line high each, from the top of a box, as many shown
/// as fit it whole, from a first row that follows the selection. A click
/// on a row selects it, and Up and Down move the selection a row,
/// stopping at the ends. Nothing is drawn outside the box.
#[derive(Clone, Debug, PartialEq)]
pub struct RowList {
    rows: Vec<String>,
    rect: Bounds,
    line_height: f32,
    visible: usize,
    first: usize,
    selected: Option<usize>,
}

impl RowList {
    /// `rows` in `rect`, measured by `metrics`, the first selected (none
    /// when there are none).
    #[must_use]
    pub fn new(rows: Vec<String>, rect: Bounds, metrics: &impl TextMetrics) -> Self {
        let line_height = metrics.line_height(Font::Geneva, ROW_SIZE);
        // A NaN or negative quotient (no room, or no line height) is 0.
        let visible = (rect.height() / line_height).floor() as usize;
        let selected = (!rows.is_empty()).then_some(0);
        Self {
            rows,
            rect,
            line_height,
            visible,
            first: 0,
            selected,
        }
    }

    /// Every row.
    #[must_use]
    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    /// The selected row's index, if any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// The first row shown.
    #[must_use]
    pub fn first(&self) -> usize {
        self.first
    }

    /// How many rows fit the box whole.
    #[must_use]
    pub fn visible(&self) -> usize {
        self.visible
    }

    /// Where row `index` is on the screen, if it is shown.
    #[must_use]
    pub fn row_bounds(&self, index: usize) -> Option<Bounds> {
        if index >= self.rows.len() || index < self.first || index >= self.first + self.visible {
            return None;
        }
        let top = self.rect.min.y + (index - self.first) as f32 * self.line_height;
        Some(Bounds::at(
            Point::new(self.rect.min.x, top),
            self.rect.width(),
            self.line_height,
        ))
    }

    /// Selects row `index`, held to the last, and scrolls to show it;
    /// nothing with no rows.
    pub fn select(&mut self, index: usize) {
        let Some(last) = self.rows.len().checked_sub(1) else {
            return;
        };
        let index = index.min(last);
        self.selected = Some(index);
        if index < self.first {
            self.first = index;
        } else if index >= self.first + self.visible {
            self.first = (index + 1).saturating_sub(self.visible);
        }
    }

    /// A press on a row shown selects it; Up and Down, repeats included,
    /// move the selection. Returns whether the selection changed.
    pub fn input(&mut self, input: &Input) -> bool {
        let before = self.selected;
        match *input {
            Input::Key {
                key: Key::Up,
                pressed: true,
                ..
            } => {
                if let Some(index) = self.selected {
                    self.select(index.saturating_sub(1));
                }
            }
            Input::Key {
                key: Key::Down,
                pressed: true,
                ..
            } => {
                if let Some(index) = self.selected {
                    self.select(index + 1);
                }
            }
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at,
            } => {
                let hit = (self.first..self.rows.len())
                    .find(|&index| self.row_bounds(index).is_some_and(|row| row.contains(at)));
                if let Some(index) = hit {
                    self.select(index);
                }
            }
            _ => {}
        }
        self.selected != before
    }

    /// Draws the selected row's highlight, then each row shown.
    pub fn draw(&self, list: &mut DrawList) {
        if let Some(row) = self.selected.and_then(|index| self.row_bounds(index)) {
            fill_rect(list, row, SELECTED);
        }
        for (index, text) in self.rows.iter().enumerate() {
            if let Some(row) = self.row_bounds(index) {
                list.text_in(Font::Geneva, text, row.min, ROW_SIZE, None, ROW_COLOR);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    /// A 100 x 40 box at (10, 20): three rows of 12 fit.
    const BOX: Bounds = Bounds {
        min: Point::new(10.0, 20.0),
        max: Point::new(110.0, 60.0),
    };

    fn list(rows: usize) -> RowList {
        RowList::new(
            (0..rows).map(|n| format!("Row {n}")).collect(),
            BOX,
            &MonoMetrics,
        )
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn press(at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        }
    }

    fn drawn(rows: &RowList) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        rows.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(rows: &RowList) -> Vec<(String, Point)> {
        drawn(rows)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } => Some((text, origin)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_rows_lay_out_a_line_apart_from_the_boxs_top_the_first_selected() {
        let rows = list(2);
        assert_eq!(rows.visible(), 3);
        assert_eq!(rows.selected(), Some(0));
        assert_eq!(rows.rows(), ["Row 0", "Row 1"]);
        assert_eq!(
            rows.row_bounds(1),
            Some(Bounds::at(Point::new(10.0, 32.0), 100.0, 12.0))
        );
        assert_eq!(rows.row_bounds(2), None);
        assert_eq!(
            texts(&rows),
            [
                ("Row 0".to_owned(), Point::new(10.0, 20.0)),
                ("Row 1".to_owned(), Point::new(10.0, 32.0)),
            ]
        );
    }

    #[test]
    fn a_click_on_a_row_selects_it_and_elsewhere_nothing() {
        let mut rows = list(3);
        assert!(rows.input(&press(Point::new(50.0, 50.0))));
        assert_eq!(rows.selected(), Some(2));
        assert!(!rows.input(&press(Point::new(50.0, 50.0))), "already");
        assert!(!rows.input(&press(Point::new(5.0, 25.0))), "outside");
        assert!(!rows.input(&press(Point::new(50.0, 70.0))), "below");
        let mut few = list(1);
        assert!(!few.input(&press(Point::new(50.0, 35.0))), "no row there");
        assert_eq!(few.selected(), Some(0));
        let release = Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: Point::new(50.0, 22.0),
        };
        assert!(!rows.input(&release));
        assert_eq!(rows.selected(), Some(2));
    }

    #[test]
    fn up_and_down_move_the_selection_held_at_the_ends() {
        let mut rows = list(3);
        assert!(!rows.input(&key(Key::Up, false)), "at the top");
        assert!(rows.input(&key(Key::Down, true)), "repeats move");
        assert_eq!(rows.selected(), Some(1));
        assert!(rows.input(&key(Key::Down, false)));
        assert!(!rows.input(&key(Key::Down, false)), "at the bottom");
        assert_eq!(rows.selected(), Some(2));
        assert!(rows.input(&key(Key::Up, false)));
        assert_eq!(rows.selected(), Some(1));
        assert!(!rows.input(&key(Key::Enter, false)));
        let released = Input::Key {
            key: Key::Down,
            pressed: false,
            repeat: false,
        };
        assert!(!rows.input(&released));
        assert_eq!(rows.selected(), Some(1));
    }

    #[test]
    fn the_list_scrolls_to_keep_the_selection_shown() {
        let mut rows = list(5);
        for _ in 0..3 {
            rows.input(&key(Key::Down, false));
        }
        assert_eq!(rows.selected(), Some(3));
        assert_eq!(rows.first(), 1);
        assert_eq!(rows.row_bounds(0), None);
        assert_eq!(
            rows.row_bounds(3),
            Some(Bounds::at(Point::new(10.0, 44.0), 100.0, 12.0))
        );
        assert_eq!(
            texts(&rows).first(),
            Some(&("Row 1".to_owned(), Point::new(10.0, 20.0)))
        );
        rows.select(0);
        assert_eq!(rows.first(), 0);
        rows.select(99);
        assert_eq!(rows.selected(), Some(4), "held to the last");
        assert_eq!(rows.first(), 2);
    }

    #[test]
    fn stepping_down_a_long_list_keeps_scrolling_a_row_at_a_time() {
        let mut rows = list(8);
        for _ in 0..5 {
            rows.input(&key(Key::Down, false));
        }
        assert_eq!(rows.selected(), Some(5));
        assert_eq!(rows.first(), 3);
        assert!(rows.row_bounds(5).is_some(), "shown");
        rows.select(3);
        assert_eq!(rows.first(), 3, "shown already");
    }

    #[test]
    fn it_draws_the_selections_highlight_under_its_row() {
        let mut rows = list(2);
        rows.select(1);
        let commands = drawn(&rows);
        let mut highlight = DrawList::new();
        fill_rect(
            &mut highlight,
            Bounds::at(Point::new(10.0, 32.0), 100.0, 12.0),
            SELECTED,
        );
        assert_eq!(commands[0], highlight.iter().next().cloned().expect("one"));
        assert!(matches!(
            &commands[1],
            DrawCommand::Text { text, color, size, .. }
                if text == "Row 0" && *color == ROW_COLOR && *size == ROW_SIZE
        ));
        assert_eq!(commands.len(), 3);
    }

    #[test]
    fn an_empty_list_selects_nothing() {
        let mut rows = list(0);
        assert_eq!(rows.selected(), None);
        assert!(!rows.input(&key(Key::Down, false)));
        rows.select(0);
        assert_eq!(rows.selected(), None);
        assert!(drawn(&rows).is_empty());
    }
}
