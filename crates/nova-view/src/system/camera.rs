//! The system view's camera: which part of the system the screen shows.
//!
//! Nova measures space in screen pixels (the Bible: "Jump Distance 1000
//! pixels"; a weapon's speed is "pixels per frame"), and a `spöb`'s
//! position is in the same units with (0, 0) at the centre of the system.
//! The logical space is Nova's own 1024 x 768 screen, so one world unit is
//! one logical unit ([`WORLD_SCALE`]): distances and speeds read from the
//! data need no conversion. y grows downwards, as on screen.
//!
//! The camera keeps its world point drawn at its anchor, the screen point
//! its [`Anchor`] picks:
//!
//! - [`Anchor::ViewCentre`], every camera's default: the view's centre,
//!   (floor(view width / 2), view height / 2), [`VIEW_CENTER`] in Nova's
//!   own 1024 x 768 ([`VIEW_SIZE`]), and (682, 384) widened to 1365 x 768
//!   ([`Camera::centred_on_in`]). The system viewer draws from it.
//! - [`Anchor::PlayAreaCentre`]: the centre of the play area left of the
//!   status bar, (trunc((width − 194) / 2), trunc(height / 2)), (415, 384)
//!   at 1024 x 768: `_screenCenter`, which `_SetupWind` sets
//!   (@0x1b720-0x1b78a) and the original draws all of flight's world
//!   from. Flight draws from it by
//!   [`RuleKey::FlightAnchor`](nova_sim::RuleKey::FlightAnchor)'s engine
//!   reading, from the view's centre by the other.
//!
//! Either way:
//!
//! - `world_to_screen(p) = anchor + (p - center) x WORLD_SCALE`
//! - `screen_to_world(s) = center + (s - anchor) / WORLD_SCALE`
//! - a sprite placed as the original places one, at whole pixels from
//!   the center: [`Camera::sprite_top_left`] and [`Camera::sprite_centre`]

use nova_sim::navigation::SpriteRect;
use nova_sim::{RuleSource, Vec2};

use crate::Point;
use crate::geometry::Bounds;

/// The view's size in logical units: the whole screen.
pub const VIEW_SIZE: (f32, f32) = (1024.0, 768.0);
/// The screen point the camera's centre is drawn at.
pub const VIEW_CENTER: Point = Point::new(VIEW_SIZE.0 / 2.0, VIEW_SIZE.1 / 2.0);
/// Logical units per world unit: Nova's own scale.
pub const WORLD_SCALE: f32 = 1.0;
/// How fast a held movement key moves the camera, in world units a second
/// on each axis: about a screen's width a second.
pub const CAMERA_SPEED: f32 = 960.0;
/// How far past the system's outermost stellars the camera may go, in
/// world units.
pub const CAMERA_MARGIN: f32 = 1024.0;
/// The status bar's width (0xc2): the stock bars'. `_SetupWind` takes it
/// from the screen's to find the play area's centre (@0x1b720-0x1b78a);
/// flight takes it for the play area's right edge, the strip at the right
/// that takes no clicks, and the bar's width when its background cannot be
/// read. The one definition, so the anchor and the play area agree.
pub const STATUS_BAR_WIDTH: f32 = 194.0;

/// The screen point a [`Camera`] draws its centre at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anchor {
    /// The view's centre, (floor(width / 2), height / 2).
    #[default]
    ViewCentre,
    /// The play area's centre left of the status bar, (trunc((width −
    /// 194) × 0.5), trunc(height × 0.5)): the original's `_screenCenter`
    /// (`_SetupWind` @0x1b720-0x1b78a, 0.5 at 0xdd128).
    PlayAreaCentre,
}

impl Anchor {
    /// The anchor flight draws from by `source`'s reading of
    /// [`RuleKey::FlightAnchor`](nova_sim::RuleKey::FlightAnchor): the
    /// play area's centre by the original's, the view's by the other.
    #[must_use]
    pub fn in_flight(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::PlayAreaCentre,
            RuleSource::Bible => Self::ViewCentre,
        }
    }

    /// The anchor's screen point in a view `view` logical units in size.
    #[must_use]
    pub fn point(self, view: (f32, f32)) -> Point {
        match self {
            Self::ViewCentre => Point::new((view.0 / 2.0).floor(), view.1 / 2.0),
            Self::PlayAreaCentre => Point::new(
                ((view.0 - STATUS_BAR_WIDTH) * 0.5).trunc(),
                (view.1 * 0.5).trunc(),
            ),
        }
    }
}

