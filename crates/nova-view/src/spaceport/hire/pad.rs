//! The hire screen with the pad (rdm:phase/key-bindings/phase-9-gamepad-spaceport):
//! the shipyard's rules (see the `shipyard::pad` module), over the ships
//! for hire, Hire Escort in Buy Ship's place.

use crate::geometry::Bounds;
use crate::input::Input;

use super::super::shipyard::pad::{grid_cells, sync};
use super::HireScreen;

impl HireScreen {
    /// Tells its dialog and its info panel the surroundings `input`;
    /// whether it was the surroundings. They are no player input.
    pub(super) fn tell_held(&mut self, input: &Input) -> bool {
        if !self.told.keep(input) {
            return false;
        }
        if let Some(panel) = &mut self.panel {
            panel.tell(input);
        }
        if let Ok(laid) = &mut self.laid {
            laid.dialog.input(input);
        }
        true
    }

    fn pad_cells(&self) -> Vec<(usize, Bounds)> {
        grid_cells(&self.grid, self.dialog(), self.list.rows.len())
    }

    /// Gives the dialog the grid's cells as they stand now.
    pub(super) fn sync_pad_cells(&mut self) {
        let cells = self.pad_cells();
        if let Ok(laid) = &mut self.laid {
            sync(&mut laid.dialog, cells);
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
