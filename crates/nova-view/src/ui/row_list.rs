//! A list of one-line rows in a box, one of them selected, with a scroll
//! bar when they don't fit: the mission computer's and the mission info
//! screen's lists, as the List Manager runs them ([`RowList`]). The keys
//! screen's list adds three things the List Manager's lists don't use: a
//! second column, heading rows and marked rows.

use std::time::Duration;

use super::list_bar::{BarPart, ListBar, scroll_delay};
use crate::color::Color;
use crate::draw::{DrawCommand, DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, MouseButton};
use crate::text::TextMetrics;
use crate::time::sixtieths;

/// The rows' size, in Geneva, as the description's.
pub const ROW_SIZE: f32 = 10.0;
/// The rows' colour.
pub const ROW_COLOR: Color = Color::WHITE;
/// The selected row's highlight.
pub const SELECTED: Color = Color::rgba(48, 64, 112, 255);
/// A heading row's colour ([`RowList::with_headings`]).
pub const HEADING_COLOR: Color = Color::DIM;
/// A marked row's colour ([`RowList::set_marked`]).
pub const MARKED_COLOR: Color = Color::ERROR;

/// How far `_SetupMissionInfoList` narrows the view for its bar
/// (`rView.right -= 15`, @0x9df84-0x9df90).
pub const BAR_INSET: f32 = 15.0;

/// Where a list's scroll bar goes when its rows don't fit the box: when
/// the row count times the line height is more than the box's height (the
/// original's `jg`, @0x9df7a, @0xa07a6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BarRoom {
    /// Carved from the box: the view is the box less [`BAR_INSET`] on its
    /// right, and the bar lies right of the view, as
    /// `_SetupMissionInfoList` makes the mission info screen's
    /// (@0x9df84-0x9df90).
    Inside,
    /// Beside the box: the view is the box and the bar lies right of it;
    /// when the rows fit, the view is widened to `fitted_right` instead, as
    /// `_SetupMissionList` widens the mission computer's over item 3
    /// (@0xa07a8-0xa07e3).
    Beside {
        /// The view's right when the rows fit.
        fitted_right: f32,
    },
}

/// What a pointer input did to a list ([`RowList::input`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListPress {
    /// Nothing: not a press the list takes, nor part of one it holds.
    Ignored,
    /// A press is held, on the rows or on the bar.
    Tracking,
    /// The press held is let go, or cancelled: `LClick` returns.
    Released {
        /// Whether the selection differs from the one at the press.
        changed: bool,
    },
}

/// A `ScrollDelay` under way: the tick it ends on, and the point the step
/// after it picks a row at (the pointer before the scroll).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Wait {
    until: u64,
    at: Point,
}

/// A press the list holds.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Held {
    /// On the rows: `LClick`'s loop.
    Rows {
        /// The pointer.
        at: Point,
        /// The row last under it (`LClick`'s last cell), if any yet.
        cell: Option<usize>,
        /// The tick the autoscroll began, while it lasts.
        start: Option<u64>,
        /// The delay after an autoscroll's step.
        wait: Option<Wait>,
    },
    /// On an arrow or a page: `TrackControl` with `ClickTrack`.
    Bar {
        part: BarPart,
        at: Point,
        /// The press's tick.
        start: u64,
        /// The tick the delay after the last step ends on.
        until: Option<u64>,
    },
    /// On the thumb: its outline dragged.
    Thumb { press: Point, at: Point },
}

/// A part of a [`RowList`] the pad can press ([`RowList::pad_cells`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListCell {
    /// Row `n` (from 0), shown.
    Row(usize),
    /// The bar's up arrow.
    Up,
    /// The bar's down arrow.
    Down,
}

/// Rows of text a line high each, in a view from the top of a box, run as
/// the List Manager runs the game's lists (see the
/// [`list_bar`](super::list_bar) module for the source).
///
/// - **What shows.** Every row the view reaches, the last one partly when
///   the view isn't a whole number of rows high: `visible` counts it
///   (`CellCommon`, `ListMgrPACK.a` line 1425). It is drawn clipped to the
///   view and a press hits it. The list scrolls a row at a time, from 0 to
///   the rows less the whole rows the view holds, so the last row can be
///   shown whole (`GetVisBounds` adds a row to the range when the fit is
///   bad, line 262; `JustRect` clamps, line 2320).
/// - **The bar.** When the rows don't fit, a [`ListBar`] scrolls them
///   ([`BarRoom`] says where it goes): a press on an arrow scrolls a row,
///   on the track a page (the rows shown less one, `ClickTrack`, line
///   1159), each at once and then again after each `ScrollDelay` while
///   the pointer stays on that part, and a drag of the thumb scrolls at its
///   release (`clickCont`, line 964).
/// - **A press on the rows** is held until its release ([`RowList::input`],
///   `LClick`, line 592): the row under the pointer, pinned into the view,
///   is the one selection, following the pointer. Over no row (below the
///   last), nothing is selected: the first pick deselects every cell
///   (line 797) and selecting a cell past the rows does nothing
///   (`ListSetSelect`, line 2563). With the pointer at or past the view's
///   top or bottom edge, the list scrolls a row towards it, then waits a
///   `ScrollDelay`, then picks the row under the pointer, so the row
///   scrolled in is selected.
/// - **A step** ([`RowList::step_wrapping`]) is the filters': with a row
///   selected, scroll down a row when its rect (`LRect`, which computes
///   one for any row, line 2250) reaches the view's bottom, or up a row
///   when it reaches the view's top; select the next row; then
///   `LAutoScroll` (line 531): a selected row not shown is scrolled to the
///   top, as far as the scroll goes.
/// - **The extent** ([`RowList::with_extent`]): steps wrap over it, which
///   may run past the rows; a selection there highlights nothing and
///   scrolls nothing, as the mission info screen's table after an abort
///   leaves one. A release reads the selection back as the filters do
///   (`LGetSelect` over the rows, `_MissionInfoFilter` @0x9dca9-0x9dcd6),
///   so one past the rows reads as none.
///
/// Nothing is drawn outside the view and the bar.
///
/// Three additions serve the keys screen and change nothing for a list
/// that doesn't use them: a second column of text, drawn at an offset
/// from each row's left ([`RowList::with_column`]); heading rows, drawn in
/// [`HEADING_COLOR`], which a press never selects and a step passes over
/// ([`RowList::with_headings`]); and marked rows, drawn in
/// [`MARKED_COLOR`] ([`RowList::set_marked`]).
#[derive(Clone, Debug, PartialEq)]
pub struct RowList {
    rows: Vec<String>,
    /// The second column, if any: its offset from the view's left and
    /// each row's text.
    column: Option<(f32, Vec<String>)>,
    /// Which rows are headings, by index; none when empty.
    headings: Vec<bool>,
    /// Which rows are marked, by index.
    marked: Vec<bool>,
    view: Bounds,
    bar: Option<ListBar>,
    line_height: f32,
    /// The rows the view holds whole.
    visible: usize,
    /// The rows the view reaches, a partly shown last one counted.
    shown: usize,
    extent: usize,
    first: usize,
    selected: Option<usize>,
    held: Option<Held>,
    /// The selection when the press held was made.
    pressed: Option<usize>,
    /// The time ticked, for the delays.
    clock: Duration,
}

