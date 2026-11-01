//! Boarding the player's own disabled escorts and fighters back: the
//! own-ship paths of `_HandlePlayerBoardAttempt` (@0x65843-0x65eb3), as
//! [`recovery`] picks among them. The session applies them
//! (`nova_sim::Session::board`).
//!
//! They come after the velocity match and the boarding's crime, and in
//! place of the plunder. Who flies the ship is tested first (+0xc8d0,
//! @0x65847):
//!
//! - **A person with a `LinkMission` and `Flags` 0x0200** offers its
//!   mission, or with the mission unavailable is plundered: never an
//!   own-ship path (@0x65867).
//! - **A person with a `LinkMission` and no 0x0200** (@0x65a21-0x65a60),
//!   or **with no `LinkMission`** (@0x65a65-0x65b85): its own former
//!   fighter is repaired when a bay has room for it (+0xc8de 0 and
//!   `_CanBuyFighter`); anything else is plundered.
//! - **No person** (@0x65b8a): a former hired or captured escort (+0xc8de
//!   1 or 2) is repaired when the fleet has room (`_CanHireEscorts`);
//!   otherwise a former fighter is repaired, and a ship that never was
//!   the player's captured, when a bay of its type or sprite base has room
//!   (+0xc8de 0 or -1, `_CanBuyFighter`); anything else is plundered.
//!
//! So a person-flown former escort never comes back as an escort, and a
//! former escort with the fleet full falls to the fighter tests and then
//! to the plunder, keeping its memory. Each path the player is told of
//! ([`Recovery::message`]) for 250 ticks with `_beepSnd[4]`
//! (`nova_sim::SimSound::EscortRepaired`).

use nova_sim_combat::message::SimMessage;
use nova_sim_combat::self_repair::FormerKind;

/// Which own-ship path a board takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// "Escort repaired." (`STR#` 2002 #127, @0x65cff): the ship rejoins
    /// the fleet.
    EscortRepaired,
    /// "Fighter repaired." (#128, @0x65af6, @0x65e29): the player's own
    /// fighter docks in its bay.
    FighterRepaired,
    /// "Fighter captured." (#129, @0x65e29): a ship of a type the player's
    /// bays carry docks in its bay.
    FighterCaptured,
}

impl Recovery {
    /// What the player is told.
    #[must_use]
    pub fn message(self) -> SimMessage {
        match self {
            Self::EscortRepaired => SimMessage::EscortRepaired,
            Self::FighterRepaired => SimMessage::FighterRepaired,
            Self::FighterCaptured => SimMessage::FighterCaptured,
        }
    }
}

/// Who flies the ship boarded, as the own-ship paths test it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlownBy {
    /// No person (+0xc8d0 -1).
    NoPerson,
    /// A person.
    Person {
        /// Whether it has a `LinkMission`.
        link_mission: bool,
        /// Whether its `Flags` hold 0x0200, its mission offered on
        /// boarding.
        offers_on_board: bool,
    },
}