/// What the system view shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// The world point drawn at the view's [`anchor`](Camera::anchor).
    center: Point,
    /// Where `center` may go.
    limits: Bounds,
    /// The view's size, in logical units.
    view: (f32, f32),
    /// Where on the view `center` is drawn.
    anchor: Anchor,
}

impl Camera {
    /// A camera on the system's centre, (0, 0), kept within `limits`,
    /// which must include (0, 0), in Nova's own view ([`VIEW_SIZE`]).
    #[must_use]
    pub fn new(limits: Bounds) -> Self {
        Self {
            center: Point::default(),
            limits,
            view: VIEW_SIZE,
            anchor: Anchor::ViewCentre,
        }
    }

    /// A camera on world point `center`, held there, in Nova's own view
    /// ([`VIEW_SIZE`]).
    #[must_use]
    pub fn centred_on(center: Point) -> Self {
        Self::centred_on_in(center, VIEW_SIZE)
    }

    /// A camera on world point `center`, held there, in a view `view`
    /// logical units in size: what the flight screen shows, following the
    /// player's ship, widened or not.
    #[must_use]
    pub fn centred_on_in(center: Point, view: (f32, f32)) -> Self {
        Self {
            center,
            limits: Bounds {
                min: center,
                max: center,
            },
            view,
            anchor: Anchor::ViewCentre,
        }
    }

    /// The camera drawing its centre at `anchor` instead.
    #[must_use]
    pub fn with_anchor(self, anchor: Anchor) -> Self {
        Self { anchor, ..self }
    }

    /// The world point drawn at the [`anchor`](Camera::anchor).
    #[must_use]
    pub fn center(&self) -> Point {
        self.center
    }

    /// The view's size, in logical units.
    #[must_use]
    pub fn view_size(&self) -> (f32, f32) {
        self.view
    }

    /// The screen point [`Camera::center`] is drawn at, as its [`Anchor`]
    /// picks: by default (floor(width / 2), height / 2), [`VIEW_CENTER`]
    /// in Nova's own view.
    #[must_use]
    pub fn anchor(&self) -> Point {
        self.anchor.point(self.view)
    }

    /// Where the world point `p` is drawn.
    #[must_use]
    pub fn world_to_screen(&self, p: Point) -> Point {
        let anchor = self.anchor();
        Point::new(
            (p.x - self.center.x).mul_add(WORLD_SCALE, anchor.x),
            (p.y - self.center.y).mul_add(WORLD_SCALE, anchor.y),
        )
    }

    /// The world point drawn at screen point `s`.
    #[must_use]
    pub fn screen_to_world(&self, s: Point) -> Point {
        let per_logical_unit = WORLD_SCALE.recip();
        let anchor = self.anchor();
        Point::new(
            (s.x - anchor.x).mul_add(per_logical_unit, self.center.x),
            (s.y - anchor.y).mul_add(per_logical_unit, self.center.y),
        )
    }

