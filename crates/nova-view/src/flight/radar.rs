//! The radar's view model: what `_DrawStatusRadar` (@0x496e0) draws,
//! worked out in whole pixels from a [`RadarScene`] snapshot.
//! [`hud`](super::hud) draws the [`RadarMark`]s. The colour decisions are
//! the simulation's ([`nova_sim::iff`]); this maps them to colours.
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
//!   ships (@0x49882 jumps to @0x49c85), but the player pixel and the
//!   arrow still.
//! - **Static** (@0x49791-0x497d4): when not blank and the frame has
//!   interference ([`static_frame`], `_HandleStatus` @0x494d7), the radar
//!   is filled with one of `ppat` 128-137 ([`STATIC_PATTERNS`],
//!   `_LoadSprites` @0x21824-0x21842), tiled from the screen's origin as
//!   `FillCRect` tiles from the port's, and the backdrop, stellars and
//!   ships are skipped (it jumps to @0x49c85). The player pixel and the
//!   arrow still draw.
//! - **IFF** ([`RadarScene::iff`], `_HasIFF` @0x4348): the radar is
//!   painted black (`PaintRect` @0x497d9-0x497f6; `_RedrawStatus` paints
//!   the same rect @0x4cbac-0x4cc2a), each stellar is in its
//!   `_ColorCodeStellar` colour (@0x49948-0x49955, @0x499c2-0x499cf) and
//!   each ship in its `_ColorCodeShip` colour (@0x49b8c-0x49bb0), the
//!   target light grey while the blink is on (`_rgbLightGrey`,
//!   @0x49b62-0x49b87), and the player cyan (`_ColorCodeShip` of slot 0,
//!   @0x49c8b-0x49c9c). The colours are `_InitColors`'
//!   ([`RadarColor::fixed`]).
//! - **Density scanner** ([`RadarScene::density_scanner`],
//!   `_HasDensityScanner` @0x42c0, @0x49bb8-0x49c2f): a ship whose class's
//!   `Mass` is above 99 ([`RadarShip::heavy`]) is the 2×2 rect about its
//!   point, clipped to the radar and framed in its dot's colour, skipped
//!   when the clip is empty, so one at x == right is a 1-pixel-wide frame
//!   at right − 1. It works the same with or without IFF.
//! - **The centre arrow** (@0x49cde-0x49ec4, [`arrow`]): while the blink
//!   is on (`_radarCount`), when the nav mode is not 3, whatever it
//!   selects ([`RadarScene::hyperspace_mode`]), the system has a stellar, and the
//!   player is more than √7,000,000 from the system's centre
//!   ([`ARROW_MIN_DISTANCE_SQUARED`], in `f32` as `_XYFloatSquaredDist`
//!   works it). Three lines in the dim colour, with or without IFF
//!   (@0x49dd3-0x49ddd), drawn last; only the part inside the radar
//!   shows (@0x49f33).
//!
//! Left out for good: the pixel the inclusive test writes just outside
//! the radar into the original's persistent offscreen bar, and the old
//! radar image a missing static pattern (@0x497bd-0x497bf) leaves there,
//! both of which ours, with no persistent bar, does not have;
//! `_preventStatusCopy` and the final `CopyBits` (compositing only); the
//! never-set `_onPlanet`; the 16-bit coordinate stores; and QuickDraw's
//! own `LineTo` raster, which is in the Toolbox (ours is
//! [`hud::line_pixels`](super::hud::line_pixels)). The same mass test in
//! `_DrawStatusTarg` (@0x4be35) is hud-fidelity phase 5's.

use std::ops::RangeInclusive;

use nova_sim::{Chance, ShipIff, StellarIff};

use crate::Color;
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
    /// What IFF shows it as.
    pub iff: StellarIff,
}

/// One ship as the radar sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadarShip {
    /// Where it is drawn, in world units.
    pub position: Point,
    /// Whether it is the player's target.
    pub targeted: bool,
    /// What IFF shows it as.
    pub iff: ShipIff,
    /// Whether its class's `Mass` is above 99 (`shïp` +0x9ee, `cmpw
    /// $0x63`): a density scanner shows it as a square.
    pub heavy: bool,
}

