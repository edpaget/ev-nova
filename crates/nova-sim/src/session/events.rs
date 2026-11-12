//! What the session tells a view about the scene it draws: the player's
//! ship or where it flies changed ([`SessionEvent`]), drained with
//! [`Session::take_events`].
//!
//! The events carry nothing: a view reads the session's current state
//! when it reacts, so several events of a kind ask for one reaction.
//!
//! - [`SessionEvent::ShipChanged`]: the ship became a class, possibly the
//!   one it was: a purchase, Use As My Ship, or `C`, `E` or `H`.
//! - [`SessionEvent::SystemChanged`]: the system whose stellars are flown
//!   in was entered afresh, another system or the same one rebuilt: a
//!   jump's arrival, a gate's exit, a wormhole that leads nowhere (its
//!   ships set up afresh in the system the player stays in; see the
//!   `wormhole_no_exit` module), a move in flight, a relocation to
//!   another system, or the take-off after a landed move to another
//!   system.
//! - [`SessionEvent::StellarChanged`]: the player was put at another
//!   stellar of the system shown, whose scene is kept: a relocation within
//!   the system.
//! - [`SessionEvent::GalaxyChanged`]: the galaxy overlay changed, through
//!   [`Session::change_galaxy`], an arrival that made a system visible, or
//!   a stellar destroyed or regenerated (`Y`, `U`, the daily step, a new
//!   pilot's): the galaxy map is rebuilt from [`Session::galaxy`], and
//!   the flight view draws each stellar of the system shown by whether it
//!   is destroyed
//!   ([`OverlaidGalaxy::destroyed`](crate::story::OverlaidGalaxy::destroyed)),
//!   with no new layout.
//!   That is how a destroyed stellar's `DeadType` sheet shows from the
//!   next frame (story-runtime phase 7), so every writer of the destroyed
//!   state goes through `Chart::set_stellar_state`, which raises this
//!   event: `Y`, `U`, the daily step, a new pilot and the shots that
//!   shoot a stellar down (story-runtime phase 8).
//! - [`SessionEvent::StellarsRebuilt`]: a rebuild point changed which
//!   stellars shown are dead since the last rebuild
//!   (`Session::dead_since_rebuild`; see the `stellars` module). It is
//!   told only on a change, as `GalaxyChanged` is, after the transition's
//!   own events: the flight view draws each stellar of the scene shown
//!   again, with no new layout (story-runtime phase 9).
//! - [`SessionEvent::StarfieldReset`]: the player entered a system the way
//!   the original's `_RandomizeStars` runs at, a jump's arrival, a
//!   hypergate's or wormhole's exit or the escape pod's rescue, told after
//!   that entry's `SystemChanged`: the flight view scatters its stars
//!   afresh about the ship. An `M` or `N` move, a developer tools' move
//!   and a take-off keep them.
//! - [`SessionEvent::StellarFramesReset`]: a take-off, a hypergate's or
//!   wormhole's exit, or a wormhole that leads nowhere reset every
//!   stellar's current and next frames to 0, as the original's dock
//!   request does (see the `takeoff_frames` module), told after that
//!   entry's other events. The session resets every stellar's frames
//!   itself, between the entry's stellar pass and the rest (see the
//!   `stellar_frames` and `stellar_passes` modules); nothing in the view
//!   acts on it, and the session's tests read it as the mark of each
//!   reset. Only a refused hypergate tells nothing.
//!
//! The session's other one-shot outputs (its sounds, messages, pay notes,
//! fighter notes, stellar passes and the rest, each with its own `take_*`) are drained
//! with these in one stream of `FlightEvent`s
//! ([`Session::drain_events`]; see the `flight_events` module), each
//! event as [`FlightEvent::Session`](crate::flight_frame::FlightEvent::Session).

pub use nova_sim_frame::events::SessionEvent;

use super::{Outbox, Session};

impl Session {
    /// The events since they were last taken, in order; taking them
    /// empties the list.
    pub(crate) fn take_events(&mut self) -> Vec<SessionEvent> {
        self.out.take_events()
    }
}

impl Outbox {
    /// Tells `event` to whoever takes the events next.
    pub(super) fn emit(&mut self, event: SessionEvent) {
        self.events.push(event);
    }

    /// The events since they were last taken, in order; taking them
    /// empties the list.
    fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::catalog;

    #[test]
    fn a_new_session_has_told_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.take_events(), []);
    }

    #[test]
    fn the_events_are_taken_in_order_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.out.emit(SessionEvent::SystemChanged);
        session.out.emit(SessionEvent::ShipChanged);
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged, SessionEvent::ShipChanged]
        );
        assert_eq!(session.take_events(), [], "taken");
    }
}
