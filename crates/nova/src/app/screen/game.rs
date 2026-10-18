//! [`AppScreen`]'s game in progress: the pilot the main screen holds, as
//! the original's `_gameInProgress` with the player globals.
//!
//! The router keeps two slots: the game held on the main screen
//! ([`AppScreen::game_in_progress`]) and the flight shown
//! ([`AppScreen::flight_view`]). New Pilot and Open Pilot build a flight
//! and hold it, never showing it, as `_DoNewPilot` (@0x18b0a) and
//! `_MainScreenChoice(1)` return to the main screen with the pilot
//! loaded. Enter Ship moves the game into the flight slot and shows it
//! (`_MainScreenChoice(3)`, @0x1aa71). A return from play started from
//! the menu (the death's, or Escape's under the engine's reading of
//! [`RuleKey::EscapeToMainScreen`](nova_sim::RuleKey::EscapeToMainScreen))
//! puts the pilot away and moves the flight back into the game slot, as
//! `_gameInProgress` is never cleared after launch (only `_InitEVNova`
//! @0x46689, @0x473b6).
//!
//! So, with the two slots:
//!
//! - a held game never ticks: only the side shown does;
//! - quitting from the menu puts nothing away a second time
//!   ([`Screen::quit`](nova_view::Screen::quit) reads the flight slot
//!   only), as the original saves nothing on a quit from the main screen;
//! - a dead pilot held can't end play again;
//! - the developer's F still flies a fresh unnamed pilot when no flight
//!   is shown, and the pilot desk is the flight's alone.

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::Pilot;
use nova_view::flight::FlightView;
use nova_view::{Sound, UiSound};

use super::{AppScreen, Side};

impl AppScreen {
    /// `pilot`'s flight, built as every flight is ([`AppScreen::flight`])
    /// and shown nowhere: one `loaded` from a save reads its load's
    /// fallbacks from the router's rules ([`FlightView::loaded`]), a new
    /// one is flown as it is ([`FlightView::with_pilot`]).
    pub(super) fn pilot_flight(&self, pilot: Pilot, loaded: bool) -> FlightView<Rc<GameData>> {
        let rules = self.rules;
        self.flight(|data| {
            if loaded {
                FlightView::loaded(data, pilot, rules)
            } else {
                FlightView::with_pilot(data, pilot)
            }
        })
    }

    /// Holds `flight` as the game in progress, in place of any game held
    /// before, whose ship slots' history, course map zoom and borders are
    /// kept for the flights after it ([`AppScreen::remember_flight`]).
    pub(super) fn hold_game(&mut self, flight: FlightView<Rc<GameData>>) {
        if let Some(old) = self.game.replace(flight) {
            self.remember_flight(&old);
        }
    }

    /// Settles what the held game's hooks queued
    /// ([`FlightView::settle_script`]) at once, as the original runs them
    /// on the main screen (an Open's nebula sweep runs its `OnExplore`
    /// there), and whether that made a save due, which it takes.
    pub(super) fn settle_game(&mut self) -> bool {
        self.game.as_mut().is_some_and(|game| {
            game.settle_script();
            game.take_save_due()
        })
    }

    /// Enter Ship, as `_MainScreenChoice(3)` runs it (@0x1aa71): with a
    /// game in progress whose pilot isn't dying ([`FlightView::is_dying`],
    /// `_IsDying` @0x253d), any flight shown is dropped, keeping its
    /// settings, and the game is flown from the main menu, which a return
    /// from play comes back to. The menu lets go of its keys and pointer
    /// as it hides, as `_FlushKeys` (@0x1aa7a) empties the key map; the
    /// game heard no press while it was held. Otherwise it beeps
    /// `_beepSnd[3]` (@0x1aa89-0x1aaac): [`UiSound::EnterRefused`].
    pub(super) fn enter_ship(&mut self) {
        if self.game.as_ref().is_none_or(FlightView::is_dying) {
            self.sounds.push(Sound::Ui(UiSound::EnterRefused));
            return;
        }
        self.drop_flight();
        self.flight = self.game.take();
        self.return_to = Side::MainMenu;
        self.switch_to(Side::Flight);
    }

    /// Leaves play for the main menu keeping the pilot: puts it away
    /// (saved, or a strict pilot dying deleted, [`AppScreen::put_away_pilot`])
    /// before anything can show the menu or list the pilots, shows the
    /// menu, drops the spaceport and every dialog over flight, and holds
    /// the flight as the game in progress, as `_EnterGameFromMainScreen`
    /// returns to the main screen with `_gameInProgress` still set.
    pub(super) fn leave_play(&mut self) {
        self.put_away_pilot();
        self.switch_to(Side::MainMenu);
        self.drop_over_flight();
        if let Some(flight) = self.flight.take() {
            self.remember_flight(&flight);
            self.hold_game(flight);
        }
    }

    /// The flights alive, shown or held: what reaches every flight (double
    /// time, the preferences, the widened size) reaches both.
    pub(super) fn flights_mut(&mut self) -> impl Iterator<Item = &mut FlightView<Rc<GameData>>> {
        self.flight.iter_mut().chain(self.game.iter_mut())
    }

    /// Saves the game held, as [`AppScreen::save_pilot`] saves the
    /// flight shown.
    fn save_game(&mut self) {
        if let Some(warning) = self.game.as_ref().and_then(|game| self.save_warning(game)) {
            self.warnings.push(warning);
        }
    }

    /// Holds a new pilot's `flight`, its game begun (the `chär`'s
    /// `OnStart`, [`FlightView::begin`]) and saved with what that did, as
    /// `_DoNewPilot` (@0x18b0a) runs `OnStart` before its first save
    /// (`_SavePilotFile` @0x1935f) and returns to the main screen with the
    /// pilot loaded; choice 0 runs no sweep after it (@0x1a795-0x1a7d2).
    /// What the settling then makes due is saved too, as after any input.
    pub(super) fn hold_new_pilot(&mut self, mut flight: FlightView<Rc<GameData>>) {
        flight.begin();
        flight.take_save_due();
        self.hold_game(flight);
        self.save_game();
        if self.settle_game() {
            self.save_game();
        }
    }
}
