//! The pilot list's pad focus
//! (rdm:phase/key-bindings/phase-11-gamepad-menus).
//!
//! The original's Open Pilot is Navigation Services' own panel
//! (`_MyOpenDocument` @0x731c5, `NavCreateGetFileDialog` @0x73298), whose
//! file browser holds the keyboard focus from the start; the engine's
//! list is its own, so the pad's focus here is a new feature, with no
//! `Rulebook` key, and a focus move is silent.
//!
//! - **Targets** are the slots that show a pilot, then Cancel, then Open
//!   while it is enabled. A row is held by its slot, as a pad's cursor
//!   is: while the left stick scrolls the list, the focus stays on its
//!   slot and South presses whichever pilot the slot now shows.
//! - **The initial target** is the selected pilot's row, as the system
//!   panel's file list starts with the focus; with no pilot, Cancel.
//! - **South** arms the focused area as a left press does, silently, as
//!   the list is under the pointer; its release activates it as a click's
//!   does: a row selects and opens its pilot, Open opens the selection,
//!   Cancel cancels. A focus move drops a held South silently, and a
//!   release whose press the list never saw does nothing.
//! - Only a pad button acts on the focus ([`pad_focus::from_pad`]): the
//!   keyboard's Space, which also means Activate, does nothing, as
//!   before. East is the reader's Back, which cancels as Escape does.

use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::input::Device;
use crate::keymap::CommandInput;
use crate::ui::pad_focus::{self, PadRead, Target as FocusTarget};

use super::{PilotList, PilotListOutcome, Target, VISIBLE_ROWS};

/// A place the pad's focus can rest on in the pilot list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadTarget {
    /// The row in this slot (from 0, at the top), whichever pilot it
    /// shows.
    Row(usize),
    /// The Cancel button.
    Cancel,
    /// The Open button.
    Open,
}

impl PilotList {
    /// The device the player used last, as the last surroundings said;
    /// the keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.focus.device()
    }

    /// Where the pad's highlight is, if it shows.
    #[must_use]
    pub fn pad_highlight(&self) -> Option<PadTarget> {
        self.focus
            .pad_highlight()
            .filter(|&target| self.pad_rect(target).is_some())
    }

    /// The pad's targets, in Tab order.
    fn pad_targets(&self) -> Vec<FocusTarget<PadTarget>> {
        let rows = self.keys.len().saturating_sub(self.first).min(VISIBLE_ROWS);
        (0..rows)
            .map(PadTarget::Row)
            .chain([PadTarget::Cancel, PadTarget::Open])
            .filter_map(|target| {
                self.pad_rect(target)
                    .map(|rect| FocusTarget::at(target, rect))
            })
            .collect()
    }

    /// Where `target` is, while it is a target.
    fn pad_rect(&self, target: PadTarget) -> Option<Bounds> {
        match target {
            PadTarget::Row(slot) => self.row(self.first + slot),
            PadTarget::Cancel => Some(self.cancel.rect),
            PadTarget::Open => self.open.enabled.then_some(self.open.rect),
        }
    }

    /// Where the pad's focus starts: the selected pilot's row, or Cancel
    /// with none.
    fn pad_initial(&self) -> PadTarget {
        self.selected
            .and_then(|index| index.checked_sub(self.first))
            .filter(|&slot| slot < VISIBLE_ROWS)
            .map_or(PadTarget::Cancel, PadTarget::Row)
    }

    /// The device used last changed.
    pub(super) fn pad_surroundings(&mut self, device: Device) {
        let initial = self.pad_initial();
        self.focus.surroundings(device, Some(initial), false);
    }

    /// Acts on what a pad button meant; `false` when it is no focus
    /// command, for the list to act on as its key's.
    pub(super) fn pad_command(&mut self, input: CommandInput) -> bool {
        match self.focus.read_pad(input) {
            PadRead::Nothing => {}
            PadRead::Move(mv) => {
                self.drop_pad_press();
                let initial = self.pad_initial();
                self.focus.move_by_pad(&self.pad_targets(), initial, mv);
            }
            PadRead::Press => self.press_focused(),
            PadRead::Release => self.release_focused(),
            PadRead::Other => return false,
        }
        true
    }

    /// South went down: arms the focused area, silently.
    fn press_focused(&mut self) {
        let Some(target) = self.pad_highlight() else {
            return;
        };
        let (tracked, enabled) = match target {
            PadTarget::Row(slot) => (Target::Row(self.first + slot), true),
            PadTarget::Cancel => (Target::Cancel, true),
            PadTarget::Open => (Target::Open, self.open.enabled),
        };
        self.tracked = Some(tracked);
        self.tracker.press_keyed(enabled);
        // Silent, as under the pointer.
        let _ = self.tracker.take_sound();
    }

    /// South came up: activates what its press armed, as a click does.
    fn release_focused(&mut self) {
        let Some(target) = self.tracked else {
            return;
        };
        if !self.tracker.release_keyed() {
            return;
        }
        let _ = self.tracker.take_sound();
        self.tracked = None;
        match target {
            Target::Row(index) => {
                self.select(index);
                self.open_selected();
            }
            Target::Open => self.open_selected(),
            Target::Cancel => self.outcome = Some(PilotListOutcome::Cancel),
        }
    }

    /// A focus move drops a held South, silently.
    fn drop_pad_press(&mut self) {
        if self.tracked.is_some() {
            self.tracker.cancel_pointer();
            self.tracked = None;
        }
    }

    /// Draws the pad's highlight round its target, if it shows.
    pub(super) fn draw_pad_highlight(&self, list: &mut DrawList) {
        if let Some(rect) = self
            .pad_highlight()
            .and_then(|target| self.pad_rect(target))
        {
            pad_focus::draw_highlight(list, rect, self.style.up);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
