//! The player info screen's pad focus
//! (rdm:phase/key-bindings/phase-8-gamepad-focus).
//!
//! The screen's own buttons have no keyboard focus: Tab and Tab Back
//! change the tab, and Space does nothing, as `_PlayerInfoFilter`
//! (@0x53ae9) maps neither to a button. The pad's focus is new:
//!
//! - **Targets** are Done, the four tabs and Jettison while it is shown,
//!   in that order. The pad's focus starts on Done.
//! - **The d-pad** moves the focus ([`pad_focus::step`]); keys bound to
//!   the focus moves do nothing here, as the keyboard has no focus.
//! - **The shoulders** are Tab (Next Field) and Previous Field, which
//!   change the tab as Tab and Shift-Tab do.
//! - **South** presses the focused button as a click does, through the
//!   screen's one tracker: drawn pressed while held, chosen on the
//!   release, silently. A focus move drops the press.
//! - **East** is Back, which closes the screen as Escape does.
//! - **The Jettison question** is told the device as it opens, and then
//!   takes the pad as any [`Dialog`](super::super::dialog::Dialog) does.

use crate::draw::DrawList;
use crate::input::{Device, Input, Surroundings};
use crate::keymap::{Command, CommandInput, UiCommand};

use super::super::pad_focus::{self, FocusMove, Target};
use super::{PlayerInfoButton, PlayerInfoScreen};

impl PlayerInfoScreen {
    /// The device the player used last, as the last surroundings it was
    /// told said; the keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.focus.device()
    }

    /// The button the pad's focus is on, if any.
    #[must_use]
    pub fn pad_focus(&self) -> Option<PlayerInfoButton> {
        self.focus.get()
    }

    /// The buttons the pad's focus can rest on, in order.
    fn pad_targets(&self) -> Vec<Target<PlayerInfoButton>> {
        PlayerInfoButton::ALL
            .into_iter()
            .filter(|&button| button != PlayerInfoButton::Jettison || self.jettison_shown())
            .filter_map(|button| Some(Target::at(button, self.button_bounds(button)?)))
            .collect()
    }

    /// The surroundings for a new Jettison question, so it shows the
    /// pad's highlight from its first frame.
    pub(super) fn told(&self) -> Input {
        Input::Surroundings(Surroundings {
            device: self.device(),
        })
    }

    /// The device used last changed.
    pub(super) fn pad_surroundings(&mut self, device: Device) {
        let initial = self.laid.is_ok().then_some(PlayerInfoButton::Done);
        self.focus.surroundings(device, initial, false);
    }

    /// Acts on what a pad button meant, if it is the pad's own: a focus
    /// move or South. `false` for anything else, which the screen's keys
    /// handle as a key's.
    pub(super) fn pad_command(&mut self, command: Option<CommandInput>) -> bool {
        let Some(command) = command else {
            return false;
        };
        if let Some(FocusMove::Toward(direction)) = FocusMove::of(command.command) {
            if command.pressed {
                self.drop_pad_press();
                if let Some(next) = pad_focus::step(
                    &self.pad_targets(),
                    self.focus.get(),
                    PlayerInfoButton::Done,
                    FocusMove::Toward(direction),
                ) {
                    self.focus.set_by_pad(next);
                }
            }
            return true;
        }
        if command.command != Command::Ui(UiCommand::Activate) {
            return false;
        }
        if command.pressed {
            self.press_focused();
        } else if let Some(which) = self.tracked
            && self.tracker.release_keyed()
        {
            let _ = self.tracker.take_sound();
            self.tracked = None;
            self.choose(which);
        }
        true
    }

    /// South went down: arms the focused button, drawn pressed.
    fn press_focused(&mut self) {
        let targets = self.pad_targets();
        let Some(which) = self
            .focus
            .get()
            .filter(|&which| targets.iter().any(|target| target.id == which))
        else {
            return;
        };
        self.tracker.cancel_pointer();
        self.tracked = Some(which);
        self.tracker.press_keyed(true);
        // Silent, as `_TrackPlayerInfoButtons` is.
        let _ = self.tracker.take_sound();
    }

    /// A focus move drops a held press, silently.
    fn drop_pad_press(&mut self) {
        self.tracker.cancel_pointer();
        self.tracked = None;
    }

    /// Draws the pad's highlight round the button it is on, if it shows.
    /// None while the Jettison question asks, whose own highlight shows.
    pub(super) fn draw_pad_highlight(&self, list: &mut DrawList) {
        let Some(which) = self
            .focus
            .pad_highlight()
            .filter(|_| self.confirm.is_none())
        else {
            return;
        };
        if let Some(target) = self.pad_targets().iter().find(|target| target.id == which) {
            pad_focus::draw_highlight(list, target.bounds, self.style.up);
        }
    }
}

#[cfg(test)]
mod tests;
