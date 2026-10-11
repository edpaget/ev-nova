//! The ships the player leads ([`Session::player_led`]): its hired and
//! captured escorts, the fighters it launched, and its mission escorts,
//! in ship-table order.
//!
//! **The original's one relation.** The original makes no difference
//! between a hired escort and a mission's `ShipBehav` 1 escort in the
//! ship table: each is a ship whose leader (+0x9a) is the player, 0. A
//! hire, a capture or a launch sets it (`_SpawnEscort` @0x3fedd,
//! `_DoShipCapture` @0x41419, `_LaunchFighter`), and so does a mission
//! escort's join (`_SetupShipsInSystem` @0x4309f-0x430c2, `_AIDispatch`
//! @0x8fc03-0x8fc22). Every site that asks whether a ship is the
//! player's escort reads that one field, and narrows it to the hired
//! escorts (AI type 6 with no mission mark, or the hired flag +0xbb) or
//! the fighters (AI type 5) only where it means to: saving, pay, the
//! escort landing screen and dialog, the hiring room, the fleet's holds,
//! jettison and fighter recall. The step-1 findings of
//! rdm:phase/player-led-ships/phase-1-one-player-led-relation list every
//! site with its address and owner.
//!
//! **Here.** The engine keeps the hired escorts in the pilot's records
//! and [`Session::fleet`](Session), each NPC carrying an
//! [`EscortDuty`](crate::escort::EscortDuty), and marks a mission escort
//! by its mission's mark ([`Npc::mission_escort`]). [`Session::player_led`]
//! is the original's relation over both: the NPCs whose
//! [`Npc::escorts_player`] holds, in the order they appeared, which is
//! ship-table order (an [`NpcId`](crate::traffic::npc::NpcId) only grows).
//! The sites that ask it narrow it as the original does:
//! [`Npc::is_players`] for the hired escorts and fighters,
//! [`Npc::mission_escort`] for the mission escorts. A disabled mission
//! escort that has joined stays led, as the original leaves a disabled
//! ship's leader as it was (@0x8fc0c); a hired escort disabled or
//! released leaves the fleet and is no longer led.

use super::Session;
use crate::traffic::npc::Npc;

impl Session {
    /// The ships the player leads, in ship-table order (see the module
    /// docs).
    pub fn player_led(&self) -> impl Iterator<Item = &Npc> + '_ {
        self.traffic.player_led()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MissionShip, ShipId};
    use crate::combat::hull::Condition;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::session::mission_ships::tests::{
        accepted, shipping, ships_catalog, take_off, ticks,
    };
    use crate::traffic::npc::NpcId;

    /// Mission 140's two ships, of `ShipBehav` `behav`, there from the
    /// start in the system the mission is accepted in.
    fn ships(behav: i16) -> MissionShip {
        MissionShip {
            behav,
            ..shipping(-1, 0)
        }
    }

    /// A hired escort of ship 200.
    fn hired() -> Escort {
        Escort {
            ship: ShipId(200),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    /// The NPCs the player leads.
    fn led(session: &Session) -> Vec<NpcId> {
        session.player_led().map(|npc| npc.id).collect()
    }

    /// In flight with mission 140's ships of `ShipBehav` `behav` and a
    /// hired escort: the session, the mission's ships and the hired
    /// escort's NPC.
    fn flying(behav: i16) -> (Session, Vec<NpcId>, NpcId) {
        let catalog = ships_catalog(ships(behav));
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, &catalog);
        let marked = session
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some())
            .map(|npc| npc.id)
            .collect();
        let hired = session.fleet[0].expect("placed");
        (session, marked, hired)
    }

    #[test]
    fn the_player_leads_hired_and_mission_escorts_in_ship_table_order() {
        let (session, mission, hired) = flying(1);
        assert_eq!(mission.len(), 2);
        // The mission's ships are set up with the system, before the hired
        // escort is placed: the ship table puts them first, where the
        // formation gives the hired escort slot 2.
        assert_eq!(led(&session), [mission[0], mission[1], hired]);
        let slot = |id| {
            session
                .npc(id)
                .and_then(Npc::formation)
                .map(|place| place.slot)
        };
        assert_eq!(slot(hired), Some(2));
        assert_eq!(slot(mission[0]), Some(3));
    }

    #[test]
    fn a_mission_ship_that_never_joins_is_not_led() {
        let (session, mission, hired) = flying(0);
        assert_eq!(mission.len(), 2);
        assert_eq!(led(&session), [hired]);
    }

    #[test]
    fn a_released_hired_escort_is_no_longer_led() {
        let (mut session, mission, hired) = flying(1);
        session.release(hired);
        assert_eq!(led(&session), mission);
        assert!(session.npc(hired).is_some(), "still here as it leaves");
    }

    #[test]
    fn a_joined_mission_escort_disabled_stays_led() {
        let catalog = ships_catalog(ships(1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let mission = led(&session);
        assert_eq!(mission.len(), 2);
        session.traffic.npcs_mut()[0].condition = Condition::Disabled;
        ticks(&mut session, &catalog, 5);
        assert_eq!(led(&session), mission);
    }
}