impl RowList {
    /// `rows` in `rect`, measured by `metrics`, with their bar, if they
    /// need one, where `room` says; the first selected (none when there
    /// are none).
    #[must_use]
    pub fn new(rows: Vec<String>, rect: Bounds, metrics: &impl TextMetrics, room: BarRoom) -> Self {
        let line_height = metrics.line_height(Font::Geneva, ROW_SIZE);
        let overflows = rows.len() as f32 * line_height > rect.height();
        let view = match (room, overflows) {
            (BarRoom::Inside, true) => Bounds {
                min: rect.min,
                max: Point::new(rect.max.x - BAR_INSET, rect.max.y),
            },
            (BarRoom::Beside { fitted_right }, false) => Bounds {
                min: rect.min,
                max: Point::new(fitted_right, rect.max.y),
            },
            _ => rect,
        };
        // A NaN or negative quotient (no room, or no line height) is 0.
        let fit = view.height() / line_height;
        let selected = (!rows.is_empty()).then_some(0);
        Self {
            extent: rows.len(),
            column: None,
            headings: Vec::new(),
            marked: vec![false; rows.len()],
            rows,
            bar: overflows.then(|| ListBar::beside(view)),
            view,
            line_height,
            visible: fit.floor() as usize,
            shown: fit.ceil() as usize,
            first: 0,
            selected,
            held: None,
            pressed: None,
            clock: Duration::ZERO,
        }
    }

    /// This list with its steps wrapping over `extent` selections, past
    /// its rows when `extent` is more (never fewer than the rows).
    #[must_use]
    pub fn with_extent(mut self, extent: usize) -> Self {
        self.extent = extent.max(self.rows.len());
        self
    }

    /// This list with a second column, `cells` (one a row, the rest
    /// empty), drawn `at` from the view's left.
    #[must_use]
    pub fn with_column(mut self, at: f32, mut cells: Vec<String>) -> Self {
        cells.resize(self.rows.len(), String::new());
        self.column = Some((at, cells));
        self
    }

    /// This list with the rows at `indices` as headings, and the first row
    /// that is no heading selected (none when every row is one).
    #[must_use]
    pub fn with_headings(mut self, indices: &[usize]) -> Self {
        self.headings = (0..self.rows.len())
            .map(|index| indices.contains(&index))
            .collect();
        self.selected = (0..self.rows.len()).find(|&index| !self.is_heading(index));
        self
    }

    /// Every row.
    #[must_use]
    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    /// Sets row `index`'s text; nothing past the rows.
    pub fn set_row(&mut self, index: usize, text: &str) {
        if let Some(row) = self.rows.get_mut(index) {
            text.clone_into(row);
        }
    }

    /// Row `index`'s second-column text, if the list has the column and
    /// the row.
    #[must_use]
    pub fn cell(&self, index: usize) -> Option<&str> {
        self.column
            .as_ref()
            .and_then(|(_, cells)| cells.get(index))
            .map(String::as_str)
    }

    /// Sets row `index`'s second-column text; nothing without the column
    /// or past the rows.
    pub fn set_cell(&mut self, index: usize, text: &str) {
        if let Some(cell) = self
            .column
            .as_mut()
            .and_then(|(_, cells)| cells.get_mut(index))
        {
            text.clone_into(cell);
        }
    }

    /// Whether row `index` is a heading.
    #[must_use]
    pub fn is_heading(&self, index: usize) -> bool {
        self.headings.get(index).copied().unwrap_or(false)
    }

    /// Marks row `index`, or not; nothing past the rows.
    pub fn set_marked(&mut self, index: usize, marked: bool) {
        if let Some(mark) = self.marked.get_mut(index) {
            *mark = marked;
        }
    }

    /// Whether row `index` is marked.
    #[must_use]
    pub fn is_marked(&self, index: usize) -> bool {
        self.marked.get(index).copied().unwrap_or(false)
    }

    /// The selected index, if any: past the rows when a step over the
    /// extent took it there.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// The first row shown.
    #[must_use]
    pub fn first(&self) -> usize {
        self.first
    }

    /// How many rows fit the view whole.
    #[must_use]
    pub fn visible(&self) -> usize {
        self.visible
    }

    /// How many rows the view reaches, a partly shown last one counted.
    #[must_use]
    pub fn shown(&self) -> usize {
        self.shown
    }

    /// Where the rows are drawn (`rView`).
    #[must_use]
    pub fn view(&self) -> Bounds {
        self.view
    }

    /// Where the scroll bar is, when the rows don't fit.
    #[must_use]
    pub fn bar(&self) -> Option<Bounds> {
        self.bar.map(|bar| bar.rect())
    }

    /// Whether a press is held, on the rows or the bar, until its release.
    #[must_use]
    pub fn tracking(&self) -> bool {
        self.held.is_some()
    }

    /// The last first row the scroll reaches.
    fn last_first(&self) -> usize {
        self.rows.len().saturating_sub(self.visible)
    }

    /// Whether row `index` is in the view, partly or whole.
    fn shows(&self, index: usize) -> bool {
        index < self.rows.len()
            && index >= self.first
            && index < self.first.saturating_add(self.shown)
    }

    /// The top of row `index`'s rect, shown or not, as `LRect` places any
    /// cell from the first shown.
    fn row_top(&self, index: usize) -> f32 {
        self.view.min.y + (index as f32 - self.first as f32) * self.line_height
    }

    /// Where row `index` is on the screen, if it is shown: a partly shown
    /// last row's rect runs past the view.
    #[must_use]
    pub fn row_bounds(&self, index: usize) -> Option<Bounds> {
        self.shows(index).then(|| {
            Bounds::at(
                Point::new(self.view.min.x, self.row_top(index)),
                self.view.width(),
                self.line_height,
            )
        })
    }

    /// Scrolls `by` rows, held to the scroll's range (`LScroll`).
    fn scroll_by(&mut self, by: isize) {
        let to = self.first.saturating_add_signed(by);
        self.first = to.min(self.last_first());
    }

    /// `LAutoScroll`: a selected row not shown, partly or whole, is
    /// scrolled to the top, as far as the scroll goes.
    fn auto_scroll(&mut self) {
        if let Some(index) = self.selected
            && index < self.rows.len()
            && !self.shows(index)
        {
            self.first = index.min(self.last_first());
        }
    }

    /// Selects index `index`, held to the extent, and scrolls to show its
    /// row whole if it has one; nothing with no rows.
    pub fn select(&mut self, index: usize) {
        let Some(last) = self.extent.checked_sub(1) else {
            return;
        };
        let index = index.min(last);
        self.selected = Some(index);
        if index >= self.rows.len() {
            return;
        }
        if index < self.first {
            self.first = index;
        } else if index >= self.first + self.visible {
            self.first = (index + 1).saturating_sub(self.visible);
        }
    }

    /// Selects nothing, leaving the list scrolled as it is.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Steps the selection forward or back over the extent, wrapping from
    /// the last to the first and from the first to the last, as the
    /// mission computer's Tab, Down and Up do (`_MissionFilter` @0x9f8cc,
    /// @0x9faf2, @0x9fce3) and the mission info screen's
    /// (`_MissionInfoFilter` @0x9d778, @0x9d9ac). With a row selected, it
    /// first scrolls down a row when that row's rect reaches the view's
    /// bottom, or, stepping back, up a row when it reaches the view's top
    /// (`LRect`/`LScroll` @0x9d7d7-0x9d7fd, @0x9d9fe-0x9da23;
    /// `_MissionFilter` @0x9f91e-0x9f944, @0x9fb52-0x9fb78,
    /// @0x9fd81-0x9fda6); after the step, `LAutoScroll` brings a row not
    /// shown to the top (@0x9d8f1). From no selection, which the mission
    /// info screen has after an abort, the forward step selects the first
    /// and the back step the last (@0x9d839, @0x9da60; `_MissionFilter`
    /// @0x9f949, @0x9fcfa). Returns whether the selection changed: never
    /// with nothing to step over, nor from the only one.
    ///
    /// A heading is stepped over, as many steps as it takes, so a list of
    /// headings alone selects nothing.
    pub fn step_wrapping(&mut self, forward: bool) -> bool {
        let before = self.selected;
        for _ in 0..self.extent {
            self.step_once(forward);
            if !self.selected.is_some_and(|index| self.is_heading(index)) {
                return self.selected != before;
            }
        }
        self.selected = before;
        false
    }

