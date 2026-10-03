//! Wiring: Charcoal opened from a real game folder in a temporary
//! directory, laid out as the Mac OS X release lays it out.

use std::fs;

use nova_data::fonts::fixture::block_font;
use nova_data::fonts::{FontError, open_charcoal};

#[test]
fn opens_charcoal_from_the_fonts_folder_beside_nova_files() {
    let game = tempfile::tempdir().expect("temp dir");
    let data = game.path().join("Nova Files");
    fs::create_dir(&data).expect("creates Nova Files");
    fs::create_dir(game.path().join("Fonts")).expect("creates Fonts");
    let font = block_font("Charcoal");
    fs::write(game.path().join("Fonts/Charcoal.ttf"), &font).expect("writes");

    assert_eq!(open_charcoal(&data).expect("opens"), font);
}

#[test]
fn a_game_folder_with_no_fonts_has_no_charcoal() {
    let game = tempfile::tempdir().expect("temp dir");
    let data = game.path().join("Nova Files");
    fs::create_dir(&data).expect("creates Nova Files");

    let err = open_charcoal(&data).expect_err("no Charcoal");
    assert!(matches!(err, FontError::Missing { .. }), "{err:?}");
}
