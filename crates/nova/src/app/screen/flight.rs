//! [`AppScreen`]'s flight: showing the spaceport, building, entering and
//! ending flight, the fades, saving the pilot and leaving port.

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_sim::flight_frame::LoadedDesc;
use nova_sim::{PilotKeeper, PilotStore, RuleKey, pilot_key};
use nova_view::fade::{DEATH_FADE_FOR, ScreenFade};
use nova_view::flight::FlightView;
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::bar::{BAR_DIALOG, BarTemplates, PICTURE_BAR_DIALOG};
use nova_view::spaceport::layout::SPACEPORT_DIALOG;
use nova_view::spaceport::missions::{COMPUTER_DIALOG, INFO_DIALOG};
use nova_view::spaceport::news::NEWS_DIALOG;
use nova_view::spaceport::offer::{OFFER_DIALOG, OfferTemplates, PICTURE_OFFER_DIALOG};
use nova_view::spaceport::outfitter::OUTFIT_DIALOG;
use nova_view::spaceport::race::RACE_DIALOG;
use nova_view::spaceport::shipyard::{
    InfoTemplates, SHIP_INFO_DIALOG, SHIP_PICTURE_INFO_DIALOG, SHIPYARD_DIALOG,
};
use nova_view::spaceport::trade::TRADE_DIALOG;
use nova_view::spaceport::{ListLooks, OutfitterCatalog, ShipyardCatalog};
use nova_view::ui::catalog::DescriptionGraphics;
use nova_view::ui::desc::{DESC_DIALOG, DescTemplates, PICTURE_DESC_DIALOG};
use nova_view::ui::mission_dialogs::{DescShown, MissionDialogs};
use nova_view::ui::player_info::{BackgroundSizes, PLAYER_INFO_DIALOG, YES_NO_DIALOG};
use nova_view::ui::quantity::QUANTITY_DIALOG;
use nova_view::ui::text_input::TEXT_INPUT_DIALOG;
use nova_view::ui::{DescriptionSource, DialogResources};

use super::port_dialogs::{close_mission_texts, close_port_dialogs};
use super::{AppScreen, NO_INTERFACE, Side};

impl AppScreen {
    /// Shows the spaceport of `stellar`, landed on, laid out by the
    /// "Spaceport" dialog when the router has dialogs, with the session's
    /// exchange and outfitter when it has them, its ambient sound's waits
    /// drawn on the router's chance. One already open, for another
    /// stellar the pilot desk moved to, is dropped first
    /// ([`AppScreen::drop_spaceport`]).
    pub(super) fn show_spaceport(&mut self, stellar: nova_sim::StellarId) {
        let template = |id| match &self.dialogs {
            Some(dialogs) => dialogs
                .resources
                .dialog_template(id)
                .map(|template| (template, Rc::clone(&dialogs.metrics))),
            None => Err(NO_INTERFACE.to_owned()),
        };
        let mut spaceport =
            SpaceportView::new(self.data.as_ref(), stellar, template(SPACEPORT_DIALOG));
        // The exchange, the outfitter, the shipyard and the room to hire,
        // read together as the spaceport opens.
        let shops = self
            .flight
            .as_mut()
            .map(FlightView::shops)
            .unwrap_or_default();
        if let Some(market) = shops.market {
            let trade = template(TRADE_DIALOG).map(|(template, _)| template);
            let quantity = template(QUANTITY_DIALOG).map(|(template, _)| template);
            let colors = self.data.list_colors();
            spaceport = spaceport.with_trade(trade, quantity, market, colors);
        }
        if let Some(outfitter) = shops.outfitter {
            let outfit = template(OUTFIT_DIALOG).map(|(template, _)| template);
            let quantity = template(QUANTITY_DIALOG).map(|(template, _)| template);
            let art: Rc<dyn OutfitterCatalog> = Rc::clone(&self.data) as Rc<dyn OutfitterCatalog>;
            let alert = template(DESC_DIALOG).map(|(template, _)| template);
            spaceport = spaceport.with_outfitter(outfit, quantity, alert, outfitter, art);
        }
        let template_of = |id| template(id).map(|(template, _)| template);
        let info_templates = || InfoTemplates {
            plain: template_of(SHIP_INFO_DIALOG),
            picture: template_of(SHIP_PICTURE_INFO_DIALOG),
        };
        if let Some(shipyard) = shops.shipyard {
            let art: Rc<dyn ShipyardCatalog> = Rc::clone(&self.data) as Rc<dyn ShipyardCatalog>;
            spaceport = spaceport.with_shipyard(
                template_of(SHIPYARD_DIALOG),
                info_templates(),
                template_of(TEXT_INPUT_DIALOG),
                shipyard,
                art,
            );
        }
        if self
            .flight
            .as_ref()
            .is_some_and(|flight| flight.failure().is_none())
        {
            let art: Rc<dyn ShipyardCatalog> = Rc::clone(&self.data) as Rc<dyn ShipyardCatalog>;
            spaceport = spaceport
                .with_bar(
                    BarTemplates {
                        plain: template_of(BAR_DIALOG),
                        picture: template_of(PICTURE_BAR_DIALOG),
                    },
                    template_of(NEWS_DIALOG),
                    template_of(SHIPYARD_DIALOG),
                    info_templates(),
                    art,
                    // `_CanHireEscorts`, as the hire itself reads it.
                    shops.can_hire_escorts,
                )
                .with_race(template_of(RACE_DIALOG), template_of(QUANTITY_DIALOG).ok())
                .with_missions(
                    template_of(COMPUTER_DIALOG),
                    template_of(INFO_DIALOG),
                    OfferTemplates {
                        plain: template_of(OFFER_DIALOG),
                        picture: template_of(PICTURE_OFFER_DIALOG),
                    },
                    DescTemplates {
                        plain: template_of(DESC_DIALOG),
                        picture: template_of(PICTURE_DESC_DIALOG),
                    },
                );
        }
        spaceport = spaceport
            .with_player_info(
                template_of(PLAYER_INFO_DIALOG),
                template_of(YES_NO_DIALOG),
                BackgroundSizes::read(&self.data),
            )
            .with_offer_clock(self.rules.source_for(RuleKey::OfferClock))
            .with_focus_ring(self.dialog_focus_ring())
            .with_movies(self.movies.clone())
            .with_chance(self.chance.clone());
        // Recharge is enabled by the cash and the tank, which only flight
        // knows; it is told again after each input.
        if let Some(flight) = &self.flight {
            spaceport.set_recharge_enabled(flight.recharge_enabled());
        }
        self.drop_spaceport();
        self.spaceport = Some(spaceport);
        self.serve_descriptions();
        self.switch_to(Side::Spaceport);
    }

