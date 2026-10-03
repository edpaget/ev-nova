//! The map's view: which part of the galaxy the map area shows, and at
//! what scale.
//!
//! The map area is the logical rectangle (0, 0) to ([`MAP_WIDTH`],
//! [`MAP_HEIGHT`]); the info panel fills the rest of the screen below it.
//! The view keeps the map (world) point drawn at the map area's centre and
//! one of Nova's seven map scales:
//!
//! - `world_to_screen(p) = MAP_CENTER + (p - center) x scale`
//! - `screen_to_world(s) = center + (s - MAP_CENTER) / scale`

use crate::Point;

/// The map area's width, in logical units.
pub const MAP_WIDTH: f32 = 1024.0;
/// The map area's height, in logical units; the info panel is below it.
pub const MAP_HEIGHT: f32 = 608.0;
/// The map area's centre.
pub const MAP_CENTER: Point = Point::new(MAP_WIDTH / 2.0, MAP_HEIGHT / 2.0);
/// Nova's map scales, smallest first: (3/4)^3 to (4/3)^3, which the Bible
/// gives as 42.1%, 56.2%, 75%, 100%, 133.3%, 177.7% and 237%.
pub const SCALES: [f32; 7] = [
    27.0 / 64.0,
    9.0 / 16.0,
    3.0 / 4.0,
    1.0,
    4.0 / 3.0,
    16.0 / 9.0,
    64.0 / 27.0,
];
/// The index of 100% in [`SCALES`].
const ACTUAL_SIZE: usize = 3;
/// The space the fitted view leaves round the galaxy, in logical units.
pub const FIT_MARGIN: f32 = 24.0;
/// How far one arrow-key press pans the view, in logical units.
pub const PAN_STEP: f32 = 64.0;

/// A rectangle of map (world) points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// The top-left corner: the smallest x and y.
    pub min: Point,
    /// The bottom-right corner: the largest x and y.
    pub max: Point,
}

impl Bounds {
    /// The smallest rectangle holding every point, or `None` for none.
    pub fn around(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        points.into_iter().fold(None, |bounds, p| {
            Some(match bounds {
                None => Self { min: p, max: p },
                Some(Self { min, max }) => Self {
                    min: Point::new(min.x.min(p.x), min.y.min(p.y)),
                    max: Point::new(max.x.max(p.x), max.y.max(p.y)),
                },
            })
        })
    }

    /// Its width.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Its height.
    #[must_use]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Its centre.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(
            f32::midpoint(self.min.x, self.max.x),
            f32::midpoint(self.min.y, self.max.y),
        )
    }

    /// The point in the rectangle nearest `p`.
    fn clamp(&self, p: Point) -> Point {
        Point::new(
            p.x.clamp(self.min.x, self.max.x),
            p.y.clamp(self.min.y, self.max.y),
        )
    }
}

/// What the map area shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapView {
    /// The world point drawn at [`MAP_CENTER`].
    center: Point,
    /// The index of the scale in [`SCALES`].
    zoom: usize,
    /// Where `center` may go: the galaxy's bounds.
    limits: Bounds,
}

impl MapView {
    /// The view of the whole of `bounds`: centred on it, at the largest
    /// scale where it fits the map area with [`FIT_MARGIN`] to spare on
    /// every side, or the smallest scale if it fits at none. With no
    /// bounds (no systems), (0, 0) at 100%.
    #[must_use]
    pub fn fit(bounds: Option<Bounds>) -> Self {
        let Some(limits) = bounds else {
            let origin = Point::default();
            return Self {
                center: origin,
                zoom: ACTUAL_SIZE,
                limits: Bounds {
                    min: origin,
                    max: origin,
                },
            };
        };
        let fits = |scale: f32| {
            limits.width() * scale + 2.0 * FIT_MARGIN <= MAP_WIDTH
                && limits.height() * scale + 2.0 * FIT_MARGIN <= MAP_HEIGHT
        };
        let zoom = (0..SCALES.len())
            .rev()
            .find(|&zoom| fits(SCALES[zoom]))
            .unwrap_or(0);
        Self {
            center: limits.center(),
            zoom,
            limits,
        }
    }

    /// The world point at the map area's centre.
    #[must_use]
    pub fn center(&self) -> Point {
        self.center
    }

    /// The index of the scale in [`SCALES`].
    #[must_use]
    pub fn zoom(&self) -> usize {
        self.zoom
    }

    /// Logical units per map unit.
    #[must_use]
    pub fn scale(&self) -> f32 {
        SCALES[self.zoom]
    }

