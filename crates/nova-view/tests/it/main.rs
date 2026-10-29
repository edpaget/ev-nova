//! nova-view's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod flight_seam;
mod hyperlink_graphs;
mod stock_combat_effects;
mod stock_dialogs;
mod stock_flight_hud;
mod stock_flight_seam;
mod stock_galaxy_map;
mod stock_main_screen;
mod stock_map_borders;
mod stock_resource_browser;
mod stock_ship_browser;
mod stock_spaceport;
mod stock_system_view;
