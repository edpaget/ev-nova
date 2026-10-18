//! [`AppScreen`]'s input routing: the spaceport's input, the `serve_*`
//! methods and the router itself.

use nova_sim::VenueOffer;
use nova_view::flight::FlightView;
use nova_view::spaceport::offer::OfferChoice;
use nova_view::{Input, Key, Point, Screen, ScreenAction, Showing};

use super::port_dialogs::{
    abort_and_refresh, close_mission_texts, close_port_dialogs, load_info_text, open_outfitter,
    open_port_dialogs, open_port_map, refresh_spaceport, refresh_status_bar,
};
use super::{AppScreen, ENTERED, LANDED, MenuAction, PREFERENCES_KEY, Side};

impl AppScreen {
    /// The spaceport's input, all of it: the outfitter opening tells the
    /// flight and shows its outfitter afresh; the bar opening has the
    /// flight draw the visit's news, which the bar's News then shows; an order on its exchange trades,
    /// and one in its outfitter buys or sells, and the exchange and the
    /// outfitter as they then are go back to it. A ship asked for in its
    /// shipyard opens the prompt for its name, when it can be bought; the
    /// ship named there is bought, and one declined there draws its roll
    /// again when the shipyard's list is next built, changing nothing
    /// else. Once it is left, the ship takes off
    /// and flight shows. Otherwise the input ends by redrawing the status
    /// bar when a closing the original ends in `_RefreshSAT` came
    /// (`refresh_status_bar`). Mission BBS and Hire Escort are answered
    /// with the port dialogs' openings (`open_port_dialogs`).
    ///
    /// The bar's map key opens flight's course map over the spaceport
    /// ([`FlightView::open_bar_map`](nova_view::flight::FlightView::open_bar_map)),
    /// and so does a mission dialog's, an offer's, the computer's, mission
    /// info's or the accept's briefing's
    /// ([`FlightView::open_dialog_map`](nova_view::flight::FlightView::open_dialog_map)). While it is open it takes every
    /// input: an Escape press closes it ([`FlightView::close_map`](nova_view::flight::FlightView::close_map)), as in
    /// flight, and anything else goes to it ([`FlightView::port_map_input`](nova_view::flight::FlightView::port_map_input),
    /// where M closes it too).
    fn spaceport_input(&mut self, input: &Input) -> ScreenAction {
        if self.port_map_input(input) {
            return ScreenAction::None;
        }
        let spaceport = self.spaceport.as_mut().expect(LANDED);
        spaceport.input(input);
        close_mission_texts(spaceport, self.flight.as_mut().expect(ENTERED));
        open_port_map(spaceport, self.flight.as_mut().expect(ENTERED));
        // A new opening clears what lasts one, and the outfitter it opened
        // on was the cached one: it shows the flight's afresh, or, listing
        // nothing, says so and never opens, the port's `_DrawStatus` after
        // it still drawing a marked nav.
        open_outfitter(spaceport, self.flight.as_mut().expect(ENTERED));
        let trade = spaceport.take_trade();
        let outfit = spaceport.take_outfit();
        let ship = spaceport.take_ship();
        let recharge = spaceport.take_recharge();
        let hire = spaceport.take_hire();
        let accepted = spaceport.take_mission_accept();
        open_port_dialogs(spaceport, self.flight.as_mut().expect(ENTERED));
        if spaceport.take_mission_info_request() {
            let info = self.flight.as_ref().expect(ENTERED).mission_info();
            spaceport.open_mission_info(info);
        }
        // An abort frees the slot, and the info goes back, closing once
        // nothing is left; a refusal (greyed) changes nothing.
        abort_and_refresh(spaceport, self.flight.as_mut());
        // The bar draws the visit's news as it opens, before News can
        // show it; News shows what was drawn.
        if spaceport.take_bar_opened() {
            self.flight.as_mut().expect(ENTERED).open_bar();
        }
        if spaceport.take_news_request() {
            let flight = self.flight.as_ref().expect(ENTERED);
            if let (Some(news), Some(picture)) = (flight.news(), flight.news_picture()) {
                spaceport.open_news(news, picture);
            }
        }
        if let Some(ship) = spaceport.take_ship_request()
            && let Ok(naming) = self.flight.as_mut().expect(ENTERED).ship_naming(ship)
        {
            spaceport.open_ship_naming(&naming);
        }
        // The list is kept for the visit, as the original keeps it; the
        // roll is drawn again when it is next built.
        if let Some(ship) = spaceport.take_declined_ship() {
            self.flight.as_mut().expect(ENTERED).decline_ship(ship);
        }
        if trade.is_some()
            || outfit.is_some()
            || ship.is_some()
            || recharge
            || hire.is_some()
            || accepted.is_some()
        {
            let flight = self.flight.as_mut().expect(ENTERED);
            // A refused order changes nothing; the screen greys what it can.
            if let Some(order) = trade {
                let _ = flight.trade(order);
            }
            // A plain click's count of 1 is exactly one order; the
            // quantity dialog's Cancel orders none, which still runs what
            // follows the original's loop once its button's check passed:
            // the buy's reset and either's status draw. A sale
            // refused in the sell loop shows its words over the
            // outfitter, at whatever pass it is refused; the sales before
            // it stand. Any other refusal changes nothing. Then the
            // outfitter follows what the order left its selection at: an
            // order that changed the listing resets it, or closes the
            // outfitter when nothing is listed.
            if let Some((order, count)) = outfit {
                let run = flight.outfit_order(order, count);
                if let Some(alert) = &run.alert {
                    spaceport.alert_outfit(alert);
                }
                spaceport.outfit_ordered(run.after);
            }
            if let Some(order) = ship {
                let _ = flight.buy_ship(order.ship, &order.name);
            }
            // A hire draws the class's roll again, so the list is built
            // afresh; the hire screen closes back to the bar.
            if let Some(ship) = hire {
                let _ = flight.hire(ship);
                if let Some(list) = flight.escorts_for_hire() {
                    spaceport.set_hire(&list);
                }
            }
            // An accept shows its texts, then the computer again, as the
            // original closes it and opens it again; one left with nothing
            // available closes silently.
            if let Some(mission) = accepted {
                let _ = flight.accept_mission(mission);
                spaceport.show_mission_texts(flight.take_mission_texts());
                spaceport.set_mission_computer(flight.open_mission_computer());
            }
            // A refused refill says why in the spaceport.
            if recharge {
                match flight.recharge() {
                    Ok(_) => spaceport.recharged(),
                    Err(refusal) => spaceport.refuse_recharge(refusal),
                }
            }
            // Each changes the cash; an outfit the cargo space and the
            // trade-in; and a ship the cargo space, the free mass and the
            // outfits: so all three go back.
            refresh_spaceport(spaceport, flight);
        }
        self.serve_closings();
        self.serve_mission_screens();
        self.serve_descriptions();
        let spaceport = self.spaceport.as_mut().expect(LANDED);
        if spaceport.left() {
            self.leave_port();
            return ScreenAction::None;
        }
        self.serve_offers();
        self.serve_player_info();
        if let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) {
            refresh_status_bar(spaceport, flight);
        }
        ScreenAction::None
    }

    /// The texts the spaceport's mission computer and mission info screen
    /// have shown selected, loaded through flight as the original loads a
    /// selected row's (`_MissionFilter` @0x9f9bd, `_MissionInfoFilter`
    /// @0x9d89f) and shown while still selected. The screens themselves
    /// open as they are asked for, earlier in the input
    /// (`open_port_dialogs`).
    fn serve_mission_screens(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        if let Some(mission) = spaceport.take_computer_text_request()
            && let Some(row) = flight.show_computer_row(mission)
        {
            spaceport.show_computer_row(&row);
        }
        load_info_text(spaceport, flight);
        if let Some(mission) = spaceport.take_offer_reload() {
            flight.reload_offer_desc(mission);
        }
    }

    /// The `dësc`s the spaceport's screens have shown, each loaded
    /// through flight as `_LoadDescription` loads one
    /// ([`FlightView::load_description`](nova_view::flight::FlightView::load_description)) and shown in its place, in the
    /// input that showed it, so no frame draws the raw read. One that
    /// cannot be read keeps the raw read's blank.
    pub(super) fn serve_descriptions(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        while let Some(id) = spaceport.take_description_request() {
            if let Ok(text) = flight.load_description(id) {
                spaceport.show_description(id, &text);
            }
        }
    }

    /// The mission info screen over flight's selected text, loaded
    /// through flight as [`AppScreen::serve_mission_screens`] loads the
    /// spaceport's.
    pub(super) fn load_flight_info_text(&mut self) {
        let (Some(missions), Some(flight)) = (&mut self.missions, &mut self.flight) else {
            return;
        };
        load_info_text(missions, flight);
    }

    /// The outfitter and the hire screen closing, however they closed this
    /// input (an outfitter's Done or Escape, an order that left nothing
    /// listed, or the input that closed that order's alert; a hire screen
    /// after its hire, on Cancel, or a bar with nothing for hire), each
    /// read once its order or hire went through: flight is told
    /// ([`FlightView::close_outfitter`](nova_view::flight::FlightView::close_outfitter), [`FlightView::close_hire`](nova_view::flight::FlightView::close_hire)), whose
    /// reset can trim the cargo, and the shops are handed back. Then a port
    /// dialog that closed redraws what its close draws and restores the
    /// nav saved as it opened (`close_port_dialogs`).
    fn serve_closings(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        let outfitter = spaceport.take_outfitter_closed();
        let hire = spaceport.take_hire_closed();
        if outfitter {
            flight.close_outfitter();
        }
        if hire {
            flight.close_hire();
        }
        if outfitter || hire {
            refresh_spaceport(spaceport, flight);
        }
        close_port_dialogs(spaceport, flight);
    }

    /// The spaceport's player info screen: asked for, it opens on what
    /// flight gives as the dialog opens ([`FlightView::open_player_info`](nova_view::flight::FlightView::open_player_info)),
    /// whose reset can trim the cargo, so the shops are handed back; its
    /// Jettison drops everything through flight, which changes the
    /// exchange, so the shops are handed back, and the session makes a
    /// save due.
    fn serve_player_info(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        if spaceport.take_player_info_request()
            && let Some(info) = flight.open_player_info()
        {
            spaceport.open_player_info(info);
            refresh_spaceport(spaceport, flight);
        }
        if spaceport.take_player_info_jettison() {
            flight.jettison_all();
            refresh_spaceport(spaceport, flight);
        }
    }

    /// The venues' mission offers (`_OfferMissionFromPort` @0xa369e): a
    /// venue's ask answered with the session's next offer there, which
    /// enters the venue, and a choice made through the session, each
    /// answer showing the texts it queued and rearming the venue with a
    /// gap drawn on the router's chance (the spaceport never asks again);
    /// an accept changes the cash, so the shops are handed back.
    pub(super) fn serve_offers(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        if spaceport.take_offer_restart() {
            spaceport.rearm_offers(flight.offer_gap());
        }
        if let Some(venue) = spaceport.take_offer_request() {
            match flight.next_offer(venue) {
                Some(VenueOffer::Prompt(prompt)) => spaceport.open_offer(prompt),
                Some(VenueOffer::Accepted { .. }) => {
                    spaceport.show_mission_texts(flight.take_mission_texts());
                    spaceport.rearm_offers(flight.offer_gap());
                    refresh_spaceport(spaceport, flight);
                }
                None => spaceport.rearm_offers(flight.offer_gap()),
            }
        }
        if let Some((mission, choice)) = spaceport.take_offer_choice() {
            match choice {
                OfferChoice::Accept => {
                    let _ = flight.accept_mission(mission);
                }
                OfferChoice::Refuse => {
                    // A `RefuseText` hides the dialog (@0xa2e8e); without
                    // one it stays drawn under what the refusal shows.
                    if flight
                        .refuse_mission(mission)
                        .is_ok_and(|shown| shown.offer_hidden)
                    {
                        spaceport.release_answered_offer();
                    }
                }
            }
            spaceport.show_mission_texts(flight.take_mission_texts());
            spaceport.rearm_offers(flight.offer_gap());
            refresh_spaceport(spaceport, flight);
        }
    }

    /// Whether `input` is a Preferences key, which the router takes, with
    /// dialogs to open them by: Command+comma on every side, as the
    /// original's app menu opens them (`_PrefAEVTHandler` @0x56181), and
    /// P on the main menu, the ship browser and the galaxy viewer, away
    /// from flight and the spaceport, where P is the player info key. A
    /// press opens them; its repeats and release do nothing.
    pub(super) fn preferences_key(&mut self, input: &Input) -> bool {
        let Input::Key {
            key,
            pressed,
            repeat,
        } = *input
        else {
            return false;
        };
        let away = matches!(self.side, Side::MainMenu | Side::Ships | Side::Galaxy);
        let taken =
            (key == PREFERENCES_KEY && self.command_held) || (key == Key::Char('p') && away);
        if !taken || self.dialogs.is_none() {
            return false;
        }
        if pressed && !repeat {
            // `_MainScreenKey`'s P (@0x1ac59) clicks first; a Command key
            // goes to the original's menus (`MenuKey`), which do not.
            // It opens once the click has played.
            if self.side == Side::MainMenu && key == Key::Char('p') && !self.command_held {
                self.click_then(MenuAction::Preferences);
            } else {
                self.open_preferences();
            }
        }
        true
    }

    /// Whether the spaceport takes every input, the Preferences keys
    /// included: while a ship's name is typed into its prompt, P and all;
    /// and while a movie plays in front, as `_PlayQTMovie`'s modal loop
    /// takes no menu events (`WaitNextEvent` with mask 0 @0x55e4e).
    fn spaceport_takes_every_key(&self) -> bool {
        let Some(spaceport) = &self.spaceport else {
            return false;
        };
        let naming = spaceport
            .open_shipyard()
            .is_some_and(|shipyard| shipyard.naming().is_some());
        naming || (self.side == Side::Spaceport && spaceport.playing())
    }

    /// Whether the side shown has a map whose Find is open, which takes
    /// every input, the Preferences keys and the side's own keys
    /// included, as `_DoSystemSearch`'s `ModalDialog` takes every event
    /// (@0x11dd1): flight's course map, the bar's map over the spaceport,
    /// or the galaxy viewer.
    fn map_takes_every_key(&self) -> bool {
        match self.side {
            // In port, flight's map is open only as the bar's or a mission
            // dialog's.
            Side::Flight | Side::Spaceport => {
                self.flight.as_ref().is_some_and(FlightView::map_finding)
            }
            Side::Galaxy => self.galaxy.system().is_none() && self.galaxy.map().finding(),
            Side::MainMenu | Side::Ships => false,
        }
    }

    /// The input to the overlay open in front, if any: what it did, or
    /// `None` with none open. A mission dialog's map over flight is in
    /// front of the dialogs it opened from, and places itself; every other
    /// overlay takes the pointer moved back by
    /// [`AppScreen::overlay_offset`], as it is drawn moved on by it.
    fn overlay_input(&mut self, input: &Input) -> Option<ScreenAction> {
        let offset = self.overlay_offset();
        // The frame moves only across.
        let moved = &input.translated(Point::new(-offset.x, 0.0));
        if self.about.is_some() {
            return Some(self.about_input(moved));
        }
        if self.preferences.is_some() {
            return Some(self.preferences_input(moved));
        }
        if self.player_info.is_some() {
            return Some(self.player_info_input(moved));
        }
        if self.haggle.is_some() {
            return Some(self.haggle_input(moved));
        }
        if self.comm.is_some() {
            return Some(self.comm_input(moved));
        }
        if self.flight_dialog_map_open() {
            return Some(self.flight_dialog_map_input(input));
        }
        if self.missions.is_some() {
            return Some(self.missions_input(moved));
        }
        if self.offer.is_some() {
            return Some(self.offer_input(moved));
        }
        if self.plunder.is_some() {
            return Some(self.plunder_input(moved));
        }
        if self.assignment.is_some() {
            return Some(self.assignment_input(moved));
        }
        if self.new_pilot.is_some() {
            return Some(self.new_pilot_input(moved));
        }
        if self.open_pilot.is_some() {
            return Some(self.open_pilot_input(moved));
        }
        if self.load_note.is_some() {
            return Some(self.load_note_input(moved));
        }
        None
    }

    /// Routes one input, as [`Screen::input`] describes.
    pub(super) fn route(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key { key, pressed, .. } = *input {
            match key {
                Key::Command => self.command_held = pressed,
                Key::Char('s') => self.keys_held.s = pressed,
                Key::Char('l') => self.keys_held.l = pressed,
                Key::Char('e') => self.keys_held.e = pressed,
                _ => {}
            }
        }
        if let Some(action) = self.overlay_input(input) {
            return action;
        }
        if self.spaceport_takes_every_key() {
            return self.spaceport_input(input);
        }
        if self.map_takes_every_key() {
            return match self.side {
                Side::Flight => self.flight_input(input),
                Side::Spaceport => self.spaceport_input(input),
                _ => self.galaxy.input(input),
            };
        }
        if self.preferences_key(input) {
            return ScreenAction::None;
        }
        match self.side {
            Side::Flight => return self.flight_input(input),
            Side::Spaceport => return self.spaceport_input(input),
            Side::MainMenu | Side::Ships | Side::Galaxy => {}
        }
        if let Input::Key {
            key: Key::Char('i'),
            pressed,
            repeat,
        } = *input
            && self.dialogs.is_some()
        {
            if pressed && !repeat {
                self.open_about();
            }
            return ScreenAction::None;
        }
        if self.side == Side::MainMenu {
            return self.menu_input(input);
        }
        if let Input::Key {
            key: key @ (Key::Tab | Key::Escape | Key::Char('f')),
            pressed,
            repeat,
        } = *input
        {
            if !pressed || repeat {
                return ScreenAction::None;
            }
            match key {
                Key::Escape if self.showing() == Showing::System => {
                    return self.galaxy.input(input);
                }
                Key::Escape if self.menu.is_some() => self.switch_to(Side::MainMenu),
                Key::Escape => return ScreenAction::Quit,
                Key::Tab if self.side == Side::Ships => self.switch_to(Side::Galaxy),
                Key::Tab => self.switch_to(Side::Ships),
                _ => self.enter_flight(),
            }
            return ScreenAction::None;
        }
        self.shown_mut().input(input)
    }
}