    /// Where the world point `p` is drawn.
    #[must_use]
    pub fn world_to_screen(&self, p: Point) -> Point {
        let scale = self.scale();
        Point::new(
            MAP_CENTER.x + (p.x - self.center.x) * scale,
            MAP_CENTER.y + (p.y - self.center.y) * scale,
        )
    }

    /// The world point drawn at screen point `s`.
    #[must_use]
    pub fn screen_to_world(&self, s: Point) -> Point {
        let scale = self.scale();
        Point::new(
            self.center.x + (s.x - MAP_CENTER.x) / scale,
            self.center.y + (s.y - MAP_CENTER.y) / scale,
        )
    }

    /// Steps to the next larger scale, if there is one, keeping the
    /// centre.
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom + 1).min(SCALES.len() - 1);
    }

    /// Steps to the next smaller scale, if there is one, keeping the
    /// centre.
    pub fn zoom_out(&mut self) {
        self.zoom = self.zoom.saturating_sub(1);
    }

    /// Moves the view `dx`, `dy` logical units (so the map moves the other
    /// way), stopping at the edge of the galaxy's bounds.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let scale = self.scale();
        let moved = Point::new(self.center.x + dx / scale, self.center.y + dy / scale);
        self.center = self.limits.clamp(moved);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn bounds(min: (f32, f32), max: (f32, f32)) -> Bounds {
        Bounds {
            min: at(min.0, min.1),
            max: at(max.0, max.1),
        }
    }

    /// The stock galaxy's bounds.
    fn stock() -> Bounds {
        bounds((-358.0, -330.0), (587.0, 300.0))
    }

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() <= 1e-3 && (a.y - b.y).abs() <= 1e-3
    }

    #[test]
    fn the_scales_are_the_bibles_percentages() {
        let bible = [42.1, 56.2, 75.0, 100.0, 133.3, 177.7, 237.0];
        for (scale, percent) in SCALES.iter().zip(bible) {
            assert!(
                (scale * 100.0 - percent).abs() < 0.1,
                "{scale} vs {percent}%"
            );
        }
        assert_eq!(SCALES[ACTUAL_SIZE], 1.0);
        assert!(SCALES.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn bounds_hold_every_point() {
        let points = [at(3.0, -1.0), at(-2.0, 5.0), at(1.0, 0.0)];
        let around = Bounds::around(points).expect("points");
        assert_eq!(around, bounds((-2.0, -1.0), (3.0, 5.0)));
        assert_eq!((around.width(), around.height()), (5.0, 6.0));
        assert_eq!(around.center(), at(0.5, 2.0));
        assert_eq!(
            Bounds::around([at(7.0, 8.0)]),
            Some(bounds((7.0, 8.0), (7.0, 8.0)))
        );
        assert_eq!(Bounds::around([]), None);
    }

    #[test]
    fn the_stock_galaxy_fits_at_75_percent_centred() {
        let view = MapView::fit(Some(stock()));
        assert_eq!(view.zoom(), 2);
        assert_eq!(view.scale(), 0.75);
        assert_eq!(view.center(), at(114.5, -15.0));
    }

    #[test]
    fn fit_picks_the_largest_scale_that_leaves_the_margin() {
        // Exactly the map area less the margins, at 100%: wide, then tall.
        let wide = bounds((0.0, 0.0), (MAP_WIDTH - 2.0 * FIT_MARGIN, 10.0));
        assert_eq!(MapView::fit(Some(wide)).zoom(), ACTUAL_SIZE);
        let tall = bounds((0.0, 0.0), (10.0, MAP_HEIGHT - 2.0 * FIT_MARGIN));
        assert_eq!(MapView::fit(Some(tall)).zoom(), ACTUAL_SIZE);
        // A unit more and they need the next scale down.
        let wider = bounds((0.0, 0.0), (MAP_WIDTH - 2.0 * FIT_MARGIN + 1.0, 10.0));
        assert_eq!(MapView::fit(Some(wider)).zoom(), ACTUAL_SIZE - 1);
        let taller = bounds((0.0, 0.0), (10.0, MAP_HEIGHT - 2.0 * FIT_MARGIN + 1.0));
        assert_eq!(MapView::fit(Some(taller)).zoom(), ACTUAL_SIZE - 1);
        // A small galaxy, or one system, shows at the largest scale.
        let small = bounds((-5.0, -5.0), (5.0, 5.0));
        assert_eq!(MapView::fit(Some(small)).zoom(), SCALES.len() - 1);
        let point = bounds((40.0, 50.0), (40.0, 50.0));
        let view = MapView::fit(Some(point));
        assert_eq!(
            (view.zoom(), view.center()),
            (SCALES.len() - 1, at(40.0, 50.0))
        );
    }

    #[test]
    fn a_galaxy_too_big_for_every_scale_shows_at_the_smallest() {
        let huge = bounds((-3000.0, -100.0), (3000.0, 100.0));
        let view = MapView::fit(Some(huge));
        assert_eq!(view.zoom(), 0);
        assert_eq!(view.center(), at(0.0, 0.0));
    }

    #[test]
    fn no_systems_shows_the_origin_at_100_percent() {
        let mut view = MapView::fit(None);
        assert_eq!((view.zoom(), view.center()), (ACTUAL_SIZE, at(0.0, 0.0)));
        view.pan(PAN_STEP, -PAN_STEP);
        assert_eq!(view.center(), at(0.0, 0.0), "nowhere to pan to");
    }

    #[test]
    fn the_centre_is_drawn_at_the_map_areas_centre() {
        let view = MapView::fit(Some(stock()));
        assert_eq!(view.world_to_screen(at(114.5, -15.0)), MAP_CENTER);
        // 100 map units right and 40 up, at 75%.
        assert_eq!(
            view.world_to_screen(at(214.5, -55.0)),
            at(MAP_CENTER.x + 75.0, MAP_CENTER.y - 30.0)
        );
        assert_eq!(view.screen_to_world(MAP_CENTER), at(114.5, -15.0));
        assert_eq!(
            view.screen_to_world(at(0.0, 0.0)),
            at(114.5 - 512.0 / 0.75, -15.0 - 304.0 / 0.75)
        );
    }

    #[test]
    fn screen_and_world_round_trip_at_every_zoom() {
        let mut view = MapView::fit(Some(stock()));
        for _ in 0..SCALES.len() {
            view.zoom_out();
        }
        for zoom in 0..SCALES.len() {
            assert_eq!(view.zoom(), zoom);
            for p in [at(-358.0, 300.0), at(587.0, -330.0), at(13.25, 7.5)] {
                let back = view.screen_to_world(view.world_to_screen(p));
                assert!(close(back, p), "{p:?} came back as {back:?} at zoom {zoom}");
            }
            view.zoom_in();
        }
    }

    #[test]
    fn zooming_steps_through_the_scales_keeping_the_centre_and_stops_at_the_ends() {
        let mut view = MapView::fit(Some(stock()));
        let center = view.center();
        let mut zooms = Vec::new();
        for _ in 0..=SCALES.len() {
            view.zoom_in();
            zooms.push(view.zoom());
            assert_eq!(view.center(), center);
            assert_eq!(view.world_to_screen(center), MAP_CENTER);
        }
        assert_eq!(zooms, [3, 4, 5, 6, 6, 6, 6, 6]);
        zooms.clear();
        for _ in 0..=SCALES.len() {
            view.zoom_out();
            zooms.push(view.zoom());
            assert_eq!(view.center(), center);
        }
        assert_eq!(zooms, [5, 4, 3, 2, 1, 0, 0, 0]);
    }

    #[test]
    fn a_pan_step_moves_the_map_one_step_on_screen_at_every_zoom() {
        let mut view = MapView::fit(Some(stock()));
        for _ in 0..SCALES.len() {
            view.zoom_out();
        }
        let p = at(100.0, -20.0);
        for zoom in 0..SCALES.len() {
            let before = view.world_to_screen(p);
            view.pan(PAN_STEP, 0.0);
            let after = view.world_to_screen(p);
            assert!(
                close(after, at(before.x - PAN_STEP, before.y)),
                "zoom {zoom}"
            );
            view.pan(0.0, -PAN_STEP);
            let after = view.world_to_screen(p);
            assert!(
                close(after, at(before.x - PAN_STEP, before.y + PAN_STEP)),
                "zoom {zoom}"
            );
            view.pan(-PAN_STEP, PAN_STEP);
            assert!(close(view.world_to_screen(p), before), "zoom {zoom}");
            view.zoom_in();
        }
    }

    #[test]
    fn panning_stops_at_the_galaxys_edges() {
        let mut view = MapView::fit(Some(stock()));
        for _ in 0..100 {
            view.pan(PAN_STEP, PAN_STEP);
        }
        assert_eq!(view.center(), at(587.0, 300.0));
        for _ in 0..100 {
            view.pan(-PAN_STEP, -PAN_STEP);
        }
        assert_eq!(view.center(), at(-358.0, -330.0));
        // Partway along one axis, clamped on the other.
        view.pan(PAN_STEP, -PAN_STEP);
        assert_eq!(view.center(), at(-358.0 + PAN_STEP / 0.75, -330.0));
    }
}
