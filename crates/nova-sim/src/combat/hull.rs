//! A ship's hull in combat: how big a target it is, when it is disabled,
//! and how it dies.
//!
//! - Its hit radius is a third of its `shän`'s `BaseXSize`
//!   ([`HIT_RADIUS_PER_SIZE`]): the half of the original beam test's
//!   0.66 x sprite width (`_HandleBeams`) that a beam's reach adds. The
//!   original tests shots against sprite masks, which the simulation does
//!   not have. A ship type without a `shän` is a placeholder
//!   [`DEFAULT_HIT_RADIUS`] across.
//! - A ship is disabled as a [`DisableRule`] says. Nova's,
//!   [`NovaDisable`], is `_IsDisabled`'s.
//! - Its `Flags` 0x1000, 0x2000 and 0x4000 ([`BLIND_SPOTS`]) blind its
//!   turrets in front, to the sides and to the rear, as a weapon's own do
//!   (`_TurretBlindSpot`, which reads them from the `shïp`).
//! - Once its armour is gone it breaks up for its `DeathDelay` ticks
//!   (its `Explode1` going off), then is destroyed (its `Explode2`). A
//!   ship of [`DEATH_SIZE_MASS`] tons or more dies in an explosion sized
//!   by its mass ([`HullSpec::death_size`], `_HandleShipDisplay`
//!   @0x2d7ef-0x2dee7), which scatters the `Explode2`'s extra explosions
//!   when it has them; the Bible's huge explosion for a `DeathDelay` of 60
//!   or more is not what the executable does.

use std::fmt::Debug;

use super::weapon::Explosion;
use crate::catalog::HullRecord;
use crate::reserves::Gauge;

/// The hit radius of a ship type without a `shän`, in pixels: a
/// placeholder.
pub const DEFAULT_HIT_RADIUS: f32 = 16.0;
/// A ship's hit radius per pixel of its `shän`'s `BaseXSize`.
pub const HIT_RADIUS_PER_SIZE: f32 = 0.33;
/// The `shïp` `Flags` bit that disables it at [`TOUGH_DISABLE_PERCENT`]
/// of its armour instead of [`DISABLE_PERCENT`].
pub const TOUGH: u16 = 0x0010;
/// The percentage of its armour below which a ship is disabled.
pub const DISABLE_PERCENT: f64 = 33.333;
/// The percentage for a ship type with [`TOUGH`] set.
pub const TOUGH_DISABLE_PERCENT: f64 = 10.0;
/// The `shïp` `Flags` bits that blind its turrets, as a `wëap`'s do.
pub const BLIND_SPOTS: u16 = 0x7000;
/// The mass, in tons, from which a ship's death explosion has a size.
pub const DEATH_SIZE_MASS: f32 = 100.0;
/// The death explosion's size per ton (the double @0xdd588).
pub const DEATH_SIZE_PER_TON: f32 = 0.075;
/// The death explosion's size before the mass (the double @0xdd590).
pub const DEATH_SIZE_BASE: f32 = 50.0;

/// A ship type's hull, in the simulation's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HullSpec {
    /// How near, in pixels, a shot must pass its centre to hit it.
    pub hit_radius: f32,
    /// Whether it is disabled at [`TOUGH_DISABLE_PERCENT`].
    pub tough: bool,
    /// Ticks it breaks up for before it is destroyed.
    pub death_delay: u32,
    /// The explosion as it breaks up, its `Explode1`.
    pub breakup: Option<Explosion>,
    /// The explosion that destroys it, its `Explode2`.
    pub explosion: Option<Explosion>,
    /// Its mass, in tons.
    pub mass: f32,
    /// Where its turrets cannot fire: its `Flags` 0x1000 (front), 0x2000
    /// (sides) and 0x4000 (rear), as a weapon's `Flags` mark them.
    pub blind_spots: u16,
    /// How strong it counts in a fight, its `Strength`; none below none.
    pub strength: f32,
}

impl Default for HullSpec {
    /// A placeholder hull: [`DEFAULT_HIT_RADIUS`] across, gone at once,
    /// with no explosions.
    fn default() -> Self {
        Self {
            hit_radius: DEFAULT_HIT_RADIUS,
            tough: false,
            death_delay: 0,
            breakup: None,
            explosion: None,
            mass: 0.0,
            blind_spots: 0,
            strength: 0.0,
        }
    }
}

