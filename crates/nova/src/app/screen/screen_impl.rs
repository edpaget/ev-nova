//! [`Screen`] for [`AppScreen`].

use std::time::Duration;

use nova_view::view_shape::classic_offset;
use nova_view::{
    Diagnostic, DrawList, FullscreenKind, Input, MenuCommand, Point, Prefs, Scaling, Screen,
    ScreenAction, Showing, Sound, UiSound, VideoMode, ViewShape,
};

use super::{
    AppScreen, CLASSIC_VIEW, ENTERED, HINT, HINT_AT, HINT_COLOR, HINT_SIZE, HINT_WITHOUT_DIALOGS,
    Held, LANDED, MENU_HINT, MENU_HINT_WITHOUT_DIALOGS, MenuHold, MenuKeys, Side,
};

impl Screen for AppScreen {
    /// A Tab press switches sides, first cancelling any pointer gesture on
    /// the side it hides and letting go of the keys it holds (their
    /// releases will now go to the other side). Holding Tab switches once:
    /// its key repeats are consumed without switching, as is its release.
    ///
    /// An Escape press goes back from an open system to the map, and
    /// otherwise quits; its repeats and release are consumed, so holding
    /// Escape goes back once and never quits. An F press enters flight,
    /// cancelling and letting go on the side it hides as Tab does; holding
    /// F enters once. Everything else, repeats included, goes to the side
    /// shown.
    ///
    /// In flight, an Escape press closes flight's map when it is open, and
    /// otherwise goes back to the side flight was entered from, letting go
    /// of the keys held in flight; it never quits, and its repeats and
    /// release are consumed. Everything else goes to flight, where Tab
    /// targets the next ship and F does nothing; I opens the mission info
    /// screen over flight, or, with no active mission listed, says "You
    /// have no active missions." on the message line.
    ///
    /// In the spaceport, every event goes to it; leaving it takes off,
    /// back into flight, letting go of its keys.
    ///
    /// M on a mission dialog (an offer, the mission computer, mission info
    /// or the accept's briefing) opens flight's course map over it, docked
    /// or over flight ([`FlightView::open_dialog_map`](nova_view::flight::FlightView::open_dialog_map)): every event
    /// goes to the map until it closes (Escape, M, Return or Done), and
    /// then to the dialog again.
    ///
    /// With dialogs, an I press outside flight opens the About dialog
    /// over the side shown, cancelling and letting go on it as Tab does.
    /// A P press, on any side (flight, its map and the spaceport
    /// included), opens the Preferences dialog the same way; P reaches the
    /// router before flight. Their repeats and releases are consumed.
    /// While either dialog is open, every event goes to it alone (Escape
    /// closes it and never quits).
    ///
    /// With a main menu ([`AppScreen::with_pilots`]), the router opens on
    /// it. Its New Pilot opens the New Pilot dialog and Open Pilot the
    /// saved pilots' list over it; while either is open, every event goes
    /// to it first, so typing a name never opens a dialog. A pilot created
    /// or opened is held on the menu, not flying; New Pilot over a pilot
    /// held asks `STR#` 140 #8 first, and a note over the menu (`STR#` 140
    /// #52 or #34) takes every event until Return, Escape or OK dismisses
    /// it. Enter Ship (E after its click, or a click) enters play with the
    /// pilot held, through the entry's fades and any intro (see `entry`),
    /// and refuses with snd 153 when none is held or it is dying. Escape
    /// in flight goes back to the menu, by the engine's reading of
    /// `escape_to_main_screen`, saving the pilot and holding it. Escape on
    /// the menu does nothing; Quit quits after
    /// [`QUIT_DELAY`](super::QUIT_DELAY) ([`Screen::take_quit`]), and its key first
    /// waits for its click, as every main-menu key does
    /// ([`Screen::keyed_click_ended`]): input meanwhile is held and
    /// replayed after, and input during the delay is dropped. I and P work
    /// on the menu, and Tab goes to the ship browser; on the ship browser
    /// and the galaxy map, Escape goes back to the menu instead of
    /// quitting. Closing the window on the menu asks whether to quit
    /// ([`Screen::close_requested`]); the question takes every event until
    /// it is answered.
    ///
    /// Command-Q quits at once, silently, on every side and under every
    /// overlay, but while the main menu appears, when it waits for the
    /// appear's end with the other keys it holds.
    ///
    /// While the main menu appears (at start-up, and on a return from
    /// play), every input goes to the appear first, which drops it, holds
    /// it for when it ends, or passes it on (see `appear`). While play is
    /// entered from the main menu, every input goes to the entry first,
    /// which drops it, holds it for the handover, or gives it to the
    /// intro's text; flight sees none from before its fade-in ends (see
    /// `entry`).
    ///
    /// Before anything else sees it, a Caps Lock press, on any side and
    /// under any overlay, turns double time on or off for flight, built or
    /// to be built ([`AppScreen::double_time_key`]); it then goes on as
    /// any other key does.
    ///
    /// After every input, what the set expressions it ran queued is
    /// settled ([`FlightView::settle_script`](nova_view::flight::FlightView::settle_script)), and then a pilot with a
    /// name is saved when flight says a save is due: on landing, on taking
    /// off, after a change in the spaceport (`AppScreen::transact`) and
    /// after a move by set operator.
    fn input(&mut self, input: &Input) -> ScreenAction {
        // The original is inside Quit's `Delay`, then exits.
        if self.quit_in.is_some() || self.quit_due {
            return ScreenAction::None;
        }
        if let Some(action) = self.entry_input(input) {
            return action;
        }
        if let Some(action) = self.appear_input(input) {
            return action;
        }
        if let Some(hold) = self.menu_hold {
            self.hold_input(hold, input);
            return ScreenAction::None;
        }
        self.routed_input(input)
    }

