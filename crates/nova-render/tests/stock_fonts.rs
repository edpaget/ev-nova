//! The player's own Charcoal through the wgpu adapter. Skips, passing, when
//! `NOVA_DATA` is unset, when the data has no `Fonts/Charcoal.ttf` beside
//! it, or with no GPU adapter.

mod common;

use nova_data::fonts::{charcoal_path, open_charcoal};
use nova_render::fonts::FontFaces;
use nova_render::wgpu::{InitError, OffscreenGpu};
use nova_render::{ImageError, ImageSource, LogicalSize, Renderer, Viewport};
use nova_view::{Color, DrawList, Font, ImageKind, Point};

/// No images: the frames here are text alone.
struct NoImages;

impl ImageSource for NoImages {
    fn frames(
        &self,
        _kind: ImageKind,
        _id: i16,
    ) -> Result<Vec<nova_data::graphics::Image>, ImageError> {
        Err(ImageError::Missing)
    }
}

/// One line of text.
const LINE: LogicalSize = LogicalSize { w: 128, h: 32 };

/// "EV Nova" with a Mac Roman accent, in `font` at `size` points, drawn with
/// `faces` at `scale` physical pixels per unit; `None` with no GPU adapter.
fn draw(faces: &FontFaces, font: Font, size: f32, scale: u32) -> Option<Vec<u8>> {
    let (w, h) = (LINE.w * scale, LINE.h * scale);
    let mut gpu = match OffscreenGpu::new(w, h, faces) {
        Ok(gpu) => gpu,
        Err(error @ (InitError::NoAdapter(_) | InitError::NoDevice(_))) => {
            eprintln!("skipping: no GPU adapter ({error})");
            return None;
        }
        Err(error) => panic!("offscreen GPU: {error}"),
    };
    assert_eq!(gpu.charcoal_loaded(), faces.charcoal().is_some());
    let mut list = DrawList::new();
    list.text_in(
        font,
        "EV Nova shïp",
        Point::new(2.0, 4.0),
        size,
        None,
        Color::WHITE,
    );
    Renderer::new(NoImages).render(
        &list,
        &Viewport::new(LINE, (w, h), f64::from(scale)),
        &mut gpu,
    );
    Some(gpu.read_pixels().expect("read back"))
}

#[test]
fn the_games_charcoal_draws_at_the_original_sizes_at_1x_and_2x() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    if !charcoal_path(&dir).is_file() {
        eprintln!("skipping: no {}", charcoal_path(&dir).display());
        return;
    }
    let charcoal = open_charcoal(&dir).expect("Charcoal loads");
    let with_charcoal = FontFaces::bundled().with_charcoal(charcoal);
    let fallback = FontFaces::bundled();
    for size in [9.0, 10.0, 12.0] {
        for scale in [1, 2] {
            let (Some(real), Some(substitute)) = (
                draw(&with_charcoal, Font::Charcoal, size, scale),
                draw(&fallback, Font::Charcoal, size, scale),
            ) else {
                return;
            };
            let lit = real.as_chunks::<4>().0.iter().filter(|p| p[0] > 0).count();
            assert!(lit > 0, "Charcoal {size}pt at {scale}x draws nothing");
            assert_ne!(
                real, substitute,
                "Charcoal {size}pt at {scale}x draws like the fallback"
            );
        }
    }
}
