//! Tests against the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so these tests read its
//! location from `NOVA_DATA` (the Mac `Nova Files` directory of `.ndat`
//! files) and `NOVA_DATA_REZ` (the Windows `Nova Files` directory of `.rez`
//! files), and skip, passing, when a variable they need is unset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nova_rsrc::{ResType, ResourceFile};

/// The program edge for these tests: the only place `NOVA_DATA` and
/// `NOVA_DATA_REZ` are read.
fn env_dir(var: &str) -> Option<PathBuf> {
    let dir = std::env::var_os(var).map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: {var} not set");
    }
    dir
}

/// The Mac `Nova Files` directory, from `NOVA_DATA`.
fn nova_data() -> Option<PathBuf> {
    env_dir("NOVA_DATA")
}

/// The Windows `Nova Files` directory, from `NOVA_DATA_REZ`.
fn nova_data_rez() -> Option<PathBuf> {
    env_dir("NOVA_DATA_REZ")
}

/// Every file with extension `ext` in the data directory, sorted by name.
fn data_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("the data directory is readable")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == ext))
        .collect();
    files.sort();
    files
}

/// Every `*.ndat` file in the data directory, sorted by name.
fn ndat_files(dir: &Path) -> Vec<PathBuf> {
    data_files(dir, "ndat")
}

/// Every `*.rez` file in the data directory, sorted by name.
fn rez_files(dir: &Path) -> Vec<PathBuf> {
    data_files(dir, "rez")
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

/// Renders per-file, per-type counts in the snapshot format, with totals.
fn render_counts(files: Vec<(String, Vec<(ResType, usize)>)>) -> String {
    let mut lines = Vec::new();
    let mut totals: BTreeMap<ResType, usize> = BTreeMap::new();
    for (file_name, mut counts) in files {
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
    lines.join("\n") + "\n"
}

fn assert_matches_snapshot(rendered: &str) {
    let expected = include_str!("snapshots/stock_type_counts.txt");
    if rendered != expected {
        eprintln!("rendered stock type counts:\n{rendered}");
    }
    assert!(
        rendered == expected,
        "stock type counts differ from tests/snapshots/stock_type_counts.txt"
    );
}

#[test]
fn per_type_count_snapshot() {
    let Some(dir) = nova_data() else { return };
    let files = ndat_files(&dir)
        .into_iter()
        .map(|path| {
            let file_name = path.file_name().expect("file name").to_string_lossy();
            (file_name.into_owned(), cross_checked_counts(&path))
        })
        .collect();
    assert_matches_snapshot(&render_counts(files));
}

/// A file's name with its extension swapped for `.ndat`, so a `.rez` file
/// is keyed like its Mac counterpart.
fn ndat_name(path: &Path) -> String {
    let stem = path.file_stem().expect("file name").to_string_lossy();
    format!("{stem}.ndat")
}

#[test]
fn parses_all_windows_stock_files() {
    let Some(dir) = nova_data_rez() else { return };
    let files = rez_files(&dir);
    assert_eq!(files.len(), 21, "stock .rez file count");
    for path in &files {
        if let Err(error) = ResourceFile::open(path) {
            panic!("{error}");
        }
    }
}

#[test]
fn windows_per_type_count_snapshot() {
    // The Windows files hold the same resources as the Mac ones, so their
    // counts match the same snapshot, keyed by the Mac file name.
    let Some(dir) = nova_data_rez() else { return };
    let files = rez_files(&dir)
        .into_iter()
        .map(|path| {
            let file = ResourceFile::open(&path).expect("stock .rez parses");
            let counts = file.types().map(|ty| (ty, file.count(ty))).collect();
            (ndat_name(&path), counts)
        })
        .collect();
    assert_matches_snapshot(&render_counts(files));
}

/// Every resource's name and data, keyed by `(type, id)`.
type Contents<'a> = BTreeMap<(ResType, i16), (Option<&'a [u8]>, &'a [u8])>;

fn contents(file: &ResourceFile) -> Contents<'_> {
    file.iter()
        .map(|r| ((r.res_type(), r.id()), (r.name_bytes(), r.data())))
        .collect()
}

#[test]
fn windows_resources_equal_their_mac_counterparts() {
    let (Some(mac), Some(windows)) = (nova_data(), nova_data_rez()) else {
        return;
    };
    let rez = rez_files(&windows);
    let ndat = ndat_files(&mac);
    assert_eq!(
        rez.iter().map(|p| ndat_name(p)).collect::<Vec<_>>(),
        ndat.iter().map(|p| ndat_name(p)).collect::<Vec<_>>(),
        "the same files on both sides"
    );
    let mut total = 0;
    for (rez, ndat) in rez.iter().zip(&ndat) {
        let windows = ResourceFile::open(rez).expect("stock .rez parses");
        let mac = ResourceFile::open(ndat).expect("stock .ndat parses");
        let (windows, mac) = (contents(&windows), contents(&mac));
        assert!(
            windows.keys().eq(mac.keys()),
            "{}: the same (type, id) keys",
            rez.display()
        );
        for (key, value) in &windows {
            assert_eq!(value, &mac[key], "{} {key:?}", rez.display());
        }
        total += windows.len();
    }
    assert_eq!(total, 8362);
}
