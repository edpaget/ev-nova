//! EV Nova's game state and rules, with no rendering, windowing or audio:
//! the simulation takes the player's controls and advances the game state
//! one fixed step at a time.
//!
//! - [`catalog`]: the [`PilotCatalog`] port, what a flight session starts
//!   from: the first `chär`, its ship's handling fields and default
//!   outfits, which systems exist and the star map.
//! - [`clock`]: the fixed-step clock, Nova's 1/30 s tick, which turns
//!   display frames of any length into whole simulation steps.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - `fixture` (with the `fixture` feature, and in this crate's tests):
//!   `MemoryPilots`, a pilot store in memory.
//! - [`date`]: the in-game [`GameDate`], a Gregorian day that advances
//!   one day a jump.
//! - [`flight`]: one tick of a ship's Newtonian flight, [`step`], under
//!   the player's [`Controls`].
//! - [`fuel`]: how much fuel a ship regenerates each tick, from its
//!   `shïp` and its outfits' fuel scoops.
//! - [`geometry`]: the simulation's own [`Vec2`], in pixels with y
//!   growing down.
//! - [`handling`]: a ship's [`Handling`], its `shïp`'s speed, acceleration
//!   and turn rate in pixels and ticks.
//! - [`hyperspace`]: the [`StarMap`] of hyperlinks and the routes along
//!   it, whether the ship can jump ([`check_jump`]), and where it arrives.
//! - [`landing`]: whether the ship can land, [`check_landing`], each
//!   [`LandingRefusal`] in the order it applies, and the [`Service`]s a
//!   stellar's flags offer.
//! - [`pilot`]: the [`Pilot`], everything about the player a save keeps:
//!   ship, location, date, cash, reserves, course, explored systems and
//!   legal records, starting from the first `chär`.
//! - [`reserves`]: a ship's [`Reserves`], its shield, armour and fuel
//!   [`Gauge`]s, full at its `shïp`'s values when it starts.
//! - [`save`]: the save schema: a pilot as versioned JSON and back,
//!   upgrading older saves.
//! - [`saves`]: the [`PilotStore`] port pilots are saved through, the key
//!   each is saved under, and the [`PilotKeeper`] that saves, lists and
//!   opens them.
//! - [`session`]: a flight [`Session`], a pilot's ship flying
//!   from its starting system, landing, and jumping along a plotted course.
//! - [`sound`]: the [`SimSound`] events a session emits as it thrusts,
//!   lands, takes off and jumps; the audio side decides what they play.

pub mod catalog;
pub mod clock;
pub mod data;
pub mod date;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod flight;
pub mod fuel;
pub mod geometry;
pub mod handling;
pub mod hyperspace;
pub mod landing;
pub mod pilot;
pub mod reserves;
pub mod save;
pub mod saves;
pub mod session;
pub mod sound;
#[cfg(test)]
mod testkit;

pub use catalog::{
    CharacterStart, GovtId, LandingSite, PilotCatalog, ShipId, SoundId, StarSystem, StartDate,
    StartError, StellarId, SystemId,
};
pub use clock::{FixedStep, MAX_STEPS, Steps, TICK, TICKS_PER_SECOND};
pub use date::GameDate;
pub use flight::{Controls, ShipState, Turn, step};
pub use fuel::{OutfitMod, fuel_regen_per_tick};
pub use geometry::Vec2;
pub use handling::{Handling, ShipFields};
pub use hyperspace::{JumpRefusal, RouteError, StarMap, check_jump};
pub use landing::{LandingRefusal, Service, check_landing, landing_radius, services};
pub use pilot::Pilot;
pub use reserves::{Gauge, Reserves};
pub use save::SaveError;
pub use saves::{PilotKeeper, PilotStore, pilot_key};
pub use session::Session;
pub use sound::SimSound;
