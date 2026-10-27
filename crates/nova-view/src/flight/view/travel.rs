//! [`FlightView`]'s travel: fades and death, the course map, the jump and
//! its steps, arrival, take-off, landing, the shipyard and outfitter
//! orders.

use nova_sim::PlayerCommand;
use nova_sim::SessionEvent;
use nova_sim::gate::next_gate_pick;
use nova_sim::{
    Assigned, Assignment, AsteroidCatalog, CombatCatalog, CommCatalog, OutfitOrder, OutfitRefusal,
    OutfitRun, PilotCatalog, Session, ShipId, ShipPurchase, ShipRefusal, StellarId, TrafficCatalog,
};

use nova_sim::flight_frame::{Answer, FlightCommand, MapClick};

use crate::devtools::SessionDesk;
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::EscortMenuLooks;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::{GalaxyCatalog, MapCycle, MapMode, PendingPress, PressAnswer};
use crate::system::catalog::SystemCatalog;
use crate::view_shape::classic_offset;
use crate::{DrawList, Input, Key, Point, Screen, UiSound};

use super::dialog_map::{MapOpened, MapOpening};
use super::{
    BOARD_KEY, FlightView, JUMP_KEY, MAP_KEY, Redrawn, Then, assigned_message, assigned_ticks,
    no_outfitter,
};

impl<
    C: PilotCatalog
        + TrafficCatalog
        + CombatCatalog
        + AsteroidCatalog
        + CommCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks,