    /// Only the side shown ticks; a hidden one is paused. The main menu is
    /// first told where the pointer is ([`AppScreen::tell_menu_pointer`]).
    /// While an overlay
    /// (the About dialog, the Preferences dialog, the New Pilot dialog or
    /// the saved pilots' list) is open, only it ticks: flight pauses under
    /// the preferences, as in the original. What the offer dialog over
    /// flight asked for is acted on, and the mission dialogs over flight
    /// dropped once closed, on the tick a movie of theirs ends. While the
    /// main menu appears unseen it doesn't tick, but waits, and the
    /// appear moves on, replaying what it held once it ends. While play is
    /// entered, neither the menu nor flight ticks, and the entry moves on:
    /// its fades, the intro, the handover and its warm-up frame. A fade back
    /// from black after a death without a main menu moves on. Then what flight's tick ended is taken: the seam at
    /// the hold's expiry, and the return to the main menu once the death's
    /// fade is over. What the set expressions queued is settled, and a
    /// save that is due is made.
    fn tick(&mut self, dt: Duration) {
        if let Some(left) = self.quit_in {
            match left.checked_sub(dt) {
                Some(left) if !left.is_zero() => self.quit_in = Some(left),
                _ => {
                    self.quit_in = None;
                    self.quit_due = true;
                }
            }
        }
        if let Some(overlay) = self.overlay_mut() {
            overlay.tick(dt);
            // The spaceport isn't ticked under it, so says so: the
            // original's port window comes back to the front once the
            // overlay closes.
            if let Some(spaceport) = &mut self.spaceport {
                spaceport.cover();
            }
        } else if !self.appear_holds_menu() && !self.entry_holds() {
            self.tell_menu_pointer();
            self.shown_mut().tick(dt);
        }
        self.settle_offer();
        self.drop_closed_missions();
        self.advance_appear(dt);
        self.advance_entry(dt);
        self.advance_fade_in(dt);
        self.after_flight_tick();
        self.settle_script();
        self.save_if_due();
    }

