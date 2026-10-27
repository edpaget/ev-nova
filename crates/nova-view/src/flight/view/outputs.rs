//! [`FlightView`]'s one-shot outputs taken over the [`FlightSim`] port:
//! the sounds and diagnostics the app takes each frame, and the flags
//! the router asks after, each drained from the flight into the inbox
//! (see the `inbox` module) and taken from it.

use nova_sim::{
    CommNote, FighterNote, Granted, MissionBoardLine, MissionShown, NavMode, PersonQuote,
    SimMessage, TextBar,
};

use crate::flight::escorts::fighters_abandoned_message;
use crate::flight::sim_port::FlightSim;
use crate::{Diagnostic, Sound};

use super::words::Words;
use super::{
    FlightView, MESSAGE_TICKS, Message, SCRIPT_MESSAGE_TICKS, Said, comm_message, pay_said,
};
use nova_sim::flight_frame::FlightCommand;

/// The mission texts taken over flight ([`FlightView::flight_texts`]),
/// with what their redraw needs.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct FlightTexts {
    /// The texts and offer movies, in order.
    pub(super) texts: Vec<MissionShown>,
    /// Whether every text with a bar was staged already, as a person's
    /// offer answered over flight stages them: nothing is redrawn for
    /// them.
    pub(super) staged: bool,
    /// The nav as the last text was queued, when that came after the nav
    /// area was last taken.
    pub(super) noted: Option<NavMode>,
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The mission texts and offer movies to show over flight
    /// ([`FlightEvent::MissionShown`]), whether their bars were all staged
    /// already ([`FlightView::stage_bar`]), and the nav the last noted when
    /// it was queued since the nav area was last taken
    /// ([`FlightFrame::texts_queued`]): what the concrete
    /// [`FlightView::take_mission_texts`] redraws in flight from.
    ///
    /// [`FlightEvent::MissionShown`]: nova_sim::flight_frame::FlightEvent::MissionShown
    /// [`FlightFrame::texts_queued`]: nova_sim::flight_frame::FlightFrame::texts_queued
    pub(super) fn flight_texts(&mut self) -> FlightTexts {
        let queued = self.frame_now().texts_queued;
        let (nav_texts, bars_staged) = (self.nav_texts, self.bars_staged);
        let inbox = self.drain();
        let noted = inbox.text_nav().filter(|_| queued > nav_texts);
        let bars = inbox.text_bars();
        let staged = !bars.is_empty() && bars.iter().all(|bar| bar.texts <= bars_staged);
        FlightTexts {
            texts: inbox.take_mission_texts(),
            staged,
            noted,
        }
    }

    /// The bars of the mission texts not yet taken that are not yet staged
    /// ([`FlightView::stage_bar`]), in order: those queued after the last
    /// staged.
    pub(super) fn unstaged_bars(&mut self) -> Vec<TextBar> {
        let staged = self.bars_staged;
        self.drain()
            .text_bars()
            .into_iter()
            .filter(|bar| bar.texts > staged)
            .collect()
    }

    /// The mission texts and offer movies not yet taken, in order
    /// ([`FlightEvent::MissionShown`]); taking them lets go of their bars.
    ///
    /// [`FlightEvent::MissionShown`]: nova_sim::flight_frame::FlightEvent::MissionShown
    pub(super) fn docked_texts(&mut self) -> Vec<MissionShown> {
        self.drain().take_mission_texts()
    }

    /// Whether the hold after the player's explosion ran out since this
    /// was last asked ([`FlightEvent::HoldExpired`]): the seam where the
    /// router may show its dialogs over the stopped flight before it
    /// begins the fade. True once.
    ///
    /// [`FlightEvent::HoldExpired`]: nova_sim::flight_frame::FlightEvent::HoldExpired
    pub fn take_hold_expired(&mut self) -> bool {
        self.drain().take_hold_expired()
    }

    /// The escape pod's rescue's `dësc`, once, after the rescue
    /// ([`FlightEvent::RescueDesc`]): the router shows it over flight.
    ///
    /// [`FlightEvent::RescueDesc`]: nova_sim::flight_frame::FlightEvent::RescueDesc
    pub fn take_rescue_desc(&mut self) -> Option<i16> {
        self.drain().take_rescue_desc()
    }

    /// Whether the pilot should be saved ([`FlightEvent::SaveDue`]);
    /// taking it clears it.
    ///
    /// [`FlightEvent::SaveDue`]: nova_sim::flight_frame::FlightEvent::SaveDue
    pub fn take_save_due(&mut self) -> bool {
        self.drain().take_save_due()
    }

    /// Shows `text` as the message, from now, for `ticks` of flight,
    /// replacing whatever was shown, as `_DisplayComm` does
    /// (@0x9024b-0x90256), and sets the flight's message line to it
    /// ([`FlightCommand::CommShown`], `nova_sim::Session::comm_shown`): a
    /// line the view raises itself, or one it composes from the flight's
    /// at an arrival, a take-off or a settling, where nothing counts down
    /// between the flight raising the parts and the view showing the
    /// whole. The lines the flight raised in a frame's steps are relayed
    /// ([`FlightView::relay`]).
    ///
    /// Every write to the message slot goes through this, `relay`,
    /// [`FlightView::clear_message`], [`FlightView::acknowledge_message`]
    /// or [`FlightView::blank_message`], so the flight's line stays in
    /// step with the slot.
    pub(super) fn say_for(&mut self, text: impl Into<String>, ticks: u16) {
        self.relay(text, ticks);
        self.send(FlightCommand::CommShown(ticks));
    }

    /// Clears the message line when it holds `text`, as the original
    /// clears one message by name (`_HandlePlayer` @0x6b231-0x6b2a7,
    /// @0x6dedb-0x6df29): it shows an empty line for 1 tick, the
    /// flight's line too.
    pub(super) fn clear_message(&mut self, text: &str) {
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.text == text)
        {
            self.message = None;
            self.send(FlightCommand::CommShown(1));
        }
    }

    /// Empties the message slot at a change the flight settles its line
    /// for itself: a landing, which sets it to 0
    /// (`_HandlePlayerDockRequest` @0x66d7f); the take-off after it, from
    /// that 0; and an arrival, whose own line the flight raised and the
    /// view shows next ([`FlightView::say_for`]).
    pub(super) fn blank_message(&mut self) {
        self.message = None;
    }

    /// Shows the flight's messages since they were last taken, each in
    /// turn, in the words `words` gives; an arrival's waits for its
    /// arrival to be caught up with (`FlightView::arrived`). Called right
    /// after a press or a settling raised them, with nothing counted down
    /// since, it sets the flight's line to the last shown
    /// ([`FlightView::say_for`]); after a frame's steps,
    /// [`FlightView::relay_messages`] leaves the line alone.
    pub(super) fn say_messages(&mut self, words: Words<Self>) {
        for (text, ticks) in self.worded_messages(words) {
            self.say_for(text, ticks);
        }
    }

    /// Shows the flight's messages raised in a frame's steps, as
    /// [`FlightView::say_messages`] does, but relayed
    /// ([`FlightView::relay`]): the flight set its line as it raised
    /// each, and the steps have counted it down since.
    pub(super) fn relay_messages(&mut self, words: Words<Self>) {
        for (text, ticks) in self.worded_messages(words) {
            self.relay(text, ticks);
        }
    }

    /// The flight's messages since they were last taken
    /// ([`FlightEvent::Message`]), worded.
    ///
    /// [`FlightEvent::Message`]: nova_sim::flight_frame::FlightEvent::Message
    fn worded_messages(&mut self, words: Words<Self>) -> Said {
        let messages = self.drain().take_messages();
        self.word(messages, words)
    }

    /// The words `words` gives each of `messages`, in order, with its
    /// ticks; an arrival's has none here, and waits for its arrival to be
    /// caught up with (`FlightView::arrived`), which leads with it.
    pub(super) fn word(&mut self, messages: Vec<SimMessage>, words: Words<Self>) -> Said {
        let mut said = Vec::new();
        for message in messages {
            match (words.text)(self, &message) {
                Some(text) => said.push((text, message.ticks())),
                None => self.pending_leads.push(message),
            }
        }
        said
    }

    /// Shows `text`, a line the session raised and has already set its
    /// message line to, for `ticks`, leaving the session's line alone: a
    /// relay comes after the frame's steps have counted it down, and in a
    /// fixed order (the comm notes, the quotes, the messages), so setting
    /// it again would undo the session's count.
    pub(super) fn relay(&mut self, text: impl Into<String>, ticks: u16) {
        self.message = Some(Message {
            text: text.into(),
            said_at: crate::time::ticks(self.elapsed),
            ticks,
        });
    }

    /// Relays the comm notes, then the quotes, that a frame's steps took
    /// ([`FlightView::relay`]): the session set its line as it raised
    /// each, and the steps have counted it down since, so they are
    /// relayed, not set again; the last stays.
    pub(super) fn relay_comm_and_quotes(&mut self, notes: &[CommNote], quotes: Vec<PersonQuote>) {
        for note in notes {
            self.relay(comm_message(note), note.ticks());
        }
        for quote in quotes {
            let ticks = quote.ticks();
            self.relay(quote.text, ticks);
        }
    }

    /// The comm line a failed mission's ship said as the last board
    /// plundered it ([`FlightEvent::BoardLine`]), once: what
    /// [`FlightView::board`] says first, worded with the session's names.
    ///
    /// [`FlightEvent::BoardLine`]: nova_sim::flight_frame::FlightEvent::BoardLine
    pub(super) fn board_line_drained(&mut self) -> Option<MissionBoardLine> {
        self.drain().take_board_line()
    }

    /// What the last boarding granted ([`FlightEvent::Grant`]), once:
    /// what [`FlightView::board`] says as the plunder opens, worded with
    /// the outfit's names.
    ///
    /// [`FlightEvent::Grant`]: nova_sim::flight_frame::FlightEvent::Grant
    pub(super) fn grant_drained(&mut self) -> Option<Granted> {
        self.drain().take_grant()
    }

    /// How many fighters the player abandoned leaving systems, said for
    /// [`MESSAGE_TICKS`] each ([`FlightEvent::FighterNote`]).
    ///
    /// [`FlightEvent::FighterNote`]: nova_sim::flight_frame::FlightEvent::FighterNote
    pub(super) fn fighters_said(&mut self) -> Said {
        self.drain()
            .take_fighter_notes()
            .into_iter()
            .map(|FighterNote::Abandoned(count)| (fighters_abandoned_message(count), MESSAGE_TICKS))
            .collect()
    }

    /// What an arrival says after its line: the fighters abandoned
    /// ([`FlightView::fighters_said`]) and the escorts that defected unpaid
    /// ([`pay_said`]), and the flight's messages, which the arrival words
    /// ([`FlightEvent::PayNote`], [`FlightEvent::Message`]).
    ///
    /// [`FlightEvent::PayNote`]: nova_sim::flight_frame::FlightEvent::PayNote
    /// [`FlightEvent::Message`]: nova_sim::flight_frame::FlightEvent::Message
    pub(super) fn arrival_outputs(&mut self) -> (Said, Said, Vec<SimMessage>) {
        let fighters = self.fighters_said();
        let inbox = self.drain();
        let pay = pay_said(&inbox.take_pay_notes());
        (fighters, pay, inbox.take_messages())
    }

    /// The message the last take-off left on, a held `Q`'s, said for
    /// [`SCRIPT_MESSAGE_TICKS`] ([`FlightEvent::LeaveMessage`]); none when
    /// it left on none.
    ///
    /// [`FlightEvent::LeaveMessage`]: nova_sim::flight_frame::FlightEvent::LeaveMessage
    pub(super) fn leave_said(&mut self) -> Said {
        self.drain()
            .take_leave_message()
            .into_iter()
            .map(|text| (text, SCRIPT_MESSAGE_TICKS))
            .collect()
    }

    /// What a take-off says and shows: the escorts that defected for want
    /// of its pay ([`pay_said`]), the flight's messages, which the
    /// take-off words, and the nav as the take-off began
    /// ([`FlightEvent::TakeoffNav`]).
    ///
    /// [`FlightEvent::TakeoffNav`]: nova_sim::flight_frame::FlightEvent::TakeoffNav
    pub(super) fn takeoff_outputs(&mut self) -> (Said, Vec<SimMessage>, Option<NavMode>) {
        let inbox = self.drain();
        let pay = pay_said(&inbox.take_pay_notes());
        (pay, inbox.take_messages(), inbox.take_takeoff_nav())
    }

    /// The flight's sounds (thrust, landing, taking off and jumping),
    /// then the interface's (the escort menu's, the floating map's zooms,
    /// the player info refused, the course map's beeps), then the fight's:
    /// what [`Screen::take_sounds`](crate::Screen::take_sounds) gives.
    pub(super) fn drained_sounds(&mut self) -> Vec<Sound> {
        let mut sounds: Vec<Sound> = self
            .drain()
            .take_sounds()
            .into_iter()
            .map(Sound::Sim)
            .collect();
        sounds.extend(self.ui_sounds.drain(..).map(Sound::Ui));
        sounds.extend(self.effects.take_sounds().into_iter().map(Sound::Combat));
        sounds
    }

    /// What could not be read of the looks, then the flight's
    /// diagnostics, each once: what
    /// [`Screen::take_diagnostics`](crate::Screen::take_diagnostics) gives.
    pub(super) fn drained_diagnostics(&mut self) -> Vec<Diagnostic> {
        let mut diagnostics = std::mem::take(&mut self.unread_looks);
        let sim = self.drain().take_diagnostics().into_iter();
        diagnostics.extend(sim.map(Diagnostic::Sim));
        diagnostics
    }
}