> FlightView<C, crate::flight::SessionFlight<C>>
{
    /// Whether the pilot may be saved now, as [`Session::may_save`] says:
    /// not while the player is dying or dead, nor without a session.
    #[must_use]
    pub fn may_save(&self) -> bool {
        self.sim.session.as_ref().is_ok_and(Session::may_save)
    }

    /// Whether the pilot is dying, as `_IsDying` (@0x253d) says: its ship
    /// breaking up or destroyed, which [`FlightView::may_save`] answers.
    /// Enter Ship refuses such a pilot. Never without a session.
    #[must_use]
    pub fn is_dying(&self) -> bool {
        self.sim.session.is_ok() && !self.may_save()
    }

    /// Whether ending play now deletes the pilot, as
    /// [`Session::deletes_pilot_on_ending_play`] says: a strict-play pilot
    /// dying outside the escape pod. Never without a session.
    #[must_use]
    pub fn deletes_pilot_on_ending_play(&self) -> bool {
        self.sim
            .session
            .as_ref()
            .is_ok_and(Session::deletes_pilot_on_ending_play)
    }

    /// The input to the map opened over a screen
    /// ([`FlightView::port_map_input_with`]), the flight's messages worded
    /// by the session ([`FlightView::WORDS`]).
    pub fn port_map_input(&mut self, input: &Input) {
        self.port_map_input_with(input, Self::WORDS);
    }

    /// Closes the course map ([`FlightView::close_map_with`]), the
    /// flight's messages worded by the session ([`FlightView::WORDS`]).
    pub fn close_map(&mut self) {
        self.close_map_with(Self::WORDS);
    }

    /// The keys `_HandlePlayer` polls with a latch, once a frame, with
    /// those held down, in its order: Option-\` and the Nav Off /
    /// Hyperspace Mode block ([`FlightView::nav_keys_frame`]), Hyper
    /// Select ([`FlightView::select_frame`]), the stellar slot keys
    /// ([`FlightView::slot_keys_frame`]), the Board key
    /// ([`FlightView::board_frame`]), then the jump key
    /// ([`FlightView::jump_frame`]), each while the session takes its
    /// command. A key held across a jump's end, a release of Option or a
    /// repair acts here, as a press would, on the first frame its latch
    /// lets it.
    ///
    /// It runs before the tick's steps, as `_HandlePlayer` runs before the
    /// ships' `_HandleShip` (@0x43e19, @0x44483): a repair made in one
    /// tick's steps is acted on at the next, the frame the original sees
    /// it. And it runs before the jump effect's arrival, which is where
    /// the original's arrival frame runs its keys (+0x50 still above 0):
    /// the arrival tick still counts as jumping, and the first tick after
    /// it is the original's first frame to act (@0x6c625).
    pub(super) fn held_keys_frame(&mut self) {
        let down = |key: Key| self.held.contains(&key);
        let (jump, board) = (down(JUMP_KEY), down(BOARD_KEY));
        self.nav_held_frame();
        if self.takes(PlayerCommand::Board) {
            self.board_frame(board);
        }
        // The jump's frame asks its own gate (`FlightCommand::gate`).
        self.jump_frame(jump);
    }

    /// The pilot flying, as the developer tools' desk: its edits reach
    /// the session, moves read the catalog, and the course map's galaxy
    /// names the places. `None` when the session failed.
    pub fn pilot_desk(&mut self) -> Option<SessionDesk<'_, C>> {
        let session = self.sim.session.as_mut().ok()?;
        Some(SessionDesk::new(session, &*self.catalog, self.map.model()))
    }

    /// Takes off from the stellar landed on ([`FlightView::take_off_with`]),
    /// the flight's messages worded by the session
    /// ([`FlightView::WORDS`]), and gives it; `None` when the ship has not
    /// landed.
    pub fn take_off(&mut self) -> Option<StellarId> {
        self.take_off_with(Self::WORDS)
    }

    /// Takes off for a held `Q`'s leave, if due
    /// ([`FlightView::leave_if_due_with`]), the flight's messages worded by
    /// the session ([`FlightView::WORDS`]).
    pub fn leave_if_due(&mut self) -> Option<StellarId> {
        self.leave_if_due_with(Self::WORDS)
    }

    /// Settles what the set expressions run since queued
    /// ([`FlightView::settle_script_with`]), the flight's messages worded
    /// by the session ([`FlightView::WORDS`]).
    pub fn settle_script(&mut self) {
        self.settle_script_with(Self::WORDS);
    }

    /// Buys a ship named `name` as [`Session::buy_ship`] does, it and its
    /// hooks drawing on the flight's chance, and catches up with the
    /// change of ship it tells ([`SessionEvent::ShipChanged`]): the sprite
    /// sheet of the class the session then flies (the one bought, or the
    /// one its hooks changed it to) is read, once, so the new hull is
    /// drawn once it takes off; a refused purchase tells nothing and
    /// reads nothing, and a session that failed has no shipyard.
    ///
    /// A purchase redraws the radar, the nav area and the target panel,
    /// as the original's does inside the purchase (`_DrawStatus`
    /// @0x5f0b1, after marking nav, target, weapons and cargo,
    /// `_statusRedraw[0]`, `[6]`, `[3]` and `[2]`, @0x5f03a-0x5f04a), so a ship whose default outfits carry IFF
    /// shows it at once, and the nav area shows what the ship's
    /// `OnPurchase` (@0x5f00d) wrote, a landed `M` or `N`'s selection,
    /// whatever its operators' order, as it runs first.
    ///
    /// The gauges (`[1]`, `[4]`) are not marked. A `C`, `E` or `H` in the old
    /// class's `OnRetire` (@0x5ecff) or the new one's `OnPurchase`
    /// (@0x5f00d) that puts the ship under another `ïntf` redraws the
    /// whole bar at that operator, so the gauges show as they stood
    /// there, before the purchase's fill (@0x5f051-0x5f06b). Then the
    /// purchase's own `_LoadIntfResource` (@0x5f0ac) loads the class's
    /// bar and redraws the gauges only when the ship's `ïntf` is still
    /// another than the one loaded (`hud::interface_for`), as its
    /// whole-bar `_RedrawStatus` (@0x739bd) runs only for an ID other than
    /// the one loaded (`_currentIntf`, @0x73663-0x73672); otherwise they
    /// wait for the shipyard to close ([`FlightView::refresh_status`]).
    ///
    /// An `S` in either hook shows its mission's texts inside the
    /// purchase (`_EvalSetExp` @0x153ff → `_DoMissionAccept` @0x15407), so
    /// the hooks' redraws are ordered against them
    /// ([`FlightView::stage_bar`]): those before the first text redraw at
    /// once, each text shows over the last bar drawn and its close
    /// redraws as the text noted it, then at the hooks' operators after
    /// it; and the purchase's own load and draw come only after the last
    /// text has closed ([`Then::Purchase`]). So under the text the old
    /// class's bar shows, with the gauges from before the purchase.
    ///
    /// Before all that, the whole bar is redrawn as the name prompt
    /// closes on OK (`_RefreshSAT` @0x5eb73, right after
    /// `_EVTextInputDialog` returns @0x5eb6c and before the purchase
    /// @0x5ebc0), whatever the purchase then does. The app checks every
    /// refusal before the prompt opens ([`FlightView::ship_naming`]), so
    /// no refusal follows it there.
    pub fn buy_ship(&mut self, ship: ShipId, name: &str) -> Result<ShipPurchase, ShipRefusal> {
        self.redraw_status();
        let before = self.interface_now();
        let session = self
            .sim
            .session
            .as_mut()
            .map_err(|_| ShipRefusal::NoShipyard)?;
        let bought = session.buy_ship(ship, name, &mut self.sim.chance)?;
        let after = self.interface_now();
        let (changes, loaded) = self.interface_changes(before.as_ref());
        self.stage_bar(
            changes,
            Then::Purchase {
                reserves: after != loaded,
            },
        );
        self.catch_up();
        Ok(bought)
    }

    /// Buys or sells an outfit as [`Session::outfit`] does, it and its
    /// hook drawing on the flight's chance; a session that failed has no
    /// outfitter. The bar is redrawn after it
    /// (`FlightView::after_outfit_order`).
    pub fn outfit(&mut self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        self.outfit_counted(order, 1).map(|_| ())
    }

    /// Buys or sells up to `count` of an outfit as
    /// [`Session::outfit_counted`] does, on the flight's chance, giving
    /// how many went through; a session that failed has no outfitter.
    /// The bar is redrawn after it (`FlightView::after_outfit_order`).
    pub fn outfit_counted(&mut self, order: OutfitOrder, count: u32) -> Result<u32, OutfitRefusal> {
        let before = self.interface_now();
        let run = match &mut self.sim.session {
            Ok(session) => session.outfit_counted_run(order, count, &mut self.sim.chance),
            Err(_) => no_outfitter(),
        };
        self.after_outfit_order(before.as_ref(), run.button_passed);
        run.run.counted()
    }

    /// Buys or sells up to `count` of an outfit as
    /// [`Session::outfit_order`] does, on the flight's chance, giving the
    /// whole run and its alert; a session that failed has no outfitter,
    /// refused at the first pass with no alert. The bar is redrawn after
    /// it (`FlightView::after_outfit_order`).
    pub fn outfit_order(&mut self, order: OutfitOrder, count: u32) -> OutfitRun {
        let before = self.interface_now();
        let run = match &mut self.sim.session {
            Ok(session) => session.outfit_order(order, count, &mut self.sim.chance),
            Err(_) => no_outfitter(),
        };
        self.after_outfit_order(before.as_ref(), run.button_passed);
        run
    }

    /// Redraws the bar after an outfitter order, given the `ïntf` the
    /// ship flew under before it ([`FlightView::interface_now`]) and
    /// whether it passed its button's own check (`passed`, the run's
    /// [`OutfitRun::button_passed`]).
    ///
    /// The order's `OnPurchase` (`_EvalMissionBitSetString` @0x5c2cb) or
    /// `OnSell` (@0x5cff6) runs inside it. When its `C`, `E` or `H`
    /// (`_EvalSetExp` @0x15493-0x156c7) puts the ship under another
    /// `ïntf`, `_LoadIntfResource` (@0x156c7) redraws the whole bar at
    /// once (`_RedrawStatus(1)` @0x739bd), held gauges, nav and target
    /// included ([`FlightView::redraw_status`]). Otherwise only the radar
    /// is redrawn ([`FlightView::retake_radar`]). A refused order changes
    /// no ship, so it redraws only the radar. The redraw runs inside the
    /// order's call, before any frame is drawn.
    ///
    /// The redraw is at the `C`, `E` or `H` itself, as the original's is
    /// ([`FlightView::stage_bar`]): the nav and gauges
    /// show as they stood at the last operator that changed the `ïntf`,
    /// so an `M` or `N` after it in the same script (@0x156c7 → @0x739bd,
    /// then @0x156f2-0x15705) shows only at the port's next nav redraw,
    /// and one before it shows at once. An `ïntf` changed and changed
    /// back within one order still redraws, though the one after the
    /// order is the one before it.
    ///
    /// An `S` in the hook shows its mission's texts inside the order
    /// (`_EvalSetExp` @0x153ff → `_DoMissionAccept` @0x15407, each text
    /// closing with `_RefreshSAT` @0xa1f13), so the hook's redraws are
    /// ordered against them as every docked script's are
    /// ([`FlightView::stage_bar`]): the `C`, `E` and `H` before the first
    /// text redraw at once, each text shows over the last bar drawn and
    /// its close redraws the bar as the text noted it, then at the
    /// operators after it; and the order's own draw (@0x5c5a1, @0x5d328,
    /// after `_SystemInfoToShipStats` and `_SetupPortAvailableItems`)
    /// comes only after the last text has closed ([`Then::Outfit`]). That
    /// text's close has drawn the nav, letting go of the shipyard's mark,
    /// so the order's draw then redraws only the radar. No stock outfit
    /// reaches it: `oütf` 363's `S731` starts a mission with
    /// no `BriefText`.
    ///
    /// The buy's and the sell's own `_DrawStatus` (@0x5c5a1, @0x5d328)
    /// draw the nav too when the shipyard's restore marked it
    /// ([`FlightView::close_port_dialog`]), but only for an order that
    /// went through: a buy `_CanBuyOutfitItem` refuses (@0x5c087,
    /// @0x5c09f-0x5c0a2) jumps past the buy's draw to @0x5c5ca, and a
    /// sell with nothing to sell (the flag at 0xded1c, @0x5c5d5-0x5c5dd)
    /// past the sell's to @0x5d351. A run stopped after its first pass
    /// still reaches them, and so does a Cancel's count of 0 once the
    /// button's check passed, as the loop then runs no pass and falls
    /// through to the draw (`_DoQuantityDialog` @0x5c1aa, @0x5c2fc-0x5c2ff).
    fn after_outfit_order(&mut self, before: Option<&Result<i16, String>>, passed: bool) {
        let (changes, _) = self.interface_changes(before);
        self.stage_bar(changes, Then::Outfit { passed });
    }

    /// Assigns the ship captured, as [`Session::assign`] does, and says
    /// what it did; after "Use As My Ship" it is drawn where it is, not on
    /// its way from the old ship, and the change of ship the session tells
    /// is caught up with ([`SessionEvent::ShipChanged`]), reading the new
    /// ship's sprite sheet, once. `None` when no capture awaits its
    /// assignment.
    pub fn assign(&mut self, choice: Assignment) -> Option<Assigned> {
        let session = self.sim.session.as_mut().ok()?;
        let assigned = session.assign(choice, &mut self.sim.chance)?;
        if assigned == Assigned::MyShip {
            // Where the new ship is, read from the frame filled afresh
            // after the assignment, before the catch-up that follows it.
            self.previous = self.player_place_now();
            self.alpha = 0.0;
        }
        self.catch_up();
        self.say_for(assigned_message(assigned), assigned_ticks(assigned));
        Some(assigned)
    }
}

