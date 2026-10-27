//! [`FlightView`]'s take-off and settling, over the [`FlightSim`] port:
//! the port's Leave and a held `Q`'s ([`FlightCommand::TakeOff`],
//! [`FlightCommand::LeaveIfDue`]), the escorts' landing
//! ([`FlightCommand::LandEscorts`]), the settling
//! ([`FlightCommand::SettleScript`]) and what each shows, the leave read
//! from the frame ([`FlightFrame::leave_due`]).
//!
//! The words of the flight's messages ride a [`Words`] callback the
//! concrete callee hands in (see the `words` module).
//!
//! [`FlightFrame::leave_due`]: nova_sim::flight_frame::FlightFrame::leave_due

use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::{Departure, Settled, SimMessage, StellarId};

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::words::Words;
use super::{FlightView, NavFlash, SCRIPT_MESSAGE_TICKS, Said, Then};

impl<C: SystemCatalog + ShipSprites + CombatLooks + StatusBars, S: FlightSim> FlightView<C, S> {
    /// Takes off from the stellar landed on ([`FlightCommand::TakeOff`]),
    /// and gives it; `None` when the ship has not landed. The flight leaves
    /// the port as its Leave does (`nova_sim::Session::leave_port`),
    /// settling the take-off's events at once, its messages worded in
    /// `words`.
    /// The next frame draws the ship where it is, at
    /// the stellar, not on its way from where it was, in the system a move
    /// made while landed went to, laid out afresh as the session tells
    /// ([`SessionEvent`](nova_sim::SessionEvent)); it shows no message
    /// from before the landing, but greets the pilot past the tutorial
    /// ([`take_off_greeting`](super::take_off_greeting)), or shows a held `Q`'s message, for
    /// [`SCRIPT_MESSAGE_TICKS`], in place of the greeting
    /// (`_PlayerLandOnStellar` @0x63588-0x635e3), followed by how many
    /// hired escorts defected
    /// for want of the take-off's pay ([`defection_message`](super::defection_message)); the
    /// session has populated the system's traffic afresh
    /// (`nova_sim::Session::take_off`), so no frame draws the ships from before
    /// the landing. Leaving passes the stay's days, their
    /// events rolled on the flight's chance, and what settling them did
    /// shows as [`FlightView::settle_script`] shows it.
    pub(super) fn take_off_with(&mut self, words: Words<Self>) -> Option<StellarId> {
        match self.send(FlightCommand::TakeOff) {
            Answer::Departed(departure) => Some(self.departed(departure, words)),
            _ => None,
        }
    }

    /// Takes off for a `Q` run while landed, if its leave is due
    /// ([`FlightCommand::LeaveIfDue`], `nova_sim::Session::leave_if_due`), as
    /// [`FlightView::take_off_with`] does for
    /// the port's Leave, and gives the stellar left; `None`, and nothing
    /// changes, when no leave is due. The router calls it once nothing
    /// shown over the spaceport holds the leave.
    pub(super) fn leave_if_due_with(&mut self, words: Words<Self>) -> Option<StellarId> {
        match self.send(FlightCommand::LeaveIfDue) {
            Answer::Departed(departure) => Some(self.departed(departure, words)),
            _ => None,
        }
    }

    /// Settles the escorts' marks as the ship leaves the stellar it is
    /// landed at ([`FlightCommand::LandEscorts`], `nova_sim::Session::land_escorts`),
    /// drawing on the flight's
    /// chance, and gives the report to show over the spaceport; none when
    /// nothing was sold or upgraded. The router calls it before either
    /// leave.
    pub fn land_escorts(&mut self) -> Option<String> {
        match self.send(FlightCommand::LandEscorts) {
            Answer::EscortReport(report) => report,
            _ => None,
        }
    }

    /// Catches the screen up with `departure`: the take-off's line, then
    /// what settling it did; gives the stellar left.
    fn departed(&mut self, departure: Departure, words: Words<Self>) -> StellarId {
        let Departure { stellar, settled } = departure;
        let mut said = self.leave_said();
        let (line, rest, system) = self.took_off(words);
        said.extend(line);
        self.say_after(&said, rest);
        self.show_settled(settled, system, words);
        stellar
    }

    /// Whether a `Q` run while landed waits for the take-off
    /// ([`FlightFrame::leave_due`](nova_sim::flight_frame::FlightFrame::leave_due)):
    /// whoever shows the spaceport leaves it
    /// ([`FlightView::leave_if_due`]), as its Leave does, once nothing
    /// shown over it holds the leave.
    #[must_use]
    pub fn leave_due(&self) -> bool {
        self.frame_now().leave_due
    }

