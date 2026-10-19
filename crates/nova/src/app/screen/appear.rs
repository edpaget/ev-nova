//! [`AppScreen`]'s main screen appearing, at start-up and on each return
//! from play: the glue around [`Appear`], which makes the decisions
//! (`nova_view::menu::appear` says what the original does).
//!
//! While an appear runs, the router gives it every input and the window's
//! close first, keeping the pointer, the button and the keys it notes as
//! it always does. What the appear holds is replayed through
//! [`Screen::input`] once it ends, in order, as the original's queue
//! reaches the main screen after its synchronous fade; a replayed key
//! that holds the menu queues the rest, as at a keyed click. The keys'
//! letting go when the window loses focus waits among them, after the
//! presses it lets go of. The pointer's cancel needs no such place: the
//! appear holds no pointer input for it to cancel. While the
//! appear waits unseen, the main menu doesn't tick, but its logo's and
//! slides' time counts ([`MainMenu::wait`](nova_view::menu::MainMenu::wait)).
//!
//! The menu's first tick comes on the tick after the appear ends, under a
//! frame later than the original's first idle after its fade; no frame
//! the player sees tells them apart, as the first frame after the fade
//! shows the screen as `_MainScreenAppearStart` draws it.

use nova_sim::{RuleKey, RuleSource};
use nova_view::menu::appear::{Appear, Deferred, Verdict};
use nova_view::{Input, Screen, ScreenAction};

use super::{AppScreen, MenuKeys, Side};

impl AppScreen {
    /// Puts play away and shows the main screen as a return from play
    /// brings it back, by [`RuleKey::MainScreenReturnFade`]: the menu
    /// appears (`_LoadMainScreenAnim` before the `StillDown` spin,
    /// @0x1a593), and the appear waits for the primary button if it is
    /// down. The death's return and Escape's put-away both come here.
    /// Play entered from the menu leaves its pilot held there
    /// ([`AppScreen::leave_play`]); the developer's flight, entered from
    /// a side, is dropped ([`AppScreen::end_flight`]).
    pub(super) fn return_to_main_screen(&mut self) {
        if self.return_to == Side::MainMenu {
            self.leave_play();
        } else {
            self.end_flight(Side::MainMenu);
        }
        if let Some(menu) = &mut self.menu {
            menu.screen.appear();
        }
        let menu_live = self.rules.source_for(RuleKey::MainScreenReturnFade) == RuleSource::Bible;
        self.appear = Some(Appear::returning(menu_live, self.button_down));
    }

    /// `input` while the main screen appears: what it did, once the
    /// appear has dropped or held it, or `None` once it passes it on (or
    /// with no appear). The pointer and the keys are noted either way.
    pub(super) fn appear_input(&mut self, input: &Input) -> Option<ScreenAction> {
        self.appear.as_ref()?;
        self.note_pointer(input);
        self.note_keys(input);
        let command_held = self.command_held;
        match self.appear.as_mut()?.input(input, command_held) {
            Verdict::Pass => None,
            Verdict::Drop | Verdict::Held => Some(ScreenAction::None),
        }
    }

    /// The window's close while the main screen appears: `Some(false)`,
    /// staying open, once the appear has held or dropped it; `None` once
    /// it passes it on (or with no appear).
    pub(super) fn appear_close(&mut self) -> Option<bool> {
        match self.appear.as_mut()?.close() {
            Verdict::Pass => None,
            Verdict::Drop | Verdict::Held => Some(false),
        }
    }

    /// The primary button was let go of outside the game: a return's wait
    /// for it ends. Whether the appear takes the cancel, the menu holding
    /// nothing to cancel under it.
    pub(super) fn appear_cancel_pointer(&mut self) -> bool {
        self.button_down = false;
        let Some(appear) = &mut self.appear else {
            return false;
        };
        appear.released();
        !appear.menu_live()
    }

    /// The keys were let go of outside the game (the window losing
    /// focus): while the main screen waits, the router's notes of them
    /// are cleared now, for the appear's own verdicts, and the letting go
    /// waits in its place among what the appear holds, so a press held
    /// before it doesn't leave its key down once replayed. Whether the
    /// appear takes it.
    pub(super) fn appear_release_keys(&mut self) -> bool {
        let Some(appear) = &mut self.appear else {
            return false;
        };
        if appear.release_keys() != Verdict::Held {
            return false;
        }
        self.command_held = false;
        self.keys_held = MenuKeys::default();
        true
    }

    /// Whether an appear keeps the main menu waiting unseen, unticked.
    pub(super) fn appear_holds_menu(&self) -> bool {
        self.appear
            .as_ref()
            .is_some_and(|appear| !appear.menu_live())
    }

    /// Moves the appear on by `dt`, the menu waiting under it unless it is
    /// live; once it ends, drops it and replays what it held, a quit a
    /// replayed input asks kept for [`Screen::take_quit`].
    pub(super) fn advance_appear(&mut self, dt: std::time::Duration) {
        let holds_menu = self.appear_holds_menu();
        let Some(appear) = &mut self.appear else {
            return;
        };
        let ended = appear.tick(dt);
        if holds_menu && let Some(menu) = &mut self.menu {
            menu.screen.wait(dt);
        }
        if !ended {
            return;
        }
        let held = self.appear.take().map(|mut appear| appear.take_held());
        for deferred in held.into_iter().flatten() {
            match deferred {
                Deferred::Input(input) => {
                    if self.input(&input) == ScreenAction::Quit {
                        self.quit_due = true;
                    }
                }
                Deferred::Close => self.replay_close(),
                Deferred::ReleaseKeys => self.release_keys(),
            }
        }
    }
}
