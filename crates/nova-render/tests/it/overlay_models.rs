//! A model drawn only in the overlay, as a screen's picture draws one
//! (`nova_view::model_picture`): an opaque UI fill, a cube fitted inside
//! it by its mesh's radius, then a text run, with no world at all.
//!
//! - On [`RecordingGpu`]: the cube is one models batch that the frame's
//!   world indices don't name, so it is drawn with the overlay.
//! - Offscreen: the cube's lit faces land only inside the fill's
//!   rectangle and over it, the fill shows round it, and the frame is the
//!   same pixels at every render scale and under the native and
//!   sharp-bilinear upscalers, so the overlay's model is drawn at the
//!   window's own resolution, which neither touches. Needs a GPU adapter;
//!   skips, passing, without one.

use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::recording::{NoImages, RecordingGpu};
use nova_render::{Gpu, Light, Mesh, RenderReport, RenderScale, Renderer, Upscaler, Viewport};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawLayer, DrawList, ModelKey, ModelPose, Orientation, Point};

use super::common::fill;
use super::common::pixels::{LOGICAL, SIZE, TOP, assert_near, color_at, pixel};
use super::wgpu_offscreen::{Images, gpu_with};

const CUBE: ModelKey = ModelKey(1);

/// The cube's faces' colour.
const FACE: [u8; 3] = [230, 230, 230];
/// The UI fill's colour.
const PANEL: [u8; 3] = [30, 60, 160];

/// The picture's rectangle, logical: x 4 to 20, y 4 to 16.
const AREA: Bounds = Bounds {
    min: Point { x: 4.0, y: 4.0 },
    max: Point { x: 20.0, y: 16.0 },
};

/// How much of the rectangle's shorter side the cube's radius spans, as
/// a screen's picture fits a model (`nova_view::model_picture::FIT`).
const FIT: f32 = 0.8;

fn color([r, g, b]: [u8; 3]) -> Color {
    Color::rgba(r, g, b, 255)
}

fn cube() -> Mesh {
    Mesh::cube([color(FACE); 6])
}

/// The cube's turn: 30° about x, so +z and +y face the camera.
fn turn() -> Orientation {
    Orientation::about([1.0, 0.0, 0.0], std::f32::consts::FRAC_PI_6)
}

/// The cube at the rectangle's centre, its radius reaching [`FIT`] of
/// half the rectangle's shorter side.
fn cube_pose() -> ModelPose {
    let side = AREA.width().min(AREA.height());
    ModelPose {
        center: AREA.center(),
        orientation: turn(),
        scale: FIT * side / 2.0 / cube().radius(),
    }
}

/// The overlay alone: the fill, the cube (if `with_cube`), then text.
fn scene(with_cube: bool) -> DrawList {
    let mut list = DrawList::new();
    fill(&mut list, AREA, color(PANEL));
    if with_cube {
        list.model(CUBE, cube_pose());
    }
    list.text("A", Point::new(24.0, 14.0), 6.0, None, Color::WHITE);
    list
}

#[test]
fn an_overlay_model_is_one_models_batch_outside_the_world() {
    let mut renderer = Renderer::new(NoImages);
    renderer.add_model(CUBE, cube());
    let mut gpu = RecordingGpu::new();
    let viewport = Viewport::new(LOGICAL, (SIZE, SIZE), 2.0);
    let report = renderer.render(&scene(true), &viewport, &mut gpu);
    assert!(report.missing_models.is_empty());
    let frame = gpu.submits()[0].clone();
    let kinds: Vec<&str> = frame.batches.iter().map(nova_render::Batch::kind).collect();
    let models: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|&(_, kind)| *kind == "models")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(models.len(), 1, "{kinds:?}");
    assert_eq!(frame.world, [0usize; 0], "{kinds:?}");
    assert_eq!(frame.layer(models[0]), DrawLayer::Overlay);
    assert_eq!(frame.in_layer(DrawLayer::World).count(), 0);
    let solid = kinds.iter().position(|kind| *kind == "solid");
    let text = kinds.iter().position(|kind| *kind == "text");
    assert!(
        solid < Some(models[0]) && Some(models[0]) < text,
        "{kinds:?}"
    );
}

/// The scene drawn under `upscaler` at `scale`, read back; `None` with no
/// adapter.
fn drawn(with_cube: bool, upscaler: Upscaler, scale: RenderScale) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    gpu.set_upscaler(upscaler);
    let mut renderer = Renderer::new(Images);
    renderer.add_model(CUBE, cube());
    renderer.set_render_scale(scale);
    let viewport = Viewport::new(LOGICAL, (SIZE, SIZE), 2.0);
    let report = renderer.render(&scene(with_cube), &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// The cube's +z face as the core shades it under the standard light.
fn lit_face() -> [u8; 3] {
    let normal = turn().rotate([0.0, 0.0, 1.0]).expect("a unit turn");
    let [r, g, b] = FACE.map(|c| f32::from(c) / 255.0);
    let [r, g, b, _] = shade(normal, [r, g, b, 1.0], Light::STANDARD);
    [r, g, b].map(|c| (c * 255.0).round() as u8)
}

/// The logical point a target pixel's top-left corner holds.
fn logical(px: u32, py: u32) -> (f32, f32) {
    (px as f32 / 2.0, (py as f32 - TOP) / 2.0)
}

#[test]
fn an_overlay_model_draws_inside_its_rectangle_over_the_fill() {
    let (Some(pixels), Some(without)) = (
        drawn(true, Upscaler::Native, RenderScale::Auto),
        drawn(false, Upscaler::Native, RenderScale::Auto),
    ) else {
        return;
    };
    let centre = AREA.center();
    assert_near(&pixels, (centre.x, centre.y), lit_face(), "the cube's face");
    // The fill round it: inside the rectangle, outside the cube's circle.
    assert_eq!(color_at(&pixels, 4.5, 4.5), PANEL, "the top-left corner");
    assert_eq!(color_at(&pixels, 19.5, 15.5), PANEL, "the bottom-right");
    assert_eq!(color_at(&pixels, 4.5, 10.0), PANEL, "the left edge");
    // Every pixel the cube changed is inside the rectangle.
    let mut changed = 0;
    for py in 0..SIZE {
        for px in 0..SIZE {
            if pixel(&pixels, (px, py)) == pixel(&without, (px, py)) {
                continue;
            }
            changed += 1;
            let (x, y) = logical(px, py);
            let inside = x >= AREA.min.x
                && x + 0.5 <= AREA.max.x
                && y >= AREA.min.y
                && y + 0.5 <= AREA.max.y;
            assert!(inside, "the cube drew at ({px}, {py}), logical ({x}, {y})");
        }
    }
    assert!(changed > 100, "the cube drew {changed} pixels");
}

#[test]
fn an_overlay_model_is_the_same_pixels_at_every_render_scale_and_upscaler() {
    let Some(native) = drawn(true, Upscaler::Native, RenderScale::Auto) else {
        return;
    };
    for upscaler in [Upscaler::Native, Upscaler::SharpBilinear] {
        for scale in [
            RenderScale::Auto,
            RenderScale::percent(100),
            RenderScale::percent(50),
        ] {
            let pixels = drawn(true, upscaler, scale).expect("an adapter");
            assert!(
                pixels == native,
                "{upscaler:?} at {scale:?} differs from native"
            );
        }
    }
}
