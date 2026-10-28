use super::*;
use crate::flight::sim_port::FlightSim;
use nova_sim::SessionEvent;
use nova_sim::flight_frame::FlightEvent;

// Session events.

fn bars_read<S: FlightSim>(view: &FlightView<FakeCatalog, S>) -> Vec<i16> {
    view.catalog().bars_asked.borrow().clone()
}

#[test]
fn a_new_ship_types_missing_bar_keeps_the_layout_over_picture_128() {
    // Which government a new ship type's bar is chosen by is the session's.
    let mut view = super::faked_hud::barred(governed(catalog(), 129, 131), among(Vec::new()));
    assert_eq!(bars_read(&view), [128]);
    view.sim.frame_mut().hud.interface_government = Some(GovtId(150));
    view.tick(TICK);
    assert_eq!(bars_read(&view), [128, 131]);
    assert_eq!(
        view.status_bar(),
        Ok(&StatusBar {
            layout: layout(),
            background: Some(hud::Background {
                id: 128,
                width: 194.0,
                height: 767.0,
            }),
        })
    );
}

// The stellars shown.

/// The IDs of the stellars `view`'s scene draws.
fn scene_ids<S: FlightSim>(view: &FlightView<FakeCatalog, S>) -> Vec<StellarId> {
    view.scene()
        .expect("laid out")
        .stellars()
        .iter()
        .map(|stellar| stellar.id)
        .collect()
}

/// [`catalog`] with Sol listing Proxima (140) too: Sol, the lower and
/// visible, owns it, though Alpha Centauri lists it.
pub(super) fn proxima_in_sol() -> FakeCatalog {
    let mut catalog = catalog();
    catalog
        .sites
        .push(site(140, (0.0, 0.0), StellarFlags::CAN_LAND));
    catalog
}

/// Sol owning a wormhole, Hole (150), 300 below the centre the ship
/// starts at, and a hypergate, Gate (151), 300 above it, each a square
/// on the radar as the data adapter reads them, after Earth and the
/// Moon.
fn sol_with_gates() -> Faked {
    let mut catalog = catalog();
    catalog.sites.extend([
        LandingSite {
            flags2: 0x2000,
            ..site(
                150,
                (0.0, 300.0),
                StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            )
        },
        LandingSite {
            flags2: 0x1000,
            ..site(151, (0.0, -300.0), StellarFlags::CAN_LAND)
        },
    ]);
    catalog.sol_more = vec![
        StellarContents {
            radar_square: true,
            ..stellar(150, "Hole", (0, 300), 1)
        },
        StellarContents {
            radar_square: true,
            ..stellar(151, "Gate", (0, -300), 1)
        },
    ];
    // Which the session shows is its own: both, in nav order.
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]);
    frame
        .stellars
        .extend([150, 151].map(|id| record::stellar(StellarId(id))));
    let mut view = super::faked_hud::barred(catalog, frame);
    view.sim
        .map_mut()
        .stellar_names
        .push((StellarId(150), "Hole".to_owned()));
    view
}

/// `_HandleStellarSprites` draws every active stellar the system owns,
/// gates too (@0x2e713-0x2e74f): no gate test.
#[test]
fn the_flight_scene_draws_the_systems_wormhole_and_hypergate() {
    let view = sol_with_gates();
    let ids = scene_ids(&view);
    assert!(ids.contains(&StellarId(150)), "{ids:?}");
    assert!(ids.contains(&StellarId(151)), "{ids:?}");
}

/// `_DrawStatusRadar` draws every nav slot, gates as squares
/// (@0x498a0-0x49a8e): no gate test.
#[test]
fn the_radar_shows_the_systems_wormhole_and_hypergate_as_squares() {
    let view = sol_with_gates();
    let radar = view.radar_shown(&view.frame_now());
    let gates: Vec<(Point, bool)> = radar
        .stellars
        .iter()
        .filter(|stellar| stellar.position.x == 0.0 && stellar.position.y.abs() == 300.0)
        .map(|stellar| (stellar.position, stellar.square))
        .collect();
    assert_eq!(
        gates,
        [
            (Point::new(0.0, 300.0), true),
            (Point::new(0.0, -300.0), true)
        ]
    );
}

/// `_DrawStatusNav` names any selected stellar (@0x4a09c-0x4a18c): F5
/// within 500 pixels selects the wormhole, and the nav area names it.
#[test]
fn f5_near_the_wormhole_selects_it_and_the_nav_area_names_it() {
    // That F5 within 500 pixels picks the wormhole is the session's.
    let mut view = sol_with_gates();
    tap(&mut view, NEAREST_STELLAR_KEY);
    view.tick(TICK);
    assert_eq!(
        super::faked_screen::picks_since(&view, 0),
        [FlightCommand::SelectStellar(nova_sim::StellarPick::Nearest)]
    );
    view.change_sim(|sim| sim.frame_mut().nav_mode = NavMode::Stellar(Some(StellarId(150))));
    view.redraw_status();
    assert_eq!(super::faked_hud::nav_of(&view), [hud::NAV_STELLAR, "Hole"]);
}

