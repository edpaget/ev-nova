//! The session's side of the beams on asteroids (see
//! [`asteroid_beam`](crate::combat::asteroid_beam)): `_HandleBeams`'
//! asteroid pass, which [`Session::beams_phase`] runs on the field it
//! lends the fight, and the boxes of each asteroid the beams destroyed.
//!
//! **The field.** The beams' phase takes the field out of the session for
//! the phase and lends it to the fight whole
//! ([`Combat::close_tick_in`](crate::combat::Combat::close_tick_in)), so a
//! beam destroys an asteroid at once, as `_DestroyAsteroid` (@0x31a69)
//! does inside the pass, and a later beam meets the fragments. Its
//! particles and explosion are raised in the fight's events as it is
//! destroyed.
//!
//! **The boxes** ([`Space::settle_asteroid_remains`]) are spawned after the
//! phase from what each asteroid left
//! ([`Combat::take_asteroid_remains`](crate::combat::Combat::take_asteroid_remains)),
//! where `_DestroyAsteroid` spawns them inside the pass (@0xae52-0xae73).
//! No beam reads a box, and both settle before the frame is drawn, so the
//! difference crosses no boundary the player sees.
//!
//! **The order.** `_DoPlayGameWork` runs `_HandleAsteroids` (@0x444f5)
//! before `_HandleBeams` (@0x4453d), so the beams meet each asteroid where
//! this pass's step has just moved it (see the `game_work` module).

use super::Space;
use crate::asteroid::Remains;
use crate::chance::Chance;
#[cfg(doc)]
use crate::session::Session;

impl Space {
    /// Spawns the scoopable boxes of each asteroid the beams or the shots
    /// destroyed since they were last settled, in the order they were
    /// destroyed, drawing on `chance`: after the beams' phase (see the
    /// module docs) and after the collision passes (see the
    /// `asteroid_hits` module). Spawning them after the phase rather than
    /// inside the hit, as `_DestroyAsteroid` does (@0xae52-0xae73), crosses
    /// no boundary the player sees: no hit of the phase reads a box, and
    /// they spawn before `_HandleBoxes` and the frame.
    pub(super) fn settle_asteroid_remains(&mut self, chance: &mut (impl Chance + ?Sized)) {
        for remains in self.combat.take_asteroid_remains() {
            self.spawn_asteroid_boxes(&remains, chance);
        }
    }