    /// The side shown, then the hint (not over flight, which has its own
    /// help line, or the spaceport), then the overlay open: the New Pilot
    /// dialog, the saved pilots' list or the load's note, then the About
    /// dialog or the Preferences dialog; then the fade back from black
    /// after a death, and the main menu's appear, over everything. Under the spaceport, flight's status
    /// bar is drawn at the right edge ([`FlightView::draw_status_bar`](nova_view::flight::FlightView::draw_status_bar)), as
    /// the original's port dialog leaves it beside the dialog and its shops
    /// cover its left edge (`_DoPortDialog` @0x5fbca). The bar's course
    /// map is drawn over them both, as the original's map dialog opens
    /// over the port window (`_DoBarDialog` @0x492a3), and so is a mission
    /// dialog's; over flight, a mission dialog's map is drawn over the
    /// offer and the mission dialogs it opened from. Over a widened
    /// flight every dialog is drawn in its 1024 x 768 frame centred in the
    /// view ([`AppScreen::overlay_offset`]); flight places its own map.
    fn draw(&self, list: &mut DrawList) {
        if self.side == Side::Spaceport {
            self.flight.as_ref().expect(ENTERED).draw_status_bar(list);
            if self.port_map_open() {
                self.spaceport.as_ref().expect(LANDED).draw(list);
            }
        }
        // A mission dialog's map over flight is drawn over the dialogs it
        // opened from, after them.
        let map_over = self.flight_dialog_map_open();
        if !map_over {
            self.shown().draw(list);
        }
        let hint = match (self.side, self.dialogs.is_some()) {
            (Side::Ships | Side::Galaxy, true) => Some(HINT),
            (Side::Ships | Side::Galaxy, false) => Some(HINT_WITHOUT_DIALOGS),
            (Side::MainMenu, true) => Some(MENU_HINT),
            (Side::MainMenu, false) => Some(MENU_HINT_WITHOUT_DIALOGS),
            (Side::Flight | Side::Spaceport, _) => None,
        };
        if let Some(hint) = hint {
            list.text(hint, HINT_AT, HINT_SIZE, None, HINT_COLOR);
        }
        // The dialogs, in their 1024 x 768 frame, centred over a widened
        // flight.
        let offset = self.overlay_offset();
        let mut dialogs = DrawList::new();
        let under_map = [
            self.new_pilot.as_ref().map(|dialog| dialog as &dyn Screen),
            self.christening
                .as_ref()
                .map(|open| &open.dialog as &dyn Screen),
            self.open_pilot.as_ref().map(|pilots| pilots as &dyn Screen),
            self.load_note.as_ref().map(|note| note as &dyn Screen),
            self.question
                .as_ref()
                .map(|(_, question)| question as &dyn Screen),
            self.plunder.as_ref().map(|dialog| dialog as &dyn Screen),
            self.assignment.as_ref().map(|dialog| dialog as &dyn Screen),
            self.capture_naming
                .as_ref()
                .map(|dialog| dialog as &dyn Screen),
            self.comm.as_ref().map(|dialog| dialog as &dyn Screen),
            self.escort_dialog
                .as_ref()
                .map(|dialog| dialog as &dyn Screen),
            self.offer.as_ref().map(|dialog| dialog as &dyn Screen),
            self.player_info
                .as_ref()
                .map(|screen| screen as &dyn Screen),
            self.missions.as_ref().map(|dialogs| dialogs as &dyn Screen),
            self.haggle.as_ref().map(|dialog| dialog as &dyn Screen),
        ];
        for overlay in under_map.into_iter().flatten() {
            overlay.draw(&mut dialogs);
        }
        list.extend_translated(&dialogs, offset);
        if map_over {
            self.shown().draw(list);
        }
        let mut dialogs = DrawList::new();
        if let Some(about) = &self.about {
            about.draw(&mut dialogs);
        }
        if let Some(dialog) = &self.preferences {
            dialog.draw(&mut dialogs);
        }
        list.extend_translated(&dialogs, offset);
        if let Some(fade) = &self.fade_in {
            fade.draw(list, CLASSIC_VIEW);
        }
        if let Some(appear) = &self.appear {
            appear.draw(list, CLASSIC_VIEW);
        }
        self.draw_entry(list);
    }

