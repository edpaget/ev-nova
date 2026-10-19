//! A ship's help to the player once asked: refuelling or repairing it.
//!
//! The rules are the original's (`_AIMakeShipRefuelPlayer` @0x82dbb,
//! `_AIMakeShipRepairPlayer` @0x82df2 and their AI states in
//! `_HighLevelAIHandler` in the `EV Nova` executable). The help the
//! player [`need`]s is a repair while it is disabled, else fuel while it
//! holds less than [`REFUEL_UNTIL`] and has a tank at all.
//!
//! The traffic flies the help itself, docking with the ship helped as a
//! pirate does: a refuel ([`refuel_dock`](crate::traffic::refuel_dock))
//! and a repair ([`repair_dock`](crate::traffic::repair_dock)).

use super::Help;
use crate::combat::hull::Condition;
use crate::reserves::Gauge;

/// The fuel a refuel tops up past: one jump's (the float 100.0 @0xdd060).
pub const REFUEL_UNTIL: f32 = 100.0;

/// The help the player needs, its ship in `condition` with `fuel` (see
/// the module docs).
#[must_use]
pub fn need(condition: Condition, fuel: Gauge) -> Option<Help> {
    if condition == Condition::Disabled {
        Some(Help::Repair)
    } else if fuel.now < REFUEL_UNTIL && fuel.max > 0.0 {
        Some(Help::Refuel)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn fuel(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    #[test]
    fn a_disabled_player_needs_a_repair_even_with_little_fuel() {
        assert_eq!(
            need(Condition::Disabled, fuel(0.0, 300.0)),
            Some(Help::Repair)
        );
        assert_eq!(
            need(Condition::Disabled, fuel(300.0, 300.0)),
            Some(Help::Repair)
        );
    }

    #[test]
    fn a_player_with_less_than_a_jumps_fuel_needs_a_refuel() {
        assert_eq!(
            need(Condition::Intact, fuel(99.0, 300.0)),
            Some(Help::Refuel)
        );
        assert_eq!(
            need(Condition::Intact, fuel(99.9, 100.0)),
            Some(Help::Refuel)
        );
        assert_eq!(need(Condition::Intact, fuel(100.0, 300.0)), None);
        assert_eq!(need(Condition::Intact, fuel(0.0, 0.0)), None, "no tank");
    }

    #[test]
    fn the_values_are_the_originals() {
        assert_eq!(REFUEL_UNTIL, 100.0);
    }
}
