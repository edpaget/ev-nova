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
//!   30°/s and the ship's 10 as about 30°/s. The turn rate is then
//!   floored as `_ShipTurnRate` floors it ([`floored_turn`]): the
//!   player's ship's ([`ShipStats::for_player`]) at 1° a tick unless its
//!   class turns none, any other's only when its class turns 1° or more.
//! - [`MORE_SHIELD`] (4), [`MORE_ARMOR`] (6) and [`MORE_FUEL`] (12) add
//!   to the shield, armour and fuel capacity, after the [`crate::reserves`]
//!   rules read the `shïp`'s own.
//! - [`MORE_CARGO`](crate::market::MORE_CARGO) (2) adds cargo space, as
//!   [`cargo_capacity`] says: the ship's own space, which the fleet's
//!   ([`fleet_holds`](crate::market::fleet_holds)) builds on.
//! - [`FUEL_SCOOP`](crate::fuel::FUEL_SCOOP) (18) regenerates fuel, as
//!   [`fuel_regen_per_tick`] says. The `shïp`'s own `FuelRegen` counts
//!   for the player's ship only when its `Flags` hold [`FUEL_REGEN_HULL`]
//!   ([`ShipStats::for_player`]).
//! - [`HYPERSPACE_DAYS`] (22) adds days to each jump's
//!   [`base_jump_days`], from the hull's `Mass`, and the total never goes
//!   below one (the Bible: "still can't go below 1 day/jump").
//! - [`MORE_SHIELD_RECHARGE`] (5) and [`MORE_ARMOR_RECHARGE`] (29) add to
//!   the `shïp`'s `ShieldRech` and `ArmorRech`, on the same scale: each
//!   [`RECHARGE_SCALE`] is a point a tick, the `shïp`'s
//!   own rate counting as none below none (`_ShipShieldRechargeRate` and
//!   `_ShipArmorRechargeRate` in the `EV Nova` executable).
//! - [`HYPERSPACE_DISTANCE`] (23) moves the edge of the no-jump zone from
//!   its standard [`MIN_JUMP_DISTANCE`] (the Bible: "the standard radius
//!   is 1000").
//! - [`FAST_JUMP`] (37), carried at all, whatever its `ModVal`, or a
//!   `shïp` whose `Flags2` has [`FAST_JUMP_HULL`] (0x0020) set, lets the
//!   ship jump without slowing down first.
//! - [`MULTI_JUMP`] (32) is how many systems a jump passes along the
//!   course. It is the one total that is not `ModVal` x count: each
//!   outfit carried adds its `ModVal` once, whatever its count, as the
//!   engine's `_ShipJumpsPerJump` does.
//! - [`INERTIAL_DAMPENER`] (38), carried at all, whatever its `ModVal`
//!   (the Bible's "inertial dampener ... makes ship inertialess"), is
//!   recorded as [`ShipStats::dampener`], and a `shïp` whose `Flags2` has
//!   [`INERTIALESS_HULL`] (0x0040, class record +0x9ea) set as
//!   [`ShipStats::inertialess_hull`]. The stats only record them:
//!   whether the ship flies inertialess is
//!   [`is_inertialess`](crate::inertialess::is_inertialess)'s to decide,
//!   as `_ShipIsInertialess` (@0x79c2) counts the outfit for the player
//!   alone.
//!
//! - [`AFTERBURNER`] (15) is recorded as [`ShipStats::afterburner`], the
//!   fuel it uses a tick: as `_ShipAfterburnerFuelUsage` (@0x7a82-0x7b0f)
//!   reads it, the last carried outfit in index order that has a slot of
//!   that `ModType` gives its first such slot's `ModVal` x
//!   [`FUEL_PER_MOD_VAL`], whatever the count. The original caches it for
//!   the player (reset in `_ResetPlayerPrecalcedValues` @0xc419), as it
//!   caches `_HasAfterburner`'s scan (@0xc3ac); the stats are worked out
//!   afresh whenever the outfits change, so they need no cache.
//!
//! Each total is summed wide, and none goes below none, so no mix of
//! outfits can overflow or turn a figure negative. Every other `ModType`
//! changes nothing here yet.

use nova_sim_base::afterburner::FUEL_PER_MOD_VAL;

