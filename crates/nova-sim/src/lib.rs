//! EV Nova's game state and rules, with no rendering, windowing or audio:
//! the simulation takes the player's controls and advances the game state
//! one fixed step at a time.
//!
//! - [`catalog`]: the [`PilotCatalog`] port, what a flight session starts
//!   from: the first `chär`, its ship's handling fields and which systems
//!   exist.
//! - [`clock`]: the fixed-step clock, Nova's 1/30 s tick, which turns
//!   display frames of any length into whole simulation steps.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`flight`]: one tick of a ship's Newtonian flight, [`step`], under
//!   the player's [`Controls`].
//! - [`geometry`]: the simulation's own [`Vec2`], in pixels with y
//!   growing down.
//! - [`handling`]: a ship's [`Handling`], its `shïp`'s speed, acceleration
//!   and turn rate in pixels and ticks.
//! - [`session`]: a flight [`Session`], the first `chär`'s ship flying in
//!   its starting system.

pub mod catalog;
pub mod clock;
pub mod data;
pub mod flight;
pub mod geometry;
pub mod handling;
pub mod session;

pub use catalog::{CharacterStart, PilotCatalog, ShipId, StartError, SystemId};
pub use clock::{FixedStep, MAX_STEPS, Steps, TICK, TICKS_PER_SECOND};
pub use flight::{Controls, ShipState, Turn, step};
pub use geometry::Vec2;
pub use handling::{Handling, ShipFields};
pub use session::Session;
