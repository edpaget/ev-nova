//! The shipyard's and the hire screen's double-click: a second press on
//! the same cell position soon after the first opens Shipyard Info, as
//! `_ShipyardFilter` reads it (@0x60cd7-0x60d12).
//!
//! The filter keeps the last pressed position (`*0xdecb4`, a visible
//! position 0-19, `column + 4·row`, or -1 for an empty cell) and that
//! press's `EventRecord.when` (`*0xded80`). A press is a double-click when
//! its position is the last one, is not -1, and comes at most
//! [`DOUBLE_CLICK_TICKS`] Mac ticks after it (`cmpl $0xf; ja`). Every
//! counted press then records itself (@0x60d59-0x60d71), the double-click
//! included. The memory is of the position, not of the ship there: a
//! press after a scroll, at the same place on screen, opens Info on the
//! ship now shown.

use std::time::Duration;

use crate::time::sixtieths;

/// The most Mac ticks (1/60 s) between two presses on one position that
/// still make a double-click: `_ShipyardFilter`'s `cmpl $0xf`
/// (@0x60d00).
pub const DOUBLE_CLICK_TICKS: u128 = 15;

/// The last cell position pressed, and the time since.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellClicks {
    /// The visible position last pressed, or `None` for an empty cell or
    /// none yet.
    last: Option<usize>,
    /// The time since that press.
    since: Duration,
}

impl CellClicks {
    /// Lets `dt` pass.
    pub fn advance(&mut self, dt: Duration) {
        self.since = self.since.saturating_add(dt);
    }

    /// A press on the visible cell `position` (`None` for an empty cell),
    /// recorded as the last; says whether it makes a double-click.
    pub fn press(&mut self, position: Option<usize>) -> bool {
        let double = position.is_some()
            && position == self.last
            && sixtieths(self.since) <= DOUBLE_CLICK_TICKS;
        self.last = position;
        self.since = Duration::ZERO;
        double
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::mission::screens::mac_ticks;

    use super::*;

    fn pressed_at(position: usize) -> CellClicks {
        let mut clicks = CellClicks::default();
        assert!(!clicks.press(Some(position)), "the first press is single");
        clicks
    }

    #[test]
    fn a_second_press_on_one_position_within_15_mac_ticks_is_a_double_click() {
        let mut clicks = pressed_at(3);
        clicks.advance(mac_ticks(15));
        assert!(clicks.press(Some(3)));
    }

    #[test]
    fn a_second_press_16_mac_ticks_later_is_single() {
        let mut clicks = pressed_at(3);
        clicks.advance(mac_ticks(16));
        assert!(!clicks.press(Some(3)));
    }

    #[test]
    fn the_gap_adds_up_over_advances() {
        let mut clicks = pressed_at(3);
        clicks.advance(mac_ticks(8));
        clicks.advance(mac_ticks(8));
        assert!(!clicks.press(Some(3)));
    }

    #[test]
    fn a_press_on_another_position_is_single() {
        let mut clicks = pressed_at(3);
        assert!(!clicks.press(Some(4)));
        assert!(clicks.press(Some(4)), "and it is the last now");
    }

    #[test]
    fn an_empty_cell_is_never_a_double_click_and_forgets_the_last() {
        let mut clicks = CellClicks::default();
        assert!(!clicks.press(None));
        assert!(!clicks.press(None));
        let mut clicks = pressed_at(3);
        assert!(!clicks.press(None));
        assert!(!clicks.press(Some(3)));
    }

    #[test]
    fn a_double_click_records_itself() {
        let mut clicks = pressed_at(3);
        assert!(clicks.press(Some(3)));
        clicks.advance(mac_ticks(1));
        assert!(clicks.press(Some(3)), "a third press is double again");
    }

    #[test]
    fn a_press_restarts_the_gap() {
        let mut clicks = pressed_at(3);
        clicks.advance(mac_ticks(10));
        assert!(!clicks.press(Some(4)));
        clicks.advance(mac_ticks(10));
        assert!(!clicks.press(Some(3)), "the gap is from the last press");
    }
}
