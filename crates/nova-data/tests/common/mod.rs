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

/// Every `*.ndat` file in the data directory, sorted by path.
pub fn ndat_files(dir: &Path) -> Vec<PathBuf> {
    data_files(dir, "ndat")
}

/// Every `*.rez` file in the data directory, sorted by path.
#[allow(dead_code)] // Only some of the binaries sharing this module use it.
pub fn rez_files(dir: &Path) -> Vec<PathBuf> {
    data_files(dir, "rez")
}

fn data_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("the data directory is readable")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == ext))
        .collect();
    files.sort();
    files
}
