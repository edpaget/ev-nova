//! The hand-offs between the spaceport, flight and the mission dialogs.

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::{MissionInfo, text_tags};
use nova_view::Screen;
use nova_view::flight::FlightView;
use nova_view::spaceport::{PortMap, SpaceportView};
use nova_view::ui::desc::DESC_DIALOG;
use nova_view::ui::mission_dialogs::MissionDialogs;
use nova_view::ui::{DescDialog, DescriptionSource};

use super::{Dialogs, ENTERED};

/// Shows `spaceport`'s outfitter as `flight` has it, once it has opened
/// ([`SpaceportView::take_outfitter_opened`]): `flight` clears what lasts
/// one opening first. Listing nothing, the outfitter says so and never
/// opens ([`SpaceportView::set_opened_outfitter`]), and the port's
/// `_DrawStatus` after it still draws a marked nav
/// ([`FlightView::empty_outfitter_returned`]), before any port dialog's
/// opening is told.
pub(super) fn open_outfitter(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    if !spaceport.take_outfitter_opened() {
        return;
    }
    flight.open_outfitter();
    if let Some(outfitter) = flight.outfitter() {
        spaceport.set_opened_outfitter(outfitter);
    }
    if spaceport.take_empty_outfitter() {
        flight.empty_outfitter_returned();
    }
}

/// Opens `flight`'s course map over `spaceport` when a map key there asked
/// for it ([`SpaceportView::take_map_request`]): the bar's (`_BarFilter`
/// item 8, `_DoSystemMap` @0x492b2), or a mission dialog's, an offer's,
/// the computer's, mission info's or the briefing's
/// ([`FlightView::open_dialog_map`]), the spaceport letting go of its
/// gesture and keys as it is covered. The screen that asked stays open
/// under the map and has the input again once it closes.
pub(super) fn open_port_map(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    let Some(map) = spaceport.take_map_request() else {
        return;
    };
    spaceport.cancel_pointer();
    spaceport.release_keys();
    match map {
        PortMap::Bar => flight.open_bar_map(),
        PortMap::Dialog(opener) => flight.open_dialog_map(opener),
    }
}

/// Answers `spaceport`'s Mission BBS with `flight`'s mission computer and
/// the bar's Hire Escort with the ships for hire, then tells `flight` of
/// the port dialog that has opened ([`FlightView::open_port_dialog`]),
/// which saves the nav before any order or offer run inside it, in the
/// input that opened it.
pub(super) fn open_port_dialogs(
    spaceport: &mut SpaceportView,
    flight: &mut FlightView<Rc<GameData>>,
) {
    if spaceport.take_mission_computer_request() {
        let computer = flight.open_mission_computer();
        spaceport.open_mission_computer_after(computer, flight.loaded_after_movie());
    }
    if spaceport.take_hire_request()
        && let Some(list) = flight.escorts_for_hire()
    {
        spaceport.open_hire(list);
    }
    if spaceport.take_port_dialog_opened().is_some() {
        flight.open_port_dialog();
    }
}

/// Tells `flight` of the port dialog of `spaceport` that has closed
/// ([`FlightView::close_port_dialog`]): its `_RefreshSAT`, then the nav's
/// restore, as each of the original's dialogs closes, the bar's hire
/// screen among them; after the input's last order, and before any frame
/// is drawn. A mission computer that closed under mission texts is told
/// once they have closed.
pub(super) fn close_port_dialogs(
    spaceport: &mut SpaceportView,
    flight: &mut FlightView<Rc<GameData>>,
) {
    if let Some(dialog) = spaceport.take_port_dialog_closed() {
        flight.close_port_dialog(dialog);
    }
}

/// Tells `flight` of each mission text the spaceport has closed since it
/// was last told, as it closed or was dropped
/// ([`SpaceportView::take_texts_closed`]): flight redraws the bar as the
/// original's caller of each text does as it returns
/// ([`FlightView::close_mission_text`]). It runs after the spaceport's
/// input, before the closings that wait for the texts redraw, and as the
/// texts queued since are shown, after every input and tick, so a text
/// closed on a movie's tick, or dropped, redraws before the next frame.
pub(super) fn close_mission_texts(
    spaceport: &mut SpaceportView,
    flight: &mut FlightView<Rc<GameData>>,
) {
    for _ in 0..spaceport.take_texts_closed() {
        flight.close_mission_text();
    }
}

/// Redraws `flight`'s whole status bar ([`FlightView::refresh_status`])
/// when a closing of `spaceport` that the original ends in `_RefreshSAT`
/// has come ([`SpaceportView::take_status_refresh`]): the bar, News, a
/// ship description, player info or mission info. It runs at the end of
/// the spaceport's input, after the input's last order (an abort's
/// mission info, a jettison), and before any frame is drawn.
pub(super) fn refresh_status_bar(
    spaceport: &mut SpaceportView,
    flight: &mut FlightView<Rc<GameData>>,
) {
    if spaceport.take_status_refresh() {
        flight.refresh_status();
    }
}

