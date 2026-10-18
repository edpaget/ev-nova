//! [`AppScreen`]'s answer to the window's close, and the quit question it
//! asks on the main menu.
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
//! While the main menu waits on a keyed click, or Quit's delay runs, the
//! close quits at once too: the click's action has not run, and Quit is
//! already under way.

use std::rc::Rc;

use nova_view::ui::yes_no::{QUIT_QUESTION, YES_NO_DIALOG};
use nova_view::ui::{DescriptionSource, YesNoDialog};
use nova_view::{Input, Screen, ScreenAction, Showing};

use super::AppScreen;

impl AppScreen {
    /// Whether the app quits now as the window's close is asked
    /// ([`Screen::close_requested`]): on the main menu alone, with
    /// nothing over it, it opens the quit question instead, cancelling the
    /// menu's pointer and letting go of its keys, and stays.
    pub(super) fn close_asked(&mut self) -> bool {
        if self.showing() != Showing::MainMenu || self.menu_hold.is_some() || self.quit_in.is_some()
        {
            return true;
        }
        let Some(question) = self.quit_question_dialog() else {
            return true;
        };
        let menu = self.shown_mut();
        menu.cancel_pointer();
        menu.release_keys();
        self.question = Some(question);
        false
    }

    /// `STR#` 140 #10 in the Yes/No dialog (`DLOG` 3002), if the router
    /// has dialogs and the dialog can be built.
    fn quit_question_dialog(&self) -> Option<YesNoDialog> {
        let dialogs = self.dialogs.as_ref()?;
        let template = dialogs.resources.dialog_template(YES_NO_DIALOG).ok()?;
        Some(YesNoDialog::new(
            &template,
            QUIT_QUESTION,
            self.data.button_style(),
            Rc::clone(&dialogs.metrics),
        ))
    }

    /// The quit question's input, all of it: Yes quits at once
    /// ([`Screen::take_quit`]), No closes it, back to the main menu.
    pub(super) fn question_input(&mut self, input: &Input) -> ScreenAction {
        let question = self.question.as_mut().expect("asked");
        question.input(input);
        match question.take_answer() {
            Some(true) => {
                self.question = None;
                self.quit_due = true;
            }
            Some(false) => self.question = None,
            None => {}
        }
        ScreenAction::None
    }
}
