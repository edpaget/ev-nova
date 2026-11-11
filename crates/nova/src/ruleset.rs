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
//! twenty keys: thirteen as a jump point's gate is the only way in or out
//! of a system, two as its story lives in port, one as its exchange
//! lists every good a port prices, three as its ships are 3D models, and
//! one as its sky is drawn in 3D:
//!
//! - the three its traffic needs: [`RuleKey::DestinationReach`] (its
//!   jump points sit past the original's 999 bound),
//!   [`RuleKey::ArrivalsThroughGates`] (every arrival with a usable gate
//!   comes out of one) and [`RuleKey::DeparturesThroughGates`] (a ship
//!   with nowhere left to go leaves through a gate);
//! - the four its map and courses need: [`RuleKey::GateLanes`] (every
//!   link reader reads the jump points' gate lanes, and J never makes a
//!   hyperspace jump), [`RuleKey::MapLinkPairing`] (a one-way lane only
//!   the higher-ID system lists is still drawn),
//!   [`RuleKey::MapLaneStyle`] (each lane drawn in its stability's look,
//!   one-way lanes arrowed) and [`RuleKey::GateCourse`] (a gate taken
//!   along the course keeps the rest of it);
//! - J's two: [`RuleKey::GateAutopilot`] (J with a system selected that
//!   a jump point here serves engages the gate autopilot, which flies the
//!   course gate by gate with the player's own controls; J never starts a
//!   hyperspace jump, as the new game has none) and
//!   [`RuleKey::NoGateRefusal`] (J with a system selected that no jump
//!   point here serves says "No jump point leads there.", where the
//!   original's J is silent, so the player learns why J did nothing);
//! - its passages' one: [`RuleKey::GateFuel`] (each hypergate passage
//!   costs the hull's gate fuel, one jump's worth when its data gives
//!   none, so the gates are what spend fuel and a port's Recharge refills
//!   it; a ship short of the fare is refused the gate, and an NPC short of
//!   it finds none open to it; the original's hypergates spend none);
//! - the three its mission ships need: [`RuleKey::MissionShipLinks`] (a
//!   mission's ships sent to "a system adjacent" are drawn over the
//!   gate lanes, not the Con slots), [`RuleKey::AuxShipLinks`] (a
//!   mission's auxiliary ships sent next to a system come into the
//!   systems its gate lanes reach) and [`RuleKey::GateExitCounts`] (a
//!   mission ship that leaves through a hypergate counts as one that
//!   left, as every departure is through a gate: by the original's
//!   reading a `take_point`'s guard driven off through the point's own
//!   gate would come back at the next setup, so its blockade could only
//!   be broken by destroying every guard). The new game's blockades
//!   (`nova_data::new_game`'s `blockades`) are its first mission ships;
//!   the first two are pinned for any that go further afield;
//! - its port's two: [`RuleKey::BarStory`] (the bar lists the landed
//!   stellar's own conversation and the storyline offers that hold there,
//!   any of which the player opens from it, and its News lists the news
//!   heard in the system with each item's source and age, as the new game
//!   has no bar `mïsn` and no news `STR#` lists; the scenes still open by
//!   themselves at landing) and [`RuleKey::EmptyServices`] (the Mission
//!   BBS is greyed while the mission computer has nothing to list, and
//!   the bar's Hire Escort while no ship is for hire, so a button the
//!   new game has nothing behind promises nothing: its only `mïsn` are
//!   hidden blockades and no ship is for hire);
//! - its exchange's one: [`RuleKey::JunkListing`] (each side of the
//!   exchange lists every special good the port prices, where the
//!   original lists only the highest-ID one, so two of the new game's
//!   goods past the six standard slots, priced at one level at one port,
//!   both trade there; `nova_data::new_game`'s `goods` maps them);
//! - its hulls' three: [`RuleKey::ContinuousHeadings`] (a ship's hull, and
//!   the weapon exit points and fixed beams that turn with it, stands at
//!   its heading itself rather than its sprite frame's angle, so a model
//!   hull turns smoothly; `nova_view::flight::ship_orientation` turns the
//!   model by it, and sprites are still drawn on their frame),
//!   [`RuleKey::DataCollision`] (a ship is struck as the disc of the
//!   collision radius its `size` gives, `nova_data::new_game`'s `fleet`,
//!   rather than by its sprite, as the new game's ships have no sprites
//!   to collide by; `nova_sim::combat::data_collision`) and
//!   [`RuleKey::MountedMuzzles`] (a shot starts at the muzzle of the
//!   outfit model mounted on its hardpoint, its weapon's reach ahead of
//!   the exit, `nova_data::new_game`'s `mounts`, rather than at the bare
//!   exit; `nova_sim::combat::muzzle`);
//! - its sky's one: [`RuleKey::LayeredStarfield`] (flight draws a sky of
//!   generated stars in layers at depth in the 3D world layer, busier
//!   than the original's 20 stars, drifting by their depth's parallax
//!   and dimmed by the system's murk, in place of the stand-in dots the
//!   new game's data draws for want of a star sheet; `nova_view::flight::sky`
//!   and `nova_render::sky`).
//!
//! It leaves [`RuleKey::MapPlotting`] at the original's Shift-click, a
//! hop at a time, so a course is plotted a lane at a time and J flies it.
//! A settings `"rule_overrides"` entry still moves each.

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

