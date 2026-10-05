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
//! - `Guidance` -1, 0, 1, 3-10 are the [`Guidance`] this simulation
//!   flies; any other (2, or anything undocumented) is
//!   [`Guidance::Other`].
//! - `Guidance` 99 is a fighter bay ([`Guidance::FighterBay`]) when its
//!   `AmmoType` is a `shïp` ID, 128 or more (the Bible: "Carried ship
//!   (`AmmoType` is the ID of the ship class)"): it launches that ship
//!   ([`WeaponSpec::carried`]), and its rounds are its own, the fighters
//!   aboard, as the original spends the bay's own slot (`_WeaponHasAmmo`
//!   @0xba87-0xbaa1, `_FirePlayerWeapon` @0x62cd5-0x62cf0). Guidance 99
//!   with any other `AmmoType` is [`Guidance::Other`], and fires nothing
//!   (see [`bay`](crate::bay)).
//! - `MaxAmmo` is how many rounds each launcher holds at most, none at
//!   none or below (see [`bay::capacity`](crate::bay::capacity)).
//! - `GuidedTurn` is a homing shot's turn a tick in tenths of a degree
//!   ([`GUIDED_TURN_PER_DEGREE`]): the IR Missile's 70 is 7 degrees.
//! - `Durability` is what point defence must take off a shot before the
//!   next hit destroys it.
//! - `SubCount`, `SubType`, `SubTheta` and `SubLimit` are its
//!   [`Submunitions`]: none without a count above none and a type, and no
//!   limit at none or below.
//! - Its range is a beam's (0, 3 and 10) `BeamLength`, and any other's
//!   `Speed` times its `Count`. (The original adds the range of its
//!   sub-munitions' chain, which no stock point defence has.)
//! - `AmmoType` is decoded as [`Ammo`]: -1 unlimited, 0-255 the rounds
//!   of `wëap` 128+n, and -1000 and below fuel, |n + 1000| / 10 units a
//!   shot.
//! - `ExplodType`, and a `shïp`'s `Explode1` and `Explode2`, decode as an
//!   [`Explosion`]: 0-63 `bööm` 128+n, 1000-1063 the same with extra
//!   type-0 explosions around it, anything else none.
//! - Negative reloads, lives, radii, inaccuracies and bursts count as none.

use crate::bay::{FIGHTER_BAY, FIRST_SHIP};
use crate::catalog::{BoomId, ShipId, WeaponId, WeaponRecord};
use crate::handling::SPEED_PER_PIXEL_PER_TICK;

/// How a weapon's shots fly, from its `Guidance`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Guidance {
    /// -1: an unguided projectile, at constant velocity.
    Unguided,
    /// 0: a beam.
    Beam,
    /// 1: a homing shot, steering at its target.
    Homing,
    /// 3: a turreted beam, held on its target.
    TurretBeam,
    /// 4: a turret, firing at its target at the lead angle.
    Turret,
    /// 5: a freefall bomb, launched at 80% of the firer's velocity, that
    /// turns into the wind.
    FreefallBomb,
    /// 6: a freeflight rocket, launched straight ahead, that accelerates
    /// to its top speed.
    Rocket,
    /// 7: a front-quadrant turret: at a target within 45 degrees of the
    /// nose, otherwise straight ahead.
    FrontTurret,
    /// 8: a rear-quadrant turret: only at a target within 45 degrees of
    /// the tail.
    RearTurret,
    /// 9: a point-defence turret, firing at missiles.
    PointDefence,
    /// 10: a point-defence beam.
    PointDefenceBeam,
    /// 99 with a `shïp` ID for its `AmmoType`: a fighter bay, launching
    /// that ship (see [`bay`](crate::bay)). [`Guidance::decode`] never
    /// gives it, as it needs the `AmmoType`; [`WeaponSpec::new`] does.
    FighterBay,
    /// Any other: 99 without a ship to carry, and anything undocumented.
    Other(i16),
}

impl Guidance {
    /// The guidance `raw` means, 99 as [`Guidance::Other`] (see
    /// [`Guidance::FighterBay`]).
    #[must_use]
    pub fn decode(raw: i16) -> Self {
        match raw {
            -1 => Self::Unguided,
            0 => Self::Beam,
            1 => Self::Homing,
            3 => Self::TurretBeam,
            4 => Self::Turret,
            5 => Self::FreefallBomb,
            6 => Self::Rocket,
            7 => Self::FrontTurret,
            8 => Self::RearTurret,
            9 => Self::PointDefence,
            10 => Self::PointDefenceBeam,
            other => Self::Other(other),
        }
    }

