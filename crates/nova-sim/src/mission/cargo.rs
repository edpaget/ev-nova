//! Mission cargo: the room an accept and a pickup need for it ([`Room`]).
//! See the [module docs](super)' "Cargo" for the original's rules.

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
