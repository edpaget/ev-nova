//! Charcoal against the stock data.
//!
//! The game data is copyrighted and never committed, so these tests find it
//! through `common`, the only place `NOVA_DATA` and `NOVA_DATA_REZ` are
//! read, and skip, passing, when the variable is unset. The Mac OS X release
//! ships Charcoal as a loose Apple TrueType file in `Fonts/` beside
//! `Nova Files`; the Windows Community Edition ships none.

mod common;

use nova_data::fonts::{FontError, charcoal_path, open_charcoal};

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
