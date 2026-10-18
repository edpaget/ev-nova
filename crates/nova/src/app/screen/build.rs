//! [`AppScreen`]'s construction: the builders, the dialog, menu and
//! pilot accessors, and the view accessors.

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::ship_layer::SlotSprites;
use nova_sim::{
    Allegiance, Behaviour, BoardingRule, CommCatalog, ControlBits, DisableRule, HailOptions,
    HireTerms, LegalCode, NovaAi, NovaBits, NovaBoarding, NovaDisable, NovaHire, NovaLaw,
    NovaPersons, PersonRules, Pilot, PilotKeeper, PilotStore, PointDefenceRule, Rulebook, Session,
};
use nova_view::devtools::SessionDesk;
use nova_view::flight::{FlightOverlays, FlightView, SharedChance};
use nova_view::galaxy::GalaxyMap;
use nova_view::galaxy::find::FIND_DIALOG;
use nova_view::menu::{MainMenu, PilotList};
use nova_view::movie::{MoviePlayer, Movies};
use nova_view::ships::ShipBrowser;
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::missions::MissionInfoScreen;
use nova_view::spaceport::offer::OfferDialog;
use nova_view::system::SystemView;
use nova_view::text::TextMetrics;
use nova_view::ui::new_pilot::NewPilotDialog;
use nova_view::ui::note::NoteDialog;
use nova_view::ui::player_info::PlayerInfoScreen;
use nova_view::ui::{
    AssignmentDialog, CommDialog, DescDialog, DescriptionSource, DialogResources, DialogTemplate,
    HaggleDialog, PlunderDialog, PrefsDialog,
};
use nova_view::{Input, Key, Navigator, Prefs, Screen, ScreenAction, Showing};

use super::port_dialogs::refresh_spaceport;
use super::{
    ABOUT_TEXT, AppScreen, CLASSIC_VIEW, Dialogs, ENTERED, LANDED, MENU, Menu, MenuKeys, Side,
};

impl AppScreen {
    /// Every screen over `data`, showing the ship browser.
    #[must_use]
    pub fn new(data: Rc<GameData>) -> Self {
        Self {
            side: Side::Ships,
            return_to: Side::Ships,
            galaxy: Navigator::new(Rc::clone(&data)),
            ships: ShipBrowser::new(Rc::clone(&data)),
            data,
            flight: None,
            slot_sprites: SlotSprites::default(),
            course_map_zoom: None,
            map_borders: true,
            dialogs: None,
            about: None,
            about_text: ABOUT_TEXT,
            spaceport: None,
            sounds: Vec::new(),
            prefs: Prefs::default(),
            preferences: None,
            prefs_change: None,
            display_choice: None,
            menu: None,
            new_pilot: None,
            open_pilot: None,
            load_note: None,
            warnings: Vec::new(),
            chance: SharedChance::default(),
            effects_chance: SharedChance::default(),
            behaviour: Rc::new(NovaAi::default()),
            disable_rule: Rc::new(NovaDisable),
            defence_rule: Rc::new(Allegiance),
            law: Rc::new(NovaLaw::default()),
            boarding_rule: Rc::new(NovaBoarding::default()),
            plunder: None,
            assignment: None,
            hail_options: HailOptions::default(),
            rules: Rulebook::default(),
            flight_overlays: FlightOverlays::default(),
            hire_terms: Rc::new(NovaHire::default()),
            control_bits: Rc::new(NovaBits),
            person_rules: Rc::new(NovaPersons::default()),
            comm: None,
            haggle: None,
            offer: None,
            missions: None,
            movies: Movies::new(
                Rc::new(crate::movie::NoMoviePlayer),
                Prefs::default().quicktime_movies,
            ),
            fade_in: None,
            player_info: None,
            command_held: false,
            keys_held: MenuKeys::default(),
            menu_hold: None,
            held: Vec::new(),
            quit_in: None,
            quit_due: false,
            double_time: false,
            widened: CLASSIC_VIEW,
        }
    }

    /// The router with each flight rolling each day's events and its
    /// traffic on `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// The router with each flight's explosions and debris rolled on
    /// `chance`, apart from the simulation's.
    #[must_use]
    pub fn with_effects_chance(self, effects_chance: SharedChance) -> Self {
        Self {
            effects_chance,
            ..self
        }
    }

