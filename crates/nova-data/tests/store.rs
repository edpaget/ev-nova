//! Wiring: a store opened from real folders in a temporary directory.

use std::fs;

use nova_data::Record;
use nova_data::records::ship::Ship;
use nova_data::store::order::IgnoreReason;
use nova_data::store::{GameData, IgnoredEntry, OpenError, Origin};
use nova_rsrc::LoadError;
use nova_rsrc::fixture::ForkBuilder;

/// A fork of ships, each `(id, holds)`, the `holds` field at offset 0.
fn ships(ships: &[(i16, i16)]) -> Vec<u8> {
    let mut builder = ForkBuilder::new();
    for &(id, holds) in ships {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed size")];
        bytes[..2].copy_from_slice(&holds.to_be_bytes());
        builder = builder.resource(Ship::TYPE, id, None, &bytes);
    }
    builder.build().bytes
}

fn holds_and_source(data: &GameData, id: i16) -> (i16, String) {
    let entry = data.get::<Ship>(id).expect("present").expect("decodes");
    let name = entry.source.path.file_name().expect("name");
    (entry.record.holds, name.to_string_lossy().into_owned())
}

#[test]
fn a_store_opens_from_real_data_and_plugin_folders() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let data_dir = tmp.path().join("Nova Files");
    let plugins = tmp.path().join("Plug-Ins");
    fs::create_dir_all(&data_dir).expect("mkdir");
    fs::create_dir_all(plugins.join("Extras")).expect("mkdir");

    fs::write(
        data_dir.join("Nova Data 1.ndat"),
        ships(&[(128, 1), (129, 1)]),
    )
    .expect("write");
    fs::write(data_dir.join("Nova Data 2.ndat"), ships(&[(130, 2)])).expect("write");
    fs::write(data_dir.join("Nova Music.mp3"), b"ID3").expect("write");
    fs::write(data_dir.join("Broken.ndat"), b"not a resource fork").expect("write");
    // `Extras` sorts before `Override.npif`, so its plug-in loads first.
    fs::write(
        plugins.join("Extras/Late Ship"),
        ships(&[(128, 10), (129, 10)]),
    )
    .expect("write");
    fs::write(plugins.join("Override.npif"), ships(&[(129, 20)])).expect("write");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&data_dir, plugins.join("Linked")).expect("link");

    let data = GameData::open(&data_dir, Some(&plugins)).expect("opens");

    assert_eq!(holds_and_source(&data, 128), (10, "Late Ship".to_owned()));
    assert_eq!(
        holds_and_source(&data, 129),
        (20, "Override.npif".to_owned())
    );
    assert_eq!(
        holds_and_source(&data, 130),
        (2, "Nova Data 2.ndat".to_owned())
    );
    let origins: Vec<Origin> = data.files().iter().map(|f| f.origin).collect();
    assert_eq!(
        origins,
        [Origin::Data, Origin::Data, Origin::PlugIn, Origin::PlugIn]
    );

    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut expected_ignored = vec![IgnoredEntry {
        path: data_dir.join("Nova Music.mp3"),
        reason: IgnoreReason::NotGameData("mp3".to_owned()),
    }];
    #[cfg(unix)]
    expected_ignored.push(IgnoredEntry {
        path: plugins.join("Linked"),
        reason: IgnoreReason::Symlink,
    });
    assert_eq!(data.ignored(), expected_ignored);

    let [failed] = data.failed() else {
        panic!("one failure: {:?}", data.failed())
    };
    assert_eq!(failed.path, data_dir.join("Broken.ndat"));
    assert!(
        matches!(failed.error, LoadError::Parse { .. }),
        "{:?}",
        failed.error
    );
}

/// A classic plug-in keeps its resources in the file's resource fork.
#[cfg(target_os = "macos")]
#[test]
fn a_classic_plugin_loads_from_its_resource_fork() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let data_dir = tmp.path().join("Nova Files");
    let plugins = tmp.path().join("Plug-Ins");
    fs::create_dir_all(&data_dir).expect("mkdir");
    fs::create_dir_all(&plugins).expect("mkdir");
    let classic = plugins.join("Classic Plug");
    fs::write(&classic, b"").expect("write");
    fs::write(classic.join("..namedfork/rsrc"), ships(&[(131, 7)])).expect("write fork");

    let data = GameData::open(&data_dir, Some(&plugins)).expect("opens");
    assert_eq!(holds_and_source(&data, 131), (7, "Classic Plug".to_owned()));
    assert!(data.failed().is_empty(), "{:?}", data.failed());
}

#[test]
fn a_missing_data_folder_does_not_open() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let err = GameData::open(&tmp.path().join("missing"), None).expect_err("missing");
    assert!(
        matches!(&err, OpenError::DataDir { path, .. } if *path == tmp.path().join("missing")),
        "{err:?}"
    );
}

#[test]
fn a_missing_plugins_folder_does_not_open() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let missing = tmp.path().join("Plug-Ins");
    let err = GameData::open(tmp.path(), Some(&missing)).expect_err("missing");
    assert!(
        matches!(&err, OpenError::PlugInsDir { path, .. } if *path == missing),
        "{err:?}"
    );
}