    /// Where a sprite whose frame is `size` (width, height) has its top
    /// left when it stands at world point `at`, placed as the original
    /// places one: `trunc(at - center) + anchor - size / 2` on each axis,
    /// the offset truncated toward zero and the half size halved toward
    /// zero, so an odd size's extra pixel falls right and below. The rule
    /// is `nova_sim::story::sprite_pool::offset`, not forked here.
    ///
    /// `_HandleExplods` places an explosion this way (@0x2f6a1-0x2f73c),
    /// as `_HandleStellarSprites` (@0x2eeab-0x2ef1a) places stellars and
    /// `_HandleShipDisplay` (@0x2cd17-0x2d083) a ship's hull and each of
    /// its layers, by its own frame's size; the scene and flight draw
    /// them all through this helper, and box a ship by
    /// [`Camera::sprite_rect`].
    ///
    /// The original takes the player's position at the step; flight's
    /// camera follows the player's shown position, interpolated between
    /// steps and equal to the step's at alpha 0. Taking the camera's
    /// center lets the whole-pixel offset move with the drawn world between
    /// steps rather than jump a pixel against it; no frame boundary sees
    /// the difference.
    #[must_use]
    pub fn sprite_top_left(&self, at: Point, size: (u32, u32)) -> Point {
        let (dx, dy) = nova_sim::story::sprite_pool::offset(
            Vec2::new(at.x, at.y),
            Vec2::new(self.center.x, self.center.y),
            size,
        );
        let anchor = self.anchor();
        // A screen offset is far within an `f32`'s whole numbers.
        #[allow(clippy::cast_precision_loss)]
        Point::new(
            (dx as f32).mul_add(WORLD_SCALE, anchor.x),
            (dy as f32).mul_add(WORLD_SCALE, anchor.y),
        )
    }

    /// The point a sprite placed by [`Camera::sprite_top_left`] is
    /// centred on, as [`DrawCommand::Sprite`](crate::DrawCommand::Sprite)
    /// takes it: the top left plus half the size.
    #[must_use]
    pub fn sprite_centre(&self, at: Point, size: (u32, u32)) -> Point {
        Self::centred(self.sprite_top_left(at, size), size)
    }

    /// The centre of a frame of `size` whose top left is `top_left`.
    fn centred(top_left: Point, size: (u32, u32)) -> Point {
        // A sprite's frame is far within an `f32`'s whole numbers.
        #[allow(clippy::cast_precision_loss)]
        Point::new(
            top_left.x + size.0 as f32 / 2.0,
            top_left.y + size.1 as f32 / 2.0,
        )
    }

    /// The whole-pixel point of world point `at`: `trunc(at - center) +
    /// anchor` on each axis, a sprite's placement before any size comes off
    /// it, so [`Camera::sprite_top_left`] of a frame of no size.
    /// `_HandleWeapTargetSprites` centres a ship's brackets here
    /// (@0x30060-0x30085), with no size in it, so an odd-sized ship's
    /// bracket centre is half a pixel up and left of its sprite's middle.
    /// A sprite with no sheet, which the original doesn't have, is placed
    /// here too.
    #[must_use]
    pub fn whole_point(&self, at: Point) -> Point {
        self.sprite_top_left(at, (0, 0))
    }

    /// The point a shot whose frame is `size` is centred on: `_HandleShot`
    /// takes half the frame's width from both axes of its offset and never
    /// asks its height (@0x35c66-0x35cc0, `_GetSpriteYSize` is not called),
    /// so a frame taller than wide sits below the standard rule's place,
    /// one wider than tall above it.
    #[must_use]
    pub fn shot_centre(&self, at: Point, size: (u32, u32)) -> Point {
        Self::centred(self.sprite_top_left(at, (size.0, size.0)), size)
    }

    /// The point a cargo box whose frame is `size` is centred on:
    /// `_HandleBoxes` takes the whole size from its offset, not half
    /// (@0x2e1c2-0x2e250), so the frame's bottom right is the
    /// [whole point](Camera::whole_point) of the box's position.
    #[must_use]
    pub fn box_centre(&self, at: Point, size: (u32, u32)) -> Point {
        let corner = self.whole_point(at);
        // A sprite's frame is far within an `f32`'s whole numbers.
        #[allow(clippy::cast_precision_loss)]
        let top_left = Point::new(corner.x - size.0 as f32, corner.y - size.1 as f32);
        Self::centred(top_left, size)
    }

    /// The point the nav brackets of a stellar at `at` are centred on:
    /// `_HandleNavTargetSprites` truncates the player alone, so `at -
    /// trunc(center) + anchor` (@0x2fdab-0x2fddb). A stellar's position is
    /// whole, and a fractional center leaves this a pixel from the
    /// stellar's own [whole point](Camera::whole_point) on the side the
    /// truncations part, as in the original.
    #[must_use]
    pub fn nav_centre(&self, at: Point) -> Point {
        let anchor = self.anchor();
        Point::new(
            (at.x - self.center.x.trunc()).mul_add(WORLD_SCALE, anchor.x),
            (at.y - self.center.y.trunc()).mul_add(WORLD_SCALE, anchor.y),
        )
    }

