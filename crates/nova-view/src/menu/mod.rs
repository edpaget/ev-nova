//! The main menu and the saved pilots' list.
//!
//! - [`main_menu`]: the [`MainMenu`], New Pilot, Open Pilot and Quit.
//! - [`pilot_list`]: the [`PilotList`], the saved pilots to open one of,
//!   over the main menu.

pub mod main_menu;
pub mod pilot_list;

pub use main_menu::{MainMenu, MenuChoice};
pub use pilot_list::{PilotList, PilotListOutcome};
