//! [`AppScreen`]'s answer to the window's close, and the questions it
//! asks on the main menu in the Yes/No dialog: the quit question, and New
//! Pilot's over a game in progress ([`Question`]).
//!
//! On the main screen, the original's close box (`_MainScreenMouse` part
//! 6, @0x1aeb4-0x1aef7) asks `STR#` 140 #10 in the Yes/No dialog
//! ([`YesNoDialog`]), and its Yes sets `_gQuit` at once, with no delay
//! and no sound. Anywhere else the window's close quits at once:
//!
//! - In play the original asks too (the close box @0x56a40, the quit
//!   event @0x56215, @0xcc9f5); the engine doesn't yet
//!   (rdm:phase/main-screen-fidelity/phase-1-ask-before-quitting-play).
//! - With a dialog open over the main screen, the original's modal loops
//!   never route a click to the window's close box (`_MyModalDialog` has
//!   no `FindWindow`), so it does nothing there. The engine's platform
//!   close can't be ignored for good, so it quits (filed with the
//!   above).
//! - Without dialogs, or without the Yes/No dialog, there is nothing to
//!   ask in.
//!
//! While Quit's delay runs, the close quits at once too: Quit is already
//! under way.
//!
//! While the main menu appears, the appear answers first (see `appear`).
//! At start-up the close waits for the fade's end, as the drain after
//! `_DoMainScreen`'s fade reads the close-box click and asks, `_gameLoaded`
//! being set (@0x569ef-0x56a55, @0x47411), and then it is replayed as a
//! held close is ([`AppScreen::replay_close`]). On a return from play, by
//! the engine's reading, it is dropped and the window stays open, as
//! `_EnterGameFromMainScreen` flushes the mouseDowns (@0x1a5a8,
//! @0x1a6af); by the other reading it is asked as on the menu.
//!
//! While the main menu waits on a keyed click (or on E's release), the
//! close waits with the other inputs ([`Held::Close`]). The original's
//! click spins on `_ST_IsSoundPlaying` with only `_BusyIdle`, which is
//! empty (`_PlayMainScreenClickSound` @0x177e0-0x177f2), and E's wait
//! reads only the keys (@0x1ac32-0x1ac57), so the close-box click stays
//! in the queue until the choice has run. Then, in its place among the
//! held inputs:
//!
//! - Back in `_DoMainScreen`'s loop (`WaitNextEvent` @0x1b20d,
//!   `_MainScreenMouse` @0x1b24f), as after E's refusal, the click
//!   reaches the close box and asks the question, as a close now would.
//! - A dialog the choice opened takes the click instead, and drops it:
//!   New Pilot's name dialog (`FlushEvents(0xff7f)` @0x16816), the
//!   About and X texts (`_ShowTextDialog` @0x5ae67) and the Yes/No dialog
//!   itself (@0x55689) flush every event but disk events before their
//!   loops; Open Pilot's Navigation dialog (`NavDialogRun` @0x732c2) and
//!   the Cocoa Preferences window (`_RunPrefsDialog` @0xcd409) run the
//!   system's modal loops, which never route it to `_MainScreenMouse`.
//!   So it asks nothing and quits nothing, and a second close held behind
//!   the first is dropped by the question's flush.
//! - Quit's choice is inside its `Delay` (@0x1a9fb) and then exits, never
//!   reading it: the close is dropped and Quit quits after its delay.

use std::rc::Rc;

use nova_view::ui::yes_no::{NEW_PILOT_QUESTION, QUIT_QUESTION, YES_NO_DIALOG};
use nova_view::ui::{DescriptionSource, YesNoDialog};
use nova_view::{Input, Screen, ScreenAction, Showing};

use super::{AppScreen, Held};

/// What a question asked over the main menu is for, which decides what
/// its Yes does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Question {
    /// `STR#` 140 #10, the window's close: Yes quits at once.
    Quit,
    /// `STR#` 140 #8, New Pilot over a game in progress: Yes opens the
    /// New Pilot dialog (`_MainScreenChoice` @0x1a74a-0x1a78f).
    NewPilot,
}

