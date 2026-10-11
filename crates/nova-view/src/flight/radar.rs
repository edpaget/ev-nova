//! The radar's view model: what `_DrawStatusRadar` (@0x496e0) draws on
//! its path without IFF, worked out in whole pixels from a [`RadarScene`]
//! snapshot. [`hud`](super::hud) draws the [`RadarMark`]s.
//!
//! - **Rect and centre:** the `ïntf` `RadarArea`, placed with the bar. An
//!   empty one (top ≥ bottom or left ≥ right, @0x496f6-0x4970b) draws
//!   nothing at all. The centre is `trunc((left + right) · 0.5)` and
//!   `trunc((top + bottom) · 0.5)` (@0x49844-0x4987f).
//! - **Scale:** a world position maps to `trunc(trunc(x − player.x) ·
//!   0.03125 + centre.x)`, and the same for y (0xdd888 = 0.03125): the
//!   offset is truncated before it is scaled (cvttss2si), and the sum
//!   again (cvttsd2si); stellars @0x498cb-0x49935, ships @0x49afb-0x49b5e.
//! - **Stellars**, in nav order (@0x498a0-0x498c5), with no visibility
//!   check: a point rect, inset, clipped to the radar by `SectRect` and
//!   framed, skipped only when the intersection is empty (@0x4999c-0x499af,
//!   @0x49a6d-0x49a80). A station (spöb `Flags` 0x10) or a hypergate or
//!   wormhole (`Flags2` 0x3000) is a 2×2 square (@0x49939-0x49946,
//!   @0x49971-0x499b8); any other stellar an oval inset by its sprite's
//!   width ([`inset_for_width`], @0x499e8-0x49a47; `FrameOval` @0x49a85).
//!   A straddling shape is framed in its clipped rect, squashed, not cut.
//! - **Ships**, in slot order: a pixel each, drawn only inside the radar.
//!   The original's test is inclusive (@0x49c31-0x49c70), but only the
//!   radar rect reaches the window (@0x49f33), so a dot on its right or
//!   bottom edge never shows: ours keeps left ≤ x < right and top ≤ y <
//!   bottom.
//! - **Colours:** `_InitStatus` (@0x4948a-0x49499) points the dim colour
//!   at the `ïntf`'s `DimRadar` and the bright one at its `BrightRadar`.
//!   Stellars (@0x4995f, @0x499d9) and ships (@0x49ba9) are dim; the
//!   targeted ship is bright while the blink is on (`_radarCount` ≠ 0,
//!   @0x49b62-0x49b87).
//! - **The player** is one bright pixel at the centre, drawn last
//!   (@0x49ca6-0x49cd9).
//! - **Blank** (`_playerIsLanded`, @0x49711-0x49724): no stellars and no
//!   ships (@0x49882 jumps to @0x49c85), but the player pixel still.
//!
//! Left out here, for hud-fidelity phase 3: the IFF path (@0x497eb,
//! `_ColorCodeStellar`, `_ColorCodeShip`, the IFF target colour), the
//! density scanner's squares (@0x49bb8-0x49c2f), the interference static
//! (@0x49797-0x497d4) and the centre arrow (@0x49cde-0x49eb3). Left out
//! for good: the pixel the inclusive test writes just outside the radar
//! into the original's persistent offscreen bar, which ours does not have;
//! `_preventStatusCopy` and the final `CopyBits` (compositing only); the
//! never-set `_onPlanet`; and the 16-bit coordinate stores.

use crate::geometry::{Bounds, Point};

/// Radar pixels per world unit: the original's 0xdd888.
pub const RADAR_SCALE: f64 = 0.03125;

/// A QuickDraw-style rect in whole pixels: `left` and `top` are in it,
/// `right` and `bottom` just past it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
    /// The left edge.
    pub left: i32,
    /// The top edge.
    pub top: i32,
    /// The right edge, just past the last column.
    pub right: i32,
    /// The bottom edge, just past the last row.
    pub bottom: i32,
}

impl IRect {
    /// The rect from `left`, `top` to `right`, `bottom`.
    #[must_use]
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// `bounds` in whole pixels, each edge truncated toward zero.
    #[must_use]
    pub fn from_bounds(bounds: Bounds) -> Self {
        Self::new(
            bounds.min.x as i32,
            bounds.min.y as i32,
            bounds.max.x as i32,
            bounds.max.y as i32,
        )
    }

