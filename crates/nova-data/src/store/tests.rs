//! `GameData` over the fake ports.

use std::path::Path;

use nova_rsrc::fixture::{ForkBuilder, MAP_OFFSET_FIELD, RezBuilder};
use nova_rsrc::{LoadError, ResType};

use super::fake::{FakeForks, FakeTree};
use super::fs::EntryKind::{Dir, File};
use super::*;
use crate::decode::Record;
use crate::records::ship::Ship;

const PICT: ResType = ResType::new(*b"PICT");

/// A `shïp` whose `holds` (offset 0) is `holds`.
fn ship_bytes(holds: i16) -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed size")];
    bytes[..2].copy_from_slice(&holds.to_be_bytes());
    bytes
}

/// A fork defining one ship per `(id, holds)`, each named `{tag} {id}`.
fn ships(tag: &str, ships: &[(i16, i16)]) -> Vec<u8> {
    let mut builder = ForkBuilder::new();
    for &(id, holds) in ships {
        let name = format!("{tag} {id}");
        builder = builder.resource(Ship::TYPE, id, Some(name.as_bytes()), &ship_bytes(holds));
    }
    builder.build().bytes
}

/// The same ships as [`ships`], written as a Windows `.rez` file.
fn rez_ships(tag: &str, ships: &[(i16, i16)]) -> Vec<u8> {
    let mut builder = RezBuilder::new();
    for &(id, holds) in ships {
        let name = format!("{tag} {id}");
        builder = builder.resource(Ship::TYPE, id, Some(name.as_bytes()), &ship_bytes(holds));
    }
    builder.build().bytes
}

fn open(tree: &FakeTree, forks: &FakeForks, plugins: bool) -> GameData {
    GameData::load(
        tree,
        forks,
        Path::new("/d"),
        plugins.then_some(Path::new("/p")),
    )
    .expect("opens")
}

/// Ship `id`'s `holds`, name and source path.
fn ship(data: &GameData, id: i16) -> (i16, Option<&str>, &Path) {
    let entry = data.get::<Ship>(id).expect("present").expect("decodes");
    (entry.record.holds, entry.name, entry.source.path.as_path())
}

fn paths<'a>(files: impl IntoIterator<Item = &'a SourceFile>) -> Vec<&'a Path> {
    files.into_iter().map(|file| file.path.as_path()).collect()
}

#[test]
fn a_plugin_redefining_one_ship_replaces_only_that_ship() {
    let tree = FakeTree::new()
        .dir("/d", &[("Nova Data 1.ndat", File)])
        .dir("/p", &[("Better 129.npif", File)]);
    let forks = FakeForks::new()
        .file(
            "/d/Nova Data 1.ndat",
            ships("data", &[(128, 1), (129, 2), (130, 3)]),
        )
        .file("/p/Better 129.npif", ships("plug", &[(129, 20)]));
    let data = open(&tree, &forks, true);

    let stock = Path::new("/d/Nova Data 1.ndat");
    assert_eq!(ship(&data, 128), (1, Some("data 128"), stock));
    assert_eq!(
        ship(&data, 129),
        (20, Some("plug 129"), Path::new("/p/Better 129.npif"))
    );
    assert_eq!(ship(&data, 130), (3, Some("data 130"), stock));
}

/// Writes a file of ships named `{tag} {id}`, one per `(id, holds)`.
type ShipWriter = fn(&str, &[(i16, i16)]) -> Vec<u8>;

/// One ship as the store resolves it: ID, `holds`, name, source file stem
/// and shadowed file stems.
type ResolvedShip = (i16, i16, Option<String>, String, Vec<String>);

