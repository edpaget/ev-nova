//! The main menu and the saved pilots' list.
//!
//! - [`main_menu`]: the [`MainMenu`], the original's main screen: New
//!   Pilot, Open Pilot, Quit, Enter Ship, Preferences and About.
//! - [`art`]: the main screen's art in the core's terms, and the port it
//!   comes through ([`MainScreenSource`]); `data` adapts the game data
//!   to it.
//! - [`background`]: where the main screen's picture goes, and the
//!   corner its positions are offset from.
//! - [`tracking`]: the main screen's own button tracking and its sounds.
//! - [`logo`]: the logo's flicker.
//! - [`rollover`]: the picture naming the button under the pointer.
//! - [`pilot_list`]: the [`PilotList`], the saved pilots to open one of,
//!   over the main menu.

pub mod art;
pub mod background;
mod data;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod logo;
pub mod main_menu;
pub mod pilot_list;
pub mod rollover;
pub mod tracking;

pub use art::{MainScreenArt, MainScreenSource};
pub use main_menu::{Chosen, ChosenBy, MainMenu, MenuChoice};
pub use pilot_list::{PilotList, PilotListOutcome};
