//! The renderer-free vocabulary that EV Nova's screens produce and consume.
//!
//! A screen turns [`Input`] into state changes and draws itself into a
//! [`DrawList`] of [`DrawCommand`]s in logical coordinates, naming images by
//! [`ImageKey`]. `nova-render` turns draw lists into pixels; this crate knows
//! nothing about windows or GPUs.
//!
//! # Screens
//!
//! - [`ships`]: the ship browser, which pages through every `shïp` and
//!   reads the game data through its [`ships::ShipCatalog`] port.
//! - [`galaxy`]: the galaxy map, which draws every system, hyperlink and
//!   nebula and reads the game data through its
//!   [`galaxy::GalaxyCatalog`] port.
//! - [`system`]: the system view, which shows one system's stellars at
//!   their positions over a procedural parallax starfield, through a camera the
//!   keyboard moves, and reads the game data through its
//!   [`system::SystemCatalog`] port.
//! - [`navigator`]: the [`Navigator`], which shows the galaxy map and the
//!   system view opened from it, and goes back to the map on Escape.
//! - [`flight`]: the flight screen, which flies the player's ship
//!   (`nova_sim`'s session) in its starting system with the original's
//!   default keys, the camera following the ship, under a HUD laid out by
//!   the status bar's `ïntf`, and reads the ship's sprite and the status
//!   bar through its [`flight::ShipSprites`] and [`flight::StatusBars`]
//!   ports. L lands on the stellar the ship is over; Space and Control
//!   fire, W selects the secondary weapon, and Tab and R pick the target,
//!   and the fight is drawn with the looks read through its
//!   [`flight::CombatLooks`] port.
//! - [`menu`]: the main menu (New Pilot, Open Pilot and Quit) and the
//!   saved pilots' list.
//! - [`menu_command`]: the commands a platform's menu bar sends the game
//!   ([`MenuCommand`]): About, Preferences… and Quit.
//! - [`fade`]: the black screen fades, the death's fade out and the main
//!   screen's fade back in ([`fade::ScreenFade`]).
//! - [`spaceport`]: the spaceport landed on, its landscape, name,
//!   description and service buttons laid out by the interface file's
//!   "Spaceport" dialog, reading the stellar through its
//!   [`spaceport::SpaceportCatalog`] port.
//!
//! # Text
//!
//! Each [`DrawCommand::Text`] names its [`Font`], one of the game's two
//! interface fonts (Geneva or Charcoal); the renderer picks the face that
//! draws it. [`text`] holds the [`text::TextMetrics`] port that measures
//! text as it will be drawn, the line-height rule the renderer shares, and
//! word wrap.
//!
//! # Widgets
//!
//! - [`ui`]: immediate-mode widgets (buttons drawn with Nova's button
//!   graphics, scrolling text) and modal dialogs laid out from the
//!   interface file's `DLOG`/`DITL`s, such as the "Desc Dialog"
//!   ([`ui::DescDialog`]).
//!
//! # Sounds
//!
//! Screens report the sounds they make as events ([`sound`]): a
//! main-menu button pressed or released, the flight session's own, and a
//! fight's, heard from where they happen. The audio side decides what each plays. The
//! player's preferences, sound and hyperspace effects alike, are reported
//! as a [`Prefs`] ([`preferences`]), the display modes it offers for
//! exclusive fullscreen are [`VideoMode`]s ([`video_mode`]), and the
//! Scaling it lists is a [`Scaling`] ([`scaling`]): one choice, a per-frame
//! [`Upscaler`] ([`upscaler`]) or a load-time [`Enlargement`].
//!
//! # Display shapes
//!
//! Each screen asks for a [`ViewShape`] ([`view_shape`]): flight widens to
//! the window's aspect ratio, and every other screen keeps 1024x768.
//!
//! # Developer tools
//!
//! - [`devtools`]: the developer tools overlay's model, which `nova`
//!   draws with egui only in its `dev-tools` build.

pub mod color;
pub mod colorize;
pub mod devtools;
pub mod draw;
pub mod fade;
pub mod flight;
pub mod font;
pub mod galaxy;
pub mod geometry;
pub mod image;
pub mod input;
pub mod menu;
pub mod menu_command;
pub mod movie;
pub mod movie_frame;
pub mod navigator;
pub mod oval;
pub mod preferences;
pub mod scaling;
pub mod screen;
pub mod sheet_image;
pub mod ships;
pub mod sound;
pub mod spaceport;
pub mod system;
pub mod target_picture;
pub mod text;
mod time;
pub mod ui;
mod ui_sound_cue;
pub mod upscaler;
pub mod video_mode;
pub mod view_shape;

pub use color::Color;
pub use colorize::Colorize;
pub use draw::{Blend, DrawCommand, DrawLayer, DrawList};
pub use font::Font;
pub use geometry::Point;
pub use image::{ImageKey, ImageKind};
pub use input::{Input, Key, MouseButton, ScrollDelta};
pub use menu_command::MenuCommand;
pub use movie_frame::{MovieFrame, MovieScreen, StreamId};
pub use navigator::Navigator;
pub use preferences::{LaunchPrefs, Prefs};
pub use scaling::{Enlargement, Scaling};
pub use screen::{Diagnostic, Screen, ScreenAction, Showing};
pub use sheet_image::SheetImage;
pub use sound::{CombatSound, Sound, SoundPrefs, UiSound};
pub use upscaler::Upscaler;
pub use video_mode::{FullscreenKind, MonitorMode, VideoMode, listed_modes};
pub use view_shape::ViewShape;