    /// While the main menu appears, the cancel ends a return's wait for
    /// the button, and the menu under it holds nothing to cancel; while
    /// the main menu waits, the cancel waits too, in its place among the
    /// inputs.
    fn cancel_pointer(&mut self) {
        if self.entry_cancel_pointer() {
            return;
        }
        if self.appear_cancel_pointer() {
            return;
        }
        if self.menu_hold.is_some() {
            self.held.push(Held::CancelPointer);
            return;
        }
        match self.overlay_mut() {
            Some(overlay) => overlay.cancel_pointer(),
            None => self.shown_mut().cancel_pointer(),
        }
    }

    /// While the main menu appears unseen or waits, the letting go waits
    /// too, in its place among the inputs; while E's wait holds, it
    /// counts as E's release.
    fn release_keys(&mut self) {
        if self.entry_release_keys() {
            return;
        }
        if self.appear_release_keys() {
            return;
        }
        if let Some(hold) = self.menu_hold {
            self.held.push(Held::ReleaseKeys);
            if hold == MenuHold::EHeld {
                self.e_released();
            }
            return;
        }
        self.command_held = false;
        self.keys_held = MenuKeys::default();
        match self.overlay_mut() {
            Some(overlay) => overlay.release_keys(),
            None => self.shown_mut().release_keys(),
        }
    }

    /// The window went to the background: `_SwitchedOut` (@0x4f381) runs
    /// wherever the game is (its callers are `_DoMainScreen`,
    /// `_IterateInGameEventLoop` and `_MyModalDialog`). The audio stops
    /// the music if it plays (@0x4f390-0x4f3a0), and every flight held,
    /// shown or not, halts its warp-ups (@0x4f3a3-0x4f3e0). No other
    /// screen keeps a sound the switch-out stops.
    fn focus_lost(&mut self) {
        self.sounds.push(Sound::Ui(UiSound::SwitchedOut));
        for flight in self.flights_mut() {
            flight.focus_lost();
        }
    }

    /// Tells every flight held, shown or not, that the window came back
    /// to the front: the original's `_SwitchedIn` (@0x513cc-0x513dc)
    /// zeroes the frame-rate readout's counters wherever the game is.
    /// No other screen keeps time across the switch.
    fn focus_gained(&mut self) {
        for flight in self.flights_mut() {
            flight.focus_gained();
        }
    }

    /// Runs the main-menu action held for its keyed click, if any (E's
    /// wait is left alone), then replays what arrived meanwhile, in
    /// order, through [`Screen::input`]: a replayed key that holds again
    /// queues the rest. A quit a replayed input asks is kept for
    /// [`Screen::take_quit`]. When the action opens the New Pilot dialog,
    /// what it flushes ([`flushed_on_opening`]) is dropped first.
    fn keyed_click_ended(&mut self) {
        let Some(MenuHold::Click(action)) = self.menu_hold else {
            return;
        };
        self.menu_hold = None;
        let had_new_pilot = self.new_pilot.is_some();
        self.run_menu_action(action);
        let mut held = std::mem::take(&mut self.held);
        if !had_new_pilot && self.new_pilot.is_some() {
            held.retain(|held| !flushed_on_opening(held));
        }
        for held in held {
            match held {
                Held::Input(input) => {
                    if self.input(&input) == ScreenAction::Quit {
                        self.quit_due = true;
                    }
                }
                Held::CancelPointer => self.cancel_pointer(),
                Held::ReleaseKeys => self.release_keys(),
                Held::Close => self.replay_close(),
            }
        }
    }

