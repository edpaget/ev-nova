//! The shipyard, the hire screen and their info panel with the pad
//! (rdm:phase/key-bindings/phase-9-gamepad-spaceport).
//!
//! - **Targets.** The grid's listed cells shown stand in for the grid's
//!   item as the dialog's cells (`Dialog::set_pad_cells`); an empty
//!   position is none. Then the dialog's buttons and arrows: Buy Ship (or
//!   Hire Escort), Info, Done and the scroll arrows. The description
//!   (item 6), the picture (8) and the info box (9) are left out, as a
//!   click on them does nothing. The pad's focus starts on Done, the
//!   default.
//! - **South on a cell** presses it as a pointer press does
//!   ([`ShipyardGrid::press`]), on South's press: a second South on the
//!   same cell within `DOUBLE_CLICK_TICKS` opens Info, as a double-click
//!   does (`_ShipyardFilter` @0x60cfd).
//! - **South presses the focused control.** The keyboard's Space still
//!   opens Info whatever the focus (`_ShipyardFilter` @0x60526), and on
//!   the info panel it is still Done whatever the focus (`_ShipDescFilter`
//!   @0x511a6); the pad's South presses the focused control in both, as
//!   its focus shows where it is.
//! - **The cells follow the grid**: they are given afresh at the end of
//!   every input and tick, as it opens and after each new listing.
//! - **The surroundings** reach the dialog, the name prompt and the info
//!   panel (open, or opened later) and the offers, and nothing else.

use crate::geometry::Bounds;
use crate::input::Input;
use crate::ui::dialog::{Dialog, DialogEvent};
use crate::ui::pad_focus::Told;

use super::super::shipyard_grid::ShipyardGrid;
use super::{
    DESCRIPTION_ITEM, GRID_ITEM, INFO_BOX_ITEM, PANEL_DONE_ITEM, PICTURE_ITEM, Panel,
    ShipyardScreen,
};

/// The listed cells of `count` shown in `dialog`'s grid, by list index,
/// with where each is.
pub(in crate::spaceport) fn grid_cells(
    grid: &ShipyardGrid,
    dialog: Option<&Dialog>,
    count: usize,
) -> Vec<(usize, Bounds)> {
    dialog
        .and_then(|dialog| dialog.item_bounds(GRID_ITEM))
        .map(|area| grid.cells().shown(area, count))
        .unwrap_or_default()
}

/// Gives `dialog` `cells` in place of the grid's item, and leaves the
/// dead items out.
pub(in crate::spaceport) fn sync(dialog: &mut Dialog, cells: Vec<(usize, Bounds)>) {
    dialog.set_pad_cells(GRID_ITEM, cells.into_iter().map(|(_, rect)| rect).collect());
    dialog.set_pad_left_out(vec![DESCRIPTION_ITEM, PICTURE_ITEM, INFO_BOX_ITEM]);
}

impl Panel {
    /// It told the surroundings `told` keeps, as it opens.
    pub(in crate::spaceport) fn told(mut self, told: Told) -> Self {
        if let Some(input) = told.input() {
            self.dialog.input(&input);
        }
        self
    }

    /// A pad button's input: its dialog's pad focus presses Done as a
    /// click does, and East closes it as Escape does. Whether it closed.
    pub(super) fn pad_input(&mut self, input: &Input) -> bool {
        self.dialog.input(input) == Some(DialogEvent::Item(PANEL_DONE_ITEM))
    }

    /// Tells it the surroundings `input`.
    pub(in crate::spaceport) fn tell(&mut self, input: &Input) {
        self.dialog.input(input);
    }
}

impl ShipyardScreen {
    /// Tells every screen the shipyard holds the surroundings `input`;
    /// whether it was the surroundings. They are no player input.
    pub(super) fn tell_held(&mut self, input: &Input) -> bool {
        if !self.told.keep(input) {
            return false;
        }
        self.naming.follow_surroundings(input);
        if let Some(offering) = &mut self.offering {
            offering.input(input, &mut self.sounds);
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
        grid_cells(&self.grid, self.dialog(), self.shipyard.rows.len())
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
