//! Charcoal against the stock data.
//!
//! The game data is copyrighted and never committed, so these tests find it
//! through `common`, the only place `NOVA_DATA` and `NOVA_DATA_REZ` are
//! read, and skip, passing, when the variable is unset. The Mac OS X release
//! ships Charcoal as a loose Apple TrueType file in `Fonts/` beside
//! `Nova Files`; the Windows Community Edition ships none.

use crate::common;

use nova_data::fonts::{DeviceWidths, FontError, charcoal_path, open_charcoal};
use ttf_parser::Face;

use common::{nova_data, nova_data_rez};

#[test]
fn the_mac_data_ships_charcoal_as_an_apple_truetype_file() {
    let Some(dir) = nova_data() else {
        return;
    };
    if !charcoal_path(&dir).is_file() {
        eprintln!("skipping: no {}", charcoal_path(&dir).display());
        return;
    }
    let bytes = open_charcoal(&dir).expect("Charcoal loads");
    assert_eq!(bytes.len(), 86_136);
    assert_eq!(&bytes[..4], b"true");
}

#[test]
fn the_windows_data_ships_no_charcoal() {
    let Some(dir) = nova_data_rez() else {
        return;
    };
    let err = open_charcoal(&dir).expect_err("no Charcoal");
    assert!(matches!(err, FontError::Missing { .. }), "{err:?}");
}

#[test]
fn the_games_charcoal_records_its_device_widths_from_9_to_24_ppem() {
    let Some(dir) = nova_data() else {
        return;
    };
    if !charcoal_path(&dir).is_file() {
        eprintln!("skipping: no {}", charcoal_path(&dir).display());
        return;
    }
    let bytes = open_charcoal(&dir).expect("Charcoal loads");
    let widths = DeviceWidths::read(&bytes);
    let mut ppems: Vec<u8> = widths.ppems().collect();
    ppems.sort_unstable();
    assert_eq!(ppems, (9..=24).collect::<Vec<u8>>());
    let face = Face::parse(&bytes, 0).expect("parses");
    let width = |c: char, ppem: u8| {
        let glyph = face.glyph_index(c).expect("in the cmap");
        widths.width(ppem, glyph.0)
    };
    // Each differs from the linear advance rounded.
    for (c, ppem, want) in [
        ('R', 9, 6),
        ('X', 9, 6),
        ('a', 10, 5),
        ('m', 10, 9),
        ('v', 10, 5),
        ('L', 10, 6),
        ('a', 12, 8),
        ('s', 12, 7),
        ('m', 12, 12),
        ('w', 12, 11),
        ('I', 12, 5),
        ('F', 12, 6),
        ('W', 12, 12),
    ] {
        assert_eq!(width(c, ppem), Some(want), "{c:?} at {ppem}");
    }
}