/// The new game's profile: its traffic's three gate readings, its map
/// and courses' four, J's two, its passages' one, its mission ships' three, its port's two,
/// its exchange's one, its hulls' three and its sky's one (see the module
/// docs), and pilots of its own.
const NEW_GAME: Profile = Profile {
    readings: &[
        (RuleKey::DestinationReach, RuleSource::Bible),
        (RuleKey::ArrivalsThroughGates, RuleSource::Bible),
        (RuleKey::DeparturesThroughGates, RuleSource::Bible),
        (RuleKey::GateLanes, RuleSource::Bible),
        (RuleKey::MapLinkPairing, RuleSource::Bible),
        (RuleKey::MapLaneStyle, RuleSource::Bible),
        (RuleKey::GateCourse, RuleSource::Bible),
        (RuleKey::GateAutopilot, RuleSource::Bible),
        (RuleKey::NoGateRefusal, RuleSource::Bible),
        (RuleKey::GateFuel, RuleSource::Bible),
        (RuleKey::MissionShipLinks, RuleSource::Bible),
        (RuleKey::AuxShipLinks, RuleSource::Bible),
        (RuleKey::GateExitCounts, RuleSource::Bible),
        (RuleKey::BarStory, RuleSource::Bible),
        (RuleKey::EmptyServices, RuleSource::Bible),
        (RuleKey::JunkListing, RuleSource::Bible),
        (RuleKey::ContinuousHeadings, RuleSource::Bible),
        (RuleKey::DataCollision, RuleSource::Bible),
        (RuleKey::MountedMuzzles, RuleSource::Bible),
        (RuleKey::LayeredStarfield, RuleSource::Bible),
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
    fn the_new_game_pins_the_other_reading_of_its_twenty_gate_lane_map_autopilot_fare_mission_ship_port_exchange_hull_and_sky_keys()
     {
        assert_eq!(
            Ruleset::NewGame.profile().readings,
            [
                (RuleKey::DestinationReach, RuleSource::Bible),
                (RuleKey::ArrivalsThroughGates, RuleSource::Bible),
                (RuleKey::DeparturesThroughGates, RuleSource::Bible),
                (RuleKey::GateLanes, RuleSource::Bible),
                (RuleKey::MapLinkPairing, RuleSource::Bible),
                (RuleKey::MapLaneStyle, RuleSource::Bible),
                (RuleKey::GateCourse, RuleSource::Bible),
                (RuleKey::GateAutopilot, RuleSource::Bible),
                (RuleKey::NoGateRefusal, RuleSource::Bible),
                (RuleKey::GateFuel, RuleSource::Bible),
                (RuleKey::MissionShipLinks, RuleSource::Bible),
                (RuleKey::AuxShipLinks, RuleSource::Bible),
                (RuleKey::GateExitCounts, RuleSource::Bible),
                (RuleKey::BarStory, RuleSource::Bible),
                (RuleKey::EmptyServices, RuleSource::Bible),
                (RuleKey::JunkListing, RuleSource::Bible),
                (RuleKey::ContinuousHeadings, RuleSource::Bible),
                (RuleKey::DataCollision, RuleSource::Bible),
                (RuleKey::MountedMuzzles, RuleSource::Bible),
                (RuleKey::LayeredStarfield, RuleSource::Bible),
            ]
        );
    }

    #[test]
    fn the_new_game_leaves_the_map_plotting_at_the_shift_click() {
        assert!(
            !Ruleset::NewGame
                .profile()
                .readings
                .iter()
                .any(|(key, _)| *key == RuleKey::MapPlotting)
        );
    }
}
