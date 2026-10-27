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
//!
//! The board's names joined it in
//! rdm:phase/view-sim-seam/phase-10-comm-escorts-boarding-on-port: the line
//! a mission's ship says as it is boarded (`mission_board_message`), a
//! grant's outfit names and the plunder's good and outfit names, which
//! [`FlightView::board`] and the plunder dialog's presses word with.

use nova_sim::{Good, MissionBoardLine, OutfitId, Session, SimMessage};

use super::{FlightView, message_text, mission_board_message, names};

/// How the view words what the flight raised (see the module docs): `V`
/// is the view.
pub(super) struct Words<V> {
    /// The words of a message, in the original's, with the stellars it
    /// names named from the scene; `None` for an arrival's lead, which
    /// waits for its arrival to be caught up with.
    pub(super) text: fn(&mut V, &SimMessage) -> Option<String>,
    /// The date as the arrival line and the take-off greeting say it.
    pub(super) date: fn(&V) -> String,
    /// The line a mission's ship says as it is boarded, or as a failed
    /// mission's ship is plundered.
    pub(super) board_line: fn(&V, MissionBoardLine) -> String,
    /// An outfit's lower-case name and plural, as a grant says them;
    /// empty for an outfit with no record.
    pub(super) outfit_names: fn(&V, OutfitId) -> (String, String),
    /// A good's and an outfit's names, as the plunder dialog and its
    /// messages say them; none for one that has no name.
    pub(super) plunder_names: fn(&V, Option<Good>, Option<OutfitId>) -> PlunderNames,
}

/// A good's name and an outfit's, each none when it has no name
/// ([`Words::plunder_names`]).
pub(super) type PlunderNames = (Option<String>, Option<String>);

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
        board_line: Self::session_board_line,
        outfit_names: Self::session_outfit_names,
        plunder_names: Self::session_plunder_names,
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

    /// `line`'s words over the session (`mission_board_message`); none
    /// with no session.
    fn session_board_line(&self, line: MissionBoardLine) -> String {
        self.sim.session.as_ref().map_or_else(
            |_| String::new(),
            |session| mission_board_message(line, session),
        )
    }

    /// `outfit`'s names as the session reads them
    /// ([`Session::outfit_names`]); empty with no session or no record.
    fn session_outfit_names(&self, outfit: OutfitId) -> (String, String) {
        let names = self
            .sim
            .session
            .as_ref()
            .ok()
            .and_then(|session| session.outfit_names(outfit));
        names.map_or_else(Default::default, |(name, plural)| {
            (name.to_owned(), plural.to_owned())
        })
    }

    /// `good`'s and `outfit`'s names as the session reads them (`names`);
    /// none with no session.
    fn session_plunder_names(&self, good: Option<Good>, outfit: Option<OutfitId>) -> PlunderNames {
        let Ok(session) = &self.sim.session else {
            return (None, None);
        };
        let (good, outfit) = names(session, good, outfit);
        (good.map(str::to_owned), outfit.map(str::to_owned))
    }
}
