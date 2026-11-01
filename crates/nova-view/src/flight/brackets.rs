//! The corner brackets round the nav target and the ship target, as the
//! original places them (`_HandleNavTargetSprites` @0x2fcaf-0x2ff19 and
//! `_HandleWeapTargetSprites` @0x2ff1a-0x301b8 in the `EV Nova`
//! executable). When they show, how far out they are and which colour a
//! ship's take are the session's ([`nova_sim::brackets`]).
//!
//! - **Art.** Four 16 x 16 `cïcn`s, one per corner, from Nova Graphics 1,
//!   so plug-ins can replace them: the nav brackets 10000-10003, or
//!   10004-10007 for a dominated stellar (`cïcn` 10000 with 8 frames,
//!   @0x21641-0x21662; frame base @0x2fd40-0x2fd76); the ship brackets
//!   from `cïcn` 10008 with 16 frames (@0x21715-0x2173e), four per
//!   [`Standing`] ([`ship_cicn`]).
//! - **Size.** The half-size is the sprite's larger side halved in whole
//!   pixels (@0x2fe00-0x2fe34, @0x3008e-0x300f2), or 64 for a stellar
//!   with no sprite (@0x2fe41), plus the expand truncated (`cvttss2si`).
//! - **Corners** ([`corners`], @0x2fe46-0x2fede, @0x300e1-0x3017d), with
//!   `h` the half-size: top-left at (cx−h−16, cy−h−16), top-right at
//!   (cx+h, cy−h−16), bottom-right at (cx+h, cy+h), bottom-left at
//!   (cx−h−16, cy+h), so the art sits just outside the 2h square.
//! - **Layer.** They sit on the interface layer (`0x3b5318`), the nav
//!   brackets added before the ship brackets (@0x2009b-0x2017b,
//!   @0x2019c-), both before the Red Alert icon (@0x20304): over every
//!   ship, the nav brackets under the ship brackets, both under the icon.
//!
//! **Centre.** Both are on whole pixels, placed by the caller from the
//! camera: the nav brackets at the stellar's position less the player
//! truncated alone
//! ([`Camera::nav_centre`](crate::system::camera::Camera::nav_centre),
//! @0x2fdab-0x2fddb), which can be a pixel from the stellar's own offset;
//! the ship brackets at the difference truncated, with no size in it
//! ([`Camera::whole_point`](crate::system::camera::Camera::whole_point),
//! @0x30060-0x30085), so an odd-sized ship's are half a pixel up and left
//! of its sprite's middle.

pub use nova_sim::brackets::Standing;

use crate::{DrawList, ImageKey, Point};

/// Each corner's size, and how far outside the square its art sits.
pub const CORNER: f32 = 16.0;
/// The half-size round a stellar with no sprite (@0x2fe41).
pub const NO_SPRITE_HALF: f32 = 64.0;
/// The nav brackets' first `cïcn`.
pub const NAV_CICN: i16 = 10000;
/// A dominated stellar's nav brackets' first `cïcn`.
pub const DOMINATED_CICN: i16 = 10004;
/// The ship brackets' first `cïcn`, the threat's.
pub const SHIP_CICN: i16 = 10008;

/// The half-size of the brackets round a sprite `sprite` (width, height)
/// across, or none, `expand` out.
#[must_use]
pub fn half_size(sprite: Option<(u32, u32)>, expand: f32) -> f32 {
    let half = sprite.map_or(NO_SPRITE_HALF, |(width, height)| {
        (width.max(height) / 2) as f32
    });
    half + expand.trunc()
}

/// Where each corner's art goes, centred on `at` with half-size `half`:
/// top-left, top-right, bottom-right, bottom-left.
#[must_use]
pub fn corners(at: Point, half: f32) -> [Point; 4] {
    let (near, far) = (-half - CORNER, half);
    [
        Point::new(at.x + near, at.y + near),
        Point::new(at.x + far, at.y + near),
        Point::new(at.x + far, at.y + far),
        Point::new(at.x + near, at.y + far),
    ]
}

/// The first of the four `cïcn`s round a ship target that stands as
/// `standing`.
#[must_use]
pub fn ship_cicn(standing: Standing) -> i16 {
    SHIP_CICN
        + match standing {
            Standing::Hostile => 0,
            Standing::Neutral => 4,
            Standing::Escort => 8,
            Standing::Disabled => 12,
        }
}

/// Draws the four corners centred on `at` with half-size `half`, from
/// `cïcn` `first` on.
pub fn draw_corners(list: &mut DrawList, at: Point, half: f32, first: i16) {
    for (cicn, corner) in (first..).zip(corners(at, half)) {
        list.picture(ImageKey::icon(cicn), corner);
    }
}

/// Draws the nav brackets round a stellar centred on `at` on screen, its
/// sprite `sprite` across (none for a stellar without one), `expand`
/// out, green when `dominated`.
pub fn draw_nav(
    list: &mut DrawList,
    at: Point,
    sprite: Option<(u32, u32)>,
    expand: f32,
    dominated: bool,
) {
    let first = if dominated { DOMINATED_CICN } else { NAV_CICN };
    draw_corners(list, at, half_size(sprite, expand), first);
}