    /// The centre of the pixel a beam's end at `at` is plotted on:
    /// `_HandleBeams` truncates the whole screen coordinate, `trunc(at -
    /// center + anchor)` (@0x30927-0x30982, @0x309df-0x30a4a,
    /// @0x31dc8-0x31de4), not the offset, so for a point on screen a
    /// negative offset rounds away from the anchor, unlike
    /// [`Camera::whole_point`]. It takes 0.5 on each axis because a line
    /// of width 1 is centred on its ends: a whole end would straddle two
    /// rows of pixels.
    #[must_use]
    pub fn pixel_centre(&self, at: Point) -> Point {
        let screen = self.world_to_screen(at);
        Point::new(screen.x.trunc() + 0.5, screen.y.trunc() + 0.5)
    }

    /// The rect, in world coordinates, a sprite whose frame is `size` is
    /// drawn in when it stands at world point `at`: its top left the world
    /// point of [`Camera::sprite_top_left`], its sides `size`. A ship's
    /// click and hover box (`nova_sim::DrawnBox`) is this rect, so the box
    /// and the drawn sprite can't disagree.
    #[must_use]
    pub fn sprite_rect(&self, at: Point, size: (u32, u32)) -> SpriteRect {
        self.rect_at(self.sprite_top_left(at, size), size)
    }

    /// The rect, in world coordinates, a sprite whose frame is `size` is
    /// placed in at `offset` from the camera's center: the offset truncated
    /// toward zero by the same rule as [`Camera::sprite_top_left`]'s, never
    /// rebuilt as a world point first, which could lose the whole number.
    /// A ship boxed where its slot was last laid out reads it
    /// (`nova_sim::HoverPlace::Stale`).
    #[must_use]
    pub fn sprite_rect_from(&self, offset: Vec2, size: (u32, u32)) -> SpriteRect {
        let (dx, dy) = nova_sim::story::sprite_pool::offset(offset, Vec2::ZERO, size);
        let anchor = self.anchor();
        // A screen offset is far within an `f32`'s whole numbers.
        #[allow(clippy::cast_precision_loss)]
        let top_left = Point::new(
            (dx as f32).mul_add(WORLD_SCALE, anchor.x),
            (dy as f32).mul_add(WORLD_SCALE, anchor.y),
        );
        self.rect_at(top_left, size)
    }

    /// The world rect of `size` whose top left is drawn at screen point
    /// `top_left`.
    fn rect_at(&self, top_left: Point, (width, height): (u32, u32)) -> SpriteRect {
        let world = self.screen_to_world(top_left);
        SpriteRect {
            left: world.x,
            top: world.y,
            width,
            height,
        }
    }

    /// Moves the camera `dx`, `dy` world units (so the system moves the
    /// other way on screen), stopping at the limits.
    pub fn move_by(&mut self, dx: f32, dy: f32) {
        let moved = Point::new(self.center.x + dx, self.center.y + dy);
        self.center = self.limits.clamp(moved);
    }

    /// Moves the camera back to the system's centre.
    pub fn recentre(&mut self) {
        self.center = Point::default();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// Sol's stellars' box, grown by the margin.
    fn sol() -> Camera {
        let limits = Bounds {
            min: at(-1700.0, -600.0),
            max: at(10_000.0, 1400.0),
        };
        Camera::new(limits.grown(CAMERA_MARGIN))
    }

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() <= 1e-3 && (a.y - b.y).abs() <= 1e-3
    }

    #[test]
    fn the_view_is_novas_screen_at_one_logical_unit_per_world_unit() {
        assert_eq!(VIEW_SIZE, (1024.0, 768.0));
        assert_eq!(VIEW_CENTER, at(512.0, 384.0));
        assert_eq!(WORLD_SCALE, 1.0);
    }

