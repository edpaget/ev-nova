//! nova's integration tests, built as one test binary, run by the full
//! gate (`mise run ci`) and by the pre-commit hook only when nova changes.
//!
//! They test **wiring** only: a window event reaches the screen, the app
//! routes between screens, game data loads into the view, preferences,
//! settings and pilots persist through the app's stores, and audio and
//! render receive the frame's output. A test of a rule (what a key
//! selects, a landing's outcome, a price, a mission's effect, wording, how
//! many frames something takes) goes in the crate that owns the rule
//! (nova-view against its `FakeFlight`, nova-sim against
//! `nova_sim::fixture::SessionBuilder`), so a change there fails there.
//!
//! Assert after `common::Frames::settle` or `settle_until`, never on a
//! particular frame. The `stock_*` modules run the stock data end to end
//! and skip without `NOVA_DATA`.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod bracket_art;
mod common;
mod prefs_fixture;

mod about_dialog;
mod app;
mod combat;
mod dev_overlay;
#[cfg(feature = "dev-tools")]
mod dev_tools_paint;
mod economy;
mod entering;
mod flight;
mod galaxy_map;
mod gamble;
mod gamepad;
mod hire;
mod hyperspace;
mod key_bindings;
mod land_key;
mod landing;
mod menu_commands;
mod missions;
mod movies;
mod navigation;
mod new_game;
mod new_game_collision;
mod new_game_conversation;
mod new_game_descriptions;
mod new_game_dialog_backing;
mod new_game_empty_services;
mod new_game_engine_strings;
mod new_game_galaxy_viewer;
mod new_game_hail_replies;
mod new_game_hull_numbers;
mod new_game_launchers;
mod new_game_looks;
mod new_game_missions;
mod new_game_models;
mod new_game_muzzles;
mod new_game_news;
mod new_game_outfits;
mod new_game_play;
mod new_game_port;
mod new_game_port_loop;
mod new_game_relations;
mod new_game_routes;
mod new_game_saved_ids;
mod new_game_screens;
mod new_game_shipyard;
mod new_game_sky;
mod new_game_trade;
mod new_game_traffic;
mod new_game_travel;
mod new_game_turning;
mod new_game_wrecks;
mod news;
mod outfitter;
mod pad_focus;
mod person_join;
mod persons;
mod pilot_move;
mod pilots;
mod preferences;
mod recharge;
mod sample_dock_scan;
mod sample_kits;
mod sample_missions;
mod sample_routes;
mod sample_starfield;
mod sample_status_bar;
mod sample_travel;
mod ship_browser;
mod shipyard;
mod startup;
mod stock_app;
mod stock_desc_movies_outside_missions;
mod stock_stellar_movies;
mod stock_take_off_cost;
mod system_view;
mod text_entry;
mod touch_pointer;
mod trade;
mod traffic;
mod widescreen;
#[cfg(windows)]
mod windows_exe;
mod world_layer;