    /// One step of [`RowList::step_wrapping`], headings and all.
    fn step_once(&mut self, forward: bool) {
        let count = self.extent;
        if count == 0 {
            return;
        }
        let before = self.selected;
        if let Some(index) = before {
            let top = self.row_top(index);
            if forward {
                if top + self.line_height >= self.view.max.y {
                    self.scroll_by(1);
                }
            } else if top <= self.view.min.y {
                self.scroll_by(-1);
            }
        }
        let next = match before {
            Some(index) if forward => (index + 1) % count,
            Some(index) => (index + count - 1) % count,
            None if forward => 0,
            None => count - 1,
        };
        self.selected = Some(next);
        self.auto_scroll();
    }

    /// What the pad can press in the list, in order: each row shown
    /// (clipped to the view), then, while the bar shows, its up and down
    /// arrows. The bar's page areas and thumb are left out, as a press
    /// can't drag.
    #[must_use]
    pub fn pad_cells(&self) -> Vec<(ListCell, Bounds)> {
        let rows = (self.first..self.first.saturating_add(self.shown)).filter_map(|index| {
            self.row_bounds(index).map(|row| {
                let clipped = Bounds {
                    min: row.min,
                    max: Point::new(row.max.x, row.max.y.min(self.view.max.y)),
                };
                (ListCell::Row(index), clipped)
            })
        });
        let arrows = self.bar.into_iter().flat_map(|bar| {
            [(ListCell::Up, BarPart::Up), (ListCell::Down, BarPart::Down)]
                .into_iter()
                .filter_map(move |(cell, part)| bar.arrow(part).map(|rect| (cell, rect)))
        });
        rows.chain(arrows).collect()
    }

    /// Presses `cell` by key, as a click (a press and release) on it
    /// does: a row is selected (none when it is a heading or past the
    /// rows), and an arrow scrolls a row. [`ListPress::Ignored`] while a
    /// pointer press is held, or for a cell not in [`RowList::pad_cells`].
    pub fn press_cell(&mut self, cell: ListCell) -> ListPress {
        if self.held.is_some() || !self.pad_cells().iter().any(|(c, _)| *c == cell) {
            return ListPress::Ignored;
        }
        self.pressed = self.selected;
        match cell {
            ListCell::Row(row) => {
                self.selected = (row < self.rows.len() && !self.is_heading(row)).then_some(row);
            }
            ListCell::Up => self.scroll_by(-1),
            ListCell::Down => self.scroll_by(1),
        }
        ListPress::Released {
            changed: self.selected != self.pressed,
        }
    }