impl HullSpec {
    /// The hull `record` describes (see the module docs).
    #[must_use]
    pub fn new(record: &HullRecord) -> Self {
        let hit_radius = record
            .size
            .filter(|&size| size > 0)
            .map_or(DEFAULT_HIT_RADIUS, |size| {
                f32::from(size) * HIT_RADIUS_PER_SIZE
            });
        Self {
            hit_radius,
            tough: record.flags & TOUGH != 0,
            death_delay: u32::try_from(record.death_delay).unwrap_or(0),
            breakup: Explosion::decode(record.explode1),
            explosion: Explosion::decode(record.explode2),
            mass: f32::from(record.mass.max(0)),
            blind_spots: record.flags & BLIND_SPOTS,
            strength: f32::from(record.strength.max(0)),
        }
    }

    /// The size of the explosion it dies in: `trunc(mass x 0.075 + 50)`
    /// for a ship of [`DEATH_SIZE_MASS`] tons or more, and none for a
    /// lighter one.
    #[must_use]
    pub fn death_size(&self) -> f32 {
        if self.mass >= DEATH_SIZE_MASS {
            self.mass
                .mul_add(DEATH_SIZE_PER_TON, DEATH_SIZE_BASE)
                .trunc()
        } else {
            0.0
        }
    }
}

/// How a ship is holding up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Condition {
    /// Flying and fighting.
    #[default]
    Intact,
    /// Disabled: it drifts, and cannot fire, land or jump. Further damage
    /// can still destroy it.
    Disabled,
    /// Its armour is gone, and it breaks up, for this many more ticks.
    Dying {
        /// The ticks before it is destroyed.
        ticks_left: u32,
    },
    /// Destroyed.
    Destroyed,
}

impl Condition {
    /// Whether shots and beams can hit it: intact or disabled.
    #[must_use]
    pub fn hittable(self) -> bool {
        matches!(self, Self::Intact | Self::Disabled)
    }
}

/// When a ship is disabled.
pub trait DisableRule: Debug {
    /// Whether a ship of `hull` with `armor` is disabled. The fight asks
    /// it only of a ship that holds some armour: one that holds none is
    /// never disabled.
    fn disabled(&self, armor: Gauge, hull: &HullSpec) -> bool;
}

/// Nova's rule, the default (`_IsDisabled`): a ship is disabled while its
/// armour is below [`DISABLE_PERCENT`] of what it holds, or
/// [`TOUGH_DISABLE_PERCENT`] for a [`TOUGH`] ship type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaDisable;

