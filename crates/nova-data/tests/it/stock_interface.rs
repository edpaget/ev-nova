//! The interface file's dialogs and pictures against the stock data.
//!
//! The game data is copyrighted and never committed, so these tests find
//! the interface file through `common`, the only place `NOVA_DATA` and
//! `NOVA_DATA_REZ` are read: `Nova-DF.rsrc` beside the Mac `Nova Files`,
//! and `Nova.rez` beside the Windows one. Each skips, passing, when its
//! variable is unset or the file is absent, and collects every failure
//! before asserting.
//!
//! Every `DLOG` and `DITL` must decode, using every byte, and every
//! `PICT` must decode: the version 1 pictures (133-137) and `PICT` 139's
//! `PackBitsRgn` included. The volume arrows, 134-137, are pinned to their
//! size and to the pressed look of 136 and 137.

use crate::common;

use std::path::Path;

use nova_data::graphics::{Image, PICT};
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::{DitlId, InterfaceData, PictId, Record};

use common::{interface_file, interface_rez, nova_data, nova_data_rez};

/// How many of each type decoded.
#[derive(Debug, PartialEq)]
struct Counts {
    dialogs: usize,
    item_lists: usize,
    pictures: usize,
}

/// Decodes every `DLOG`, `DITL` and `PICT` in the interface file at `path`,
/// asserting none fails, and no dialog or item list has trailing bytes.
fn decode_everything(path: &Path) -> Counts {
    let ui = InterfaceData::open(path).expect("the interface file opens");
    let mut failures = Vec::new();
    let dialogs = ui.ids(Dlog::TYPE);
    for &id in &dialogs {
        match ui.dialog(id) {
            Some(Ok((_, None))) => {}
            Some(Ok((_, Some(warning)))) => failures.push(warning.to_string()),
            Some(Err(err)) => failures.push(err.to_string()),
            None => failures.push(format!("dialog {id}: listed but absent")),
        }
    }
    let item_lists = ui.ids(Ditl::TYPE);
    for &id in &item_lists {
        match ui.items(DitlId(id)) {
            Some(Ok((_, None))) => {}
            Some(Ok((_, Some(warning)))) => failures.push(warning.to_string()),
            Some(Err(err)) => failures.push(err.to_string()),
            None => failures.push(format!("items {id}: listed but absent")),
        }
    }
    let pictures = ui.ids(PICT);
    for &id in &pictures {
        match ui.picture(PictId(id)) {
            Some(Ok(_)) => {}
            Some(Err(err)) => failures.push(format!("PICT {id}: {err}")),
            None => failures.push(format!("PICT {id}: listed but absent")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let counts = Counts {
        dialogs: dialogs.len(),
        item_lists: item_lists.len(),
        pictures: pictures.len(),
    };
    assert!(counts.dialogs > 0, "{counts:?}");
    assert!(counts.item_lists > 0, "{counts:?}");
    counts
}

#[test]
fn every_mac_interface_resource_decodes() {
    let Some(dir) = nova_data() else { return };
    let Some(path) = interface_file(&dir) else {
        return;
    };
    let counts = decode_everything(&path);
    assert_eq!(counts.pictures, 10, "{counts:?}");
    check_volume_arrows(&path);
}

#[test]
fn every_windows_interface_resource_decodes() {
    let Some(dir) = nova_data_rez() else { return };
    let Some(path) = interface_rez(&dir) else {
        return;
    };
    let counts = decode_everything(&path);
    assert_eq!(
        counts,
        Counts {
            dialogs: 41,
            item_lists: 40,
            pictures: 11,
        }
    );
    check_volume_arrows(&path);
}

/// The interface `PICT` `id`, decoded.
fn picture(ui: &InterfaceData, id: i16) -> Image {
    ui.picture(PictId(id))
        .expect("the picture is present")
        .expect("the picture decodes")
}

/// `PICT`s 134-137 are the 11x9 volume arrows. 136 and 137 are 134 and
/// 135 pressed: the box filled, the arrow cut out of it, so the pixel at
/// (5, 2), on the up arrow's tip, is black at rest and white pressed.
fn check_volume_arrows(path: &Path) {
    const BLACK: [u8; 4] = [0, 0, 0, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    let ui = InterfaceData::open(path).expect("the interface file opens");
    for id in 134..=137 {
        let arrow = picture(&ui, id);
        assert_eq!((arrow.width(), arrow.height()), (11, 9), "PICT {id}");
    }
    assert_eq!(picture(&ui, 134).pixel(5, 2), Some(BLACK));
    assert_eq!(picture(&ui, 136).pixel(5, 2), Some(WHITE));
}
