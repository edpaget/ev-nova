//! Beams on asteroids: `_HandleBeams`' asteroid pass (@0x316c2-0x31d54),
//! which cuts, wears down, breaks, pushes and tractors the asteroids a
//! beam reaches.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`, on the advance pass alone
//! (`GamePass`, @0x303a8-0x303af):
//!
//! - **The entry** (@0x30f4c-0x30f94): the pass runs only when the ship
//!   pass found no ship, or when the firer is an NPC mining asteroid slot 0
//!   (`_AIShipIsAttackingAsteroid`, [`Ties::mining`](super::sides::Ties::mining)),
//!   whose beam passes every ship. So a ship in the cone shields every
//!   asteroid, even a nearer one. A point-defence beam held on its missile
//!   jumps to the beam's end (@0x309ab-0x309c9 → @0x31e04) and never
//!   reaches it, and a weapon with `Seeker` 0x0001 skips it (@0x316e0).
//! - **The cone** ([`pick`], @0x3176b-0x317f0) is the ships' ([`cone`]),
//!   measured from the beam's start (-0x28), with r the asteroid's sprite
//!   height, trunc(`_GetSpriteYSize`) ([`AsteroidType::height`]), and no
//!   0.66 factor: reach `BeamLength` + trunc(r / 2), inclusive, and an angle
//!   tolerance of trunc(10r / 32) degrees.
//! - **The nearest** (@0x317f2-0x31841): the slots are walked 0-15, the
//!   active ones in the cone are candidates, and one becomes the best when
//!   its weight, its squared distance from the start truncated, is below
//!   the best's or no best is set yet, so the first of equals wins. The
//!   original weighs asteroid k by the squared distance of **ship-table
//!   slot k** (+0x18, stride 0xc940): slot 0 the player, slot k ≥ 1 the
//!   NPC whose berth is k.
//! - **The cut** ([`cut`], @0x318a7-0x31927): the beam ends
//!   trunc(√d − 0.2 × height) along its heading, d the asteroid's own
//!   squared distance whatever weighed it. With no asteroid hit the beam is
//!   drawn at its full `BeamLength` (@0x31d56-0x31d86).
//! - **The explosion and the particles**, every tick it hits:
//!   `_CreateExplosion(end, ExplodType, BlastRadius, 1)` (@0x3196f) and,
//!   with a `HitPartCount`, `_SpawnParticles` (@0x31993-0x319f6) at
//!   [`BEAM_HIT_PART_VARIATION`], as on a ship. No blast, no ionization, no
//!   `_DamageShip`, so no strike, crime or reaction; no sound but the
//!   explosion's.
//! - **The damage** ([`asteroid_hit::damaged_by`]): `MassDmg` (wëap +0x2),
//!   ×2 under the double-time flag (@0x31a23-0x31a2d), then ×10 under
//!   `Flags2` 0x8000 (@0x31a30-0x31a3d). Below 0 (`jns`) the asteroid is
//!   destroyed at once (`_DestroyAsteroid` @0x31a69), so a later beam of
//!   the same pass meets its fragments; at exactly 0 it stands.
//! - **The push or the pull** ([`moves`]): nothing more with an `Impact`
//!   of none or the röid's `Mass` (+0xe) at or below none. A positive
//!   `Impact` pushes. A negative one pulls the firer only when the beam
//!   has a firer (+0x16 ≠ −1), the asteroid's `Mass` ×
//!   [`ASTEROID_PULL_SHARE`] (compared as doubles, strictly) is above the
//!   firer class's `Mass` (shïp +0x9ee), that `Mass` is above none and the
//!   firer is not planet-type (+0x9e9 & 0x04); every other negative case
//!   pushes (@0x31d00/@0x31d03), pulling the asteroid back along the beam.
//!   - The push (@0x31d03-0x31d4f) is `_AdjustedAccel(heading, Impact /
//!     Mass, &vel, 2.0)` then `_SpeedLimit(&vel, 2.0)`
//!     ([`asteroid_hit::push`]).
//!   - The pull (@0x31aa7-0x31cfb): unless the beam's start is within 50
//!     pixels of the asteroid on **both** axes (strict, the start, not the
//!     firer's centre), the firer gains `Impact` / its `Mass` along the
//!     bearing from the asteroid to the start against its class's top
//!     speed (+0x40), clamped to `_ShipMaxSpeed` (@0x31bd4-0x31ca3,
//!     [`self_pull`]). Either way its lock is touched (@0x31ca8-0x31cf5,
//!     [`TractorLock::touch_self`]).
//!
//! # The engine
//!
//! [`Combat::close_tick_in`](super::Combat::close_tick_in) runs the pass
//! for each beam whose ship hit gave nothing, in beam order, on the field
//! the session lends ([`AsteroidField`]). It destroys an asteroid inline, as
//! the original does, so a later beam meets the fragments: the remains'
//! particles and explosion are raised at once, and the boxes are left for
//! the session to spawn ([`Combat::take_asteroid_remains`](super::Combat::take_asteroid_remains)),
//! after the phase and before the frame is drawn, which no beam reads.
//! The shots destroy theirs the same way ([`asteroid_walk`](super::asteroid_walk)).
//!
//! How the nearest is weighed is
//! [`RuleKey::BeamConeQuirks`](crate::RuleKey::BeamConeQuirks)'s
//! ([`ConeAngle`]): by the engine's reading ([`ConeAngle::Plain`]) by ship
//! slot k's distance, the fight's last note of that slot's position
//! (`Combat::note_slots`), which the player's wrap carries as it moves an
//! empty slot ([`wrap_carry`](super::wrap_carry)); by the other, the
//! default, by the asteroid's own. The angle test reads the same [`ConeAngle`]. By the other reading
//! of [`RuleKey::BeamHitTest`](crate::RuleKey::BeamHitTest)
//! ([`BeamHitTest::Segment`]) the beam takes the first asteroid its segment
//! meets, within its hit circle ([`AsteroidType::radius`]), and ends there,
//! as it does for ships; ships still come first.
//!
//! Left out:
//!
//! - **A never-held ship slot under the engine's nearest.** The ship table
//!   comes from `_NewPtr`, not `_NewPtrClear` (`_InitArrays` @0x1b7a8), so
//!   a slot no ship has held is uninitialised memory, which can't be
//!   replicated: such a slot weighs its asteroid by the asteroid's own
//!   distance.
//! - **A pull whose firer has left the fight** reads the original's stale
//!   slot (beam +0x16); here a firer no longer among the fighters takes the
//!   push, as a beam of no firer does (@0x31ab7 → @0x31d00), so the
//!   asteroid is pulled. Stock's one tractor beam (`wëap` 171, `Count` 1)
//!   never meets this case.
//!
//! [`AsteroidType::height`]: crate::asteroid::AsteroidType::height
//! [`AsteroidType::radius`]: crate::asteroid::AsteroidType::radius
//! [`cone`]: super::beam::cone
//! [`BEAM_HIT_PART_VARIATION`]: super::asteroid_hit::BEAM_HIT_PART_VARIATION
//! [`asteroid_hit::damaged_by`]: super::asteroid_hit::damaged_by
//! [`asteroid_hit::push`]: super::asteroid_hit::push
//! [`ASTEROID_PULL_SHARE`]: super::impact::ASTEROID_PULL_SHARE
//! [`self_pull`]: super::impact::self_pull
//! [`TractorLock::touch_self`]: super::impact::TractorLock::touch_self