    /// Whether Quit's delay has run out, a replayed input quit, or the
    /// quit question was answered yes.
    fn take_quit(&mut self) -> bool {
        std::mem::take(&mut self.quit_due)
    }

    /// On the main menu with nothing over it, the window's close asks
    /// `STR#` 140 #10 in the Yes/No dialog and keeps the window open;
    /// while the main menu waits on a keyed click, it waits too, keeping
    /// the window open; anywhere else it quits at once (see
    /// `quit_question`).
    fn close_requested(&mut self) -> bool {
        self.close_asked()
    }

    /// The sounds of the screens closed since, then of every live screen.
    fn take_sounds(&mut self) -> Vec<Sound> {
        let mut sounds = std::mem::take(&mut self.sounds);
        sounds.extend(self.take_entry_sounds());
        sounds.extend(self.ships.take_sounds());
        sounds.extend(self.galaxy.take_sounds());
        let open = [
            self.menu
                .as_mut()
                .map(|menu| &mut menu.screen as &mut dyn Screen),
            self.new_pilot
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.christening
                .as_mut()
                .map(|open| &mut open.dialog as &mut dyn Screen),
            self.open_pilot.as_mut().map(|list| list as &mut dyn Screen),
            self.load_note.as_mut().map(|note| note as &mut dyn Screen),
            self.question
                .as_mut()
                .map(|(_, question)| question as &mut dyn Screen),
            self.about.as_mut().map(|about| about as &mut dyn Screen),
            self.preferences
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.plunder
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.assignment
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.capture_naming
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.comm.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.escort_dialog
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.haggle.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.offer.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.missions
                .as_mut()
                .map(|dialogs| dialogs as &mut dyn Screen),
            self.spaceport.as_mut().map(|port| port as &mut dyn Screen),
            self.flight.as_mut().map(|flight| flight as &mut dyn Screen),
            self.game.as_mut().map(|game| game as &mut dyn Screen),
        ];
        for screen in open.into_iter().flatten() {
            sounds.extend(screen.take_sounds());
        }
        sounds
    }

    /// The preferences chosen in the Preferences dialog, once after each
    /// change.
    fn take_prefs(&mut self) -> Option<Prefs> {
        self.prefs_change.take()
    }

    /// Offers the open Preferences dialog the display modes, on the kind
    /// the game runs fullscreen in now and the kind chosen; with none
    /// open, nothing changes.
    fn offer_display(
        &mut self,
        modes: &[VideoMode],
        current: FullscreenKind,
        chosen: FullscreenKind,
    ) {
        self.offer_display_to_preferences(modes, current, chosen);
    }

    /// Keeps the Scaling list for every Preferences dialog, and takes
    /// `running` as the player's preferences' one, which each dialog opens
    /// on.
    fn offer_scaling(&mut self, listed: &[Scaling], running: Scaling) {
        self.scaling = listed.to_vec();
        self.prefs.scaling = running;
    }

    /// The fullscreen kind picked in the Preferences dialog, once after it
    /// closed on the pick.
    fn take_display_choice(&mut self) -> Option<FullscreenKind> {
        self.display_choice.take()
    }

    /// The flight's borders setting to save, once after an undocked close
    /// of its map that changed it
    /// ([`FlightView::take_map_borders_save`](nova_view::flight::FlightView::take_map_borders_save)).
    fn take_map_borders_save(&mut self) -> Option<bool> {
        self.flight
            .as_mut()
            .and_then(nova_view::flight::FlightView::take_map_borders_save)
    }

    fn now_showing(&self) -> Option<Showing> {
        Some(self.showing())
    }

