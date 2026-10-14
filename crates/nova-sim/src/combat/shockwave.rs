//! The shockwave of a heavy ship's final explosion: what it does to each
//! ship it reaches, and how it pushes it.
//!
//! In the original, `_HandleShipDisplay`'s final-explosion branch
//! (0x2d8ef-0x2da9c) calls `_DamageShip(target, &wreck.pos, 750, d, d,
//! wreck, 0, 0, 1, 1, 0, 0)` on every active ship within the explosion's
//! size of the wreck on each axis, the wreck's own slot left out:
//!
//! - **Who makes one.** A ship whose [`HullSpec::death_size`] is above
//!   none: of 100 tons or more (`Mass` above 99, signed, 0x2d92b) and not
//!   planet-type (`Flags` 0x0400, 0x2d90d). The player's ship makes one
//!   too: nothing in the loop tests for slot 0.
//! - **The damage** ([`damage`]) is `trunc(Mass x 0.0375 + 25)`
//!   ([`DAMAGE_PER_TON`] @0xdd598, [`DAMAGE_BASE`] @0xdd5a0, in doubles,
//!   `cvttsd2si` 0x2d96f), the same to shields and armour, with no
//!   falloff. It is taken as any hit that does not pass shields
//!   ([`damage::apply`](super::damage::apply), `_DamageShip`'s plain
//!   path @0x3ac00).
//! - **It never destroys** (arg +0x28, @0x3ac3a-0x3ac64): armour above
//!   none that the hit takes to none or below is left at
//!   [`SPARED_ARMOR`].
//! - **It never newly disables** (arg +0x2c, @0x3acc2-0x3ad4a): a ship not
//!   disabled before the hit that is after it has its armour set to what a
//!   self-repair leaves
//!   ([`repaired_armor`](crate::self_repair::repaired_armor), the same
//!   0.3333, 0.1 and 1.0). That clears the disabled line only while the
//!   ship holds 33,333 armour or less: a non-`TOUGH` hull that holds more
//!   is left disabled, and counts as newly disabled.
//! - **A dying ship** was disabled before and has no armour above none,
//!   so it takes the hit with neither floor.
//! - **The push** ([`shove`], @0x3aa6e-0x3ab7f): a ship whose jump is not
//!   under way (+0x50 at or below 0, @0x3aa7d), of a `Mass` above none and
//!   not planet-type (@0x3aac2-0x3aad2) gains [`IMPACT`] / its `Mass` along
//!   the bearing from the wreck to it (`_CalcInterceptAngle` @0x3aa9e,
//!   [`aim::bearing`](super::aim::bearing)) through `_AdjustedAccel`
//!   (@0x3ab14, [`launch_velocity`]) against its class's top speed (class
//!   +0x40, the `shïp` `Speed` / 100). `_SpeedLimit` (@0x2af91) then
//!   clamps each axis to its top speed. The player's 1.8 x cap while its
//!   afterburner is on (0x3ab19-0x3ab68) is left to
//!   `ship-handling-fidelity/phase-6-afterburner`.
//!
//! The hit is no attack of the wreck's (arg +0x20 is 0): nothing answers
//! it (see [`Strike::provokes`](super::Strike::provokes)). It makes no
//! sound of its own.

use super::aim;
use super::damage::{self, Hit};
use super::hull::{Condition, DisableRule, HullSpec};
use crate::bay::launch_velocity;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::reserves::{Gauge, Reserves};
use crate::self_repair::repaired_armor;

/// The shockwave's damage per ton of the wreck (the double @0xdd598).
pub const DAMAGE_PER_TON: f64 = 0.0375;
/// The shockwave's damage before the wreck's mass (the double @0xdd5a0).
pub const DAMAGE_BASE: f64 = 25.0;
/// The shockwave's impact: its push is this over the mass of the ship it
/// reaches (0x2ee, pushed @0x2da61).
pub const IMPACT: f32 = 750.0;
/// The armour a shockwave leaves a ship it would destroy (0x3f800000
/// @0x3ac64).
pub const SPARED_ARMOR: f32 = 1.0;

