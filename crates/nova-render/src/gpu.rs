//! The GPU port and the frames the core hands it.
//!
//! A [`Frame`] is everything the GPU needs to draw one frame, already
//! decided by the core: the letterboxed viewport, and the draw list as
//! batches in strict draw order. Positions are logical; the adapter only
//! projects the logical space onto the viewport rectangle.

use nova_data::graphics::Image;
use nova_view::{Color, Font, Point};

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
    /// RGBA multiplier, 0 to 1.
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
    /// Textured quads from one atlas page.
    Sprites {
        /// The page.
        page: PageId,
        /// The quads, in draw order.
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
