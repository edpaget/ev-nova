//! The auto-refueller: landing at an inhabited stellar with one tops up
//! the tank at a credit a unit, as `_PlayerLandOnStellar` calls
//! `_DoAutoRefuel` (@0x632c5, body @0x41bc5-0x41c8c in `EV Nova.app`'s
//! i386 executable).
//!
//! `_PlayerLandOnStellar` tests the docked stellar's runtime `Flags`
//! (+0x30) for 0x20 (@0x632ac-0x632c3) and calls `_DoAutoRefuel` only
//! when it is clear, so only at an inhabited stellar; an uninhabited one
//! beeps instead (@0x632cc-0x632ef). It runs after the landing's precalced
//! reset (@0x63211) and before `_DoPortDialog` (@0x63381), so before the
//! port's mission land check (@0x5fbeb) and any Recharge.
//!
//! The body, whole:
//!
//! - @0x41bd0-0x41c0f: the player's owned outfit counts are walked beside
//!   the outfit records, and any owned outfit (count above 0) with
//!   [`AUTO_REFUELLER`] among its four `ModType`s sets the flag
//!   ([`carries_auto_refueller`]). `ModVal` is never read.
//! - @0x41c11-0x41c33: it returns at once unless the flag is set and the
//!   whole-number fuel is below `_ShipFuelCapacity`, so a tank full in
//!   whole units, or above capacity, is left as it is: no truncation and
//!   no refund ([`fill`] gives `None`), unlike the port's Recharge.
//! - @0x41c35-0x41c78: otherwise the fuel is truncated toward zero, the
//!   missing units are the capacity less that, truncated, clamped to the
//!   cash (`cmovgl` @0x41c5d) and charged a credit each, exactly as
//!   Recharge's fill at a stellar not dominated
//!   ([`recharge::fill`](crate::recharge::fill)). In debt the clamp makes
//!   the units the negative cash, so by the engine the debt is cleared and
//!   the tank drained by as many units; [`RuleKey::RechargeDebt`] governs
//!   it as it does Recharge's.
//! - It never reads the stellar's domination byte (+0x46), so a dominated
//!   stellar still charges.
//! - @0x41c6e-0x41c81: it marks the status bar for a redraw, which the
//!   view needs no analogue of: it draws the status bar from the
//!   session's state every frame, and the session settles the refuel as
//!   it docks, before the spaceport's first frame.
//! - It makes no sound: no `PlaySound`, no `_beepSnd`.
//!
//! The original's 16-bit casts of the fuel, the capacity and the clamped
//! cash (@0x41c35, @0x41c57, @0x41c58) are not replicated: a debt below
//! -32768 would wrap the clamp there.
//!
//! [`RuleKey::RechargeDebt`]: crate::RuleKey::RechargeDebt

use crate::fuel::OutfitMod;
use crate::recharge::{self, Fill};
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;

/// The `oütf` `ModType` that tops up the fuel on landing: the Bible's
/// "auto-refueller", the 0x13 `_DoAutoRefuel` scans for (@0x41bd0-0x41c0f).
pub const AUTO_REFUELLER: i16 = 19;

/// Whether any of the ship's outfit `mods` is an [`AUTO_REFUELLER`] it
/// carries (@0x41bd0-0x41c0f).
#[must_use]
pub fn carries_auto_refueller(mods: &[OutfitMod]) -> bool {
    mods.iter()
        .any(|outfit| outfit.mod_type == AUTO_REFUELLER && outfit.count > 0)
}

