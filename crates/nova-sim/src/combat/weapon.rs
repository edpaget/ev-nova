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
//!   flies, and 2 as 1 (see below); any other (anything undocumented) is
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
//! - Its range ([`WeaponSpec::range`]) is a beam's (0, 3 and 10)
//!   `BeamLength`; for guidance -1, 1, 4, 6, 7, 8 and 9, its `Speed` times
//!   its `Count` and its sub-munitions' down their chain
//!   ([`chain_range`]), so the Nanites (163) reach 2437.5 pixels and the
//!   Polaron Multi-Torpedoes (182, 185) 3510; and nothing for a bomb (5),
//!   a bay (99) or anything else. How far a shot itself flies is
//!   [`WeaponSpec::flight`].
//! - Guidance 2, which the Bible lists unused, flies as homing (1) with
//!   its `Seeker` replaced by 0x30fe (@0x79b9f-0x79bb4).
//! - `AmmoType` is decoded as [`Ammo`]: -1 unlimited, 0-255 the rounds
//!   of `wëap` 128+n, and -1000 and below fuel, |n + 1000| / 10 units a
//!   shot.
//! - `ExplodType`, and a `shïp`'s `Explode1` and `Explode2`, decode as an
//!   [`Explosion`]: 0-63 `bööm` 128+n, 1000-1063 the same with extra
//!   type-0 explosions around it, anything else none.
//! - Negative reloads, lives, radii, inaccuracies and bursts count as none.

use std::collections::{BTreeMap, BTreeSet};

use super::exits::ExitKind;
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
            // -1 is no spread (@0x796c6-0x796cd).
            theta: if record.sub_theta == NO_SPREAD_THETA {
                0
            } else {
                record.sub_theta
            },
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

/// The `SubTheta` that `_LoadObjectData` reads as no spread at all
/// (@0x796c6-0x796cd), where any other negative is a starburst.
pub const NO_SPREAD_THETA: i16 = -1;

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
    /// Its `AmmoType`, raw, as the plunder compares it
    /// ([`board::player_fires`](crate::board::player_fires)).
    pub ammo_type: i16,
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
    /// Its `Falloff` as the original loads it ([`loaded_falloff`]): how
    /// fast a beam's corona fades, and how long a decaying beam fades for
    /// (see [`beam`](super::beam)).
    pub falloff: i16,
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
    /// How far it reaches, in pixels, down its sub-munition chain, as the
    /// AI reads it ([`WeaponSpec::range`]): its own reach by
    /// [`WeaponSpec::new`], the whole chain's once an
    /// [`Arsenal`](super::armament::Arsenal) has read every weapon
    /// ([`chain_range`]).
    pub chain_range: f32,
    /// Its `ProxSafety`, none when that is below none: a shot of it hits
    /// no ship by contact until it has flown this many ticks, and none by
    /// its proximity fuse, nor a stellar, until it has flown more
    /// ([`Shot::hit`](super::projectile::Shot::hit)).
    pub prox_safety: u32,
    /// Its `Decay`, none when that is at or below none: a shot of it loses
    /// a point of mass and of energy damage from its direct hit for each
    /// `Decay` + 1 ticks it has flown, by the original's reading
    /// ([`DecayStep`](super::projectile::DecayStep)).
    pub decay: u32,
    /// The weapon whose rounds the secondary weapon line counts for it,
    /// if it counts any ([`line_count`]).
    pub line_count: Option<WeaponId>,
    /// Its `Recoil`, raw: the push its firer takes away from its heading,
    /// over the firer's mass, when it [recoils](WeaponSpec::recoils).
    pub recoil: f32,
    /// Its `Impact`, raw (the original's weapon +0x10): the push a ship it
    /// hits, blasts or holds in its beam takes, over that ship's mass (see
    /// [`impact`](super::impact)); a negative one pulls, and a beam's is
    /// a tractor's. It is never decayed, reduced by blast falloff or
    /// doubled in double time.
    pub impact: f32,
    /// Its `Ionization`, raw and widened: the ion charge a direct hit or a
    /// beam tick adds to the ship it lands on, and a blast scaled over
    /// its circle ([`ion`](super::ion)); never decayed or doubled.
    pub ionization: i32,
    /// Its `IonizeColor`, `00RRGGBB`: the colour it ORs into the ion
    /// colour of a ship it ionizes ([`ion`](super::ion)).
    pub ionize_colour: u32,
    /// The exit points it fires from, by its `ExitType`; none, the
    /// centre, for any but 0-3 (see [`exits`](super::exits)).
    pub exit: Option<ExitKind>,
    /// Whether it fires from the exit point nearest its target: its
    /// `Flags3` [`NEAREST_EXIT`].
    pub nearest_exit: bool,
    /// How far off its firer's heading, in degrees, it flies out to the
    /// side of the exit it leaves from: its `Inaccuracy` made positive
    /// when that is below 0, which then draws no inaccuracy (`_SpawnShot`
    /// @0x3ec2d-0x3ec95, @0x3ee10-0x3eec0); none otherwise.
    pub broadside: u32,
}

