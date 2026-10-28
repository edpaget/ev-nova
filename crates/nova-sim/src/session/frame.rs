//! [`Session::fill_frame`]: what the flight view draws, filled into a
//! [`FlightFrame`] for the view's `FlightSim` port
//! (see [`flight_frame`](crate::flight_frame)): the nav and course here,
//! the world's ships, shots and stellars in the `world` module.

use super::Session;
use crate::flight_frame::{FlightFrame, FlightOpening, Gates, MapKey};

mod hud;
mod names;
mod world;

impl Session {
    /// Fills `into` with what the flight view draws now. Every field is
    /// written, so a frame kept from an earlier fill, or from another
    /// session, shows nothing stale: the destructuring names every field,
    /// so a field added to [`FlightFrame`] doesn't compile until it is
    /// filled here. The player's ship's name, the date and the systems
    /// explored are written into the strings and the list the frame kept,
    /// not fresh ones.
    pub fn fill_frame(&self, into: &mut FlightFrame) {
        let FlightFrame {
            nav_mode,
            system,
            course_shown,
            course_plotted,
            landed,
            jump_under_way,
            warping,
            warp_end_due,
            fate,
            hold_expired,
            leave_due,
            jump_readiness,
            texts_queued,
            gates,
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
            target,
            death_descs,
            player_ship_name,
            long_date,
            explored,
        } = into;
        *nav_mode = self.nav_mode();
        *system = self.system();
        *course_shown = self.course_shown();
        *course_plotted = !self.course().is_empty();
        *landed = self.mode.landed();
        *jump_under_way = self.mode.jump().is_some();
        *warping = super::jumping(self.mode.jump()).is_some();
        *warp_end_due = self.ship.warp_end_due(self.mode.jump());
        *fate = self.ship.player_fate(self.player_end, self.pod);
        *hold_expired = self.ship.hold_expired(self.player_end);
        *leave_due = self.run.leave_due(self.mode.landed());
        *jump_readiness = self.jump_readiness();
        *texts_queued = self.texts_queued();
        *gates = Gates::from_fn(|command| self.takes(command));
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
        self.fill_target(target);
        *death_descs = self.sabotage_death_descs();
        write_name(player_ship_name, self.pilot.ship_name());
        crate::date::write_long_date_text(long_date, self.pilot.date, &self.statics.date_affixes);
        explored.clear();
        explored.extend(self.pilot.explored());
    }

    /// What the view is handed as the flight opens ([`FlightOpening`]):
    /// the weapons of the session's arsenal, by ascending ID
    /// ([`Session::weapon_ids`]), the names it read as it started
    /// ([`Names`](crate::flight_frame::Names), built once here, as nothing
    /// changes them for the session's life), and no failure, as the
    /// session started.
    #[must_use]
    pub fn opening(&self) -> FlightOpening {
        FlightOpening {
            weapons: self.weapon_ids(),
            names: self.statics.names(),
            failure: None,
        }
    }
}

