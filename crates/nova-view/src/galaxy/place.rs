//! Where the in-flight maps look: the original's `_mapZoom` and
//! `_mapScroll`, as `_DoSystemMap` and `_MapFilter` keep them.
//!
//! - **The zoom** divides map coordinates, so larger is further out. Its
//!   data initialiser is [`INITIAL_ZOOM`] (0xde950), and nothing but the
//!   map's zoom items writes it, so the first map of a run opens at
//!   0.5625 and every later one where the last was left. On open a zoom
//!   of 0 or less becomes 1.0 (@0x12305-0x1234d), which never happens on
//!   stock data but is kept ([`MapPlace::opened`]).
//! - **A step** out multiplies it by [`ZOOM_OUT_STEP`], the decimal
//!   1.333333 (0xdd1f0), while `_canZoomOut` (zoom < [`ZOOM_OUT_LIMIT`]);
//!   a step in by [`ZOOM_IN_STEP`] (0xdd1f8) while `_canZoomIn` (zoom >
//!   [`ZOOM_IN_LIMIT`]) (@0x12803-0x12838, @0x12a36-0x12a6b). The flags
//!   are worked out before each step (@0x13c6f-0x13cb4), so from 0.5625
//!   the reachable zooms are 0.421875 up to 2.3703674…, seven in all.
//! - **The scroll** is the galaxy point drawn at item 3's centre. The
//!   arrows move it [`ARROW_STEP`] units, unscaled (@0x142a3, @0x142c3,
//!   @0x14370, @0x14417), a drag sets it from the press's start
//!   ([`MapPlace::drag`], @0x13856-0x138b1), and Space, Find and every
//!   reset set it to a system's position.
//!
//! Nothing clamps either, as the original never does. The scroll is held
//! in `i32`, not the original's 16-bit words, which wrap.

use super::view::Transform;
use crate::Point;

/// `_mapZoom`'s data initialiser (0xde950): the first map of a run.
pub const INITIAL_ZOOM: f64 = 0.5625;
/// What item 4, "-", multiplies the zoom by (0xdd1f0): the decimal
/// 1.333333, not 4/3.
pub const ZOOM_OUT_STEP: f64 = 1.333_333;
/// What item 5, "+", multiplies the zoom by (0xdd1f8).
pub const ZOOM_IN_STEP: f64 = 0.75;
/// `_canZoomIn` is the zoom above this (0xdd128).
pub const ZOOM_IN_LIMIT: f64 = 0.5;
/// `_canZoomOut` is the zoom below this (0xdd1c8).
pub const ZOOM_OUT_LIMIT: f64 = 2.0;
/// How far an arrow moves the scroll, in galaxy units.
pub const ARROW_STEP: i32 = 10;
/// The zoom the open's guard gives a zoom of 0 or less (0xdd0c0).
const GUARD_ZOOM: f64 = 1.0;

/// An in-flight map's zoom and scroll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapPlace {
    zoom: f64,
    scroll: (i32, i32),
}

impl MapPlace {
    /// Looking at `scroll`, at [`INITIAL_ZOOM`].
    #[must_use]
    pub fn new(scroll: (i32, i32)) -> Self {
        Self {
            zoom: INITIAL_ZOOM,
            scroll,
        }
    }

    /// The same place at `zoom`.
    #[must_use]
    pub fn with_zoom(self, zoom: f64) -> Self {
        Self { zoom, ..self }
    }

    /// The zoom: map units per screen pixel.
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// The galaxy point drawn at the map's centre.
    #[must_use]
    pub fn scroll(&self) -> (i32, i32) {
        self.scroll
    }

    /// The open's guard (@0x12305-0x1234d): a zoom of 0 or less becomes
    /// 1.0; any other is kept.
    pub fn opened(&mut self) {
        if self.zoom <= 0.0 {
            self.zoom = GUARD_ZOOM;
        }
    }

    /// `_canZoomIn`: the zoom above [`ZOOM_IN_LIMIT`].
    #[must_use]
    pub fn can_zoom_in(&self) -> bool {
        self.zoom > ZOOM_IN_LIMIT
    }

    /// `_canZoomOut`: the zoom below [`ZOOM_OUT_LIMIT`].
    #[must_use]
    pub fn can_zoom_out(&self) -> bool {
        self.zoom < ZOOM_OUT_LIMIT
    }