/// The damage the shockwave of a wreck of `hull` does to shields and to
/// armour: `trunc(Mass x 0.0375 + 25)`, in doubles.
#[must_use]
pub fn damage(hull: &HullSpec) -> f32 {
    // A `Mass` is an `i16`, so the damage stays far within an `f32`.
    #[allow(clippy::cast_possible_truncation)]
    let damage = f64::from(hull.mass)
        .mul_add(DAMAGE_PER_TON, DAMAGE_BASE)
        .trunc() as f32;
    damage
}

/// Takes a shockwave of `damage` off `reserves`, of a ship of `hull` that
/// was in `condition` before it, with the floors that keep it from
/// destroying or newly disabling the ship (see the module docs); `rule`
/// says when a ship is disabled.
pub fn hit(
    reserves: &mut Reserves,
    condition: Condition,
    hull: &HullSpec,
    rule: &dyn DisableRule,
    damage: f32,
) {
    let whole = reserves.armor.now > 0.0;
    let was_disabled = condition != Condition::Intact;
    damage::apply(
        reserves,
        Hit {
            mass: damage,
            energy: damage,
            passes_shields: false,
        },
    );
    let armor = &mut reserves.armor;
    if whole && armor.now <= 0.0 {
        armor.now = SPARED_ARMOR;
    }
    if !was_disabled && disabled(*armor, hull, rule) {
        armor.now = repaired_armor(armor.max, hull.tough);
    }
}

/// Whether a ship of `hull` with `armor` is disabled, as `rule` says: never
/// one that holds no armour.
fn disabled(armor: Gauge, hull: &HullSpec, rule: &dyn DisableRule) -> bool {
    armor.max > 0.0 && rule.disabled(armor, hull)
}

/// The velocity a shockwave centred on `centre` leaves a ship of `hull`
/// in `state`, of class speed `class_speed` and top speed `top_speed`;
/// none when it is not pushed: while its jump is under way (`jumping`),
/// or for a massless or planet-type hull.
#[must_use]
pub fn shove(
    state: &ShipState,
    centre: Vec2,
    hull: &HullSpec,
    jumping: bool,
    class_speed: f32,
    top_speed: f32,
) -> Option<Vec2> {
    if jumping || hull.mass <= 0.0 || hull.planet_type {
        return None;
    }
    let heading = aim::bearing(centre, state.position);
    let pushed = launch_velocity(state.velocity, heading, IMPACT / hull.mass, class_speed);
    Some(speed_limit(pushed, top_speed))
}

