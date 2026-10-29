//! Drawing the cargo boxes a jettison leaves and a destroyed asteroid
//! drops, as `_HandleBoxes` (@0x2df85-0x2e29e in the `EV Nova`
//! executable) shows them.
//!
//! - A box is a frame of its graphic's sheet, `spïn` 500 + graphic (the
//!   box masters `_InitSprites` loads; a jettisoned box is graphic 0, an
//!   asteroid's 1-4),
//!   centred on where it is (@0x2e1c2-0x2e250). Its frame is the
//!   simulation's, `trunc(frame)` (@0x2e1ae-0x2e1bd), taken round the
//!   sheet's frames.
//! - Over its last 32 ticks it fades: its blend level `n`, 0 to 31
//!   ([`CargoBox::fade`](nova_sim::cargo_box::CargoBox::fade)), draws it
//!   `n`/32 transparent, white at alpha `(32 − n)/32`.
//! - A box whose sheet was not read is the shots' crossed box.
//! - Boxes draw in `_boxLayer`, over the stellars and under the ships and
//!   the shots.
//!
//! Left out: `_HandleSpriteMurkiness` (@0x2e181): the view murks shots
//! alone so far ([`murk`](super::murk)), and
//! rdm:task/murk-other-sprites draws the boxes murky.

use super::catalog::Looks;
use super::weapons::SHOT_PLACEHOLDER_SIZE;
use crate::draw::crossed_box;
use crate::system::scene::PLACEHOLDER;
use crate::{Color, DrawList, Point};

/// The levels of a box's fade: a level of 32 would be fully transparent.
pub const FADE_LEVELS: u8 = 32;

/// A cargo box to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShown {
    /// Where its centre is drawn, on screen.
    pub at: Point,
    /// Its frame.
    pub frame: u16,
    /// How faded it is, 0 to 31.
    pub fade: u8,
    /// Its graphic, `spïn` 500 + graphic.
    pub graphic: u16,
}

/// Draws each of `boxes` as its graphic's frame, faded as it says; a
/// crossed box for one whose sheet was not read.
pub fn draw_boxes(list: &mut DrawList, boxes: &[BoxShown], looks: &Looks) {
    for shown in boxes {
        match looks.box_sheet(shown.graphic) {
            Some(sheet) => {
                let frame = shown.frame % sheet.frames.get();
                list.sprite(sheet.image.frame(frame), shown.at, fade_tint(shown.fade));
            }
            None => crossed_box(list, shown.at, SHOT_PLACEHOLDER_SIZE, PLACEHOLDER),
        }
    }
}

/// White at the alpha a box `fade` levels faded is drawn at.
#[must_use]
pub fn fade_tint(fade: u8) -> Color {
    let levels = u16::from(FADE_LEVELS);
    let shown = levels - u16::from(fade.min(FADE_LEVELS));
    let alpha = u8::try_from((255 * shown + levels / 2) / levels).unwrap_or(u8::MAX);
    Color::rgba(255, 255, 255, alpha)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::num::NonZeroU16;

    use super::*;
    use crate::ImageKey;
    use crate::flight::catalog::EffectSheet;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn looks(sheet: Result<EffectSheet, String>) -> Looks {
        Looks {
            boxes: BTreeMap::from([(0, sheet)]),
            ..Looks::default()
        }
    }

    fn sheet(frames: u16) -> EffectSheet {
        EffectSheet {
            frame_width: 0,
            frame_height: 0,
            image: crate::SheetImage::Rled(600),
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    fn shown(x: f32, frame: u16, fade: u8) -> BoxShown {
        BoxShown {
            at: at(x, 5.0),
            frame,
            fade,
            graphic: 0,
        }
    }

    #[test]
    fn a_box_is_its_frame_centred_where_it_is() {
        let mut list = DrawList::new();
        draw_boxes(
            &mut list,
            &[shown(1.0, 0, 0), shown(2.0, 35, 0)],
            &looks(Ok(sheet(36))),
        );
        let mut expected = DrawList::new();
        expected.sprite(ImageKey::sprite(600, 0), at(1.0, 5.0), Color::WHITE);
        expected.sprite(ImageKey::sprite(600, 35), at(2.0, 5.0), Color::WHITE);
        assert_eq!(list, expected);
    }

    #[test]
    fn an_asteroids_boxes_draw_graphics_1_to_4_from_spin_501_to_504() {
        let sheets = Looks {
            boxes: (0..=4)
                .map(|graphic: u16| {
                    let sheet = EffectSheet {
                        frame_width: 0,
                        frame_height: 0,
                        image: crate::SheetImage::Rled(500 + graphic.cast_signed()),
                        frames: NonZeroU16::new(36).expect("non-zero"),
                    };
                    (graphic, Ok(sheet))
                })
                .collect(),
            ..Looks::default()
        };
        let shown: Vec<BoxShown> = (1..=4)
            .map(|graphic| BoxShown {
                graphic,
                ..shown(f32::from(graphic), 7, 0)
            })
            .collect();
        let mut list = DrawList::new();
        draw_boxes(&mut list, &shown, &sheets);
        let mut expected = DrawList::new();
        for graphic in 1..=4_u16 {
            expected.sprite(
                ImageKey::sprite(500 + graphic.cast_signed(), 7),
                at(f32::from(graphic), 5.0),
                Color::WHITE,
            );
        }
        assert_eq!(list, expected);
    }

    #[test]
    fn a_frame_past_the_sheets_wraps_round_it() {
        let mut list = DrawList::new();
        draw_boxes(&mut list, &[shown(1.0, 14, 0)], &looks(Ok(sheet(12))));
        let mut expected = DrawList::new();
        expected.sprite(ImageKey::sprite(600, 2), at(1.0, 5.0), Color::WHITE);
        assert_eq!(list, expected);
    }

    #[test]
    fn a_fading_box_is_drawn_its_fade_in_32nds_transparent() {
        assert_eq!(fade_tint(0), Color::WHITE);
        assert_eq!(fade_tint(16), Color::rgba(255, 255, 255, 128));
        assert_eq!(fade_tint(31), Color::rgba(255, 255, 255, 8));
        let mut list = DrawList::new();
        draw_boxes(&mut list, &[shown(1.0, 3, 16)], &looks(Ok(sheet(36))));
        let mut expected = DrawList::new();
        expected.sprite(ImageKey::sprite(600, 3), at(1.0, 5.0), fade_tint(16));
        assert_eq!(list, expected);
    }

    #[test]
    fn a_box_whose_sheet_was_not_read_is_a_crossed_box() {
        for looks in [looks(Err("no spïn 500".to_owned())), Looks::default()] {
            let mut list = DrawList::new();
            draw_boxes(&mut list, &[shown(4.0, 3, 0)], &looks);
            let mut expected = DrawList::new();
            crossed_box(
                &mut expected,
                at(4.0, 5.0),
                SHOT_PLACEHOLDER_SIZE,
                PLACEHOLDER,
            );
            assert_eq!(list, expected);
        }
    }
}
