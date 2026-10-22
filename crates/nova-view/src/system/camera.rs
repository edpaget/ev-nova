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

use nova_sim::RuleSource;

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
/// The status bar's width `_SetupWind` takes from the screen's to find the
/// play area's centre (0xc2, @0x1b720-0x1b78a): the stock bars' width.
pub const PLAY_AREA_BAR_WIDTH: f32 = 194.0;

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
                ((view.0 - PLAY_AREA_BAR_WIDTH) * 0.5).trunc(),
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
}