/// The top-up `_DoAutoRefuel` makes of a tank at `fuel` with `cash`, its
/// debt branch read by `debt` (`RuleKey::RechargeDebt`): `None` when the
/// whole-number fuel is not below capacity (@0x41c23-0x41c33), and
/// otherwise Recharge's fill at a stellar not dominated, as
/// `_DoAutoRefuel` never reads +0x46. [`recharge::settle`] applies it.
#[must_use]
pub fn fill(fuel: Gauge, cash: i64, debt: RuleSource) -> Option<Fill> {
    (fuel.now.trunc() < fuel.max).then(|| recharge::fill(fuel, cash, false, debt))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::fuel::FUEL_SCOOP;
    use crate::pilot::Pilot;
    use nova_sim_catalog::fixture::catalog;

    fn tank(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    fn mod_of(mod_type: i16, count: u16) -> OutfitMod {
        OutfitMod {
            mod_type,
            mod_val: 0,
            count,
        }
    }

    #[test]
    fn the_auto_refueller_is_mod_type_19() {
        assert_eq!(AUTO_REFUELLER, 19);
    }

    #[test]
    fn an_auto_refueller_carried_in_any_of_its_four_mods_counts() {
        // `outfit_mods` gives each record's four mods in turn.
        for at in 0..4 {
            let mut mods = [mod_of(0, 1); 4];
            mods[at] = mod_of(AUTO_REFUELLER, 1);
            assert!(carries_auto_refueller(&mods), "mod {at}");
        }
        assert!(carries_auto_refueller(&[
            mod_of(FUEL_SCOOP, 2),
            mod_of(AUTO_REFUELLER, 3)
        ]));
    }

    #[test]
    fn none_carried_or_another_mod_type_is_no_auto_refueller() {
        assert!(!carries_auto_refueller(&[]));
        assert!(!carries_auto_refueller(&[mod_of(AUTO_REFUELLER, 0)]));
        assert!(!carries_auto_refueller(&[
            mod_of(FUEL_SCOOP, 1),
            mod_of(12, 1),
            mod_of(20, 1)
        ]));
    }

    #[test]
    fn a_tank_full_in_whole_units_or_above_is_left_alone() {
        for debt in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(fill(tank(400.0, 400.0), 1000, debt), None);
            assert_eq!(fill(tank(400.7, 400.0), 1000, debt), None);
            assert_eq!(fill(tank(450.0, 400.0), 1000, debt), None);
            assert_eq!(fill(tank(0.0, 0.0), 1000, debt), None, "no tank");
        }
    }

    /// `fill` settled on a pilot with the tank at `now` of `max` and
    /// `cash`: the tank and the cash after.
    fn settled(now: f32, max: f32, cash: i64, debt: RuleSource) -> (f32, i64) {
        let mut pilot = Pilot::new(&catalog(), "Kane").expect("a pilot");
        pilot.reserves.fuel = tank(now, max);
        pilot.cash = cash;
        let top_up = fill(pilot.reserves.fuel, pilot.cash, debt).expect("tops up");
        recharge::settle(&mut pilot, top_up);
        (pilot.reserves.fuel.now, pilot.cash)
    }

    #[test]
    fn a_tank_below_capacity_fills_at_a_credit_a_unit() {
        assert_eq!(
            fill(tank(150.5, 400.0), 1000, RuleSource::Engine),
            Some(Fill {
                units: 250,
                price: 250
            })
        );
        assert_eq!(
            fill(tank(399.5, 400.0), 1000, RuleSource::Engine),
            Some(Fill { units: 1, price: 1 })
        );
        assert_eq!(
            settled(150.5, 400.0, 1000, RuleSource::Engine),
            (400.0, 750)
        );
    }

    #[test]
    fn too_little_cash_fills_what_it_buys_and_none_only_truncates() {
        assert_eq!(
            fill(tank(100.0, 400.0), 30, RuleSource::Engine),
            Some(Fill {
                units: 30,
                price: 30
            })
        );
        assert_eq!(settled(100.0, 400.0, 30, RuleSource::Engine), (130.0, 0));
        assert_eq!(
            fill(tank(50.5, 400.0), 0, RuleSource::Engine),
            Some(Fill { units: 0, price: 0 })
        );
        assert_eq!(settled(50.5, 400.0, 0, RuleSource::Engine), (50.0, 0));
    }

    #[test]
    fn by_the_engine_a_debt_is_cleared_with_fuel_by_the_other_reading_it_stands() {
        assert_eq!(
            fill(tank(100.5, 400.0), -30, RuleSource::Engine),
            Some(Fill {
                units: -30,
                price: -30
            })
        );
        assert_eq!(settled(100.5, 400.0, -30, RuleSource::Engine), (70.0, 0));
        assert_eq!(
            fill(tank(100.5, 400.0), -30, RuleSource::Bible),
            Some(Fill { units: 0, price: 0 })
        );
        assert_eq!(settled(100.5, 400.0, -30, RuleSource::Bible), (100.0, -30));
    }

    #[test]
    fn a_tank_below_empty_truncates_toward_zero() {
        assert_eq!(
            fill(tank(-3.5, 400.0), 1000, RuleSource::Engine),
            Some(Fill {
                units: 403,
                price: 403
            })
        );
        assert_eq!(settled(-3.5, 400.0, 1000, RuleSource::Engine), (400.0, 597));
    }
}
