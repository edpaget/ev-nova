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
//!   jump's arrival, a gate's exit, a move in flight, a relocation to
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
//!   state goes through `Session::set_stellar_state`, which raises this
//!   event: `Y`, `U`, the daily step, a new pilot and the shots that
//!   shoot a stellar down (story-runtime phase 8).
//! - [`SessionEvent::StellarsRebuilt`]: a rebuild point changed which
//!   stellars shown are dead since the last rebuild
//!   ([`Session::dead_since_rebuild`]; see the `stellars` module). It is
//!   told only on a change, as `GalaxyChanged` is, after the transition's
//!   own events: the flight view draws each stellar of the scene shown
//!   again, with no new layout (story-runtime phase 9).
//!
//! The session's other outboxes (its sounds, messages, script notes, pay
//! notes, fighter notes and the rest, each with its own `take_*`) are not
//! folded into this one; doing so is a possible follow-up.

use super::Session;

/// A change to what a view draws of the session (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    /// The player's ship became a class, possibly the one it was.
    ShipChanged,
    /// The system whose stellars are flown in was entered afresh.
    SystemChanged,
    /// The player was put at another stellar of the system shown.
    StellarChanged,
    /// The galaxy overlay changed.
    GalaxyChanged,
    /// A rebuild point changed which stellars shown are dead since the
    /// last rebuild.
    StellarsRebuilt,
}

impl Session {
    /// Tells `event` to whoever takes the events next.
    pub(super) fn emit(&mut self, event: SessionEvent) {
        self.out.events.push(event);
    }

    /// The events since they were last taken, in order; taking them
    /// empties the list.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.out.events)
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
        session.emit(SessionEvent::SystemChanged);
        session.emit(SessionEvent::ShipChanged);
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged, SessionEvent::ShipChanged]
        );
        assert_eq!(session.take_events(), [], "taken");
    }
}
