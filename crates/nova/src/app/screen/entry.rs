//! [`AppScreen`]'s way into play from the main screen: the glue around
//! [`Entry`], which makes the decisions (`nova_view::menu::entry` says
//! what the original's `_EnterGameFromMainScreen` does, and
//! `nova_view::menu::intro` what its `_ShowIntroScreen` does).
//!
//! Enter Ship starts an entry ([`AppScreen::enter_ship`]). The main
//! screen's four sounds are halted ([`UiSound::MainScreenSoundsHalted`]),
//! after the click's own, and the menu lets go of its pointer and keys.
//! While the entry runs, the router gives it every input and the window's
//! close first, noting the pointer, the button and the keys as it always
//! does; neither the menu nor flight ticks. The entry holds the game in
//! the game slot until the handover, so the intro's text is munged for
//! the pilot held, as the About and X texts are, and the pilot is marked
//! seen there ([`FlightView::mark_intro_seen`]); nothing saves at entry,
//! as the original doesn't, so the flag reaches disk with the next save.
//!
//! **Who sees an intro.** A pilot that hasn't seen the intro screen
//! ([`FlightView::seen_intro`]) sees, by the engine's reading of
//! [`RuleKey::IntroForReopenedPilot`], the intro of the last pilot New
//! Pilot made in this run (remembered by [`AppScreen::hold_new_pilot`]),
//! or an empty one when none has been made since launch; by the other
//! reading, the first `chär`'s.
//!
//! **The text** opens in "Desc Dialog" over black, its `dësc` loaded and
//! munged as `_LoadDescription` + `_MungeBriefing(0,-1)` do for the About
//! and X texts; a `dësc` that can't be found shows an empty text, as
//! `_LoadDescription` @0x71a5e-0x71a81 gives one. Without dialogs it is
//! passed over.
//!
//! **The handover** moves the game into the flight slot and shows flight,
//! which a return from play comes back to the main menu from. Before
//! flight's first frame it lets go of its keys and pointer, so its cursor
//! starts hidden (`_PlayGame` zeroes the cursor's fade @0x45822), its
//! escort menu is shut (`escortMenuOnscreen` = 0 @0x1a39e), and it is
//! ticked once with no time and no key held: the warm-up frame
//! (`_DoPlayGameWork(1)` under `_ignoreKeys` and `_gameFrozen`
//! @0x1a37d-0x1a397, and `_PlayGame` @0x457f3-0x45819), which settles
//! the asteroid field and catches up with the session's events but runs
//! no step. Under the black of that frame, flight also reads the sheets
//! and target cards of the ship types the system's traffic will bring
//! ([`FlightView::warm_up_traffic`]), so the first step, which fills the
//! system, doesn't stall on reading them. The load or New Pilot already ran the rebuilds the original
//! makes at entry (`_PropagateMissionBitEffects`,
//! `_ResetPlayerPrecalcedValues`, `_HandleStellarSystemVisibility`,
//! `_HandleStellarSprites`), and nothing changes the game on the main
//! screen between that rebuild and entry, so a re-entry after Escape's
//! return, under the engine's reading of `escape_to_main_screen`, runs
//! no fresh one.
//!
//! **What the entry held** (Command chords, the window's close) is
//! replayed into the router at the handover, where
//! `_IterateInGameEventLoop(-1)` (@0x1a446) drains the original's queue,
//! so what they open shows over flight's fade-in; what it holds during
//! the fade-in is replayed as it ends. The replay goes past the entry,
//! which is set aside meanwhile. A chord replays as the menu command it
//! names, never as keys into flight, which so holds no key down from the
//! entry.
//!
//! Ordering: the main-screen sound halt lands one audio feed after a
//! keyed click's 601, which the click's end raised: inaudible, and before
//! any boundary the player sees.
//!
//! Left out, the display-mode check and the text's movie:
//! rdm:phase/main-screen-fidelity/phase-6-entry-left-outs. The starfield's
//! re-randomising at entry, and its clear in a system of negative murk,
//! are the flight's (`nova_view::flight::FlightView`), which scatters its
//! stars when it is first caught up with.

