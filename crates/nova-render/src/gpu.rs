//! The GPU port and the frames the core hands it.
//!
//! A [`Frame`] is everything the GPU needs to draw one frame, already
//! decided by the core: the letterboxed viewport, and the draw list as
//! batches in strict draw order. Positions are logical; the adapter only
//! projects the logical space onto the viewport rectangle.

use nova_data::graphics::Image;
use nova_view::{Blend, Color, Font, Point};

pub use crate::atlas::{PageId, Uv};
use crate::viewport::{LogicalSize, PixelRect};

/// What the core needs from a GPU.
pub trait Gpu {
    /// Creates atlas page `page`, `size` x `size` transparent texels.
    fn create_page(&mut self, page: PageId, size: u32);
    /// Copies `image` onto page `page` at `at` (the image's size).
    fn upload(&mut self, page: PageId, at: PixelRect, image: &Image);
    /// Draws `frame` to the window or target.
    fn submit(&mut self, frame: &Frame);
}

/// An axis-aligned rectangle in logical units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

/// One textured quad: an atlas image drawn into `dest`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadInstance {
    /// Where it goes, in logical units.
    pub dest: Rect,
    /// The image's texture coordinates on its page.
    pub uv: Uv,
    /// RGBA multiplier, 0 to 1. In a [`Blend::Or`] batch it is instead
    /// each colour channel's level as a fraction of 32 (`n / 32`, exact in
    /// `f32`), with alpha 1.
    pub tint: [f32; 4],
}

/// One untextured quadrilateral, for lines and dots.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolidQuad {
    /// Its corners in logical units, in order around the edge.
    pub corners: [Point; 4],
    /// RGBA, 0 to 1.
    pub color: [f32; 4],
}

/// One run of text, already in physical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    /// The text.
    pub text: String,
    /// The font it is drawn in.
    pub font: Font,
    /// The top-left corner of its first line, in window pixels.
    pub origin_px: (f32, f32),
    /// The font size in pixels.
    pub size_px: f32,
    /// The distance between lines in pixels.
    pub line_height_px: f32,
    /// The wrap width in pixels, if it wraps.
    pub wrap_px: Option<f32>,
    /// The text colour.
    pub color: Color,
    /// Nothing is drawn outside this rectangle (the viewport).
    pub clip: PixelRect,
}

/// A run of same-kind draws, drawn in one go.
#[derive(Clone, Debug, PartialEq)]
pub enum Batch {
    /// Textured quads from one atlas page, all combined with what is
    /// beneath them the same way.
    Sprites {
        /// The page.
        page: PageId,
        /// How the quads combine with what is beneath them.
        blend: Blend,
        /// The quads, in draw order. In a [`Blend::Or`] batch no two
        /// overlap, and each tint holds levels in 32nds (see
        /// [`QuadInstance::tint`]).
        quads: Vec<QuadInstance>,
    },
    /// Untextured quads.
    Solid(Vec<SolidQuad>),
    /// Text runs.
    Text(Vec<TextRun>),
}

/// One frame for the GPU.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// The target's size in physical pixels.
    pub target: (u32, u32),
    /// The letterboxed content rectangle, in physical pixels.
    pub viewport: PixelRect,
    /// The logical space, projected onto `viewport`.
    pub logical: LogicalSize,
    /// The colour of the whole target before drawing (and so the bars).
    pub clear: Color,
    /// The batches, in draw order.
    pub batches: Vec<Batch>,
}

