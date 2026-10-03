//! The EV Nova player: the app layer, its command line, and the winit
//! platform adapter.
//!
//! - [`app`]: the core. It owns the window port, turns window events into
//!   `nova-view` input, routes them to the current screen and hands each
//!   frame's draw list to `nova-render`. No winit or wgpu types.
//! - [`cli`]: where the game data is, from the arguments or `NOVA_DATA`.
//! - [`exit`]: how the program ends: its exit code and message when the
//!   window could not be opened.
//! - [`fonts`]: the fonts text is drawn in, from the result of loading the
//!   game's Charcoal.
//! - [`platform`]: the thin winit adapter: event translation, the window
//!   port over a real window, and the event-loop runner.
//!
//! With the `dev-tools` feature (`mise run dev`), `devtools` draws the
//! developer tools overlay with egui. Without it, egui is not linked.

pub mod app;
pub mod cli;
#[cfg(feature = "dev-tools")]
pub mod devtools;
pub mod exit;
pub mod fonts;
pub mod platform;