    /// Catches the screen up with the session having taken off: the ship
    /// drawn where it is, in the system laid out afresh if the session
    /// tells a move while landed changed it, the traffic populated afresh
    /// drawn where it is with its ship types' sheets read, no message from
    /// before kept. Gives the line to say, each part with its ticks: the
    /// take-off greeting past the tutorial ([`SimMessage::TookOff`]), if
    /// any, then what paying the escorts for the take-off has to say; and
    /// the session's other messages, each said after the line and
    /// replacing it, as they come in later frames in the original.
    ///
    /// The original keeps the slot and replaces it with its greeting
    /// (`_PlayerLandOnStellar` @0x63732), which a newbie, or a `Q`'s
    /// message, goes without; a message from before the landing would be
    /// further from what the player sees than none.
    ///
    /// The whole bar is redrawn once the take-off has let go of the nav
    /// selection. Under [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s engine reading, the nav
    /// area then shows the nav as the take-off began
    /// ([`FlightEvent::TakeoffNav`](nova_sim::flight_frame::FlightEvent::TakeoffNav)) in the first frame drawn after it
    /// and to the end of that frame's tick ([`NavFlash`],
    /// [`FlightView::advance_hud`]), when that differs from the nav now,
    /// as after a landed `M`: built by operator decision on 2026-10-15.
    /// The flash isn't timed from the take-off's own tick: the take-off
    /// comes between frames, and the next frame's tick may already be
    /// past it.
    ///
    /// The original redraws the whole bar after the port closes
    /// (`_PlayerLandOnStellar` @0x63399-0x633a0, `_RedrawStatus(1)`) with
    /// +0x6c still the landed selection, and flushes it at once
    /// (`QDFlushPortBuffer` @0x4cd4d-0x4cd71, unless
    /// `_useLegacyRenderingPipeline`). The deselect (@0x674d2) and the
    /// redraws after it (`_RefreshSAT` @0x6765d, `_RedrawStatus(1)`
    /// @0x67662-0x67669) come inside the same `_HandlePlayerDockRequest`
    /// call (@0x66f29 → @0x67669). No game frame passes between the two
    /// redraws: the flash lasts from one flush to the next, at least one
    /// display refresh, under one of ours' 30 Hz ticks. Ours holds it for
    /// at least one presented frame of ours, up to 1/30 s past that
    /// frame's start.
    ///
    /// Every take-off reaches it, not only a landed `M`'s. While docked
    /// the original holds the selection the ship landed with: the land
    /// key puts mode 2 over any mode, a course's mode-3 cue too, with the
    /// nearest stellar (`_HandlePlayer` @0x6a8ed-0x6a932); the dock
    /// request lands on +0x6c (`_StellarIsLandable` @0x66968-0x66977);
    /// and `_PlayerLandOnStellar` passes +0x6c on as the stellar docked
    /// at, to `_DoAutoRefuel` (@0x632b2-0x632c5), `_DoPortDialog`
    /// (@0x6336e-0x63381) and `_SavePilotFile` (@0x6353e-0x6354b). Nothing
    /// between the landing and @0x633a0 writes the player's +0x6a or
    /// +0x6c but a landed `M` or `N` (`_EvalSetExp` @0x15705, @0x157e3,
    /// @0x158fd): the port dialogs put back what they saved
    /// (`_DoPortDialog` @0x5fcf0-0x5fd1b, @0x60084-0x60096, and the
    /// `nav` module's restores), and `_PlayerLandOnStellar`'s and
    /// `_HandlePlayerDockRequest`'s other writes at +0x6c (@0x63520,
    /// @0x672ac) are the auxiliary ships'. So the take-off's redraw names
    /// the landed stellar in mode 2 (`_DrawStatusNav` @0x4a09e-0x4a18c),
    /// or a landed `M`'s (the seven stock finales' `M472 Q25048` name
    /// "Our Spiel"), until the deselect's redraw shows "No Destination",
    /// or the course cued again in mode 3 (`_CueNextHyperRouteDest`
    /// @0xe33c-0xe344, from @0x674d8). Only a take-off whose held
    /// selection is let go already, after an `N` or an `M` to a system
    /// with no stellar, shows no flash.
    ///
    /// Only a set expression run in that window could hold the bar
    /// longer, by starting a mission whose text shows (`S` →
    /// `_DoMissionAccept` @0x15407, its texts @0xa1bb2, @0xa1f0e,
    /// @0xa1fb5): a `crön`'s `OnStart` or `OnEnd` (@0xb58b), a `spöb`'s
    /// `OnRegen` (@0xb6ee), a `nëbu`'s `OnExplore` (@0x634da), or an
    /// auto-aborting mission's `OnAbort` (@0x4317e → @0x99c3e). No stock
    /// one does. Ours runs the stay's days inside the take-off, so a text
    /// they queue is newer than the take-off's held nav: the count of
    /// texts noted at the port's last nav redraw is kept, and
    /// [`FlightView::take_mission_texts`] draws that text's nav, noted
    /// before the deselect, while the flash lasts.
    ///
    /// It gives, last, whether the system was laid out afresh.
    fn took_off(&mut self, words: Words<Self>) -> (Said, Said, bool) {
        let (pay, messages, held) = self.takeoff_outputs();
        let nav_texts = self.nav_texts;
        let system = self.catch_up().system;
        self.place_ship();
        self.read_npc_sheets();
        self.blank_message();
        self.redraw_status();
        if let Some(held) = held
            && self.hud_waits()
            && held != self.nav_mode()
        {
            self.nav_scene = Some((held, self.nav_display(held)));
            self.nav_flash = Some(NavFlash::Held);
            self.nav_texts = nav_texts;
        }
        let (greeting, rest): (Vec<_>, Vec<_>) = messages
            .into_iter()
            .partition(|message| matches!(message, SimMessage::TookOff { .. }));
        let mut line = self.word(greeting, words);
        line.extend(pay);
        (line, self.word(rest, words), system)
    }