    /// The rect as view bounds.
    #[must_use]
    pub fn to_bounds(self) -> Bounds {
        Bounds {
            min: Point::new(self.left as f32, self.top as f32),
            max: Point::new(self.right as f32, self.bottom as f32),
        }
    }

    /// Whether it holds no pixel: top ≥ bottom or left ≥ right.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// The point rect at (`x`, `y`) inset by −`by` on every side, as
    /// `InsetRect` grows it.
    #[must_use]
    pub fn around(x: i32, y: i32, by: i32) -> Self {
        Self::new(x - by, y - by, x + by, y + by)
    }

    /// Where it overlaps `other`, as `SectRect` gives it, or `None` when
    /// that is empty.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let overlap = Self::new(
            self.left.max(other.left),
            self.top.max(other.top),
            self.right.min(other.right),
            self.bottom.min(other.bottom),
        );
        (!overlap.is_empty()).then_some(overlap)
    }

    /// Its centre as `_DrawStatusRadar` works it out: each half-sum
    /// truncated toward zero.
    #[must_use]
    pub fn center(self) -> (i32, i32) {
        (
            i32::midpoint(self.left, self.right),
            i32::midpoint(self.top, self.bottom),
        )
    }

    /// Whether the pixel at (`x`, `y`) is in it.
    #[must_use]
    pub fn holds(self, x: i32, y: i32) -> bool {
        self.left <= x && x < self.right && self.top <= y && y < self.bottom
    }
}

/// One stellar as the radar sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadarStellar {
    /// Where it is, in world units.
    pub position: Point,
    /// Whether it is drawn as a square: a station, hypergate or wormhole.
    pub square: bool,
    /// The width of its sprite's frames, or `None` without one.
    pub sprite_width: Option<u32>,
}

/// One ship as the radar sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadarShip {
    /// Where it is drawn, in world units.
    pub position: Point,
    /// Whether it is the player's target.
    pub targeted: bool,
}

/// What the radar shows, taken at a refresh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RadarScene {
    /// Where the player's ship is drawn, in world units.
    pub player: Point,
    /// The system's stellars, in nav order.
    pub stellars: Vec<RadarStellar>,
    /// The other ships the radar shows, in slot order.
    pub ships: Vec<RadarShip>,
    /// Whether the stellars and ships are left out: while the player is
    /// landed.
    pub blank: bool,
}

/// Which of the bar's two radar colours a mark is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadarColor {
    /// The `ïntf`'s `DimRadar`.
    Dim,
    /// The `ïntf`'s `BrightRadar`.
    Bright,
}

/// One thing the radar draws, in whole screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadarMark {
    /// One pixel, at (`x`, `y`).
    Pixel {
        /// Its column.
        x: i32,
        /// Its row.
        y: i32,
        /// Its colour.
        color: RadarColor,
    },
    /// A framed square, already clipped to the radar.
    Square {
        /// The square's rect.
        rect: IRect,
        /// Its colour.
        color: RadarColor,
    },
    /// A framed oval inscribed in `rect`, already clipped to the radar.
    Oval {
        /// The oval's rect.
        rect: IRect,
        /// Its colour.
        color: RadarColor,
    },
}

/// How far an oval stellar's point rect is grown on each side for a
/// sprite `width` wide: 3 past 199 pixels, 2 past 89, otherwise 1, and 1
/// with no sprite (@0x499e8-0x49a47).
#[must_use]
pub fn inset_for_width(width: Option<u32>) -> i32 {
    match width {
        Some(width) if width > 199 => 3,
        Some(width) if width > 89 => 2,
        _ => 1,
    }
}

/// The radar coordinate of world coordinate `world` for a player at
/// `player` on that axis, about the radar's `center`: the offset
/// truncated, scaled by [`RADAR_SCALE`], added to the centre and
/// truncated again.
#[must_use]
pub fn radar_offset(world: f32, player: f32, center: i32) -> i32 {
    let offset = (world - player) as i32;
    f64::from(offset).mul_add(RADAR_SCALE, f64::from(center)) as i32
}