    /// Item 5: one step in, when [`MapPlace::can_zoom_in`]; whether the
    /// zoom moved.
    pub fn zoom_in(&mut self) -> bool {
        let can = self.can_zoom_in();
        if can {
            self.zoom *= ZOOM_IN_STEP;
        }
        can
    }

    /// Item 4: one step out, when [`MapPlace::can_zoom_out`]; whether
    /// the zoom moved.
    pub fn zoom_out(&mut self) -> bool {
        let can = self.can_zoom_out();
        if can {
            self.zoom *= ZOOM_OUT_STEP;
        }
        can
    }

    /// Moves the scroll by (`dx`, `dy`) galaxy units, as the arrows do.
    pub fn scroll_by(&mut self, dx: i32, dy: i32) {
        self.scroll = (self.scroll.0 + dx, self.scroll.1 + dy);
    }

    /// Looks at `at`, a system's position, truncated toward zero to whole
    /// units, unclamped.
    #[allow(clippy::cast_possible_truncation)] // Galaxy units.
    pub fn centre_on(&mut self, at: Point) {
        self.scroll = (at.x.trunc() as i32, at.y.trunc() as i32);
    }

    /// The drag's poll (@0x13856-0x138b1): each axis of the scroll set to
    /// `start` − (`to` − `from`) × zoom, truncated toward zero
    /// (`cvttsd2si`), with the pointer's points taken as the whole pixels
    /// `GetMouse` gives.
    #[allow(clippy::cast_possible_truncation)] // Galaxy units.
    pub fn drag(&mut self, start: (i32, i32), from: Point, to: Point) {
        let axis = |start: i32, from: f32, to: f32| {
            let travel = f64::from(to.floor()) - f64::from(from.floor());
            (f64::from(start) - travel * self.zoom).trunc() as i32
        };
        self.scroll = (axis(start.0, from.x, to.x), axis(start.1, from.y, to.y));
    }

