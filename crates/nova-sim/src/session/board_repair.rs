//! Boarding the player's own disabled escorts and fighters back: the
//! session's side of [`board_repair`](crate::board_repair), the own-ship
//! paths of `_HandlePlayerBoardAttempt` (@0x65843-0x65eb3).
//!
//! **Memory.** A ship that left the fleet disabled remembers what it was
//! ([`Npc::former`](crate::traffic::npc::Npc::former), the original's
//! +0xc8de). Boarding it on the plunder, offer or mission-ship paths
//! leaves that memory, as the original clears +0xc8de only on the
//! own-ship paths (@0x65b2f, @0x65be8, @0x65e62): so a former escort
//! plundered while the fleet is full can be boarded again, plundered
//! again, and repaired once there is room.

#[cfg(test)]
mod tests {
    use crate::board::{Boarding, MAX_ESCORTS, Take, Taken};
    use crate::catalog::ShipId;
    use crate::legal::NovaLaw;
    use crate::pilot::Escort;
    use crate::self_repair::FormerEscort;
    use crate::session::Session;
    use crate::session::tests::{alongside, board_by, boardable, take, trader_escort};

    /// A former escort's memory: the trader, hired at 30 a day.
    fn former_trader() -> FormerEscort {
        FormerEscort {
            record: Escort {
                wage: Some(30),
                ..trader_escort()
            },
        }
    }

    /// Fills the fleet with [`MAX_ESCORTS`] interceptors, unplaced.
    fn fill_fleet(session: &mut Session) {
        session.pilot.escorts = vec![
            Escort {
                ship: ShipId(130),
                ..trader_escort()
            };
            MAX_ESCORTS
        ];
    }

    #[test]
    fn a_former_escort_plundered_keeps_its_memory_and_can_be_boarded_again() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        fill_fleet(&mut session);
        session.space.traffic.npcs_mut()[0].former = Some(former_trader());
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(_))
        ));
        let npc = &session.npcs()[0];
        assert!(npc.boarded);
        assert_eq!(npc.former, Some(former_trader()), "the plunder keeps it");
        assert_eq!(take(&mut session, Take::Abort, &[]).0, Taken::Aborted);
        assert!(
            matches!(
                board_by(&mut session, &NovaLaw::default()),
                Ok(Boarding::Opened(_))
            ),
            "boarded again (@0x65064-0x6507a)"
        );
        assert_eq!(session.npcs()[0].former, Some(former_trader()));
    }

    #[test]
    fn a_ship_boarded_before_that_never_was_an_escort_is_refused() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.space.traffic.npcs_mut()[0].boarded = true;
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(crate::board::BoardRefusal::CantBoard)
        );
    }
}
