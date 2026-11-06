//! The device the player used last ([`LastDevice`]): what the app tells
//! the screens as [`Surroundings::device`](nova_view::Surroundings), so
//! prompts and the pad's focus highlight follow the hand on the controls.
//!
//! It becomes:
//!
//! - `Keyboard` on a key's press;
//! - `Mouse` on a pointer button, a move or a scroll;
//! - `Touch` on a finger touching down, moving or lifting, wherever it is,
//!   a letterbox bar included; a touch the system cancels, and a pinch,
//!   change nothing;
//! - `Gamepad(family)` on a pad button's press, a trigger pulled past
//!   the press threshold, or a stick leaving the flight dead zone (analog
//!   flight input below the press threshold is real input). A stick
//!   moving within the dead zone, or staying outside it, changes nothing.
//!
//! Releases change nothing. The family is the pad's, read from the ids
//! its connect carried ([`family_of`]) and forgotten when it is lost; a
//! pad connected with no ids is generic. A use of a pad of another
//! family than the last is a change, so the screens are told it. A
//! screen's pad button, which comes from the pads merged, is taken to be
//! the last pad used's.

use std::collections::BTreeMap;

use nova_view::keymap::dead_zoned;
use nova_view::{ControllerFamily, Device, Input, PadAxis, Touch, TouchPhase};

use crate::gamepad::family::family_of;
use crate::gamepad::{PadEvent, PadId};
use nova_view::keymap::gamepad::PRESS_THRESHOLD;

