//! nova-sim's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod asteroids;
mod frame_rate;
mod headless;
mod landing;
mod missions;
mod plugin_strings;
mod stock;
mod stock_afterburner;
mod stock_asteroids;
mod stock_conditional_text;
mod stock_crons;
mod stock_escape_pod;
mod stock_galaxy_map;
mod stock_galaxy_visibility;
mod stock_inertialess;
mod stock_mission_ships;
mod stock_missions;
mod stock_news;
mod stock_npc_escape;
mod stock_ranks;
mod stock_scan;
mod stock_ship_goal_walk;
mod stock_star_map;
mod stock_storyline;
mod stock_turning;
mod story;
