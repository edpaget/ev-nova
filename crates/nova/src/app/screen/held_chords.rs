//! The menu chords typed while a dialog holds back its keys for a held
//! press, or while a map waits for M's release, held by the router until
//! the dialog's or the map's replay has run. All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! **The original.** A text dialog's and the offer's held loops and
//! trackers read no events (`_TextDialogFilter` @0x52172-0x5237f,
//! `_SingleMissionFilter` @0x99060-0x991e6, `_TrackTextButtons` @0x26bb0,
//! `_TrackSingleMissionButtons` @0x28188), so every key typed meanwhile
//! waits in the queue, the Command chords too. Once the press is over,
//! `_MyModalDialog`'s `WaitNextEvent` (@0x5999a) reads them in order. The
//! standard handler `RunApplicationEventLoop` installs (`_main` @0xccb46)
//! matches a Command chord to its menu item there: Command-comma becomes
//! the `pref` command, which `_AppEventHandler` runs (@0xccc49,
//! `_DoPrefsDialog` @0xcccf1), and Command-Q the quit Apple event, which
//! the next `WaitNextEvent` hands to `AEProcessAppleEvent` (@0x599a9) and
//! `_HandleQuitEvent` (@0xcc958). Quitting never asks here, by the user's
//! decision, so the quit happens once the chord is read, with no sound.
//!
//! `_DoSystemMap`'s wait for M's release (@0x13a2e-0x13a46) reads no
//! events either: it calls only `_BusyIdle` (@0x5981, empty) and
//! `_KeyCheck` (@0xa19b-0xa1dc, a `GetKeys` snapshot), so neither
//! `_AppEventHandler` nor `_PrefAEVTHandler` (@0x5617b) runs in it, and
//! its closing `FlushEvents(7, 0)` (@0x13c44-0x13c53) keeps key-downs.
//! Every map shares the loop: the spaceport's, a mission dialog's over
//! flight and flight's own. The chord is read after the close by the
//! next loop, the screen's `_MyModalDialog` or flight's
//! `_IterateInGameEventLoop` (@0x568b4), after the keys typed before it.
//!
//! **The hold.** The router takes the chords before the screen in front
//! sees them ([`AppScreen::route`]), so it holds them itself while
//! [`AppScreen::holds_back_keys`] reads true:
//!
//! - A chord press (Command-Q or Command-comma, with Command held) starts
//!   the queue, resolved as it comes to its [`MenuCommand`], so Command
//!   let go of before the release doesn't unmake it.
//! - While the queue isn't empty, every key and typed character joins it,
//!   in order. Pointer input still goes to the front, so the release that
//!   ends the press reaches the dialog. The keys before the chord are the
//!   dialog's own, held back by it, so its replay runs first.
//! - While a map waits, the map key (M by default, as flight reads it,
//!   [`FlightView::is_map_key`]) passes the queue too, so its release reaches
//!   the map and ends the wait. The keys the wait took before the chord
//!   replay to the screen under it at the close with the queue set aside
//!   ([`AppScreen::ahead_of_held_chords`]), so they go first.
//! - [`Screen::release_keys`] while the queue isn't empty adds a let-go,
//!   in its place, so a key queued after the chord and let go of while
//!   the window had lost its keys isn't left down once it replays.
//!
//! **The replay.** After each outermost call that can end the hold (an
//! input, a menu command, a tick, a pointer cancel and a keys' release),
//! once [`AppScreen::holds_back_keys`] reads false, the queue is taken
//! whole and run in order: a command through
//! [`AppScreen::run_menu_command`], an input through
//! [`AppScreen::rerouted_input`] (Caps Lock's double time ran on it live),
//! a let-go through [`AppScreen::let_go_of_keys`]. The first Quit ends it:
//! after an input it is that input's [`ScreenAction::Quit`], and after a
//! tick, a cancel or a release it is due for [`Screen::take_quit`]. Nested
//! routes see the queue empty, so a replay never holds again.
//!
//! **The flush.** A screen that opens with `FlushEvents(0xff7f, 0)` drops
//! a queued chord's key-down with the rest: a text dialog
//! (`_ShowTextDialog` @0x5ae67), the map (`_DoSystemMap` @0x1257b), player
//! info (`_DoPlayerInfoDialog` @0x5a544) and mission info
//! (`_DoMissionInfoDialog` @0x9e287). So if one opened since the chord was
//! held (the `map_wait_keys` module's flush mark, and any map: nothing
//! opens while the press holds, so it opened in the call that ended it),
//! or opens as the replay runs, what is left is dropped, but for the
//! releases of the presses already replayed, as the map's wait keeps
//! them. A chord dropped so quits nothing. The Preferences are the
//! exception: `_RunPrefsDialog` (@0xcd401) runs a Cocoa modal session
//! (`runModalForWindow:` @0xcd50e) and flushes nothing, so the keys queued
//! behind Command-comma reach the Preferences.
//!
//! The replay settles within the call that ends the hold, before the frame
//! is drawn, so no boundary the player sees is crossed.