    /// Flight's ([`FlightView`](nova_view::flight::FlightView)'s cursor), while it shows with nothing
    /// over it; and while a cinematic movie plays in front, over flight or
    /// the spaceport (`_HideTheCursor` @0x55a73), never under a dialog
    /// opened over it.
    fn hides_pointer(&self) -> bool {
        if let Some(hides) = self.entry_hides_pointer() {
            return hides;
        }
        match self.showing() {
            Showing::Movie => match &self.missions {
                Some(missions) => missions.hides_pointer(),
                None => self.offer.as_ref().is_some_and(Screen::hides_pointer),
            },
            Showing::Flight | Showing::Spaceport => self.shown().hides_pointer(),
            _ => false,
        }
    }

    /// Puts the pilot flying away, in flight or in the spaceport, when it
    /// has a name: saves it, or deletes a strict pilot dying
    /// ([`AppScreen::put_away_pilot`]). A game only held on the menu is
    /// put away by nothing: the original saves nothing on a quit from the
    /// main screen.
    fn quit(&mut self) {
        self.put_away_pilot();
    }

    /// The warnings since they were last taken: saves that failed, pilots
    /// that could not be listed or opened, and a New Pilot dialog missing
    /// from the interface file.
    fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// The diagnostics of the flight shown and of the game held, so a
    /// game's come out on the menu as it is built.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        self.flights_mut()
            .flat_map(Screen::take_diagnostics)
            .collect()
    }

    /// Widened while flight shows, with anything over it; classic on the
    /// main menu, the developer's sides and the spaceport, with anything
    /// over it (see the module docs).
    fn view_shape(&self) -> ViewShape {
        if self.side == Side::Flight {
            ViewShape::Widened
        } else {
            ViewShape::Classic
        }
    }

    /// Keeps `size` for every flight, and tells those built, shown or
    /// held.
    fn set_widened_size(&mut self, size: (f32, f32)) {
        self.widened = size;
        for flight in self.flights_mut() {
            flight.set_view_size(size);
        }
    }

    /// Quit and Preferences… as their Command chords, and About as I's
    /// About text where nothing holds input or is in front (see
    /// `menu_commands`).
    fn menu_command(&mut self, command: MenuCommand) -> ScreenAction {
        self.run_menu_command(command)
    }

    /// Quit always; Preferences… and About where the command would do
    /// something (see `menu_commands`).
    fn menu_enabled(&self, command: MenuCommand) -> bool {
        self.menu_enabled_now(command)
    }
}

impl AppScreen {
    /// `input` routed as the router routes it with nothing holding it:
    /// Caps Lock's double time, the route and the frame-rate readout
    /// kept ([`AppScreen::keep_frame_rate`]), then what the set
    /// expressions queued settled and a save that is due made.
    pub(super) fn routed_input(&mut self, input: &Input) -> ScreenAction {
        self.double_time_key(input);
        let action = self.route(input);
        self.keep_frame_rate();
        self.settle_script();
        self.save_if_due();
        action
    }

    /// How far right the dialogs over the side shown are drawn, their
    /// pointer moved back by as much: their 1024 x 768 frame centred over
    /// a widened flight, and nothing elsewhere.
    pub(super) fn overlay_offset(&self) -> Point {
        let x = if self.view_shape() == ViewShape::Widened {
            classic_offset(self.widened.0)
        } else {
            0.0
        };
        Point::new(x, 0.0)
    }
}

/// Whether the New Pilot dialog, opening as a keyed click ends, drops
/// `held`: `_DoNewPilotNameDialog` flushes every queued event but disk
/// events before its loop (`FlushEvents(0xff7f, 0)` @0x16807-0x16816), so
/// every input held through the click goes, but a key's release, kept so
/// its held state settles (the dialog acts on presses only). The pointer
/// cancels and key releases the router queued in their place still run,
/// and a window close is already dropped under a dialog
/// ([`AppScreen::replay_close`]).
pub(super) fn flushed_on_opening(held: &Held) -> bool {
    matches!(held, Held::Input(input) if !matches!(input, Input::Key { pressed: false, .. }))
}
