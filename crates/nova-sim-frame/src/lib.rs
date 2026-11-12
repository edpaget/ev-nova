//! The view vocabulary of EV Nova's simulation: the view models the
//! session fills for the flight view, as plain data and the pure rules
//! that decide them, with nothing of the session itself. It builds on the
//! foundation, `nova-sim-base`, the catalog, `nova-sim-catalog`, and the
//! state layer, `nova-sim-state`, whose `shown` module holds the view
//! types the layers above build. `nova-sim` re-exports each module at its
//! old path (`nova_sim::map_panel`, `nova_sim::glow`, …), so the rest of
//! the workspace reaches them through it, and keeps the session code that
//! gathers their facts.
//!
//! - [`glow`]: how bright a ship's engine glow is: its base level, which
//!   thrust ramps up and coasting down ([`ramp_glow`](glow::ramp_glow)),
//!   and the level drawn at a tick, flickering on the blink chance
//!   ([`glow_level`](glow::glow_level)).
//! - [`key_fade`]: what fades the flight cursor: which keys set
//!   `_keyPressedThisFrame`, past which gates
//!   ([`KeyFades`](key_fade::KeyFades)).
//! - [`map_borders`]: the galaxy map's government borders, as
//!   `_AddSystemInfluenceToMap` and `_RecalcMapInfluence` work them out: a
//!   grid of screen cells of four slots each ([`map_borders::grid`]), and
//!   the coarse and fine passes over it ([`map_borders::passes`]).
//! - [`map_panel`]: what the galaxy map's two info panels say about the
//!   system selected, as `_MapDialogUpdate` decides it: the ports, the
//!   hazards, the title, and the explored level's goods and services
//!   ([`map_panel::map_panel`]).
//! - [`map_search`]: the galaxy map's Find, as `_DoSystemSearch` matches a
//!   typed name against the systems' names ([`map_search::search`]).
//! - [`pilot_panel`]: what the main screen's pilot panel says about the
//!   pilot loaded ([`PilotPanel`](pilot_panel::PilotPanel)).
//! - [`red_alert`]: the Red Alert: when a threat to the player sounds it
//!   and the icon's blink ([`AlertIcon`](red_alert::AlertIcon)).
//! - [`status_cargo`]: what the status bar's cargo area lists
//!   ([`StatusCargo`](status_cargo::StatusCargo)).
//! - [`target_panel`]: what the status bar's target panel says of the
//!   target at its bottom: its
//!   [`TargetStatus`](target_panel::TargetStatus) and its
//!   [`TargetTag`](target_panel::TargetTag).
//!
//! A doc here links only to items of this crate, `nova-sim-base`,
//! `nova-sim-catalog` and `nova-sim-state`. An item of a crate above it is
//! named by its facade path as inline code, such as `nova_sim::Session`.

pub mod glow;
pub mod key_fade;
pub mod map_borders;
pub mod map_panel;
pub mod map_search;
pub mod pilot_panel;
pub mod red_alert;
pub mod status_cargo;
pub mod target_panel;
