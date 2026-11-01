//! The run-length tokens a sprite is drawn and touched by, over the game
//! data: [`PilotCatalog::rle_sheet`](crate::catalog::PilotCatalog::rle_sheet)'s
//! adapter.
//!
//! A ship's base sheet is its `shän`'s `rlëD` (`GameData::ship_sprite_words`;
//! a `PICT` base image isn't read, so its touch falls back to discs). A
//! stellar graphic's is `spïn` 1000 + its type's, an `rlëD` or a cut
//! `PICT` pair (`GameData::spin_words`). The shift is the sheet's depth
//! shifted right by 4, as `_SWCreateFrame` sets +0x5e (@0xad5f3-0xad611).
//! Each call reads the sheet afresh: the session asks once per sheet and
//! keeps it.

use nova_data::graphics::RleWords;
use nova_data::{GameData, stellar_spin_id};
use nova_sim_base::sprite_mask::RleSheet;

use crate::catalog::MaskSheet;

/// `sheet`'s tokens in `data`, or none when it cannot be read.
pub(super) fn rle_sheet(data: &GameData, sheet: MaskSheet) -> Option<RleSheet> {
    let words = match sheet {
        MaskSheet::Ship(ship) => data.ship_sprite_words(ship).ok()?.sheet,
        MaskSheet::Stellar(graphic) => data.spin_words(stellar_spin_id(graphic)?).ok()?,
    };
    Some(of_words(words))
}

/// `words` as the collision reads them.
fn of_words(words: RleWords) -> RleSheet {
    // 16 >> 4 and 8 >> 4 fit a byte.
    #[allow(clippy::cast_possible_truncation)]
    let shift = (words.depth >> 4) as u8;
    RleSheet {
        width: words.frame_width,
        height: words.frame_height,
        shift,
        words: words.words,
        starts: words.starts,
    }
}

#[cfg(test)]
mod tests {
    use nova_data::Record;
    use nova_data::graphics::fixture::{RledBuilder, rgb888_pict};
    use nova_data::graphics::{PICT, RLED};
    use nova_data::records::fixture::ShipAnimBuilder;
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::spin::Spin;

    use super::*;
    use crate::catalog::{PilotCatalog, ShipId};
    use crate::fixture::rle_sheets::check_contract;
    use crate::test_store::{put_i16s, store};

    const SHIP: MaskSheet = MaskSheet::Ship(ShipId(128));
    const RLED_TYPE: MaskSheet = MaskSheet::Stellar(3);
    const PICT_TYPE: MaskSheet = MaskSheet::Stellar(4);

    /// A `spïn` naming sprites `sprites` and masks `masks`, frames
    /// `width` x `height`, `x_tiles` across.
    fn spin(sprites: i16, masks: i16, width: i16, height: i16, x_tiles: i16) -> Vec<u8> {
        let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
        put_i16s(
            &mut bytes,
            0x00,
            &[sprites, masks, width, height, x_tiles, 1],
        );
        bytes
    }

    /// Ship 128 (base image 1000, a 2x1 sheet of one frame), graphic type
    /// 3 (an 8-bit `rlëD` of two 3x1 frames) and graphic type 4 (a `PICT`
    /// pair cut into two 2x1 frames, `#.` and `.#`).
    fn data() -> GameData {
        let white = [0xFF; 3];
        let black = [0; 3];
        store(&[
            (Ship::TYPE, 128, vec![0; Ship::SIZE.expect("fixed")]),
            (
                ShipAnim::TYPE,
                128,
                ShipAnimBuilder::new().base_image_id(1000).build(),
            ),
            (
                RLED,
                1000,
                RledBuilder::new(2, 1)
                    .frame(|f| f.token(1, 8).pixels(&[0x7C00, 0x7C00]))
                    .build(),
            ),
            (Spin::TYPE, 1003, spin(2003, -1, 3, 1, 2)),
            (
                RLED,
                2003,
                RledBuilder::new(3, 1)
                    .depth(8)
                    .frame(|f| f.token(1, 8).pixels8(&[1, 2, 3]))
                    .frame(|f| f.token(1, 4).skip8(3))
                    .build(),
            ),
            (Spin::TYPE, 1004, spin(2004, 2104, 2, 1, 2)),
            (PICT, 2004, rgb888_pict(4, 1, &[[0x80; 3]; 4])),
            (PICT, 2104, rgb888_pict(4, 1, &[white, black, black, white])),
        ])
    }

    #[test]
    fn a_ships_base_sheet_is_its_rleds_tokens_at_a_shift_of_1() {
        let rle = data().rle_sheet(SHIP).expect("reads");
        assert_eq!((rle.width, rle.height, rle.shift), (2, 1, 1));
        assert_eq!(rle.words, [0x0100_0008, 0x0200_0004, 0x7C00_7C00, 0]);
        assert_eq!(rle.starts, [Some(0)]);
    }

    #[test]
    fn a_stellar_graphics_sheet_is_its_spins_rled_or_cut_pict_pair() {
        let data = data();
        let eight = data.rle_sheet(RLED_TYPE).expect("an rlëD");
        assert_eq!((eight.width, eight.height, eight.shift), (3, 1, 0));
        assert_eq!(eight.starts, [Some(0), Some(4)]);
        assert_eq!(eight.words[4..], [0x0100_0004, 0x0300_0003, 0]);
        let cut = data.rle_sheet(PICT_TYPE).expect("a PICT pair");
        assert_eq!((cut.width, cut.height, cut.shift), (2, 1, 1));
        assert_eq!(
            cut.words,
            [
                0x0100_0008,
                0x0200_0002,
                0,
                0,
                0x0100_000C,
                0x0300_0002,
                0x0200_0002,
                0,
                0
            ]
        );
        assert_eq!(cut.starts, [Some(0), Some(4)]);
    }

    #[test]
    fn the_adapter_meets_the_contract_a_sheet_it_cannot_read_none() {
        let missing = [
            MaskSheet::Ship(ShipId(129)),
            MaskSheet::Stellar(5),
            MaskSheet::Stellar(-1),
            MaskSheet::Stellar(i16::MAX),
        ];
        assert_eq!(
            check_contract(&data(), &[SHIP, RLED_TYPE, PICT_TYPE], &missing),
            Ok(())
        );
    }
}
