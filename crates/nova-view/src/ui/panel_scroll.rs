//! A page of placed lines scrolled a row at a time inside a panel: the
//! new game's story log ([`StoryLogScreen`](super::story_log::StoryLogScreen))
//! and conversation panel
//! ([`ConversationScreen`](crate::spaceport::ConversationScreen)) share it.
//! Neither is a stock reading, so it reads no rulebook.
//!
//! **The page.** The host lays its page out as one column, every line at
//! its own origin and colour, with its first line at the column's top;
//! the column is the panel inset on every side. [`PanelScroll`] hosts the
//! stock scrolling text in its placed-rows mode
//! ([`ScrollText::with_rows`]) for what is shown and how it moves, and
//! draws the lines shown itself, so a page that fits draws exactly the
//! lines it was given, at their origins. Only whole lines inside the
//! column are shown: a line shows only while its top and its line height
//! are inside, from any first line, so a page scrolled shows each window
//! of lines that fits whatever its gaps.
//!
//! **Moving.** While the page overflows:
//!
//! - **Keys.** The shared UI context's Up and Down, through the bindings
//!   given ([`PanelScroll::with_bindings`]): one row a press. A repeat is
//!   ignored, as the new game's dialog keys act on the press only; the
//!   stock dialogs' Up and Down, which take repeats, are their own.
//! - **Wheel.** A wheel or trackpad over the column, a row for each line
//!   height it builds up to. Whether a wheel reaches the panel at all is
//!   the router's (`RuleKey::DialogTextWheel`).
//! - **Pointer.** An up and a down arrow ([`PanelScroll::arrow_bounds`])
//!   in the panel's right inset, outside the column, so no line moves to
//!   make room. Each is the stock pen glyph ([`ScrollArrow::label`]) with
//!   no button face, as the new game has no button art: bright while its
//!   way can move, [`Color::DIM`] when it can't. A press on one moves a
//!   row its way, or nothing at an end, and is used either way. There is
//!   no held repeat, in keeping with the keys.
//!
//! A page that fits shows no arrows and uses no input. Nothing here plays
//! a sound: the stock arrows and `_ScrollDialogText` play none.

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::Input;
use crate::keymap::SharedBindings;
use crate::text::TextMetrics;

use super::button_glyph;
use super::scroll_arrows::ScrollArrow;
use super::scroll_text::{ScrollText, TextRow, TextRun};

/// A line of a page, placed on the panel.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLine {
    /// Its text.
    pub text: String,
    /// Its top-left corner, as it is drawn unscrolled.
    pub origin: Point,
    /// Its colour.
    pub color: Color,
}

/// A page of [`PageLine`]s scrolled inside a panel (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct PanelScroll {
    panel: Bounds,
    inset: f32,
    font: Font,
    size: f32,
    line_height: f32,
    lines: Vec<PageLine>,
    text: ScrollText,
}

impl PanelScroll {
    /// `lines` in `font` at `size`, measured by `metrics`, on `panel`
    /// inset by `inset`, shown from the first.
    #[must_use]
    pub fn new(
        panel: Bounds,
        inset: f32,
        font: Font,
        size: f32,
        lines: Vec<PageLine>,
        metrics: &impl TextMetrics,
    ) -> Self {
        let column = column_of(panel, inset);
        // The placed rows count a row in while its baseline is inside the
        // box. A row's baseline is its top plus `size`, so a box taller
        // than the column by `size` less a line height counts it in
        // exactly while its line height ends inside the column.
        let line_height = metrics.line_height(font, size);
        let rows = Bounds {
            min: column.min,
            max: Point::new(column.max.x, column.max.y + size - line_height),
        };
        let blank = ScrollText::new("", font, size, Color::WHITE, rows, metrics);
        let text = blank.with_rows(placed_rows(&lines, column, size));
        Self {
            panel,
            inset,
            font,
            size,
            line_height,
            lines,
            text,
        }
    }

    /// The page reading its keys through `bindings`.
    #[must_use]
    pub fn with_bindings(self, bindings: SharedBindings) -> Self {
        Self {
            text: self.text.with_bindings(bindings),
            ..self
        }
    }

    /// `lines` on this one's panel, in its font and size and reading its
    /// bindings, shown from the first.
    #[must_use]
    pub fn relaid(&self, lines: Vec<PageLine>) -> Self {
        let column = column_of(self.panel, self.inset);
        Self {
            text: self.text.with_rows(placed_rows(&lines, column, self.size)),
            lines,
            ..self.clone()
        }
    }

