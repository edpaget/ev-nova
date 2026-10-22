//! The session's side of [`asteroid`](crate::asteroid): the field about
//! the player, its system's `Asteroids` and `AstTypes` spawning one
//! asteroid at the edge and stepping the field on each game-work pass,
//! and the kills and placements as the player enters a system. The field
//! is never saved.
//!
//! **The pass.** [`Session::tick_asteroids`] is one pass's asteroid work:
//! `_DoPlayGameWork`'s edge spawn (@0x44243) and `_HandleAsteroids`
//! (@0x444f5). [`Session::game_work`] and [`Session::tick_combat`] run
//! it after the fight, landed or jumping, so it steps the field through a
//! jump's lead-in, and [`Session::tick_again`] runs it after the second
//! pass's fight. The
//! original spawns and steps the field after the collision passes, which
//! test the shots' last moves, as the fight's hit pass does here (see
//! [`Combat`](crate::combat::Combat)): so a shot meets an asteroid where
//! the last pass's step left it, where the frame before drew it, and a
//! hit's push moves the asteroid in this pass's step. The spawn comes
//! before the AI and the ships' fire in the original and after them
//! here; a new asteroid is met by no shot until the next pass either way,
//! so that crosses no boundary the player sees.
//!
//! **The shots** (see [`asteroid_hit`](crate::combat::asteroid_hit)).
//! The fight hands its tick the field's asteroids
//! ([`Space::asteroid_targets`]), keeps what the shots leave of them,
//! and destroys those they broke ([`Space::settle_asteroid_hits`]).
//!
//! **The kill.** Every way into a system but the escape pod's rescue kills
//! the field: a jump's arrival (@0x6c4fc), a hypergate (@0x63f47), a
//! wormhole (@0x645f6), a landing (`_HandlePlayerDockRequest` @0x66f57),
//! an in-flight `M` (@0x158ca) or `N` (@0x1598e), and the developer
//! tools' move. `_ResetPlayer` sets no `_killAsteroids`
//! (@0x1d8cd-0x1d8e2), so the rescue keeps it. The original's
//! `_killAsteroids` holds until the end of the frame (@0x463a1), so
//! every pass of that frame kills what it would spawn: here a kill
//! records the pass frame, and a pass on that frame empties the field
//! and spawns nothing. A jump's arrival comes between frames, after the
//! warp, and the next frame's pass spawns as the original's arrival
//! frame spawns after its kill. A move settled inside a frame kills the
//! rest of that frame's passes.
//!
//! **The placed field.** By the other reading of
//! [`RuleKey::AsteroidEntryField`] a jump's arrival places the field
//! `_CreateAsteroids` places (@0x6c51a) and keeps it, emptied of the
//! system left first, in the arrival itself, so the frame drawn after
//! it shows the field. So does a session started or loaded in flight,
//! as `_DoNewPilot` (@0x19384) and the main screen's entries
//! (`_MainScreenChoice` @0x1a8cf, `_DoMainScreen` @0x1aff0) place one:
//! [`Session::settle_asteroid_field`], which the view runs before it
//! draws the first frame. A session starting docked takes off into an
//! empty field, as every take-off does.

use std::collections::BTreeMap;

use super::{Session, Setup, Space, Statics};
use crate::asteroid::{Asteroid, AsteroidType, FieldSpec, Origin, Ring};
use crate::catalog::{AsteroidFieldRecord, SystemId};
use crate::chance::Chance;
use crate::combat::asteroid_hit::AsteroidTarget;
use crate::rulebook::{RuleKey, RuleSource};

/// Each system's field, by system, from `records`.
pub(super) fn fields(records: &[AsteroidFieldRecord]) -> BTreeMap<SystemId, FieldSpec> {
    records
        .iter()
        .map(|record| {
            (
                record.system,
                FieldSpec {
                    count: record.count,
                    types: record.types,
                },
            )
        })
        .collect()
}

impl Session {
    /// The asteroids in the field about the player, in slot order (see
    /// [`asteroid`](crate::asteroid)).
    pub fn asteroids(&self) -> impl Iterator<Item = &Asteroid> {
        self.space.asteroids()
    }

    /// Runs one game-work pass's asteroid work, drawing on `chance`: an
    /// asteroid spawned at the field's edge, on the ring
    /// [`RuleKey::AsteroidRespawnRing`] says, and every asteroid stepped
    /// and culled in the play area (see the module docs). Nothing while
    /// landed; on a frame a kill holds through, the field is emptied.
    pub fn tick_asteroids(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if self.mode.landed().is_some() {
            // The landing's kill is spent: the field stays empty until the
            // take-off, and a later frame's counter can't match it.
            self.space.asteroid_kill = None;
            return;
        }
        self.settle_asteroid_field(chance);
        if self.space.asteroid_kill == Some(self.space.pass_frame) {
            self.space.asteroids.clear();
            return;
        }
        self.space.asteroid_kill = None;
        let ring = match self.setup.rules.source_for(RuleKey::AsteroidRespawnRing) {
            RuleSource::Engine => Ring::Original,
            RuleSource::Bible => Ring::Outside,
        };
        let player = self.ship.player.position;
        self.space.asteroids.spawn(
            Origin::Edge(ring),
            self.field_spec(),
            &self.statics.asteroid_types,
            player,
            self.space.play_area,
            chance,
        );
        self.space
            .asteroids
            .step(&self.statics.asteroid_types, player, self.space.play_area);
    }

    /// Places the field of a session started or loaded in flight, once,
    /// drawing on `chance`, by the other reading of
    /// [`RuleKey::AsteroidEntryField`]; by the engine, and once docked,
    /// it places none (see the module docs). The view runs it before it
    /// draws a frame; [`Session::tick_asteroids`] runs it too.
    pub fn settle_asteroid_field(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if !std::mem::take(&mut self.space.asteroid_field_due) || self.mode.landed().is_some() {
            return;
        }
        if self.setup.places_asteroids() {
            self.place_asteroids(chance);
        }
    }

