//! A [`Dialog`]'s pad focus (rdm:phase/key-bindings/phase-8-gamepad-focus):
//! its targets, the pad's moves and presses, and the highlight.
//!
//! - **Targets** are the items a left click acts on that are shown,
//!   enabled and drawn: buttons, role-less user items (click areas), and
//!   the scroll arrows while their text overflows. Their ids are item
//!   numbers, and item order is Tab order.
//! - **Until the dialog is told** the pad was the device used last, the
//!   pad's focus moves and South do nothing, as its highlight can't show
//!   ([`pad_focus`]'s "Nothing hidden acts"); East and the rest act.
//! - **The initial target**, where the pad's focus lands on its first
//!   `Surroundings(Gamepad)` or first move, is the default item (the one
//!   Return presses) when it is a target, otherwise the first target. So
//!   South at once does what Return does.
//! - **The d-pad and the shoulders** move the focus under either
//!   [`FocusRing`] reading ([`pad_focus::step`]).
//! - **South** (Activate) presses the focused item as a left press does:
//!   it arms it through the dialog's tracker, drawn pressed while held,
//!   and its release activates it as a click's does; on a scroll arrow
//!   whose flag is set it holds the arrow, scrolling at once and on each
//!   tick, until its release.
//!   The press is silent, as a click is in every in-game dialog. A focus
//!   move drops a held press silently, and a release whose press the
//!   dialog never saw does nothing.
//! - **Cells.** A host that draws rows or grid cells inside one user item
//!   (the mission lists, the outfitter's and shipyard's grids) gives
//!   them as cells ([`Dialog::set_pad_cells`]): they stand in for that
//!   item among the targets, in its place in Tab order. South on a cell
//!   reports [`DialogEvent::Cell`] on its press, silently, as a pointer
//!   press acts on a grid or a list; the host runs its own press on it.
//!   Keyboard Tab never visits a cell, and from a cell the pad placed it
//!   goes on from the cells' item. A cell gone (a scroll, a shorter list)
//!   is no target: it draws no highlight, and the next move lands on the
//!   initial target.
//! - **Items left out.** A host takes role-less user items whose click
//!   does nothing out of the targets ([`Dialog::set_pad_left_out`], the
//!   whole set each call), so the d-pad never stops on a dead area.
//! - **Everything else** a pad button means (Back, the left stick's Up
//!   and Down) goes through [`Dialog::command`], as its key's does.
//! - **The highlight** ([`pad_focus::draw_highlight`]) shows only while the
//!   pad was the device used last; the keyboard's ring shows only a focus
//!   the keyboard placed.
//! - **A dialog laid out again** while it stays open (a new hail, a new
//!   view, a relabel) goes through [`Dialog::lay_out_again`], which keeps
//!   the device it was told and the pad's focused item: the app tells an
//!   open screen the device only when it changes.
//!
//! The original has no focus here (`_MyModalDialog` @0x59954), so none of
//! this has a `Rulebook` key, and a focus move is silent.

use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::input::{Device, Input, Surroundings};
use crate::keymap::{Command, CommandInput};
// The tests name commands through `super::*`.
#[cfg(test)]
use crate::keymap::UiCommand;

use super::super::pad_focus::{self, FocusMove, PadRead, Target};
use super::super::scroll_arrows::ArrowHold;
use super::{Dialog, DialogEvent, FocusRing, Widget};

/// A place the pad's focus can rest on in a [`Dialog`]: an item (from 1)
/// or one of the host's cells (from 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PadSpot {
    /// Item `n` (from 1).
    Item(usize),
    /// Cell `n` (from 0) of the host's cells.
    Cell(usize),
}

impl PadSpot {
    /// The item, if it is one.
    pub(crate) const fn item(self) -> Option<usize> {
        match self {
            Self::Item(item) => Some(item),
            Self::Cell(_) => None,
        }
    }
}

impl Dialog {
    /// Makes `cells` (rects in logical units, in the host's order) stand
    /// in for item `item` (from 1) among the pad's targets, in its place
    /// in Tab order; the item itself is no target while it has cells. An
    /// empty list leaves the item with none. The host gives them afresh
    /// whenever they move: after a scroll, a relayout or a new listing.
    pub fn set_pad_cells(&mut self, item: usize, cells: Vec<Bounds>) {
        self.pad_cells = Some((item, cells));
    }