    #[test]
    fn it_starts_on_the_systems_centre_drawn_at_the_screens_centre() {
        let camera = sol();
        assert_eq!(camera.center(), at(0.0, 0.0));
        assert_eq!(camera.world_to_screen(at(0.0, 0.0)), at(512.0, 384.0));
        // Mars, up and to the right of Earth.
        assert_eq!(
            camera.world_to_screen(at(900.0, -600.0)),
            at(1412.0, -216.0)
        );
        assert_eq!(camera.screen_to_world(at(512.0, 384.0)), at(0.0, 0.0));
        assert_eq!(camera.screen_to_world(at(0.0, 0.0)), at(-512.0, -384.0));
    }

    #[test]
    fn a_move_shifts_every_drawn_point_by_minus_the_move() {
        let mut camera = sol();
        let points = [at(0.0, 0.0), at(900.0, -600.0), at(-1700.0, 1400.0)];
        let before: Vec<Point> = points.iter().map(|&p| camera.world_to_screen(p)).collect();
        camera.move_by(250.0, -120.0);
        assert_eq!(camera.center(), at(250.0, -120.0));
        for (&p, b) in points.iter().zip(&before) {
            assert_eq!(
                camera.world_to_screen(p),
                at(b.x - 250.0, b.y + 120.0),
                "{p:?}"
            );
        }
    }

    #[test]
    fn screen_and_world_round_trip_after_moves() {
        let mut camera = sol();
        for (dx, dy) in [(0.0, 0.0), (333.3, -71.25), (-1234.5, 987.6)] {
            camera.move_by(dx, dy);
            for p in [at(0.0, 0.0), at(900.0, -600.0), at(13.25, 7.5)] {
                let back = camera.screen_to_world(camera.world_to_screen(p));
                assert!(close(back, p), "{p:?} came back as {back:?}");
            }
            for s in [at(0.0, 0.0), at(1024.0, 768.0), at(100.5, 700.25)] {
                let back = camera.world_to_screen(camera.screen_to_world(s));
                assert!(close(back, s), "{s:?} came back as {back:?}");
            }
        }
    }

    #[test]
    fn moving_stops_at_each_edge_of_the_limits() {
        let mut camera = sol();
        for _ in 0..100 {
            camera.move_by(500.0, 500.0);
        }
        assert_eq!(camera.center(), at(11_024.0, 2424.0));
        for _ in 0..100 {
            camera.move_by(-500.0, -500.0);
        }
        assert_eq!(camera.center(), at(-2724.0, -1624.0));
        // Partway along one axis, held on the other.
        camera.move_by(100.0, -100.0);
        assert_eq!(camera.center(), at(-2624.0, -1624.0));
    }

    #[test]
    fn a_camera_centred_on_a_point_draws_it_at_the_screens_centre_and_stays() {
        let mut camera = Camera::centred_on(at(-250.5, 1200.25));
        assert_eq!(camera.center(), at(-250.5, 1200.25));
        assert_eq!(camera.world_to_screen(at(-250.5, 1200.25)), VIEW_CENTER);
        assert_eq!(camera.world_to_screen(at(0.0, 0.0)), at(762.5, -816.25));
        camera.move_by(100.0, -100.0);
        assert_eq!(camera.center(), at(-250.5, 1200.25), "held there");
    }

    #[test]
    fn a_camera_in_a_widened_view_draws_its_centre_at_the_views_centre() {
        let p = at(-250.5, 1200.25);
        let camera = Camera::centred_on_in(p, (1365.0, 768.0));
        assert_eq!(camera.view_size(), (1365.0, 768.0));
        assert_eq!(camera.anchor(), at(682.0, 384.0));
        assert_eq!(camera.world_to_screen(p), at(682.0, 384.0));
        assert_eq!(
            camera.world_to_screen(at(p.x - 600.0, p.y)),
            at(82.0, 384.0),
            "600 left of the ship, inside the widened view"
        );
        assert_eq!(camera.screen_to_world(at(682.0, 384.0)), p);
        for s in [at(0.0, 0.0), at(1365.0, 768.0), at(1100.5, 700.25)] {
            let back = camera.world_to_screen(camera.screen_to_world(s));
            assert!(close(back, s), "{s:?} came back as {back:?}");
        }
        let even = Camera::centred_on_in(p, (1366.0, 768.0));
        assert_eq!(even.world_to_screen(p), at(683.0, 384.0));
    }