/// The course map, over the [`FlightSim`] port: its opening, drawing,
/// panels, input and Clear Route read the flight through its frame
/// ([`FlightView::frame_now`]) and [`MapSource`](crate::flight::sim_port::MapSource),
/// and send their commands through it. Its close stays concrete
/// ([`FlightView::close_map`]), as the hypergate entry it may run is still
/// the session's: [`FlightView::map_input_taken`] says when it is due.
impl<C, S: FlightSim> FlightView<C, S> {
    /// The course map's close, the hypergate's entry left to
    /// `enter_gate`, handed the system picked on the hypergate map
    /// ([`FlightView::close_map_with`]): the borders' save, the map shut,
    /// then the course map's close, docked or in flight, or the hypergate
    /// map's redraw before `enter_gate` runs.
    ///
    /// Closing the course map in flight cues the course's next hop
    /// ([`FlightCommand::CueCourse`]), as the original's map does unless
    /// it is the hypergate's (`_DoSystemMap` @0x13c16), and beeps
    /// [`UiSound::MapClosed`], as `_HandlePlayer` does once
    /// `_DoSystemMap` returns (@0x6a85a).
    ///
    /// Docked, its close is `_DoSystemMap`'s in port: the whole bar is
    /// redrawn (`_RefreshSAT` @0x13bc9), showing the nav as the map left
    /// it, and the course is not cued (@0x13be2-0x13c0a skips
    /// `_CueNextHyperRouteDest` in port). Then the selection the map
    /// opened with is put back, the mode kept, with no redraw
    /// ([`FlightCommand::RestoreMapSelection`], `_DoSystemMap` @0x12349,
    /// @0x13bee-0x13c06), undoing a Clear Route. It beeps nothing: the
    /// [`UiSound::MapClosed`] beep is `_HandlePlayer`'s, around the map M
    /// opens in flight. A mission dialog's map beeps in flight neither.
    ///
    /// Then whoever opened it puts back what it saved: when the bar opened
    /// it ([`FlightView::open_bar_map`]), the nav the bar opened with
    /// ([`FlightCommand::RestoreBarNav`], `_DoBarDialog` @0x492b7-0x492d3),
    /// with no redraw, as `_UpdateAllPortDialogs` (@0x492d7) draws no
    /// status, so the nav area shows the map's selection until the port's
    /// next nav redraw, such as the bar's close; a port screen its own
    /// (`FlightView::open_port_screen_map`), with no redraw either: the
    /// exchange and the outfitter the nav they opened with, and the
    /// spaceport, the shipyard, the hire screen and the ship description
    /// the nav as the map opened; a mission dialog its own
    /// (`FlightView::open_dialog_map`): the offer's, the computer's and
    /// the briefing's the nav as the map opened, docked or in flight, the
    /// computer's then redrawing the nav area, and mission info's nothing.
    pub(super) fn close_map_then(
        &mut self,
        enter_gate: impl FnOnce(&mut Self, Option<nova_sim::SystemId>),
    ) {
        // `_SaveMapBordersState` for a change, whatever the map's mode,
        // unless docked (@0x13c21-0x13c3f).
        let docked = self.frame_now().docked();
        if !docked && self.map.borders_changed_since_open() {
            self.map_borders_save = Some(self.map.borders_shown());
        }
        self.map.cancel_pointer();
        self.map.release_keys();
        self.map.close_find();
        self.map_open = false;
        self.map_key_held = false;
        let opened = self.map_opened.take();
        if self.map.mode() != MapMode::Hypergate {
            if docked {
                self.redraw_status();
                if let Some(nav) = opened.and_then(|opened| opened.nav) {
                    self.send(FlightCommand::RestoreMapSelection(nav));
                }
                self.restore_opener_nav(opened);
                return;
            }
            let by_dialog =
                opened.is_some_and(|opened| matches!(opened.by, MapOpening::Dialog(..)));
            if !by_dialog {
                self.ui_sounds.push(UiSound::MapClosed);
            }
            self.send(FlightCommand::CueCourse);
            self.redraw_status();
            self.restore_opener_nav(opened);
            return;
        }
        self.redraw_status();
        let choice = self.map.take_gate_choice();
        enter_gate(self, choice);
    }

