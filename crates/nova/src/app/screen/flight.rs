//! [`AppScreen`]'s flight: showing the spaceport, building, entering and
//! ending flight, the fades, saving the pilot and leaving port.

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_sim::{Pilot, PilotKeeper, PilotStore, RuleKey, pilot_key};
use nova_view::fade::{DEATH_FADE_FOR, ScreenFade};
use nova_view::flight::FlightView;
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::bar::{BAR_DIALOG, BarTemplates, PICTURE_BAR_DIALOG};
use nova_view::spaceport::layout::SPACEPORT_DIALOG;
use nova_view::spaceport::missions::{COMPUTER_DIALOG, INFO_DIALOG};
use nova_view::spaceport::news::NEWS_DIALOG;
use nova_view::spaceport::offer::{OFFER_DIALOG, OfferTemplates, PICTURE_OFFER_DIALOG};
use nova_view::spaceport::outfitter::OUTFIT_DIALOG;
use nova_view::spaceport::shipyard::{
    InfoTemplates, SHIP_INFO_DIALOG, SHIP_PICTURE_INFO_DIALOG, SHIPYARD_DIALOG,
};
use nova_view::spaceport::trade::TRADE_DIALOG;
use nova_view::spaceport::{OutfitterCatalog, ShipyardCatalog};
use nova_view::ui::catalog::DescriptionGraphics;
use nova_view::ui::desc::{DESC_DIALOG, DescTemplates, PICTURE_DESC_DIALOG};
use nova_view::ui::mission_dialogs::{DescShown, MissionDialogs};
use nova_view::ui::player_info::{PLAYER_INFO_DIALOG, YES_NO_DIALOG};
use nova_view::ui::quantity::QUANTITY_DIALOG;
use nova_view::ui::text_input::TEXT_INPUT_DIALOG;
use nova_view::ui::{DescriptionSource, DialogResources};

