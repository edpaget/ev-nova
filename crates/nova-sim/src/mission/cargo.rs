//! Mission cargo: the room an accept and a pickup need for it ([`Room`]),
//! and when it is loaded ([`loads_at_accept`]). See the [module
//! docs](super)' "Cargo" for the original's rules.

use crate::catalog::MissionRecord;

/// `PickupMode`: the cargo is loaded as the mission is accepted.
pub const PICKUP_AT_ACCEPT: i16 = 0;
/// `PickupMode`: the cargo is loaded at the destination.
pub const PICKUP_AT_DESTINATION: i16 = 1;
/// `PickupMode`: the cargo is loaded on boarding the mission's special
/// ship.
pub const PICKUP_BOARDING: i16 = 2;
/// `DropOffMode`: the cargo is unloaded at the destination.
pub const DROP_OFF_AT_DESTINATION: i16 = 0;
/// `DropOffMode`: the cargo is unloaded at the return.
pub const DROP_OFF_AT_RETURN: i16 = 1;

/// Whether accepting `record` loads its cargo (`_DoMissionAccept`
/// @0xa1f28-0xa1f3f): with `PickupMode` 0, whatever its cargo type.
#[must_use]
pub fn loads_at_accept(record: &MissionRecord) -> bool {
    record.pickup_mode == PICKUP_AT_ACCEPT
}

/// The room a mission's cargo needs, as an accept (`_DoMissionAccept`
/// @0xa1b61-0xa1be5) and a pickup (`_DoMissionCargoPickup` @0x98292)
/// reckon it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Room {
    /// The cargo's tons.
    pub tons: i64,
    /// The ship's own holds, with its cargo outfits (`_ShipTotalHolds`
    /// @0x5cb3).
    pub holds: i64,
    /// The room for a mission's cargo (`_TotalMissionCargoSpace`
    /// @0xcffb): the ship's own holds less what it holds that the trader
    /// escorts' holds do not take; below 0 when it holds more.
    pub space: i64,
}

impl Room {
    /// Whether the ship's holds take the tons: always for none or fewer.
    #[must_use]
    pub fn holds_take(self) -> bool {
        self.tons <= 0 || self.holds >= self.tons
    }

    /// Whether the free space takes the tons: always for none or fewer.
    #[must_use]
    pub fn space_takes(self) -> bool {
        self.tons <= 0 || self.space >= self.tons
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::mission;

    fn picking_up(pickup_mode: i16, cargo_type: i16) -> MissionRecord {
        MissionRecord {
            cargo_type,
            pickup_mode,
            ..mission(140)
        }
    }

    #[test]
    fn only_pickup_mode_0_loads_at_accept_whatever_the_cargo() {
        assert!(loads_at_accept(&picking_up(0, 0)));
        assert!(loads_at_accept(&picking_up(0, -1)), "no cargo type");
        for mode in [1, 2, -1, 3] {
            assert!(!loads_at_accept(&picking_up(mode, 0)), "{mode}");
        }
    }

    fn room(tons: i64, holds: i64, space: i64) -> Room {
        Room { tons, holds, space }
    }

    #[test]
    fn the_holds_and_the_space_each_take_up_to_their_tons() {
        assert!(room(10, 10, 10).holds_take(), "exactly");
        assert!(room(10, 10, 10).space_takes(), "exactly");
        assert!(!room(10, 9, 10).holds_take());
        assert!(room(10, 9, 10).space_takes());
        assert!(!room(10, 10, 9).space_takes());
        assert!(room(10, 10, 9).holds_take());
    }

    #[test]
    fn no_tons_need_no_room() {
        for tons in [0, -1, i64::MIN] {
            assert!(room(tons, -5, -5).holds_take(), "{tons}");
            assert!(room(tons, -5, -5).space_takes(), "{tons}");
        }
        assert!(!room(1, 0, 0).holds_take());
        assert!(!room(1, 0, 0).space_takes());
    }
}
