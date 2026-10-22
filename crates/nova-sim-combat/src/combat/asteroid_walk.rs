//! The asteroid field as the collision passes meet it (rdm asteroids
//! phase 10): which asteroids each contact walk visits, what a walk does
//! with an asteroid it destroyed, and a shot's hit destroying one at
//! once, as `_HandleAsteroidHit` does.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`:
//!
//! - **The hit destroys at once.** `_HandleAsteroidHit` (@0x36637-0x369c7)
//!   calls `_DestroyAsteroid` (@0x3691a) as soon as the strength is below
//!   0, inside the collision passes (`_AsteroidCollisionHandler`
//!   @0x369c8-0x36a44, from `_SWCollideSpriteLayer` @0x43eec, @0x43f62,
//!   @0x43fd8, @0x43ffa) and `_CheckShotProximities` (@0x37744). Its boxes,
//!   particles, explosion and fragments come then, before the walk's next
//!   pair. Neither routine plays a sound but the explosions'.
//! - **A fragment's sprite stays hidden.** `_SpawnSubAsteroid`
//!   (@0x3fbc7-0x3fd21) sets only the slot's struct (+0x20 @0x3fc01), and
//!   never shows or moves its sprite; only `_HandleAsteroids` (@0x3638a,
//!   run @0x444f5, after every pass) shows it. `_SWCollideSpriteLayer`
//!   skips a hidden outer sprite (@0xacba3), so no contact walk meets a
//!   fragment seated in the same passes, in a higher slot of the walk in
//!   progress or in a later layer's walk. The walks visit the slots active
//!   as the passes begin, less each one destroyed in an earlier walk
//!   (`Shown`). The proximity pass reads only the slot's +0x20 and
//!   position (@0x376dd-0x37744), so a fuse there is set off by a
//!   fragment seated earlier in the passes, and never by a destroyed slot.
//! - **A destroyed asteroid goes on meeting its walk.** The walk reads the
//!   outer sprite's visibility once, as its turn begins (@0xacba3), and
//!   not again after its proc (@0xacd5b-0xacd6d); nothing on the hit's way
//!   tests +0x20. So each later shot of that walk that touches it is spent
//!   on it: the hit's explosion, particles and blast go off, the strength
//!   drops again (@0x368e5-0x3690d), and below 0 `_DestroyAsteroid` runs
//!   again on the inactive struct, leaving more boxes, particles, an
//!   explosion and fragments ([`Asteroids::destroy_ghost`]).
//! - **A fragment in its own slot.** On that second destruction the slot
//!   is clear, so its first fragment may be seated there, overwriting the
//!   struct, and the trailing clear (@0xb008-0xb01c) loses it. A third hit
//!   of the walk reads the overwritten struct: the fragment's strength
//!   less the hit, its kind's `Mass` and fragments. The sprite keeps the
//!   parent's frames until `_HandleAsteroids` clones new ones (@0x363a8),
//!   so the walk's contact shape stays the parent's.
//!
//! # The engine
//!
//! The session lends the passes the field ([`AsteroidField`]), as it lends
//! it to the beams. Each walk takes its turns from `Shown`, in slot
//! order; a turn (`WalkTurn`) keeps the asteroid's contact circle as the
//! turn began, and once the asteroid is destroyed, a copy of its struct,
//! the ghost, which the rest of the walk's hits wear and destroy again.
//! The boxes are left for the session ([`Combat::take_asteroid_remains`]).
//!
//! Left out: the strength's 16-bit wrap (@0x368e5-0x3690d); the engine
//! holds it within an `i16` instead ([`asteroid_hit::damaged`]). The
//! circle stands in for the parent's sprite mask (`_SWRLECollision`), as
//! it does for every asteroid.
//!
//! [`Asteroids::destroy_ghost`]: crate::asteroid::Asteroids::destroy_ghost

use super::asteroid_beam::Rock;
use super::asteroid_hit;
use super::damage::Hit;
use super::ion::{Burst, Dose};
use super::projectile::Shot;
use super::{Combat, CombatEvent, Fighter, Landed, Push};
use crate::asteroid::{Asteroid, AsteroidType, Asteroids, KINDS, Remains, SLOTS};
use crate::chance::Chance;
use crate::geometry::Vec2;

/// The asteroid field a phase of the fight meets, lent by the session:
/// its slots, written as the shots and the beams wear and destroy them,
/// and every type.
#[derive(Debug)]
pub struct AsteroidField<'a> {
    /// The field's slots.
    pub asteroids: &'a mut Asteroids,
    /// Every asteroid type, by kind.
    pub types: &'a [AsteroidType; KINDS],
}

/// Every type missing: the types of a fight lent no field.
pub(super) static NO_TYPES: [AsteroidType; KINDS] = [AsteroidType::MISSING; KINDS];

/// The slots whose sprites the contact walks visit: those active as the
/// collision passes begin, less each one destroyed in a walk since (see
/// the module docs). A fragment seated in the passes never joins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Shown([bool; SLOTS]);