use super::asteroid_hit::{BEAM_HIT_PART_VARIATION, Rock, broken, damaged_by, push};
use super::asteroid_walk::AsteroidField;
use super::beam::{Aiming, Beam, BeamHitTest, BeamRules, ConeAngle, HIT_SHORT_PER_WIDTH, cone};
use super::impact::{ASTEROID_PULL_SHARE, Body, TractorLock, self_pull};
use super::projectile::contact;
use super::{Combat, CombatEvent, Fighter, GamePass};
use crate::chance::Chance;
use crate::flight::facing;
use crate::geometry::Vec2;
use crate::ship_layer::Slot;

/// Whether `beam` reaches the asteroids at all: not a point-defence beam
/// held on its missile, and not a weapon of `Seeker` 0x0001 (see the
/// module docs).
#[must_use]
pub fn reaches_asteroids(beam: &Beam) -> bool {
    !matches!(beam.aiming, Aiming::Shot(_)) && !beam.weapon.ignores_asteroids()
}

/// Where `beam` cuts the asteroid it hits among `rocks`, in slot order,
/// found as `rules` say, and that asteroid's index among them (see the
/// module docs): by the cone, r = trunc(height), the best by its weight,
/// the first of equals, the beam ending at its [`cut`]; by the segment,
/// the first contact, where it meets. `ship_at` gives where ship slot k
/// was last noted, which the engine's reading ([`ConeAngle::Plain`])
/// weighs asteroid k by; a slot never held weighs it by its own distance.
#[must_use]
pub fn pick(
    beam: &Beam,
    rocks: &[Rock],
    rules: BeamRules,
    ship_at: impl Fn(usize) -> Option<Vec2>,
) -> Option<(usize, Vec2)> {
    match rules.hit_test {
        BeamHitTest::Cone => rocks
            .iter()
            .enumerate()
            .filter_map(|(index, rock)| {
                let r = whole(rock.height);
                let length = beam.weapon.beam_length;
                let own = cone(
                    beam.start,
                    beam.heading,
                    length,
                    r,
                    rock.position,
                    rules.angle,
                )?;
                let weighed = match rules.angle {
                    ConeAngle::Plain => ship_at(rock.slot).map_or(own, |ship| {
                        let off = ship - beam.start;
                        off.x * off.x + off.y * off.y
                    }),
                    ConeAngle::Wrapped => own,
                };
                Some((index, own, whole(weighed)))
            })
            .min_by_key(|&(_, _, weight)| weight)
            .map(|(index, own, _)| {
                let rock = rocks[index];
                (index, cut(beam.start, beam.heading, own, rock.height))
            }),
        BeamHitTest::Segment => rocks
            .iter()
            .enumerate()
            .filter_map(|(index, rock)| {
                Some((
                    index,
                    contact(beam.start, beam.end, rock.position, rock.radius)?,
                ))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, t)| (index, beam.start + (beam.end - beam.start) * t)),
    }
}

