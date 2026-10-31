//! The dialogs [`AppScreen`] opens over the screens and over flight:
//! About, the texts, Preferences, the mission dialogs, player info, offers,
//! comm, haggle, plunder and assignment (and the rename after it, in the
//! `capture_naming` module).

use std::rc::Rc;

use nova_sim::{HailView, OfferPrompt, RuleKey, RuleSource, Taken};
use nova_view::flight::{FlightStart, FlightView, MapOpener};
use nova_view::spaceport::missions::INFO_DIALOG;
use nova_view::spaceport::offer::{
    OFFER_DIALOG, OfferChoice, OfferDialog, OfferTemplates, PICTURE_OFFER_DIALOG,
};
use nova_view::ui::catalog::DescriptionGraphics;
use nova_view::ui::comm::{COMM_DIALOG, HAGGLE_DIALOG};
use nova_view::ui::desc::{DESC_DIALOG, DescTemplates, PICTURE_DESC_DIALOG};
use nova_view::ui::mission_dialogs::{DescShown, MissionDialogs};
use nova_view::ui::player_info::{
    BackgroundSizes, PLAYER_INFO_DIALOG, PlayerInfoScreen, YES_NO_DIALOG,
};
use nova_view::ui::plunder::{ASSIGNMENT_DIALOG, PLUNDER_DIALOG};
use nova_view::ui::prefs::PREFS_DIALOG;
use nova_view::ui::{
    AssignmentDialog, CommDialog, CommPress, DescriptionSource, DialogResources, FocusRing,
    HaggleDialog, PlunderDialog, PlunderShown, PrefsDialog,
};
use nova_view::{FullscreenKind, Input, Key, Screen, ScreenAction, VideoMode};

use super::port_dialogs::{abort_and_refresh, start_up_words};
use super::{ABOUT_TEXT, AppScreen, ENTERED, MENU_X_TEXT, NO_INTERFACE, Side};
use crate::app::keyboard_access::prefs_focus_ring;

impl AppScreen {
    /// Whether the in-game dialogs' buttons take the keyboard focus, as
    /// the rulebook reads `dialog_focus_ring`: by default they do.
    pub(super) fn dialog_focus_ring(&self) -> FocusRing {
        FocusRing::from_rulebook(&self.rules)
    }

    /// Hides the side shown, first cancelling its pointer gesture and
    /// letting go of its keys (their releases will go elsewhere), and shows
    /// `side`, telling the movies the view it shows.
    pub(super) fn switch_to(&mut self, side: Side) {
        let hidden = self.shown_mut();
        hidden.cancel_pointer();
        hidden.release_keys();
        self.side = side;
        self.movies.set_view(self.movie_view());
    }

    /// Shows flight, building it the first time, and remembers the side to
    /// go back to.
    pub(super) fn enter_flight(&mut self) {
        if self.flight.is_none() {
            self.flight = Some(self.flight(FlightStart::New, None));
        }
        self.return_to = self.side;
        self.switch_to(Side::Flight);
    }

    /// Opens the About dialog over the side shown, first cancelling its
    /// pointer gesture and letting go of its keys (their releases will go
    /// to the dialog). With no dialogs, nothing opens; when the dialog
    /// cannot be built, nothing opens and the reason goes to stderr.
    pub(super) fn open_about(&mut self) {
        self.open_text(ABOUT_TEXT);
    }

    /// Opens "Desc Dialog" on `dësc` `id` over the side shown, as
    /// [`open_about`](Self::open_about) opens the About text. The About
    /// text and the main menu's X text ([`MENU_X_TEXT`]) are first loaded
    /// and munged as `_DoAboutBox` and `_MainScreenKey` do before
    /// `_ShowTextDialog` ([`AppScreen::main_screen_words`]). Each is shown
    /// with its third argument 1 (@0x16062, @0x1acde), so it plays its own
    /// `dësc`'s movie before it or, `Flags` 0x0001 set, after it, whatever
    /// `desc_movie_globals` says, and a `Graphic` above 127 shows it in the
    /// picture variant: through [`MissionDialogs`], which says how. An
    /// empty text with no movie before it shows nothing (@0x5a7b0).
    pub(super) fn open_text(&mut self, id: i16) {
        if self.dialogs.is_none() {
            return;
        }
        let text = if id == MENU_X_TEXT || id == ABOUT_TEXT {
            self.main_screen_words(id)
        } else {
            self.data.description(id)
        };
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        let checked =
            text.and_then(|text| dialogs.resources.dialog_template(DESC_DIALOG).map(|_| text));
        let text = match checked {
            Ok(text) => text,
            Err(reason) => {
                eprintln!("nova: cannot show dësc {id}: {reason}");
                return;
            }
        };
        let Some(mut queue) = self.text_dialogs() else {
            return;
        };
        if self.white_faded() {
            queue.set_faded();
        }
        queue.show_descs(vec![DescShown {
            text,
            graphic: self.data.description_graphic(id),
            movie: self.data.description_movie(id),
        }]);
        if !queue.is_open() {
            return;
        }
        let below = self.shown_mut();
        below.cancel_pointer();
        below.release_keys();
        self.about = Some(queue);
        self.about_text = id;
        self.tell_faded_in();
    }

