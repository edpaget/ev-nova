//! Shots at asteroids: which shots touch an asteroid or set their fuse
//! off by one, what a hit takes off it and how it pushes it, as the
//! asteroid branches of the original's `_AsteroidCollisionHandler`,
//! `_CheckShotProximities` and `_HandleAsteroidHit` read them.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`:
//!
//! - **The contact** (`_AsteroidCollisionHandler` @0x369c8-0x36a44): the
//!   asteroid layer is collided with the three shot layers (@0x43eec,
//!   @0x43f62, @0x43fd8) and the point-defence layer (@0x43ffa), each
//!   after that layer's ship passes, so a shot that touches a ship never
//!   reaches the asteroids. A weapon with `Seeker` 0x0001 (tested
//!   @0x36a00, whatever its guidance) does nothing; any other's RLE
//!   overlap tail-calls `_HandleAsteroidHit` (@0x36a38). Lost and homing
//!   shots touch asteroids too. Here an asteroid is a circle of its
//!   sprite's radius ([`AsteroidType::radius`]): the engine has no pixel
//!   masks. The shot's path is swept from where it was, as against a
//!   ship ([`touches`]). The asteroid layer is the outer list of each
//!   pass, its sprites added in slot order (`_InitSprites` @0x2000d), so
//!   each asteroid in slot order meets every shot of the layer, and a
//!   shot that touches two is spent on the one in the lower slot,
//!   wherever each is along its move (`Combat::meet_asteroids`).
//! - **The fuse** (`_CheckShotProximities` @0x376ab-0x37758): only when no
//!   ship set the fuse off, skipped under `Seeker` 0x0001
//!   (@0x37566-0x37575), and only with a `ProxRadius` above none, the
//!   shot flown past its `ProxSafety` (@0x3720e-0x3721e), it takes the
//!   first active asteroid in slot order with trunc(dx)² + trunc(dy)² ≤
//!   `ProxRadius`², with no sprite-size term ([`fused`]). Then it calls
//!   `_HandleAsteroidHit` (@0x37744). No code reads `Flags2` 0x0004, the
//!   Bible's "proximity fuse ignores asteroids": the
//!   [`AsteroidFuse`] reads it as
//!   [`RuleKey::FuseIgnoresAsteroids`](crate::RuleKey::FuseIgnoresAsteroids)
//!   says. The branch tests no life, so a shot point defence ended at
//!   life 0 is still set off, its fuse armed when `Count` is above
//!   `ProxSafety` (`_CheckShotProximities` @0x371ea-0x371f6, see
//!   [`hidden`](super::hidden)).
//! - **The hit** (`_HandleAsteroidHit` @0x36637-0x369c7), in order: the
//!   weapon's explosion where the shot is (@0x3667d); its hit particles
//!   (@0x366a0-0x36701); for a player's shot with a `BlastRadius` from a
//!   weapon that is not planet-type, the blast (@0x3670d-0x368ca) on
//!   every active ship not planet-type (`shïp` `Flags` 0x0400, @0x367a8)
//!   within the square, the player spared only by `Flags` 0x0100; the
//!   damage, `MassDmg`, ten times under `Flags2` 0x8000 (@0x368e5-0x36909,
//!   [`damaged`]), with no `EnergyDmg`, decay, `Durability` or
//!   sub-munitions; below 0 the asteroid is destroyed (@0x3691a,
//!   [`broken`]); otherwise, with an `Impact` and a `Mass` above none, it
//!   is pushed (@0x36921-0x36996, [`push`]). The shot is always used up
//!   (@0x3699b-0x369bc). The [`Combat`](super::Combat) tick runs it.
//! - **The blast's hits** are `_DamageShip`'s (@0x36888) as a shot's own
//!   blast passes them: by the shot's source, neither direct nor intended,
//!   can't-kill as the shot is disable-only, and pushing each ship it
//!   reaches by the weapon's `Impact` away from the blast's centre, with
//!   no source gate ([`impact`](super::impact)); then `_IonizeShip`
//!   (@0x368b7) ionizes it over the blast's circle ([`ion`](super::ion)).
//!
//! A broken asteroid is skipped by every later shot in the tick: by a
//! later layer's pass and the proximity pass as its hidden sprite and
//! cleared slot are (@0xacba3, @0x376dd), and within the walk that broke
//! it as the engine's skip until
//! rdm:phase/asteroids/phase-10-shots-break-asteroids-inline, as the
//! original's walk goes on meeting it (@0xacd5b-0xacd6d).
//!
//! [`AsteroidType::radius`]: crate::asteroid::AsteroidType::radius

