//! The GPU port and the frames the core hands it.
//!
//! A [`Frame`] is everything the GPU needs to draw one frame, already
//! decided by the core: the letterboxed viewport, the draw list as
//! batches in strict draw order, which of them are the world layer
//! ([`DrawLayer`]) and where that layer is drawn ([`WorldSpace`]).
//! Positions are logical; the adapter only projects the logical space
//! onto the viewport rectangle.

use nova_data::graphics::Image;
use nova_view::{Blend, Color, DrawLayer, Font, Point};

pub use crate::atlas::{PageId, Uv};
use crate::upscale::WorldResolution;
use crate::viewport::{LogicalSize, PixelRect};

/// What the core needs from a GPU.
pub trait Gpu {
    /// Creates atlas page `page`, `size` x `size` transparent texels.
    fn create_page(&mut self, page: PageId, size: u32);
    /// Copies `image` onto page `page` at `at` (the image's size).
    fn upload(&mut self, page: PageId, at: PixelRect, image: &Image);
    /// Draws `frame` to the window or target.
    fn submit(&mut self, frame: &Frame);
    /// The resolution it draws the world layer at, which decides each
    /// frame's [`WorldSpace`] ([`world_space`](crate::world_space)).
    fn world_resolution(&self) -> WorldResolution;
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

/// Where a frame's world layer ([`DrawLayer::World`]) is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldSpace {
    /// Straight into the target, projected onto the viewport at the
    /// target's resolution, as the overlay is. The native upscaler draws
    /// the world so, which changes nothing: every batch is drawn in the
    /// frame's order, whatever its layer.
    Target,
    /// Into a texture of its own, `size` pixels, the logical space filling
    /// it at one texel a unit, for an upscaler to enlarge onto the
    /// viewport (each axis by its own scale in a widened view, and a
    /// shrink in a window smaller than the logical space), with the
    /// overlay drawn over it.
    Texture {
        /// The texture's width and height in texels: the logical size.
        size: (u32, u32),
    },
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
    /// Where the world layer is drawn.
    pub world_space: WorldSpace,
    /// The batches, in draw order, both layers'.
    pub batches: Vec<Batch>,
    /// Which of `batches` are in the world layer, by index, ascending;
    /// the rest are overlay. Empty on every screen but flight.
    pub world: Vec<usize>,
}

impl Frame {
    /// The layer of the batch at `index`.
    #[must_use]
    pub fn layer(&self, index: usize) -> DrawLayer {
        if self.world.binary_search(&index).is_ok() {
            DrawLayer::World
        } else {
            DrawLayer::Overlay
        }
    }

    /// The batches in `layer`, in draw order.
    pub fn in_layer(&self, layer: DrawLayer) -> impl Iterator<Item = &Batch> {
        self.batches
            .iter()
            .enumerate()
            .filter(move |&(index, _)| self.layer(index) == layer)
            .map(|(_, batch)| batch)
    }