impl Shown {
    /// The slots of `asteroids` active as the passes begin.
    pub(super) fn of(asteroids: &Asteroids) -> Self {
        let mut shown = [false; SLOTS];
        for (slot, _) in asteroids.indexed() {
            shown[slot] = true;
        }
        Self(shown)
    }

    /// Whether the walks visit `slot`.
    pub(super) fn has(&self, slot: usize) -> bool {
        self.0.get(slot).copied().unwrap_or(false)
    }

    /// Hides `slot`, destroyed: no later walk visits it (@0xb01c).
    pub(super) fn hide(&mut self, slot: usize) {
        if let Some(shown) = self.0.get_mut(slot) {
            *shown = false;
        }
    }
}

/// One asteroid's turn as a walk's outer sprite: its contact circle as
/// the turn began, and once a hit of the walk destroyed it, its struct,
/// the ghost, which the walk's later hits meet (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WalkTurn {
    /// Its slot, centre and hit radius as the turn began.
    pub(super) rock: Rock,
    /// Its struct once destroyed: none while it stands.
    ghost: Option<Asteroid>,
}

impl WalkTurn {
    /// The turn of `asteroid` in `slot`, its circle read from `types`.
    pub(super) fn begin(slot: usize, asteroid: &Asteroid, types: &[AsteroidType; KINDS]) -> Self {
        Self {
            rock: Rock::of(slot, asteroid, types),
            ghost: None,
        }
    }

    /// Whether a hit of the walk has destroyed it.
    pub(super) fn destroyed(&self) -> bool {
        self.ghost.is_some()
    }
}

/// What a shot's hit lands on: a slot of the field, met by a fuse, or the
/// outer asteroid of a contact walk's turn, which may be destroyed.
#[derive(Debug)]
pub(super) enum Struck<'t> {
    /// The asteroid in this slot, active.
    Slot(usize),
    /// A contact walk's turn.
    Turn(&'t mut WalkTurn),
}

/// Wears `asteroid` by `shot`'s hit, as `_HandleAsteroidHit` does
/// (@0x368e5-0x36996): its damage taken off, and if that leaves it
/// standing, pushed by the weapon's `Impact` over its kind's `Mass`, read
/// from `types`. Whether it is broken.
fn wear(asteroid: &mut Asteroid, shot: &Shot, types: &[AsteroidType; KINDS]) -> bool {
    let weapon = &shot.weapon;
    asteroid.strength = asteroid_hit::damaged(asteroid.strength, weapon);
    if asteroid_hit::broken(asteroid.strength) {
        return true;
    }
    let mass = types
        .get(usize::from(asteroid.kind))
        .map_or(0, |kind_type| kind_type.mass);
    if weapon.impact != 0.0 && mass > 0 {
        asteroid.velocity =
            asteroid_hit::push(asteroid.velocity, shot.heading, weapon.impact, mass);
    }
    false
}

