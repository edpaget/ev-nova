//! What [`FlightView`] keeps of the keys pressed while the map waits for
//! [`MAP_KEY`](super::MAP_KEY)'s release to close (`_DoSystemMap`
//! @0x13a2e-0x13a46). All addresses are in `EV Nova.app`'s `EV Nova`
//! (i386).
//!
//! The wait reads no events (`_BusyIdle` @0x5981 is empty) and plays no
//! sound up to the return (@0x13a2e-0x13c62), and the closing
//! `FlushEvents(7, 0)` (@0x13c44-0x13c53) drops only null, mouse-down and
//! mouse-up events. So a click during the wait is gone, and a key-down,
//! an auto-key and a key-up stay queued for whoever reads events next.
//! What that does depends on who that is:
//!
//! - **Flight reads no queued key.** `_IterateInGameEventLoop` drops them
//!   (@0x56a6b, @0x56cec), and `_HandlePlayer` polls every key from the
//!   `GetKeys` snapshot of `_QuickKeyCheckHandler` (@0x5d5a, read by
//!   `_QuickKeyCheck` @0xa0f5). A key tapped during the wait does nothing
//!   after the close, and one still down reads as down, whenever it went
//!   down, before the wait and before the map opened too. So over flight
//!   the view keeps [`UNLATCHED_KEYS`] held across the map, presses and
//!   releases, and everything else is let go as the map opens. The latched
//!   keys wait for their release: `_HandlePlayer` sets all 23 latches after
//!   M's map (`_SetAllKeyCmdFlags` @0x61e6d, called @0x6a898).
//!
//! Left out: `MenuKey` for a queued Command key in flight (@0x56a81), whose
//! result is ignored, as ours' Command-Q acts on its press in the router.

use super::dialog_map::MapOpening;
use super::{AFTERBURNER_KEY, FACE_TARGET_KEY, FIRE_KEY, FlightView, SECONDARY_KEY};
use crate::flight::sim_port::FlightSim;
use crate::{Input, Key};

/// The keys `_HandlePlayer` acts on the moment they read down, with no
/// latch to wait on, so the map keeps them held: thrust +0x2a (@0x6cb71),
/// turn +0x26 and +0x28 (@0x6c9cc), reverse +0x2c (@0x6c72f), face-target
/// +0xe (@0x6aec6), afterburner +0x30 (@0x6cadf), primary +0x4 (@0x6ab06),
/// secondary +0x6 (@0x6ab87), and raw Option, Command and Shift (0x3a,
/// 0x37, 0x38).
const UNLATCHED_KEYS: [Key; 11] = [
    Key::Up,
    Key::Left,
    Key::Right,
    Key::Down,
    FACE_TARGET_KEY,
    AFTERBURNER_KEY,
    FIRE_KEY,
    SECONDARY_KEY,
    Key::Alt,
    Key::Command,
    Key::Shift,
];

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether the map open is flight's own: M's or the hypergate's, which
    /// takes its input from flight ([`MapOpening::Plain`]).
    pub(super) fn plain_map_open(&self) -> bool {
        self.map_opened
            .is_some_and(|opened| opened.by == MapOpening::Plain)
    }

    /// Keeps of the keys held only those flight goes on reading live
    /// across the map ([`UNLATCHED_KEYS`]).
    pub(super) fn keep_unlatched_keys(&mut self) {
        self.held.retain(|key| UNLATCHED_KEYS.contains(key));
    }

    /// Tracks `input`'s key as held if it is one flight goes on reading
    /// live across the map ([`UNLATCHED_KEYS`]).
    pub(super) fn track_unlatched(&mut self, input: &Input) {
        if let Input::Key { key, .. } = *input
            && UNLATCHED_KEYS.contains(&key)
        {
            self.track_held(input);
        }
    }
}
