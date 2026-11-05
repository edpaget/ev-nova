//! The main menu's pad focus
//! (rdm:phase/key-bindings/phase-11-gamepad-menus).
//!
//! The original's main screen has no focus, no default and no Tab:
//! `_MainScreenKey` @0x1aba0 branches only on its letters, and Return,
//! Enter, Tab and Escape fall through to its return (@0x1ac2d). So the
//! pad's focus here is a new feature, with no `Rulebook` key, and a focus
//! move is silent.
//!
//! - **Targets** are the six buttons in [`MenuChoice::ALL`] order, at
//!   [`MainMenu::button_bounds`], while their row shows: a row still
//!   sliding in has no target, so the highlight never surrounds a button
//!   that isn't drawn. The focus starts on New Pilot, the first in
//!   `_MainScreenChoice`'s order. With the art, left and right go to a
//!   button's row-mate, as explicit neighbours: the stock columns
//!   stagger enough to mislead the picker.
//! - **South** does what a primary press does on the focused button: it
//!   finishes every slide and presses the button with 600, drawn
//!   pressed; its release plays 601 and chooses it with
//!   [`ChosenBy::Pointer`](super::ChosenBy::Pointer), so the router runs it at once, as a click's.
//!   A focus move drops a held South silently, a pointer press takes over
//!   from it, and a release whose press the menu never saw does nothing.
//! - Only a pad button acts on the focus ([`pad_focus::from_pad`]): the
//!   keyboard's Space and Tab never reach it as focus commands (the
//!   router keeps Tab, and so the right shoulder, for the ship browser).
//!   East does nothing, as Escape does.
//! - **The highlight** is drawn over the buttons and under the rollover;
//!   the rollover keeps following the pointer alone.
//! - [`MainMenu::appear`] leaves the focus where it was.

use crate::geometry::Bounds;
use crate::input::{Device, Input};
use crate::keymap::CommandInput;
use crate::ui::pad_focus::{self, Direction, PadRead, Target};

use super::{MainMenu, MenuChoice, row_of};

impl MainMenu {
    /// The device the player used last, as the last surroundings said;
    /// the keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.focus.device()
    }

    /// The button the pad's highlight is on, if it shows.
    #[must_use]
    pub fn pad_highlight(&self) -> Option<MenuChoice> {
        self.focus
            .pad_highlight()
            .filter(|&choice| self.row_shown(row_of(choice.index())))
    }

    /// The rect the pad's highlight surrounds, if it shows.
    pub(super) fn pad_highlight_rect(&self) -> Option<Bounds> {
        self.pad_highlight()
            .map(|choice| self.button_bounds(choice))
    }

    /// The pad's targets: the buttons whose row shows. With the art, each
    /// button's left or right neighbour is its row-mate (0 and 3, 1 and
    /// 4, 2 and 5): the stock columns stagger, so the geometry alone would
    /// take Quit's right to New Pilot and Preferences' left to New Pilot.
    fn pad_targets(&self) -> Vec<Target<MenuChoice>> {
        let art = self.draws_art();
        MenuChoice::ALL
            .into_iter()
            .filter(|choice| self.row_shown(row_of(choice.index())))
            .map(|choice| {
                let target = Target::at(choice, self.button_bounds(choice));
                let index = choice.index();
                match (art, index < 3) {
                    (false, _) => target,
                    (true, true) => {
                        target.with_neighbour(Direction::Right, MenuChoice::ALL[index + 3])
                    }
                    (true, false) => {
                        target.with_neighbour(Direction::Left, MenuChoice::ALL[index - 3])
                    }
                }
            })
            .collect()
    }

    /// Takes a pad button or the surroundings in; `true` when `input` was
    /// one, which nothing else on the menu acts on.
    pub(super) fn pad_input(&mut self, input: &Input) -> bool {
        let meant = self.reader.follow(input);
        if let Input::Surroundings(surroundings) = *input {
            self.focus
                .surroundings(surroundings.device, Some(MenuChoice::NewPilot), false);
            return true;
        }
        if !pad_focus::from_pad(input) {
            return false;
        }
        if let Some(meant) = meant {
            self.pad_command(meant);
        }
        true
    }

    fn pad_command(&mut self, input: CommandInput) {
        match self.focus.read_pad(input) {
            PadRead::Move(mv) => {
                self.drop_pad_press();
                self.focus
                    .move_by_pad(&self.pad_targets(), MenuChoice::NewPilot, mv);
            }
            PadRead::Press => self.press_focused(),
            PadRead::Release => self.release_focused(),
            // East, and the rest, do nothing, as their keys do.
            PadRead::Nothing | PadRead::Other => {}
        }
    }

    /// South went down: a primary press on the focused button.
    fn press_focused(&mut self) {
        let Some(choice) = self.pad_highlight() else {
            return;
        };
        self.skip_slides();
        self.tracker.cancel();
        let tracked = self.tracker.press(Some(choice.index()));
        self.tracked(tracked);
        self.pad_pressed = true;
    }

    /// South came up: lets go of its press over its button, choosing it.
    fn release_focused(&mut self) {
        if !std::mem::take(&mut self.pad_pressed) {
            return;
        }
        let tracked = self.tracker.release(self.tracker.pressed());
        self.tracked(tracked);
    }

    /// A focus move drops a held South, silently.
    fn drop_pad_press(&mut self) {
        if std::mem::take(&mut self.pad_pressed) {
            self.tracker.cancel();
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
