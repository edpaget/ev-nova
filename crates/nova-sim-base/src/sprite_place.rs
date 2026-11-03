//! Where the original places each kind of sprite's rect, and its radius
//! test of two placed rects (`EV Nova.app`, i386). Every rect is in whole
//! screen pixels from the player: the sprite's offset from it, truncated
//! toward zero on each axis, less a share of its frame, then the frame's
//! size on (`_SWSetSpriteLocation` @0xab640, a frame's hot spot 0). Each
//! routine takes a different share ([`Placement`]):
//!
//! - **The standard rule** ([`Placement::Standard`]): half each side,
//!   halved toward zero, so an odd side's extra pixel falls right and
//!   below. `_HandleShipDisplay` lays a ship out this way
//!   (@0x2cd17-0x2d083), `_HandleStellarSprites` a stellar
//!   (@0x2eeab-0x2ef1a), `_HandleAsteroids` an asteroid and
//!   `_HandleExplods` an explosion (@0x2f6a1-0x2f73c).
//! - **The shot's** ([`Placement::Shot`]): half its frame's width off both
//!   axes, as `_HandleShot` asks `_GetSpriteXSize` alone and never its
//!   height (@0x35c66-0x35cc0). So a frame taller than wide sits
//!   (height − width) / 2 below the standard rule's place, and one wider
//!   than tall above it.
//! - **The box's** ([`Placement::CargoBox`]): its whole size off, not
//!   half, as `_HandleBoxes` places it (@0x2e1c2-0x2e250), so the frame's
//!   bottom right is the box's point.
//!
//! **The radius test** ([`radius_touch`]), `_SWRadiusCollision`
//! (@0xabc88-0xabd26), centres each rect half its **width** right of its
//! left and half its width below its top, each half halved toward zero,
//! and touches when the centres are nearer than the two halves together,
//! strictly. A non-square rect's centre is so off its middle. The
//! original takes the rects, the halves and the centres' differences in
//! 16 bits; they are whole numbers here, as wide as they need. The
//! routine only asserts both sprites are there, and plays no sound.

use crate::geometry::Vec2;
use crate::sprite_mask::MaskRect;

/// How a routine places a sprite's rect about its offset from the player
/// (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Half each side off its axis.
    Standard,
    /// Half its width off both axes.
    Shot,
    /// Its whole size off, its bottom right on its point.
    CargoBox,
}

impl Placement {
    /// The top left of a frame of `size` (width, height) whose sprite is
    /// `offset` from the player, as this rule places it.
    #[must_use]
    pub fn top_left(self, offset: Vec2, size: (u32, u32)) -> (i32, i32) {
        // A screen offset is far within an `i32`, saturated past it, and
        // the original truncates.
        #[allow(clippy::cast_possible_truncation)]
        let trunc = |delta: f32| delta.trunc() as i32;
        let whole = |side: u32| i32::try_from(side).unwrap_or(i32::MAX);
        let half = |side: u32| whole(side / 2);
        let (x, y) = (trunc(offset.x), trunc(offset.y));
        let (width, height) = size;
        match self {
            Self::Standard => (
                x.saturating_sub(half(width)),
                y.saturating_sub(half(height)),
            ),
            Self::Shot => (x.saturating_sub(half(width)), y.saturating_sub(half(width))),
            Self::CargoBox => (
                x.saturating_sub(whole(width)),
                y.saturating_sub(whole(height)),
            ),
        }
    }

    /// The rect of a frame of `size` whose sprite is `offset` from the
    /// player, as this rule places it.
    #[must_use]
    pub fn rect(self, offset: Vec2, size: (u32, u32)) -> MaskRect {
        MaskRect::placed(self.top_left(offset, size), size)
    }
}