    /// The router with each flight's NPCs deciding as `behaviour` says.
    #[must_use]
    pub fn with_behaviour(self, behaviour: Rc<dyn Behaviour>) -> Self {
        Self { behaviour, ..self }
    }

    /// The router with each flight's ships disabled as `rule` says.
    #[must_use]
    pub fn with_disable_rule(self, disable_rule: Rc<dyn DisableRule>) -> Self {
        Self {
            disable_rule,
            ..self
        }
    }

    /// The router with each flight's point defence engaging the missiles
    /// `rule` calls hostile.
    #[must_use]
    pub fn with_point_defence_rule(self, defence_rule: Rc<dyn PointDefenceRule>) -> Self {
        Self {
            defence_rule,
            ..self
        }
    }

    /// The router with each flight's player's crimes judged by `law`.
    #[must_use]
    pub fn with_law(self, law: Rc<dyn LegalCode>) -> Self {
        Self { law, ..self }
    }

    /// The router with each flight's boardings by `rule`: who repels
    /// boarders, the capture odds and the capture roll.
    #[must_use]
    pub fn with_boarding_rule(self, boarding_rule: Rc<dyn BoardingRule>) -> Self {
        Self {
            boarding_rule,
            ..self
        }
    }

    /// The router with each flight's comm dialog listing `hail_options`.
    #[must_use]
    pub fn with_hail_options(self, hail_options: HailOptions) -> Self {
        Self {
            hail_options,
            ..self
        }
    }

    /// The router with `terms` giving the fee and wage of a hire in each
    /// flight ([`FlightView::with_hire_terms`]); Nova's until others are
    /// given.
    #[must_use]
    pub fn with_hire_terms(self, hire_terms: Rc<dyn HireTerms>) -> Self {
        Self { hire_terms, ..self }
    }

    /// The router with `bits` testing a ship for hire's `Availability` and
    /// a person's `ActiveOn` against the pilot in each flight
    /// ([`FlightView::with_control_bits`]); Nova's
    /// ([`NovaBits`](nova_sim::NovaBits)) until others are given.
    #[must_use]
    pub fn with_control_bits(self, control_bits: Rc<dyn ControlBits>) -> Self {
        Self {
            control_bits,
            ..self
        }
    }

    /// The router with `rules` deciding how persons appear in each flight
    /// ([`FlightView::with_person_rules`]); Nova's until others are given.
    #[must_use]
    pub fn with_person_rules(self, person_rules: Rc<dyn PersonRules>) -> Self {
        Self {
            person_rules,
            ..self
        }
    }

    /// The router with each flight's disputed rules reading as `rules`
    /// says ([`FlightView::with_rules`]): every rule the engine's until
    /// others are given. Only the rule set changes: the strategies the
    /// router hands each flight (its behaviour, law, boarding, hail
    /// options, hire terms and persons) stay as they are, so a rule one
    /// of them decides changes only through [`AppScreen::with_rulebook`].
    #[must_use]
    pub fn with_rules(self, rules: Rulebook) -> Self {
        Self { rules, ..self }
    }

    /// The router with each flight drawing the port's own `overlays`, the
    /// title and the help line ([`FlightView::with_overlays`]): both off
    /// until others are given. They are developer settings, not rules.
    #[must_use]
    pub fn with_flight_overlays(self, flight_overlays: FlightOverlays) -> Self {
        Self {
            flight_overlays,
            ..self
        }
    }

    /// The router with Nova's rules, each disputed one as `rulebook`
    /// chooses: the strategies each flight decides by (the NPCs'
    /// behaviour, disabling, point defence, the law, boarding, hailing,
    /// the hire terms, the control bits and the persons), built from it,
    /// and the rule set every flight reads the rest from
    /// ([`AppScreen::with_rules`]). This is the edge where every
    /// [`RuleKey`](nova_sim::RuleKey) meets its setting.
    #[must_use]
    pub fn with_rulebook(self, rulebook: &Rulebook) -> Self {
        self.with_behaviour(Rc::new(NovaAi::from_rulebook(rulebook)))
            .with_disable_rule(Rc::new(NovaDisable))
            .with_point_defence_rule(Rc::new(Allegiance))
            .with_law(Rc::new(NovaLaw::from_rulebook(rulebook)))
            .with_boarding_rule(Rc::new(NovaBoarding::from_rulebook(rulebook)))
            .with_hail_options(HailOptions::nova(rulebook))
            .with_hire_terms(Rc::new(NovaHire::from_rulebook(rulebook)))
            .with_control_bits(Rc::new(NovaBits))
            .with_person_rules(Rc::new(NovaPersons::from_rulebook(rulebook)))
            .with_rules(*rulebook)
    }