    /// Drops the About text once it and its movies are done, keeping its
    /// sounds. What was pressed under its last movie is dropped with it:
    /// the side beneath let go of its keys as it opened.
    pub(super) fn drop_closed_about(&mut self) {
        self.tell_faded_in();
        if let Some(about) = &mut self.about
            && !about.is_open()
        {
            about.take_unclaimed();
            self.sounds.extend(about.take_sounds());
            self.about = None;
        }
    }

    /// The About text or the X text `id` as the original loads and munges
    /// it: with a game in play (flight or the spaceport shown) or held,
    /// whose session started, its conditional text resolved for that
    /// pilot (`_LoadDescription`, about @0x16021, X @0x1ac9d, through
    /// [`FlightView::load_description`]: the rank governments reset, the
    /// `l33t` mode's draws, and the last `dësc` loaded, whose after-movie a
    /// later `STR#` text plays) and its tags filled for it with no mission
    /// (`_MungeBriefing(0, -1)`, about @0x16035, X @0x1acb1,
    /// [`nova_sim::Session::plain_text`]), as the original reads the
    /// player globals of the pilot loaded; with none, as before any pilot
    /// ([`start_up_words`]).
    pub(super) fn main_screen_words(&mut self, id: i16) -> Result<String, String> {
        let game = match self.side {
            Side::Flight | Side::Spaceport => self.flight.as_mut(),
            Side::MainMenu | Side::Ships | Side::Galaxy => self.game.as_mut(),
        };
        if let Some(game) = game.filter(|game| game.failure().is_none()) {
            let text = game.load_description(id)?;
            return Ok(game.plain_text(&text));
        }
        let rules = nova_sim::text_parse::ParseRules::from_rulebook(&self.rules);
        self.data
            .description(id)
            .map(|text| start_up_words(&text, &*self.data, rules))
    }

    /// Opens the Preferences dialog over the side shown, on the player's
    /// preferences, listing the Scaling offered (native and Super-xBR
    /// before an offer), its focus ring as the keyboard access port
    /// answers now ([`prefs_focus_ring`]), first cancelling the side's pointer gesture and
    /// letting go of its keys. With no dialogs, nothing opens; when the
    /// dialog cannot be built, nothing opens and the reason goes to stderr.
    pub(super) fn open_preferences(&mut self) {
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        let dialog = dialogs
            .resources
            .dialog_template(PREFS_DIALOG)
            .and_then(|template| {
                PrefsDialog::new(
                    &template,
                    self.prefs,
                    self.data.button_style(),
                    Rc::clone(&dialogs.metrics),
                )
            })
            .map(|dialog| {
                let access = self.keyboard_access.full_keyboard_access();
                dialog
                    .with_scaling(&self.scaling)
                    .with_focus_ring(prefs_focus_ring(access))
            })
            .map(|dialog| {
                if self.launch_prefs_wait() {
                    dialog.with_launch(self.launch)
                } else {
                    dialog
                }
            });
        match dialog {
            Ok(dialog) => {
                let below = self.shown_mut();
                below.cancel_pointer();
                below.release_keys();
                self.preferences = Some(dialog);
            }
            Err(reason) => eprintln!("nova: cannot show the preferences: {reason}"),
        }
    }

    /// Turns double time on or off on a Caps Lock press, not on its
    /// repeats nor its release, and tells flight, if built; a flight
    /// built later starts with it ([`FlightView::with_double_time`]).
    ///
    /// The original follows Caps Lock's lock state (`_QuickKeyCheck(0x39)`
    /// @0x463c3); ours toggles on each press the platform reports, an
    /// accepted difference. It drifts from the lock when Caps Lock is
    /// pressed while the window is not focused, or is already on at
    /// launch. On macOS winit 0.30.13 reports no Caps Lock key at all, so
    /// there double time never comes on.
    pub(super) fn double_time_key(&mut self, input: &Input) {
        if let Input::Key {
            key: Key::CapsLock,
            pressed: true,
            repeat: false,
        } = *input
        {
            self.double_time = !self.double_time;
            let on = self.double_time;
            for flight in self.flights_mut() {
                flight.set_double_time(on);
            }
        }
    }

