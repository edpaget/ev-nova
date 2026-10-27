//! [`FlightView`]'s mission screens over the [`FlightSim`] port: the
//! mission computer and its rows, the offers and their venues, the
//! accept, the refusal and the abort, the mission info and its key, and
//! a person's offer held for the router. Each sends its command
//! ([`FlightCommand`]) and hands back the flight's answer, a flight that
//! refuses it (one that never started) answering as today's failed
//! session did; what a screen lists is read through
//! [`PortSource`](crate::flight::sim_port::PortSource). The bar's
//! redraws around an accept, a refusal, an abort and an offer accepted at
//! once are staged as their scripts ran ([`FlightView::stage_bar`]).

use std::time::Duration;

use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::mission::screens::{NO_ACTIVE_MISSIONS, mac_ticks};
use nova_sim::{
    AbortError, AcceptError, ComputerRefusal, ComputerRow, MissionComputer, MissionId, MissionInfo,
    MissionInfoPress, NotOffered, OfferPrompt, RefuseShown, VenueOffer,
};

use crate::flight::catalog::StatusBars;
use crate::flight::sim_port::FlightSim;

use super::{FlightView, MESSAGE_TICKS, Then};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Opens the mission computer of the stellar landed on
    /// ([`FlightCommand::OpenMissionComputer`],
    /// `nova_sim::Session::open_mission_computer`).
    ///
    /// # Errors
    ///
    /// The [`ComputerRefusal`] the flight answers; a flight that never
    /// started has nothing available ([`ComputerRefusal::NoneAvailable`]).
    pub fn open_mission_computer(&mut self) -> Result<MissionComputer, ComputerRefusal> {
        match self.send(FlightCommand::OpenMissionComputer) {
            Answer::MissionComputer(computer) => computer,
            _ => Err(ComputerRefusal::NoneAvailable),
        }
    }

    /// The mission computer's row of `mission`, selected and its text
    /// loaded on the flight's chance ([`FlightCommand::ShowComputerRow`],
    /// `nova_sim::Session::show_computer_row`); none for a mission not
    /// offered there, or a flight that never started.
    pub fn show_computer_row(&mut self, mission: MissionId) -> Option<ComputerRow> {
        match self.send(FlightCommand::ShowComputerRow(mission)) {
            Answer::ComputerRow(row) => row,
            _ => None,
        }
    }

    /// Loads `mission`'s offer `dësc` again as the mission info screen
    /// opened over its offer dialog closes, on the flight's chance
    /// ([`FlightCommand::ReloadOfferDesc`],
    /// `nova_sim::Session::reload_offer_desc`).
    pub fn reload_offer_desc(&mut self, mission: MissionId) {
        self.send(FlightCommand::ReloadOfferDesc(mission));
    }

    /// The mission info's text of `slot`, selected and loaded on the
    /// flight's chance ([`FlightCommand::ShowInfoRow`],
    /// `nova_sim::Session::show_info_row`); none for a free slot, or a
    /// flight that never started.
    pub fn show_info_row(&mut self, slot: u8) -> Option<String> {
        match self.send(FlightCommand::ShowInfoRow(slot)) {
            Answer::InfoRow(text) => text,
            _ => None,
        }
    }

    /// The mission info screen's view
    /// ([`PortSource::mission_info`](crate::flight::sim_port::PortSource::mission_info)):
    /// nothing listed for a flight that never started.
    pub fn mission_info(&self) -> MissionInfo {
        self.sim.mission_info()
    }

    /// Enters venue `loc` for its offers ([`FlightCommand::EnterOfferVenue`],
    /// `nova_sim::Session::enter_offer_venue`).
    pub fn enter_offer_venue(&mut self, loc: i16) {
        self.send(FlightCommand::EnterOfferVenue(loc));
    }

    /// The wait before a venue's next offer, drawn on the flight's chance
    /// ([`PortSource::offer_gap`](crate::flight::sim_port::PortSource::offer_gap),
    /// `nova_sim::mission::offer_gap`).
    pub fn offer_gap(&mut self) -> Duration {
        mac_ticks(self.sim.offer_gap())
    }

    /// The mission a person hailed or boarded just offered, once: the
    /// router takes it to open the offer dialog over the paused flight,
    /// and answers it through [`FlightView::accept_mission`] or
    /// [`FlightView::refuse_mission`].
    pub fn take_offer(&mut self) -> Option<OfferPrompt> {
        self.pending_offer.take()
    }

    /// The mission info [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// asked for the mission info screen on, once: the router opens the
    /// screen on it over the paused flight.
    pub fn take_mission_info_request(&mut self) -> Option<MissionInfo> {
        self.pending_mission_info.take()
    }

    /// Presses [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY),
    /// as `_HandlePlayer` does (@0x70405-0x705a8), the flight deciding
    /// ([`FlightCommand::PressMissionInfo`],
    /// `nova_sim::Session::press_mission_info`): with an active mission
    /// shown, asks the router for the mission info screen, letting go of
    /// the flight keys; with none, says the flight's #354 on the message
    /// line, after its refusal beep; refused by the
    /// flight itself, nothing. A flight that refuses the command, one
    /// that never started, says #354 alone, with no session to beep, as
    /// a failed session always has: the decision is the view's, not the
    /// adapter's.
    pub(super) fn ask_mission_info(&mut self) {
        match self.send(FlightCommand::PressMissionInfo) {
            Answer::MissionInfo(MissionInfoPress::Opens(info)) => {
                self.pending_mission_info = Some(info);
                self.held.clear();
            }
            Answer::MissionInfo(MissionInfoPress::NoActiveMissions) => self.say_messages(),
            Answer::Refused => self.say_for(NO_ACTIVE_MISSIONS, MESSAGE_TICKS),
            _ => {}
        }
    }
}