use super::damage::Blast;
use super::impact::speed_limit;
use super::projectile::{Shot, contact};
use super::weapon::{HitParticles, WeaponSpec};
use super::{ParticleBurst, ShipRef};
use crate::bay::launch_velocity;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

/// The fastest a hit pushes an asteroid on each axis, in pixels a tick
/// (`_AdjustedAccel` and `_SpeedLimit`'s 2.0, @0x36921-0x36996).
pub const PUSH_TOP_SPEED: f32 = 2.0;

/// How far, in percent, a shot's hit particle's speed varies either way,
/// on an asteroid (@0x366a0-0x36701), a ship (`_HandleShipHit`
/// @0x36b44-0x36bae) or a stellar (`_CheckShotProximities`
/// @0x3734d-0x373b4).
pub const HIT_PART_VARIATION: u32 = 20;

/// How far, in percent, a beam's hit particle's speed varies either way
/// on a ship (the 0x19 of `_HandleBeams` @0x310eb-0x31152).
pub const BEAM_HIT_PART_VARIATION: u32 = 25;

/// A hit particle's most life over its least (the 1.25 @0xdd5c8).
pub const HIT_PART_LIFE_SPAN: f64 = 1.25;

/// An asteroid shots may hit this tick, taken from its slot by the
/// session, which writes its strength and velocity back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsteroidTarget {
    /// Its slot in the field.
    pub slot: usize,
    /// Its centre.
    pub position: Vec2,
    /// How it is moving.
    pub velocity: Vec2,
    /// Its hit radius.
    pub radius: f32,
    /// Its strength: below 0 once broken.
    pub strength: i16,
    /// Its type's `Mass`.
    pub mass: i16,
}

/// Which asteroids a shot's armed proximity fuse is set off by, as
/// [`RuleKey::FuseIgnoresAsteroids`](crate::RuleKey::FuseIgnoresAsteroids)
/// reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AsteroidFuse {
    /// The original's: every armed fuse is set off by an asteroid, as no
    /// code reads `Flags2` 0x0004. The default.
    #[default]
    Original,
    /// The Bible's: the fuse of a weapon with `Flags2` 0x0004 ignores
    /// asteroids. Its shots still hit one they touch.
    Bible,
}

impl From<RuleSource> for AsteroidFuse {
    /// The engine's reading [`AsteroidFuse::Original`]; the other
    /// [`AsteroidFuse::Bible`].
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Original,
            RuleSource::Bible => Self::Bible,
        }
    }
}

/// Whether an asteroid of `strength` is broken: below 0, which destroys it
/// (@0x3691a). One at exactly 0 is still whole.
#[must_use]
pub fn broken(strength: i16) -> bool {
    strength < 0
}

/// Where `shot`, having flown here from `was`, first came within
/// `target`'s radius on its way, if it did: where it was for a shot that
/// started within. None for a broken asteroid, or for a weapon that
/// ignores asteroids (see the module docs).
#[must_use]
pub fn touches(shot: &Shot, was: Vec2, target: &AsteroidTarget) -> Option<Vec2> {
    if shot.weapon.ignores_asteroids() || broken(target.strength) {
        return None;
    }
    let t = contact(was, shot.position, target.position, target.radius)?;
    Some(was + (shot.position - was) * t)
}

/// The asteroid among `targets`, by its index, that sets `shot`'s armed
/// proximity fuse off by `fuse`'s reading: the first not broken, in slot
/// order, whose whole offset from the shot on each axis, squared and
/// summed, is at most `ProxRadius`² (see the module docs).
#[must_use]
pub fn fused(shot: &Shot, targets: &[AsteroidTarget], fuse: AsteroidFuse) -> Option<usize> {
    let weapon = &shot.weapon;
    if weapon.ignores_asteroids()
        || weapon.prox_radius <= 0.0
        || !shot.fuse_armed()
        || fuse == AsteroidFuse::Bible && weapon.fuse_spares_asteroids()
    {
        return None;
    }
    let reach = whole(weapon.prox_radius);
    targets.iter().position(|target| {
        let off = target.position - shot.position;
        let (dx, dy) = (whole(off.x), whole(off.y));
        !broken(target.strength) && dx * dx + dy * dy <= reach * reach
    })
}