/// The weapon whose rounds the status bar's secondary weapon line counts
/// for `record`, as `_DrawStatusWeap` (@0x4c00a) picks it: none, and the
/// line is the name alone, when its `AmmoType` is -1, its `Flags2` has
/// [`HIDES_COUNT`], or its `AmmoType` is [`FUEL_AMMO`] or below
/// (@0x4c180-0x4c19d); the weapon's own when its `AmmoType` is above 255
/// read unsigned (so -999 to -2 too) or its `Guidance` is 99, a fighter
/// bay's (@0x4c1e7-0x4c1f2); otherwise its ammunition's, `wëap`
/// 128 + `AmmoType` (@0x4c1f4).
#[must_use]
pub fn line_count(record: &WeaponRecord) -> Option<WeaponId> {
    let ammo = record.ammo_type;
    if ammo == UNLIMITED_AMMO || record.flags2 & HIDES_COUNT != 0 || ammo <= FUEL_AMMO {
        None
    } else if ammo.cast_unsigned() > LAST_AMMO_TYPE.cast_unsigned()
        || record.guidance == FIGHTER_BAY
    {
        Some(record.id)
    } else {
        Some(WeaponId(FIRST_AMMO_WEAPON + ammo))
    }
}

/// The corona falloff a beam of none or less loads with (`_LoadObjectData`
/// @0x795e2-0x795f0).
pub const DEFAULT_FALLOFF: i16 = 16;

/// A weapon's `Falloff` as `_LoadObjectData` loads it from the `wëap`'s
/// `falloff`, `beam_length` and `li_density` (@0x795e2-0x79645): a beam
/// (a `BeamLength` above none) of a `Falloff` of none or less takes
/// [`DEFAULT_FALLOFF`], and a lightning beam (a `LiDensity` above none)
/// none at all, whatever it says. Any other weapon's is kept. The view
/// draws a beam's corona from it too.
#[must_use]
pub fn loaded_falloff(beam_length: i16, falloff: i16, li_density: i16) -> i16 {
    if beam_length <= 0 {
        falloff
    } else if li_density > 0 {
        0
    } else if falloff <= 0 {
        DEFAULT_FALLOFF
    } else {
        falloff
    }
}