    /// The comm dialog, while a hail is under way.
    #[must_use]
    pub fn comm(&self) -> Option<&CommDialog> {
        self.comm.as_ref()
    }

    /// The haggle dialog, while a price is asked.
    #[must_use]
    pub fn haggle(&self) -> Option<&HaggleDialog> {
        self.haggle.as_ref()
    }

    /// The offer dialog of a person hailed or boarded, while it is open.
    #[must_use]
    pub fn offer(&self) -> Option<&OfferDialog> {
        self.offer.as_ref()
    }

    /// The player info screen open over flight, if it is.
    #[must_use]
    pub fn flight_player_info(&self) -> Option<&PlayerInfoScreen> {
        self.player_info.as_ref()
    }

    /// The mission text shown over flight, if any.
    #[must_use]
    pub fn flight_mission_text(&self) -> Option<&DescDialog> {
        self.missions.as_ref()?.text()
    }

    /// The mission info screen open over flight, if it is.
    #[must_use]
    pub fn flight_mission_info(&self) -> Option<&MissionInfoScreen> {
        self.missions.as_ref()?.info()
    }

    /// The plunder dialog, while a boarding is under way.
    #[must_use]
    pub fn plunder(&self) -> Option<&PlunderDialog> {
        self.plunder.as_ref()
    }

    /// The captured-ship assignment dialog, while a capture awaits it.
    #[must_use]
    pub fn assignment(&self) -> Option<&AssignmentDialog> {
        self.assignment.as_ref()
    }

    /// A flight over the game data, `new` from it, rolling on the router's
    /// chance, its effects on the router's effects chance, deciding as its
    /// behaviour says, disabling ships, engaging missiles and judging
    /// crimes as its rules say, naming its ship from the game data's
    /// string lists, laying out its HUD's text with the router's
    /// metrics, if it has any, and in the widened size the router was last
    /// told ([`Screen::set_widened_size`]), so its first tick already
    /// uses it.
    pub(super) fn flight(
        &self,
        new: impl FnOnce(Rc<GameData>) -> FlightView<Rc<GameData>>,
    ) -> FlightView<Rc<GameData>> {
        let mut flight = self.configured_flight(new);
        flight.set_view_size(self.widened);
        flight
    }

    /// [`AppScreen::flight`]'s flight, before it is sized.
    fn configured_flight(
        &self,
        new: impl FnOnce(Rc<GameData>) -> FlightView<Rc<GameData>>,
    ) -> FlightView<Rc<GameData>> {
        let flight = new(Rc::clone(&self.data))
            .with_chance(self.chance.clone())
            .with_effects_chance(self.effects_chance.clone())
            .with_behaviour(Rc::clone(&self.behaviour))
            .with_disable_rule(Rc::clone(&self.disable_rule))
            .with_point_defence_rule(Rc::clone(&self.defence_rule))
            .with_law(Rc::clone(&self.law))
            .with_boarding_rule(Rc::clone(&self.boarding_rule))
            .with_hail_options(self.hail_options.clone())
            .with_rules(self.rules)
            .with_slot_sprites(self.slot_sprites.clone())
            .with_overlays(self.flight_overlays)
            .with_hire_terms(Rc::clone(&self.hire_terms))
            .with_control_bits(Rc::clone(&self.control_bits))
            .with_person_rules(Rc::clone(&self.person_rules))
            .with_strings(Rc::clone(&self.data) as Rc<dyn CommCatalog>)
            .with_hyperspace_effects(self.prefs.hyperspace_effects)
            .with_sound_prefs(self.prefs.sound)
            .with_double_time(self.double_time)
            .with_find(self.find_template(), self.data.button_style())
            .with_map_borders(self.map_borders);
        let flight = match self.course_map_zoom {
            Some(zoom) => flight.with_course_map_zoom(zoom),
            None => flight,
        };
        match self.metrics() {
            Some(metrics) => flight.with_metrics(metrics),
            None => flight,
        }
    }

