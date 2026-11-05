//! The outfitter with the pad (rdm:phase/key-bindings/phase-9-gamepad-spaceport).
//!
//! - **Targets.** The grid's listed cells shown stand in for the grid's
//!   item as the dialog's cells (`Dialog::set_pad_cells`); an empty
//!   position is none. Then Done, Buy, Sell and the two scroll arrows.
//!   The description (item 6), the picture (8) and the info (9) are left
//!   out, as a click on them does nothing. The pad's focus starts on Done,
//!   the default.
//! - **South on a cell** presses it as a pointer press does
//!   ([`OutfitterScreen::press_cell`]), on South's press.
//! - **The cells follow the grid**: they are given afresh at the end of
//!   every input and tick and whenever the selection changes, so a scroll
//!   (the left stick, the arrows) moves them.
//! - **West held** is Option ([`OutfitterCommand::Count`](crate::keymap::OutfitterCommand::Count)
//!   on the pad): South on Buy or Sell with West held asks for a count,
//!   as an Option-click does (`_DoOutfitDialog` @0x5c0c0, @0x5c5e3).
//! - **The surroundings** reach its dialog, the quantity dialog and the
//!   alert (open, or opened later) and the offers, and nothing else.

use crate::input::Input;

use super::{DESCRIPTION_ITEM, GRID_ITEM, INFO_ITEM, OutfitterScreen, PICTURE_ITEM};

impl OutfitterScreen {
    /// Tells every screen the outfitter holds the surroundings `input`;
    /// whether it was the surroundings. They are no player input.
    pub(super) fn tell_held(&mut self, input: &Input) -> bool {
        if !self.quantity.follow_surroundings(input) {
            return false;
        }
        self.alert.follow_surroundings(input);
        if let Some(offering) = &mut self.offering {
            offering.input(input, &mut self.sounds);
        }
        if let Ok(laid) = &mut self.laid {
            laid.dialog.input(input);
        }
        true
    }

    /// The listed cells shown, by their index in the outfitter's rows,
    /// with where each is.
    fn pad_cells(&self) -> Vec<(usize, crate::geometry::Bounds)> {
        self.dialog()
            .and_then(|dialog| dialog.item_bounds(GRID_ITEM))
            .map(|grid| self.grid.shown(grid, self.outfitter.rows.len()))
            .unwrap_or_default()
    }

    /// Gives the dialog the grid's cells as they stand now, and leaves
    /// the dead items out.
    pub(super) fn sync_pad_cells(&mut self) {
        let cells = self.pad_cells().into_iter().map(|(_, rect)| rect).collect();
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_pad_cells(GRID_ITEM, cells);
            laid.dialog
                .set_pad_left_out(vec![DESCRIPTION_ITEM, PICTURE_ITEM, INFO_ITEM]);
        }
    }

    /// South pressed cell `cell` of the dialog's cells: presses its grid
    /// position as a pointer press does.
    pub(super) fn press_pad_cell(&mut self, cell: usize) {
        if let Some(&(index, _)) = self.pad_cells().get(cell) {
            self.press_cell(index);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
