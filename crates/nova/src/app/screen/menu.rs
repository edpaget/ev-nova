//! [`AppScreen`]'s main menu: its input and actions, the New Pilot
//! dialog, the pilot list and the load note.

use std::rc::Rc;

use nova_sim::{Pilot, pilot_key};
use nova_view::flight::FlightView;
use nova_view::menu::{Chosen, ChosenBy, MenuChoice, PilotList, PilotListOutcome};
use nova_view::ui::new_pilot::{NAME_TAKEN, NEW_PILOT_DIALOG, NewPilotDialog, NewPilotOutcome};
use nova_view::ui::note::{LOAD_FALLBACK_NOTE, NOTE_DIALOG, NoteDialog};
use nova_view::ui::{DescriptionSource, DialogResources};
use nova_view::{Input, Key, Screen, ScreenAction, Sound, UiSound};

use super::{
    ABOUT_TEXT, AppScreen, Held, MENU, MENU_X_TEXT, MenuAction, MenuHold, QUIT_DELAY, Side,
    UNUSABLE_NAME,
};

impl AppScreen {
    /// The main menu's input, as `_MainScreenKey` @0x1aba0 and
    /// `_MainScreenMouse` read it.
    ///
    /// A click on New Pilot or Open Pilot opens the New Pilot dialog or
    /// the saved pilots' list over the menu at once (its sounds have
    /// played, under the pointer). A click on Quit waits out
    /// [`QUIT_DELAY`] and then quits ([`Screen::take_quit`]), as
    /// `_MainScreenMouse` (@0x1aea4-0x1aead) reaches the same
    /// `_MainScreenChoice(2)` as the key. The engine's Escape quits at
    /// once, silently. Tab goes to the ship browser.
    ///
    /// Each key clicks ([`UiSound::ButtonKeyed`]) and its choice waits
    /// for the click to end ([`MenuHold`]): typed N, O and Q (the menu's
    /// own), and, with dialogs, A (the About text, @0x1ac70) and X
    /// (`dësc` 32766, @0x1ac7e, munged as before any pilot). P is the
    /// router's ([`preferences_key`](Self::preferences_key)).
    ///
    /// - O does nothing while the physical S or L key is held
    ///   (@0x1abd3-0x1abf5). No other code reads those two keys, so the
    ///   guard's purpose is unknown; it is built as read.
    /// - Typed E with the physical E key held waits for its release
    ///   (@0x1ac32-0x1ac57) before it clicks; typed with the E key up
    ///   (another layout) it clicks at once. Then it is Enter Ship
    ///   (`_MainScreenChoice(3)`), which this menu, never holding a
    ///   loaded pilot, always refuses: [`UiSound::EnterRefused`]. Choice
    ///   3's entering the game and its dying-pilot refusal are left out
    ///   (rdm:task/main-screen-loaded-pilot).
    ///
    /// With Command held, the typed keys and A and X go to the
    /// original's menus and do nothing here.
    pub(super) fn menu_input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: key @ Key::Char('a' | 'x'),
            pressed,
            repeat,
        } = *input
            && self.dialogs.is_some()
            && !self.command_held
        {
            if pressed && !repeat {
                self.click_then(if key == Key::Char('a') {
                    MenuAction::About
                } else {
                    MenuAction::Text
                });
            }
            return ScreenAction::None;
        }
        if let Input::Key {
            key: Key::Tab,
            pressed,
            repeat,
        } = *input
        {
            if pressed && !repeat {
                self.switch_to(Side::Ships);
            }
            return ScreenAction::None;
        }
        // A Command key goes to the original's menus (`MenuKey`, offered
        // first by `_DoMainScreen` @0x1b259-0x1b2bb), never to
        // `_MainScreenKey`, so it neither clicks nor chooses.
        if matches!(input, Input::Text(_)) && self.command_held {
            return ScreenAction::None;
        }
        match *input {
            Input::Text('e' | 'E') => {
                if self.keys_held.e {
                    self.menu_hold = Some(MenuHold::EHeld);
                } else {
                    self.click_then(MenuAction::Enter);
                }
                return ScreenAction::None;
            }
            Input::Text('o' | 'O') if self.keys_held.s || self.keys_held.l => {
                return ScreenAction::None;
            }
            _ => {}
        }
        let menu = self.menu.as_mut().expect(MENU);
        menu.screen.input(input);
        match menu.screen.take_choice() {
            Some(Chosen {
                choice,
                by: ChosenBy::Shortcut,
            }) => self.menu_hold = Some(MenuHold::Click(MenuAction::Choice(choice))),
            Some(Chosen {
                choice: MenuChoice::Quit,
                by: ChosenBy::Escape,
            }) => return ScreenAction::Quit,
            Some(Chosen { choice, .. }) => self.run_menu_action(MenuAction::Choice(choice)),
            None => {}
        }
        ScreenAction::None
    }

    /// Clicks a main-menu key ([`UiSound::ButtonKeyed`]) and holds
    /// `action` until the click has played.
    pub(super) fn click_then(&mut self, action: MenuAction) {
        self.sounds.push(Sound::Ui(UiSound::ButtonKeyed));
        self.menu_hold = Some(MenuHold::Click(action));
    }

    /// Runs a main-menu `action`, as `_MainScreenChoice` @0x1a72a does.
    pub(super) fn run_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Choice(MenuChoice::NewPilot) => self.open_new_pilot(),
            MenuAction::Choice(MenuChoice::OpenPilot) => self.open_pilot_list(),
            MenuAction::Choice(MenuChoice::Quit) => self.quit_in = Some(QUIT_DELAY),
            // `_MainScreenChoice(3)` with no game in progress beeps
            // `_beepSnd[3]` and redraws (@0x1aa89-0x1aac2).
            MenuAction::Enter => self.sounds.push(Sound::Ui(UiSound::EnterRefused)),
            MenuAction::Preferences => self.open_preferences(),
            MenuAction::About => self.open_text(ABOUT_TEXT),
            MenuAction::Text => self.open_text(MENU_X_TEXT),
        }
    }

    /// An input that arrives while the main menu waits ([`MenuHold`]):
    /// queued for the replay, but for E's own while it is held. E's
    /// release clicks and turns the wait into the click's; E's repeats
    /// and their text are dropped (whether the OS queues autoKeys during
    /// the original's spin is not read, and replaying them would type E
    /// again).
    pub(super) fn hold_input(&mut self, hold: MenuHold, input: &Input) {
        if hold == MenuHold::EHeld {
            match *input {
                Input::Key {
                    key: Key::Char('e'),
                    pressed: false,
                    ..
                } => {
                    self.e_released();
                    return;
                }
                Input::Key {
                    key: Key::Char('e'),
                    pressed: true,
                    ..
                }
                | Input::Text('e' | 'E') => return,
                _ => {}
            }
        }
        self.held.push(Held::Input(*input));
    }

    /// E is let go of while E's wait holds: it clicks, and Enter Ship
    /// waits on the click.
    pub(super) fn e_released(&mut self) {
        self.keys_held.e = false;
        self.click_then(MenuAction::Enter);
    }

    /// Opens the New Pilot dialog: the interface file's, or the built-in
    /// one without it (with a warning when the interface file has none).
    fn open_new_pilot(&mut self) {
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(NEW_PILOT_DIALOG)
                .and_then(|template| {
                    NewPilotDialog::new(&template, style, Rc::clone(&dialogs.metrics))
                })
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings.push(format!(
                        "nova: using the built-in New Pilot dialog: {reason}"
                    ));
                }
                let metrics = Rc::clone(&self.menu.as_ref().expect(MENU).metrics);
                NewPilotDialog::fallback(style, metrics)
            }
        };
        // Chosen by a click let go of on its button, the menu below holds
        // no press and no keys to let go of.
        self.new_pilot = Some(dialog);
    }

    /// Opens the saved pilots' list; when they cannot be listed, it shows
    /// why, and so does a warning.
    fn open_pilot_list(&mut self) {
        let (keys, error) = match self.keeper().map(|keeper| keeper.list()) {
            Some(Ok(keys)) => (keys, None),
            Some(Err(error)) => (Vec::new(), Some(error)),
            None => (Vec::new(), None),
        };
        let metrics = Rc::clone(&self.menu.as_ref().expect(MENU).metrics);
        let mut list = PilotList::new(keys, self.data.button_style(), metrics);
        if let Some(error) = error {
            list.show_error(&error);
            self.warnings.push(error);
        }
        self.open_pilot = Some(list);
    }

    /// The New Pilot dialog's input. A name already saved, or one no file
    /// can be saved under, is refused; any other creates the pilot with the
    /// gender and Strict Play chosen and flies it, begins its game (the `chär`'s `OnStart`,
    /// [`FlightView::begin`]) and saves it at once, with what that did (so
    /// Open Pilot lists it before it lands), as `_DoNewPilot` runs
    /// `OnStart` before the first save.
    pub(super) fn new_pilot_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.new_pilot.as_mut().expect("open");
        dialog.input(input);
        let outcome = dialog.take_outcome();
        match outcome {
            Some(NewPilotOutcome::Cancel) => self.close_new_pilot(),
            Some(NewPilotOutcome::Create {
                name,
                gender,
                strict_play,
            }) => {
                let keeper = self.keeper();
                let refusal = if pilot_key(&name).is_none() {
                    Some(UNUSABLE_NAME.to_owned())
                } else if keeper.as_ref().is_some_and(|keeper| keeper.exists(&name)) {
                    Some(NAME_TAKEN.to_owned())
                } else {
                    None
                };
                if let Some(refusal) = refusal {
                    self.new_pilot.as_mut().expect("open").refuse(&refusal);
                    return ScreenAction::None;
                }
                match Pilot::new(self.data.as_ref(), &name) {
                    Ok(pilot) => {
                        self.close_new_pilot();
                        self.start_flight(pilot.with_gender(gender).with_strict_play(strict_play));
                        if let Some(flight) = &mut self.flight {
                            flight.begin();
                            flight.take_save_due();
                        }
                        self.save_pilot();
                    }
                    Err(error) => {
                        let why = error.to_string();
                        self.new_pilot.as_mut().expect("open").refuse(&why);
                        self.warnings
                            .push(format!("nova: cannot create a pilot: {why}"));
                    }
                }
            }
            None => {}
        }
        ScreenAction::None
    }

    /// Closes the New Pilot dialog, keeping its sounds.
    fn close_new_pilot(&mut self) {
        if let Some(mut dialog) = self.new_pilot.take() {
            self.sounds.extend(dialog.take_sounds());
        }
    }

    /// The saved pilots' list's input: the pilot chosen is opened and
    /// flown; one that cannot be opened says why in the list, and in a
    /// warning. One whose load fell back ([`FlightView::load_repaired`])
    /// has its flight built, faced and kept behind the note `STR#` 140 #52
    /// over the main menu, as `_MainScreenChoice` shows it through
    /// `_EVNoteAlert` over the main screen (@0x1a817-0x1a85b), so the note
    /// is seen before any flight frame; flight shows once it is dismissed
    /// ([`AppScreen::load_note_input`]). Without dialogs, or with no
    /// note to build, the note is a warning and flight shows at once.
    pub(super) fn open_pilot_input(&mut self, input: &Input) -> ScreenAction {
        let list = self.open_pilot.as_mut().expect("open");
        list.input(input);
        match list.take_outcome() {
            Some(PilotListOutcome::Cancel) => self.close_pilot_list(),
            Some(PilotListOutcome::Open(key)) => {
                let opened = match self.keeper() {
                    Some(keeper) => keeper.open(&key),
                    None => Err(format!(
                        "nova: there is nowhere to open the pilot {key} from"
                    )),
                };
                match opened {
                    Ok(pilot) => {
                        self.close_pilot_list();
                        self.build_flight(pilot, true);
                        // The load's heading, before any frame is drawn.
                        if let Some(flight) = &mut self.flight {
                            flight.resume();
                        }
                        if self.flight.as_ref().is_some_and(FlightView::load_repaired) {
                            match self.load_note_dialog() {
                                Some(note) => {
                                    let menu = self.shown_mut();
                                    menu.cancel_pointer();
                                    menu.release_keys();
                                    self.load_note = Some(note);
                                    return ScreenAction::None;
                                }
                                None => self.warnings.push(LOAD_FALLBACK_NOTE.to_owned()),
                            }
                        }
                        self.enter_built_flight();
                    }
                    Err(error) => {
                        self.open_pilot.as_mut().expect("open").show_error(&error);
                        self.warnings.push(error);
                    }
                }
            }
            None => {}
        }
        ScreenAction::None
    }

    /// The note `STR#` 140 #52 in the note alert (`DLOG` 3000), if the
    /// router has dialogs and the alert can be built.
    fn load_note_dialog(&self) -> Option<NoteDialog> {
        let dialogs = self.dialogs.as_ref()?;
        let template = dialogs.resources.dialog_template(NOTE_DIALOG).ok()?;
        Some(NoteDialog::new(
            &template,
            LOAD_FALLBACK_NOTE,
            self.data.button_style(),
            Rc::clone(&dialogs.metrics),
        ))
    }

    /// The load's note's input, all of it: once OK is activated, the note
    /// closes, keeping its sounds, and the flight built behind it shows.
    pub(super) fn load_note_input(&mut self, input: &Input) -> ScreenAction {
        let note = self.load_note.as_mut().expect("open");
        note.input(input);
        if note.closed() {
            if let Some(mut note) = self.load_note.take() {
                self.sounds.extend(note.take_sounds());
            }
            self.enter_built_flight();
        }
        ScreenAction::None
    }

    /// Closes the saved pilots' list, keeping its sounds.
    fn close_pilot_list(&mut self) {
        if let Some(mut list) = self.open_pilot.take() {
            self.sounds.extend(list.take_sounds());
        }
    }
}
