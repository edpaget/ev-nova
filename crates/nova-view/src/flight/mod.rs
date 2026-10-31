//! The flight screen: the player's ship flying in its starting system.
//!
//! - [`alert`]: the Red Alert icon, drawn at the playfield's top-right.
//! - [`asteroids`]: drawing the asteroid field.
//! - [`boxes`]: drawing the cargo boxes a jettison leaves.
//! - [`brackets`]: the corner brackets round the nav target and the ship
//!   target, from the original's `cïcn` art.
//! - [`cadence`]: when the HUD refreshes, [`HudCadence`](cadence::HudCadence),
//!   and the target's blink on the radar.
//! - [`catalog`]: the [`ShipSprites`] port, the player ship's sprite
//!   sheet, and the [`StatusBars`] port, the `ïntf` status bar layouts.
//! - [`cursor`]: the mouse cursor sprite, animating over what the player
//!   can target.
//! - [`data`]: the ports' adapters over `nova_data`'s `GameData`.
//! - [`effects`]: the fight's explosions, debris and sounds, from the
//!   session's combat events.
//! - [`escorts`]: the escort menu, which picks the escorts a command goes
//!   to, its look ([`EscortMenuLooks`]), and the message a command gives.
//! - [`flash`]: a ship's shield flash, the timer a hit sets and the level
//!   its shield layer shows.
//! - [`frame_rate`]: the frame-rate readout's meter, which counts the
//!   frames drawn and says "Calculating FPS..." or "31 fps".
//! - [`held_alt`]: a ship's alt layer as last drawn, which a disabled
//!   ship of `shän` `Flags` 0x0020 keeps showing where it was.
//! - [`floating`]: the floating map shown while H or `\` is held: its
//!   frame, clock, zoom and click boxes, and the pass that clips and
//!   fades it.
//! - [`jump`]: the hyperspace jump's [`JumpEffect`], the screen fading to
//!   white as the ship warps away, and back.
//! - [`hud`]: the HUD, the `ïntf` status bar with its radar, shield,
//!   armour and fuel bars and the system's name.
//! - [`lightning`]: a lightning beam's zig-zag.
//! - [`murk`]: how murky a sprite is far from the player's ship in a murky
//!   system, and the colorize that mixes it toward its background.
//! - `pen`: the `Pen` the HUD's panels set their text with, and
//!   `_DrawCenteredText`'s centring.
//! - [`pods`]: the NPCs' escape pods adrift, from the fight's
//!   `PodEjected` events: their drift, their fade and their pool.
//! - [`radar`]: the radar's view model, the marks `_DrawStatusRadar`
//!   draws for a [`RadarScene`](radar::RadarScene) snapshot.
//! - [`shot_fade`]: how a shot fades in and out, and how a translucent
//!   one is added to what is under it ([`ShotShade`](shot_fade::ShotShade)).
//! - [`sim_adapter`]: [`SessionFlight`], the [`FlightSim`] adapter over a
//!   real session, with the collaborators its calls are handed.
//! - [`sim_port`]: the [`FlightSim`] port, what the flight view needs of
//!   the flight it shows: a frame, a step, the player's commands and the
//!   events to drain.
//! - `slot_pool`: the original's fixed sprite tables, a new item taking
//!   the first free slot and each drawn in slot order.
//! - [`smoke`]: the puffs a smoking shot leaves.
//! - [`spin`]: the index each spinning shot's frames start on.
//! - [`sprite`]: [`rotation_frame`], which frame of the ship's sheet shows
//!   its heading.
//! - [`stars`]: the starfield as the original builds it, 20 star sprites
//!   scattered, wrapped and scrolled about the ship.
//! - [`target`]: the target panel and the secondary weapon's line.
//! - [`trails`]: the particle trail a shot leaves at its tail.
//! - [`view`]: the [`FlightView`] screen, which runs the flight and draws
//!   it.
//! - [`weapon_glow`]: a ship's weapon glow, lit by a `Flags2` 0x0200
//!   weapon it fires.
//! - [`weapons`]: drawing shots and beams.

pub mod alert;
pub mod asteroids;
pub mod boxes;
pub mod brackets;
pub mod cadence;
pub mod catalog;
pub mod cloak_fade;
pub mod cursor;
pub mod data;
pub mod edge;
pub mod effects;
pub mod escorts;
#[cfg(test)]
mod fake_catalog;
pub mod flash;
pub mod floating;
pub mod frame_rate;
pub mod gate_colorize;
pub mod held_alt;
pub mod hud;
pub mod ion_tint;
pub mod jump;
pub mod lightning;
pub mod murk;
pub mod paint;
pub(crate) mod pen;
pub mod pods;
pub mod radar;
pub mod ship_layers;
pub mod shot_fade;
pub mod sim_adapter;
pub mod sim_port;
pub(crate) mod slot_pool;
pub mod smoke;
pub mod solid_runs;
pub mod spin;
pub mod sprite;
pub mod stars;
pub mod target;
pub mod trails;
pub mod view;
pub mod weapon_glow;
pub mod weapons;

pub use catalog::{
    BoomLook, CombatLooks, EffectSheet, LayerSheet, Looks, ShipSheet, ShipSprites, StarSheet,
    StatusBarLayout, StatusBars, TargetCard, WeaponLook,
};
#[cfg(any(test, feature = "fixture"))]
pub use edge::test_flight;
pub use edge::{EdgeCatalog, FlightStart, begin_game, enter_game, open_flight};
pub use escorts::{EscortMenu, EscortMenuColors, EscortMenuLooks};
pub use jump::{JumpEffect, JumpPhase};
pub use sim_adapter::{Collaborators, SessionFlight};
pub use sim_port::FlightSim;
pub use sprite::rotation_frame;
pub use view::{FlightOverlays, FlightView, MapOpener, PortScreen, RaceShown, SharedChance};