    #[test]
    fn the_classic_cameras_keep_nova_s_screen() {
        let p = at(10.0, 20.0);
        assert_eq!(Camera::centred_on(p).view_size(), VIEW_SIZE);
        assert_eq!(Camera::centred_on(p).anchor(), VIEW_CENTER);
        assert_eq!(Camera::centred_on(p), Camera::centred_on_in(p, VIEW_SIZE));
        assert_eq!(sol().view_size(), VIEW_SIZE);
    }

    #[test]
    fn the_play_area_anchor_is_the_screen_centre_left_of_the_status_bar() {
        let p = at(-250.5, 1200.25);
        for (view, anchor) in [
            ((1024.0, 768.0), at(415.0, 384.0)),
            ((1365.0, 768.0), at(585.0, 384.0)),
            // 1367 − 194 = 1173: trunc(586.5).
            ((1367.0, 768.0), at(586.0, 384.0)),
            // An odd height: trunc(384.5).
            ((1024.0, 769.0), at(415.0, 384.0)),
        ] {
            let camera = Camera::centred_on_in(p, view).with_anchor(Anchor::PlayAreaCentre);
            assert_eq!(camera.anchor(), anchor, "{view:?}");
            assert_eq!(Anchor::PlayAreaCentre.point(view), anchor, "{view:?}");
            assert_eq!(camera.world_to_screen(p), anchor, "{view:?}");
            assert_eq!(camera.screen_to_world(anchor), p, "{view:?}");
            for s in [at(0.0, 0.0), at(view.0, view.1), at(100.5, 700.25)] {
                let back = camera.world_to_screen(camera.screen_to_world(s));
                assert!(close(back, s), "{s:?} came back as {back:?}");
            }
        }
    }

    #[test]
    fn the_view_centre_anchor_is_every_camera_s_default() {
        let p = at(10.0, 20.0);
        for view in [(1024.0, 768.0), (1365.0, 768.0), (1024.0, 769.0)] {
            let camera = Camera::centred_on_in(p, view);
            assert_eq!(camera.with_anchor(Anchor::ViewCentre), camera);
            assert_eq!(camera.anchor(), Anchor::ViewCentre.point(view));
        }
        assert_eq!(
            Anchor::ViewCentre.point((1024.0, 769.0)),
            at(512.0, 384.5),
            "as before: half the height, untruncated"
        );
        assert_eq!(sol().with_anchor(Anchor::ViewCentre), sol());
    }

    #[test]
    fn flight_takes_the_play_area_anchor_by_the_original_and_the_view_centre_otherwise() {
        assert_eq!(
            Anchor::in_flight(RuleSource::Engine),
            Anchor::PlayAreaCentre
        );
        assert_eq!(Anchor::in_flight(RuleSource::Bible), Anchor::ViewCentre);
    }

    #[test]
    fn a_play_area_anchored_camera_moves_and_draws_as_a_centred_one_does() {
        let mut camera = sol().with_anchor(Anchor::PlayAreaCentre);
        camera.move_by(250.0, -120.0);
        assert_eq!(camera.center(), at(250.0, -120.0));
        assert_eq!(camera.world_to_screen(at(250.0, -120.0)), at(415.0, 384.0));
        assert_eq!(camera.world_to_screen(at(350.0, -100.0)), at(515.0, 404.0));
        assert_eq!(camera.view_size(), VIEW_SIZE);
    }

    #[test]
    fn recentring_returns_to_the_systems_centre() {
        let mut camera = sol();
        camera.move_by(4000.0, -900.0);
        camera.recentre();
        assert_eq!(camera.center(), at(0.0, 0.0));
        assert_eq!(camera.world_to_screen(at(0.0, 0.0)), VIEW_CENTER);
        camera.move_by(-50.0, 25.0);
        assert_eq!(camera.center(), at(-50.0, 25.0), "the limits are kept");
    }