use std::time::Duration;

use nova_sim::{RuleKey, RuleSource};
use nova_view::flight::FlightView;
use nova_view::menu::entry::{Deferred, Entry, Verdict};
use nova_view::menu::intro::{CharacterIntro, Intro, IntroSource};
use nova_view::ui::DescDialog;
use nova_view::{DrawList, Input, Key, MenuCommand, Screen, ScreenAction, Showing, Sound, UiSound};

use super::port_dialogs::desc_dialog;
use super::{AppScreen, CLASSIC_VIEW, Side, chord_command};

/// An entry running, and its intro's text while it is open.
#[derive(Clone, Debug)]
pub(super) struct Entering {
    core: Entry,
    text: Option<DescDialog>,
}

impl AppScreen {
    /// Enter Ship, as `_MainScreenChoice(3)` runs it (@0x1aa71): with a
    /// game in progress whose pilot isn't dying ([`FlightView::is_dying`],
    /// `_IsDying` @0x253d), play is entered from the main menu through
    /// the entry (see the module docs). The menu lets go of its keys and
    /// pointer as it hides, as `_FlushKeys` (@0x1aa7a) empties the key
    /// map; the game heard no press while it was held. Otherwise it beeps
    /// `_beepSnd[3]` (@0x1aa89-0x1aaac): [`UiSound::EnterRefused`].
    pub(super) fn enter_ship(&mut self) {
        if self.game.as_ref().is_none_or(FlightView::is_dying) {
            self.sounds.push(Sound::Ui(UiSound::EnterRefused));
            return;
        }
        self.sounds.push(Sound::Ui(UiSound::MainScreenSoundsHalted));
        let menu = self.shown_mut();
        menu.cancel_pointer();
        menu.release_keys();
        self.appear = None;
        let intro = self.entry_intro();
        self.entry = Some(Entering {
            core: Entry::new(intro),
            text: None,
        });
    }

    /// The intro the game held sees as it enters play: none once it has
    /// seen the intro screen; otherwise, by
    /// [`RuleKey::IntroForReopenedPilot`], the last New Pilot's intro in
    /// this run (an empty one with none), or the first `chär`'s.
    fn entry_intro(&self) -> Option<Intro> {
        let game = self.game.as_ref()?;
        if game.seen_intro() {
            return None;
        }
        let intro = match self.rules.source_for(RuleKey::IntroForReopenedPilot) {
            RuleSource::Engine => self.new_pilot_intro,
            RuleSource::Bible => self.data.first_character_intro(),
        };
        Some(Intro::new(
            &intro.unwrap_or(CharacterIntro::EMPTY),
            &*self.data,
        ))
    }

    /// Remembers the intro of the pilot New Pilot just made, the first
    /// `chär`'s, as `_DoNewPilot` fills the intro globals
    /// (`_GetCharResourceIntroIDs` @0x193ad).
    pub(super) fn remember_new_pilot_intro(&mut self) {
        self.new_pilot_intro = self.data.first_character_intro();
    }

    /// Whether play is being entered from the main screen: from Enter
    /// Ship until flight's fade-in ends.
    #[must_use]
    pub fn entering(&self) -> bool {
        self.entry.is_some()
    }

    /// The intro's text, while it is open.
    #[must_use]
    pub fn intro_text(&self) -> Option<&DescDialog> {
        self.entry.as_ref()?.text.as_ref()
    }

    /// `input` while play is entered: what it did, once the entry has
    /// dropped, held or given it to the intro's text, or `None` with no
    /// entry. The pointer and the keys are noted either way.
    pub(super) fn entry_input(&mut self, input: &Input) -> Option<ScreenAction> {
        self.entry.as_ref()?;
        self.note_pointer(input);
        self.note_keys(input);
        let command_held = self.command_held;
        let entering = self.entry.as_mut()?;
        if entering.core.input(input, command_held) == Verdict::Text
            && let Some(text) = &mut entering.text
        {
            text.input(input);
        }
        self.poll_intro_text();
        Some(ScreenAction::None)
    }