/// Whether rects `a` and `b` touch by `_SWRadiusCollision` (see the
/// module docs).
#[must_use]
pub fn radius_touch(a: MaskRect, b: MaskRect) -> bool {
    let half = |rect: MaskRect| (i64::from(rect.right) - i64::from(rect.left)) / 2;
    let centre = |rect: MaskRect| {
        let h = half(rect);
        (i64::from(rect.left) + h, i64::from(rect.top) + h)
    };
    let ((ax, ay), (bx, by)) = (centre(a), centre(b));
    let (dx, dy) = (ax - bx, ay - by);
    let reach = half(a) + half(b);
    dx * dx + dy * dy < reach * reach
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_rule_takes_half_each_side_its_odd_pixel_right_and_below() {
        let offset = Vec2::new(10.9, -20.9);
        assert_eq!(Placement::Standard.top_left(offset, (31, 20)), (-5, -30));
        assert_eq!(Placement::Standard.top_left(offset, (1, 3)), (10, -21));
        assert_eq!(
            Placement::Standard.rect(offset, (31, 20)),
            MaskRect {
                top: -30,
                left: -5,
                bottom: -10,
                right: 26,
            },
            "15 left of the point, 16 right"
        );
    }

    #[test]
    fn a_shot_takes_half_its_width_off_both_axes_so_a_tall_one_sits_below() {
        let offset = Vec2::new(100.7, 50.2);
        let rect = Placement::Shot.rect(offset, (10, 30));
        assert_eq!((rect.left, rect.top), (95, 45));
        assert_eq!((rect.right, rect.bottom), (105, 75));
        let centred = Placement::Standard.rect(offset, (10, 30));
        assert_eq!(rect.top - centred.top, 10, "(30 − 10) / 2 below");
        assert_eq!(rect.left, centred.left);
        // A wide one sits above.
        let wide = Placement::Shot.rect(offset, (30, 10));
        assert_eq!((wide.left, wide.top), (85, 35));
    }

    #[test]
    fn a_boxs_bottom_right_is_its_point() {
        let rect = Placement::CargoBox.rect(Vec2::new(-7.8, 12.5), (8, 6));
        assert_eq!(
            rect,
            MaskRect {
                top: 6,
                left: -15,
                bottom: 12,
                right: -7,
            }
        );
    }

    #[test]
    fn a_rule_saturates_far_off_the_screen() {
        let far = Vec2::new(f32::MAX, f32::MIN);
        assert_eq!(
            Placement::CargoBox.top_left(far, (u32::MAX, 4)),
            (0, i32::MIN)
        );
    }

    /// A square of `side` with its top left at (`x`, `y`).
    fn square(x: i32, y: i32, side: u32) -> MaskRect {
        MaskRect::placed((x, y), (side, side))
    }

    #[test]
    fn rects_touch_by_radius_strictly_nearer_than_their_two_halves() {
        // Halves 5 and 3, centred at (5, 5) and on: they touch nearer
        // than 8 apart.
        let a = square(0, 0, 10);
        assert!(!radius_touch(a, square(10, 2, 6)), "centres 8 apart");
        assert!(radius_touch(a, square(9, 2, 6)), "7 apart");
        assert!(!radius_touch(square(10, 2, 6), a), "either way round");
        assert!(radius_touch(square(9, 2, 6), a));
        // Off both axes: (5, 5) apart is 7.07, nearer than 8.
        assert!(radius_touch(a, square(7, 7, 6)));
        assert!(!radius_touch(a, square(8, 8, 6)), "(6, 6) is 8.49");
        // An odd width's half is halved toward zero: 5 of 11.
        assert!(!radius_touch(square(0, 0, 11), square(10, 2, 6)));
    }

    #[test]
    fn a_wide_rects_centre_is_half_its_width_below_its_top() {
        // 40 by 20: centred at (20, 20), on its bottom, not at (20, 10).
        let ship = MaskRect::placed((0, 0), (40, 20));
        // A 4-square shot centred at (20, 41): 21 below (20, 20), within
        // the 22 of their halves, though 31 from the middle.
        assert!(radius_touch(ship, square(18, 39, 4)));
        // One centred at (20, -5): 15 above the middle, but 25 from
        // (20, 20).
        assert!(!radius_touch(ship, square(18, -7, 4)));
        assert!(radius_touch(ship, square(18, -3, 4)), "21 from (20, 20)");
    }
}