    /// Keeps whether the frame-rate readout is shown, as the flight
    /// entered, if any, has it after an input, for the flights built
    /// later ([`FlightView::with_frame_rate_shown`]): only flight's own
    /// F12 changes it.
    pub(super) fn keep_frame_rate(&mut self) {
        if let Some(flight) = &self.flight {
            self.frame_rate_shown = flight.frame_rate_shown();
        }
    }

    /// Offers the open Preferences dialog `modes` for its display picker,
    /// on `current`, which the player's preferences take as the kind the
    /// game runs fullscreen in, the player having chosen `chosen`. With no
    /// dialog open, nothing changes.
    pub(super) fn offer_display_to_preferences(
        &mut self,
        modes: &[VideoMode],
        current: FullscreenKind,
        chosen: FullscreenKind,
    ) {
        if let Some(dialog) = &mut self.preferences {
            dialog.set_display_modes(modes, current, chosen);
            self.prefs.fullscreen = current;
        }
    }

    /// The Preferences dialog's input: each change, and the fullscreen
    /// kind picked as it closes, is kept, to be taken once, and the dialog
    /// closes once OK is activated.
    pub(super) fn preferences_input(&mut self, input: &Input) -> ScreenAction {
        let Some(dialog) = &mut self.preferences else {
            return ScreenAction::None;
        };
        dialog.input(input);
        let change = dialog.take_change();
        if let Some(kind) = dialog.take_display_choice() {
            self.display_choice = Some(kind);
        }
        if dialog.closed() {
            self.sounds.extend(dialog.take_sounds());
            self.preferences = None;
        }
        if let Some(prefs) = change {
            self.prefs = prefs;
            self.prefs_change = Some(prefs);
            self.movies.set_on(prefs.quicktime_movies);
            let launch = self.flight_launch_prefs();
            for flight in self.flights_mut() {
                flight.set_hyperspace_effects(prefs.hyperspace_effects);
                flight.set_smoke_trails(prefs.smoke_trails);
                flight.set_parallax_starfield(prefs.parallax_starfield);
                flight.set_weapon_effects(launch.weapon_effects);
                flight.set_engine_glows(launch.engine_glows);
                flight.set_sprite_load(launch.sprite_load());
                flight.set_sound_prefs(prefs.sound);
            }
        }
        ScreenAction::None
    }