    /// The maps' Find dialog, `DLOG` 3005, from the dialogs, if they
    /// were given and have it; `None` lays it out from its fallback.
    pub(super) fn find_template(&self) -> Option<DialogTemplate> {
        let dialogs = self.dialogs.as_ref()?;
        dialogs.resources.dialog_template(FIND_DIALOG).ok()
    }

    /// The metrics the router's text is laid out by: the dialogs', or the
    /// main menu's, if either was given.
    pub(super) fn metrics(&self) -> Option<Rc<dyn TextMetrics>> {
        let dialogs = self.dialogs.as_ref().map(|dialogs| &dialogs.metrics);
        let menu = self.menu.as_ref().map(|menu| &menu.metrics);
        dialogs.or(menu).map(Rc::clone)
    }

    /// The router with the main menu, which it now opens on: New Pilot
    /// and Open Pilot save and open pilots through `pilots`, and its
    /// screens lay out their text with `metrics` when there are no
    /// dialogs. With no `pilots` (nowhere to keep them), New Pilot still
    /// flies, saving nothing, and Open Pilot lists nothing.
    #[must_use]
    pub fn with_pilots(
        self,
        pilots: Option<PilotKeeper<Box<dyn PilotStore>>>,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let screen = MainMenu::new(self.data.button_style(), Rc::clone(&metrics));
        Self {
            side: Side::MainMenu,
            menu: Some(Menu {
                screen,
                metrics,
                pilots: pilots.map(Rc::new),
            }),
            ..self
        }
    }

    /// The main menu, once given.
    #[must_use]
    pub fn main_menu(&self) -> Option<&MainMenu> {
        self.menu.as_ref().map(|menu| &menu.screen)
    }

    /// The New Pilot dialog, while it is open.
    #[must_use]
    pub fn new_pilot(&self) -> Option<&NewPilotDialog> {
        self.new_pilot.as_ref()
    }

    /// The saved pilots' list, while it is open.
    #[must_use]
    pub fn pilot_list(&self) -> Option<&PilotList> {
        self.open_pilot.as_ref()
    }

    /// The note that an opened pilot's load fell back, while it is open
    /// over the main menu.
    #[must_use]
    pub fn load_note(&self) -> Option<&NoteDialog> {
        self.load_note.as_ref()
    }

