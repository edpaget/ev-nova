//! The keys screen's pad focus
//! (rdm:phase/key-bindings/phase-8-gamepad-focus).
//!
//! The screen reads its keys raw (see the module docs), but a pad has no
//! keys: it reads the pad through a [`CommandReader`] over
//! [`contexts::DIALOG`](crate::keymap::contexts::DIALOG), on the bindings
//! it opened on, for pad inputs only.
//!
//! - **Targets** are the list (its view, one target), Restore Defaults,
//!   Cancel and OK. The focus is always somewhere, the list at first, and
//!   is kept when the player goes back to the keyboard
//!   ([`FocusState::surroundings`]'s `keeps`).
//! - **The d-pad** moves it; **the shoulders** show the keyboard's tab
//!   (left) and the gamepad's (right), as Left and Right do; **the left
//!   stick** steps the list's selection as the arrow keys do.
//! - **South** on the list starts recording the selected row, as Space
//!   does (on an invert row, toggles it); on a button it presses it as a
//!   click does, on its release.
//! - **East** is Cancel, as Escape is. During a recording on the
//!   keyboard's tab it ends the recording, as a click elsewhere does, and
//!   the pad's other buttons record nothing there; on the gamepad tab
//!   every button records (`pad_record`).
//! - **West**, outside a recording, clears the selected row on the shown
//!   tab, as Delete does.
//! - **Until the screen is told** the pad was the device used last, the
//!   pad's focus moves and South do nothing, though the outline shows the
//!   focus: one rule for every screen ([`pad_focus`]'s "Nothing hidden
//!   acts"). East and the left stick act.
//! - **Drawing**: while the pad was the device used last, its highlight
//!   replaces the outline the focus always has.
//!
//! [`FocusState::surroundings`]: super::super::pad_focus::FocusState::surroundings

use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::input::{Device, Input};
use crate::keymap::{Command, CommandInput, CommandReader, UiCommand};

use super::super::dialog::outline;
use super::super::pad_focus::{self, PadRead, Target};
use super::{Column, KeysButton, KeysFocus, KeysScreen};

impl KeysScreen {
    /// The device the player used last, as the last surroundings it was
    /// told said; the keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.focus.device()
    }

    /// The targets, in order: the list, then the buttons left to right.
    fn pad_targets(&self) -> Vec<Target<KeysFocus>> {
        let mut targets = vec![Target::at(KeysFocus::List, self.list.view())];
        targets.extend(
            KeysButton::ALL
                .map(|button| Target::at(KeysFocus::Button(button), self.button_bounds(button))),
        );
        targets
    }

    /// Where the focus `focus` is drawn.
    fn focus_rect(&self, focus: KeysFocus) -> Bounds {
        match focus {
            KeysFocus::List => self.list.view(),
            KeysFocus::Button(button) => self.button_bounds(button),
        }
    }

    /// Follows a pad input, or the surroundings, through the pad's reader.
    pub(super) fn pad_input(&mut self, input: &Input) {
        if let Input::Surroundings(surroundings) = *input {
            self.reader.follow(input);
            self.focus
                .surroundings(surroundings.device, Some(KeysFocus::List), true);
            self.follow_family(surroundings);
            return;
        }
        let taken = match *input {
            Input::PadButton {
                button, pressed, ..
            } => self.pad_button(button, pressed),
            _ => false,
        };
        if let Some(command) = self.reader.follow(input)
            && !taken
        {
            self.pad_command(command);
        }
    }

    /// Acts on what a pad button meant: see the module docs.
    fn pad_command(&mut self, input: CommandInput) {
        let Command::Ui(command) = input.command else {
            return;
        };
        if self.capture.is_some() {
            if command == UiCommand::Back && input.pressed {
                self.end_capture();
            }
            return;
        }
        match (command, input.pressed) {
            (UiCommand::PrevFocus, true) => return self.switch_tab(Column::Keyboard),
            (UiCommand::Tab, true) => return self.switch_tab(Column::Gamepad),
            _ => {}
        }
        match self.focus.read_pad(input) {
            PadRead::Nothing => return,
            PadRead::Move(mv) => {
                self.drop_pad_press();
                self.focus
                    .move_by_pad(&self.pad_targets(), KeysFocus::List, mv);
                return;
            }
            PadRead::Press | PadRead::Release | PadRead::Other => {}
        }
        match (command, input.pressed) {
            (UiCommand::Up | UiCommand::Down, true) => {
                if self.focus() == KeysFocus::List {
                    self.list.step_wrapping(command == UiCommand::Down);
                }
            }
            (UiCommand::Activate, true) => match self.focus() {
                KeysFocus::List => self.start_capture(),
                KeysFocus::Button(button) => {
                    self.cancel_pointer_press();
                    self.trackers[button.index()].press_keyed(true);
                    self.pad_pressed = Some(button);
                }
            },
            (UiCommand::Activate, false) => {
                if let Some(button) = self.pad_pressed.take()
                    && self.trackers[button.index()].release_keyed()
                {
                    self.press_button(button);
                }
            }
            (UiCommand::Back, true) => self.press_button(KeysButton::Cancel),
            _ => {}
        }
    }

    /// A focus move drops a held press, silently.
    fn drop_pad_press(&mut self) {
        if let Some(button) = self.pad_pressed.take() {
            self.trackers[button.index()].cancel_pointer();
        }
    }

    /// The focus's mark: the pad's highlight while the pad was the device
    /// used last, the outline otherwise.
    pub(super) fn draw_focus(&self, list: &mut DrawList) {
        let rect = self.focus_rect(self.focus());
        if self.focus.pad_shown() {
            pad_focus::draw_highlight(list, rect, self.style.up);
        } else {
            outline(list, rect, self.style.up);
        }
    }

    /// The pad's reader on `bindings`.
    pub(super) fn pad_reader(bindings: crate::keymap::Bindings) -> CommandReader {
        CommandReader::new(crate::keymap::contexts::DIALOG)
            .with_bindings(crate::keymap::SharedBindings::new(bindings))
    }
}

#[cfg(test)]
pub(super) mod tests;
