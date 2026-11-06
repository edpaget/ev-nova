//! The touch side: flight's on-screen touch controls as a device of their
//! own, the keymap's third beside the keyboard and the gamepad
//! (rdm:phase/key-bindings/phase-15-touch-flight-controls), and the only
//! keymap file that reads an [`Input::TouchControl`].
//!
//! - A control names its command outright, so a button's press means its
//!   command while that command's context is active, and nothing
//!   otherwise: a group button does nothing while the escort menu is
//!   shut, as a key bound in an inactive context does.
//! - [`TouchDevice`] holds the commands whose controls are down, in the
//!   order pressed, what each press meant, and the stick's raw position.
//! - The stick reads as the pad's flight axes: Turn is its x and Throttle
//!   its y, through the flight dead zone ([`dead_zoned`]). The
//!   reader takes the larger of the pad's and the stick's amounts
//!   ([`Bindings::axis`](super::Bindings::axis)).
//!
//! The overlay (`crate::flight::touch_controls`) decides when a control
//! goes down or up; this module only follows what it says.

use std::collections::BTreeSet;

use super::bindings::Change;
use super::command::{Command, Context, FlightCommand};
use super::gamepad::dead_zoned;
use crate::input::{Input, TouchControl};

/// The touch controls' device state: the commands down, in the order
/// pressed, what each press meant, and the stick's raw position.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TouchDevice {
    /// The commands whose controls are down, in the order pressed.
    down: Vec<Command>,
    /// The commands whose press meant them, while down.
    presses: BTreeSet<Command>,
    /// The stick's raw offset, x right and y forward; centred at rest.
    stick: (f32, f32),
}

impl TouchDevice {
    /// Follows a touch control going down or up, or the stick moving;
    /// anything else changes nothing. Says whether it was the touch
    /// controls' input.
    pub fn follow(&mut self, input: &Input) -> bool {
        match *input {
            Input::TouchControl(TouchControl::Button {
                command,
                pressed: true,
            }) => {
                if !self.down.contains(&command) {
                    self.down.push(command);
                }
                true
            }
            Input::TouchControl(TouchControl::Button {
                command,
                pressed: false,
            }) => {
                self.down.retain(|&down| down != command);
                self.presses.remove(&command);
                true
            }
            Input::TouchControl(TouchControl::Stick { x, y }) => {
                self.stick = (x, y);
                true
            }
            _ => false,
        }
    }

    /// Notes that `input`, a touch button's press, meant `command`.
    pub fn note_press(&mut self, input: &Input, command: Option<Command>) {
        if let Some((button, true)) = button(input) {
            if command.is_some() {
                self.presses.insert(button);
            } else {
                self.presses.remove(&button);
            }
        }
    }

    /// What the press of the touch button `input` presses or releases
    /// meant, if it was noted and is still down.
    #[must_use]
    pub fn press_of(&self, input: &Input) -> Option<Command> {
        let (command, _) = button(input)?;
        self.presses.contains(&command).then_some(command)
    }

    /// Whether the control holding `command` is down.
    #[must_use]
    pub fn is_down(&self, command: Command) -> bool {
        self.down.contains(&command)
    }

    /// The commands whose controls are down, in the order pressed.
    pub fn buttons(&self) -> impl Iterator<Item = Command> + '_ {
        self.down.iter().copied()
    }

    /// The stick's raw offset, x right and y forward.
    #[must_use]
    pub const fn stick(&self) -> (f32, f32) {
        self.stick
    }

    /// Lets go of every control and centres the stick.
    pub fn let_go(&mut self) {
        self.down.clear();
        self.presses.clear();
        self.stick = (0.0, 0.0);
    }

    /// How far the stick pushes the flight axis `command`, through the
    /// flight dead zone: Turn its x, Throttle its y, none for any other
    /// command.
    #[must_use]
    pub fn axis_amount(&self, command: Command) -> f32 {
        let (x, y) = dead_zoned(self.stick.0, self.stick.1);
        match command {
            Command::Flight(FlightCommand::TurnAxis) => x,
            Command::Flight(FlightCommand::ThrottleAxis) => y,
            _ => 0.0,
        }
    }

    /// Whether the stick lies outside the flight dead zone.
    #[must_use]
    pub fn flight_axis_pushed(&self) -> bool {
        dead_zoned(self.stick.0, self.stick.1) != (0.0, 0.0)
    }

    /// Every command the controls down hold in the `active` contexts.
    #[must_use]
    pub fn held(&self, active: &[Context]) -> BTreeSet<Command> {
        self.down
            .iter()
            .copied()
            .filter(|command| active.contains(&command.context()))
            .collect()
    }
}