    /// Catches the screen up with what the flight has told since this
    /// was last called ([`FlightEvent::Session`]), and gives what it
    /// redrew. Several events of a kind ask for one reaction, decided by
    /// the events alone and never by what the flight was last asked to
    /// do: a change to the galaxy draws the flight's galaxy on the course
    /// map with the course as it is flown ([`FlightView::catch_up_then`]);
    /// a change of ship reads the player's sprite sheet afresh
    /// (`FlightView::follow_ship`); a system entered afresh is laid out
    /// (`FlightView::lay_out`); otherwise a move to another stellar of
    /// the system shown draws the ship there ([`FlightView::place_ship`]);
    /// and a rebuild point that changed which stellars are shown, with no
    /// system entered, reads the scene again with them
    /// ([`FlightView::redraw_scene_then`], with the status bar's follow
    /// and redraw, [`FlightView::follow_bar`] and
    /// [`FlightView::redraw_status`]).
    ///
    /// The only place the session events are taken: each frame calls it
    /// first, so a change made outside the view's own methods (a
    /// developer tools' edit, a set expression a control bit runs) is
    /// drawn by the next frame; and each of the view's methods that drives
    /// the flight between frames calls it last, so its caller sees the
    /// screen caught up at once. Taking the events again finds none. A
    /// flight that failed drains nothing, and its frame is the default.
    ///
    /// [`FlightEvent::Session`]: nova_sim::flight_frame::FlightEvent::Session
    pub(super) fn catch_up(&mut self) -> Redrawn
    where
        C: SystemCatalog + ShipSprites + CombatLooks + StatusBars,
    {
        self.catch_up_then(|view, redrawn| {
            view.redraw_scene_then(redrawn, Self::follow_bar, Self::redraw_status);
        })
    }

