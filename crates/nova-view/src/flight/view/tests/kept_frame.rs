//! The frame the view keeps (rdm:phase/view-sim-seam/phase-7-world-frame):
//! filled afresh at the end of each input and tick, after each command and
//! around a catch-up's redraws, so nothing is drawn from a frame older than
//! the flight; and read afresh where concrete code changes the session and
//! then draws the ship where it is (a ship swapped, an arrival, a
//! take-off), so the ship is never drawn on its way from its old place.

use super::*;
use nova_sim::flight_frame::FrameShip;

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
fn use_as_my_ship_draws_the_captured_ship_where_it_is_not_from_the_old_ships_place() {
    let mut view = captured();
    view.tick(TICK / 2);
    assert!(view.alpha() > 0.0, "between steps");
    // Where the old ship was drawn from, a step ago.
    view.previous = nova_sim::flight_frame::ShipPlace {
        position: Vec2::new(-40.0, 30.0),
        heading: 90.0,
    };
    assert_eq!(
        view.frame.player().class,
        ShipId(128),
        "the old ship's frame"
    );
    assert_eq!(view.assign(Assignment::MyShip), Some(Assigned::MyShip));
    let now = *view.session().expect("flying").player();
    assert_eq!(view.previous, now.into(), "not from the old place");
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), at(now.position.x, now.position.y));
    assert_eq!(
        view.frame.player().class,
        ShipId(129),
        "the new ship's frame"
    );
}

#[test]
fn an_arrival_draws_the_ship_where_it_arrives_and_the_new_systems_stellars_as_it_holds_them() {
    let mut view = destructible();
    run_set(&mut view, "Y140");
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    while view.session().expect("flying").system() == SystemId(130) {
        view.tick(TICK);
    }
    let arrived = *view.session().expect("flying").player();
    assert_eq!(view.alpha(), 0.0, "drawn where it arrived");
    assert_eq!(view.previous.position, arrived.position);
    assert_eq!(
        view.shown_position(),
        at(arrived.position.x, arrived.position.y)
    );
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    let record = view.frame.stellar(StellarId(140)).expect("Proxima's");
    assert!(record.destroyed, "the new system's record");
    let scene = view.scene().expect("a scene");
    assert!(scene.stellars().iter().any(|stellar| stellar.destroyed));
}

#[test]
fn a_new_pilots_beginning_draws_the_ship_where_it_begins() {
    let mut view = flight();
    view.begin();
    let begun = *view.session().expect("flying").player();
    assert_eq!(begun.position, Vec2::new(50.0, 50.0), "where begin puts it");
    assert_eq!(view.previous.position, begun.position);
    assert_eq!(view.frame.player().position, begun.position);
}

#[test]
fn a_failed_flight_keeps_the_default_frame_through_its_catch_ups() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(view.frame, FlightFrame::default());
    view.catch_up();
    view.tick(TICK);
    assert_eq!(view.frame, FlightFrame::default());
    assert_eq!(view.frame.player(), FrameShip::default());
}
