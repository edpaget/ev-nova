//! The EV Nova player: the app layer, its command line, and the winit
//! platform adapter.
//!
//! - [`app`]: the core. It owns the window port, turns window events into
//!   `nova-view` input, routes them to the current screen and hands each
//!   frame's draw list to `nova-render`. No winit or wgpu types.
//! - [`cli`]: where the game data is, from the arguments or `NOVA_DATA`.
//! - [`platform`]: the thin winit adapter: event translation, the window
//!   port over a real window, and the event-loop runner.

pub mod app;
pub mod cli;
pub mod platform;