/// Every ship from a store whose plug-in `file` is written by `plugin`.
fn ships_with_override(file: &str, plugin: ShipWriter) -> Vec<ResolvedShip> {
    let stem = |path: &Path| {
        path.file_stem()
            .expect("stem")
            .to_string_lossy()
            .into_owned()
    };
    let path = format!("/p/{file}");
    let tree = FakeTree::new()
        .dir("/d", &[("Nova Data 1.ndat", File)])
        .dir("/p", &[(file, File), ("Zed.npif", File)]);
    let forks = FakeForks::new()
        .file(
            "/d/Nova Data 1.ndat",
            ships("data", &[(128, 1), (129, 2), (130, 3)]),
        )
        .file(&path, plugin("plug", &[(129, 20), (130, 30), (140, 40)]))
        // Loads after `Override`, so it wins ship 130 over it.
        .file("/p/Zed.npif", ships("zed", &[(130, 300)]));
    let data = open(&tree, &forks, true);
    assert!(data.failed().is_empty(), "{:?}", data.failed());
    assert!(data.ignored().is_empty(), "{:?}", data.ignored());
    data.ids(Ship::TYPE)
        .iter()
        .map(|&id| {
            let (holds, name, source) = ship(&data, id);
            let provenance = data.provenance(Ship::TYPE, id).expect("present");
            let shadowed = provenance.shadowed.iter().map(|f| stem(&f.path)).collect();
            (id, holds, name.map(str::to_owned), stem(source), shadowed)
        })
        .collect()
}

#[test]
fn a_rez_plugin_overrides_exactly_like_the_same_fork_plugin() {
    let rez = ships_with_override("Override.rez", rez_ships);
    assert_eq!(rez, ships_with_override("Override.npif", ships));
    let s = |text: &str| text.to_owned();
    assert_eq!(
        rez,
        [
            (128, 1, Some(s("data 128")), s("Nova Data 1"), vec![]),
            (
                129,
                20,
                Some(s("plug 129")),
                s("Override"),
                vec![s("Nova Data 1")]
            ),
            (
                130,
                300,
                Some(s("zed 130")),
                s("Zed"),
                vec![s("Nova Data 1"), s("Override")]
            ),
            (140, 40, Some(s("plug 140")), s("Override"), vec![]),
        ]
    );
}

#[test]
fn of_two_plugins_the_one_sorting_later_case_insensitively_wins() {
    // Finder order loads `a` then `B`, so `B` wins; byte order ('B' is
    // 0x42, 'a' 0x61) would load `B` first and let `a` win.
    let tree = FakeTree::new()
        .dir("/d", &[("Data", File)])
        .dir("/p", &[("B.npif", File), ("a.npif", File)]);
    let forks = FakeForks::new()
        .file("/d/Data", ships("data", &[(128, 1)]))
        .file("/p/a.npif", ships("a", &[(128, 2)]))
        .file("/p/B.npif", ships("B", &[(128, 3)]));
    let data = open(&tree, &forks, true);

    assert_eq!(ship(&data, 128), (3, Some("B 128"), Path::new("/p/B.npif")));
    let provenance = data.provenance(Ship::TYPE, 128).expect("present");
    assert_eq!(provenance.winner.path, Path::new("/p/B.npif"));
    assert_eq!(
        paths(provenance.shadowed),
        [Path::new("/d/Data"), Path::new("/p/a.npif")]
    );
}

#[test]
fn plugin_order_folds_case_beyond_ascii() {
    // Folded, `éz` sorts after `éa`, so `Éz` wins. Byte order (É is C3 89,
    // é is C3 A9) and ASCII-only folding both load `Éz` first instead.
    let tree = FakeTree::new()
        .dir("/d", &[])
        .dir("/p", &[("Éz.npif", File), ("éa.npif", File)]);
    let forks = FakeForks::new()
        .file("/p/éa.npif", ships("éa", &[(128, 2)]))
        .file("/p/Éz.npif", ships("Éz", &[(128, 3)]));
    let data = open(&tree, &forks, true);
    assert_eq!(ship(&data, 128).2, Path::new("/p/Éz.npif"));
}

#[test]
fn a_plugin_in_a_sub_folder_loads_at_its_folders_position() {
    let tree = FakeTree::new()
        .dir("/d", &[])
        .dir("/p", &[("Charlie", File), ("beta", Dir), ("Alpha", File)])
        .dir("/p/beta", &[("inner", File)]);
    let forks = FakeForks::new()
        .file("/p/Alpha", ships("Alpha", &[(128, 1)]))
        .file("/p/beta/inner", ships("inner", &[(128, 2), (129, 2)]))
        .file("/p/Charlie", ships("Charlie", &[(129, 3)]));
    let data = open(&tree, &forks, true);

    assert_eq!(
        paths(data.files()),
        [
            Path::new("/p/Alpha"),
            Path::new("/p/beta/inner"),
            Path::new("/p/Charlie")
        ]
    );
    assert_eq!(ship(&data, 128).2, Path::new("/p/beta/inner"));
    assert_eq!(ship(&data, 129).2, Path::new("/p/Charlie"));
    assert_eq!(data.files()[1].origin, Origin::PlugIn);
}

