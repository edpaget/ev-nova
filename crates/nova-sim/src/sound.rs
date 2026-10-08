//! The sounds a flight session makes, as events: what happened, not what
//! to play. The audio side decides which `snd ` each one plays, if any.

use crate::catalog::SoundId;

/// Something a [`Session`](crate::Session) did that may make a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimSound {
    /// The ship began thrusting.
    ThrustStarted,
    /// The ship stopped thrusting: the player let go, or it landed or
    /// began a jump.
    ThrustStopped,
    /// The ship landed, on a stellar with this landing sound, if any.
    Landed {
        /// The stellar's own landing sound.
        stellar_sound: Option<SoundId>,
    },
    /// The ship took off.
    TookOff,
    /// A jump began.
    JumpBegan,
    /// A jump ended in the next system.
    Arrived,
    /// A `Q` set operator's message is shown in flight.
    ScriptMessage,
}
