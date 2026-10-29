//! [`FlightView`]'s frame-rate key and readout: F12 toggles the
//! readout (`_HandlePlayer` @0x6e454-0x6e4bd), and each frame drawn
//! outside the map and the port is counted, the readout said on the
//! message line as `_PlayGame` says it (@0x45bcd-0x45d57). The meter's
//! rules are [`crate::flight::frame_rate`]'s.

use std::time::Duration;

use crate::flight::frame_rate::Toggled;
use crate::flight::sim_port::FlightSim;

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// A press of [`FRAME_RATE_KEY`](super::FRAME_RATE_KEY): toggles the
    /// readout, silently. Turned on, the message line is freed at once
    /// (`_commCount` := 0); turned off, it keeps what it shows for 1
    /// tick (`_commCount` := 1), so `_HandleComm` paints it empty on the
    /// next frame.
    pub(super) fn toggle_frame_rate(&mut self) {
        let ticks = match self.frame_rate.toggle() {
            Toggled::On => 0,
            Toggled::Off => 1,
        };
        self.cut_message(ticks);
    }

    /// Moves the readout's clock on by `dt`: every tick, whatever runs.
    pub(super) fn elapse_frame_rate(&mut self, dt: Duration) {
        self.frame_rate.elapse(dt);
    }

    /// Counts a frame drawn, and says the readout on the message line
    /// when one is due ([`FlightView::say_for`]): `_DisplayComm`, as any
    /// flight line, which sets the flight's line too. Nothing while
    /// docked: the original's game loop doesn't run in the spaceport.
    pub(super) fn frame_rate_frame(&mut self) {
        if self.frame.docked() {
            return;
        }
        if let Some(readout) = self.frame_rate.frame() {
            self.say_for(readout.text(), readout.ticks());
        }
    }

    /// The app came back to the front (`_SwitchedIn` @0x513cc-0x513dc):
    /// a readout shown says "Calculating FPS..." on the next frame.
    pub(super) fn switched_in(&mut self) {
        self.frame_rate.switched_in();
    }
}
