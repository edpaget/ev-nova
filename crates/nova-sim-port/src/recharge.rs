//! Recharging in the spaceport: which stellars sell fuel, when the
//! Recharge button is enabled, and the refill itself, as `_DoPortDialog`
//! answers Recharge (item 4, @0x5fd5d-0x5fe25 in `EV Nova.app`'s i386
//! executable).
//!
//! A stellar sells fuel unless it is uninhabited ([`sells_fuel`]): the
//! answer does nothing at a stellar with `Flags` 0x20 (@0x5fd6c). The
//! Nova Bible's `spöb` flag list agrees: `0x00000020  Stellar is
//! uninhabited (no traffic control or refuelling)`.
//!
//! `_DrawPlanetButtons` draws Recharge enabled, and `_TrackPlanetButtons`
//! takes a click on it, only where fuel is sold, the cash (a signed long,
//! `_shipData`+0xa0) is above 0 and the tank (`_shipData`+0x38) is below
//! its capacity (@0x297a3-0x297f4, @0x29ae2-0x29b28): [`enabled`]. The
//! keys R and F skip that check (`_PortFilter` @0x4f209-0x4f257), so the
//! fill below meets a full tank, an empty purse or a debt too.
//!
//! The fill ([`fill`], applied by [`settle`]):
//!
//! - The fuel is truncated to whole units in place (@0x5fd8b-0x5fd97),
//!   then the missing units are the capacity less that, truncated
//!   (@0x5fda2-0x5fdb6).
//! - At a stellar not dominated (its +0x46 byte clear, @0x5fdb9), the
//!   missing units are clamped to the cash and cost a credit each
//!   ([`FUEL_PRICE_PER_UNIT`], @0x5fdbf-0x5fdcf), so too little cash
//!   fills as many units as it buys and leaves the cash at 0.
//! - At a dominated stellar the whole tank fills free. The engine knows
//!   only the start state of +0x46, `Flags2` 0x0020 (`_ResetPlayerRecord`
//!   @0x1dbcf-0x1dbef); domination at run time is not modelled yet.
//! - Above capacity, the missing units are negative and not clamped, so
//!   the tank comes down to capacity and the excess is credited.
//! - In debt, by the engine (`RuleKey::RechargeDebt`), the clamp makes
//!   the missing units the negative cash: the debt is cleared and the
//!   tank drained by as many units, possibly below empty. By the other
//!   reading the cash counts as 0 and nothing fills.
//!
//! There is no refusal text and no sound on any path. The original's
//! 16-bit casts of the fuel, the units and the cash (@0x5fd90, @0x5fdb6,
//! @0x5fdc5) are not replicated. Ammunition and fighters are not
//! recharged here.

use crate::landing::StellarFlags;
use crate::pilot::Pilot;
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;

/// What a unit of fuel costs, in credits: 100 credits a jump. The fill
/// takes one credit from the cash for each unit (`_DoPortDialog`
/// @0x5fdcd).
pub const FUEL_PRICE_PER_UNIT: i64 = 1;

/// Why the spaceport won't recharge the ship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RechargeRefusal {
    /// No fuel is sold: the ship is not landed, or the stellar sells none.
    NoFuel,
}

/// Whether a stellar with these `spöb` flags sells fuel: any but an
/// uninhabited one.
#[must_use]
pub fn sells_fuel(flags: u32) -> bool {
    flags & StellarFlags::UNINHABITED == 0
}

/// Whether the Recharge button is enabled at a stellar with `flags`, with
/// the tank at `fuel` and `cash`: fuel is sold, the cash is above 0 and
/// the tank is below its capacity (`_DrawPlanetButtons` @0x297a3-0x297f4).
/// A part unit short of full counts. Domination is never read here.
#[must_use]
pub fn enabled(flags: u32, fuel: Gauge, cash: i64) -> bool {
    sells_fuel(flags) && cash > 0 && fuel.max > fuel.now
}

/// A refill: the whole units added to the truncated tank, which may be
/// negative, and what it costs, which may be 0 or a refund.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fill {
    /// The units added to the fuel once it is truncated.
    pub units: i64,
    /// The credits taken from the cash.
    pub price: i64,
}

/// The refill `_DoPortDialog` makes of a tank at `fuel` with `cash`, at a
/// stellar `dominated` or not, its debt branch read by `debt`
/// (`RuleKey::RechargeDebt`). See the module docs.
#[must_use]
pub fn fill(fuel: Gauge, cash: i64, dominated: bool, debt: RuleSource) -> Fill {
    // The gauge holds at most a ship's `Fuel` (an `i16`), so the units
    // always fit.
    #[allow(clippy::cast_possible_truncation)]
    let missing = (fuel.max - fuel.now.trunc()).trunc() as i64;
    if dominated {
        return Fill {
            units: missing,
            price: 0,
        };
    }
    let cash = match debt {
        RuleSource::Engine => cash,
        RuleSource::Bible => cash.max(0),
    };
    let units = missing.min(cash);
    Fill {
        units,
        price: units.saturating_mul(FUEL_PRICE_PER_UNIT),
    }
}

