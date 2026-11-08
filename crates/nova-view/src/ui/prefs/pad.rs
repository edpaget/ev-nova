//! Preferences' pad focus (rdm:phase/key-bindings/phase-8-gamepad-focus).
//!
//! - **Targets**, in [`Focus::next`]'s order: the ten toggles, each
//!   stepper's two arrows in its part's place (the display picker, the
//!   Scaling list, the effects volume, the music volume), then Key
//!   Settings and OK: twenty in all. A stepper's arrows are targets of
//!   their own, so South presses each as a click on it does.
//! - **The initial target** is OK: the original's Preferences window
//!   (`Keys.nib`'s `EVPrefsDialogController` window) names no
//!   `initialFirstResponder` and authors no `nextKeyView` loop, and OK is
//!   its default button (the phase's step-1 findings).
//! - **The d-pad** picks by geometry; **the shoulders** step in the
//!   keyboard's Tab order. No layout here misleads the picker, so no
//!   target names an explicit neighbour.
//! - **South** presses the focused toggle, arrow or button as a click
//!   does, on its release; **the left stick** steps whichever stepper
//!   holds the focus, as the arrow keys do; **East** closes, as Escape
//!   does.
//! - **Until Preferences is told** the pad was the device used last, the
//!   pad's focus moves and South do nothing, as the highlight can't show
//!   ([`pad_focus`]'s "Nothing hidden acts"); East and the left stick act.
//! - The pad acts under either [`FocusRing`] reading. The keyboard keeps
//!   its rules: under [`FocusRing::Off`] it has no focus, and under
//!   [`FocusRing::Tab`] Tab goes from any target to the next part.
//! - **The inner [`Dialog`](super::super::dialog::Dialog)** is never told
//!   the surroundings: Preferences owns the focus over OK and Key
//!   Settings, so the dialog draws no highlight of its own.

use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::input::Device;
use crate::keymap::{Command, CommandInput, UiCommand};

use super::super::dialog::DialogEvent;
use super::super::pad_focus::{self, FocusMove, PadRead, Target};
use super::super::volume::Arrow;
use super::{Focus, FocusRing, KEY_SETTINGS_ITEM, OK_ITEM, PrefsDialog};

/// Where the pad's focus rests on Preferences: a part, or one arrow of a
/// stepper's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefsTarget {
    /// A toggle, Key Settings or OK.
    Part(Focus),
    /// One of a stepper's two arrows.
    Arrow(Focus, Arrow),
}

impl PrefsTarget {
    /// The part it belongs to.
    #[must_use]
    pub const fn part(self) -> Focus {
        match self {
            Self::Part(part) | Self::Arrow(part, _) => part,
        }
    }
}

/// The toggles, in [`Focus::next`]'s order.
const TOGGLES: [Focus; 10] = [
    Focus::Music,
    Focus::Sound,
    Focus::HyperspaceEffects,
    Focus::QuickTimeMovies,
    Focus::ShipAnimations,
    Focus::EngineGlows,
    Focus::RunningLights,
    Focus::WeaponEffects,
    Focus::SmokeTrails,
    Focus::ParallaxStarfield,
];

/// The steppers, in [`Focus::next`]'s order.
const STEPPERS: [Focus; 4] = [
    Focus::Display,
    Focus::Scaling,
    Focus::EffectsVolume,
    Focus::MusicVolume,
];

impl PrefsDialog {
    /// The device the player used last, as the last surroundings it was
    /// told said; the keyboard before any.
    #[must_use]
    pub fn device(&self) -> Device {
        self.focus.device()
    }

    /// The target the focus is on, an arrow included, whoever placed it.
    #[must_use]
    pub fn pad_target(&self) -> Option<PrefsTarget> {
        self.focus.get()
    }

    /// Every target, in order: see the module docs.
    pub(crate) fn pad_targets(&self) -> Vec<Target<PrefsTarget>> {
        let mut targets: Vec<Target<PrefsTarget>> = TOGGLES
            .into_iter()
            .filter_map(|part| {
                let toggle = self.toggle(part)?;
                Some(Target::at(PrefsTarget::Part(part), toggle.rect()))
            })
            .collect();
        for part in STEPPERS {
            for arrow in [Arrow::Up, Arrow::Down] {
                targets.push(Target::at(
                    PrefsTarget::Arrow(part, arrow),
                    self.arrow_rect(part, arrow),
                ));
            }
        }
        for (part, item) in [
            (Focus::KeySettings, KEY_SETTINGS_ITEM),
            (Focus::Ok, OK_ITEM),
        ] {
            if let Some(bounds) = self.dialog.item_bounds(item) {
                targets.push(Target::at(PrefsTarget::Part(part), bounds));
            }
        }
        targets
    }

    /// Where a stepper's `arrow` is.
    fn arrow_rect(&self, part: Focus, arrow: Arrow) -> Bounds {
        let (up, down) = match part {
            Focus::Display => (self.display.rects().up, self.display.rects().down),
            Focus::Scaling => {
                let rects = self.scaling_list.picker.rects();
                (rects.up, rects.down)
            }
            Focus::EffectsVolume => (
                self.effects_volume.rects().up,
                self.effects_volume.rects().down,
            ),
            _ => (self.music_volume.rects().up, self.music_volume.rects().down),
        };
        match arrow {
            Arrow::Up => up,
            Arrow::Down => down,
        }
    }

