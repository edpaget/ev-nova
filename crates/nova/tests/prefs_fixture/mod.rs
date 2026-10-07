//! A synthetic interface file holding the stock "new prefs dialog"
//! (`DLOG` and `DITL` 4003), item for item, for the tests that open the
//! Preferences dialog through the app.

use std::io;
use std::path::Path;

use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::{InterfaceData, Record};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};

/// The interface file's bytes, as its data fork, wherever it is read.
struct InterfaceFork(Vec<u8>);

impl ForkReader for InterfaceFork {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

fn be(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

/// One `DITL` item: (left, top, right, bottom), type byte and data.
fn ditl_item((l, t, r, b): (i16, i16, i16, i16), type_byte: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    bytes.extend(be(&[t, l, b, r]));
    bytes.push(type_byte);
    bytes.push(data.len() as u8);
    bytes.extend(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

const BUTTON: u8 = 4;
const CHECK_BOX: u8 = 5;
const STATIC_TEXT: u8 = 8;
const PICTURE: u8 = 64;
const USER: u8 = 0;
const DISABLED: u8 = 0x80;

/// Stock "new prefs dialog": `DLOG` 4003, 336 x 278 and centred, and its
/// twenty-two items.
pub fn interface() -> InterfaceData {
    let mut dlog = be(&[54, 37, 332, 373, 1]);
    dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    dlog.extend(be(&[4003]));
    dlog.extend([0, 0, 0xA8, 0x0A]);
    let check = |bounds, title: &str| ditl_item(bounds, CHECK_BOX, title.as_bytes());
    let items = [
        ditl_item((225, 245, 295, 265), BUTTON, b"OK"),
        check((171, 55, 342, 73), "Share Processor Time"),
        ditl_item((69, 213, 314, 230), USER | DISABLED, &[]),
        ditl_item(
            (171, 167, 277, 183),
            STATIC_TEXT | DISABLED,
            b"Sound Volume:",
        ),
        ditl_item((189, 186, 311, 202), STATIC_TEXT | DISABLED, b"Static Text"),
        ditl_item((172, 194, 183, 203), PICTURE, &be(&[135])),
        ditl_item((172, 185, 183, 194), PICTURE, &be(&[134])),
        check((171, 33, 270, 51), "Intro Music"),
        check((171, 99, 307, 117), "QuickTime Movies"),
        check((11, 121, 172, 139), "Smoke Trails"),
        check((171, 77, 302, 95), "Run in a window"),
        check((11, 33, 172, 51), "Ship Animations"),
        check((11, 55, 172, 73), "Engine Glows"),
        check((11, 77, 172, 95), "Running Lights"),
        check((11, 99, 172, 117), "Weapon Effects"),
        ditl_item((49, 245, 184, 265), BUTTON, b"Key Settings"),
        ditl_item((186, 416, 306, 436), USER, &[]),
        check((11, 143, 156, 161), "Parallax Starfield"),
        ditl_item((12, 5, 325, 28), USER | DISABLED, &[]),
        check((171, 121, 307, 139), "Ambient Sounds"),
        check((171, 143, 316, 161), "Hyperspace Effects"),
        check((11, 165, 156, 183), "Check For Updates"),
    ];
    let mut ditl = be(&[items.len() as i16 - 1]);
    ditl.extend(items.concat());
    let bytes = ForkBuilder::new()
        .resource(Dlog::TYPE, 4003, None, &dlog)
        .resource(Ditl::TYPE, 4003, None, &ditl)
        .build()
        .bytes;
    InterfaceData::load(&InterfaceFork(bytes), Path::new("/Nova-DF.rsrc")).expect("loads")
}