    /// The pixels `quads` drawn in `layer` can touch, in that layer's
    /// pixels: the union of their rectangles projected onto the pixels
    /// the layer's logical space fills, widened to whole pixels (a
    /// superset of the pixels whose centres they cover) and clipped to
    /// those pixels, outside which nothing is drawn. `None` when that is
    /// empty, as it is with no quads.
    ///
    /// The overlay fills the viewport, in target pixels. The world fills
    /// it too when it is drawn into the target ([`WorldSpace::Target`]);
    /// drawn into a texture ([`WorldSpace::Texture`]), it fills the whole
    /// texture, in texels. An overlay quad over the world reads the world
    /// once it is enlarged into the target.
    #[must_use]
    pub fn pixels_covered(&self, layer: DrawLayer, quads: &[QuadInstance]) -> Option<PixelRect> {
        let onto = match (layer, self.world_space) {
            (DrawLayer::World, WorldSpace::Texture { size: (w, h) }) => {
                PixelRect { x: 0, y: 0, w, h }
            }
            _ => self.viewport,
        };
        let (vx, vy) = (onto.x as f32, onto.y as f32);
        let sx = onto.w as f32 / self.logical.w as f32;
        let sy = onto.h as f32 / self.logical.h as f32;
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
        let right = (vx + x1 * sx).ceil().min(vx + onto.w as f32);
        let bottom = (vy + y1 * sy).ceil().min(vy + onto.h as f32);
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

    /// Logical 32x24 at scale 2 in a 64x64 target: content (0, 8, 64, 48),
    /// its world drawn straight into it.
    fn letterboxed() -> Frame {
        Frame {
            target: (64, 64),
            viewport: rect(0, 8, 64, 48),
            logical: LogicalSize { w: 32, h: 24 },
            clear: Color::BLACK,
            world_space: WorldSpace::Target,
            batches: Vec::new(),
            world: Vec::new(),
        }
    }

    /// [`letterboxed`], its world drawn into a 32x24 texture of its own.
    fn textured() -> Frame {
        Frame {
            world_space: WorldSpace::Texture { size: (32, 24) },
            ..letterboxed()
        }
    }

    const OVERLAY: DrawLayer = DrawLayer::Overlay;
    const WORLD: DrawLayer = DrawLayer::World;

    #[test]
    fn a_quad_covers_its_rectangle_projected_onto_the_viewport() {
        let frame = letterboxed();
        let one = quad(4.0, 4.0, 4.0, 4.0);
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[one]),
            Some(rect(8, 16, 8, 8))
        );
    }

    #[test]
    fn quads_cover_the_union_of_their_rectangles() {
        let frame = letterboxed();
        let (a, b) = (quad(4.0, 4.0, 4.0, 4.0), quad(20.0, 10.0, 2.0, 2.0));
        let union = Some(rect(8, 16, 36, 16));
        assert_eq!(frame.pixels_covered(OVERLAY, &[a, b]), union);
        assert_eq!(frame.pixels_covered(OVERLAY, &[b, a]), union);
    }

    #[test]
    fn a_fractional_rectangle_covers_every_pixel_it_touches() {
        let frame = letterboxed();
        let part = quad(4.25, 4.0, 1.0, 1.0);
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[part]),
            Some(rect(8, 16, 3, 2))
        );
    }

    #[test]
    fn coverage_is_clipped_to_the_viewport() {
        let frame = letterboxed();
        let top_left = quad(-2.0, -2.0, 4.0, 4.0);
        let bottom_right = quad(30.0, 22.0, 4.0, 4.0);
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[top_left]),
            Some(rect(0, 8, 4, 4))
        );
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[bottom_right]),
            Some(rect(60, 52, 4, 4))
        );
    }

    #[test]
    fn quads_off_the_viewport_or_none_at_all_cover_nothing() {
        let frame = letterboxed();
        let covered = |quads: &[QuadInstance]| frame.pixels_covered(OVERLAY, quads);
        assert_eq!(covered(&[quad(40.0, 4.0, 4.0, 4.0)]), None);
        assert_eq!(covered(&[quad(4.0, 30.0, 4.0, 4.0)]), None);
        // Touching the viewport's right or bottom edge from outside.
        assert_eq!(covered(&[quad(32.0, 4.0, 4.0, 4.0)]), None);
        assert_eq!(covered(&[quad(4.0, 24.0, 4.0, 4.0)]), None);
        assert_eq!(covered(&[]), None);
    }

    #[test]
    fn a_pillarboxed_viewport_offsets_coverage_by_its_bar() {
        let frame = Frame {
            target: (80, 48),
            viewport: rect(8, 0, 64, 48),
            ..letterboxed()
        };
        let one = quad(4.0, 4.0, 4.0, 4.0);
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[one]),
            Some(rect(16, 8, 8, 8))
        );
    }

    #[test]
    fn a_world_drawn_into_the_target_covers_what_the_overlay_does() {
        let frame = letterboxed();
        let one = quad(4.0, 4.0, 4.0, 4.0);
        assert_eq!(frame.pixels_covered(WORLD, &[one]), Some(rect(8, 16, 8, 8)));
        let part = quad(4.25, 4.0, 1.0, 1.0);
        assert_eq!(
            frame.pixels_covered(WORLD, &[part]),
            Some(rect(8, 16, 3, 2))
        );
    }

    #[test]
    fn a_world_drawn_into_a_texture_covers_its_texels_and_the_overlay_target_pixels() {
        let frame = textured();
        let one = quad(4.0, 4.0, 4.0, 4.0);
        // One texel a logical unit.
        assert_eq!(frame.pixels_covered(WORLD, &[one]), Some(rect(4, 4, 4, 4)));
        // Twice that, down the letterbox's bar, in the target.
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[one]),
            Some(rect(8, 16, 8, 8))
        );
        // (4, 4)-(5.25, 5): every texel it touches.
        let part = quad(4.25, 4.0, 1.0, 1.0);
        assert_eq!(frame.pixels_covered(WORLD, &[part]), Some(rect(4, 4, 2, 1)));
    }

    #[test]
    fn an_overlay_quad_over_the_world_covers_the_world_quads_texels_enlarged() {
        // The world quad's texels, (4, 4, 4, 4) of a 32x24 texture,
        // enlarged onto the viewport (0, 8, 64, 48) at 2 a texel, land on
        // (0 + 4 x 2, 8 + 4 x 2, 4 x 2, 4 x 2) = (8, 16, 8, 8) of the
        // target: what the overlay's OR quad over it reads.
        let frame = textured();
        let one = quad(4.0, 4.0, 4.0, 4.0);
        assert_eq!(frame.pixels_covered(WORLD, &[one]), Some(rect(4, 4, 4, 4)));
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[one]),
            Some(rect(8, 16, 8, 8))
        );
    }

    #[test]
    fn a_textured_worlds_coverage_is_clipped_to_its_texture() {
        let frame = textured();
        let top_left = quad(-2.0, -2.0, 4.0, 4.0);
        let bottom_right = quad(30.0, 22.0, 4.0, 4.0);
        assert_eq!(
            frame.pixels_covered(WORLD, &[top_left]),
            Some(rect(0, 0, 2, 2))
        );
        assert_eq!(
            frame.pixels_covered(WORLD, &[bottom_right]),
            Some(rect(30, 22, 2, 2))
        );
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[bottom_right]),
            Some(rect(60, 52, 4, 4)),
            "the overlay's, to the viewport"
        );
        let covered = |quads: &[QuadInstance]| frame.pixels_covered(WORLD, quads);
        assert_eq!(covered(&[quad(32.0, 4.0, 4.0, 4.0)]), None);
        assert_eq!(covered(&[quad(4.0, 24.0, 4.0, 4.0)]), None);
        assert_eq!(covered(&[quad(-8.0, 4.0, 4.0, 4.0)]), None);
        assert_eq!(covered(&[]), None);
    }

    #[test]
    fn a_widened_worlds_texels_are_enlarged_per_axis_onto_the_target() {
        // 1365x768 logical filling a 1920x1080 window.
        let frame = Frame {
            target: (1920, 1080),
            viewport: rect(0, 0, 1920, 1080),
            logical: LogicalSize { w: 1365, h: 768 },
            world_space: WorldSpace::Texture { size: (1365, 768) },
            ..letterboxed()
        };
        let one = quad(100.0, 100.0, 10.0, 10.0);
        assert_eq!(
            frame.pixels_covered(WORLD, &[one]),
            Some(rect(100, 100, 10, 10))
        );
        // Across, 100 and 110 x 1920 / 1365 are 140.66 and 154.73; down,
        // x 1080 / 768, 140.63 and 154.69: 140 to 155 both ways.
        assert_eq!(
            frame.pixels_covered(OVERLAY, &[one]),
            Some(rect(140, 140, 15, 15))
        );
    }

    #[test]
    fn a_batch_is_world_only_when_the_frame_says() {
        let solid = |n: f32| {
            Batch::Solid(vec![SolidQuad {
                corners: [Point::new(n, n); 4],
                color: [1.0; 4],
            }])
        };
        let frame = Frame {
            batches: vec![solid(0.0), solid(1.0), solid(2.0), solid(3.0)],
            world: vec![1, 2],
            ..letterboxed()
        };
        assert_eq!(
            (0..4).map(|at| frame.layer(at)).collect::<Vec<_>>(),
            [OVERLAY, WORLD, WORLD, OVERLAY]
        );
        assert_eq!(
            frame.in_layer(WORLD).collect::<Vec<_>>(),
            [&solid(1.0), &solid(2.0)]
        );
        assert_eq!(
            frame.in_layer(OVERLAY).collect::<Vec<_>>(),
            [&solid(0.0), &solid(3.0)]
        );
        assert_eq!(letterboxed().in_layer(WORLD).count(), 0);
    }
}
