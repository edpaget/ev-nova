//! The session's side of the shots on asteroids (see
//! [`asteroid_hit`](crate::combat::asteroid_hit) and
//! [`asteroid_walk`](crate::combat::asteroid_walk)), rdm asteroids phase 10.
//!
//! **The field.** [`Session::collision_passes`] takes the field out of the
//! session for the passes and lends it to the fight whole, as the beams'
//! phase does (see the `asteroid_beams` module). So a shot destroys an
//! asteroid at once, as `_HandleAsteroidHit` does (@0x3691a): its
//! particles and explosion are raised in the fight's events before the
//! next hit's, and its fragments are seated in the field. They are
//! hidden from every contact walk of the passes, as their sprites are
//! until `_HandleAsteroids` (@0x3638a), but a proximity fuse later in the
//! passes is set off by one (`_CheckShotProximities` @0x376dd-0x37744).
//!
//! **The boxes** are spawned after the passes from what each asteroid
//! left ([`Space::settle_asteroid_remains`]), where `_DestroyAsteroid`
//! spawns them inside the hit (@0xae52-0xae73). No pass reads a box: the
//! ships' pass over the boxes, the scooping, comes first
//! (`_SWCollideSpriteLayer` @0x43e70), and the boxes spawn before
//! `_HandleBoxes` (@0x444f0) and the frame, so the difference crosses no
//! boundary the player sees.

#[cfg(doc)]
use super::{Session, Space};

#[cfg(test)]
mod tests {
    use crate::asteroid::Asteroid;
    use crate::catalog::{AsteroidTypeRecord, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::weapon::Explosion;
    use crate::combat::{CombatEvent, Rules};
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::session::tests::{arming, blaster};
    use crate::testkit::FakePilotCatalog;

    /// Kind 0, 20 across, breaking into one fragment of kind 1, dropping
    /// 2 boxes of commodity 2 on average, 12 particles and `bööm` 131;
    /// kind 1, 60 across, of `Strength` 50.
    fn rock_types() -> Vec<AsteroidTypeRecord> {
        vec![
            AsteroidTypeRecord {
                kind: 0,
                strength: 100,
                frames: 1,
                frame_size: Some((20, 20)),
                mass: 20,
                yield_type: 2,
                yield_qty: 4,
                part_count: 12,
                part_color: 0x00FF_8000,
                explode_type: 3,
                frag_types: [129, -1],
                frag_count: 2,
                ..AsteroidTypeRecord::default()
            },
            AsteroidTypeRecord {
                kind: 1,
                strength: 50,
                frames: 1,
                frame_size: Some((60, 60)),
                mass: 20,
                ..AsteroidTypeRecord::default()
            },
        ]
    }

    /// The player armed with a blaster of `ProxRadius` 30, the asteroid
    /// types [`rock_types`], and no field.
    fn armed() -> FakePilotCatalog {
        let gun = WeaponRecord {
            prox_radius: 30,
            ..blaster()
        };
        let npc_gun = WeaponRecord {
            id: crate::catalog::WeaponId(140),
            ..blaster()
        };
        FakePilotCatalog {
            asteroid_types: rock_types(),
            asteroid_fields: Vec::new(),
            ..arming(gun, npc_gun, 0)
        }
    }

    /// `catalog`'s session in flight with no traffic, the player having
    /// fired a shot of its gun for each of `moves`, each then set on that
    /// move, from and to the points given, for the next pass's collision
    /// passes to test.
    fn shots_set(catalog: &FakePilotCatalog, moves: &[(Vec2, Vec2)]) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.space.traffic_due = false;
        session.hold_fire(true, false);
        for _ in 0..10 {
            if session.shots().len() >= moves.len() {
                break;
            }
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.shots().len(), moves.len(), "fired");
        session.hold_fire(false, false);
        session.take_combat_events();
        for (shot, &(from, to)) in session.space.combat.shots_mut().iter_mut().zip(moves) {
            shot.moved_from = Some(from);
            shot.position = to;
        }
        session
    }

    /// Puts a kind-0 asteroid of strength 1 at (0, -100) in `session`'s
    /// field: a blaster's hit destroys it.
    fn frail_rock(session: &mut Session) {
        let slot = session.space.asteroids.place(Asteroid {
            position: Vec2::new(0.0, -100.0),
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 1,
            kind: 0,
        });
        assert_eq!(slot, Some(0));
    }

    /// The first shot's move into the asteroid at (0, -100), meeting its
    /// 10-pixel circle at (0, -110).
    const INTO_THE_ROCK: (Vec2, Vec2) = (Vec2 { x: 0.0, y: -130.0 }, Vec2 { x: 0.0, y: -105.0 });

    #[test]
    fn a_fuse_later_in_the_passes_is_spent_on_the_fragment_a_shot_broke_off() {
        // The first shot destroys the asteroid in the contact walk; its
        // fragment is seated in slot 1, its sprite hidden. The second shot
        // touches nothing on its move, (25, -130) to (25, -112), but its
        // armed fuse reaches the fragment in the same proximity pass.
        let catalog = armed();
        let beside = (Vec2::new(25.0, -130.0), Vec2::new(25.0, -112.0));
        let mut session = shots_set(&catalog, &[INTO_THE_ROCK, beside]);
        frail_rock(&mut session);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.shots(), [], "both spent");
        let left: Vec<(usize, Asteroid)> = session
            .space
            .asteroids
            .indexed()
            .map(|(slot, rock)| (slot, *rock))
            .collect();
        let [(1, fragment)] = left[..] else {
            panic!("only the fragment: {left:?}");
        };
        assert_eq!(fragment.kind, 1);
        assert_eq!(fragment.strength, 45, "the second shot's 5 that pass");
    }

    #[test]
    fn a_contact_shot_touching_only_the_fragment_flies_on_that_pass() {
        // Its sprite is hidden until `_HandleAsteroids` (@0x3638a), so the
        // walk passes it: the second shot, of no `ProxRadius`, crosses its
        // 30-pixel circle and not its parent's 10.
        let catalog = armed();
        let across = (Vec2::new(20.0, -130.0), Vec2::new(20.0, -80.0));
        let mut session = shots_set(&catalog, &[INTO_THE_ROCK, across]);
        for shot in session.space.combat.shots_mut() {
            shot.weapon.prox_radius = 0.0;
        }
        frail_rock(&mut session);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.shots().len(), 1, "the second flies on");
        let fragment = session.space.asteroids.get(1).expect("the fragment");
        assert_eq!(fragment.strength, 50, "untouched");
        assert_eq!(session.space.asteroids.get(0), None, "its parent gone");
    }

    #[test]
    fn a_shots_asteroid_drops_its_boxes_and_raises_its_debris_that_pass() {
        let catalog = armed();
        let mut session = shots_set(&catalog, &[INTO_THE_ROCK]);
        frail_rock(&mut session);
        session.tick_combat(Rules::default(), &mut NeverFires);
        // 4 × (Rand(101) + 50) %: 2 boxes of commodity 2.
        let boxes = session.cargo_boxes();
        assert_eq!(boxes.len(), 2, "{boxes:?}");
        assert!(boxes.iter().all(|made| made.contents == Some(2)));
        let events = session.take_combat_events();
        let boom = Explosion::decode(3);
        let debris: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                CombatEvent::Particles(burst) if burst.count == 12 => Some("particles"),
                CombatEvent::Exploded { explosion, .. } if Some(*explosion) == boom => {
                    Some("exploded")
                }
                _ => None,
            })
            .collect();
        assert_eq!(debris, ["particles", "exploded"], "{events:?}");
    }
}
