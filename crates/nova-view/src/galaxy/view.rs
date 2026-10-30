//! The viewer's view: which part of the galaxy its map area shows, and at
//! what scale; and the [`Transform`] every map draws through. The
//! in-flight maps look where their dialog's place says instead
//! ([`place`](super::place)).
//!
//! The viewer's map area is the logical rectangle (0, 0) to
//! ([`MAP_WIDTH`], [`MAP_HEIGHT`]); its info panel fills the rest of the
//! screen below it.
//! The view keeps the map (world) point drawn at the map area's centre and
//! one of Nova's seven map scales:
//!
//! - `world_to_screen(p) = MAP_CENTER + (p - center) x scale`
//! - `screen_to_world(s) = center + (s - MAP_CENTER) / scale`

use crate::Point;
pub use crate::geometry::Bounds;

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

/// Where a map draws a world point. There are two kinds of arithmetic:
///
/// - **Scaled** ([`Transform::scaled`], `divide` `None`): the viewer's
///   own, `screen + (p - world) x scale` in `f32`, truncated on the
///   screen ([`MapView::transform`]).
/// - **Divided** ([`Transform::divided`], `divide` `Some`): `_DrawMap`'s,
///   each position divided by the double zoom in the map's port and
///   truncated there ([`Divide`]). The in-flight maps draw through it:
///   the course and hypergate maps through their place's
///   ([`MapPlace::transform`](super::place::MapPlace::transform)), the
///   floating map through its own, centred on the current system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Where `world` is drawn.
    pub screen: Point,
    /// The world point drawn at `screen`.
    pub world: Point,
    /// Logical units per map unit. The viewer draws with it; a divided
    /// map only reports it, as (1 / zoom) in `f32`.
    pub scale: f32,
    /// `None`: the viewer's `f32` arithmetic. `Some`: `_DrawMap`'s divide.
    pub divide: Option<Divide>,
}

/// `_DrawMap`'s arithmetic (@0xe574-0xf5d0): per axis,
/// port + trunc((p / zoom + c) − world / zoom), every operand a double,
/// the add before the subtract, and the truncation (`cvttsd2si`) in port
/// coordinates before QuickDraw adds the port's origin. c = screen − port
/// is the map rect's whole centre in the port. The order matters:
/// (a / z + c) − a / z need not be c.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Divide {
    /// `_mapZoom` (or `_floatingMapZoom`), exact: map units per pixel.
    pub zoom: f64,
    /// The port's top-left on the screen, in whole pixels.
    pub port: (i32, i32),
}

impl Transform {
    /// The viewer's transform: `world` drawn at `screen`, `scale`
    /// logical units per map unit, in `f32`.
    #[must_use]
    pub fn scaled(screen: Point, world: Point, scale: f32) -> Self {
        Self {
            screen,
            world,
            scale,
            divide: None,
        }
    }

    /// `_DrawMap`'s transform: `world` at `centre` in a port whose
    /// top-left is at `port` on the screen, each position divided by
    /// `zoom` ([`Divide`]).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // A scale near 1.
    #[allow(clippy::cast_precision_loss)] // Screen pixels.
    pub fn divided(port: (i32, i32), centre: (i32, i32), world: Point, zoom: f64) -> Self {
        Self {
            screen: Point::new((port.0 + centre.0) as f32, (port.1 + centre.1) as f32),
            world,
            scale: (1.0 / zoom) as f32,
            divide: Some(Divide { zoom, port }),
        }
    }

    /// Where `p` is in the port, unrounded: for a divided transform
    /// (p / zoom + c) − world / zoom in double, each axis in that order;
    /// for the viewer its screen point, the port being the screen.
    fn in_port(&self, p: Point) -> (f64, f64) {
        if let Some(Divide { zoom, port }) = self.divide {
            let axis = |p: f32, screen: f32, port: i32, world: f32| {
                let c = f64::from(screen) - f64::from(port);
                (f64::from(p) / zoom + c) - f64::from(world) / zoom
            };
            (
                axis(p.x, self.screen.x, port.0, self.world.x),
                axis(p.y, self.screen.y, port.1, self.world.y),
            )
        } else {
            let at = self.scaled_point(p);
            (f64::from(at.x), f64::from(at.y))
        }
    }

    /// The viewer's `f32` arithmetic: `screen + (p - world) x scale`.
    fn scaled_point(&self, p: Point) -> Point {
        Point::new(
            self.screen.x + (p.x - self.world.x) * self.scale,
            self.screen.y + (p.y - self.world.y) * self.scale,
        )
    }

