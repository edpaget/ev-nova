//! A text shown on a faded screen, as `_ShowTextDialog` (@0x5a752) shows
//! one with `_screenFaded` (*0x3b5198) set: under the jump's white, or
//! over the intro's black. [`MissionDialogs`](super::mission_dialogs::MissionDialogs)
//! keeps one [`FadedScreen`] and asks it what to do with the text in front.
//!
//! Whoever holds the dialogs says the screen is faded
//! ([`MissionDialogs::set_faded`](super::mission_dialogs::MissionDialogs::set_faded)),
//! and draws its own fade over them while it still covers the screen
//! ([`MissionDialogs::faded`](super::mission_dialogs::MissionDialogs::faded)).
//! The front text then shows in this order:
//!
//! 1. Its before-movie, begun on the faded screen
//!    ([`MovieStage::begin_faded`](crate::movie::MovieStage::begin_faded)).
//!    A plain one plays under the holder's fade, unseen. A cinematic one's
//!    first fade does nothing and its window fades in from black, so it
//!    takes the screen from the holder's fade there and then.
//! 2. If the screen is still faded, the dialog is built and drawn, and the
//!    screen fades in from black over [`TEXT_FADE_IN_FOR`] before its
//!    modal loop (`_FadeScreenIn` @0x5b038, after the repaint
//!    @0x5af96-0x5b033). `_FadeScreenIn` jumps the display to black first,
//!    so whatever the holder's fade was (the jump's white) is gone at once.
//!    An empty text shows no dialog and fades in all the same
//!    (@0x5a7bf-0x5a874), returning before its after-check. What is
//!    pressed during the fade-in, which is synchronous, stays queued for
//!    the dialog's loop: it reaches the dialog once the fade ends.
//! 3. The dialog, then its after-movie, on a screen faded in.
//!
//! Once the screen has faded in, by a text's fade-in or by a cinematic
//! movie, the holder is told once
//! ([`MissionDialogs::take_faded_in`](super::mission_dialogs::MissionDialogs::take_faded_in)),
//! and the texts after it show unfaded.
//!
//! `_ShowTextDialog`'s third faded branch, @0x5b454-0x5b4f6 after the
//! after-check, is dead in the original: the dialog's own `_FadeScreenIn`
//! @0x5b038 always clears the flag, a cinematic after-movie ends with
//! `_FadeScreenIn` @0x55edb, and nothing in the modal dialog fades the
//! screen out again. It is not built.
//!
//! The fade-in is drawn in the dialogs' own 1024 x 768 frame, as a
//! cinematic movie's window is: over a widened flight view its sides are
//! left uncovered, as the movie's are
//! (rdm:phase/movies/phase-8-cinematic-movie-widened).

use std::time::Duration;

use crate::draw::DrawList;
use crate::fade::{DEATH_FADE_FOR, ScreenFade};
use crate::input::Input;
use crate::system::camera::VIEW_SIZE;

/// How long a text on a faded screen takes to fade in from black: the
/// 1.0 s of `_FadeScreenIn` (@0x542d), synchronous.
pub const TEXT_FADE_IN_FOR: Duration = DEATH_FADE_FOR;

/// Whether the screen under the texts is faded, and the front text's
/// fade-in from black while it runs.
#[derive(Clone, Debug, Default)]
pub(crate) struct FadedScreen {
    /// Whether the holder's fade covers the screen (`_screenFaded`).
    faded: bool,
    /// The front text's fade-in from black, while it runs.
    fade_in: Option<ScreenFade>,
    /// What was pressed during the fade-in, for the dialog.
    held: Vec<Input>,
    /// Whether the screen has faded in since this was last taken.
    faded_in: bool,
}

impl FadedScreen {
    /// The holder's fade covers the screen.
    pub(crate) fn fade(&mut self) {
        self.faded = true;
    }

    /// Whether the holder's fade still covers the screen.
    pub(crate) fn faded(&self) -> bool {
        self.faded
    }

    /// The screen has faded in, by a cinematic movie or a text's fade-in:
    /// the holder's fade is gone, and the holder is told once.
    pub(crate) fn clear(&mut self) {
        if self.faded {
            self.faded = false;
            self.faded_in = true;
        }
    }

    /// The front text shows: on a faded screen, its fade-in from black
    /// begins. Whether it is fading in.
    pub(crate) fn show_text(&mut self) -> bool {
        if self.faded {
            self.clear();
            self.fade_in = Some(ScreenFade::from_black(TEXT_FADE_IN_FOR));
        }
        self.fading_in()
    }

    /// Whether the front text's fade-in runs.
    pub(crate) fn fading_in(&self) -> bool {
        self.fade_in.is_some()
    }

    /// Keeps `input`, pressed during the fade-in, for the dialog.
    pub(crate) fn hold(&mut self, input: &Input) {
        self.held.push(*input);
    }

    /// Moves the fade-in on by `dt`: once it has ended, what was pressed
    /// during it, for the dialog; `None` while it runs, or with none.
    pub(crate) fn tick(&mut self, dt: Duration) -> Option<Vec<Input>> {
        let fade = self.fade_in.as_mut()?;
        fade.advance(dt);
        if !fade.done() {
            return None;
        }
        self.fade_in = None;
        Some(std::mem::take(&mut self.held))
    }

    /// Whether the screen has faded in since this was last asked, once.
    pub(crate) fn take_faded_in(&mut self) -> bool {
        std::mem::take(&mut self.faded_in)
    }

    /// Draws the fade-in from black, while it runs, over the text.
    pub(crate) fn draw(&self, list: &mut DrawList) {
        if let Some(fade) = &self.fade_in {
            fade.draw(list, VIEW_SIZE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;

    #[test]
    fn it_fades_in_only_once_faded_and_tells_it_once() {
        let mut screen = FadedScreen::default();
        assert!(!screen.show_text(), "unfaded, nothing fades in");
        screen.clear();
        assert!(!screen.take_faded_in(), "nothing to clear");
        screen.fade();
        assert!(screen.faded());
        assert!(screen.show_text());
        assert!(!screen.faded());
        assert!(screen.take_faded_in());
        assert!(!screen.take_faded_in(), "once");
        assert!(screen.show_text(), "still running");
    }

    #[test]
    fn what_is_held_during_the_fade_in_comes_back_as_it_ends() {
        let mut screen = FadedScreen::default();
        assert_eq!(screen.tick(TEXT_FADE_IN_FOR), None, "no fade-in");
        screen.fade();
        screen.show_text();
        let enter = Input::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        };
        screen.hold(&enter);
        assert_eq!(screen.tick(TEXT_FADE_IN_FOR / 2), None);
        assert_eq!(screen.tick(TEXT_FADE_IN_FOR / 2), Some(vec![enter]));
        assert!(!screen.fading_in());
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert!(list.is_empty());
    }

    #[test]
    fn the_fade_in_takes_a_second() {
        assert_eq!(TEXT_FADE_IN_FOR, Duration::from_secs(1));
    }
}
