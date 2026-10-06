//! A ship's computed stats: everything about how it performs, from its
//! `shïp`'s fields and the outfits it carries, in one place.
//!
//! Every reader of a ship's performance (flight, the exchange, the
//! outfitter and, later, combat) goes through [`ShipStats::new`]; nothing
//! else reads the `shïp`'s performance fields. Each outfit is an
//! [`OutfitMod`], its `ModType` and `ModVal` with how many are carried,
//! and each total is `ModVal` x count summed over every outfit of its
//! `ModType` (the Bible's `oütf`):
//!
//! - [`MORE_SPEED`] (8) and [`MORE_ACCEL`] (7) add to `Speed` and `Accel`,
//!   "see shïp": the same scale.
//! - [`TURN_CHANGE`] (9) adds to `Maneuver`, a tenth as much
//!   ([`TURN_MOD_PER_MANEUVER`]): the Bible gives the outfit's 100 as
//!   30°/s and the ship's 10 as about 30°/s.
//! - [`MORE_SHIELD`] (4), [`MORE_ARMOR`] (6) and [`MORE_FUEL`] (12) add
//!   to the shield, armour and fuel capacity, after the [`crate::reserves`]
//!   rules read the `shïp`'s own.
//! - [`MORE_CARGO`](crate::market::MORE_CARGO) (2) adds cargo space, as
//!   [`cargo_capacity`] says.
//! - [`FUEL_SCOOP`](crate::fuel::FUEL_SCOOP) (18) regenerates fuel, as
//!   [`fuel_regen_per_tick`] says.
//! - [`HYPERSPACE_DAYS`] (22) adds days to each jump's
//!   [`DAYS_PER_JUMP`], which never goes below one (the Bible: "still
//!   can't go below 1 day/jump").
//! - [`HYPERSPACE_DISTANCE`] (23) moves the edge of the no-jump zone from
//!   its standard [`MIN_JUMP_DISTANCE`] (the Bible: "the standard radius
//!   is 1000").
//!
//! Each total is summed wide, and none goes below none, so no mix of
//! outfits can overflow or turn a figure negative. Multi-jump (32), fast
//! jumping (37), the inertial dampener (38) and every other `ModType`
//! change nothing here yet.

use crate::fuel::{OutfitMod, fuel_regen_per_tick};
use crate::handling::{Handling, ShipFields};
use crate::hyperspace::{DAYS_PER_JUMP, MIN_JUMP_DISTANCE};
use crate::market::cargo_capacity;
use crate::reserves::{Reserves, shield_points};

/// The `oütf` `ModType` that adds shield points.
pub const MORE_SHIELD: i16 = 4;
/// The `oütf` `ModType` that adds armour points.
pub const MORE_ARMOR: i16 = 6;
/// The `oütf` `ModType` that adds acceleration, on `Accel`'s scale.
pub const MORE_ACCEL: i16 = 7;
/// The `oütf` `ModType` that adds top speed, on `Speed`'s scale.
pub const MORE_SPEED: i16 = 8;
/// The `oütf` `ModType` that changes the turn rate.
pub const TURN_CHANGE: i16 = 9;
/// The `oütf` `ModType` that adds fuel capacity; 100 is one jump.
pub const MORE_FUEL: i16 = 12;
/// The `oütf` `ModType` that adds days to each jump.
pub const HYPERSPACE_DAYS: i16 = 22;
/// The `oütf` `ModType` that moves the no-jump zone's edge, in pixels.
pub const HYPERSPACE_DISTANCE: i16 = 23;

/// A [`TURN_CHANGE`] `ModVal` per unit of `Maneuver`: the Bible's "100 =
/// 30°/sec" for the outfit and "10 is about 30°/s" for the ship.
pub const TURN_MOD_PER_MANEUVER: f32 = 10.0;

/// How a ship performs, its outfits included.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShipStats {
    /// How it flies.
    pub handling: Handling,
    /// The shield points it can hold.
    pub shield: f32,
    /// The armour points it can hold.
    pub armor: f32,
    /// The fuel it can hold; 100 is one jump.
    pub fuel: f32,
    /// The fuel it gains each tick in flight (negative when it drains).
    pub fuel_regen: f32,
    /// Its cargo space, in tons.
    pub capacity: u32,
    /// How far from a system's centre, in pixels, it must be to jump.
    pub jump_distance: f32,
    /// How many days each of its jumps takes; at least one.
    pub jump_days: u32,
}

