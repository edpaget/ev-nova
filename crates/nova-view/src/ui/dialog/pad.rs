//! A [`Dialog`]'s pad focus (rdm:phase/key-bindings/phase-8-gamepad-focus):
//! its targets, the pad's moves and presses, and the highlight.
//!
//! - **Targets** are the items a left click acts on that are shown,
//!   enabled and drawn: buttons, role-less user items (click areas), and
//!   the scroll arrows while their text overflows. Their ids are item
//!   numbers, and item order is Tab order.
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
//! - **Everything else** a pad button means (Back, the left stick's Up
//!   and Down) goes through [`Dialog::command`], as its key's does.
//! - **The highlight** ([`pad_focus::draw_highlight`]) shows only while the
//!   pad was the device used last; the keyboard's ring shows only a focus
//!   the keyboard placed.
//!
//! The original has no focus here (`_MyModalDialog` @0x59954), so none of
//! this has a `Rulebook` key, and a focus move is silent.

use crate::draw::DrawList;
use crate::input::{Device, Input, Surroundings};
use crate::keymap::{Command, CommandInput, UiCommand};

use super::super::pad_focus::{self, FocusMove, Target};
use super::super::scroll_arrows::ArrowHold;
use super::{Dialog, DialogEvent, FocusRing, Widget};

impl Dialog {
    /// The device the player used last, as the last surroundings it was
    /// told said; the keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.focus.device()
    }

    /// Takes the device `old` was told and the item the pad focused on
    /// it: how a host that lays its dialog out again keeps the pad's
    /// place, as it keeps the focus ring.
    pub fn carry_pad_from(&mut self, old: &Self) {
        self.focus.carry_pad_from(&old.focus);
        self.keys.follow(&Input::Surroundings(Surroundings {
            device: old.device(),
        }));
    }

    /// The item the pad's highlight is on (from 1), if it shows.
    #[must_use]
    pub fn pad_highlight(&self) -> Option<usize> {
        self.focus
            .pad_highlight()
            .filter(|&item| self.is_pad_target(item))
    }

    /// The items the pad's focus can rest on, in item order: see the
    /// module docs.
    pub(crate) fn pad_targets(&self) -> Vec<Target<usize>> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.active()
                    && item.drawn
                    && match &item.widget {
                        Widget::Button(_) => true,
                        Widget::ScrollArrow(arrow) => self
                            .scroll_text()
                            .is_some_and(|text| arrow.button(item.bounds, text).is_some()),
                        Widget::Blank => item.user,
                        Widget::Text { .. } | Widget::Picture(_) => false,
                    }
            })
            .map(|(index, item)| Target::at(index + 1, item.bounds))
            .collect()
    }

    fn is_pad_target(&self, item: usize) -> bool {
        self.pad_targets().iter().any(|target| target.id == item)
    }

    /// Where the pad's focus starts: the default item when it is a
    /// target, otherwise the first target.
    fn initial_target(&self) -> Option<usize> {
        let targets = self.pad_targets();
        self.default_item
            .filter(|&item| targets.iter().any(|target| target.id == item))
            .or_else(|| targets.first().map(|target| target.id))
    }

    /// The device used last changed.
    pub(super) fn surroundings(&mut self, device: Device) {
        let initial = self.initial_target();
        self.focus.surroundings(device, initial, false);
    }

    /// Acts on what a pad button meant: see the module docs.
    pub(super) fn pad_command(&mut self, input: CommandInput) -> Option<DialogEvent> {
        if let Some(mv) = FocusMove::of(input.command) {
            if input.pressed {
                self.drop_pad_press();
                if let Some(next) = self.stepped(mv) {
                    self.focus.set_by_pad(next);
                }
            }
            return None;
        }
        if input.command == Command::Ui(UiCommand::Activate) {
            if input.pressed {
                self.press_focused();
                return None;
            }
            return self.release_focused();
        }
        self.command(input)
    }

    /// A focus move a key is bound to, under [`FocusRing::Tab`]: it moves
    /// the one focus as the pad's would, as the keyboard's.
    pub(super) fn keyed_focus_move(&mut self, command: Command) {
        debug_assert_eq!(self.focus_ring, FocusRing::Tab);
        if let Some(next) = FocusMove::of(command).and_then(|mv| self.stepped(mv)) {
            self.focus.set_by_keys(Some(next));
        }
    }

    /// Where `mv` takes the focus.
    fn stepped(&self, mv: FocusMove) -> Option<usize> {
        let initial = self.initial_target()?;
        pad_focus::step(&self.pad_targets(), self.focus.get(), initial, mv)
    }

    /// South went down: presses the focused item as a left press on it
    /// does. A scroll arrow whose flag is set starts a hold, which scrolls
    /// at once and on each tick until South's release; anything else is
    /// armed, drawn pressed.
    fn press_focused(&mut self) {
        let Some(item) = self.focus.get().filter(|&item| self.is_pad_target(item)) else {
            return;
        };
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
    fn release_focused(&mut self) -> Option<DialogEvent> {
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
        if let Some(bounds) = self.pad_highlight().and_then(|item| self.item_bounds(item)) {
            pad_focus::draw_highlight(list, bounds, self.style.up);
        }
    }
}

#[cfg(test)]
mod tests;