    /// A camera on (100, 200) drawing it at `anchor`.
    fn on_100_200(anchor: Anchor) -> Camera {
        Camera::centred_on(at(100.0, 200.0)).with_anchor(anchor)
    }

    #[test]
    fn a_sprites_top_left_is_its_offset_truncated_toward_zero_less_half_its_size() {
        let camera = on_100_200(Anchor::PlayAreaCentre);
        assert_eq!(camera.anchor(), at(415.0, 384.0));
        // (10.7, -4.6) truncates to (10, -4); a 7 x 5 frame's half is (3, 2).
        assert_eq!(
            camera.sprite_top_left(at(110.7, 195.4), (7, 5)),
            at(422.0, 378.0)
        );
        // (-10.7, 4.6) truncates to (-10, 4), toward zero, not floored.
        assert_eq!(
            camera.sprite_top_left(at(89.3, 204.6), (7, 5)),
            at(402.0, 386.0)
        );
    }

    #[test]
    fn a_sprites_top_left_is_taken_from_the_cameras_anchor() {
        let camera = on_100_200(Anchor::ViewCentre);
        assert_eq!(
            camera.sprite_top_left(at(110.7, 195.4), (7, 5)),
            at(519.0, 378.0)
        );
        assert_eq!(
            camera.sprite_top_left(at(89.3, 204.6), (7, 5)),
            at(499.0, 386.0)
        );
    }

    #[test]
    fn a_sprites_centre_draws_its_rect_from_the_whole_pixel_top_left() {
        let camera = on_100_200(Anchor::PlayAreaCentre);
        let centre = camera.sprite_centre(at(110.7, 195.4), (7, 5));
        assert_eq!(centre, at(425.5, 380.5));
        // The rect a sprite centred there draws starts at the top left.
        assert_eq!(at(centre.x - 3.5, centre.y - 2.5), at(422.0, 378.0));
        assert_eq!(
            camera.sprite_centre(at(89.3, 204.6), (8, 6)),
            at(405.0, 388.0)
        );
    }

    /// A camera on a fractional point, drawing it at the play area's
    /// centre.
    fn on_fraction() -> Camera {
        Camera::centred_on(at(100.25, 200.75)).with_anchor(Anchor::PlayAreaCentre)
    }

    fn top_left(rect: SpriteRect) -> Point {
        at(rect.left, rect.top)
    }

    #[test]
    fn a_sprites_rect_is_its_placed_top_left_in_the_world_at_its_size() {
        let camera = on_fraction();
        for (p, size) in [
            (at(110.7, 195.4), (7, 6)),
            (at(89.3, 204.6), (8, 5)),
            (at(110.7, 204.6), (7, 6)),
            (at(89.3, 195.4), (8, 5)),
        ] {
            let rect = camera.sprite_rect(p, size);
            assert_eq!(
                top_left(rect),
                camera.screen_to_world(camera.sprite_top_left(p, size)),
                "{p:?} {size:?}"
            );
            assert_eq!((rect.width, rect.height), size, "{p:?} {size:?}");
            // Its centre is the world point the sprite is centred on.
            #[allow(clippy::cast_precision_loss)]
            let centre = at(
                rect.left + size.0 as f32 / 2.0,
                rect.top + size.1 as f32 / 2.0,
            );
            assert_eq!(
                centre,
                camera.screen_to_world(camera.sprite_centre(p, size)),
                "{p:?} {size:?}"
            );
        }
    }

    #[test]
    fn a_sprites_rect_truncates_toward_zero_with_the_odd_pixel_right_and_below() {
        let camera = on_fraction();
        let screen = |rect: SpriteRect| camera.world_to_screen(top_left(rect));
        // (10.45, -5.35) truncates to (10, -5); 7 x 6 halves to (3, 3).
        assert_eq!(
            screen(camera.sprite_rect(at(110.7, 195.4), (7, 6))),
            at(422.0, 376.0)
        );
        // (-10.95, 3.85) truncates to (-10, 3), toward zero; 8 x 5 halves
        // to (4, 2).
        assert_eq!(
            screen(camera.sprite_rect(at(89.3, 204.6), (8, 5))),
            at(401.0, 385.0)
        );
        // The axes are not swapped: an even × odd frame is placed apart
        // from an odd × even one.
        assert_ne!(
            camera.sprite_rect(at(110.7, 195.4), (7, 6)),
            camera.sprite_rect(at(110.7, 195.4), (6, 7))
        );
        assert_eq!(
            screen(camera.sprite_rect(at(110.7, 195.4), (6, 7))),
            at(422.0, 376.0)
        );
        assert_eq!(
            screen(camera.sprite_rect(at(110.7, 195.4), (9, 4))),
            at(421.0, 377.0)
        );
    }

