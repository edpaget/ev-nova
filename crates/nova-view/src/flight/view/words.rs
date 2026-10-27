//! [`Words`]: how the view words what the flight raised, handed by the
//! concrete callee to the code over the port.
//!
//! The words of the session's messages read the session's names, rules
//! and date (`message_text`, `Session::long_date_text`), which the port
//! doesn't carry yet: phase 12 (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks)
//! gives the view its own `NameSource`. Until then the code over the port
//! that says the flight's messages (the message line's
//! [`FlightView::say_messages`], the arrival, the take-off, the settling)
//! takes the wording as a callback, split at the concrete callee as phase
//! 4's idiom has it: the concrete blocks hand it [`FlightView::WORDS`],
//! which words with `message_text` over the session at the same point the
//! concrete code once did, and a component test hands its own. Phase 12
//! retires both.

use nova_sim::{Session, SimMessage};

use super::{FlightView, message_text};

/// How the view words what the flight raised (see the module docs): `V`
/// is the view.
pub(super) struct Words<V> {
    /// The words of a message, in the original's, with the stellars it
    /// names named from the scene; `None` for an arrival's lead, which
    /// waits for its arrival to be caught up with.
    pub(super) text: fn(&mut V, &SimMessage) -> Option<String>,
    /// The date as the arrival line and the take-off greeting say it.
    pub(super) date: fn(&V) -> String,
}

// By hand: the derives would ask `V: Clone`.
impl<V> Clone for Words<V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V> Copy for Words<V> {}

impl<C> FlightView<C, crate::flight::SessionFlight<C>> {
    /// The session's words: `message_text` over the session, the stellars
    /// named from the scene and the picks rolled on the flight's chance,
    /// and the session's long date; a session that failed words nothing.
    pub(super) const WORDS: Words<Self> = Words {
        text: Self::session_text,
        date: Self::session_date,
    };

    /// `message`'s words, as the session's messages are worded
    /// (`message_text`); none with no session.
    fn session_text(&mut self, message: &SimMessage) -> Option<String> {
        let Ok(session) = &self.sim.session else {
            return None;
        };
        message_text(message, session, self.scene.as_ref(), &mut self.sim.chance)
    }

    /// The session's long date ([`Session::long_date_text`]); none with
    /// no session.
    fn session_date(&self) -> String {
        self.sim
            .session
            .as_ref()
            .map_or_else(|_| String::new(), Session::long_date_text)
    }
}