    /// Places the field of the player's system about the ship afresh,
    /// the field held emptied first, drawing on `chance`: a jump's
    /// arrival by the other reading of [`RuleKey::AsteroidEntryField`]
    /// (see the module docs).
    pub(super) fn place_asteroids(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.space.asteroids.clear();
        self.space.asteroid_kill = None;
        let player = self.ship.player.position;
        self.space.asteroids.place_field(
            self.field_spec(),
            &self.statics.asteroid_types,
            player,
            self.space.play_area,
            chance,
        );
    }

    /// The player's system's field: none for one the catalog doesn't
    /// list.
    fn field_spec(&self) -> FieldSpec {
        self.statics
            .asteroid_fields
            .get(&self.pilot.system)
            .copied()
            .unwrap_or(FieldSpec::NONE)
    }
}

impl Setup {
    /// Whether an arrival keeps the field `_CreateAsteroids` places, as
    /// [`RuleKey::AsteroidEntryField`] reads.
    pub(super) fn places_asteroids(&self) -> bool {
        self.rules.source_for(RuleKey::AsteroidEntryField) == RuleSource::Bible
    }
}

impl Space {
    /// The asteroids in the field about the player, in slot order (see
    /// [`asteroid`](crate::asteroid)).
    fn asteroids(&self) -> impl Iterator<Item = &Asteroid> {
        self.asteroids.iter()
    }

    /// Kills the field, through the rest of the pass frame, as
    /// `_killAsteroids` kills it (see the module docs).
    pub(super) fn kill_asteroids(&mut self) {
        self.asteroids.clear();
        self.asteroid_kill = Some(self.pass_frame);
    }

    /// The asteroids shots may hit this tick, slot by slot, each with its
    /// type's radius and `Mass` (see
    /// [`asteroid_hit`](crate::combat::asteroid_hit)).
    pub(super) fn asteroid_targets(&self, statics: &Statics) -> Vec<AsteroidTarget> {
        self.asteroids
            .indexed()
            .map(|(slot, asteroid)| {
                let kind_type = statics
                    .asteroid_types
                    .get(usize::from(asteroid.kind))
                    .copied()
                    .unwrap_or(AsteroidType::MISSING);
                AsteroidTarget {
                    slot,
                    position: asteroid.position,
                    velocity: asteroid.velocity,
                    radius: kind_type.radius(),
                    strength: asteroid.strength,
                    mass: kind_type.mass,
                }
            })
            .collect()
    }

    /// Keeps the strength and velocity each of `targets` is left with
    /// after the fight's tick, then destroys each asteroid the shots
    /// broke, in the order they broke, drawing on `chance`
    /// ([`Asteroids::destroy`](crate::asteroid::Asteroids::destroy)): its
    /// particles and explosion go off, and its scoopable boxes are
    /// spawned where it was
    /// ([`CargoBoxes::spawn_scoopable`](crate::cargo_box::CargoBoxes::spawn_scoopable)).
    ///
    /// The original destroys an asteroid inside the hit
    /// (`_HandleAsteroidHit` @0x3691a), so its booms and particles come
    /// before the rest of the frame's collisions, and a fragment is in the
    /// field for the shots after it. Here they come after the fight's own
    /// events, and its fragments are hit from the next pass on. Both
    /// settle before the frame is drawn, which is no boundary the player
    /// sees.
    pub(super) fn settle_asteroid_hits(
        &mut self,
        statics: &Statics,
        targets: &[AsteroidTarget],
        chance: &mut (impl Chance + ?Sized),
    ) {
        for target in targets {
            if let Some(asteroid) = self.asteroids.get_mut(target.slot) {
                asteroid.strength = target.strength;
                asteroid.velocity = target.velocity;
            }
        }
        for slot in self.combat.take_broken_asteroids() {
            let Some(remains) = self
                .asteroids
                .destroy(slot, &statics.asteroid_types, chance)
            else {
                continue;
            };
            for _ in 0..remains.boxes {
                self.boxes
                    .spawn_scoopable(remains.at, remains.contents, remains.graphic, chance);
            }
            if let Some(particles) = remains.particles {
                self.combat.particles(particles);
            }
            if let Some(explosion) = remains.explosion {
                self.combat.explode(remains.at, explosion);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::asteroid::SLOTS;
    use crate::catalog::{AsteroidTypeRecord, StellarId};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{at_gate, at_wormhole, gated, wormholes};
    use crate::testkit::{FakePilotCatalog, Scripted, begin_jump_now, catalog, fly_out, land_now};

    /// Every roll its last outcome.
    struct Last;

    impl Chance for Last {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n.saturating_sub(1)
        }

        fn roll(&mut self, sides: u16) -> u16 {
            sides.saturating_sub(1)
        }
    }

    /// `r` on the session's default play area: 0.7 × (303 + 128).
    const R: f32 = 0.7 * 431.0;

    /// Asteroid type `kind`: `Strength` `strength`, `SpinRate` 100 + 10
    /// × `kind`, 36 frames of 40 x 30.
    fn kind(kind: u8, strength: i16) -> AsteroidTypeRecord {
        AsteroidTypeRecord {
            kind,
            strength,
            spin_rate: 100 + 10 * i16::from(kind),
            frames: 36,
            frame_size: Some((40, 30)),
            ..AsteroidTypeRecord::default()
        }
    }

    fn field(system: i16, count: i16, types: u16) -> AsteroidFieldRecord {
        AsteroidFieldRecord {
            system: SystemId(system),
            count,
            types,
        }
    }

    /// `base` with asteroid types 0 (of `Strength` 0) and 1 (60), system
    /// 130 holding 4 of type 0 and system 131 holding 3 of type 1.
    fn rocky(base: FakePilotCatalog) -> FakePilotCatalog {
        FakePilotCatalog {
            asteroid_types: vec![kind(0, 0), kind(1, 60)],
            asteroid_fields: vec![field(130, 4, 0x0001), field(131, 3, 0x0002)],
            ..base
        }
    }

    /// `catalog`'s session in flight at system 130's centre.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        let session = Session::start(catalog).expect("starts");
        assert_eq!(session.landed(), None);
        session
    }

    /// One advance pass: the player's tick, then the asteroids'.
    fn pass(session: &mut Session, chance: &mut (impl Chance + ?Sized)) {
        session.tick(Controls::default());
        session.tick_asteroids(chance);
    }

    /// Grows `session`'s field to at least `count` asteroids, on the
    /// frame it is on.
    fn grown(session: &mut Session, count: usize) {
        for _ in 0..count {
            session.tick_asteroids(&mut NeverFires);
        }
        assert!(session.asteroids().count() >= count);
    }

    /// A spawn of type 0 on the near side of both axes, `r` out, at 1.99
    /// a tick toward the player, frame 35, spinning 120 % forward.
    fn near() -> Scripted {
        Scripted::rolling(&[0, 1, 0, 199, 1, 0, 199, 35, 40, 1])
    }

    fn kinds(session: &Session) -> Vec<u8> {
        session.asteroids().map(|asteroid| asteroid.kind).collect()
    }

    #[test]
    fn one_asteroid_spawns_each_pass_until_the_systems_count() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        assert_eq!(session.asteroids().count(), 0, "none to start with");
        for count in [1, 2, 3, 4, 4, 4] {
            pass(&mut session, &mut NeverFires);
            assert_eq!(session.asteroids().count(), count);
        }
        assert_eq!(kinds(&session), [0; 4]);
    }

