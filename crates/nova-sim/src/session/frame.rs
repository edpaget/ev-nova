//! [`Session::fill_frame`]: what the flight view draws, filled into a
//! [`FlightFrame`] for the view's `FlightSim` port
//! (see [`flight_frame`](crate::flight_frame)): the nav and course here,
//! the world's ships, shots and stellars in the `world` module.

use super::Session;
use crate::flight_frame::{FlightFrame, MapKey};

mod hud;
mod world;

impl Session {
    /// Fills `into` with what the flight view draws now. Every field is
    /// written, so a frame kept from an earlier fill, or from another
    /// session, shows nothing stale: the destructuring names every field,
    /// so a field added to [`FlightFrame`] doesn't compile until it is
    /// filled here.
    pub fn fill_frame(&self, into: &mut FlightFrame) {
        let FlightFrame {
            nav_mode,
            system,
            course_shown,
            course_plotted,
            docked,
            jump_under_way,
            jump_readiness,
            texts_queued,
            map_key,
            ships,
            drawn,
            shots,
            beams,
            cargo_boxes,
            asteroids,
            stellars,
            effective_murk,
            system_murk,
            system_background,
            double_time,
            hyper_gamma,
            traffic_ships,
            hud,
        } = into;
        *nav_mode = self.nav_mode();
        *system = self.system();
        *course_shown = self.course_shown();
        *course_plotted = !self.course().is_empty();
        *docked = self.landed().is_some();
        *jump_under_way = self.jumping().is_some() || self.preparing_jump().is_some();
        *jump_readiness = self.jump_readiness();
        *texts_queued = self.texts_queued();
        *map_key = MapKey {
            system: self.system(),
            explored: self.pilot.explored().collect(),
            charted: self.pilot.charted().collect(),
            known: self.known().clone(),
            course: self.course().to_vec(),
            nav: self.nav_mode(),
        };
        self.fill_ships(ships);
        self.fill_drawn(drawn);
        self.space.fill_shots(shots);
        self.space.fill_beams(beams);
        self.space.fill_boxes(cargo_boxes);
        self.space.fill_asteroids(asteroids);
        self.fill_stellars(stellars);
        *effective_murk = self.effective_murk();
        *system_murk = self.system_murk();
        *system_background = self.system_background();
        *double_time = self.double_time();
        *hyper_gamma = self.hyper_gamma();
        *traffic_ships = self.traffic_ships();
        self.fill_hud(hud);
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{ShipId, StellarId, SystemId, WeaponId};
    use crate::cloak::CloakShown;
    use crate::combat::ShipRef;
    use crate::combat::projectile::ShotId;
    use crate::combat::ship_frames::FrameSpec;
    use crate::combat::shot_layer::ShotLayer;
    use crate::flight_frame::{
        FrameAsteroid, FrameBeam, FrameBox, FrameShip, FrameShot, FrameStellar, GateFade,
    };
    use crate::geometry::Vec2;
    use crate::glow::drawn_base;
    use crate::hyperspace::JumpReadiness;
    use crate::iff::ShipIff;
    use crate::navigation::NavMode;
    use crate::player_fate::Drawn as ShipDrawn;
    use crate::session::HoverPlace;
    use crate::session::StellarPick;
    use crate::session::tests::three_stellars;
    use crate::ship_layer::ShipLayer;
    use crate::story::DrawnSheet;
    use crate::testkit::{begin_jump_now, catalog, fly_out, land_now};
    use crate::traffic::npc::Npc;

    /// The player's record, from the session's own getters.
    pub(in crate::session) fn player_record(session: &Session) -> FrameShip {
        let hull = session.hull();
        FrameShip {
            ship: ShipRef::Player,
            class: session.ship(),
            position: session.player().position,
            velocity: session.player().velocity,
            heading: session.player().heading,
            condition: session.player_condition(),
            breakup: hull.breakup,
            shield: session.reserves().shield,
            class_shield: session.class_shield(session.ship()),
            frames: hull.frames,
            drawn: session.player_drawn(),
            shown: session.player_fate().row().drawn,
            glow: session.engine_glow(),
            cloak: session.player_cloak_shown(),
            ion_tint: session.player_ion_tint(),
            gate: GateFade::None,
            hover: session.hover_place(ShipRef::Player),
            framed: session.framed_by(ShipRef::Player),
            govt: None,
            on_radar: false,
            iff: ShipIff::Other,
            mass: hull.mass,
        }
    }

    /// `npc`'s record, from the session's own getters and the NPC's.
    pub(in crate::session) fn npc_record(session: &Session, npc: &Npc) -> FrameShip {
        let ship = ShipRef::Npc(npc.id);
        FrameShip {
            ship,
            class: npc.ship,
            position: npc.state.position,
            velocity: npc.state.velocity,
            heading: npc.state.heading,
            condition: npc.condition,
            breakup: npc.hull.breakup,
            shield: npc.reserves.shield,
            class_shield: session.class_shield(npc.ship),
            frames: npc.hull.frames,
            drawn: npc.display.drawn,
            shown: ShipDrawn::Full,
            glow: drawn_base(npc.display.glow),
            cloak: session.cloak_shown(npc),
            ion_tint: npc.display.ion_tint,
            gate: GateFade::of(&npc.mode),
            hover: session.hover_place(ship),
            framed: session.framed_by(ship),
            govt: npc.govt,
            on_radar: session.on_radar(npc),
            iff: session.ship_iff(npc),
            mass: npc.hull.mass,
        }
    }

    /// A ship record unlike any a test session fills.
    fn stale_ship() -> FrameShip {
        FrameShip {
            ship: ShipRef::Npc(crate::traffic::npc::NpcId(999)),
            class: ShipId(999),
            position: Vec2::new(-999.0, 999.0),
            heading: 99.0,
            frames: FrameSpec::NONE,
            shown: ShipDrawn::Hidden,
            cloak: CloakShown::Hidden,
            gate: GateFade::Entering { ticks_left: 99 },
            hover: HoverPlace::Unplaced,
            framed: Some(ShipId(999)),
            on_radar: true,
            iff: ShipIff::Threat,
            mass: 999.0,
            ..FrameShip::default()
        }
    }

    /// A frame unlike any a test session fills, in every field.
    pub(super) fn stale() -> FlightFrame {
        FlightFrame {
            nav_mode: NavMode::Hyperspace(Some(SystemId(999))),
            system: SystemId(999),
            course_shown: vec![SystemId(998)],
            course_plotted: true,
            docked: true,
            jump_under_way: true,
            jump_readiness: JumpReadiness::Underway,
            texts_queued: u64::MAX,
            map_key: MapKey {
                system: SystemId(999),
                explored: vec![SystemId(997)],
                charted: vec![SystemId(997)],
                known: BTreeSet::from([SystemId(996)]),
                course: vec![SystemId(995)],
                nav: NavMode::Hyperspace(Some(SystemId(999))),
            },
            ships: vec![stale_ship(); 3],
            drawn: vec![(
                ShipRef::Npc(crate::traffic::npc::NpcId(999)),
                ShipLayer::Led,
            )],
            shots: vec![FrameShot {
                id: ShotId(999),
                weapon: WeaponId(999),
                position: Vec2::new(9.0, 9.0),
                velocity: Vec2::new(9.0, 9.0),
                heading: 9.0,
                age: 9,
                spin_timer: 9,
                layer: ShotLayer::OverShips,
            }],
            beams: vec![FrameBeam {
                firer: ShipRef::Npc(crate::traffic::npc::NpcId(999)),
                start: Vec2::new(9.0, 9.0),
                end: Vec2::new(99.0, 99.0),
                weapon: WeaponId(999),
                fade: 9,
            }],
            cargo_boxes: vec![FrameBox {
                position: Vec2::new(9.0, 9.0),
                velocity: Vec2::new(9.0, 9.0),
                frame: 9,
                fade: 9,
                graphic: 9,
            }],
            asteroids: vec![FrameAsteroid {
                position: Vec2::new(9.0, 9.0),
                velocity: Vec2::new(9.0, 9.0),
                frame: 9.0,
                kind: 9,
            }],
            stellars: vec![FrameStellar {
                id: StellarId(999),
                shown: true,
                destroyed: true,
                drawn: DrawnSheet::Dead,
                counted: DrawnSheet::Dead,
                frozen: true,
                counts_none: true,
                gate_frame: Some(9),
            }],
            effective_murk: 99,
            system_murk: 99,
            system_background: Some(0x99),
            double_time: true,
            hyper_gamma: 9.0,
            traffic_ships: vec![ShipId(999)],
            hud: stale_hud(),
        }
    }

    /// A HUD unlike any a test session fills, in every field.
    fn stale_hud() -> crate::flight_frame::FrameHud {
        use crate::brackets::{NavBrackets, Standing, TargetBrackets};
        use crate::escort::{ClassRow, EscortClass};
        use crate::reserves::{Gauge, Reserves};
        crate::flight_frame::FrameHud {
            reserves: Reserves {
                shield: Gauge::full(999.0),
                armor: Gauge::full(999.0),
                fuel: Gauge::full(999.0),
            },
            cargo: crate::status_cargo::StatusCargo {
                cash: -999,
                ..crate::status_cargo::StatusCargo::default()
            },
            secondary: Some(WeaponId(999)),
            secondary_line_count: Some(999),
            radar_gear: crate::iff::RadarGear {
                iff: true,
                density_scanner: true,
                anti_interference: 999,
            },
            interference: 99,
            interface_government: Some(crate::catalog::GovtId(999)),
            nav_brackets: Some(NavBrackets {
                stellar: StellarId(999),
                position: Vec2::new(9.0, 9.0),
                expand: 9.0,
                dominated: true,
            }),
            target_brackets: Some(TargetBrackets {
                npc: crate::traffic::npc::NpcId(999),
                expand: 9.0,
                standing: Standing::Hostile,
            }),
            red_alert: Some(crate::red_alert::AlertIcon::Red),
            escort_rows: EscortClass::ALL.map(|class| ClassRow {
                class,
                present: true,
                order: Some(crate::escort::EscortOrder::Hold),
            }),
        }
    }

    /// `session`'s frame, filled over [`stale`].
    pub(super) fn filled(session: &Session) -> FlightFrame {
        let mut frame = stale();
        session.fill_frame(&mut frame);
        frame
    }

    #[test]
    fn a_fill_writes_the_sessions_nav_mode_over_a_stale_one() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        let frame = filled(&session);
        assert_eq!(frame.nav_mode, NavMode::Stellar(Some(StellarId(131))));
        assert_eq!(frame.nav_mode, session.nav_mode());
    }

