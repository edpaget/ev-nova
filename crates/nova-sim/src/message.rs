//! The messages a flight session raises, as events: what happened, not
//! the words. The view decides what each one says on the message line.

use crate::catalog::{ShipId, SystemId};

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
    /// A mission's deadline ran out in flight and failed it, for a
    /// mission whose `Flags` lacks 0x0400 (`_MissionObjectivesCheck`
    /// @0x9ebd0-0x9ec53).
    DeadlineMissed,
    /// Every ton of cargo was jettisoned (`_JettisonCargo` @0x3e354).
    CargoJettisoned,
    /// The cargo but the missions' was jettisoned (`_JettisonCargo`
    /// @0x3e35e).
    NonMissionCargoJettisoned,
    /// A scan found a mission's cargo and failed the mission
    /// (`_ScanPlayer` @0x7e6e2-0x7e7c9).
    MissionScanned,
    /// A scan found a mission's cargo: the player is caught smuggling
    /// (`_ScanPlayer` @0x7e7ce-0x7ea9b).
    Smuggling {
        /// The scanner's ship class, which names it.
        scanner: ShipId,
        /// The credits fined: none for no fine.
        fine: i64,
    },
}