/// Writes `name` into `into`, reusing the name it kept when both are some,
/// so a kept frame's fill allocates nothing for an unchanged name.
fn write_name(into: &mut Option<String>, name: Option<&str>) {
    match (name, into.as_mut()) {
        (Some(name), Some(kept)) => {
            kept.clear();
            kept.push_str(name);
        }
        (name, _) => *into = name.map(str::to_owned),
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{ShipId, StellarId, SystemId, WeaponId};
    use crate::cloak::CloakShown;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
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
    use crate::player_fate::PlayerCommand;
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
            landed: Some(StellarId(999)),
            jump_under_way: true,
            warping: true,
            warp_end_due: true,
            fate: crate::player_fate::PlayerFate::GameOver,
            hold_expired: true,
            leave_due: true,
            jump_readiness: JumpReadiness::Underway,
            texts_queued: u64::MAX,
            // Only the cloak taken: no state of the player's gives it.
            gates: Gates::from_fn(|command| command == PlayerCommand::Cloak),
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
                iff: Some(crate::iff::StellarIff::Dominated),
            }],
            effective_murk: 99,
            system_murk: 99,
            system_background: Some(0x99),
            double_time: true,
            hyper_gamma: 9.0,
            traffic_ships: vec![ShipId(999)],
            hud: stale_hud(),
            target: Some(crate::flight_frame::FrameTarget {
                npc: crate::traffic::npc::NpcId(999),
                name: "Stale".to_owned(),
                subtitle: Some("Stale".to_owned()),
                status: crate::target_panel::TargetStatus::Waiting,
                tag: Some(crate::target_panel::TargetTag::Fighter),
            }),
            death_descs: vec![999],
            player_ship_name: Some("Stale".to_owned()),
            long_date: "A stale date".to_owned(),
            explored: vec![SystemId(997), SystemId(999)],
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
            escorts_flying: true,
        }
    }

    /// `session`'s frame, filled over [`stale`].
    pub(in crate::session) fn filled(session: &Session) -> FlightFrame {
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
    fn a_fill_writes_the_stellar_the_player_is_docked_at() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        assert_eq!(filled(&session).landed, None);
        session.select_stellar(StellarPick::Slot(1));
        land_now(&mut session).expect("lands");
        let mut frame = FlightFrame::default();
        session.fill_frame(&mut frame);
        assert_eq!(frame.landed, Some(StellarId(128)));
        assert_eq!(frame.landed, session.landed());
        assert!(frame.docked());
    }

    #[test]
    fn a_fill_writes_whether_the_jump_itself_has_begun_and_its_warp_has_ended() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let frame = filled(&session);
        assert_eq!((frame.warping, frame.warp_end_due), (false, false));
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        session.begin_jump().expect("accepted");
        let frame = filled(&session);
        assert!(frame.jump_under_way);
        assert!(!frame.warping, "the pre-jump stage isn't the warp");
        begin_jump_now(&mut session, &catalog).expect("jumps");
        let frame = filled(&session);
        assert_eq!((frame.warping, frame.warp_end_due), (true, false));
        for _ in 0..1000 {
            if session.warp_end_due() {
                break;
            }
            crate::testkit::work(&mut session, &catalog, crate::flight::Controls::default());
        }
        assert!(session.warp_end_due(), "the warp ended");
        let frame = filled(&session);
        assert_eq!((frame.warping, frame.warp_end_due), (true, true));
    }

    #[test]
    fn a_fill_writes_the_players_fate_and_whether_its_hold_has_expired() {
        use crate::player_fate::PlayerFate;
        let session = Session::start(&catalog()).expect("starts");
        let frame = filled(&session);
        assert_eq!(
            (frame.fate, frame.hold_expired),
            (PlayerFate::Flying, false)
        );
        let mut session = crate::session::tests::expired_hold();
        let frame = filled(&session);
        assert_eq!(frame.fate, PlayerFate::Holding { ticks_left: 0 });
        assert!(frame.hold_expired);
        session.begin_fade();
        let frame = filled(&session);
        assert_eq!((frame.fate, frame.hold_expired), (PlayerFate::Fading, true));
        assert_eq!(frame.fate, session.player_fate());
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
    fn a_fill_writes_whether_a_leave_waits_for_the_take_off() {
        let catalog = crate::session::script_effects::tests::moving();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(!filled(&session).leave_due);
        session = crate::session::script_effects::tests::leave_held(&catalog);
        assert!(filled(&session).leave_due);
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
            iff: session.stellar_iff(id),
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
    fn a_fill_writes_the_systems_wormhole_and_hypergate_shown_in_nav_order() {
        use crate::session::stellar_picks::tests::{hypergate, wormhole};
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            crate::testkit::planet(128, 30.0, -40.0),
            wormhole(150, 0.0, 300.0),
            hypergate(151, 0.0, -300.0),
        ];
        let session = Session::start(&catalog).expect("starts");
        let frame = filled(&session);
        let gates = [StellarId(128), StellarId(150), StellarId(151)];
        assert!(frame.stellars_shown().eq(gates), "{:?}", frame.stellars);
        let expected: Vec<FrameStellar> = gates
            .iter()
            .map(|&id| stellar_record(&session, id, true))
            .collect();
        assert_eq!(frame.stellars, expected);
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

    // The gates (rdm:phase/view-sim-seam/phase-20-view-frame-fills-per-tick).

    /// Breaks the player up, as a ship whose armour has gone does.
    fn breaking_up(session: &mut Session) {
        session.set_player_condition(Condition::Dying {
            ticks_left: 30,
            starting: false,
        });
    }

    /// Explodes the player's ship.
    fn exploded(session: &mut Session) {
        session.set_player_condition(Condition::Destroyed);
    }

    /// Whether `gate` is taken in `session`'s frame, filled over
    /// [`stale`], checked against [`Session::takes`].
    fn gate_filled(session: &Session, gate: PlayerCommand) -> bool {
        let taken = filled(session).gates.takes(gate);
        assert_eq!(taken, session.takes(gate), "{gate:?}: the session's");
        taken
    }

    /// Pins `gate` in the frame: taken while flying, and refused once
    /// `refuse` has put the session in the first state that refuses it.
    fn pins_gate(gate: PlayerCommand, refuse: fn(&mut Session)) {
        let mut session = Session::start(&catalog()).expect("starts");
        assert!(gate_filled(&session, gate), "{gate:?}: taken flying");
        refuse(&mut session);
        assert!(!gate_filled(&session, gate), "{gate:?}: refused");
    }

    /// Pins `gate`, which a breaking-up player still takes, in the frame:
    /// taken while flying and breaking up, refused once exploded.
    fn pins_gate_to_the_explosion(gate: PlayerCommand) {
        let mut session = Session::start(&catalog()).expect("starts");
        assert!(gate_filled(&session, gate), "{gate:?}: taken flying");
        breaking_up(&mut session);
        assert!(gate_filled(&session, gate), "{gate:?}: taken breaking up");
        exploded(&mut session);
        assert!(!gate_filled(&session, gate), "{gate:?}: refused exploded");
    }

    #[test]
    fn the_map_gate_is_refused_under_the_take_off_grace_and_breaking_up() {
        pins_gate(PlayerCommand::Map, Session::start_takeoff_grace);
        pins_gate(PlayerCommand::Map, breaking_up);
    }

    #[test]
    fn the_land_gate_is_refused_breaking_up() {
        pins_gate(PlayerCommand::Land, breaking_up);
    }

    #[test]
    fn the_mission_info_gate_is_refused_breaking_up() {
        pins_gate(PlayerCommand::MissionInfo, breaking_up);
    }

    #[test]
    fn the_cloak_gate_is_refused_breaking_up() {
        pins_gate(PlayerCommand::Cloak, breaking_up);
    }

    #[test]
    fn the_jump_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Jump);
    }

    #[test]
    fn the_hail_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Hail);
    }

    #[test]
    fn the_board_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Board);
    }

    #[test]
    fn the_jettison_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Jettison);
    }

    #[test]
    fn the_target_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Target);
    }

    #[test]
    fn the_nav_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Nav);
    }

    #[test]
    fn the_select_system_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::SelectSystem);
    }

    #[test]
    fn the_select_weapon_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::SelectWeapon);
    }

    #[test]
    fn the_deselect_weapon_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::DeselectWeapon);
    }

    #[test]
    fn the_escort_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Escort);
    }

    #[test]
    fn the_eject_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::Eject);
    }

    #[test]
    fn the_player_info_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::PlayerInfo);
    }

    #[test]
    fn the_self_destruct_gate_is_refused_once_exploded() {
        pins_gate_to_the_explosion(PlayerCommand::SelfDestruct);
    }

    #[test]
    fn a_fill_writes_a_sabotaged_players_death_descs_and_none_unsabotaged() {
        use crate::catalog::OutfitId;
        use crate::session::sabotage::tests::{KILL, sabotaged};
        let catalog = sabotaged();
        let mut session = crate::session::tests::facing_an_npc(&catalog, 180);
        assert_eq!(
            filled(&session).death_descs,
            Vec::<i16>::new(),
            "unsabotaged"
        );
        session.pilot.outfits.insert(OutfitId(KILL), 1);
        session.reset_precalced();
        let frame = filled(&session);
        assert_eq!(frame.death_descs, [4100], "over the stale list");
        assert_eq!(frame.death_descs, session.sabotage_death_descs());
    }

    // The names the flight decides
    // (rdm:phase/view-sim-seam/phase-21-names-off-the-port).

    #[test]
    fn a_fill_writes_the_players_ship_name_and_none_unnamed() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.ship_name = Some("Kestrel".to_owned());
        let frame = filled(&session);
        assert_eq!(frame.player_ship_name.as_deref(), Some("Kestrel"));
        assert_eq!(
            frame.player_ship_name.as_deref(),
            session.pilot().ship_name()
        );
        session.pilot.ship_name = None;
        assert_eq!(filled(&session).player_ship_name, None, "over a stale one");
    }

    #[test]
    fn a_fill_writes_the_long_date_and_moves_on_with_the_date() {
        let mut session = Session::start(&catalog()).expect("starts");
        let frame = filled(&session);
        assert_eq!(frame.long_date, session.long_date_text());
        assert!(!frame.long_date.is_empty(), "the date is said");
        session.pilot.date = session.pilot.date.next_day();
        let moved = filled(&session);
        assert_ne!(moved.long_date, frame.long_date, "the date moved on");
        assert_eq!(moved.long_date, session.long_date_text());
    }

    #[test]
    fn a_fill_writes_every_system_explored_one_the_nav_doesnt_select_among_them() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.explore(SystemId(131));
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
        session.pilot.explore(SystemId(133));
        let frame = filled(&session);
        let explored: Vec<SystemId> = session.pilot().explored().collect();
        assert_eq!(frame.explored, explored);
        assert!(
            frame.has_explored(SystemId(133)),
            "one the nav doesn't select"
        );
        assert!(frame.has_explored(SystemId(131)), "the nav's own");
        assert!(!frame.has_explored(SystemId(132)), "not yet explored");
        assert!(!frame.has_explored(SystemId(999)), "over the stale set");
        let explore = crate::control::SetExpr::parse("X132").expect("parses");
        session.run_set(&explore, &mut crate::testkit::Scripted::default());
        assert!(
            filled(&session).has_explored(SystemId(132)),
            "explored by X"
        );
    }

    // What changes the session behind the port without a command
    // (rdm:phase/view-sim-seam/phase-20-view-frame-fills-per-tick): the view
    // keeps its frame across these, so none may change a fill.

    #[test]
    fn the_ports_rolling_reads_leave_the_frame_as_it_was() {
        use crate::chance::NeverFires;
        let catalog = crate::session::tests::shipbuying();
        let mut session = crate::session::tests::outfitted(&catalog);
        let before = filled(&session);
        assert!(session.outfitter(&mut NeverFires).is_some(), "an outfitter");
        assert_eq!(filled(&session), before, "the outfitter's rolls");
        assert!(session.shipyard(&mut NeverFires).is_some(), "a shipyard");
        assert_eq!(filled(&session), before, "the shipyard's rolls");
        let _ = session.escorts_for_hire(&mut NeverFires);
        assert_eq!(filled(&session), before, "the hire list's rolls");
        let _ = session.ship_naming(ShipId(129), &catalog, &mut NeverFires);
        assert_eq!(filled(&session), before, "the name prompt");
        let _ = session.load_description(Some("{b5 \"set\" \"clear\"}"), None, &mut NeverFires);
        assert_eq!(filled(&session), before, "a description's load");
    }

    #[test]
    fn a_drain_leaves_the_frame_as_it_was() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        session.space.say(
            &mut session.out,
            crate::message::SimMessage::Buoy("Welcome.".to_owned()),
        );
        session
            .out
            .emit(crate::session::SessionEvent::GalaxyChanged);
        session.out.save_due = true;
        session.out.hold_expiry_due = true;
        session.out.texts_queued += 1;
        let before = filled(&session);
        let mut drained = Vec::new();
        session.drain_events(&mut drained);
        assert!(
            drained.len() >= 5,
            "a sound, a message, an event, the save and the hold: {drained:?}"
        );
        let after = filled(&session);
        assert_eq!(after.texts_queued, before.texts_queued, "a running count");
        assert_eq!(after, before);
    }

    #[test]
    fn a_sessions_opening_lists_its_weapons_by_ascending_id_and_no_failure() {
        let catalog = crate::testkit::FakePilotCatalog {
            weapons: vec![
                crate::catalog::WeaponRecord {
                    id: WeaponId(140),
                    ..crate::session::tests::blaster()
                },
                crate::session::tests::blaster(),
            ],
            ..crate::session::tests::armed()
        };
        let session = Session::start(&catalog).expect("starts");
        let opening = session.opening();
        assert_eq!(opening.weapons, session.weapon_ids());
        assert!(opening.weapons.len() >= 2, "{:?}", opening.weapons);
        assert!(opening.weapons.is_sorted(), "by ascending ID");
        assert_eq!(opening.failure, None);
        let failed = FlightOpening::failed("no chär");
        assert_eq!(failed.weapons, [], "a failed flight lists none");
        assert_eq!(failed.failure.as_deref(), Some("no chär"));
    }

    #[test]
    fn a_failed_sessions_default_frame_holds_the_player_alone_in_full() {
        let frame = FlightFrame::default();
        assert_eq!(frame.gates, Gates::EVERY, "nothing to refuse with");
        assert_eq!(frame.death_descs, Vec::<i16>::new());
        assert_eq!(frame.ships, [FrameShip::default()]);
        assert_eq!(frame.player().shown, ShipDrawn::Full);
        assert_eq!(frame.player().hover, HoverPlace::Own);
        assert_eq!(frame.drawn, [(ShipRef::Player, ShipLayer::Ships)]);
        assert!(frame.shots.is_empty() && frame.stellars.is_empty());
        assert_eq!(frame.effective_murk, 0);
        assert_eq!(frame.system_background, None);
        assert_eq!(frame.player_ship_name, None);
        assert_eq!(frame.long_date, "");
        assert!(frame.explored.is_empty(), "every system unexplored");
        assert!(!frame.has_explored(SystemId(130)));
    }
}
