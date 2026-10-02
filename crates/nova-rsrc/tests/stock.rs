//! Tests against the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so these tests read its
//! location from `NOVA_DATA` (the `Nova Files` directory) and skip, passing,
//! when it is unset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nova_rsrc::{ResType, ResourceFile};

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

#[test]
fn parses_all_stock_files() {
    let Some(dir) = nova_data() else { return };
    let files = ndat_files(&dir);
    assert_eq!(files.len(), 21, "stock .ndat file count");
    for path in &files {
        if let Err(error) = ResourceFile::open(path) {
            panic!("{error}");
        }
    }
}

#[test]
fn known_counts() {
    let Some(dir) = nova_data() else { return };
    let mut totals: BTreeMap<ResType, usize> = BTreeMap::new();
    for path in ndat_files(&dir) {
        let file = ResourceFile::open(&path).expect("stock file parses");
        for ty in file.types() {
            *totals.entry(ty).or_default() += file.count(ty);
        }
    }
    let total = |code| totals[&ResType::from_mac_roman(code).expect("Mac Roman code")];
    assert_eq!(total("shïp"), 288);
    assert_eq!(total("mïsn"), 791);
    assert_eq!(total("sÿst"), 545);
    assert_eq!(total("PICT"), 671);
}

/// Cross-checks one file against macbinary and returns its per-type counts.
fn cross_checked_counts(path: &Path) -> Vec<(ResType, usize)> {
    let bytes = std::fs::read(path).expect("read stock file");
    let ours = ResourceFile::open(path).expect("stock file parses");
    let theirs = macbinary::ResourceFork::new(&bytes).expect("macbinary parses");
    let name = path.display();

    let their_types: Vec<_> = theirs.resource_types().collect();
    let our_types: Vec<ResType> = ours.types().collect();
    assert_eq!(
        our_types,
        their_types
            .iter()
            .map(|&item| ResType(macbinary_type(item)))
            .collect::<Vec<_>>(),
        "{name}: type list"
    );

    let declared = macbinary_counts(&bytes);
    let mut counts = Vec::new();
    for (&ty, &item) in our_types.iter().zip(&their_types) {
        let (declared, iterated) = declared[&ty.bytes()];
        assert_eq!(ours.count(ty), declared, "{name} {ty}: declared count");
        assert_eq!(ours.count(ty), iterated, "{name} {ty}: macbinary count");
        for (our, their) in ours.resources(ty).zip(theirs.resources(item)) {
            assert_eq!(our.id(), their.id(), "{name} {ty}: id");
            assert_eq!(our.data(), their.data(), "{name} {ty} {}: data", our.id());
            assert_eq!(
                our.name_bytes(),
                their.name_bytes(),
                "{name} {ty} {}: name",
                our.id()
            );
        }
        counts.push((ty, ours.count(ty)));
    }
    counts
}

#[test]
fn per_type_count_snapshot() {
    let Some(dir) = nova_data() else { return };
    let mut lines = Vec::new();
    let mut totals: BTreeMap<ResType, usize> = BTreeMap::new();
    for path in ndat_files(&dir) {
        let file_name = path.file_name().expect("file name").to_string_lossy();
        let mut counts = cross_checked_counts(&path);
        counts.sort();
        for (ty, count) in counts {
            lines.push(format!("{file_name}\t{ty}\t{count}"));
            *totals.entry(ty).or_default() += count;
        }
    }
    for (ty, count) in &totals {
        lines.push(format!("TOTAL\t{ty}\t{count}"));
    }
    lines.push(format!(
        "TOTAL\t{} types\t{}",
        totals.len(),
        totals.values().sum::<usize>()
    ));
    let rendered = lines.join("\n") + "\n";

    let expected = include_str!("snapshots/stock_type_counts.txt");
    if rendered != expected {
        eprintln!("rendered stock type counts:\n{rendered}");
    }
    assert!(
        rendered == expected,
        "stock type counts differ from tests/snapshots/stock_type_counts.txt"
    );
}
