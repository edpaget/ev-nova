//! The floating map: the small galaxy map the original shows over flight
//! while H or `\` is held (`_ShowFloatingMap` @0x11440-0x114f2 and
//! `_DrawMapInFloatingFrame` @0x11304-0x1143f in the `EV Nova`
//! executable), its frame, clock, zoom, click boxes, and the pass that
//! clips and fades what it draws. The flight view ([`FlightView`]) shows
//! it, takes its clicks and keys and draws it; this module is its pure
//! logic.
//!
//! - **The frame** ([`frame`]) is `_SetupSpriteWorld`'s
//!   (@0x21e86-0x21f61): a square of side max(200, trunc(width × 0.25)),
//!   at (width − side − 210, 150). In the 1024 x 768 window it is 256 x
//!   256 at (558, 150)-(814, 406). Its clicks are tested half-open, as
//!   `PtInRect` does ([`contains`]).
//! - **The clock.** Each show ([`FloatingMap::show`]) restarts it. The map
//!   is drawn at level w = clamp(150 − (now − start), 0, 32) in 32nds
//!   ([`FloatingMap::level`], @0x38ba0-0x38bce): full until 118 ticks
//!   after it was last shown, 31/32 at 119, and gone from 150, though it
//!   stays up, and takes clicks and zooms, until it is hidden once more
//!   than 500 ticks have passed ([`FloatingMap::expire`], `_HandleComm`
//!   @0x30262-0x30285) or by the acknowledge key.
//! - **The zoom** is an index into the course map's [`SCALES`], starting
//!   at 16/9, the inverse of the original's `_floatingMapZoom` 0.5625
//!   (data @0xde948). `-` steps it toward 27/64 and `=` toward 64/27
//!   (`_HandlePlayer` @0x70f4e-0x71110), each saying whether it moved. The
//!   original's zoom steps by ×4/3 while it is below 2.0 and by ×0.75
//!   while it is above 0.5, which reaches exactly these seven.
//! - **A click** ([`hit`], `_HandlePlayer` @0x6a130-0x6a762) on the
//!   current system's centre ±8 picks it; otherwise the first of its links
//!   whose box, trunc(centre + (link − current) / zoom) ±8 (±12 at the
//!   largest scale, where the zoom ≤ 0.5), holds the point picks that
//!   link; anything else in the frame is a miss, which the flight
//!   swallows.
//! - **Drawing** ([`draw`]): the frame's border in `cölr` `FloatingMap`,
//!   then the course map's own drawing ([`GalaxyMap::draw_marks`])
//!   centred on the current system at the zoom, in the 16-bit order
//!   (@0x113e3-0x11418), so the map can draw over the border. Then
//!   [`clip_and_fade`] clips it to the frame and fades it to the level.
//!
//! # Clipping and fading in the view
//!
//! The original paints the frame black, draws the map into it, and ORs it
//! into the scene at level w/32 per 5-bit channel:
//! ((map × w) >> 5) | screen (`_BlitPixieFloatingRectDrawProc`
//! @0xba54f-0xba55b, `_BlitPixieTranslucentCopy` @0xc1110). Black ORs in
//! nothing, so over black space the frame's lines come out as map × w/32,
//! which alpha w/32 over black gives too, rounding aside. Ours draws no
//! black fill: it clips the map's lines and dots to the frame by hand and
//! draws them straight onto the scene at alpha w/32. The renderer has no
//! clipped layer to OR a texture with.
//!
//! The original fades the map as one opaque image. Ours fades each
//! command on its own, so where the map's own marks overlap they would
//! show through each other while it fades (ticks 119-149). The pass
//! resolves the overlaps the map draws:
//!
//! - every line and dot is cut away where a dot drawn after it covers it:
//!   the outline under each system's dot, the current system's mark under
//!   both, and the ends of the links and the course under the dots;
//! - a line is dropped where a later line has the same ends and is at
//!   least as wide: a link under the course, or under the link to the
//!   system selected in mode 3.
//!
//! # Trade-offs (recorded, not built)
//!
//! - **A line over a lit pixel blends where the original ORs.** Where a line
//!   crosses a star, a ship or a planet, ours mixes the two; the original
//!   ORs them, which brightens. Over black the two agree.
//! - **Overlaps the pass leaves.** While the map fades, a crossing of two
//!   lines blends there; the course, three wide, shows through the
//!   narrower link to the system selected in mode 3 drawn along it; and a
//!   wide line is clipped along its centre line, so it can stand up to
//!   half its width past the frame's edge or a dot's. Each is a pixel or
//!   two for half a second.
//! - **Only lines and dots are clipped.** The pass drops every text,
//!   picture and sprite. The map draws none of these in the frame. When
//!   the galaxy-map-fidelity roadmap's `_DrawMap` port adds names, ovals
//!   or arrow `cicn`s to [`GalaxyMap::draw_marks`], it must teach this
//!   pass to clip them.
//! - **Particles and the cursor.** The original ORs the frame over the
//!   cursor sprite and draws its particles after it (`_PostDrawCallback`
//!   @0x38c80-0x38c9a). Ours draws its effects before the HUD and the
//!   cursor last, so a particle or the cursor crossing the frame is the
//!   only place the order shows.
//! - **The clock is flight time.** The map counts Mac ticks of the
//!   flight's elapsed time ([`sixtieths`](crate::time::sixtieths)), as the
//!   escort menu's timeout and the HUD cadence do. It runs through a
//!   jump's streak and fade, so `\` shows the map during a jump and its
//!   fade goes on. It stops where flight time stops: while the course map
//!   or a dialog over paused flight is open, and in the spaceport. The
//!   original's `TickCount()` runs on through all of them, so a map shown
//!   less than 150 ticks before one of those pauses shows the rest of its
//!   fade after it, where the original would usually have run it out.
//!   Double time doesn't change the clock: elapsed time is real time
//!   either way, as `TickCount()` is.
//!
//! # Left out, each with its address and reason
//!
//! - `_ShowFloatingMap` @0x1147b-0x114c8, the `_selSystem` preselect, and
//!   @0x114ea, `_RecalcMissionSystsForMap`. They only feed `_DrawMap`'s
//!   names and mission arrows, which ours' map doesn't draw. They go with
//!   the galaxy-map-fidelity roadmap's `_DrawMap` port.
//! - `_ShowFloatingMap` @0x11445-0x11480, the zoom reset to 1.0 when it
//!   is ≤ 0: the zoom starts at 0.5625 and only ever multiplies by
//!   positive steps.
//! - `_DrawMapInFloatingFrame` @0x1139b-0x113d5, the 8-bit order (the
//!   map, then the border): ours renders one depth.
//! - `_PostAnimateHandler` @0x38d87-0x38e3c and `_FloaterEraseCallback`
//!   (@0x389d2), which erase the frame: ours redraws every frame.
//! - `_HandlePlayer` @0x6a060-0x6a12b, the escort menu's row clicks, which
//!   come before the map's (the escorts follow-up's). Ours' menu doesn't
//!   meet the frame, so the order can't matter.
//! - The zoom outlives a change of pilot in the original, a static. Ours
//!   keeps it on the flight view, which is rebuilt for each pilot.
//! - The zoom keys share one latch (data 0xdee58, @0x710c1-0x71110), so
//!   holding one blocks the others until all are up. Ours acts on each
//!   press, so holding one zoom key while pressing another zooms.
//! - `_quickKeyCheckResult` blocks a zoom while any key `_HandlePlayer`
//!   polled is down. Ours blocks it while a key the flight holds is down
//!   (the flight keys, the modifiers, H and `\`); keys it doesn't hold (J,
//!   B, Y, …) don't block it.
//!
//! [`FlightView`]: super::FlightView
//! [`GalaxyMap::draw_marks`]: crate::galaxy::GalaxyMap::draw_marks

