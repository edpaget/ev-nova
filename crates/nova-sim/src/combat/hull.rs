//! A ship's hull in combat: how big a target it is, when it is disabled,
//! and how it dies.
//!
//! - Its hit radius is a third of its `shän`'s `BaseXSize`
//!   ([`HIT_RADIUS_PER_SIZE`]): the half of the original beam test's
//!   0.66 x sprite width (`_HandleBeams`) that a beam's reach adds. The
//!   original tests shots against sprite masks, which the simulation does
//!   not have. A ship type without a `shän` is a placeholder
//!   [`DEFAULT_HIT_RADIUS`] across.
//! - Its board reach, how far off its centre on either axis the player
//!   can board it, is half its `shän`'s size ([`BOARD_REACH_PER_SIZE`],
//!   0.5 @0xdd128 x the sprite's width and height in
//!   `_HandlePlayerBoardAttempt`; the simulation reads only the width),
//!   and [`DEFAULT_BOARD_REACH`] without a `shän`.
//! - A ship is disabled as a [`DisableRule`] says. Nova's,
//!   [`NovaDisable`], is `_IsDisabled`'s.
//! - Its `Flags` 0x1000, 0x2000 and 0x4000 ([`BLIND_SPOTS`]) blind its
//!   turrets in front, to the sides and to the rear, as a weapon's own do
//!   (`_TurretBlindSpot`, which reads them from the `shïp`).
//! - Once its armour is gone it breaks up (its `Explode1` going off),
//!   counting down a timer of its `DeathDelay`, three times it for the
//!   player ([`PLAYER_BREAKUP_FACTOR`]), to [`EXPLOSION_BAND`], then is
//!   destroyed (its `Explode2`); see [`Breakup`] for a timer that never
//!   rises above 0. A
//!   ship of [`DEATH_SIZE_MASS`] tons or more dies in an explosion sized
//!   by its mass ([`HullSpec::death_size`], `_HandleShipDisplay`
//!   @0x2d7ef-0x2dee7), which scatters the `Explode2`'s extra explosions
//!   when it has them; the Bible's huge explosion for a `DeathDelay` of 60
//!   or more is not what the executable does.
//! - Its `Flags` 0x0400 ([`PLANET_TYPE`]) makes it a planet-type ship: an
//!   NPC's missile flies at it only from a planet-type weapon, and at any
//!   other ship only from any other weapon (`_AIFireMissile`
//!   @0x812b4-0x812fb, see [`fire`](crate::ai::fire)).
//! - Its `KeyCarried` ([`HullSpec::key_carried`]) is the ship a weapon of
//!   `Flags2` 0x0080 needs aboard (`_WeaponHasAmmo` @0xba07-0xba85, see
//!   [`Armament::has_ammo`](super::armament::Armament::has_ammo)); none
//!   below a `shïp` ID, as the original keeps -1 below 128 (@0x7a64b-0x7a67d).

use std::fmt::Debug;

use super::weapon::Explosion;
use crate::bay::FIRST_SHIP;
use crate::catalog::{HullRecord, ShipId};
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;

