//! [`Session::fill_frame`]: what the flight view draws, filled into a
//! [`FlightFrame`] for the view's `FlightSim` port
//! (see [`flight_frame`](crate::flight_frame)).

use super::Session;
use crate::flight_frame::FlightFrame;

impl Session {
    /// Fills `into` with what the flight view draws now. Every field is
    /// written, so a frame kept from an earlier fill, or from another
    /// session, shows nothing stale: the destructuring names every field,
    /// so a field added to [`FlightFrame`] doesn't compile until it is
    /// filled here.
    pub fn fill_frame(&self, into: &mut FlightFrame) {
        let FlightFrame { nav_mode } = into;
        *nav_mode = self.nav_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::navigation::NavMode;
    use crate::session::StellarPick;
    use crate::session::tests::three_stellars;

    #[test]
    fn a_fill_writes_the_sessions_nav_mode_over_a_stale_one() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        let mut frame = FlightFrame {
            nav_mode: NavMode::Hyperspace(Some(SystemId(999))),
        };
        session.fill_frame(&mut frame);
        assert_eq!(frame.nav_mode, NavMode::Stellar(Some(StellarId(131))));
        assert_eq!(frame.nav_mode, session.nav_mode());
    }
}