/// What the radar `radar` draws for `scene` (see the module docs), in
/// order: the stellars, the ships, then the player. `blink_on` is
/// whether the target shows bright.
#[must_use]
pub fn marks(radar: IRect, scene: &RadarScene, blink_on: bool) -> Vec<RadarMark> {
    if radar.is_empty() {
        return Vec::new();
    }
    let (cx, cy) = radar.center();
    let at = |position: Point| {
        (
            radar_offset(position.x, scene.player.x, cx),
            radar_offset(position.y, scene.player.y, cy),
        )
    };
    let mut marks = Vec::new();
    if !scene.blank {
        for stellar in &scene.stellars {
            let (x, y) = at(stellar.position);
            let inset = if stellar.square {
                1
            } else {
                inset_for_width(stellar.sprite_width)
            };
            let Some(rect) = IRect::around(x, y, inset).intersection(radar) else {
                continue;
            };
            let color = RadarColor::Dim;
            marks.push(if stellar.square {
                RadarMark::Square { rect, color }
            } else {
                RadarMark::Oval { rect, color }
            });
        }
        for ship in &scene.ships {
            let (x, y) = at(ship.position);
            if radar.holds(x, y) {
                let color = if ship.targeted && blink_on {
                    RadarColor::Bright
                } else {
                    RadarColor::Dim
                };
                marks.push(RadarMark::Pixel { x, y, color });
            }
        }
    }
    marks.push(RadarMark::Pixel {
        x: cx,
        y: cy,
        color: RadarColor::Bright,
    });
    marks
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// The stock bar's radar, at (830, 0): (8, 8)-(184, 184) on the bar,
    /// centred on (926, 96).
    const STOCK: IRect = IRect::new(838, 8, 1014, 184);

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn scene(stellars: Vec<RadarStellar>, ships: Vec<RadarShip>) -> RadarScene {
        RadarScene {
            player: at(0.0, 0.0),
            stellars,
            ships,
            blank: false,
        }
    }

    fn oval(x: f32, y: f32, sprite_width: Option<u32>) -> RadarStellar {
        RadarStellar {
            position: at(x, y),
            square: false,
            sprite_width,
        }
    }

    fn square(x: f32, y: f32) -> RadarStellar {
        RadarStellar {
            position: at(x, y),
            square: true,
            sprite_width: Some(300),
        }
    }

    fn ship(x: f32, y: f32) -> RadarShip {
        RadarShip {
            position: at(x, y),
            targeted: false,
        }
    }

    fn pixel(x: i32, y: i32, color: RadarColor) -> RadarMark {
        RadarMark::Pixel { x, y, color }
    }

    const PLAYER: RadarMark = RadarMark::Pixel {
        x: 926,
        y: 96,
        color: RadarColor::Bright,
    };

    // The rect.

    #[test]
    fn the_centre_is_each_half_sum_truncated() {
        assert_eq!(STOCK.center(), (926, 96));
        assert_eq!(IRect::new(0, 0, 5, 7).center(), (2, 3));
        assert_eq!(IRect::new(-5, -7, 0, 0).center(), (-2, -3), "toward zero");
    }

    #[test]
    fn a_rect_is_empty_when_either_side_is_not_positive() {
        assert!(!STOCK.is_empty());
        assert!(!IRect::new(0, 0, 1, 1).is_empty());
        assert!(IRect::new(0, 0, 0, 1).is_empty());
        assert!(IRect::new(0, 0, 1, 0).is_empty());
        assert!(IRect::new(2, 0, 1, 1).is_empty());
        assert!(IRect::new(0, 2, 1, 1).is_empty());
    }

    #[test]
    fn rects_intersect_as_sect_rect_does() {
        let a = IRect::new(0, 0, 10, 10);
        assert_eq!(
            a.intersection(IRect::new(5, -5, 15, 5)),
            Some(IRect::new(5, 0, 10, 5))
        );
        assert_eq!(
            a.intersection(IRect::new(-5, 5, 5, 15)),
            Some(IRect::new(0, 5, 5, 10))
        );
        assert_eq!(a.intersection(IRect::new(10, 0, 12, 10)), None, "touching");
        assert_eq!(a.intersection(IRect::new(0, 10, 10, 12)), None, "touching");
    }

    #[test]
    fn a_pixel_is_held_from_the_left_and_top_to_just_before_the_right_and_bottom() {
        let a = IRect::new(0, 0, 10, 10);
        assert!(a.holds(0, 0));
        assert!(a.holds(9, 9));
        assert!(!a.holds(10, 5));
        assert!(!a.holds(5, 10));
        assert!(!a.holds(-1, 5));
        assert!(!a.holds(5, -1));
    }

    #[test]
    fn bounds_convert_by_truncating_each_edge() {
        let bounds = Bounds {
            min: at(838.0, 8.9),
            max: at(1014.5, -184.7),
        };
        assert_eq!(IRect::from_bounds(bounds), IRect::new(838, 8, 1014, -184));
        assert_eq!(
            IRect::new(1, 2, 3, 4).to_bounds(),
            Bounds {
                min: at(1.0, 2.0),
                max: at(3.0, 4.0)
            }
        );
    }

    // The scale.

    #[test]
    fn the_scale_is_a_thirty_second_with_the_offset_and_the_sum_truncated() {
        assert_eq!(RADAR_SCALE, 0.03125);
        // +(320, −64) from the player: 10 right and 2 up.
        assert_eq!(radar_offset(320.0, 0.0, 926), 936);
        assert_eq!(radar_offset(-64.0, 0.0, 96), 94);
        // The offset is truncated before it is scaled: −32.5 is −32, one
        // pixel left (925); scaled untruncated it would be −1.016, and the
        // sum, 924.98, truncates to 924. Likewise −0.5 is 0 (926, not 925).
        assert_eq!(radar_offset(-32.5, 0.0, 926), 925);
        assert_eq!(radar_offset(-0.5, 0.0, 926), 926);
        // +63.9 is +63, 1.97 pixels: 927.97 truncates to 927, not 928.
        assert_eq!(radar_offset(63.9, 0.0, 926), 927);
        // The sum truncates (toward zero): −33 is 924.97, so 924, not 925.
        assert_eq!(radar_offset(-33.0, 0.0, 926), 924);
        // About the player: the offset, not the position.
        assert_eq!(radar_offset(1320.0, 1000.0, 926), 936);
        assert_eq!(radar_offset(-1000.0, -1064.0, 96), 98);
    }

    #[test]
    fn a_stellar_maps_about_the_player_on_both_axes() {
        let mut shown = scene(vec![oval(320.0, -64.0, None)], Vec::new());
        assert_eq!(
            marks(STOCK, &shown, false)[0],
            RadarMark::Oval {
                rect: IRect::new(935, 93, 937, 95),
                color: RadarColor::Dim
            }
        );
        shown.player = at(320.0, -64.0);
        assert_eq!(
            marks(STOCK, &shown, false)[0],
            RadarMark::Oval {
                rect: IRect::new(925, 95, 927, 97),
                color: RadarColor::Dim
            }
        );
    }

    // Ships and the player.

    #[test]
    fn a_ship_is_one_dim_pixel_and_the_player_a_bright_one_last() {
        let shown = scene(Vec::new(), vec![ship(320.0, -64.0), ship(-32.0, 32.0)]);
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                pixel(936, 94, RadarColor::Dim),
                pixel(925, 97, RadarColor::Dim),
                PLAYER
            ]
        );
    }

    #[test]
    fn the_player_pixel_is_at_the_centre_of_any_radar() {
        let shown = scene(Vec::new(), Vec::new());
        assert_eq!(marks(STOCK, &shown, false), [PLAYER]);
        assert_eq!(
            marks(IRect::new(10, 20, 31, 61), &shown, true),
            [pixel(20, 40, RadarColor::Bright)]
        );
    }

    #[test]
    fn stellars_come_before_ships_in_their_own_order() {
        let shown = scene(
            vec![square(0.0, -320.0), oval(320.0, 0.0, None)],
            vec![ship(-320.0, 0.0), ship(0.0, 320.0)],
        );
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                RadarMark::Square {
                    rect: IRect::new(925, 85, 927, 87),
                    color: RadarColor::Dim
                },
                RadarMark::Oval {
                    rect: IRect::new(935, 95, 937, 97),
                    color: RadarColor::Dim
                },
                pixel(916, 96, RadarColor::Dim),
                pixel(926, 106, RadarColor::Dim),
                PLAYER
            ]
        );
    }

    // Shapes.

    #[test]
    fn a_station_or_gate_is_a_two_pixel_square_whatever_its_size() {
        let shown = scene(vec![square(320.0, -64.0)], Vec::new());
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                RadarMark::Square {
                    rect: IRect::new(935, 93, 937, 95),
                    color: RadarColor::Dim
                },
                PLAYER
            ]
        );
    }

    #[test]
    fn an_oval_grows_with_its_sprites_width() {
        assert_eq!(
            [
                None,
                Some(0),
                Some(89),
                Some(90),
                Some(199),
                Some(200),
                Some(5000)
            ]
            .map(inset_for_width),
            [1, 1, 1, 2, 2, 3, 3]
        );
        for (width, rect) in [
            (Some(89), IRect::new(925, 95, 927, 97)),
            (Some(90), IRect::new(924, 94, 928, 98)),
            (Some(199), IRect::new(924, 94, 928, 98)),
            (Some(200), IRect::new(923, 93, 929, 99)),
            (None, IRect::new(925, 95, 927, 97)),
        ] {
            let shown = scene(vec![oval(0.0, 0.0, width)], Vec::new());
            assert_eq!(
                marks(STOCK, &shown, false)[0],
                RadarMark::Oval {
                    rect,
                    color: RadarColor::Dim
                },
                "{width:?}"
            );
        }
    }

    // Clipping.

    #[test]
    fn a_shape_straddling_the_edge_is_clipped_and_one_outside_is_dropped() {
        // x = 1013: the 6×6 oval (1010, 93)-(1016, 99) is clipped at 1014.
        let straddling = oval(2784.0, 0.0, Some(200));
        // x = 1017: (1014, 93)-(1020, 99) only touches the radar.
        let outside = oval(2912.0, 0.0, Some(200));
        // (838, 8), the top-left corner: (837, 7)-(839, 9) is clipped.
        let corner = square(-2816.0, -2816.0);
        let shown = scene(vec![straddling, outside, corner], Vec::new());
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                RadarMark::Oval {
                    rect: IRect::new(1010, 93, 1014, 99),
                    color: RadarColor::Dim
                },
                RadarMark::Square {
                    rect: IRect::new(838, 8, 839, 9),
                    color: RadarColor::Dim
                },
                PLAYER
            ]
        );
    }

    #[test]
    fn a_ship_shows_from_the_left_and_top_edges_to_just_before_the_right_and_bottom() {
        // Columns 838 (left), 1013 and 1014 (right); rows 8, 183 and 184.
        let shown = scene(
            Vec::new(),
            vec![
                ship(-2816.0, 0.0),
                ship(2784.0, 0.0),
                ship(2816.0, 0.0),
                ship(0.0, -2816.0),
                ship(0.0, 2784.0),
                ship(0.0, 2816.0),
                ship(-2848.0, 0.0),
                ship(0.0, -2848.0),
            ],
        );
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                pixel(838, 96, RadarColor::Dim),
                pixel(1013, 96, RadarColor::Dim),
                pixel(926, 8, RadarColor::Dim),
                pixel(926, 183, RadarColor::Dim),
                PLAYER
            ]
        );
    }

    // Colours and the blink.

    #[test]
    fn the_target_is_bright_only_while_the_blink_is_on() {
        let target = RadarShip {
            position: at(32.0, 0.0),
            targeted: true,
        };
        let shown = scene(vec![oval(0.0, 32.0, None)], vec![ship(-32.0, 0.0), target]);
        let ships = |blink_on| {
            marks(STOCK, &shown, blink_on)
                .into_iter()
                .filter(|mark| matches!(mark, RadarMark::Pixel { x, .. } if *x != 926))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ships(true),
            [
                pixel(925, 96, RadarColor::Dim),
                pixel(927, 96, RadarColor::Bright)
            ]
        );
        assert_eq!(
            ships(false),
            [
                pixel(925, 96, RadarColor::Dim),
                pixel(927, 96, RadarColor::Dim)
            ]
        );
        assert_eq!(
            marks(STOCK, &shown, true)[0],
            RadarMark::Oval {
                rect: IRect::new(925, 96, 927, 98),
                color: RadarColor::Dim
            },
            "a stellar stays dim"
        );
    }

    // Blank and empty.

    #[test]
    fn a_blank_radar_shows_only_the_player() {
        let mut shown = scene(
            vec![oval(0.0, 32.0, None), square(32.0, 0.0)],
            vec![ship(-32.0, 0.0)],
        );
        shown.blank = true;
        assert_eq!(marks(STOCK, &shown, true), [PLAYER]);
    }

    #[test]
    fn an_empty_radar_area_draws_nothing_not_even_the_player() {
        let shown = scene(vec![oval(0.0, 0.0, None)], vec![ship(0.0, 0.0)]);
        for radar in [
            IRect::new(838, 8, 838, 184),
            IRect::new(838, 8, 1014, 8),
            IRect::new(1014, 8, 838, 184),
            IRect::new(838, 184, 1014, 8),
        ] {
            assert_eq!(marks(radar, &shown, true), [], "{radar:?}");
        }
    }
}
