//! The device the player used last ([`LastDevice`]): what the app tells
//! the screens as [`Surroundings::device`](nova_view::Surroundings), so
//! prompts and the pad's focus highlight follow the hand on the controls.
//!
//! It becomes:
//!
//! - `Keyboard` on a key's press;
//! - `Mouse` on a pointer button, a move or a scroll;
//! - `Gamepad(Generic)` on a pad button's press, a trigger pulled past
//!   the press threshold, or a stick leaving the flight dead zone (analog
//!   flight input below the press threshold is real input). A stick
//!   moving within the dead zone, or staying outside it, changes nothing.
//!
//! Releases change nothing. The family is `Generic` until phase 12
//! (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs) reads it from
//! the pad.

use std::collections::BTreeMap;

use nova_view::keymap::dead_zoned;
use nova_view::{ControllerFamily, Device, Input, PadAxis};

use crate::gamepad::inputs::PRESS_THRESHOLD;
use crate::gamepad::{PadEvent, PadId};

/// The device the player used last, and what it reads to tell.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LastDevice {
    /// The device used last; the keyboard before any.
    device: Device,
    /// Each pad's last value of each stick axis.
    axes: BTreeMap<(PadId, PadAxis), f32>,
}

/// The device a pad's use makes it.
const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

impl LastDevice {
    /// The keyboard, before any input.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The device used last.
    #[must_use]
    pub const fn device(&self) -> Device {
        self.device
    }

    /// Follows a screen input from the keyboard or the pointer, and says
    /// the device it switched to, if it switched.
    pub fn input(&mut self, input: &Input) -> Option<Device> {
        let used = match *input {
            Input::Key { pressed: true, .. } => Device::Keyboard,
            Input::PointerButton { .. }
            | Input::PointerMoved(_)
            | Input::PointerMovedOutside(_)
            | Input::Scroll { .. } => Device::Mouse,
            Input::PadButton { pressed: true, .. } => PAD,
            _ => return None,
        };
        self.switch(used)
    }

    /// Follows what a pad did, and says the device it switched to, if it
    /// switched.
    pub fn pad_event(&mut self, event: &PadEvent) -> Option<Device> {
        let used = match *event {
            PadEvent::Button { pressed: true, .. } => true,
            PadEvent::Trigger { value, .. } => value >= PRESS_THRESHOLD,
            PadEvent::Axis { pad, axis, value } => self.leaves_dead_zone(pad, axis, value),
            PadEvent::Disconnected(pad) => {
                self.axes.retain(|&(of, _), _| of != pad);
                false
            }
            PadEvent::Button { .. } | PadEvent::Connected(_) => false,
        };
        if used { self.switch(PAD) } else { None }
    }

    /// Whether `pad`'s stick, with `axis` moved to `value`, has just left
    /// the flight dead zone.
    fn leaves_dead_zone(&mut self, pad: PadId, axis: PadAxis, value: f32) -> bool {
        let across = self.axes.get(&(pad, axis.across())).copied().unwrap_or(0.0);
        let before = self.axes.insert((pad, axis), value).unwrap_or(0.0);
        let outside = |along: f32| dead_zoned(along, across) != (0.0, 0.0);
        outside(value) && !outside(before)
    }

    /// Makes `used` the device, saying so if it changed.
    fn switch(&mut self, used: Device) -> Option<Device> {
        (used != self.device).then(|| {
            self.device = used;
            used
        })
    }
}

#[cfg(test)]
mod tests {
    use nova_view::{Key, MouseButton, PadButton, Point, ScrollDelta};

    use super::*;

    const ONE: PadId = PadId(0);

    fn key(pressed: bool) -> Input {
        Input::Key {
            key: Key::KeyA,
            pressed,
            repeat: false,
        }
    }

    fn stick(axis: PadAxis, value: f32) -> PadEvent {
        PadEvent::Axis {
            pad: ONE,
            axis,
            value,
        }
    }

    #[test]
    fn it_starts_on_the_keyboard() {
        assert_eq!(LastDevice::new().device(), Device::Keyboard);
    }

    #[test]
    fn a_key_press_makes_it_keyboard() {
        let mut last = LastDevice::new();
        last.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        assert_eq!(last.input(&key(true)), Some(Device::Keyboard));
        assert_eq!(last.input(&key(true)), None, "no change");
        assert_eq!(last.device(), Device::Keyboard);
    }

