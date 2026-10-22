//! nova-sim's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod asteroids;
mod frame_rate;
mod headless;
mod idle_block;
mod landing;
mod lost_sight;
mod missions;
mod odds_cache;
mod plugin_strings;
mod stellar_hazards;
mod stock;
mod stock_afterburner;
mod stock_answer_flag;
mod stock_asteroids;
mod stock_board_reach;
mod stock_cloak_when_hit_burst;
mod stock_comm_calls;
mod stock_conditional_text;
mod stock_crons;
mod stock_escape_pod;
mod stock_galaxy_map;
mod stock_galaxy_visibility;
mod stock_inertialess;
mod stock_ionization;
mod stock_jump_arrival;
mod stock_leave;
mod stock_map_panel;
mod stock_map_search;
mod stock_miners;
mod stock_mission_ships;
mod stock_missions;
mod stock_murk;
mod stock_news;
mod stock_npc_escape;
mod stock_npc_skill;
mod stock_pirates;
mod stock_plunder;
mod stock_pursuit;
mod stock_ranks;
mod stock_refuel_dock;
mod stock_reinforcements;
mod stock_repair_dock;
mod stock_scan;
mod stock_ship_display;
mod stock_ship_goal_walk;
mod stock_ship_lights;
mod stock_spawn_availability;
mod stock_stand_off;
mod stock_star_map;
mod stock_stellar_hazards;
mod stock_storyline;
mod stock_swarm;
mod stock_target_selection;
mod stock_tractor_hold;
mod stock_trade;
mod stock_turning;
mod stock_warp_end;
mod story;
mod swarm;