    /// Whether play is being entered and the entry holds a Command chord
    /// typed now, for the handover ([`Entry::holds_commands`]).
    pub(super) fn entry_holds_commands(&self) -> bool {
        self.entry
            .as_ref()
            .is_some_and(|entering| entering.core.holds_commands())
    }

    /// Whether play is being entered and the entry takes a Command chord
    /// typed now without holding it: the intro's text, due or showing,
    /// takes it as it takes every key.
    pub(super) fn entry_takes_commands(&self) -> bool {
        self.entry.is_some() && !self.entry_holds_commands()
    }

    /// The window's close while play is entered: `Some(false)`, staying
    /// open, once the entry has held or dropped it; `None` with no entry.
    pub(super) fn entry_close(&mut self) -> Option<bool> {
        self.entry.as_mut()?.core.close();
        Some(false)
    }

    /// The pointer cancelled while play is entered: the button counts as
    /// up, and the intro's text, if open, lets go. Whether an entry took
    /// it.
    pub(super) fn entry_cancel_pointer(&mut self) -> bool {
        let Some(entering) = &mut self.entry else {
            return false;
        };
        self.button_down = false;
        entering.core.cancel_pointer();
        if let Some(text) = &mut entering.text {
            text.cancel_pointer();
        }
        true
    }

    /// The keys let go of while play is entered: the router's notes of
    /// them are cleared, the entry's too, and the intro's text, if open,
    /// lets go. Whether an entry took it.
    pub(super) fn entry_release_keys(&mut self) -> bool {
        let Some(entering) = &mut self.entry else {
            return false;
        };
        entering.core.release_keys();
        if let Some(text) = &mut entering.text {
            text.release_keys();
        }
        self.command_held = false;
        self.keys_held = super::MenuKeys::default();
        true
    }

    /// Whether play is being entered, which keeps the menu and flight
    /// from ticking.
    pub(super) fn entry_holds(&self) -> bool {
        self.entry.is_some()
    }

    /// The scene the entry shows, until flight takes over.
    pub(super) fn entry_showing(&self) -> Option<Showing> {
        let showing = self.entry.as_ref()?.core.showing();
        (showing != Showing::Flight).then_some(showing)
    }

    /// Whether the entry hides the pointer, until flight takes over.
    pub(super) fn entry_hides_pointer(&self) -> Option<bool> {
        self.entry_showing()?;
        self.entry
            .as_ref()
            .map(|entering| entering.core.hides_pointer())
    }

    /// The entry's sounds and its text's since they were last taken.
    pub(super) fn take_entry_sounds(&mut self) -> Vec<Sound> {
        let Some(entering) = &mut self.entry else {
            return Vec::new();
        };
        let mut sounds: Vec<Sound> = entering
            .core
            .take_sounds()
            .into_iter()
            .map(Sound::Ui)
            .collect();
        if let Some(text) = &mut entering.text {
            sounds.extend(text.take_sounds());
        }
        sounds
    }

    /// Draws the entry over what the side shown drew: under the intro's
    /// text, the text, then the fades over it all, in the classic frame
    /// or in flight's widened view once flight shows.
    pub(super) fn draw_entry(&self, list: &mut DrawList) {
        let Some(entering) = &self.entry else {
            return;
        };
        let view = if self.side == Side::Flight {
            self.widened
        } else {
            CLASSIC_VIEW
        };
        entering.core.draw_under(list, view);
        if let Some(text) = &entering.text {
            text.draw(list);
        }
        entering.core.draw_over(list, view);
    }

    /// The intro's text, once closed: dropped, its sounds kept, and the
    /// entry told.
    fn poll_intro_text(&mut self) {
        let Some(entering) = &mut self.entry else {
            return;
        };
        if entering.text.as_ref().is_some_and(DescDialog::closed)
            && let Some(mut text) = entering.text.take()
        {
            self.sounds.extend(text.take_sounds());
            entering.core.text_closed();
        }
    }