use std::cmp::Ordering;

use crate::color::Color;
use crate::draw::{DrawCommand, DrawList, fill_rect};
use crate::galaxy::SystemId;
use crate::galaxy::view::{SCALES, Transform};
use crate::geometry::{Bounds, Point};

/// The frame's smallest side.
pub const MIN_SIDE: f32 = 200.0;
/// How far the frame's right edge stands from the window's, less its side.
pub const RIGHT_GAP: f32 = 210.0;
/// The frame's top.
pub const TOP: f32 = 150.0;
/// The level the map is drawn at in full: 32 of 32.
pub const FULL: u8 = 32;
/// The ticks after a show at which the level reaches 0.
pub const FADED_AT: u128 = 150;
/// The ticks a show keeps the map up: it is hidden once more have passed.
pub const SHOWN_FOR: u128 = 500;
/// The zoom index the map starts at: 16/9 in [`SCALES`].
pub const START_ZOOM: usize = 5;
/// Half a click box's side: the current system's centre box, and a link's
/// box below the largest scale.
pub const BOX_HALF: f32 = 8.0;
/// Half a link's click box's side at the largest scale (see
/// [`FloatingMap::big_boxes`]).
pub const BIG_BOX_HALF: f32 = 12.0;

/// The original's zooms, the inverses of [`SCALES`], as the click boxes
/// divide by them.
const ZOOMS: [f64; 7] = [
    64.0 / 27.0,
    16.0 / 9.0,
    4.0 / 3.0,
    1.0,
    0.75,
    0.5625,
    0.421_875,
];

/// The frame in a window of `view` (width, height): see the module docs.
#[must_use]
pub fn frame(view: (f32, f32)) -> Bounds {
    let side = (view.0 * 0.25).trunc().max(MIN_SIDE);
    Bounds::at(Point::new(view.0 - side - RIGHT_GAP, TOP), side, side)
}

/// Whether `at` is in `area`, half-open as `PtInRect` tests it: the left
/// and top edges in, the right and bottom out.
#[must_use]
pub fn contains(area: Bounds, at: Point) -> bool {
    (area.min.x..area.max.x).contains(&at.x) && (area.min.y..area.max.y).contains(&at.y)
}

/// The frame's centre, in whole units as the original takes it:
/// ((left + right) / 2, (top + bottom) / 2) (@0x6a16c-0x6a1b6).
#[must_use]
pub fn centre(frame: Bounds) -> Point {
    let half = |a: f32, b: f32| i32::midpoint(a as i32, b as i32) as f32;
    Point::new(
        half(frame.min.x, frame.max.x),
        half(frame.min.y, frame.max.y),
    )
}

/// Where the map at zoom index `zoom` draws a world point: `current`, the
/// current system's position, at the frame's [`centre`].
#[must_use]
pub fn transform(frame: Bounds, current: Point, zoom: usize) -> Transform {
    Transform {
        screen: centre(frame),
        world: current,
        scale: SCALES[zoom],
    }
}

/// The floating map's state: up or not, when it was last shown, and its
/// zoom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatingMap {
    /// The Mac tick it was last shown at, while it is up.
    shown_at: Option<u128>,
    /// The index of its scale in [`SCALES`].
    zoom: usize,
}