// The galaxy overlay.

/// Whether `view`'s course map draws system `id`.
fn on_the_course_map<S: FlightSim>(view: &FlightView<FakeCatalog, S>, id: i16) -> bool {
    view.map.model().system(SystemId(id)).is_some()
}

#[test]
fn the_course_map_shows_the_course_the_flight_flies() {
    let mut view = faked(FlightFrame {
        course_shown: vec![SystemId(131)],
        course_plotted: true,
        ..in_sol()
    });
    assert_eq!(view.map.route(), [SystemId(131)], "as the view starts");
    view.change_sim(|sim| {
        sim.set_frame(FlightFrame {
            course_plotted: true,
            ..in_sol()
        });
    });
    view.show_course();
    assert_eq!(view.map.route(), [], "none flown, one plotted");
}

/// The galaxy event, queued on `view`'s flight with its galaxy as
/// `galaxy`.
fn galaxy_changed(view: &mut Faked, galaxy: nova_sim::story::OverlaidGalaxy) {
    view.sim.map_mut().galaxy = Some(galaxy);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::GalaxyChanged));
}

#[test]
fn a_galaxy_event_draws_the_flights_galaxy_and_the_course_as_flown_before_the_redraws() {
    let mut view = faked(FlightFrame {
        course_shown: vec![SystemId(131)],
        course_plotted: true,
        ..in_sol()
    });
    view.sim.map_mut().galaxy = Some(galaxy_with(hiding(132)));
    view.sim.set_frame(FlightFrame {
        course_plotted: true,
        ..in_sol()
    });
    let redrawn = view.catch_up_then(|view, redrawn| {
        assert_eq!(redrawn, Redrawn::default(), "nothing asked");
        assert!(on_the_course_map(view, 132), "nothing told");
    });
    assert_eq!(redrawn, Redrawn::default());
    assert_eq!(view.map.route(), [SystemId(131)], "not before the event");
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::GalaxyChanged));
    let mut told = Vec::new();
    let redrawn = view.catch_up_then(|view, redrawn| {
        told.push(redrawn);
        assert!(!on_the_course_map(view, 132), "drawn first");
        assert_eq!(view.map.route(), [], "with the course as flown");
    });
    let galaxy = Redrawn {
        galaxy: true,
        ..Redrawn::default()
    };
    assert_eq!((told, redrawn), (vec![galaxy], galaxy));
}

#[test]
fn several_galaxy_changes_before_a_catch_up_draw_the_galaxy_as_it_is_then_once() {
    let mut view = faked(in_sol());
    galaxy_changed(&mut view, galaxy_with(hiding(132)));
    galaxy_changed(
        &mut view,
        galaxy_with(|overlay| {
            overlay.set_visible(SystemId(131), Some(false));
        }),
    );
    let mut told = 0;
    let redrawn = view.catch_up_then(|_, _| told += 1);
    assert!(redrawn.galaxy);
    assert_eq!(told, 1, "one reaction");
    assert!(on_the_course_map(&view, 132), "the galaxy as it is now");
    assert!(!on_the_course_map(&view, 131));
    assert!(!view.catch_up_then(|_, _| {}).galaxy, "once");
}

/// A scripted flight of `reborn` Sol, Alpha Reborn (133), at Alpha
/// Centauri's (131) place, explored, its nav as `nav`.
fn reborn_sol(nav: NavMode) -> Faked {
    let catalog = FakeCatalog {
        reborn: true,
        ..catalog()
    };
    let mut view = faked_in(
        catalog,
        FlightFrame {
            nav_mode: nav,
            ..in_sol()
        },
    );
    view.sim.map_mut().explored = vec![SystemId(130), SystemId(133)];
    view
}

#[test]
fn after_a_galaxy_event_the_nav_area_names_the_system_standing_for_a_hidden_next_hop() {
    let catalog = FakeCatalog {
        reborn: true,
        ..catalog()
    };
    let mut view = reborn_sol(NavMode::Hyperspace(Some(SystemId(133))));
    galaxy_changed(&mut view, galaxy_of(&catalog, hiding(131)));
    view.catch_up_then(|_, _| {});
    assert!(!on_the_course_map(&view, 131), "the galaxy drawn");
    assert_eq!(
        view.nav_now(),
        NavDisplay::Hyperspace {
            destination: hud::NavDestination::Named("Alpha Reborn".to_owned()),
            readiness: nova_sim::JumpReadiness::Blocked,
        }
    );
}