use crate::flight::Flyer;
use crate::fuel::{OutfitMod, fuel_regen_per_tick};
use crate::handling::{Handling, ShipFields, floored_turn};
use crate::market::cargo_capacity;
use crate::reserves::{Reserves, shield_points};

/// `shïp` `Flags`: the player's hull regenerates fuel by its `FuelRegen`
/// (`_ShipFuelGenRate` @0x3043). An NPC's does without it.
pub const FUEL_REGEN_HULL: u16 = 0x0008;

/// How far from the system's centre, in pixels, a ship must be to jump
/// unless its outfits say otherwise (the Bible: "Jump Distance 1000
/// pixels"): the no-jump zone's standard radius.
pub const MIN_JUMP_DISTANCE: f32 = 1000.0;

/// How many days pass in a jump before any outfit changes it, from the
/// hull's `shïp` `Mass` alone: the Nova Bible's bands, 1-99 tons take one
/// day, 100-199 take two and 200 or more take three. The original's
/// `_ShipHyperTransitTime` (@0x43d0) reads the same bands from the
/// ship type's `Mass` and no outfit mass, and gives a `Mass` of 0 or less
/// one day too.
#[must_use]
pub fn base_jump_days(mass: i16) -> u32 {
    match mass {
        ..=99 => 1,
        100..=199 => 2,
        _ => 3,
    }
}

/// The `oütf` `ModType` that adds shield points.
pub const MORE_SHIELD: i16 = 4;
/// The `oütf` `ModType` that speeds up shield regeneration.
pub const MORE_SHIELD_RECHARGE: i16 = 5;
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
/// The `oütf` `ModType` that speeds up armour regeneration.
pub const MORE_ARMOR_RECHARGE: i16 = 29;

/// How many units of `ShieldRech` or `ArmorRech` regenerate a point a
/// tick: the record's "x1000 per frame".
pub const RECHARGE_SCALE: f32 = 1000.0;
/// The `oütf` `ModType` whose `ModVal` is how many systems a jump passes
/// along the course.
pub const MULTI_JUMP: i16 = 32;
/// The `oütf` `ModType` that lets the ship jump without slowing down.
pub const FAST_JUMP: i16 = 37;
/// The `shïp` `Flags2` bit for a hull that jumps without slowing down.
pub const FAST_JUMP_HULL: u16 = 0x0020;
/// The `oütf` `ModType` of the inertial dampener: carried at all, it may
/// make the ship inertialess (`_ShipIsInertialess` @0x7a4f, `cmpw $0x26`
/// over each of an outfit's four `ModType`s).
pub const INERTIAL_DAMPENER: i16 = 38;
/// The `oütf` `ModType` of an afterburner, whose `ModVal` is the fuel it
/// uses a second (`cmpw $0xf` over each of an outfit's four `ModType`s,
/// `_HasAfterburner` @0x3acc and `_ShipAfterburnerFuelUsage` @0x7ad0).
pub const AFTERBURNER: i16 = 15;
/// How many mods each outfit carries: an `oütf`'s four `ModType`s and
/// `ModVal`s, which `nova_sim_catalog::catalog::outfit_mods` lists in
/// turn for each outfit, in its records' order.
pub const MOD_SLOTS: usize = 4;
/// The `shïp` `Flags2` bit for an inertialess hull (`testb $0x40` on the
/// class record's `Flags2`, +0x9ea, @0x7a06).
pub const INERTIALESS_HULL: u16 = 0x0040;

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
    /// The shield points it regenerates each tick; none below none.
    pub shield_regen: f32,
    /// The armour points it regenerates each tick; none below none.
    pub armor_regen: f32,
    /// Its cargo space, in tons.
    pub capacity: u32,
    /// How far from a system's centre, in pixels, it must be to jump.
    pub jump_distance: f32,
    /// How many days each of its jumps takes; at least one.
    pub jump_days: u32,
    /// Whether it jumps without slowing down first.
    pub fast_jump: bool,
    /// How many systems a jump passes along the course, as the outfits
    /// give it (see `nova_sim::hyperspace::hops_per_jump`).
    pub multi_jump: u32,
    /// Whether its hull's `Flags2` holds [`INERTIALESS_HULL`].
    pub inertialess_hull: bool,
    /// Whether it carries an [`INERTIAL_DAMPENER`], whatever its `ModVal`.
    pub dampener: bool,
    /// The fuel its afterburner uses a tick, if it carries an
    /// [`AFTERBURNER`] (see the module docs).
    pub afterburner: Option<f32>,
}

