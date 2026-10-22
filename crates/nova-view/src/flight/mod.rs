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
//! - [`murk`]: how murky a shot is far from the player's ship in a murky
//!   system, and the tint that darkens it.
//! - `pen`: the `Pen` the HUD's panels set their text with, and
//!   `_DrawCenteredText`'s centring.
//! - [`pods`]: the NPCs' escape pods adrift, from the fight's
//!   `PodEjected` events: their drift, their fade and their pool.
//! - [`radar`]: the radar's view model, the marks `_DrawStatusRadar`
//!   draws for a [`RadarScene`](radar::RadarScene) snapshot.
//! - [`shot_fade`]: how a shot fades in and out, and how a translucent
//!   one is added to what is under it ([`ShotShade`](shot_fade::ShotShade)).
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
pub mod effects;
pub mod escorts;
pub mod flash;
pub mod floating;
pub mod gate_colorize;
pub mod held_alt;
pub mod hud;
pub mod ion_tint;
pub mod jump;
pub mod lightning;
pub mod murk;
pub(crate) mod pen;
pub mod pods;
pub mod radar;
pub mod ship_layers;
pub mod shot_fade;
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
pub use escorts::{EscortMenu, EscortMenuColors, EscortMenuLooks};
pub use jump::{JumpEffect, JumpPhase};
pub use sprite::rotation_frame;
pub use view::{FlightOverlays, FlightView, MapOpener, PortScreen, SharedChance};