/// What the radar shows, taken at a refresh.
// Each flag is one of the original's own tests.
#[allow(clippy::struct_excessive_bools)]
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
    /// Whether the player has IFF.
    pub iff: bool,
    /// Whether the player has a density scanner.
    pub density_scanner: bool,
    /// The static the radar shows this refresh, a `ppat` of
    /// [`STATIC_PATTERNS`], or `None`.
    pub static_pattern: Option<i16>,
    /// Whether the player's nav mode is 3, hyperspace, with or without a
    /// system selected (@0x49cf5), which hides the centre arrow.
    pub hyperspace_mode: bool,
}

/// Which colour a mark is in: one of the bar's two radar colours, or one
/// of the fixed colours IFF draws in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadarColor {
    /// The `ïntf`'s `DimRadar`.
    Dim,
    /// The `ïntf`'s `BrightRadar`.
    Bright,
    /// The IFF backdrop: QuickDraw's `blackColor` (33).
    Black,
    /// The player under IFF: (0, 0xFFFF, 0xFFFF) (@0x4afa-0x4b0f).
    Cyan,
    /// A disabled ship: `_rgbDarkGrey`, 0x4000 grey (@0x1bee4).
    DarkGrey,
    /// A threat, or a stellar hostile to the player: (0xFFFF, 0, 0)
    /// (@0x4b27-0x4b44).
    Red,
    /// An escort, or a dominated stellar: `_brightGreen`, (0, 0xFFFF, 0)
    /// (@0x1beb8).
    Green,
    /// Any other ship: (0, 0, 0xFFFF) (@0x4b91-0x4b9c).
    Blue,
    /// An unlandable stellar: `_rgbMedGrey`, 0x8000 grey (@0x1bf10).
    MedGrey,
    /// A wormhole: (0, 0x55F0, 0x55F0) (@0xaa71-0xaa87).
    Teal,
    /// A stellar the player may land on: (0xFFFF, 0xFFFF, 0).
    Yellow,
    /// A stellar that refuses the player: (0xFFFF, 0x6666, 0).
    Orange,
    /// The target under IFF: `_rgbLightGrey`, 0xC000 grey
    /// (@0x1bf3c-0x1bf4d).
    LightGrey,
}

impl RadarColor {
    /// The colour, as `nova_data` reads an `RGBColor` (each channel's high
    /// byte), for a fixed colour; `None` for [`Dim`](Self::Dim) and
    /// [`Bright`](Self::Bright), which the bar's layout gives.
    #[must_use]
    pub const fn fixed(self) -> Option<Color> {
        let (r, g, b) = match self {
            Self::Dim | Self::Bright => return None,
            Self::Black => (0, 0, 0),
            Self::Cyan => (0, 0xFF, 0xFF),
            Self::DarkGrey => (0x40, 0x40, 0x40),
            Self::Red => (0xFF, 0, 0),
            Self::Green => (0, 0xFF, 0),
            Self::Blue => (0, 0, 0xFF),
            Self::MedGrey => (0x80, 0x80, 0x80),
            Self::Teal => (0, 0x55, 0x55),
            Self::Yellow => (0xFF, 0xFF, 0),
            Self::Orange => (0xFF, 0x66, 0),
            Self::LightGrey => (0xC0, 0xC0, 0xC0),
        };
        Some(Color::rgba(r, g, b, 255))
    }
}

/// The colour IFF shows a ship of `iff` in.
#[must_use]
pub const fn ship_color(iff: ShipIff) -> RadarColor {
    match iff {
        ShipIff::Disabled => RadarColor::DarkGrey,
        ShipIff::Threat => RadarColor::Red,
        ShipIff::Escort => RadarColor::Green,
        ShipIff::Other => RadarColor::Blue,
    }
}

/// The colour IFF shows a stellar of `iff` in.
#[must_use]
pub const fn stellar_color(iff: StellarIff) -> RadarColor {
    match iff {
        StellarIff::Unlandable => RadarColor::MedGrey,
        StellarIff::Wormhole => RadarColor::Teal,
        StellarIff::Dominated => RadarColor::Green,
        StellarIff::Cleared => RadarColor::Yellow,
        StellarIff::Denied => RadarColor::Orange,
        StellarIff::Hostile => RadarColor::Red,
    }
}