/// The own-ship path a board takes of a ship `flown_by`, remembering
/// being the player's `former` escort or fighter, with room in the
/// fleet for one more escort (`fleet_room`) and room for one more of its
/// type in a bay of the player's (`bay_room`); none for the plunder (or
/// a person's offer). See the module docs.
#[must_use]
pub fn recovery(
    flown_by: FlownBy,
    former: Option<FormerKind>,
    fleet_room: bool,
    bay_room: bool,
) -> Option<Recovery> {
    let fighter = former == Some(FormerKind::Fighter);
    match flown_by {
        // Its offer, or its plunder (@0x65867).
        FlownBy::Person {
            link_mission: true,
            offers_on_board: true,
        } => None,
        // @0x65a21-0x65a60 → @0x65d7b, and @0x65a65-0x65b85.
        FlownBy::Person { .. } => (fighter && bay_room).then_some(Recovery::FighterRepaired),
        FlownBy::NoPerson => match former {
            // +0xc8de 1 or 2 while `_CanHireEscorts` (@0x65b98).
            Some(FormerKind::Hired | FormerKind::Captured) if fleet_room => {
                Some(Recovery::EscortRepaired)
            }
            // +0xc8de 0 (@0x65d31-0x65d7b).
            Some(FormerKind::Fighter) if bay_room => Some(Recovery::FighterRepaired),
            // +0xc8de -1 (@0x65d85-0x65dd7).
            None if bay_room => Some(Recovery::FighterCaptured),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINKED_OFFERING: FlownBy = FlownBy::Person {
        link_mission: true,
        offers_on_board: true,
    };
    const LINKED: FlownBy = FlownBy::Person {
        link_mission: true,
        offers_on_board: false,
    };
    const UNLINKED: FlownBy = FlownBy::Person {
        link_mission: false,
        offers_on_board: false,
    };
    /// A person of 0x0200 with no `LinkMission`: no offer to make.
    const UNLINKED_OFFERING: FlownBy = FlownBy::Person {
        link_mission: false,
        offers_on_board: true,
    };

    const FORMERS: [Option<FormerKind>; 4] = [
        None,
        Some(FormerKind::Fighter),
        Some(FormerKind::Hired),
        Some(FormerKind::Captured),
    ];

    #[test]
    fn a_person_with_a_link_mission_and_0x0200_is_never_recovered() {
        // @0x65867: its offer, or its plunder.
        for former in FORMERS {
            for (fleet, bay) in [(true, true), (true, false), (false, true)] {
                assert_eq!(
                    recovery(LINKED_OFFERING, former, fleet, bay),
                    None,
                    "{former:?} {fleet} {bay}"
                );
            }
        }
    }

    #[test]
    fn a_person_without_0x0200_or_a_link_mission_has_only_its_fighter_repaired() {
        // @0x65a21-0x65a60 → @0x65d7b; @0x65a65-0x65b85.
        for person in [LINKED, UNLINKED, UNLINKED_OFFERING] {
            assert_eq!(
                recovery(person, Some(FormerKind::Fighter), false, true),
                Some(Recovery::FighterRepaired),
                "{person:?}"
            );
            assert_eq!(
                recovery(person, Some(FormerKind::Fighter), true, false),
                None,
                "{person:?}: no bay room"
            );
            assert_eq!(
                recovery(person, None, true, true),
                None,
                "{person:?}: never captured"
            );
        }
    }

    #[test]
    fn a_person_flown_former_escort_is_never_repaired_as_an_escort() {
        for person in [LINKED_OFFERING, LINKED, UNLINKED, UNLINKED_OFFERING] {
            for kind in [FormerKind::Hired, FormerKind::Captured] {
                assert_eq!(
                    recovery(person, Some(kind), true, true),
                    None,
                    "{person:?} {kind:?}"
                );
            }
        }
    }

    #[test]
    fn a_former_hired_or_captured_escort_no_person_flies_is_repaired_with_fleet_room() {
        // @0x65b8a-0x65d2c.
        for kind in [FormerKind::Hired, FormerKind::Captured] {
            for bay in [true, false] {
                assert_eq!(
                    recovery(FlownBy::NoPerson, Some(kind), true, bay),
                    Some(Recovery::EscortRepaired),
                    "{kind:?} {bay}"
                );
            }
        }
    }

    #[test]
    fn a_former_escort_with_the_fleet_full_falls_to_the_fighter_tests_then_the_plunder() {
        // +0xc8de 1 or 2 is neither 0 nor -1, so neither fighter test
        // passes it (@0x65d31, @0x65d85).
        for kind in [FormerKind::Hired, FormerKind::Captured] {
            for bay in [true, false] {
                assert_eq!(
                    recovery(FlownBy::NoPerson, Some(kind), false, bay),
                    None,
                    "{kind:?} {bay}"
                );
            }
        }
    }

    #[test]
    fn a_former_fighter_no_person_flies_is_repaired_with_bay_room_whatever_the_fleet() {
        // @0x65d31-0x65d7b.
        for fleet in [true, false] {
            assert_eq!(
                recovery(FlownBy::NoPerson, Some(FormerKind::Fighter), fleet, true),
                Some(Recovery::FighterRepaired),
                "{fleet}"
            );
            assert_eq!(
                recovery(FlownBy::NoPerson, Some(FormerKind::Fighter), fleet, false),
                None,
                "{fleet}: no bay room"
            );
        }
    }

    #[test]
    fn a_ship_no_person_flies_that_never_was_the_players_is_captured_with_bay_room() {
        // @0x65d85-0x65dd7.
        for fleet in [true, false] {
            assert_eq!(
                recovery(FlownBy::NoPerson, None, fleet, true),
                Some(Recovery::FighterCaptured),
                "{fleet}"
            );
            assert_eq!(recovery(FlownBy::NoPerson, None, fleet, false), None);
        }
    }

    #[test]
    fn each_recovery_says_its_line() {
        assert_eq!(
            Recovery::EscortRepaired.message(),
            SimMessage::EscortRepaired
        );
        assert_eq!(
            Recovery::FighterRepaired.message(),
            SimMessage::FighterRepaired
        );
        assert_eq!(
            Recovery::FighterCaptured.message(),
            SimMessage::FighterCaptured
        );
    }
}
