//! The messages a flight session raises, as events: what happened, not
//! the words. The view decides what each one says on the message line.

use crate::catalog::SystemId;

/// Something a [`Session`](crate::Session) did that the player is told of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimMessage {
    /// A jump ended in this system, the last of a multi-jump.
    Arrived(SystemId),
    /// The ship came out of a hypergate in this system.
    ExitedHypergate(SystemId),
    /// The ship passed through a wormhole into this system.
    PassedWormhole(SystemId),
    /// A mission failed as its cargo was jettisoned (`_JettisonCargo`
    /// @0x3e114-0x3e182).
    MissionFailed,
    /// Every ton of cargo was jettisoned (`_JettisonCargo` @0x3e354).
    CargoJettisoned,
    /// The cargo but the missions' was jettisoned (`_JettisonCargo`
    /// @0x3e35e).
    NonMissionCargoJettisoned,
}
