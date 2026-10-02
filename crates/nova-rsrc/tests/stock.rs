//! Tests against the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so these tests read its
//! location from `NOVA_DATA` (the `Nova Files` directory) and skip, passing,
//! when it is unset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The program edge for these tests: the only place `NOVA_DATA` is read.
fn nova_data() -> Option<PathBuf> {
    let dir = std::env::var_os("NOVA_DATA").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: NOVA_DATA not set");
    }
    dir
}

/// Every `*.ndat` file in the data directory, sorted by name.
fn ndat_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("NOVA_DATA is a readable directory")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ndat"))
        .collect();
    files.sort();
    files
}

/// The four raw type-code bytes of a macbinary type list item.
fn macbinary_type(item: macbinary::resource::TypeListItem) -> [u8; 4] {
    item.resource_type().0.to_be_bytes()
}

/// One file's per-type counts as read by macbinary: `(declared, iterated)`.
///
/// The declared count is the file's own type list entry, which macbinary only
/// exposes through `size_hint` on a fresh iterator.
fn macbinary_counts(bytes: &[u8]) -> BTreeMap<[u8; 4], (usize, usize)> {
    let fork = macbinary::ResourceFork::new(bytes).expect("macbinary parses the fork");
    fork.resource_types()
        .map(|item| {
            let declared = fork
                .resources(item)
                .size_hint()
                .1
                .expect("reference list in bounds");
            let iterated = fork.resources(item).count();
            (macbinary_type(item), (declared, iterated))
        })
        .collect()
}

/// `shïp`, `mïsn`, `sÿst` in Mac Roman.
const SHIP: [u8; 4] = [b's', b'h', 0x95, b'p'];
const MISN: [u8; 4] = [b'm', 0x95, b's', b'n'];
const SYST: [u8; 4] = [b's', 0xD8, b's', b't'];
const PICT: [u8; 4] = *b"PICT";

#[test]
fn macbinary_reads_stock_counts() {
    let Some(dir) = nova_data() else { return };
    let files = ndat_files(&dir);
    assert_eq!(files.len(), 21, "stock .ndat file count");

    let mut totals: BTreeMap<[u8; 4], usize> = BTreeMap::new();
    for path in &files {
        let bytes = std::fs::read(path).expect("read stock file");
        for (ty, (declared, iterated)) in macbinary_counts(&bytes) {
            assert_eq!(declared, iterated, "{} {ty:?}", path.display());
            *totals.entry(ty).or_default() += iterated;
        }
    }

    assert_eq!(totals[&SHIP], 288);
    assert_eq!(totals[&MISN], 791);
    assert_eq!(totals[&SYST], 545);
    assert_eq!(totals[&PICT], 671);
    assert_eq!(totals.len(), 30);
    assert_eq!(totals.values().sum::<usize>(), 8362);
}