use std::mem;

use nova_view::flight::FlightView;
use nova_view::keymap::KeysDown;
use nova_view::{Input, Key, MenuCommand, Screen, ScreenAction};

use super::map_wait_keys::{FlushMark, let_go_of};
use super::routing::Front;
use super::{AppScreen, Side, chord_command};

/// What the router holds for the release, in the order it came.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum HeldChord {
    /// A chord's press, as its command.
    Command(MenuCommand),
    /// A key or character typed after the chord.
    Input(Input),
    /// The keys were let go of ([`Screen::release_keys`]).
    LetGo,
}

/// The router's held chords, and where the screens stood as the first
/// was held.
#[derive(Clone, Debug, Default)]
pub(super) struct HeldChords {
    queue: Vec<HeldChord>,
    /// Where the screens stood as the chord was held, while the queue
    /// isn't empty. Nothing opens while a press holds its keys, as the
    /// original's held loops run nothing else, so it is where they stand
    /// as the call that ends the hold begins.
    mark: Option<ChordMark>,
    /// The tests' probe: each command the replay ran, with the screen in
    /// front as it ran, for a chord whose effect leaves nothing to see.
    #[cfg(test)]
    replayed: Vec<(MenuCommand, nova_view::Showing)>,
}

/// What says that something opened that flushes the queue: the
/// `map_wait_keys` flush mark, less the Preferences, and whether a map is
/// open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChordMark {
    flush: FlushMark,
    map: bool,
}

impl ChordMark {
    /// Whether something that flushes opened since `before`.
    fn flushed_since(self, before: Self) -> bool {
        self.flush.flushed_since(before.flush) || (self.map && !before.map)
    }
}

impl AppScreen {
    /// Whether keys typed now would come after keys a screen holds back
    /// for a held press ([`Screen::holds_back_keys`]) or a map's wait:
    /// always while a map waits for M's release, else the overlay in
    /// front's, the spaceport's when it is in front or shown with nothing
    /// in front, and none otherwise.
    pub(super) fn holds_back_keys(&self) -> bool {
        if self.map_waits() {
            return true;
        }
        let spaceport = || self.spaceport.as_ref().is_some_and(Screen::holds_back_keys);
        match self.front() {
            Some(Front::Overlay(overlay)) => self.overlay_holds_back_keys(overlay),
            Some(Front::Spaceport) => spaceport(),
            Some(Front::Map) => false,
            None => self.side == Side::Spaceport && spaceport(),
        }
    }

    /// Whether a map waits for M's release ([`FlightView::map_waiting`]).
    fn map_waits(&self) -> bool {
        self.flight.as_ref().is_some_and(FlightView::map_waiting)
    }

    /// Holds `input` for the replay, if it is a chord typed while keys are
    /// held back or anything typed after one but M during a map's wait:
    /// whether it did.
    pub(super) fn hold_chord(&mut self, input: &Input) -> bool {
        if !matches!(input, Input::Key { .. } | Input::Text(_)) {
            return false;
        }
        if !self.held_chords.queue.is_empty() {
            if self.map_waits()
                && self
                    .flight
                    .as_ref()
                    .is_some_and(|flight| flight.is_map_key(input))
            {
                return false;
            }
            self.held_chords.queue.push(HeldChord::Input(*input));
            return true;
        }
        let Input::Key {
            key, pressed: true, ..
        } = *input
        else {
            return false;
        };
        let Some(command) = self.held_command(key) else {
            return false;
        };
        self.held_chords.mark = Some(self.chord_mark());
        self.held_chords.queue.push(HeldChord::Command(command));
        true
    }