impl Combat {
    /// `shot`'s hit on what it `struck` at `at` (`_HandleAsteroidHit`
    /// @0x36637-0x369c7, see [`asteroid_hit`]): its explosion and its hit
    /// particles go off there; a player's shot blasts the active ships
    /// among `fighters` in its square, but planet-type ones, into `hits`;
    /// the asteroid takes its damage and, below 0, is destroyed at once
    /// in `field`, drawing on `chance` ([`Combat::destroy_asteroid`]), or
    /// else is pushed. A walk's turn destroyed already takes the hit on
    /// its ghost, destroyed again below 0 (see the module docs). The shot
    /// is used up, and releases nothing.
    ///
    /// The blast's hits are `_DamageShip`'s as @0x36888 passes them: by the
    /// shot's source, neither direct nor intended, can't-kill as the shot is
    /// disable-only, pushing by the weapon's `Impact` from the blast's
    /// centre with no source gate, and ionizing over its circle
    /// (`_IonizeShip` @0x368b7), as a shot's own blast does.
    pub(super) fn hit_asteroid(
        &mut self,
        (shot, struck, at): (&Shot, Struck<'_>, Vec2),
        fighters: &[Fighter],
        hits: &mut Vec<Landed>,
        (field, chance): (&mut AsteroidField<'_>, &mut dyn Chance),
    ) {
        let weapon = &shot.weapon;
        if let Some(explosion) = weapon.explosion {
            self.events.push(CombatEvent::Exploded {
                at,
                explosion,
                size: weapon.blast_radius,
            });
        }
        self.spray(weapon, at, asteroid_hit::HIT_PART_VARIATION);
        if let Some(blast) = asteroid_hit::blast(shot, at) {
            let ships = fighters
                .iter()
                .filter(|fighter| fighter.condition.active() && !fighter.hull.planet_type)
                .map(|fighter| (fighter.ship, fighter.state.position));
            let hit = Hit {
                cant_kill: shot.disable_only,
                ..Hit::of(weapon)
            };
            hits.extend(blast.reaches(ships).into_iter().map(|ship| Landed {
                ship,
                by: shot.attacker(),
                weapon: Some(weapon.id),
                at: blast.at,
                hit,
                direct: false,
                intended: false,
                lock: false,
                weight: 0.0,
                push: Push {
                    from: blast.at,
                    impact: weapon.impact,
                    beam: false,
                },
                ion: Some(Dose::of(
                    weapon,
                    Some(Burst {
                        centre: blast.at,
                        radius: blast.radius,
                    }),
                )),
            }));
        }
        match struck {
            Struck::Slot(index) => {
                self.wear_slot(index, shot, field, chance);
            }
            Struck::Turn(turn) => {
                let index = turn.rock.slot;
                if let Some(ghost) = &mut turn.ghost {
                    if wear(ghost, shot, field.types) {
                        let remains =
                            field
                                .asteroids
                                .destroy_ghost(index, ghost, field.types, chance);
                        self.keep_remains(remains);
                    }
                } else {
                    turn.ghost = self.wear_slot(index, shot, field, chance);
                }
            }
        }
    }

    /// Wears the asteroid in slot `index` of `field` by `shot`'s hit, and
    /// below 0 destroys it at once, drawing on `chance`: its struct as it
    /// was destroyed, if it was.
    fn wear_slot(
        &mut self,
        index: usize,
        shot: &Shot,
        field: &mut AsteroidField<'_>,
        chance: &mut dyn Chance,
    ) -> Option<Asteroid> {
        let asteroid = field.asteroids.get_mut(index)?;
        if !wear(asteroid, shot, field.types) {
            return None;
        }
        let destroyed = *asteroid;
        self.destroy_asteroid(index, field, chance);
        Some(destroyed)
    }

    /// Destroys the asteroid in `slot` of `field` at once, drawing on
    /// `chance` (`_DestroyAsteroid`, from `_HandleAsteroidHit` @0x3691a or
    /// `_HandleBeams` @0x31a69,
    /// [`Asteroids::destroy`](crate::asteroid::Asteroids::destroy)): its
    /// fragments go into the field, its particles and explosion are
    /// raised, and what it left is kept for the session's boxes.
    pub(super) fn destroy_asteroid(
        &mut self,
        slot: usize,
        field: &mut AsteroidField<'_>,
        chance: &mut dyn Chance,
    ) {
        if let Some(remains) = field.asteroids.destroy(slot, field.types, chance) {
            self.keep_remains(remains);
        }
    }

    /// Raises a destroyed asteroid's particles, then its explosion, as
    /// `_DestroyAsteroid` makes them (@0xaf3a, @0xaf7f), and keeps its
    /// `remains` for the session's boxes.
    fn keep_remains(&mut self, remains: Remains) {
        if let Some(particles) = remains.particles {
            self.particles(particles);
        }
        if let Some(explosion) = remains.explosion {
            self.explode(remains.at, explosion);
        }
        self.asteroid_remains.push(remains);
    }

    /// What each asteroid the shots and the beams destroyed left, since it
    /// was last taken, in the order they were destroyed; taking it
    /// empties the list. Its particles and explosion are raised already;
    /// the session spawns its boxes (`nova_sim::Session::beams_phase`,
    /// `nova_sim::Session::collision_passes`).
    pub fn take_asteroid_remains(&mut self) -> Vec<Remains> {
        std::mem::take(&mut self.asteroid_remains)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn kind(kind: u8, at: Vec2) -> Asteroid {
        Asteroid {
            position: at,
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 5,
            kind,
        }
    }

    #[test]
    fn the_walks_visit_the_slots_active_as_the_passes_begin_less_those_hidden() {
        let mut field = Asteroids::default();
        field.seat(2, kind(0, Vec2::ZERO));
        field.seat(7, kind(0, Vec2::ZERO));
        let mut shown = Shown::of(&field);
        let visited = |shown: &Shown| (0..SLOTS).filter(|&s| shown.has(s)).collect::<Vec<_>>();
        assert_eq!(visited(&shown), [2, 7]);
        field.seat(0, kind(1, Vec2::ZERO));
        assert_eq!(visited(&shown), [2, 7], "a later seat never joins");
        shown.hide(2);
        assert_eq!(visited(&shown), [7]);
        shown.hide(SLOTS);
        assert!(!shown.has(SLOTS), "past the field");
    }

    #[test]
    fn a_turn_keeps_the_circle_its_asteroid_had_as_it_began() {
        let mut types = [AsteroidType::MISSING; KINDS];
        types[3].size = (40.0, 30.0);
        let turn = WalkTurn::begin(6, &kind(3, Vec2::new(1.0, 2.0)), &types);
        assert_eq!(turn.rock.slot, 6);
        assert_eq!(turn.rock.position, Vec2::new(1.0, 2.0));
        assert_eq!(turn.rock.radius, 20.0);
        assert!(!turn.destroyed());
    }
}