/// The accept, the refusal, the abort and the venues' offers, over the
/// [`FlightSim`] port: their bar is staged as their scripts ran
/// ([`FlightView::run_docked`]), or held over flight under their texts
/// ([`FlightView::answer_in_flight`]).
impl<C: StatusBars, S: FlightSim> FlightView<C, S> {
    /// Accepts `mission` ([`FlightCommand::AcceptMission`],
    /// `nova_sim::Session::accept_mission`), on the flight's chance: the
    /// slot it went into. The HUD is redrawn, as the original's mission
    /// dialog does on closing ([`FlightView::redraw_status`]): every close
    /// of `_DoMissionDialog` that disposes the dialog, an accept with no
    /// text included, ends in `_RefreshSAT` (@0xa40c8-0xa40d7), which
    /// redraws the whole bar, and so does the offer dialog after its
    /// accept (@0xa2e44, then @0xa365c). The port's nav and cargo
    /// `_DrawStatus` after the computer's (@0x6003a-0x60046) adds nothing,
    /// so ours matches. The redraw comes after `OnAccept` runs (@0xa3bb4 →
    /// @0xa40c8), and a landed `M` or `N` in it writes the nav at its
    /// operator, so the redraw shows it.
    ///
    /// Docked, the bar is redrawn as the original draws it around the
    /// accept ([`FlightView::stage_bar`]). The offer dialog with a refuse
    /// button redraws the whole bar first (@0xa2df8,
    /// [`FlightView::redraw_before_answer`]); the mission computer and the
    /// one-button dialog don't. An `OnAccept` `C`, `E` or `H` that changes
    /// the `ïntf` redraws at its operator. The `BriefText` (@0xa1f0e) and
    /// the `LoadCargText` (@0xa1fb5) show over the last bar drawn, each
    /// close redraws the whole bar as the text noted it (@0xa1f13,
    /// @0xa1fba; [`FlightView::close_mission_text`]), and the dialog's own
    /// redraw comes once the last has closed. So under the `BriefText` the
    /// gauges and nav show as before `OnAccept`, or as its last `ïntf`
    /// change left them, as stock `mïsn` 197's and 320's do.
    ///
    /// In flight, a person's offer is answered the same way
    /// ([`FlightView::answer_in_flight`]): the bar is taken before the
    /// accept and held under its texts, which show over it, and drawn
    /// live again once the last has closed.
    ///
    /// # Errors
    ///
    /// The [`AcceptError`] the flight answers; a flight that never started
    /// offers nothing ([`AcceptError::NotOffered`]).
    pub fn accept_mission(&mut self, mission: MissionId) -> Result<u8, AcceptError> {
        let accept = |view: &mut Self| match view.send(FlightCommand::AcceptMission(mission)) {
            Answer::Accepted(accepted) => accepted,
            _ => Err(AcceptError::NotOffered),
        };
        if !self.docked() {
            return self.answer_in_flight(accept);
        }
        self.redraw_before_answer(mission);
        self.run_docked(Then::Redraw, accept)
    }

    /// Answers a person's offer over flight with `answer`, as
    /// `_OfferOneMission` does from a hail (@0x62293) or a boarding
    /// (@0x65936). The dialog with a refuse button hides and redraws the
    /// whole bar first (`_RefreshSAT` @0xa2df8 before the accept, @0xa2e93
    /// before a refusal's `RefuseText`). The one-button dialog makes no
    /// such redraw (@0xa2f52-0xa2fba), but flight doesn't tick under the
    /// comm dialog, so the bar it last drew is the same, and ours redraws
    /// for both. The answer's texts then show over that bar: the text
    /// dialog's modal loop draws no bar (`_TextDialogFilter`, and
    /// `_HandleStatus`'s only caller, `_DoPlayGameWork` @0x4403c, doesn't
    /// run under it), so `OnAccept`'s writes, or `OnRefuse`'s, show only
    /// from the text's close (@0xa1f13, @0xa2f09), then the dialog's own
    /// redraw (@0xa365c), after which flight draws the gauges and nav live
    /// again ([`FlightView::reserves_shown`], [`FlightView::nav_shown`]).
    /// The texts' closes come from the app ([`FlightView::close_mission_text`]).
    fn answer_in_flight<T>(&mut self, answer: impl FnOnce(&mut Self) -> T) -> T {
        self.redraw_status();
        let answered = answer(self);
        self.stage_bar(Vec::new(), Then::Redraw);
        answered
    }

