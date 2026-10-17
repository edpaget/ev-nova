//! The dialogs [`AppScreen`] opens over the screens and over flight:
//! About, the texts, Preferences, the mission dialogs, player info, offers,
//! comm, haggle, plunder and assignment.

use std::rc::Rc;

use nova_sim::{HailView, OfferPrompt, Take, Taken};
use nova_view::flight::FlightView;
use nova_view::spaceport::missions::INFO_DIALOG;
use nova_view::spaceport::offer::{
    OFFER_DIALOG, OfferChoice, OfferDialog, OfferTemplates, PICTURE_OFFER_DIALOG,
};
use nova_view::ui::comm::{COMM_DIALOG, HAGGLE_DIALOG};
use nova_view::ui::desc::{DESC_DIALOG, DescTemplates, PICTURE_DESC_DIALOG};
use nova_view::ui::mission_dialogs::MissionDialogs;
use nova_view::ui::player_info::{PLAYER_INFO_DIALOG, PlayerInfoScreen, YES_NO_DIALOG};
use nova_view::ui::plunder::{ASSIGNMENT_DIALOG, PLUNDER_DIALOG};
use nova_view::ui::prefs::PREFS_DIALOG;
use nova_view::ui::{
    AssignmentDialog, CommDialog, CommPress, DescriptionSource, DialogResources, HaggleDialog,
    PlunderDialog, PlunderShown, PrefsDialog,
};
use nova_view::{Input, Key, Screen, ScreenAction};

use super::port_dialogs::{abort_and_refresh, desc_dialog, start_up_words};
use super::{ABOUT_TEXT, AppScreen, ENTERED, MENU_X_TEXT, NO_INTERFACE, Side};

impl AppScreen {
    /// Hides the side shown, first cancelling its pointer gesture and
    /// letting go of its keys (their releases will go elsewhere), and shows
    /// `side`.
    pub(super) fn switch_to(&mut self, side: Side) {
        let hidden = self.shown_mut();
        hidden.cancel_pointer();
        hidden.release_keys();
        self.side = side;
    }

    /// Shows flight, building it the first time, and remembers the side to
    /// go back to.
    pub(super) fn enter_flight(&mut self) {
        if self.flight.is_none() {
            self.flight = Some(self.flight(FlightView::new));
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
    /// and munged as before any pilot is loaded ([`start_up_words`]), as
    /// `_DoAboutBox` and `_MainScreenKey` do before `_ShowTextDialog`.
    pub(super) fn open_text(&mut self, id: i16) {
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        let rules = nova_sim::text_parse::ParseRules::from_rulebook(&self.rules);
        let dialog = self.data.description(id).and_then(|text| {
            let text = if id == MENU_X_TEXT || id == ABOUT_TEXT {
                start_up_words(&text, &*self.data, rules)
            } else {
                text
            };
            desc_dialog(dialogs, &self.data, &text)
        });
        match dialog {
            Ok(dialog) => {
                let below = self.shown_mut();
                below.cancel_pointer();
                below.release_keys();
                self.about = Some(dialog);
                self.about_text = id;
            }
            Err(reason) => eprintln!("nova: cannot show dësc {id}: {reason}"),
        }
    }

    /// Opens the Preferences dialog over the side shown, on the player's
    /// preferences, first cancelling the side's pointer gesture and
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
            if let Some(flight) = &mut self.flight {
                flight.set_double_time(self.double_time);
            }
        }
    }