/// Where a beam from `start` along `heading` ends on an asteroid of frame
/// `height` at squared distance `squared`: trunc(√d − 0.2 × height) along
/// it (@0x318a7-0x31927).
#[must_use]
pub fn cut(start: Vec2, heading: f32, squared: f32, height: f32) -> Vec2 {
    let distance = f64::from(squared).sqrt();
    // A distance in pixels: well within an `f32`.
    #[allow(clippy::cast_possible_truncation)]
    let along = f64::from(height)
        .mul_add(-HIT_SHORT_PER_WIDTH, distance)
        .trunc() as f32;
    start + facing(heading) * along
}

/// What a beam's hit does to an asteroid it leaves standing, besides its
/// damage (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moves {
    /// Nothing: an `Impact` of none, or a massless asteroid.
    Still,
    /// The asteroid is pushed along the beam, or pulled back along it by a
    /// negative `Impact` ([`asteroid_hit::push`](super::asteroid_hit::push)).
    Push,
    /// The firer pulls itself toward the asteroid and holds itself.
    Pull,
}

/// What a beam of `impact` does to an asteroid of `Mass` `mass` it hits,
/// its firer as a push reads it, if it is in the fight (see the module
/// docs).
#[must_use]
pub fn moves(impact: f32, mass: i16, firer: Option<&Body>) -> Moves {
    if impact == 0.0 || mass <= 0 {
        return Moves::Still;
    }
    let pulls = impact < 0.0
        && firer.is_some_and(|firer| {
            f64::from(mass) * ASTEROID_PULL_SHARE > f64::from(firer.mass)
                && firer.mass > 0.0
                && !firer.planet_type
        });
    if pulls { Moves::Pull } else { Moves::Push }
}