impl ShipStats {
    /// The stats of a ship with `fields`, carrying `outfits`.
    #[must_use]
    pub fn new(fields: ShipFields, outfits: &[OutfitMod]) -> Self {
        let total = |mod_type: i16| -> i64 {
            outfits
                .iter()
                .filter(|outfit| outfit.mod_type == mod_type)
                .map(|outfit| i64::from(outfit.mod_val) * i64::from(outfit.count))
                .sum()
        };
        let maneuver =
            f32::from(fields.maneuver) + total(TURN_CHANGE) as f32 / TURN_MOD_PER_MANEUVER;
        let positive = |total: i64| total.max(0) as f32;
        Self {
            handling: Handling::from_totals(
                i64::from(fields.speed) + total(MORE_SPEED),
                i64::from(fields.accel) + total(MORE_ACCEL),
                maneuver,
            ),
            shield: positive(shield_points(fields.shield) + total(MORE_SHIELD)),
            armor: positive(i64::from(fields.armor.max(0)) + total(MORE_ARMOR)),
            fuel: positive(i64::from(fields.fuel.max(0)) + total(MORE_FUEL)),
            fuel_regen: fuel_regen_per_tick(fields.fuel_regen, outfits),
            capacity: cargo_capacity(fields.holds, outfits),
            jump_distance: positive(MIN_JUMP_DISTANCE as i64 + total(HYPERSPACE_DISTANCE)),
            jump_days: u32::try_from((i64::from(DAYS_PER_JUMP) + total(HYPERSPACE_DAYS)).max(1))
                .unwrap_or(u32::MAX),
        }
    }

    /// A ship's reserves with these stats, full.
    #[must_use]
    pub fn full(&self) -> Reserves {
        Reserves::full(self.shield, self.armor, self.fuel)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::fuel::FUEL_SCOOP;
    use crate::hyperspace::{DAYS_PER_JUMP, MIN_JUMP_DISTANCE};
    use crate::market::MORE_CARGO;
    use crate::reserves::Gauge;

    /// An average ship: speed 300, accel 300, maneuver 10, shield 30,
    /// armour 45, fuel 300, regenerating a unit every 8 ticks, holding 10
    /// tons.
    const AVERAGE: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 10,
        shield: 30,
        armor: 45,
        fuel: 300,
        fuel_regen: 8,
        holds: 10,
        mass: 15,
        free_mass: 8,
        contribute: 1,
        flags2: 0,
    };

    fn outfit(mod_type: i16, mod_val: i16, count: u16) -> OutfitMod {
        OutfitMod {
            mod_type,
            mod_val,
            count,
        }
    }

    #[test]
    fn the_ships_own_fields_alone_give_its_handling_reserves_regeneration_and_space() {
        let stats = ShipStats::new(AVERAGE, &[]);
        assert_eq!(
            stats.handling,
            Handling {
                max_speed: 3.0,
                accel: 0.1,
                turn_rate: 1.0,
            }
        );
        assert_eq!((stats.shield, stats.armor, stats.fuel), (30.0, 45.0, 300.0));
        assert_eq!(stats.fuel_regen, 0.125);
        assert_eq!(stats.capacity, 10);
        assert_eq!(
            stats.full(),
            Reserves {
                shield: Gauge::full(30.0),
                armor: Gauge::full(45.0),
                fuel: Gauge::full(300.0),
            }
        );
    }

    #[test]
    fn the_ships_reserve_rules_still_apply() {
        let odd = ShipFields {
            shield: -20,
            armor: -5,
            fuel: -100,
            ..AVERAGE
        };
        let stats = ShipStats::new(odd, &[]);
        assert_eq!((stats.shield, stats.armor, stats.fuel), (100.0, 0.0, 0.0));
        let plus = ShipStats::new(odd, &[outfit(MORE_ARMOR, 10, 1), outfit(MORE_FUEL, 50, 1)]);
        assert_eq!((plus.armor, plus.fuel), (10.0, 50.0), "added to none");
    }

    #[test]
    fn speed_and_acceleration_add_their_mod_val_for_each_one_carried() {
        let stats = ShipStats::new(
            AVERAGE,
            &[outfit(MORE_SPEED, 100, 2), outfit(MORE_ACCEL, 150, 2)],
        );
        assert_eq!(stats.handling.max_speed, 5.0);
        assert_eq!(stats.handling.accel, 0.2);
        assert_eq!(stats.handling.turn_rate, 1.0);
    }