    /// Takes `items` (from 1), and only them, out of the pad's targets:
    /// role-less user items whose click does nothing. Each call replaces
    /// the items the last one left out, so a host whose dead areas change
    /// as it runs (the trade center's empty slots) gives the whole set
    /// afresh, and an empty list leaves none out. The set outlives
    /// [`Dialog::lay_out_again`].
    pub fn set_pad_left_out(&mut self, items: Vec<usize>) {
        self.left_out = items;
    }

    /// The cell the pad's highlight is on (from 0), if it shows.
    #[must_use]
    pub fn pad_highlight_cell(&self) -> Option<usize> {
        match self.focus.pad_highlight() {
            Some(spot @ PadSpot::Cell(cell)) if self.is_pad_target(spot) => Some(cell),
            _ => None,
        }
    }

    /// The device the player used last, as the last surroundings it was
    /// told said; the keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.focus.device()
    }

    /// Becomes `laid`, the same dialog laid out afresh (a new reply, new
    /// labels, a new column of buttons), keeping what this one was given
    /// and told: its focus ring, its bindings, the device the player used
    /// last and the item the pad focused.
    ///
    /// Every host that rebuilds its dialog while it stays open goes
    /// through here. The app tells a screen the device only when it
    /// changes or the screen opens, so a rebuilt dialog that forgot it
    /// would hold back every pad focus move and South from then on
    /// ([`pad_focus`]'s "Nothing hidden acts").
    pub fn lay_out_again(&mut self, laid: Self) {
        let mut laid = laid
            .with_focus_ring(self.focus_ring)
            .with_bindings(self.bindings().clone());
        laid.pad_cells = self.pad_cells.take();
        laid.left_out = std::mem::take(&mut self.left_out);
        laid.carry_pad_from(self);
        *self = laid;
    }

    /// Takes the device `old` was told and the item the pad focused on
    /// it, while that item is still a target here. While the pad shows,
    /// a focus not carried starts again on the initial target, as the
    /// pad's surroundings place it.
    fn carry_pad_from(&mut self, old: &Self) {
        self.focus.carry_pad_from(&old.focus);
        if self.focus.pad_moved()
            && let Some(spot) = self.focus.get()
            && !self.is_pad_target(spot)
        {
            self.focus.set_by_keys(None);
        }
        self.surroundings(old.device());
        self.keys.follow(&Input::Surroundings(Surroundings {
            device: old.device(),
        }));
    }

    /// The item the pad's highlight is on (from 1), if it shows.
    #[must_use]
    pub fn pad_highlight(&self) -> Option<usize> {
        self.focus
            .pad_highlight()
            .filter(|&spot| self.is_pad_target(spot))
            .and_then(PadSpot::item)
    }

    /// The items the pad's focus can rest on, in item order, leaving out
    /// the cells and the item they stand in for: see the module docs.
    #[cfg(test)]
    pub(crate) fn pad_targets(&self) -> Vec<Target<usize>> {
        self.pad_spots()
            .into_iter()
            .filter_map(|target| target.id.item().map(|item| Target::at(item, target.bounds)))
            .collect()
    }

    /// Everything the pad's focus can rest on, in Tab order: the items,
    /// with the cells in their item's place.
    pub(crate) fn pad_spots(&self) -> Vec<Target<PadSpot>> {
        let mut spots = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            let number = index + 1;
            if let Some((cells_item, cells)) = &self.pad_cells
                && *cells_item == number
            {
                spots.extend(
                    cells
                        .iter()
                        .enumerate()
                        .map(|(cell, &bounds)| Target::at(PadSpot::Cell(cell), bounds)),
                );
                continue;
            }
            if self.left_out.contains(&number) {
                continue;
            }
            let target = item.active()
                && item.drawn
                && match &item.widget {
                    Widget::Button(_) => true,
                    Widget::ScrollArrow(arrow) => self
                        .scroll_text()
                        .is_some_and(|text| arrow.button(item.bounds, text).is_some()),
                    Widget::Blank => item.user,
                    Widget::Text { .. } | Widget::Picture(_) => false,
                };
            if target {
                spots.push(Target::at(PadSpot::Item(number), item.bounds));
            }
        }
        spots
    }

    fn is_pad_target(&self, spot: PadSpot) -> bool {
        self.pad_spots().iter().any(|target| target.id == spot)
    }

    /// Where the pad's focus starts: the default item when it is a
    /// target, otherwise the first target.
    fn initial_target(&self) -> Option<PadSpot> {
        let targets = self.pad_spots();
        self.default_item
            .map(PadSpot::Item)
            .filter(|&spot| targets.iter().any(|target| target.id == spot))
            .or_else(|| targets.first().map(|target| target.id))
    }

    /// The device used last changed.
    pub(super) fn surroundings(&mut self, device: Device) {
        let initial = self.initial_target();
        self.focus.surroundings(device, initial, false);
    }

    /// Acts on what a pad button meant: see the module docs.
    pub(super) fn pad_command(&mut self, input: CommandInput) -> Option<DialogEvent> {
        match self.focus.read_pad(input) {
            PadRead::Nothing => None,
            PadRead::Move(mv) => {
                self.drop_pad_press();
                if let Some(initial) = self.initial_target() {
                    self.focus.move_by_pad(&self.pad_spots(), initial, mv);
                }
                None
            }
            PadRead::Press => self.press_focused(),
            PadRead::Release => self.release_item_keyed(),
            PadRead::Other => self.command(input),
        }
    }

    /// A focus move a key is bound to, under [`FocusRing::Tab`]: it moves
    /// the one focus as the pad's would, as the keyboard's.
    pub(super) fn keyed_focus_move(&mut self, command: Command) {
        debug_assert_eq!(self.focus_ring, FocusRing::Tab);
        if let (Some(mv), Some(initial)) = (FocusMove::of(command), self.initial_target()) {
            self.focus.move_by_keys(&self.pad_spots(), initial, mv);
        }
    }

    /// South went down: presses the focused item as a left press on it
    /// does. A scroll arrow whose flag is set starts a hold, which scrolls
    /// at once and on each tick until South's release; anything else is
    /// armed, drawn pressed. A cell is reported at once, silently.
    fn press_focused(&mut self) -> Option<DialogEvent> {
        match self.focus.get().filter(|&spot| self.is_pad_target(spot))? {
            PadSpot::Item(item) => {
                self.press_item_keyed(item);
                None
            }
            PadSpot::Cell(cell) => Some(DialogEvent::Cell(cell)),
        }
    }

    /// A pad button presses item `item` (from 1) as a left press on it
    /// does: how a host that keeps the focus itself (Preferences, over its
    /// OK and Key Settings) presses one of the dialog's buttons. Its
    /// release is [`Dialog::release_item_keyed`].
    pub(crate) fn press_item_keyed(&mut self, item: usize) {
        if !self.item(item).is_some_and(|found| found.shown) {
            return;
        }
        self.cancel_pointer();
        let index = item - 1;
        if let Widget::ScrollArrow(arrow) = self.items[index].widget
            && self
                .scroll_text()
                .is_some_and(|text| arrow.can_scroll(text))
        {
            self.hold = Some(ArrowHold::new(arrow));
            self.pad_hold = true;
            if let Some(text) = self.scroll_text_mut() {
                text.scroll_units(arrow.step());
            }
            return;
        }
        self.tracked = Some(index);
        self.tracker.press_keyed(self.items[index].active());
        // Silent, as a click is in every in-game dialog.
        let _ = self.tracker.take_sound();
    }

    /// South came up: ends its hold on an arrow, or activates the item its
    /// press armed, as a click's release does.
    pub(crate) fn release_item_keyed(&mut self) -> Option<DialogEvent> {
        if self.pad_hold {
            self.hold = None;
            self.pad_hold = false;
            self.release_keys(None);
            return None;
        }
        let index = self.tracked?;
        if !self.tracker.release_keyed() {
            return None;
        }
        let _ = self.tracker.take_sound();
        self.tracked = None;
        self.release_keys(None);
        if let Widget::ScrollArrow(arrow) = self.items[index].widget {
            if let Some(text) = self.scroll_text_mut() {
                text.scroll_arrow(arrow);
            }
            return None;
        }
        Some(DialogEvent::Item(index + 1))
    }

    /// A focus move drops a held press, or ends a hold, silently.
    fn drop_pad_press(&mut self) {
        if self.tracked.is_some() || self.pad_hold {
            self.cancel_pointer();
        }
    }

    /// Draws the pad's highlight round the item it is on, if it shows.
    pub(super) fn draw_pad_highlight(&self, list: &mut DrawList) {
        let Some(spot) = self
            .focus
            .pad_highlight()
            .filter(|&spot| self.is_pad_target(spot))
        else {
            return;
        };
        let bounds = match spot {
            PadSpot::Item(item) => self.item_bounds(item),
            PadSpot::Cell(cell) => self
                .pad_cells
                .as_ref()
                .and_then(|(_, cells)| cells.get(cell).copied()),
        };
        if let Some(bounds) = bounds {
            pad_focus::draw_highlight(list, bounds, self.style.up);
        }
    }
}

#[cfg(test)]
mod tests;
