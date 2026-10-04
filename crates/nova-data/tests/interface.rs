//! Wiring: the interface file opened from a real file in a temporary
//! directory.

use std::fs;

use nova_data::records::dialog::Dlog;
use nova_data::{DitlId, InterfaceData, Record, open_interface};
use nova_rsrc::LoadError;
use nova_rsrc::fixture::ForkBuilder;

/// A flattened fork holding `DLOG` 128: bounds, procID, visible, goAway,
/// refCon, items 1000, no title.
fn fork() -> Vec<u8> {
    let dlog = [
        0, 0, 0, 0, 0, 100, 0, 200, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0x03, 0xE8, 0,
    ];
    ForkBuilder::new()
        .resource(Dlog::TYPE, 128, None, &dlog)
        .build()
        .bytes
}

#[test]
fn opens_a_flattened_fork_from_disk() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Nova-DF.rsrc");
    fs::write(&path, fork()).expect("writes");

    let ui = InterfaceData::open(&path).expect("opens");
    assert_eq!(ui.path(), path);
    let (dialog, _) = ui.dialog(128).expect("present").expect("decodes");
    assert_eq!(dialog.record.items_id, DitlId(1000));
}

#[test]
fn a_missing_file_is_an_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let err = InterfaceData::open(&dir.path().join("Nova-DF.rsrc")).expect_err("absent");
    assert!(matches!(err, LoadError::Io { .. }), "{err:?}");
}

#[test]
fn the_interface_file_is_found_beside_the_data_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let data_dir = dir.path().join("Nova Files");
    fs::create_dir(&data_dir).expect("creates");
    let err = open_interface(&data_dir).expect_err("nothing beside it yet");
    assert!(matches!(err.tried[1].1, LoadError::Io { .. }), "{err:?}");

    fs::write(dir.path().join("Nova-DF.rsrc"), fork()).expect("writes");
    let ui = open_interface(&data_dir).expect("opens");
    assert_eq!(ui.path(), data_dir.join("../Nova-DF.rsrc"));
    assert!(ui.dialog(128).is_some());
}
