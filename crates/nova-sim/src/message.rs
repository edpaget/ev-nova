//! The messages a flight session raises, as events: what happened, not
//! the words. The view decides what each one says on the message line.

use crate::catalog::{JunkId, OutfitId, ShipId, StellarId, SystemId};
use crate::tutorial::TutorialHint;

/// Something a [`Session`](crate::Session) did that the player is told of.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// A mission failed as one of its special ships was destroyed or
    /// disabled, for a mission whose `Flags` lacks 0x0400
    /// (`_HandleShipDisplay` @0x2db31-0x2dbab, `_DamageShip`
    /// @0x3aee8-0x3af62).
    MissionShipLost,
    /// The player's ship was newly disabled: "Ship disabled.", `STR#`
    /// 2002 #287 (`_DamageShip` @0x3b181; see the session's
    /// `player_downed` module).
    ShipDisabled,
    /// The player's ship newly started to break up: "Ship destroyed.",
    /// `STR#` 2002 #288 (`_DamageShip` @0x3b3b9).
    ShipDestroyed,
    /// The player's ship being disabled or breaking up failed its first
    /// unfailed `Flags2` 0x0004 mission: "Mission failed.", `STR#` 2002
    /// #284 (`_DamageShip` @0x3b0b6, @0x3b353), hidden or not.
    DownedMissionFailed,
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
        /// The scanner's ship class, which names it when there is no
        /// `ship_name` (ship +0x76 → `shïp` +0x6c, @0x7e7d7-0x7e7ec).
        scanner: ShipId,
        /// The drawn ship name of the mission whose cargo the scan found,
        /// when not empty, which names the scanner in place of its class,
        /// whichever ship it is (the found slot's +0x70, @0x7e7ce).
        ship_name: Option<String>,
        /// The credits fined: none for no fine.
        fine: i64,
    },
    /// A disabled former escort repaired itself and rejoined the fleet
    /// (`_HandleShip` @0x33b45-0x33bac, `STR#` 2002 #127).
    EscortRepaired,
    /// A disabled former fighter repaired itself and rejoined the fleet
    /// (`_HandleShip` @0x33b32-0x33bac, `STR#` 2002 #128).
    FighterRepaired,
    /// A scan found illegal `jünk`: the player is caught smuggling it
    /// (`_ScanPlayer` @0x7eba9-0x7eece; see [`scan`](crate::scan)).
    IllegalJunk {
        /// The scanner's ship class, which names it when it is no person
        /// (ship +0x76 → `shïp` +0x6c, @0x7ebf7-0x7ec0c).
        scanner: ShipId,
        /// The scanner's person's name, which names it when it is one
        /// (+0xc8d0 → `përs` +0x624, @0x7ebd1-0x7ebf5).
        person: Option<String>,
        /// The `jünk` found, named by its `LCName`.
        junk: JunkId,
        /// The credits fined: none for no fine.
        fine: i64,
    },
    /// A scan found an illegal outfit: the player is caught with it
    /// (`_ScanPlayer` @0x7effc-0x7f38f; see [`scan`](crate::scan)).
    IllegalOutfit {
        /// The scanner's ship class, which names it when it is no person
        /// (@0x7f04a-0x7f05f).
        scanner: ShipId,
        /// The scanner's person's name, which names it when it is one
        /// (@0x7f024-0x7f048).
        person: Option<String>,
        /// The outfit found, named by its `LCName` for one, else its
        /// `LCPlural`.
        outfit: OutfitId,
        /// Whether the player owns exactly one (@0x7f103-0x7f113).
        one: bool,
        /// The credits fined: none for no fine.
        fine: i64,
    },
    /// The ship took off past the tutorial with no `Q` message: the
    /// take-off greeting, "{lead} {the stellar} on {the date}."
    /// (`_PlayerLandOnStellar` @0x635f7-0x63732; see
    /// [`tutorial`](crate::tutorial)).
    TookOff {
        /// The stellar taken off from.
        stellar: StellarId,
        /// Which lead, 0-4: `STR#` 2002 #(55 + `lead`), rolled
        /// `Rand(5)` (@0x63604).
        lead: u16,
    },
    /// A first-flight tutorial hint (`_HandlePlayer` @0x7093d-0x70ea2; see
    /// [`tutorial`](crate::tutorial)).
    Tutorial(TutorialHint),
}