/// The fuel a tick of the afterburner among `outfits` uses (see the module
/// docs): `outfits` holds each outfit's [`MOD_SLOTS`] mods in turn, and
/// the last carried one with an [`AFTERBURNER`] slot gives its first.
fn afterburner_usage(outfits: &[OutfitMod]) -> Option<f32> {
    outfits
        .chunks(MOD_SLOTS)
        .rev()
        .find_map(|slots| {
            slots
                .iter()
                .find(|slot| slot.mod_type == AFTERBURNER && slot.count > 0)
        })
        .map(|slot| (f64::from(slot.mod_val) * FUEL_PER_MOD_VAL) as f32)
}

/// `ModVal` x count summed over every one of `outfits` of `ModType`
/// `mod_type`, wide.
fn mod_total(outfits: &[OutfitMod], mod_type: i16) -> i64 {
    outfits
        .iter()
        .filter(|outfit| outfit.mod_type == mod_type)
        .map(|outfit| i64::from(outfit.mod_val) * i64::from(outfit.count))
        .sum()
}

/// A ship's `Speed`, `Accel` and `Maneuver`, its outfits' included, not
/// yet floored: what its [`Handling`] is worked out from, and what the
/// player info's stat rows show
/// (`nova_sim::player_info`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HandlingTotals {
    /// `Speed` plus the [`MORE_SPEED`] outfits.
    pub speed: i64,
    /// `Accel` plus the [`MORE_ACCEL`] outfits.
    pub accel: i64,
    /// `Maneuver` plus a tenth of the [`TURN_CHANGE`] outfits
    /// ([`TURN_MOD_PER_MANEUVER`]).
    pub maneuver: f32,
    /// `Maneuver` alone, which the turn's floor reads
    /// ([`floored_turn`]).
    pub class_maneuver: i16,
}

impl HandlingTotals {
    /// The totals of a ship with `fields`, carrying `outfits`.
    #[must_use]
    pub fn new(fields: ShipFields, outfits: &[OutfitMod]) -> Self {
        Self {
            speed: i64::from(fields.speed) + mod_total(outfits, MORE_SPEED),
            accel: i64::from(fields.accel) + mod_total(outfits, MORE_ACCEL),
            maneuver: f32::from(fields.maneuver)
                + mod_total(outfits, TURN_CHANGE) as f32 / TURN_MOD_PER_MANEUVER,
            class_maneuver: fields.maneuver,
        }
    }
}

impl ShipStats {
    /// The stats of a ship with `fields`, carrying `outfits`, an NPC's:
    /// its turn floored as `_ShipTurnRate` floors an NPC's
    /// ([`floored_turn`]).
    #[must_use]
    pub fn new(fields: ShipFields, outfits: &[OutfitMod]) -> Self {
        Self::flown_by(fields, outfits, Flyer::Npc)
    }