    /// The command `key` chords, with Command held, while keys are held
    /// back.
    fn held_command(&self, key: Key) -> Option<MenuCommand> {
        if !self.command_held.held() || !self.holds_back_keys() {
            return None;
        }
        chord_command(key)
    }

    /// Runs `replay`, keys typed before the held chords, with the queue
    /// set aside, so its nested routes see it empty and the keys reach
    /// the screens rather than queue behind the chords.
    pub(super) fn ahead_of_held_chords(&mut self, replay: impl FnOnce(&mut Self)) {
        let held = mem::take(&mut self.held_chords);
        replay(self);
        self.held_chords = held;
    }

    /// Adds a let-go to the held chords, if any are held.
    pub(super) fn hold_let_go(&mut self) {
        if !self.held_chords.queue.is_empty() {
            self.held_chords.queue.push(HeldChord::LetGo);
        }
    }

    /// Runs the held chords once nothing holds keys back, as the module
    /// docs say: [`ScreenAction::Quit`] when one quits.
    pub(super) fn replay_held_chords(&mut self) -> ScreenAction {
        if self.held_chords.queue.is_empty() || self.holds_back_keys() {
            return ScreenAction::None;
        }
        let queue = mem::take(&mut self.held_chords.queue);
        let mut mark = self.held_chords.mark.take();
        let mut down = KeysDown::default();
        let mut held = queue.into_iter();
        while let Some(item) = held.next() {
            if mark.is_some_and(|before| self.chord_mark().flushed_since(before)) {
                self.let_go_of_held(std::iter::once(item).chain(held), down);
                return ScreenAction::None;
            }
            mark = Some(self.chord_mark());
            let action = match item {
                HeldChord::Command(command) => {
                    #[cfg(test)]
                    {
                        let showing = self.showing();
                        self.held_chords.replayed.push((command, showing));
                    }
                    self.run_menu_command(command)
                }
                HeldChord::Input(input) => {
                    down.follow(&input);
                    self.rerouted_input(&input)
                }
                HeldChord::LetGo => {
                    self.let_go_of_keys();
                    down.let_go();
                    ScreenAction::None
                }
            };
            if action == ScreenAction::Quit {
                return ScreenAction::Quit;
            }
        }
        ScreenAction::None
    }

    /// Past a flush, the queued release of each key in `down`, the presses
    /// replayed and not yet let go, and the let-goes, in order; nothing
    /// else of `rest`, as the map's wait keeps only those
    /// (`map_wait_keys`).
    fn let_go_of_held(&mut self, rest: impl Iterator<Item = HeldChord>, mut down: KeysDown) {
        for item in rest {
            match item {
                HeldChord::Input(input) => {
                    if let_go_of(&mut down, &input) {
                        let _ = self.rerouted_input(&input);
                    }
                }
                HeldChord::LetGo => {
                    self.let_go_of_keys();
                    down.let_go();
                }
                HeldChord::Command(_) => {}
            }
        }
    }

    /// Whether the router holds chords for a replay: the tests' probe.
    #[cfg(test)]
    pub(super) fn holds_chords(&self) -> bool {
        !self.held_chords.queue.is_empty()
    }

    /// The commands the held chords' replays ran, with the screen in front
    /// as each ran: the tests' probe.
    #[cfg(test)]
    pub(super) fn replayed_chords(&self) -> &[(MenuCommand, nova_view::Showing)] {
        &self.held_chords.replayed
    }

    /// Where the screens stand for [`ChordMark`]: the Preferences don't
    /// count, as `_RunPrefsDialog` (@0xcd401) flushes nothing.
    fn chord_mark(&self) -> ChordMark {
        let mut flush = self.flush_mark();
        flush.layers -= usize::from(self.preferences.is_some());
        ChordMark {
            flush,
            map: self.flight.as_ref().is_some_and(FlightView::map_open),
        }
    }
}
