//! The grid's selection as `_ShipyardFilter` keeps it, once for the
//! shipyard and the hire screen, which the original runs through the one
//! filter (the hire flag `*0xdeda0` picks its mode).
//!
//! [`ShipyardGrid`] holds the [`CellGrid`], the last press
//! ([`CellClicks`]) and the reading of
//! [`RuleKey::GridSelection`], and
//! decides what opening the screen, a press on a cell and a scroll arrow
//! do to the selection. Each screen then maps the outcome to its own
//! effects: its buttons, picture, description and Info panel.
//!
//! By the engine:
//!
//! - it opens with nothing selected (`_DoShipyardDialog`
//!   @0x5e6d8-0x5e705);
//! - a press on an empty cell selects nothing (`_ShipyardFilter`
//!   @0x60a5d-0x60a73) and goes no further;
//! - a scroll that moves selects nothing, as items 12 and 13 set the cell
//!   and the ship to -1 (`_DoShipyardDialog` @0x5f580-0x5f5be,
//!   @0x5f60b-0x5f670).
//!
//! By the other reading it opens on the first cell and keeps a cell
//! selected: an empty cell's press is left to the dialog, as the port
//! ignored it, and a scroll keeps the selection. By both, a listed cell
//! pressed is selected, and a second press on its visible position within
//! [`DOUBLE_CLICK_TICKS`](super::grid_clicks::DOUBLE_CLICK_TICKS) is a
//! double-click, which opens Info (@0x60cd7-0x60d71).

use std::time::Duration;

use nova_sim::{RuleKey, RuleSource, Rulebook};

use super::grid::CellGrid;
use super::grid_clicks::CellClicks;
use crate::input::Key;

/// What a press on the grid did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressOutcome {
    /// A listed cell was pressed and is selected; `double` when it makes
    /// a double-click, which opens Info.
    Selected {
        /// Whether the press is a double-click.
        double: bool,
    },
    /// An empty cell was pressed, and nothing is selected (by the engine).
    Cleared,
    /// An empty cell was pressed and the grid did not take it: it is left
    /// to the dialog (by the other reading).
    FallThrough,
}

/// The shipyard's and the hire screen's grid: its cells, its last press
/// and the reading of
/// [`RuleKey::GridSelection`] it
/// follows, fixed for the visit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipyardGrid {
    cells: CellGrid,
    clicks: CellClicks,
    /// Whether the selection follows `_ShipyardFilter`'s, which can be
    /// none (the engine's reading).
    follows_filter: bool,
}

impl ShipyardGrid {
    /// The grid as it opens on `count` cells, by `rules`' reading of
    /// [`RuleKey::GridSelection`]:
    /// nothing selected by the engine, the first cell by the other.
    #[must_use]
    pub fn open(rules: &Rulebook, count: usize) -> Self {
        let mut grid = Self {
            cells: CellGrid::default(),
            clicks: CellClicks::default(),
            follows_filter: rules.source_for(RuleKey::GridSelection) == RuleSource::Engine,
        };
        if grid.follows_filter {
            grid.cells.clear();
        } else {
            grid.cells.select(0, count);
        }
        grid
    }

    /// The cells, their selection and scroll.
    #[must_use]
    pub fn cells(&self) -> &CellGrid {
        &self.cells
    }

    /// Selects cell `index` of `count`, or the last when there is no such
    /// cell, and scrolls to show it ([`CellGrid::select`]).
    pub fn select(&mut self, index: usize, count: usize) {
        self.cells.select(index, count);
    }

    /// Moves the selection for an arrow `key` among `count` cells, and
    /// says whether it was an arrow ([`CellGrid::arrow`]).
    pub fn arrow(&mut self, key: Key, count: usize) -> bool {
        self.cells.arrow(key, count)
    }

    /// Lets `dt` pass since the last press.
    pub fn advance(&mut self, dt: Duration) {
        self.clicks.advance(dt);
    }

    /// A press on list index `index` of `count` cells, listed or not. The
    /// press is remembered by its visible position, an empty cell
    /// forgetting the last, by either reading.
    pub fn press(&mut self, index: usize, count: usize) -> PressOutcome {
        let listed = index < count;
        let position = self.cells.visible_position(index);
        let double = self.clicks.press(listed.then_some(position));
        if listed {
            self.cells.select(index, count);
            PressOutcome::Selected { double }
        } else if self.follows_filter {
            self.cells.clear();
            PressOutcome::Cleared
        } else {
            PressOutcome::FallThrough
        }
    }