/// `velocity` with each axis clamped to `top_speed` either way
/// (`_SpeedLimit` @0x2af91).
fn speed_limit(velocity: Vec2, top_speed: f32) -> Vec2 {
    // Down to the top speed first, then up to its negative, as the
    // original tests them; `f32::clamp` would panic on a negative one.
    let clamp = |axis: f32| axis.min(top_speed).max(-top_speed);
    Vec2::new(clamp(velocity.x), clamp(velocity.y))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::hull::NovaDisable;

    fn of_mass(mass: f32) -> HullSpec {
        HullSpec {
            mass,
            ..HullSpec::default()
        }
    }

    /// Shields `shield` of `shield_max`, armour `armor` of `armor_max`.
    fn reserves(shield: f32, shield_max: f32, armor: f32, armor_max: f32) -> Reserves {
        Reserves {
            shield: Gauge {
                now: shield,
                max: shield_max,
            },
            armor: Gauge {
                now: armor,
                max: armor_max,
            },
            fuel: Gauge::full(100.0),
        }
    }

    /// The shields and armour `reserves` are left with by a shockwave of
    /// `damage` on a ship of `hull` in `condition`.
    fn after(
        mut reserves: Reserves,
        condition: Condition,
        hull: &HullSpec,
        damage: f32,
    ) -> (f32, f32) {
        hit(&mut reserves, condition, hull, &NovaDisable, damage);
        (reserves.shield.now, reserves.armor.now)
    }

    #[test]
    fn a_mass_200_shockwave_does_32_damage_over_65_pixels() {
        let wreck = of_mass(200.0);
        assert_eq!(damage(&wreck), 32.0, "32.5, truncated");
        assert_eq!(wreck.death_size(), 65.0);
        assert_eq!(damage(&of_mass(100.0)), 28.0, "28.75, truncated");
        assert_eq!(damage(&of_mass(1000.0)), 62.0, "62.5, truncated");
        assert_eq!((DAMAGE_PER_TON, DAMAGE_BASE), (0.0375, 25.0));
    }

    #[test]
    fn the_shockwave_takes_shields_first_and_leaves_armour_while_they_hold() {
        let hull = HullSpec::default();
        assert_eq!(
            after(
                reserves(100.0, 100.0, 300.0, 300.0),
                Condition::Intact,
                &hull,
                32.0
            ),
            (68.0, 300.0)
        );
        assert_eq!(
            after(
                reserves(20.0, 100.0, 300.0, 300.0),
                Condition::Intact,
                &hull,
                32.0
            ),
            (-10.0, 268.0),
            "the shield floored at a tenth below none, and the armour hit"
        );
    }

    #[test]
    fn the_shockwave_never_destroys_an_intact_or_disabled_ship() {
        let hull = HullSpec::default();
        let tough = HullSpec {
            tough: true,
            ..hull
        };
        let (_, intact) = after(
            reserves(0.0, 0.0, 120.0, 300.0),
            Condition::Intact,
            &hull,
            150.0,
        );
        assert!((intact - 100.99).abs() < 1e-4, "300 x 0.3333 + 1: {intact}");
        // A tough hull of 300 is disabled below 30.
        let (_, tough_intact) = after(
            reserves(0.0, 0.0, 35.0, 300.0),
            Condition::Intact,
            &tough,
            40.0,
        );
        assert_eq!(tough_intact, 31.0);
        let (_, disabled) = after(
            reserves(0.0, 0.0, 20.0, 300.0),
            Condition::Disabled,
            &hull,
            32.0,
        );
        assert_eq!(disabled, SPARED_ARMOR);
        assert_eq!(SPARED_ARMOR, 1.0);
        let (_, untouched) = after(
            reserves(0.0, 0.0, 250.0, 300.0),
            Condition::Intact,
            &hull,
            32.0,
        );
        assert_eq!(
            untouched, 218.0,
            "a hit that leaves it intact is taken whole"
        );
    }

    #[test]
    fn a_shockwave_that_disables_an_intact_ship_leaves_it_just_above_the_line() {
        let hull = HullSpec::default();
        // 300 x 33.333% is 99.999: 110 - 32 = 78 would disable it.
        let (_, armor) = after(
            reserves(0.0, 0.0, 110.0, 300.0),
            Condition::Intact,
            &hull,
            32.0,
        );
        assert!((armor - 100.99).abs() < 1e-4, "{armor}");
        let (_, disabled) = after(
            reserves(0.0, 0.0, 90.0, 300.0),
            Condition::Disabled,
            &hull,
            32.0,
        );
        assert_eq!(disabled, 58.0, "a disabled ship stays as the hit leaves it");
    }

    #[test]
    fn a_hull_over_33333_armour_is_floored_still_disabled() {
        let hull = HullSpec::default();
        let mut big = reserves(0.0, 0.0, 13_350.0, 40_000.0);
        assert!(!NovaDisable.disabled(big.armor, &hull), "intact before");
        hit(&mut big, Condition::Intact, &hull, &NovaDisable, 32.0);
        assert_eq!(big.armor.now, 13_333.0);
        assert!(NovaDisable.disabled(big.armor, &hull), "still disabled");
    }

    #[test]
    fn a_ship_that_holds_no_armour_is_neither_floored_nor_disabled() {
        /// Calls every ship disabled.
        #[derive(Debug)]
        struct AlwaysDisabled;

        impl DisableRule for AlwaysDisabled {
            fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
                true
            }
        }

        let hull = HullSpec::default();
        let mut none = reserves(0.0, 0.0, 0.0, 0.0);
        hit(&mut none, Condition::Intact, &hull, &AlwaysDisabled, 32.0);
        assert_eq!((none.shield.now, none.armor.now), (0.0, 0.0));
        let mut some = reserves(0.0, 0.0, 250.0, 300.0);
        hit(&mut some, Condition::Intact, &hull, &AlwaysDisabled, 32.0);
        assert!(
            (some.armor.now - 100.99).abs() < 1e-4,
            "the rule decides for one that holds some: {:?}",
            some.armor
        );
    }

    #[test]
    fn a_dying_ship_takes_the_shockwave_unfloored() {
        let hull = HullSpec::default();
        let dying = Condition::Dying {
            ticks_left: 5,
            starting: false,
        };
        assert_eq!(
            after(reserves(0.0, 0.0, -4.0, 300.0), dying, &hull, 32.0),
            (0.0, -36.0)
        );
    }

    fn at_rest(x: f32, y: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            heading: 0.0,
        }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn a_mass_200_ship_at_rest_is_shoved_3_75_away_from_the_wreck() {
        let hull = of_mass(200.0);
        let right = shove(&at_rest(40.0, 0.0), Vec2::ZERO, &hull, false, 10.0, 10.0);
        assert!(
            near(right.expect("pushed"), Vec2::new(3.75, 0.0)),
            "{right:?}"
        );
        let up = shove(
            &at_rest(5.0, -60.0),
            Vec2::new(5.0, 0.0),
            &hull,
            false,
            10.0,
            10.0,
        );
        assert!(near(up.expect("pushed"), Vec2::new(0.0, -3.75)), "{up:?}");
        assert_eq!(IMPACT, 750.0);
    }

    #[test]
    fn a_planet_type_massless_or_jumping_target_is_not_shoved() {
        let state = at_rest(40.0, 0.0);
        let planet = HullSpec {
            planet_type: true,
            ..of_mass(200.0)
        };
        assert_eq!(shove(&state, Vec2::ZERO, &planet, false, 10.0, 10.0), None);
        assert_eq!(
            shove(&state, Vec2::ZERO, &of_mass(0.0), false, 10.0, 10.0),
            None
        );
        assert_eq!(
            shove(&state, Vec2::ZERO, &of_mass(200.0), true, 10.0, 10.0),
            None
        );
    }

    #[test]
    fn the_shove_is_capped_at_the_class_speed_and_clamped_per_axis_to_top_speed() {
        let hull = of_mass(100.0);
        // 7.5 a tick to the right: none past the class speed of 5.
        let moving = ShipState {
            velocity: Vec2::new(6.0, 0.0),
            ..at_rest(40.0, 0.0)
        };
        let capped = shove(&moving, Vec2::ZERO, &hull, false, 5.0, 10.0);
        assert!(
            near(capped.expect("pushed"), Vec2::new(6.0, 0.0)),
            "already past the class speed: {capped:?}"
        );
        let slow = ShipState {
            velocity: Vec2::new(4.0, 0.0),
            ..moving
        };
        let pushed = shove(&slow, Vec2::ZERO, &hull, false, 5.0, 10.0);
        assert!(
            near(pushed.expect("pushed"), Vec2::new(10.0, 0.0)),
            "11.5, clamped to 10: {pushed:?}"
        );
        // Each axis clamped on its own to the top speed, either way.
        let fast = ShipState {
            velocity: Vec2::new(-20.0, 9.0),
            ..slow
        };
        let clamped = shove(&fast, Vec2::ZERO, &hull, false, 20.0, 8.0);
        assert!(
            near(clamped.expect("pushed"), Vec2::new(-8.0, 8.0)),
            "{clamped:?}"
        );
    }
}