    /// The Preferences dialog's input: each change is kept, to be taken
    /// once, and the dialog closes once OK is activated.
    pub(super) fn preferences_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(dialog) = &mut self.preferences {
            dialog.input(input);
            if let Some(prefs) = dialog.take_change() {
                self.prefs = prefs;
                self.prefs_change = Some(prefs);
                self.movies.set_on(prefs.quicktime_movies);
                if let Some(flight) = &mut self.flight {
                    flight.set_hyperspace_effects(prefs.hyperspace_effects);
                    flight.set_sound_prefs(prefs.sound);
                }
            }
            if dialog.closed() {
                self.sounds.extend(dialog.take_sounds());
                self.preferences = None;
            }
        }
        ScreenAction::None
    }

    /// The overlay open over the side shown, if any: the About dialog, the
    /// Preferences dialog, the New Pilot dialog, the saved pilots' list or
    /// the load's note.
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
        if let Some(dialogs) = &mut self.missions {
            return Some(dialogs);
        }
        if let Some(dialog) = &mut self.offer {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.plunder {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.assignment {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.new_pilot {
            return Some(dialog);
        }
        if let Some(list) = &mut self.open_pilot {
            return Some(list);
        }
        self.load_note.as_mut().map(|note| note as &mut dyn Screen)
    }

    /// The About dialog's input; it closes once Done is activated.
    pub(super) fn about_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(about) = &mut self.about {
            about.input(input);
            if about.closed() {
                self.sounds.extend(about.take_sounds());
                self.about = None;
            }
        }
        ScreenAction::None
    }

    /// Flight's input: an Escape press closes flight's map when it is
    /// open (unless its Find is, which takes it), and otherwise goes
    /// back; everything else goes to flight
    /// (where L, the stellar slot keys and a click select the navigation
    /// target). When it lands, the
    /// spaceport shows.
    pub(super) fn flight_input(&mut self, input: &Input) -> ScreenAction {
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
                    // The pilot is put away: saved, and flight dropped, so
                    // the developer's F flies a fresh, unnamed pilot.
                    self.end_flight(Side::MainMenu);
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
        if let Some(shown) = self.flight.as_mut().and_then(FlightView::take_boarding) {
            self.open_plunder(&shown);
        }
        if let Some(view) = self.flight.as_mut().and_then(FlightView::take_hail) {
            self.open_comm(&view);
        }
        if let Some(prompt) = self.flight.as_mut().and_then(FlightView::take_offer) {
            self.open_offer(prompt);
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
        self.drop_closed_missions();
        ScreenAction::None
    }

    /// The mission dialogs over flight, built from the interface file's
    /// "Desc Dialog" and mission info screen the first time they are
    /// needed; none without dialogs.
    pub(super) fn flight_missions(&mut self) -> Option<&mut MissionDialogs> {
        if self.missions.is_none() {
            let dialogs = self.dialogs.as_ref()?;
            self.missions = Some(
                MissionDialogs::new(
                    DescTemplates {
                        plain: dialogs.resources.dialog_template(DESC_DIALOG),
                        picture: dialogs.resources.dialog_template(PICTURE_DESC_DIALOG),
                    },
                    dialogs.resources.dialog_template(INFO_DIALOG),
                    Rc::clone(&dialogs.metrics),
                    self.data.button_style(),
                )
                .with_movies(self.movies.clone()),
            );
        }
        self.missions.as_mut()
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
        if let (Some(missions), Some(flight)) = (&mut self.missions, &mut self.flight) {
            for _ in 0..missions.take_texts_closed() {
                flight.close_mission_text();
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
    /// saying why not. The screen below lets go of its keys. With no
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
        self.player_info = Some(PlayerInfoScreen::new(
            layout,
            confirm,
            info,
            self.data.button_style(),
        ));
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

    /// Opens the offer dialog on `prompt`, a person's mission offered on
    /// its hail or boarding, over flight, which pauses: the interface
    /// file's, in the variant its graphic picks
    /// ([`OfferTemplates::layout`]), or, without it or the metrics to lay
    /// it out, the dialog's own note that Return accepts it.
    pub(super) fn open_offer(&mut self, prompt: OfferPrompt) {
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
            OfferDialog::new(layout, prompt, self.data.button_style()).with_movies(&self.movies),
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
    /// ends, on a tick): the player info screen opens over it; or the
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
        self.comm = Some(dialog);
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
    fn open_haggle(&mut self, price: i64, pay_me: bool) {
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
        self.haggle = Some(dialog);
    }

    /// The haggle dialog's input: the choice goes through flight; a price
    /// lowered is shown, and otherwise the dialog closes, keeping its
    /// sounds, and the comm dialog shows what came of it.
    pub(super) fn haggle_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.haggle.as_mut().expect("open");
        dialog.input(input);
        let Some(choice) = dialog.take_choice() else {
            return ScreenAction::None;
        };
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
    /// the boarding is let go, with a warning.
    fn open_plunder(&mut self, shown: &PlunderShown) {
        let Some(metrics) = self.metrics() else {
            if let Some(flight) = &mut self.flight {
                flight.plunder(Take::Abort);
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
        self.plunder = Some(dialog);
    }

    /// The plunder dialog's input: each press goes through flight, and
    /// the dialog shows what is still on board, or closes once the
    /// boarding is over; a capture awaiting its assignment opens the
    /// assignment dialog.
    pub(super) fn plunder_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.plunder.as_mut().expect("open");
        dialog.input(input);
        let Some(take) = dialog.take_take() else {
            return ScreenAction::None;
        };
        let flight = self.flight.as_mut().expect(ENTERED);
        let taken = flight.plunder(take);
        match flight.plunder_shown() {
            Some(shown) => {
                if let Some(dialog) = &mut self.plunder {
                    dialog.set_plunder(&shown);
                }
            }
            None => {
                if let Some(mut dialog) = self.plunder.take() {
                    self.sounds.extend(dialog.take_sounds());
                }
            }
        }
        if taken == Taken::Captured {
            self.open_assignment();
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
        self.assignment = Some(dialog);
    }

    /// The assignment dialog's input: the choice goes through flight, and
    /// the dialog closes, keeping its sounds.
    pub(super) fn assignment_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.assignment.as_mut().expect("open");
        dialog.input(input);
        if let Some(choice) = dialog.take_choice() {
            self.sounds.extend(dialog.take_sounds());
            self.assignment = None;
            self.flight.as_mut().expect(ENTERED).assign(choice);
        }
        ScreenAction::None
    }
}