    #[test]
    fn a_system_with_no_count_or_no_types_spawns_none() {
        for spec in [
            field(130, 0, 0x0001),
            field(130, -2, 0x0001),
            field(130, 4, 0),
        ] {
            let catalog = FakePilotCatalog {
                asteroid_fields: vec![spec],
                ..rocky(catalog())
            };
            let mut session = flying(&catalog);
            pass(&mut session, &mut NeverFires);
            assert_eq!(session.asteroids().count(), 0, "{spec:?}");
        }
        let unlisted = FakePilotCatalog {
            asteroid_fields: Vec::new(),
            ..rocky(catalog())
        };
        let mut session = flying(&unlisted);
        pass(&mut session, &mut NeverFires);
        assert_eq!(session.asteroids().count(), 0, "a system not listed");
    }

    #[test]
    fn never_more_than_16_spawn() {
        let catalog = FakePilotCatalog {
            asteroid_fields: vec![field(130, 20, 0x0001)],
            ..rocky(catalog())
        };
        let mut session = flying(&catalog);
        for _ in 0..20 {
            pass(&mut session, &mut NeverFires);
        }
        assert_eq!(session.asteroids().count(), SLOTS);
    }

    #[test]
    fn an_asteroid_takes_a_set_type_its_strength_a_frame_in_range_and_a_spin_of_80_to_120_percent()
    {
        let catalog = FakePilotCatalog {
            asteroid_fields: vec![field(130, 4, 0x0003)],
            ..rocky(catalog())
        };
        // The type, each axis's side, jitter and speed, the frame, the
        // spin's size and its sign.
        for (rolls, expected) in [
            // Frame 0, spinning 80 % back: 0 − 0.8 + 36.
            ([0, 0, 0, 0, 0, 0, 0, 0, 0, 0], (0, 1, 35.2, -0.8)),
            // Frame 35, spinning 120 % forward: 35 + 1.32 − 36.
            ([1, 0, 0, 0, 0, 0, 0, 35, 40, 1], (1, 60, 0.32, 1.1 * 1.2)),
        ] {
            let mut session = flying(&catalog);
            pass(&mut session, &mut Scripted::rolling(&rolls));
            let made = *session.asteroids().next().expect("spawned");
            assert_eq!((made.kind, made.strength), (expected.0, expected.1));
            // Its frame as drawn, stepped once by its spin: wrapped once.
            assert!((made.frame - expected.2).abs() < 1e-4, "{made:?}");
            assert!((made.spin - expected.3).abs() < 1e-5, "{made:?}");
        }
    }

    #[test]
    fn an_edge_spawn_lies_at_least_r_out_on_both_axes_and_drifts_toward_the_player() {
        let catalog = rocky(catalog());
        for chance in [&mut NeverFires as &mut dyn Chance, &mut near()] {
            let mut session = flying(&catalog);
            pass(&mut session, chance);
            let made = *session.asteroids().next().expect("spawned");
            let off = made.position - made.velocity - session.player().position;
            assert!(off.x.abs() >= R && off.y.abs() >= R, "{off:?}");
            assert!(made.velocity.x * off.x <= 0.0, "{made:?}");
            assert!(made.velocity.y * off.y <= 0.0, "{made:?}");
        }
        let mut session = flying(&catalog);
        pass(&mut session, &mut near());
        let made = *session.asteroids().next().expect("spawned");
        assert_eq!(
            made.velocity,
            Vec2::new(1.99, 1.99),
            "the near side, at speed"
        );
    }

    #[test]
    fn the_asteroids_drift_by_their_velocity_and_spin_by_their_spin_each_pass() {
        let catalog = FakePilotCatalog {
            asteroid_fields: vec![field(130, 1, 0x0001)],
            ..rocky(catalog())
        };
        let mut session = flying(&catalog);
        pass(&mut session, &mut near());
        let first = *session.asteroids().next().expect("spawned");
        assert!((first.spin - 1.2).abs() < 1e-6, "{first:?}");
        // The spawn's own pass stepped it: 35 + 1.2 wrapped once.
        assert!((first.frame - 0.2).abs() < 1e-5, "{first:?}");
        pass(&mut session, &mut Last);
        let second = *session.asteroids().next().expect("still there");
        assert_eq!(second.position, first.position + first.velocity);
        assert!((second.frame - 1.4).abs() < 1e-5, "{second:?}");
    }