/// The command and the direction of the touch button `input`, if it is
/// one.
const fn button(input: &Input) -> Option<(Command, bool)> {
    match *input {
        Input::TouchControl(TouchControl::Button { command, pressed }) => Some((command, pressed)),
        _ => None,
    }
}

/// How `input`, a touch button going down or up, changes what is down;
/// `None` for any other input. A touch control never repeats.
pub(super) fn change(input: &Input) -> Option<Change> {
    let (_, pressed) = button(input)?;
    Some(Change {
        pressed,
        repeat: false,
    })
}

/// A fresh press of the touch button of `input`; `None` for any other
/// input.
pub(super) fn fresh_press(input: &Input) -> Option<Input> {
    let (command, _) = button(input)?;
    Some(Input::TouchControl(TouchControl::Button {
        command,
        pressed: true,
    }))
}

/// The command the touch button of `input` (a press, a repeat or a
/// release) holds in the `active` contexts: its own, while its context is
/// active. Empty for any other input.
pub(super) fn meanings(input: &Input, active: &[Context]) -> Vec<Command> {
    button(input)
        .map(|(command, _)| command)
        .filter(|command| active.contains(&command.context()))
        .into_iter()
        .collect()
}

/// The command a touch button's press `input` means in the `active`
/// contexts; a release, or any other input, means none.
pub(super) fn resolve(input: &Input, active: &[Context]) -> Option<Command> {
    match button(input) {
        Some((_, true)) => meanings(input, active).first().copied(),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::command::{EscortMenuCommand, PlayCommand};
    use super::super::contexts::FLIGHT;
    use super::super::reader::CommandReader;
    use super::*;
    use crate::input::{Key, PadAxis, PadButton};

    fn touch(command: impl Into<Command>, pressed: bool) -> Input {
        Input::TouchControl(TouchControl::Button {
            command: command.into(),
            pressed,
        })
    }

    fn stick(x: f32, y: f32) -> Input {
        Input::TouchControl(TouchControl::Stick { x, y })
    }

    fn flight() -> CommandReader {
        CommandReader::new(FLIGHT)
    }

    const FIRE: FlightCommand = FlightCommand::FirePrimary;

    #[test]
    fn a_touch_button_means_its_command_in_an_active_context() {
        let mut reader = flight();
        let pressed = reader.follow(&touch(FIRE, true)).expect("a command");
        assert_eq!(pressed.command, FIRE.into());
        assert!(pressed.pressed && !pressed.repeat);
        let map = reader
            .follow(&touch(PlayCommand::GalaxyMap, true))
            .expect("a command");
        assert_eq!(map.command, PlayCommand::GalaxyMap.into());
    }

    #[test]
    fn a_touch_button_in_an_inactive_context_means_nothing() {
        let mut reader = flight();
        let group = touch(EscortMenuCommand::Group1, true);
        assert_eq!(reader.peek(&group), None);
        assert_eq!(reader.follow(&group), None);
        assert!(!reader.held(EscortMenuCommand::Group1));
        assert_eq!(reader.meanings(&group), []);
        assert_eq!(reader.bound_to(&group), []);
        reader.set_active(Context::EscortMenu, true);
        assert!(reader.held(EscortMenuCommand::Group1), "held once shown");
        assert_eq!(reader.peek(&group), Some(EscortMenuCommand::Group1.into()));
    }

    #[test]
    fn a_touch_button_holds_its_command_until_its_release() {
        let mut reader = flight();
        reader.follow(&touch(FIRE, true));
        assert!(reader.held(FIRE));
        assert!(reader.is_down(&touch(FIRE, false)));
        assert!(reader.anything_down());
        let released = reader.follow(&touch(FIRE, false)).expect("its release");
        assert_eq!(released.command, FIRE.into());
        assert!(!released.pressed);
        assert!(!reader.held(FIRE));
        assert!(!reader.anything_down());
    }

    #[test]
    fn the_touch_stick_gives_dead_zoned_turn_and_throttle_axes() {
        let mut reader = flight();
        assert_eq!(reader.follow(&stick(0.5, -0.8)), None, "presses nothing");
        let (x, y) = dead_zoned(0.5, -0.8);
        assert_eq!(reader.axis(FlightCommand::TurnAxis), x);
        assert_eq!(reader.axis(FlightCommand::ThrottleAxis), y);
        assert!(x > 0.0 && y < 0.0);
        assert_eq!(reader.axis(FlightCommand::Accelerate), 0.0);
    }

    #[test]
    fn inside_the_dead_zone_the_touch_stick_gives_none() {
        let mut reader = flight();
        reader.follow(&stick(0.1, 0.1));
        assert_eq!(reader.axis(FlightCommand::TurnAxis), 0.0);
        assert_eq!(reader.axis(FlightCommand::ThrottleAxis), 0.0);
        assert!(!reader.flight_axis_pushed());
    }

    #[test]
    fn the_larger_of_the_pads_and_the_sticks_amounts_wins() {
        let mut reader = flight();
        reader.follow(&Input::PadAxis {
            axis: PadAxis::RightX,
            value: -0.4,
        });
        reader.follow(&stick(0.9, 0.0));
        assert_eq!(reader.axis(FlightCommand::TurnAxis), dead_zoned(0.9, 0.0).0);
        reader.follow(&stick(0.2, 0.0));
        assert_eq!(
            reader.axis(FlightCommand::TurnAxis),
            dead_zoned(-0.4, 0.0).0,
            "the pad's, larger the other way"
        );
    }

    #[test]
    fn on_a_tie_the_pads_amount_stays() {
        let mut reader = flight();
        reader.follow(&Input::PadAxis {
            axis: PadAxis::RightX,
            value: -0.5,
        });
        reader.follow(&stick(0.5, 0.0));
        assert_eq!(
            reader.axis(FlightCommand::TurnAxis),
            dead_zoned(-0.5, 0.0).0
        );
    }

    #[test]
    fn only_a_touch_button_held_is_down() {
        let mut reader = flight();
        assert!(!reader.is_down(&touch(FIRE, true)));
        reader.follow(&touch(FlightCommand::Afterburner, true));
        assert!(!reader.is_down(&touch(FIRE, true)), "another is down");
        assert!(reader.is_down(&touch(FlightCommand::Afterburner, false)));
    }

    #[test]
    fn a_touch_stick_outside_the_dead_zone_pushes_a_flight_axis() {
        let mut reader = flight();
        reader.follow(&stick(0.0, 0.5));
        assert!(reader.flight_axis_pushed());
    }

    #[test]
    fn let_go_drops_the_touch_buttons_and_centres_the_stick() {
        let mut reader = flight();
        reader.follow(&touch(FIRE, true));
        reader.follow(&stick(0.9, 0.9));
        reader.let_go();
        assert!(!reader.held(FIRE));
        assert!(!reader.anything_down());
        assert_eq!(reader.axis(FlightCommand::TurnAxis), 0.0);
        assert!(!reader.flight_axis_pushed());
        assert_eq!(reader.follow(&touch(FIRE, false)), None, "forgotten");
    }

    #[test]
    fn down_lists_touch_buttons_after_keys_and_pad_buttons() {
        let mut reader = flight();
        reader.follow(&touch(FIRE, true));
        reader.follow(&Input::PadButton {
            button: PadButton::South,
            pressed: true,
        });
        reader.follow(&Input::Key {
            key: Key::KeyA,
            pressed: true,
            repeat: false,
        });
        reader.follow(&touch(FlightCommand::Afterburner, true));
        assert_eq!(
            reader.down(),
            [
                Input::Key {
                    key: Key::KeyA,
                    pressed: true,
                    repeat: false,
                },
                Input::PadButton {
                    button: PadButton::South,
                    pressed: true,
                },
                touch(FIRE, true),
                touch(FlightCommand::Afterburner, true),
            ]
        );
    }

    #[test]
    fn means_and_peek_read_a_touch_button() {
        let mut reader = flight();
        assert_eq!(reader.peek(&touch(FIRE, true)), Some(FIRE.into()));
        assert_eq!(reader.peek(&touch(FIRE, false)), None, "a release");
        assert_eq!(reader.means(&touch(FIRE, false)), Some(FIRE.into()));
        reader.follow(&touch(FIRE, true));
        assert_eq!(reader.means(&touch(FIRE, false)), Some(FIRE.into()));
        assert_eq!(reader.meanings(&touch(FIRE, true)), [Command::from(FIRE)]);
        assert_eq!(reader.bound_to(&touch(FIRE, false)), [Command::from(FIRE)]);
        assert_eq!(reader.means(&stick(1.0, 0.0)), None);
    }

    #[test]
    fn the_touch_device_follows_only_touch_controls() {
        let mut device = TouchDevice::default();
        assert!(!device.follow(&Input::Text('a')));
        assert!(device.follow(&stick(0.3, 0.0)));
        assert_eq!(device.stick(), (0.3, 0.0));
        assert!(device.follow(&touch(FIRE, true)));
        assert!(device.follow(&touch(FIRE, true)), "a second press");
        assert_eq!(device.buttons().collect::<Vec<_>>(), [Command::from(FIRE)]);
        assert!(device.is_down(FIRE.into()));
    }
}