    #[test]
    fn a_pointer_makes_it_mouse() {
        for input in [
            Input::PointerMoved(Point::new(1.0, 1.0)),
            Input::PointerMovedOutside(Point::new(-1.0, 1.0)),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at: Point::new(1.0, 1.0),
            },
            Input::Scroll {
                delta: ScrollDelta::Lines(1.0),
                at: Point::new(1.0, 1.0),
            },
        ] {
            let mut last = LastDevice::new();
            assert_eq!(last.input(&input), Some(Device::Mouse), "{input:?}");
        }
    }

    #[test]
    fn a_pad_button_makes_it_gamepad() {
        let mut last = LastDevice::new();
        let press = PadEvent::Button {
            pad: ONE,
            button: PadButton::South,
            pressed: true,
        };
        assert_eq!(last.pad_event(&PadEvent::Connected(ONE)), None);
        assert_eq!(last.pad_event(&press), Some(PAD));
        let mut screen_side = LastDevice::new();
        assert_eq!(
            screen_side.input(&Input::PadButton {
                button: PadButton::South,
                pressed: true
            }),
            Some(PAD)
        );
    }

    #[test]
    fn a_stick_just_past_the_dead_zone_below_the_press_threshold_makes_it_gamepad() {
        let mut last = LastDevice::new();
        let just_past = nova_view::keymap::FLIGHT_DEAD_ZONE + 0.01;
        assert!(just_past < PRESS_THRESHOLD);
        assert_eq!(
            last.pad_event(&stick(PadAxis::RightX, just_past)),
            Some(PAD)
        );
    }

    #[test]
    fn a_stick_inside_the_dead_zone_does_not() {
        let mut last = LastDevice::new();
        for value in [0.05, 0.1, -0.12, 0.0] {
            assert_eq!(last.pad_event(&stick(PadAxis::LeftY, value)), None);
        }
        assert_eq!(last.device(), Device::Keyboard);
    }

    #[test]
    fn a_stick_staying_outside_the_dead_zone_does_not_switch_it_back() {
        let mut last = LastDevice::new();
        last.pad_event(&stick(PadAxis::LeftY, 0.8));
        last.input(&key(true));
        assert_eq!(last.pad_event(&stick(PadAxis::LeftY, 0.9)), None);
        assert_eq!(last.device(), Device::Keyboard);
        last.pad_event(&stick(PadAxis::LeftY, 0.0));
        assert_eq!(
            last.pad_event(&stick(PadAxis::LeftY, -0.6)),
            Some(PAD),
            "back in, then out again"
        );
    }

    #[test]
    fn the_dead_zone_is_the_sticks_radial_one() {
        let mut last = LastDevice::new();
        assert_eq!(last.pad_event(&stick(PadAxis::LeftX, 0.12)), None);
        assert_eq!(
            last.pad_event(&stick(PadAxis::LeftY, 0.12)),
            Some(PAD),
            "0.12 both ways is past 0.15 from the centre"
        );
    }

    #[test]
    fn a_lost_pad_forgets_its_sticks() {
        let mut last = LastDevice::new();
        last.pad_event(&stick(PadAxis::RightX, 0.9));
        last.input(&key(true));
        last.pad_event(&PadEvent::Disconnected(ONE));
        assert_eq!(
            last.pad_event(&stick(PadAxis::RightX, 0.9)),
            Some(PAD),
            "a pad found again starts centred"
        );
    }

    #[test]
    fn a_trigger_past_the_threshold_makes_it_gamepad_and_below_it_does_not() {
        let mut last = LastDevice::new();
        let trigger = |value| PadEvent::Trigger {
            pad: ONE,
            button: PadButton::LeftTrigger,
            value,
        };
        assert_eq!(last.pad_event(&trigger(0.3)), None);
        assert_eq!(last.pad_event(&trigger(PRESS_THRESHOLD)), Some(PAD));
    }

    #[test]
    fn a_release_changes_nothing() {
        let mut last = LastDevice::new();
        last.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        assert_eq!(last.input(&key(false)), None);
        assert_eq!(
            last.pad_event(&PadEvent::Button {
                pad: ONE,
                button: PadButton::East,
                pressed: false,
            }),
            None
        );
        assert_eq!(
            last.input(&Input::PadButton {
                button: PadButton::East,
                pressed: false
            }),
            None
        );
        assert_eq!(last.input(&Input::Text('a')), None);
        assert_eq!(last.device(), Device::Mouse);
    }
}
