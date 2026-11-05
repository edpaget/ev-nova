//! The app's side of the gamepads: the port it polls once a frame
//! ([`App::with_gamepads`]), what the pads' events mean for the screen
//! ([`PadInputs`](crate::gamepad::inputs::PadInputs)), and the device used
//! last ([`LastDevice`](super::last_device::LastDevice)), told to the
//! screen as [`Input::Surroundings`] whenever it changes.

use nova_render::ImageSource;
use nova_view::{Input, Screen, Surroundings};

use super::{App, Control, DevOverlay};
use crate::gamepad::Gamepads;

impl<S: ImageSource, C: Screen> App<S, C> {
    /// The app reading `pads`, polled once a frame, before the screen's
    /// tick ([`App::poll_gamepads`]).
    #[must_use]
    pub fn with_gamepads(mut self, pads: Box<dyn Gamepads>) -> Self {
        self.gamepads = Some(pads);
        self
    }

    /// One poll of the gamepads, on a redraw before the screen ticks: the
    /// device used last follows what the pads did, and the screen is told
    /// of a change, then given what each event means, as a key is routed.
    /// While the developer overlay shows, the pads' input goes nowhere, as
    /// keys go to the overlay alone, though the screen is still told a
    /// change of device, as a key's press tells it under the overlay. With
    /// no gamepads, nothing.
    pub(super) fn poll_gamepads(&mut self) -> Control {
        let Some(pads) = &mut self.gamepads else {
            return Control::Continue;
        };
        let mut events = Vec::new();
        pads.poll(&mut events);
        let mut changed = None;
        for event in &events {
            changed = self.last_device.pad_event(event).or(changed);
        }
        let mut inputs = Vec::new();
        self.pad_inputs.feed(&events, &mut inputs);
        if changed.is_some() && self.tell_device() == Control::Exit {
            return Control::Exit;
        }
        if self.overlay.as_ref().is_some_and(DevOverlay::visible) {
            return Control::Continue;
        }
        for input in inputs {
            if self.route(input) == Control::Exit {
                return Control::Exit;
            }
        }
        Control::Continue
    }

    /// Drains what the pads did while the window was in the background
    /// into the pad state, routing none of it: a press queued while away
    /// doesn't fire, as a key released in the background reaches no
    /// screen. The state still follows it, so a button let go of away is
    /// up, and one still down reads down from its next change.
    pub(super) fn drain_gamepads(&mut self) {
        let Some(pads) = &mut self.gamepads else {
            return;
        };
        let mut events = Vec::new();
        pads.poll(&mut events);
        self.pad_inputs.feed(&events, &mut Vec::new());
    }

    /// Follows a keyboard or pointer `input` for the device used last, and
    /// says whether it changed: the caller tells the screen
    /// ([`App::tell_device`]) once the input itself is routed.
    pub(super) fn note_device(&mut self, input: &Input) -> bool {
        self.last_device.input(input).is_some()
    }

    /// Tells the screen the device used last.
    pub(super) fn tell_device(&mut self) -> Control {
        self.route(Input::Surroundings(Surroundings {
            device: self.last_device.device(),
        }))
    }
}