impl Combat {
    /// `_HandleBeams`' asteroid pass for beam `index` (see the module
    /// docs), on `pass`, its ship pass having hit nothing: the asteroid it
    /// reaches in `field` cuts it, explodes and sprays where it ends, and
    /// takes its damage; below 0 it is destroyed at once, drawing on
    /// `chance`, its particles and explosion raised and its remains kept
    /// for the session ([`Combat::take_asteroid_remains`]); otherwise it is
    /// pushed, or the firer among `fighters` pulls itself toward it.
    pub(super) fn cut_asteroids(
        &mut self,
        index: usize,
        fighters: &mut [Fighter],
        field: &mut AsteroidField<'_>,
        pass: GamePass,
        chance: &mut dyn Chance,
    ) {
        let beam = self.beams[index];
        if !reaches_asteroids(&beam) {
            return;
        }
        let rocks: Vec<Rock> = field
            .asteroids
            .indexed()
            .map(|(slot, asteroid)| Rock::of(slot, asteroid, field.types))
            .collect();
        let ship_at = |k: usize| {
            let slot = match u32::try_from(k) {
                Ok(0) => Slot::Player,
                Ok(berth) => Slot::Berth(berth),
                Err(_) => return None,
            };
            self.slot_ships.get(&slot).map(|ship| ship.position)
        };
        let Some((hit, end)) = pick(&beam, &rocks, self.beam_rules, ship_at) else {
            return;
        };
        self.beams[index].end = end;
        let weapon = beam.weapon;
        if let Some(explosion) = weapon.explosion {
            self.events.push(CombatEvent::Exploded {
                at: end,
                explosion,
                size: weapon.blast_radius,
            });
        }
        self.spray(&weapon, end, BEAM_HIT_PART_VARIATION);
        let slot = rocks[hit].slot;
        let Some(asteroid) = field.asteroids.get_mut(slot) else {
            return;
        };
        asteroid.strength = damaged_by(asteroid.strength, &weapon, i32::from(pass.beam_times()));
        if broken(asteroid.strength) {
            self.destroy_asteroid(slot, field, chance);
            return;
        }
        let mass = field
            .types
            .get(usize::from(asteroid.kind))
            .map_or(0, |kind_type| kind_type.mass);
        let firer = fighters
            .iter()
            .position(|fighter| fighter.ship == beam.firer);
        let body = firer.map(|firer| fighters[firer].body());
        match moves(weapon.impact, mass, body.as_ref()) {
            Moves::Still => {}
            Moves::Push => {
                asteroid.velocity = push(asteroid.velocity, beam.heading, weapon.impact, mass);
            }
            Moves::Pull => {
                // `moves` pulls only for a firer in the fight.
                let (Some(firer), Some(body)) = (firer, body) else {
                    return;
                };
                let toward = asteroid.position;
                let pulled = self_pull(&body, toward, beam.start, weapon.impact, self.speed_cap);
                let firer = &mut fighters[firer];
                TractorLock::touch_self(firer.tractor, firer.ship);
                if let Some(velocity) = pulled {
                    self.shove(firer, velocity);
                }
            }
        }
    }
}

