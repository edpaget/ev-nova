//! What the session fills and the view takes: the sounds, messages,
//! events and notes raised since they were last taken, the line raised
//! last ([`Outbox::raise`]), and the flags the view asks after, each taken with its `take_*` method, and all of them
//! drained as one stream of `FlightEvent`s by
//! [`Session::drain_events`](super::Session::drain_events)
//! (see the `flight_events` module). Nothing in the session reads them
//! back, except to undo its own writes.

use crate::bay::FighterNote;
use crate::board::MissionBoardLine;
use crate::combat::report::Reports;
use crate::control::ScriptNote;
use crate::flight_frame::RaisedLine;
use crate::grant::Granted;
use crate::hail::CommNote;
use crate::hire::PayNote;
use crate::message::SimMessage;
use crate::navigation::NavMode;
use crate::sound::SimSound;

use super::{InterfaceLoad, MissionShown, PersonQuote, SessionEvent, TextBar};

/// What the session fills for the view to take, in `Session`'s former
/// field order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Outbox {
    /// The sounds emitted since they were last taken.
    pub(super) sounds: Vec<SimSound>,
    /// The messages raised since they were last taken.
    pub(super) messages: Vec<SimMessage>,
    /// The events told since they were last taken.
    pub(super) events: Vec<SessionEvent>,
    /// Whether the pilot has changed in a way that should be saved since
    /// this was last taken.
    pub(super) save_due: bool,
    /// Whether the hold ran out on a tick since
    /// [`Session::take_hold_expired`](super::Session::take_hold_expired) was last asked.
    pub(super) hold_expiry_due: bool,
    /// What the session reports outside combat.
    pub(super) reports: Reports,
    /// What the last boarding granted, until it is taken.
    pub(super) granted: Option<Granted>,
    /// The comm line a failed mission's ship said as the last board
    /// plundered it, until it is taken (see the `board_attempt` module).
    pub(super) board_line: Option<MissionBoardLine>,
    /// What the NPCs assisting the player have done since this was last
    /// taken.
    pub(super) comm: Vec<CommNote>,
    /// What a test raises as double time's second pass would, said onto
    /// [`Outbox::comm`] as that pass ends (see the `step` module).
    #[cfg(test)]
    pub(super) second_pass_comm: Vec<CommNote>,
    /// What the player's fighters met as it left systems since this was
    /// last taken.
    pub(super) fighter_notes: Vec<FighterNote>,
    /// What paying the escorts did since this was last taken.
    pub(super) pay_notes: Vec<PayNote>,
    /// What running set expressions had to tell since this was last taken.
    pub(super) script_notes: Vec<ScriptNote>,
    /// The hail quotes said since they were last taken.
    pub(super) quotes: Vec<PersonQuote>,
    /// The mission texts to show, until they are taken (see the
    /// `mission_screens` module); never saved.
    pub(super) shown_texts: Vec<MissionShown>,
    /// How many mission texts have been queued, never reset (see the
    /// `mission_screens` module); never saved.
    pub(super) texts_queued: u64,
    /// The status bar as each text queued since they were last taken was
    /// queued: what its caller's redraw shows as it closes (see the
    /// `mission_screens` module's [`TextBar`]); never saved.
    pub(super) text_bars: Vec<TextBar>,
    /// The `C`, `E` and `H` run while docked since the last outfit order
    /// or ship purchase began, in order (see the `ship_change` module's
    /// [`InterfaceLoad`]); never saved.
    pub(super) interface_loads: Vec<InterfaceLoad>,
    /// Whether a run of docked scripts began since the last drain, letting
    /// go of the loads before it (see the `flight_events` module's
    /// [`Outbox::begin_docked_run`]); never saved.
    pub(super) interface_loads_cleared: bool,
    /// The nav mode as the last take-off began, before its deselect: what
    /// the original's take-off redraw shows (`_PlayerLandOnStellar`
    /// @0x633a0), until taken
    /// ([`Session::take_takeoff_nav`](super::Session::take_takeoff_nav));
    /// never saved.
    pub(super) takeoff_nav: Option<NavMode>,
    /// The line raised last, what `_DisplayComm`'s one slot holds
    /// (@0x90251), until a step takes it (see the `step` module); never
    /// saved.
    pub(super) last_line: Option<RaisedLine>,
}

impl Outbox {
    /// Raises `line`: puts it on its kind's list (the messages, the comm
    /// notes or the quotes) and records it as the line raised last, as
    /// `_DisplayComm` replaces its slot with each line (@0x90251-0x90265).
    pub(super) fn raise(&mut self, line: RaisedLine) {
        match &line {
            RaisedLine::Message(message) => self.messages.push(message.clone()),
            RaisedLine::Comm(note) => self.comm.push(note.clone()),
            RaisedLine::Quote(quote) => self.quotes.push(quote.clone()),
        }
        self.last_line = Some(line);
    }
}