    #[test]
    fn a_fill_writes_the_current_system() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(filled(&session).system, SystemId(130));
    }

    #[test]
    fn a_fill_writes_the_course_as_it_is_flown_and_whether_one_is_plotted() {
        let mut session = Session::start(&catalog()).expect("starts");
        let frame = filled(&session);
        assert_eq!(
            (frame.course_shown, frame.course_plotted),
            (Vec::new(), false)
        );
        session.plot_course(SystemId(132)).expect("a route");
        let frame = filled(&session);
        assert_eq!(frame.course_shown, [SystemId(131), SystemId(132)]);
        assert_eq!(frame.course_shown, session.course_shown());
        assert!(frame.course_plotted);
    }

    #[test]
    fn a_fill_writes_whether_the_player_is_docked() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        assert!(!filled(&session).docked);
        session.select_stellar(StellarPick::Slot(1));
        land_now(&mut session).expect("lands");
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert!(frame.docked);
    }

    #[test]
    fn a_fill_writes_whether_a_jump_is_under_way_and_the_ships_readiness() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let frame = filled(&session);
        assert_eq!(
            (frame.jump_under_way, frame.jump_readiness),
            (false, JumpReadiness::Blocked)
        );
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert_eq!(
            (frame.jump_under_way, frame.jump_readiness),
            (false, JumpReadiness::Clear)
        );
        session.begin_jump().expect("accepted");
        assert!(session.preparing_jump().is_some());
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert!(frame.jump_under_way, "the pre-jump stage counts");
        assert_eq!(frame.jump_readiness, JumpReadiness::Underway);
    }

    #[test]
    fn a_fill_counts_the_jump_itself_under_way() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        assert!(session.jumping().is_some());
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert!(frame.jump_under_way);
    }

    #[test]
    fn a_fill_writes_how_many_texts_are_queued() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(filled(&session).texts_queued, session.texts_queued());
    }

    #[test]
    fn a_fill_writes_what_the_course_map_is_drawn_from() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.mark_known([SystemId(132)]);
        session.plot_course(SystemId(131)).expect("a route");
        let frame = filled(&session);
        assert_eq!(
            frame.map_key,
            MapKey {
                system: SystemId(130),
                explored: session.pilot().explored().collect(),
                charted: session.pilot().charted().collect(),
                known: session.known().clone(),
                course: vec![SystemId(131)],
                nav: session.nav_mode(),
            }
        );
        assert!(frame.map_key.known.contains(&SystemId(132)));
        assert!(frame.map_key.explored.contains(&SystemId(130)));
    }

    #[test]
    fn a_change_to_any_of_the_maps_inputs_changes_its_key() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = filled(&session).map_key;
        assert_eq!(filled(&session).map_key, before, "nothing changed");
        session.mark_known([SystemId(133)]);
        let known = filled(&session).map_key;
        assert_ne!(known, before);
        session.plot_course(SystemId(131)).expect("a route");
        assert_ne!(filled(&session).map_key, known);
    }

    // The world (rdm:phase/view-sim-seam/phase-7-world-frame).

    #[test]
    fn a_fill_writes_every_ship_the_player_first_then_each_npc_in_the_systems_order() {
        let catalog = crate::session::tests::armed();
        let session = crate::session::tests::facing_an_npc(&catalog, 90);
        let frame = filled(&session);
        let mut expected = vec![player_record(&session)];
        expected.extend(session.npcs().iter().map(|npc| npc_record(&session, npc)));
        assert_eq!(expected.len(), 2, "the player and its NPC");
        assert_eq!(frame.ships, expected);
        assert_eq!(frame.player(), expected[0]);
        assert_eq!(frame.npcs().count(), 1);
        assert_eq!(frame.ships[1].position, Vec2::new(0.0, -100.0));
    }

    #[test]
    fn a_fill_writes_the_ships_drawn_in_their_layers() {
        let catalog = crate::session::tests::armed();
        let session = crate::session::tests::facing_an_npc(&catalog, 0);
        let frame = filled(&session);
        assert!(!frame.drawn.is_empty());
        assert_eq!(frame.drawn, session.draw_layers());
    }

    #[test]
    fn a_fill_writes_the_shots_in_the_order_they_are_drawn() {
        let catalog = crate::session::tests::armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(crate::session::tests::FIRE);
        while session.shots().len() < 2 {
            crate::testkit::work(&mut session, &catalog, crate::flight::Controls::default());
        }
        session.space.combat.shots_mut()[0].layer = ShotLayer::OverShips;
        let frame = filled(&session);
        let drawn: Vec<FrameShot> = session
            .shots_drawn()
            .into_iter()
            .map(FrameShot::of)
            .collect();
        assert_eq!(frame.shots, drawn);
        assert_eq!(frame.shots[1].id, session.shots()[0].id, "over the ships");
        assert_eq!(frame.shots[1].layer, ShotLayer::OverShips);
    }

    #[test]
    fn a_fill_writes_the_beams_being_fired() {
        let catalog = crate::testkit::FakePilotCatalog {
            weapons: vec![crate::catalog::WeaponRecord {
                guidance: 0,
                count: 3,
                reload: 10,
                speed: 0,
                beam_length: 200,
                ..crate::session::tests::blaster()
            }],
            ..crate::session::tests::armed()
        };
        let mut session = crate::session::tests::facing_an_npc(&catalog, 0);
        session.hold_trigger(crate::session::tests::FIRE);
        crate::testkit::work(&mut session, &catalog, crate::flight::Controls::default());
        let frame = filled(&session);
        assert!(!session.beams().is_empty(), "a beam fired");
        let beam = &session.beams()[0];
        assert_ne!(beam.start, beam.end, "a start and an end to tell apart");
        assert_eq!(
            frame.beams,
            [FrameBeam {
                firer: ShipRef::Player,
                start: beam.start,
                end: beam.end,
                weapon: beam.weapon.id,
                fade: beam.fade,
            }]
        );
    }

    #[test]
    fn a_fill_writes_the_cargo_boxes_in_space() {
        let mut session = Session::start(&catalog()).expect("starts");
        // Of graphic 5, on frame 0: each field its own value.
        session.space.boxes.spawn_scoopable(
            Vec2::new(30.0, -40.0),
            None,
            5,
            &mut crate::chance::NeverFires,
        );
        let frame = filled(&session);
        let spawned = &session.cargo_boxes()[0];
        assert_ne!(spawned.velocity, spawned.position);
        assert_ne!(spawned.frame, spawned.graphic);
        assert_eq!(
            frame.cargo_boxes,
            [FrameBox {
                position: Vec2::new(30.0, -40.0),
                velocity: spawned.velocity,
                frame: spawned.frame,
                fade: spawned.fade(),
                graphic: 5,
            }]
        );
    }

    #[test]
    fn a_fill_writes_the_asteroids_in_the_field() {
        let mut session = Session::start(&catalog()).expect("starts");
        let asteroid = crate::asteroid::Asteroid {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(1.0, -1.0),
            frame: 3.5,
            spin: 0.5,
            strength: 10,
            kind: 2,
        };
        session.space.asteroids.place(asteroid);
        let frame = filled(&session);
        assert_eq!(
            frame.asteroids,
            [FrameAsteroid {
                position: Vec2::new(10.0, 20.0),
                velocity: Vec2::new(1.0, -1.0),
                frame: 3.5,
                kind: 2,
            }]
        );
    }

    /// `session`'s record of `id`, shown or not, from its own getters.
    fn stellar_record(session: &Session, id: StellarId, shown: bool) -> FrameStellar {
        FrameStellar {
            id,
            shown,
            destroyed: session.galaxy().destroyed(id),
            drawn: session.drawn_sheet(id),
            counted: session.counted_sheet(id),
            frozen: session.frame_frozen(id),
            counts_none: session.counts_no_frames(id),
            gate_frame: session.gate_frame(id),
        }
    }

    #[test]
    fn a_fill_writes_each_stellar_shown_in_nav_order_with_how_it_is_drawn() {
        let session = Session::start(&three_stellars()).expect("starts");
        let frame = filled(&session);
        let shown: Vec<StellarId> = session.stellars_shown().collect();
        assert_eq!(shown, [StellarId(131), StellarId(128), StellarId(129)]);
        assert!(frame.stellars_shown().eq(shown.iter().copied()));
        let expected: Vec<FrameStellar> = shown
            .iter()
            .map(|&id| stellar_record(&session, id, true))
            .collect();
        assert_eq!(frame.stellars, expected);
        assert_eq!(frame.stellar(StellarId(128)), Some(&expected[1]));
    }

    #[test]
    fn a_stellar_its_system_lists_but_doesnt_show_is_recorded_unshown_after_those_shown() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.chart.sites.remove(0);
        let frame = filled(&session);
        assert_eq!(
            frame.stellars,
            [
                stellar_record(&session, StellarId(128), true),
                stellar_record(&session, StellarId(129), true),
                stellar_record(&session, StellarId(131), false),
            ]
        );
        assert!(frame.stellars_shown().eq([StellarId(128), StellarId(129)]));
    }

    #[test]
    fn a_fill_writes_the_murk_the_background_double_time_the_gamma_and_the_traffic() {
        let catalog = crate::session::tests::armed();
        let session = crate::session::tests::facing_an_npc(&catalog, 0);
        let frame = filled(&session);
        assert_eq!(frame.effective_murk, session.effective_murk());
        assert_eq!(frame.system_murk, session.system_murk());
        assert_eq!(frame.system_background, session.system_background());
        assert_eq!(frame.double_time, session.double_time());
        assert_eq!(frame.hyper_gamma.to_bits(), session.hyper_gamma().to_bits());
        assert_eq!(frame.traffic_ships, session.traffic_ships());
        assert!(!frame.traffic_ships.is_empty(), "the system's traffic");
    }

    #[test]
    fn a_failed_sessions_default_frame_holds_the_player_alone_in_full() {
        let frame = FlightFrame::default();
        assert_eq!(frame.ships, [FrameShip::default()]);
        assert_eq!(frame.player().shown, ShipDrawn::Full);
        assert_eq!(frame.player().hover, HoverPlace::Own);
        assert_eq!(frame.drawn, [(ShipRef::Player, ShipLayer::Ships)]);
        assert!(frame.shots.is_empty() && frame.stellars.is_empty());
        assert_eq!(frame.effective_murk, 0);
        assert_eq!(frame.system_background, None);
    }
}