    /// Puts the pilot away and shows `side`: saves it (unless it may not
    /// be saved), or deletes a strict pilot dying
    /// ([`AppScreen::put_away_pilot`]), before anything can show the menu
    /// or list the pilots, and drops flight, the spaceport and any dialog
    /// open over flight, so nothing is left to re-enter
    /// ([`AppScreen::drop_flight`]).
    pub(super) fn end_flight(&mut self, side: Side) {
        self.put_away_pilot();
        self.switch_to(side);
        self.drop_flight();
    }

    /// Drops the flight shown, if any, keeping its settings for the next
    /// flight ([`AppScreen::remember_flight`]), then the spaceport and
    /// every dialog open over flight ([`AppScreen::drop_over_flight`]).
    pub(super) fn drop_flight(&mut self) {
        if let Some(flight) = self.flight.take() {
            self.remember_flight(&flight);
        }
        self.drop_over_flight();
    }

    /// Drops the spaceport and every dialog open over flight.
    pub(super) fn drop_over_flight(&mut self) {
        self.drop_spaceport();
        self.plunder = None;
        self.assignment = None;
        self.capture_naming = None;
        self.comm = None;
        self.escort_dialog = None;
        self.planet_comm = None;
        self.haggle = None;
        self.offer = None;
        self.missions = None;
        self.player_info = None;
    }

    /// The seam at the expiry of the hold after the player's death, before
    /// the original's `gPause = 1` (0x68838): the `dësc` of each of a
    /// sabotaged player's outfits shows over the stopped flight, one
    /// after another (`_HandlePlayer` @0x68739-0x68832,
    /// [`FlightView::sabotage_death_descs`]), and the fade
    /// ([`FlightView::begin_fade`]) begins once the last closes
    /// ([`AppScreen::fade_once_dialogs_close`]). With none to show, no
    /// dialogs or no "Desc Dialog", it begins at once, on the same tick.
    ///
    /// Each is loaded through flight as `_LoadDescription` (@0x687c9)
    /// loads it ([`FlightView::load_description`]). A `dësc` that cannot
    /// be read is skipped with a warning; what the original does with a missing one
    /// was not checked. Mission texts the death's tick queued share the
    /// queue, after these, and the fade waits for them too.
    ///
    /// The dialogs open in the tick the hold expires, before its frame is
    /// drawn, and the fade begins on the input that closes the last, so
    /// no boundary the player sees lies between them and the original's
    /// order.
    fn at_hold_expiry(&mut self) {
        let Some(flight) = &mut self.flight else {
            return;
        };
        let mut texts = Vec::new();
        for id in flight.sabotage_death_descs() {
            match flight.load_description(id) {
                Ok(text) => texts.push(DescShown {
                    text,
                    graphic: self.data.description_graphic(id),
                    movie: self.data.description_movie(id),
                }),
                Err(reason) => self.warnings.push(format!(
                    "nova: cannot show the sabotage dësc {id}: {reason}"
                )),
            }
        }
        if !texts.is_empty() {
            self.show_over_flight(texts);
        }
        self.fade_once_dialogs_close();
    }

