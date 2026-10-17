//! The program edge shared by the stock-data test binaries.
//!
//! This module is the only place `NOVA_DATA` and `NOVA_DATA_REZ` are read.
//! The game data is copyrighted and never committed, so each stock test asks
//! [`nova_data`] for the Mac `Nova Files` directory (or [`nova_data_rez`] for
//! the Windows one, of `.rez` files) and skips, passing, when it is unset.

use std::path::PathBuf;

/// The `Nova Files` directory named by `NOVA_DATA`, or `None` (after printing
/// a skip message) when it is unset.
pub fn nova_data() -> Option<PathBuf> {
    env_dir("NOVA_DATA")
}

/// The Windows `Nova Files` directory named by `NOVA_DATA_REZ`, or `None`
/// (after printing a skip message) when it is unset.
pub fn nova_data_rez() -> Option<PathBuf> {
    env_dir("NOVA_DATA_REZ")
}

fn env_dir(var: &str) -> Option<PathBuf> {
    let dir = std::env::var_os(var).map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: {var} not set");
    }
    dir
}
