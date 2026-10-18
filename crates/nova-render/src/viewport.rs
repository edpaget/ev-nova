//! The logical coordinate space of each screen's shape, and its fit into
//! the window.
//!
//! Screens draw in a logical space whose size is a per-screen shape
//! ([`ViewShape`]; one logical unit is one source image pixel). The
//! classic shape is [`LOGICAL`], 1024x768, Nova's largest original mode:
//! the [`Viewport`] scales it into the window by the largest factor that
//! keeps its aspect ratio, centres it on whole physical pixels and leaves
//! the rest as bars. Flight's widened shape is 768 high and as wide as the
//! window's aspect ratio makes it ([`widened_size`]), so in a window at
//! least 4:3 it fills the whole window ([`Viewport::for_shape`]); each
//! axis then has its own scale, which differ by under half a logical unit
//! over the width.

use nova_view::Point;
use nova_view::ViewShape;
use nova_view::view_shape::widened_size;

/// The size of the logical space, in logical units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LogicalSize {
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// Nova's largest original screen mode, the logical space screens draw in.
pub const LOGICAL: LogicalSize = LogicalSize { w: 1024, h: 768 };

/// A rectangle of whole pixels: physical window pixels or atlas texels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PixelRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// Where the logical space lands in the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Letterbox {
    /// The window has no area (it is minimised): nothing is drawn.
    None,
    /// The logical space fills `rect`, at `scale` physical pixels per
    /// logical unit; the rest of the window is bars.
    Content {
        /// The content rectangle, in physical pixels.
        rect: PixelRect,
        /// Physical pixels per logical unit.
        scale: f32,
    },
}

/// The logical space fitted into a window of a given physical size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    logical: LogicalSize,
    window_px: (u32, u32),
    scale_factor: f64,
    letterbox: Letterbox,
    /// Physical pixels per logical unit across and down: both the
    /// letterbox's scale, except in a widened view filling the window.
    axis_scales: (f32, f32),
}

impl Viewport {
    /// Fits `logical` into a window of `window_px` physical pixels whose
    /// display has `scale_factor` physical pixels per point.
    ///
    /// The scale is the largest that fits both dimensions; the content
    /// size is rounded to whole pixels and centred with the spare pixels
    /// floor-divided between the bars.
    #[must_use]
    pub fn new(logical: LogicalSize, window_px: (u32, u32), scale_factor: f64) -> Self {
        let (w_px, h_px) = window_px;
        let letterbox = if w_px == 0 || h_px == 0 {
            Letterbox::None
        } else {
            let scale = (w_px as f32 / logical.w as f32).min(h_px as f32 / logical.h as f32);
            let w = (logical.w as f32 * scale).round() as u32;
            let h = (logical.h as f32 * scale).round() as u32;
            let rect = PixelRect {
                x: (w_px - w) / 2,
                y: (h_px - h) / 2,
                w,
                h,
            };
            Letterbox::Content { rect, scale }
        };
        let axis_scales = match letterbox {
            Letterbox::None => (0.0, 0.0),
            Letterbox::Content { scale, .. } => (scale, scale),
        };
        Self {
            logical,
            window_px,
            scale_factor,
            letterbox,
            axis_scales,
        }
    }

    /// Fits `shape` into a window of `window_px` physical pixels whose
    /// display has `scale_factor` physical pixels per point.
    ///
    /// [`ViewShape::Classic`] is [`LOGICAL`] fitted as [`Viewport::new`]
    /// fits it. [`ViewShape::Widened`] is [`widened_size`]: in a window
    /// wider than 4:3 its content is the whole window, each axis scaled to
    /// fill it, so no bars and no rounding slivers show, the letterbox's
    /// scale being the vertical one, which text is sized by; in a window
    /// 4:3 or taller it is 1024 wide, the classic viewport.
    #[must_use]
    pub fn for_shape(shape: ViewShape, window_px: (u32, u32), scale_factor: f64) -> Self {
        let (w, h) = match shape {
            ViewShape::Classic => (LOGICAL.w, LOGICAL.h),
            ViewShape::Widened => widened_size(window_px),
        };
        if w <= LOGICAL.w {
            return Self::new(LOGICAL, window_px, scale_factor);
        }
        let (w_px, h_px) = window_px;
        let across = w_px as f32 / w as f32;
        let down = h_px as f32 / h as f32;
        Self {
            logical: LogicalSize { w, h },
            window_px,
            scale_factor,
            letterbox: Letterbox::Content {
                rect: PixelRect {
                    x: 0,
                    y: 0,
                    w: w_px,
                    h: h_px,
                },
                scale: down,
            },
            axis_scales: (across, down),
        }
    }