    /// The stats of a ship with `fields`, carrying `outfits`, flown by
    /// `flyer`, whose floor its turn takes.
    fn flown_by(fields: ShipFields, outfits: &[OutfitMod], flyer: Flyer) -> Self {
        let total = |mod_type: i16| mod_total(outfits, mod_type);
        let totals = HandlingTotals::new(fields, outfits);
        let positive = |total: i64| total.max(0) as f32;
        let handling = Handling::from_totals(totals.speed, totals.accel, totals.maneuver);
        Self {
            handling: Handling {
                turn_rate: floored_turn(fields.class_turn(), handling.turn_rate, flyer),
                ..handling
            },
            shield: positive(shield_points(fields.shield) + total(MORE_SHIELD)),
            armor: positive(i64::from(fields.armor.max(0)) + total(MORE_ARMOR)),
            fuel: positive(i64::from(fields.fuel.max(0)) + total(MORE_FUEL)),
            fuel_regen: fuel_regen_per_tick(fields.fuel_regen, outfits),
            shield_regen: positive(
                i64::from(fields.shield_rech.max(0)) + total(MORE_SHIELD_RECHARGE),
            ) / RECHARGE_SCALE,
            armor_regen: positive(i64::from(fields.armor_rech.max(0)) + total(MORE_ARMOR_RECHARGE))
                / RECHARGE_SCALE,
            capacity: cargo_capacity(fields.holds, outfits),
            jump_distance: positive(MIN_JUMP_DISTANCE as i64 + total(HYPERSPACE_DISTANCE)),
            jump_days: u32::try_from(
                (i64::from(base_jump_days(fields.mass)) + total(HYPERSPACE_DAYS)).max(1),
            )
            .unwrap_or(u32::MAX),
            fast_jump: fields.flags2 & FAST_JUMP_HULL != 0
                || outfits
                    .iter()
                    .any(|outfit| outfit.mod_type == FAST_JUMP && outfit.count > 0),
            multi_jump: u32::try_from(
                outfits
                    .iter()
                    .filter(|outfit| outfit.mod_type == MULTI_JUMP && outfit.count > 0)
                    .map(|outfit| i64::from(outfit.mod_val))
                    .sum::<i64>()
                    .max(0),
            )
            .unwrap_or(u32::MAX),
            inertialess_hull: fields.flags2 & INERTIALESS_HULL != 0,
            dampener: outfits
                .iter()
                .any(|outfit| outfit.mod_type == INERTIAL_DAMPENER && outfit.count > 0),
            afterburner: afterburner_usage(outfits),
        }
    }

