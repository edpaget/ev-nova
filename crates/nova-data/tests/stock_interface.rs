//! The interface file's dialogs and pictures against the stock data.
//!
//! The game data is copyrighted and never committed, so these tests find
//! the interface file through `common`, the only place `NOVA_DATA` and
//! `NOVA_DATA_REZ` are read: `Nova-DF.rsrc` beside the Mac `Nova Files`,
//! and `Nova.rez` beside the Windows one. Each skips, passing, when its
//! variable is unset or the file is absent, and collects every failure
//! before asserting.
//!
//! Every `DLOG` and `DITL` must decode, using every byte. Some interface pictures use what the
//! `PICT` decoder does not support yet (version 1 pictures, and the
//! `PackBitsRgn` opcode `0x0099`), so a picture may fail only with
//! [`GraphicsError::UnsupportedVersion`] or
//! [`GraphicsError::UnsupportedOpcode`], and the number that decode is
//! pinned.

mod common;

use std::path::Path;

use nova_data::graphics::{GraphicsError, PICT};
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
    /// Pictures that decoded; the rest use unsupported `PICT` features.
    pictures_decoded: usize,
}

/// Decodes every `DLOG`, `DITL` and `PICT` in the interface file at `path`,
/// asserting no dialog or item list fails or has trailing bytes, and no picture fails except for
/// an unsupported `PICT` feature.
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
    let mut pictures_decoded = 0;
    for &id in &pictures {
        match ui.picture(PictId(id)) {
            Some(Ok(_)) => pictures_decoded += 1,
            Some(Err(
                GraphicsError::UnsupportedVersion { .. } | GraphicsError::UnsupportedOpcode { .. },
            )) => {}
            Some(Err(err)) => failures.push(format!("PICT {id}: {err}")),
            None => failures.push(format!("PICT {id}: listed but absent")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let counts = Counts {
        dialogs: dialogs.len(),
        item_lists: item_lists.len(),
        pictures: pictures.len(),
        pictures_decoded,
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
    decode_everything(&path);
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
            pictures_decoded: 5,
        }
    );
}