    /// Shows the escape pod's rescue's `dësc` over flight, as
    /// `_HandlePlayer`'s `_ShowTextDialog` does (@0x68eac), in the variant
    /// its `Graphic` picks: the text flight's session loaded first in the
    /// rescue (`_LoadDescription` @0x68e90, before the missions ended), so
    /// it is not loaded again here. One that could not be read is a
    /// warning, saying why as the catalog reads it. The rescue settled
    /// before it, and flight goes on once it closes: no fade follows,
    /// unlike the death's.
    fn show_rescue_desc(&mut self, desc: LoadedDesc) {
        let id = desc.id;
        if let Some(text) = desc.text {
            let graphic = self.data.description_graphic(id);
            self.show_over_flight(vec![DescShown {
                text,
                graphic,
                movie: self.data.description_movie(id),
            }]);
        } else {
            let reason = self.data.description(id).err().unwrap_or_default();
            self.warnings
                .push(format!("nova: cannot show the rescue dësc {id}: {reason}"));
        }
    }

    /// Begins the death's fade once no dialog is open over flight: at
    /// the hold's expiry with none to show, or as the last closes.
    /// Before the hold has expired [`FlightView::begin_fade`] does
    /// nothing, so a dialog closing in live flight leaves flight as it is.
    pub(super) fn fade_once_dialogs_close(&mut self) {
        if self.missions.is_none()
            && let Some(flight) = &mut self.flight
        {
            flight.begin_fade();
        }
    }

    /// Takes what flight's tick ended: at the hold's expiry, the seam
    /// ([`AppScreen::at_hold_expiry`]); once the death's fade is over, the
    /// return to the main menu as it appears
    /// ([`AppScreen::return_to_main_screen`]), or, without one, to the side
    /// flight was entered from, which fades in from black. A board the
    /// tick made, the Board key held, opens as a press's does
    /// ([`AppScreen::open_frame_boarding`]), and so does a dialog a key
    /// read past a jump asked for ([`AppScreen::open_frame_dialogs`]).
    pub(super) fn after_flight_tick(&mut self) {
        self.settle_flight_movie();
        self.open_frame_boarding();
        self.open_frame_dialogs();
        if self
            .flight
            .as_mut()
            .is_some_and(FlightView::take_hold_expired)
        {
            self.at_hold_expiry();
        }
        if let Some(desc) = self.flight.as_mut().and_then(FlightView::take_rescue_desc) {
            self.show_rescue_desc(desc);
        }
        if self.flight.as_ref().is_some_and(FlightView::game_over) {
            if self.menu.is_some() {
                self.return_to_main_screen();
            } else {
                self.end_flight(self.return_to);
                self.fade_in = Some(ScreenFade::from_black(DEATH_FADE_FOR));
            }
        }
    }

    /// Takes what a stellar's movie in front of flight left once it has
    /// ended ([`FlightView::playing`]): the landing it held back shows the
    /// spaceport, and what was pressed under it goes to what is shown
    /// next, the spaceport or flight
    /// ([`FlightView::take_movie_replay`]), as the original's queued
    /// events reach the next event loop. Nothing while it plays.
    ///
    /// After a landing, the land checks' texts the movie held
    /// ([`FlightView::take_mission_texts`]) and the venues' offers settle
    /// over the opened spaceport before the replay, as `_DoPortDialog`
    /// shows the spaceport (@0x5faff), then runs `_MissionLandCheck`
    /// (@0x5fbeb) and `_OfferMissionFromPort` (@0x5fc37) before its own
    /// loop: the replayed keys reach the first text.
    pub(super) fn settle_flight_movie(&mut self) {
        let Some(flight) = self.flight.as_mut().filter(|flight| !flight.playing()) else {
            return;
        };
        let replay = flight.take_movie_replay();
        if let Some(stellar) = flight.take_landing() {
            self.show_spaceport(stellar);
            self.settle_script();
        }
        for input in replay {
            let _ = self.routed_input(&input);
        }
    }

