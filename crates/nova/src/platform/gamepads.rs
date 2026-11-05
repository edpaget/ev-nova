//! The [`Gamepads`] port over gilrs, on the desktops: `IOKit` on macOS,
//! Windows.Gaming.Input on Windows (which reports only while the window
//! has focus) and evdev on Linux. gilrs runs its own thread behind a
//! channel, so a poll a frame from the main thread drains it without
//! blocking (the findings of rdm:phase/key-bindings/phase-7-gamepad-input
//! record each backend).
//!
//! The adapter forwards and decides nothing. gilrs's default filters are
//! off, so the core's dead zone and thresholds are the only ones: of
//! them it keeps `axis_dpad_to_button` alone, which turns a d-pad a pad
//! reports as hat axes into the d-pad's buttons, applied in gilrs's
//! documented order for its own filters (the filter, then
//! `Gilrs::update`). It reports each pad found at the first poll as
//! connected, as gilrs registers those without an event, names buttons
//! and axes by position ([`pad_button`], [`pad_trigger`], [`pad_axis`]),
//! and forwards each trigger's value rather than gilrs's own press.

use std::collections::BTreeSet;

use gilrs::ev::filter::{Filter, axis_dpad_to_button};
use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs, GilrsBuilder};
use nova_view::{PadAxis, PadButton};

use crate::gamepad::{Gamepads, PadEvent, PadId};

/// The gamepads, read through gilrs.
pub struct GilrsGamepads {
    /// gilrs, its default filters and automatic state off.
    gilrs: Gilrs,
    /// Whether the pads found at the start were reported.
    started: bool,
    /// The pads reported connected and not lost since.
    connected: BTreeSet<PadId>,
}

impl GilrsGamepads {
    /// The gamepads, through gilrs's backend for this system, or its
    /// dummy, which reports nothing, where it has none.
    ///
    /// # Errors
    ///
    /// Why gilrs could not start (on Linux, no udev), for the startup
    /// report; the game then runs without gamepads.
    pub fn new() -> Result<Self, String> {
        let built = GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build();
        let gilrs = match built {
            Ok(gilrs) | Err(gilrs::Error::NotImplemented(gilrs)) => gilrs,
            Err(error) => return Err(format!("nova: no gamepads: {error}")),
        };
        Ok(Self {
            gilrs,
            started: false,
            connected: BTreeSet::new(),
        })
    }

    /// Reports `pad` connected, unless it is already.
    fn connect(&mut self, pad: PadId, out: &mut Vec<PadEvent>) {
        if self.connected.insert(pad) {
            out.push(PadEvent::Connected(pad));
        }
    }

    /// What gilrs's `event` is in core terms, appended to `out`.
    fn forward(&mut self, event: &Event, out: &mut Vec<PadEvent>) {
        let pad = pad_id(event.id);
        match event.event {
            EventType::Connected => self.connect(pad, out),
            EventType::Disconnected => {
                if self.connected.remove(&pad) {
                    out.push(PadEvent::Disconnected(pad));
                }
            }
            EventType::ButtonPressed(button, _) | EventType::ButtonReleased(button, _) => {
                if let Some(button) = pad_button(button) {
                    self.connect(pad, out);
                    let pressed = matches!(event.event, EventType::ButtonPressed(..));
                    out.push(PadEvent::Button {
                        pad,
                        button,
                        pressed,
                    });
                }
            }
            EventType::ButtonChanged(button, value, _) => {
                if let Some(button) = pad_trigger(button) {
                    self.connect(pad, out);
                    out.push(PadEvent::Trigger {
                        pad,
                        button,
                        value: value.clamp(0.0, 1.0),
                    });
                }
            }
            EventType::AxisChanged(axis, value, _) => {
                if let Some(axis) = pad_axis(axis) {
                    self.connect(pad, out);
                    out.push(PadEvent::Axis {
                        pad,
                        axis,
                        value: value.clamp(-1.0, 1.0),
                    });
                }
            }
            _ => {}
        }
    }
}

impl Gamepads for GilrsGamepads {
    fn poll(&mut self, out: &mut Vec<PadEvent>) {
        if !self.started {
            self.started = true;
            let found: Vec<PadId> = self.gilrs.gamepads().map(|(id, _)| pad_id(id)).collect();
            for pad in found {
                self.connect(pad, out);
            }
        }
        while let Some(event) = self
            .gilrs
            .next_event()
            .filter_ev(&axis_dpad_to_button, &mut self.gilrs)
        {
            if event.is_dropped() {
                continue;
            }
            self.gilrs.update(&event);
            self.forward(&event, out);
        }
    }
}

/// gilrs's pad number as the port's.
fn pad_id(id: GamepadId) -> PadId {
    PadId(usize::from(id))
}