    /// Whether it is a turret: guidance 3, 4, 7 or 8.
    #[must_use]
    pub fn turret(self) -> bool {
        matches!(
            self,
            Self::TurretBeam | Self::Turret | Self::FrontTurret | Self::RearTurret
        )
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

/// A weapon's sub-munitions, released by each of its shots on a hit and
/// at the end of its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submunitions {
    /// Their weapon, its `SubType`.
    pub weapon: WeaponId,
    /// How many each shot releases, its `SubCount`.
    pub count: u32,
    /// Their spread, its `SubTheta`, in degrees: within this either way,
    /// or a starburst this far apart when negative.
    pub theta: i16,
    /// How many generations there may be, its `SubLimit`; none for no
    /// limit.
    pub limit: Option<u32>,
}

impl Submunitions {
    /// The sub-munitions `record` releases: none without a `SubCount` or
    /// a `SubType`.
    fn of(record: &WeaponRecord) -> Option<Self> {
        let count = u32::try_from(record.sub_count).ok().filter(|&n| n > 0)?;
        Some(Self {
            weapon: record.sub_type?,
            count,
            theta: record.sub_theta,
            limit: u32::try_from(record.sub_limit).ok().filter(|&n| n > 0),
        })
    }

    /// Whether a shot of `generation` (0 for one a ship fired) releases
    /// them.
    #[must_use]
    pub fn releases_at(&self, generation: u32) -> bool {
        self.limit.is_none_or(|limit| generation < limit)
    }
}

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
    /// A homing shot's turn a tick, in degrees.
    pub turn: f32,
    /// What point defence must take off a shot of it before the next hit
    /// destroys it.
    pub durability: f32,
    /// Its sub-munitions, if any.
    pub submunitions: Option<Submunitions>,
    /// The ship a fighter bay launches; none for any other weapon.
    pub carried: Option<ShipId>,
    /// Its `MaxAmmo`: the rounds each launcher holds at most; none for no
    /// limit of its own.
    pub max_ammo: u32,
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
/// `Flags`: point defence cannot target its shots.
pub const PD_IMMUNE: u16 = 0x0080;
/// `Flags2`: a homing shot's proximity fuse is set off by other ships
/// than its target.
pub const PROXIMITY_BY_OTHERS: u16 = 0x0008;
/// `Flags2`: its sub-munitions are aimed at the nearest ship they can
/// hit.
pub const SUBS_SEEK_NEAREST: u16 = 0x0010;
/// `Flags2`: its shots release no sub-munitions at the end of their life.
pub const NO_SUBS_ON_EXPIRY: u16 = 0x0020;
/// `GuidedTurn` is in tenths of a degree.
pub const GUIDED_TURN_PER_DEGREE: f32 = 10.0;

impl WeaponSpec {
    /// The weapon `record` describes (see the module docs).
    #[must_use]
    pub fn new(record: &WeaponRecord) -> Self {
        let ticks = |raw: i16| f32::from(raw.max(0));
        let count = |raw: i16| u32::try_from(raw).unwrap_or(0);
        let carried = (record.guidance == FIGHTER_BAY && record.ammo_type >= FIRST_SHIP)
            .then_some(ShipId(record.ammo_type));
        let (guidance, ammo) = match carried {
            Some(_) => (Guidance::FighterBay, Ammo::Rounds(record.id)),
            None => (
                Guidance::decode(record.guidance),
                Ammo::decode(record.ammo_type),
            ),
        };
        Self {
            id: record.id,
            guidance,
            reload: ticks(record.reload),
            lifetime: count(record.count),
            mass_damage: f32::from(record.mass_dmg),
            energy_damage: f32::from(record.energy_dmg),
            speed: f32::from(record.speed) / SPEED_PER_PIXEL_PER_TICK,
            ammo,
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
            turn: f32::from(record.guided_turn) / GUIDED_TURN_PER_DEGREE,
            durability: ticks(record.durability),
            submunitions: Submunitions::of(record),
            carried,
            max_ammo: count(record.max_ammo),
        }
    }

    /// Whether it is a fighter bay ([`Guidance::FighterBay`]).
    #[must_use]
    pub fn is_bay(&self) -> bool {
        self.guidance == Guidance::FighterBay
    }

