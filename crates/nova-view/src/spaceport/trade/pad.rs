//! The trade center with the pad (rdm:phase/key-bindings/phase-9-gamepad-spaceport).
//!
//! - **Targets** are its dialog's: Done, Buy, Sell and each slot showing a
//!   good. The header (item 3), the cargo summary (12), the event line
//!   (15) and the empty slots are left out, as a click on them does
//!   nothing. The pad's focus starts on Done, the default.
//! - **The shoulders** under [`FocusRing::Off`](crate::ui::dialog::FocusRing::Off)
//!   step the selection as Tab and Shift-Tab do (`_TradeFilter` @0x4e184);
//!   under the Tab ring they move the focus. The left stick steps it as the
//!   arrows do.
//! - **West held** is Option ([`TradeCommand::MaxLot`](crate::keymap::TradeCommand::MaxLot)
//!   on the pad): South on Buy or Sell with West held asks for a count, as
//!   an Option-click does (`_DoTradeDialog` @0x5e211, @0x5e45c).
//! - **The surroundings** reach its dialog, the quantity dialog (open, or
//!   opened later) and the offers, and nothing else.

use crate::input::Input;

use super::{EVENT_ITEM, FIRST_ROW_ITEM, HEADER_ITEM, ROWS, SUMMARY_ITEM, TradeScreen, positions};

impl TradeScreen {
    /// Tells every screen the trade center holds the surroundings `input`;
    /// whether it was the surroundings. They are no player input.
    pub(super) fn tell_held(&mut self, input: &Input) -> bool {
        if !self.quantity.follow_surroundings(input) {
            return false;
        }
        if let Some(offering) = &mut self.offering {
            offering.input(input, &mut self.sounds);
        }
        if let Ok(laid) = &mut self.laid {
            laid.dialog.input(input);
        }
        true
    }

    /// Takes the dead items out of the pad's targets: the header, the
    /// summary, the event line and each slot showing no good. Run as the
    /// list scrolls or changes ([`TradeScreen::select`]).
    pub(super) fn sync_pad_targets(&mut self) {
        let positions = positions(&self.market);
        let empty = (0..ROWS)
            .filter(|&slot| !matches!(positions.get(self.top + slot), Some(Some(_))))
            .map(|slot| FIRST_ROW_ITEM + slot);
        let left_out = [HEADER_ITEM, SUMMARY_ITEM, EVENT_ITEM]
            .into_iter()
            .chain(empty)
            .collect();
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_pad_left_out(left_out);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