/// `value`'s whole part, as the original truncates a size or a distance.
fn whole(value: f32) -> i64 {
    // Pixels, or pixels squared: far within an `i64`.
    #[allow(clippy::cast_possible_truncation)]
    {
        value.trunc() as i64
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::WeaponRecord;
    use crate::combat::NpcId;
    use crate::combat::ShipRef;
    use crate::combat::projectile::ShotId;
    use crate::combat::weapon::WeaponSpec;
    use crate::flight::ShipState;
    use nova_sim_catalog::fixture::weapon;

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));

    /// A 100-pixel beam of `record` from the centre, along `heading`.
    fn beam_of(record: WeaponRecord, heading: f32) -> Beam {
        let weapon = WeaponSpec::new(&WeaponRecord {
            guidance: 0,
            count: 3,
            beam_length: 100,
            ..record
        });
        let from = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading,
        };
        Beam::launch(weapon, FIRER, FIRER, &from, Aiming::Ahead)
    }

    /// A 100-pixel beam from the centre pointing right.
    fn right() -> Beam {
        beam_of(weapon(146), 90.0)
    }

    /// An asteroid in `slot` at (`x`, `y`), 30 high, of radius 20.
    fn rock(slot: usize, x: f32, y: f32) -> Rock {
        Rock {
            slot,
            position: Vec2::new(x, y),
            height: 30.0,
            radius: 20.0,
        }
    }

    const WRAPPED: BeamRules = BeamRules {
        hit_test: BeamHitTest::Cone,
        angle: ConeAngle::Wrapped,
    };
    const PLAIN: BeamRules = BeamRules {
        hit_test: BeamHitTest::Cone,
        angle: ConeAngle::Plain,
    };
    const SEGMENT: BeamRules = BeamRules {
        hit_test: BeamHitTest::Segment,
        angle: ConeAngle::Wrapped,
    };

    fn nowhere(_: usize) -> Option<Vec2> {
        None
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn the_cone_reaches_the_length_plus_half_the_whole_height_from_the_start() {
        // r = trunc(30.9) = 30: reach 115, inclusive.
        let tall = |x| Rock {
            height: 30.9,
            ..rock(0, x, 0.0)
        };
        assert!(pick(&right(), &[tall(115.0)], WRAPPED, nowhere).is_some());
        assert_eq!(pick(&right(), &[tall(115.5)], WRAPPED, nowhere), None);
        let mut moved = right();
        moved.start = Vec2::new(50.0, 0.0);
        assert!(
            pick(&moved, &[tall(165.0)], WRAPPED, nowhere).is_some(),
            "from the start"
        );
    }

    #[test]
    fn the_cone_holds_trunc_10r_over_32_degrees_either_side() {
        // r = 30: 9 degrees; a bearing of 99 is in, 101 out.
        let at = |degrees: f32| {
            let along = facing(degrees) * 60.0;
            rock(0, along.x, along.y)
        };
        assert!(pick(&right(), &[at(99.0)], WRAPPED, nowhere).is_some());
        assert_eq!(pick(&right(), &[at(101.0)], WRAPPED, nowhere), None);
    }

    #[test]
    fn the_nearest_is_by_its_own_whole_distance_the_first_of_equals() {
        let rocks = [rock(0, 80.0, 0.0), rock(1, 40.0, 0.0), rock(2, 40.3, 0.0)];
        let (index, _) = pick(&right(), &rocks, WRAPPED, nowhere).expect("hit");
        assert_eq!(index, 1);
        // 40² and 40.01²: whole, equal, so the first.
        let tied = [rock(3, 40.01, 0.0), rock(4, 40.0, 0.0)];
        assert_eq!(
            pick(&right(), &tied, WRAPPED, nowhere).map(|(i, _)| i),
            Some(0)
        );
    }

    #[test]
    fn the_cut_ends_trunc_root_d_less_a_fifth_of_the_height_along_the_heading() {
        // √(40²) − 0.2 × 30 = 34.
        let (_, end) = pick(&right(), &[rock(0, 40.0, 0.0)], WRAPPED, nowhere).expect("hit");
        assert!(close(end, Vec2::new(34.0, 0.0)), "{end:?}");
        // √(43.7²) − 0.2 × 33 = 37.1: 37, along the heading, not at the rock.
        let off = Rock {
            height: 33.0,
            ..rock(0, 43.0, 7.8)
        };
        let squared = 43.0 * 43.0 + 7.8 * 7.8;
        let along = (f64::from(squared).sqrt() - 6.6).trunc() as f32;
        assert!(close(
            cut(Vec2::ZERO, 90.0, squared, 33.0),
            Vec2::new(along, 0.0)
        ));
        let (_, end) = pick(&right(), &[off], WRAPPED, nowhere).expect("hit");
        assert!(close(end, Vec2::new(along, 0.0)), "{end:?}");
    }

    #[test]
    fn by_the_engine_asteroid_k_is_weighed_by_ship_slot_ks_distance() {
        let rocks = [rock(0, 80.0, 0.0), rock(1, 40.0, 0.0)];
        // Ship slot 0 (the player) at the start, slot 1 far off: the
        // farther asteroid, in slot 0, wins.
        let ships = |k| match k {
            0 => Some(Vec2::ZERO),
            1 => Some(Vec2::new(500.0, 500.0)),
            _ => None,
        };
        let (index, end) = pick(&right(), &rocks, PLAIN, ships).expect("hit");
        assert_eq!(index, 0);
        assert!(close(end, Vec2::new(74.0, 0.0)), "cut by its own: {end:?}");
        let (index, _) = pick(&right(), &rocks, WRAPPED, ships).expect("hit");
        assert_eq!(index, 1, "by its own distance");
    }

    #[test]
    fn by_the_engine_slot_ks_distance_is_measured_from_the_beams_start_on_both_axes() {
        let mut beam = right();
        beam.start = Vec2::new(10.0, 0.0);
        let rocks = [rock(0, 110.0, 0.0), rock(1, 70.0, 0.0)];
        // Slot 0's ship 30 and 40 off the start (2500), slot 1's 60 below
        // it (3600): the farther asteroid, in slot 0, wins.
        let ships = |k| match k {
            0 => Some(Vec2::new(40.0, 40.0)),
            1 => Some(Vec2::new(10.0, 60.0)),
            _ => None,
        };
        let (index, _) = pick(&beam, &rocks, PLAIN, ships).expect("hit");
        assert_eq!(index, 0);
    }

    #[test]
    fn by_the_engine_a_slot_never_held_weighs_its_asteroid_by_its_own_distance() {
        let rocks = [rock(0, 80.0, 0.0), rock(1, 40.0, 0.0)];
        let only_0 = |k| (k == 0).then_some(Vec2::new(70.0, 0.0));
        // Slot 0 weighs 70² = 4900, slot 1 its own 40² = 1600.
        let (index, _) = pick(&right(), &rocks, PLAIN, only_0).expect("hit");
        assert_eq!(index, 1);
    }

    #[test]
    fn the_segment_takes_the_first_contact_and_ends_there() {
        let rocks = [rock(0, 80.0, 0.0), rock(1, 50.0, 15.0)];
        let (index, end) = pick(&right(), &rocks, SEGMENT, nowhere).expect("hit");
        assert_eq!(index, 1);
        // Radius 20 about (50, 15): meets y = 0 at x = 50 − √175.
        let x = 50.0 - 175.0_f32.sqrt();
        assert!(close(end, Vec2::new(x, 0.0)), "{end:?}");
        let far = [rock(0, 50.0, 25.0)];
        assert_eq!(pick(&right(), &far, SEGMENT, nowhere), None, "passes by");
        let mut moved = right();
        moved.start = Vec2::new(10.0, 5.0);
        moved.end = Vec2::new(110.0, 5.0);
        let (_, end) = pick(&moved, &[rock(0, 60.0, 5.0)], SEGMENT, nowhere).expect("hit");
        assert!(close(end, Vec2::new(40.0, 5.0)), "from its start: {end:?}");
    }

    #[test]
    fn a_point_defence_beam_and_a_seeker_0x0001_beam_reach_no_asteroid() {
        assert!(reaches_asteroids(&right()));
        let mut held = right();
        held.aiming = Aiming::Shot(ShotId(3));
        assert!(!reaches_asteroids(&held));
        let seeker = beam_of(
            WeaponRecord {
                seeker: 0x0001,
                ..weapon(146)
            },
            90.0,
        );
        assert!(!reaches_asteroids(&seeker));
    }

    /// A firer of `mass`, not planet-type.
    fn firer(mass: f32) -> Body {
        Body {
            mass,
            ..Body::default()
        }
    }

    #[test]
    fn no_impact_or_no_mass_moves_nothing() {
        assert_eq!(moves(0.0, 20, Some(&firer(5.0))), Moves::Still);
        assert_eq!(moves(10.0, 0, Some(&firer(5.0))), Moves::Still);
        assert_eq!(moves(-10.0, -1, Some(&firer(5.0))), Moves::Still);
        assert_eq!(moves(10.0, 20, Some(&firer(5.0))), Moves::Push);
    }

    #[test]
    fn a_tractor_pulls_its_firer_only_while_half_the_asteroids_mass_is_above_the_firers() {
        assert_eq!(moves(-10.0, 21, Some(&firer(10.0))), Moves::Pull);
        assert_eq!(
            moves(-10.0, 20, Some(&firer(10.0))),
            Moves::Push,
            "equal: the asteroid is pulled"
        );
        assert_eq!(moves(-10.0, 20, Some(&firer(10.5))), Moves::Push);
        assert_eq!(ASTEROID_PULL_SHARE, 0.5);
    }

    #[test]
    fn a_massless_planet_type_or_absent_firer_pulls_the_asteroid() {
        assert_eq!(moves(-10.0, 100, Some(&firer(0.0))), Moves::Push);
        let planet = Body {
            planet_type: true,
            ..firer(5.0)
        };
        assert_eq!(moves(-10.0, 100, Some(&planet)), Moves::Push);
        assert_eq!(moves(-10.0, 100, None), Moves::Push);
    }
}
