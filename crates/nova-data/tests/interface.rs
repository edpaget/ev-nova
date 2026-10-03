//! Wiring: the interface file opened from a real file in a temporary
//! directory.

use std::fs;

use nova_data::records::dialog::Dlog;
use nova_data::{DitlId, InterfaceData, Record};
use nova_rsrc::LoadError;
use nova_rsrc::fixture::ForkBuilder;

#[test]
fn opens_a_flattened_fork_from_disk() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Nova-DF.rsrc");
    // DLOG 128: bounds, procID, visible, goAway, refCon, items 1000, no title.
    let dlog = [
        0, 0, 0, 0, 0, 100, 0, 200, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0x03, 0xE8, 0,
    ];
    let fork = ForkBuilder::new().resource(Dlog::TYPE, 128, None, &dlog);
    fs::write(&path, fork.build().bytes).expect("writes");

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