    /// The device used last changed: the pad's focus lands on OK.
    pub(super) fn pad_surroundings(&mut self, device: Device) {
        self.focus
            .surroundings(device, Some(PrefsTarget::Part(Focus::Ok)), false);
    }

    /// A focus move a key is bound to, under [`FocusRing::Tab`]: it moves
    /// the one focus as the pad's would, as the keyboard's.
    pub(super) fn keyed_focus_move(&mut self, command: Command) {
        debug_assert_eq!(self.focus_ring, FocusRing::Tab);
        if let Some(mv) = FocusMove::of(command) {
            self.focus
                .move_by_keys(&self.pad_targets(), PrefsTarget::Part(Focus::Ok), mv);
        }
    }

    /// Acts on what a pad button meant: see the module docs.
    pub(super) fn pad_command(&mut self, input: CommandInput) {
        match self.focus.read_pad(input) {
            PadRead::Nothing => {}
            PadRead::Move(mv) => {
                self.drop_pad_press();
                self.focus
                    .move_by_pad(&self.pad_targets(), PrefsTarget::Part(Focus::Ok), mv);
            }
            PadRead::Press => self.press_focused(),
            PadRead::Release => self.release_pressed(),
            PadRead::Other => self.pad_other(input),
        }
    }

    /// A pad command that is no focus move and no South: the steppers'
    /// arrows on a focused stepper, and the rest as their keys'.
    fn pad_other(&mut self, input: CommandInput) {
        match input.command {
            Command::Ui(UiCommand::Up | UiCommand::Down | UiCommand::Left | UiCommand::Right) => {
                if let Some(part) = self.focus().filter(|part| STEPPERS.contains(part)) {
                    self.part_command(part, input);
                }
            }
            _ => self.command(input),
        }
    }

    /// South went down: arms the focused target, drawn pressed.
    fn press_focused(&mut self) {
        let Some(target) = self.focus.get() else {
            return;
        };
        self.drop_pad_press();
        match target {
            PrefsTarget::Part(Focus::Ok) => self.dialog.press_item_keyed(OK_ITEM),
            PrefsTarget::Part(Focus::KeySettings) => {
                self.dialog.press_item_keyed(KEY_SETTINGS_ITEM);
            }
            PrefsTarget::Part(part) => match self.toggle_mut(part) {
                Some(toggle) => toggle.press_keyed(),
                None => return,
            },
            PrefsTarget::Arrow(Focus::Display, arrow) => self.display.press_keyed(arrow),
            PrefsTarget::Arrow(Focus::Scaling, arrow) => {
                self.scaling_list.picker.press_keyed(arrow);
            }
            PrefsTarget::Arrow(Focus::EffectsVolume, arrow) => {
                self.effects_volume.press_keyed(arrow);
            }
            PrefsTarget::Arrow(_, arrow) => self.music_volume.press_keyed(arrow),
        }
        self.pad_pressed = Some(target);
    }

    /// South came up: activates what its press armed, as a click's
    /// release does.
    fn release_pressed(&mut self) {
        let Some(target) = self.pad_pressed.take() else {
            return;
        };
        match target {
            PrefsTarget::Part(Focus::Ok | Focus::KeySettings) => {
                match self.dialog.release_item_keyed() {
                    Some(DialogEvent::Item(OK_ITEM)) => self.close(),
                    Some(DialogEvent::Item(KEY_SETTINGS_ITEM)) => self.open_keys_screen(),
                    _ => {}
                }
            }
            PrefsTarget::Part(part) => {
                if let Some(toggle) = self.toggle_mut(part) {
                    self.changed |= toggle.release_keyed();
                }
            }
            PrefsTarget::Arrow(Focus::Display, _) => self.stepped |= self.display.release_keyed(),
            PrefsTarget::Arrow(Focus::Scaling, _) => {
                self.scaling_list.stepped |= self.scaling_list.picker.release_keyed();
            }
            PrefsTarget::Arrow(Focus::EffectsVolume, _) => {
                self.changed |= self.effects_volume.release_keyed();
            }
            PrefsTarget::Arrow(..) => self.changed |= self.music_volume.release_keyed(),
        }
    }

    /// A focus move drops a held press, silently.
    fn drop_pad_press(&mut self) {
        if self.pad_pressed.take().is_some() {
            self.cancel_parts();
        }
    }

    /// Draws the pad's highlight round the target it is on, if it shows.
    pub(super) fn draw_pad_highlight(&self, list: &mut DrawList) {
        let Some(on) = self.focus.pad_highlight() else {
            return;
        };
        if let Some(target) = self.pad_targets().iter().find(|target| target.id == on) {
            pad_focus::draw_highlight(list, target.bounds, self.style.up);
        }
    }
}

#[cfg(test)]
mod tests;