    /// Moves the entry on by `dt`: its fades and intro, the intro's text
    /// opened when due and given what waited for it, the pilot marked
    /// seen, the handover when due, and the entry dropped once flight's
    /// fade-in ends, each replaying what the entry held.
    pub(super) fn advance_entry(&mut self, dt: Duration) {
        let Some(entering) = &mut self.entry else {
            return;
        };
        entering.core.tick(dt);
        if let Some(text) = &mut entering.text {
            text.tick(dt);
        }
        if let Some(id) = entering.core.take_text_due() {
            let text = self.intro_text_dialog(id);
            let Some(entering) = &mut self.entry else {
                return;
            };
            entering.core.text_opened(text.is_some());
            entering.text = text;
        }
        let Some(entering) = &mut self.entry else {
            return;
        };
        let waiting = entering.core.take_for_text();
        if let Some(text) = &mut entering.text {
            for input in &waiting {
                text.input(input);
            }
        }
        self.poll_intro_text();
        let Some(entering) = &mut self.entry else {
            return;
        };
        if entering.core.take_mark_seen()
            && let Some(game) = &mut self.game
        {
            game.mark_intro_seen();
        }
        if entering.core.handover_due() {
            self.hand_over();
        }
        let Some(entering) = &mut self.entry else {
            return;
        };
        if entering.core.done() {
            let held = entering.core.take_held();
            self.entry = None;
            self.replay_entry(held);
        }
    }

    /// "Desc Dialog" on the intro's text `id`, munged for the game held,
    /// or `None` without dialogs or when it can't be built (the reason to
    /// stderr). A `dësc` that can't be found is an empty text.
    fn intro_text_dialog(&mut self, id: i16) -> Option<DescDialog> {
        self.dialogs.as_ref()?;
        let words = self.main_screen_words(id).unwrap_or_default();
        let dialogs = self.dialogs.as_ref()?;
        match desc_dialog(dialogs, &self.data, &words) {
            Ok(dialog) => Some(dialog),
            Err(reason) => {
                eprintln!("nova: cannot show the intro's dësc {id}: {reason}");
                None
            }
        }
    }

    /// Moves the game into the flight slot and shows flight, which a
    /// return from play comes back to the main menu from; then enters play
    /// ([`FlightView::enter_game`]), readies flight's first frame (see the
    /// module docs), and replays what the entry held.
    fn hand_over(&mut self) {
        self.drop_flight();
        self.flight = self.game.take();
        self.return_to = Side::MainMenu;
        self.switch_to(Side::Flight);
        if let Some(flight) = &mut self.flight {
            // `_EnterGameFromMainScreen`'s refresh (@0x19f81).
            flight.enter_game();
            flight.cancel_pointer();
            flight.release_keys();
            flight.shut_escort_menu();
            flight.tick(Duration::ZERO);
            flight.warm_up_traffic();
        }
        let held = self
            .entry
            .as_mut()
            .map(|entering| entering.core.handed_over())
            .unwrap_or_default();
        self.replay_entry(held);
    }

    /// Replays `held` into the router past the entry, set aside
    /// meanwhile: each Command chord taken as the menu command it names
    /// ([`AppScreen::entry_command`]), and each close asked as a close
    /// now is, a quit either asks kept for [`Screen::take_quit`].
    fn replay_entry(&mut self, held: Vec<Deferred>) {
        if held.is_empty() {
            return;
        }
        let entering = self.entry.take();
        for deferred in held {
            match deferred {
                Deferred::Command(key) => self.entry_command(key),
                Deferred::Close => {
                    if self.close_asked() {
                        self.quit_due = true;
                    }
                }
            }
        }
        self.entry = entering;
    }

    /// A Command chord on `key` held by the entry, taken as a menu
    /// command, as the original's `MenuKey` takes it, and never routed as
    /// a key, so flight's keys stay up: Command-Q quits, as it does
    /// wherever it is pressed ([`AppScreen::route`]), and Command-comma
    /// opens the Preferences, with dialogs
    /// ([`AppScreen::preferences_key`]). Any other chord does nothing.
    fn entry_command(&mut self, key: Key) {
        match chord_command(key) {
            Some(MenuCommand::Quit) => self.quit_due = true,
            Some(MenuCommand::Preferences) => self.open_preferences(),
            Some(MenuCommand::About) | None => {}
        }
    }
}
