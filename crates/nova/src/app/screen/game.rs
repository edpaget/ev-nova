//! [`AppScreen`]'s game in progress: the pilot the main screen holds, as
//! the original's `_gameInProgress` with the player globals.
//!
//! The router keeps two slots: the game held on the main screen
//! ([`AppScreen::game_in_progress`]) and the flight shown
//! ([`AppScreen::flight_view`]). The launch fills the game slot with the
//! last pilot ([`AppScreen::open_last_pilot`], `_OpenLastPilotFile`
//! @0x764ad sets `_gameInProgress` @0x7659f). New Pilot and Open Pilot build a flight
//! and hold it, never showing it, as `_DoNewPilot` (@0x18b0a) and
//! `_MainScreenChoice(1)` return to the main screen with the pilot
//! loaded. Enter Ship moves the game into the flight slot and shows it
//! (`_MainScreenChoice(3)`, @0x1aa71), once the entry's fade out and any
//! intro have run (see `entry`). A return from play started from
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
use nova_sim::{LastRefresh, Pilot};
use nova_view::flight::{FlightStart, FlightView, begin_game};

use super::{AppScreen, Side};

impl AppScreen {
    /// `pilot`'s flight, built as every flight is ([`AppScreen::flight`])
    /// and shown nowhere: one `loaded` from a save reads its load's
    /// fallbacks from the router's rules ([`FlightStart::Loaded`]), a new
    /// one is flown as it is ([`FlightStart::Pilot`]), its neighbour pass
    /// reading `last_refresh`, the run's as the pilot was made
    /// ([`AppScreen::run_refresh`]); a loaded one is handed none.
    pub(super) fn pilot_flight(
        &self,
        pilot: Pilot,
        loaded: bool,
        last_refresh: Option<LastRefresh>,
    ) -> FlightView<Rc<GameData>> {
        if loaded {
            self.flight(FlightStart::Loaded(pilot), None)
        } else {
            self.flight(FlightStart::Pilot(pilot), last_refresh)
        }
    }

    /// Holds `flight` as the game in progress, in place of any game held
    /// before, whose ship slots' history, course map zoom and borders are
    /// kept for the flights after it ([`AppScreen::remember_flight`]).
    pub(super) fn hold_game(&mut self, flight: FlightView<Rc<GameData>>) {
        if let Some(old) = self.game.replace(flight) {
            self.remember_flight(&old);
        }
    }

    /// What the run's last galaxy refresh left hidden, for a new pilot's
    /// neighbour pass (see [`nova_sim::LastRefresh`]): the game held's,
    /// else the last flight put away's, else `None`, the launch's.
    pub(super) fn run_refresh(&self) -> Option<LastRefresh> {
        self.game
            .as_ref()
            .and_then(|game| game.sim().carry_over())
            .and_then(|carry| carry.last_refresh)
            .or_else(|| self.last_refresh.clone())
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
    /// flight shown, in place of the pilot saved under `replaces`, if any.
    fn save_game(&mut self, replaces: Option<&str>) {
        if let Some(warning) = self
            .game
            .as_ref()
            .and_then(|game| self.save_warning(game, replaces))
        {
            self.warnings.push(warning);
        }
    }

    /// Holds a new pilot's `flight`, its game begun (the `chär`'s
    /// `OnStart`, the edge's [`begin_game`]) and
    /// saved with what that did, as
    /// `_DoNewPilot` (@0x18b0a) runs `OnStart` before its first save
    /// (`_SavePilotFile` @0x1935f) and returns to the main screen with the
    /// pilot loaded; choice 0 runs no sweep after it (@0x1a795-0x1a7d2).
    /// What the settling then makes due is saved too, as after any input.
    /// Each save is in place of the pilot saved under `replaces`, if any
    /// (`nova_sim::saves::PilotKeeper::save_replacing`).
    /// Its intro is remembered for the entry
    /// ([`AppScreen::remember_new_pilot_intro`]). Its neighbour pass
    /// reads the run's last refresh, which it was built with
    /// ([`AppScreen::pilot_flight`]).
    pub(super) fn hold_new_pilot(
        &mut self,
        mut flight: FlightView<Rc<GameData>>,
        replaces: Option<&str>,
    ) {
        self.remember_new_pilot_intro();
        begin_game(&mut flight);
        flight.take_save_due();
        self.hold_game(flight);
        self.save_game(replaces);
        if self.settle_game() {
            self.save_game(replaces);
        }
    }
}