use super::port_dialogs::close_mission_texts;
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
        if let Some(market) = self.flight.as_ref().and_then(FlightView::market) {
            let trade = template(TRADE_DIALOG).map(|(template, _)| template);
            let quantity = template(QUANTITY_DIALOG).map(|(template, _)| template);
            spaceport = spaceport.with_trade(trade, quantity, market);
        }
        if let Some(outfitter) = self.flight.as_mut().and_then(FlightView::outfitter) {
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
        if let Some(shipyard) = self.flight.as_mut().and_then(FlightView::shipyard) {
            let art: Rc<dyn ShipyardCatalog> = Rc::clone(&self.data) as Rc<dyn ShipyardCatalog>;
            spaceport = spaceport.with_shipyard(
                template_of(SHIPYARD_DIALOG),
                info_templates(),
                template_of(TEXT_INPUT_DIALOG),
                shipyard,
                art,
            );
        }
        if let Some(session) = self
            .flight
            .as_ref()
            .and_then(|flight| flight.session().ok())
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
                    session.can_hire_escorts(),
                )
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
            .with_player_info(template_of(PLAYER_INFO_DIALOG), template_of(YES_NO_DIALOG))
            .with_offer_clock(self.rules.source_for(RuleKey::OfferClock))
            .with_movies(self.movies.clone())
            .with_chance(self.chance.clone());
        self.drop_spaceport();
        self.spaceport = Some(spaceport);
        self.serve_descriptions();
        self.switch_to(Side::Spaceport);
    }

    /// Flies `pilot`, from the main menu, in place of any flight; Escape
    /// goes back to the menu. A pilot last landed on a stellar resumes in
    /// flight over it ([`FlightView::with_pilot`]).
    pub(super) fn start_flight(&mut self, pilot: Pilot) {
        self.build_flight(pilot, false);
        self.enter_built_flight();
    }

    /// Builds `pilot`'s flight in place of any flight, without showing
    /// it: one `loaded` from a save reads its load's fallbacks from the
    /// router's rules ([`FlightView::loaded`]), a new one is flown as it
    /// is.
    pub(super) fn build_flight(&mut self, pilot: Pilot, loaded: bool) {
        self.drop_spaceport();
        self.put_away_flight();
        let rules = self.rules;
        let flight = self.flight(|data| {
            if loaded {
                FlightView::loaded(data, pilot, rules)
            } else {
                FlightView::with_pilot(data, pilot)
            }
        });
        self.flight = Some(flight);
    }

    /// Shows the flight just built, from the main menu, which Escape goes
    /// back to: in space, as `_EnterGameFromMainScreen` and `_PlayGame`
    /// land nothing.
    pub(super) fn enter_built_flight(&mut self) {
        self.return_to = Side::MainMenu;
        self.switch_to(Side::Flight);
    }

    /// Puts the pilot away and shows `side`: saves it (unless it may not
    /// be saved), or deletes a strict pilot dying
    /// ([`AppScreen::put_away_pilot`]), before anything can show the menu
    /// or list the pilots, and drops flight, the spaceport and any dialog
    /// open over flight, so nothing is left to re-enter.
    pub(super) fn end_flight(&mut self, side: Side) {
        self.put_away_pilot();
        self.switch_to(side);
        self.put_away_flight();
        self.drop_spaceport();
        self.plunder = None;
        self.assignment = None;
        self.comm = None;
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

    /// Shows the escape pod's rescue's `dësc` `id` over flight, as
    /// `_HandlePlayer`'s `_ShowTextDialog` does (@0x68e80-0x68eac), in the
    /// variant its `Graphic` picks, loaded through flight as
    /// `_LoadDescription` (@0x68e90) loads it
    /// ([`FlightView::load_description`]); one that cannot be read is a
    /// warning. The rescue settled before it, and flight goes on once it closes: no
    /// fade follows, unlike the death's.
    fn show_rescue_desc(&mut self, id: i16) {
        let loaded = match &mut self.flight {
            Some(flight) => flight.load_description(id),
            None => self.data.description(id),
        };
        match loaded {
            Ok(text) => {
                let graphic = self.data.description_graphic(id);
                self.show_over_flight(vec![DescShown {
                    text,
                    graphic,
                    movie: self.data.description_movie(id),
                }]);
            }
            Err(reason) => self
                .warnings
                .push(format!("nova: cannot show the rescue dësc {id}: {reason}")),
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
    /// flight was entered from, which fades in from black.
    pub(super) fn after_flight_tick(&mut self) {
        if self
            .flight
            .as_mut()
            .is_some_and(FlightView::take_hold_expired)
        {
            self.at_hold_expiry();
        }
        if let Some(id) = self.flight.as_mut().and_then(FlightView::take_rescue_desc) {
            self.show_rescue_desc(id);
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

    /// Drops the flight, if any, keeping its ship slots' draw-layer and
    /// cloned-frame history, its course map's zoom and whether the map
    /// shows the borders, for the next flight in this run.
    fn put_away_flight(&mut self) {
        let Some(flight) = self.flight.take() else {
            return;
        };
        if let Some(slot_sprites) = flight.slot_sprites() {
            self.slot_sprites = slot_sprites.clone();
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
    /// escape pod ([`FlightView::deletes_pilot_on_ending_play`]), as
    /// `_EnterGameFromMainScreen` does once `_PlayGame` returns
    /// (@0x1a4ab-0x1a4e1), and otherwise saves it
    /// ([`AppScreen::save_pilot`]). A failed delete becomes a warning.
    /// Nothing writes the pilot back afterwards: a dying pilot may not be
    /// saved.
    pub(super) fn put_away_pilot(&mut self) {
        let (Some(flight), Some(keeper)) = (&self.flight, self.keeper()) else {
            return;
        };
        if !flight.deletes_pilot_on_ending_play() {
            self.save_pilot();
            return;
        }
        let Some(pilot) = flight.pilot() else {
            return;
        };
        if let Err(warning) = keeper.delete(pilot.name()) {
            self.warnings.push(warning);
        }
    }

    /// Saves the pilot flying, when it has a name, there is somewhere to
    /// keep it and it may be saved ([`FlightView::may_save`]: not while its
    /// ship breaks up or after it is destroyed); a failure becomes a
    /// warning. Pilots without a name, which the developer's F flies, are
    /// never saved.
    pub(super) fn save_pilot(&mut self) {
        let (Some(flight), Some(keeper)) = (&self.flight, self.keeper()) else {
            return;
        };
        if !flight.may_save() {
            return;
        }
        let Some(pilot) = flight.pilot() else {
            return;
        };
        if pilot_key(pilot.name()).is_none() {
            return;
        }
        if let Err(warning) = keeper.save(pilot) {
            self.warnings.push(warning);
        }
    }

    /// Settles what the set expressions run in flight or in the spaceport
    /// queued ([`FlightView::settle_script`]); a move while landed leaves
    /// the spaceport open as it is. The mission texts queued show over
    /// it, and then the venues' offers are served: a landing rolls the
    /// visit's offers as it settles, so the spaceport's offer is asked for
    /// here, after the texts the landing queued, as `_DoPortDialog` offers
    /// after `_MissionLandCheck` (@0x5fbeb-0x5fc37), and before any input
    /// can open a service. Last, a `Q` run while landed leaves, once
    /// nothing shown holds it ([`AppScreen::leave_if_due`]).
    pub(super) fn settle_script(&mut self) {
        if let Some(flight) = self.flight.as_mut() {
            flight.settle_script();
        }
        self.show_mission_texts();
        self.serve_offers();
        self.leave_if_due();
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
    fn leave_if_due(&mut self) {
        if !self.flight.as_ref().is_some_and(FlightView::leave_due) {
            return;
        }
        if self
            .spaceport
            .as_ref()
            .is_some_and(SpaceportView::holds_leave)
        {
            return;
        }
        self.leave_port_by(FlightView::leave_if_due);
    }

    /// Leaves the spaceport by its Leave ([`FlightView::take_off`]).
    pub(super) fn leave_port(&mut self) {
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
        if let Some(missions) = &mut self.missions {
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
