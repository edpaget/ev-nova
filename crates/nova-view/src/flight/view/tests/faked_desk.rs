//! The developer tools' desk and the view's catch-up after an edit, on
//! `FakeFlight`, converted from pilot.rs (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge).
//! The edits themselves are the session's (`nova_sim::session::edit`'s
//! tests) and the adapter's forwards `sim_adapter`'s; a desk edit on a
//! real flight, then a resync, is `tests/it/flight_seam.rs`'s.

use super::*;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{DeskReading, GameDate, Reserves, SessionEvent};

/// Ada, docked on Earth in Sol.
fn ada() -> DeskReading {
    DeskReading {
        name: "Ada".to_owned(),
        credits: 1000,
        reserves: Reserves::default(),
        date: GameDate::new(1177, 6, 23).expect("a date"),
        date_text: "June 23, 1177 NC".to_owned(),
        system: SystemId(130),
        landed: Some(StellarId(128)),
        bits: Vec::new(),
    }
}

#[test]
fn the_pilot_desk_is_the_flights_desk_named_from_the_course_maps_galaxy() {
    let mut view = faked(in_sol());
    view.sim.desk_mut().reading = Some(ada());
    let mut desk = view.pilot_desk().expect("flying");
    let sheet = desk.sheet().expect("a pilot");
    assert_eq!(sheet.name, "Ada");
    assert_eq!(sheet.system.name, "Sol");
    assert_eq!(desk.systems().len(), 3);
    assert_eq!(desk.edit(PilotEdit::Credits(5)), Ok(()));
    assert_eq!(view.sim.edits(), [PilotEdit::Credits(5)]);
}

#[test]
fn a_flight_that_failed_has_no_pilot_desk() {
    let mut fake = FakeFlight::new();
    fake.fail("no chär");
    let opening = fake.opening();
    let mut view = FlightView::over(
        Rc::new(catalog()),
        fake,
        opening,
        &Rulebook::default(),
        SharedChance::default(),
    );
    assert!(view.pilot_desk().is_none());
    view.resync();
    assert!(view.scene().is_none(), "nothing to lay out");
}

#[test]
fn after_a_move_resync_lays_out_the_new_system() {
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, true));
    view.catalog().systems_read.borrow_mut().clear();
    view.sim.set_frame(in_system(131, 128, true));
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.resync();
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
    assert_eq!(view.message(), None, "no arrival message");
    view.resync();
    assert_eq!(
        *view.catalog().systems_read.borrow(),
        [SystemId(131)],
        "nothing moved, nothing read"
    );
}

#[test]
fn after_a_move_within_the_system_resync_draws_the_ship_there_and_reads_nothing() {
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, true));
    view.catalog().systems_read.borrow_mut().clear();
    let mut moved = in_system(130, 128, true);
    moved.ships = vec![record::player(128, Vec2::new(300.0, -200.0), 0.0)];
    view.sim.set_frame(moved);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::StellarChanged));
    view.resync();
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    assert_eq!(*view.catalog().systems_read.borrow(), []);
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), Point::new(300.0, -200.0));
}

#[test]
fn a_resync_with_nothing_moved_reads_nothing() {
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, true));
    view.catalog().systems_read.borrow_mut().clear();
    view.resync();
    assert_eq!(*view.catalog().systems_read.borrow(), []);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
}

// The router's reads of the flight, after an edit and on the main screen.

#[test]
fn the_flights_failure_is_the_views_to_tell_as_it_opened() {
    let view = faked(in_sol());
    assert_eq!(view.failure(), None, "started");
    let mut fake = FakeFlight::new();
    fake.fail("no chär");
    let opening = fake.opening();
    let view = FlightView::over(
        Rc::new(catalog()),
        fake,
        opening,
        &Rulebook::default(),
        SharedChance::default(),
    );
    assert_eq!(view.failure(), Some("no chär"));
}

#[test]
fn the_stellar_landed_on_is_read_from_the_frame_a_change_left() {
    let mut view = faked(in_sol());
    assert_eq!(view.landed(), None, "in flight");
    view.change_sim(|sim| sim.set_frame(in_system(130, 128, true)));
    assert_eq!(view.landed(), Some(StellarId(128)), "the flight's now");
}

#[test]
fn the_plain_text_and_the_hire_are_the_flights() {
    let mut view = faked(in_sol());
    let fills = view.sim.fills();
    view.sim
        .queue_screen(ScreenAnswer::PlainText("Hi Ada.".to_owned()));
    assert_eq!(view.plain_text("Hi <PN>."), "Hi Ada.");
    assert_eq!(
        view.sim.screens_asked(),
        [ScreenAsk::PlainText("Hi <PN>.".to_owned())],
        "asked"
    );
    assert_eq!(view.sim.fills(), fills, "a screen's ask: no refill");
    assert!(!view.can_hire_escorts());
    view.sim.port_mut().can_hire_escorts = true;
    assert!(view.can_hire_escorts());
}

#[test]
fn a_plain_text_the_flight_answers_nothing_to_comes_back_unchanged() {
    let mut view = faked(in_sol());
    assert_eq!(
        view.plain_text("Hi <PN>."),
        "Hi <PN>.",
        "nothing queued: nothing answered"
    );
    view.sim.fail("no chär");
    assert_eq!(view.plain_text("Hi <PN>."), "Hi <PN>.", "a failed flight's");
    assert_eq!(
        view.sim.screens_asked(),
        [
            ScreenAsk::PlainText("Hi <PN>.".to_owned()),
            ScreenAsk::PlainText("Hi <PN>.".to_owned()),
        ],
        "each asked, the failed flight's too"
    );
}