/// The `ppat`s the static is drawn from (`_staticPat`, loaded by
/// `_LoadSprites` @0x21824-0x21842).
pub const STATIC_PATTERNS: RangeInclusive<i16> = 128..=137;

/// The static the radar shows this frame at interference `level`
/// (0..=100), drawn on `chance`: `_HandleStatus` (@0x4951f-0x4954a) shows
/// it when `Rand(100) + 1` ≤ the level, and `_DrawStatusRadar` then
/// picks one of the ten patterns with `Rand(10)` (@0x49797-0x497b5).
///
/// At level 0 nothing can show, so nothing is drawn. The original draws
/// the threshold every frame and the pattern as it draws; ours draws
/// both together, which changes no decision the player can see.
pub fn static_frame(level: u8, chance: &mut impl Chance) -> Option<i16> {
    if level == 0 || chance.roll(100) + 1 > u16::from(level) {
        return None;
    }
    let first = *STATIC_PATTERNS.start();
    Some(first + i16::try_from(chance.roll(10)).unwrap_or_default())
}

/// The distance squared from the system's centre past which the arrow
/// shows (0xdd890), strictly.
pub const ARROW_MIN_DISTANCE_SQUARED: f32 = 7_000_000.0;
/// How far from the radar's centre the arrow's shaft starts (0x41c80000).
pub const ARROW_START: f32 = 25.0;
/// How far from the radar's centre the arrow's shaft ends (0x42480000).
pub const ARROW_END: f32 = 50.0;
/// How long each of the arrow's heads is (0x40c00000).
pub const ARROW_HEAD: f32 = 6.0;
/// How far each head turns from the shaft, in degrees (`leal ±0x87`).
pub const ARROW_HEAD_TURN: i16 = 135;

pub use nova_sim::trig::table_arctan;

/// `_BuildTrigTable`'s angle for whole degree `degrees` normalized to
/// 0..360 as `_TableSin` and `_TableCos` do (@0x25913-0x25926): `(float)i
/// × 0.01745329`, in doubles, rounded to `f32` (@0x2583b-0x2584c).
fn table_radians(degrees: i16) -> f64 {
    let index = f32::from(degrees.rem_euclid(360));
    f64::from((f64::from(index) * 0.017_453_29) as f32)
}

/// `_TableSin` (@0x2590a): the `f32` sine of whole degree `degrees`.
#[must_use]
pub fn table_sin(degrees: i16) -> f32 {
    table_radians(degrees).sin() as f32
}

/// `_TableCos` (@0x2593c): the `f32` cosine of whole degree `degrees`.
#[must_use]
pub fn table_cos(degrees: i16) -> f32 {
    table_radians(degrees).cos() as f32
}

/// `_Accel` (@0x2ade6): `point` moved `magnitude` along `angle`, x by
/// `magnitude · sin` and y by `−magnitude · cos`, in `f32`.
#[must_use]
pub fn accel(angle: i16, magnitude: f32, point: (f32, f32)) -> (f32, f32) {
    (
        magnitude * table_sin(angle) + point.0,
        point.1 - magnitude * table_cos(angle),
    )
}

/// A line from one pixel to another, both ends drawn.
pub type PixelLine = ((i32, i32), (i32, i32));

/// The centre arrow on a radar centred on `center`, for the player at
/// `player` (@0x49d7b-0x49eb3): along the heading toward the system's
/// centre ([`table_arctan`]), the shaft from 25 to 50 out, then each head
/// 6 long from the shaft's end at ±135°. Each end is truncated as it is
/// drawn; each head starts again from the shaft's unrounded end.
#[must_use]
pub fn arrow(center: (i32, i32), player: Point) -> [PixelLine; 3] {
    let heading = table_arctan(player.x, player.y);
    let center = (center.0 as f32, center.1 as f32);
    let start = accel(heading, ARROW_START, center);
    let end = accel(heading, ARROW_END, center);
    let pixel = |(x, y): (f32, f32)| (x as i32, y as i32);
    let head = |turn: i16| (pixel(end), pixel(accel(heading + turn, ARROW_HEAD, end)));
    [
        (pixel(start), pixel(end)),
        head(ARROW_HEAD_TURN),
        head(-ARROW_HEAD_TURN),
    ]
}