    /// The pointer's input: a left press on the bar or in the view starts
    /// a press the list holds ([`ListPress::Tracking`]), pointer moves
    /// track it, and the left release ends it ([`ListPress::Released`]),
    /// as `LClick` runs until the button comes up. Anything else is
    /// [`ListPress::Ignored`], keys included.
    pub fn input(&mut self, input: &Input) -> ListPress {
        match *input {
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at,
            } => self.press(at),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: false,
                ..
            } => self.release(),
            Input::PointerMoved(to) => {
                let Some(held) = &mut self.held else {
                    return ListPress::Ignored;
                };
                match held {
                    Held::Rows { at, .. } | Held::Bar { at, .. } | Held::Thumb { at, .. } => {
                        *at = to;
                    }
                }
                self.run();
                ListPress::Tracking
            }
            _ => ListPress::Ignored,
        }
    }

    /// Ends a press held as its release would ([`ListPress::Released`]),
    /// wherever the pointer is: `LClick` ends on any release. Nothing is
    /// held after, and [`ListPress::Ignored`] when nothing was.
    pub fn cancel_pointer(&mut self) -> ListPress {
        self.release()
    }

    /// Time passing: a held arrow or page steps again, and an autoscroll
    /// goes on, as their delays end.
    pub fn tick(&mut self, dt: Duration) {
        self.clock += dt;
        self.run();
    }

    /// The tick it is.
    fn now(&self) -> u64 {
        u64::try_from(sixtieths(self.clock)).unwrap_or(u64::MAX)
    }

    /// A left press at `at`.
    fn press(&mut self, at: Point) -> ListPress {
        if self.held.is_some() {
            return ListPress::Tracking;
        }
        let part = self
            .bar
            .and_then(|bar| bar.part_at(at, self.first, self.last_first()));
        let held = match part {
            Some(BarPart::Thumb) => Held::Thumb { press: at, at },
            Some(part) => Held::Bar {
                part,
                at,
                start: self.now(),
                until: None,
            },
            None if self.view.contains_half_open(at) => Held::Rows {
                at,
                cell: None,
                start: None,
                wait: None,
            },
            None => return ListPress::Ignored,
        };
        self.pressed = self.selected;
        self.held = Some(held);
        self.run();
        ListPress::Tracking
    }

    /// The press held does what is due now.
    fn run(&mut self) {
        let now = self.now();
        match self.held {
            Some(Held::Rows {
                at,
                mut cell,
                mut start,
                mut wait,
            }) => {
                if let Some(due) = wait {
                    if now < due.until {
                        return;
                    }
                    wait = None;
                    self.pick(due.at, &mut cell);
                }
                let toward = self.autoscroll_toward(at);
                if toward == 0 {
                    start = None;
                    self.pick(at, &mut cell);
                } else {
                    let began = *start.get_or_insert(now);
                    self.scroll_by(toward);
                    wait = Some(Wait {
                        until: now + scroll_delay(now - began).max(1),
                        at,
                    });
                }
                self.held = Some(Held::Rows {
                    at,
                    cell,
                    start,
                    wait,
                });
            }
            Some(Held::Bar {
                part,
                at,
                start,
                until,
            }) => {
                if until.is_some_and(|until| now < until) {
                    return;
                }
                let on_part = self
                    .bar
                    .and_then(|bar| bar.part_at(at, self.first, self.last_first()))
                    == Some(part);
                let until = on_part.then(|| {
                    let page = self.shown.saturating_sub(1).max(1).cast_signed();
                    self.scroll_by(match part {
                        BarPart::Up => -1,
                        BarPart::Down => 1,
                        BarPart::PageUp => -page,
                        BarPart::PageDown => page,
                        BarPart::Thumb => 0,
                    });
                    now + scroll_delay(now - start).max(1)
                });
                self.held = Some(Held::Bar {
                    part,
                    at,
                    start,
                    until,
                });
            }
            Some(Held::Thumb { .. }) | None => {}
        }
    }

    /// Which way `LClick` autoscrolls with the pointer at `at`: up a row
    /// at or above the view's top with rows above, down a row at or below
    /// its last pixel row with rows below, or not (0).
    fn autoscroll_toward(&self, at: Point) -> isize {
        let mut toward = 0;
        if at.y <= self.view.min.y && self.first > 0 {
            toward = -1;
        }
        if at.y + 1.0 >= self.view.max.y && self.first < self.last_first() {
            toward = 1;
        }
        toward
    }

    /// `LClick` picks the row under `at`, pinned into the view, when it
    /// isn't the last row picked (`cell`): the row is the one selection, or
    /// nothing is selected when there is no row there. `LClick`'s first
    /// pick deselects every cell unless its own is the one selected
    /// (`OneClick`, `ClearAll`), and each later one deselects the last cell
    /// picked and selects the new (`NoMods`); with one selection, which
    /// nothing else moves while the press is held, both leave just that.
    fn pick(&mut self, at: Point, cell: &mut Option<usize>) {
        let bottom = (self.view.max.y - 1.0).max(self.view.min.y);
        let offset = (at.y.clamp(self.view.min.y, bottom) - self.view.min.y) / self.line_height;
        let row = self.first.saturating_add(offset.floor() as usize);
        if *cell == Some(row) {
            return;
        }
        self.selected = (row < self.rows.len() && !self.is_heading(row)).then_some(row);
        *cell = Some(row);
    }

    /// The release of the press held: a pick the autoscroll's delay held
    /// back is made (`ScrollDelay` ends on the release), a dragged thumb
    /// scrolls to where it was dropped, and the selection is read back.
    fn release(&mut self) -> ListPress {
        let Some(held) = self.held.take() else {
            return ListPress::Ignored;
        };
        match held {
            Held::Rows {
                mut cell,
                wait: Some(due),
                ..
            } => self.pick(due.at, &mut cell),
            Held::Thumb { press, at } => {
                if let Some(bar) = self.bar {
                    let to = bar.dragged_value(self.first, self.last_first(), press, at);
                    self.first = to;
                }
            }
            Held::Rows { .. } | Held::Bar { .. } => {}
        }
        if self.selected.is_some_and(|index| index >= self.rows.len()) {
            self.selected = None;
        }
        ListPress::Released {
            changed: self.selected != self.pressed,
        }
    }

    /// Draws the selected row's highlight, then each row shown, a partly
    /// shown last one clipped to the view, then the bar.
    pub fn draw(&self, list: &mut DrawList) {
        let view = self.view;
        if let Some(row) = self.selected.and_then(|index| self.row_bounds(index)) {
            let shown = Bounds {
                min: row.min,
                max: Point::new(row.max.x, row.max.y.min(view.max.y)),
            };
            fill_rect(list, shown, SELECTED);
        }
        for (index, text) in self.rows.iter().enumerate() {
            if let Some(row) = self.row_bounds(index) {
                let color = if self.is_heading(index) {
                    HEADING_COLOR
                } else if self.is_marked(index) {
                    MARKED_COLOR
                } else {
                    ROW_COLOR
                };
                let cell = self
                    .column
                    .as_ref()
                    .map(|(at, cells)| (Point::new(row.min.x + at, row.min.y), &cells[index]));
                for (origin, text) in std::iter::once((row.min, text)).chain(cell) {
                    let command = DrawCommand::Text {
                        text: text.clone(),
                        font: Font::Geneva,
                        origin,
                        size: ROW_SIZE,
                        wrap_width: None,
                        color,
                    };
                    if row.max.y > view.max.y {
                        list.clipped(view, command);
                    } else {
                        list.push(command);
                    }
                }
            }
        }
        if let Some(bar) = self.bar {
            let max = self.last_first();
            let (held, outline) = match self.held {
                Some(Held::Bar { part, at, .. }) => (
                    (bar.part_at(at, self.first, max) == Some(part)).then_some(part),
                    None,
                ),
                Some(Held::Thumb { press, at }) => {
                    (None, bar.drag_outline(self.first, max, press, at))
                }
                _ => (None, None),
            };
            bar.draw(list, self.first, max, held, outline);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::Key;
    use crate::text::fixture::MonoMetrics;

    /// A 100 x 40 box at (10, 20): three rows of 12 whole, and a fourth's
    /// top third.
    const BOX: Bounds = Bounds {
        min: Point::new(10.0, 20.0),
        max: Point::new(110.0, 60.0),
    };

    /// A 100 x 84 box at (10, 20): seven rows of 12 exactly, as stock
    /// `DITL` 1012's item 2 holds the engine's Geneva 10.
    const TALL: Bounds = Bounds {
        min: Point::new(10.0, 20.0),
        max: Point::new(110.0, 104.0),
    };

    /// A 100 x 36 box at (10, 20): three rows of 12 exactly.
    const EXACT: Bounds = Bounds {
        min: Point::new(10.0, 20.0),
        max: Point::new(110.0, 56.0),
    };

    fn names(rows: usize) -> Vec<String> {
        (0..rows).map(|n| format!("Row {n}")).collect()
    }

    /// `rows` rows in [`BOX`], the bar beside it.
    fn list(rows: usize) -> RowList {
        in_box(rows, BOX)
    }

    /// `rows` rows in `rect`, the bar beside it.
    fn in_box(rows: usize, rect: Bounds) -> RowList {
        RowList::new(
            names(rows),
            rect,
            &MonoMetrics,
            BarRoom::Beside {
                fitted_right: rect.max.x,
            },
        )
    }

    /// `rows` rows in [`TALL`], the bar carved from it: the view (10, 20)
    /// to (95, 104), the bar (95, 19) to (111, 105) when they don't fit.
    fn tall(rows: usize) -> RowList {
        RowList::new(names(rows), TALL, &MonoMetrics, BarRoom::Inside)
    }

    fn press(at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        }
    }

    fn release(at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        }
    }

    /// `n` Mac ticks.
    fn ticks(n: u32) -> Duration {
        Duration::from_secs(1) / 60 * n
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
        assert_eq!(rows.shown(), 4, "the fourth partly");
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
    fn the_bar_shows_only_when_the_rows_dont_fit() {
        let fit = tall(7);
        assert_eq!(fit.bar(), None, "7 x 12 is not more than 84 (@0x9df7a)");
        assert_eq!(fit.view(), TALL);
        let over = tall(8);
        assert_eq!(
            over.bar(),
            Some(Bounds {
                min: Point::new(95.0, 19.0),
                max: Point::new(111.0, 105.0),
            })
        );
        assert_eq!(
            over.row_bounds(0),
            Some(Bounds::at(Point::new(10.0, 20.0), 85.0, 12.0)),
            "the rows narrowed by 15 (@0x9df84-0x9df90)"
        );
        let mut list = DrawList::new();
        over.draw(&mut list);
        let mut bar = DrawList::new();
        ListBar::beside(over.view()).draw(&mut bar, 0, 1, None, None);
        let bar: Vec<DrawCommand> = bar.iter().cloned().collect();
        assert!(list.iter().cloned().collect::<Vec<_>>().ends_with(&bar));
    }

    /// What `bar` draws over `rows`' view at the scroll `rows` has.
    fn bar_drawn(
        rows: &RowList,
        held: Option<BarPart>,
        outline: Option<Bounds>,
    ) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        let max = rows.rows().len() - rows.visible();
        ListBar::beside(rows.view()).draw(&mut list, rows.first(), max, held, outline);
        list.iter().cloned().collect()
    }

    #[test]
    fn the_bar_draws_its_held_part_while_on_it_and_a_dragged_thumbs_outline() {
        let mut rows = tall(20);
        let down = Point::new(100.0, 100.0);
        rows.input(&press(down));
        assert!(drawn(&rows).ends_with(&bar_drawn(&rows, Some(BarPart::Down), None)));
        rows.input(&Input::PointerMoved(Point::new(100.0, 60.0)));
        assert!(
            drawn(&rows).ends_with(&bar_drawn(&rows, None, None)),
            "off the arrow, not held down"
        );
        rows.input(&release(down));
        let at = Point::new(100.0, 60.0);
        let thumb = rows.bar.expect("a bar").thumb(rows.first(), 13).center();
        rows.input(&press(thumb));
        rows.input(&Input::PointerMoved(at));
        let outline = rows
            .bar
            .expect("a bar")
            .drag_outline(rows.first(), 13, thumb, at);
        assert!(outline.is_some());
        assert!(drawn(&rows).ends_with(&bar_drawn(&rows, None, outline)));
    }

    #[test]
    fn a_row_that_ends_at_the_views_bottom_is_drawn_whole() {
        assert_eq!(texts(&in_box(3, EXACT)).len(), 3, "no clip for the third");
    }

    #[test]
    fn beside_widens_the_box_when_the_rows_fit() {
        let room = BarRoom::Beside {
            fitted_right: 125.0,
        };
        let fit = RowList::new(names(7), TALL, &MonoMetrics, room);
        assert_eq!(fit.bar(), None);
        assert_eq!(
            fit.row_bounds(0).map(|row| row.width()),
            Some(115.0),
            "over item 3 (@0xa07a8-0xa07e3)"
        );
        let over = RowList::new(names(8), TALL, &MonoMetrics, room);
        assert_eq!(over.view(), TALL, "the box itself (@0xa07a6)");
        assert_eq!(over.bar().map(|bar| bar.min.x), Some(110.0), "right of it");
    }

    /// A click's press and release at `rect`'s centre on `rows`.
    fn clicked(rows: &mut RowList, rect: Bounds) -> ListPress {
        rows.input(&press(rect.center()));
        rows.input(&release(rect.center()))
    }

    #[test]
    fn pressing_a_row_by_key_selects_it_as_a_click_does() {
        for list in [tall(10), list(2), tall(10).with_headings(&[2])] {
            for (cell, rect) in list.pad_cells() {
                if !matches!(cell, ListCell::Row(_)) {
                    continue;
                }
                let mut keyed = list.clone();
                let mut pointed = list.clone();
                assert_eq!(
                    keyed.press_cell(cell),
                    clicked(&mut pointed, rect),
                    "{cell:?}"
                );
                assert_eq!(keyed, pointed, "{cell:?}");
            }
        }
    }

    #[test]
    fn pressing_the_bar_arrows_by_key_steps_as_a_click_does() {
        let mut list = tall(10);
        list.press_cell(ListCell::Down);
        for cell in [ListCell::Up, ListCell::Down] {
            let rect = list
                .pad_cells()
                .into_iter()
                .find(|(c, _)| *c == cell)
                .map(|(_, rect)| rect)
                .expect("an arrow");
            let mut keyed = list.clone();
            let mut pointed = list.clone();
            assert_eq!(keyed.press_cell(cell), clicked(&mut pointed, rect));
            assert_eq!(keyed, pointed, "{cell:?}");
            assert_ne!(keyed.first(), list.first(), "{cell:?} scrolled");
        }
    }

    #[test]
    fn the_visible_rows_and_bar_arrows_are_the_lists_cells() {
        let cells: Vec<ListCell> = tall(10).pad_cells().into_iter().map(|(c, _)| c).collect();
        let mut expected: Vec<ListCell> = (0..7).map(ListCell::Row).collect();
        expected.extend([ListCell::Up, ListCell::Down]);
        assert_eq!(cells, expected);
        let mut scrolled = tall(10);
        scrolled.press_cell(ListCell::Down);
        assert_eq!(
            scrolled.pad_cells()[0].0,
            ListCell::Row(1),
            "follows a scroll"
        );
        let short: Vec<ListCell> = list(2).pad_cells().into_iter().map(|(c, _)| c).collect();
        assert_eq!(short, [ListCell::Row(0), ListCell::Row(1)], "no bar");
        // A partly shown last row is clipped to the view.
        let partly = list(6).pad_cells();
        assert_eq!(partly[3].1.max.y, BOX.max.y);
        assert_eq!(partly[3].0, ListCell::Row(3));
        assert_eq!(
            list(6).press_cell(ListCell::Row(5)),
            ListPress::Ignored,
            "not shown"
        );
    }

    #[test]
    fn a_press_on_the_bars_arrow_scrolls_a_row_and_holding_it_repeats() {
        // Ten rows in seven: the scroll runs 0 to 3.
        let mut rows = tall(10);
        let down = Point::new(100.0, 100.0);
        assert_eq!(rows.input(&press(down)), ListPress::Tracking);
        assert_eq!(rows.first(), 1, "a step at once");
        rows.tick(ticks(11));
        assert_eq!(rows.first(), 1);
        rows.tick(ticks(1));
        assert_eq!(rows.first(), 2, "after ScrollDelay's 12");
        rows.tick(ticks(2));
        assert_eq!(rows.first(), 2);
        rows.tick(ticks(1));
        assert_eq!(rows.first(), 3, "then 3");
        rows.tick(ticks(30));
        assert_eq!(rows.first(), 3, "held at the end");
        assert_eq!(rows.selected(), Some(0), "the selection stays");
        assert_eq!(
            rows.input(&release(down)),
            ListPress::Released { changed: false }
        );
        assert!(!rows.tracking());
        rows.tick(ticks(60));
        assert_eq!(rows.first(), 3, "let go");
        assert_eq!(
            rows.input(&press(Point::new(100.0, 25.0))),
            ListPress::Tracking
        );
        assert_eq!(rows.first(), 2, "the up arrow");
        let mut later = tall(10);
        later.tick(ticks(20));
        later.input(&press(down));
        later.tick(ticks(11));
        assert_eq!(later.first(), 1, "the delay counts from the press");
        later.tick(ticks(1));
        assert_eq!(later.first(), 2);
    }

    #[test]
    fn a_drag_autoscrolls_only_at_the_views_edges_and_with_rows_to_bring_in() {
        // Selected row 1, a press on the top edge row with nothing above
        // picks row 0 at once: no autoscroll, no delay.
        let mut top = list(3);
        top.select(1);
        top.input(&press(Point::new(50.0, 20.0)));
        assert_eq!(top.selected(), Some(0), "nothing above to scroll in");
        // The view's last pixel row autoscrolls; the one above doesn't.
        let mut low = list(8);
        low.input(&press(Point::new(50.0, 22.0)));
        low.input(&Input::PointerMoved(Point::new(50.0, 58.0)));
        assert_eq!(
            (low.first(), low.selected()),
            (0, Some(3)),
            "row 3, partly shown"
        );
        low.input(&Input::PointerMoved(Point::new(50.0, 59.0)));
        assert_eq!(low.first(), 1, "the last pixel row");
        // At the end of the scroll, past the bottom picks at once.
        let mut end = list(8);
        end.scroll_by(5);
        end.input(&press(Point::new(50.0, 22.0)));
        assert_eq!(end.selected(), Some(5));
        end.input(&Input::PointerMoved(Point::new(50.0, 70.0)));
        assert_eq!(
            (end.first(), end.selected()),
            (5, None),
            "pinned to the empty part row below the last, at once"
        );
        // Pinned to the view's last pixel row, not past it.
        let mut fit = in_box(3, EXACT);
        fit.input(&press(Point::new(50.0, 22.0)));
        fit.input(&Input::PointerMoved(Point::new(50.0, 80.0)));
        assert_eq!(fit.selected(), Some(2), "the last row, pinned to y 55");
    }

    #[test]
    fn a_held_arrow_waits_while_the_pointer_is_off_it() {
        let mut rows = tall(30);
        let down = Point::new(100.0, 100.0);
        rows.input(&press(down));
        assert_eq!(rows.first(), 1);
        rows.input(&Input::PointerMoved(Point::new(100.0, 60.0)));
        rows.tick(ticks(20));
        assert_eq!(rows.first(), 1, "off the arrow");
        rows.input(&Input::PointerMoved(down));
        assert_eq!(rows.first(), 2, "back on it, its delay over");
        rows.tick(ticks(1));
        assert_eq!(rows.first(), 2, "48 / (20 + 4) is 2 ticks");
        rows.tick(ticks(1));
        assert_eq!(rows.first(), 3);
    }

    #[test]
    fn a_press_in_the_page_area_scrolls_a_page_until_the_thumb_reaches_the_pointer() {
        // Twenty rows in seven: the scroll runs 0 to 13, a page is 6.
        let mut rows = tall(20);
        let below = Point::new(100.0, 80.0);
        rows.input(&press(below));
        assert_eq!(rows.first(), 6, "the rows shown less one (ClickTrack)");
        rows.tick(ticks(12));
        assert_eq!(rows.first(), 12, "again after the delay");
        rows.tick(ticks(30));
        assert_eq!(rows.first(), 12, "the thumb is under the pointer now");
        rows.input(&release(below));
        rows.input(&press(Point::new(100.0, 50.0)));
        assert_eq!(rows.first(), 6, "a page back");
    }

    #[test]
    fn dragging_the_thumb_scrolls_the_list_at_the_release() {
        // The thumb at 0 is 35 to 51; the track less it, 38.
        let mut rows = tall(20);
        rows.input(&press(Point::new(100.0, 40.0)));
        rows.input(&Input::PointerMoved(Point::new(100.0, 59.0)));
        assert_eq!(rows.first(), 0, "not while dragged");
        let outline = drawn(&rows).len();
        assert_eq!(
            rows.input(&release(Point::new(100.0, 59.0))),
            ListPress::Released { changed: false }
        );
        // 19 of 38 over 0 to 13: 247 / 38 = 6 r 19, not more than half.
        assert_eq!(rows.first(), 6);
        assert!(drawn(&rows).len() < outline, "the outline goes");
    }

    #[test]
    fn the_partly_shown_last_row_is_drawn_clipped_and_hit() {
        let mut rows = list(4);
        assert_eq!(
            rows.row_bounds(3),
            Some(Bounds::at(Point::new(10.0, 56.0), 100.0, 12.0))
        );
        assert!(drawn(&rows).iter().any(|command| matches!(
            command,
            DrawCommand::Clipped { clip, command }
                if *clip == BOX && matches!(&**command, DrawCommand::Text { text, .. } if text == "Row 3")
        )));
        rows.input(&press(Point::new(50.0, 58.0)));
        assert_eq!(rows.selected(), Some(3), "a press there hits it");
        assert_eq!(rows.first(), 0, "and it stays where it is");
        let mut highlight = DrawList::new();
        fill_rect(
            &mut highlight,
            Bounds::at(Point::new(10.0, 56.0), 100.0, 4.0),
            SELECTED,
        );
        assert_eq!(
            drawn(&rows)[0],
            highlight.iter().next().cloned().expect("one")
        );
        assert_eq!(rows.bar().map(|_| ()), Some(()), "4 x 12 is more than 40");
        let mut far = list(6);
        assert_eq!(far.row_bounds(4), None, "the row past the partly shown one");
        far.input(&press(Point::new(117.0, 58.0)));
        far.input(&release(Point::new(117.0, 58.0)));
        assert_eq!(far.first(), 1, "the down arrow");
        assert_eq!(
            far.input(&press(Point::new(117.0, 58.0))),
            ListPress::Tracking
        );
        assert_eq!(
            far.first(),
            2,
            "the scroll ends at the rows less the 3 whole"
        );
        far.tick(ticks(60));
        assert_eq!(far.first(), 3);
        assert_eq!(
            far.row_bounds(5).map(|row| row.min.y),
            Some(44.0),
            "shown whole"
        );
    }

    #[test]
    fn a_click_on_a_row_selects_it_and_elsewhere_nothing() {
        let mut rows = list(3);
        assert_eq!(
            rows.input(&press(Point::new(50.0, 50.0))),
            ListPress::Tracking
        );
        assert_eq!(rows.selected(), Some(2));
        assert_eq!(
            rows.input(&release(Point::new(50.0, 50.0))),
            ListPress::Released { changed: true }
        );
        rows.input(&press(Point::new(50.0, 50.0)));
        assert_eq!(
            rows.input(&release(Point::new(50.0, 50.0))),
            ListPress::Released { changed: false },
            "already"
        );
        assert_eq!(
            rows.input(&press(Point::new(5.0, 25.0))),
            ListPress::Ignored,
            "outside"
        );
        assert_eq!(
            rows.input(&press(Point::new(50.0, 60.0))),
            ListPress::Ignored,
            "below"
        );
        assert_eq!(
            rows.input(&release(Point::new(50.0, 22.0))),
            ListPress::Ignored,
            "a release with nothing held"
        );
        assert_eq!(rows.selected(), Some(2));
    }

    #[test]
    fn a_click_below_the_last_row_selects_nothing() {
        let mut few = list(1);
        assert_eq!(
            few.input(&press(Point::new(50.0, 35.0))),
            ListPress::Tracking
        );
        assert_eq!(
            few.selected(),
            None,
            "ListMgrPACK.a line 797, then ClearAll"
        );
        assert_eq!(
            few.input(&release(Point::new(50.0, 35.0))),
            ListPress::Released { changed: true }
        );
        assert_eq!(few.first(), 0, "the scroll stays");
        few.input(&press(Point::new(50.0, 22.0)));
        few.input(&Input::PointerMoved(Point::new(50.0, 40.0)));
        assert_eq!(few.selected(), None, "dragged off the rows");
        few.input(&Input::PointerMoved(Point::new(50.0, 25.0)));
        assert_eq!(few.selected(), Some(0), "and back");
    }

    #[test]
    fn a_press_highlights_its_row_and_a_drag_moves_it_to_the_release() {
        let mut rows = list(3);
        rows.input(&press(Point::new(50.0, 22.0)));
        assert_eq!(rows.selected(), Some(0));
        assert!(rows.tracking());
        assert_eq!(
            rows.input(&Input::PointerMoved(Point::new(50.0, 46.0))),
            ListPress::Tracking
        );
        assert_eq!(rows.selected(), Some(2));
        let mut highlight = DrawList::new();
        fill_rect(
            &mut highlight,
            Bounds::at(Point::new(10.0, 44.0), 100.0, 12.0),
            SELECTED,
        );
        assert_eq!(
            drawn(&rows)[0],
            highlight.iter().next().cloned().expect("one")
        );
        rows.input(&Input::PointerMoved(Point::new(500.0, 34.0)));
        assert_eq!(rows.selected(), Some(1), "pinned into the view");
        assert_eq!(
            rows.input(&release(Point::new(50.0, 34.0))),
            ListPress::Released { changed: true }
        );
        assert!(!rows.tracking());
        assert_eq!(
            rows.input(&Input::PointerMoved(Point::new(50.0, 22.0))),
            ListPress::Ignored
        );
        assert_eq!(rows.selected(), Some(1));
    }

    #[test]
    fn dragging_past_the_bottom_or_top_autoscrolls_selecting_the_row_scrolled_in() {
        // Eight rows, three whole and a part: the scroll runs 0 to 5.
        let mut rows = list(8);
        rows.input(&press(Point::new(50.0, 22.0)));
        rows.input(&Input::PointerMoved(Point::new(50.0, 70.0)));
        assert_eq!(rows.first(), 1, "a row at once");
        assert_eq!(rows.selected(), Some(0), "picked after the delay");
        rows.tick(ticks(11));
        assert_eq!((rows.first(), rows.selected()), (1, Some(0)));
        rows.tick(ticks(1));
        assert_eq!(rows.selected(), Some(4), "the partly shown row, pinned");
        assert_eq!(rows.first(), 2, "and on");
        rows.tick(ticks(3));
        assert_eq!((rows.first(), rows.selected()), (3, Some(5)));
        assert_eq!(
            rows.input(&release(Point::new(50.0, 70.0))),
            ListPress::Released { changed: true }
        );
        assert_eq!(
            rows.selected(),
            Some(6),
            "the release ends the delay's pick"
        );
        rows.input(&press(Point::new(50.0, 22.0)));
        assert_eq!(
            rows.selected(),
            Some(3),
            "no autoscroll inside the top edge"
        );
        rows.input(&Input::PointerMoved(Point::new(50.0, 20.0)));
        assert_eq!(rows.first(), 2, "at the top edge, up a row");
        rows.tick(ticks(12));
        assert_eq!(rows.selected(), Some(2), "the row scrolled in");
        assert_eq!(rows.first(), 1, "and up again");
        rows.tick(ticks(2));
        assert_eq!(
            (rows.first(), rows.selected()),
            (1, Some(2)),
            "3 ticks this time: 48 over the 12 held plus 4"
        );
        rows.input(&Input::PointerMoved(Point::new(50.0, 40.0)));
        rows.tick(ticks(5));
        assert_eq!(rows.first(), 1, "the delay ran out first");
        assert_eq!(
            rows.selected(),
            Some(2),
            "rows 1 to 4 shown: (40 - 20) / 12 is 1"
        );
    }

    #[test]
    fn a_release_reports_whether_the_selection_changed_since_the_press() {
        let mut rows = list(3);
        rows.input(&press(Point::new(50.0, 22.0)));
        rows.input(&Input::PointerMoved(Point::new(50.0, 46.0)));
        rows.input(&Input::PointerMoved(Point::new(50.0, 22.0)));
        assert_eq!(
            rows.input(&release(Point::new(50.0, 22.0))),
            ListPress::Released { changed: false },
            "dragged back to the row it began on"
        );
    }

    #[test]
    fn cancel_pointer_ends_a_track_as_its_release() {
        let mut rows = list(3);
        assert_eq!(rows.cancel_pointer(), ListPress::Ignored, "nothing held");
        rows.input(&press(Point::new(50.0, 22.0)));
        rows.input(&Input::PointerMoved(Point::new(50.0, 46.0)));
        assert_eq!(rows.cancel_pointer(), ListPress::Released { changed: true });
        assert_eq!(rows.selected(), Some(2), "the row it reached");
        assert!(!rows.tracking());
        let mut long = tall(30);
        long.input(&press(Point::new(100.0, 100.0)));
        long.cancel_pointer();
        long.tick(ticks(60));
        assert_eq!(long.first(), 1, "a held arrow stops");
    }

    #[test]
    fn keys_go_to_the_screen_not_the_list() {
        let mut rows = list(3);
        for key in [Key::ArrowUp, Key::ArrowDown, Key::Enter] {
            let press = Input::Key {
                key,
                pressed: true,
                repeat: false,
            };
            assert_eq!(rows.input(&press), ListPress::Ignored, "{key:?}");
            assert_eq!(rows.selected(), Some(0), "{key:?}");
        }
    }

    #[test]
    fn the_list_scrolls_to_keep_the_selection_shown() {
        let mut rows = list(5);
        for _ in 0..3 {
            rows.step_wrapping(true);
        }
        assert_eq!(rows.selected(), Some(3));
        assert_eq!(
            rows.first(),
            0,
            "row 3 is partly shown, which LAutoScroll counts as shown (ListMgrPACK.a line 531)"
        );
        rows.step_wrapping(true);
        assert_eq!(rows.selected(), Some(4));
        assert_eq!(
            rows.first(),
            1,
            "row 3's rect reached the bottom (@0x9d7dc)"
        );
        assert_eq!(rows.row_bounds(0), None);
        assert_eq!(
            rows.row_bounds(4),
            Some(Bounds::at(Point::new(10.0, 56.0), 100.0, 12.0))
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
            rows.step_wrapping(true);
        }
        assert_eq!(rows.selected(), Some(5));
        assert_eq!(rows.first(), 2);
        assert!(rows.row_bounds(5).is_some(), "shown");
        rows.select(3);
        assert_eq!(rows.first(), 2, "shown already");
        let mut exact = in_box(8, EXACT);
        for _ in 0..5 {
            exact.step_wrapping(true);
        }
        assert_eq!(
            (exact.selected(), exact.first()),
            (Some(5), 3),
            "with whole rows only, the last shown at the bottom"
        );
    }

    #[test]
    fn a_forward_step_with_the_selection_scrolled_above_the_view_lands_where_lautoscroll_puts_it() {
        let mut rows = in_box(8, EXACT);
        rows.scroll_by(3);
        assert_eq!((rows.selected(), rows.first()), (Some(0), 3));
        rows.step_wrapping(true);
        assert_eq!(
            (rows.selected(), rows.first()),
            (Some(1), 1),
            "no LRect scroll (@0x9d7e8), then LAutoScroll to the top (@0x9d8f1)"
        );
    }

    #[test]
    fn a_forward_step_with_the_selection_scrolled_below_the_view_lands_where_lautoscroll_puts_it() {
        let mut rows = in_box(8, EXACT);
        rows.select(6);
        rows.scroll_by(-4);
        assert_eq!((rows.selected(), rows.first()), (Some(6), 0));
        rows.step_wrapping(true);
        assert_eq!(
            (rows.selected(), rows.first()),
            (Some(7), 5),
            "down a row (@0x9d7fd), then LAutoScroll as far as the scroll goes"
        );
    }

    #[test]
    fn a_back_step_with_the_selection_scrolled_above_the_view_lands_where_lautoscroll_puts_it() {
        let mut rows = in_box(8, EXACT);
        rows.select(1);
        rows.scroll_by(4);
        assert_eq!((rows.selected(), rows.first()), (Some(1), 4));
        rows.step_wrapping(false);
        assert_eq!(
            (rows.selected(), rows.first()),
            (Some(0), 0),
            "up a row (@0x9da23), then LAutoScroll to the top"
        );
    }

    #[test]
    fn a_back_step_with_the_selection_scrolled_below_the_view_lands_where_lautoscroll_puts_it() {
        let mut rows = in_box(8, EXACT);
        rows.select(6);
        rows.scroll_by(-4);
        rows.step_wrapping(false);
        assert_eq!(
            (rows.selected(), rows.first()),
            (Some(5), 5),
            "no LRect scroll (@0x9da0e), then LAutoScroll to the top, held to the end"
        );
        let mut wrapped = list(8);
        wrapped.scroll_by(5);
        assert_eq!((wrapped.selected(), wrapped.first()), (Some(0), 5));
        wrapped.step_wrapping(false);
        assert_eq!(
            (wrapped.selected(), wrapped.first()),
            (Some(7), 4),
            "the guard's row up stays: the wrap to row 7 is shown from 4 (@0x9da23)"
        );
        let mut next = in_box(8, EXACT);
        next.select(3);
        next.scroll_by(-1);
        assert_eq!(next.first(), 0);
        next.step_wrapping(false);
        assert_eq!(
            (next.selected(), next.first()),
            (Some(2), 0),
            "the row above the one just below is shown"
        );
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
    fn step_wrapping_moves_forward_and_back_and_wraps() {
        let mut rows = list(3);
        assert!(rows.step_wrapping(true));
        assert_eq!(rows.selected(), Some(1));
        assert!(rows.step_wrapping(true));
        assert_eq!(rows.selected(), Some(2));
        assert!(rows.step_wrapping(true), "wraps to the first");
        assert_eq!(rows.selected(), Some(0));
        assert!(rows.step_wrapping(false), "wraps to the last");
        assert_eq!(rows.selected(), Some(2));
        assert!(rows.step_wrapping(false));
        assert_eq!(rows.selected(), Some(1));
        let mut long = list(8);
        assert!(long.step_wrapping(false));
        assert_eq!(long.selected(), Some(7));
        assert_eq!(long.first(), 8 - long.visible(), "the last row shown");
        assert!(long.step_wrapping(true));
        assert_eq!((long.selected(), long.first()), (Some(0), 0));
    }

    #[test]
    fn step_wrapping_with_one_row_or_none_changes_nothing() {
        let mut empty = list(0);
        assert!(!empty.step_wrapping(true));
        assert!(!empty.step_wrapping(false));
        assert_eq!(empty.selected(), None);
        let mut one = list(1);
        assert!(!one.step_wrapping(true));
        assert!(!one.step_wrapping(false));
        assert_eq!(one.selected(), Some(0));
    }

    #[test]
    fn step_wrapping_from_no_selection_takes_the_first_forward_and_the_last_back() {
        let mut rows = list(8);
        rows.select(7);
        rows.clear_selection();
        assert_eq!(rows.selected(), None);
        assert_eq!(rows.first(), 5, "clearing leaves the scroll");
        assert!(rows.step_wrapping(true), "@0x9d839");
        assert_eq!((rows.selected(), rows.first()), (Some(0), 0), "scrolled");
        rows.clear_selection();
        assert!(rows.step_wrapping(false), "@0x9da60");
        assert_eq!((rows.selected(), rows.first()), (Some(7), 5), "scrolled");
        let mut one = list(1);
        one.clear_selection();
        assert!(one.step_wrapping(false), "one row is still a change");
        assert_eq!(one.selected(), Some(0));
    }

    #[test]
    fn steps_wrap_over_the_extent_past_the_cells() {
        let mut rows = list(3).with_extent(5);
        rows.select(2);
        assert!(rows.step_wrapping(true));
        assert_eq!(
            (rows.selected(), rows.first()),
            (Some(3), 0),
            "past the rows"
        );
        assert!(rows.step_wrapping(true));
        assert_eq!(rows.selected(), Some(4));
        assert!(rows.step_wrapping(true));
        assert_eq!(rows.selected(), Some(0), "wrapped over 5");
        assert!(rows.step_wrapping(false));
        assert_eq!(rows.selected(), Some(4), "and back");
        rows.select(9);
        assert_eq!(rows.selected(), Some(4), "held to the extent");
        assert!(
            list(3).with_extent(1).step_wrapping(true),
            "never fewer than the rows"
        );
    }

    #[test]
    fn a_selection_past_the_cells_highlights_nothing_and_reads_back_as_none() {
        let mut rows = list(3).with_extent(5);
        rows.select(4);
        assert_eq!(drawn(&rows).len(), 3, "the three rows' text alone");
        let mut stepped = tall(10).with_extent(12);
        stepped.select(9);
        stepped.scroll_by(-3);
        stepped.step_wrapping(true);
        assert_eq!(
            (stepped.selected(), stepped.first()),
            (Some(10), 1),
            "the guard's row down, then no LAutoScroll for a cell past the rows"
        );
        let mut long = tall(10).with_extent(12);
        long.select(11);
        assert_eq!(long.first(), 0, "nothing to scroll to");
        long.input(&press(Point::new(100.0, 100.0)));
        assert_eq!(
            long.input(&release(Point::new(100.0, 100.0))),
            ListPress::Released { changed: true },
            "read back over the rows (@0x9dca9-0x9dcd6)"
        );
        assert_eq!(long.selected(), None);
    }

    #[test]
    fn with_nothing_selected_no_row_is_highlighted() {
        let mut rows = list(2);
        rows.clear_selection();
        assert_eq!(drawn(&rows).len(), 2, "the two rows' text alone");
    }

    /// Each text drawn, with where and in what colour.
    fn coloured(rows: &RowList) -> Vec<(String, Point, Color)> {
        drawn(rows)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } => Some((text, origin, color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_second_column_draws_each_row_s_cell_at_its_offset() {
        let mut rows = list(2).with_column(40.0, vec!["A".into(), "B".into()]);
        rows.set_cell(1, "C");
        assert_eq!(rows.cell(1), Some("C"));
        assert_eq!(rows.cell(2), None);
        assert_eq!(
            texts(&rows),
            [
                ("Row 0".to_owned(), Point::new(10.0, 20.0)),
                ("A".to_owned(), Point::new(50.0, 20.0)),
                ("Row 1".to_owned(), Point::new(10.0, 32.0)),
                ("C".to_owned(), Point::new(50.0, 32.0)),
            ]
        );
        rows.set_row(0, "Renamed");
        assert_eq!(rows.rows()[0], "Renamed");
    }

    #[test]
    fn a_partly_shown_row_s_cell_is_clipped_to_the_view_too() {
        let rows = list(4).with_column(40.0, (0..4).map(|n| format!("c{n}")).collect());
        let clipped = drawn(&rows)
            .into_iter()
            .filter(|command| matches!(command, DrawCommand::Clipped { .. }))
            .count();
        assert_eq!(clipped, 2, "the fourth row's text and its cell");
    }

    #[test]
    fn a_heading_is_drawn_dim_never_selected_and_stepped_over() {
        let mut rows = list(3).with_headings(&[0, 2]);
        assert_eq!(rows.selected(), Some(1), "the first row that is no heading");
        assert!(rows.is_heading(0));
        assert!(!rows.is_heading(1));
        assert!(!rows.step_wrapping(true), "the one row left to select");
        assert_eq!(rows.selected(), Some(1));
        rows.input(&press(Point::new(20.0, 21.0)));
        rows.input(&release(Point::new(20.0, 21.0)));
        assert_eq!(
            rows.selected(),
            None,
            "a press on a heading selects nothing"
        );
        assert_eq!(coloured(&rows)[0].2, HEADING_COLOR);
        assert_eq!(coloured(&rows)[1].2, ROW_COLOR);
        let mut all = list(2).with_headings(&[0, 1]);
        assert_eq!(all.selected(), None);
        assert!(!all.step_wrapping(true), "nothing to select");
        assert_eq!(all.selected(), None);
    }

    #[test]
    fn stepping_wraps_over_headings_both_ways() {
        let mut rows = tall(5).with_headings(&[0, 3]);
        assert_eq!(rows.selected(), Some(1));
        assert!(rows.step_wrapping(true));
        assert_eq!(rows.selected(), Some(2));
        assert!(rows.step_wrapping(true));
        assert_eq!(rows.selected(), Some(4), "past the heading at 3");
        assert!(rows.step_wrapping(true));
        assert_eq!(rows.selected(), Some(1), "round, past the heading at 0");
        assert!(rows.step_wrapping(false));
        assert_eq!(rows.selected(), Some(4));
    }

    #[test]
    fn a_marked_row_draws_its_text_and_cell_in_the_marked_colour() {
        let mut rows = list(2).with_column(40.0, vec!["A".into(), "B".into()]);
        rows.set_marked(1, true);
        assert!(rows.is_marked(1));
        let colours: Vec<Color> = coloured(&rows).into_iter().map(|(.., c)| c).collect();
        assert_eq!(colours, [ROW_COLOR, ROW_COLOR, MARKED_COLOR, MARKED_COLOR]);
        rows.set_marked(1, false);
        assert!(!rows.is_marked(1));
        rows.set_marked(9, true);
        assert!(!rows.is_marked(9), "no such row");
    }

    #[test]
    fn an_empty_list_selects_nothing() {
        let mut rows = list(0);
        assert_eq!(rows.selected(), None);
        assert!(!rows.step_wrapping(true));
        rows.select(0);
        assert_eq!(rows.selected(), None);
        assert!(drawn(&rows).is_empty());
    }
}