    /// The overlay open over the side shown, if any: the About dialog, the
    /// Preferences dialog, the New Pilot dialog, the saved pilots' list,
    /// the load's note or the quit question, or one of flight's.
    pub(super) fn overlay_mut(&mut self) -> Option<&mut dyn Screen> {
        if let Some(about) = &mut self.about {
            return Some(about);
        }
        if let Some(dialog) = &mut self.preferences {
            return Some(dialog);
        }
        if let Some(screen) = &mut self.player_info {
            return Some(screen);
        }
        if let Some(dialog) = &mut self.haggle {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.comm {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.escort_dialog {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.planet_comm {
            return Some(dialog);
        }
        if let Some(dialogs) = &mut self.missions {
            return Some(dialogs);
        }
        if let Some(dialog) = &mut self.offer {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.capture_naming {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.assignment {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.plunder {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.new_pilot {
            return Some(dialog);
        }
        if let Some(open) = &mut self.christening {
            return Some(&mut open.dialog);
        }
        if let Some(list) = &mut self.open_pilot {
            return Some(list);
        }
        if let Some(note) = &mut self.load_note {
            return Some(note);
        }
        self.question
            .as_mut()
            .map(|(_, question)| question as &mut dyn Screen)
    }

    /// The About dialog's input; it closes once Done is activated and its
    /// after-movie, if any, has ended.
    pub(super) fn about_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(about) = &mut self.about {
            about.input(input);
        }
        self.drop_closed_about();
        ScreenAction::None
    }

    /// Flight's input: an Escape press closes flight's map when it is
    /// open (unless its Find is, which takes it), and otherwise goes
    /// back to the side flight was entered from; from the main menu, by
    /// [`RuleKey::EscapeToMainScreen`]: by the engine it saves the pilot
    /// and returns to the menu with it held, and by the other reading it
    /// does nothing, as the original's `_PlayGame` is left only by death
    /// or Quit. Everything else goes to flight
    /// (where L, the stellar slot keys and a click select the navigation
    /// target). When it lands, the
    /// spaceport shows.
    pub(super) fn flight_input(&mut self, input: &Input) -> ScreenAction {
        let flight = self.flight.as_mut().expect(ENTERED);
        if flight.playing() {
            flight.input(input);
            self.settle_flight_movie();
            return ScreenAction::None;
        }
        if let Input::Key {
            key: Key::Escape,
            pressed,
            repeat,
        } = *input
        {
            let flight = self.flight.as_mut().expect(ENTERED);
            // Find, while open, takes Escape too, doing nothing with it.
            if flight.map_finding() {
                flight.input(input);
                return ScreenAction::None;
            }
            if pressed && !repeat {
                if flight.map_open() {
                    flight.close_map();
                } else if self.return_to == Side::MainMenu {
                    // The pilot is saved and held, so Enter Ship resumes
                    // it; the main screen appears as on any return from
                    // play.
                    if self.rules.source_for(RuleKey::EscapeToMainScreen) == RuleSource::Engine {
                        self.return_to_main_screen();
                    }
                } else {
                    self.switch_to(self.return_to);
                }
            }
            return ScreenAction::None;
        }
        let flight = self.flight.as_mut().expect(ENTERED);
        flight.input(input);
        if let Some(stellar) = flight.take_landing() {
            self.show_spaceport(stellar);
        }
        self.open_frame_boarding();
        self.open_frame_dialogs();
        self.drop_closed_missions();
        ScreenAction::None
    }

    /// Opens what a key asked of flight, by a press or from flight's tick
    /// (`nova_view::flight::FlightView`'s `faded_keys` module: a key held
    /// past a jump): the comm dialog, the escort dialog, the planet comm
    /// dialog, the player info or the mission info.
    pub(super) fn open_frame_dialogs(&mut self) {
        if let Some(view) = self.flight.as_mut().and_then(FlightView::take_hail) {
            self.open_comm(&view);
        }
        if let Some(view) = self
            .flight
            .as_mut()
            .and_then(FlightView::take_escort_dialog)
        {
            self.open_escort_dialog(&view);
        }
        if let Some(view) = self.flight.as_mut().and_then(FlightView::take_planet_comm) {
            self.open_planet_comm(&view);
        }
        if self
            .flight
            .as_mut()
            .is_some_and(FlightView::take_player_info_request)
        {
            self.open_flight_player_info();
        }
        if let Some(info) = self
            .flight
            .as_mut()
            .and_then(FlightView::take_mission_info_request)
            && let Some(missions) = self.flight_missions()
        {
            missions.open_info(info);
            self.load_flight_info_text();
        }
    }

    /// The mission dialogs over flight, built from the interface file's
    /// "Desc Dialog" and mission info screen the first time they are
    /// needed; none without dialogs.
    pub(super) fn flight_missions(&mut self) -> Option<&mut MissionDialogs> {
        if self.missions.is_none() {
            self.missions = Some(self.text_dialogs()?);
        }
        self.missions.as_mut()
    }

    /// Text dialogs, nothing open, built from the interface file's "Desc
    /// Dialog", both variants, and mission info screen, playing movies as
    /// the preference says; none without dialogs.
    pub(super) fn text_dialogs(&self) -> Option<MissionDialogs> {
        let dialogs = self.dialogs.as_ref()?;
        Some(
            MissionDialogs::new(
                DescTemplates {
                    plain: dialogs.resources.dialog_template(DESC_DIALOG),
                    picture: dialogs.resources.dialog_template(PICTURE_DESC_DIALOG),
                },
                dialogs.resources.dialog_template(INFO_DIALOG),
                Rc::clone(&dialogs.metrics),
                self.data.button_style(),
            )
            .with_movies(self.movies.clone())
            .with_focus_ring(self.dialog_focus_ring()),
        )
    }

    /// Tells flight of each mission text over flight closed or dropped
    /// since it was last told ([`MissionDialogs::take_texts_closed`]), so
    /// it lets go of the bar a person's offer answered over flight holds
    /// under them ([`FlightView::close_mission_text`]); then drops the
    /// mission dialogs over flight once nothing is open, keeping their
    /// sounds, and begins the death's fade if the hold has expired
    /// ([`AppScreen::fade_once_dialogs_close`]).
    ///
    /// What was pressed under the last movie is dropped: flight reads its
    /// keys as held, and the router lets go of them as the dialogs open.
    pub(super) fn drop_closed_missions(&mut self) {
        self.tell_faded_in();
        if let (Some(missions), Some(flight)) = (&mut self.missions, &mut self.flight) {
            for _ in 0..missions.take_texts_closed() {
                flight.close_mission_text();
            }
            // The mission info's close over a person's offer loads the
            // offer's `dësc` again (`_OfferOneMission` @0xa335c), as the
            // port's does; the port-only `PortDialogUpdate`
            // (@0xa3377-0xa33d1) has nothing to do over flight. Taken
            // whatever opened it, so a close of flight's own I never
            // reaches a later offer.
            if missions.take_info_closed()
                && let Some(offer) = self.offer.as_ref().filter(|offer| !offer.answered())
            {
                flight.reload_offer_desc(offer.prompt().mission);
                self.release_keys();
            }
        }
        if let Some(missions) = &mut self.missions
            && !missions.is_open()
        {
            missions.take_unclaimed();
            self.sounds.extend(missions.take_sounds());
            self.missions = None;
            // The offer dialog left drawn under them is disposed of
            // (`DisposeDialogNoFlash` @0xa361e).
            if self.offer.as_ref().is_some_and(OfferDialog::answered) {
                self.offer = None;
            }
            self.fade_once_dialogs_close();
        }
    }

    /// The input of the mission dialogs over flight: an Abort on the
    /// mission info screen goes through flight, and the screen shows the
    /// missions left. Its close, laid out, redraws the whole status bar
    /// ([`FlightView::refresh_status`]), as `_DoMissionInfoDialog` ends in
    /// `_RefreshSAT` (@0x9e769). They are dropped once nothing is open.
    pub(super) fn missions_input(&mut self, input: &Input) -> ScreenAction {
        let missions = self.missions.as_mut().expect("open");
        missions.input(input);
        if let Some(opener) = missions.take_map_request() {
            missions.cancel_pointer();
            self.open_flight_dialog_map(opener);
            return ScreenAction::None;
        }
        let missions = self.missions.as_mut().expect("open");
        abort_and_refresh(missions, self.flight.as_mut());
        self.load_flight_info_text();
        if self
            .missions
            .as_mut()
            .is_some_and(MissionDialogs::take_info_closed_laid_out)
            && let Some(flight) = &mut self.flight
        {
            flight.refresh_status();
        }
        self.drop_closed_missions();
        ScreenAction::None
    }

    /// Opens the player info screen over flight, which pauses, and over
    /// any offer dialog open there, on what flight gives as the dialog
    /// opens ([`FlightView::open_player_info`], which resets the session
    /// first; nothing opens in a jump or while the ship breaks up): laid out by
    /// the interface file's `DLOG` 1017 and its Yes/No dialog, 3002, or
    /// saying why not, over the game data's background pictures
    /// ([`BackgroundSizes::read`]). The screen below lets go of its keys. With no
    /// dialogs, nothing opens, as the mission info does not.
    fn open_flight_player_info(&mut self) {
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        let Some(info) = self.flight.as_mut().and_then(FlightView::open_player_info) else {
            return;
        };
        let layout = dialogs
            .resources
            .dialog_template(PLAYER_INFO_DIALOG)
            .map(|template| (template, Rc::clone(&dialogs.metrics)));
        let confirm = dialogs.resources.dialog_template(YES_NO_DIALOG);
        self.cancel_pointer();
        self.release_keys();
        self.player_info = Some(
            PlayerInfoScreen::new(
                layout,
                confirm,
                info,
                self.data.button_style(),
                BackgroundSizes::read(&self.data),
            )
            .with_focus_ring(self.dialog_focus_ring()),
        );
    }

    /// The player info screen's input over flight: a confirmed Jettison
    /// jettisons everything through flight. Once it closes, the screen
    /// below lets go of its keys, and, laid out, it redraws the whole
    /// status bar ([`FlightView::refresh_status`]), as
    /// `_DoPlayerInfoDialog` ends in `_RefreshSAT` (@0x5a725).
    pub(super) fn player_info_input(&mut self, input: &Input) -> ScreenAction {
        let screen = self.player_info.as_mut().expect("open");
        screen.input(input);
        let jettison = screen.take_jettison();
        let closed = screen.closed();
        let laid_out = screen.problem().is_none();
        if jettison && let Some(flight) = &mut self.flight {
            flight.jettison_all();
        }
        if closed {
            self.player_info = None;
            self.release_keys();
            if laid_out && let Some(flight) = &mut self.flight {
                flight.refresh_status();
            }
        }
        ScreenAction::None
    }

    /// Opens what a board made in flight's tick, the Board key held
    /// (`nova_view::flight::FlightView`'s `board_frame`): the plunder
    /// dialog, or a person's mission offered in its place, as the press's
    /// board opens them (the flight branch of the router's input).
    pub(super) fn open_frame_boarding(&mut self) {
        if let Some(shown) = self.flight.as_mut().and_then(FlightView::take_boarding) {
            self.open_plunder(&shown);
        }
        if let Some(prompt) = self.flight.as_mut().and_then(FlightView::take_offer) {
            self.open_offer(prompt);
        }
    }

    /// Opens the offer dialog on `prompt`, a person's mission offered on
    /// its hail or boarding, over flight, which pauses: the interface
    /// file's, in the variant its graphic picks
    /// ([`OfferTemplates::layout`]), or, without it or the metrics to lay
    /// it out, the dialog's own note that Return accepts it. Flight's
    /// cursor goes first, as `_HandlePlayerCommunication` and
    /// `_HandlePlayerBoardAttempt` hide its sprite (@0x6225a, @0x658d6).
    /// Only the pointer is cancelled here: flight's `hold_offer` has
    /// already let go of its keys. The original keeps reading them live
    /// over the offer's close
    /// (rdm:phase/play-feel-7-fidelity/phase-2-flight-keys-still-down-after-a-dialog).
    pub(super) fn open_offer(&mut self, prompt: OfferPrompt) {
        if let Some(flight) = &mut self.flight {
            flight.cancel_pointer();
        }
        let layout = match (&self.dialogs, self.metrics()) {
            (Some(dialogs), Some(metrics)) => OfferTemplates {
                plain: dialogs.resources.dialog_template(OFFER_DIALOG),
                picture: dialogs.resources.dialog_template(PICTURE_OFFER_DIALOG),
            }
            .layout(prompt.graphic)
            .map(|template| (template, metrics)),
            _ => Err(NO_INTERFACE.to_owned()),
        };
        self.offer = Some(
            OfferDialog::new(layout, prompt, self.data.button_style())
                .with_focus_ring(self.dialog_focus_ring())
                .with_movies(&self.movies),
        );
    }

    /// The offer dialog's input, then what it asked for
    /// ([`AppScreen::settle_offer`]).
    pub(super) fn offer_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(dialog) = &mut self.offer {
            dialog.input(input);
        }
        self.settle_offer();
        ScreenAction::None
    }

    /// Acts on what the offer dialog over flight asked for, after an input
    /// or a tick (the keys pressed under its movie reach it as the movie
    /// ends, on a tick): the player info screen opens over it; the
    /// mission info screen opens over it, with no map, when a mission is
    /// shown there ([`AppScreen::open_offer_mission_info`]); or the
    /// choice goes through flight, and the dialog closes, keeping its
    /// sounds, and flight resumes. The texts the answer queued show over
    /// flight at once, the offer's after-movie among them where the
    /// session queued it.
    ///
    /// The dialog is hidden for an accept from the dialog with a refuse
    /// button (`_OfferOneMission` @0xa2df3) and for a refusal that showed
    /// a `RefuseText` (@0xa2e8e). Otherwise, the one-button dialog's accept
    /// (@0xa2f52-0xa2ff7) or a refusal with none, it stays drawn, answered
    /// and inert, under the mission dialogs the answer opened, until they
    /// have all closed (`DisposeDialogNoFlash` @0xa361e,
    /// [`AppScreen::drop_closed_missions`]).
    pub(super) fn settle_offer(&mut self) {
        let Some(dialog) = &mut self.offer else {
            return;
        };
        if dialog.take_player_info_request() {
            self.open_flight_player_info();
            return;
        }
        if dialog.take_info_request() {
            self.open_offer_mission_info();
            return;
        }
        if dialog.take_map_request() {
            let mission = dialog.prompt().mission;
            dialog.cancel_pointer();
            self.open_flight_dialog_map(MapOpener::Offer(mission));
            return;
        }
        let Some(choice) = dialog.take_choice() else {
            return;
        };
        let mission = dialog.prompt().mission;
        let two_buttons = dialog.prompt().labels.refuse.is_some();
        self.sounds.extend(dialog.take_sounds());
        let flight = self.flight.as_mut().expect(ENTERED);
        let hidden = match choice {
            OfferChoice::Accept => {
                let _ = flight.accept_mission(mission);
                two_buttons
            }
            OfferChoice::Refuse => flight
                .refuse_mission(mission)
                .is_ok_and(|shown| shown.offer_hidden),
        };
        if hidden {
            self.offer = None;
        }
        self.show_mission_texts();
        if self.missions.is_none() {
            self.offer = None;
        }
    }

    /// Opens the mission info screen over the person's offer open over
    /// flight, as `_OfferOneMission`'s item 7 does whoever offers
    /// (@0xa32b6, no test of `_offeringFromShip` or `_playerIsLanded`).
    /// With no active mission that is not hidden (the count
    /// @0xa32c1-0xa32ee), nothing happens: no flush, no sound, and the
    /// offer takes the next event. Otherwise the held keys and the
    /// pointer are let go (the four `FlushEvents` @0xa32f4-0xa333f) and
    /// the screen opens (@0xa3344) with no map, as `_missionDialog`
    /// holds the offer (`_DoMissionInfoDialog` @0x9e3c6-0x9e3cd). Its
    /// close reloads the offer's `dësc` ([`AppScreen::drop_closed_missions`]).
    fn open_offer_mission_info(&mut self) {
        let info = self.flight.as_mut().expect(ENTERED).mission_info();
        if info.rows.is_empty() {
            return;
        }
        if let Some(dialog) = &mut self.offer {
            dialog.cancel_pointer();
        }
        self.release_keys();
        if let Some(missions) = self.flight_missions() {
            missions.open_info_with_map(info, false);
        }
        self.load_flight_info_text();
    }

    /// Opens flight's course map for `opener`, a mission dialog over
    /// flight ([`FlightView::open_dialog_map`]): the offer from a ship,
    /// mission info or the accept's briefing, which stays open under it;
    /// the keys held let go. Its input goes to the map before them
    /// ([`AppScreen::flight_dialog_map_input`]).
    fn open_flight_dialog_map(&mut self, opener: MapOpener) {
        self.release_keys();
        if let Some(flight) = &mut self.flight {
            flight.open_dialog_map(opener);
        }
    }

    /// Opens the comm dialog on `view` over flight, which pauses: the
    /// interface file's, or the built-in one without it (with a warning
    /// when the interface file has none). With no metrics to lay it out,
    /// the hail is hung up, with a warning.
    fn open_comm(&mut self, view: &HailView) {
        let Some(metrics) = self.metrics() else {
            if let Some(flight) = &mut self.flight {
                flight.hang_up();
            }
            self.warnings
                .push("nova: cannot show the comm dialog: no metrics to lay it out".to_owned());
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(COMM_DIALOG)
                .and_then(|template| CommDialog::new(&template, view, style, Rc::clone(&metrics)))
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings
                        .push(format!("nova: using the built-in comm dialog: {reason}"));
                }
                CommDialog::fallback(view, style, metrics)
            }
        };
        self.comm = Some(dialog.with_focus_ring(self.dialog_focus_ring()));
    }

    /// The comm dialog's input: each option pressed goes through flight,
    /// and the dialog shows what came of it, opening the haggle dialog
    /// over it when a price is asked; Close Channel hangs up and resumes
    /// flight, keeping the dialog's sounds, as does a press with nothing
    /// to show (the ship gone).
    pub(super) fn comm_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.comm.as_mut().expect("open");
        dialog.input(input);
        let Some(press) = dialog.take_press() else {
            return ScreenAction::None;
        };
        let flight = self.flight.as_mut().expect(ENTERED);
        let shown = match press {
            CommPress::Option(place) => flight.answer(place),
            CommPress::Close => None,
        };
        if let Some(view) = shown {
            if let Some(dialog) = &mut self.comm {
                dialog.set_hail(&view);
            }
            if let Some(price) = view.asking {
                self.open_haggle(price, view.pay_me);
            }
        } else {
            flight.hang_up();
            if let Some(mut dialog) = self.comm.take() {
                self.sounds.extend(dialog.take_sounds());
            }
        }
        ScreenAction::None
    }

    /// Opens the haggle dialog asking `price` over the comm dialog: the
    /// interface file's, or the built-in one without it.
    pub(super) fn open_haggle(&mut self, price: i64, pay_me: bool) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(HAGGLE_DIALOG)
                .and_then(|template| {
                    HaggleDialog::new(&template, price, pay_me, style, Rc::clone(&metrics))
                })
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings
                        .push(format!("nova: using the built-in haggle dialog: {reason}"));
                }
                HaggleDialog::fallback(price, pay_me, style, metrics)
            }
        };
        self.haggle = Some(dialog.with_focus_ring(self.dialog_focus_ring()));
    }

    /// The haggle dialog's input: the choice goes through flight; a price
    /// lowered is shown, and otherwise the dialog closes, keeping its
    /// sounds, and the comm dialog shows what came of it. Over the planet
    /// comm dialog it is the bribe's haggle (see the `planet_comm`
    /// module).
    pub(super) fn haggle_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.haggle.as_mut().expect("open");
        dialog.input(input);
        let Some(choice) = dialog.take_choice() else {
            return ScreenAction::None;
        };
        if self.planet_comm.is_some() {
            self.bribe_haggled(choice);
            return ScreenAction::None;
        }
        let flight = self.flight.as_mut().expect(ENTERED);
        let shown = flight.haggle(choice);
        if let Some(price) = shown.as_ref().and_then(|view| view.asking) {
            let pay_me = shown.as_ref().is_none_or(|view| view.pay_me);
            if let Some(dialog) = &mut self.haggle {
                dialog.set_asking(price, pay_me);
            }
            return ScreenAction::None;
        }
        if let Some(mut dialog) = self.haggle.take() {
            self.sounds.extend(dialog.take_sounds());
        }
        if let Some(view) = shown.or_else(|| flight.hailing()) {
            if let Some(dialog) = &mut self.comm {
                dialog.set_hail(&view);
            }
        } else {
            flight.hang_up();
            if let Some(mut dialog) = self.comm.take() {
                self.sounds.extend(dialog.take_sounds());
            }
        }
        ScreenAction::None
    }

    /// Opens the plunder dialog on `shown` over flight, which pauses: the
    /// interface file's, or the built-in one without it (with a warning
    /// when the interface file has none). With no metrics to lay it out,
    /// the boarding is let go silently, with a warning.
    fn open_plunder(&mut self, shown: &PlunderShown) {
        let Some(metrics) = self.metrics() else {
            if let Some(flight) = &mut self.flight {
                flight.let_go_of_boarding();
            }
            self.warnings
                .push("nova: cannot show the plunder dialog: no metrics to lay it out".to_owned());
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(PLUNDER_DIALOG)
                .and_then(|template| {
                    PlunderDialog::new(&template, shown, style, Rc::clone(&metrics))
                })
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings
                        .push(format!("nova: using the built-in plunder dialog: {reason}"));
                }
                PlunderDialog::fallback(shown, style, metrics)
            }
        };
        self.plunder = Some(dialog.with_focus_ring(self.dialog_focus_ring()));
    }

    /// The plunder dialog's input: each press goes through flight, a
    /// miss of every enabled button included, with the time the dialog
    /// has been open, and the dialog shows what is still on board, or
    /// closes once the boarding is over; a capture awaiting its
    /// assignment opens the assignment dialog over it, the plunder dialog
    /// staying drawn under it (see the `capture_naming` module). Every pointer event goes
    /// to the dialog, so a click outside its frame is a miss too, as
    /// `_PlunderFilter` takes every mouse-down (@0x92f20).
    pub(super) fn plunder_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.plunder.as_mut().expect("open");
        dialog.input(input);
        let Some(take) = dialog.take_take() else {
            return ScreenAction::None;
        };
        let held = dialog.held();
        let flight = self.flight.as_mut().expect(ENTERED);
        let taken = flight.plunder_held(take, held);
        if taken == Taken::Captured {
            self.open_assignment();
            return ScreenAction::None;
        }
        match flight.plunder_shown() {
            Some(shown) => {
                if let Some(dialog) = &mut self.plunder {
                    dialog.set_plunder(&shown);
                }
            }
            None => self.close_plunder(),
        }
        ScreenAction::None
    }

    /// Opens the captured-ship assignment dialog over flight: the
    /// interface file's, or the built-in one without it.
    fn open_assignment(&mut self) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(ASSIGNMENT_DIALOG)
                .and_then(|template| AssignmentDialog::new(&template, style, Rc::clone(&metrics)))
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings.push(format!(
                        "nova: using the built-in assignment dialog: {reason}"
                    ));
                }
                AssignmentDialog::fallback(style, metrics)
            }
        };
        self.assignment = Some(dialog.with_focus_ring(self.dialog_focus_ring()));
    }

    /// The assignment dialog's input: the choice goes through flight, and
    /// the dialog closes, keeping its sounds. Use As My Ship opens the
    /// rename over the plunder dialog, the key that chose it typing
    /// nothing into it; otherwise the plunder dialog closes too.
    pub(super) fn assignment_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.assignment.as_mut().expect("open");
        dialog.input(input);
        if let Some(choice) = dialog.take_choice() {
            self.sounds.extend(dialog.take_sounds());
            self.assignment = None;
            let assigned = self.flight.as_mut().expect(ENTERED).assign(choice);
            if assigned == Some(nova_sim::Assigned::Naming) {
                self.open_capture_naming(matches!(input, Input::Key { .. }));
            } else {
                self.close_plunder();
            }
        }
        ScreenAction::None
    }
}