    /// The port's top-left on the screen: the divide's, or (0, 0) for
    /// the viewer, whose port is the screen.
    #[must_use]
    pub fn port(&self) -> (i32, i32) {
        self.divide.map_or((0, 0), |divide| divide.port)
    }

    /// Where the world point `p` is drawn, unrounded. For a divided
    /// transform it is the port's origin plus [`Transform::port_point`],
    /// for callers that want a float point (a test aiming a click); the
    /// maps draw at [`Transform::pixel`].
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Screen pixels.
    pub fn world_to_screen(&self, p: Point) -> Point {
        match self.divide {
            Some(Divide { port, .. }) => {
                let at = self.port_point(p);
                Point::new(port.0 as f32 + at.x, port.1 as f32 + at.y)
            }
            None => self.scaled_point(p),
        }
    }

    /// Where `p` is drawn in whole pixels. For a divided transform:
    /// port + trunc((p / zoom + c) − world / zoom), truncated toward zero
    /// in the port (`cvttsd2si`, `_DrawMap` @0xe5e9-0xe662). For the
    /// viewer: its screen point truncated toward zero.
    #[must_use]
    pub fn pixel(&self, p: Point) -> (i32, i32) {
        self.pixel_offset(p, (0.0, 0.0))
    }

    /// Where `p` offset by `d` is drawn in whole pixels, the offset added
    /// before truncating: for a divided transform in double in the port,
    /// as the names add (7.0, 4.0) (`_DrawMap` @0xf559-0xf595,
    /// `_MapDialogUpdate` @0xffae-0xfff4); for the viewer in `f32` on
    /// the screen.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Screen pixels.
    pub fn pixel_offset(&self, p: Point, (dx, dy): (f32, f32)) -> (i32, i32) {
        if self.divide.is_some() {
            let (x, y) = self.in_port(p);
            let (px, py) = self.port();
            (
                px + (x + f64::from(dx)).trunc() as i32,
                py + (y + f64::from(dy)).trunc() as i32,
            )
        } else {
            let at = self.scaled_point(p);
            ((at.x + dx).trunc() as i32, (at.y + dy).trunc() as i32)
        }
    }

    /// Where `p` is in the port as a float point: for a divided transform
    /// the double point narrowed to single (`cvtsd2ss`), as the hypergate
    /// arrows keep their ends (@0xf2d3-0xf330); for the viewer its screen
    /// point.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // To single, as the original.
    pub fn port_point(&self, p: Point) -> Point {
        let (x, y) = self.in_port(p);
        Point::new(x as f32, y as f32)
    }

    /// The original's zoom, as `_mapZoom` divides each position by it:
    /// the divide's, or for the viewer the inverse of its scale.
    #[must_use]
    pub fn zoom(&self) -> f64 {
        self.divide
            .map_or_else(|| 1.0 / f64::from(self.scale), |divide| divide.zoom)
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
        self.transform().world_to_screen(p)
    }