    /// Settles what the set expressions run since queued, as
    /// `nova_sim::Session::settle_script` does ([`FlightCommand::SettleScript`]), reading the catalog and drawing on
    /// the flight's chance. A `Q` run while landed never takes off here:
    /// it is held ([`FlightView::leave_due`]) for the router to leave the
    /// spaceport once nothing shown over it holds the leave
    /// ([`FlightView::leave_if_due`]), which shows its message. The screen
    /// catches up with what the flight told ([`SessionEvent`](nova_sim::SessionEvent)): a
    /// move in flight lays out the system the ship is in afresh, even the
    /// same one, as the original kills its explosions and smoke, and how
    /// many fighters were abandoned, if any, is shown after a `Q`'s
    /// message; a move while landed shows once the ship takes off; and a
    /// `C`, `E` or `H` reads the player's sprite sheet afresh. Then the
    /// session's messages raised as it settled show at once, as a
    /// deadline missed on the take-off does, before a text dialog can
    /// pause flight; an arrival's waits for the arrival as on a tick.
    ///
    /// Docked, the settling is a run of docked scripts as the bar sees it
    /// ([`FlightView::stage_bar`]): a landing's `_MissionLandCheck`
    /// (@0x5fbeb) shows each `CompText`, `FailText`, `ShipDoneText`,
    /// `LoadCargText` and `DumpCargoText` over the last bar drawn, and the
    /// `C`, `E` and `H` that `OnSuccess`, `OnFailure`, `OnShipDone`, an
    /// `S`'s `OnAccept` and the crons of a success's days run redraw at
    /// their operators, ordered against those texts. With no dialog of
    /// its own, nothing redraws after the last text.
    pub(super) fn settle_script_with(&mut self, words: Words<Self>) {
        let settled = self.run_docked(Then::Nothing, |view| {
            match view.send(FlightCommand::SettleScript) {
                Answer::Settled(settled) => settled,
                _ => Settled::default(),
            }
        });
        self.show_settled(settled, false, words);
    }

    /// Shows what a settling did ([`FlightView::settle_script`]), the
    /// screen caught up first; `laid_out` says the system was laid out
    /// afresh since the settling, before this catches up.
    fn show_settled(&mut self, settled: Settled, laid_out: bool, words: Words<Self>) {
        let flying = !self.frame_now().docked();
        let mut said: Vec<(String, u16)> = settled
            .message
            .into_iter()
            .map(|text| (text, SCRIPT_MESSAGE_TICKS))
            .collect();
        if (self.catch_up().system || laid_out) && flying {
            said.extend(self.fighters_said());
        }
        self.say_all(&said);
        self.say_messages(words);
    }
}
