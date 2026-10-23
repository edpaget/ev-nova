//! Where an arrow key moves the outfitter's, the shipyard's and the hire
//! screen's highlighted cell, by the reading of
//! [`RuleKey::GridArrows`]: one copy of the move for all three grids.
//!
//! `_OutfitFilter` (@0x58203-0x58440) and `_ShipyardFilter`
//! (@0x6057a-0x607b7, which also runs the hire screen) move the cell with
//! the same instructions, on the cell's visible position 0-19 and the
//! scroll, a multiple of 4. Page Up (Mac key 0x74) moves as Up and Page
//! Down (0x79) as Down, one row and not a page (@0x581b0-0x581dd,
//! @0x604ea-0x60554), by both readings. Home and End are not arrows. No
//! arrow plays a sound.

use nova_sim::{RuleKey, RuleSource, Rulebook};

use super::grid::{COLUMNS, CellGrid, GRID_ROWS};
use crate::input::Key;

/// How many cells the grid shows: the filters' 20 visible positions.
const SHOWN: usize = COLUMNS * GRID_ROWS;

/// Which reading of [`RuleKey::GridArrows`] an arrow follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowMoves {
    /// The engine's: the filters' moves, where Up on the list's first row
    /// and Down onto an empty cell stay.
    Filter,
    /// The other reading, the port's earlier moves: Up on the first row
    /// selects the first cell, and Down past the end the last listed.
    Clamped,
}

impl ArrowMoves {
    /// The reading of [`RuleKey::GridArrows`] that `rules` take.
    #[must_use]
    pub fn from_rules(rules: &Rulebook) -> Self {
        if rules.source_for(RuleKey::GridArrows) == RuleSource::Engine {
            Self::Filter
        } else {
            Self::Clamped
        }
    }
}

/// Where an arrow left the selection: the selected cell's index and the
/// grid's first row shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    /// The selected cell, by index into the cells.
    pub selected: usize,
    /// The grid's first row shown.
    pub top_row: usize,
}

/// The four directions, Page Up and Page Down taken as Up and Down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arrow {
    Left,
    Right,
    Up,
    Down,
}

impl Arrow {
    /// The direction of `key`, if it is an arrow: the filters take Page Up
    /// as Up and Page Down as Down (@0x582c4-0x58307).
    fn of(key: Key) -> Option<Self> {
        match key {
            Key::Left => Some(Self::Left),
            Key::Right => Some(Self::Right),
            Key::Up | Key::PageUp => Some(Self::Up),
            Key::Down | Key::PageDown => Some(Self::Down),
            _ => None,
        }
    }
}

/// Where an arrow `key` going down moves the selection of a grid of
/// `count` cells, with cell `selected` (highlighted or not) and first row
/// `top_row` shown, by `moves`; `None` when `key` is no arrow.
///
/// With no cell highlighted, by both readings, Right, Down and Page Down
/// pick the first cell shown, and Left, Up and Page Up the last, position
/// 19 (`sbb` / `andl $0x13`), or the last listed when fewer are shown,
/// without scrolling (@0x583bd-0x58440, @0x60734-0x607b7).
///
/// With a cell highlighted, by [`ArrowMoves::Filter`], see [`filter`]; by
/// [`ArrowMoves::Clamped`], the cell a column or a row on, clamped to the
/// list, and the grid scrolled the least that shows it.
#[must_use]
pub fn arrow_move(
    key: Key,
    highlighted: bool,
    selected: usize,
    top_row: usize,
    count: usize,
    moves: ArrowMoves,
) -> Option<Placed> {
    let arrow = Arrow::of(key)?;
    if !highlighted {
        let first = top_row * COLUMNS;
        let index = match arrow {
            Arrow::Right | Arrow::Down => first,
            Arrow::Left | Arrow::Up => first + SHOWN - 1,
        };
        return Some(shown(index, top_row, count));
    }
    Some(match moves {
        ArrowMoves::Filter => filter(arrow, selected, top_row, count),
        ArrowMoves::Clamped => {
            let index = match arrow {
                Arrow::Left => selected.saturating_sub(1),
                Arrow::Right => selected + 1,
                Arrow::Up => selected.saturating_sub(COLUMNS),
                Arrow::Down => selected + COLUMNS,
            };
            shown(index, top_row, count)
        }
    })
}