    #[test]
    fn an_asteroid_left_behind_is_culled_and_its_slot_refilled() {
        let catalog = FakePilotCatalog {
            asteroid_fields: vec![field(130, 1, 0x0001)],
            ..rocky(catalog())
        };
        let mut session = flying(&catalog);
        pass(&mut session, &mut NeverFires);
        let first = *session.asteroids().next().expect("spawned");
        session.ship.player.position = session.ship.player.position + Vec2::new(1000.0, 0.0);
        pass(&mut session, &mut NeverFires);
        assert_eq!(session.asteroids().count(), 0, "culled, none spawned");
        pass(&mut session, &mut NeverFires);
        let refilled = *session.asteroids().next().expect("refilled");
        let off = refilled.position - session.player().position;
        assert!(
            (off - first.position).length() < 1e-3,
            "about the player where it is now: {off:?}"
        );
    }

    #[test]
    fn nothing_spawns_or_steps_while_landed() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        land_now(&mut session).expect("lands");
        for _ in 0..3 {
            pass(&mut session, &mut NeverFires);
        }
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn double_times_second_pass_spawns_and_steps_again() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        session.set_double_time_key(true);
        pass(&mut session, &mut NeverFires);
        assert_eq!(session.asteroids().count(), 1);
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            crate::combat::Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.asteroids().count(), 2, "the second pass's spawn");
    }

    #[test]
    fn a_jumps_arrival_empties_the_field_and_the_next_pass_spawns_again() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        grown(&mut session, 3);
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.asteroids().count(), 0);
        pass(&mut session, &mut NeverFires);
        assert_eq!(kinds(&session), [1], "131's type, one a pass");
    }

    #[test]
    fn a_hypergate_empties_the_field() {
        let catalog = rocky(gated());
        let mut session = at_gate(&catalog);
        grown(&mut session, 2);
        session.open_hypergate(&catalog).expect("opens");
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn a_wormhole_empties_the_field() {
        let catalog = rocky(wormholes());
        let mut session = at_wormhole(&catalog);
        grown(&mut session, 2);
        session
            .enter_wormhole(&catalog, &mut Scripted::rolling(&[0]))
            .expect("enters");
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn a_landing_empties_the_field() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        grown(&mut session, 3);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn an_m_or_n_in_flight_empties_the_field_for_the_rest_of_the_frame() {
        for expr in ["M131", "N131"] {
            let catalog = rocky(catalog());
            let mut session = flying(&catalog);
            pass(&mut session, &mut NeverFires);
            grown(&mut session, 3);
            session.run_set(&SetExpr::parse(expr).expect("parses"), &mut NeverFires);
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.system(), SystemId(131), "{expr}");
            assert_eq!(session.asteroids().count(), 0, "{expr}");
            session.tick_asteroids(&mut NeverFires);
            assert_eq!(session.asteroids().count(), 0, "{expr}: the same frame");
            pass(&mut session, &mut NeverFires);
            assert_eq!(kinds(&session), [1], "{expr}: the next frame");
        }
    }

    #[test]
    fn a_miner_in_the_traffic_mines_while_slot_0_holds_an_asteroid_and_stops_without() {
        use crate::ai::miner::MINES;
        use crate::ai::{Goal, NovaAi};
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        // The first pass populates the system.
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        let mut zephyr = crate::traffic::fixture::npc(
            1,
            crate::stats::ShipStats::new(nova_sim_base::fixture::FAST, &[]),
        );
        zephyr.miner = MINES;
        zephyr.state.position = Vec2::new(600.0, 600.0);
        let id = session.space.traffic.add_npc(zephyr);
        let think = |session: &mut Session| {
            for _ in 0..4 {
                session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
            }
            session.space.npc(id).expect("here").goal
        };
        assert_eq!(think(&mut session), Goal::Wait, "state 6");
        grown(&mut session, 1);
        assert!(session.space.asteroids.get(0).is_some(), "slot 0");
        assert_eq!(think(&mut session), Goal::Mine, "state 16");
    }

    #[test]
    fn the_escape_pods_rescue_keeps_the_field() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        grown(&mut session, 3);
        let before: Vec<Asteroid> = session.asteroids().copied().collect();
        session.pilot.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        session.pod = Some(0);
        session.ship.fate.rescue_due = true;
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        let after: Vec<Asteroid> = session.asteroids().copied().collect();
        assert_eq!(after, before);
    }

    /// The rules with `key` read as the other reading.
    fn other(key: RuleKey) -> Rulebook {
        Rulebook::default().with_override(key, RuleSource::Bible)
    }

    /// `catalog`'s session in flight, jumped to 131 with a field of 4 of
    /// type 0 grown in 130, under `rules`.
    fn arrived(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = flying(catalog).with_rules(rules);
        session.settle_asteroid_field(&mut NeverFires);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.space.asteroids.clear();
        grown(&mut session, 4);
        assert_eq!(kinds(&session), [0; 4]);
        session.arrive(catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        session
    }

    #[test]
    fn by_the_engine_a_jumps_arrival_leaves_the_field_empty_to_fill_from_the_edges() {
        let catalog = rocky(catalog());
        let mut session = arrived(&catalog, Rulebook::default());
        assert_eq!(session.asteroids().count(), 0);
        for count in 1..=3 {
            pass(&mut session, &mut NeverFires);
            assert_eq!(session.asteroids().count(), count);
        }
    }

    #[test]
    fn placed_a_jumps_arrival_keeps_its_systems_field_and_none_of_the_last() {
        let catalog = rocky(catalog());
        let session = arrived(&catalog, other(RuleKey::AsteroidEntryField));
        // Before any pass: the frame drawn after the arrival shows it.
        assert_eq!(kinds(&session), [1; 3], "exactly 131's 3, all of its type");
        let player = session.player().position;
        for made in session.asteroids() {
            let off = made.position - player;
            // W = 431, H = 428 about the ship.
            assert!(off.x.abs() <= 215.0 && off.y.abs() <= 214.0, "{off:?}");
            assert!(made.velocity.x.abs() <= 2.0 && made.velocity.y.abs() <= 2.0);
        }
    }

    #[test]
    fn placed_a_jumps_arrival_into_a_system_with_none_empties_the_field() {
        let catalog = FakePilotCatalog {
            asteroid_fields: vec![field(130, 4, 0x0001)],
            ..rocky(catalog())
        };
        let session = arrived(&catalog, other(RuleKey::AsteroidEntryField));
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn placed_a_gate_or_a_move_still_empties_the_field() {
        let placed = other(RuleKey::AsteroidEntryField);
        let gates = rocky(gated());
        let mut session = at_gate(&gates).with_rules(placed);
        grown(&mut session, 2);
        session.open_hypergate(&gates).expect("opens");
        session
            .enter_hypergate(Some(SystemId(131)), &gates, &mut NeverFires)
            .expect("enters");
        assert_eq!(session.asteroids().count(), 0, "a hypergate");
        let catalog = rocky(catalog());
        let mut session = flying(&catalog).with_rules(placed);
        grown(&mut session, 2);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.asteroids().count(), 0, "a move");
    }

    #[test]
    fn a_session_starting_in_flight_places_its_field_only_when_placed() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        session.settle_asteroid_field(&mut NeverFires);
        assert_eq!(session.asteroids().count(), 0, "by the engine");
        let mut session = flying(&catalog).with_rules(other(RuleKey::AsteroidEntryField));
        session.settle_asteroid_field(&mut NeverFires);
        assert_eq!(kinds(&session), [0; 4], "placed");
        session.space.asteroids.clear();
        session.settle_asteroid_field(&mut NeverFires);
        assert_eq!(session.asteroids().count(), 0, "once");
    }

    #[test]
    fn a_session_starting_in_flight_places_its_field_by_its_first_pass_too() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog).with_rules(other(RuleKey::AsteroidEntryField));
        pass(&mut session, &mut NeverFires);
        assert_eq!(session.asteroids().count(), 4);
    }

    #[test]
    fn a_pilot_saved_at_a_stellar_resumes_in_flight_and_places_its_field() {
        // `_LoadPilotData` resumes over the stellar, in flight, and the
        // main screen's entry places the field (@0x1a8cf).
        let catalog = rocky(catalog());
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(other(RuleKey::AsteroidEntryField));
        assert!(session.landed().is_none());
        session.settle_asteroid_field(&mut NeverFires);
        assert_eq!(kinds(&session), [0; 4]);
    }

    #[test]
    fn the_respawn_ring_reads_its_rule() {
        let catalog = rocky(catalog());
        let mut engine = flying(&catalog);
        pass(&mut engine, &mut NeverFires);
        let off = engine.asteroids().next().expect("spawned").position;
        assert!((off.x - R).abs() < 1e-3, "{off:?}: r out");
        let mut outside = flying(&catalog).with_rules(other(RuleKey::AsteroidRespawnRing));
        pass(&mut outside, &mut NeverFires);
        let off = outside.asteroids().next().expect("spawned").position;
        // ceil(303 + 20) and ceil(300 + 15): just off screen.
        assert_eq!(off, Vec2::new(323.0, 315.0));
    }

    // Shots at asteroids.

    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::{CombatEvent, ParticleBurst, Rules};
    use crate::reserves::Reserves;
    use crate::session::tests::{Attacking, arming, blaster, homing_missile, with_an_npc_at};
    use crate::targeting::TargetPick;

    /// Asteroid type `kind` of `Strength` `strength` and `Mass` 20, its
    /// frames `size` across: a hit radius of half of it.
    fn rock_type(kind: u8, strength: i16, size: u32) -> AsteroidTypeRecord {
        AsteroidTypeRecord {
            mass: 20,
            frame_size: Some((size, size)),
            ..self::kind(kind, strength)
        }
    }

    /// The NPC's gun: a blaster of `wëap` 140.
    fn npc_gun() -> WeaponRecord {
        WeaponRecord {
            id: WeaponId(140),
            ..blaster()
        }
    }

    /// `arming`'s catalog, the player's gun `gun` and its NPC's `npc`
    /// (of another `wëap`), with asteroid `types` and no field.
    fn rocky_arming(
        gun: WeaponRecord,
        npc: WeaponRecord,
        types: Vec<AsteroidTypeRecord>,
    ) -> FakePilotCatalog {
        FakePilotCatalog {
            asteroid_types: types,
            asteroid_fields: Vec::new(),
            ..arming(gun, npc, 0)
        }
    }

    /// Puts an asteroid of `kind`, as strong as `strength`, at (`x`, `y`)
    /// at rest in `session`'s field; its slot.
    fn put(session: &mut Session, kind: u8, strength: i16, x: f32, y: f32) -> usize {
        session
            .space
            .asteroids
            .place(Asteroid {
                position: Vec2::new(x, y),
                velocity: Vec2::ZERO,
                frame: 0.0,
                spin: 0.0,
                strength,
                kind,
            })
            .expect("a free slot")
    }

    /// The asteroid in `slot`.
    fn slot(session: &Session, slot: usize) -> Asteroid {
        *session
            .space
            .asteroids
            .indexed()
            .find(|&(at, _)| at == slot)
            .expect("there")
            .1
    }

    /// The player, at the centre facing up, fires once; the fight goes on
    /// `ticks` ticks in all. The events, in order.
    fn shoot_once(session: &mut Session, ticks: u32) -> Vec<CombatEvent> {
        let mut events = Vec::new();
        session.hold_fire(true, false);
        for tick in 0..ticks {
            session.tick_combat(Rules::default(), &mut NeverFires);
            if tick == 0 {
                session.hold_fire(false, false);
            }
            events.extend(session.take_combat_events());
        }
        events
    }

    /// A gun 20 pixels a tick of `MassDmg` 5, `EnergyDmg` `energy`,
    /// `Impact` `impact` and these `Flags2`.
    fn rock_gun(energy: i16, impact: i16, flags2: u16) -> WeaponRecord {
        WeaponRecord {
            impact,
            energy_dmg: energy,
            flags2,
            ..blaster()
        }
    }

    /// The session of the player armed with `gun` and an asteroid of type
    /// 0 (`Strength` 100, 40 across) 100 above it, after one shot of 6
    /// ticks: the asteroid, and the events.
    fn struck(gun: WeaponRecord) -> (Session, Asteroid, Vec<CombatEvent>) {
        let catalog = rocky_arming(gun, npc_gun(), vec![rock_type(0, 100, 40)]);
        let mut session = flying(&catalog);
        let at = put(&mut session, 0, 100, 0.0, -100.0);
        let events = shoot_once(&mut session, 6);
        let rock = slot(&session, at);
        (session, rock, events)
    }

    #[test]
    fn a_players_shot_takes_its_mass_damage_off_an_asteroid_and_is_used_up() {
        for energy in [10, 0] {
            let (session, rock, _) = struck(rock_gun(energy, 0, 0));
            assert_eq!(rock.strength, 95, "{energy}: MassDmg alone");
            assert_eq!(session.shots(), [], "{energy}: used up");
            assert_eq!(rock.velocity, Vec2::ZERO, "no Impact, no push");
        }
        let (_, rock, _) = struck(rock_gun(10, 0, 0x8000));
        assert_eq!(rock.strength, 50, "ten times under Flags2 0x8000");
    }

    #[test]
    fn a_hit_pushes_an_asteroid_by_impact_over_mass_along_the_shot_up_to_2() {
        let (_, rock, _) = struck(rock_gun(10, 10, 0));
        assert!(
            (rock.velocity - Vec2::new(0.0, -0.5)).length() < 1e-5,
            "{rock:?}"
        );
        let (_, rock, _) = struck(rock_gun(10, 100, 0));
        assert!(
            (rock.velocity - Vec2::new(0.0, -2.0)).length() < 1e-5,
            "held to 2: {rock:?}"
        );
    }

    #[test]
    fn a_hit_raises_the_weapons_hit_particles() {
        let gun = WeaponRecord {
            hit_particles: 6,
            hit_part_vel: 250,
            hit_part_life: 40,
            hit_part_color: 0x00FF_8000,
            ..rock_gun(10, 0, 0)
        };
        let (_, _, events) = struck(gun);
        let bursts: Vec<ParticleBurst> = events
            .iter()
            .filter_map(|event| match event {
                CombatEvent::Particles(burst) => Some(*burst),
                _ => None,
            })
            .collect();
        assert_eq!(bursts.len(), 1, "{events:?}");
        assert!(
            (bursts[0].at - Vec2::new(0.0, -80.0)).length() < 1e-3,
            "{bursts:?}"
        );
        assert_eq!(
            bursts[0],
            ParticleBurst {
                at: bursts[0].at,
                speed: (250.0_f64 * 0.01) as f32,
                variation: 20,
                life: (40, 50),
                color: 0x00FF_8000,
                count: 6,
                spread: 0.0,
            }
        );
    }

    #[test]
    fn a_seeker_0x0001_gun_passes_through_an_asteroid() {
        let gun = WeaponRecord {
            seeker: 0x0001,
            ..rock_gun(10, 0, 0)
        };
        let (session, rock, _) = struck(gun);
        assert_eq!(rock.strength, 100);
        assert_eq!(session.shots().len(), 1, "flying on");
    }

    /// The player's homing missile, `Seeker` `seeker`, fired at its NPC
    /// 100 above with an asteroid between: the asteroid's strength, and
    /// the NPC's shield.
    fn missile_past_a_rock(seeker: u16) -> (i16, f32) {
        let missile = WeaponRecord {
            seeker,
            ..homing_missile(134, 1000)
        };
        let catalog = rocky_arming(missile, npc_gun(), vec![rock_type(0, 100, 20)]);
        let mut session = with_an_npc_at(&catalog, 750, 650, 180);
        let at = put(&mut session, 0, 100, 0.0, -50.0);
        session.select_target(TargetPick::Nearest);
        shoot_once(&mut session, 30);
        (
            slot(&session, at).strength,
            session.npcs()[0].reserves.shield.now,
        )
    }

    #[test]
    fn a_seeker_0x0001_missile_passes_through_an_asteroid_to_its_target() {
        let (rock, shield) = missile_past_a_rock(0x0001);
        assert_eq!(rock, 100, "passed through");
        assert!(shield < 30.0, "its target hit: {shield}");
        let (rock, shield) = missile_past_a_rock(0);
        assert!(rock < 100, "without it, the asteroid takes it");
        assert_eq!(shield, 30.0);
    }

    /// A gun of `ProxRadius` 40 armed at once.
    fn fused_gun() -> WeaponRecord {
        WeaponRecord {
            prox_radius: 40,
            ..rock_gun(10, 0, 0)
        }
    }

    #[test]
    fn an_armed_fuse_with_no_ship_in_reach_takes_the_first_asteroid_in_slot_order() {
        let catalog = rocky_arming(fused_gun(), npc_gun(), vec![rock_type(0, 100, 10)]);
        let mut session = flying(&catalog);
        // Both come within 40 as the shot reaches (0, -80); the second is
        // the nearer.
        let first = put(&mut session, 0, 100, 35.0, -80.0);
        let second = put(&mut session, 0, 100, 5.0, -100.0);
        shoot_once(&mut session, 6);
        assert_eq!(slot(&session, first).strength, 95);
        assert_eq!(slot(&session, second).strength, 100);
        assert_eq!(session.shots(), []);
    }

    #[test]
    fn a_flags2_0x0004_fuse_takes_an_asteroid_by_the_engine_and_passes_it_by_the_bible() {
        for (rules, hit) in [
            (Rulebook::default(), true),
            (other(RuleKey::FuseIgnoresAsteroids), false),
        ] {
            let gun = WeaponRecord {
                flags2: 0x0004,
                ..fused_gun()
            };
            let catalog = rocky_arming(gun, npc_gun(), vec![rock_type(0, 100, 10)]);
            let mut session = flying(&catalog).with_rules(rules);
            let at = put(&mut session, 0, 100, 35.0, -80.0);
            shoot_once(&mut session, 6);
            assert_eq!(slot(&session, at).strength < 100, hit, "{rules:?}");
            assert_eq!(session.shots().is_empty(), hit, "{rules:?}");
        }
    }

    #[test]
    fn a_ship_that_sets_the_fuse_off_takes_the_shot_and_leaves_the_asteroids_whole() {
        let catalog = rocky_arming(fused_gun(), npc_gun(), vec![rock_type(0, 100, 10)]);
        // The NPC at (45, -90), 30 wide: within trunc(40 + 30 x 0.333) =
        // 49 of where the shot's fourth move ends, (0, -80), not of where
        // its third does, (0, -60).
        let mut session = with_an_npc_at(&catalog, 795, 660, 180);
        let first = put(&mut session, 0, 100, 35.0, -80.0);
        let second = put(&mut session, 0, 100, 5.0, -100.0);
        shoot_once(&mut session, 6);
        assert_eq!(slot(&session, first).strength, 100);
        assert_eq!(slot(&session, second).strength, 100);
        assert!(session.npcs()[0].reserves.shield.now < 30.0);
    }

    #[test]
    fn a_ship_beside_a_fused_shots_path_sets_it_off_by_the_bible_only() {
        let catalog = arming(fused_gun(), npc_gun(), 0);
        // The NPC at (49, -90), 30 wide: within its hit radius and the
        // `ProxRadius`, 9.9 + 40, of the shot's path up the y axis, but
        // never within trunc(40 + 30 x 0.333) = 49 of where a move ends,
        // (0, -80) and (0, -100) being 10 short of and past it.
        for (source, struck) in [(RuleSource::Engine, false), (RuleSource::Bible, true)] {
            let rules = Rulebook::default().with_override(RuleKey::ProximityFuse, source);
            let mut session = with_an_npc_at(&catalog, 799, 660, 180).with_rules(rules);
            shoot_once(&mut session, 12);
            let shield = session.npcs()[0].reserves.shield.now;
            assert_eq!(shield < 30.0, struck, "{source:?}: {shield}");
        }
    }

    #[test]
    fn a_point_defence_shot_meets_an_asteroid_on_its_way_to_a_missile() {
        let quad = WeaponRecord {
            id: WeaponId(133),
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..blaster()
        };
        let slow = WeaponRecord {
            speed: 500,
            count: 200,
            ..homing_missile(134, 1000)
        };
        let catalog = rocky_arming(quad, slow, vec![rock_type(0, 100, 20)]);
        let mut session = with_an_npc_at(&catalog, 750, 550, 180);
        let at = put(&mut session, 0, 100, 0.0, -50.0);
        let mut events = Vec::new();
        for _ in 0..15 {
            session.tick_traffic(&catalog, &Attacking, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert!(
            events.iter().any(|event| matches!(
                event,
                CombatEvent::Fired { weapon, .. } if *weapon == WeaponId(133)
            )),
            "point defence fired"
        );
        assert!(
            slot(&session, at).strength < 100,
            "its shots met the asteroid"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. })),
            "none reached the missile"
        );
    }

    /// A blaster of `BlastRadius` `radius`.
    fn blasting(radius: i16) -> WeaponRecord {
        WeaponRecord {
            blast_radius: radius,
            ..blaster()
        }
    }

    #[test]
    fn a_players_shot_at_an_asteroid_blasts_the_npcs_in_its_square() {
        let catalog = rocky_arming(blasting(30), npc_gun(), vec![rock_type(0, 100, 40)]);
        // The NPC at (20, -70): off the shot's way, in the square about
        // (0, -80) where it meets the asteroid.
        let mut session = with_an_npc_at(&catalog, 770, 680, 180);
        put(&mut session, 0, 100, 0.0, -100.0);
        shoot_once(&mut session, 6);
        assert!(session.npcs()[0].reserves.shield.now < 30.0, "blasted");
    }

    #[test]
    fn an_npcs_shot_at_an_asteroid_blasts_nobody() {
        let gun = WeaponRecord {
            id: WeaponId(140),
            ..blasting(60)
        };
        let catalog = rocky_arming(blaster(), gun, vec![rock_type(0, 100, 40)]);
        // The NPC 200 above fires down at the player; its shot meets the
        // asteroid at (0, -45), the player within 60 of it.
        let mut session = with_an_npc_at(&catalog, 750, 550, 180);
        let at = put(&mut session, 0, 100, 0.0, -25.0);
        for _ in 0..12 {
            session.tick_traffic(&catalog, &Attacking, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert!(slot(&session, at).strength < 100, "the NPC's shots met it");
        assert_eq!(session.reserves(), Reserves::full(30.0, 45.0, 300.0));
    }

    // Destruction.

    /// Asteroid type 4 (`Strength` 100, 40 across): `YieldQty` 4 of
    /// commodity 2, 12 orange particles, 4 fragments of types 1 and 2,
    /// `bööm` 131; and types 1 and 2 themselves.
    fn breaking() -> Vec<AsteroidTypeRecord> {
        vec![
            AsteroidTypeRecord {
                yield_type: 2,
                yield_qty: 4,
                part_count: 12,
                part_color: 0x00FF_8000,
                frag_types: [129, 130],
                frag_count: 4,
                explode_type: 3,
                ..rock_type(4, 100, 40)
            },
            rock_type(1, 30, 20),
            rock_type(2, 40, 20),
        ]
    }

    /// The player shoots an asteroid of type 4 left at `strength` 100 above
    /// it, moving (0.5, 0), with `others` asteroids of type 0 placed far
    /// off first: the session, the asteroid's slot and the events.
    fn breaking_one(strength: i16, others: usize) -> (Session, usize, Vec<CombatEvent>) {
        let mut types = breaking();
        types.push(rock_type(0, 100, 20));
        let catalog = rocky_arming(rock_gun(10, 0, 0), npc_gun(), types);
        let mut session = flying(&catalog);
        for n in 0..others {
            put(&mut session, 0, 100, 200.0, 20.0 * n as f32 - 150.0);
        }
        let at = put(&mut session, 4, strength, 0.0, -100.0);
        session.space.asteroids.get_mut(at).expect("there").velocity = Vec2::new(0.5, 0.0);
        let events = shoot_once(&mut session, 6);
        (session, at, events)
    }

    #[test]
    fn an_asteroid_left_at_0_is_still_there() {
        let (session, at, _) = breaking_one(5, 0);
        assert_eq!(slot(&session, at).strength, 0);
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn an_asteroid_below_0_explodes_and_throws_off_its_particles() {
        let (_, _, events) = breaking_one(4, 0);
        let seen: Vec<CombatEvent> = events
            .iter()
            .filter(|event| !matches!(event, CombatEvent::Fired { .. }))
            .copied()
            .collect();
        let at = seen
            .iter()
            .find_map(|event| match event {
                CombatEvent::Exploded { at, .. } => Some(*at),
                _ => None,
            })
            .expect("its explosion");
        // Where the asteroid was, drifting right.
        assert!(
            (at.y + 100.0).abs() < 1e-3 && (0.0..=3.0).contains(&at.x),
            "{at:?}"
        );
        assert_eq!(
            seen,
            [
                CombatEvent::Particles(ParticleBurst {
                    at,
                    speed: 0.2,
                    variation: 40,
                    life: (240, 480),
                    color: 0x00FF_8000,
                    count: 12,
                    spread: 13.0,
                }),
                CombatEvent::Exploded {
                    at,
                    explosion: crate::combat::weapon::Explosion::decode(3).expect("bööm 131"),
                    // `_CreateExplosion(.., 0, 1)` @0xaf7f.
                    size: 0.0,
                },
            ]
        );
    }

    #[test]
    fn an_asteroid_below_0_breaks_into_fragments_of_its_frag_types_each_with_its_own_velocity() {
        let (session, at, _) = breaking_one(4, 0);
        let made: Vec<Asteroid> = session.asteroids().copied().collect();
        // FragCount / 2 + Rand(FragCount): 2, each of type 1 by Rand(2) 0.
        assert_eq!(made.len(), 2, "{made:?}");
        for fragment in &made {
            assert_eq!(fragment.kind, 1);
            assert_eq!(fragment.strength, 30);
            assert_eq!(fragment.velocity, Vec2::new(-1.0, -1.0), "not its parent's");
        }
        assert!(
            session
                .space
                .asteroids
                .indexed()
                .all(|(slot, _)| slot != at),
            "the parent gone"
        );
    }

    #[test]
    fn a_full_field_takes_no_fragments_of_an_asteroid_destroyed() {
        let (session, _, _) = breaking_one(4, crate::asteroid::SLOTS - 1);
        assert_eq!(session.asteroids().count(), crate::asteroid::SLOTS - 1);
        assert!(session.asteroids().all(|made| made.kind == 0));
    }

    #[test]
    fn an_asteroid_below_0_drops_scoopable_boxes_of_its_yield() {
        let (session, _, _) = breaking_one(4, 0);
        let boxes = session.cargo_boxes();
        // 4 × (Rand(101) + 50) %: 2.
        assert_eq!(boxes.len(), 2, "{boxes:?}");
        for made in boxes {
            assert!(made.scoopable);
            assert_eq!(made.contents, Some(2));
            assert_eq!(made.graphic, 2, "type 4 / 4 + 1");
            assert_eq!(made.life, 300.0);
            assert_eq!(made.spin, -1);
            assert!(
                (made.velocity - Vec2::new(0.0, -0.12)).length() < 1e-5,
                "{made:?}"
            );
        }
    }

    // The pass after the fight.

    /// A gun whose shots stand still where they were fired, for 100
    /// ticks.
    fn still_gun() -> WeaponRecord {
        WeaponRecord {
            speed: 0,
            count: 100,
            ..blaster()
        }
    }

    /// An asteroid of type 0 (20 across) 30 left of the player, drifting
    /// 25 a tick toward it: 5 off it after a step.
    fn drifting_in(session: &mut Session) -> usize {
        let at = put(session, 0, 100, -30.0, 0.0);
        session.space.asteroids.get_mut(at).expect("there").velocity = Vec2::new(25.0, 0.0);
        at
    }

    #[test]
    fn a_shot_meets_an_asteroid_where_the_last_pass_left_it() {
        let catalog = rocky_arming(still_gun(), npc_gun(), vec![rock_type(0, 100, 20)]);
        let mut session = flying(&catalog);
        let at = drifting_in(&mut session);
        session.hold_fire(true, false);
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.hold_fire(false, false);
        let rock = slot(&session, at);
        assert_eq!(
            rock.position,
            Vec2::new(-5.0, 0.0),
            "stepped after the shots"
        );
        assert_eq!(rock.strength, 100, "the shot's move not yet tested");
        session.tick_combat(Rules::default(), &mut NeverFires);
        let rock = slot(&session, at);
        assert_eq!(rock.strength, 95, "met where the last pass left it");
        assert_eq!(rock.position, Vec2::new(20.0, 0.0), "then stepped on");
    }

    #[test]
    fn the_second_pass_steps_the_field_after_its_fight_too() {
        let catalog = rocky_arming(still_gun(), npc_gun(), vec![rock_type(0, 100, 20)]);
        let mut session = flying(&catalog);
        session.hold_fire(true, false);
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.hold_fire(false, false);
        assert_eq!(session.shots().len(), 1, "one still shot");
        let at = drifting_in(&mut session);
        session.set_double_time_key(true);
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        let rock = slot(&session, at);
        assert_eq!(rock.strength, 100, "30 off as the pass's shots met it");
        assert_eq!(rock.position, Vec2::new(-5.0, 0.0), "then stepped");
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            slot(&session, at).strength,
            95,
            "met where the step left it"
        );
    }

    #[test]
    fn the_fights_pass_spawns_nothing_while_landed() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        land_now(&mut session).expect("lands");
        for _ in 0..3 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn the_fights_pass_spawns_and_steps_the_field_through_a_jumps_lead_in() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        assert!(session.jumping().is_some());
        for count in 1..=3 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(session.asteroids().count(), count);
        }
    }
}