impl Frame {
    /// The target pixels `quads` can touch: the union of their rectangles
    /// projected onto the viewport, widened to whole pixels (a superset of
    /// the pixels whose centres they cover) and clipped to the viewport,
    /// outside which nothing is drawn. `None` when that is empty, as it is
    /// with no quads.
    #[must_use]
    pub fn pixels_covered(&self, quads: &[QuadInstance]) -> Option<PixelRect> {
        let view = self.viewport;
        let (vx, vy) = (view.x as f32, view.y as f32);
        let sx = view.w as f32 / self.logical.w as f32;
        let sy = view.h as f32 / self.logical.h as f32;
        let (mut x0, mut y0) = (f32::INFINITY, f32::INFINITY);
        let (mut x1, mut y1) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for quad in quads {
            let dest = quad.dest;
            x0 = x0.min(dest.x);
            y0 = y0.min(dest.y);
            x1 = x1.max(dest.x + dest.w);
            y1 = y1.max(dest.y + dest.h);
        }
        let left = (vx + x0 * sx).floor().max(vx);
        let top = (vy + y0 * sy).floor().max(vy);
        let right = (vx + x1 * sx).ceil().min(vx + view.w as f32);
        let bottom = (vy + y1 * sy).ceil().min(vy + view.h as f32);
        if left >= right || top >= bottom {
            return None;
        }
        Some(PixelRect {
            x: left as u32,
            y: top as u32,
            w: (right - left) as u32,
            h: (bottom - top) as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u32, y: u32, w: u32, h: u32) -> PixelRect {
        PixelRect { x, y, w, h }
    }

    /// A quad drawn into the logical rectangle (`x`, `y`, `w`, `h`).
    fn quad(x: f32, y: f32, w: f32, h: f32) -> QuadInstance {
        QuadInstance {
            dest: Rect { x, y, w, h },
            uv: Uv {
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            },
            tint: [1.0; 4],
        }
    }

    /// Logical 32x24 at scale 2 in a 64x64 target: content (0, 8, 64, 48).
    fn letterboxed() -> Frame {
        Frame {
            target: (64, 64),
            viewport: rect(0, 8, 64, 48),
            logical: LogicalSize { w: 32, h: 24 },
            clear: Color::BLACK,
            batches: Vec::new(),
        }
    }

    #[test]
    fn a_quad_covers_its_rectangle_projected_onto_the_viewport() {
        let frame = letterboxed();
        let one = quad(4.0, 4.0, 4.0, 4.0);
        assert_eq!(frame.pixels_covered(&[one]), Some(rect(8, 16, 8, 8)));
    }

    #[test]
    fn quads_cover_the_union_of_their_rectangles() {
        let frame = letterboxed();
        let (a, b) = (quad(4.0, 4.0, 4.0, 4.0), quad(20.0, 10.0, 2.0, 2.0));
        assert_eq!(frame.pixels_covered(&[a, b]), Some(rect(8, 16, 36, 16)));
        assert_eq!(frame.pixels_covered(&[b, a]), Some(rect(8, 16, 36, 16)));
    }

    #[test]
    fn a_fractional_rectangle_covers_every_pixel_it_touches() {
        let frame = letterboxed();
        let part = quad(4.25, 4.0, 1.0, 1.0);
        assert_eq!(frame.pixels_covered(&[part]), Some(rect(8, 16, 3, 2)));
    }

    #[test]
    fn coverage_is_clipped_to_the_viewport() {
        let frame = letterboxed();
        let top_left = quad(-2.0, -2.0, 4.0, 4.0);
        let bottom_right = quad(30.0, 22.0, 4.0, 4.0);
        assert_eq!(frame.pixels_covered(&[top_left]), Some(rect(0, 8, 4, 4)));
        assert_eq!(
            frame.pixels_covered(&[bottom_right]),
            Some(rect(60, 52, 4, 4))
        );
    }

    #[test]
    fn quads_off_the_viewport_or_none_at_all_cover_nothing() {
        let frame = letterboxed();
        assert_eq!(frame.pixels_covered(&[quad(40.0, 4.0, 4.0, 4.0)]), None);
        assert_eq!(frame.pixels_covered(&[quad(4.0, 30.0, 4.0, 4.0)]), None);
        // Touching the viewport's right or bottom edge from outside.
        assert_eq!(frame.pixels_covered(&[quad(32.0, 4.0, 4.0, 4.0)]), None);
        assert_eq!(frame.pixels_covered(&[quad(4.0, 24.0, 4.0, 4.0)]), None);
        assert_eq!(frame.pixels_covered(&[]), None);
    }

    #[test]
    fn a_pillarboxed_viewport_offsets_coverage_by_its_bar() {
        let frame = Frame {
            target: (80, 48),
            viewport: rect(8, 0, 64, 48),
            ..letterboxed()
        };
        let one = quad(4.0, 4.0, 4.0, 4.0);
        assert_eq!(frame.pixels_covered(&[one]), Some(rect(16, 8, 8, 8)));
    }
}
