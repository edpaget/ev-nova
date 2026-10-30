//! The router's away state: the original's `_inBackground` (0x3b50e0),
//! set by `_SwitchedOut` (@0x4f388) and cleared by `_SwitchedIn`
//! (@0x513c4) as the app goes to the background and comes back
//! ([`Screen::focus_lost`](nova_view::Screen::focus_lost),
//! [`Screen::focus_gained`](nova_view::Screen::focus_gained)).
//!
//! While away, nothing plays on:
//!
//! - **Flight** doesn't tick, as `_PlayGame` skips its whole frame while
//!   the flag is set (@0x45b2d-0x45b3e): no `_DoPlayGameWork`, no draw,
//!   no sound.
//! - **A dialog** (an overlay of the router's) neither ticks nor takes
//!   input, as `_MyModalDialog` calls no filter in the background, null
//!   events included (@0x59fcc-0x59fee).
//! - **Input** is dropped, but the last pointer move or leave, which is
//!   kept and passed on as the app comes back: the first idle after a
//!   resume reads `GetMouse`, so the screen sees where the pointer is now.
//!   The rest isn't replayed, as the original's filter-less loop takes the
//!   events from the queue too. A release has nothing left to end: the
//!   app abandons the press and lets go of the keys before it says the
//!   focus went, as the original's `Button()` goes false wherever the
//!   pointer is. The window's close is untouched. What a hold kept (a
//!   keyed click's, which ends by its sound, not by a tick) passed this
//!   gate as it arrived, and is replayed past it.
//! - **The menu commands** aren't input, and pass. The original's event
//!   loops take Apple events whatever `_inBackground` says
//!   (`AEProcessAppleEvent` @0x1b242, @0x568e1, @0x599a9), and no handler
//!   reads it: not `_QuitAEVTHandler` (@0x5619e), `_PrefAEVTHandler`
//!   (@0x5617b, `_DoPrefsDialog`) nor `_AppEventHandler`'s `pref` and
//!   `abou` (@0xcccf1, @0xcccb0).
//!   - **Quit**, the menu's, the Dock's, a logout's or the updater's,
//!     quits at once. What holds it in the front (the appear, the entry,
//!     a keyed click, Quit's delay, the intro's text) is paused here or
//!     waits for a key, so waiting would keep it until the app is back,
//!     where the original's loops run to their end and take it.
//!   - **Preferences…** and **About** do what they do in the front, under
//!     the same gates and holds: what they open stands still until the
//!     app is back, as any dialog does here, and the app asks for one
//!     redraw to show it.
//! - **The main menu**, when it is the screen a tick would idle, idles in
//!   the background
//!   ([`MainMenu::idle_in_background`](nova_view::menu::MainMenu::idle_in_background)): its logo and
//!   slides wait, its rollover shows the none frame in full
//!   (`_MainScreenAnimIdle` @0x18845, @0x18891; `_MainScreenRolloverIdle`
//!   @0x16593-0x165bc).
//! - The appear, the entry, the fade back from black and Quit's delay
//!   wait too. Those are synchronous loops in the original, which run to
//!   their end before a switch-out takes hold; that is left to
//!   rdm:phase/play-feel-6-followups/phase-1-synchronous-loops-before-switch-out.
//!
//! Every tick while away counts its time. The first tick after the app
//! comes back is still background time (the app asks for no redraws
//! while away, so it carries the time since the last one), and reports
//! the whole of it ([`UiSound::SwitchedIn`]), for the music's switched-out
//! fade; the tick after is a normal one.

use std::time::Duration;

use nova_view::{Input, Screen, Sound, UiSound};

use super::{AppScreen, Side};

/// The app in the background, or back from it until its first tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Away {
    /// How long it has been away, over every tick since it went.
    time: Duration,
    /// Whether the app has come back to the front, its first tick to come.
    back: bool,
    /// The last pointer move or leave while away, to pass on as it comes
    /// back.
    pointer: Option<Input>,
}

impl AppScreen {
    /// The app went to the background: away from now, or away again,
    /// counting on, if it came back without a tick since.
    pub(super) fn go_away(&mut self) {
        let away = self.away.get_or_insert_default();
        away.back = false;
    }

    /// The app came back to the front: the pointer's last move or leave
    /// while away goes to the screen, and the next tick is the last away.
    /// Not away, nothing. A pointer move or leave never quits: the router
    /// quits only on a key, and no screen under it asks to, so its answer
    /// isn't read.
    pub(super) fn come_back(&mut self) {
        let Some(away) = &mut self.away else {
            return;
        };
        away.back = true;
        if let Some(pointer) = away.pointer.take() {
            self.input(&pointer);
        }
    }

    /// Whether the app is in the background now, not yet back.
    pub(super) fn is_away(&self) -> bool {
        self.away.is_some_and(|away| !away.back)
    }

    /// Whether `input` stops here, the app in the background: a pointer
    /// move or leave is kept, the last winning, and the rest dropped.
    pub(super) fn away_takes_input(&mut self, input: &Input) -> bool {
        let Some(away) = self.away.as_mut().filter(|away| !away.back) else {
            return false;
        };
        if matches!(input, Input::PointerMoved(_) | Input::PointerLeft) {
            away.pointer = Some(*input);
        }
        true
    }

    /// Whether `dt` passed in the background: it is counted, and the main
    /// menu idles there when a tick would idle it. Back, it also reports
    /// the time away and ends it.
    pub(super) fn tick_away(&mut self, dt: Duration) -> bool {
        let idles_menu = self.side == Side::MainMenu
            && !self.has_overlay()
            && !self.appear_holds_menu()
            && !self.entry_holds();
        let Some(away) = &mut self.away else {
            return false;
        };
        away.time = away.time.saturating_add(dt);
        let (time, back) = (away.time, away.back);
        if idles_menu && let Some(menu) = &mut self.menu {
            menu.screen.idle_in_background(dt);
        }
        if back {
            self.away = None;
            self.sounds
                .push(Sound::Ui(UiSound::SwitchedIn { away: time }));
        }
        true
    }

    /// Whether any overlay is open over the side shown.
    fn has_overlay(&mut self) -> bool {
        self.overlay_mut().is_some()
    }
}
