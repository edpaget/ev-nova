//! The flight's message line as the view keeps it in step, on
//! [`FakeFlight`]: Return clears the view's line and acknowledges the
//! flight's ([`FlightCommand::CommAcknowledged`]). The lines the view
//! raises and clears by name set the flight's line
//! ([`FlightCommand::CommShown`], `tests/faked_travel.rs`), and the
//! session's own lines relayed after a frame's steps leave it alone
//! (`tests/stepping.rs`); how the flight's line keeps its calls off is
//! nova-sim's (`session::comm_calls`).

use super::faked_travel::shown_to_flight;
use super::*;

#[test]
fn return_clears_the_line_and_acknowledges_the_flights() {
    let mut view = faked(in_sol());
    view.say_for(NO_ESCORTS, ESCORT_TICKS);
    view.acknowledge_message();
    assert_eq!(view.message(), None, "cleared");
    assert_eq!(
        sent(&view),
        [
            FlightCommand::CommShown(ESCORT_TICKS),
            FlightCommand::CommAcknowledged
        ]
    );
    view.acknowledge_message();
    assert_eq!(
        sent(&view).last(),
        Some(&FlightCommand::CommAcknowledged),
        "with nothing shown too: the flight's line decides"
    );
    assert_eq!(shown_to_flight(&view), [ESCORT_TICKS], "nothing set");
}
