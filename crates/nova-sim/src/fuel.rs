//! Fuel regeneration: how much fuel a ship gains (or loses) each tick, from
//! its `shïp`'s `FuelRegen` and its outfits' fuel scoops.
//!
//! Both are counted in frames (1/30 s ticks) per unit of fuel:
//!
//! - A `shïp`'s `FuelRegen` of n > 0 gains a unit every n ticks, so smaller
//!   is faster (the stock Vell-os Javelin, 2, outpaces the Dart, 8). 0, or
//!   anything below, gains nothing.
//! - An `oütf` of [`FUEL_SCOOP`] `ModType` with `ModVal` n > 0 gains a unit
//!   every n ticks for each one carried (the stock Fusion Reactor is 8,
//!   the Fission Reactor 16); n < 0 drains a unit every |n| ticks (the
//!   Capacitor Pulse Laser, -20); 0 does nothing.
//!
//! Every source adds up. Any other `ModType` is not fuel's business.

use crate::reserves::Gauge;

/// The `oütf` `ModType` that regenerates (or drains) fuel: the Bible's
/// "fuel scoop".
pub const FUEL_SCOOP: i16 = 18;

/// One kind of outfit the ship carries, raw from its `oütf`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutfitMod {
    /// Its `ModType`.
    pub mod_type: i16,
    /// Its `ModVal`.
    pub mod_val: i16,
    /// How many the ship carries.
    pub count: u16,
}

/// The fuel a ship gains each tick (negative when it drains) from its
/// `shïp`'s `ship_regen` and its `outfits`.
#[must_use]
pub fn fuel_regen_per_tick(ship_regen: i16, outfits: &[OutfitMod]) -> f32 {
    let ship = if ship_regen > 0 {
        1.0 / f32::from(ship_regen)
    } else {
        0.0
    };
    outfits
        .iter()
        .filter(|outfit| outfit.mod_type == FUEL_SCOOP && outfit.mod_val != 0)
        .fold(ship, |sum, outfit| {
            let every = f32::from(outfit.mod_val);
            let units = f32::from(outfit.count) / every.abs();
            sum + units.copysign(every)
        })
}

/// Adds `per_tick` to `fuel`, kept between empty and full.
pub fn regenerate(fuel: &mut Gauge, per_tick: f32) {
    fuel.now = (fuel.now + per_tick).min(fuel.max).max(0.0);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn scoop(mod_val: i16, count: u16) -> OutfitMod {
        OutfitMod {
            mod_type: FUEL_SCOOP,
            mod_val,
            count,
        }
    }

    #[test]
    fn a_ships_regen_is_one_unit_every_that_many_ticks() {
        assert_eq!(fuel_regen_per_tick(1, &[]), 1.0);
        assert_eq!(fuel_regen_per_tick(2, &[]), 0.5);
        assert_eq!(fuel_regen_per_tick(8, &[]), 0.125);
    }

    #[test]
    fn a_ship_regen_of_0_or_below_gains_nothing() {
        assert_eq!(fuel_regen_per_tick(0, &[]), 0.0);
        assert_eq!(fuel_regen_per_tick(-1, &[]), 0.0);
        assert_eq!(fuel_regen_per_tick(i16::MIN, &[]), 0.0);
    }

    #[test]
    fn scoops_and_the_ships_regen_add_up() {
        assert_eq!(fuel_regen_per_tick(10, &[scoop(8, 1)]), 0.1 + 0.125);
        assert_eq!(fuel_regen_per_tick(0, &[scoop(8, 1), scoop(16, 1)]), 0.1875);
    }

    #[test]
    fn a_negative_scoop_drains_and_each_one_carried_counts() {
        assert_eq!(fuel_regen_per_tick(0, &[scoop(-20, 1)]), -0.05);
        assert_eq!(fuel_regen_per_tick(0, &[scoop(-20, 2)]), -0.1);
        assert_eq!(fuel_regen_per_tick(0, &[scoop(8, 3)]), 0.375);
        assert_eq!(fuel_regen_per_tick(4, &[scoop(-20, 1)]), 0.25 - 0.05);
        assert_eq!(fuel_regen_per_tick(0, &[scoop(8, 0)]), 0.0);
    }

    #[test]
    fn other_mod_types_and_a_scoop_of_0_are_ignored() {
        let others = [
            OutfitMod {
                mod_type: 12,
                mod_val: 100,
                count: 1,
            },
            OutfitMod {
                mod_type: 17,
                mod_val: 4,
                count: 1,
            },
            OutfitMod {
                mod_type: 19,
                mod_val: 4,
                count: 1,
            },
            scoop(0, 3),
        ];
        assert_eq!(fuel_regen_per_tick(2, &others), 0.5);
        assert_eq!(FUEL_SCOOP, 18);
    }

    #[test]
    fn regenerating_adds_and_stops_at_full_and_at_empty() {
        let mut fuel = Gauge {
            now: 100.0,
            max: 300.0,
        };
        regenerate(&mut fuel, 0.5);
        assert_eq!(
            fuel,
            Gauge {
                now: 100.5,
                max: 300.0
            }
        );
        regenerate(&mut fuel, -0.25);
        assert_eq!(fuel.now, 100.25);
        let mut nearly_full = Gauge {
            now: 299.9,
            max: 300.0,
        };
        regenerate(&mut nearly_full, 1.0);
        assert_eq!(nearly_full, Gauge::full(300.0));
        let mut nearly_empty = Gauge {
            now: 0.03,
            max: 300.0,
        };
        regenerate(&mut nearly_empty, -0.05);
        assert_eq!(nearly_empty.now, 0.0);
        let mut none = Gauge::full(0.0);
        regenerate(&mut none, 1.0);
        assert_eq!(none, Gauge::full(0.0));
    }
}