#[test]
fn after_a_galaxy_event_a_hop_with_no_system_at_its_place_shows_no_destination() {
    let catalog = FakeCatalog {
        reborn: true,
        ..catalog()
    };
    let mut view = reborn_sol(NavMode::Hyperspace(None));
    galaxy_changed(
        &mut view,
        galaxy_of(&catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
            overlay.set_visible(SystemId(133), Some(false));
        }),
    );
    view.catch_up_then(|_, _| {});
    assert_eq!(
        view.nav_now(),
        NavDisplay::Hyperspace {
            destination: hud::NavDestination::None,
            readiness: nova_sim::JumpReadiness::Blocked,
        }
    );
}

#[test]
fn with_the_map_open_the_events_still_ask_for_their_redraws() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    assert!(view.map_open);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::ShipChanged));
    let mut told = Vec::new();
    view.catch_up_then(|_, redrawn| told.push(redrawn));
    assert_eq!(
        told,
        [Redrawn {
            ship: true,
            system: true,
            ..Redrawn::default()
        }]
    );
    assert!(view.map_open, "still open");
}

#[test]
fn each_event_asks_for_its_own_redraw() {
    let mut view = faked(in_sol());
    for event in [
        SessionEvent::ShipChanged,
        SessionEvent::SystemChanged,
        SessionEvent::StellarChanged,
        SessionEvent::StellarsRebuilt,
        SessionEvent::StarfieldReset,
    ] {
        view.sim.queue_event(FlightEvent::Session(event));
    }
    let redrawn = view.catch_up_then(|_, _| {});
    assert_eq!(
        redrawn,
        Redrawn {
            ship: true,
            system: true,
            stellar: true,
            galaxy: false,
            rebuilt: true,
            stars: true,
        }
    );
    assert_eq!(view.catch_up_then(|_, _| {}), Redrawn::default(), "taken");
}

// The stellars' animation through the concrete flows (stellar-fidelity
// phase 12): a take-off, a real jump, a docked move and a load. What a
// frame draws of a stellar's record is `stellar_looks.rs`'s, on
// FakeFlight.

/// The Moon's frame shown.
fn moon_frame<S: FlightSim>(view: &FlightView<FakeCatalog, S>) -> u16 {
    sprites(&drawn(view))[1].0.frame
}

#[test]
fn a_load_shows_every_stellar_from_frame_0() {
    // Whatever the flight held: the view's stellars start at frame 0 as
    // it is read, and go on a frame a tick.
    let opened = || {
        let mut fake = FakeFlight::new();
        fake.set_frame(sol());
        FlightView::over(
            Rc::new(catalog()),
            fake,
            &Rulebook::default(),
            SharedChance::default(),
        )
    };
    let mut flown = opened();
    flown.tick(TICK);
    flown.tick(TICK);
    assert_eq!(moon_frame(&flown), 2);
    let mut loaded = opened();
    assert_eq!(moon_frame(&loaded), 0, "loaded");
    loaded.tick(TICK);
    assert_eq!(moon_frame(&loaded), 1, "loaded, then on");
    // A new pilot's flight is begun, then ticked once past the land hint.
    let mut new = opened();
    new.begin(|_| true);
    new.tick(TICK);
    assert_eq!(moon_frame(&new), 1, "a new pilot's, one tick in");
    new.tick(TICK);
    assert_eq!(moon_frame(&new), 2, "a new pilot's, then on");
}

#[test]
fn a_desk_move_redraws_with_no_extra_call() {
    // Where the desk's move puts the pilot, and that a move within the
    // system clears the course, are the session's.
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, true));
    view.catalog().systems_read.borrow_mut().clear();
    let mut moved = in_system(131, 128, true);
    moved.landed = Some(StellarId(140));
    view.sim.set_frame(moved);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");

    let mut plotted = in_system(130, 128, true);
    plotted.course_shown = vec![SystemId(131)];
    plotted.course_plotted = true;
    let mut view = super::faked_world::world(catalog(), plotted);
    view.catalog().systems_read.borrow_mut().clear();
    assert_eq!(view.course_map().route(), [SystemId(131)]);
    let mut moved = in_system(130, 128, true);
    moved.landed = Some(StellarId(129));
    moved.ships = vec![record::player(128, Vec2::new(300.0, -200.0), 0.0)];
    view.sim.set_frame(moved);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::StellarChanged));
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    assert_eq!(*view.catalog().systems_read.borrow(), [], "the scene kept");
    assert_eq!(view.course_map().route(), [], "the course cleared");
    assert_eq!(
        view.shown_position(),
        Point::new(300.0, -200.0),
        "at the Moon"
    );
}

// Random next frames (stellar-fidelity phase 10).

// Draw order (crons-news-ranks-fidelity phase 14).
