//! The game the engine runs, chosen once at startup: [`Ruleset::Stock`],
//! EV Nova from its `Nova Files`, or [`Ruleset::NewGame`], the new game
//! from the folder `GAME_DATA` names ([`chosen`]), which
//! `nova_data::new_game` reads (see [`crate::startup::run`]).
//!
//! A ruleset selects, as data on its [`Profile`], the readings its game
//! pins in the rulebook (see [`crate::rulebook::ruleset_rulebook`]) and
//! the folder its pilots are saved in (see [`crate::config::pilots_dir`]).
//! The simulation never sees the ruleset: it reads the rulebook's keys,
//! each defaulting to the original's reading.
//!
//! The stock game pins nothing. The new game pins the other reading of
//! the three keys its traffic needs, where a jump point's gate is the
//! only way in or out of a system: [`RuleKey::DestinationReach`] (its
//! jump points sit past the original's 999 bound),
//! [`RuleKey::ArrivalsThroughGates`] (every arrival with a usable gate
//! comes out of one) and [`RuleKey::DeparturesThroughGates`] (a ship with
//! nowhere left to go leaves through a gate). A settings
//! `"rule_overrides"` entry still moves each.

use std::ffi::OsString;
use std::path::PathBuf;

use nova_sim::{RuleKey, RuleSource};

use crate::config::PILOTS_DIR;

/// The game the engine runs, chosen at startup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ruleset {
    /// EV Nova from its `Nova Files`: the original's behaviour.
    #[default]
    Stock,
    /// The new game from its `GAME_DATA` folder.
    NewGame,
}

/// What a ruleset selects, as data.
#[derive(Debug, PartialEq, Eq)]
pub struct Profile {
    /// The readings the game pins: applied over the settings' `"rules"`
    /// and under its `"rule_overrides"`.
    pub readings: &'static [(RuleKey, RuleSource)],
    /// The folder, in the `nova` data directory beside the stock one, its
    /// pilots are saved in.
    pub pilots: &'static str,
}

/// The stock game's profile: no pinned readings, today's `Pilots`.
const STOCK: Profile = Profile {
    readings: &[],
    pilots: PILOTS_DIR,
};

/// The new game's profile: its traffic's three gate readings (see the
/// module docs), and pilots of its own.
const NEW_GAME: Profile = Profile {
    readings: &[
        (RuleKey::DestinationReach, RuleSource::Bible),
        (RuleKey::ArrivalsThroughGates, RuleSource::Bible),
        (RuleKey::DeparturesThroughGates, RuleSource::Bible),
    ],
    pilots: "New Game Pilots",
};

impl Ruleset {
    /// What this ruleset selects. An exhaustive match: the app acts on
    /// every ruleset.
    #[must_use]
    pub const fn profile(self) -> &'static Profile {
        match self {
            Self::Stock => &STOCK,
            Self::NewGame => &NEW_GAME,
        }
    }
}

/// The ruleset `game_data`, the value of `GAME_DATA`, chooses:
/// [`Ruleset::NewGame`] with its folder when it is set and not empty,
/// else [`Ruleset::Stock`] with none, so a shell that exports it empty
/// runs the stock game.
#[must_use]
pub fn chosen(game_data: Option<OsString>) -> (Ruleset, Option<PathBuf>) {
    match game_data.filter(|value| !value.is_empty()) {
        Some(dir) => (Ruleset::NewGame, Some(PathBuf::from(dir))),
        None => (Ruleset::Stock, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_stock_game() {
        assert_eq!(Ruleset::default(), Ruleset::Stock);
    }

    #[test]
    fn without_game_data_or_with_it_empty_the_stock_game_runs() {
        assert_eq!(chosen(None), (Ruleset::Stock, None));
        assert_eq!(chosen(Some(OsString::new())), (Ruleset::Stock, None));
    }

    #[test]
    fn game_data_chooses_the_new_game_with_its_folder() {
        assert_eq!(
            chosen(Some(OsString::from("/g"))),
            (Ruleset::NewGame, Some(PathBuf::from("/g")))
        );
    }

    #[test]
    fn the_stock_pilots_stay_where_they_are_and_the_new_game_s_go_beside_them() {
        assert_eq!(Ruleset::Stock.profile().pilots, PILOTS_DIR);
        let new_game = Ruleset::NewGame.profile().pilots;
        assert!(!new_game.is_empty());
        assert_ne!(new_game, PILOTS_DIR);
        assert_eq!(new_game, "New Game Pilots");
    }

    #[test]
    fn the_stock_game_pins_no_reading() {
        assert_eq!(Ruleset::Stock.profile().readings, []);
    }

    #[test]
    fn the_new_game_pins_the_other_reading_of_its_three_gate_keys() {
        assert_eq!(
            Ruleset::NewGame.profile().readings,
            [
                (RuleKey::DestinationReach, RuleSource::Bible),
                (RuleKey::ArrivalsThroughGates, RuleSource::Bible),
                (RuleKey::DeparturesThroughGates, RuleSource::Bible),
            ]
        );
    }
}