    #[test]
    fn a_sprites_rect_from_a_whole_offset_keeps_that_offset() {
        let camera = on_fraction();
        let screen = |rect: SpriteRect| camera.world_to_screen(top_left(rect));
        // 30 right of and 12 above the player: 7 x 6 halves to (3, 3).
        let rect = camera.sprite_rect_from(Vec2::new(30.0, -12.0), (7, 6));
        assert_eq!(screen(rect), at(442.0, 369.0));
        assert_eq!((rect.width, rect.height), (7, 6));
    }

    #[test]
    fn a_sprites_rect_from_a_fractional_offset_truncates_it_toward_zero() {
        let camera = on_fraction();
        let screen = |rect: SpriteRect| camera.world_to_screen(top_left(rect));
        assert_eq!(
            screen(camera.sprite_rect_from(Vec2::new(30.9, -12.9), (7, 6))),
            at(442.0, 369.0)
        );
        assert_eq!(
            screen(camera.sprite_rect_from(Vec2::new(-30.9, 12.9), (8, 5))),
            at(381.0, 394.0)
        );
    }

    #[test]
    fn a_whole_point_is_the_offset_truncated_toward_zero_from_the_anchor() {
        let camera = on_fraction();
        assert_eq!(camera.whole_point(at(110.7, 195.4)), at(425.0, 379.0));
        assert_eq!(camera.whole_point(at(89.3, 204.6)), at(405.0, 387.0));
    }

    #[test]
    fn a_shot_is_centred_from_a_top_left_its_width_places_on_both_axes() {
        let camera = on_fraction();
        // The top left is (422, 376): 7 halved on both axes, not 12.
        assert_eq!(
            camera.shot_centre(at(110.7, 195.4), (7, 12)),
            at(425.5, 382.0)
        );
        // 12 halved on both: the top left is (419, 373).
        assert_eq!(
            camera.shot_centre(at(110.7, 195.4), (12, 7)),
            at(425.0, 376.5)
        );
    }

    #[test]
    fn a_box_is_centred_up_and_left_of_its_point_by_half_its_size() {
        // Its bottom right is the point, so its top left is (418, 374).
        assert_eq!(
            on_fraction().box_centre(at(110.7, 195.4), (7, 5)),
            at(421.5, 376.5)
        );
    }

    #[test]
    fn a_nav_centre_takes_the_center_truncated_alone() {
        let camera = on_fraction();
        assert_eq!(camera.nav_centre(at(300.0, -200.0)), at(615.0, -16.0));
        assert_eq!(camera.whole_point(at(300.0, -200.0)), at(614.0, -16.0));
        let behind = Camera::centred_on(at(-100.25, -200.75)).with_anchor(Anchor::PlayAreaCentre);
        assert_eq!(behind.nav_centre(at(-300.0, -400.0)), at(215.0, 184.0));
        assert_eq!(behind.whole_point(at(-300.0, -400.0)), at(216.0, 185.0));
    }

    #[test]
    fn a_pixel_centre_truncates_the_whole_screen_coordinate() {
        let camera = on_fraction();
        assert_eq!(camera.pixel_centre(at(110.7, 195.4)), at(425.5, 378.5));
        // Unlike a whole point, a negative offset rounds away from the
        // center: 404.05 truncates to 404, where the offset gave 405.
        assert_eq!(camera.pixel_centre(at(89.3, 204.6)), at(404.5, 387.5));
        // A point drawn left of the screen: -10.7 truncates to -10.
        assert_eq!(camera.pixel_centre(at(100.25 - 425.7, 200.75)).x, -9.5);
    }
}