    /// Moves the fade back from black on by `dt`, letting go of it once it
    /// is over.
    pub(super) fn advance_fade_in(&mut self, dt: Duration) {
        if let Some(fade) = &mut self.fade_in {
            fade.advance(dt);
            if fade.done() {
                self.fade_in = None;
            }
        }
    }

    /// Keeps `flight`'s ship slots' draw-layer and cloned-frame history,
    /// its stellar sprite pool, what its berths keep, what its last galaxy
    /// refresh left hidden, its comm dialog's release flag, its course
    /// map's zoom and whether the map
    /// shows the borders, for the next flight in this run.
    pub(super) fn remember_flight(&mut self, flight: &FlightView<Rc<GameData>>) {
        if let Some(carry) = flight.sim().carry_over() {
            if let Some(slot_sprites) = carry.slot_sprites {
                self.slot_sprites = slot_sprites;
            }
            if let Some(last_refresh) = carry.last_refresh {
                self.last_refresh = Some(last_refresh);
            }
            if let Some(pool) = carry.stellar_sprite_pool {
                self.stellar_sprites = pool;
            }
            if let Some(kept_berths) = carry.kept_berths {
                self.kept_berths = kept_berths;
            }
            if let Some(release) = carry.release_comm_escort {
                self.release_comm_escort = release;
            }
        }
        if let Some(zoom) = flight.course_map_zoom() {
            self.course_map_zoom = Some(zoom);
        }
        self.map_borders = flight.map_borders_shown();
    }

    /// Where pilots are kept, if anywhere.
    pub(super) fn keeper(&self) -> Option<Rc<PilotKeeper<Box<dyn PilotStore>>>> {
        self.menu.as_ref().and_then(|menu| menu.pilots.clone())
    }

    /// Puts the pilot flying away as play ends: deletes it, when there
    /// is somewhere it is kept and it is a strict pilot dying outside the
    /// escape pod ([`SessionFlight::deletes_pilot_on_ending_play`](nova_view::flight::SessionFlight::deletes_pilot_on_ending_play)), as
    /// `_EnterGameFromMainScreen` does once `_PlayGame` returns
    /// (@0x1a4ab-0x1a4e1), and otherwise saves it
    /// ([`AppScreen::save_pilot`]). A failed delete becomes a warning.
    /// Nothing writes the pilot back afterwards: a dying pilot may not be
    /// saved.
    pub(super) fn put_away_pilot(&mut self) {
        let (Some(flight), Some(keeper)) = (&self.flight, self.keeper()) else {
            return;
        };
        if !flight.sim().deletes_pilot_on_ending_play() {
            self.save_pilot();
            return;
        }
        let Some(pilot) = flight.sim().pilot() else {
            return;
        };
        if let Err(warning) = keeper.delete(pilot.name()) {
            self.warnings.push(warning);
        }
    }

    /// Saves the pilot flying, when it has a name, there is somewhere to
    /// keep it and it may be saved ([`SessionFlight::may_save`](nova_view::flight::SessionFlight::may_save): not while its
    /// ship breaks up or after it is destroyed); a failure becomes a
    /// warning. Pilots without a name, which the developer's F flies, are
    /// never saved.
    pub(super) fn save_pilot(&mut self) {
        if let Some(warning) = self
            .flight
            .as_ref()
            .and_then(|flight| self.save_warning(flight, None))
        {
            self.warnings.push(warning);
        }
    }

    /// Saves `flight`'s pilot as [`AppScreen::save_pilot`] says, in place
    /// of the pilot saved under `replaces` when there is one
    /// (`nova_sim::saves::PilotKeeper::save_replacing`), and the warning a
    /// failure gives.
    pub(super) fn save_warning(
        &self,
        flight: &FlightView<Rc<GameData>>,
        replaces: Option<&str>,
    ) -> Option<String> {
        let keeper = self.keeper()?;
        if !flight.sim().may_save() {
            return None;
        }
        let pilot = flight.sim().pilot()?;
        pilot_key(pilot.name())?;
        match replaces {
            Some(replaces) => keeper.save_replacing(pilot, replaces).err(),
            None => keeper.save(pilot).err(),
        }
    }

