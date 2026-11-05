//! The in-flight maps' pad focus over their six buttons
//! (rdm:phase/key-bindings/phase-10-gamepad-map).
//!
//! The original moves no focus between the map's buttons: `_MapFilter`
//! (@0x13c63) turns keys into item hits and Tab cycles the selection, and
//! the binary imports neither `SetKeyboardFocus` nor
//! `AdvanceKeyboardFocus` (the phase's findings). So the pad's focus here
//! is a new feature with no `Rulebook` key, and a focus move is silent.
//!
//! - **Targets.** The enabled buttons among the six ([`ButtonState::enabled`]),
//!   each at its [`MapButtons::rect`], in `_DrawMapButtons`' order ([`ORDER`]):
//!   Done, "-", "+", Clear Route, Borders, Find. Item 3, the map itself, is
//!   none. A focus left on a button that disables (the zoom at its limit)
//!   draws nothing, and the next move lands on Done.
//! - **Initial target.** Done, the item Return and Escape give.
//! - **South** presses the focused button as a click does: drawn pressed
//!   while South is down, acting on its release, silent as
//!   `_TrackMapButtons` is. A pad's South no longer centres the map;
//!   keyboard Space still does.
//! - **The shoulders** keep their Tab meanings: the right one cycles the
//!   selection, the left one steps it back as Tab Back does. Neither moves
//!   the button focus, so neither is held back before the map is told the
//!   pad.
//! - **Everything else** the pad means (the left stick's arrows, the
//!   triggers' zoom, East) goes on to the map as its key's.
//!
//! The developer viewer has no buttons and no pad focus.

use crate::color::Color;
use crate::draw::DrawList;
use crate::input::{Device, Input};
use crate::keymap::{Command, CommandInput, MapCommand, UiCommand};
use crate::ui::pad_focus::{FocusState, PadRead, Target, Told, draw_highlight, from_pad};

use super::buttons::{ButtonState, MapButton, MapButtons, ORDER};

/// The control the pad's focus starts on: Done.
pub const INITIAL: MapButton = MapButton::Done;

/// What the map does with a pad input ([`MapPad::read`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadStep {
    /// Nothing more: the pad's focus took it, or it meant nothing.
    Taken,
    /// Press this button, as a click on it does.
    Press(MapButton),
    /// Act on this command as its key's.
    Command(CommandInput),
}

/// An in-flight map's pad focus over its six buttons.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapPad {
    focus: FocusState<MapButton>,
    told: Told,
    /// The button South went down on, pressed keyed, until South comes up.
    armed: Option<MapButton>,
}

/// The enabled buttons among the six, in [`ORDER`].
fn targets(buttons: &MapButtons, state: ButtonState) -> Vec<Target<MapButton>> {
    ORDER
        .into_iter()
        .filter(|&button| state.enabled(button))
        .map(|button| Target::at(button, buttons.rect(button)))
        .collect()
}

impl MapPad {
    /// Keeps `input` if it is the surroundings, and moves the focus as
    /// they say; whether it was. The focus lands on Done as the pad is
    /// told, unless the map is the viewer (`buttons` false).
    pub fn surroundings(&mut self, input: &Input, buttons: bool) -> bool {
        if !self.told.keep(input) {
            return false;
        }
        let initial = buttons.then_some(INITIAL);
        self.focus.surroundings(self.told.device(), initial, false);
        true
    }

    /// The surroundings kept, to tell a screen the map opens.
    #[must_use]
    pub const fn told(&self) -> Told {
        self.told
    }

    /// The device the player used last, as the map was told; the
    /// keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.told.device()
    }

    /// A map opening: a fresh focus, on Done if the pad was used last, and
    /// no press armed.
    pub fn opened(&mut self, buttons: &mut MapButtons) {
        self.let_go(buttons);
        self.focus = FocusState::default();
        self.focus
            .surroundings(self.told.device(), Some(INITIAL), false);
    }

    /// Drops a press South armed, unpressing its button.
    pub fn let_go(&mut self, buttons: &mut MapButtons) {
        if self.armed.take().is_some() {
            buttons.cancel_pointer();
        }
    }

    /// Forgets a press South armed, its button already let go.
    pub const fn forget_press(&mut self) {
        self.armed = None;
    }

    /// What the pad's `input`, meaning `meant`, does on an in-flight map
    /// whose buttons are `buttons` in `state`; `None` when `input` is no
    /// pad button's, for the map to handle as it always has.
    pub fn read(
        &mut self,
        input: &Input,
        meant: Option<CommandInput>,
        buttons: &mut MapButtons,
        state: &ButtonState,
    ) -> Option<PadStep> {
        if !from_pad(input) {
            return None;
        }
        let Some(meant) = meant else {
            return Some(PadStep::Taken);
        };
        // The shoulders keep their Tab meanings: the cycle, and Tab Back.
        match meant.command {
            Command::Ui(UiCommand::Tab) => return Some(PadStep::Command(meant)),
            Command::Ui(UiCommand::PrevFocus) => {
                return Some(PadStep::Command(CommandInput {
                    command: Command::Map(MapCommand::TabBack),
                    ..meant
                }));
            }
            _ => {}
        }
        let targets = targets(buttons, *state);
        Some(match self.focus.read_pad(meant) {
            PadRead::Move(mv) => {
                self.let_go(buttons);
                self.focus.move_by_pad(&targets, INITIAL, mv);
                PadStep::Taken
            }
            PadRead::Press => {
                if let Some(button) = self.focus.get()
                    && targets.iter().any(|target| target.id == button)
                {
                    buttons.press_keyed(button, state);
                    self.armed = Some(button);
                }
                PadStep::Taken
            }
            PadRead::Release => self
                .armed
                .take()
                .and_then(|button| buttons.release_keyed(button))
                .map_or(PadStep::Taken, PadStep::Press),
            PadRead::Nothing => PadStep::Taken,
            PadRead::Other => PadStep::Command(meant),
        })
    }

    /// Draws the pad's highlight round the focused button, while the pad
    /// placed it, was used last, and the button is enabled.
    pub fn draw(
        &self,
        buttons: &MapButtons,
        state: &ButtonState,
        color: Color,
        list: &mut DrawList,
    ) {
        if let Some(button) = self.focus.pad_highlight()
            && state.enabled(button)
        {
            draw_highlight(list, buttons.rect(button), color);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
