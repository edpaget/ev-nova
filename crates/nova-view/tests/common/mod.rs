//! The program edge shared by the stock-data test binaries.
//!
//! This module is the only place `NOVA_DATA` and `NOVA_DATA_REZ` are read.
//! The game data is copyrighted and never committed, so each stock test asks
//! [`nova_data`] for the Mac `Nova Files` directory (or [`nova_data_rez`] for
//! the Windows one, of `.rez` files) and skips, passing, when it is unset.

use std::path::{Path, PathBuf};

/// The `Nova Files` directory named by `NOVA_DATA`, or `None` (after printing
/// a skip message) when it is unset.
pub fn nova_data() -> Option<PathBuf> {
    env_dir("NOVA_DATA")
}

/// The Windows `Nova Files` directory named by `NOVA_DATA_REZ`, or `None`
/// (after printing a skip message) when it is unset.
#[allow(dead_code)] // Only some of the binaries sharing this module use it.
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

/// The Mac OS X interface file, `Nova-DF.rsrc`, beside the `Nova Files`
/// directory `data_dir`, or `None` (after printing a skip message) when it
/// is absent.
#[allow(dead_code)] // Only some of the binaries sharing this module use it.
pub fn interface_file(data_dir: &Path) -> Option<PathBuf> {
    beside(data_dir, "Nova-DF.rsrc")
}

/// The Windows interface file, `Nova.rez`, beside the Windows `Nova Files`
/// directory `data_dir`, or `None` (after printing a skip message) when it
/// is absent.
#[allow(dead_code)] // Only some of the binaries sharing this module use it.
pub fn interface_rez(data_dir: &Path) -> Option<PathBuf> {
    beside(data_dir, "Nova.rez")
}

#[allow(dead_code)] // Only some of the binaries sharing this module use it.
fn beside(data_dir: &Path, name: &str) -> Option<PathBuf> {
    let path = data_dir.parent().unwrap_or(data_dir).join(name);
    if path.is_file() {
        Some(path)
    } else {
        eprintln!("skipping: no {}", path.display());
        None
    }
}
