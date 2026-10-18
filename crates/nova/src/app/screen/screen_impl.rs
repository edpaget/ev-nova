//! [`Screen`] for [`AppScreen`].

use std::time::Duration;

use nova_view::{Diagnostic, DrawList, Input, Prefs, Screen, ScreenAction, Showing, Sound};

use super::{
    AppScreen, ENTERED, HINT, HINT_AT, HINT_COLOR, HINT_SIZE, HINT_WITHOUT_DIALOGS, Held, LANDED,
    MENU_HINT, MENU_HINT_WITHOUT_DIALOGS, MenuHold, MenuKeys, Side,
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
    /// to it first, so typing a name never opens a dialog. Choosing a
    /// pilot flies it; one whose load fell back first shows its note over
    /// the menu, which takes every event until Return, Escape or OK
    /// dismisses it and flight shows. Escape in flight goes back to the menu, saving
    /// the pilot and putting it away. Escape quits at once; Quit quits
    /// after [`QUIT_DELAY`](super::QUIT_DELAY) ([`Screen::take_quit`]), and its key first
    /// waits for its click, as every main-menu key does
    /// ([`Screen::keyed_click_ended`]): input meanwhile is held and
    /// replayed after, and input during the delay is dropped. I and P work
    /// on the menu, and Tab goes to the ship browser; on the ship browser
    /// and the galaxy map, Escape goes back to the menu instead of
    /// quitting.
    ///
    /// Before anything else sees it, a Caps Lock press, on any side and
    /// under any overlay, turns double time on or off for flight, built or
    /// to be built ([`AppScreen::double_time_key`]); it then goes on as
    /// any other key does.
    ///
    /// After every input, what the set expressions it ran queued is
    /// settled ([`FlightView::settle_script`](nova_view::flight::FlightView::settle_script)), and then a pilot with a
    /// name is saved when flight says a save is due: on landing, on taking
    /// off, after a change in the spaceport ([`AppScreen::transact`]) and
    /// after a move by set operator.
    fn input(&mut self, input: &Input) -> ScreenAction {
        // The original is inside Quit's `Delay`, then exits.
        if self.quit_in.is_some() || self.quit_due {
            return ScreenAction::None;
        }
        if let Some(hold) = self.menu_hold {
            self.hold_input(hold, input);
            return ScreenAction::None;
        }
        self.double_time_key(input);
        let action = self.route(input);
        self.settle_script();
        self.save_if_due();
        action
    }

    /// Only the side shown ticks; a hidden one is paused. While an overlay
    /// (the About dialog, the Preferences dialog, the New Pilot dialog or
    /// the saved pilots' list) is open, only it ticks: flight pauses under
    /// the preferences, as in the original. What the offer dialog over
    /// flight asked for is acted on, and the mission dialogs over flight
    /// dropped once closed, on the tick a movie of theirs ends. A fade back from black after a
    /// death moves on. Then what flight's tick ended is taken: the seam at
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
        match self.overlay_mut() {
            Some(overlay) => {
                overlay.tick(dt);
                // The spaceport isn't ticked under it, so says so: the
                // original's port window comes back to the front once the
                // overlay closes.
                if let Some(spaceport) = &mut self.spaceport {
                    spaceport.cover();
                }
            }
            None => self.shown_mut().tick(dt),
        }
        self.settle_offer();
        self.drop_closed_missions();
        self.advance_fade_in(dt);
        self.after_flight_tick();
        self.settle_script();
        self.save_if_due();
    }

    /// The side shown, then the hint (not over flight, which has its own
    /// help line, or the spaceport), then the overlay open: the New Pilot
    /// dialog, the saved pilots' list or the load's note, then the About
    /// dialog or the Preferences dialog; then the fade back from black
    /// after a death, over everything. Under the spaceport, flight's status
    /// bar is drawn at the right edge ([`FlightView::draw_status_bar`](nova_view::flight::FlightView::draw_status_bar)), as
    /// the original's port dialog leaves it beside the dialog and its shops
    /// cover its left edge (`_DoPortDialog` @0x5fbca). The bar's course
    /// map is drawn over them both, as the original's map dialog opens
    /// over the port window (`_DoBarDialog` @0x492a3), and so is a mission
    /// dialog's; over flight, a mission dialog's map is drawn over the
    /// offer and the mission dialogs it opened from.
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
        if let Some(dialog) = &self.new_pilot {
            dialog.draw(list);
        }
        if let Some(pilots) = &self.open_pilot {
            pilots.draw(list);
        }
        if let Some(note) = &self.load_note {
            note.draw(list);
        }
        if let Some(dialog) = &self.plunder {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.assignment {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.comm {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.offer {
            dialog.draw(list);
        }
        if let Some(screen) = &self.player_info {
            screen.draw(list);
        }
        if let Some(dialogs) = &self.missions {
            dialogs.draw(list);
        }
        if let Some(dialog) = &self.haggle {
            dialog.draw(list);
        }
        if map_over {
            self.shown().draw(list);
        }
        if let Some(about) = &self.about {
            about.draw(list);
        }
        if let Some(dialog) = &self.preferences {
            dialog.draw(list);
        }
        if let Some(fade) = &self.fade_in {
            fade.draw(list);
        }
    }

    /// While the main menu waits, the cancel waits too, in its place
    /// among the inputs.
    fn cancel_pointer(&mut self) {
        if self.menu_hold.is_some() {
            self.held.push(Held::CancelPointer);
            return;
        }
        match self.overlay_mut() {
            Some(overlay) => overlay.cancel_pointer(),
            None => self.shown_mut().cancel_pointer(),
        }
    }

    /// While the main menu waits, the letting go waits too, in its place
    /// among the inputs; while E's wait holds, it counts as E's release.
    fn release_keys(&mut self) {
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

    /// Runs the main-menu action held for its keyed click, if any (E's
    /// wait is left alone), then replays what arrived meanwhile, in
    /// order, through [`Screen::input`]: a replayed key that holds again
    /// queues the rest. A quit a replayed input asks is kept for
    /// [`Screen::take_quit`].
    fn keyed_click_ended(&mut self) {
        let Some(MenuHold::Click(action)) = self.menu_hold else {
            return;
        };
        self.menu_hold = None;
        self.run_menu_action(action);
        for held in std::mem::take(&mut self.held) {
            match held {
                Held::Input(input) => {
                    if self.input(&input) == ScreenAction::Quit {
                        self.quit_due = true;
                    }
                }
                Held::CancelPointer => self.cancel_pointer(),
                Held::ReleaseKeys => self.release_keys(),
            }
        }
    }

    /// Whether Quit's delay has run out, or a replayed input quit.
    fn take_quit(&mut self) -> bool {
        std::mem::take(&mut self.quit_due)
    }

    /// The sounds of the screens closed since, then of every live screen.
    fn take_sounds(&mut self) -> Vec<Sound> {
        let mut sounds = std::mem::take(&mut self.sounds);
        sounds.extend(self.ships.take_sounds());
        sounds.extend(self.galaxy.take_sounds());
        let open = [
            self.menu
                .as_mut()
                .map(|menu| &mut menu.screen as &mut dyn Screen),
            self.new_pilot
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.open_pilot.as_mut().map(|list| list as &mut dyn Screen),
            self.load_note.as_mut().map(|note| note as &mut dyn Screen),
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
            self.comm.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.haggle.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.offer.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.missions
                .as_mut()
                .map(|dialogs| dialogs as &mut dyn Screen),
            self.spaceport.as_mut().map(|port| port as &mut dyn Screen),
            self.flight.as_mut().map(|flight| flight as &mut dyn Screen),
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
    /// ([`AppScreen::put_away_pilot`]).
    fn quit(&mut self) {
        self.put_away_pilot();
    }

    /// The warnings since they were last taken: saves that failed, pilots
    /// that could not be listed or opened, and a New Pilot dialog missing
    /// from the interface file.
    fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// The flight's diagnostics, once flight has been entered.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        self.flight
            .as_mut()
            .map(Screen::take_diagnostics)
            .unwrap_or_default()
    }
}