/// Shows `spaceport` the exchange, outfitter and shipyard as `flight`
/// now has them.
pub(super) fn refresh_spaceport(
    spaceport: &mut SpaceportView,
    flight: &mut FlightView<Rc<GameData>>,
) {
    if let Some(market) = flight.market() {
        spaceport.set_market(market);
    }
    if let Some(outfitter) = flight.outfitter() {
        spaceport.set_outfitter(outfitter);
    }
    if let Some(shipyard) = flight.shipyard() {
        spaceport.set_shipyard(shipyard);
    }
}

/// An open mission info screen, as whatever holds one shows it: the
/// spaceport, or the mission dialogs over flight. Its Abort and the text
/// it asks for are answered through flight
/// ([`abort_and_refresh`], [`load_info_text`]).
pub(super) trait MissionInfoHolder {
    /// The slot the screen asked to abort since this was last taken,
    /// once.
    fn take_abort(&mut self) -> Option<u8>;
    /// After an abort, the screen shows `info`, closing once nothing is
    /// left.
    fn set_info(&mut self, info: MissionInfo);
    /// The slot whose `QuickBrief` the screen asked for since this was
    /// last taken, once.
    fn take_info_text_request(&mut self) -> Option<u8>;
    /// Shows `text`, slot `slot`'s, while that slot is selected.
    fn show_info_text(&mut self, slot: u8, text: &str);
}

impl MissionInfoHolder for SpaceportView {
    fn take_abort(&mut self) -> Option<u8> {
        self.take_mission_abort()
    }

    fn set_info(&mut self, info: MissionInfo) {
        self.set_mission_info(info);
    }

    fn take_info_text_request(&mut self) -> Option<u8> {
        SpaceportView::take_info_text_request(self)
    }

    fn show_info_text(&mut self, slot: u8, text: &str) {
        SpaceportView::show_info_text(self, slot, text);
    }
}

impl MissionInfoHolder for MissionDialogs {
    fn take_abort(&mut self) -> Option<u8> {
        MissionDialogs::take_abort(self)
    }

    fn set_info(&mut self, info: MissionInfo) {
        MissionDialogs::set_info(self, info);
    }

    fn take_info_text_request(&mut self) -> Option<u8> {
        MissionDialogs::take_info_text_request(self)
    }

    fn show_info_text(&mut self, slot: u8, text: &str) {
        MissionDialogs::show_info_text(self, slot, text);
    }
}

/// An Abort `holder`'s mission info screen asked for frees the slot
/// through `flight`, and the screen shows the missions left, closing once
/// nothing is left; a refusal (greyed) changes nothing. Flight is needed
/// only once an Abort was asked for.
pub(super) fn abort_and_refresh(
    holder: &mut impl MissionInfoHolder,
    flight: Option<&mut FlightView<Rc<GameData>>>,
) {
    if let Some(slot) = holder.take_abort() {
        let flight = flight.expect(ENTERED);
        let _ = flight.abort_mission(slot);
        holder.set_info(flight.mission_info());
    }
}

/// The text `holder`'s mission info screen has shown selected, loaded
/// through `flight` as the original loads a selected row's
/// (`_MissionInfoFilter` @0x9d89f) and shown while still selected.
pub(super) fn load_info_text(
    holder: &mut impl MissionInfoHolder,
    flight: &mut FlightView<Rc<GameData>>,
) {
    if let Some(slot) = holder.take_info_text_request()
        && let Some(text) = flight.show_info_row(slot)
    {
        holder.show_info_text(slot, &text);
    }
}

/// "Desc Dialog" showing `text`, from the dialogs, in the game data's
/// button style.
pub(super) fn desc_dialog(
    dialogs: &Dialogs,
    data: &GameData,
    text: &str,
) -> Result<DescDialog, String> {
    let template = dialogs.resources.dialog_template(DESC_DIALOG)?;
    Ok(DescDialog::new(
        &template,
        text,
        data.button_style(),
        Rc::clone(&dialogs.metrics),
    ))
}

/// The about text or the main menu's X text `raw` as the original loads
/// and munges it at start-up: `_LoadDescription` (about @0x16021, X
/// @0x1ac9d) resolves its conditional text by `rules` for
/// [`StartUpPilot`](nova_sim::control::StartUpPilot), with no rank
/// governments and no draws, as there is no flight; then
/// `_MungeBriefing(0, -1)` (about @0x16035, X @0x1acb1) fills its tags
/// from the zeroed globals ([`text_tags::expand_before_any_pilot`]).
/// After a flight the original reads the last pilot's globals instead
/// (no `_ResetPlayer` after `_PlayGame`); the engine holds no pilot
/// outside flight, which rdm:phase/main-screen/phase-3-game-in-progress
/// builds.
pub(super) fn start_up_words(
    raw: &str,
    data: &(impl nova_sim::PilotCatalog + ?Sized),
    rules: nova_sim::text_parse::ParseRules,
) -> String {
    let resolved =
        nova_sim::text_parse::conditional_text(raw, &nova_sim::control::StartUpPilot, rules);
    text_tags::expand_before_any_pilot(&resolved, data)
}