/// `Flags`: fired by the second trigger.
pub const SECONDARY: u16 = 0x0002;
/// `Flags`: passes through shields.
pub const PASSES_SHIELDS: u16 = 0x0020;
/// `Flags`: its blast does not hurt the player.
pub const SPARES_PLAYER: u16 = 0x0100;
/// `Flags`: its shots detonate at the end of their life.
pub const DETONATES: u16 = 0x8000;
/// `Flags`: its copies fire together, one shot each a call, and it
/// reloads by its whole `Reload` (`_WeaponMaxSimultShots` @0x8350,
/// `_FireAIShipWeapon` @0x88f85).
pub const SIMULTANEOUS: u16 = 0x0040;
/// `Flags3`: exclusive: firing it holds every other weapon's reload at
/// its own plus [`EXCLUSIVE_HOLD`] (`_FireAIShipWeapon` @0x891c8-0x8922a).
pub const EXCLUSIVE: u16 = 0x0020;
/// The ticks past an exclusive weapon's reload every other weapon is held
/// (2.0 @0xdd074).
pub const EXCLUSIVE_HOLD: f32 = 2.0;
/// `Flags3`: it fires from its ship's exit point nearest the target
/// (`_ModifyShotStartPosition` @0x73d9-0x742c, `_SpawnBeam`
/// @0x45126-0x451af; see [`exits`](super::exits)).
pub const NEAREST_EXIT: u16 = 0x0010;
/// `Flags3`: it uses ammo only at the end of a burst.
pub const AMMO_AT_BURST_END: u16 = 0x0001;
/// `Flags3`: one shot at a time: while a shot of it is in flight, its
/// firer's reload for it is held at its `Reload` (`_HandleShot`
/// @0x35c09-0x35c42).
pub const ONE_AT_A_TIME: u16 = 0x0004;
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
/// `Flags2`: a disable-only weapon, whose shots, blasts and beams never
/// destroy a ship (`_WeaponDisableOnly` @0x704c; see
/// [`Hit::cant_kill`](super::damage::Hit::cant_kill)).
pub const DISABLE_ONLY: u16 = 0x1000;
/// `Flags2`: a planet-type weapon, whose shots can hit a destroyable
/// stellar (see [`combat`](super)'s "Stellars").
pub const PLANET_TYPE: u16 = 0x0400;
/// `Flags`: a guided weapon that won't fire at fast ships: an NPC holds it
/// at a target turning more than 3 degrees a tick (`_SuitableMissileType`
/// @0x3507-0x3516, see [`fire`](crate::ai::fire)).
pub const AVOIDS_FAST_SHIPS: u16 = 0x0008;
/// `Flags2`: it needs its ship's `KeyCarried` aboard, a fighter bay of
/// that ship holding rounds (`_WeaponHasAmmo` @0xba07-0xba85).
pub const NEEDS_KEY_SHIP: u16 = 0x0080;
/// `Flags2`: AI ships won't use it (`_WeaponHasAmmo` @0xb963-0xb993).
pub const AI_WONT_USE: u16 = 0x0100;
/// `Flags2`: it fires while its ship is cloaked, and no other weapon does
/// (`_WeaponHasAmmo` @0xb9a2-0xba01).
pub const FIRES_CLOAKED: u16 = 0x4000;
/// `Flags2`: the secondary weapon's line shows no count of its rounds
/// (`_DrawStatusWeap` @0x4c190).
pub const HIDES_COUNT: u16 = 0x0040;
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
                Guidance::decode(effective_guidance(record.guidance)),
                Ammo::decode(record.ammo_type),
            ),
        };
        // Guidance 2 flies as homing with this `Seeker` (@0x79b9f-0x79bb4).
        let seeker = if record.guidance == OLD_HOMING {
            OLD_HOMING_SEEKER
        } else {
            record.seeker
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
            ammo_type: record.ammo_type,
            inaccuracy: count(record.inaccuracy),
            explosion: Explosion::decode(record.explod_type),
            prox_radius: ticks(record.prox_radius),
            blast_radius: ticks(record.blast_radius),
            beam_length: ticks(record.beam_length),
            falloff: loaded_falloff(record.beam_length, record.falloff, record.li_density),
            burst_count: count(record.burst_count),
            burst_reload: ticks(record.burst_reload),
            flags: record.flags,
            seeker,
            flags2: record.flags2,
            flags3: record.flags3,
            turn: f32::from(record.guided_turn) / GUIDED_TURN_PER_DEGREE,
            durability: ticks(record.durability),
            submunitions: Submunitions::of(record),
            carried,
            max_ammo: count(record.max_ammo),
            chain_range: own_reach(record),
            prox_safety: count(record.prox_safety),
            decay: count(record.decay),
            line_count: line_count(record),
            recoil: f32::from(record.recoil),
            impact: f32::from(record.impact),
            ionization: i32::from(record.ionization),
            ionize_colour: record.ionize_color,
            exit: ExitKind::from_raw(record.exit_type),
            nearest_exit: record.flags3 & NEAREST_EXIT != 0,
            broadside: if record.inaccuracy < 0 {
                u32::from(record.inaccuracy.unsigned_abs())
            } else {
                0
            },
        }
    }

    /// Whether it is a fighter bay ([`Guidance::FighterBay`]).
    #[must_use]
    pub fn is_bay(&self) -> bool {
        self.guidance == Guidance::FighterBay
    }

    /// Whether its raw guidance is 99: a fighter bay, or a guidance-99
    /// weapon with no ship to carry ([`Guidance::Other`]`(99)`), each
    /// paying from its own rounds (`_WeaponHasAmmo` @0xba87-0xbaa1).
    #[must_use]
    pub fn guidance_99(&self) -> bool {
        matches!(
            self.guidance,
            Guidance::FighterBay | Guidance::Other(FIGHTER_BAY)
        )
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

    /// Whether it fires one shot at a time ([`ONE_AT_A_TIME`]).
    #[must_use]
    pub fn one_at_a_time(&self) -> bool {
        self.flags3 & ONE_AT_A_TIME != 0
    }

    /// Whether it is a planet-type weapon (`Flags2` 0x0400), whose shots
    /// can hit a destroyable stellar.
    #[must_use]
    pub fn planet_type(&self) -> bool {
        self.flags2 & PLANET_TYPE != 0
    }

    /// Whether it is disable-only (`Flags2` 0x1000): its shots, blasts
    /// and beams never destroy a ship.
    #[must_use]
    pub fn disable_only(&self) -> bool {
        self.flags2 & DISABLE_ONLY != 0
    }

    /// Whether it is hidden while out of ammo.
    #[must_use]
    pub fn hides_when_empty(&self) -> bool {
        self.flags2 & HIDE_WHEN_EMPTY != 0
    }

    /// How far it reaches, in pixels, as the AI reads it (the original's
    /// +0x5c, `_LoadObjectData` @0x79e8c-0x79fa6): a beam (0, 3 and 10) its
    /// length; a projectile of guidance -1, 1, 4, 6, 7, 8 or 9 its flight
    /// and its sub-munitions' down their chain ([`chain_range`]); a bomb,
    /// a bay and any other guidance nothing.
    #[must_use]
    pub fn range(&self) -> f32 {
        self.chain_range
    }

    /// Whether it pushes its firer back as it fires: a `Recoil` of neither
    /// 0 nor -1 (`_FireAIShipWeapon` @0x88ee2-0x88efc).
    #[must_use]
    pub fn recoils(&self) -> bool {
        self.recoil != 0.0 && self.recoil != NO_RECOIL
    }

    /// How far a shot of it flies, in pixels: its speed for its life, from
    /// a resting firer at its top speed, whatever its guidance.
    #[must_use]
    pub fn flight(&self) -> f32 {
        self.speed * self.lifetime as f32
    }
}