/// Applies `fill` to `pilot`: truncates its fuel to whole units, adds the
/// units and takes the price from its cash.
pub fn settle(pilot: &mut Pilot, fill: Fill) {
    let fuel = &mut pilot.reserves.fuel;
    // The units fit in a ship's `Fuel` (see [`fill`]).
    #[allow(clippy::cast_precision_loss)]
    let units = fill.units as f32;
    fuel.now = fuel.now.trunc() + units;
    pilot.cash = pilot.cash.saturating_sub(fill.price);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rulebook::RuleSource;
    use nova_sim_catalog::fixture::catalog;

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
    fn recharge_is_enabled_with_fuel_sold_cash_and_room_in_the_tank() {
        const LAND: u32 = 0x01;
        assert!(enabled(LAND, tank(299.5, 300.0), 1));
        assert!(enabled(LAND, tank(0.0, 300.0), 25_000));
        assert!(!enabled(
            LAND | StellarFlags::UNINHABITED,
            tank(0.0, 300.0),
            25_000
        ));
        assert!(!enabled(LAND, tank(0.0, 300.0), 0));
        assert!(!enabled(LAND, tank(0.0, 300.0), -1));
        assert!(!enabled(LAND, tank(300.0, 300.0), 25_000));
        assert!(!enabled(LAND, tank(301.0, 300.0), 25_000));
        assert!(!enabled(LAND, tank(0.0, 0.0), 25_000), "no tank");
    }

    /// The fill as `_DoPortDialog` makes it at a stellar not dominated,
    /// by the engine's reading, settled on a pilot: the tank and the cash
    /// after.
    fn filled(now: f32, max: f32, cash: i64) -> (f32, i64) {
        settled(now, max, cash, false, RuleSource::Engine)
    }

    fn settled(now: f32, max: f32, cash: i64, dominated: bool, debt: RuleSource) -> (f32, i64) {
        let mut pilot = Pilot::new(&catalog(), "Kane").expect("a pilot");
        pilot.reserves.fuel = tank(now, max);
        pilot.cash = cash;
        let fill = fill(pilot.reserves.fuel, pilot.cash, dominated, debt);
        settle(&mut pilot, fill);
        (pilot.reserves.fuel.now, pilot.cash)
    }

    #[test]
    fn a_unit_costs_a_credit() {
        assert_eq!(FUEL_PRICE_PER_UNIT, 1);
        let fill = fill(tank(150.0, 300.0), 25_000, false, RuleSource::Engine);
        assert_eq!(
            fill,
            Fill {
                units: 150,
                price: 150
            }
        );
        assert_eq!(filled(150.0, 300.0, 25_000), (300.0, 24_850));
        assert_eq!(filled(0.0, 300.0, 25_000), (300.0, 24_700));
    }

    #[test]
    fn too_little_cash_fills_as_many_units_as_it_buys() {
        let fill = fill(tank(150.0, 300.0), 100, false, RuleSource::Engine);
        assert_eq!(
            fill,
            Fill {
                units: 100,
                price: 100
            }
        );
        assert_eq!(filled(150.0, 300.0, 100), (250.0, 0));
    }

    #[test]
    fn the_fuel_is_truncated_to_whole_units_first() {
        assert_eq!(filled(149.25, 300.0, 25_000), (300.0, 24_849));
        assert_eq!(filled(149.25, 300.0, 0), (149.0, 0));
        assert_eq!(filled(299.5, 300.0, 25_000), (300.0, 24_999));
    }

    #[test]
    fn a_full_tank_or_none_fills_nothing() {
        let full = fill(tank(300.0, 300.0), 25_000, false, RuleSource::Engine);
        assert_eq!(full, Fill { units: 0, price: 0 });
        assert_eq!(filled(300.0, 300.0, 25_000), (300.0, 25_000));
        assert_eq!(filled(0.0, 0.0, 25_000), (0.0, 25_000));
    }

    #[test]
    fn above_capacity_the_tank_comes_down_and_the_excess_is_credited() {
        let over = fill(tank(320.0, 300.0), 0, false, RuleSource::Engine);
        assert_eq!(
            over,
            Fill {
                units: -20,
                price: -20
            }
        );
        assert_eq!(filled(320.0, 300.0, 0), (300.0, 20));
    }

    #[test]
    fn a_dominated_stellar_fills_the_whole_tank_free() {
        for debt in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(settled(0.0, 300.0, 0, true, debt), (300.0, 0));
            assert_eq!(settled(150.0, 300.0, -50, true, debt), (300.0, -50));
            let free = fill(tank(0.0, 300.0), 0, true, debt);
            assert_eq!(
                free,
                Fill {
                    units: 300,
                    price: 0
                }
            );
        }
    }

    #[test]
    fn by_the_engine_a_debt_is_cleared_with_fuel_by_the_other_reading_it_stands() {
        assert_eq!(
            settled(150.0, 300.0, -50, false, RuleSource::Engine),
            (100.0, 0)
        );
        assert_eq!(
            fill(tank(150.0, 300.0), -50, false, RuleSource::Engine),
            Fill {
                units: -50,
                price: -50
            }
        );
        assert_eq!(
            settled(150.0, 300.0, -50, false, RuleSource::Bible),
            (150.0, -50)
        );
        assert_eq!(
            settled(149.5, 300.0, -50, false, RuleSource::Bible),
            (149.0, -50),
            "nothing but the truncation"
        );
        assert_eq!(
            settled(30.0, 300.0, -50, false, RuleSource::Engine),
            (-20.0, 0),
            "below empty"
        );
    }

    #[test]
    fn settling_fills_the_tank_and_takes_the_price() {
        let mut pilot = Pilot::new(&catalog(), "Kane").expect("a pilot");
        pilot.reserves.fuel = tank(150.0, 300.0);
        pilot.cash = 1000;
        let before = pilot.clone();
        settle(
            &mut pilot,
            Fill {
                units: 150,
                price: 150,
            },
        );
        assert_eq!(pilot.reserves.fuel, Gauge::full(300.0));
        assert_eq!(pilot.cash, 850);
        pilot.reserves.fuel = before.reserves.fuel;
        pilot.cash = before.cash;
        assert_eq!(pilot, before, "nothing else changes");
    }
}