    /// Physical pixels per logical unit across and down, or `None` in a
    /// zero-area window. Both are the letterbox's scale except in a
    /// widened view filling a window wider than 4:3
    /// ([`Viewport::for_shape`]).
    #[must_use]
    pub fn axis_scales(&self) -> Option<(f32, f32)> {
        self.content().map(|_| self.axis_scales)
    }

    /// Where the logical space lands.
    #[must_use]
    pub fn letterbox(&self) -> Letterbox {
        self.letterbox
    }

    /// The content rectangle and scale, or `None` in a zero-area window.
    #[must_use]
    pub fn content(&self) -> Option<(PixelRect, f32)> {
        match self.letterbox {
            Letterbox::None => None,
            Letterbox::Content { rect, scale } => Some((rect, scale)),
        }
    }

    /// The logical space's size.
    #[must_use]
    pub fn logical(&self) -> LogicalSize {
        self.logical
    }

    /// The window's size in physical pixels.
    #[must_use]
    pub fn window_px(&self) -> (u32, u32) {
        self.window_px
    }

    /// The display's physical pixels per point.
    #[must_use]
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// The window's size in points.
    #[must_use]
    pub fn points(&self) -> (f64, f64) {
        (
            f64::from(self.window_px.0) / self.scale_factor,
            f64::from(self.window_px.1) / self.scale_factor,
        )
    }