/// The `Recoil` that, like 0, pushes its firer not at all.
pub const NO_RECOIL: f32 = -1.0;

/// The guidance that becomes homing, with [`OLD_HOMING_SEEKER`]
/// (@0x79b9f-0x79bb4); the Bible lists it unused.
pub const OLD_HOMING: i16 = 2;
/// The `Seeker` a weapon of guidance [`OLD_HOMING`] takes in place of its
/// own.
pub const OLD_HOMING_SEEKER: u16 = 0x30fe;

/// The guidance `raw` flies as: [`OLD_HOMING`] as homing (1), any other as
/// it is.
fn effective_guidance(raw: i16) -> i16 {
    if raw == OLD_HOMING { 1 } else { raw }
}

/// Whether a weapon of raw `guidance` reaches down its sub-munition chain:
/// -1, 1, 4, 6, 7, 8 and 9 (guidance 2 counting as 1).
fn chains(guidance: i16) -> bool {
    matches!(effective_guidance(guidance), -1 | 1 | 4 | 6 | 7 | 8 | 9)
}

/// Whether a weapon of raw `guidance` reaches its `BeamLength`: 0, 3 and
/// 10.
fn beams(guidance: i16) -> bool {
    matches!(guidance, 0 | 3 | 10)
}

/// How far a shot of `record` flies, raw: `Count` x `Speed`, as the range
/// sums it.
fn raw_flight(record: &WeaponRecord) -> f32 {
    f32::from(record.count) * f32::from(record.speed) / SPEED_PER_PIXEL_PER_TICK
}

