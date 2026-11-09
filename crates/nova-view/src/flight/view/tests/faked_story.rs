//! The new game's story readouts on [`FakeFlight`]: what the view takes
//! of the flight's room refusals and asks of its news, and hands back as
//! the flight gave them. What the session raises and answers is
//! nova-sim's (`session/storylines.rs`, `session/story_screens.rs`).

use super::faked_shops::docked;
use nova_sim::flight_frame::{FlightEvent, RoomRefusal};

/// A job of 10 tons refused with `room` free.
fn refused(room: i64) -> RoomRefusal {
    RoomRefusal {
        mission: "job".to_owned(),
        title: "Plate".to_owned(),
        cargo: "Plate".to_owned(),
        tons: 10,
        room,
    }
}

#[test]
fn a_room_refusal_the_flight_raised_is_taken_once_in_order() {
    let mut view = docked();
    view.sim.queue_event(FlightEvent::RoomRefused(refused(1)));
    view.sim.queue_event(FlightEvent::RoomRefused(refused(2)));
    assert_eq!(view.take_room_refusals(), [refused(1), refused(2)]);
    assert_eq!(view.take_room_refusals(), []);
}
