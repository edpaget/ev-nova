//! Tests against the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so these tests read its
//! location from `NOVA_DATA` (the `Nova Files` directory) and skip, passing,
//! when it is unset.

use std::path::{Path, PathBuf};

use encoding_rs::MACINTOSH;
use nova_data::records::desc::Desc;
use nova_data::records::string_list::StrList;
use nova_data::{AnyRecord, Entry, Record, decode_all, decode_file};
use nova_rsrc::ResourceFile;

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

/// Every stock file, opened.
fn stock_files(dir: &Path) -> Vec<(PathBuf, ResourceFile)> {
    ndat_files(dir)
        .into_iter()
        .map(|path| {
            let file = ResourceFile::open(&path).expect("stock file opens");
            (path, file)
        })
        .collect()
}

#[test]
fn every_stock_record_decodes_cleanly_and_round_trips_through_json() {
    let Some(dir) = nova_data() else { return };
    let mut problems = Vec::new();
    let mut decoded = 0;
    for (path, file) in stock_files(&dir) {
        let report = decode_file(&file);
        let name = path.file_name().expect("file name").to_string_lossy();
        problems.extend(report.errors.iter().map(|e| format!("{name}: error: {e}")));
        problems.extend(
            report
                .warnings
                .iter()
                .map(|w| format!("{name}: warning: {w}")),
        );
        for entry in &report.records {
            let json = serde_json::to_string(&entry.record).expect("serializes");
            let back: AnyRecord = serde_json::from_str(&json).expect("deserializes");
            if back != entry.record {
                problems.push(format!(
                    "{name}: {} {} does not round-trip through JSON",
                    entry.record.res_type(),
                    entry.id
                ));
            }
        }
        decoded += report.records.len();
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert!(decoded > 0, "decoded some records");
}

/// Every record of type `T` across the stock files.
fn all<T: Record>(files: &[(PathBuf, ResourceFile)]) -> Vec<Entry<T>> {
    files
        .iter()
        .flat_map(|(_, file)| {
            let report = decode_all::<T>(file);
            assert!(report.errors.is_empty(), "{:?}", report.errors);
            report.entries
        })
        .collect()
}

fn mac_roman(text: &str) -> Vec<u8> {
    let (bytes, _, had_errors) = MACINTOSH.encode(text);
    assert!(!had_errors, "{text:?} is Mac Roman");
    bytes.into_owned()
}

/// Every stock `dësc` and `STR#` text maps back to exactly the bytes it was
/// decoded from, so no Mac Roman character was lost or substituted. The
/// stock text has no curly quotes (the synthetic tests cover those) but does
/// use accented letters.
#[test]
fn stock_text_decodes_mac_roman_losslessly() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let mut non_ascii = String::new();

    for (_, file) in &files {
        for res in file.resources(Desc::TYPE) {
            let raw_text = &res.data()[..res
                .data()
                .iter()
                .position(|&b| b == 0)
                .expect("NUL-terminated")];
            let (entry, _) = nova_data::decode::<Desc>(&res).expect("decodes");
            assert_eq!(mac_roman(&entry.record.text), raw_text, "dësc {}", res.id());
            non_ascii.extend(entry.record.text.chars().filter(|c| !c.is_ascii()));
        }
        for res in file.resources(StrList::TYPE) {
            let data = res.data();
            let (entry, _) = nova_data::decode::<StrList>(&res).expect("decodes");
            let mut pos = 2;
            for text in &entry.record.strings {
                let len = usize::from(data[pos]);
                assert_eq!(
                    mac_roman(text),
                    &data[pos + 1..pos + 1 + len],
                    "STR# {}",
                    res.id()
                );
                pos += 1 + len;
                non_ascii.extend(text.chars().filter(|c| !c.is_ascii()));
            }
            assert_eq!(pos, data.len(), "STR# {} has no leftover bytes", res.id());
        }
    }

    for expected in ['é', 'ë', 'ö', 'æ', '°'] {
        assert!(non_ascii.contains(expected), "{expected} in stock text");
    }
    assert!(!non_ascii.contains('\u{FFFD}'));
    assert_eq!(all::<Desc>(&files).len(), 3032);
    assert_eq!(all::<StrList>(&files).len(), 227);
}