impl Question {
    /// The question's text.
    fn text(self) -> &'static str {
        match self {
            Self::Quit => QUIT_QUESTION,
            Self::NewPilot => NEW_PILOT_QUESTION,
        }
    }
}

impl AppScreen {
    /// Whether the app quits now as the window's close is asked
    /// ([`Screen::close_requested`]): on the main menu alone, with
    /// nothing over it, it opens the quit question instead, cancelling the
    /// menu's pointer and letting go of its keys, and stays.
    ///
    /// While the main menu appears, the appear answers first, holding or
    /// dropping the close with the window kept open; while the main menu
    /// waits on a keyed click, the close waits too and the window stays
    /// open ([`AppScreen::replay_close`]).
    pub(super) fn close_asked(&mut self) -> bool {
        if let Some(quits) = self.entry_close() {
            return quits;
        }
        if let Some(quits) = self.appear_close() {
            return quits;
        }
        if self.menu_hold.is_some() {
            self.held.push(Held::Close);
            return false;
        }
        if self.showing() != Showing::MainMenu || self.quit_in.is_some() {
            return true;
        }
        self.ask_to_quit()
    }

    /// A close held while the main menu waited, replayed once its choice
    /// has run: held again if the menu waits again; on the main menu
    /// alone, asked as a close now is, a quit at once kept for
    /// [`Screen::take_quit`]; dropped under a dialog the choice opened, or
    /// once Quit's delay runs (see the module docs).
    pub(super) fn replay_close(&mut self) {
        if self.menu_hold.is_some() {
            self.held.push(Held::Close);
            return;
        }
        if self.showing() == Showing::MainMenu && self.quit_in.is_none() && self.ask_to_quit() {
            self.quit_due = true;
        }
    }

    /// On the main menu alone: opens the quit question and stays
    /// (`false`), or, with no question to ask in, quits at once (`true`).
    fn ask_to_quit(&mut self) -> bool {
        !self.ask(Question::Quit)
    }

    /// Opens `question` in the Yes/No dialog (`DLOG` 3002) over the main
    /// menu, cancelling the menu's pointer and letting go of its keys,
    /// and whether it could: the router needs dialogs, and the dialog has
    /// to build.
    pub(super) fn ask(&mut self, question: Question) -> bool {
        let Some(dialog) = self.question_dialog(question) else {
            return false;
        };
        let menu = self.shown_mut();
        menu.cancel_pointer();
        menu.release_keys();
        self.question = Some((question, dialog));
        true
    }

    /// `question` in the Yes/No dialog, if the router has dialogs and the
    /// dialog can be built.
    fn question_dialog(&self, question: Question) -> Option<YesNoDialog> {
        let dialogs = self.dialogs.as_ref()?;
        let template = dialogs.resources.dialog_template(YES_NO_DIALOG).ok()?;
        Some(YesNoDialog::new(
            &template,
            question.text(),
            self.data.button_style(),
            Rc::clone(&dialogs.metrics),
        ))
    }

    /// The question's input, all of it: No closes it, back to the main
    /// menu, leaving everything as it was; Yes closes it and does what
    /// the question is for: the quit question quits at once
    /// ([`Screen::take_quit`]), and New Pilot's opens the New Pilot
    /// dialog.
    pub(super) fn question_input(&mut self, input: &Input) -> ScreenAction {
        let (asked, dialog) = self.question.as_mut().expect("asked");
        let asked = *asked;
        dialog.input(input);
        match dialog.take_answer() {
            Some(true) => {
                self.question = None;
                match asked {
                    Question::Quit => self.quit_due = true,
                    Question::NewPilot => self.open_new_pilot(),
                }
            }
            Some(false) => self.question = None,
            None => {}
        }
        ScreenAction::None
    }
}