    /// The map's transform, as `_DrawMap` places each point: the scroll
    /// drawn at `centre`, the map rect's whole centre in a port whose
    /// top-left is at `port` on the screen, each position divided by the
    /// zoom in double and truncated in the port
    /// ([`Transform::divided`]).
    #[allow(clippy::cast_precision_loss)] // Galaxy units.
    #[must_use]
    pub fn transform(&self, port: (i32, i32), centre: (i32, i32)) -> Transform {
        let scroll = Point::new(self.scroll.0 as f32, self.scroll.1 as f32);
        Transform::divided(port, centre, scroll, self.zoom)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::galaxy::view::Divide;

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// A place looking at (0, 0) at `zoom`.
    fn zoomed(zoom: f64) -> MapPlace {
        MapPlace::new((0, 0)).with_zoom(zoom)
    }

    #[test]
    fn a_new_place_is_at_0_5625_looking_where_it_is_told() {
        let place = MapPlace::new((3, -4));
        assert_eq!(place.zoom(), 0.5625);
        assert_eq!(place.scroll(), (3, -4));
        assert_eq!(INITIAL_ZOOM, 0.5625);
    }

    #[test]
    fn opening_makes_a_zoom_of_0_or_less_1_and_keeps_any_other() {
        for (zoom, opened) in [(0.0, 1.0), (-1.0, 1.0), (0.5625, 0.5625), (1e-9, 1e-9)] {
            let mut place = zoomed(zoom);
            place.opened();
            assert_eq!(place.zoom(), opened, "{zoom}");
        }
    }

    #[test]
    fn one_step_in_from_0_5625_reaches_0_421875_and_a_second_is_refused() {
        let mut place = MapPlace::new((0, 0));
        assert!(place.zoom_in());
        assert_eq!(place.zoom(), 0.421_875);
        assert!(!place.can_zoom_in());
        assert!(!place.zoom_in(), "0.421875 is not above 0.5");
        assert_eq!(place.zoom(), 0.421_875);
    }

    #[test]
    fn five_steps_out_multiply_by_1_333333_and_a_sixth_is_refused() {
        let mut place = MapPlace::new((0, 0));
        let mut zooms = Vec::new();
        for _ in 0..5 {
            assert!(place.zoom_out());
            zooms.push(place.zoom());
        }
        assert_eq!(
            zooms,
            [
                0.749_999_812_5,
                0.999_999_500_000_062_7,
                1.333_332_333_333_583_6,
                1.777_776_000_000_667,
                2.370_367_407_408_889_6,
            ]
        );
        assert!(!place.can_zoom_out());
        assert!(!place.zoom_out(), "2.37 is not below 2.0");
        assert_eq!(place.zoom(), 2.370_367_407_408_889_6);
    }

    #[test]
    fn the_flags_at_each_bound() {
        assert!(!zoomed(0.421_875).can_zoom_in());
        assert!(zoomed(0.5625).can_zoom_in());
        assert!(!zoomed(0.5).can_zoom_in(), "the limit itself");
        assert!(!zoomed(2.370_367_407_408_889_6).can_zoom_out());
        assert!(zoomed(1.777_776).can_zoom_out());
        assert!(!zoomed(2.0).can_zoom_out(), "the limit itself");
    }

    #[test]
    fn the_limits_are_tested_before_a_step_so_one_steps_past_each() {
        let mut place = zoomed(0.500_000_1);
        assert!(place.zoom_in());
        assert_eq!(place.zoom(), 0.500_000_1 * 0.75, "below the limit");
        let mut place = zoomed(1.999_999);
        assert!(place.zoom_out());
        assert_eq!(place.zoom(), 1.999_999 * 1.333_333, "above the limit");
    }

    #[test]
    fn a_step_in_and_one_out_drift_rather_than_cancel() {
        let mut place = MapPlace::new((0, 0));
        place.zoom_out();
        place.zoom_in();
        assert_ne!(place.zoom(), 0.5625);
        assert!((place.zoom() - 0.5625).abs() < 1e-6);
    }

    #[test]
    fn the_arrows_move_the_scroll_10_units_at_any_zoom() {
        for zoom in [0.5625, 1.777_776] {
            let mut place = MapPlace::new((5, 5)).with_zoom(zoom);
            place.scroll_by(ARROW_STEP, 0);
            place.scroll_by(0, -ARROW_STEP);
            assert_eq!(place.scroll(), (15, -5), "{zoom}");
            assert_eq!(place.zoom(), zoom);
        }
    }

    #[test]
    fn a_drag_scrolls_by_the_travel_times_the_zoom_truncated_toward_zero() {
        let mut place = MapPlace::new((0, 0));
        place.drag((0, 0), at(100.0, 100.0), at(140.0, 84.0));
        assert_eq!(place.scroll(), (-22, 9), "trunc(−22.5), trunc(9.0)");
        place.drag((100, 100), at(100.0, 100.0), at(140.0, 84.0));
        assert_eq!(place.scroll(), (77, 109), "trunc(77.5), trunc(109.0)");
        let mut place = zoomed(1.333_332_333_333_583_6);
        place.drag((0, 0), at(100.0, 100.0), at(140.0, 84.0));
        assert_eq!(place.scroll(), (-53, 21), "trunc(−53.33), trunc(21.33)");
    }

    #[test]
    fn a_drag_takes_the_pointer_as_whole_pixels() {
        let mut place = MapPlace::new((0, 0)).with_zoom(1.0);
        // From 100.9 to 140.1: pixels 100 to 140, 40 across.
        place.drag((0, 0), at(100.9, 100.9), at(140.1, 100.2));
        assert_eq!(place.scroll(), (-40, 0));
        // Back across a pixel's edge: 100 to 99.
        place.drag((0, 0), at(100.2, 100.0), at(99.9, 100.0));
        assert_eq!(place.scroll(), (1, 0));
    }

    #[test]
    fn centring_sets_the_scroll_unclamped() {
        let mut place = MapPlace::new((0, 0));
        place.centre_on(at(30_000.0, -30_000.0));
        assert_eq!(place.scroll(), (30_000, -30_000));
        place.centre_on(at(-12.7, 12.7));
        assert_eq!(place.scroll(), (-12, 12), "toward zero");
    }

    #[test]
    fn the_transform_divides_by_the_zoom_with_the_scroll_at_the_centre() {
        let place = MapPlace::new((40, -20));
        let transform = place.transform((212, 128), (238, 218));
        assert_eq!(
            transform.divide,
            Some(Divide {
                zoom: 0.5625,
                port: (212, 128)
            })
        );
        assert_eq!(transform.world, at(40.0, -20.0));
        // (40 / 0.5625 + 238) − 40 / 0.5625 is a hair under 238, so the
        // scroll lands a pixel left of item 3's centre, as in the original.
        assert_eq!(transform.pixel(at(40.0, -20.0)), (449, 346));
        let place = place.with_zoom(2.0);
        assert_eq!(place.transform((0, 0), (0, 0)).zoom(), 2.0);
    }
}
