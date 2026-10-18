//! The atlas holding enlarged images, on a real GPU: a native rebuild
//! draws exactly what a fresh renderer draws, and an enlarged atlas draws
//! every image at its own logical size.
//!
//! Needs a GPU adapter; each test skips, passing, without one.

use std::borrow::Cow;
use std::cell::Cell;
use std::rc::Rc;

use nova_data::graphics::Image;
use nova_render::fonts::FontFaces;
use nova_render::{
    Enlarger, ImageError, ImageSource, NativeDetail, Renderer, Viewport, wgpu::OffscreenGpu,
};
use nova_view::geometry::Bounds;
use nova_view::{Blend, Color, DrawCommand, DrawList, ImageKey, ImageKind, Point};

use super::flight_frame::{LOGICAL, Sprites, flight};
use super::wgpu_offscreen::gpu_with;

/// The target's side: logical 32x24 at scale 2, content (0, 8, 64, 48).
const SIZE: u32 = 64;

/// The `ppat` the scene tiles.
const PATTERN: i16 = 130;

/// [`Sprites`], with a 2x2 pattern of four colours (`ppat` 130).
struct Scene;

impl ImageSource for Scene {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        if (kind, id) == (ImageKind::Pattern, PATTERN) {
            let pixels = [
                [200, 30, 30, 255],
                [30, 200, 30, 255],
                [30, 30, 200, 255],
                [200, 200, 30, 255],
            ]
            .concat();
            return Ok(vec![Image::from_rgba(2, 2, pixels).expect("2x2")]);
        }
        Sprites.frames(kind, id)
    }
}

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// Pictures plain, stretched and clipped, sprites plain, tinted and
/// clipped, a tiled pattern, a line and a dot.
fn scene() -> DrawList {
    let mut list = DrawList::new();
    list.picture(ImageKey::picture(1), at(2.0, 2.0))
        .sprite(
            ImageKey::sprite(2, 0),
            at(16.0, 6.0),
            Color::rgba(0, 255, 0, 128),
        )
        .line(
            at(2.0, 20.0),
            at(30.0, 20.0),
            2.0,
            Color::rgba(0, 0, 255, 255),
        )
        .dot(at(28.0, 4.0), 4.0, Color::rgba(255, 255, 0, 255))
        .sprite(ImageKey::sprite(3, 0), at(24.0, 12.0), Color::WHITE)
        .stretched_picture(ImageKey::picture(1), at(20.0, 15.0), 8.0, 4.0)
        .tiled_picture(
            ImageKey::pattern(PATTERN),
            Bounds {
                min: at(2.0, 9.0),
                max: at(9.0, 15.0),
            },
        );
    let clip = Bounds {
        min: at(11.0, 10.0),
        max: at(14.0, 13.0),
    };
    list.clipped(
        clip,
        DrawCommand::Sprite {
            image: ImageKey::sprite(3, 0),
            center: at(12.0, 12.0),
            tint: Color::WHITE,
            blend: Blend::Normal,
        },
    );
    list
}

/// The scene's viewport: logical 32x24 at scale 2 in the square target.
fn viewport() -> Viewport {
    Viewport::new(LOGICAL, (SIZE, SIZE), 2.0)
}

/// A fresh offscreen GPU, or `None` (after a skip message) with no
/// adapter.
fn gpu() -> Option<OffscreenGpu> {
    gpu_with(SIZE, SIZE, &FontFaces::bundled())
}

/// `renderer`'s frame of `list`, drawn on `gpu` and read back, with no
/// failure.
fn frame(renderer: &mut Renderer<Scene>, list: &DrawList, gpu: &mut OffscreenGpu) -> Vec<u8> {
    let report = renderer.render(list, &viewport(), gpu);
    assert_eq!(report.new_failures, vec![]);
    gpu.read_pixels().expect("read back")
}

/// Whether any pixel of `pixels` is not opaque black.
fn anything_drawn(pixels: &[u8]) -> bool {
    pixels.chunks(4).any(|pixel| pixel != [0, 0, 0, 255])
}

/// Replicates each texel 2x2 for every kind, and counts its calls.
#[derive(Debug, Default)]
struct Replicated {
    calls: Rc<Cell<usize>>,
}

impl Enlarger for Replicated {
    fn enlarge<'a>(&self, _kind: ImageKind, image: &'a Image) -> Cow<'a, Image> {
        self.calls.set(self.calls.get() + 1);
        let (w, h) = (image.width(), image.height());
        let pixels = (0..h * 2)
            .flat_map(|y| (0..w * 2).flat_map(move |x| image.pixel(x / 2, y / 2).expect("inside")))
            .collect();
        Cow::Owned(Image::from_rgba(w * 2, h * 2, pixels).expect("doubled"))
    }
}

#[test]
fn a_native_rebuild_draws_exactly_what_the_fresh_renderer_drew() {
    for list in [scene(), flight()] {
        let Some(mut gpu) = gpu() else {
            return;
        };
        let mut renderer = Renderer::new(Scene);
        let fresh = frame(&mut renderer, &list, &mut gpu);
        assert!(anything_drawn(&fresh));

        renderer.set_enlarger(Box::new(NativeDetail));
        let rebuilt = frame(&mut renderer, &list, &mut gpu);

        assert!(fresh == rebuilt, "the rebuilt frame differs: {list:?}");
    }
}

#[test]
fn an_atlas_of_texels_replicated_2x2_draws_every_pixel_as_native_at_scale_2() {
    // At scale 2 each pixel samples one texel of the 2x2-replicated image,
    // the same colour as native's texel, so a quad of the wrong size or
    // texture coordinates off the bigger image moves some pixel.
    for list in [scene(), flight()] {
        let Some(mut gpu) = gpu() else {
            return;
        };
        let native = frame(&mut Renderer::new(Scene), &list, &mut gpu);
        let Some(mut gpu) = self::gpu() else {
            return;
        };
        let mut renderer = Renderer::new(Scene);
        let calls = Rc::new(Cell::new(0));
        renderer.set_enlarger(Box::new(Replicated {
            calls: Rc::clone(&calls),
        }));

        let enlarged = frame(&mut renderer, &list, &mut gpu);

        assert!(calls.get() > 0, "enlarged");
        assert!(anything_drawn(&native));
        assert!(native == enlarged, "the enlarged frame differs: {list:?}");
    }
}
