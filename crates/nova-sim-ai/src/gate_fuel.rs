//! What a passage through a hypergate costs in fuel, by each reading of
//! [`RuleKey::GateFuel`](crate::RuleKey::GateFuel): the rule's one home,
//! which the player's gate entry (`nova_sim::Session::enter_hypergate`)
//! and the traffic's destination pick
//! ([`traffic::destination`](crate::traffic::destination)) both read.
//!
//! By the engine, a passage costs nothing and asks for nothing: the
//! original's `_PlayerEnterHypergate` @0x637bf-0x64005 never touches the
//! ship's fuel (+0x38), nor does an NPC's gate entry (`_HighLevelAIHandler`
//! state 20, @0x8db5f-0x8dd5e). By the other reading, the new game's, a
//! passage costs the hull's gate fuel ([`ShipStats::gate_fuel`]), or one
//! jump's worth ([`JUMP_FUEL`]) when its data gives none, and a ship
//! holding less can't take it: [`affords`] is inclusive, as a jump's fuel
//! check is.

use crate::hyperspace::JUMP_FUEL;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;

/// The fuel a hypergate passage costs a ship with `stats` by the
/// spending reading: its hull's gate fuel, or [`JUMP_FUEL`] when its data
/// gives none.
#[must_use]
pub fn fare(stats: &ShipStats) -> f32 {
    stats.gate_fuel.unwrap_or(JUMP_FUEL)
}

/// Whether a ship with `stats` holding `fuel_now` may pass through a
/// hypergate by `reading`: always by the engine, and by the other reading
/// when it holds at least its [`fare`].
#[must_use]
pub fn affords(fuel_now: f32, stats: &ShipStats, reading: RuleSource) -> bool {
    match reading {
        RuleSource::Engine => true,
        RuleSource::Bible => fuel_now >= fare(stats),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn hull(gate_fuel: Option<f32>) -> ShipStats {
        ShipStats {
            gate_fuel,
            ..ShipStats::default()
        }
    }

    #[test]
    fn a_hull_with_no_amount_costs_one_jumps_worth() {
        assert_eq!(fare(&hull(None)), JUMP_FUEL);
    }

    #[test]
    fn a_hull_with_an_amount_costs_that_amount() {
        assert_eq!(fare(&hull(Some(150.0))), 150.0);
        assert_eq!(fare(&hull(Some(0.0))), 0.0);
    }

    #[test]
    fn by_the_spending_reading_a_ship_affords_its_fare_and_no_less() {
        let stats = hull(Some(150.0));
        assert!(affords(150.0, &stats, RuleSource::Bible));
        assert!(affords(400.0, &stats, RuleSource::Bible));
        assert!(!affords(149.0, &stats, RuleSource::Bible));
        assert!(!affords(0.0, &stats, RuleSource::Bible));
        let default = hull(None);
        assert!(affords(JUMP_FUEL, &default, RuleSource::Bible));
        assert!(!affords(JUMP_FUEL - 1.0, &default, RuleSource::Bible));
    }

    #[test]
    fn a_free_passage_is_afforded_on_an_empty_tank() {
        assert!(affords(0.0, &hull(Some(0.0)), RuleSource::Bible));
    }

    #[test]
    fn by_the_engine_any_fuel_affords_a_passage() {
        for fuel in [0.0, 1.0, 149.0, 400.0] {
            assert!(
                affords(fuel, &hull(Some(150.0)), RuleSource::Engine),
                "{fuel}"
            );
            assert!(affords(fuel, &hull(None), RuleSource::Engine), "{fuel}");
        }
    }
}
