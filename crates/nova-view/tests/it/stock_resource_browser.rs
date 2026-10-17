//! The developer tools' resource browser over the stock data: every
//! resource is listed, found by type and ID and by name, and shows its
//! record, its file and, for pictures and sprite sheets, its frames.
//! Skips, passing, when `NOVA_DATA` is unset. Every loop is bounded by the
//! store's own resource count.

mod common;

use std::collections::BTreeMap;

use nova_data::graphics::{PICT, RLED, decode_pict, decode_rled};
use nova_data::{GameData, TYPES};
use nova_view::devtools::{Preview, ResType, ResourceBrowser, fold, type_code};

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

fn every_resource(data: &GameData) -> Vec<(ResType, i16)> {
    data.types()
        .flat_map(|ty| data.ids(ty).iter().map(move |&id| (ty, id)))
        .collect()
}

#[test]
fn the_index_is_the_store() {
    let Some(data) = open() else {
        return;
    };
    let browser = ResourceBrowser::new(&data);
    let expected: Vec<(ResType, i16, Option<String>)> = every_resource(&data)
        .into_iter()
        .map(|(ty, id)| {
            let name = data
                .resource(ty, id)
                .and_then(|found| found.resource.name())
                .map(str::to_owned);
            (ty, id, name)
        })
        .collect();
    assert_eq!(browser.len(), expected.len());
    assert!(browser.len() > 8000, "{} resources", browser.len());
    let listed: Vec<(ResType, i16, Option<String>)> = browser
        .results()
        .map(|s| (s.ty, s.id, s.name.clone()))
        .collect();
    assert_eq!(listed, expected);
}

#[test]
fn every_resource_is_found_first_by_type_and_id() {
    let Some(data) = open() else {
        return;
    };
    let mut browser = ResourceBrowser::new(&data);
    let mut failures = Vec::new();
    for (ty, id) in every_resource(&data) {
        browser.set_query(&format!("{ty} {id}"));
        let first = browser.result(0).map(|s| (s.ty, s.id));
        if first != Some((ty, id)) {
            failures.push(format!("{ty} {id}: first result {first:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn every_type_is_found_however_its_code_is_spelled() {
    let Some(data) = open() else {
        return;
    };
    let mut browser = ResourceBrowser::new(&data);
    let types: Vec<ResType> = data.types().collect();
    assert!(types.len() >= 30, "{} types", types.len());
    for ty in types {
        let id = data.ids(ty)[0];
        let ascii = fold(&type_code(ty));
        for spelling in [ascii.clone(), ascii.to_uppercase()] {
            browser.set_query(&format!("{spelling} {id}"));
            assert_eq!(
                browser.result(0).map(|s| (s.ty, s.id)),
                Some((ty, id)),
                "{spelling} {id}"
            );
        }
    }
}

#[test]
fn every_name_finds_all_of_its_resources() {
    let Some(data) = open() else {
        return;
    };
    let mut by_name: BTreeMap<String, Vec<(ResType, i16)>> = BTreeMap::new();
    for (ty, id) in every_resource(&data) {
        if let Some(name) = data.resource(ty, id).and_then(|f| f.resource.name()) {
            by_name.entry(name.to_owned()).or_default().push((ty, id));
        }
    }
    assert!(by_name.len() > 3000, "{} names", by_name.len());
    let mut browser = ResourceBrowser::new(&data);
    let mut failures = Vec::new();
    for (name, resources) in &by_name {
        browser.set_query(name);
        let found: Vec<(ResType, i16)> = browser.results().map(|s| (s.ty, s.id)).collect();
        for resource in resources {
            if !found.contains(resource) {
                failures.push(format!("{name:?} misses {resource:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn every_selection_shows_its_record_and_file() {
    let Some(data) = open() else {
        return;
    };
    let mut browser = ResourceBrowser::new(&data);
    let mut records = 0;
    for (ty, id) in every_resource(&data) {
        assert!(browser.select(ty, id), "{ty} {id}");
        let selection = browser.selection().expect("selected");
        let winner = data.provenance(ty, id).expect("present").winner;
        let file = winner.path.display().to_string();
        assert!(
            selection.file_line().contains(&file),
            "{ty} {id}: {}",
            selection.file_line()
        );
        let text = selection.record_text();
        if TYPES.contains(&ty) {
            let entry = data.get_any(ty, id).expect("present").expect("decodes");
            let expected = serde_json::to_value(entry.record).expect("serializes")["record"].take();
            let shown: serde_json::Value = serde_json::from_str(&text)
                .unwrap_or_else(|err| panic!("{ty} {id}: {err}: {text}"));
            assert_eq!(shown, expected, "{ty} {id}");
            records += 1;
        } else {
            assert_eq!(text, format!("{} has no record layout", type_code(ty)));
        }
    }
    assert!(records > 7000, "{records} records");
}

#[test]
fn every_picture_and_sprite_sheet_previews() {
    let Some(data) = open() else {
        return;
    };
    let mut browser = ResourceBrowser::new(&data);
    let mut previewed = 0;
    for ty in [PICT, RLED] {
        for &id in data.ids(ty) {
            browser.select(ty, id);
            let selection = browser.selection().expect("selected");
            let bytes = data.resource(ty, id).expect("present").resource.data();
            let expected: Vec<(u32, u32)> = if ty == PICT {
                let image = decode_pict(bytes).expect("decodes");
                vec![(image.width(), image.height())]
            } else {
                let sheet = decode_rled(bytes, None).expect("decodes");
                vec![(sheet.frame_width(), sheet.frame_height()); sheet.frames().len()]
            };
            let Preview::Frames(frames) = selection.preview() else {
                panic!("{ty} {id}: {:?}", selection.preview());
            };
            let sizes: Vec<(u32, u32)> = frames.iter().map(|f| (f.width(), f.height())).collect();
            assert_eq!(sizes, expected, "{ty} {id}");
            previewed += 1;
        }
    }
    assert_eq!(previewed, 671 + 282);
}
