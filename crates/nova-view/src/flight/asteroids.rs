//! Drawing the asteroid field, as `_HandleAsteroids` (@0x361ac in the `EV
//! Nova` executable) shows it.
//!
//! - An asteroid is a frame of its type's master sprite, `spïn` 800 +
//!   type (`_LoadSprites` @0x20bb7), placed at whole pixels from the
//!   player by its own sides
//!   ([`Camera::sprite_centre`](crate::system::camera::Camera::sprite_centre),
//!   @0x36459-0x364df). Its frame is the simulation's whole frame
//!   (@0x363f6), taken round the sheet's frames.
//! - An asteroid whose sheet was not read is the shots' crossed box, on
//!   the whole point of its position.
//! - Asteroids draw in their own layer, over the stellars and under the
//!   disabled ships and the cargo boxes (`_SetupSpriteWorld`
//!   @0x21d0c-0x21d74).
//! - An asteroid is murked at its position, seen from the player's ship
//!   ([`murk`](super::murk); `_HandleSpriteMurkiness` @0x36422-0x36446,
//!   rdm:phase/play-feel-4/phase-17-murk-other-sprites).
//! - An asteroid is a sprite of the sprite world, drawn through
//!   `_BlitPixieRLETranslucentDrawProc` (@0xbb3fc-0xbba11): where the
//!   world's edge cuts its box, an overhanging sheet is cut to the box and
//!   one with clip quirks draws the clipped blit's frame
//!   ([`BoxCut`], @0xbb56d-0xbb579, @0xbb770-0xbb789).

use super::catalog::Looks;
use super::weapons::SHOT_PLACEHOLDER_SIZE;
use crate::box_cut::BoxCut;
use crate::draw::crossed_box;
use crate::system::scene::PLACEHOLDER;
use crate::{Blend, Color, Colorize, DrawList, Point};

/// An asteroid to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsteroidShown {
    /// Where its centre is drawn, on screen.
    pub at: Point,
    /// Its frame.
    pub frame: u16,
    /// Its type, `spïn` 800 + `kind`.
    pub kind: u8,
    /// Its murk's colorize ([`SeenMurk::at`](super::murk::SeenMurk::at)).
    pub colorize: Colorize,
}

/// Draws each of `asteroids` as its type's frame, murked as it says; a
/// crossed box for one whose sheet was not read.
pub fn draw_asteroids(list: &mut DrawList, asteroids: &[AsteroidShown], looks: &Looks) {
    for shown in asteroids {
        match looks.asteroid_sheet(shown.kind) {
            Some(sheet) => {
                let frame = shown.frame % sheet.frames.get();
                let image = sheet.image.frame(frame);
                let cut = BoxCut::of(list.world_clip(), shown.at, sheet.edge());
                let look = (Color::WHITE, Blend::Normal, Some(shown.colorize));
                cut.mixed_sprite(list, image, shown.at, look);
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
    use crate::box_cut::{box_of, drawn_cuts};
    use crate::flight::catalog::EffectSheet;
    use crate::geometry::Bounds;

    /// A stock system's murk colour.
    const COLOUR: u16 = 0x1081;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn sheet(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            frame_width: 0,
            frame_height: 0,
            image_size: (0, 0),
            clip_quirks: false,
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
            colorize: Colorize::default(),
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
    fn a_murky_asteroid_is_mixed_toward_its_colour() {
        let murk = Colorize::toward_pixel(16, COLOUR);
        let murky = AsteroidShown {
            colorize: murk,
            ..shown(1.0, 0, 0)
        };
        let mut list = DrawList::new();
        draw_asteroids(&mut list, &[murky, shown(2.0, 0, 0)], &looks());
        let mut expected = DrawList::new();
        let image = ImageKey::sprite(1800, 0);
        expected.colorized_sprite(image, at(1.0, 5.0), Color::WHITE, Blend::Normal, murk);
        expected.sprite(image, at(2.0, 5.0), Color::WHITE);
        assert_eq!(list, expected, "no murk draws plainly");
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

    /// What a type-0 asteroid of a 10 x 8 sheet `rlëD` 1800, edged by
    /// `edge`, draws at `centre` in a 100-square world.
    fn cut_at(
        edge: impl Fn(EffectSheet) -> EffectSheet,
        centre: Point,
    ) -> Vec<(ImageKey, Option<Bounds>)> {
        let sheet = EffectSheet {
            frame_width: 10,
            frame_height: 8,
            image_size: (10, 8),
            ..sheet(1800, 36)
        };
        let looks = Looks {
            asteroids: BTreeMap::from([(0, Ok(edge(sheet)))]),
            ..Looks::default()
        };
        let asteroid = AsteroidShown {
            at: centre,
            ..shown(0.0, 3, 0)
        };
        let world = Bounds::at(at(0.0, 0.0), 100.0, 100.0);
        let mut list = DrawList::new();
        list.world_in(world, |list| draw_asteroids(list, &[asteroid], &looks));
        drawn_cuts(&list)
    }

    #[test]
    fn an_overhanging_asteroid_straddling_the_worlds_edge_is_cut_to_its_box() {
        let overhanging = |sheet| EffectSheet {
            image_size: (14, 12),
            ..sheet
        };
        let top = at(50.0, 1.0);
        assert_eq!(
            cut_at(overhanging, top),
            [(ImageKey::sprite(1800, 3), Some(box_of(top, (10.0, 8.0))))]
        );
        assert_eq!(
            cut_at(overhanging, at(50.0, 50.0)),
            [(ImageKey::sprite(1800, 3), None)],
            "wholly inside, drawn whole"
        );
    }

    #[test]
    fn a_quirky_asteroid_cut_at_the_top_draws_its_clipped_frame() {
        let quirky = |sheet| EffectSheet {
            clip_quirks: true,
            ..sheet
        };
        let top = at(50.0, 1.0);
        assert_eq!(
            cut_at(quirky, top),
            [(
                ImageKey::clipped_sprite(1800, 3, (3, 0)),
                Some(box_of(top, (10.0, 8.0)))
            )]
        );
    }
}