    /// Whether the page holds more than the column shows.
    #[must_use]
    pub fn overflows(&self) -> bool {
        self.text.overflows()
    }

    /// The index of the first line shown.
    #[must_use]
    pub fn first(&self) -> usize {
        self.text.first()
    }

    /// How far up the lines shown are drawn from their origins: the first
    /// shown is drawn where the page's first is.
    #[must_use]
    pub fn shift(&self) -> f32 {
        let top = |at: usize| self.lines.get(at).map_or(0.0, |line| line.origin.y);
        top(self.first()) - top(0)
    }

    /// The lines shown, each at the origin it is drawn at.
    #[must_use]
    pub fn shown(&self) -> Vec<PageLine> {
        let shift = self.shift();
        self.lines
            .iter()
            .skip(self.first())
            .take(self.text.visible())
            .map(|line| PageLine {
                origin: Point::new(line.origin.x, line.origin.y - shift),
                ..line.clone()
            })
            .collect()
    }

    /// The area the lines shown are drawn over, across the column: from
    /// the first one's top to the last one's foot, a line height below
    /// its top. None with nothing shown.
    #[must_use]
    pub fn shown_span(&self) -> Option<Bounds> {
        let shown = self.shown();
        let (first, last) = (shown.first()?, shown.last()?);
        let column = column_of(self.panel, self.inset);
        Some(Bounds {
            min: Point::new(column.min.x, first.origin.y),
            max: Point::new(column.max.x, last.origin.y + self.line_height),
        })
    }

    /// Where `arrow` is, while the page overflows: a square of the inset
    /// in the panel's right inset, the up arrow at the column's top and
    /// the down arrow at its foot.
    #[must_use]
    pub fn arrow_bounds(&self, arrow: ScrollArrow) -> Option<Bounds> {
        if !self.overflows() {
            return None;
        }
        let column = column_of(self.panel, self.inset);
        let top = match arrow {
            ScrollArrow::Up => column.min.y,
            ScrollArrow::Down => column.max.y - self.inset,
        };
        Some(Bounds::at(
            Point::new(column.max.x, top),
            self.inset,
            self.inset,
        ))
    }

    /// Moves the page for `input` (see the module docs): whether it used
    /// it. A page that fits uses nothing.
    pub fn input(&mut self, input: &Input) -> bool {
        match *input {
            // A repeat, or a release, of a key or a button.
            Input::Key { repeat: true, .. } | Input::PointerButton { pressed: false, .. } => false,
            Input::PointerButton { at, .. } => {
                let Some(arrow) = [ScrollArrow::Up, ScrollArrow::Down]
                    .into_iter()
                    .find(|&arrow| {
                        self.arrow_bounds(arrow)
                            .is_some_and(|area| area.contains(at))
                    })
                else {
                    return false;
                };
                if arrow.can_scroll(&self.text) {
                    self.text.scroll_by(match arrow {
                        ScrollArrow::Up => -1,
                        ScrollArrow::Down => 1,
                    });
                }
                true
            }
            _ => self.text.input(input) && self.overflows(),
        }
    }

    /// Draws the lines shown, then the arrows while the page overflows.
    pub fn draw(&self, list: &mut DrawList) {
        for line in self.shown() {
            list.text_in(
                self.font,
                line.text,
                line.origin,
                self.size,
                None,
                line.color,
            );
        }
        for arrow in [ScrollArrow::Up, ScrollArrow::Down] {
            let Some(area) = self.arrow_bounds(arrow) else {
                continue;
            };
            let color = if arrow.can_scroll(&self.text) {
                Color::WHITE
            } else {
                Color::DIM
            };
            let strokes = button_glyph::strokes(arrow.label(), area.width(), area.height());
            for stroke in strokes.into_iter().flatten() {
                fill_rect(list, stroke.offset(area.min), color);
            }
        }
    }
}

/// `panel` inset by `inset` on every side.
fn column_of(panel: Bounds, inset: f32) -> Bounds {
    Bounds {
        min: Point::new(panel.min.x + inset, panel.min.y + inset),
        max: Point::new(panel.max.x - inset, panel.max.y - inset),
    }
}

/// `lines` as the placed rows of a box at `column`'s top-left, each
/// baseline its top plus `size` below the box's top.
fn placed_rows(lines: &[PageLine], column: Bounds, size: f32) -> Vec<TextRow> {
    lines
        .iter()
        .map(|line| TextRow {
            baseline: line.origin.y - column.min.y + size,
            runs: vec![TextRun {
                x: line.origin.x - column.min.x,
                text: line.text.clone(),
                color: line.color,
            }],
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // whole-number positions on whole-number metrics
mod tests;
