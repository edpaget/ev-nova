//! The system view's camera: which part of the system the screen shows.
//!
//! Nova measures space in screen pixels (the Bible: "Jump Distance 1000
//! pixels"; a weapon's speed is "pixels per frame"), and a `spöb`'s
//! position is in the same units with (0, 0) at the centre of the system.
//! The logical space is Nova's own 1024 x 768 screen, so one world unit is
//! one logical unit ([`WORLD_SCALE`]): distances and speeds read from the
//! data need no conversion. y grows downwards, as on screen.
//!
//! The camera keeps the world point drawn at the screen's centre:
//!
//! - `world_to_screen(p) = VIEW_CENTER + (p - center) x WORLD_SCALE`
//! - `screen_to_world(s) = center + (s - VIEW_CENTER) / WORLD_SCALE`

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

/// What the system view shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// The world point drawn at [`VIEW_CENTER`].
    center: Point,
    /// Where `center` may go.
    limits: Bounds,
}

impl Camera {
    /// A camera on the system's centre, (0, 0), kept within `limits`,
    /// which must include (0, 0).
    #[must_use]
    pub fn new(limits: Bounds) -> Self {
        Self {
            center: Point::default(),
            limits,
        }
    }

    /// The world point at the screen's centre.
    #[must_use]
    pub fn center(&self) -> Point {
        self.center
    }

    /// Where the world point `p` is drawn.
    #[must_use]
    pub fn world_to_screen(&self, p: Point) -> Point {
        Point::new(
            (p.x - self.center.x).mul_add(WORLD_SCALE, VIEW_CENTER.x),
            (p.y - self.center.y).mul_add(WORLD_SCALE, VIEW_CENTER.y),
        )
    }

    /// The world point drawn at screen point `s`.
    #[must_use]
    pub fn screen_to_world(&self, s: Point) -> Point {
        let per_logical_unit = WORLD_SCALE.recip();
        Point::new(
            (s.x - VIEW_CENTER.x).mul_add(per_logical_unit, self.center.x),
            (s.y - VIEW_CENTER.y).mul_add(per_logical_unit, self.center.y),
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