/// The hit radius of a ship type without a `shän`, in pixels: a
/// placeholder.
pub const DEFAULT_HIT_RADIUS: f32 = 16.0;
/// A ship's hit radius per pixel of its `shän`'s `BaseXSize`.
pub const HIT_RADIUS_PER_SIZE: f32 = 0.33;
/// The board reach of a ship type without a `shän`, in pixels: half a
/// 48-pixel sprite, a placeholder.
pub const DEFAULT_BOARD_REACH: f32 = 24.0;
/// A ship's board reach per pixel of its `shän`'s `BaseXSize`.
pub const BOARD_REACH_PER_SIZE: f32 = 0.5;
/// The `shïp` `Flags` bit that disables it at [`TOUGH_DISABLE_PERCENT`]
/// of its armour instead of [`DISABLE_PERCENT`].
pub const TOUGH: u16 = 0x0010;
/// The `shïp` `Flags` bit that shows its armour on the target panel
/// while its shields are down (`_DrawStatusTarg` @0x4bbaa; see
/// [`target_panel`](crate::target_panel)).
pub const SHOWS_ARMOR: u16 = 0x0100;
/// The `shïp` `Flags` bit that hides the target panel's status line
/// (`_DrawStatusTarg` @0x4b941).
pub const HIDES_STATUS: u16 = 0x0200;
/// The percentage of its armour below which a ship is disabled.
pub const DISABLE_PERCENT: f64 = 33.333;
/// The percentage for a ship type with [`TOUGH`] set.
pub const TOUGH_DISABLE_PERCENT: f64 = 10.0;
/// The `shïp` `Flags` bits that blind its turrets, as a `wëap`'s do.
pub const BLIND_SPOTS: u16 = 0x7000;
/// The `shïp` `Flags` bit of a planet-type ship.
pub const PLANET_TYPE: u16 = 0x0400;
/// The mass, in tons, from which a ship's death explosion has a size.
pub const DEATH_SIZE_MASS: f32 = 100.0;
/// The death explosion's size per ton (the double @0xdd588).
pub const DEATH_SIZE_PER_TON: f32 = 0.075;
/// The death explosion's size before the mass (the double @0xdd590).
pub const DEATH_SIZE_BASE: f32 = 50.0;

/// A ship type's hull, in the simulation's units.
// Each flag is one of the ship type's `Flags` bits, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HullSpec {
    /// How near, in pixels, a shot must pass its centre to hit it.
    pub hit_radius: f32,
    /// Whether it is disabled at [`TOUGH_DISABLE_PERCENT`].
    pub tough: bool,
    /// Whether the target panel shows its armour while its shields are
    /// down: its `Flags` [`SHOWS_ARMOR`].
    pub shows_armor: bool,
    /// Whether the target panel hides its status line: its `Flags`
    /// [`HIDES_STATUS`].
    pub hides_status: bool,
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
    /// How far off its centre, in pixels on either axis, the player can
    /// board it.
    pub board_reach: f32,
    /// Its sprite's width, its `shän`'s `BaseXSize`, in pixels; none
    /// without a sprite. A formation is spaced by it (see
    /// [`escort::spacing`](crate::escort::spacing)).
    pub sprite_size: Option<f32>,
    /// Its sprite's height, its `shän`'s `BaseYSize`, in pixels; none
    /// without a sprite. Whether it is on screen reads it with the width
    /// (see [`cloak::on_screen`](crate::cloak::on_screen)).
    pub sprite_height: Option<f32>,
    /// Whether it is a planet-type ship, its `Flags` [`PLANET_TYPE`].
    pub planet_type: bool,
    /// The ship a weapon of `Flags2` 0x0080 needs aboard, its
    /// `KeyCarried`; none below a `shïp` ID.
    pub key_carried: Option<ShipId>,
}

impl Default for HullSpec {
    /// A placeholder hull: [`DEFAULT_HIT_RADIUS`] across, gone at once,
    /// with no explosions.
    fn default() -> Self {
        Self {
            hit_radius: DEFAULT_HIT_RADIUS,
            tough: false,
            shows_armor: false,
            hides_status: false,
            death_delay: 0,
            breakup: None,
            explosion: None,
            mass: 0.0,
            blind_spots: 0,
            strength: 0.0,
            board_reach: DEFAULT_BOARD_REACH,
            sprite_size: None,
            sprite_height: None,
            planet_type: false,
            key_carried: None,
        }
    }
}

