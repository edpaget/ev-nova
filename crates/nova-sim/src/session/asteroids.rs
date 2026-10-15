//! The session's side of [`asteroid`](crate::asteroid): the field about
//! the player, its system's `Asteroids` and `AstTypes` spawning one
//! asteroid at the edge and stepping the field on each game-work pass,
//! and the kills and placements as the player enters a system. The field
//! is never saved.
//!
//! **The pass.** [`Session::tick_asteroids`] is one pass's asteroid work:
//! `_DoPlayGameWork`'s edge spawn (@0x44243) and `_HandleAsteroids`
//! (@0x444f5). The view runs it at the end of the advance pass, after
//! the fight and the quotes, and [`Session::tick_again`] at the end of
//! the second. The original spawns before the AI, the ships and the
//! shots, and steps after `_HandleBoxes`; here both come after the
//! fight. Nothing reads the asteroids yet, so the order changes nothing
//! the player sees; once shots hit them
//! (rdm:phase/asteroids/phase-2-shoot-and-break-asteroids), the pass
//! must move ahead of the fight.
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
//! streak, and the next frame's pass spawns as the original's arrival
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

use super::Session;
use crate::asteroid::{Asteroid, FieldSpec, Origin, Ring};
use crate::catalog::{AsteroidFieldRecord, SystemId};
use crate::chance::Chance;
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
        self.asteroids.iter()
    }

    /// Runs one game-work pass's asteroid work, drawing on `chance`: an
    /// asteroid spawned at the field's edge, on the ring
    /// [`RuleKey::AsteroidRespawnRing`] says, and every asteroid stepped
    /// and culled in the play area (see the module docs). Nothing while
    /// landed; on a frame a kill holds through, the field is emptied.
    pub fn tick_asteroids(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if self.landed.is_some() {
            // The landing's kill is spent: the field stays empty until the
            // take-off, and a later frame's counter can't match it.
            self.asteroid_kill = None;
            return;
        }
        self.settle_asteroid_field(chance);
        if self.asteroid_kill == Some(self.pass_frame) {
            self.asteroids.clear();
            return;
        }
        self.asteroid_kill = None;
        let ring = match self.rules.source_for(RuleKey::AsteroidRespawnRing) {
            RuleSource::Engine => Ring::Original,
            RuleSource::Bible => Ring::Outside,
        };
        let player = self.player.position;
        self.asteroids.spawn(
            Origin::Edge(ring),
            self.field_spec(),
            &self.asteroid_types,
            player,
            self.play_area,
            chance,
        );
        self.asteroids
            .step(&self.asteroid_types, player, self.play_area);
    }

    /// Places the field of a session started or loaded in flight, once,
    /// drawing on `chance`, by the other reading of
    /// [`RuleKey::AsteroidEntryField`]; by the engine, and once docked,
    /// it places none (see the module docs). The view runs it before it
    /// draws a frame; [`Session::tick_asteroids`] runs it too.
    pub fn settle_asteroid_field(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if !std::mem::take(&mut self.asteroid_field_due) || self.landed.is_some() {
            return;
        }
        if self.places_asteroids() {
            self.place_asteroids(chance);
        }
    }

    /// Kills the field, through the rest of the pass frame, as
    /// `_killAsteroids` kills it (see the module docs).
    pub(super) fn kill_asteroids(&mut self) {
        self.asteroids.clear();
        self.asteroid_kill = Some(self.pass_frame);
    }

    /// Places the field of the player's system about the ship afresh,
    /// the field held emptied first, drawing on `chance`: a jump's
    /// arrival by the other reading of [`RuleKey::AsteroidEntryField`]
    /// (see the module docs).
    pub(super) fn place_asteroids(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.asteroids.clear();
        self.asteroid_kill = None;
        let player = self.player.position;
        self.asteroids.place_field(
            self.field_spec(),
            &self.asteroid_types,
            player,
            self.play_area,
            chance,
        );
    }

    /// Whether an arrival keeps the field `_CreateAsteroids` places, as
    /// [`RuleKey::AsteroidEntryField`] reads.
    pub(super) fn places_asteroids(&self) -> bool {
        self.rules.source_for(RuleKey::AsteroidEntryField) == RuleSource::Bible
    }

    /// The player's system's field: none for one the catalog doesn't
    /// list.
    fn field_spec(&self) -> FieldSpec {
        self.asteroid_fields
            .get(&self.pilot.system)
            .copied()
            .unwrap_or(FieldSpec::NONE)
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
        session.player.position = session.player.position + Vec2::new(1000.0, 0.0);
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
    fn the_escape_pods_rescue_keeps_the_field() {
        let catalog = rocky(catalog());
        let mut session = flying(&catalog);
        grown(&mut session, 3);
        let before: Vec<Asteroid> = session.asteroids().copied().collect();
        session.pilot.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        session.pod = Some(0);
        session.rescue_due = true;
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
        session.asteroids.clear();
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
        session.asteroids.clear();
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
    fn a_session_starting_docked_places_no_field() {
        let catalog = rocky(catalog());
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(other(RuleKey::AsteroidEntryField));
        assert!(session.landed().is_some());
        session.settle_asteroid_field(&mut NeverFires);
        assert_eq!(session.asteroids().count(), 0);
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
}
