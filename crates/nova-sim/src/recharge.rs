//! Recharging in the spaceport: which stellars sell fuel, what filling the
//! tank costs, and the refill itself.
//!
//! A stellar sells fuel unless it is uninhabited. The Nova Bible's `spöb`
//! flag list has one fuel bit, `0x00000020  Stellar is uninhabited (no
//! traffic control or refuelling)` ([`StellarFlags::UNINHABITED`]); no
//! other `spöb` field or `Flags2` bit mentions fuel.
//!
//! Recharging always fills the tank, for [`FUEL_PRICE_PER_UNIT`] credits
//! a unit, rounded up to whole credits. It is refused, and nothing
//! changes, when the tank is already full (a ship with no tank counts as
//! full) or the pilot can't pay for the whole refill. Ammunition and
//! fighters are not recharged yet.

use crate::landing::StellarFlags;
use crate::pilot::Pilot;
use crate::reserves::Gauge;

/// What a unit of fuel costs, in credits: 100 credits a jump.
///
/// A placeholder: neither the Nova Bible nor the game's documentation
/// gives a fuel price, so no formula backs this number.
pub const FUEL_PRICE_PER_UNIT: i64 = 1;

/// Why the spaceport won't recharge the ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RechargeRefusal {
    /// No fuel is sold: the ship is not landed, or the stellar sells none.
    NoFuel,
    /// The tank is already full.
    Full,
    /// The player cannot pay for the whole refill.
    CannotAfford,
}

/// Whether a stellar with these `spöb` flags sells fuel: any but an
/// uninhabited one.
#[must_use]
pub fn sells_fuel(flags: u32) -> bool {
    flags & StellarFlags::UNINHABITED == 0
}

/// What filling `fuel` costs: [`FUEL_PRICE_PER_UNIT`] for each unit
/// missing, rounded up to whole credits. Refused when the tank is full,
/// or `cash` cannot pay for it.
pub fn quote(fuel: Gauge, cash: i64) -> Result<i64, RechargeRefusal> {
    if fuel.now >= fuel.max {
        return Err(RechargeRefusal::Full);
    }
    // The gauge holds at most a ship's `Fuel` (an `i16`), so the missing
    // units always fit.
    #[allow(clippy::cast_possible_truncation)]
    let units = (fuel.max - fuel.now).ceil() as i64;
    let price = units.saturating_mul(FUEL_PRICE_PER_UNIT);
    if price > cash {
        return Err(RechargeRefusal::CannotAfford);
    }
    Ok(price)
}

/// Fills `pilot`'s tank and takes `price` from its cash.
pub(crate) fn settle(pilot: &mut Pilot, price: i64) {
    pilot.reserves.fuel.now = pilot.reserves.fuel.max;
    pilot.cash = pilot.cash.saturating_sub(price);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::catalog;

    fn tank(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    #[test]
    fn every_stellar_but_an_uninhabited_one_sells_fuel() {
        for flags in [0x01, 0x57, 0x4242_2157] {
            assert!(sells_fuel(flags), "{flags:#x}");
        }
        for flags in [0x20, 0x21, 0x4242_2177, u32::MAX] {
            assert!(!sells_fuel(flags), "{flags:#x}");
        }
    }

    #[test]
    fn a_refill_costs_a_credit_for_each_unit_missing() {
        assert_eq!(FUEL_PRICE_PER_UNIT, 1);
        assert_eq!(quote(tank(150.0, 300.0), 25_000), Ok(150));
        assert_eq!(quote(tank(0.0, 300.0), 25_000), Ok(300));
    }

    #[test]
    fn a_part_unit_missing_costs_a_whole_credit() {
        assert_eq!(quote(tank(299.5, 300.0), 25_000), Ok(1));
        assert_eq!(quote(tank(149.25, 300.0), 25_000), Ok(151));
    }

    #[test]
    fn a_full_tank_or_none_is_refused() {
        assert_eq!(
            quote(tank(300.0, 300.0), 25_000),
            Err(RechargeRefusal::Full)
        );
        assert_eq!(quote(tank(0.0, 0.0), 25_000), Err(RechargeRefusal::Full));
    }

    #[test]
    fn the_whole_refill_must_be_paid_for() {
        assert_eq!(
            quote(tank(150.0, 300.0), 149),
            Err(RechargeRefusal::CannotAfford)
        );
        assert_eq!(quote(tank(150.0, 300.0), 150), Ok(150));
    }

    #[test]
    fn settling_fills_the_tank_and_takes_the_price() {
        let mut pilot = Pilot::new(&catalog(), "Kane").expect("a pilot");
        pilot.reserves.fuel = tank(150.0, 300.0);
        pilot.cash = 1000;
        let before = pilot.clone();
        settle(&mut pilot, 150);
        assert_eq!(pilot.reserves.fuel, Gauge::full(300.0));
        assert_eq!(pilot.cash, 850);
        pilot.reserves.fuel = before.reserves.fuel;
        pilot.cash = before.cash;
        assert_eq!(pilot, before, "nothing else changes");
    }
}