    /// Catches the screen up with an edit made through the pilot desk now,
    /// rather than on the next frame, from the flight's events (see the
    /// module docs): a move to another system is laid out as an arrival
    /// is, with no message, and one to another stellar of the system shown
    /// draws the ship there, the course map showing the flight's course
    /// again. Nothing is read when nothing moved.
    pub fn resync(&mut self)
    where
        C: SystemCatalog + ShipSprites + CombatLooks + StatusBars,
    {
        self.catch_up();
    }

    /// [`FlightView::catch_up`]'s work over the port, with `redraw` the
    /// redraws that still read the session ([`FlightView::redraw_scene_then`]
    /// for a session's flight): drains the flight's session events
    /// ([`FlightEvent::Session`]) and folds them into what they ask for,
    /// several of a kind asking for one reaction. A change to the galaxy
    /// draws the flight's galaxy on the course map
    /// ([`MapSource::galaxy`](crate::flight::sim_port::MapSource::galaxy),
    /// [`GalaxyMap::show_galaxy`](crate::galaxy::GalaxyMap::show_galaxy))
    /// with the course as it is flown, before anything else is drawn, only
    /// reads coming between. Then `redraw` runs, and last the open map's
    /// drawing follows the galaxy, the explored and known sets, the course
    /// and the nav ([`FlightView::redraw_map`]): the flight is paused while
    /// it is open, so this is rare, a set expression or the developer
    /// tools. Gives what was redrawn.
    ///
    /// [`FlightEvent::Session`]: nova_sim::flight_frame::FlightEvent::Session
    pub(super) fn catch_up_then(&mut self, redraw: impl FnOnce(&mut Self, Redrawn)) -> Redrawn {
        let mut redrawn = Redrawn::default();
        let events = self.drain().take_events();
        // The kept frame afresh before anything redraws from it.
        self.refresh_frame();
        for event in events {
            match event {
                SessionEvent::ShipChanged => redrawn.ship = true,
                SessionEvent::SystemChanged => redrawn.system = true,
                SessionEvent::StellarChanged => redrawn.stellar = true,
                SessionEvent::GalaxyChanged => redrawn.galaxy = true,
                SessionEvent::StellarsRebuilt => redrawn.rebuilt = true,
                SessionEvent::StarfieldReset => redrawn.stars = true,
            }
        }
        if redrawn.galaxy {
            if let Some(galaxy) = self.sim.galaxy() {
                self.map.show_galaxy(galaxy);
            }
            self.show_course();
        }
        redraw(self, redrawn);
        // And again after the redraws, which may have changed it.
        self.refresh_frame();
        if self.map_open && (redrawn.galaxy || self.map_seen.as_ref() != Some(&self.frame.map_key))
        {
            self.redraw_map();
        }
        redrawn
    }

