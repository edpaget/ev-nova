//! The free cargo space the player's takes measure: holds less every
//! ton carried (`_TotalCargo(player)`: commodities, `jünk` and the
//! mission cargo aboard), each caller naming its holds.
//!
//! - **The outfitter's mass expansion** reads the ship's own holds
//!   (`_CanBuyOutfitItem` @0x4e980-0x4e9a8 subtracts `_TotalCargo(player)`
//!   from `_ShipTotalHolds()`; [`PlayerShip::free_cargo`]).
//! - **The plunder's cargo take** reads the whole fleet's
//!   (`_DoPlunderDialog` @0x9372e-0x9374b subtracts it from
//!   `_TotalFleetHolds()`; [`PlayerShip::fleet_free_cargo`]): a trader
//!   escort's `Holds` add room, a warship's don't
//!   ([`market::fleet_holds`](crate::market::fleet_holds)). An offer above
//!   the room is cut to it (@0x9374f-0x93768); none or less is the hold
//!   full (@0x9377b-0x93808).

use super::{PlayerShip, Statics};
use crate::pilot::Pilot;

/// `holds` less every ton `pilot` carries: below none when it carries
/// more.
pub(super) fn free_cargo_in(holds: u32, pilot: &Pilot) -> i64 {
    i64::from(holds) - pilot.total_cargo()
}

impl PlayerShip {
    /// The fleet's free cargo, as the plunder's cargo take reads it (see
    /// the module docs): the fleet's holds
    /// ([`PlayerShip::capacity`]) less every ton carried.
    pub(super) fn fleet_free_cargo(&self, pilot: &Pilot, statics: &Statics) -> i64 {
        free_cargo_in(self.capacity(pilot, statics), pilot)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::board::{Boarding, Take, Taken};
    use crate::catalog::ShipId;
    use crate::legal::NovaLaw;
    use crate::market::Good;
    use crate::pilot::{Escort, Pilot};
    use crate::session::Session;
    use crate::session::tests::{alongside, board_by, boardable, take, trader_escort};
    use crate::testkit::FakePilotCatalog;

    const FOOD: Good = Good::Commodity(0);

    /// [`alongside`]'s trader boarded in `catalog`, with `fleet` and the
    /// player holding 15 of its 20 tons in food: the cargo take, of the 13
    /// food on board.
    fn take_cargo(catalog: &FakePilotCatalog, fleet: Vec<Escort>) -> (Taken, Session) {
        let mut session = alongside(catalog);
        session.pilot.escorts = fleet;
        session.pilot.cargo = BTreeMap::from([(FOOD, 15)]);
        assert!(matches!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(_))
        ));
        (take(&mut session, Take::Cargo, &[]).0, session)
    }

    #[test]
    fn plundered_cargo_fills_a_trader_escorts_holds_too() {
        // `_TotalFleetHolds()` − `_TotalCargo(player)` (@0x9372e-0x9374b):
        // 20 + its 20 less 15.
        let (taken, session) = take_cargo(&boardable(), vec![trader_escort()]);
        assert_eq!(
            taken,
            Taken::Cargo {
                good: FOOD,
                stored: 13
            }
        );
        assert_eq!(session.pilot().held(FOOD), 28);
    }

    #[test]
    fn a_warship_escort_adds_no_room() {
        let mut catalog = boardable();
        for record in &mut catalog.ship_records {
            if record.id == ShipId(130) {
                record.inherent_ai = 3;
            }
        }
        let warship = Escort {
            ship: ShipId(130),
            ..trader_escort()
        };
        let (taken, _) = take_cargo(&catalog, vec![warship]);
        assert_eq!(
            taken,
            Taken::Cargo {
                good: FOOD,
                stored: 5
            },
            "the ship's own 20 less 15"
        );
    }

    #[test]
    fn the_free_cargo_is_the_holds_less_everything_carried() {
        let mut pilot = Pilot::new(&boardable(), "Ada").expect("starts");
        pilot.cargo = BTreeMap::from([(FOOD, 15)]);
        assert_eq!(super::free_cargo_in(20, &pilot), 5);
        assert_eq!(super::free_cargo_in(10, &pilot), -5, "below none");
    }
}
