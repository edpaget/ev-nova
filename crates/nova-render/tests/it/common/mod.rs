//! The program edge shared by the stock-data test modules, and the
//! drawing helpers the test modules share.
//!
//! This module is the only place `NOVA_DATA` and `GAME_DATA` are read. The
//! game data is copyrighted and never committed, so each stock test asks
//! [`nova_data`] for the `Nova Files` directory and skips, passing, when it
//! is unset. The new game's sample lives in another repository, so its
//! test asks [`game_data`] for its `data` folder the same way.

use std::path::PathBuf;

use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, Point};

/// The `Nova Files` directory named by `NOVA_DATA`, or `None` (after printing
/// a skip message) when it is unset.
pub fn nova_data() -> Option<PathBuf> {
    env_dir("NOVA_DATA")
}

/// The new game's data folder named by `GAME_DATA` (game-bible's `data`),
/// or `None` (after printing a skip message) when it is unset.
pub fn game_data() -> Option<PathBuf> {
    env_dir("GAME_DATA")
}

/// The directory the environment variable `var` names, or `None` (after
/// printing a skip message) when it is unset.
fn env_dir(var: &str) -> Option<PathBuf> {
    let dir = std::env::var_os(var).map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: {var} not set");
    }
    dir
}

/// Fills `area` with `color` as the port draws a filled rectangle: one
/// [`DrawList::line`] along its middle, as thick as the area. nova-view's
/// `draw::fill_rect` makes the same command, and its tests pin that.
pub fn fill(list: &mut DrawList, area: Bounds, color: Color) {
    let middle = area.center().y;
    list.line(
        Point::new(area.min.x, middle),
        Point::new(area.max.x, middle),
        area.height(),
        color,
    );
}