    /// Whether it fires beams: guidance 0, 3 and 10.
    #[must_use]
    pub fn is_beam(&self) -> bool {
        matches!(
            self.guidance,
            Guidance::Beam | Guidance::TurretBeam | Guidance::PointDefenceBeam
        )
    }

    /// Whether point defence cannot target its shots.
    #[must_use]
    pub fn pd_immune(&self) -> bool {
        self.flags & PD_IMMUNE != 0
    }

    /// Whether its sub-munitions are aimed at the nearest ship they can
    /// hit.
    #[must_use]
    pub fn seeks_with_subs(&self) -> bool {
        self.flags2 & SUBS_SEEK_NEAREST != 0
    }

    /// Whether its shots release their sub-munitions at the end of their
    /// life.
    #[must_use]
    pub fn subs_on_expiry(&self) -> bool {
        self.flags2 & NO_SUBS_ON_EXPIRY == 0
    }

    /// Whether a homing shot of it is set off by other ships than its
    /// target.
    #[must_use]
    pub fn proximity_by_others(&self) -> bool {
        self.flags2 & PROXIMITY_BY_OTHERS != 0
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
        if self.is_beam() {
            self.beam_length
        } else {
            self.speed * self.lifetime as f32
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
            (1, Guidance::Homing),
            (2, Guidance::Other(2)),
            (3, Guidance::TurretBeam),
            (4, Guidance::Turret),
            (5, Guidance::FreefallBomb),
            (6, Guidance::Rocket),
            (7, Guidance::FrontTurret),
            (8, Guidance::RearTurret),
            (9, Guidance::PointDefence),
            (10, Guidance::PointDefenceBeam),
            (11, Guidance::Other(11)),
            (99, Guidance::Other(99)),
            (-2, Guidance::Other(-2)),
        ] {
            assert_eq!(Guidance::decode(raw), guidance, "{raw}");
        }
    }

    #[test]
    fn beams_are_guidance_0_3_and_10_and_reach_their_length() {
        for raw in [-1, 1, 2, 4, 5, 6, 7, 8, 9, 99] {
            let spec = WeaponSpec::new(&WeaponRecord {
                guidance: raw,
                count: 10,
                speed: 300,
                beam_length: 400,
                ..weapon(146)
            });
            assert!(!spec.is_beam(), "{raw}");
            assert_eq!(spec.range(), 30.0, "{raw}");
        }
        for raw in [0, 3, 10] {
            let spec = WeaponSpec::new(&WeaponRecord {
                guidance: raw,
                count: 10,
                speed: 300,
                beam_length: 400,
                ..weapon(146)
            });
            assert!(spec.is_beam(), "{raw}");
            assert_eq!(spec.range(), 400.0, "{raw}");
        }
    }

    #[test]
    fn a_guided_turn_of_70_is_7_degrees_a_tick() {
        let spec = WeaponSpec::new(&WeaponRecord {
            guided_turn: 70,
            durability: 4,
            ..weapon(134)
        });
        assert_eq!(spec.turn, 7.0);
        assert_eq!(spec.durability, 4.0);
        assert_eq!(GUIDED_TURN_PER_DEGREE, 10.0);
        let unsteered = WeaponSpec::new(&WeaponRecord {
            guided_turn: 25,
            durability: -1,
            ..weapon(134)
        });
        assert_eq!(unsteered.turn, 2.5);
        assert_eq!(unsteered.durability, 0.0, "none below none");
    }

    #[test]
    fn sub_munitions_need_a_count_and_a_type() {
        let polaron = |sub_count, sub_type: Option<i16>| {
            WeaponSpec::new(&WeaponRecord {
                sub_count,
                sub_type: sub_type.map(WeaponId),
                sub_theta: 45,
                sub_limit: 2,
                ..weapon(182)
            })
            .submunitions
        };
        assert_eq!(
            polaron(5, Some(148)),
            Some(Submunitions {
                weapon: WeaponId(148),
                count: 5,
                theta: 45,
                limit: Some(2)
            })
        );
        assert_eq!(polaron(0, Some(148)), None, "none of them");
        assert_eq!(polaron(-1, Some(148)), None, "unused");
        assert_eq!(polaron(1, None), None, "no type");
        assert_eq!(polaron(1, Some(148)).map(|subs| subs.count), Some(1));
        let unlimited = |sub_limit| {
            WeaponSpec::new(&WeaponRecord {
                sub_count: 1,
                sub_type: Some(WeaponId(229)),
                sub_limit,
                ..weapon(163)
            })
            .submunitions
            .map(|subs| subs.limit)
        };
        assert_eq!(unlimited(0), Some(None));
        assert_eq!(unlimited(-1), Some(None));
        assert_eq!(unlimited(1), Some(Some(1)));
    }

