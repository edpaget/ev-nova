//! The program edge shared by the stock-data test modules.
//!
//! This module is the only place `NOVA_DATA`, `NOVA_DATA_REZ` and
//! `GAME_DATA` are read. The game data is copyrighted and never committed,
//! so each stock test asks [`nova_data`] for the Mac `Nova Files` directory
//! (or [`nova_data_rez`] for the Windows one, of `.rez` files) and skips,
//! passing, when it is unset. The new game's sample data lives in the
//! game-bible checkout, so the sample tests ask [`game_data`] for its
//! `data` folder and skip the same way.
//!
//! It also holds the helpers that open the stock files and read big-endian
//! fields from resource bytes.

use std::path::{Path, PathBuf};

use nova_rsrc::ResourceFile;

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

/// The new game's data folder named by `GAME_DATA` (game-bible's `data`),
/// or `None` (after printing a skip message) when it is unset.
pub fn game_data() -> Option<PathBuf> {
    env_dir("GAME_DATA")
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
pub fn interface_file(data_dir: &Path) -> Option<PathBuf> {
    beside(data_dir, "Nova-DF.rsrc")
}

/// The Windows interface file, `Nova.rez`, beside the Windows `Nova Files`
/// directory `data_dir`, or `None` (after printing a skip message) when it
/// is absent.
pub fn interface_rez(data_dir: &Path) -> Option<PathBuf> {
    beside(data_dir, "Nova.rez")
}

fn beside(data_dir: &Path, name: &str) -> Option<PathBuf> {
    let path = data_dir.parent().unwrap_or(data_dir).join(name);
    if path.is_file() {
        Some(path)
    } else {
        eprintln!("skipping: no {}", path.display());
        None
    }
}

/// Every `*.ndat` file in the data directory, sorted by path.
pub fn ndat_files(dir: &Path) -> Vec<PathBuf> {
    data_files(dir, "ndat")
}

/// Every `*.rez` file in the data directory, sorted by path.
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

/// Every stock file, opened, with its name.
pub fn stock_files(dir: &Path) -> Vec<(String, ResourceFile)> {
    ndat_files(dir)
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            let file = ResourceFile::open(&path).expect("stock file opens");
            (name, file)
        })
        .collect()
}

/// The big-endian `u16` at byte `at` of `data`.
pub fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

/// The big-endian `u32` at byte `at` of `data`.
pub fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(data[at..at + 4].try_into().expect("four bytes"))
}