    /// Shows the course as it is flown on the course map, from the
    /// current system ([`FlightFrame::course_shown`](nova_sim::flight_frame::FlightFrame::course_shown)).
    pub(super) fn show_course(&mut self) {
        let frame = self.frame_now();
        self.map.show_course(frame.system, &frame.course_shown);
    }

    /// The input `input` as the course map takes it: in its own
    /// 1024 x 768 frame, which a widened view over flight draws centred,
    /// moved only across ([`FlightView::map_offset`]).
    pub(super) fn map_frame_input(&self, input: &Input) -> Input {
        let offset = self.map_offset();
        input.translated(Point::new(-offset.x, 0.0))
    }

    /// [`FlightView::map_input`]'s work, with `close` the map's close
    /// ([`FlightView::close_map`] for a session's flight), run at once
    /// when the input closes the map ([`FlightView::map_input_taken`]).
    /// Then a click, a Tab cycle or a gate pick that changed the
    /// selection takes the panels again ([`FlightView::retake_map_panel`]),
    /// as the original's next `_MapDialogUpdate` draws them for its
    /// `_selSystem`. After an input that closed the map the panels are
    /// taken for nothing: the map isn't drawn until it opens again, which
    /// takes them.
    pub(super) fn map_input_then(&mut self, input: &Input, close: impl FnOnce(&mut Self)) {
        let selected = self.map.selected();
        let input = self.map_frame_input(input);
        if self.map_input_taken(&input) {
            close(self);
        }
        if self.map.selected() != selected {
            self.retake_map_panel();
        }
    }

    /// The open course map, drawn into `list`: its 1024 x 768 frame moved
    /// right by [`FlightView::map_offset`].
    pub(super) fn draw_open_map(&self, list: &mut DrawList) {
        let mut map = DrawList::new();
        self.map.draw(&mut map);
        list.extend_translated(&map, self.map_offset());
    }

    /// How far right the course map's 1024 x 768 frame is drawn, and its
    /// clicks taken: centred in the widened view over flight
    /// ([`classic_offset`]), and at 0 over the spaceport, which is
    /// classic, where the bar and the docked mission dialogs open it.
    pub(super) fn map_offset(&self) -> Point {
        let in_port = self.frame_now().docked();
        let x = if in_port {
            0.0
        } else {
            classic_offset(self.view.0)
        };
        Point::new(x, 0.0)
    }

    /// Lets go of the flight keys and shows the course map, with the
    /// system the original's `_DoSystemMap` selects as it opens
    /// ([`Session::map_preselection`]) selected, and the system selected
    /// in nav mode 3 drawn as its link. The hypergate map opens this way
    /// too, its pick still only an offered system.
    ///
    /// Clear Route is enabled off the hypergate map with a course
    /// (`showClearRouteButton`, @0x12396-0x123c3), and never on it.
    pub(super) fn open_map(&mut self) {
        self.open_map_as(MapOpening::Plain);
    }

    /// [`FlightView::open_map`], opened `by` whoever opens it (flight, the
    /// bar, a port screen or a mission dialog), which gives the map its
    /// marks ([`FlightView::open_dialog_map`]) and its close's restore, and
    /// the nav noted as it opens (`_DoSystemMap` @0x12349).
    pub(super) fn open_map_as(&mut self, by: MapOpening) {
        self.map_opened = Some(MapOpened {
            by,
            nav: self.opening_nav(),
        });
        self.held.clear();
        self.map_open = true;
        if let Some(sources) = self.sim.map_border_sources() {
            self.map.show_border_sources(sources);
        }
        self.map.opened();
        self.map.preselect(self.sim.map_preselection());
        self.map_missions = self.sim.map_mission_systems();
        self.map.set_clear_route(
            self.map.mode() != MapMode::Hypergate && self.frame_now().course_plotted,
        );
        self.redraw_map();
    }