/// Cell `index` of `count`, or the last when there is no such cell, with
/// the grid from `top_row` scrolled the least that shows it, as
/// [`CellGrid::select`] places it.
fn shown(index: usize, top_row: usize, count: usize) -> Placed {
    let selected = index.min(count.saturating_sub(1));
    let row = selected / COLUMNS;
    let top_row = top_row
        .min(CellGrid::last_top_row(count))
        .min(row)
        .max((row + 1).saturating_sub(GRID_ROWS));
    Placed { selected, top_row }
}

/// The filters' move of a highlighted cell (`_OutfitFilter`
/// @0x58227-0x583b8, `_ShipyardFilter` @0x6059e-0x6072f), on its visible
/// position `cell` 0-19 and the `scroll`, the first index shown. The grid
/// can scroll down while `scroll < count - 20`.
///
/// - Right: from cells 0-18, on one only onto a listed cell; from cell
///   19, a row's scroll and back 3, the next index (@0x58227-0x5828a).
/// - Left: from cells 1-19, back one; from cell 0, a row's scroll back
///   and on 3, the previous index (@0x5828f-0x582bf).
/// - Up: from cells 4-19, up a row; from cells 0-3, a row's scroll back,
///   so on the list's first row it stays (@0x582c4-0x582f5).
/// - Down: from cells 0-15, down a row only onto a listed cell; from
///   cells 16-19, a row's scroll (@0x582f7-0x58352).
///
/// Then, while the cell is empty and not the first, it steps back one
/// (@0x5835a-0x583b8). Only the Down scroll can land on an empty cell:
/// the Right scroll's new index, `scroll + 20`, is below `count` by its
/// bound.
///
/// The original's cell is always on screen. The port's highlighted cell
/// can sit off it (the outfitter after a scroll arrow, and the shipyard
/// and hire screens under
/// [`RuleKey::GridSelection`](nova_sim::RuleKey::GridSelection)'s other
/// reading), so the grid is first scrolled the least that shows it, as
/// [`CellGrid::select`] does; the original cannot reach that state.
fn filter(arrow: Arrow, selected: usize, top_row: usize, count: usize) -> Placed {
    let start = shown(selected, top_row, count);
    let mut scroll = start.top_row * COLUMNS;
    let mut cell = start.selected - scroll;
    let can_scroll_down = scroll + SHOWN < count;
    let listed = |index: usize| index < count;
    match arrow {
        Arrow::Right if cell < SHOWN - 1 => {
            if listed(scroll + cell + 1) {
                cell += 1;
            }
        }
        Arrow::Right => {
            if can_scroll_down {
                scroll += COLUMNS;
                cell -= COLUMNS - 1;
            }
        }
        Arrow::Left if cell > 0 => cell -= 1,
        Arrow::Left => {
            if scroll > 0 {
                scroll -= COLUMNS;
                cell += COLUMNS - 1;
            }
        }
        Arrow::Up if cell >= COLUMNS => cell -= COLUMNS,
        Arrow::Up => scroll = scroll.saturating_sub(COLUMNS),
        Arrow::Down if cell < SHOWN - COLUMNS => {
            if listed(scroll + cell + COLUMNS) {
                cell += COLUMNS;
            }
        }
        Arrow::Down => {
            if can_scroll_down {
                scroll += COLUMNS;
            }
        }
    }
    while !listed(scroll + cell) && cell > 0 {
        cell -= 1;
    }
    Placed {
        selected: scroll + cell,
        top_row: scroll / COLUMNS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The filters' move of highlighted index `selected` of `count`, with
    /// first row `top_row` shown: the new index and first row.
    fn by_filter(key: Key, selected: usize, top_row: usize, count: usize) -> (usize, usize) {
        moved(ArrowMoves::Filter, key, selected, top_row, count)
    }

    fn moved(
        moves: ArrowMoves,
        key: Key,
        selected: usize,
        top_row: usize,
        count: usize,
    ) -> (usize, usize) {
        let placed = arrow_move(key, true, selected, top_row, count, moves)
            .unwrap_or_else(|| panic!("{key:?} is an arrow"));
        (placed.selected, placed.top_row)
    }

    /// Every highlighted index shown at every first row the grid can take
    /// with `count` cells.
    fn on_screen(count: usize) -> Vec<(usize, usize)> {
        (0..=CellGrid::last_top_row(count))
            .flat_map(|top| {
                (top * COLUMNS..count.min(top * COLUMNS + SHOWN)).map(move |index| (index, top))
            })
            .collect()
    }

    const COUNTS: [usize; 7] = [1, 6, 10, 20, 21, 22, 30];

    #[test]
    fn by_the_filter_up_on_the_lists_first_row_stays() {
        for index in 0..4 {
            assert_eq!(by_filter(Key::Up, index, 0, 10), (index, 0), "{index}");
        }
        assert_eq!(by_filter(Key::Up, 5, 0, 10), (1, 0));
    }

    #[test]
    fn by_the_filter_up_on_the_top_row_shown_scrolls_a_row_and_keeps_the_cell() {
        // 30 listed, scrolled 2 rows: index 9 is visible position 1.
        assert_eq!(by_filter(Key::Up, 9, 2, 30), (5, 1));
    }

    #[test]
    fn by_the_filter_down_onto_an_empty_cell_stays() {
        // Worked examples 1, 3 and 4.
        assert_eq!(by_filter(Key::Down, 6, 0, 10), (6, 0));
        assert_eq!(by_filter(Key::Down, 26, 3, 30), (26, 3));
        assert_eq!(by_filter(Key::Down, 3, 0, 6), (3, 0));
        assert_eq!(by_filter(Key::Down, 1, 0, 6), (5, 0), "onto a listed one");
    }

    #[test]
    fn by_the_filter_down_from_the_bottom_row_scrolls_and_steps_back_to_the_last_listed() {
        // Worked example 2: 22 listed, index 19 at cell 19.
        assert_eq!(by_filter(Key::Down, 19, 0, 22), (21, 1));
        assert_eq!(
            by_filter(Key::Down, 16, 0, 22),
            (20, 1),
            "onto a listed one"
        );
        // At scroll 4 the same index is cell 15, and index 23 is empty.
        assert_eq!(by_filter(Key::Down, 19, 1, 22), (19, 1));
    }

    #[test]
    fn by_the_filter_down_from_the_last_row_with_no_scroll_left_stays() {
        assert_eq!(by_filter(Key::Down, 29, 3, 30), (29, 3));
        assert_eq!(by_filter(Key::Down, 17, 0, 20), (17, 0));
    }

    #[test]
    fn by_the_filter_right_and_left_cross_a_row_by_scrolling() {
        assert_eq!(by_filter(Key::Right, 19, 0, 21), (20, 1), "the next index");
        assert_eq!(
            by_filter(Key::Right, 19, 0, 20),
            (19, 0),
            "no row to scroll"
        );
        assert_eq!(
            by_filter(Key::Right, 9, 0, 10),
            (9, 0),
            "onto an empty cell"
        );
        assert_eq!(by_filter(Key::Right, 3, 0, 10), (4, 0));
        assert_eq!(by_filter(Key::Left, 4, 1, 30), (3, 0), "the previous index");
        assert_eq!(by_filter(Key::Left, 5, 1, 30), (4, 1));
        assert_eq!(by_filter(Key::Left, 0, 0, 30), (0, 0));
    }

    #[test]
    fn page_up_and_page_down_move_as_up_and_down_by_both_readings() {
        for moves in [ArrowMoves::Filter, ArrowMoves::Clamped] {
            for count in COUNTS {
                for (index, top) in on_screen(count) {
                    for (page, arrow) in [(Key::PageUp, Key::Up), (Key::PageDown, Key::Down)] {
                        assert_eq!(
                            moved(moves, page, index, top, count),
                            moved(moves, arrow, index, top, count),
                            "{moves:?} {page:?} from {index} at row {top} of {count}"
                        );
                    }
                }
            }
        }
        assert_eq!(by_filter(Key::PageDown, 1, 0, 10), (5, 0));
        assert_eq!(by_filter(Key::PageUp, 5, 0, 10), (1, 0));
    }

    #[test]
    fn left_and_right_move_alike_by_both_readings() {
        for count in COUNTS {
            for (index, top) in on_screen(count) {
                for key in [Key::Left, Key::Right] {
                    assert_eq!(
                        moved(ArrowMoves::Filter, key, index, top, count),
                        moved(ArrowMoves::Clamped, key, index, top, count),
                        "{key:?} from {index} at row {top} of {count}"
                    );
                }
            }
        }
    }

    #[test]
    fn by_the_clamped_reading_up_and_down_go_to_the_ends() {
        let clamped = |key, index, top, count| moved(ArrowMoves::Clamped, key, index, top, count);
        assert_eq!(clamped(Key::Up, 2, 0, 10), (0, 0));
        assert_eq!(clamped(Key::Down, 6, 0, 10), (9, 0));
        assert_eq!(clamped(Key::PageUp, 2, 0, 10), (0, 0));
        assert_eq!(clamped(Key::PageDown, 6, 0, 10), (9, 0));
        assert_eq!(clamped(Key::Down, 19, 1, 22), (21, 1));
        assert_eq!(clamped(Key::Down, 9, 0, 30), (13, 0));
    }

    #[test]
    fn with_no_cell_highlighted_an_arrow_picks_the_first_or_the_last_shown_by_both_readings() {
        for moves in [ArrowMoves::Filter, ArrowMoves::Clamped] {
            for (key, top, count, expected) in [
                (Key::Right, 0, 30, 0),
                (Key::Down, 0, 30, 0),
                (Key::PageDown, 0, 30, 0),
                (Key::Left, 0, 30, 19),
                (Key::Up, 0, 30, 19),
                (Key::PageUp, 0, 30, 19),
                (Key::PageDown, 1, 30, 4),
                (Key::PageUp, 1, 30, 23),
                (Key::PageUp, 0, 7, 6),
                (Key::Up, 1, 22, 21),
            ] {
                let placed = arrow_move(key, false, 12, top, count, moves);
                assert_eq!(
                    placed,
                    Some(Placed {
                        selected: expected,
                        top_row: top
                    }),
                    "{moves:?} {key:?} from row {top} of {count}"
                );
            }
        }
    }

    #[test]
    fn by_the_filter_an_off_screen_selection_is_brought_into_view_then_moved() {
        // Index 2 with the grid scrolled 3 rows of 30: shown at the top,
        // then Up on the first row stays and Down moves a row.
        assert_eq!(by_filter(Key::Up, 2, 3, 30), (2, 0));
        assert_eq!(by_filter(Key::Down, 2, 3, 30), (6, 0));
        // Index 29 with the grid at its top: shown on the last row.
        assert_eq!(by_filter(Key::Up, 29, 0, 30), (25, 3));
    }

    #[test]
    fn with_nothing_listed_an_arrow_selects_no_listed_cell() {
        for key in [Key::Left, Key::Right, Key::Up, Key::Down] {
            assert_eq!(by_filter(key, 0, 0, 0), (0, 0), "{key:?}");
        }
    }

    #[test]
    fn other_keys_are_not_arrows() {
        for key in [Key::Enter, Key::Char('j'), Key::Other] {
            for highlighted in [true, false] {
                assert_eq!(
                    arrow_move(key, highlighted, 0, 0, 10, ArrowMoves::Filter),
                    None,
                    "{key:?}"
                );
            }
        }
    }

    #[test]
    fn the_reading_comes_from_the_rules() {
        assert_eq!(
            ArrowMoves::from_rules(&Rulebook::default()),
            ArrowMoves::Filter
        );
        assert_eq!(
            ArrowMoves::from_rules(
                &Rulebook::default().with_override(RuleKey::GridArrows, RuleSource::Bible)
            ),
            ArrowMoves::Clamped
        );
    }
}