    /// Changes the pilot with `change` while the ship is landed, as a
    /// spaceport screen does, and says whether it did; the pilot is saved
    /// after the next input or tick.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        self.flight
            .as_mut()
            .is_some_and(|flight| flight.transact(change))
    }

    /// The pilot flying, as the developer tools' desk; `None` before
    /// flight is entered and when the flight could not start.
    pub fn pilot_desk(&mut self) -> Option<SessionDesk<'_, Rc<GameData>>> {
        self.flight.as_mut()?.pilot_desk()
    }

    /// Catches up with an edit made through the pilot desk: flight catches
    /// up at once with what the session told of it ([`FlightView::resync`],
    /// which its next frame would do anyway), laying out the system the
    /// pilot moved to or drawing the ship at the stellar it moved to; the
    /// open spaceport is rebuilt for the stellar the pilot is now landed
    /// on when it moved, settling at once so the session rolls the new
    /// stellar's offers and the spaceport asks its landing offer before
    /// the next frame, and otherwise shows the exchange, outfitter and
    /// shipyard as they now are; then the pilot is saved if a save is due,
    /// as after any change.
    pub fn after_pilot_edit(&mut self) {
        let Some(flight) = self.flight.as_mut() else {
            return;
        };
        flight.resync();
        let landed = flight.session().ok().and_then(Session::landed);
        if let Some(spaceport) = &mut self.spaceport {
            match landed {
                Some(stellar) if stellar != spaceport.stellar() => {
                    self.show_spaceport(stellar);
                    self.settle_script();
                }
                _ => refresh_spaceport(spaceport, flight),
            }
        }
        self.save_if_due();
    }

    /// The router with dialogs: I opens the About text in a dialog built
    /// from `dialogs`, its text laid out by `metrics`, and the galaxy
    /// viewer's Find is laid out from them (`DLOG` 3005, or its
    /// fallback) and measured by `metrics`. Without dialogs, I does
    /// nothing, and the viewer's Find never opens.
    #[must_use]
    pub fn with_dialogs(
        self,
        dialogs: Rc<dyn DialogResources>,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let template = dialogs.dialog_template(FIND_DIALOG).ok();
        let galaxy = self
            .galaxy
            .with_metrics(Rc::clone(&metrics))
            .with_find(template, self.data.button_style());
        Self {
            dialogs: Some(Dialogs {
                resources: dialogs,
                metrics,
            }),
            galaxy,
            ..self
        }
    }

    /// The router with each flight's course map showing the borders or
    /// not, as the player's `ShowMapBorders` setting says.
    #[must_use]
    pub fn with_map_borders(self, map_borders: bool) -> Self {
        Self {
            map_borders,
            ..self
        }
    }

    /// The router with the player's preferences, `prefs`, which the
    /// Preferences dialog opens on.
    #[must_use]
    pub fn with_prefs(self, prefs: Prefs) -> Self {
        self.movies.set_on(prefs.quicktime_movies);
        Self { prefs, ..self }
    }

    /// The router playing the mission texts' and offers' movies through
    /// `player`, while the preferences have them on.
    #[must_use]
    pub fn with_movie_player(self, player: Rc<dyn MoviePlayer>) -> Self {
        let movies = Movies::new(player, self.prefs.quicktime_movies);
        Self { movies, ..self }
    }

    /// The player's preferences, as last chosen.
    #[must_use]
    pub fn prefs(&self) -> Prefs {
        self.prefs
    }

    /// The About dialog, or the main menu's X text, while it is open.
    #[must_use]
    pub fn about(&self) -> Option<&DescDialog> {
        self.about.as_ref()
    }

    /// The Preferences dialog, while it is open.
    #[must_use]
    pub fn preferences(&self) -> Option<&PrefsDialog> {
        self.preferences.as_ref()
    }

    /// Which screen is showing.
    #[must_use]
    pub fn showing(&self) -> Showing {
        if self.about.is_some() {
            // The X text is no About box: it leaves the music playing.
            return if self.about_text == ABOUT_TEXT {
                Showing::About
            } else {
                Showing::Note
            };
        }
        if self.preferences.is_some() {
            return Showing::Preferences;
        }
        if self.player_info.is_some() {
            return Showing::PlayerInfo;
        }
        if self.haggle.is_some() {
            return Showing::Haggle;
        }
        if self.comm.is_some() {
            return Showing::Comm;
        }
        if self.flight_dialog_map_open() {
            return Showing::FlightMap;
        }
        if let Some(missions) = &self.missions {
            if missions.playing() {
                return Showing::Movie;
            }
            return if missions.text().is_some() {
                Showing::MissionText
            } else {
                Showing::MissionInfo
            };
        }
        if let Some(offer) = &self.offer {
            return if offer.playing() {
                Showing::Movie
            } else {
                Showing::Offer
            };
        }
        if self.plunder.is_some() {
            return Showing::Plunder;
        }
        if self.assignment.is_some() {
            return Showing::Assignment;
        }
        if self.new_pilot.is_some() {
            return Showing::NewPilot;
        }
        if self.open_pilot.is_some() {
            return Showing::OpenPilot;
        }
        if self.load_note.is_some() {
            return Showing::Note;
        }
        match self.side {
            Side::MainMenu => Showing::MainMenu,
            Side::Ships => Showing::ShipBrowser,
            Side::Galaxy if self.galaxy.system().is_some() => Showing::System,
            Side::Galaxy => Showing::GalaxyMap,
            Side::Flight if self.flight.as_ref().is_some_and(FlightView::map_open) => {
                Showing::FlightMap
            }
            Side::Flight => Showing::Flight,
            Side::Spaceport if self.port_map_open() => Showing::FlightMap,
            Side::Spaceport => Showing::Spaceport,
        }
    }

    /// The input to flight's course map open over the spaceport, taking
    /// it all, and true; false, taking nothing, with no map open there. An
    /// Escape press closes it ([`FlightView::close_map`]), as in flight,
    /// and anything else goes to it ([`FlightView::port_map_input`]).
    pub(super) fn port_map_input(&mut self, input: &Input) -> bool {
        if !self.port_map_open() {
            return false;
        }
        self.map_over_input(input);
        true
    }

    /// The input to flight's course map open over something: an Escape
    /// press closes it ([`FlightView::close_map`]), unless its Find is
    /// open, and anything else goes to it ([`FlightView::port_map_input`]).
    fn map_over_input(&mut self, input: &Input) {
        let flight = self.flight.as_mut().expect(ENTERED);
        let escape = Input::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false,
        };
        // Find, while open, takes Escape too, doing nothing with it.
        if *input == escape && !flight.map_finding() {
            flight.close_map();
        } else {
            flight.port_map_input(input);
        }
    }

    /// Whether a mission dialog's map is open over flight
    /// ([`FlightView::open_dialog_map`]): over the offer from a ship or the
    /// mission dialogs, which stay open under it.
    pub(super) fn flight_dialog_map_open(&self) -> bool {
        self.side == Side::Flight
            && self
                .flight
                .as_ref()
                .is_some_and(FlightView::dialog_map_open)
    }

    /// The input to a mission dialog's map over flight, before anything
    /// open under it sees it: as flight's own map takes it, the Preferences
    /// keys first unless Find is open, then the map's
    /// ([`AppScreen::port_map_input`]'s).
    pub(super) fn flight_dialog_map_input(&mut self, input: &Input) -> ScreenAction {
        let finding = self.flight.as_ref().is_some_and(FlightView::map_finding);
        if !finding && self.preferences_key(input) {
            return ScreenAction::None;
        }
        self.map_over_input(input);
        ScreenAction::None
    }

    /// Whether flight's course map is open over the spaceport: the bar's
    /// map ([`FlightView::open_bar_map`]) or a mission dialog's
    /// ([`FlightView::open_dialog_map`]).
    pub(super) fn port_map_open(&self) -> bool {
        self.side == Side::Spaceport && self.flight.as_ref().is_some_and(FlightView::map_open)
    }

    /// The ship browser, shown or not.
    #[must_use]
    pub fn ship_browser(&self) -> &ShipBrowser<Rc<GameData>> {
        &self.ships
    }

    /// The galaxy map, shown or not.
    #[must_use]
    pub fn galaxy_map(&self) -> &GalaxyMap {
        self.galaxy.map()
    }

    /// The open system's view, shown or not, if a system is open.
    #[must_use]
    pub fn system_view(&self) -> Option<&SystemView> {
        self.galaxy.system()
    }

    /// The flight screen, shown or not, once flight has been entered.
    #[must_use]
    pub fn flight_view(&self) -> Option<&FlightView<Rc<GameData>>> {
        self.flight.as_ref()
    }

    /// The spaceport, while the ship is landed.
    #[must_use]
    pub fn spaceport_view(&self) -> Option<&SpaceportView> {
        self.spaceport.as_ref()
    }

    /// The galaxy side: the map and any open system.
    #[must_use]
    pub fn navigator(&self) -> &Navigator<Rc<GameData>> {
        &self.galaxy
    }

    /// The screen of the side shown: flight's while its course map is
    /// open over the spaceport ([`AppScreen::port_map_open`]), so the
    /// spaceport doesn't tick beneath it, paused as under any modal
    /// dialog, though it is drawn beneath it, as under the other modal
    /// dialogs. The bar's offer wait pausing under it too is ours'
    /// existing reading
    /// (rdm:phase/missions-fidelity/phase-15-offer-timer-under-nested-dialogs).
    pub(super) fn shown(&self) -> &dyn Screen {
        match self.side {
            Side::MainMenu => &self.menu.as_ref().expect(MENU).screen,
            Side::Ships => &self.ships,
            Side::Galaxy => &self.galaxy,
            Side::Flight => self.flight.as_ref().expect(ENTERED),
            Side::Spaceport if self.port_map_open() => self.flight.as_ref().expect(ENTERED),
            Side::Spaceport => self.spaceport.as_ref().expect(LANDED),
        }
    }

    /// The screen of the side shown, as [`AppScreen::shown`] gives it.
    pub(super) fn shown_mut(&mut self) -> &mut dyn Screen {
        let port_map = self.port_map_open();
        match self.side {
            Side::MainMenu => &mut self.menu.as_mut().expect(MENU).screen,
            Side::Ships => &mut self.ships,
            Side::Galaxy => &mut self.galaxy,
            Side::Flight => self.flight.as_mut().expect(ENTERED),
            Side::Spaceport if port_map => self.flight.as_mut().expect(ENTERED),
            Side::Spaceport => self.spaceport.as_mut().expect(LANDED),
        }
    }
}