    /// Settles what the set expressions run in flight or in the spaceport
    /// queued ([`FlightView::settle_script`]); a move while landed leaves
    /// the spaceport open as it is. The mission texts queued show over
    /// it, and then the venues' offers are served: a landing rolls the
    /// visit's offers as it settles, so the spaceport's offer is asked for
    /// here, after the texts the landing queued, as `_DoPortDialog` offers
    /// after `_MissionLandCheck` (@0x5fbeb-0x5fc37), and before any input
    /// can open a service. Then a `Q` run while landed leaves, once
    /// nothing shown holds it ([`AppScreen::leave_if_due`]). Last, the
    /// main menu's pilot panel is brought in step with the game held
    /// ([`AppScreen::show_game_on_menu`]): this runs after every input and
    /// tick, so every frame drawn on the menu shows the game as it stands.
    pub(super) fn settle_script(&mut self) {
        if let Some(flight) = self.flight.as_mut() {
            flight.settle_script();
        }
        self.show_mission_texts();
        self.serve_offers();
        self.leave_if_due();
        self.show_game_on_menu();
    }

    /// Leaves the spaceport for a `Q` run while landed
    /// ([`FlightView::leave_due`]), as its Leave does
    /// ([`AppScreen::leave_port`]), unless a text or dialog shown over it
    /// holds the leave ([`SpaceportView::holds_leave`]): the router keeps
    /// only that gate, and the session carries the leave out
    /// ([`FlightView::leave_if_due`]). The original's
    /// port filters leave on their next call while `_absquatulateStr` is
    /// set (`_PortFilter` @0x4f047-0x4f063, which chooses Leave,
    /// @0x4f1ff), and the texts and dialogs over them never read it. As
    /// this runs after every input and tick, the input that closes the
    /// last of them leaves before a frame draws the spaceport bare. With
    /// no spaceport open, the ship leaves at once.
    ///
    /// The galaxy map needs no term here: a `Q` pending on it always has
    /// a holding dialog under it ([`SpaceportView::holds_leave`]). So the
    /// leave goes in the settle of the replayed key that answers that
    /// dialog, before anything reaches the port.
    ///
    /// Either leave first settles the escorts' marks: a report of what
    /// they sold and upgraded shows over the spaceport and holds the
    /// leave until it closes ([`AppScreen::report_escorts_landed`]).
    fn leave_if_due(&mut self) {
        let holds = self
            .spaceport
            .as_ref()
            .is_some_and(SpaceportView::holds_leave);
        if self.leave_held {
            if !holds {
                self.leave_held = false;
                self.leave_port_by(FlightView::take_off);
            }
            return;
        }
        if !self.flight.as_ref().is_some_and(FlightView::leave_due) {
            return;
        }
        if holds || self.report_escorts_landed() {
            return;
        }
        self.leave_port_by(FlightView::leave_if_due);
    }

    /// Leaves the spaceport by its Leave ([`FlightView::take_off`]), once
    /// the report of the escorts sold and upgraded, if any, has closed.
    pub(super) fn leave_port(&mut self) {
        // The spaceport stays left under the report: the held leave goes
        // on once it closes (`AppScreen::leave_if_due`).
        if self.leave_held {
            return;
        }
        if self.report_escorts_landed() {
            self.leave_held = true;
            return;
        }
        self.leave_port_by(FlightView::take_off);
    }

    /// Leaves the spaceport: it closes and the ship takes off by `leave`,
    /// the one path for the port's Leave and a `Q`'s, as `_DoPortDialog`
    /// has one (@0x5fcd3, @0x601a0-0x6025e). The session settles the
    /// take-off's events (the stay's `DayPassed`, `Explored`, `TookOff`)
    /// at once itself ([`Session::leave_port`](nova_sim::Session::leave_port)),
    /// and the mission texts they queue show over flight, so on either
    /// path they are settled before the take-off's save and its first
    /// frame, as `_PlayerLandOnStellar` runs them inside the take-off.
    fn leave_port_by(
        &mut self,
        leave: impl FnOnce(&mut FlightView<Rc<GameData>>) -> Option<nova_sim::StellarId>,
    ) {
        self.close_spaceport();
        if let Some(flight) = self.flight.as_mut() {
            let _ = leave(flight);
        }
        self.show_mission_texts();
    }