impl SimMessage {
    /// How many ticks the message line shows this message for: the
    /// frames its caller passes `_DisplayComm` (@0x90201).
    ///
    /// - [`Arrived`](Self::Arrived): 240 (`_HandlePlayer` @0x6c46d).
    /// - [`ExitedHypergate`](Self::ExitedHypergate): 360
    ///   (`_PlayerEnterHypergate` @0x63f1d).
    /// - [`PassedWormhole`](Self::PassedWormhole): 240
    ///   (`_PlayerEnterWormhole` @0x645be).
    /// - [`MissionFailed`](Self::MissionFailed),
    ///   [`CargoJettisoned`](Self::CargoJettisoned) and
    ///   [`NonMissionCargoJettisoned`](Self::NonMissionCargoJettisoned):
    ///   240 (`_JettisonCargo` @0x3e17d, @0x3e3b0).
    /// - [`MissionShipLost`](Self::MissionShipLost): 240
    ///   (`_HandleShipDisplay` @0x2dbab, `_DamageShip` @0x3af62).
    /// - [`ShipDisabled`](Self::ShipDisabled),
    ///   [`ShipDestroyed`](Self::ShipDestroyed) and
    ///   [`DownedMissionFailed`](Self::DownedMissionFailed): 240
    ///   (`_DamageShip` @0x3b181, @0x3b3b9, @0x3b0b6, @0x3b353).
    /// - [`DeadlineMissed`](Self::DeadlineMissed): 240
    ///   (`_MissionObjectivesCheck` @0x9ec53).
    /// - [`MissionScanned`](Self::MissionScanned),
    ///   [`Smuggling`](Self::Smuggling),
    ///   [`IllegalJunk`](Self::IllegalJunk) and
    ///   [`IllegalOutfit`](Self::IllegalOutfit): 400 (`_ScanPlayer`
    ///   @0x7e79d, @0x7ea96, @0x7eece, @0x7f38f).
    /// - [`EscortRepaired`](Self::EscortRepaired) and
    ///   [`FighterRepaired`](Self::FighterRepaired): 250 (`_HandleShip`
    ///   @0x33ba1).
    /// - [`TookOff`](Self::TookOff): 240 (`_PlayerLandOnStellar`
    ///   @0x63727).
    /// - [`Tutorial`](Self::Tutorial): 512 (0x200, `_HandlePlayer`
    ///   @0x70b8f, @0x70d28, @0x70ddf, @0x70e8d).
    #[must_use]
    pub fn ticks(&self) -> u16 {
        match self {
            Self::ExitedHypergate(_) => 360,
            Self::MissionScanned
            | Self::Smuggling { .. }
            | Self::IllegalJunk { .. }
            | Self::IllegalOutfit { .. } => 400,
            Self::Arrived(_)
            | Self::PassedWormhole(_)
            | Self::MissionFailed
            | Self::DeadlineMissed
            | Self::MissionShipLost
            | Self::ShipDisabled
            | Self::ShipDestroyed
            | Self::DownedMissionFailed
            | Self::CargoJettisoned
            | Self::NonMissionCargoJettisoned
            | Self::TookOff { .. } => 240,
            Self::EscortRepaired | Self::FighterRepaired => 250,
            Self::Tutorial(_) => 512,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_sim_message_lasts_its_callers_ticks() {
        let smuggling = SimMessage::Smuggling {
            scanner: ShipId(128),
            ship_name: None,
            fine: 0,
        };
        let cases = [
            (SimMessage::Arrived(SystemId(128)), 240),
            (SimMessage::ExitedHypergate(SystemId(128)), 360),
            (SimMessage::PassedWormhole(SystemId(128)), 240),
            (SimMessage::MissionFailed, 240),
            (SimMessage::DeadlineMissed, 240),
            (SimMessage::MissionShipLost, 240),
            (SimMessage::ShipDisabled, 240),
            (SimMessage::ShipDestroyed, 240),
            (SimMessage::DownedMissionFailed, 240),
            (SimMessage::CargoJettisoned, 240),
            (SimMessage::NonMissionCargoJettisoned, 240),
            (SimMessage::MissionScanned, 400),
            (smuggling, 400),
            (SimMessage::EscortRepaired, 250),
            (SimMessage::FighterRepaired, 250),
            (
                SimMessage::TookOff {
                    stellar: StellarId(128),
                    lead: 2,
                },
                240,
            ),
            (SimMessage::Tutorial(TutorialHint::Hyperspace), 512),
            (SimMessage::Tutorial(TutorialHint::FarEnough), 512),
            (
                SimMessage::IllegalJunk {
                    scanner: ShipId(128),
                    person: None,
                    junk: JunkId(137),
                    fine: 0,
                },
                400,
            ),
            (
                SimMessage::IllegalOutfit {
                    scanner: ShipId(128),
                    person: None,
                    outfit: OutfitId(320),
                    one: true,
                    fine: 0,
                },
                400,
            ),
        ];
        for (message, ticks) in cases {
            assert_eq!(message.ticks(), ticks, "{message:?}");
        }
    }
}
