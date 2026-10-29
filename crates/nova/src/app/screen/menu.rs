//! [`AppScreen`]'s main menu: its input and actions, the New Pilot
//! dialog, the pilot list and the load note.

use std::rc::Rc;

use nova_sim::new_pilot::{kept_name, name_defaults};
use nova_sim::{OpenError, Pilot, RuleKey, RuleSource, SavedOpening, pilot_key};
#[cfg(doc)]
use nova_view::flight::FlightView;
use nova_view::menu::{Chosen, ChosenBy, MenuChoice, PilotList, PilotListOutcome};
use nova_view::ui::new_pilot::{NAME_TAKEN, NEW_PILOT_DIALOG, NewPilotDialog, NewPilotOutcome};
use nova_view::ui::note::{LOAD_FALLBACK_NOTE, NOTE_DIALOG, NoteDialog, VERSION_NOTE};
use nova_view::ui::{DescriptionSource, DialogResources};
use nova_view::{Input, Key, Screen, ScreenAction, Sound, UiSound};

use super::quit_question::Question;
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
    /// `_MainScreenChoice(2)` as the key. A click on Enter Ship,
    /// Preferences or About runs at once the choice E, P or A runs once
    /// its click has played: the same arm of `_MainScreenChoice`. Escape
    /// does nothing, as `_MainScreenKey` has no branch for it. Tab goes
    /// to the ship browser.
    ///
    /// Each key clicks ([`UiSound::ButtonKeyed`]) and its choice waits
    /// for the click to end ([`MenuHold`]): typed N, O and Q (the menu's
    /// own), and, with dialogs, A (the About text, @0x1ac70) and X
    /// (`dësc` 32766, @0x1ac7e, munged for the pilot held, or as before
    /// any pilot). A's and X's repeats act as presses (see below). P is
    /// the router's ([`preferences_key`](Self::preferences_key)).
    ///
    /// - O does nothing while the physical S or L key is held
    ///   (@0x1abd3-0x1abf5). No other code reads those two keys, so the
    ///   guard's purpose is unknown; it is built as read.
    /// - Typed E with the physical E key held waits for its release
    ///   (@0x1ac32-0x1ac57) before it clicks; typed with the E key up
    ///   (another layout) it clicks at once. Then it is Enter Ship
    ///   (`_MainScreenChoice(3)`, [`AppScreen::enter_ship`]): it enters
    ///   play with the game held, through the entry, or, with none or a
    ///   dying pilot, refuses with [`UiSound::EnterRefused`].
    ///
    /// With Command held, the typed keys and A and X go to the
    /// original's menus and do nothing here.
    pub(super) fn menu_input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: key @ Key::Char('a' | 'x'),
            pressed,
            ..
        } = *input
            && self.dialogs.is_some()
            && !self.command_held
        {
            // A held key's repeats act as presses: `_MainScreenKey`
            // reads the character alone, never the event's kind (@0x1abaf),
            // so an autoKey that reaches it once the text's dialog has
            // closed opens it again; the dialog itself drops those that
            // reach it (`_TextDialogFilter` @0x51f86-0x51fe5).
            if pressed {
                self.click_then(if key == Key::Char('a') {
                    MenuAction::Choice(MenuChoice::About)
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
                    self.click_then(MenuAction::Choice(MenuChoice::EnterShip));
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
        // The click's sounds come before what its choice plays: 601,
        // then Enter Ship's refusal (@0x19c38, then @0x1aaac).
        self.sounds.extend(menu.screen.take_sounds());
        let menu = self.menu.as_mut().expect(MENU);
        match menu.screen.take_choice() {
            Some(Chosen {
                choice,
                by: ChosenBy::Shortcut,
            }) => self.menu_hold = Some(MenuHold::Click(MenuAction::Choice(choice))),
            Some(Chosen { choice, .. }) => self.run_menu_action(MenuAction::Choice(choice)),
            None => {}
        }
        ScreenAction::None
    }

    /// Tells the main menu, if any, where the pointer is now
    /// ([`MainMenu::set_pointer`](nova_view::menu::MainMenu::set_pointer)),
    /// as `_MainScreenRolloverIdle` reads
    /// `GetMouse` each idle (@0x16500-0x16528). Pointer moves go only to
    /// the overlay or side shown, so without this the rollover would aim
    /// where the pointer last was over the menu once a dialog over it
    /// closes, the menu is switched back to, or flight returns to it.
    /// Only its ticks read it, so telling it while it is hidden changes
    /// nothing.
    pub(super) fn tell_menu_pointer(&mut self) {
        let pointer = self.pointer;
        if let Some(menu) = &mut self.menu {
            menu.screen.set_pointer(pointer);
        }
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
            MenuAction::Choice(MenuChoice::NewPilot) => self.new_pilot_chosen(),
            MenuAction::Choice(MenuChoice::OpenPilot) => self.open_pilot_list(),
            MenuAction::Choice(MenuChoice::Quit) => self.quit_in = Some(QUIT_DELAY),
            MenuAction::Choice(MenuChoice::EnterShip) => self.enter_ship(),
            MenuAction::Choice(MenuChoice::Preferences) => self.open_preferences(),
            MenuAction::Choice(MenuChoice::About) => self.open_text(ABOUT_TEXT),
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
        self.click_then(MenuAction::Choice(MenuChoice::EnterShip));
    }

    /// New Pilot, as `_MainScreenChoice(0)` runs it: with a game in
    /// progress it asks `STR#` 140 #8 in the Yes/No dialog first, and only
    /// Yes goes on (@0x1a74a-0x1a78f); without one, or with no Yes/No
    /// dialog to ask in, the New Pilot dialog opens at once.
    fn new_pilot_chosen(&mut self) {
        if self.game.is_some() && self.ask(Question::NewPilot) {
            return;
        }
        self.open_new_pilot();
    }

    /// Opens the New Pilot dialog: the interface file's, or the built-in
    /// one without it (with a warning when the interface file has none),
    /// its fields holding the defaults drawn on the router's chance each
    /// time it opens, as `_DoNewPilot` draws them before the dialog
    /// (@0x18b44-0x18ba7; `nova_sim::new_pilot`).
    pub(super) fn open_new_pilot(&mut self) {
        let defaults = name_defaults(self.data.as_ref(), &mut self.chance);
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
        self.new_pilot = Some(dialog.with_defaults(&defaults.name, &defaults.nickname));
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

    /// The New Pilot dialog's input. The name and nickname confirmed are
    /// kept culled, then trimmed (`kept_name`, @0x18bd6-0x18c03), and the
    /// pilot is built with them, the gender and Strict Play chosen. A name
    /// no file can be saved under (one culled or trimmed to nothing among
    /// them) is refused.
    ///
    /// A name already saved, ignoring case, is checked next, as
    /// `_DoNewPilot` checks it (`_DoesPilotExist` @0x18c0b) before it
    /// christens the ship. By the engine's reading of
    /// [`RuleKey::NewPilotNameTaken`], the New Pilot dialog closes and
    /// `STR#` 140 #9 is asked in the Yes/No dialog (@0x18c08-0x18c63; see
    /// `quit_question`): Yes goes on to the christening and the new pilot
    /// replaces the saved one, No returns to the main menu. By the other
    /// reading, or with no Yes/No dialog to ask in, the name is refused
    /// with [`NAME_TAKEN`] and the dialog stays open.
    ///
    /// Any other name asks to christen the ship
    /// ([`AppScreen::open_christening`]). Once christened, its game begins
    /// (the `chär`'s `OnStart`, [`FlightView::begin`]), it is saved at
    /// once, with what that did (so Open Pilot lists it), as `_DoNewPilot`
    /// runs `OnStart` before the first save, and it is held on the main
    /// menu, not flying, in place of any game held
    /// ([`AppScreen::hold_new_pilot`]).
    pub(super) fn new_pilot_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.new_pilot.as_mut().expect("open");
        dialog.input(input);
        let outcome = dialog.take_outcome();
        match outcome {
            Some(NewPilotOutcome::Cancel) => self.close_new_pilot(),
            Some(NewPilotOutcome::Create {
                name,
                nickname,
                gender,
                strict_play,
            }) => {
                let name = kept_name(&name);
                if pilot_key(&name).is_none() {
                    self.refuse_new_pilot(UNUSABLE_NAME);
                    return ScreenAction::None;
                }
                let pilot = match Pilot::new(self.data.as_ref(), &name) {
                    Ok(pilot) => pilot
                        .with_nickname(kept_name(&nickname))
                        .with_gender(gender)
                        .with_strict_play(strict_play),
                    Err(error) => {
                        let why = error.to_string();
                        self.refuse_new_pilot(&why);
                        self.warnings
                            .push(format!("nova: cannot create a pilot: {why}"));
                        return ScreenAction::None;
                    }
                };
                let by_key = matches!(input, Input::Key { .. });
                if let Some(saved) = self.keeper().and_then(|keeper| keeper.saved_key(&name)) {
                    self.ask_to_replace(pilot, saved);
                } else {
                    self.close_new_pilot();
                    self.open_christening(pilot, by_key, None);
                }
            }
            None => {}
        }
        ScreenAction::None
    }

    /// Refuses the New Pilot dialog's name, saying `why` under it; the
    /// dialog stays open.
    fn refuse_new_pilot(&mut self, why: &str) {
        self.new_pilot.as_mut().expect("open").refuse(why);
    }

    /// A new `pilot` named as the pilot saved under `saved`: by the
    /// engine's reading of [`RuleKey::NewPilotNameTaken`], the New Pilot
    /// dialog closes, keeping its sounds, and `STR#` 140 #9 is asked
    /// ([`Question::ReplacePilot`]); by the other reading, or with no
    /// Yes/No dialog to ask in, the name is refused and the dialog stays
    /// open.
    fn ask_to_replace(&mut self, pilot: Pilot, saved: String) {
        // Out of the way while the question opens, so the menu under it is
        // the screen whose pointer and keys are let go of.
        let mut dialog = self.new_pilot.take().expect("open");
        let asks = self.rules.source_for(RuleKey::NewPilotNameTaken) == RuleSource::Engine;
        if asks
            && self.ask(Question::ReplacePilot {
                pilot: Box::new(pilot),
                saved,
            })
        {
            self.sounds.extend(dialog.take_sounds());
            return;
        }
        dialog.refuse(NAME_TAKEN);
        self.new_pilot = Some(dialog);
    }

    /// Closes the New Pilot dialog, keeping its sounds.
    fn close_new_pilot(&mut self) {
        if let Some(mut dialog) = self.new_pilot.take() {
            self.sounds.extend(dialog.take_sounds());
        }
    }

    /// The saved pilots' list's input. The pilot chosen is opened as
    /// `_MainScreenChoice(1)` opens it and held on the main menu, not
    /// flying, in place of any game held: its explored systems' nebulae are
    /// swept as its session is built ([`nova_sim::Session::load`]), then its
    /// ship faces a drawn heading and its active missions' auxiliary ships
    /// restart (@0x1a918-0x1a982), in one call
    /// ([`SessionFlight::open_saved`](nova_view::flight::SessionFlight::open_saved)
    /// with [`SavedOpening::OpenPilot`]). The engine
    /// sweeps as it builds the session, before the note below, where the
    /// original sweeps after it (@0x1a8d4, after @0x1a85b); nothing the
    /// sweep changes shows on the main screen.
    ///
    /// One whose load fell back ([`SessionFlight::load_repaired`](nova_view::flight::SessionFlight::load_repaired)) shows the
    /// note `STR#` 140 #52 over the main menu, as `_MainScreenChoice`
    /// shows it through `_EVNoteAlert` (@0x1a817-0x1a85b); without
    /// dialogs, or with no note to build, the note is a warning. Under
    /// the note the pilot panel shows the opened pilot when a game was held
    /// before the Open, and no pilot when none was, as the original draws
    /// the screen with `_gameInProgress` as it was (see `pilot_panel`).
    ///
    /// One that can't be opened closes the list, as the original's
    /// Navigation panel is gone by then, and leaves the game held as it
    /// was (`_MainScreenChoice` does nothing more on false,
    /// @0x1a80f-0x1a811): a save of another version shows `STR#` 140 #34
    /// in the note alert ([`VERSION_NOTE`], `_OpenPilotFile`'s -42,
    /// @0x766c8-0x766ef), and any other failure shows nothing
    /// (@0x766fb-0x76701); each is a warning too. The original resets the
    /// held player before its load fails, which the engine's whole-file
    /// load leaves out
    /// (rdm:phase/main-screen-fidelity/phase-3-main-screen-left-outs).
    pub(super) fn open_pilot_input(&mut self, input: &Input) -> ScreenAction {
        let list = self.open_pilot.as_mut().expect("open");
        list.input(input);
        match list.take_outcome() {
            Some(PilotListOutcome::Cancel) => self.close_pilot_list(),
            Some(PilotListOutcome::Open(key)) => {
                let opened = match self.keeper() {
                    Some(keeper) => keeper.open(&key),
                    None => Err(OpenError::Unreadable(format!(
                        "nova: there is nowhere to open the pilot {key} from"
                    ))),
                };
                self.close_pilot_list();
                match opened {
                    Ok(pilot) => {
                        let mut flight = self.pilot_flight(pilot, true, None);
                        // The load's heading, before any frame is drawn.
                        flight.change_sim(|sim| sim.open_saved(SavedOpening::OpenPilot));
                        let repaired = flight.sim().load_repaired();
                        let held_before = self.game.is_some();
                        self.hold_game(flight);
                        // The original saves only on creation, landing
                        // and rescue (`_SavePilotFile` @0x76876), never on
                        // an open: a save its hooks make due is dropped.
                        let _ = self.settle_game();
                        if repaired {
                            self.show_note(LOAD_FALLBACK_NOTE);
                            // The original draws the screen under the
                            // note with `_gameInProgress` as it was (see
                            // `pilot_panel`).
                            self.note_hides_game = !held_before && self.load_note.is_some();
                        }
                    }
                    Err(OpenError::WrongVersion(warning)) => {
                        self.warnings.push(warning);
                        self.show_note(VERSION_NOTE);
                    }
                    Err(OpenError::Unreadable(warning)) => self.warnings.push(warning),
                }
            }
            None => {}
        }
        ScreenAction::None
    }

    /// Shows `text` in the note alert over the main menu, cancelling the
    /// menu's pointer and letting go of its keys; without dialogs, or with
    /// no note to build, `text` is a warning.
    fn show_note(&mut self, text: &str) {
        match self.note_dialog(text) {
            Some(note) => {
                let menu = self.shown_mut();
                menu.cancel_pointer();
                menu.release_keys();
                self.load_note = Some(note);
            }
            None => self.warnings.push(text.to_owned()),
        }
    }

    /// `text` in the note alert (`DLOG` 3000), if the router has dialogs
    /// and the alert can be built.
    fn note_dialog(&self, text: &str) -> Option<NoteDialog> {
        let dialogs = self.dialogs.as_ref()?;
        let template = dialogs.resources.dialog_template(NOTE_DIALOG).ok()?;
        Some(
            NoteDialog::new(
                &template,
                text,
                self.data.button_style(),
                Rc::clone(&dialogs.metrics),
            )
            .with_focus_ring(self.dialog_focus_ring()),
        )
    }

    /// The note's input, all of it: once OK is activated, the note
    /// closes, keeping its sounds, back to the main menu.
    pub(super) fn load_note_input(&mut self, input: &Input) -> ScreenAction {
        let note = self.load_note.as_mut().expect("open");
        note.input(input);
        if note.closed()
            && let Some(mut note) = self.load_note.take()
        {
            self.sounds.extend(note.take_sounds());
            self.note_hides_game = false;
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