/// `value`'s whole part, as the original truncates an offset.
fn whole(value: f32) -> i64 {
    // An offset in pixels: far within an `i64`.
    #[allow(clippy::cast_possible_truncation)]
    {
        value.trunc() as i64
    }
}

/// The velocity of an asteroid moving at `velocity`, of `Mass` `mass`
/// above none, after a hit of `impact` along `heading`: `_AdjustedAccel`
/// along the whole-degree heading by `impact` / `mass`, held to
/// [`PUSH_TOP_SPEED`], then `_SpeedLimit` at it (@0x36921-0x36996). A
/// negative `impact` pulls it back along the shot.
#[must_use]
pub fn push(velocity: Vec2, heading: f32, impact: f32, mass: i16) -> Vec2 {
    let pushed = launch_velocity(
        velocity,
        heading.trunc(),
        impact / f32::from(mass),
        PUSH_TOP_SPEED,
    );
    speed_limit(pushed, PUSH_TOP_SPEED)
}

/// The burst of a hit's `particles` at `at`: their speed varied by
/// `variation` percent, their life from `HitPartLife` to its whole ×
/// [`HIT_PART_LIFE_SPAN`], all at `at`, as every hit's `_SpawnParticles`
/// call passes them: a shot's on an asteroid (@0x366a0-0x36701), a ship
/// (@0x36b44-0x36bae) or a stellar (@0x3734d-0x373b4), at
/// [`HIT_PART_VARIATION`], and a beam's on a ship (@0x310eb-0x31152), at
/// [`BEAM_HIT_PART_VARIATION`].
#[must_use]
pub fn hit_burst(at: Vec2, particles: &HitParticles, variation: u32) -> ParticleBurst {
    // A whole `u32` times 1.25, truncated: within a `u32` for any life
    // an `i16` gives.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let most = (f64::from(particles.life) * HIT_PART_LIFE_SPAN).trunc() as u32;
    ParticleBurst {
        at,
        speed: particles.speed,
        variation,
        life: (particles.life, most),
        color: particles.color,
        count: particles.count,
        spread: 0.0,
    }
}

/// The blast of `shot`'s hit on an asteroid at `at` (@0x3670d-0x368ca):
/// only a player's shot's (the player its [`Shot::attacker`]), with a
/// `BlastRadius` above none, from a weapon that is not planet-type; it
/// hits no ship directly, and spares the player only by `Flags` 0x0100.
/// The caller leaves planet-type ships out of those it may reach
/// (@0x367a8).
#[must_use]
pub fn blast(shot: &Shot, at: Vec2) -> Option<Blast> {
    let weapon = &shot.weapon;
    let players = shot.attacker() == Some(ShipRef::Player);
    (players && weapon.blast_radius > 0.0 && !weapon.planet_type()).then(|| Blast {
        at,
        radius: weapon.blast_radius,
        direct: None,
        firer: Some(ShipRef::Player),
        spares_player: weapon.spares_player(),
    })
}

/// What is left of an asteroid's `strength` after a hit of `weapon`: its
/// [`WeaponSpec::asteroid_damage`] taken off, held within an `i16`. Below
/// 0 breaks it, so holding it there never changes the outcome.
#[must_use]
pub fn damaged(strength: i16, weapon: &WeaponSpec) -> i16 {
    damaged_by(strength, weapon, 1)
}