    /// Spawns the scoopable boxes a destroyed asteroid's `remains` drop,
    /// where it was, drawing on `chance`
    /// ([`CargoBoxes::spawn_scoopable`](crate::cargo_box::CargoBoxes::spawn_scoopable)).
    pub(super) fn spawn_asteroid_boxes(
        &mut self,
        remains: &Remains,
        chance: &mut (impl Chance + ?Sized),
    ) {
        for _ in 0..remains.boxes {
            self.boxes
                .spawn_scoopable(remains.at, remains.contents, remains.graphic, chance);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::NovaAi;
    use crate::asteroid::Asteroid;
    use crate::catalog::{AsteroidTypeRecord, WeaponId, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::impact::TractorLock;
    use crate::combat::weapon::Explosion;
    use crate::combat::{CombatEvent, ParticleBurst, Rules, ShipRef};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::session::tests::{arming, blaster};
    use crate::testkit::FakePilotCatalog;

    /// A 200-pixel beam lasting 3 ticks, doing 5 mass damage, of `Impact`
    /// `impact`, exploding in `bööm` 130 at `BlastRadius` 7, its hits
    /// throwing off 6 orange particles of `HitPartVel` 250 and
    /// `HitPartLife` 40.
    fn cutter(impact: i16) -> WeaponRecord {
        WeaponRecord {
            guidance: 0,
            count: 3,
            reload: 10,
            speed: 0,
            beam_length: 200,
            mass_dmg: 5,
            energy_dmg: 0,
            impact,
            explod_type: 2,
            blast_radius: 7,
            hit_particles: 6,
            hit_part_vel: 250,
            hit_part_life: 40,
            hit_part_color: 0x00FF_8000,
            ..blaster()
        }
    }

    /// Asteroid type 0: 40 x 30, of `Mass` `mass`, breaking into 2
    /// fragments of type 1, dropping 4 boxes of commodity 2 on average, 12
    /// particles; type 1, 20 x 20, of `Strength` 50.
    fn rock_types(mass: i16) -> Vec<AsteroidTypeRecord> {
        vec![
            AsteroidTypeRecord {
                kind: 0,
                strength: 100,
                frames: 1,
                frame_size: Some((40, 30)),
                mass,
                yield_type: 2,
                yield_qty: 4,
                part_count: 12,
                part_color: 0x00FF_8000,
                frag_types: [129, -1],
                frag_count: 4,
                ..AsteroidTypeRecord::default()
            },
            AsteroidTypeRecord {
                kind: 1,
                strength: 50,
                frames: 1,
                frame_size: Some((20, 20)),
                ..AsteroidTypeRecord::default()
            },
        ]
    }

    /// The player armed with `beam`, asteroid types [`rock_types`] of
    /// `mass`, and no field.
    fn armed_with(beam: WeaponRecord, mass: i16) -> FakePilotCatalog {
        let npc_gun = WeaponRecord {
            id: WeaponId(140),
            ..blaster()
        };
        FakePilotCatalog {
            asteroid_types: rock_types(mass),
            asteroid_fields: Vec::new(),
            ..arming(beam, npc_gun, 0)
        }
    }

    /// `catalog`'s session in flight by `rules`, the player at the centre
    /// facing up, of `Mass` 40, and no traffic.
    fn flying(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        assert_eq!(session.landed(), None);
        session.space.traffic_due = false;
        session.ship.fit.hull.mass = 40.0;
        session
    }

    /// Puts an asteroid of `kind` and `strength` at (`x`, `y`) moving at
    /// `velocity` in `session`'s field; its slot.
    fn put(session: &mut Session, kind: u8, strength: i16, at: Vec2, velocity: Vec2) -> usize {
        session
            .space
            .asteroids
            .place(Asteroid {
                position: at,
                velocity,
                frame: 0.0,
                spin: 0.0,
                strength,
                kind,
            })
            .expect("a free slot")
    }

    fn rock(session: &Session, slot: usize) -> Option<Asteroid> {
        session.space.asteroids.get(slot).copied()
    }

    /// One advance pass of `session`'s game work: the events.
    fn pass(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<CombatEvent> {
        let rescued = session.game_work(
            Controls::default(),
            catalog,
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
        session.take_combat_events()
    }

    /// Double time's second pass of `session`: the events.
    fn again(session: &mut Session) -> Vec<CombatEvent> {
        session.tick_again(
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        session.take_combat_events()
    }

    /// The player fires its beam on one pass of `session`, its trigger let
    /// go after: the events.
    fn fire(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<CombatEvent> {
        session.hold_fire(true, false);
        let events = pass(session, catalog);
        session.hold_fire(false, false);
        events
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    /// The player's beam's end.
    fn end(session: &Session) -> Vec2 {
        let beam = session
            .beams()
            .iter()
            .find(|beam| beam.firer == ShipRef::Player)
            .expect("a beam");
        beam.end
    }

    /// The explosions and particle bursts among `events`.
    fn hits(events: &[CombatEvent]) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    CombatEvent::Exploded { .. } | CombatEvent::Particles(_)
                )
            })
            .copied()
            .collect()
    }

    #[test]
    fn a_beam_is_cut_short_of_an_asteroid_and_explodes_and_sprays_there_each_pass() {
        let catalog = armed_with(cutter(0), 20);
        let mut session = flying(&catalog, Rulebook::default());
        put(&mut session, 0, 1000, Vec2::new(0.0, -100.0), Vec2::ZERO);
        let first = fire(&mut session, &catalog);
        // trunc(√(100²) − 0.2 × 30) along the beam, straight up.
        let at = Vec2::new(0.0, -94.0);
        assert!(close(end(&session), at), "{:?}", end(&session));
        let second = pass(&mut session, &catalog);
        assert!(close(end(&session), at), "{:?}", end(&session));
        for events in [first, second] {
            let seen = hits(&events);
            let [
                CombatEvent::Exploded {
                    at: boom,
                    explosion,
                    size,
                },
                CombatEvent::Particles(burst),
            ] = seen[..]
            else {
                panic!("{seen:?}");
            };
            assert!(close(boom, at), "{boom:?}");
            assert_eq!(explosion, Explosion::decode(2).expect("bööm 130"));
            assert_eq!(size, 7.0, "its BlastRadius");
            assert_eq!(
                burst,
                ParticleBurst {
                    at: boom,
                    speed: (250.0_f64 * 0.01) as f32,
                    variation: 25,
                    life: (40, 50),
                    color: 0x00FF_8000,
                    count: 6,
                    spread: 0.0,
                }
            );
        }
        assert_eq!(rock(&session, 0).expect("there").strength, 990);
    }

    #[test]
    fn a_beam_wears_an_asteroid_down_by_its_mass_damage_or_ten_times_it() {
        for (flags2, left) in [(0, 95), (0x8000, 50)] {
            let catalog = armed_with(
                WeaponRecord {
                    energy_dmg: 30,
                    flags2,
                    ..cutter(0)
                },
                20,
            );
            let mut session = flying(&catalog, Rulebook::default());
            put(&mut session, 0, 100, Vec2::new(0.0, -100.0), Vec2::ZERO);
            fire(&mut session, &catalog);
            assert_eq!(
                rock(&session, 0).expect("there").strength,
                left,
                "{flags2:#x}"
            );
        }
    }

    #[test]
    fn in_double_time_the_advance_pass_does_twice_the_damage_and_the_second_none() {
        let catalog = armed_with(cutter(0), 20);
        let mut session = flying(&catalog, Rulebook::default());
        session.set_double_time_key(true);
        put(&mut session, 0, 100, Vec2::new(0.0, -100.0), Vec2::ZERO);
        let events = fire(&mut session, &catalog);
        assert_eq!(rock(&session, 0).expect("there").strength, 90);
        assert_eq!(hits(&events).len(), 2);
        let events = again(&mut session);
        assert_eq!(rock(&session, 0).expect("there").strength, 90, "no hit");
        assert_eq!(hits(&events), [], "{events:?}");
    }

    #[test]
    fn an_asteroid_worn_to_0_stands_and_one_below_0_breaks_into_fragments_and_boxes() {
        let catalog = armed_with(cutter(0), 20);
        let mut session = flying(&catalog, Rulebook::default());
        put(&mut session, 0, 5, Vec2::new(0.0, -100.0), Vec2::ZERO);
        fire(&mut session, &catalog);
        assert_eq!(rock(&session, 0).expect("still there").strength, 0);
        assert!(session.cargo_boxes().is_empty());
        let events = pass(&mut session, &catalog);
        assert_eq!(rock(&session, 0), None, "its slot freed");
        // FragCount / 2 + Rand(FragCount): 2 of type 1, where it was.
        let fragments: Vec<Asteroid> = session.asteroids().copied().collect();
        assert_eq!(fragments.len(), 2, "{fragments:?}");
        assert!(fragments.iter().all(|made| made.kind == 1));
        // 4 × (Rand(101) + 50) %: 2 boxes of commodity 2.
        let boxes = session.cargo_boxes();
        assert_eq!(boxes.len(), 2, "{boxes:?}");
        assert!(boxes.iter().all(|made| made.contents == Some(2)));
        let debris = events
            .iter()
            .any(|event| matches!(event, CombatEvent::Particles(burst) if burst.count == 12));
        assert!(debris, "its particles: {events:?}");
    }

    #[test]
    fn a_beam_of_positive_impact_pushes_the_asteroid_along_it_up_to_2_a_tick() {
        for (impact, pushed) in [(10, -0.5), (100, -2.0)] {
            let catalog = armed_with(cutter(impact), 20);
            let mut session = flying(&catalog, Rulebook::default());
            put(&mut session, 0, 100, Vec2::new(0.0, -100.0), Vec2::ZERO);
            fire(&mut session, &catalog);
            let velocity = rock(&session, 0).expect("there").velocity;
            assert!(
                close(velocity, Vec2::new(0.0, pushed)),
                "{impact}: {velocity:?}"
            );
            assert_eq!(session.player().velocity, Vec2::ZERO);
        }
    }

    #[test]
    fn a_tractor_on_an_asteroid_over_twice_the_ships_mass_pulls_the_ship_and_holds_it() {
        // Mass 100 × 0.5 = 50 above the player's 40.
        let catalog = armed_with(cutter(-10), 100);
        let mut session = flying(&catalog, Rulebook::default());
        put(&mut session, 0, 100, Vec2::new(0.0, -100.0), Vec2::ZERO);
        fire(&mut session, &catalog);
        // −10 / 40 along the bearing from the asteroid down to the start.
        let velocity = session.player().velocity;
        assert!(close(velocity, Vec2::new(0.0, -0.25)), "{velocity:?}");
        assert_eq!(session.tractor(), Some(TractorLock::by(ShipRef::Player)));
        assert_eq!(rock(&session, 0).expect("there").velocity, Vec2::ZERO);
    }

    #[test]
    fn a_tractor_on_an_asteroid_of_twice_the_ships_mass_pulls_the_asteroid_instead() {
        // Mass 80 × 0.5 = 40, not above the player's 40.
        let catalog = armed_with(cutter(-10), 80);
        let mut session = flying(&catalog, Rulebook::default());
        put(&mut session, 0, 100, Vec2::new(0.0, -100.0), Vec2::ZERO);
        fire(&mut session, &catalog);
        let velocity = rock(&session, 0).expect("there").velocity;
        assert!(
            close(velocity, Vec2::new(0.0, 0.125)),
            "back along it: {velocity:?}"
        );
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.tractor(), None);
    }

    #[test]
    fn a_tractor_starting_within_50_of_the_asteroid_on_both_axes_locks_but_never_moves_the_ship() {
        for (y, moved) in [(-49.5, false), (-50.0, true)] {
            let catalog = armed_with(cutter(-10), 100);
            let mut session = flying(&catalog, Rulebook::default());
            put(&mut session, 0, 100, Vec2::new(0.0, y), Vec2::ZERO);
            fire(&mut session, &catalog);
            let velocity = session.player().velocity;
            assert_eq!(velocity != Vec2::ZERO, moved, "{y}: {velocity:?}");
            assert_eq!(
                session.tractor(),
                Some(TractorLock::by(ShipRef::Player)),
                "{y}"
            );
        }
    }

    #[test]
    fn a_seeker_0x0001_beam_passes_the_asteroids_at_full_length() {
        let catalog = armed_with(
            WeaponRecord {
                seeker: 0x0001,
                ..cutter(10)
            },
            20,
        );
        let mut session = flying(&catalog, Rulebook::default());
        put(&mut session, 0, 100, Vec2::new(0.0, -100.0), Vec2::ZERO);
        let events = fire(&mut session, &catalog);
        assert!(close(end(&session), Vec2::new(0.0, -200.0)));
        assert_eq!(hits(&events), [], "{events:?}");
        let left = rock(&session, 0).expect("there");
        assert_eq!((left.strength, left.velocity), (100, Vec2::ZERO));
    }

    /// The strengths of slots 0 and 1, an asteroid 150 above the player
    /// in slot 0 and one 60 above in slot 1, after the player's beam
    /// fires once by `rules`.
    fn two_in_line(rules: Rulebook) -> (i16, i16) {
        let catalog = armed_with(cutter(0), 20);
        let mut session = flying(&catalog, rules);
        put(&mut session, 0, 100, Vec2::new(0.0, -150.0), Vec2::ZERO);
        put(&mut session, 0, 100, Vec2::new(0.0, -60.0), Vec2::ZERO);
        fire(&mut session, &catalog);
        let strength = |slot| rock(&session, slot).expect("there").strength;
        (strength(0), strength(1))
    }

    #[test]
    fn the_nearest_asteroid_is_by_its_own_distance_by_default_and_by_the_ships_slots_by_the_engine()
    {
        assert_eq!(two_in_line(Rulebook::default()), (100, 95), "the nearer");
        // Slot 0 weighs by the player's distance, none; slot 1 by its own,
        // as no ship ever held slot 1.
        let engine = Rulebook::default().with_override(RuleKey::BeamConeQuirks, RuleSource::Engine);
        assert_eq!(two_in_line(engine), (95, 100), "slot 0's");
    }

    #[test]
    fn by_the_segment_a_beam_ends_where_it_meets_the_asteroids_circle() {
        let segment = Rulebook::default().with_override(RuleKey::BeamHitTest, RuleSource::Bible);
        let catalog = armed_with(cutter(0), 20);
        let mut session = flying(&catalog, segment);
        // Radius 20, 15 off the beam: met √(20² − 15²) short of level.
        put(&mut session, 0, 100, Vec2::new(15.0, -100.0), Vec2::ZERO);
        fire(&mut session, &catalog);
        let y = -100.0 + 175.0_f32.sqrt();
        assert!(
            close(end(&session), Vec2::new(0.0, y)),
            "{:?}",
            end(&session)
        );
        assert_eq!(rock(&session, 0).expect("there").strength, 95);
    }

    #[test]
    fn a_beam_meets_an_asteroid_where_this_passs_step_has_just_moved_it() {
        // `_HandleAsteroids` (@0x444f5) runs before `_HandleBeams`
        // (@0x4453d): 130 up is past the reach, 100 + 15; stepped 30
        // nearer, it is not.
        let catalog = armed_with(
            WeaponRecord {
                beam_length: 100,
                ..cutter(0)
            },
            20,
        );
        let mut session = flying(&catalog, Rulebook::default());
        put(
            &mut session,
            0,
            100,
            Vec2::new(0.0, -130.0),
            Vec2::new(0.0, 30.0),
        );
        fire(&mut session, &catalog);
        let met = rock(&session, 0).expect("there");
        assert_eq!(met.position, Vec2::new(0.0, -100.0));
        assert_eq!(met.strength, 95, "met where the step left it");
        assert!(close(end(&session), Vec2::new(0.0, -94.0)));
    }

    #[test]
    fn in_double_time_the_second_pass_steps_the_asteroid_and_the_next_advance_pass_meets_it_stepped()
     {
        let catalog = armed_with(
            WeaponRecord {
                beam_length: 100,
                count: 6,
                ..cutter(0)
            },
            20,
        );
        let mut session = flying(&catalog, Rulebook::default());
        session.set_double_time_key(true);
        put(
            &mut session,
            0,
            100,
            Vec2::new(0.0, -190.0),
            Vec2::new(0.0, 30.0),
        );
        fire(&mut session, &catalog);
        assert_eq!(rock(&session, 0).expect("there").strength, 100, "160 off");
        again(&mut session);
        let stepped = rock(&session, 0).expect("there");
        assert_eq!(stepped.position, Vec2::new(0.0, -130.0), "stepped again");
        assert_eq!(stepped.strength, 100, "no beam acts on it");
        pass(&mut session, &catalog);
        let met = rock(&session, 0).expect("there");
        assert_eq!(met.position, Vec2::new(0.0, -100.0));
        assert_eq!(met.strength, 90, "met after this pass's step, doubled");
    }
}
