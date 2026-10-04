//! A weapon in the simulation's units: its `wëap` decoded once into a
//! [`WeaponSpec`].
//!
//! The values are the original's (`_SpawnShot`, `_HandleShotGuidance` and
//! `_FirePlayerWeapon` in the `EV Nova` executable) and the Bible's:
//!
//! - `Speed` is pixels a tick x100, as a ship's is
//!   ([`SPEED_PER_PIXEL_PER_TICK`]): the Light Blaster's 1500 is 15 pixels
//!   a tick.
//! - `Count` is a shot's or beam's life in ticks, so a projectile's range
//!   from a resting firer is its speed times its life.
//! - `Guidance` -1, 0, 5 and 6 are the [`Guidance`] this simulation flies;
//!   any other is [`Guidance::Other`].
//! - `AmmoType` is decoded as [`Ammo`]: -1 unlimited, 0-255 the rounds
//!   of `wëap` 128+n, and -1000 and below fuel, |n + 1000| / 10 units a
//!   shot.
//! - `ExplodType`, and a `shïp`'s `Explode1` and `Explode2`, decode as an
//!   [`Explosion`]: 0-63 `bööm` 128+n, 1000-1063 the same with extra
//!   type-0 explosions around it, anything else none.
//! - Negative reloads, lives, radii, inaccuracies and bursts count as none.

use crate::catalog::{BoomId, WeaponId, WeaponRecord};
use crate::handling::SPEED_PER_PIXEL_PER_TICK;

/// How a weapon's shots fly, from its `Guidance`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guidance {
    /// -1: an unguided projectile, at constant velocity.
    Unguided,
    /// 0: a beam.
    Beam,
    /// 5: a freefall bomb, launched at 80% of the firer's velocity, that
    /// turns into the wind.
    FreefallBomb,
    /// 6: a freeflight rocket, launched straight ahead, that accelerates
    /// to its top speed.
    Rocket,
    /// Any other: guided weapons, turrets and fighter bays, which later
    /// phases fly.
    Other(i16),
}

impl Guidance {
    /// The guidance `raw` means.
    #[must_use]
    pub fn decode(raw: i16) -> Self {
        match raw {
            -1 => Self::Unguided,
            0 => Self::Beam,
            5 => Self::FreefallBomb,
            6 => Self::Rocket,
            other => Self::Other(other),
        }
    }
}

/// The `AmmoType` that fires without ammo.
pub const UNLIMITED_AMMO: i16 = -1;
/// The `wëap` an `AmmoType` of 0 draws its rounds from.
pub const FIRST_AMMO_WEAPON: i16 = 128;
/// The highest `AmmoType` that draws rounds.
pub const LAST_AMMO_TYPE: i16 = 255;
/// The `AmmoType` from which down a weapon burns fuel.
pub const FUEL_AMMO: i16 = -1000;
/// How many steps of `AmmoType` below [`FUEL_AMMO`] are a unit of fuel a
/// shot (the Bible: "-1005 = 0.5 units per shot").
pub const FUEL_AMMO_PER_UNIT: f32 = 10.0;

/// What a weapon spends on each shot, from its `AmmoType`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ammo {
    /// Nothing.
    Unlimited,
    /// A round of this weapon's ammunition.
    Rounds(WeaponId),
    /// This much fuel.
    Fuel(f32),
    /// Anything else (-999 destroys the firer, which a later phase does):
    /// for now, nothing.
    Other(i16),
}

impl Ammo {
    /// The ammo `raw` means.
    #[must_use]
    pub fn decode(raw: i16) -> Self {
        match raw {
            UNLIMITED_AMMO => Self::Unlimited,
            0..=LAST_AMMO_TYPE => Self::Rounds(WeaponId(FIRST_AMMO_WEAPON + raw)),
            ..=FUEL_AMMO => {
                let steps = i32::from(FUEL_AMMO) - i32::from(raw);
                Self::Fuel(steps as f32 / FUEL_AMMO_PER_UNIT)
            }
            other => Self::Other(other),
        }
    }
}