/// The device the player used last, and what it reads to tell.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LastDevice {
    /// The device used last; the keyboard before any.
    device: Device,
    /// Each pad's last value of each stick axis.
    axes: BTreeMap<(PadId, PadAxis), f32>,
    /// Each connected pad's family, from its ids ([`family_of`]).
    families: BTreeMap<PadId, ControllerFamily>,
    /// The family of the pad used last, which a screen's merged pad
    /// button ([`Input::PadButton`]) is taken to be; generic before any.
    last_family: ControllerFamily,
}

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
            // A cancel is the system's, not the player's.
            Input::Touch(Touch {
                phase: TouchPhase::Started(_) | TouchPhase::Moved(_) | TouchPhase::Ended(_),
                ..
            }) => Device::Touch,
            Input::PadButton { pressed: true, .. } => Device::Gamepad(self.last_family),
            _ => return None,
        };
        self.switch(used)
    }

    /// Follows what a pad did, and says the device it switched to, if it
    /// switched.
    pub fn pad_event(&mut self, event: &PadEvent) -> Option<Device> {
        self.track(event);
        let used = match *event {
            PadEvent::Button { pressed: true, .. } => true,
            PadEvent::Trigger { value, .. } => value >= PRESS_THRESHOLD,
            PadEvent::Axis { pad, axis, value } => self.leaves_dead_zone(pad, axis, value),
            PadEvent::Connected { .. } | PadEvent::Disconnected(_) | PadEvent::Button { .. } => {
                false
            }
        };
        if !used {
            return None;
        }
        let pad = event.pad();
        self.last_family = self.families.get(&pad).copied().unwrap_or_default();
        self.switch(Device::Gamepad(self.last_family))
    }

    /// Follows which pads are connected, switching nothing: a connect
    /// records the pad's family from its ids ([`family_of`]), a loss
    /// forgets its family and its sticks. [`LastDevice::pad_event`] runs
    /// it on every event, and the app runs it alone on what the pads did
    /// while the window was in the background, which is no use.
    pub fn track(&mut self, event: &PadEvent) {
        match *event {
            PadEvent::Connected { pad, ids } => {
                self.families.insert(pad, family_of(ids));
            }
            PadEvent::Disconnected(pad) => {
                self.axes.retain(|&(of, _), _| of != pad);
                self.families.remove(&pad);
            }
            _ => {}
        }
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
    use nova_view::keymap::FLIGHT_DEAD_ZONE;
    use nova_view::{
        FingerId, Key, MouseButton, PadButton, Pinch, Point, ScrollDelta, Touch, TouchPhase,
    };

    use super::*;

    const ONE: PadId = PadId(0);

    /// A pad of no known family.
    const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

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

    fn touch(phase: TouchPhase) -> Input {
        Input::Touch(Touch {
            finger: FingerId::new(1),
            phase,
        })
    }

    #[test]
    fn a_touch_makes_it_touch() {
        let at = Point::new(1.0, 1.0);
        for phase in [
            TouchPhase::Started(at),
            TouchPhase::Moved(at),
            TouchPhase::Ended(at),
        ] {
            let mut last = LastDevice::new();
            assert_eq!(last.input(&touch(phase)), Some(Device::Touch), "{phase:?}");
            assert_eq!(last.input(&touch(phase)), None, "no change");
        }
    }

    #[test]
    fn a_cancelled_touch_or_a_pinch_changes_nothing() {
        let mut last = LastDevice::new();
        assert_eq!(last.input(&touch(TouchPhase::Cancelled)), None);
        assert_eq!(last.input(&Input::Pinch(Pinch::ZoomIn)), None);
        assert_eq!(last.device(), Device::Keyboard);
    }

    #[test]
    fn a_key_or_the_mouse_after_a_touch_switches_back() {
        let mut last = LastDevice::new();
        last.input(&touch(TouchPhase::Started(Point::new(1.0, 1.0))));
        assert_eq!(last.input(&key(true)), Some(Device::Keyboard));
        last.input(&touch(TouchPhase::Started(Point::new(1.0, 1.0))));
        assert_eq!(
            last.input(&Input::PointerMoved(Point::new(1.0, 1.0))),
            Some(Device::Mouse)
        );
    }

    #[test]
    fn a_pad_button_makes_it_gamepad() {
        let mut last = LastDevice::new();
        let press = PadEvent::Button {
            pad: ONE,
            button: PadButton::South,
            pressed: true,
        };
        assert_eq!(last.pad_event(&PadEvent::connected(ONE)), None);
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

    const TWO: PadId = PadId(1);

    fn connected(pad: PadId, vendor: u16) -> PadEvent {
        PadEvent::connected_with(pad, crate::gamepad::PadIds { vendor, product: 1 })
    }

    fn south(pad: PadId) -> PadEvent {
        PadEvent::Button {
            pad,
            button: PadButton::South,
            pressed: true,
        }
    }

    const XBOX: Device = Device::Gamepad(ControllerFamily::Xbox);

    #[test]
    fn a_pad_s_use_says_its_family() {
        let mut last = LastDevice::new();
        assert_eq!(last.pad_event(&connected(ONE, 0x045E)), None);
        assert_eq!(last.pad_event(&south(ONE)), Some(XBOX));
        assert_eq!(last.device(), XBOX);
        assert_eq!(
            last.input(&Input::PadButton {
                button: PadButton::South,
                pressed: true
            }),
            None,
            "a merged press is the last pad's family"
        );
        let mut trigger = LastDevice::new();
        trigger.pad_event(&connected(ONE, 0x054C));
        assert_eq!(
            trigger.pad_event(&PadEvent::Trigger {
                pad: ONE,
                button: PadButton::LeftTrigger,
                value: 1.0
            }),
            Some(Device::Gamepad(ControllerFamily::PlayStation))
        );
        let mut stick = LastDevice::new();
        stick.pad_event(&connected(ONE, 0x057E));
        assert_eq!(
            stick.pad_event(&stick_of(ONE, PadAxis::LeftX, outside())),
            Some(Device::Gamepad(ControllerFamily::Nintendo))
        );
    }

    fn stick_of(pad: PadId, axis: PadAxis, value: f32) -> PadEvent {
        PadEvent::Axis { pad, axis, value }
    }

    #[test]
    fn a_second_pad_of_another_family_switches_the_family() {
        let mut last = LastDevice::new();
        last.pad_event(&connected(ONE, 0x045E));
        last.pad_event(&connected(TWO, 0x054C));
        assert_eq!(last.pad_event(&south(ONE)), Some(XBOX));
        assert_eq!(
            last.pad_event(&south(TWO)),
            Some(Device::Gamepad(ControllerFamily::PlayStation))
        );
        assert_eq!(last.pad_event(&south(TWO)), None, "no change");
        last.pad_event(&PadEvent::Disconnected(TWO));
        last.pad_event(&PadEvent::connected(TWO));
        assert_eq!(
            last.pad_event(&south(TWO)),
            Some(PAD),
            "found again with no ids, generic"
        );
    }

    #[test]
    fn tracking_a_connect_records_its_family_without_switching() {
        let mut last = LastDevice::new();
        let ids = crate::gamepad::PadIds {
            vendor: 0x045E,
            product: 1,
        };
        last.track(&PadEvent::connected_with(ONE, ids));
        assert_eq!(last.device(), Device::Keyboard, "a connect is no use");
        assert_eq!(
            last.pad_event(&south(ONE)),
            Some(Device::Gamepad(family_of(Some(ids))))
        );
    }

    #[test]
    fn tracking_a_disconnect_forgets_its_family_and_sticks() {
        let mut last = LastDevice::new();
        last.pad_event(&connected(ONE, 0x045E));
        last.pad_event(&stick(PadAxis::RightX, outside()));
        last.input(&key(true));
        last.track(&PadEvent::Disconnected(ONE));
        assert_eq!(last.device(), Device::Keyboard);
        assert_eq!(
            last.pad_event(&stick(PadAxis::RightX, outside())),
            Some(PAD),
            "found with no connect, generic and centred"
        );
    }

    #[test]
    fn a_key_after_the_pad_switches_back_to_the_keyboard() {
        let mut last = LastDevice::new();
        last.pad_event(&connected(ONE, 0x045E));
        last.pad_event(&south(ONE));
        assert_eq!(last.input(&key(true)), Some(Device::Keyboard));
        assert_eq!(last.pad_event(&south(ONE)), Some(XBOX));
    }

    #[test]
    fn a_stick_just_past_the_dead_zone_below_the_press_threshold_makes_it_gamepad() {
        let mut last = LastDevice::new();
        let just_past = FLIGHT_DEAD_ZONE + 0.01;
        assert!(just_past < PRESS_THRESHOLD);
        assert_eq!(
            last.pad_event(&stick(PadAxis::RightX, just_past)),
            Some(PAD)
        );
    }

    /// Values on one axis that `nova_view`'s dead zone reads as none, the
    /// stick centred across: fractions of its published radius.
    fn inside() -> [f32; 4] {
        [0.3, 0.6, -0.8, 0.0].map(|share| share * FLIGHT_DEAD_ZONE)
    }

    /// A value on one axis that `nova_view`'s dead zone reads as pushed:
    /// halfway from its radius to full deflection.
    fn outside() -> f32 {
        f32::midpoint(FLIGHT_DEAD_ZONE, 1.0)
    }

    #[test]
    fn a_stick_inside_the_dead_zone_does_not() {
        let mut last = LastDevice::new();
        for value in inside() {
            assert_eq!(last.pad_event(&stick(PadAxis::LeftY, value)), None);
        }
        assert_eq!(last.device(), Device::Keyboard);
    }

    #[test]
    fn a_stick_staying_outside_the_dead_zone_does_not_switch_it_back() {
        let mut last = LastDevice::new();
        let further = f32::midpoint(outside(), 1.0);
        last.pad_event(&stick(PadAxis::LeftY, outside()));
        last.input(&key(true));
        assert_eq!(last.pad_event(&stick(PadAxis::LeftY, further)), None);
        assert_eq!(last.device(), Device::Keyboard);
        last.pad_event(&stick(PadAxis::LeftY, 0.0));
        assert_eq!(
            last.pad_event(&stick(PadAxis::LeftY, -outside())),
            Some(PAD),
            "back in, then out again"
        );
    }

    #[test]
    fn the_stick_leaves_the_dead_zone_as_nova_view_reads_both_its_axes() {
        let shares = [0.0, 0.5, 0.8, -0.8, 1.2, -2.0];
        for x in shares.map(|share| share * FLIGHT_DEAD_ZONE) {
            for y in shares.map(|share| share * FLIGHT_DEAD_ZONE) {
                let pushed = |x, y| dead_zoned(x, y) != (0.0, 0.0);
                let mut last = LastDevice::new();
                let first = last.pad_event(&stick(PadAxis::LeftX, x));
                assert_eq!(first, pushed(x, 0.0).then_some(PAD), "({x}, 0)");
                let second = last.pad_event(&stick(PadAxis::LeftY, y));
                let leaves = pushed(x, y) && !pushed(x, 0.0);
                assert_eq!(second, leaves.then_some(PAD), "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_lost_pad_forgets_its_sticks() {
        let mut last = LastDevice::new();
        last.pad_event(&stick(PadAxis::RightX, outside()));
        last.input(&key(true));
        last.pad_event(&PadEvent::Disconnected(ONE));
        assert_eq!(
            last.pad_event(&stick(PadAxis::RightX, outside())),
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