    /// Shows the mission texts the session queued: over the spaceport, if
    /// it is open, even when a `Q` has made a leave due, as the leave
    /// waits for them ([`AppScreen::leave_if_due`]); and otherwise, in
    /// flight, over flight, which pauses
    /// under them, as `_ShowTextDialog` (@0x5a752) shows them (a
    /// `BriefText` an `S` accepted, the texts of a person's offer
    /// accepted or refused over flight), first cancelling flight's
    /// pointer gesture and letting go of its keys (their releases will go
    /// to the texts), as any dialog over flight does; flight hears of each
    /// as it closes or is dropped ([`AppScreen::drop_closed_missions`]).
    /// Without dialogs, those in flight are dropped, as the spaceport
    /// drops them without its template, and flight hears of each text
    /// dropped at once, as the mission dialogs count a dropped text
    /// closed: the offer dialog still shows without dialogs, by its
    /// fallback layout, so a person's offer answered over flight holds
    /// the bar under its texts until then
    /// ([`FlightView::close_mission_text`]). On any other side they stay
    /// queued in the session.
    /// An offer's movies come in the places the session queued them.
    /// Over the spaceport, the texts it closed or dropped are told to
    /// flight first and last (`close_mission_texts`).
    pub(super) fn show_mission_texts(&mut self) {
        if let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) {
            close_mission_texts(spaceport, flight);
            spaceport.show_mission_texts(flight.take_mission_texts());
            close_mission_texts(spaceport, flight);
            // A shipyard whose purchase's texts have all closed closes
            // here (`SpaceportView::show_mission_texts`), and is told at
            // once, before the next frame.
            close_port_dialogs(spaceport, flight);
            return;
        }
        if self.side != Side::Flight {
            return;
        }
        let Some(texts) = self
            .flight
            .as_mut()
            .map(FlightView::take_mission_texts)
            .filter(|texts| !texts.is_empty())
        else {
            return;
        };
        if self.flight_missions().is_none() {
            let flight = self.flight.as_mut().expect("texts came from flight");
            for _ in texts
                .iter()
                .filter(|shown| matches!(shown, nova_sim::MissionShown::Text(_)))
            {
                flight.close_mission_text();
            }
            return;
        }
        self.over_flight(|missions| missions.show_texts(texts));
    }

    /// Shows `texts`, each a text and its `dësc`'s `Graphic`, over flight
    /// in "Desc Dialog" of the variant each picks, one after another, as
    /// [`AppScreen::over_flight`] shows them. Without dialogs they are
    /// dropped.
    pub(super) fn show_over_flight(&mut self, texts: Vec<DescShown>) {
        self.over_flight(|missions| missions.show_descs(texts));
    }

    /// Queues what `show` queues on the mission dialogs over flight, first
    /// cancelling flight's pointer gesture and letting go of its keys
    /// (their releases will go to the dialogs), as any dialog over flight
    /// does. Without dialogs nothing is queued.
    fn over_flight(&mut self, show: impl FnOnce(&mut MissionDialogs)) {
        if self.flight_missions().is_none() {
            return;
        }
        let below = self.shown_mut();
        below.cancel_pointer();
        below.release_keys();
        let faded = self.white_faded();
        if let Some(missions) = &mut self.missions {
            if faded {
                missions.set_faded();
            }
            show(missions);
        }
        self.drop_closed_missions();
    }

    /// Closes the spaceport, if it is open, and shows flight, letting go
    /// of the spaceport's keys, then drops it
    /// ([`AppScreen::drop_spaceport`]).
    fn close_spaceport(&mut self) {
        if self.spaceport.is_none() {
            return;
        }
        self.switch_to(Side::Flight);
        self.drop_spaceport();
    }

    /// Drops the spaceport, if it is open, keeping its sounds and its
    /// ambient sound's halt ([`SpaceportView::close_sounds`]): every way
    /// the spaceport goes comes through here, Leave, a script's take-off,
    /// the pilot put away or replaced, or the pilot desk moving it, as
    /// `_DoPortDialog` halts `_planetSnd` however its loop ends
    /// (@0x601d2-0x601eb). A move while landed keeps the spaceport open,
    /// and its sound with it.
    fn drop_spaceport(&mut self) {
        self.leave_held = false;
        if let Some(mut spaceport) = self.spaceport.take() {
            self.sounds.extend(spaceport.close_sounds());
        }
    }

    /// Saves the pilot if flight says a save is due: it has landed, taken
    /// off or changed in the spaceport.
    pub(super) fn save_if_due(&mut self) {
        if self.flight.as_mut().is_some_and(FlightView::take_save_due) {
            self.save_pilot();
        }
    }
}