impl Default for FloatingMap {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatingMap {
    /// Down, at [`START_ZOOM`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            shown_at: None,
            zoom: START_ZOOM,
        }
    }

    /// Shows it at Mac tick `now`, restarting its clock.
    pub fn show(&mut self, now: u128) {
        self.shown_at = Some(now);
    }

    /// Hides it.
    pub fn hide(&mut self) {
        self.shown_at = None;
    }

    /// Hides it once more than [`SHOWN_FOR`] ticks have passed since it
    /// was last shown.
    pub fn expire(&mut self, now: u128) {
        if self
            .shown_at
            .is_some_and(|start| now.saturating_sub(start) > SHOWN_FOR)
        {
            self.hide();
        }
    }

    /// Whether it is up, faded out or not.
    #[must_use]
    pub fn is_up(&self) -> bool {
        self.shown_at.is_some()
    }

    /// The level it is drawn at, in 32nds, at Mac tick `now`; `None` while
    /// it is down.
    #[must_use]
    pub fn level(&self, now: u128) -> Option<u8> {
        let start = self.shown_at?;
        let left = FADED_AT.saturating_sub(now.saturating_sub(start));
        Some(left.min(u128::from(FULL)) as u8)
    }

    /// The index of its scale in [`SCALES`].
    #[must_use]
    pub fn zoom(&self) -> usize {
        self.zoom
    }

    /// Steps to the next smaller scale, if there is one: whether it did.
    pub fn zoom_out(&mut self) -> bool {
        let moved = self.zoom > 0;
        if moved {
            self.zoom -= 1;
        }
        moved
    }

    /// Steps to the next larger scale, if there is one: whether it did.
    pub fn zoom_in(&mut self) -> bool {
        let moved = self.zoom + 1 < SCALES.len();
        if moved {
            self.zoom += 1;
        }
        moved
    }

    /// Whether the click boxes are the big ones: where the original's zoom
    /// is ≤ 0.5, the largest scale alone.
    #[must_use]
    pub fn big_boxes(&self) -> bool {
        ZOOMS[self.zoom] <= 0.5
    }
}

/// What a click hits on the floating map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// Outside the frame: no click of the map's.
    Outside,
    /// The current system's centre.
    Centre,
    /// A link's box.
    Link(SystemId),
    /// Anything else in the frame.
    Miss,
}

/// What a click at `at` hits on `map` in `frame`, with the current system
/// at `current` and its `links` at theirs, in order (see the module docs).
#[must_use]
pub fn hit(
    map: &FloatingMap,
    frame: Bounds,
    at: Point,
    current: Point,
    links: &[(SystemId, Point)],
) -> Hit {
    if !contains(frame, at) {
        return Hit::Outside;
    }
    let middle = centre(frame);
    let boxed = |x: f32, y: f32, half: f32| {
        contains(
            Bounds {
                min: Point::new(x - half, y - half),
                max: Point::new(x + half, y + half),
            },
            at,
        )
    };
    if boxed(middle.x, middle.y, BOX_HALF) {
        return Hit::Centre;
    }
    let zoom = ZOOMS[map.zoom];
    let half = if map.big_boxes() {
        BIG_BOX_HALF
    } else {
        BOX_HALF
    };
    let placed = |from: f32, to: f32, here: f32| {
        (f64::from(from) + (f64::from(to) - f64::from(here)) / zoom).trunc() as f32
    };
    links
        .iter()
        .find(|(_, position)| {
            boxed(
                placed(middle.x, position.x, current.x),
                placed(middle.y, position.y, current.y),
                half,
            )
        })
        .map_or(Hit::Miss, |&(link, _)| Hit::Link(link))
}

/// Draws the floating map in `frame` at `level` onto `list`: the border in
/// `border`, then what `map` draws, clipped and faded together
/// ([`clip_and_fade`]).
pub fn draw(
    list: &mut DrawList,
    frame: Bounds,
    level: u8,
    border: Color,
    map: impl FnOnce(&mut DrawList),
) {
    let mut raw = DrawList::new();
    let (left, top, right, bottom) = (frame.min.x, frame.min.y, frame.max.x, frame.max.y);
    let corners = [
        Point::new(left, top),
        Point::new(right, top),
        Point::new(right, bottom),
        Point::new(left, bottom),
    ];
    for (at, &from) in corners.iter().enumerate() {
        raw.line(from, corners[(at + 1) % 4], 1.0, border);
    }
    map(&mut raw);
    clip_and_fade(&raw, frame, level, list);
}

/// One line or dot of the map, clipped to the frame, with what is left of
/// it once the dots drawn after it have cut it.
enum Piece {
    /// A line from `from` to `to`, of which the stretches `kept` (as
    /// fractions of the way from `from`) are left.
    Line {
        from: Point,
        to: Point,
        width: f32,
        color: Color,
        kept: Vec<(f32, f32)>,
    },
    /// A dot, its square `whole` and the part of it in the frame, `area`,
    /// of which the rectangles `kept` are left: drawn as the dot it was
    /// while it is whole.
    Dot {
        center: Point,
        size: f32,
        color: Color,
        whole: Bounds,
        area: Bounds,
        kept: Vec<Bounds>,
    },
}

impl Piece {
    /// What it covers of the pieces drawn before it: a dot's area.
    fn cover(&self) -> Option<Bounds> {
        match self {
            Self::Dot { area, .. } => Some(*area),
            Self::Line { .. } => None,
        }
    }