    #[test]
    fn a_turn_change_is_a_tenth_of_maneuver_and_keeps_its_fraction() {
        // Vectored Thrust's 125 is 12.5 more Maneuver.
        let stats = ShipStats::new(AVERAGE, &[outfit(TURN_CHANGE, 125, 1)]);
        assert_eq!(stats.handling.turn_rate, 2.25);
        let two = ShipStats::new(AVERAGE, &[outfit(TURN_CHANGE, 5, 2)]);
        assert_eq!(two.handling.turn_rate, 1.1);
        assert_eq!(TURN_MOD_PER_MANEUVER, 10.0);
    }

    #[test]
    fn shield_armour_fuel_and_cargo_add_their_mod_val_for_each_one_carried() {
        let stats = ShipStats::new(
            AVERAGE,
            &[
                outfit(MORE_SHIELD, 50, 3),
                outfit(MORE_ARMOR, 20, 2),
                outfit(MORE_FUEL, 100, 1),
                outfit(MORE_CARGO, 5, 2),
            ],
        );
        assert_eq!(
            (stats.shield, stats.armor, stats.fuel),
            (180.0, 85.0, 400.0)
        );
        assert_eq!(stats.capacity, 20);
        assert_eq!(stats.full().fuel, Gauge::full(400.0));
    }

    #[test]
    fn a_fuel_scoop_adds_to_the_regeneration_as_fuel_says() {
        let scoops = [outfit(FUEL_SCOOP, 4, 2)];
        let stats = ShipStats::new(AVERAGE, &scoops);
        assert_eq!(stats.fuel_regen, fuel_regen_per_tick(8, &scoops));
        assert_eq!(stats.fuel_regen, 0.125 + 0.5);
    }

    #[test]
    fn the_standard_jump_distance_is_the_no_jump_zones_radius() {
        let stats = ShipStats::new(AVERAGE, &[]);
        assert_eq!(stats.jump_distance, MIN_JUMP_DISTANCE);
        assert_eq!(stats.jump_distance, 1000.0);
    }

