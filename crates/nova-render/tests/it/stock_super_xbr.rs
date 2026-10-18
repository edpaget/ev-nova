//! Super-xBR over the stock images: sprite sheets and a picture come back
//! twice their size, and where a frame is clear all round it stays clear,
//! so no halo spreads into empty space. Skips, passing, when `NOVA_DATA`
//! is unset.

use nova_data::GameData;
use nova_data::graphics::{Image, PICT, RLED};
use nova_render::{Enlarger, ImageSource, SuperXbr};
use nova_view::ImageKind;

use crate::common;

/// The largest frame side a sheet here may have, so a debug build enlarges
/// them quickly.
const SMALL: u32 = 96;

/// Whether every texel of `image` within two of (`x`, `y`), clamped to it,
/// is clear: the texels an enlarged texel at (2x or 2x + 1, 2y or 2y + 1)
/// draws on.
fn clear_round(image: &Image, x: u32, y: u32) -> bool {
    let near = |at: u32, size: u32| at.saturating_sub(2)..=(at + 2).min(size - 1);
    near(y, image.height())
        .all(|sy| near(x, image.width()).all(|sx| image.pixel(sx, sy).is_some_and(|t| t[3] == 0)))
}

/// Checks `enlarged` is `frame` at twice its size, clear wherever `frame`
/// is clear all round; the count of texels it checked clear.
fn check(frame: &Image, enlarged: &Image, name: &str) -> usize {
    assert_eq!(
        (enlarged.width(), enlarged.height()),
        (2 * frame.width(), 2 * frame.height()),
        "{name}"
    );
    let mut clear = 0;
    for y in 0..enlarged.height() {
        for x in 0..enlarged.width() {
            if clear_round(frame, x / 2, y / 2) {
                assert_eq!(enlarged.pixel(x, y), Some([0; 4]), "{name} at ({x}, {y})");
                clear += 1;
            }
        }
    }
    clear
}

#[test]
fn stock_sheets_and_a_picture_enlarge_to_twice_their_size_with_no_halo() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    // Small sheets from the start, the middle and the end of the IDs, so
    // ships, planets and explosions all come up in the stock data.
    let small: Vec<(i16, Vec<Image>)> = data
        .ids(RLED)
        .iter()
        .filter_map(|&id| {
            let frames = data.frames(ImageKind::Rled, id).ok()?;
            let fits = frames
                .iter()
                .all(|f| f.width() <= SMALL && f.height() <= SMALL);
            (fits && !frames.is_empty() && frames.len() <= 40).then_some((id, frames))
        })
        .collect();
    assert!(small.len() >= 3, "small stock sheets");
    let mut clear = 0;
    for (id, frames) in [0, small.len() / 2, small.len() - 1].map(|n| &small[n]) {
        let enlarged = SuperXbr.enlarge_frames(ImageKind::Rled, frames);
        assert_eq!(enlarged.len(), frames.len());
        for (frame, enlarged) in frames.iter().zip(&enlarged) {
            clear += check(frame, enlarged, &format!("rlëD {id}"));
        }
    }
    assert!(clear > 0, "some sprite has clear space round it");
    let (pict, picture) = data
        .ids(PICT)
        .iter()
        .find_map(|&id| Some((id, data.frames(ImageKind::Pict, id).ok()?)))
        .expect("a stock picture");
    let enlarged = SuperXbr.enlarge(ImageKind::Pict, &picture[0]);
    check(&picture[0], &enlarged, &format!("PICT {pict}"));
}