    /// Cuts each of `covers` out of what is left of it.
    fn cut(&mut self, covers: &[Bounds]) {
        match self {
            Self::Line { from, to, kept, .. } => {
                for cover in covers {
                    if let Some(span) = span_in(*from, *to, *cover) {
                        *kept = kept.iter().flat_map(|&have| cut(have, span)).collect();
                    }
                }
            }
            Self::Dot { kept, .. } => {
                for cover in covers {
                    *kept = kept.iter().flat_map(|&have| minus(have, *cover)).collect();
                }
            }
        }
    }

    /// Draws what is left of it onto `out` at `level`.
    fn draw(self, level: u8, out: &mut DrawList) {
        match self {
            Self::Line {
                from,
                to,
                width,
                color,
                kept,
            } => {
                for (t0, t1) in kept {
                    out.line(
                        lerp(from, to, t0),
                        lerp(from, to, t1),
                        width,
                        fade(color, level),
                    );
                }
            }
            Self::Dot {
                center,
                size,
                color,
                whole,
                kept,
                ..
            } => {
                if kept == [whole] {
                    out.dot(center, size, fade(color, level));
                } else {
                    for area in kept {
                        fill_rect(out, area, fade(color, level));
                    }
                }
            }
        }
    }
}

/// Draws `map`'s lines and dots onto `out`, clipped to `frame` and faded
/// to `level` (see the module docs): each line cut to the frame, and
/// dropped when nothing is left; each dot whole when inside, its overlap
/// when partly inside, and dropped when outside; every other command
/// dropped. Each is then cut where a dot drawn after it covers it, a line
/// dropped where a later line has its ends and at least its width, and
/// every colour's alpha becomes round(alpha × level / 32). Level 0 draws
/// nothing.
pub fn clip_and_fade(map: &DrawList, frame: Bounds, level: u8, out: &mut DrawList) {
    if level == 0 {
        return;
    }
    let mut pieces = clipped(map, frame);
    for index in 0..pieces.len() {
        let (head, later) = pieces.split_at_mut(index + 1);
        let covers: Vec<Bounds> = later.iter().filter_map(Piece::cover).collect();
        head[index].cut(&covers);
    }
    for piece in pieces {
        piece.draw(level, out);
    }
}

/// `map`'s lines and dots clipped to `frame`, less each line a later line
/// with its ends and at least its width covers.
fn clipped(map: &DrawList, frame: Bounds) -> Vec<Piece> {
    let commands: Vec<&DrawCommand> = map.iter().collect();
    let mut pieces = Vec::new();
    for (index, command) in commands.iter().enumerate() {
        match **command {
            DrawCommand::Line {
                from,
                to,
                width,
                color,
            } => {
                let covered = commands[index + 1..].iter().any(|later| {
                    matches!(**later, DrawCommand::Line { from: f, to: t, width: w, .. }
                        if w >= width && ((f, t) == (from, to) || (f, t) == (to, from)))
                });
                if covered {
                    continue;
                }
                if let Some((t0, t1)) = span_in(from, to, frame) {
                    let along = |t: f32| lerp(from, to, t);
                    pieces.push(Piece::Line {
                        from: along(t0),
                        to: along(t1),
                        width,
                        color,
                        kept: vec![(0.0, 1.0)],
                    });
                }
            }
            DrawCommand::Dot {
                center,
                size,
                color,
            } => {
                let whole = dot_rect(center, size);
                if let Some(area) = intersection(whole, frame) {
                    pieces.push(Piece::Dot {
                        center,
                        size,
                        color,
                        whole,
                        area,
                        kept: vec![area],
                    });
                }
            }
            _ => {}
        }
    }
    pieces
}

/// `color` with its alpha scaled to `level` in 32nds, rounded.
fn fade(color: Color, level: u8) -> Color {
    let alpha = (u32::from(color.a) * u32::from(level) + u32::from(FULL) / 2) / u32::from(FULL);
    Color {
        a: alpha as u8,
        ..color
    }
}

/// The point `t` of the way from `from` to `to`: exactly `from` at 0 and
/// `to` at 1.
fn lerp(from: Point, to: Point, t: f32) -> Point {
    let mix = |a: f32, b: f32| a * (1.0 - t) + b * t;
    Point::new(mix(from.x, to.x), mix(from.y, to.y))
}

/// A dot's square.
fn dot_rect(center: Point, size: f32) -> Bounds {
    let half = size / 2.0;
    Bounds {
        min: Point::new(center.x - half, center.y - half),
        max: Point::new(center.x + half, center.y + half),
    }
}

/// Where `a` and `b` overlap, if they share any area.
fn intersection(a: Bounds, b: Bounds) -> Option<Bounds> {
    let min = Point::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
    let max = Point::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
    (min.x < max.x && min.y < max.y).then_some(Bounds { min, max })
}

/// What is left of `a` once `b` is taken out of it: up to four
/// rectangles, the strips above and below `b`, then those to its left and
/// right.
fn minus(a: Bounds, b: Bounds) -> Vec<Bounds> {
    let Some(inner) = intersection(a, b) else {
        return vec![a];
    };
    let strips = [
        (a.min, Point::new(a.max.x, inner.min.y)),
        (Point::new(a.min.x, inner.max.y), a.max),
        (
            Point::new(a.min.x, inner.min.y),
            Point::new(inner.min.x, inner.max.y),
        ),
        (
            Point::new(inner.max.x, inner.min.y),
            Point::new(a.max.x, inner.max.y),
        ),
    ];
    strips
        .into_iter()
        .map(|(min, max)| Bounds { min, max })
        .filter(|strip| strip.min.x < strip.max.x && strip.min.y < strip.max.y)
        .collect()
}