    /// The offer dialog with a refuse button hides and redraws the whole
    /// bar before it answers (`_OfferOneMission` @0xa2df8 before the
    /// accept, @0xa2e93 before a refusal's `RefuseText`;
    /// [`PortSource::refusable_offer_dialog`](crate::flight::sim_port::PortSource::refusable_offer_dialog));
    /// the one-button dialog and the mission computer redraw nothing
    /// before theirs (@0xa2f52-0xa2fba, @0xa3b37-0xa3bb4). A refusal with
    /// no `RefuseText` skips the redraw (@0xa2e70-0xa2e78), but nothing
    /// shows before the dialog's own redraw after it, so ours redraws
    /// either way.
    fn redraw_before_answer(&mut self, mission: MissionId) {
        if self.sim.refusable_offer_dialog(mission) {
            self.redraw_status();
        }
    }

    /// Refuses `mission` ([`FlightCommand::RefuseMission`],
    /// `nova_sim::Session::refuse_mission`), on the flight's chance. The
    /// HUD is redrawn, as on accepting, after `OnRefuse` runs, so a landed
    /// `M` or `N` in it shows, as the offer dialog's `_RefreshSAT`
    /// (@0xa365c) redraws last.
    ///
    /// Docked, the offer dialog first hides and redraws the whole bar
    /// (@0xa2e93, [`FlightView::redraw_before_answer`]), the `RefuseText`
    /// shows over it (@0xa2f04), and its close redraws the bar as the text
    /// noted it (@0xa2f09), before `OnRefuse` (@0xa2f41), whose `C`, `E`
    /// or `H` changing the `ïntf` redraws at its operator; the dialog's
    /// own redraw comes last ([`FlightView::stage_bar`]). A person's offer
    /// refused over flight holds the bar the same way
    /// ([`FlightView::answer_in_flight`]): under the `RefuseText`, it
    /// shows as before the refusal.
    ///
    /// # Errors
    ///
    /// [`NotOffered`] when it is not offered; a flight that never started
    /// offers nothing.
    pub fn refuse_mission(&mut self, mission: MissionId) -> Result<RefuseShown, NotOffered> {
        let refuse = |view: &mut Self| match view.send(FlightCommand::RefuseMission(mission)) {
            Answer::RefuseShown(shown) => shown,
            _ => Err(NotOffered),
        };
        if !self.docked() {
            return self.answer_in_flight(refuse);
        }
        self.redraw_before_answer(mission);
        self.run_docked(Then::Redraw, refuse)
    }

    /// Aborts the mission in `slot` ([`FlightCommand::AbortMission`],
    /// `nova_sim::Session::abort_mission`), on the flight's chance.
    /// Docked, an `OnAbort` `C`, `E` or `H` that changes the `ïntf`
    /// redraws the whole bar at its operator (`_AbortMission` @0x9e5e6 →
    /// @0x99ba1), and nothing else redraws while the mission info screen
    /// stays open: its close does (@0x9e769, [`FlightView::refresh_status`]).
    ///
    /// # Errors
    ///
    /// The [`AbortError`] the flight answers; a flight that never started
    /// has none active ([`AbortError::NotActive`]).
    pub fn abort_mission(&mut self, slot: u8) -> Result<(), AbortError> {
        self.run_docked(Then::Nothing, |view| {
            match view.send(FlightCommand::AbortMission(slot)) {
                Answer::Aborted(aborted) => aborted,
                _ => Err(AbortError::NotActive),
            }
        })
    }

    /// The next offer at venue `loc` ([`FlightCommand::NextOffer`],
    /// `nova_sim::Session::next_offer`), on the flight's chance; none with
    /// nothing to offer, or for a flight that never started. An offer
    /// accepted at once (@0xa230a) redraws as its accept's scripts and
    /// texts do ([`FlightView::stage_bar`]), with no dialog's redraw
    /// after them: `_OfferOneMission` returns at once (@0xa230f →
    /// @0xa368e).
    pub fn next_offer(&mut self, loc: i16) -> Option<VenueOffer> {
        self.run_docked(Then::Nothing, |view| {
            match view.send(FlightCommand::NextOffer(loc)) {
                Answer::Offer(offer) => offer,
                _ => None,
            }
        })
    }
}
