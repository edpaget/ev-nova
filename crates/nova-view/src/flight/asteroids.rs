//! Drawing the asteroid field, as `_HandleAsteroids` (@0x361ac in the `EV
//! Nova` executable) shows it.
//!
//! - An asteroid is a frame of its type's master sprite, `spïn` 800 +
//!   type (`_LoadSprites` @0x20bb7), centred on where it is (@0x364df).
//!   Its frame is the simulation's whole frame (@0x363f6), taken round
//!   the sheet's frames.
//! - An asteroid whose sheet was not read is the shots' crossed box.
//! - Asteroids draw in their own layer, over the stellars and under the
//!   disabled ships and the cargo boxes (`_SetupSpriteWorld`
//!   @0x21d0c-0x21d74).
//! - An asteroid is murked at its position, seen from the player's ship
//!   ([`murk`](super::murk); `_HandleSpriteMurkiness` @0x36422-0x36446,
//!   rdm:phase/play-feel-4/phase-17-murk-other-sprites).

use super::catalog::Looks;
use super::murk::murk_tint;
use super::weapons::SHOT_PLACEHOLDER_SIZE;
use crate::draw::crossed_box;
use crate::system::scene::PLACEHOLDER;
use crate::{Color, DrawList, Point};

/// An asteroid to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsteroidShown {
    /// Where its centre is drawn, on screen.
    pub at: Point,
    /// Its frame.
    pub frame: u16,
    /// Its type, `spïn` 800 + `kind`.
    pub kind: u8,
    /// How murky it is, out of 32 ([`sprite_murk`](super::murk::sprite_murk)).
    pub murk: u8,
}

/// Draws each of `asteroids` as its type's frame, murked as it says; a
/// crossed box for one whose sheet was not read.
pub fn draw_asteroids(list: &mut DrawList, asteroids: &[AsteroidShown], looks: &Looks) {
    for shown in asteroids {
        match looks.asteroid_sheet(shown.kind) {
            Some(sheet) => {
                let frame = shown.frame % sheet.frames.get();
                let tint = murk_tint(Color::WHITE, shown.murk);
                list.sprite(sheet.image.frame(frame), shown.at, tint);
            }
            None => crossed_box(list, shown.at, SHOT_PLACEHOLDER_SIZE, PLACEHOLDER),
        }
    }
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

    fn sheet(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            frame_width: 0,
            frame_height: 0,
            image: crate::SheetImage::Rled(image_id),
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    /// Type 0's sheet `rlëD` 1800 of 36 frames, type 4's `rlëD` 1804 of
    /// 12, and type 2's unread.
    fn looks() -> Looks {
        Looks {
            asteroids: BTreeMap::from([
                (0, Ok(sheet(1800, 36))),
                (2, Err("no spïn 802".to_owned())),
                (4, Ok(sheet(1804, 12))),
            ]),
            ..Looks::default()
        }
    }

    fn shown(x: f32, frame: u16, kind: u8) -> AsteroidShown {
        AsteroidShown {
            at: at(x, 5.0),
            frame,
            kind,
            murk: 0,
        }
    }

    #[test]
    fn an_asteroid_is_its_types_frame_centred_where_it_is() {
        let mut list = DrawList::new();
        draw_asteroids(
            &mut list,
            &[shown(1.0, 0, 0), shown(2.0, 35, 0), shown(3.0, 7, 4)],
            &looks(),
        );
        let mut expected = DrawList::new();
        expected.sprite(ImageKey::sprite(1800, 0), at(1.0, 5.0), Color::WHITE);
        expected.sprite(ImageKey::sprite(1800, 35), at(2.0, 5.0), Color::WHITE);
        expected.sprite(ImageKey::sprite(1804, 7), at(3.0, 5.0), Color::WHITE);
        assert_eq!(list, expected);
    }

    #[test]
    fn a_frame_past_the_sheets_wraps_round_it() {
        let mut list = DrawList::new();
        draw_asteroids(&mut list, &[shown(1.0, 14, 4)], &looks());
        let mut expected = DrawList::new();
        expected.sprite(ImageKey::sprite(1804, 2), at(1.0, 5.0), Color::WHITE);
        assert_eq!(list, expected);
    }

    #[test]
    fn a_murky_asteroid_is_darkened_by_its_murk() {
        let murky = AsteroidShown {
            murk: 16,
            ..shown(1.0, 0, 0)
        };
        let mut list = DrawList::new();
        draw_asteroids(&mut list, &[murky, shown(2.0, 0, 0)], &looks());
        let mut expected = DrawList::new();
        let darkened = murk_tint(Color::WHITE, 16);
        assert_eq!(darkened, Color::rgba(127, 127, 127, 255));
        expected.sprite(ImageKey::sprite(1800, 0), at(1.0, 5.0), darkened);
        expected.sprite(ImageKey::sprite(1800, 0), at(2.0, 5.0), Color::WHITE);
        assert_eq!(list, expected, "murk 0 is white");
    }

    #[test]
    fn an_asteroid_whose_sheet_was_not_read_is_a_crossed_box() {
        for kind in [2, 9] {
            let mut list = DrawList::new();
            draw_asteroids(&mut list, &[shown(4.0, 3, kind)], &looks());
            let mut expected = DrawList::new();
            crossed_box(
                &mut expected,
                at(4.0, 5.0),
                SHOT_PLACEHOLDER_SIZE,
                PLACEHOLDER,
            );
            assert_eq!(list, expected, "{kind}");
        }
    }
}