    #[test]
    fn sub_munitions_are_released_by_generations_below_their_limit() {
        let subs = |limit| Submunitions {
            weapon: WeaponId(148),
            count: 1,
            theta: 0,
            limit,
        };
        assert!(subs(None).releases_at(0));
        assert!(subs(None).releases_at(1_000));
        assert!(subs(Some(2)).releases_at(0));
        assert!(subs(Some(2)).releases_at(1));
        assert!(!subs(Some(2)).releases_at(2));
        assert!(!subs(Some(2)).releases_at(3));
    }

    #[test]
    fn the_guided_flags_say_what_they_do() {
        let flagged = |flags, flags2| {
            WeaponSpec::new(&WeaponRecord {
                flags,
                flags2,
                ..weapon(148)
            })
        };
        let plain = flagged(0, 0);
        assert!(!plain.pd_immune() && !plain.seeks_with_subs());
        assert!(plain.subs_on_expiry() && !plain.proximity_by_others());
        assert!(flagged(0x0080, 0).pd_immune());
        assert!(flagged(0, 0x0010).seeks_with_subs());
        assert!(!flagged(0, 0x0020).subs_on_expiry());
        assert!(flagged(0, 0x0008).proximity_by_others());
        let all = flagged(!0, !0);
        assert!(all.pd_immune() && all.seeks_with_subs());
        assert!(!all.subs_on_expiry() && all.proximity_by_others());
        let others = flagged(!0x0080, !0x0038);
        assert!(!others.pd_immune() && !others.seeks_with_subs());
        assert!(others.subs_on_expiry() && !others.proximity_by_others());
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
        assert_eq!((spec.carried, spec.max_ammo), (None, 0));
        assert!(!spec.is_bay());
    }

    /// A Viper Bay: guidance 99 carrying `AmmoType` `ammo_type`, of
    /// `MaxAmmo` `max_ammo`.
    fn bay(ammo_type: i16, max_ammo: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 99,
            ammo_type,
            max_ammo,
            reload: 60,
            ..weapon(149)
        })
    }

    #[test]
    fn guidance_99_with_a_ship_id_is_a_fighter_bay_firing_its_own_rounds() {
        let viper = bay(144, 4);
        assert_eq!(viper.guidance, Guidance::FighterBay);
        assert_eq!(viper.carried, Some(ShipId(144)));
        assert_eq!(viper.ammo, Ammo::Rounds(WeaponId(149)));
        assert_eq!(viper.max_ammo, 4);
        assert!(viper.is_bay());
        assert_eq!(bay(128, 0).carried, Some(ShipId(128)), "the first ship");
        assert_eq!(bay(i16::MAX, 0).carried, Some(ShipId(i16::MAX)));
    }

    #[test]
    fn guidance_99_without_a_ship_id_still_fires_nothing() {
        for (ammo_type, ammo) in [
            (12, Ammo::Rounds(WeaponId(140))),
            (127, Ammo::Rounds(WeaponId(255))),
            (-1, Ammo::Unlimited),
        ] {
            let spec = bay(ammo_type, 4);
            assert_eq!(spec.guidance, Guidance::Other(99), "{ammo_type}");
            assert_eq!(spec.carried, None, "{ammo_type}");
            assert_eq!(spec.ammo, ammo, "{ammo_type}");
            assert!(!spec.is_bay(), "{ammo_type}");
        }
        let other = WeaponSpec::new(&WeaponRecord {
            guidance: 4,
            ammo_type: 144,
            ..weapon(150)
        });
        assert_eq!(other.carried, None, "only a bay carries ships");
        assert!(!other.is_bay());
    }

    #[test]
    fn a_max_ammo_of_none_or_below_is_none() {
        assert_eq!(bay(144, -1).max_ammo, 0);
        assert_eq!(bay(144, 0).max_ammo, 0);
        assert_eq!(bay(144, 2).max_ammo, 2);
    }
}
