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
mod hire;
mod hyperspace;
mod land_key;
mod landing;
mod menu_commands;
mod missions;
mod navigation;
mod news;
mod outfitter;
mod person_join;
mod persons;
mod pilot_move;
mod pilots;
mod preferences;
mod recharge;
mod ship_browser;
mod shipyard;
mod startup;
mod stock_app;
mod stock_take_off_cost;
mod system_view;
mod trade;
mod traffic;
mod widescreen;
#[cfg(windows)]
mod windows_exe;
mod world_layer;
