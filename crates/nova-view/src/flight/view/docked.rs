//! [`FlightView`]'s accessors and its docked screens: messages, boarding
//! and plunder, trade, the outfitter, bar, hiring, news, the mission
//! computer and offers, and the shipyard.

use std::time::Duration;

use nova_sim::mission::offer_gap;
use nova_sim::mission::screens::mac_ticks;
use nova_sim::{
    AbortError, AcceptError, ComputerRefusal, ComputerRow, HireList, HireRefusal, Hired,
    InterfaceLoad, MissionComputer, MissionId, MissionInfo, MissionShown, News, NotOffered,
    RefuseShown, VenueOffer,
};
use nova_sim::{
    Boarding, Market, Order, Outfitter, Pilot, PlunderView, RechargeRefusal, Session, ShipId,
    ShipNaming, ShipRefusal, Shipyard, SimMessage, StellarId, Take, Taken, TradeRefusal,
};

use crate::flight::catalog::StatusBars;
use crate::flight::jump::JumpEffect;
use crate::galaxy::GalaxyMap;
use crate::time::sixtieths;
use crate::ui::PlunderShown;
use crate::{Key, Point};

use super::outputs::FlightTexts;
use super::{
    BOARD_LINE_TICKS, BOARD_REFUSAL_TICKS, BarStep, FlightView, GRANT_TICKS, MESSAGE_TICKS,
    NavFlash, REPELLED, Then, board_refusal_message, grant_message, message_text,
    mission_board_message, names, plunder_message,
};

impl<C> FlightView<C, crate::flight::SessionFlight<C>> {
    /// Where the hover cursor is drawn, or `None` while it isn't.
    #[must_use]
    pub fn cursor_at(&self) -> Option<Point> {
        self.cursor.shown_at()
    }

    /// Whether the course map is shown.
    #[must_use]
    pub fn map_open(&self) -> bool {
        self.map_open
    }

    /// Whether the course map is shown with its Find dialog open: while it
    /// is, every input goes to it, M and Escape included.
    #[must_use]
    pub fn map_finding(&self) -> bool {
        self.map_open && self.map.finding()
    }

    /// The course map, shown or not.
    #[must_use]
    pub fn course_map(&self) -> &GalaxyMap {
        &self.map
    }

    /// Whether the course map shows the borders: the player's
    /// `ShowMapBorders`, which a run keeps for its next flight.
    #[must_use]
    pub fn map_borders_shown(&self) -> bool {
        self.map.borders_shown()
    }

    /// The borders setting to save, once, after an undocked close of the
    /// course or hypergate map that changed it, as `_SaveMapBordersState`
    /// writes `ShowMapBorders` then (@0x13c21-0x13c3f, @0xcf110); taking
    /// it clears it. A change made docked lasts for the run, unsaved.
    pub fn take_map_borders_save(&mut self) -> Option<bool> {
        self.map_borders_save.take()
    }

    /// The course map's zoom, which the run keeps for the next flight
    /// ([`FlightView::with_course_map_zoom`]).
    #[must_use]
    pub fn course_map_zoom(&self) -> Option<f64> {
        self.map.zoom()
    }

    /// The jump's effect, while it plays: the warp and its fade to white
    /// until the ship arrives, or else the arrival's fade-in (or flash).
    #[must_use]
    pub fn jump_effect(&self) -> Option<&JumpEffect> {
        self.jump.as_ref().or(self.fade_in.as_ref())
    }

    /// The stellar the ship has just landed on, once: the router takes it
    /// to show the spaceport.
    pub fn take_landing(&mut self) -> Option<StellarId> {
        self.pending_landing.take()
    }

    /// Shows `text` as the message, from now, for `ticks` of flight,
    /// replacing whatever was shown, as `_DisplayComm` does
    /// (@0x9024b-0x90256), and sets the session's message line to it
    /// ([`Session::comm_shown`]): a line the view raises itself, or one
    /// it composes from the session's at an arrival, a take-off or a
    /// settling, where nothing counts down between the session raising
    /// the parts and the view showing the whole. The lines the session
    /// raised in a frame's steps are relayed ([`FlightView::relay`]).
    ///
    /// Every write to the message slot goes through this, `relay`,
    /// [`FlightView::clear_message`], [`FlightView::acknowledge_message`]
    /// or [`FlightView::blank_message`], so the session's line stays in
    /// step with the slot.
    pub(super) fn say_for(&mut self, text: impl Into<String>, ticks: u16) {
        self.relay(text, ticks);
        if let Ok(session) = &mut self.sim.session {
            session.comm_shown(ticks);
        }
    }