    #[test]
    fn a_hyperspace_distance_mod_moves_the_no_jump_zones_edge() {
        // The stock Horizontal Booster: -500, halving the zone.
        let booster = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DISTANCE, -500, 1)]);
        assert_eq!(booster.jump_distance, 500.0);
        let two = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DISTANCE, -200, 2)]);
        assert_eq!(two.jump_distance, 600.0, "ModVal x count");
        let wider = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DISTANCE, 250, 1)]);
        assert_eq!(wider.jump_distance, 1250.0);
        for (mod_val, count) in [(-1000, 1), (-600, 2), (i16::MIN, u16::MAX)] {
            let none = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DISTANCE, mod_val, count)]);
            assert_eq!(none.jump_distance, 0.0, "{mod_val} x {count}");
        }
    }

    #[test]
    fn a_jump_takes_a_day_unless_a_hyperspace_speed_mod_changes_it() {
        assert_eq!(ShipStats::new(AVERAGE, &[]).jump_days, DAYS_PER_JUMP);
        assert_eq!(ShipStats::new(AVERAGE, &[]).jump_days, 1);
        let slower = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DAYS, 1, 2)]);
        assert_eq!(slower.jump_days, 3, "ModVal x count");
        let mixed = ShipStats::new(
            AVERAGE,
            &[
                outfit(HYPERSPACE_DAYS, 3, 1),
                outfit(HYPERSPACE_DAYS, -1, 1),
            ],
        );
        assert_eq!(mixed.jump_days, 3);
    }

    #[test]
    fn a_jump_takes_at_least_a_day() {
        // The stock Sutherland Alluvial Dampener: -1, on a one-day base.
        let dampener = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DAYS, -1, 1)]);
        assert_eq!(dampener.jump_days, 1);
        let far = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DAYS, -30, 4)]);
        assert_eq!(far.jump_days, 1);
        let least = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DAYS, i16::MIN, u16::MAX)]);
        assert_eq!(least.jump_days, 1);
    }

    #[test]
    fn the_most_days_a_jump_takes_do_not_overflow() {
        let many: Vec<_> =
            std::iter::repeat_n(outfit(HYPERSPACE_DAYS, i16::MAX, u16::MAX), 600).collect();
        let stats = ShipStats::new(AVERAGE, &many);
        assert_eq!(stats.jump_days, u32::MAX, "saturating");
        let one = ShipStats::new(AVERAGE, &[outfit(HYPERSPACE_DAYS, i16::MAX, u16::MAX)]);
        let most = 1 + i64::from(i16::MAX) * i64::from(u16::MAX);
        assert_eq!(i64::from(one.jump_days), most);
    }

    #[test]
    fn every_other_mod_type_changes_nothing() {
        let plain = ShipStats::new(AVERAGE, &[]);
        for mod_type in [-1, 0, 1, 3, 5, 10, 11, 13, 15, 17, 32, 37, 38, 45, 99] {
            assert_eq!(
                ShipStats::new(AVERAGE, &[outfit(mod_type, 500, 3)]),
                plain,
                "{mod_type}"
            );
        }
        assert_eq!(
            [
                MORE_SHIELD,
                MORE_ARMOR,
                MORE_ACCEL,
                MORE_SPEED,
                TURN_CHANGE,
                MORE_FUEL,
                HYPERSPACE_DAYS,
                HYPERSPACE_DISTANCE
            ],
            [4, 6, 7, 8, 9, 12, 22, 23]
        );
    }

    #[test]
    fn negative_mods_take_away_and_stop_at_none() {
        let stats = ShipStats::new(
            AVERAGE,
            &[
                outfit(MORE_SPEED, -100, 1),
                outfit(TURN_CHANGE, -2, 1),
                outfit(MORE_SHIELD, -10, 1),
            ],
        );
        assert_eq!(stats.handling.max_speed, 2.0);
        assert_eq!(stats.handling.turn_rate, 0.98);
        assert_eq!(stats.shield, 20.0);
        let none = ShipStats::new(
            AVERAGE,
            &[
                outfit(MORE_SPEED, -400, 1),
                outfit(MORE_ACCEL, -400, 1),
                outfit(TURN_CHANGE, -101, 1),
                outfit(MORE_SHIELD, -31, 1),
                outfit(MORE_ARMOR, -46, 1),
                outfit(MORE_FUEL, -301, 1),
                outfit(MORE_CARGO, -11, 1),
            ],
        );
        assert_eq!(none.handling, Handling::default());
        assert_eq!((none.shield, none.armor, none.fuel), (0.0, 0.0, 0.0));
        assert_eq!(none.capacity, 0);
    }

    #[test]
    fn extreme_values_do_not_overflow() {
        let most = ShipFields {
            speed: i16::MAX,
            accel: i16::MAX,
            maneuver: i16::MAX,
            shield: i16::MIN,
            armor: i16::MAX,
            fuel: i16::MAX,
            ..AVERAGE
        };
        let mods: Vec<_> = [
            MORE_SPEED,
            MORE_ACCEL,
            TURN_CHANGE,
            MORE_SHIELD,
            MORE_ARMOR,
            MORE_FUEL,
            HYPERSPACE_DAYS,
            HYPERSPACE_DISTANCE,
        ]
        .into_iter()
        .map(|mod_type| outfit(mod_type, i16::MAX, u16::MAX))
        .collect();
        let many: Vec<_> = mods.iter().cycle().take(800).copied().collect();
        let stats = ShipStats::new(most, &many);
        let per = 100.0 * f32::from(i16::MAX) * f32::from(u16::MAX);
        assert!(stats.shield > 163_840.0 + per - 1e6, "{}", stats.shield);
        assert!(stats.handling.max_speed > 0.0);
        assert_eq!(stats.jump_days, u32::MAX);
        let each = f32::from(i16::MAX) * f32::from(u16::MAX);
        assert!(stats.jump_distance > 99.0 * each, "{}", stats.jump_distance);
        let least: Vec<_> = many
            .iter()
            .map(|mod_| OutfitMod {
                mod_val: i16::MIN,
                ..*mod_
            })
            .collect();
        let stats = ShipStats::new(most, &least);
        assert_eq!(stats.handling, Handling::default());
        assert_eq!((stats.shield, stats.armor, stats.fuel), (0.0, 0.0, 0.0));
        assert_eq!((stats.jump_distance, stats.jump_days), (0.0, 1));
    }
}