/// Draws the ship brackets round a target centred on `at` on screen, its
/// sprite `sprite` across, `expand` out, in its `standing`'s frames.
pub fn draw_ship(
    list: &mut DrawList,
    at: Point,
    sprite: Option<(u32, u32)>,
    expand: f32,
    standing: Standing,
) {
    draw_corners(list, at, half_size(sprite, expand), ship_cicn(standing));
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::DrawCommand;

    const AT: Point = Point::new(500.0, 400.0);

    fn picture(cicn: i16, x: f32, y: f32) -> DrawCommand {
        DrawCommand::Picture {
            image: ImageKey::icon(cicn),
            top_left: Point::new(x, y),
        }
    }

    fn ship(sprite: Option<(u32, u32)>, expand: f32, standing: Standing) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_ship(&mut list, AT, sprite, expand, standing);
        list.iter().cloned().collect()
    }

    fn nav(sprite: Option<(u32, u32)>, expand: f32, dominated: bool) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_nav(&mut list, AT, sprite, expand, dominated);
        list.iter().cloned().collect()
    }

    /// The `cïcn`s drawn.
    fn cicns(commands: &[DrawCommand]) -> Vec<i16> {
        commands
            .iter()
            .map(|command| match command {
                DrawCommand::Picture { image, .. } => image.id,
                other => panic!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn the_corners_sit_just_outside_the_sprite_square() {
        // 48 x 40: half 24.
        assert_eq!(
            ship(Some((48, 40)), 0.0, Standing::Hostile),
            [
                picture(10008, 460.0, 360.0),
                picture(10009, 524.0, 360.0),
                picture(10010, 524.0, 424.0),
                picture(10011, 460.0, 424.0),
            ]
        );
        // 196 out: half 220.
        assert_eq!(
            ship(Some((40, 48)), 196.0, Standing::Hostile),
            [
                picture(10008, 264.0, 164.0),
                picture(10009, 720.0, 164.0),
                picture(10010, 720.0, 620.0),
                picture(10011, 264.0, 620.0),
            ]
        );
        assert_eq!(CORNER, 16.0);
    }

    #[test]
    fn the_half_size_is_whole_pixels_and_the_expand_truncated() {
        assert_eq!(half_size(Some((41, 10)), 0.0), 20.0, "41 halves to 20");
        assert_eq!(half_size(Some((10, 41)), 0.0), 20.0);
        assert_eq!(half_size(Some((48, 40)), 76.9), 100.0, "76.9 → 76");
        assert_eq!(half_size(None, 16.0), 80.0);
        assert_eq!(corners(AT, 20.0)[0], Point::new(464.0, 364.0));
    }

    #[test]
    fn a_threat_draws_cicn_10008_to_10011() {
        let drawn = ship(Some((48, 40)), 0.0, Standing::Hostile);
        assert_eq!(cicns(&drawn), [10008, 10009, 10010, 10011]);
        assert_eq!(SHIP_CICN, 10008);
    }

    #[test]
    fn a_neutral_target_draws_the_yellow_10012_to_10015() {
        let drawn = ship(Some((48, 40)), 0.0, Standing::Neutral);
        assert_eq!(cicns(&drawn), [10012, 10013, 10014, 10015]);
    }

    #[test]
    fn an_escort_draws_the_green_10016_to_10019() {
        let drawn = ship(Some((48, 40)), 0.0, Standing::Escort);
        assert_eq!(cicns(&drawn), [10016, 10017, 10018, 10019]);
    }

    #[test]
    fn a_disabled_target_draws_cicn_10020_to_10023() {
        let drawn = ship(Some((48, 40)), 0.0, Standing::Disabled);
        assert_eq!(cicns(&drawn), [10020, 10021, 10022, 10023]);
    }

    #[test]
    fn the_nav_brackets_draw_10000_to_10003_or_10004_to_10007_when_dominated() {
        assert_eq!(
            nav(Some((48, 40)), 136.0, false),
            [
                picture(10000, 324.0, 224.0),
                picture(10001, 660.0, 224.0),
                picture(10002, 660.0, 560.0),
                picture(10003, 324.0, 560.0),
            ]
        );
        let dominated = nav(Some((48, 40)), 136.0, true);
        assert_eq!(cicns(&dominated), [10004, 10005, 10006, 10007]);
        assert_eq!((NAV_CICN, DOMINATED_CICN), (10000, 10004));
    }

    #[test]
    fn a_stellar_with_no_sprite_is_bracketed_at_half_64() {
        assert_eq!(
            nav(None, 0.0, false),
            [
                picture(10000, 420.0, 320.0),
                picture(10001, 564.0, 320.0),
                picture(10002, 564.0, 464.0),
                picture(10003, 420.0, 464.0),
            ]
        );
        assert_eq!(NO_SPRITE_HALF, 64.0);
    }
}
