//! The safe area and points per logical pixel, as the screens are told
//! them in [`Surroundings`]
//! (rdm:phase/key-bindings/phase-15-touch-flight-controls).
//!
//! - [`fit`] works both out from the window's safe area (physical pixels
//!   from its top-left, [`WindowPort::safe_area`]) and the [`Viewport`]
//!   the app draws through: the safe rect in the logical space of the
//!   shape shown now, clipped to its canvas, so an inset that only covers
//!   a letterbox or pillarbox bar cuts nothing; and points per logical
//!   pixel, the window's size in points against the logical size, not the
//!   scale factor alone.
//! - The app keeps the safe area it last read and the surroundings it
//!   last told. winit announces no change to the safe area (winit#3911),
//!   so the app reads it again at start, on every resize and at the head
//!   of every redraw, and tells the screen new [`Surroundings`] after
//!   every event and before every frame, only when they differ from what
//!   it last told ([`App::tell_surroundings`]). A shape change (flight
//!   entered or left) moves the safe rect into the new shape's
//!   coordinates, so it is told again.

use nova_render::Viewport;
use nova_view::geometry::{Bounds, Point};
use nova_view::{Input, Screen, Surroundings};

use super::{App, Control, PxRect, WindowPort};
use nova_render::ImageSource;

/// The logical safe rect and points per logical pixel of the window's
/// safe area `safe_px` through `viewport`, or `None` when the window has
/// no area.
///
/// Points per logical pixel is the smaller of the viewport's two axis
/// scales over the scale factor: in a widened view the two differ by
/// under half a unit, and the smaller keeps a size in points at least
/// that size along both axes.
pub(super) fn fit(safe_px: PxRect, viewport: &Viewport) -> Option<(Bounds, f32)> {
    let (across, down) = viewport.axis_scales()?;
    #[allow(clippy::cast_possible_truncation)] // A display's scale: small.
    let points_per_pixel = across.min(down) / viewport.scale_factor() as f32;
    let corner =
        |x: u32, y: u32| viewport.window_to_logical_unbounded((f64::from(x), f64::from(y)));
    let top_left = corner(safe_px.x, safe_px.y)?;
    let bottom_right = corner(
        safe_px.x.saturating_add(safe_px.width),
        safe_px.y.saturating_add(safe_px.height),
    )?;
    let logical = viewport.logical();
    #[allow(clippy::cast_precision_loss)] // A canvas: a few thousand units.
    let canvas = Bounds::at(Point::new(0.0, 0.0), logical.w as f32, logical.h as f32);
    let safe = Bounds {
        min: canvas.clamp(top_left),
        max: canvas.clamp(bottom_right),
    };
    Some((safe, points_per_pixel))
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// Reads the window's safe area again: winit announces no change to
    /// it, so the app asks at start, on each resize and on each redraw.
    pub(super) fn read_safe_area(&mut self, window: &impl WindowPort) {
        self.safe_px = window.safe_area();
    }

    /// The surroundings as they are now: the device used last, and the
    /// safe area and points per pixel through the viewport, kept as last
    /// told while the window has no area.
    fn surroundings(&self) -> Surroundings {
        let (safe_area, points_per_pixel) = fit(self.safe_px, &self.viewport)
            .unwrap_or((self.told.safe_area, self.told.points_per_pixel));
        Surroundings {
            device: self.last_device.device(),
            safe_area,
            points_per_pixel,
            soft_keyboard: false,
        }
    }

    /// Tells the screen the surroundings when they differ from what it
    /// was last told: after every event, and before every frame.
    pub(super) fn tell_surroundings(&mut self) -> Control {
        let now = self.surroundings();
        if now == self.told {
            return Control::Continue;
        }
        self.told = now;
        self.route(Input::Surroundings(now))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_view::ViewShape;
    use nova_view::view_shape::widened_size;

    use super::*;

    fn rect(x: u32, y: u32, width: u32, height: u32) -> PxRect {
        PxRect {
            x,
            y,
            width,
            height,
        }
    }

    fn whole(size_px: (u32, u32)) -> PxRect {
        rect(0, 0, size_px.0, size_px.1)
    }

    fn canvas(w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(0.0, 0.0), w, h)
    }

    #[test]
    fn a_whole_window_fit_is_the_whole_canvas() {
        let size = (2048, 1536);
        let viewport = Viewport::for_shape(ViewShape::Classic, size, 2.0);
        assert_eq!(
            fit(whole(size), &viewport),
            Some((canvas(1024.0, 768.0), 1.0))
        );
    }

    #[test]
    fn a_pillarboxed_viewport_cuts_only_the_inset_past_its_bars() {
        // Content 2048 wide at x 256, two pixels a unit.
        let size = (2560, 1536);
        let viewport = Viewport::for_shape(ViewShape::Classic, size, 2.0);
        let left_in_bar = rect(120, 0, 2560 - 120, 1536);
        let (safe, _) = fit(left_in_bar, &viewport).expect("an area");
        assert_eq!(safe, canvas(1024.0, 768.0), "the bar takes it all");
        let right_past_bar = rect(0, 0, 2560 - 400, 1536);
        let (safe, _) = fit(right_past_bar, &viewport).expect("an area");
        // 400 px in from the right is 144 px past the 256 px bar: 72 units.
        assert_eq!(safe, canvas(1024.0 - 72.0, 768.0));
    }

    #[test]
    fn a_letterboxed_viewport_cuts_only_the_inset_past_its_bars() {
        // An iPad Pro held upright: the classic canvas at y 598.
        let size = (2048, 2732);
        let viewport = Viewport::for_shape(ViewShape::Widened, size, 2.0);
        let insets = rect(0, 48, 2048, 2732 - 48 - 40);
        let (safe, points_per_pixel) = fit(insets, &viewport).expect("an area");
        assert_eq!(safe, canvas(1024.0, 768.0));
        assert_eq!(points_per_pixel, 1.0);
        let past = rect(0, 598 + 20, 2048, 2732 - 618);
        let (safe, _) = fit(past, &viewport).expect("an area");
        assert_eq!(safe.min, Point::new(0.0, 10.0), "20 px past the bar");
    }

    #[test]
    fn a_widened_viewport_takes_the_smaller_axis_scale() {
        let size = (2388, 1668);
        let viewport = Viewport::for_shape(ViewShape::Widened, size, 2.0);
        let (w, h) = widened_size(size);
        assert!(w > 1024, "widened: {w}");
        let (across, down) = viewport.axis_scales().expect("an area");
        assert_ne!(across, down);
        #[allow(clippy::cast_precision_loss)]
        let expected = (canvas(w as f32, h as f32), across.min(down) / 2.0);
        assert_eq!(fit(whole(size), &viewport), Some(expected));
    }

    #[test]
    fn points_per_pixel_follows_the_window_in_points_not_the_scale_factor_alone() {
        let small = (1400, 1050);
        let large = (2800, 2100);
        let at = |size: (u32, u32), scale: f64| {
            let viewport = Viewport::for_shape(ViewShape::Classic, size, scale);
            fit(whole(size), &viewport).expect("an area").1
        };
        let (small_ppp, large_ppp) = (at(small, 2.0), at(large, 2.0));
        assert!(
            (large_ppp - 2.0 * small_ppp).abs() < 1e-6,
            "{small_ppp} {large_ppp}"
        );
        assert!((small_ppp - 700.0 / 1024.0).abs() < 1e-6, "{small_ppp}");
        assert_eq!(at(large, 2.0), at((1400, 1050), 1.0), "the same points");
    }

    #[test]
    fn an_inset_past_the_content_clamps_to_the_canvas() {
        let size = (1024, 768);
        let viewport = Viewport::for_shape(ViewShape::Classic, size, 1.0);
        let wider = rect(0, 0, 4000, 4000);
        assert_eq!(fit(wider, &viewport), Some((canvas(1024.0, 768.0), 1.0)));
        let past = rect(2000, 0, 100, 768);
        let (safe, _) = fit(past, &viewport).expect("an area");
        assert_eq!(safe.width(), 0.0);
        assert_eq!(safe.min.x, 1024.0);
    }

    #[test]
    fn a_window_with_no_area_gives_nothing() {
        let viewport = Viewport::for_shape(ViewShape::Classic, (0, 0), 2.0);
        assert_eq!(fit(whole((0, 0)), &viewport), None);
    }
}