    /// The stats of the player's ship with `fields`, carrying `outfits`:
    /// as [`ShipStats::new`], but its hull's `FuelRegen` counts only when
    /// its `Flags` hold [`FUEL_REGEN_HULL`], while its fuel scoops always
    /// count (`_ShipFuelGenRate` @0x3043-0x3057, which counts it for any
    /// other ship whatever its flags), and its turn takes the player's
    /// floor ([`floored_turn`]).
    #[must_use]
    pub fn for_player(fields: ShipFields, outfits: &[OutfitMod]) -> Self {
        let fuel_regen = if fields.flags & FUEL_REGEN_HULL == 0 {
            0
        } else {
            fields.fuel_regen
        };
        Self::flown_by(
            ShipFields {
                fuel_regen,
                ..fields
            },
            outfits,
            Flyer::Player,
        )
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
        shield_rech: 10,
        armor_rech: 0,
        flags: 0,
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
                accel: 0.06,
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
        assert_eq!(stats.handling.accel, 0.12);
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
    fn the_players_hull_regenerates_fuel_only_with_flags_0x0008() {
        let scoops = [outfit(FUEL_SCOOP, 4, 2)];
        let without = ShipStats::for_player(AVERAGE, &[]);
        assert_eq!(without.fuel_regen, 0.0, "FuelRegen 8 but no 0x0008");
        let scooping = ShipStats::for_player(AVERAGE, &scoops);
        assert_eq!(scooping.fuel_regen, 0.5, "its scoops still count");
        let flagged = ShipFields {
            flags: FUEL_REGEN_HULL | 0x0010,
            ..AVERAGE
        };
        assert_eq!(ShipStats::for_player(flagged, &[]).fuel_regen, 0.125);
        assert_eq!(
            ShipStats::for_player(flagged, &scoops).fuel_regen,
            0.125 + 0.5
        );
        assert_eq!(FUEL_REGEN_HULL, 0x0008);
        assert_eq!(
            ShipStats::for_player(AVERAGE, &scoops),
            ShipStats {
                fuel_regen: 0.5,
                ..ShipStats::new(AVERAGE, &scoops)
            },
            "nothing else changes"
        );
    }

    #[test]
    fn any_other_ships_hull_regenerates_fuel_without_flags_0x0008() {
        assert_eq!(AVERAGE.flags, 0);
        assert_eq!(ShipStats::new(AVERAGE, &[]).fuel_regen, 0.125);
    }

    #[test]
    fn a_fuel_scoop_adds_to_the_regeneration_as_fuel_says() {
        let scoops = [outfit(FUEL_SCOOP, 4, 2)];
        let stats = ShipStats::new(AVERAGE, &scoops);
        assert_eq!(stats.fuel_regen, fuel_regen_per_tick(8, &scoops));
        assert_eq!(stats.fuel_regen, 0.125 + 0.5);
    }

    #[test]
    fn shields_and_armour_recharge_a_thousandth_of_their_rate_a_tick() {
        let stats = ShipStats::new(AVERAGE, &[]);
        assert_eq!((stats.shield_regen, stats.armor_regen), (0.01, 0.0));
        let both = ShipStats::new(
            ShipFields {
                shield_rech: 125,
                armor_rech: 20,
                ..AVERAGE
            },
            &[],
        );
        assert_eq!((both.shield_regen, both.armor_regen), (0.125, 0.02));
        assert_eq!(RECHARGE_SCALE, 1000.0);
    }

    #[test]
    fn a_rate_of_none_or_less_recharges_nothing() {
        let none = ShipStats::new(
            ShipFields {
                shield_rech: -10,
                armor_rech: -1,
                ..AVERAGE
            },
            &[],
        );
        assert_eq!((none.shield_regen, none.armor_regen), (0.0, 0.0));
    }

    #[test]
    fn recharge_outfits_add_their_mod_val_for_each_one_carried() {
        let stats = ShipStats::new(
            AVERAGE,
            &[
                outfit(MORE_SHIELD_RECHARGE, 15, 2),
                outfit(MORE_ARMOR_RECHARGE, 5, 3),
            ],
        );
        assert_eq!((stats.shield_regen, stats.armor_regen), (0.04, 0.015));
        let on_none = ShipStats::new(
            ShipFields {
                shield_rech: -50,
                ..AVERAGE
            },
            &[outfit(MORE_SHIELD_RECHARGE, 20, 1)],
        );
        assert_eq!(on_none.shield_regen, 0.02, "added to none");
        assert_eq!((MORE_SHIELD_RECHARGE, MORE_ARMOR_RECHARGE), (5, 29));
    }

    #[test]
    fn a_negative_recharge_total_is_none() {
        let stats = ShipStats::new(
            ShipFields {
                armor_rech: 10,
                ..AVERAGE
            },
            &[
                outfit(MORE_SHIELD_RECHARGE, -11, 1),
                outfit(MORE_ARMOR_RECHARGE, -4, 3),
            ],
        );
        assert_eq!((stats.shield_regen, stats.armor_regen), (0.0, 0.0));
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
    fn a_light_a_medium_and_a_heavy_hull_jump_in_1_2_and_3_days() {
        for (mass, days) in [(15, 1), (150, 2), (250, 3)] {
            let hull = ShipFields { mass, ..AVERAGE };
            assert_eq!(ShipStats::new(hull, &[]).jump_days, days, "mass {mass}");
        }
    }

    #[test]
    fn a_heavy_hull_with_the_dampener_jumps_in_fewer_days_but_at_least_one() {
        let heavy = ShipFields {
            mass: 250,
            ..AVERAGE
        };
        // The stock Sutherland Alluvial Dampener: -1.
        let dampener = ShipStats::new(heavy, &[outfit(HYPERSPACE_DAYS, -1, 1)]);
        assert_eq!(dampener.jump_days, 2);
        let two = ShipStats::new(heavy, &[outfit(HYPERSPACE_DAYS, -1, 2)]);
        assert_eq!(two.jump_days, 1);
        for (mod_val, count) in [(-1, 5), (i16::MIN, u16::MAX)] {
            let least = ShipStats::new(heavy, &[outfit(HYPERSPACE_DAYS, mod_val, count)]);
            assert_eq!(least.jump_days, 1, "{mod_val} x {count}");
        }
        let medium = ShipFields {
            mass: 150,
            ..AVERAGE
        };
        let medium_dampened = ShipStats::new(medium, &[outfit(HYPERSPACE_DAYS, -1, 1)]);
        assert_eq!(medium_dampened.jump_days, 1);
        let slower = ShipStats::new(heavy, &[outfit(HYPERSPACE_DAYS, 1, 2)]);
        assert_eq!(slower.jump_days, 5, "ModVal x count on top of three");
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
    fn a_plain_ship_does_not_fast_jump() {
        assert!(!ShipStats::new(AVERAGE, &[]).fast_jump);
    }

    #[test]
    fn a_fast_jump_outfit_makes_the_ship_fast_jump() {
        assert_eq!(FAST_JUMP, 37);
        // ModVal is ignored.
        assert!(ShipStats::new(AVERAGE, &[outfit(FAST_JUMP, 0, 1)]).fast_jump);
        assert!(
            !ShipStats::new(AVERAGE, &[outfit(FAST_JUMP, 1, 0)]).fast_jump,
            "none carried"
        );
    }

    #[test]
    fn a_hull_with_flags2_0x0020_fast_jumps() {
        assert_eq!(FAST_JUMP_HULL, 0x0020);
        let hull = |flags2| ShipFields { flags2, ..AVERAGE };
        assert!(ShipStats::new(hull(0x0020), &[]).fast_jump);
        assert!(
            !ShipStats::new(hull(0xFFDF), &[]).fast_jump,
            "every other bit"
        );
    }

    #[test]
    fn a_plain_ship_has_neither_an_inertialess_hull_nor_a_dampener() {
        let stats = ShipStats::new(AVERAGE, &[]);
        assert!(!stats.inertialess_hull);
        assert!(!stats.dampener);
    }

    #[test]
    fn a_hull_with_flags2_0x0040_is_inertialess() {
        assert_eq!(INERTIALESS_HULL, 0x0040);
        let hull = |flags2| ShipFields { flags2, ..AVERAGE };
        assert!(ShipStats::new(hull(0x0040), &[]).inertialess_hull);
        assert!(ShipStats::for_player(hull(0x0040), &[]).inertialess_hull);
        assert!(
            !ShipStats::new(hull(0xFFBF), &[]).inertialess_hull,
            "every other bit"
        );
    }

    #[test]
    fn an_inertial_dampener_outfit_is_carried_whatever_its_modval() {
        assert_eq!(INERTIAL_DAMPENER, 38);
        assert!(ShipStats::new(AVERAGE, &[outfit(INERTIAL_DAMPENER, 0, 1)]).dampener);
        assert!(ShipStats::new(AVERAGE, &[outfit(INERTIAL_DAMPENER, -7, 3)]).dampener);
        assert!(ShipStats::for_player(AVERAGE, &[outfit(INERTIAL_DAMPENER, 1, 1)]).dampener);
        assert!(
            !ShipStats::new(AVERAGE, &[outfit(INERTIAL_DAMPENER, 1, 0)]).dampener,
            "none carried"
        );
        assert!(
            !ShipStats::new(AVERAGE, &[outfit(FAST_JUMP, 1, 1)]).dampener,
            "another ModType"
        );
    }

    /// An outfit's four slots, `mods` first and blanks after, `count`
    /// carried.
    fn slots(mods: &[(i16, i16)], count: u16) -> Vec<OutfitMod> {
        (0..MOD_SLOTS)
            .map(|at| {
                let (mod_type, mod_val) = mods.get(at).copied().unwrap_or_default();
                outfit(mod_type, mod_val, count)
            })
            .collect()
    }

    #[test]
    fn a_plain_ship_has_no_afterburner() {
        assert_eq!(ShipStats::new(AVERAGE, &[]).afterburner, None);
        let speed = slots(&[(MORE_SPEED, 10)], 1);
        assert_eq!(ShipStats::for_player(AVERAGE, &speed).afterburner, None);
    }

    #[test]
    fn an_afterburner_uses_its_mod_val_x_0_0333_fuel_a_tick_whatever_the_count() {
        let one = slots(&[(MORE_SPEED, 10), (AFTERBURNER, 30)], 1);
        let usage = ShipStats::for_player(AVERAGE, &one).afterburner;
        assert_eq!(usage, Some((30.0 * 0.0333_f64) as f32));
        let three = slots(&[(AFTERBURNER, 30)], 3);
        assert_eq!(ShipStats::new(AVERAGE, &three).afterburner, usage);
    }

    #[test]
    fn the_last_carried_afterburner_gives_its_first_afterburner_slot() {
        let both = [
            slots(&[(AFTERBURNER, 30)], 1),
            slots(&[(MORE_SPEED, 5), (AFTERBURNER, 60), (AFTERBURNER, 90)], 1),
        ]
        .concat();
        let usage = ShipStats::for_player(AVERAGE, &both).afterburner;
        assert_eq!(usage, Some((60.0 * 0.0333_f64) as f32));
        let unowned = [both, slots(&[(AFTERBURNER, 120)], 0)].concat();
        assert_eq!(ShipStats::for_player(AVERAGE, &unowned).afterburner, usage);
    }

    #[test]
    fn none_carried_of_an_afterburner_is_no_afterburner() {
        let none = slots(&[(AFTERBURNER, 30)], 0);
        assert_eq!(ShipStats::for_player(AVERAGE, &none).afterburner, None);
    }

    #[test]
    fn a_multi_jump_outfit_gives_its_modval_once_whatever_the_count() {
        assert_eq!(MULTI_JUMP, 32);
        let total = |outfits: &[OutfitMod]| ShipStats::new(AVERAGE, outfits).multi_jump;
        // The stock Multi-Jumping Organ: 10.
        assert_eq!(total(&[outfit(MULTI_JUMP, 10, 1)]), 10);
        assert_eq!(total(&[outfit(MULTI_JUMP, 10, 2)]), 10, "not x count");
        assert_eq!(
            total(&[outfit(MULTI_JUMP, 3, 1), outfit(MULTI_JUMP, 4, 5)]),
            7,
            "summed over the outfits"
        );
        assert_eq!(total(&[outfit(MULTI_JUMP, 10, 0)]), 0, "none carried");
        assert_eq!(total(&[]), 0);
    }

    #[test]
    fn a_negative_multi_jump_total_is_none() {
        let stats = ShipStats::new(AVERAGE, &[outfit(MULTI_JUMP, -5, 1)]);
        assert_eq!(stats.multi_jump, 0);
        let mixed = ShipStats::new(
            AVERAGE,
            &[outfit(MULTI_JUMP, -5, 1), outfit(MULTI_JUMP, 8, 1)],
        );
        assert_eq!(mixed.multi_jump, 3);
    }

    #[test]
    fn every_other_mod_type_changes_nothing() {
        let plain = ShipStats::new(AVERAGE, &[]);
        for mod_type in [-1, 0, 1, 3, 10, 11, 13, 17, 39, 45, 99] {
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
        assert_eq!(stats.handling.turn_rate, 1.0, "floored, not 0.98");
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
        assert_eq!(
            none.handling,
            Handling {
                turn_rate: 1.0,
                ..Handling::default()
            },
            "the turn floored"
        );
        assert_eq!((none.shield, none.armor, none.fuel), (0.0, 0.0, 0.0));
        assert_eq!(none.capacity, 0);
    }

    #[test]
    fn a_turn_driven_to_none_or_below_by_outfits_is_floored_at_1_for_the_player_and_an_npc() {
        for mods in [
            [outfit(TURN_CHANGE, -2, 1)],
            [outfit(TURN_CHANGE, -101, 1)],
            [outfit(TURN_CHANGE, i16::MIN, u16::MAX)],
        ] {
            assert_eq!(ShipStats::new(AVERAGE, &mods).handling.turn_rate, 1.0);
            assert_eq!(
                ShipStats::for_player(AVERAGE, &mods).handling.turn_rate,
                1.0
            );
        }
    }

    #[test]
    fn a_class_turning_under_1_is_floored_for_the_player_alone_and_one_turning_none_for_neither() {
        let slow = ShipFields {
            maneuver: 5,
            ..AVERAGE
        };
        assert_eq!(ShipStats::for_player(slow, &[]).handling.turn_rate, 1.0);
        assert_eq!(ShipStats::new(slow, &[]).handling.turn_rate, 0.5);
        let rigid = ShipFields {
            maneuver: 0,
            ..AVERAGE
        };
        assert_eq!(ShipStats::for_player(rigid, &[]).handling.turn_rate, 0.0);
        assert_eq!(ShipStats::new(rigid, &[]).handling.turn_rate, 0.0);
        let lifted = [outfit(TURN_CHANGE, 50, 1)];
        assert_eq!(
            ShipStats::for_player(rigid, &lifted).handling.turn_rate,
            0.5,
            "a class of none keeps what its outfits give"
        );
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
        assert_eq!(
            stats.handling,
            Handling {
                turn_rate: 1.0,
                ..Handling::default()
            }
        );
        assert_eq!((stats.shield, stats.armor, stats.fuel), (0.0, 0.0, 0.0));
        assert_eq!((stats.jump_distance, stats.jump_days), (0.0, 1));
    }
}
