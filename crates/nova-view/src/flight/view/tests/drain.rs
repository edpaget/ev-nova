//! The adapter's drain over a real session: what the session raised
//! comes out as its `FlightEvent`s, once.

use super::*;
use crate::flight::SessionFlight;
use crate::flight::sim_port::FlightSim;
use nova_sim::flight_frame::FlightEvent;

#[test]
fn the_adapter_drains_what_the_session_raised_once() {
    let session = Session::start(&catalog()).expect("starts");
    let mut flight = SessionFlight::<FakeCatalog>::from(session);
    flight
        .session_mut()
        .expect("started")
        .select_stellar(StellarPick::Slot(0));
    let mut into = Vec::new();
    flight.drain(&mut into);
    assert!(
        into.contains(&FlightEvent::Sound(SimSound::NavSelected)),
        "{into:?}"
    );
    let mut again = Vec::new();
    flight.drain(&mut again);
    assert_eq!(again, [], "drained");
}