impl HullSpec {
    /// The hull `record` describes (see the module docs).
    #[must_use]
    pub fn new(record: &HullRecord) -> Self {
        let size = record.size.filter(|&size| size > 0).map(f32::from);
        let hit_radius = size.map_or(DEFAULT_HIT_RADIUS, |size| size * HIT_RADIUS_PER_SIZE);
        let board_reach = size.map_or(DEFAULT_BOARD_REACH, |size| size * BOARD_REACH_PER_SIZE);
        Self {
            hit_radius,
            tough: record.flags & TOUGH != 0,
            shows_armor: record.flags & SHOWS_ARMOR != 0,
            hides_status: record.flags & HIDES_STATUS != 0,
            death_delay: u32::try_from(record.death_delay).unwrap_or(0),
            breakup: Explosion::decode(record.explode1),
            explosion: Explosion::decode(record.explode2),
            mass: f32::from(record.mass.max(0)),
            blind_spots: record.flags & BLIND_SPOTS,
            strength: f32::from(record.strength.max(0)),
            board_reach,
            sprite_size: size,
            sprite_height: record.height.filter(|&height| height > 0).map(f32::from),
            planet_type: record.flags & PLANET_TYPE != 0,
            key_carried: record.key_carried.filter(|ship| ship.0 >= FIRST_SHIP),
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
    /// Its armour is gone, and it breaks up, counting down the original's
    /// death timer (field 0x3c) to its final explosion (see
    /// [`Combat`](super::Combat)'s step 9).
    Dying {
        /// The death timer: its `DeathDelay`, times
        /// [`PLAYER_BREAKUP_FACTOR`] for the player, as set on the
        /// breakup's first tick, less a tick for each tick since. The ship
        /// explodes on the tick it comes down to [`EXPLOSION_BAND`] or
        /// below.
        ticks_left: u32,
        /// Whether the timer was set on this tick: the original's "dying,
        /// timer not yet set" frame, on which `_HandleShipDisplay` sets it
        /// (@0x2d834-0x2d850) and does nothing else. False from the next
        /// tick on; under [`Breakup::Stalls`], true again on every tick
        /// the timer is set again. Phase 9's eject reads it, as
        /// `_HandlePlayer` (@0x6f248-0x6f286) runs before
        /// `_HandleShipDisplay` sets the timer.
        starting: bool,
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

/// What the player's `DeathDelay` is multiplied by as its breakup starts
/// (the 3.0 @0xdd664, applied @0x2d848 when +0x86 is 0, the player).
pub const PLAYER_BREAKUP_FACTOR: u32 = 3;
/// The death timer at or below which a breaking-up ship makes its final
/// explosion (the 2.0 @0xdd074, tested @0x2d85a).
pub const EXPLOSION_BAND: u32 = 2;

/// What becomes of a breakup whose death timer never rises above 0 by
/// the time `_HandleShipDisplay` checks it: a `DeathDelay` of 0 or less
/// for any ship, or of 1 for an NPC, whose timer `_HandleShip` takes back
/// to 0 before the check. No stock ship has one (the lowest stock
/// `DeathDelay` is 3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Breakup {
    /// The ship explodes: on its breakup tick with no `DeathDelay`, on the
    /// tick after with 1. The default, and the other reading.
    #[default]
    Explodes,
    /// The original's: `_HandleShipDisplay` sets a timer that is not above
    /// 0 again every frame (@0x2d80d-0x2d83d), and the decrements skip one
    /// that is not (@0x6d7eb, @0x35088), so the ship breaks up for good.
    /// For the player, the game never ends.
    Stalls,
}

impl From<RuleSource> for Breakup {
    /// The engine's reading [`Breakup::Stalls`]; the other
    /// [`Breakup::Explodes`].
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Stalls,
            RuleSource::Bible => Self::Explodes,
        }
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
    use crate::catalog::{BoomId, ShipId};
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
    fn the_sprite_size_is_the_shäns_size() {
        let shuttle = HullSpec::new(&HullRecord {
            size: Some(24),
            ..hull(128)
        });
        assert_eq!(shuttle.sprite_size, Some(24.0));
        for size in [None, Some(0), Some(-5)] {
            let placeholder = HullSpec::new(&HullRecord { size, ..hull(128) });
            assert_eq!(placeholder.sprite_size, None, "{size:?}");
        }
        assert_eq!(HullSpec::default().sprite_size, None);
    }

    #[test]
    fn the_sprite_height_is_the_shäns_height() {
        let tall = HullSpec::new(&HullRecord {
            size: Some(24),
            height: Some(40),
            ..hull(128)
        });
        assert_eq!(tall.sprite_height, Some(40.0));
        assert_eq!(tall.sprite_size, Some(24.0));
        for height in [None, Some(0), Some(-5)] {
            let placeholder = HullSpec::new(&HullRecord {
                height,
                ..hull(128)
            });
            assert_eq!(placeholder.sprite_height, None, "{height:?}");
        }
        assert_eq!(HullSpec::default().sprite_height, None);
    }

    #[test]
    fn the_board_reach_is_half_the_shäns_size() {
        let shuttle = HullSpec::new(&HullRecord {
            size: Some(25),
            ..hull(128)
        });
        assert_eq!(shuttle.board_reach, 12.5);
        assert_eq!(BOARD_REACH_PER_SIZE, 0.5);
        for size in [None, Some(0), Some(-5)] {
            let placeholder = HullSpec::new(&HullRecord { size, ..hull(128) });
            assert_eq!(placeholder.board_reach, DEFAULT_BOARD_REACH, "{size:?}");
        }
        assert_eq!(DEFAULT_BOARD_REACH, 24.0);
        assert_eq!(HullSpec::default().board_reach, 24.0);
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
    fn a_hull_is_planet_type_with_its_flags_0x0400() {
        let planet = HullSpec::new(&HullRecord {
            flags: 0x0410,
            ..hull(141)
        });
        assert!(planet.planet_type);
        assert!(planet.tough, "the other bits still read");
        let ordinary = HullSpec::new(&HullRecord {
            flags: 0xfbff,
            ..hull(141)
        });
        assert!(!ordinary.planet_type);
        assert!(!HullSpec::default().planet_type);
        assert_eq!(PLANET_TYPE, 0x0400);
    }

    #[test]
    fn a_hulls_key_carried_ship_is_none_below_128() {
        let carrier = |key| {
            HullSpec::new(&HullRecord {
                key_carried: key,
                ..hull(141)
            })
            .key_carried
        };
        assert_eq!(carrier(Some(ShipId(128))), Some(ShipId(128)));
        assert_eq!(carrier(Some(ShipId(300))), Some(ShipId(300)));
        assert_eq!(carrier(Some(ShipId(127))), None);
        assert_eq!(carrier(Some(ShipId(0))), None);
        assert_eq!(carrier(Some(ShipId(-5))), None);
        assert_eq!(carrier(None), None);
        assert_eq!(HullSpec::default().key_carried, None);
    }

    #[test]
    fn a_hulls_target_panel_flags_are_its_flags_0x0100_and_0x0200() {
        let armour = HullSpec::new(&HullRecord {
            flags: 0x0100,
            ..hull(128)
        });
        assert!(armour.shows_armor);
        assert!(!armour.hides_status);
        let hidden = HullSpec::new(&HullRecord {
            flags: 0x0200,
            ..hull(129)
        });
        assert!(!hidden.shows_armor);
        assert!(hidden.hides_status);
        let neither = HullSpec::new(&HullRecord {
            flags: !0x0300,
            ..hull(130)
        });
        assert!(!neither.shows_armor);
        assert!(!neither.hides_status);
        assert!(!HullSpec::default().shows_armor);
        assert!(!HullSpec::default().hides_status);
        assert_eq!((SHOWS_ARMOR, HIDES_STATUS), (0x0100, 0x0200));
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
        for starting in [false, true] {
            assert!(
                !Condition::Dying {
                    ticks_left: 3,
                    starting
                }
                .hittable()
            );
        }
        assert!(!Condition::Destroyed.hittable());
        assert_eq!(Condition::default(), Condition::Intact);
    }

    #[test]
    fn the_engine_reading_stalls_and_the_other_explodes() {
        assert_eq!(Breakup::from(RuleSource::Engine), Breakup::Stalls);
        assert_eq!(Breakup::from(RuleSource::Bible), Breakup::Explodes);
        assert_eq!(Breakup::default(), Breakup::Explodes);
        assert_eq!((PLAYER_BREAKUP_FACTOR, EXPLOSION_BAND), (3, 2));
    }
}
