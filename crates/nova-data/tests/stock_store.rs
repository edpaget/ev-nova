//! The game data store over the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so these tests get its
//! location from `common`, the only place `NOVA_DATA` (the Mac `Nova Files`
//! directory) and `NOVA_DATA_REZ` (the Windows one) are read, and skip,
//! passing, when a variable they need is unset. Expected contents come from
//! an independent later-wins union of the stock files, opened one by one in
//! sorted order, and the Windows store must equal the Mac one. Each test
//! collects every failure before asserting.

mod common;

use std::collections::BTreeMap;
use std::path::Path;

use nova_data::records::mission::Mission;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::order::IgnoreReason;
use nova_data::store::{GameData, Origin};
use nova_data::{AnyRecord, Record, Registered, ShipId, TYPES, decode_any};
use nova_rsrc::{ResType, ResourceFile};

use common::{ndat_files, nova_data, nova_data_rez, rez_files};

fn name(path: &Path) -> String {
    path.file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned()
}

fn open(dir: &Path) -> GameData {
    GameData::open(dir, None).expect("the stock data opens")
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .expect("file name")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn the_stock_folder_loads_every_data_file_and_ignores_the_media() {
    let Some(dir) = nova_data() else { return };
    assert_loads(&open(&dir), &ndat_files(&dir));
}

#[test]
fn the_windows_stock_folder_loads_every_rez_file_and_ignores_the_media() {
    let Some(dir) = nova_data_rez() else { return };
    assert_loads(&open(&dir), &rez_files(&dir));
}

/// The store loaded exactly the 21 `files`, with no failures, and ignored
/// only the music and the four race movies.
fn assert_loads(data: &GameData, files: &[std::path::PathBuf]) {
    let loaded: Vec<String> = data.files().iter().map(|f| name(&f.path)).collect();
    let expected: Vec<String> = files.iter().map(|p| name(p)).collect();
    assert_eq!(loaded, expected);
    assert_eq!(loaded.len(), 21);
    assert!(data.files().iter().all(|f| f.origin == Origin::Data));
    assert!(data.failed().is_empty(), "{:?}", data.failed());

    let ignored: Vec<(String, IgnoreReason)> = data
        .ignored()
        .iter()
        .map(|entry| (name(&entry.path), entry.reason.clone()))
        .collect();
    let media =
        |file: &str, ext: &str| (file.to_owned(), IgnoreReason::NotGameData(ext.to_owned()));
    assert_eq!(
        ignored,
        [
            media("Nova Music.mp3", "mp3"),
            media("Race 1.mov", "mov"),
            media("Race 2.mov", "mov"),
            media("Race 3.mov", "mov"),
            media("Race 4.mov", "mov"),
        ]
    );
}

/// Every registered resource, later files winning: (type, ID) to the
/// winning file's name and its decoded record.
fn independent_union(dir: &Path) -> BTreeMap<(ResType, i16), (String, AnyRecord)> {
    let mut union = BTreeMap::new();
    for path in ndat_files(dir) {
        let file = ResourceFile::open(&path).expect("stock file opens");
        for &ty in TYPES {
            for res in file.resources(ty) {
                let (entry, _) = decode_any(&res)
                    .expect("registered")
                    .expect("stock records decode");
                union.insert((ty, res.id()), (name(&path), entry.record));
            }
        }
    }
    union
}

#[test]
fn every_registered_record_matches_an_independent_later_wins_union() {
    let Some(dir) = nova_data() else { return };
    let data = open(&dir);
    let union = independent_union(&dir);

    let mut problems = Vec::new();
    for &ty in TYPES {
        let expected: Vec<i16> = union
            .range((ty, i16::MIN)..=(ty, i16::MAX))
            .map(|((_, id), _)| *id)
            .collect();
        if data.ids(ty) != expected.as_slice() {
            problems.push(format!("{ty}: ids differ"));
        }
        for &id in &expected {
            let (file, record) = &union[&(ty, id)];
            match data.get_any(ty, id) {
                Some(Ok(entry)) => {
                    if entry.record != record || name(&entry.source.path) != *file {
                        problems.push(format!("{ty} {id}: differs from {file}"));
                    }
                }
                Some(Err(err)) => problems.push(err.to_string()),
                None => problems.push(format!("{ty} {id}: missing")),
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    // The sum of the per-type stock counts: no stock file overrides
    // another's registered records.
    assert_eq!(union.len(), 7143);
}

#[test]
fn every_resource_of_every_type_is_reachable_raw_from_its_file() {
    let Some(dir) = nova_data() else { return };
    let data = open(&dir);
    let mut union: BTreeMap<(ResType, i16), String> = BTreeMap::new();
    for path in ndat_files(&dir) {
        let file = ResourceFile::open(&path).expect("stock file opens");
        for res in file.iter() {
            union.insert((res.res_type(), res.id()), name(&path));
        }
    }

    let types: Vec<ResType> = data.types().collect();
    let mut expected_types: Vec<ResType> = union.keys().map(|(ty, _)| *ty).collect();
    expected_types.dedup();
    assert_eq!(types, expected_types);
    let mut problems = Vec::new();
    for (&(ty, id), file) in &union {
        match data.resource(ty, id) {
            Some(found) if name(&found.source.path) == *file => {}
            Some(found) => problems.push(format!("{ty} {id}: from {:?}", found.source.path)),
            None => problems.push(format!("{ty} {id}: missing")),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    let total: usize = types.iter().map(|&ty| data.ids(ty).len()).sum();
    assert_eq!(total, union.len());
}

fn typed_count<T: Registered>(data: &GameData) -> usize {
    data.records::<T>()
        .map(|(id, result)| {
            let entry = result.unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(entry.id, id);
        })
        .count()
}

/// The stock survey's counts, and the shuttle's source file.
fn assert_survey_counts(data: &GameData, shuttle_file: &str) {
    assert_eq!(typed_count::<Ship>(data), 288);
    assert_eq!(typed_count::<System>(data), 545);
    assert_eq!(typed_count::<Mission>(data), 791);
    assert_eq!(typed_count::<Stellar>(data), 411);
    let shuttle = data.get::<Ship>(128).expect("present").expect("decodes");
    assert_eq!(name(&shuttle.source.path), shuttle_file);
}

#[test]
fn typed_iteration_counts_match_the_stock_survey() {
    let Some(dir) = nova_data() else { return };
    assert_survey_counts(&open(&dir), "Nova Data 1.ndat");
}

#[test]
fn windows_typed_iteration_counts_match_the_stock_survey() {
    let Some(dir) = nova_data_rez() else { return };
    assert_survey_counts(&open(&dir), "Nova Data 1.rez");
}

#[test]
fn the_windows_store_equals_the_mac_store() {
    let (Some(mac), Some(windows)) = (nova_data(), nova_data_rez()) else {
        return;
    };
    let (mac, windows) = (open(&mac), open(&windows));

    let mut problems = Vec::new();
    let mut records = 0;
    for &ty in TYPES {
        if windows.ids(ty) != mac.ids(ty) {
            problems.push(format!("{ty}: ids differ"));
            continue;
        }
        for &id in mac.ids(ty) {
            records += 1;
            match (windows.get_any(ty, id), mac.get_any(ty, id)) {
                (Some(Ok(w)), Some(Ok(m))) => {
                    if w.record != m.record || stem(&w.source.path) != stem(&m.source.path) {
                        problems.push(format!("{ty} {id}: differs"));
                    }
                }
                other => problems.push(format!("{ty} {id}: {other:?}")),
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(records, 7143);
}

#[test]
fn every_stock_ship_resolves_to_its_sprite_sheet() {
    let Some(dir) = nova_data() else { return };
    let data = open(&dir);

    let mut problems = Vec::new();
    let mut resolved = 0;
    for &id in data.ids(Ship::TYPE) {
        match data.ship_sprite(ShipId(id)) {
            Ok(sprite) => {
                let anim = data.get::<ShipAnim>(id).expect("present").expect("decodes");
                let frames =
                    i64::from(anim.record.frames_per) * i64::from(anim.record.base_set_count);
                if sprite.sheet.frames().len() as i64 != frames {
                    problems.push(format!(
                        "shïp {id}: {} frames, expected {frames}",
                        sprite.sheet.frames().len()
                    ));
                }
                resolved += 1;
            }
            Err(err) => problems.push(format!("shïp {id}: {err}")),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(resolved, 288);
}

/// The files a ship's `shïp`, `shän` and `rlëD` came from.
fn sprite_sources(data: &GameData, id: i16) -> [String; 3] {
    let sprite = data.ship_sprite(ShipId(id)).expect("resolves");
    [
        name(&sprite.ship.path),
        name(&sprite.anim.path),
        name(&sprite.sheet_source.path),
    ]
}

/// The stock files split ship classes (`Nova Data 1`), most `shän`s (`Nova
/// Ships 7`) and sprite sheets (`Nova Ships 1` to `4`). Ship 128 (the
/// shuttle) keeps its `shän` beside its sheet; ship 183 is the first whose
/// three resources are in three different files.
#[test]
fn ship_sprites_resolve_across_stock_files() {
    let Some(dir) = nova_data() else { return };
    let data = open(&dir);
    assert_eq!(
        sprite_sources(&data, 128),
        ["Nova Data 1.ndat", "Nova Ships 1.ndat", "Nova Ships 1.ndat"]
    );
    let [ship, anim, sheet] = sprite_sources(&data, 183);
    assert!(
        ship != anim && anim != sheet && ship != sheet,
        "{ship}, {anim}, {sheet}"
    );

    let spread = data
        .ids(Ship::TYPE)
        .iter()
        .filter(|&&id| {
            let [ship, anim, sheet] = sprite_sources(&data, id);
            ship != anim && anim != sheet && ship != sheet
        })
        .count();
    assert_eq!(spread, 223);
}
