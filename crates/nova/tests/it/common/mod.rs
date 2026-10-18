//! The program edge shared by the test modules: the stock data's
//! directory, and the router past its start-up appear ([`appeared`]).
//!
//! This module is the only place `NOVA_DATA` is read. The game data is
//! copyrighted and never committed, so each stock test asks [`nova_data`] for
//! the `Nova Files` directory and skips, passing, when it is unset.

use std::path::PathBuf;

/// The `Nova Files` directory named by `NOVA_DATA`, or `None` (after printing
/// a skip message) when it is unset.
pub fn nova_data() -> Option<PathBuf> {
    let dir = std::env::var_os("NOVA_DATA").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: NOVA_DATA not set");
    }
    dir
}

/// `screen` once its main screen has appeared: the second's fade from
/// black ticked out, then a click off every button, which finishes the
/// slides silently, and the sounds taken, so a test starts on the menu as
/// a player sees it settled.
#[must_use]
pub fn appeared(mut screen: nova::app::screen::AppScreen) -> nova::app::screen::AppScreen {
    use nova_view::{Input, MouseButton, Point, Screen};
    screen.tick(nova_view::fade::DEATH_FADE_FOR);
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(1.0, 1.0),
        });
    }
    screen.take_sounds();
    screen
}
