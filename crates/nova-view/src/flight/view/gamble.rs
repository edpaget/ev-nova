//! [`FlightView`]'s bar race over the [`FlightSim`] port: what Gamble
//! opens ([`ScreenAsk::Gamble`]), a race run ([`FlightCommand::Race`])
//! and its winnings collected ([`FlightCommand::CollectWinnings`]), and
//! the race's Help text. Each forwards one call to the flight; what the
//! race draws, takes and pays is nova-sim's (`session/gamble.rs`).
//!
//! The race's result `dësc` and its Help `dësc` are read from the
//! catalog here and loaded through the flight
//! ([`FlightView::load_description`], `FlightCommand::LoadDescription`),
//! as every screen's `dësc` is: the session answers the race's draw alone
//! ([`RaceRun`]), and the winnings line it worded goes after the loaded
//! text ([`result_text`]). So the load writes the run's last movie, as
//! `_LoadDescription` does in `_DoRaceDialog` (@0x48d05-0x48d17,
//! @0x48b89).

use nova_sim::flight_frame::{Answer, FlightCommand, ScreenAnswer, ScreenAsk};
use nova_sim::gamble::{Bet, GambleOpening, RACE_HELP_DESC, RaceRun, result_text};

use crate::flight::sim_port::FlightSim;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::ui::mission_dialogs::DescShown;

use super::FlightView;

/// A race run, as the race and its result text show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceShown {
    /// The racer that won, 0 to 3.
    pub winner: u8,
    /// The most the next asked bet can stake.
    pub asked_max: i64,
    /// The result text: the winner's `dësc` loaded, with the winnings line
    /// on a win, its `Graphic` and its movie, which plays before it.
    pub desc: DescShown,
}

impl<C: DescriptionSource + DescriptionGraphics, S: FlightSim> FlightView<C, S> {
    /// What the bar's Gamble opens ([`ScreenAsk::Gamble`]), asked as it is
    /// pressed: none where there is no bar, or for a flight that never
    /// started.
    pub fn gamble_opening(&mut self) -> Option<GambleOpening> {
        match self.sim.screen(ScreenAsk::Gamble) {
            ScreenAnswer::Gamble(opening) => Some(opening),
            _ => None,
        }
    }

    /// Runs a race on `racer`, staking `bet` ([`FlightCommand::Race`],
    /// `nova_sim::Session::race`), then loads its result `dësc`: none when
    /// the flight runs no race.
    pub fn race(&mut self, racer: u8, bet: Bet) -> Option<RaceShown> {
        let Answer::Raced(run) = self.send(FlightCommand::Race { racer, bet }) else {
            return None;
        };
        Some(self.race_shown(&run))
    }

    /// `run`'s result: its `dësc` loaded, empty when it cannot be read,
    /// with its winnings line after it.
    fn race_shown(&mut self, run: &RaceRun) -> RaceShown {
        let id = run.desc();
        let text = self.load_description(id).unwrap_or_default();
        RaceShown {
            winner: run.winner,
            asked_max: run.asked_max,
            desc: DescShown {
                text: result_text(&text, run.winnings_line.as_deref()),
                graphic: self.catalog.description_graphic(id),
                movie: self.catalog.description_movie(id),
            },
        }
    }

    /// Credits the race's winnings due once its result text has closed
    /// ([`FlightCommand::CollectWinnings`],
    /// `nova_sim::Session::collect_winnings`): whether there were any.
    pub fn collect_winnings(&mut self) -> bool {
        self.send(FlightCommand::CollectWinnings) == Answer::Done
    }

    /// The race's Help text: `dësc` 32764 loaded ([`RACE_HELP_DESC`],
    /// @0x48b89), empty when it cannot be read.
    pub fn race_help(&mut self) -> String {
        self.load_description(RACE_HELP_DESC).unwrap_or_default()
    }
}