/// What is left of an asteroid's `strength` after a hit of `weapon` of
/// `times` its [`WeaponSpec::asteroid_damage`], held within an `i16`: a
/// shot's once ([`damaged`]), a beam's twice on double time's advance
/// pass (`_HandleBeams` @0x31a23-0x31a3d, see
/// [`asteroid_beam`](super::asteroid_beam)).
#[must_use]
pub fn damaged_by(strength: i16, weapon: &WeaponSpec, times: i32) -> i16 {
    let left = i32::from(strength) - weapon.asteroid_damage() * times;
    // Clamped into the `i16` range first.
    #[allow(clippy::cast_possible_truncation)]
    {
        left.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::projectile::Guide;
    use crate::flight::ShipState;
    use nova_sim_catalog::fixture::weapon;

    /// A gun of `record` fired from the centre facing right.
    fn shot_of(record: WeaponRecord) -> Shot {
        let state = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: 90.0,
        };
        Shot::launch(
            WeaponSpec::new(&record),
            ShipRef::Player,
            ShipRef::Player,
            &state,
            90.0,
        )
    }

    /// A gun 100 pixels a tick, flown one tick: from the centre to
    /// (100, 0).
    fn fast(record: WeaponRecord) -> (Shot, Vec2) {
        let mut shot = shot_of(WeaponRecord {
            speed: 10_000,
            count: 30,
            ..record
        });
        let was = shot.step(&Guide::among(&[]), &mut NeverFires);
        (shot, was)
    }

    fn rock(slot: usize, x: f32, y: f32, radius: f32) -> AsteroidTarget {
        AsteroidTarget {
            slot,
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            radius,
            strength: 10,
            mass: 20,
        }
    }

    #[test]
    fn a_fast_shot_touches_an_asteroid_it_passed_through_between_ticks() {
        let (shot, was) = fast(weapon(128));
        let at = touches(&shot, was, &rock(0, 50.0, 0.0, 10.0)).expect("touched");
        assert!((at - Vec2::new(40.0, 0.0)).length() < 1e-3, "{at:?}");
    }

    #[test]
    fn a_shot_that_passes_outside_the_radius_touches_nothing() {
        let (shot, was) = fast(weapon(128));
        assert_eq!(touches(&shot, was, &rock(0, 50.0, 10.5, 10.0)), None);
        assert!(touches(&shot, was, &rock(0, 50.0, 9.5, 10.0)).is_some());
    }

    #[test]
    fn a_shot_starting_within_an_asteroid_touches_it_where_it_was() {
        let (shot, was) = fast(weapon(128));
        let at = touches(&shot, was, &rock(0, 5.0, 0.0, 10.0)).expect("touched");
        assert_eq!(at, was);
    }

    #[test]
    fn a_broken_asteroid_is_touched_by_no_shot() {
        let (shot, was) = fast(weapon(128));
        let mut target = rock(0, 30.0, 0.0, 5.0);
        target.strength = -1;
        assert_eq!(touches(&shot, was, &target), None);
        target.strength = 0;
        assert!(touches(&shot, was, &target).is_some(), "at 0, whole");
    }

    #[test]
    fn seeker_0x0001_touches_no_asteroid_as_a_gun_or_a_homing_shot() {
        for guidance in [-1, 1] {
            let (shot, was) = fast(WeaponRecord {
                guidance,
                seeker: 0x0001,
                ..weapon(128)
            });
            assert_eq!(
                touches(&shot, was, &rock(0, 50.0, 0.0, 10.0)),
                None,
                "{guidance}"
            );
        }
        let (homing, was) = fast(WeaponRecord {
            guidance: 1,
            ..weapon(128)
        });
        assert!(
            touches(&homing, was, &rock(0, 50.0, 0.0, 10.0)).is_some(),
            "a homing shot without it touches"
        );
    }

    /// A shot of a weapon of `ProxRadius` `prox`, `ProxSafety` `safety`
    /// and these flags, at the centre, flown `age` ticks.
    fn fuse(prox: i16, safety: i16, seeker: u16, flags2: u16, age: u32) -> Shot {
        let mut shot = shot_of(WeaponRecord {
            prox_radius: prox,
            prox_safety: safety,
            seeker,
            flags2,
            ..weapon(128)
        });
        shot.age = age;
        shot
    }

    #[test]
    fn the_fuse_takes_the_first_asteroid_in_slot_order_within_reach_not_the_nearest() {
        let shot = fuse(50, 0, 0, 0, 1);
        let targets = [rock(0, 40.0, 0.0, 1.0), rock(1, 5.0, 0.0, 1.0)];
        assert_eq!(fused(&shot, &targets, AsteroidFuse::Original), Some(0));
        let mut targets = targets;
        targets[0].strength = -1;
        assert_eq!(
            fused(&shot, &targets, AsteroidFuse::Original),
            Some(1),
            "a broken one skipped"
        );
    }

    #[test]
    fn the_fuse_reaches_its_radius_by_the_whole_offsets_squared_with_no_size() {
        let shot = fuse(50, 0, 0, 0, 1);
        for (x, y, reached) in [
            (50.0, 0.0, true),
            (50.9, 0.0, true),
            (51.0, 0.0, false),
            (-50.9, 0.0, true),
            (30.0, 40.0, true),
            (30.0, 40.9, true),
            (30.0, 41.0, false),
        ] {
            let target = rock(0, x, y, 200.0);
            assert_eq!(
                fused(&shot, &[target], AsteroidFuse::Original).is_some(),
                reached,
                "({x}, {y})"
            );
        }
    }

    #[test]
    fn no_fuse_is_set_off_without_a_radius_before_it_is_armed_or_under_seeker_0x0001() {
        let near = [rock(0, 5.0, 0.0, 10.0)];
        assert_eq!(
            fused(&fuse(0, 0, 0, 0, 1), &near, AsteroidFuse::Original),
            None
        );
        assert_eq!(
            fused(&fuse(50, 3, 0, 0, 3), &near, AsteroidFuse::Original),
            None,
            "not yet past ProxSafety"
        );
        assert_eq!(
            fused(&fuse(50, 3, 0, 0, 4), &near, AsteroidFuse::Original),
            Some(0)
        );
        assert_eq!(
            fused(&fuse(50, 0, 0x0001, 0, 1), &near, AsteroidFuse::Original),
            None
        );
    }

    #[test]
    fn flags2_0x0004_sets_the_fuse_off_by_the_original_and_not_by_the_bible() {
        let near = [rock(0, 5.0, 0.0, 10.0)];
        let shot = fuse(50, 0, 0, 0x0004, 1);
        assert_eq!(fused(&shot, &near, AsteroidFuse::Original), Some(0));
        assert_eq!(fused(&shot, &near, AsteroidFuse::Bible), None);
        let plain = fuse(50, 0, 0, 0, 1);
        assert_eq!(fused(&plain, &near, AsteroidFuse::Bible), Some(0));
    }

    #[test]
    fn the_fuse_reads_its_rule() {
        assert_eq!(
            AsteroidFuse::from(RuleSource::Engine),
            AsteroidFuse::Original
        );
        assert_eq!(AsteroidFuse::from(RuleSource::Bible), AsteroidFuse::Bible);
        assert_eq!(AsteroidFuse::default(), AsteroidFuse::Original);
    }

    #[test]
    fn a_hit_pushes_by_impact_over_mass_along_the_whole_degree_heading() {
        let pushed = push(Vec2::ZERO, 90.7, 10.0, 20);
        assert!((pushed - Vec2::new(0.5, 0.0)).length() < 1e-5, "{pushed:?}");
        let up = push(Vec2::new(0.25, 0.0), 0.0, 30.0, 20);
        assert!((up - Vec2::new(0.25, -1.5)).length() < 1e-5, "{up:?}");
    }

    #[test]
    fn a_push_is_held_to_2_a_tick_on_each_axis() {
        let pushed = push(Vec2::new(1.8, 0.0), 90.0, 100.0, 10);
        assert!((pushed.x - 2.0).abs() < 1e-6, "{pushed:?}");
        let held = push(Vec2::new(0.0, 3.0), 90.0, 10.0, 20);
        assert_eq!(held.y, 2.0, "past the top already: held to it");
        assert_eq!(push(Vec2::new(0.0, -3.0), 90.0, 10.0, 20).y, -2.0);
        assert_eq!(PUSH_TOP_SPEED, 2.0);
    }

    #[test]
    fn a_negative_impact_pulls_back_along_the_shot() {
        let pulled = push(Vec2::new(-1.0, 0.0), 90.0, -10.0, 20);
        assert!(
            (pulled - Vec2::new(-1.5, 0.0)).length() < 1e-5,
            "{pulled:?}"
        );
        let held = push(Vec2::new(-1.8, 0.0), 90.0, -10.0, 20);
        assert!((held.x + 2.0).abs() < 1e-6, "{held:?}");
    }

    fn mining(mass_dmg: i16, flags2: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            mass_dmg,
            energy_dmg: 500,
            flags2,
            ..weapon(128)
        })
    }

    #[test]
    fn a_hit_takes_its_mass_damage_or_ten_times_it_and_no_energy_damage() {
        assert_eq!(damaged(40, &mining(7, 0)), 33);
        assert_eq!(damaged(40, &mining(7, 0x8000)), -30);
        assert_eq!(damaged(40, &mining(0, 0)), 40, "energy damage alone");
    }

    #[test]
    fn a_beams_hit_in_double_time_takes_twice_its_damage_mining_or_not() {
        assert_eq!(damaged_by(40, &mining(7, 0), 2), 26);
        assert_eq!(damaged_by(400, &mining(7, 0x8000), 2), 260);
        assert_eq!(damaged_by(40, &mining(7, 0), 1), damaged(40, &mining(7, 0)));
        assert_eq!(damaged_by(-32_000, &mining(i16::MAX, 0x8000), 2), i16::MIN);
    }

    #[test]
    fn what_is_left_is_held_within_an_i16() {
        assert_eq!(damaged(-32_000, &mining(i16::MAX, 0x8000)), i16::MIN);
        assert_eq!(damaged(i16::MAX, &mining(-5, 0)), i16::MAX);
    }

    #[test]
    fn a_hits_burst_varies_its_speed_20_percent_and_lives_up_to_a_quarter_longer() {
        let particles = HitParticles {
            count: 6,
            speed: 2.5,
            life: 30,
            color: 0x00FF_8000,
        };
        let at = Vec2::new(5.0, -7.0);
        assert_eq!(
            hit_burst(at, &particles, HIT_PART_VARIATION),
            ParticleBurst {
                at,
                speed: 2.5,
                variation: 20,
                life: (30, 37),
                color: 0x00FF_8000,
                count: 6,
                spread: 0.0,
            }
        );
        let short = HitParticles {
            life: 3,
            ..particles
        };
        assert_eq!(hit_burst(at, &short, HIT_PART_VARIATION).life, (3, 3));
        assert_eq!(HIT_PART_LIFE_SPAN, 1.25);
    }

    #[test]
    fn a_beams_burst_varies_its_speed_25_percent_with_a_shots_life() {
        let particles = HitParticles {
            count: 6,
            speed: 2.5,
            life: 30,
            color: 0x00FF_8000,
        };
        let burst = hit_burst(Vec2::ZERO, &particles, BEAM_HIT_PART_VARIATION);
        assert_eq!((burst.variation, burst.life), (25, (30, 37)));
    }

    /// A shot of `firer`'s weapon of `BlastRadius` `radius` and these
    /// `Flags` and `Flags2`.
    fn blasting(firer: ShipRef, radius: i16, flags: u16, flags2: u16) -> Shot {
        Shot {
            firer,
            ..shot_of(WeaponRecord {
                blast_radius: radius,
                flags,
                flags2,
                ..weapon(128)
            })
        }
    }

    #[test]
    fn only_a_players_shot_with_a_blast_radius_and_not_planet_type_blasts() {
        let at = Vec2::new(3.0, 4.0);
        assert_eq!(
            blast(&blasting(ShipRef::Player, 30, 0, 0), at),
            Some(Blast {
                at,
                radius: 30.0,
                direct: None,
                firer: Some(ShipRef::Player),
                spares_player: false,
            })
        );
        assert_eq!(
            blast(&blasting(ShipRef::Player, 30, 0x0100, 0), at).map(|b| b.spares_player),
            Some(true)
        );
        let npc = ShipRef::Npc(crate::combat::NpcId(1));
        assert_eq!(blast(&blasting(npc, 30, 0, 0), at), None);
        assert_eq!(blast(&blasting(ShipRef::Player, 0, 0, 0), at), None);
        assert_eq!(blast(&blasting(ShipRef::Player, 30, 0, 0x0400), at), None);
    }

    #[test]
    fn an_asteroid_is_broken_below_0_and_not_at_0() {
        assert!(!broken(0));
        assert!(!broken(1));
        assert!(broken(-1));
        assert!(broken(i16::MIN));
    }
}