    /// Takes the session's drawing onto the course map
    /// ([`Session::map_drawing`]), as `_MapDialogUpdate` runs `_DrawMap`:
    /// on the hypergate map with the gate's offers arrowed, each of which
    /// that draw marks known ([`Session::mark_known`], `_DrawMap`
    /// @0xf457-0xf45f). So the first draw of the hypergate map shows an
    /// offer the player knew nothing of without its oval, and the next one,
    /// at a Tab, a click or the course map opened again before the next
    /// rebuild point, shows it, as the original's next `_MapDialogUpdate`
    /// does. What the drawing was taken from is noted after the mark
    /// ([`MapKey`](nova_sim::flight_frame::MapKey)), so the mark itself
    /// doesn't take it again. The
    /// panels are taken with it ([`FlightView::retake_map_panel`]).
    pub(super) fn redraw_map(&mut self) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered().to_vec());
        let marks = MapOpened::marks(self.map_opened.as_ref());
        let Some(drawing) = self
            .sim
            .map_drawing(offered.as_deref(), &self.map_missions, marks)
        else {
            return;
        };
        self.map.show_drawing(&drawing);
        if offered.is_some() {
            self.send(FlightCommand::MarkKnown(drawing.gate_arrows.clone()));
        }
        if let Some(sources) = self.sim.map_border_sources() {
            self.map.show_border_sources(sources);
        }
        self.map_seen = Some(self.frame_now().map_key);
        self.retake_map_panel();
    }

    /// Takes the session's info panels onto the course map
    /// ([`Session::map_panel`]) for the system it selects, as
    /// `_MapDialogUpdate` draws them after `_DrawMap`: on the hypergate
    /// map with the gate's offers, which title a destination.
    pub(super) fn retake_map_panel(&mut self) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered());
        if let Some(panel) = self.sim.map_panel(self.map.selected(), offered) {
            self.map.show_panel(panel);
        }
    }

    /// Opens the course map on a press of [`MAP_KEY`] in flight, held from
    /// then on. `_HandlePlayer`'s `_beepSnd[2]` (@0x6a809) is the key's,
    /// not the map's: the hypergate's map opens without it.
    pub(super) fn open_map_by_key(&mut self) {
        self.ui_sounds.push(UiSound::MapOpened);
        self.open_map();
        self.map_key_held = true;
    }

    /// Opens the course map from the bar, as `_DoBarDialog`'s item 8 does
    /// (@0x492a3-0x492b2), over the spaceport: in
    /// [`MapMode::Course`](crate::galaxy::MapMode::Course), as the bar
    /// clears the hypergate flag first (@0x492aa-0x492af). Ours' map is
    /// in that mode whenever it is closed, as the hypergate map's close
    /// takes its pick ([`GalaxyMap::take_gate_choice`](crate::galaxy::GalaxyMap::take_gate_choice)). Its close is
    /// [`FlightView::close_map`]'s in port.
    pub fn open_bar_map(&mut self) {
        self.open_map_as(MapOpening::Bar);
    }

    /// [`FlightView::map_input`]'s input, translated into the map's frame,
    /// before the panels follow the selection: whether it closes the map,
    /// which the caller then does ([`FlightView::close_map`]), at once.
    /// A key-down the filter doesn't take with [`MAP_KEY`] held closes it
    /// before anything else; Return or Done after the rest.
    pub(super) fn map_input_taken(&mut self, input: &Input) -> bool {
        if let Input::Key {
            key: MAP_KEY,
            pressed,
            ..
        } = *input
        {
            self.map_key_held = pressed;
        }
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && self.map_key_held
            && !self.map.finding()
            && !self.map_filter_takes(key)
        {
            return true;
        }
        self.map.input(input);
        self.ui_sounds.extend(self.map.take_ui_sounds());
        if let Some(query) = self.map.take_find()
            && let Some(found) = self.sim.find_map_system(&query)
        {
            self.map.show_search(found);
            self.ui_sounds.extend(self.map.take_ui_sounds());
        }
        if let Some(press) = self.map.take_press() {
            self.answer_map_press(&press);
        }
        if self.map.take_clear_route() {
            self.clear_route();
        }
        if let Some(cycle) = self.map.take_cycle() {
            self.cycle_map(cycle);
        }
        self.map.take_close()
    }

    /// Answers a press on the map that hit systems
    /// ([`GalaxyMap::take_press`](crate::galaxy::GalaxyMap::take_press)) as
    /// `_DoSystemMap` takes item 3 ([`Session::map_click`]): the hypergate
    /// map passes the systems the gate offers, and every map the mission
    /// systems taken as it opened. With no session, or no hit accepted,
    /// the map drags. An accepted press draws the map again, as the
    /// original invalidates its items 2, 3 and 6 (@0x1315c-0x13259), so the
    /// hypergate map's offers show known; the course changed is shown, and
    /// the nav area is redrawn at once when the click wrote it
    /// (`_DrawStatus`), as [`FlightView::cycle_map`] does.
    fn answer_map_press(&mut self, press: &PendingPress) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered().to_vec());
        let answer = self.send(FlightCommand::MapClick(MapClick {
            hits: press.hits.clone(),
            shift: press.shift,
            hypergate: offered,
            missions: self.map_missions.clone(),
            green: MapOpened::marks(self.map_opened.as_ref()).green,
        }));
        let clicked = match answer {
            Answer::Clicked(clicked) => Some(clicked),
            _ => None,
        };
        self.map.answer_press(clicked.map(|clicked| PressAnswer {
            selected: clicked.selected,
            gate_pick: clicked.gate_pick,
            clear_route: clicked.clear_route,
        }));
        let Some(clicked) = clicked else {
            return;
        };
        if clicked.course_changed {
            self.show_course();
        }
        self.redraw_map();
        if clicked.nav_redraw {
            self.retake_nav();
        }
    }

    /// Clear Route, from its button or C (`_DoSystemMap` item 8,
    /// @0x12c69-0x12dbf): the session's course cleared and its selection
    /// let go ([`Session::clear_route`]), Clear Route disabled and the
    /// hypergate map's pick dropped
    /// ([`GalaxyMap::route_cleared`](crate::galaxy::GalaxyMap::route_cleared)),
    /// the map drawn again and the nav area redrawn at once (`_DrawStatus`
    /// @0x12dba).
    fn clear_route(&mut self) {
        self.map.route_cleared();
        if !self.send(FlightCommand::ClearRoute).is_refused() {
            self.show_course();
        }
        self.redraw_map();
        self.retake_nav();
    }

    /// Whether `_MapFilter` takes `key` before its map-key close, which a
    /// key it takes never reaches (@0x13cdd-0x144d6): Return, Enter,
    /// Escape, Tab and `\` (off a planet, @0x13d20), Space, B, C, `-`,
    /// `=` and `+` (the keypad's too), the arrows, and F and S. The keys
    /// later phases port are taken already, so none of them closes the
    /// map. A modifier is taken too: a Mac sends no key-down for one
    /// alone, so the filter never sees it.
    fn map_filter_takes(&self, key: Key) -> bool {
        let docked = self.frame_now().docked();
        match key {
            Key::Shift
            | Key::Control
            | Key::Alt
            | Key::Command
            | Key::CapsLock
            | Key::Enter
            | Key::Escape
            | Key::Space
            | Key::Left
            | Key::Right
            | Key::Up
            | Key::Down => true,
            Key::Tab => !docked,
            Key::Char(c) => match c.to_ascii_lowercase() {
                '\\' => !docked,
                'b' | 'c' | '-' | '=' | '+' | 'f' | 's' => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// A Tab or `\` on the map ([`MapCycle`]), as `_MapFilter` takes it
    /// off a planet (@0x13d0c-0x140d8). On the hypergate map, the map's
    /// selection steps to the next of the pending hypergate's
    /// destinations ([`Session::hypergate_cycle`],
    /// [`next_gate_pick`](nova_sim::gate::next_gate_pick)), back with
    /// Shift, held as it is so a hidden one steps on too; no nav mode is
    /// written and the nav area isn't redrawn, as the original's mode 4 is
    /// never drawn (@0x13dd8-0x13ebd). A pick redraws the map's item 3
    /// ([`GalaxyMap::redraw`](crate::galaxy::GalaxyMap::redraw),
    /// @0x13e6f); with no destination to pick nothing is invalidated
    /// (`jle` @0x13daf). On the course map, the session cycles the
    /// hyperspace selection ([`Session::cycle_map_selection`]); the map
    /// then draws the new selected link and selects the system the
    /// original's `_selSystem` names (@0x1408c-0x140d8,
    /// [`Session::map_preselection`]), item 3 is redrawn whether a link
    /// was picked or none (@0x14032), and the nav area is redrawn at
    /// once, as the filter runs `_DrawStatus` with nav marked
    /// (`_statusRedraw[0]`, @0x13fb2-0x1406e). Docked, as on the bar's
    /// map, nothing changes.
    fn cycle_map(&mut self, cycle: MapCycle) {
        if self.map.mode() == MapMode::Hypergate {
            let destinations = self.sim.hypergate_cycle();
            if let Some(pick) = next_gate_pick(&destinations, self.map.selected(), cycle.back) {
                self.map.select_gate(pick);
                self.map.redraw();
            }
            self.redraw_map();
            return;
        }
        if !self.send(FlightCommand::CycleMapSelection).is_refused() {
            self.map.preselect(self.sim.map_preselection());
            self.map.redraw();
            self.redraw_map();
            self.retake_nav();
        }
    }
}