#[test]
fn every_resource_reports_its_file() {
    let tree = FakeTree::new()
        .dir("/d", &[("one", File), ("two", File)])
        .dir("/p", &[("plug", File)]);
    let forks = FakeForks::new()
        .file(
            "/d/one",
            ForkBuilder::new()
                .resource(Ship::TYPE, 128, None, &ship_bytes(1))
                .resource(PICT, 200, Some(b"one pict"), b"pict one")
                .build()
                .bytes,
        )
        .file(
            "/d/two",
            ForkBuilder::new()
                .resource(PICT, 201, None, b"pict two")
                .build()
                .bytes,
        )
        .file(
            "/p/plug",
            ForkBuilder::new()
                .resource(PICT, 200, Some(b"plug pict"), b"pict plug")
                .build()
                .bytes,
        );
    let data = open(&tree, &forks, true);

    let raw = |id| {
        let found = data.resource(PICT, id).expect("present");
        (found.resource.data(), found.source.path.as_path())
    };
    assert_eq!(raw(200), (&b"pict plug"[..], Path::new("/p/plug")));
    assert_eq!(raw(201), (&b"pict two"[..], Path::new("/d/two")));
    let found = data.resource(PICT, 200).expect("present");
    assert_eq!(
        (
            found.resource.res_type(),
            found.resource.id(),
            found.resource.name()
        ),
        (PICT, 200, Some("plug pict"))
    );
    assert_eq!(found.source.origin, Origin::PlugIn);
    assert_eq!(ship(&data, 128).2, Path::new("/d/one"));
    assert_eq!(
        data.resource(Ship::TYPE, 128)
            .expect("present")
            .source
            .origin,
        Origin::Data
    );

    let provenance = data.provenance(PICT, 201).expect("present");
    assert_eq!(provenance.winner.path, Path::new("/d/two"));
    assert!(provenance.shadowed.is_empty());

    // Unregistered types are reachable raw but have no typed decode.
    assert!(data.get_any(PICT, 200).is_none());
}

#[test]
fn unreadable_files_are_listed_as_failed_and_skipped() {
    let mut corrupt = ForkBuilder::new()
        .resource(Ship::TYPE, 129, None, &ship_bytes(9))
        .build();
    corrupt.put_u32(MAP_OFFSET_FIELD, 0x00FF_FFFF);
    let tree = FakeTree::new().dir(
        "/d",
        &[
            ("good", File),
            ("corrupt", File),
            ("empty", File),
            ("io", File),
        ],
    );
    let forks = FakeForks::new()
        .file("/d/good", ships("good", &[(128, 1)]))
        .file("/d/corrupt", corrupt.bytes)
        .file("/d/empty", Vec::new())
        .unreadable("/d/io");
    let data = open(&tree, &forks, false);

    assert_eq!(paths(data.files()), [Path::new("/d/good")]);
    assert_eq!(ship(&data, 128).0, 1);
    assert!(data.get::<Ship>(129).is_none());

    let failed: Vec<(&Path, Origin, &LoadError)> = data
        .failed()
        .iter()
        .map(|f| (f.path.as_path(), f.origin, &f.error))
        .collect();
    let [corrupt, empty, io] = failed.as_slice() else {
        panic!("three failures: {failed:?}")
    };
    assert_eq!(
        (corrupt.0, corrupt.1),
        (Path::new("/d/corrupt"), Origin::Data)
    );
    assert!(
        matches!(corrupt.2, LoadError::Parse { .. }),
        "{:?}",
        corrupt.2
    );
    assert_eq!(empty.0, Path::new("/d/empty"));
    assert!(
        matches!(empty.2, LoadError::NoResourceFork { .. }),
        "{:?}",
        empty.2
    );
    assert_eq!(io.0, Path::new("/d/io"));
    assert!(matches!(io.2, LoadError::Io { .. }), "{:?}", io.2);
}

#[test]
fn walk_results_are_kept() {
    let tree = FakeTree::new()
        .dir("/d", &[("Music.mp3", File), ("a", File)])
        .dir("/p", &[("gone", Dir)]);
    let forks = FakeForks::new().file("/d/a", ships("a", &[(128, 1)]));
    let data = open(&tree, &forks, true);
    assert_eq!(
        data.ignored(),
        [IgnoredEntry {
            path: PathBuf::from("/d/Music.mp3"),
            reason: IgnoreReason::NotGameData("mp3".to_owned()),
        }]
    );
    let [failed] = data.failed() else {
        panic!("one failure: {:?}", data.failed())
    };
    assert_eq!(failed.path, Path::new("/p/gone"));
}