    /// The logical point under window pixel position `px`, or `None` when
    /// it is in a bar (or the window has no area).
    #[must_use]
    pub fn window_to_logical(&self, px: (f64, f64)) -> Option<Point> {
        let (rect, _) = self.content()?;
        let (across, down) = self.axis_scales;
        let x = px.0 - f64::from(rect.x);
        let y = px.1 - f64::from(rect.y);
        let inside = (0.0..f64::from(rect.w)).contains(&x) && (0.0..f64::from(rect.h)).contains(&y);
        inside.then(|| Point::new((x / f64::from(across)) as f32, (y / f64::from(down)) as f32))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use nova_view::ViewShape;

    fn rect(x: u32, y: u32, w: u32, h: u32) -> PixelRect {
        PixelRect { x, y, w, h }
    }

    fn content(viewport: &Viewport) -> (PixelRect, f32) {
        match viewport.letterbox() {
            Letterbox::Content { rect, scale } => (rect, scale),
            Letterbox::None => panic!("no content in {viewport:?}"),
        }
    }

    #[test]
    fn the_default_logical_space_is_1024_by_768() {
        assert_eq!(LOGICAL, LogicalSize { w: 1024, h: 768 });
    }

    #[test]
    fn a_window_the_logical_size_fills_exactly() {
        let viewport = Viewport::new(LOGICAL, (1024, 768), 1.0);
        assert_eq!(content(&viewport), (rect(0, 0, 1024, 768), 1.0));
        assert_eq!(viewport.points(), (1024.0, 768.0));
        assert_eq!(viewport.window_px(), (1024, 768));
        assert_eq!(viewport.logical(), LOGICAL);
    }

    #[test]
    fn a_retina_window_doubles_the_scale_without_bars() {
        let viewport = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(content(&viewport), (rect(0, 0, 2048, 1536), 2.0));
        assert_eq!(viewport.points(), (1024.0, 768.0));
        assert_eq!(viewport.scale_factor(), 2.0);
    }

    #[test]
    fn a_wide_window_is_pillarboxed() {
        let viewport = Viewport::new(LOGICAL, (1280, 768), 1.0);
        assert_eq!(content(&viewport), (rect(128, 0, 1024, 768), 1.0));
        let retina = Viewport::new(LOGICAL, (2560, 1536), 2.0);
        assert_eq!(content(&retina), (rect(256, 0, 2048, 1536), 2.0));
    }

    #[test]
    fn a_tall_window_is_letterboxed() {
        let viewport = Viewport::new(LOGICAL, (1024, 1024), 1.0);
        assert_eq!(content(&viewport), (rect(0, 128, 1024, 768), 1.0));
        let retina = Viewport::new(LOGICAL, (2048, 2048), 2.0);
        assert_eq!(content(&retina), (rect(0, 256, 2048, 1536), 2.0));
    }

    #[test]
    fn an_odd_spare_pixel_is_floored_out_of_the_offset() {
        let viewport = Viewport::new(LOGICAL, (1025, 768), 1.0);
        assert_eq!(content(&viewport), (rect(0, 0, 1024, 768), 1.0));
        let viewport = Viewport::new(LOGICAL, (1024, 771), 1.0);
        assert_eq!(content(&viewport), (rect(0, 1, 1024, 768), 1.0));
    }

    #[test]
    fn a_smaller_window_scales_down_fractionally() {
        let viewport = Viewport::new(LOGICAL, (800, 600), 1.0);
        assert_eq!(content(&viewport), (rect(0, 0, 800, 600), 0.781_25));
    }

    #[test]
    fn content_size_rounds_to_the_nearest_pixel() {
        // Scale 1.25: 2 logical rows are 2.5 px, which rounds up to 3.
        let half_up = LogicalSize { w: 4, h: 2 };
        assert_eq!(
            content(&Viewport::new(half_up, (5, 10), 1.0)),
            (rect(0, 3, 5, 3), 1.25)
        );
        // Scale 1.125: 3 logical rows are 3.375 px, which rounds down to 3.
        let down = LogicalSize { w: 8, h: 3 };
        assert_eq!(
            content(&Viewport::new(down, (9, 10), 1.0)),
            (rect(0, 3, 9, 3), 1.125)
        );
    }

    #[test]
    fn a_zero_area_window_has_no_content() {
        assert_eq!(
            Viewport::new(LOGICAL, (0, 0), 1.0).letterbox(),
            Letterbox::None
        );
        assert_eq!(
            Viewport::new(LOGICAL, (0, 768), 1.0).letterbox(),
            Letterbox::None
        );
        assert_eq!(
            Viewport::new(LOGICAL, (1024, 0), 1.0).letterbox(),
            Letterbox::None
        );
        assert_eq!(Viewport::new(LOGICAL, (0, 0), 1.0).content(), None);
    }

    #[test]
    fn content_is_the_rect_and_scale() {
        let viewport = Viewport::new(LOGICAL, (1280, 768), 1.0);
        assert_eq!(viewport.content(), Some((rect(128, 0, 1024, 768), 1.0)));
    }

    #[test]
    fn a_content_point_maps_back_to_logical() {
        let viewport = Viewport::new(LOGICAL, (1280, 768), 1.0);
        assert_eq!(
            viewport.window_to_logical((128.0, 0.0)),
            Some(Point::new(0.0, 0.0))
        );
        assert_eq!(
            viewport.window_to_logical((640.5, 384.25)),
            Some(Point::new(512.5, 384.25))
        );
        assert_eq!(
            viewport.window_to_logical((1151.5, 767.5)),
            Some(Point::new(1023.5, 767.5))
        );
    }

    #[test]
    fn a_point_in_a_bar_is_none() {
        let pillar = Viewport::new(LOGICAL, (1280, 768), 1.0);
        assert_eq!(pillar.window_to_logical((127.5, 10.0)), None);
        assert_eq!(pillar.window_to_logical((1152.0, 10.0)), None);
        let letter = Viewport::new(LOGICAL, (1024, 1024), 1.0);
        assert_eq!(letter.window_to_logical((10.0, 127.5)), None);
        assert_eq!(letter.window_to_logical((10.0, 896.0)), None);
        assert_eq!(
            letter.window_to_logical((10.0, 128.0)),
            Some(Point::new(10.0, 0.0))
        );
        assert_eq!(
            letter.window_to_logical((10.0, 895.5)),
            Some(Point::new(10.0, 767.5))
        );
    }

    #[test]
    fn nothing_maps_in_a_zero_area_window() {
        let viewport = Viewport::new(LOGICAL, (0, 0), 1.0);
        assert_eq!(viewport.window_to_logical((0.0, 0.0)), None);
    }

    #[test]
    fn a_widened_shape_in_a_4_by_3_window_is_the_classic_viewport() {
        for window in [(1024, 768), (2048, 1536), (800, 600), (1440, 1080)] {
            for scale_factor in [1.0, 2.0] {
                assert_eq!(
                    Viewport::for_shape(ViewShape::Widened, window, scale_factor),
                    Viewport::new(LOGICAL, window, scale_factor),
                    "{window:?} at {scale_factor}"
                );
            }
        }
    }

    #[test]
    fn a_widened_shape_fills_a_wider_window_whole() {
        let viewport = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
        assert_eq!(viewport.logical(), LogicalSize { w: 1365, h: 768 });
        assert_eq!(content(&viewport).0, rect(0, 0, 1920, 1080));
        for (window, scale_factor, width) in [
            ((2560, 1440), 2.0, 1365),
            ((3840, 2160), 2.0, 1365),
            ((3440, 1440), 1.0, 1835),
            ((2880, 1800), 2.0, 1229),
        ] {
            let viewport = Viewport::for_shape(ViewShape::Widened, window, scale_factor);
            assert_eq!(viewport.logical(), LogicalSize { w: width, h: 768 });
            assert_eq!(
                content(&viewport).0,
                rect(0, 0, window.0, window.1),
                "{window:?}"
            );
            assert_eq!(viewport.window_px(), window);
            assert_eq!(viewport.scale_factor(), scale_factor);
        }
    }

    #[test]
    fn a_widened_shape_scales_each_axis_to_the_window_and_text_by_the_height() {
        let viewport = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
        assert_eq!(content(&viewport).1, 1.406_25, "the vertical scale");
        assert_eq!(viewport.axis_scales(), Some((1920.0 / 1365.0, 1.406_25)));
        let classic = Viewport::new(LOGICAL, (1280, 768), 1.0);
        assert_eq!(classic.axis_scales(), Some((1.0, 1.0)));
        assert_eq!(Viewport::new(LOGICAL, (0, 0), 1.0).axis_scales(), None);
    }

    #[test]
    fn a_widened_shape_in_a_taller_window_is_letterboxed() {
        let viewport = Viewport::for_shape(ViewShape::Widened, (1024, 1280), 1.0);
        assert_eq!(viewport, Viewport::new(LOGICAL, (1024, 1280), 1.0));
        assert_eq!(content(&viewport).0, rect(0, 256, 1024, 768));
    }

    #[test]
    fn a_classic_shape_is_pillarboxed_in_a_wide_window() {
        let viewport = Viewport::for_shape(ViewShape::Classic, (1920, 1080), 1.0);
        assert_eq!(viewport, Viewport::new(LOGICAL, (1920, 1080), 1.0));
        assert_eq!(content(&viewport).0, rect(240, 0, 1440, 1080));
    }

    #[test]
    fn a_widened_shape_with_no_area_draws_nothing() {
        let viewport = Viewport::for_shape(ViewShape::Widened, (0, 0), 1.0);
        assert_eq!(viewport.letterbox(), Letterbox::None);
    }

    #[test]
    fn every_pixel_of_a_widened_window_maps_into_the_logical_space() {
        let viewport = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
        assert_eq!(
            viewport.window_to_logical((0.0, 540.0)),
            Some(Point::new(0.0, 384.0))
        );
        let right = viewport.window_to_logical((1919.5, 540.0)).expect("inside");
        assert!(1364.0 < right.x && right.x < 1365.0, "{right:?}");
        assert_eq!(right.y, 384.0);
        for px in [0.0, 5.0, 960.0, 1900.0, 1919.99] {
            for py in [0.0, 1079.99] {
                assert!(
                    viewport.window_to_logical((px, py)).is_some(),
                    "({px}, {py})"
                );
            }
        }
        let at = viewport.window_to_logical((1900.0, 540.0)).expect("inside");
        assert!((at.x - 1350.78).abs() < 0.01, "{at:?}");
    }

    #[test]
    fn the_round_trip_holds_at_retina_scale() {
        let viewport = Viewport::new(LOGICAL, (2560, 1536), 2.0);
        let (rect, scale) = content(&viewport);
        let logical = Point::new(100.0, 50.0);
        let px = (
            f64::from(rect.x) + f64::from(logical.x * scale),
            f64::from(rect.y) + f64::from(logical.y * scale),
        );
        assert_eq!(px, (456.0, 100.0));
        assert_eq!(viewport.window_to_logical(px), Some(logical));
    }
}
