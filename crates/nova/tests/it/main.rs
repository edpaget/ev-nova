//! nova's integration tests, built as one test binary.
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
mod hire;
mod hyper_select;
mod hyperspace;
mod hyperspace_outfits;
mod land_key;
mod landing;
mod menu_commands;
mod missions;
mod navigation;
mod news;
mod outfitter;
mod person_join;
mod persons;
mod pilots;
mod preferences;
mod recharge;
mod ship_browser;
mod shipyard;
mod stock_app;
mod system_view;
mod trade;
mod traffic;
mod widescreen;
mod world_layer;
