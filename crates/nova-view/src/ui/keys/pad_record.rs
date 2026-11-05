//! The keys screen's tabs and its gamepad tab's recorder
//! (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs).
//!
//! - **Tabs.** The list shows one cell column, the shown tab's: the
//!   keyboard's or the gamepad's ([`Column`]). Their labels sit on the
//!   title row ([`TAB_AREAS`]), the shown one white. Left and Right on the
//!   keyboard (outside a recording), the pad's shoulders, or a click on a
//!   label switch them, keeping the selected command when the other tab
//!   lists it.
//! - **Cells.** The gamepad tab names each command's pad input
//!   ([`name_on_pad`]) for the family of the pad the screen was told of
//!   last, generic before any, so a player back on the keyboard still sees
//!   their pad's names. A command the pad leaves unbound reads
//!   [`PAD_UNBOUND`]. An axis's invert row reads "On" or "Off".
//! - **Recording.** Every pad button press records, South, East and the
//!   shoulders included; a release records nothing, so the South that
//!   started a recording doesn't. Select or Start pressed alone records
//!   nothing: it is a layer held, and the next button pressed while it is
//!   held records on that layer (the other layer button records the
//!   chord). A stick or trigger counts only as the button `nova`'s pad
//!   module presses past its threshold, so a stick moved less records
//!   nothing. An axis command's recording takes only a stick direction,
//!   binding its axis with the row's inversion. A refused input
//!   ([`Bindings::assign_pad`](crate::keymap::Bindings::assign_pad))
//!   reads "Held by" the command holding it, and the recording waits on.
//! - **Ending it.** A binding made, [`PAD_RECORD_TIMEOUT`] with no pad
//!   press, a click off the row, or any key press (the keyboard can't
//!   record a pad input). The keyboard tab's recorder keeps the
//!   original's: no timeout.
//! - **Clearing.** West, outside a recording, clears the selected row on
//!   the shown tab, as Delete does.

use std::time::Duration;

use super::{
    AXIS_RECORDING, Capture, Column, INDENT, INVERT, KEY_COLUMN, KeysScreen, LIST_AREA,
    PAD_RECORD_TIMEOUT, PAD_RECORDING, PAD_UNBOUND, PadRecorded, RECORDING, Row, TAB_AREAS,
    TAB_DIM, TITLE_SIZE, placed,
};
use crate::bound_names::name_on_pad;
use crate::color::Color;
use crate::draw::DrawList;
use crate::geometry::{Bounds, Point};
use crate::input::{PadButton, Surroundings};
use crate::keymap::{AxisBinding, Command, Kind, PadBinding, PadEntry};
use crate::text::TextMetrics;
use crate::ui::row_list::{BarRoom, RowList};

/// The cell of the row `capture` records on: who refused the last input,
/// or what it waits for.
pub(super) fn capture_cell(capture: Capture) -> String {
    if let Some(holder) = capture.refused {
        return format!("Held by {}", holder.label());
    }
    let waiting = match (capture.pad, capture.command.kind()) {
        (false, _) => RECORDING,
        (true, Kind::Axis) => AXIS_RECORDING,
        (true, Kind::Held | Kind::Pressed) => PAD_RECORDING,
    };
    waiting.to_owned()
}

impl KeysScreen {
    /// The tab shown.
    #[must_use]
    pub const fn tab(&self) -> Column {
        self.tab
    }

    /// `column`'s tab label's rectangle on the screen.
    #[must_use]
    pub fn tab_bounds(&self, column: Column) -> Bounds {
        placed(self.bounds.min, TAB_AREAS[column.index()])
    }