/// `record`'s own reach, without its sub-munitions (see
/// [`WeaponSpec::range`]).
fn own_reach(record: &WeaponRecord) -> f32 {
    if chains(record.guidance) {
        raw_flight(record)
    } else if beams(record.guidance) {
        f32::from(record.beam_length)
    } else {
        0.0
    }
}

/// How far weapon `id` among `records` reaches (see [`WeaponSpec::range`]),
/// as `_LoadObjectData` sums it (@0x79e8c-0x79fa6): a beam its
/// `BeamLength`; a weapon of guidance -1, 1, 4, 6, 7, 8 or 9 its `Count` x
/// `Speed`, then, while the weapon reached has a `SubCount` above none,
/// its sub-munition's in turn, whatever that one's guidance; a weapon
/// whose `SubType` is itself adds `SubLimit` more of its own flights, when
/// that is above none, and the chain ends there. A chain ends at a
/// `SubType` it cannot read or outside 128-383 (-1 to the original,
/// @0x796d5-0x796ee). One that comes back round to a weapon it passed
/// (A to B to A), on which the original would loop for ever, ends there
/// instead. None for a weapon that cannot be read, a bomb, a bay or any
/// other guidance.
#[must_use]
pub fn chain_range(records: &BTreeMap<WeaponId, WeaponRecord>, id: WeaponId) -> f32 {
    let Some(root) = records.get(&id) else {
        return 0.0;
    };
    if !chains(root.guidance) {
        return own_reach(root);
    }
    let mut reach = 0.0;
    let mut passed = BTreeSet::new();
    let mut at = root;
    loop {
        passed.insert(at.id);
        reach += raw_flight(at);
        if at.sub_count <= 0 {
            break;
        }
        let Some(sub) = at.sub_type.filter(|sub| {
            (FIRST_AMMO_WEAPON..=FIRST_AMMO_WEAPON + LAST_AMMO_TYPE).contains(&sub.0)
        }) else {
            break;
        };
        if sub == at.id {
            if at.sub_limit > 0 {
                reach += f32::from(at.sub_limit) * raw_flight(at);
            }
            break;
        }
        match records.get(&sub) {
            Some(next) if !passed.contains(&next.id) => at = next,
            _ => break,
        }
    }
    reach
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

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
    fn a_weapon_reads_its_exit_type_nearest_exit_flag_and_broadside() {
        for (raw, exit) in [
            (0, Some(ExitKind::Gun)),
            (1, Some(ExitKind::Turret)),
            (2, Some(ExitKind::Guided)),
            (3, Some(ExitKind::Beam)),
            (-1, None),
            (4, None),
        ] {
            let spec = WeaponSpec::new(&WeaponRecord {
                exit_type: raw,
                ..light_blaster()
            });
            assert_eq!(spec.exit, exit, "{raw}");
        }
        assert_eq!(NEAREST_EXIT, 0x0010);
        for (flags3, nearest) in [(0x0010, true), (0x0030, true), (0x0020, false), (0, false)] {
            let spec = WeaponSpec::new(&WeaponRecord {
                flags3,
                ..light_blaster()
            });
            assert_eq!(spec.nearest_exit, nearest, "{flags3:#x}");
        }
        for (inaccuracy, aimed, broadside) in
            [(9, 9, 0), (0, 0, 0), (-90, 0, 90), (i16::MIN, 0, 32768)]
        {
            let spec = WeaponSpec::new(&WeaponRecord {
                inaccuracy,
                ..light_blaster()
            });
            assert_eq!(
                (spec.inaccuracy, spec.broadside),
                (aimed, broadside),
                "{inaccuracy}"
            );
        }
    }

    #[test]
    fn the_light_blasters_range_is_195_pixels() {
        assert_eq!(WeaponSpec::new(&light_blaster()).range(), 195.0);
    }

    #[test]
    fn a_weapon_recoils_unless_its_recoil_is_0_or_minus_1() {
        for (raw, recoils) in [
            (0, false),
            (-1, false),
            (1, true),
            (-2, true),
            (300, true),
            (i16::MIN, true),
        ] {
            let spec = WeaponSpec::new(&WeaponRecord {
                recoil: raw,
                ..light_blaster()
            });
            assert_eq!(spec.recoil, f32::from(raw), "{raw}");
            assert_eq!(spec.recoils(), recoils, "{raw}");
        }
    }

    #[test]
    fn a_weapons_impact_is_its_raw_impact_either_way() {
        for raw in [250, -5, 0] {
            let spec = WeaponSpec::new(&WeaponRecord {
                impact: raw,
                ..light_blaster()
            });
            assert_eq!(spec.impact, f32::from(raw), "{raw}");
        }
    }

    #[test]
    fn a_weapons_ionization_and_ion_colour_are_its_raw_ones() {
        for (raw, colour) in [(100, 0x00d4_ffff), (-5, 0x0034_c2ff), (0, 0)] {
            let spec = WeaponSpec::new(&WeaponRecord {
                ionization: raw,
                ionize_color: colour,
                ..light_blaster()
            });
            assert_eq!(
                (spec.ionization, spec.ionize_colour),
                (i32::from(raw), colour),
                "{raw}"
            );
        }
    }

    #[test]
    fn a_weapons_decay_is_its_decay_none_at_or_below_none() {
        for (raw, ticks) in [(4, 4), (1, 1), (0, 0), (-3, 0)] {
            let record = WeaponRecord {
                decay: raw,
                ..light_blaster()
            };
            assert_eq!(WeaponSpec::new(&record).decay, ticks, "{raw}");
        }
    }

    #[test]
    fn a_weapons_proximity_safety_is_its_prox_safety_in_ticks_none_below_none() {
        for (raw, ticks) in [(8, 8), (0, 0), (-5, 0), (i16::MAX, 32_767)] {
            let record = WeaponRecord {
                prox_safety: raw,
                ..light_blaster()
            };
            assert_eq!(WeaponSpec::new(&record).prox_safety, ticks, "{raw}");
        }
    }

    #[test]
    fn a_disable_only_weapon_has_flags2_0x1000_whatever_its_other_bits() {
        for (flags2, disable_only) in [
            (0x1000, true),
            (0x1200, true),
            (0xffff, true),
            (0, false),
            (!0x1000, false),
        ] {
            let record = WeaponRecord {
                flags2,
                ..light_blaster()
            };
            assert_eq!(
                WeaponSpec::new(&record).disable_only(),
                disable_only,
                "{flags2:#x}"
            );
        }
    }

    #[test]
    fn a_planet_type_weapon_has_flags2_0x0400_whatever_its_other_bits() {
        for (flags2, planet) in [
            (0x0400, true),
            (0xffff, true),
            (0x0408, true),
            (0, false),
            (!0x0400, false),
        ] {
            let record = WeaponRecord {
                flags2,
                ..light_blaster()
            };
            assert_eq!(
                WeaponSpec::new(&record).planet_type(),
                planet,
                "{flags2:#x}"
            );
        }
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
    fn a_beams_falloff_loads_as_the_original_normalises_it() {
        let falloff = |beam_length, falloff, li_density| {
            WeaponSpec::new(&WeaponRecord {
                guidance: 0,
                beam_length,
                falloff,
                li_density,
                ..weapon(146)
            })
            .falloff
        };
        assert_eq!(falloff(400, 0, 0), 16, "none is 16");
        assert_eq!(falloff(400, -3, 0), 16);
        assert_eq!(falloff(400, 1, 0), 1);
        assert_eq!(falloff(400, 20, 0), 20, "kept above 16");
        assert_eq!(falloff(400, 8, 3), 0, "a lightning beam has none");
        assert_eq!(falloff(400, 0, 1), 0);
        assert_eq!(falloff(0, 0, 0), 0, "a shot's is kept");
        assert_eq!(falloff(0, 8, 3), 8);
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
        for raw in [-1, 1, 2, 4, 6, 7, 8, 9] {
            let spec = WeaponSpec::new(&WeaponRecord {
                guidance: raw,
                count: 10,
                speed: 300,
                beam_length: 400,
                ..weapon(146)
            });
            assert!(!spec.is_beam(), "{raw}");
            assert_eq!(spec.range(), 30.0, "{raw}");
            assert_eq!(spec.flight(), 30.0, "{raw}");
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
    fn bombs_bays_and_undocumented_guidances_reach_nothing() {
        for (raw, ammo_type) in [(5, -1), (99, 144), (99, 5), (11, -1), (-2, -1)] {
            let spec = WeaponSpec::new(&WeaponRecord {
                guidance: raw,
                ammo_type,
                count: 10,
                speed: 300,
                beam_length: 400,
                ..weapon(146)
            });
            assert_eq!(spec.range(), 0.0, "{raw} {ammo_type}");
            assert_eq!(spec.flight(), 30.0, "{raw} {ammo_type}: it still flies");
        }
    }

    #[test]
    fn guidance_2_is_homing_with_the_seeker_0x30fe() {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 2,
            seeker: 0x0001,
            count: 10,
            speed: 300,
            ..weapon(146)
        });
        assert_eq!(spec.guidance, Guidance::Homing);
        assert_eq!(spec.seeker, 0x30fe, "replaced, not added to");
        assert_eq!(spec.range(), 30.0);
        assert_eq!(Guidance::decode(2), Guidance::Other(2), "raw");
    }

    /// Weapon `id` flying `count` ticks at `speed` hundredths of a pixel
    /// a tick, of `guidance`, releasing `sub_count` of `sub_type` up to
    /// `sub_limit` generations.
    fn chained(
        id: i16,
        (guidance, count, speed): (i16, i16, i16),
        (sub_count, sub_type, sub_limit): (i16, Option<i16>, i16),
    ) -> WeaponRecord {
        WeaponRecord {
            guidance,
            count,
            speed,
            sub_count,
            sub_type: sub_type.map(WeaponId),
            sub_limit,
            ..weapon(id)
        }
    }

    fn range_of(records: &[WeaponRecord], id: i16) -> f32 {
        let records: BTreeMap<WeaponId, WeaponRecord> =
            records.iter().map(|record| (record.id, *record)).collect();
        chain_range(&records, WeaponId(id))
    }

    #[test]
    fn a_weapons_range_runs_down_its_sub_munition_chain() {
        let a = chained(140, (-1, 10, 200), (1, Some(141), 0));
        let b = chained(141, (-1, 30, 300), (0, None, 0));
        assert_eq!(range_of(&[a, b], 140), 110.0, "20 and 90");
        let c = chained(142, (1, 4, 100), (3, Some(140), 0));
        assert_eq!(range_of(&[a, b, c], 142), 114.0, "4, 20 and 90");
        assert_eq!(range_of(&[a, b], 141), 90.0);
    }

    #[test]
    fn a_self_sub_munition_adds_its_sub_limit_flights_and_stops() {
        let nanites = chained(163, (4, 10, 200), (1, Some(163), 3));
        assert_eq!(range_of(&[nanites], 163), 80.0, "its own 20, and 3 more");
    }

    #[test]
    fn a_self_sub_munition_of_no_limit_adds_nothing_more() {
        for limit in [0, -1] {
            let nanites = chained(163, (4, 10, 200), (1, Some(163), limit));
            assert_eq!(range_of(&[nanites], 163), 20.0, "{limit}");
        }
    }

    #[test]
    fn a_chain_ends_at_a_sub_munition_of_no_count() {
        let a = chained(140, (-1, 10, 200), (0, Some(141), 0));
        let b = chained(141, (-1, 30, 300), (0, None, 0));
        assert_eq!(range_of(&[a, b], 140), 20.0, "no count: no sub");
        let a = chained(140, (-1, 10, 200), (-1, Some(141), 0));
        assert_eq!(range_of(&[a, b], 140), 20.0);
        let lost = chained(140, (-1, 10, 200), (1, Some(150), 0));
        assert_eq!(range_of(&[lost], 140), 20.0, "a sub that cannot be read");
        let odd = chained(140, (-1, 10, 200), (1, Some(100), 0));
        assert_eq!(range_of(&[odd], 140), 20.0, "a sub outside 128-383");
    }

    #[test]
    fn a_sub_munitions_guidance_does_not_change_what_it_adds() {
        let a = chained(140, (1, 10, 200), (1, Some(141), 0));
        let beam = WeaponRecord {
            beam_length: 500,
            ..chained(141, (0, 30, 300), (0, None, 0))
        };
        assert_eq!(range_of(&[a, beam], 140), 110.0, "Count x Speed, not 500");
        let bay = chained(141, (99, 30, 300), (0, None, 0));
        assert_eq!(range_of(&[a, bay], 140), 110.0);
    }

    #[test]
    fn a_beam_reaches_its_length_whatever_its_sub_munitions() {
        let beam = WeaponRecord {
            beam_length: 500,
            ..chained(140, (3, 10, 200), (1, Some(141), 0))
        };
        let b = chained(141, (-1, 30, 300), (0, None, 0));
        assert_eq!(range_of(&[beam, b], 140), 500.0);
        let bomb = chained(140, (5, 10, 200), (1, Some(141), 0));
        assert_eq!(range_of(&[bomb, b], 140), 0.0, "a bomb reaches nothing");
    }

    #[test]
    fn a_chain_that_loops_back_stops_where_the_original_hangs() {
        let a = chained(140, (-1, 10, 200), (1, Some(141), 0));
        let b = chained(141, (-1, 30, 300), (1, Some(140), 0));
        assert_eq!(range_of(&[a, b], 140), 110.0, "A and B once each");
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
    fn a_sub_theta_of_minus_1_is_no_spread() {
        let theta = |sub_theta| {
            WeaponSpec::new(&WeaponRecord {
                sub_count: 5,
                sub_type: Some(WeaponId(148)),
                sub_theta,
                ..weapon(182)
            })
            .submunitions
            .map(|subs| subs.theta)
        };
        assert_eq!(theta(-1), Some(0), "@0x796c6-0x796cd");
        assert_eq!(theta(-2), Some(-2), "a starburst");
        assert_eq!(theta(45), Some(45));
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

    /// The rounds the secondary weapon line counts for a weapon of
    /// `ammo_type`, `flags2` and `guidance`, `wëap` 150.
    fn counted(ammo_type: i16, flags2: u16, guidance: i16) -> Option<WeaponId> {
        let record = WeaponRecord {
            guidance,
            ammo_type,
            flags2,
            ..weapon(150)
        };
        let count = line_count(&record);
        assert_eq!(WeaponSpec::new(&record).line_count, count, "kept");
        count
    }

    #[test]
    fn an_unlimited_weapon_shows_no_count() {
        assert_eq!(counted(-1, 0, -1), None);
    }

    #[test]
    fn flags2_0x0040_hides_the_count() {
        assert_eq!(counted(12, 0x0040, -1), None);
        assert_eq!(counted(12, 0xffff, 1), None);
        assert_eq!(counted(12, !0x0040, 1), Some(WeaponId(140)));
        assert_eq!(HIDES_COUNT, 0x0040);
    }

    #[test]
    fn an_ammo_type_of_minus_1000_or_below_shows_no_count() {
        assert_eq!(counted(-1000, 0, -1), None);
        assert_eq!(counted(-1001, 0, -1), None);
        assert_eq!(counted(i16::MIN, 0, 99), None);
    }

    #[test]
    fn an_ammo_type_of_0_to_255_counts_its_ammunition() {
        assert_eq!(counted(0, 0, 1), Some(WeaponId(128)));
        assert_eq!(counted(12, 0, 1), Some(WeaponId(140)));
        assert_eq!(counted(255, 0, 1), Some(WeaponId(383)));
    }

    #[test]
    fn an_ammo_type_above_255_unsigned_counts_the_weapons_own() {
        assert_eq!(counted(256, 0, 1), Some(WeaponId(150)));
        assert_eq!(counted(300, 0, -1), Some(WeaponId(150)));
        assert_eq!(counted(-999, 0, -1), Some(WeaponId(150)));
        assert_eq!(counted(-2, 0, -1), Some(WeaponId(150)));
    }

    #[test]
    fn guidance_99_counts_the_weapons_own() {
        assert_eq!(counted(12, 0, 99), Some(WeaponId(150)));
        assert_eq!(counted(144, 0, 99), Some(WeaponId(150)), "a bay");
        assert_eq!(counted(12, 0, 98), Some(WeaponId(140)));
    }
}