    /// The view as a [`Transform`]: `center` drawn at [`MAP_CENTER`].
    #[must_use]
    pub fn transform(&self) -> Transform {
        Transform::scaled(MAP_CENTER, self.center, self.scale())
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

    /// Centres the view on the world point `at`, stopping at the edge of
    /// the galaxy's bounds and keeping the zoom, as Space and Find set the
    /// original's `_mapScroll` to a system's position (`_MapFilter`
    /// @0x1411e-0x14153, `_DoSystemSearch` @0x12080-0x120aa). A system is
    /// always inside the bounds.
    pub fn centre_on(&mut self, at: Point) {
        self.center = self.limits.clamp(at);
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
    fn a_transform_draws_its_world_point_at_its_screen_point_scaled() {
        let transform = Transform::scaled(at(686.0, 278.0), at(10.0, -20.0), 0.75);
        assert_eq!(transform.world_to_screen(at(10.0, -20.0)), at(686.0, 278.0));
        assert_eq!(
            transform.world_to_screen(at(50.0, -40.0)),
            at(686.0 + 30.0, 278.0 - 15.0)
        );
    }

    /// The course map's port (the dialog's origin) and item 3's centre in
    /// it.
    const PORT: (i32, i32) = (212, 128);
    const CENTRE: (i32, i32) = (238, 218);

    /// The dialog's transform looking at (0, 0) after `steps` zooms out
    /// from 0.5625, as `_DoSystemMap` steps `_mapZoom`.
    fn dialog_transform(steps: usize) -> Transform {
        let mut place = super::super::place::MapPlace::new((0, 0));
        for _ in 0..steps {
            assert!(place.zoom_out());
        }
        Transform::divided(PORT, CENTRE, at(0.0, 0.0), place.zoom())
    }

    /// The old f32 arithmetic: trunc(screen + p × (1 / zoom as f32)).
    #[allow(clippy::cast_possible_truncation)]
    fn f32_pixel(transform: &Transform, p: Point) -> (i32, i32) {
        let scale = (1.0 / transform.zoom()) as f32;
        (
            (450.0 + p.x * scale).trunc() as i32,
            (346.0 + p.y * scale).trunc() as i32,
        )
    }

    #[test]
    fn a_divided_transform_places_a_point_as_draw_map_does_at_two_zooms() {
        for (steps, zoom, p, pixel) in [
            (1, 0.749_999_812_5, at(-66.0, -66.0), (361, 257)),
            (2, 0.999_999_500_000_062_7, at(-36.0, -36.0), (413, 309)),
        ] {
            let transform = dialog_transform(steps);
            assert!((transform.zoom() - zoom).abs() < 1e-12, "{steps}");
            assert_eq!(transform.pixel(p), pixel, "{steps}");
            assert_ne!(f32_pixel(&transform, p), pixel, "{steps}: f32 is off");
        }
    }

    #[test]
    fn a_divided_transform_truncates_in_the_port() {
        // −253 / 0.5625 + 238 = −211.78 in the port: 212 − 211 = 1, where
        // the screen's trunc(0.22) would be 0.
        let transform = dialog_transform(0);
        assert_eq!(transform.pixel(at(-253.0, 0.0)).0, 1);
        assert_eq!(transform.port(), PORT);
    }

    #[test]
    fn a_divided_transform_adds_before_it_subtracts() {
        // (37 / 0.421875 + 128) − 37 / 0.421875 is 127.99999999999999.
        let transform = Transform::divided((558, 150), (128, 128), at(37.0, 37.0), 0.421_875);
        assert_eq!(transform.pixel(at(37.0, 37.0)), (685, 277));
    }

    #[test]
    fn a_divided_transform_adds_the_names_offset_before_truncating() {
        let transform = dialog_transform(2);
        let p = at(-36.0, -36.0);
        assert_eq!(transform.pixel_offset(p, (7.0, 4.0)), (420, 313));
        let (x, y) = f32_pixel(&transform, p);
        assert_ne!((x + 7, y + 4), (420, 313), "f32 is off");
    }

    #[test]
    fn a_divided_transform_reports_its_divide_and_its_port_point() {
        let transform = dialog_transform(0);
        assert_eq!(
            transform.divide,
            Some(Divide {
                zoom: 0.5625,
                port: PORT
            })
        );
        assert_eq!(transform.screen, at(450.0, 346.0));
        assert_eq!(transform.world, at(0.0, 0.0));
        assert_eq!(transform.scale, 16.0 / 9.0);
        assert_eq!(transform.zoom(), 0.5625);
        // 9 / 0.5625 = 16 right of item 3's centre, in the port and on the
        // screen.
        assert_eq!(transform.port_point(at(9.0, -9.0)), at(254.0, 202.0));
        assert_eq!(transform.world_to_screen(at(9.0, -9.0)), at(466.0, 330.0));
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn the_viewers_transform_is_unchanged() {
        let mut view = MapView::fit(None);
        view.zoom_in();
        let transform = view.transform();
        assert_eq!(transform.divide, None);
        assert_eq!(transform.port(), (0, 0));
        let scale = 4.0_f32 / 3.0;
        let p = at(-381.0, 0.0);
        let f32_screen = at(MAP_CENTER.x + p.x * scale, MAP_CENTER.y + p.y * scale);
        assert_eq!(transform.world_to_screen(p), f32_screen);
        assert_eq!(transform.port_point(p), f32_screen);
        assert_eq!(transform.pixel(p), (4, 304));
        assert_eq!(transform.zoom(), 1.0 / f64::from(scale));
        // Dividing in double by that zoom would put it at 3.
        let divided = (f64::from(p.x) / transform.zoom() + 512.0).trunc() as i32;
        assert_ne!(transform.pixel(p).0, divided);
        assert_eq!(
            transform.pixel_offset(p, (7.0, 4.0)),
            (
                (f32_screen.x + 7.0).trunc() as i32,
                (f32_screen.y + 4.0).trunc() as i32
            )
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
    fn centring_on_a_point_keeps_the_zoom_and_stops_at_the_galaxys_edges() {
        let mut view = MapView::fit(Some(stock()));
        view.zoom_in();
        let zoom = view.zoom();
        view.centre_on(at(-100.0, 250.0));
        assert_eq!((view.center(), view.zoom()), (at(-100.0, 250.0), zoom));
        view.centre_on(at(1000.0, -1000.0));
        assert_eq!(view.center(), at(587.0, -330.0), "clamped");
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