#[test]
fn opening_fails_only_when_a_root_cannot_be_listed() {
    let forks = FakeForks::new();
    let err = GameData::load(&FakeTree::new(), &forks, Path::new("/d"), None).expect_err("fails");
    assert!(matches!(err, OpenError::DataDir { .. }), "{err:?}");
}

#[test]
fn ids_ascend_and_types_cover_everything_present() {
    let tree = FakeTree::new()
        .dir("/d", &[("one", File)])
        .dir("/p", &[("plug", File)]);
    let forks = FakeForks::new()
        .file(
            "/d/one",
            ForkBuilder::new()
                .resource(Ship::TYPE, 130, None, &ship_bytes(1))
                .resource(PICT, 7, None, b"")
                .resource(Ship::TYPE, -5, None, &ship_bytes(1))
                .resource(Ship::TYPE, 128, None, &ship_bytes(1))
                .build()
                .bytes,
        )
        .file("/p/plug", ships("plug", &[(129, 2), (128, 2)]));
    let data = open(&tree, &forks, true);

    assert_eq!(data.ids(Ship::TYPE), [-5, 128, 129, 130]);
    assert_eq!(data.ids(PICT), [7]);
    let mut types: Vec<ResType> = data.types().collect();
    types.sort();
    let mut expected = vec![Ship::TYPE, PICT];
    expected.sort();
    assert_eq!(types, expected);

    let absent = ResType::new(*b"none");
    assert!(data.ids(absent).is_empty());
    assert!(data.resource(Ship::TYPE, 131).is_none());
    assert!(data.resource(absent, 128).is_none());
    assert!(data.provenance(Ship::TYPE, 131).is_none());
    assert!(data.provenance(absent, 128).is_none());
    assert!(data.get::<Ship>(131).is_none());
    assert!(data.get_any(Ship::TYPE, 131).is_none());
    assert!(data.get_any(absent, 128).is_none());
}

/// A data directory holding one file with ships 128 (good), 129 (one byte
/// short) and 130 (two trailing bytes).
fn mixed_ships() -> GameData {
    let mut long = ship_bytes(3);
    long.extend([0, 0]);
    let tree = FakeTree::new().dir("/d", &[("one", File)]);
    let forks = FakeForks::new().file(
        "/d/one",
        ForkBuilder::new()
            .resource(Ship::TYPE, 130, None, &long)
            .resource(Ship::TYPE, 129, Some(b"short"), &ship_bytes(2)[1..])
            .resource(Ship::TYPE, 128, Some(b"good"), &ship_bytes(1))
            .build()
            .bytes,
    );
    open(&tree, &forks, false)
}

fn decoded(data: &GameData, id: i16) -> bool {
    data.slot(Ship::TYPE, id)
        .expect("present")
        .decoded
        .get()
        .is_some()
}

#[test]
fn records_decode_only_when_first_asked_for() {
    let data = mixed_ships();
    assert!(!decoded(&data, 128));
    assert!(data.resource(Ship::TYPE, 128).is_some());
    assert!(!decoded(&data, 128), "raw access does not decode");
    let _ = data.get::<Ship>(128);
    assert!(decoded(&data, 128));
    assert!(!decoded(&data, 129));
    assert!(!decoded(&data, 130));
}

#[test]
fn a_record_is_decoded_once_and_shared() {
    let data = mixed_ships();
    let first = data.get::<Ship>(128).expect("present").expect("decodes");
    let second = data.get::<Ship>(128).expect("present").expect("decodes");
    assert!(std::ptr::eq(first.record, second.record));
    assert_eq!(
        (first.id, first.name, first.record.holds),
        (128, Some("good"), 1)
    );
    assert!(first.warning.is_none());

    let any = data
        .get_any(Ship::TYPE, 128)
        .expect("present")
        .expect("decodes");
    let AnyRecord::Ship(ship) = any.record else {
        panic!("a ship: {:?}", any.record)
    };
    let ship: &Ship = ship;
    assert!(std::ptr::eq(ship, first.record));
    assert_eq!((any.id, any.name), (128, Some("good")));
    assert_eq!(any.source.path, Path::new("/d/one"));
}

