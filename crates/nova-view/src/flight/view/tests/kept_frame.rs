//! The frame the view keeps (rdm:phase/view-sim-seam/phase-7-world-frame):
//! filled afresh at the end of each input and tick, after each command and
//! around a catch-up's redraws, so nothing is drawn from a frame older than
//! the flight; and read afresh where concrete code changes the session and
//! then draws the ship where it is (a ship swapped, an arrival, a
//! take-off), so the ship is never drawn on its way from its old place.

use super::faked_landing::departed;
use super::faked_travel::{jump_now, lit};
use super::faked_world::world;
use super::*;
use nova_sim::SessionEvent;
use nova_sim::flight_frame::{FlightEvent, FrameShip, FrameStellar, ShipPlace};

/// The fake's frame with the player's ship at `x`.
fn player_at(x: f32) -> FlightFrame {
    FlightFrame {
        ships: vec![record::player(128, Vec2::new(x, 0.0), 0.0)],
        ..in_sol()
    }
}

#[test]
fn an_input_leaves_the_kept_frame_the_flights_as_it_stands() {
    let mut view = faked(player_at(5.0));
    assert_eq!(view.frame.player().position.x, 5.0, "filled as it opens");
    view.sim.set_frame(player_at(9.0));
    assert_eq!(view.frame.player().position.x, 5.0, "kept between fills");
    view.refresh_frame();
    assert_eq!(view.frame.player().position.x, 9.0);
}

#[test]
fn each_command_sent_fills_the_kept_frame_afresh() {
    let mut view = faked(player_at(5.0));
    view.sim.set_frame(player_at(9.0));
    view.send(FlightCommand::ClearTarget);
    assert_eq!(view.frame.player().position.x, 9.0);
}

#[test]
fn a_frames_steps_fill_the_kept_frame_after_them() {
    let mut view = faked(player_at(5.0));
    view.sim.set_frame(player_at(9.0));
    view.step_frame(1, view.anchor());
    assert_eq!(view.frame.player().position.x, 9.0);
}

#[test]
fn a_catch_up_redraws_from_the_frame_as_the_events_left_it_and_keeps_it_after() {
    let mut view = faked(player_at(5.0));
    view.sim.set_frame(player_at(9.0));
    view.sim
        .queue_event(nova_sim::flight_frame::FlightEvent::Session(
            nova_sim::SessionEvent::StellarChanged,
        ));
    let seen = std::cell::Cell::new(0.0);
    view.catch_up_then(|view, redrawn| {
        assert!(redrawn.stellar);
        seen.set(view.frame.player().position.x);
        // A redraw that changes the flight is drawn from its change too.
        view.sim.set_frame(player_at(12.0));
    });
    assert_eq!(seen.get(), 9.0, "afresh before the redraw");
    assert_eq!(view.frame.player().position.x, 12.0, "and after it");
}

#[test]
fn a_take_off_draws_the_ship_where_it_leaves_the_stellar_not_from_its_place_before_landing() {
    let mut view = faked(docked_at(128, Vec2::new(6.0, -8.0)));
    // Where the ship was drawn from, and kept, before it landed.
    let before = ShipPlace {
        position: Vec2::new(-40.0, 30.0),
        heading: 90.0,
    };
    view.previous = before;
    view.frame.ships[0].position = before.position;
    view.frame.ships[0].heading = before.heading;
    view.alpha = 0.5;
    let flying = FlightFrame {
        landed: None,
        ..docked_at(128, Vec2::new(6.0, -8.0))
    };
    view.sim.queue_answer_showing(departed(128), flying);
    assert_eq!(view.take_off(), Some(StellarId(128)));
    let now = ShipPlace {
        position: Vec2::new(6.0, -8.0),
        heading: 0.0,
    };
    assert_eq!(view.previous, now, "not from the old place");
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), at(6.0, -8.0));
    assert_eq!(
        view.frame.player().position,
        now.position,
        "the kept frame's"
    );
}

#[test]
fn an_arrival_draws_the_ship_where_it_arrives_and_the_new_systems_stellars_as_it_holds_them() {
    let catalog = FakeCatalog {
        lives: vec![life(128, 0), life(129, 0), life(140, 0)],
        ..catalog()
    };
    let mut view = world(catalog, sol());
    jump_now(&mut view);
    lit(&mut view, 1.0);
    view.sim.set_frame(FlightFrame {
        warp_end_due: true,
        ..under_way(true)
    });
    view.refresh_frame();
    let arrived = Vec2::new(-300.0, 200.0);
    let in_alpha = FlightFrame {
        system: SystemId(131),
        ships: vec![the_player_at(arrived, 45.0)],
        stellars: vec![FrameStellar {
            destroyed: true,
            ..record::stellar(StellarId(140))
        }],
        ..sol()
    };
    view.sim.queue_answer_telling(
        Answer::Arrived {
            system: SystemId(131),
            combat: Vec::new(),
        },
        in_alpha,
        vec![FlightEvent::Session(SessionEvent::SystemChanged)],
    );
    view.tick(TICK);
    assert!(sent(&view).contains(&FlightCommand::Arrive), "arrived");
    assert_eq!(view.alpha(), 0.0, "drawn where it arrived");
    assert_eq!(view.previous.position, arrived);
    assert_eq!(view.shown_position(), at(arrived.x, arrived.y));
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    let record = view.frame.stellar(StellarId(140)).expect("Proxima's");
    assert!(record.destroyed, "the new system's record");
    let scene = view.scene().expect("a scene");
    assert!(scene.stellars().iter().any(|stellar| stellar.destroyed));
}

#[test]
fn a_failed_flight_keeps_the_default_frame_through_its_catch_ups() {
    let mut fake = FakeFlight::new();
    fake.fail("no chär to start from");
    let mut view = FlightView::over(
        Rc::new(catalog()),
        fake,
        &Rulebook::default(),
        SharedChance::default(),
    );
    assert_eq!(view.frame, FlightFrame::default());
    view.catch_up();
    view.tick(TICK);
    assert_eq!(view.frame, FlightFrame::default());
    assert_eq!(view.frame.player(), FrameShip::default());
}

#[test]
fn a_status_redraw_after_a_change_behind_the_port_snapshots_the_change() {
    // A change behind the port with no resync after it, as the adapter's
    // own API (a desk edit) makes one.
    let mut view = faked(sol());
    view.redraw_status();
    let before = view.reserves_scene.expect("taken");
    let mut changed = view.frame.clone();
    changed.hud.reserves.fuel = nova_sim::Gauge::full(100.0);
    view.sim.set_frame(changed);
    assert_eq!(view.frame.hud.reserves, before, "the kept frame, stale");
    view.redraw_status();
    let taken = view.reserves_scene.expect("taken");
    assert_eq!(taken.fuel.now, 100.0, "the change");
    assert_eq!(
        view.frame.hud.reserves, taken,
        "the kept frame filled first"
    );
}