    /// Clears the message line when it holds `text`, as the original
    /// clears one message by name (`_HandlePlayer` @0x6b231-0x6b2a7,
    /// @0x6dedb-0x6df29): it shows an empty line for 1 tick, the
    /// session's line too.
    pub(super) fn clear_message(&mut self, text: &str) {
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.text == text)
        {
            self.message = None;
            if let Ok(session) = &mut self.sim.session {
                session.comm_shown(1);
            }
        }
    }

    /// Return (`ackComm`): clears the message line, and the session's
    /// for a tick while a line shows ([`Session::comm_acknowledged`],
    /// `_HandlePlayer` @0x6dd22-0x6dd65).
    pub(super) fn acknowledge_message(&mut self) {
        self.message = None;
        if let Ok(session) = &mut self.sim.session {
            session.comm_acknowledged();
        }
    }

    /// Empties the message slot at a change the session settles its line
    /// for itself: a landing, which sets it to 0
    /// (`_HandlePlayerDockRequest` @0x66d7f); the take-off after it, from
    /// that 0; and an arrival, whose own line the session raised and the
    /// view shows next ([`FlightView::say_for`]).
    pub(super) fn blank_message(&mut self) {
        self.message = None;
    }

    /// Holds `offer`'s prompt for the router, letting go of the flight
    /// keys; a mission accepted at once shows nothing here, its texts
    /// queued in the session, its before-movie first, for the router to
    /// show over flight.
    pub(super) fn hold_offer(&mut self, offer: VenueOffer) {
        if let VenueOffer::Prompt(prompt) = offer {
            self.pending_offer = Some(prompt);
            self.held.clear();
        }
    }

    /// Boards the target, or says why not: a boarding that opens is held
    /// for the router ([`FlightView::take_boarding`]), and a mission
    /// offered in its place too ([`FlightView::take_offer`]), each letting
    /// go of the flight keys. A failed mission's ship boarded for plunder
    /// says its pickup or assist line first ([`FlightEvent::BoardLine`](nova_sim::flight_frame::FlightEvent::BoardLine)).
    /// Whether the attempt sets the board latch
    /// ([`BoardRefusal::latches`](nova_sim::board::BoardRefusal::latches);
    /// every board that passes does).
    pub(super) fn board(&mut self) -> bool {
        let Ok(session) = &mut self.sim.session else {
            return false;
        };
        let boarded = session.board(
            &*self.sim.law,
            &*self.sim.boarding_rule,
            &mut self.sim.chance,
        );
        if let Some(line) = self.board_line_drained()
            && let Ok(session) = &self.sim.session
        {
            let said = mission_board_message(line, session);
            self.say_for(said, BOARD_LINE_TICKS);
        }
        let latches = boarded
            .as_ref()
            .map_or_else(|refusal| refusal.latches(), |_| true);
        if self.sim.session.is_err() {
            return latches;
        }
        match boarded {
            Ok(Boarding::Opened(_)) => {
                self.pending_boarding = true;
                self.held.clear();
                if let Some(granted) = self.grant_drained()
                    && let Ok(session) = &self.sim.session
                {
                    let (name, plural) = session.outfit_names(granted.outfit).unwrap_or_default();
                    let said = grant_message(granted.count, name, plural);
                    self.say_for(said, GRANT_TICKS);
                }
            }
            Ok(Boarding::Repelled) => self.say_for(REPELLED, MESSAGE_TICKS),
            Ok(Boarding::Offered(offer)) => self.hold_offer(offer),
            Ok(Boarding::MissionShip(line)) => {
                if let Some(line) = line
                    && let Ok(session) = &self.sim.session
                {
                    let said = mission_board_message(line, session);
                    self.say_for(said, BOARD_LINE_TICKS);
                }
            }
            // Its line is the session's message (`FlightEvent::Message`).
            Ok(Boarding::Recovered(_)) => {}
            Err(refusal) => {
                if let Some(text) = board_refusal_message(refusal) {
                    self.say_for(text, BOARD_REFUSAL_TICKS);
                }
            }
        }
        latches
    }

    /// One frame of the Board key, with B down or not, as `_HandlePlayer`
    /// runs it (@0x6a98b-0x6a9a8) on [`FlightView::board_latch`]: with B
    /// down and the latch clear, a board attempt
    /// ([`FlightView::board`]), which sets the latch unless it found no
    /// target or the player cloaked (`*0xdee18` @0x6504c, after
    /// @0x65031 and @0x6503f); with B up, the latch cleared. So B held
    /// with no target, or cloaked, boards on the first frame a target is
    /// taken or the cloak drops, and any other outcome waits for B's
    /// release.
    pub(super) fn board_frame(&mut self, down: bool) {
        if !down {
            self.board_latch = false;
        } else if !self.board_latch {
            self.board_latch = self.board();
        }
    }

    /// Ends the hail under way, if any ([`Session::hang_up`]), and
    /// refreshes the HUD as the original's comm dialog does on closing
    /// ([`FlightView::redraw_status`]).
    pub fn hang_up(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.hang_up();
        }
        self.redraw_status();
    }

    /// What is on board the ship just boarded, named, once: the router
    /// takes it to open the plunder dialog.
    pub fn take_boarding(&mut self) -> Option<PlunderShown> {
        if !std::mem::take(&mut self.pending_boarding) {
            return None;
        }
        self.plunder_shown()
    }

    /// What is on board the ship being boarded, while the plunder dialog
    /// is open ([`Session::boarding`]).
    #[must_use]
    pub fn boarding(&self) -> Option<PlunderView> {
        self.sim.session.as_ref().ok()?.boarding()
    }

    /// What the plunder dialog shows while it is open: [`Self::boarding`]
    /// with its cargo's good and its ammunition's outfit named, as
    /// [`Self::plunder`]'s messages name them.
    #[must_use]
    pub fn plunder_shown(&self) -> Option<PlunderShown> {
        let session = self.sim.session.as_ref().ok()?;
        let view = session.boarding()?;
        let (good, outfit) = names(
            session,
            view.cargo.map(|(good, _)| good),
            view.ammo.map(|(outfit, _)| outfit),
        );
        Some(PlunderShown {
            view,
            good: good.map(str::to_owned),
            outfit: outfit.map(str::to_owned),
        })
    }

    /// Presses `take` in the plunder dialog, through the session
    /// ([`Session::plunder`]), and says what it did. The target panel is
    /// taken afresh, as `_HandlePlayer` redraws it once the dialog closes
    /// (@0x6eb97, @0x6eda6). A press made as the dialog opens: see
    /// [`FlightView::plunder_held`].
    pub fn plunder(&mut self, take: Take) -> Taken {
        self.plunder_held(take, Duration::ZERO)
    }

    /// [`FlightView::plunder`] with the dialog open for `held`, real time.
    /// A press that leaves the dialog open ends the loop's pass with
    /// `_HandleComm` (@0x949a1): the message line counts down one tick,
    /// a take's own line after `_DisplayComm` sets it, so it shows 239,
    /// and the floating map is hidden once more than 500 ticks have
    /// passed since it was shown, `TickCount` moving on under the dialog
    /// (@0x30262-0x30285).
    pub fn plunder_held(&mut self, take: Take, held: Duration) -> Taken {
        let Ok(session) = &mut self.sim.session else {
            return Taken::Nothing;
        };
        let taken = session.plunder(take, &*self.sim.boarding_rule, &mut self.sim.chance);
        let open = session.boarding().is_some();
        let (good, outfit) = match taken {
            Taken::Cargo { good, .. } => names(session, Some(good), None),
            Taken::Ammo { outfit, .. } => names(session, None, Some(outfit)),
            _ => (None, None),
        };
        if let Some(text) = plunder_message(taken, good, outfit) {
            self.say_for(text, MESSAGE_TICKS);
            // The session's pass counted the line it had; the take's own
            // comes first in the original (`_DisplayComm`, then
            // `_HandleComm`).
            if open && let Ok(session) = &mut self.sim.session {
                session.comm_ticked();
            }
        }
        if open {
            if let Some(message) = &mut self.message {
                message.ticks = message.ticks.saturating_sub(1);
            }
            self.floating.expire(sixtieths(self.elapsed + held));
        }
        self.retake_target();
        taken
    }

    /// Lets the boarding under way go with no dialog shown and no sound
    /// ([`Session::let_go_of_boarding`]): the router's way out when it
    /// cannot lay the plunder dialog out.
    pub fn let_go_of_boarding(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.let_go_of_boarding();
        }
    }

    /// Shows the session's messages since they were last taken, each in
    /// turn, in the original's words; an arrival's waits for its arrival
    /// to be caught up with ([`FlightView::arrived`]). Called right after
    /// a press or a settling raised them, with nothing counted down
    /// since, it sets the session's line to the last shown
    /// ([`FlightView::say_for`]); after a frame's steps,
    /// [`FlightView::relay_messages`] leaves the line alone.
    pub(super) fn say_messages(&mut self) {
        for (text, ticks) in self.worded_messages() {
            self.say_for(text, ticks);
        }
    }

    /// Shows the session's messages raised in a frame's steps, as
    /// [`FlightView::say_messages`] does, but relayed
    /// ([`FlightView::relay`]): the session set its line as it raised
    /// each, and the steps have counted it down since.
    pub(super) fn relay_messages(&mut self) {
        for (text, ticks) in self.worded_messages() {
            self.relay(text, ticks);
        }
    }

    /// The session's messages since they were last taken, worded.
    fn worded_messages(&mut self) -> Vec<(String, u16)> {
        let messages = self.drain().take_messages();
        self.word(messages)
    }

    /// The words of each of `messages`, in order, with its ticks, the
    /// stellars named from the scene; an arrival's has none here, and
    /// waits for its arrival to be caught up with
    /// ([`FlightView::arrived`]), which leads with it.
    pub(super) fn word(&mut self, messages: Vec<SimMessage>) -> Vec<(String, u16)> {
        let Ok(session) = &self.sim.session else {
            return Vec::new();
        };
        let mut said = Vec::new();
        for message in messages {
            match message_text(&message, session, self.scene.as_ref(), &mut self.sim.chance) {
                Some(text) => said.push((text, message.ticks())),
                None => self.pending_leads.push(message),
            }
        }
        said
    }

    /// Presses the jettison key: with Command held, jettisons the cargo,
    /// and with Option (Alt) too, all but the mission cargo (see
    /// [`Session::jettison_cargo`]), saying what went.
    pub(super) fn jettison(&mut self) {
        if !self.held.contains(&Key::Command) {
            return;
        }
        let all = !self.held.contains(&Key::Alt);
        if let Ok(session) = &mut self.sim.session {
            session.jettison_cargo(all, &mut self.sim.chance);
        }
        self.say_messages();
    }

    /// The flight, or why it could not start.
    pub fn session(&self) -> Result<&Session, &str> {
        self.sim.session.as_ref().map_err(String::as_str)
    }

    /// Whether the pilot's load had to fall back for data the save
    /// references and the catalog lacks ([`Session::load_repaired`]):
    /// false for a session that failed.
    #[must_use]
    pub fn load_repaired(&self) -> bool {
        self.sim.session.as_ref().is_ok_and(Session::load_repaired)
    }

    /// The pilot flying, if the flight started.
    #[must_use]
    pub fn pilot(&self) -> Option<&Pilot> {
        self.sim.session.as_ref().ok().map(Session::pilot)
    }

    /// Whether the pilot flying has seen the intro screen
    /// ([`Session::seen_intro`]); false when the flight never started.
    #[must_use]
    pub fn seen_intro(&self) -> bool {
        self.sim.session.as_ref().is_ok_and(Session::seen_intro)
    }

    /// Marks the pilot flying as having seen the intro screen
    /// ([`Session::mark_intro_seen`]); nothing when the flight never
    /// started.
    pub fn mark_intro_seen(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.mark_intro_seen();
        }
    }

    /// Changes the pilot with `change` while landed, as
    /// [`Session::transact`] does, and says whether it did.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        self.sim
            .session
            .as_mut()
            .is_ok_and(|session| session.transact(change))
    }

    /// The exchange of the stellar landed on, as [`Session::market`] gives
    /// it; none for a session that failed.
    #[must_use]
    pub fn market(&self) -> Option<Market> {
        self.sim.session.as_ref().ok()?.market()
    }

    /// Trades as [`Session::trade`] does; a session that failed has no
    /// exchange. A trade that goes through runs the exchange's
    /// `_DrawStatus` (@0x5e33a, @0x5e59e), which draws the nav when the
    /// shipyard's restore marked it ([`FlightView::close_port_dialog`]).
    pub fn trade(&mut self, order: Order) -> Result<u32, TradeRefusal> {
        let traded = match &mut self.sim.session {
            Ok(session) => session.trade(order)?,
            Err(_) => return Err(TradeRefusal::NoMarket),
        };
        // The cargo area shows the hold traded (`FrameHud::cargo`).
        self.refresh_frame();
        self.draw_marked_nav();
        Ok(traded)
    }

    /// The outfitter of the stellar landed on, as [`Session::outfitter`]
    /// gives it, the day's rolls drawn on the flight's chance; none for a
    /// session that failed.
    pub fn outfitter(&mut self) -> Option<Outfitter> {
        self.sim
            .session
            .as_mut()
            .ok()?
            .outfitter(&mut self.sim.chance)
    }

    /// The outfitter opens, as [`Session::open_outfitter`] has it; a
    /// session that failed has none.
    pub fn open_outfitter(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.open_outfitter();
        }
    }

    /// The outfitter closes, as [`Session::close_outfitter`] has it; a
    /// session that failed has none.
    pub fn close_outfitter(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.close_outfitter();
        }
    }

    /// The bar's hire screen closes, as [`Session::close_hire`] has it; a
    /// session that failed has no bar.
    pub fn close_hire(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.close_hire();
        }
    }

    /// Recharges as [`Session::recharge`] does; a session that failed
    /// sells no fuel. A recharge redraws the fuel gauge alone, as the
    /// original's marks only fuel and cargo (`_statusRedraw[4]` @0x5fdf6)
    /// before its `_DrawStatus` (@0x5fe05); the shield and armour stay as
    /// they were drawn. The nav is drawn too when the shipyard's restore
    /// marked it ([`FlightView::close_port_dialog`]). A refused recharge
    /// draws nothing.
    pub fn recharge(&mut self) -> Result<i64, RechargeRefusal> {
        let paid = match &mut self.sim.session {
            Ok(session) => session.recharge()?,
            Err(_) => return Err(RechargeRefusal::NoFuel),
        };
        // The fuel gauge shows the fuel bought (`FrameHud::reserves`).
        self.refresh_frame();
        self.draw_marked_nav();
        let fuel = self.reserves().fuel;
        if let Some(held) = &mut self.reserves_scene {
            held.fuel = fuel;
        }
        Ok(paid)
    }

    /// Whether the spaceport's Recharge button is enabled, as
    /// [`Session::recharge_enabled`] says; a session that failed sells no
    /// fuel.
    #[must_use]
    pub fn recharge_enabled(&self) -> bool {
        self.sim
            .session
            .as_ref()
            .is_ok_and(Session::recharge_enabled)
    }

    /// The ships for hire in the bar of the stellar landed on, as
    /// [`Session::escorts_for_hire`] gives them, the day's rolls drawn on
    /// the flight's chance; none for a session that failed.
    pub fn escorts_for_hire(&mut self) -> Option<HireList> {
        self.sim
            .session
            .as_mut()
            .ok()?
            .escorts_for_hire(&mut self.sim.chance)
    }

    /// Opens the bar of the stellar landed on as [`Session::open_bar`]
    /// does, drawing the visit's news on the flight's chance; false for a
    /// session that failed, or not landed at a bar.
    pub fn open_bar(&mut self) -> bool {
        self.sim
            .session
            .as_mut()
            .is_ok_and(|session| session.open_bar(&mut self.sim.chance))
    }

    /// The news of the bar visit under way, as [`Session::news`] gives
    /// it; none for a session that failed.
    #[must_use]
    pub fn news(&self) -> Option<&News> {
        self.sim.session.as_ref().ok()?.news()
    }

    /// The picture the bar's news is drawn over, as
    /// [`Session::news_picture`] gives it; none for a session that failed.
    #[must_use]
    pub fn news_picture(&self) -> Option<i16> {
        self.sim.session.as_ref().ok()?.news_picture()
    }

    /// Hires a ship as [`Session::hire`] does, on the flight's chance; a
    /// session that failed has no bar.
    pub fn hire(&mut self, ship: ShipId) -> Result<Hired, HireRefusal> {
        match &mut self.sim.session {
            Ok(session) => session.hire(ship, &mut self.sim.chance),
            Err(_) => Err(HireRefusal::NoBar),
        }
    }

    /// The mission computer of the stellar landed on, as
    /// [`Session::mission_computer`] gives it; a session that failed has
    /// nothing available.
    ///
    /// # Errors
    ///
    /// The [`ComputerRefusal`] that applies.
    pub fn mission_computer(&self) -> Result<MissionComputer, ComputerRefusal> {
        match &self.sim.session {
            Ok(session) => session.mission_computer(),
            Err(_) => Err(ComputerRefusal::NoneAvailable),
        }
    }

    /// Opens the mission computer of the stellar landed on, as
    /// [`Session::open_mission_computer`] does; a session that failed has
    /// nothing available.
    ///
    /// # Errors
    ///
    /// The [`ComputerRefusal`] that applies.
    pub fn open_mission_computer(&mut self) -> Result<MissionComputer, ComputerRefusal> {
        match &mut self.sim.session {
            Ok(session) => session.open_mission_computer(),
            Err(_) => Err(ComputerRefusal::NoneAvailable),
        }
    }

    /// The mission computer's row of `mission`, selected and its text
    /// loaded, as [`Session::show_computer_row`] gives it on the flight's
    /// chance; none for a session that failed.
    pub fn show_computer_row(&mut self, mission: MissionId) -> Option<ComputerRow> {
        match &mut self.sim.session {
            Ok(session) => session.show_computer_row(mission, &mut self.sim.chance),
            Err(_) => None,
        }
    }

    /// The after-movie a `STR#` text shown now plays, as
    /// [`Session::loaded_after_movie`] gives it; none for a session that
    /// failed.
    #[must_use]
    pub fn loaded_after_movie(&self) -> Option<nova_sim::DescMovie> {
        self.sim
            .session
            .as_ref()
            .ok()
            .and_then(Session::loaded_after_movie)
    }

    /// Loads `mission`'s offer `dësc` again as the mission info screen
    /// opened over its offer dialog closes, as
    /// [`Session::reload_offer_desc`] does on the flight's chance; nothing
    /// for a session that failed.
    pub fn reload_offer_desc(&mut self, mission: MissionId) {
        if let Ok(session) = &mut self.sim.session {
            session.reload_offer_desc(mission, &mut self.sim.chance);
        }
    }

    /// The mission info's text of `slot`, selected and loaded, as
    /// [`Session::show_info_row`] gives it on the flight's chance; none
    /// for a session that failed.
    pub fn show_info_row(&mut self, slot: u8) -> Option<String> {
        match &mut self.sim.session {
            Ok(session) => session.show_info_row(slot, &mut self.sim.chance),
            Err(_) => None,
        }
    }

    /// Accepts `mission` as [`Session::accept_mission`] does, on the
    /// flight's chance: the slot it went into. The HUD is redrawn, as the
    /// original's mission dialog does on closing
    /// ([`FlightView::redraw_status`]): every close of `_DoMissionDialog`
    /// that disposes the dialog, an accept with no text included, ends in
    /// `_RefreshSAT` (@0xa40c8-0xa40d7), which redraws the whole bar, and
    /// so does the offer dialog after its accept (@0xa2e44, then
    /// @0xa365c). The port's nav and cargo `_DrawStatus` after the
    /// computer's (@0x6003a-0x60046) adds nothing, so ours matches. The
    /// redraw comes after `OnAccept` runs (@0xa3bb4 → @0xa40c8), and a
    /// landed `M` or `N` in it writes the nav at its operator, so the
    /// redraw shows it.
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
    /// The [`AcceptError`] that applies; a session that failed offers
    /// nothing.
    pub fn accept_mission(&mut self, mission: MissionId) -> Result<u8, AcceptError>
    where
        C: StatusBars,
    {
        if !self.docked() {
            return self.answer_in_flight(|view| match &mut view.sim.session {
                Ok(session) => session.accept_mission(mission, &mut view.sim.chance),
                Err(_) => Err(AcceptError::NotOffered),
            });
        }
        self.redraw_before_answer(mission);
        self.run_docked(Then::Redraw, |view| match &mut view.sim.session {
            Ok(session) => session.accept_mission(mission, &mut view.sim.chance),
            Err(_) => Err(AcceptError::NotOffered),
        })
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
    fn answer_in_flight<T>(&mut self, answer: impl FnOnce(&mut Self) -> T) -> T
    where
        C: StatusBars,
    {
        self.redraw_status();
        let answered = answer(self);
        self.stage_bar(Vec::new(), Then::Redraw);
        answered
    }

    /// The offer dialog with a refuse button hides and redraws the whole
    /// bar before it answers (`_OfferOneMission` @0xa2df8 before the
    /// accept, @0xa2e93 before a refusal's `RefuseText`;
    /// [`Session::refusable_offer_dialog`]); the one-button dialog and the
    /// mission computer redraw nothing before theirs (@0xa2f52-0xa2fba,
    /// @0xa3b37-0xa3bb4). A refusal with no `RefuseText` skips the redraw
    /// (@0xa2e70-0xa2e78), but nothing shows before the dialog's own redraw
    /// after it, so ours redraws either way.
    fn redraw_before_answer(&mut self, mission: MissionId) {
        if self
            .sim
            .session
            .as_ref()
            .is_ok_and(|session| session.refusable_offer_dialog(mission))
        {
            self.redraw_status();
        }
    }

    /// Refuses `mission` as [`Session::refuse_mission`] does, on the
    /// flight's chance. The HUD is redrawn, as on accepting, after
    /// `OnRefuse` runs, so a landed `M` or `N` in it shows, as the offer
    /// dialog's `_RefreshSAT` (@0xa365c) redraws last.
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
    /// [`NotOffered`] when it is not offered; a session that failed offers
    /// nothing.
    pub fn refuse_mission(&mut self, mission: MissionId) -> Result<RefuseShown, NotOffered>
    where
        C: StatusBars,
    {
        if !self.docked() {
            return self.answer_in_flight(|view| match &mut view.sim.session {
                Ok(session) => session.refuse_mission(mission, &mut view.sim.chance),
                Err(_) => Err(NotOffered),
            });
        }
        self.redraw_before_answer(mission);
        self.run_docked(Then::Redraw, |view| match &mut view.sim.session {
            Ok(session) => session.refuse_mission(mission, &mut view.sim.chance),
            Err(_) => Err(NotOffered),
        })
    }

    /// The mission info screen's view, as [`Session::mission_info`] gives
    /// it; nothing listed for a session that failed.
    pub fn mission_info(&self) -> MissionInfo {
        match &self.sim.session {
            Ok(session) => session.mission_info(),
            Err(_) => MissionInfo {
                rows: Vec::new(),
                today: String::new(),
            },
        }
    }

    /// Aborts the mission in `slot` as [`Session::abort_mission`] does, on
    /// the flight's chance. Docked, an `OnAbort` `C`, `E` or `H` that
    /// changes the `ïntf` redraws the whole bar at its operator
    /// (`_AbortMission` @0x9e5e6 → @0x99ba1), and nothing else redraws
    /// while the mission info screen stays open: its close does
    /// (@0x9e769, [`FlightView::refresh_status`]).
    ///
    /// # Errors
    ///
    /// The [`AbortError`] that applies; a session that failed has none
    /// active.
    pub fn abort_mission(&mut self, slot: u8) -> Result<(), AbortError>
    where
        C: StatusBars,
    {
        self.run_docked(Then::Nothing, |view| match &mut view.sim.session {
            Ok(session) => session.abort_mission(slot, &mut view.sim.chance),
            Err(_) => Err(AbortError::NotActive),
        })
    }

    /// The next offer at venue `loc`, as [`Session::next_offer`] gives it,
    /// on the flight's chance; none for a session that failed. An offer
    /// accepted at once (@0xa230a) redraws as its accept's scripts and
    /// texts do ([`FlightView::stage_bar`]), with no dialog's redraw
    /// after them: `_OfferOneMission` returns at once (@0xa230f →
    /// @0xa368e).
    pub fn next_offer(&mut self, loc: i16) -> Option<VenueOffer>
    where
        C: StatusBars,
    {
        self.run_docked(Then::Nothing, |view| {
            view.sim
                .session
                .as_mut()
                .ok()?
                .next_offer(loc, &mut view.sim.chance)
        })
    }

    /// Enters venue `loc` for its offers, as
    /// [`Session::enter_offer_venue`] does.
    pub fn enter_offer_venue(&mut self, loc: i16) {
        if let Ok(session) = &mut self.sim.session {
            session.enter_offer_venue(loc);
        }
    }

    /// The wait before a venue's next offer, drawn on the flight's chance
    /// ([`offer_gap`]).
    pub fn offer_gap(&mut self) -> Duration {
        mac_ticks(offer_gap(&mut self.sim.chance))
    }

    /// The mission texts and offer movies to show since they were last
    /// taken ([`FlightEvent::MissionShown`](nova_sim::flight_frame::FlightEvent::MissionShown)); none for a
    /// session that failed.
    ///
    /// Docked, nothing is redrawn as they are taken: each text shows over
    /// the last bar drawn, and its close redraws the whole bar as it noted
    /// it ([`FlightView::close_mission_text`]), as the original's caller
    /// of each text runs `_RefreshSAT` once it returns. The runs of docked
    /// scripts stage those redraws as they queue the texts
    /// ([`FlightView::stage_bar`]), the outfitter's orders and the
    /// shipyard's purchases among them; a text queued anywhere else is
    /// staged here, with no `ïntf` change to order against it. So a
    /// landing's `CompText`,
    /// queued after the landing's redraw and before `OnSuccess` (@0xa048c,
    /// @0xa0491, then @0xa04f8), shows over the landing's bar, and its
    /// close draws the bar as it stood then, not where `OnSuccess`'s `M`
    /// selects, which shows at the port's next nav redraw.
    ///
    /// In flight, when there are any, the HUD is redrawn
    /// ([`FlightView::redraw_status`]): the original's text dialog redraws
    /// it on closing, and flight does not tick under the dialog, so
    /// redrawing as the texts are taken shows the same. The nav area is
    /// redrawn as the last text noted it (the inbox's `text_nav`) when that
    /// text was queued after the nav area was last taken: a nav mode other
    /// than the session's shows live anyway ([`FlightView::nav_shown`]),
    /// so this matters only while the take-off's held nav lasts, under
    /// which a text queued in the take-off's days shows
    /// ([`FlightView::took_off`]). The dialog has then shown the held nav,
    /// and its closing redraws the bar, so the flash ends at the next tick
    /// ([`NavFlash::UnderText`]). The texts of a person's offer answered
    /// over flight were staged as it was answered
    /// ([`FlightView::answer_in_flight`]), so nothing is redrawn for them:
    /// they show over the bar held from before the answer.
    pub fn take_mission_texts(&mut self) -> Vec<MissionShown>
    where
        C: StatusBars,
    {
        if self.docked() {
            self.stage_bar(Vec::new(), Then::Nothing);
            return self.docked_texts();
        }
        let FlightTexts {
            texts,
            staged,
            noted,
        } = self.flight_texts();
        if !texts.is_empty() && !staged {
            self.redraw_status();
            if let Some(noted) = noted {
                self.nav_scene = Some((noted, self.nav_display(noted)));
            }
            if self.nav_flash.is_some() {
                self.nav_flash = Some(NavFlash::UnderText);
            }
        }
        texts
    }

    /// Redraws the whole bar as `_RedrawStatus(1)` does at `load`'s
    /// operator (@0x739b6-0x739bd): the radar and the target panel as
    /// they are, the nav and the gauges as they stood there.
    fn redraw_at_load(&mut self, load: &InterfaceLoad)
    where
        C: StatusBars,
    {
        if self.hud_waits() {
            self.choose_bar(load.government);
        }
        self.redraw_whole();
        self.nav_scene = Some((load.nav, self.nav_display(load.nav)));
        self.reserves_scene = Some(load.reserves);
    }

    /// Whether the ship is docked: the bar is the port's, held between
    /// its redraws.
    pub(super) fn docked(&self) -> bool {
        self.sim
            .session
            .as_ref()
            .is_ok_and(|session| session.landed().is_some())
    }

    /// Stages the bar's redraws for a run of docked scripts, given the
    /// `ïntf` changes it made (`changes`, [`FlightView::interface_changes`])
    /// and the mission texts it queued that are not yet staged
    /// ([`FlightEvent::MissionShown`](nova_sim::flight_frame::FlightEvent::MissionShown)'s bars). Each text the original shows docked is
    /// modal and followed by its caller's `_RefreshSAT` (`_DoMissionAccept`
    /// @0xa1f13, @0xa1fba, @0xa1bb7; `_OfferOneMission` @0xa2f09;
    /// `_DoMissionSuccess` @0xa0491; `_DoMissionFailure` @0xa03d2;
    /// `_MissionObjectivesCheck` @0x9ed14; `_MissionLandCargoCheck`
    /// @0x9ef0c, @0x9effe, @0x9f13d), and `_ShowTextDialog` itself redraws
    /// only on a faded screen, which the port never is (@0x5a7c7,
    /// @0x5af9e, @0x5b45c). So:
    ///
    /// - the changes made before the first text redraw now, as at the last
    ///   of them ([`FlightView::redraw_at_load`]);
    /// - each text shows over the last bar drawn, and its close
    ///   ([`FlightView::close_mission_text`]) redraws the whole bar as the
    ///   text noted it, then at each change made after it and before the
    ///   next text;
    /// - `then`: what the dialog or order that ran the scripts draws once
    ///   its last text has closed, or at once with none ([`Then`]).
    ///
    /// Each operator's `_LoadIntfResource` (`_EvalSetExp` @0x156c7) asks
    /// for its ship's `ïntf` and compares it with the one loaded at that
    /// moment (`_currentIntf`, @0x73663-0x73672): the same returns at once,
    /// and another is loaded and redraws the whole bar (`_RedrawStatus(1)`
    /// @0x739b6-0x739bd), its graphic included. So two changes between two
    /// texts redraw twice, the last staying, and a change and a change
    /// back redraw though the `ïntf` after the script is the one before
    /// it. Each redraw draws the nav and the gauges as they stood at its
    /// operator: `M` and `N` mark nothing (@0x15705, @0x157e3, @0x158fd).
    /// Under [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s engine reading, the graphic waits
    /// with the scenes ([`FlightView::follow_bar`]).
    ///
    /// The offer dialog's after-movie plays over the last text's redraw,
    /// with no redraw of its own: `_PlayQTMovie` leaves the screen as
    /// unfaded as it found it (@0x55eb8, @0x55edb), so its caller's
    /// faded-only redraw never runs in port (@0xa3635-0xa3657).
    pub(super) fn stage_bar(&mut self, changes: Vec<InterfaceLoad>, then: Then)
    where
        C: StatusBars,
    {
        let bars = self.unstaged_bars();
        let first = bars.first().map_or(u64::MAX, |bar| bar.texts);
        let (now, later): (Vec<_>, Vec<_>) =
            changes.into_iter().partition(|load| load.texts < first);
        if let Some(load) = now.last() {
            self.redraw_at_load(load);
        }
        let Some(last) = bars.last() else {
            self.draw_then(then);
            return;
        };
        self.bars_staged = last.texts;
        for (i, bar) in bars.iter().enumerate() {
            let next = bars.get(i + 1).map_or(u64::MAX, |next| next.texts);
            self.bar_steps.push_back(BarStep {
                bar: *bar,
                loads: later
                    .iter()
                    .filter(|load| (bar.texts..next).contains(&load.texts))
                    .copied()
                    .collect(),
                then: if next == u64::MAX {
                    then
                } else {
                    Then::Nothing
                },
            });
        }
    }

    /// A mission text has closed, the first of those shown that had not:
    /// the whole bar is redrawn as the text noted it as it was queued
    /// ([`FlightEvent::MissionShown`](nova_sim::flight_frame::FlightEvent::MissionShown)'s bar), as the original's caller of
    /// `_ShowTextDialog` runs `_RefreshSAT` once it returns; then at each
    /// `C`, `E` or `H` run after it, before the next text, that changed
    /// the `ïntf`, as at its operator, the bar's graphic with it
    /// ([`FlightView::follow_bar`]); and, after the last text of a run,
    /// what the dialog or order that ran it draws then (see
    /// [`FlightView::stage_bar`]). Docked, or for a person's offer
    /// answered over flight ([`FlightView::answer_in_flight`]). Nothing
    /// for a text whose redraws were dropped by a whole-bar redraw since,
    /// or by leaving the dock, or for any other text shown over flight.
    pub fn close_mission_text(&mut self)
    where
        C: StatusBars,
    {
        let Some(step) = self.bar_steps.pop_front() else {
            return;
        };
        self.redraw_whole();
        self.nav_scene = Some((step.bar.nav, self.nav_display(step.bar.nav)));
        self.reserves_scene = Some(step.bar.reserves);
        for load in &step.loads {
            self.redraw_at_load(load);
        }
        self.draw_then(step.then);
        self.follow_bar();
    }

    /// Draws what a run of docked scripts' caller draws once they are
    /// done ([`Then`]).
    fn draw_then(&mut self, then: Then)
    where
        C: StatusBars,
    {
        match then {
            Then::Nothing => {}
            Then::Redraw => self.redraw_whole(),
            Then::Outfit { passed } => {
                // After a whole-bar redraw at an operator, both are what it
                // drew: it took the radar, and its nav consumed the
                // shipyard's mark, as `_RedrawStatus` marks and draws every
                // area.
                self.retake_radar();
                if passed {
                    self.draw_marked_nav();
                }
            }
            Then::Purchase { reserves } => {
                self.follow_bar();
                self.retake_radar();
                self.retake_nav();
                self.retake_target();
                if reserves {
                    self.retake_reserves();
                }
            }
        }
    }

    /// The shipyard of the stellar landed on, as [`Session::shipyard`]
    /// gives it, the day's rolls drawn on the flight's chance; none for a
    /// session that failed.
    pub fn shipyard(&mut self) -> Option<Shipyard> {
        self.sim
            .session
            .as_mut()
            .ok()?
            .shipyard(&mut self.sim.chance)
    }

    /// Declines to buy a ship of class `ship`, as [`Session::decline_ship`]
    /// does; nothing for a session that failed. The whole bar is redrawn
    /// as the name prompt closes on Cancel (`_RefreshSAT` @0x5eb73).
    pub fn decline_ship(&mut self, ship: ShipId) {
        self.redraw_status();
        if let Ok(session) = &mut self.sim.session {
            session.decline_ship(ship);
        }
    }
}

/// The mission computer's cut offer names are filled by the session, as
/// [`Session::offer_text`] fills them; a session that failed fills none.
impl<C> crate::spaceport::OfferNames for FlightView<C, crate::flight::SessionFlight<C>> {
    fn munged(&self, mission: MissionId, raw: &str) -> Option<String> {
        self.sim.session.as_ref().ok()?.offer_text(mission, raw)
    }
}

impl<C: nova_sim::CommCatalog> FlightView<C, crate::flight::SessionFlight<C>> {
    /// The prompt for naming a ship of class `ship` before it is bought,
    /// as [`Session::ship_naming`] gives it, its words read from the
    /// flight's catalog, on the flight's chance; a session that failed
    /// has no shipyard.
    pub fn ship_naming(&mut self, ship: ShipId) -> Result<ShipNaming, ShipRefusal> {
        let session = self
            .sim
            .session
            .as_mut()
            .map_err(|_| ShipRefusal::NoShipyard)?;
        session.ship_naming(ship, &*self.catalog, &mut self.sim.chance)
    }
}