/// Whether the arrow shows for the player at `player` by its distance
/// from the system's centre: the `f32` sum of each axis squared above
/// [`ARROW_MIN_DISTANCE_SQUARED`] (`_XYFloatSquaredDist` @0x2afdb).
#[must_use]
pub fn far_from_centre(player: Point) -> bool {
    let (dx, dy) = (player.x.abs(), player.y.abs());
    dx * dx + dy * dy > ARROW_MIN_DISTANCE_SQUARED
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
    /// A filled rect: the IFF backdrop.
    Fill {
        /// The rect.
        rect: IRect,
        /// Its colour.
        color: RadarColor,
    },
    /// A rect filled with a `ppat`, tiled from the screen's origin: the
    /// static.
    Pattern {
        /// The rect.
        rect: IRect,
        /// The `ppat`'s ID.
        id: i16,
    },
    /// A 1-pixel line, both ends drawn: only its pixels inside the radar
    /// show.
    Line {
        /// One end.
        from: (i32, i32),
        /// The other end.
        to: (i32, i32),
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
/// order: the static, or the IFF backdrop, the stellars and the ships;
/// then the player and the centre arrow. `blink_on` is whether the target
/// shows bright, and the arrow shows.
#[must_use]
pub fn marks(radar: IRect, scene: &RadarScene, blink_on: bool) -> Vec<RadarMark> {
    if radar.is_empty() {
        return Vec::new();
    }
    let (cx, cy) = radar.center();
    let mut marks = Vec::new();
    match scene.static_pattern {
        Some(id) if !scene.blank => marks.push(RadarMark::Pattern { rect: radar, id }),
        _ => {
            if scene.iff {
                marks.push(RadarMark::Fill {
                    rect: radar,
                    color: RadarColor::Black,
                });
            }
            if !scene.blank {
                contacts(&mut marks, radar, scene, blink_on);
            }
        }
    }
    let player = if scene.iff {
        RadarColor::Cyan
    } else {
        RadarColor::Bright
    };
    marks.push(RadarMark::Pixel {
        x: cx,
        y: cy,
        color: player,
    });
    let shows_arrow = blink_on
        && !scene.hyperspace_mode
        && !scene.stellars.is_empty()
        && far_from_centre(scene.player);
    if shows_arrow {
        marks.extend(
            arrow((cx, cy), scene.player).map(|(from, to)| RadarMark::Line {
                from,
                to,
                color: RadarColor::Dim,
            }),
        );
    }
    marks
}

/// Pushes the marks of `scene`'s stellars, then its ships, on `radar`.
fn contacts(marks: &mut Vec<RadarMark>, radar: IRect, scene: &RadarScene, blink_on: bool) {
    let (cx, cy) = radar.center();
    let at = |position: Point| {
        (
            radar_offset(position.x, scene.player.x, cx),
            radar_offset(position.y, scene.player.y, cy),
        )
    };
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
        let color = if scene.iff {
            stellar_color(stellar.iff)
        } else {
            RadarColor::Dim
        };
        marks.push(if stellar.square {
            RadarMark::Square { rect, color }
        } else {
            RadarMark::Oval { rect, color }
        });
    }
    for ship in &scene.ships {
        let (x, y) = at(ship.position);
        let color = match (ship.targeted && blink_on, scene.iff) {
            (true, true) => RadarColor::LightGrey,
            (true, false) => RadarColor::Bright,
            (false, true) => ship_color(ship.iff),
            (false, false) => RadarColor::Dim,
        };
        if scene.density_scanner && ship.heavy {
            if let Some(rect) = IRect::around(x, y, 1).intersection(radar) {
                marks.push(RadarMark::Square { rect, color });
            }
        } else if radar.holds(x, y) {
            marks.push(RadarMark::Pixel { x, y, color });
        }
    }
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
            ..RadarScene::default()
        }
    }

    fn oval(x: f32, y: f32, sprite_width: Option<u32>) -> RadarStellar {
        RadarStellar {
            position: at(x, y),
            square: false,
            sprite_width,
            iff: StellarIff::Cleared,
        }
    }

    fn square(x: f32, y: f32) -> RadarStellar {
        RadarStellar {
            position: at(x, y),
            square: true,
            sprite_width: Some(300),
            iff: StellarIff::Wormhole,
        }
    }

    fn ship(x: f32, y: f32) -> RadarShip {
        RadarShip {
            position: at(x, y),
            targeted: false,
            iff: ShipIff::Other,
            heavy: false,
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
            targeted: true,
            ..ship(32.0, 0.0)
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

    // IFF.

    fn with_iff(mut shown: RadarScene) -> RadarScene {
        shown.iff = true;
        shown
    }

    fn coded(x: f32, y: f32, iff: ShipIff) -> RadarShip {
        RadarShip { iff, ..ship(x, y) }
    }

    const BLACK_BACKDROP: RadarMark = RadarMark::Fill {
        rect: STOCK,
        color: RadarColor::Black,
    };

    const CYAN_PLAYER: RadarMark = RadarMark::Pixel {
        x: 926,
        y: 96,
        color: RadarColor::Cyan,
    };

    #[test]
    fn with_iff_the_radar_is_black_and_each_contact_in_its_code_and_the_player_cyan() {
        let shown = with_iff(scene(
            vec![square(0.0, -320.0), oval(320.0, 0.0, None)],
            vec![
                coded(-320.0, 0.0, ShipIff::Other),
                coded(0.0, 320.0, ShipIff::Threat),
            ],
        ));
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                BLACK_BACKDROP,
                RadarMark::Square {
                    rect: IRect::new(925, 85, 927, 87),
                    color: RadarColor::Teal
                },
                RadarMark::Oval {
                    rect: IRect::new(935, 95, 937, 97),
                    color: RadarColor::Yellow
                },
                pixel(916, 96, RadarColor::Blue),
                pixel(926, 106, RadarColor::Red),
                CYAN_PLAYER
            ]
        );
    }

    #[test]
    fn each_iff_code_has_its_colour() {
        assert_eq!(
            [
                ShipIff::Disabled,
                ShipIff::Threat,
                ShipIff::Escort,
                ShipIff::Other
            ]
            .map(ship_color),
            [
                RadarColor::DarkGrey,
                RadarColor::Red,
                RadarColor::Green,
                RadarColor::Blue
            ]
        );
        assert_eq!(
            [
                StellarIff::Unlandable,
                StellarIff::Wormhole,
                StellarIff::Dominated,
                StellarIff::Cleared,
                StellarIff::Denied,
                StellarIff::Hostile
            ]
            .map(stellar_color),
            [
                RadarColor::MedGrey,
                RadarColor::Teal,
                RadarColor::Green,
                RadarColor::Yellow,
                RadarColor::Orange,
                RadarColor::Red
            ]
        );
    }

    #[test]
    fn each_fixed_colour_is_the_high_byte_of_the_originals() {
        let rgb = |color: RadarColor| color.fixed().map(|c| (c.r, c.g, c.b, c.a));
        assert_eq!(rgb(RadarColor::Dim), None);
        assert_eq!(rgb(RadarColor::Bright), None);
        for (color, expected) in [
            (RadarColor::Black, (0, 0, 0)),
            (RadarColor::Cyan, (0, 255, 255)),
            (RadarColor::DarkGrey, (64, 64, 64)),
            (RadarColor::Red, (255, 0, 0)),
            (RadarColor::Green, (0, 255, 0)),
            (RadarColor::Blue, (0, 0, 255)),
            (RadarColor::MedGrey, (128, 128, 128)),
            (RadarColor::Teal, (0, 0x55, 0x55)),
            (RadarColor::Yellow, (255, 255, 0)),
            (RadarColor::Orange, (255, 0x66, 0)),
            (RadarColor::LightGrey, (192, 192, 192)),
        ] {
            let (r, g, b) = expected;
            assert_eq!(rgb(color), Some((r, g, b, 255)), "{color:?}");
        }
    }

    #[test]
    fn with_iff_the_target_is_light_grey_while_the_blink_is_on() {
        let target = RadarShip {
            targeted: true,
            ..coded(32.0, 0.0, ShipIff::Escort)
        };
        let shown = with_iff(scene(Vec::new(), vec![target]));
        assert_eq!(
            marks(STOCK, &shown, true)[1],
            pixel(927, 96, RadarColor::LightGrey)
        );
        assert_eq!(
            marks(STOCK, &shown, false)[1],
            pixel(927, 96, RadarColor::Green)
        );
    }

    #[test]
    fn with_iff_a_blank_radar_is_black_with_the_player_cyan() {
        let mut shown = with_iff(scene(
            vec![oval(0.0, 32.0, None)],
            vec![coded(-32.0, 0.0, ShipIff::Threat)],
        ));
        shown.blank = true;
        assert_eq!(marks(STOCK, &shown, false), [BLACK_BACKDROP, CYAN_PLAYER]);
    }

    #[test]
    fn without_iff_there_is_no_backdrop_and_no_code() {
        let shown = scene(
            vec![square(0.0, -320.0)],
            vec![coded(-320.0, 0.0, ShipIff::Threat)],
        );
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                RadarMark::Square {
                    rect: IRect::new(925, 85, 927, 87),
                    color: RadarColor::Dim
                },
                pixel(916, 96, RadarColor::Dim),
                PLAYER
            ]
        );
    }

    // The density scanner.

    fn heavy(x: f32, y: f32) -> RadarShip {
        RadarShip {
            heavy: true,
            ..ship(x, y)
        }
    }

    fn scanning(mut shown: RadarScene) -> RadarScene {
        shown.density_scanner = true;
        shown
    }

    #[test]
    fn with_a_scanner_a_heavy_ship_is_a_square_in_its_dots_colour() {
        let target = RadarShip {
            targeted: true,
            ..heavy(-32.0, 0.0)
        };
        let shown = scanning(scene(
            Vec::new(),
            vec![heavy(32.0, 0.0), target, ship(0.0, 32.0)],
        ));
        assert_eq!(
            marks(STOCK, &shown, true),
            [
                RadarMark::Square {
                    rect: IRect::new(926, 95, 928, 97),
                    color: RadarColor::Dim
                },
                RadarMark::Square {
                    rect: IRect::new(924, 95, 926, 97),
                    color: RadarColor::Bright
                },
                pixel(926, 97, RadarColor::Dim),
                PLAYER
            ]
        );
        let coded = RadarShip {
            iff: ShipIff::Threat,
            ..heavy(32.0, 0.0)
        };
        let shown = with_iff(scanning(scene(Vec::new(), vec![coded])));
        assert_eq!(
            marks(STOCK, &shown, true)[1],
            RadarMark::Square {
                rect: IRect::new(926, 95, 928, 97),
                color: RadarColor::Red
            }
        );
    }

    #[test]
    fn without_a_scanner_a_heavy_ship_is_a_pixel() {
        let shown = scene(Vec::new(), vec![heavy(32.0, 0.0)]);
        assert_eq!(
            marks(STOCK, &shown, true),
            [pixel(927, 96, RadarColor::Dim), PLAYER]
        );
    }

    #[test]
    fn a_heavy_ships_square_is_clipped_to_the_radar() {
        // x = 1014, the right edge: a 1-pixel-wide frame at 1013; x = 837,
        // just past the left edge: nothing.
        let shown = scanning(scene(
            Vec::new(),
            vec![heavy(2816.0, 0.0), heavy(-2848.0, 0.0)],
        ));
        assert_eq!(
            marks(STOCK, &shown, true),
            [
                RadarMark::Square {
                    rect: IRect::new(1013, 95, 1014, 97),
                    color: RadarColor::Dim
                },
                PLAYER
            ]
        );
    }

    // The centre arrow's heading and geometry.

    #[test]
    fn sine_and_cosine_normalize_whole_degrees() {
        for (a, b) in [(-45, 315), (405, 45), (360, 0), (-360, 0), (720, 0)] {
            assert_eq!(table_sin(a), table_sin(b), "{a}");
            assert_eq!(table_cos(a), table_cos(b), "{a}");
        }
        assert_eq!(table_sin(90), 1.0);
        assert_eq!(table_cos(0), 1.0);
        assert_eq!(table_sin(0), 0.0);
        assert!(table_cos(90) > 0.0, "a float's 90 degrees is a hair short");
        assert!(table_cos(90) < 1e-6);
        assert!(table_sin(-90) < 0.0);
    }

    #[test]
    fn accel_moves_along_the_angle_with_y_down() {
        assert_eq!(accel(90, 10.0, (1.0, 2.0)).0, 11.0);
        assert_eq!(accel(0, 10.0, (1.0, 2.0)), (1.0, -8.0));
        assert_eq!(accel(180, 10.0, (1.0, 2.0)).1, 12.0);
    }

    const CENTRE: (i32, i32) = (926, 96);

    #[test]
    fn the_arrow_runs_from_25_to_50_out_with_heads_6_long_at_135_degrees() {
        assert_eq!(
            arrow(CENTRE, at(3000.0, 0.0)),
            [
                ((901, 96), (876, 96)),
                ((876, 96), (880, 91)),
                ((876, 96), (880, 100))
            ]
        );
        assert_eq!(
            arrow(CENTRE, at(0.0, -3000.0)),
            [
                ((926, 121), (926, 146)),
                ((926, 146), (921, 141)),
                ((926, 146), (930, 141))
            ]
        );
    }

    #[test]
    fn the_arrows_ends_truncate_from_floats() {
        // The f32 cosine of 90 degrees is a hair above 0, so 96 − 25 · cos
        // truncates to 95.
        assert_eq!(
            arrow(CENTRE, at(-3000.0, 0.0)),
            [
                ((951, 95), (976, 95)),
                ((976, 95), (971, 100)),
                ((976, 95), (971, 91))
            ]
        );
        assert_eq!(
            arrow(CENTRE, at(3000.0, 3000.0)),
            [
                ((908, 78), (890, 61)),
                ((890, 61), (896, 61)),
                ((890, 61), (890, 67))
            ]
        );
    }

    // The centre arrow's conditions.

    /// A scene far from the centre, at (3000, 0), with a stellar.
    fn far() -> RadarScene {
        RadarScene {
            player: at(3000.0, 0.0),
            ..scene(vec![oval(0.0, 0.0, None)], Vec::new())
        }
    }

    fn arrow_marks(color: RadarColor) -> [RadarMark; 3] {
        arrow(CENTRE, at(3000.0, 0.0)).map(|(from, to)| RadarMark::Line { from, to, color })
    }

    fn lines(shown: &RadarScene, blink_on: bool) -> Vec<RadarMark> {
        marks(STOCK, shown, blink_on)
            .into_iter()
            .filter(|mark| matches!(mark, RadarMark::Line { .. }))
            .collect()
    }

    #[test]
    fn the_arrow_is_dim_and_drawn_last_while_the_blink_is_on() {
        let shown = far();
        // The stellar at the centre is off the radar: only the player and
        // the arrow, after it.
        let drawn = marks(STOCK, &shown, true);
        let mut expected = vec![PLAYER];
        expected.extend(arrow_marks(RadarColor::Dim));
        assert_eq!(drawn, expected);
        assert_eq!(lines(&shown, false), [], "blinks with the target");
        let iff = with_iff(far());
        assert_eq!(
            lines(&iff, true),
            arrow_marks(RadarColor::Dim),
            "dim with IFF"
        );
    }

    #[test]
    fn no_arrow_shows_in_hyperspace_mode_or_without_a_stellar() {
        let route = RadarScene {
            hyperspace_mode: true,
            ..far()
        };
        assert_eq!(lines(&route, true), []);
        let empty = RadarScene {
            stellars: Vec::new(),
            ..far()
        };
        assert_eq!(lines(&empty, true), []);
    }

    #[test]
    fn the_arrow_shows_only_past_the_distance_in_f32() {
        assert_eq!(ARROW_MIN_DISTANCE_SQUARED, 7_000_000.0);
        // 2000² + 1732.05078125² is exactly 7,000,000 in f32: not past it.
        let at_limit = at(2000.0, 1_732.050_8);
        assert_eq!(f64::from(at_limit.y), 1_732.050_781_25);
        assert!(!far_from_centre(at_limit));
        assert!(!far_from_centre(at(-2000.0, -1_732.050_8)));
        let past = at(2000.0, 1_732.051_2);
        assert!(far_from_centre(past));
        assert!(far_from_centre(at(-2000.0, -1_732.051_2)));
        let shown = |player| RadarScene { player, ..far() };
        assert_eq!(lines(&shown(at_limit), true), []);
        assert_eq!(lines(&shown(past), true).len(), 3);
    }

    #[test]
    fn the_arrow_shows_while_blank_and_under_static() {
        let blank = RadarScene {
            blank: true,
            ..far()
        };
        assert_eq!(lines(&blank, true), arrow_marks(RadarColor::Dim));
        let noisy = RadarScene {
            static_pattern: Some(130),
            ..far()
        };
        assert_eq!(lines(&noisy, true), arrow_marks(RadarColor::Dim));
    }

    // Interference static.

    /// Rolls these outcomes in order, recording each roll's sides.
    struct Rolls {
        outcomes: Vec<u16>,
        sides: Vec<u16>,
    }

    impl Rolls {
        fn of(outcomes: &[u16]) -> Self {
            Self {
                outcomes: outcomes.iter().rev().copied().collect(),
                sides: Vec::new(),
            }
        }
    }

    impl Chance for Rolls {
        fn fires(&mut self, _percent: u8) -> bool {
            unreachable!("the static rolls")
        }

        fn below(&mut self, _n: u32) -> u32 {
            unreachable!("the static rolls")
        }

        fn roll(&mut self, sides: u16) -> u16 {
            self.sides.push(sides);
            self.outcomes.pop().expect("scripted")
        }
    }

    #[test]
    fn static_shows_when_the_roll_plus_one_is_at_most_the_level() {
        let mut chance = Rolls::of(&[29, 0]);
        assert_eq!(static_frame(30, &mut chance), Some(128));
        assert_eq!(chance.sides, [100, 10]);
        let mut chance = Rolls::of(&[30]);
        assert_eq!(static_frame(30, &mut chance), None);
        assert_eq!(chance.sides, [100], "no pattern drawn");
        let mut chance = Rolls::of(&[99, 9]);
        assert_eq!(static_frame(100, &mut chance), Some(137));
    }

    #[test]
    fn no_static_shows_at_level_0_and_nothing_is_drawn() {
        let mut chance = Rolls::of(&[0, 0]);
        assert_eq!(static_frame(0, &mut chance), None);
        assert!(chance.sides.is_empty());
    }

    #[test]
    fn the_static_is_one_of_ppat_128_to_137() {
        assert_eq!(STATIC_PATTERNS, 128..=137);
        for roll in 0..10 {
            let mut chance = Rolls::of(&[0, roll]);
            assert_eq!(
                static_frame(1, &mut chance),
                Some(128 + i16::try_from(roll).unwrap())
            );
        }
    }

    #[test]
    fn static_hides_the_backdrop_stellars_and_ships_but_not_the_player() {
        let shown = RadarScene {
            static_pattern: Some(131),
            ..with_iff(scene(
                vec![oval(0.0, 32.0, None)],
                vec![coded(-32.0, 0.0, ShipIff::Threat)],
            ))
        };
        assert_eq!(
            marks(STOCK, &shown, false),
            [
                RadarMark::Pattern {
                    rect: STOCK,
                    id: 131
                },
                CYAN_PLAYER
            ]
        );
    }

    #[test]
    fn a_blank_radar_shows_no_static() {
        let shown = RadarScene {
            static_pattern: Some(131),
            blank: true,
            ..scene(vec![oval(0.0, 32.0, None)], Vec::new())
        };
        assert_eq!(marks(STOCK, &shown, false), [PLAYER]);
    }
}