    /// Scrolls the grid of `count` cells a row down, or up, never past
    /// either end, and says whether that cleared the selection: by the
    /// engine, when it moved. The last press is kept.
    pub fn scroll(&mut self, down: bool, count: usize) -> bool {
        let cleared = self.cells.scroll(down, count) && self.follows_filter;
        if cleared {
            self.cells.clear();
        }
        cleared
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::mission::screens::mac_ticks;

    use super::*;

    /// Thirty ships: eight rows, three more than the grid shows.
    const LISTED: usize = 30;

    fn engine() -> Rulebook {
        Rulebook::default()
    }

    fn other() -> Rulebook {
        Rulebook::default().with_override(RuleKey::GridSelection, RuleSource::Bible)
    }

    fn both() -> [Rulebook; 2] {
        [engine(), other()]
    }

    #[test]
    fn by_the_engine_it_opens_with_nothing_selected() {
        let grid = ShipyardGrid::open(&engine(), LISTED);
        assert_eq!(grid.cells().selected(LISTED), None);
        assert!(!grid.cells().highlighted());
        assert_eq!(grid.cells().top_row(), 0);
    }

    #[test]
    fn by_the_other_reading_it_opens_on_the_first_cell() {
        let grid = ShipyardGrid::open(&other(), LISTED);
        assert_eq!(grid.cells().selected(LISTED), Some(0));
    }

    #[test]
    fn a_listed_cell_pressed_is_selected_by_either_reading() {
        for rules in both() {
            let mut grid = ShipyardGrid::open(&rules, LISTED);
            assert_eq!(
                grid.press(5, LISTED),
                PressOutcome::Selected { double: false }
            );
            assert_eq!(grid.cells().selected(LISTED), Some(5));
        }
    }

    #[test]
    fn by_the_engine_an_empty_cell_pressed_clears_the_selection() {
        for empty in [3, 4] {
            let mut grid = ShipyardGrid::open(&engine(), 3);
            grid.press(1, 3);
            assert_eq!(grid.press(empty, 3), PressOutcome::Cleared, "cell {empty}");
            assert_eq!(grid.cells().selected(3), None);
        }
    }

    #[test]
    fn the_last_listed_cell_pressed_is_selected() {
        let mut grid = ShipyardGrid::open(&engine(), 3);
        assert_eq!(grid.press(2, 3), PressOutcome::Selected { double: false });
        assert_eq!(grid.cells().selected(3), Some(2));
    }

    #[test]
    fn by_the_other_reading_an_empty_cell_pressed_falls_through_and_keeps_the_selection() {
        let mut grid = ShipyardGrid::open(&other(), 3);
        grid.press(1, 3);
        assert_eq!(grid.press(3, 3), PressOutcome::FallThrough);
        assert_eq!(grid.cells().selected(3), Some(1));
    }

    #[test]
    fn a_second_press_within_15_mac_ticks_is_a_double_click_by_either_reading() {
        for rules in both() {
            let mut grid = ShipyardGrid::open(&rules, LISTED);
            grid.press(5, LISTED);
            grid.advance(mac_ticks(15));
            assert_eq!(
                grid.press(5, LISTED),
                PressOutcome::Selected { double: true }
            );
            grid.advance(mac_ticks(16));
            assert_eq!(
                grid.press(5, LISTED),
                PressOutcome::Selected { double: false }
            );
        }
    }

    #[test]
    fn an_empty_cell_pressed_forgets_the_last_by_either_reading() {
        for rules in both() {
            let mut grid = ShipyardGrid::open(&rules, 3);
            grid.press(1, 3);
            grid.press(3, 3);
            assert_eq!(grid.press(1, 3), PressOutcome::Selected { double: false });
        }
    }

    #[test]
    fn the_press_is_remembered_by_its_visible_position() {
        let mut grid = ShipyardGrid::open(&engine(), LISTED);
        grid.press(5, LISTED);
        assert!(grid.scroll(true, LISTED));
        assert_eq!(
            grid.press(9, LISTED),
            PressOutcome::Selected { double: true },
            "cell 9 is shown where cell 5 was"
        );
    }

    #[test]
    fn by_the_engine_a_scroll_that_moves_clears_the_selection() {
        let mut grid = ShipyardGrid::open(&engine(), LISTED);
        grid.press(5, LISTED);
        assert!(grid.scroll(true, LISTED));
        assert_eq!(grid.cells().top_row(), 1);
        assert_eq!(grid.cells().selected(LISTED), None);
    }

    #[test]
    fn a_scroll_that_does_not_move_keeps_the_selection() {
        let mut grid = ShipyardGrid::open(&engine(), LISTED);
        grid.press(5, LISTED);
        assert!(!grid.scroll(false, LISTED));
        assert_eq!(grid.cells().selected(LISTED), Some(5));
    }

    #[test]
    fn by_the_other_reading_a_scroll_keeps_the_selection() {
        let mut grid = ShipyardGrid::open(&other(), LISTED);
        grid.press(5, LISTED);
        assert!(!grid.scroll(true, LISTED));
        assert_eq!(grid.cells().top_row(), 1);
        assert_eq!(grid.cells().selected(LISTED), Some(5));
    }

    #[test]
    fn select_and_arrow_move_the_cells() {
        let mut grid = ShipyardGrid::open(&engine(), LISTED);
        grid.select(6, LISTED);
        assert_eq!(grid.cells().selected(LISTED), Some(6));
        assert!(grid.arrow(Key::Right, LISTED));
        assert_eq!(grid.cells().selected(LISTED), Some(7));
        assert!(!grid.arrow(Key::Char('x'), LISTED));
    }
}