impl DisableRule for NovaDisable {
    fn disabled(&self, armor: Gauge, hull: &HullSpec) -> bool {
        let percent = if hull.tough {
            TOUGH_DISABLE_PERCENT
        } else {
            DISABLE_PERCENT
        };
        f64::from(armor.now) * 100.0 < f64::from(armor.max) * percent
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::BoomId;
    use crate::testkit::hull;

    fn armor(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    fn disabled(now: f32, max: f32, tough: bool) -> bool {
        let hull = HullSpec {
            tough,
            ..HullSpec::default()
        };
        NovaDisable.disabled(armor(now, max), &hull)
    }

    #[test]
    fn at_exactly_a_third_a_ship_is_not_disabled_and_just_below_it_is() {
        // 33.333% of 100,000 is 33,333.
        assert!(!disabled(33_333.0, 100_000.0, false));
        assert!(disabled(33_332.99, 100_000.0, false));
        assert!(disabled(33_332.0, 100_000.0, false));
        assert!(!disabled(45.0, 45.0, false));
        assert!(!disabled(15.0, 45.0, false), "15 is a third and more");
        assert!(disabled(14.99, 45.0, false));
        assert!(disabled(0.0, 45.0, false));
        assert!(disabled(-3.0, 45.0, false));
    }

    #[test]
    fn a_tough_ship_type_is_disabled_at_a_tenth() {
        assert!(!disabled(10.0, 100.0, true));
        assert!(disabled(9.999, 100.0, true));
        assert!(!disabled(14.99, 45.0, true), "a third does not disable it");
        assert_eq!((DISABLE_PERCENT, TOUGH_DISABLE_PERCENT), (33.333, 10.0));
    }

    #[test]
    fn a_ship_that_holds_no_armour_is_never_disabled() {
        assert!(!disabled(0.0, 0.0, false));
        assert!(!disabled(0.0, 0.0, true));
    }

    #[test]
    fn the_hit_radius_is_a_third_of_the_shäns_size() {
        let shuttle = HullSpec::new(&HullRecord {
            size: Some(24),
            ..hull(128)
        });
        assert!((shuttle.hit_radius - 7.92).abs() < 1e-5, "{shuttle:?}");
        assert_eq!(HIT_RADIUS_PER_SIZE, 0.33);
        for size in [None, Some(0), Some(-5)] {
            let placeholder = HullSpec::new(&HullRecord { size, ..hull(128) });
            assert_eq!(placeholder.hit_radius, DEFAULT_HIT_RADIUS, "{size:?}");
        }
        assert_eq!(DEFAULT_HIT_RADIUS, 16.0);
        assert_eq!(HullSpec::default().hit_radius, 16.0);
    }

    #[test]
    fn a_hull_reads_its_flag_delay_explosions_and_mass() {
        let spec = HullSpec::new(&HullRecord {
            flags: 0x0130,
            death_delay: 60,
            explode1: 4,
            explode2: 1005,
            mass: 120,
            strength: 325,
            ..hull(141)
        });
        assert_eq!(spec.strength, 325.0);
        assert_eq!(HullSpec::default().strength, 0.0);
        assert!(spec.tough);
        assert_eq!(spec.death_delay, 60);
        assert_eq!(
            spec.breakup,
            Some(Explosion {
                boom: BoomId(132),
                extra: false
            })
        );
        assert_eq!(
            spec.explosion,
            Some(Explosion {
                boom: BoomId(133),
                extra: true
            })
        );
        assert_eq!(spec.mass, 120.0);
        let trader = HullSpec::new(&HullRecord {
            flags: 0x0120,
            death_delay: -5,
            mass: -1,
            strength: -2,
            ..hull(128)
        });
        assert!(!trader.tough);
        assert_eq!((trader.death_delay, trader.mass), (0, 0.0));
        assert_eq!(trader.strength, 0.0, "none below none");
        assert_eq!((trader.breakup, trader.explosion), (None, None));
        assert_eq!(TOUGH, 0x0010);
    }

    #[test]
    fn a_hulls_turret_blind_spots_are_its_flags_0x7000() {
        let destroyer = HullSpec::new(&HullRecord {
            flags: 0x4130,
            ..hull(141)
        });
        assert_eq!(destroyer.blind_spots, 0x4000);
        let carrier = HullSpec::new(&HullRecord {
            flags: 0xffff,
            ..hull(222)
        });
        assert_eq!(carrier.blind_spots, 0x7000);
        assert_eq!(HullSpec::new(&hull(128)).blind_spots, 0);
        assert_eq!(HullSpec::default().blind_spots, 0);
        assert_eq!(BLIND_SPOTS, 0x7000);
    }

    #[test]
    fn a_ship_of_100_tons_or_more_dies_in_an_explosion_sized_by_its_mass() {
        let of = |mass| {
            HullSpec {
                mass,
                ..HullSpec::default()
            }
            .death_size()
        };
        assert_eq!(of(0.0), 0.0);
        assert_eq!(of(99.0), 0.0);
        assert_eq!(of(99.99), 0.0);
        assert_eq!(of(100.0), 57.0, "57.5, truncated");
        assert_eq!(of(120.0), 59.0);
        assert_eq!(of(10_000.0), 800.0);
        assert_eq!(
            (DEATH_SIZE_MASS, DEATH_SIZE_PER_TON, DEATH_SIZE_BASE),
            (100.0, 0.075, 50.0)
        );
    }

    #[test]
    fn intact_and_disabled_ships_can_be_hit_and_dying_or_destroyed_ones_cannot() {
        assert!(Condition::Intact.hittable());
        assert!(Condition::Disabled.hittable());
        assert!(!Condition::Dying { ticks_left: 3 }.hittable());
        assert!(!Condition::Destroyed.hittable());
        assert_eq!(Condition::default(), Condition::Intact);
    }
}