#[test]
fn a_decode_error_is_returned_every_time_without_harming_the_store() {
    let data = mixed_ships();
    let first = data.get::<Ship>(129).expect("present").expect_err("short");
    let second = data.get::<Ship>(129).expect("present").expect_err("short");
    assert!(std::ptr::eq(first, second));
    assert_eq!((first.res_type, first.id), (Ship::TYPE, 129));
    assert_eq!(first.name.as_deref(), Some("short"));
    let any = data
        .get_any(Ship::TYPE, 129)
        .expect("present")
        .expect_err("short");
    assert!(std::ptr::eq(any, first));
    assert_eq!(ship(&data, 128).0, 1);
}

#[test]
fn trailing_bytes_decode_with_a_warning() {
    let data = mixed_ships();
    let entry = data.get::<Ship>(130).expect("present").expect("decodes");
    assert_eq!(entry.record.holds, 3);
    let warning = entry.warning.expect("trailing bytes");
    assert!(
        matches!(warning, DecodeWarning::TrailingBytes { id: 130, actual_len, .. } if *actual_len == 1862),
        "{warning:?}"
    );
}

#[test]
fn records_iterate_every_id_ascending_and_keep_going_past_errors() {
    let data = mixed_ships();
    let all: Vec<(i16, Result<i16, i16>)> = data
        .records::<Ship>()
        .map(|(id, result)| (id, result.map(|e| e.record.holds).map_err(|e| e.id)))
        .collect();
    assert_eq!(all, [(128, Ok(1)), (129, Err(129)), (130, Ok(3))]);
    assert_eq!(data.records::<crate::records::spin::Spin>().count(), 0);
}

#[test]
fn each_test_expression_is_parsed_once_and_shared() {
    let data = mixed_ships();
    let first = data.test_expr("b1 & b2");
    let again = data.test_expr("b1 & b2");
    assert!(std::sync::Arc::ptr_eq(&first, &again), "parsed once");
    assert_eq!(*first, crate::expr::TestExpr::parse("b1 & b2"));
    let other = data.test_expr("b1 | b2");
    assert!(!std::sync::Arc::ptr_eq(&first, &other), "another text");
    assert_eq!(*other, crate::expr::TestExpr::parse("b1 | b2"));
}

#[test]
fn a_malformed_test_expression_keeps_its_error_and_a_blank_one_always_holds() {
    let data = mixed_ships();
    let bad = data.test_expr("b1 &");
    assert_eq!(*bad, crate::expr::TestExpr::parse("b1 &"));
    assert!(bad.is_err());
    assert!(std::sync::Arc::ptr_eq(&bad, &data.test_expr("b1 &")));
    assert_eq!(*data.test_expr(""), Ok(crate::expr::TestExpr::Always));
}

#[test]
fn each_set_expression_is_parsed_once_and_shared() {
    let data = mixed_ships();
    let first = data.set_expr("b1 !b2");
    let again = data.set_expr("b1 !b2");
    assert!(std::sync::Arc::ptr_eq(&first, &again), "parsed once");
    assert_eq!(*first, crate::expr::SetExpr::parse("b1 !b2"));
    let other = data.set_expr("^b3");
    assert!(!std::sync::Arc::ptr_eq(&first, &other), "another text");
    assert_eq!(*other, crate::expr::SetExpr::parse("^b3"));
}

#[test]
fn a_malformed_set_expression_keeps_its_error_and_a_blank_one_has_no_ops() {
    let data = mixed_ships();
    let bad = data.set_expr("b1&");
    assert_eq!(*bad, crate::expr::SetExpr::parse("b1&"));
    assert!(bad.is_err());
    assert!(std::sync::Arc::ptr_eq(&bad, &data.set_expr("b1&")));
    assert_eq!(
        data.set_expr("").as_ref().as_ref().map(|set| set.ops.len()),
        Ok(0)
    );
}

#[test]
fn a_test_and_a_set_with_one_text_are_kept_apart() {
    let data = mixed_ships();
    assert!(data.test_expr("b1").is_ok());
    assert_eq!(*data.set_expr("b1"), crate::expr::SetExpr::parse("b1"));
}

/// The store can be shared across threads.
const _: fn() = || {
    fn is<T: Send + Sync>() {}
    is::<GameData>();
};