    /// The list over `rows`, its top left at `origin`'s [`LIST_AREA`].
    pub(super) fn list_of(rows: &[Row], origin: Point, metrics: &impl TextMetrics) -> RowList {
        let labels = rows
            .iter()
            .map(|row| match row {
                Row::Heading(context) => context.label().to_owned(),
                Row::Command(command) => format!("{INDENT}{}", command.label()),
                Row::Invert(_) => format!("{INDENT}{INDENT}{INVERT}"),
            })
            .collect();
        let headings: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| matches!(row, Row::Heading(_)).then_some(index))
            .collect();
        RowList::new(labels, placed(origin, LIST_AREA), metrics, BarRoom::Inside)
            .with_column(KEY_COLUMN, Vec::new())
            .with_headings(&headings)
    }

    /// Shows `column`'s tab, keeping the selected command when it lists
    /// it.
    pub(super) fn switch_tab(&mut self, column: Column) {
        if column == self.tab {
            return;
        }
        let selected = self
            .list
            .selected()
            .and_then(|row| self.rows.get(row))
            .and_then(|row| match *row {
                Row::Command(command) | Row::Invert(command) => Some(command),
                Row::Heading(_) => None,
            });
        self.tab = column;
        self.rows = super::rows(column);
        self.list = Self::list_of(&self.rows, self.bounds.min, &self.metrics);
        if let Some(row) = selected.and_then(|command| {
            self.rows
                .iter()
                .position(|&row| row == Row::Command(command))
        }) {
            self.list.select(row);
        }
        self.refresh();
    }

    /// The gamepad tab's cell of `command`: its pad input named for the
    /// family last told.
    pub(super) fn pad_cell(&self, command: Command) -> String {
        name_on_pad(self.edit.draft(), command, self.pad_family)
            .unwrap_or_else(|| PAD_UNBOUND.to_owned())
    }

    /// The invert row's cell of the axis command `command`.
    pub(super) fn invert_cell(&self, command: Command) -> &'static str {
        let inverted = self
            .edit
            .draft()
            .gamepad
            .axis_binding(command)
            .is_some_and(|binding| binding.inverted);
        if inverted { "On" } else { "Off" }
    }

    /// Follows the surroundings for the gamepad tab's names: a pad's
    /// family is kept until another pad's is told.
    pub(super) fn follow_family(&mut self, surroundings: Surroundings) {
        if let crate::input::Device::Gamepad(family) = surroundings.device
            && family != self.pad_family
        {
            self.pad_family = family;
            self.refresh();
        }
    }

    /// Follows a pad button going down or up, recording it while the
    /// gamepad tab records, or clearing the selected row on West. Says
    /// whether it took the input, so the pad's reader acts on none of it.
    pub(super) fn pad_button(&mut self, button: PadButton, pressed: bool) -> bool {
        let held = self.pads_down.clone();
        self.pads_down.retain(|&down| down != button);
        if pressed {
            self.pads_down.push(button);
        }
        if self.capture.is_some_and(|capture| capture.pad) {
            if pressed {
                self.record_pad_press(button, &held);
            }
            return true;
        }
        if pressed && button == PadButton::West && self.capture.is_none() {
            self.clear_selected();
            return true;
        }
        false
    }

    /// Records `button`'s press, with `held` the pad buttons down before
    /// it, on the gamepad tab's recording (see the module docs).
    fn record_pad_press(&mut self, button: PadButton, held: &[PadButton]) {
        let Some(mut capture) = self.capture else {
            return;
        };
        capture.waited = Duration::ZERO;
        self.capture = Some(capture);
        let binding = if capture.command.kind() == Kind::Axis {
            let Some((axis, _)) = button.stick_direction() else {
                return;
            };
            let inverted = self
                .edit
                .draft()
                .gamepad
                .axis_binding(capture.command)
                .is_some_and(|binding| binding.inverted);
            PadBinding::Axis(AxisBinding { axis, inverted })
        } else {
            let layer = held
                .iter()
                .copied()
                .find(|down| PadButton::LAYERS.contains(down) && *down != button);
            if layer.is_none() && PadButton::LAYERS.contains(&button) {
                return;
            }
            PadBinding::Entry(PadEntry { button, layer })
        };
        match self.edit.record_pad(capture.command, binding) {
            PadRecorded::Bound(_) => self.end_capture(),
            PadRecorded::Refused(holder) => {
                self.capture = Some(Capture {
                    refused: Some(holder),
                    ..capture
                });
                self.refresh();
            }
        }
    }

    /// Lets a recording on the gamepad tab wait `dt` longer, ending it at
    /// [`PAD_RECORD_TIMEOUT`].
    pub(super) fn wait_on(&mut self, dt: Duration) {
        let Some(mut capture) = self.capture.filter(|capture| capture.pad) else {
            return;
        };
        capture.waited += dt;
        if capture.waited >= PAD_RECORD_TIMEOUT {
            self.end_capture();
        } else {
            self.capture = Some(capture);
        }
    }

    /// The tab labels on the title row, the shown one white.
    pub(super) fn draw_tabs(&self, list: &mut DrawList) {
        for column in Column::ALL {
            let color = if column == self.tab {
                Color::WHITE
            } else {
                TAB_DIM
            };
            list.text(
                column.label(),
                self.tab_bounds(column).min,
                TITLE_SIZE,
                None,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests;
