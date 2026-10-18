//! The main menu and the saved pilots' list.
//!
//! - [`main_menu`]: the [`MainMenu`], the original's main screen: New
//!   Pilot, Open Pilot, Quit, Enter Ship, Preferences and About.
//! - [`appear`]: how the main screen appears, at start-up and on each
//!   return from play: its fade, and what it does with the input
//!   meanwhile.
//! - [`intro`]: the intro screen a pilot sees as it first enters play,
//!   and the port its pictures come through ([`IntroSource`]).
//! - [`art`]: the main screen's art in the core's terms, and the port it
//!   comes through ([`MainScreenSource`]); `data` adapts the game data
//!   to it.
//! - [`background`]: where the main screen's picture goes, and the
//!   corner its positions are offset from.
//! - [`tracking`]: the main screen's own button tracking and its sounds.
//! - [`logo`]: the logo's flicker.
//! - [`rollover`]: the picture naming the button under the pointer.
//! - [`slides`]: the rows of buttons sliding in as the screen appears.
//! - [`pilot_panel`]: the panel about the pilot loaded under the
//!   buttons, or the message in its place, and the port its looks come
//!   through ([`PilotPanelSource`]).
//! - [`pilot_list`]: the [`PilotList`], the saved pilots to open one of,
//!   over the main menu.

pub mod appear;
pub mod art;
pub mod background;
mod data;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod intro;
pub mod logo;
pub mod main_menu;
pub mod pilot_list;
pub mod pilot_panel;
pub mod rollover;
pub mod slides;
pub mod tracking;

pub use art::{MainScreenArt, MainScreenSource};
pub use intro::{CharacterIntro, IntroSource};
pub use main_menu::{Chosen, ChosenBy, MainMenu, MenuChoice};
pub use pilot_list::{PilotList, PilotListOutcome};
pub use pilot_panel::{PanelPilot, PilotPanelSource};
