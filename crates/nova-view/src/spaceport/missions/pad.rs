//! The mission computer and the mission info screen with the pad
//! (rdm:phase/key-bindings/phase-9-gamepad-spaceport).
//!
//! - **Targets.** Each list's rows shown, and its bar's two arrows while
//!   the bar shows, stand in for the list's item as the dialog's cells
//!   ([`Dialog::set_pad_cells`]); then the dialog's buttons. The bar's
//!   page areas and thumb are left out, as a press can't drag, and so is
//!   the computer's item 3, the bar's column, whose click does nothing
//!   over a list that fits. The pad's focus starts on the default: Leave,
//!   or the info screen's Done.
//! - **South on a cell** presses it as a click does
//!   ([`RowList::press_cell`]): a row is selected and its text shown, an
//!   arrow scrolls a row.
//! - **The cells follow the list**: they are given afresh at the end of
//!   every input and tick and after each new listing, so a scroll moves
//!   them. A focused row scrolled away is no target: it draws no
//!   highlight, and the next move lands on the default.
//! - **The shoulders**: under the Off ring the right one steps the list,
//!   as Tab does (`_MissionFilter` @0x9f8cc, `_MissionInfoFilter`
//!   @0x9d778); under the Tab ring it moves the focus. Neither filter reads
//!   Shift, so there is no Tab Back command, and the left shoulder moves
//!   the focus back under either ring. The left stick steps the list as
//!   the arrows do, and the d-pad never scrolls.

use crate::ui::dialog::Dialog;
use crate::ui::row_list::{ListPress, RowList};

use super::{BAR_ITEM, INFO_LIST_ITEM, LIST_ITEM, MissionComputerScreen, MissionInfoScreen};

/// Gives `dialog` the cells of `rows` in place of item `item`.
fn sync(dialog: &mut Dialog, rows: &RowList, item: usize) {
    let cells = rows.pad_cells().into_iter().map(|(_, rect)| rect).collect();
    dialog.set_pad_cells(item, cells);
}

/// Presses cell `cell` of `rows` as a click does; whether the selection
/// changed.
pub(super) fn press_cell(rows: &mut RowList, cell: usize) -> bool {
    let Some((cell, _)) = rows.pad_cells().get(cell).copied() else {
        return false;
    };
    rows.press_cell(cell) == (ListPress::Released { changed: true })
}

impl MissionComputerScreen {
    /// Gives the dialog the list's cells as they stand now, and leaves
    /// the bar's column out.
    pub(super) fn sync_pad_cells(&mut self) {
        if let Ok(laid) = &mut self.laid {
            sync(&mut laid.dialog, &laid.rows, LIST_ITEM);
            laid.dialog.set_pad_left_out(vec![BAR_ITEM]);
        }
    }
}

impl MissionInfoScreen {
    /// Gives the dialog the list's cells as they stand now.
    pub(super) fn sync_pad_cells(&mut self) {
        if let Ok(laid) = &mut self.laid {
            sync(&mut laid.dialog, &laid.rows, INFO_LIST_ITEM);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
