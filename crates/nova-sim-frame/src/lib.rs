//! The view vocabulary of EV Nova's simulation: the view models the
//! session fills for the flight view and the value types its calls take
//! and give, as plain data and the pure rules that decide them, with
//! nothing of the session itself. It builds on the
//! foundation, `nova-sim-base`, the catalog, `nova-sim-catalog`, and the
//! state layer, `nova-sim-state`, whose `shown` module holds the view
//! types the layers above build. `nova-sim` re-exports each module at its
//! old path (`nova_sim::map_panel`, `nova_sim::glow`, …), or each item
//! of a session module at its old path (`nova_sim::session::SavedNav`,
//! `nova_sim::SavedNav`), so the rest of the workspace reaches them
//! through it, and keeps the session code that gathers their facts.
//!
//! The view models:
//!
//! - [`flight_frame`]: the values the flight view's port carries, as far
//!   as they have moved down: the new game's storyline screens.
//! - [`galaxy_map`]: what a map marks beside the mission systems
//!   ([`MapMarks`](galaxy_map::MapMarks)).
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
//! The session's value types, each module named for the `nova_sim::session`
//! module that builds them:
//!
//! - [`edit`]: the pilot as the developer tools' desk reads it.
//! - [`events`]: what the session tells a view about the scene it draws.
//! - [`map_click`]: a press on an in-flight map and what the click did.
//! - [`mission_screens`]: the mission computer, the mission texts and the
//!   mission info screen.
//! - [`missions`]: what a mission's offer and abort refused or left shown.
//! - [`nav`]: the nav a mission dialog saves and puts back.
//! - [`person_offers`]: what hailing the player's target opened.
//! - [`persons`]: a person's hail quote said.
//! - [`script_effects`]: what settling a script and a take-off did.
//! - [`ship_change`]: a ship change's interface load.
//! - [`slot_visibility`]: where a ship's hover and click box is.
//! - [`stellar_pick`]: which stellar a navigation key selects and what the
//!   land key did, from `nova_sim::session` itself.
//!
//! A doc here links only to items of this crate, `nova-sim-base`,
//! `nova-sim-catalog` and `nova-sim-state`. An item of a crate above it is
//! named by its facade path as inline code, such as `nova_sim::Session`.

pub mod edit;
pub mod events;
pub mod flight_frame;
pub mod galaxy_map;
pub mod glow;
pub mod key_fade;
pub mod map_borders;
pub mod map_click;
pub mod map_panel;
pub mod map_search;
pub mod mission_screens;
pub mod missions;
pub mod nav;
pub mod person_offers;
pub mod persons;
pub mod pilot_panel;
pub mod red_alert;
pub mod script_effects;
pub mod ship_change;
pub mod slot_visibility;
pub mod status_cargo;
pub mod stellar_pick;
pub mod target_panel;