/// The stretch of the segment from `from` to `to` inside `area`, edges
/// included, as fractions of the way along it (Liang-Barsky); `None` when
/// it misses or only touches it at a point. A segment of no length inside
/// is all of it.
fn span_in(from: Point, to: Point, area: Bounds) -> Option<(f32, f32)> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let mut span = (0.0_f32, 1.0_f32);
    for (p, q) in [
        (-dx, from.x - area.min.x),
        (dx, area.max.x - from.x),
        (-dy, from.y - area.min.y),
        (dy, area.max.y - from.y),
    ] {
        match p.partial_cmp(&0.0) {
            Some(Ordering::Less) => span.0 = span.0.max(q / p),
            Some(Ordering::Greater) => span.1 = span.1.min(q / p),
            // Parallel to this edge: in or out along its whole length.
            _ => {
                if q < 0.0 {
                    return None;
                }
            }
        }
    }
    (span.0 < span.1).then_some(span)
}

/// What is left of the stretch `have` once `gone` is cut out of it: the
/// parts before and after, each only when it has length.
fn cut(have: (f32, f32), gone: (f32, f32)) -> Vec<(f32, f32)> {
    [(have.0, have.1.min(gone.0)), (have.0.max(gone.1), have.1)]
        .into_iter()
        .filter(|(t0, t1)| t0 < t1)
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ImageKey;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn stock() -> Bounds {
        frame((1024.0, 768.0))
    }

    // The frame.

    #[test]
    fn the_frame_is_a_quarter_of_the_width_square_210_in_from_the_right_at_150() {
        assert_eq!(stock(), Bounds::at(at(558.0, 150.0), 256.0, 256.0));
        assert_eq!(
            frame((640.0, 480.0)),
            Bounds::at(at(230.0, 150.0), 200.0, 200.0),
            "never smaller than 200"
        );
        assert_eq!(
            frame((1030.0, 768.0)),
            Bounds::at(at(563.0, 150.0), 257.0, 257.0),
            "trunc(257.5)"
        );
    }

    #[test]
    fn the_frame_holds_points_half_open() {
        let frame = stock();
        assert!(contains(frame, at(558.0, 150.0)));
        assert!(contains(frame, at(813.9, 405.9)));
        assert!(!contains(frame, at(814.0, 200.0)));
        assert!(!contains(frame, at(600.0, 406.0)));
        assert!(!contains(frame, at(557.9, 200.0)));
        assert!(!contains(frame, at(600.0, 149.9)));
    }

    #[test]
    fn the_centre_is_the_whole_middle() {
        assert_eq!(centre(stock()), at(686.0, 278.0));
        assert_eq!(
            centre(Bounds::at(at(1.0, 2.0), 4.0, 5.0)),
            at(3.0, 4.0),
            "(1 + 5) / 2, (2 + 7) / 2 rounded down"
        );
    }

    // The clock.

    #[test]
    fn the_level_is_full_to_118_ticks_then_fades_out_by_150() {
        let mut map = FloatingMap::new();
        assert_eq!(map.level(0), None, "down");
        map.show(1000);
        assert!(map.is_up());
        for (now, level) in [
            (1000, 32),
            (1118, 32),
            (1119, 31),
            (1135, 15),
            (1149, 1),
            (1150, 0),
            (1400, 0),
        ] {
            assert_eq!(map.level(now), Some(level), "{now}");
        }
        map.hide();
        assert!(!map.is_up());
        assert_eq!(map.level(1000), None);
    }

    #[test]
    fn it_hides_once_more_than_500_ticks_have_passed() {
        let mut map = FloatingMap::new();
        map.expire(10_000);
        assert!(!map.is_up(), "down stays down");
        map.show(1000);
        map.expire(1500);
        assert!(map.is_up(), "500 ticks: still up");
        map.expire(1501);
        assert!(!map.is_up(), "501");
    }

    #[test]
    fn the_default_is_the_new_map_down_at_16_9() {
        assert_eq!(FloatingMap::default(), FloatingMap::new());
        assert!(!FloatingMap::default().is_up());
        assert_eq!(FloatingMap::default().zoom(), START_ZOOM);
    }

    #[test]
    fn showing_again_restarts_the_clock() {
        let mut map = FloatingMap::new();
        map.show(0);
        map.show(400);
        assert_eq!(map.level(518), Some(32));
        map.expire(900);
        assert!(map.is_up());
        map.expire(901);
        assert!(!map.is_up());
    }

    // The zoom.

    #[test]
    fn the_zoom_starts_at_16_9_and_steps_within_the_seven_scales() {
        let mut map = FloatingMap::new();
        assert_eq!(map.zoom(), 5);
        assert_eq!(SCALES[map.zoom()], 16.0 / 9.0);
        for expected in [4, 3, 2, 1, 0] {
            assert!(map.zoom_out());
            assert_eq!(map.zoom(), expected);
            assert!(!map.big_boxes());
        }
        assert!(!map.zoom_out(), "the smallest");
        assert_eq!(map.zoom(), 0);
        for expected in 1..=6 {
            assert!(map.zoom_in());
            assert_eq!(map.zoom(), expected);
            assert_eq!(map.big_boxes(), expected == 6, "{expected}");
        }
        assert!(!map.zoom_in(), "the largest");
        assert_eq!(map.zoom(), 6);
    }

    #[test]
    fn the_original_zooms_are_the_scales_inverses() {
        for (zoom, scale) in ZOOMS.iter().zip(SCALES) {
            assert!((zoom * f64::from(scale) - 1.0).abs() < 1e-6, "{zoom}");
        }
        assert_eq!(ZOOMS[START_ZOOM], 0.5625);
    }

    #[test]
    fn the_transform_centres_the_current_system_at_the_zoom() {
        let transform = transform(stock(), at(40.0, -10.0), 2);
        assert_eq!(transform.world_to_screen(at(40.0, -10.0)), at(686.0, 278.0));
        assert_eq!(transform.world_to_screen(at(80.0, -10.0)), at(716.0, 278.0));
    }

    // The click.

    const NEAR: SystemId = SystemId(134);
    const NEXT: SystemId = SystemId(135);

    fn zoomed(index: usize) -> FloatingMap {
        let mut map = FloatingMap::new();
        while map.zoom() > index {
            map.zoom_out();
        }
        while map.zoom() < index {
            map.zoom_in();
        }
        map
    }

    #[test]
    fn the_centre_box_is_8_either_way_half_open() {
        let map = FloatingMap::new();
        let here = at(100.0, 100.0);
        for point in [at(678.0, 270.0), at(693.9, 285.9), at(686.0, 278.0)] {
            assert_eq!(
                hit(&map, stock(), point, here, &[]),
                Hit::Centre,
                "{point:?}"
            );
        }
        for point in [
            at(694.0, 278.0),
            at(686.0, 286.0),
            at(677.9, 278.0),
            at(686.0, 269.9),
            at(686.0, 200.0),
        ] {
            assert_eq!(hit(&map, stock(), point, here, &[]), Hit::Miss, "{point:?}");
        }
    }

    #[test]
    fn a_links_box_is_at_its_scaled_offset_truncated() {
        let map = FloatingMap::new();
        let here = at(10.0, 20.0);
        // 9 right and 18 down at 16/9: 16 and 32, so (702, 310).
        let links = [(NEAR, at(19.0, 38.0))];
        assert_eq!(
            hit(&map, stock(), at(694.0, 302.0), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(709.9, 317.9), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(710.0, 310.0), here, &links),
            Hit::Miss
        );
        assert_eq!(
            hit(&map, stock(), at(702.0, 318.0), here, &links),
            Hit::Miss
        );
        assert_eq!(
            hit(&map, stock(), at(693.9, 310.0), here, &links),
            Hit::Miss
        );
        // 9.5 right: 16.89, so trunc(702.89) = 702 and the box ends at 710.
        let links = [(NEAR, at(19.5, 38.0))];
        assert_eq!(
            hit(&map, stock(), at(709.9, 310.0), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(710.0, 310.0), here, &links),
            Hit::Miss
        );
        // 9.5 left: 686 − 16.89 = 669.11, truncated to 669.
        let links = [(NEAR, at(0.5, 20.0))];
        assert_eq!(
            hit(&map, stock(), at(661.0, 278.0), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(660.9, 278.0), here, &links),
            Hit::Miss
        );
    }

    #[test]
    fn the_boxes_follow_the_zoom_and_are_12_either_way_at_the_largest_scale() {
        let here = at(0.0, 0.0);
        let links = [(NEAR, at(48.0, 0.0))];
        // 48 × 3/4 = 36: (722, 278), ±8.
        let map = zoomed(2);
        assert_eq!(
            hit(&map, stock(), at(714.0, 278.0), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(713.9, 278.0), here, &links),
            Hit::Miss
        );
        // 48 × 64/27 = 113.78: (799, 278), ±12.
        let map = zoomed(6);
        assert_eq!(
            hit(&map, stock(), at(787.0, 278.0), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(786.9, 278.0), here, &links),
            Hit::Miss
        );
        assert_eq!(
            hit(&map, stock(), at(799.0, 289.9), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(799.0, 290.0), here, &links),
            Hit::Miss
        );
        // At 16/9 the box is ±8 again.
        let map = zoomed(5);
        let links = [(NEAR, at(45.0, 0.0))];
        assert_eq!(
            hit(&map, stock(), at(758.0, 286.0), here, &links),
            Hit::Miss
        );
        assert_eq!(
            hit(&map, stock(), at(758.0, 285.9), here, &links),
            Hit::Link(NEAR)
        );
    }

    #[test]
    fn the_centre_comes_first_then_the_first_link_whose_box_holds_the_point() {
        let map = FloatingMap::new();
        let here = at(0.0, 0.0);
        let links = [(NEAR, at(4.0, 0.0)), (NEXT, at(5.0, 0.0))];
        assert_eq!(
            hit(&map, stock(), at(690.0, 278.0), here, &links),
            Hit::Centre
        );
        assert_eq!(
            hit(&map, stock(), at(698.0, 278.0), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(701.0, 278.0), here, &links),
            Hit::Link(NEXT)
        );
        let links = [(NEXT, at(5.0, 0.0)), (NEAR, at(4.0, 0.0))];
        assert_eq!(
            hit(&map, stock(), at(698.0, 278.0), here, &links),
            Hit::Link(NEXT)
        );
    }

    #[test]
    fn a_point_outside_the_frame_is_outside_even_on_a_box() {
        let map = FloatingMap::new();
        let here = at(0.0, 0.0);
        let links = [(NEAR, at(72.0, 0.0))];
        assert_eq!(
            hit(&map, stock(), at(814.0, 278.0), here, &links),
            Hit::Outside
        );
        assert_eq!(
            hit(&map, stock(), at(813.0, 278.0), here, &links),
            Hit::Link(NEAR)
        );
        assert_eq!(
            hit(&map, stock(), at(600.0, 160.0), here, &links),
            Hit::Miss
        );
    }

    // The clip and fade.

    const RED: Color = Color::rgba(255, 0, 0, 255);
    const GREEN: Color = Color::rgba(0, 255, 0, 255);
    const BLUE: Color = Color::rgba(0, 0, 255, 200);

    fn passed(map: &DrawList, level: u8) -> Vec<DrawCommand> {
        let mut out = DrawList::new();
        clip_and_fade(map, stock(), level, &mut out);
        out.iter().cloned().collect()
    }

    fn line(from: Point, to: Point, width: f32, color: Color) -> DrawCommand {
        DrawCommand::Line {
            from,
            to,
            width,
            color,
        }
    }

    fn dot(center: Point, size: f32, color: Color) -> DrawCommand {
        DrawCommand::Dot {
            center,
            size,
            color,
        }
    }

    fn rect(min: (f32, f32), max: (f32, f32), color: Color) -> DrawCommand {
        let mut list = DrawList::new();
        fill_rect(
            &mut list,
            Bounds {
                min: at(min.0, min.1),
                max: at(max.0, max.1),
            },
            color,
        );
        list.iter().next().cloned().expect("one line")
    }

    fn list(commands: &[DrawCommand]) -> DrawList {
        let mut list = DrawList::new();
        for command in commands {
            list.push(command.clone());
        }
        list
    }

    #[test]
    fn a_line_inside_is_kept_and_one_outside_dropped() {
        let inside = line(at(600.0, 200.0), at(700.0, 300.0), 1.0, RED);
        let outside = line(at(100.0, 100.0), at(500.0, 120.0), 1.0, RED);
        let beside = line(at(820.0, 100.0), at(820.0, 500.0), 1.0, RED);
        let above = line(at(600.0, 140.0), at(700.0, 140.0), 1.0, RED);
        let below = line(at(600.0, 410.0), at(700.0, 410.0), 1.0, RED);
        let left = line(at(550.0, 200.0), at(550.0, 300.0), 1.0, RED);
        let corner = line(at(500.0, 100.0), at(558.0, 150.0), 1.0, RED);
        let point = line(at(600.0, 200.0), at(600.0, 200.0), 1.0, GREEN);
        assert_eq!(
            passed(
                &list(&[
                    inside.clone(),
                    outside,
                    beside,
                    above,
                    below,
                    left,
                    corner,
                    point.clone()
                ]),
                32
            ),
            [inside, point],
            "one only touching the frame's corner is dropped; a point inside is kept"
        );
    }

    #[test]
    fn a_line_crossing_one_edge_or_two_is_cut_to_the_frame() {
        let one = line(at(686.0, 278.0), at(942.0, 278.0), 1.0, RED);
        let two = line(at(430.0, 300.0), at(942.0, 300.0), 1.0, GREEN);
        let diagonal = line(at(686.0, 278.0), at(430.0, 22.0), 1.0, RED);
        let down = line(at(700.0, 100.0), at(700.0, 300.0), 1.0, RED);
        assert_eq!(
            passed(&list(&[one, two, diagonal, down]), 32),
            [
                line(at(686.0, 278.0), at(814.0, 278.0), 1.0, RED),
                line(at(558.0, 300.0), at(814.0, 300.0), 1.0, GREEN),
                line(at(686.0, 278.0), at(558.0, 150.0), 1.0, RED),
                line(at(700.0, 150.0), at(700.0, 300.0), 1.0, RED),
            ]
        );
    }

    #[test]
    fn a_dot_inside_is_kept_partly_inside_becomes_its_overlap_and_outside_dropped() {
        let inside = dot(at(600.0, 200.0), 6.0, RED);
        let edge = dot(at(812.0, 404.0), 8.0, GREEN);
        let outside = dot(at(820.0, 200.0), 6.0, RED);
        let touching = dot(at(817.0, 200.0), 6.0, RED);
        let below = dot(at(600.0, 409.0), 6.0, RED);
        assert_eq!(
            passed(&list(&[inside.clone(), edge, outside, touching, below]), 32),
            [inside, rect((808.0, 400.0), (814.0, 406.0), GREEN)],
            "one touching the frame's edge from outside is dropped"
        );
    }

    #[test]
    fn everything_but_lines_and_dots_is_dropped() {
        let mut map = DrawList::new();
        map.picture(ImageKey::picture(128), at(600.0, 200.0));
        map.text("Sol", at(600.0, 200.0), 10.0, None, RED);
        map.sprite(ImageKey::picture(129), at(600.0, 200.0), Color::WHITE);
        assert_eq!(passed(&map, 32), []);
    }

    #[test]
    fn the_level_scales_alpha_rounded_and_0_draws_nothing() {
        let map = list(&[
            line(at(600.0, 200.0), at(700.0, 300.0), 1.0, RED),
            dot(at(600.0, 300.0), 6.0, BLUE),
        ]);
        assert_eq!(
            passed(&map, 16),
            [
                line(
                    at(600.0, 200.0),
                    at(700.0, 300.0),
                    1.0,
                    Color { a: 128, ..RED }
                ),
                dot(at(600.0, 300.0), 6.0, Color { a: 100, ..BLUE }),
            ],
            "255 / 2 rounds up, 200 / 2"
        );
        assert_eq!(
            passed(&map, 31),
            [
                line(
                    at(600.0, 200.0),
                    at(700.0, 300.0),
                    1.0,
                    Color { a: 247, ..RED }
                ),
                dot(at(600.0, 300.0), 6.0, Color { a: 194, ..BLUE }),
            ],
            "247.03 and 193.75"
        );
        assert_eq!(
            fade(Color { a: 1, ..RED }, 16),
            Color { a: 1, ..RED },
            "0.5 up"
        );
        assert_eq!(
            fade(Color { a: 3, ..RED }, 5),
            Color { a: 0, ..RED },
            "0.47 down"
        );
        assert_eq!(passed(&map, 32), map.iter().cloned().collect::<Vec<_>>());
        assert_eq!(passed(&map, 0), []);
    }

    #[test]
    fn a_dot_drawn_over_another_cuts_it_to_the_ring_left() {
        // The current system's mark, then the outline, then the dot.
        let map = list(&[
            dot(at(686.0, 278.0), 14.0, GREEN),
            dot(at(686.0, 278.0), 8.0, BLUE),
            dot(at(686.0, 278.0), 6.0, RED),
        ]);
        let half = Color { a: 128, ..GREEN };
        let blue = Color { a: 100, ..BLUE };
        let red = Color { a: 128, ..RED };
        assert_eq!(
            passed(&map, 16),
            [
                rect((679.0, 271.0), (693.0, 274.0), half),
                rect((679.0, 282.0), (693.0, 285.0), half),
                rect((679.0, 274.0), (682.0, 282.0), half),
                rect((690.0, 274.0), (693.0, 282.0), half),
                rect((682.0, 274.0), (690.0, 275.0), blue),
                rect((682.0, 281.0), (690.0, 282.0), blue),
                rect((682.0, 275.0), (683.0, 281.0), blue),
                rect((689.0, 275.0), (690.0, 281.0), blue),
                dot(at(686.0, 278.0), 6.0, red),
            ]
        );
    }

    #[test]
    fn a_dot_wholly_under_a_later_one_is_dropped_and_one_beside_it_stays_whole() {
        let map = list(&[
            dot(at(600.0, 200.0), 6.0, RED),
            dot(at(600.0, 200.0), 8.0, BLUE),
            dot(at(620.0, 200.0), 6.0, GREEN),
        ]);
        assert_eq!(
            passed(&map, 32),
            [
                dot(at(600.0, 200.0), 8.0, BLUE),
                dot(at(620.0, 200.0), 6.0, GREEN),
            ]
        );
    }

    #[test]
    fn a_line_is_cut_where_a_later_dot_covers_it() {
        let map = list(&[
            line(at(600.0, 200.0), at(728.0, 200.0), 1.0, RED),
            dot(at(600.0, 200.0), 8.0, BLUE),
            dot(at(664.0, 200.0), 16.0, BLUE),
            dot(at(728.0, 200.0), 8.0, BLUE),
            dot(at(664.0, 300.0), 6.0, BLUE),
        ]);
        let cut: Vec<DrawCommand> = passed(&map, 32).into_iter().take(3).collect();
        assert_eq!(
            cut,
            [
                line(at(604.0, 200.0), at(656.0, 200.0), 1.0, RED),
                line(at(672.0, 200.0), at(724.0, 200.0), 1.0, RED),
                dot(at(600.0, 200.0), 8.0, BLUE),
            ]
        );
    }

    #[test]
    fn a_line_wholly_under_a_later_dot_is_dropped_and_one_after_every_dot_is_whole() {
        let map = list(&[
            line(at(600.0, 200.0), at(602.0, 200.0), 1.0, RED),
            dot(at(600.0, 200.0), 8.0, BLUE),
            line(at(600.0, 200.0), at(603.0, 200.0), 1.0, GREEN),
        ]);
        assert_eq!(
            passed(&map, 32),
            [
                dot(at(600.0, 200.0), 8.0, BLUE),
                line(at(600.0, 200.0), at(603.0, 200.0), 1.0, GREEN),
            ]
        );
    }

    #[test]
    fn a_line_under_a_later_one_with_its_ends_at_least_as_wide_is_dropped() {
        let (a, b) = (at(600.0, 200.0), at(700.0, 300.0));
        let map = list(&[
            line(a, b, 1.0, RED),
            line(b, a, 1.0, GREEN),
            line(a, b, 3.0, BLUE),
            line(a, b, 1.0, RED),
            line(a, at(700.0, 301.0), 3.0, GREEN),
        ]);
        assert_eq!(
            passed(&map, 32),
            [
                line(a, b, 3.0, BLUE),
                line(a, b, 1.0, RED),
                line(a, at(700.0, 301.0), 3.0, GREEN),
            ],
            "a wider line under a narrower one stays"
        );
    }

    #[test]
    fn the_draw_borders_the_frame_under_the_map_and_fades_both() {
        let mut out = DrawList::new();
        draw(&mut out, stock(), 16, GREEN, |map| {
            map.dot(at(600.0, 200.0), 6.0, RED);
            map.picture(ImageKey::picture(128), at(600.0, 200.0));
        });
        let border = Color { a: 128, ..GREEN };
        assert_eq!(
            out.iter().cloned().collect::<Vec<_>>(),
            [
                line(at(558.0, 150.0), at(814.0, 150.0), 1.0, border),
                line(at(814.0, 150.0), at(814.0, 406.0), 1.0, border),
                line(at(814.0, 406.0), at(558.0, 406.0), 1.0, border),
                line(at(558.0, 406.0), at(558.0, 150.0), 1.0, border),
                dot(at(600.0, 200.0), 6.0, Color { a: 128, ..RED }),
            ]
        );
    }
}