/// The first `bööm`, explosion type 0.
pub const FIRST_BOOM: i16 = 128;
/// How many explosion types there are.
pub const BOOM_TYPES: i16 = 64;
/// What an explosion type is offset by to add extra small explosions.
pub const EXTRA_EXPLOSIONS: i16 = 1000;

/// An explosion: its `bööm`, and whether extra type-0 explosions go off
/// around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Explosion {
    /// Its `bööm`.
    pub boom: BoomId,
    /// Whether a random number of type-0 explosions go off around it.
    pub extra: bool,
}

impl Explosion {
    /// The explosion an `ExplodType`, `Explode1` or `Explode2` of `raw`
    /// means, if any.
    #[must_use]
    pub fn decode(raw: i16) -> Option<Self> {
        let (kind, extra) = if raw >= EXTRA_EXPLOSIONS {
            (raw - EXTRA_EXPLOSIONS, true)
        } else {
            (raw, false)
        };
        (0..BOOM_TYPES).contains(&kind).then_some(Self {
            boom: BoomId(FIRST_BOOM + kind),
            extra,
        })
    }
}

/// A weapon, in the simulation's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponSpec {
    /// Its `wëap`.
    pub id: WeaponId,
    /// How its shots fly.
    pub guidance: Guidance,
    /// Ticks between one copy's shots.
    pub reload: f32,
    /// A shot's or a beam's life, in ticks.
    pub lifetime: u32,
    /// Damage to armour.
    pub mass_damage: f32,
    /// Damage to shields.
    pub energy_damage: f32,
    /// A shot's speed, in pixels a tick.
    pub speed: f32,
    /// What each shot spends.
    pub ammo: Ammo,
    /// How far off its heading a shot may leave, in degrees.
    pub inaccuracy: u32,
    /// The explosion where it hits or detonates, if any.
    pub explosion: Option<Explosion>,
    /// How near a ship sets its shots off, in pixels.
    pub prox_radius: f32,
    /// How far its blast reaches, in pixels; none for no blast.
    pub blast_radius: f32,
    /// A beam's length, in pixels.
    pub beam_length: f32,
    /// Shots a copy fires before the burst reload; none for no bursts.
    pub burst_count: u32,
    /// Ticks the burst reload takes.
    pub burst_reload: f32,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Seeker` flags.
    pub seeker: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `Flags3`.
    pub flags3: u16,
}

/// `Flags`: fired by the second trigger.
pub const SECONDARY: u16 = 0x0002;
/// `Flags`: passes through shields.
pub const PASSES_SHIELDS: u16 = 0x0020;
/// `Flags`: its blast does not hurt the player.
pub const SPARES_PLAYER: u16 = 0x0100;
/// `Flags`: its shots detonate at the end of their life.
pub const DETONATES: u16 = 0x8000;
/// `Flags3`: it uses ammo only at the end of a burst.
pub const AMMO_AT_BURST_END: u16 = 0x0001;
/// `Flags2`: hidden when out of ammo, so selecting a secondary skips it
/// while it has no rounds.
pub const HIDE_WHEN_EMPTY: u16 = 0x0800;

impl WeaponSpec {
    /// The weapon `record` describes (see the module docs).
    #[must_use]
    pub fn new(record: &WeaponRecord) -> Self {
        let ticks = |raw: i16| f32::from(raw.max(0));
        let count = |raw: i16| u32::try_from(raw).unwrap_or(0);
        Self {
            id: record.id,
            guidance: Guidance::decode(record.guidance),
            reload: ticks(record.reload),
            lifetime: count(record.count),
            mass_damage: f32::from(record.mass_dmg),
            energy_damage: f32::from(record.energy_dmg),
            speed: f32::from(record.speed) / SPEED_PER_PIXEL_PER_TICK,
            ammo: Ammo::decode(record.ammo_type),
            inaccuracy: count(record.inaccuracy),
            explosion: Explosion::decode(record.explod_type),
            prox_radius: ticks(record.prox_radius),
            blast_radius: ticks(record.blast_radius),
            beam_length: ticks(record.beam_length),
            burst_count: count(record.burst_count),
            burst_reload: ticks(record.burst_reload),
            flags: record.flags,
            seeker: record.seeker,
            flags2: record.flags2,
            flags3: record.flags3,
        }
    }

    /// Whether the second trigger fires it.
    #[must_use]
    pub fn secondary(&self) -> bool {
        self.flags & SECONDARY != 0
    }

    /// Whether it passes through shields.
    #[must_use]
    pub fn passes_shields(&self) -> bool {
        self.flags & PASSES_SHIELDS != 0
    }

    /// Whether its blast spares the player who fired it.
    #[must_use]
    pub fn spares_player(&self) -> bool {
        self.flags & SPARES_PLAYER != 0
    }

    /// Whether its shots blow up at the end of their life.
    #[must_use]
    pub fn detonates(&self) -> bool {
        self.flags & DETONATES != 0
    }

    /// Whether it uses ammo only at the end of a burst.
    #[must_use]
    pub fn ammo_at_burst_end(&self) -> bool {
        self.flags3 & AMMO_AT_BURST_END != 0
    }

    /// Whether it is hidden while out of ammo.
    #[must_use]
    pub fn hides_when_empty(&self) -> bool {
        self.flags2 & HIDE_WHEN_EMPTY != 0
    }

    /// How far it reaches, in pixels: a beam its length, and a shot its
    /// speed for its life (from a resting firer, at its top speed).
    #[must_use]
    pub fn range(&self) -> f32 {
        match self.guidance {
            Guidance::Beam => self.beam_length,
            _ => self.speed * self.lifetime as f32,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::testkit::weapon;

    /// The stock Light Blaster's numbers.
    fn light_blaster() -> WeaponRecord {
        WeaponRecord {
            reload: 10,
            count: 13,
            mass_dmg: 1,
            energy_dmg: 4,
            speed: 1500,
            inaccuracy: 9,
            prox_radius: 5,
            blast_radius: 6,
            flags: 0x6100,
            ..weapon(128)
        }
    }

    #[test]
    fn a_speed_of_1500_is_15_pixels_a_tick() {
        let spec = WeaponSpec::new(&light_blaster());
        assert_eq!(spec.speed, 15.0);
        assert_eq!(spec.lifetime, 13);
        assert_eq!(spec.reload, 10.0);
        assert_eq!((spec.mass_damage, spec.energy_damage), (1.0, 4.0));
        assert_eq!(spec.inaccuracy, 9);
        assert_eq!((spec.prox_radius, spec.blast_radius), (5.0, 6.0));
        assert_eq!(spec.id, WeaponId(128));
    }

    #[test]
    fn the_light_blasters_range_is_195_pixels() {
        assert_eq!(WeaponSpec::new(&light_blaster()).range(), 195.0);
    }

    #[test]
    fn a_beams_range_is_its_length() {
        let beam = WeaponSpec::new(&WeaponRecord {
            guidance: 0,
            count: 15,
            speed: 1000,
            beam_length: 400,
            ..weapon(146)
        });
        assert_eq!(beam.range(), 400.0);
        assert_eq!(beam.beam_length, 400.0);
    }

    #[test]
    fn guidance_decodes_at_each_value() {
        for (raw, guidance) in [
            (-1, Guidance::Unguided),
            (0, Guidance::Beam),
            (5, Guidance::FreefallBomb),
            (6, Guidance::Rocket),
            (1, Guidance::Other(1)),
            (2, Guidance::Other(2)),
            (3, Guidance::Other(3)),
            (4, Guidance::Other(4)),
            (7, Guidance::Other(7)),
            (10, Guidance::Other(10)),
            (99, Guidance::Other(99)),
            (-2, Guidance::Other(-2)),
        ] {
            assert_eq!(Guidance::decode(raw), guidance, "{raw}");
        }
    }

    #[test]
    fn ammo_type_decodes_at_each_edge() {
        for (raw, ammo) in [
            (-1, Ammo::Unlimited),
            (0, Ammo::Rounds(WeaponId(128))),
            (10, Ammo::Rounds(WeaponId(138))),
            (255, Ammo::Rounds(WeaponId(383))),
            (256, Ammo::Other(256)),
            (-2, Ammo::Other(-2)),
            (-999, Ammo::Other(-999)),
            (-1000, Ammo::Fuel(0.0)),
            (-1005, Ammo::Fuel(0.5)),
            (-1010, Ammo::Fuel(1.0)),
            (-1250, Ammo::Fuel(25.0)),
            (i16::MIN, Ammo::Fuel(3176.8)),
        ] {
            assert_eq!(Ammo::decode(raw), ammo, "{raw}");
        }
    }

    #[test]
    fn explosions_decode_at_each_edge() {
        let boom = |id, extra| {
            Some(Explosion {
                boom: BoomId(id),
                extra,
            })
        };
        for (raw, explosion) in [
            (-1, None),
            (-2, None),
            (0, boom(128, false)),
            (63, boom(191, false)),
            (64, None),
            (999, None),
            (1000, boom(128, true)),
            (1063, boom(191, true)),
            (1064, None),
        ] {
            assert_eq!(Explosion::decode(raw), explosion, "{raw}");
        }
    }

    #[test]
    fn negative_counts_and_radii_are_none() {
        let spec = WeaponSpec::new(&WeaponRecord {
            reload: -5,
            count: -1,
            inaccuracy: -10,
            prox_radius: -3,
            blast_radius: -4,
            beam_length: -2,
            burst_count: -1,
            burst_reload: -30,
            ..weapon(128)
        });
        assert_eq!((spec.reload, spec.lifetime, spec.inaccuracy), (0.0, 0, 0));
        assert_eq!((spec.prox_radius, spec.blast_radius), (0.0, 0.0));
        assert_eq!(spec.beam_length, 0.0);
        assert_eq!((spec.burst_count, spec.burst_reload), (0, 0.0));
    }

    #[test]
    fn the_flags_say_what_they_do() {
        let flagged = |flags, flags3| {
            WeaponSpec::new(&WeaponRecord {
                flags,
                flags3,
                ..weapon(128)
            })
        };
        let plain = flagged(0, 0);
        assert!(!plain.secondary() && !plain.passes_shields());
        assert!(!plain.spares_player() && !plain.detonates());
        assert!(!plain.ammo_at_burst_end());
        assert!(flagged(0x0002, 0).secondary());
        assert!(flagged(0x0020, 0).passes_shields());
        assert!(flagged(0x0100, 0).spares_player());
        assert!(flagged(0x8000, 0).detonates());
        assert!(flagged(0, 0x0001).ammo_at_burst_end());
        let hiding = WeaponSpec::new(&WeaponRecord {
            flags2: 0x0800,
            ..weapon(128)
        });
        assert!(hiding.hides_when_empty() && !plain.hides_when_empty());
        let others = WeaponSpec::new(&WeaponRecord {
            flags2: !0x0800,
            ..weapon(128)
        });
        assert!(!others.hides_when_empty());
        let all = flagged(!0, !0);
        assert!(all.secondary() && all.passes_shields() && all.spares_player());
        assert!(all.detonates() && all.ammo_at_burst_end());
        let others = flagged(!0x8122, !0x0001);
        assert!(!others.secondary() && !others.passes_shields());
        assert!(!others.spares_player() && !others.detonates());
        assert!(!others.ammo_at_burst_end());
    }

    #[test]
    fn the_other_fields_carry_over() {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 6,
            ammo_type: 10,
            explod_type: 1003,
            seeker: 0x0021,
            flags2: 0x8200,
            flags3: 0x0002,
            burst_count: 60,
            burst_reload: 30,
            ..weapon(138)
        });
        assert_eq!(spec.guidance, Guidance::Rocket);
        assert_eq!(spec.ammo, Ammo::Rounds(WeaponId(138)));
        assert_eq!(
            spec.explosion,
            Some(Explosion {
                boom: BoomId(131),
                extra: true
            })
        );
        assert_eq!((spec.seeker, spec.flags2, spec.flags3), (0x21, 0x8200, 2));
        assert_eq!((spec.burst_count, spec.burst_reload), (60, 30.0));
    }
}