/// The pad button gilrs's `button` is, by position: none for a trigger
/// (its value is forwarded, [`pad_trigger`]) or a button the pad has no
/// position for (Mode, C, Z, an unknown one). gilrs calls the shoulders
/// `LeftTrigger` and `RightTrigger`, and the triggers `…Trigger2`.
#[must_use]
pub fn pad_button(button: Button) -> Option<PadButton> {
    Some(match button {
        Button::South => PadButton::South,
        Button::East => PadButton::East,
        Button::West => PadButton::West,
        Button::North => PadButton::North,
        Button::LeftTrigger => PadButton::LeftShoulder,
        Button::RightTrigger => PadButton::RightShoulder,
        Button::DPadUp => PadButton::DPadUp,
        Button::DPadDown => PadButton::DPadDown,
        Button::DPadLeft => PadButton::DPadLeft,
        Button::DPadRight => PadButton::DPadRight,
        Button::Start => PadButton::Start,
        Button::Select => PadButton::Select,
        Button::LeftThumb => PadButton::LeftStick,
        Button::RightThumb => PadButton::RightStick,
        _ => return None,
    })
}

/// The trigger gilrs's `button` is, if one.
#[must_use]
pub fn pad_trigger(button: Button) -> Option<PadButton> {
    match button {
        Button::LeftTrigger2 => Some(PadButton::LeftTrigger),
        Button::RightTrigger2 => Some(PadButton::RightTrigger),
        _ => None,
    }
}

/// The stick axis gilrs's `axis` is, if one. gilrs gives Y positive up on
/// every system, as [`PadAxis`] does.
#[must_use]
pub fn pad_axis(axis: Axis) -> Option<PadAxis> {
    match axis {
        Axis::LeftStickX => Some(PadAxis::LeftX),
        Axis::LeftStickY => Some(PadAxis::LeftY),
        Axis::RightStickX => Some(PadAxis::RightX),
        Axis::RightStickY => Some(PadAxis::RightY),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamepad::fixture::check_contract;

    #[test]
    fn every_gilrs_button_the_pad_names_maps_to_its_position() {
        let named = [
            (Button::South, PadButton::South),
            (Button::East, PadButton::East),
            (Button::West, PadButton::West),
            (Button::North, PadButton::North),
            (Button::LeftTrigger, PadButton::LeftShoulder),
            (Button::RightTrigger, PadButton::RightShoulder),
            (Button::DPadUp, PadButton::DPadUp),
            (Button::DPadDown, PadButton::DPadDown),
            (Button::DPadLeft, PadButton::DPadLeft),
            (Button::DPadRight, PadButton::DPadRight),
            (Button::Start, PadButton::Start),
            (Button::Select, PadButton::Select),
            (Button::LeftThumb, PadButton::LeftStick),
            (Button::RightThumb, PadButton::RightStick),
        ];
        for (gilrs, pad) in named {
            assert_eq!(pad_button(gilrs), Some(pad), "{gilrs:?}");
            assert_eq!(pad_trigger(gilrs), None, "{gilrs:?}");
        }
        for unnamed in [Button::Mode, Button::C, Button::Z, Button::Unknown] {
            assert_eq!(pad_button(unnamed), None, "{unnamed:?}");
            assert_eq!(pad_trigger(unnamed), None, "{unnamed:?}");
        }
        assert_eq!(pad_button(Button::LeftTrigger2), None, "a trigger's value");
        assert_eq!(
            pad_trigger(Button::LeftTrigger2),
            Some(PadButton::LeftTrigger)
        );
        assert_eq!(
            pad_trigger(Button::RightTrigger2),
            Some(PadButton::RightTrigger)
        );
    }

    #[test]
    fn the_stick_axes_map_with_up_positive() {
        assert_eq!(pad_axis(Axis::LeftStickX), Some(PadAxis::LeftX));
        assert_eq!(pad_axis(Axis::LeftStickY), Some(PadAxis::LeftY));
        assert_eq!(pad_axis(Axis::RightStickX), Some(PadAxis::RightX));
        assert_eq!(pad_axis(Axis::RightStickY), Some(PadAxis::RightY));
        for other in [
            Axis::LeftZ,
            Axis::RightZ,
            Axis::DPadX,
            Axis::DPadY,
            Axis::Unknown,
        ] {
            assert_eq!(pad_axis(other), None, "{other:?}");
        }
    }

    /// Reads the devices only, so it leaves the machine as it found it,
    /// and passes with or without a pad attached. Off macOS a host with
    /// no input stack (a container with no udev) can refuse gilrs, which
    /// the game reports and runs past; that says nothing of the adapter.
    #[test]
    fn the_gilrs_adapter_passes_the_gamepads_contract() {
        match GilrsGamepads::new() {
            Ok(mut pads) => assert_eq!(check_contract(&mut pads), Ok(())),
            Err(why) if cfg!(not(target_os = "macos")) => eprintln!("{why}"),
            Err(why) => panic!("{why}"),
        }
    }
}
