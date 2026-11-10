//! The render scale through the offscreen adapter: the world layer, a
//! solid and a cube model, drawn into a texture a share of the backing
//! size and enlarged by the chosen upscaler, the HUD drawn over it at the
//! target's own resolution, its pixels the same at every scale.
//!
//! Logical 32x24 in a 64x64 target at scale 2: the content is rows 8 to
//! 56, 64x48 pixels, so 25% is a 16x12 world texture, four target pixels
//! a texel, and 50% a 32x24 one. Needs a GPU adapter; skips, passing,
//! without one.

use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::{
    Gpu, Light, Mesh, RenderReport, RenderScale, Renderer, Upscaler, UpscalerProbe, Viewport,
};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, ModelKey, ModelPose, Orientation, Point};

use super::common::fill;
use super::common::pixels::{LOGICAL, SIZE, assert_near, color_at, near, pixel, pixel_of, rgb};
use super::wgpu_offscreen::{Images, gpu_with};

const CUBE: ModelKey = ModelKey(1);

/// The world solid's colour.
const WORLD: [u8; 3] = [40, 200, 90];
/// The HUD solid's colour.
const HUD: [u8; 3] = [220, 60, 30];
/// The cube's faces' colour.
const FACE: [u8; 3] = [230, 230, 230];
const BLACK: [u8; 3] = [0, 0, 0];

fn color([r, g, b]: [u8; 3]) -> Color {
    Color::rgba(r, g, b, 255)
}

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// The cube at (20, 12), 8 units a side, straight on: +z alone shows,
/// over x 16 to 24 and y 8 to 16.
fn cube_pose() -> ModelPose {
    ModelPose {
        center: at(20.0, 12.0),
        orientation: Orientation::IDENTITY,
        scale: 8.0,
    }
}

/// The world: a solid from x 5 (an odd unit, half a texel at 25%) to 13
/// over y 2 to 6, and the cube. The HUD: a solid from x 5 to 20 over y
/// 14 to 18, over the cube's bottom.
fn scene() -> DrawList {
    let mut list = DrawList::new();
    list.world(|list| {
        let solid = Bounds {
            min: at(5.0, 2.0),
            max: at(13.0, 6.0),
        };
        fill(list, solid, color(WORLD));
        list.model(CUBE, cube_pose());
    });
    let hud = Bounds {
        min: at(5.0, 14.0),
        max: at(20.0, 18.0),
    };
    fill(&mut list, hud, color(HUD));
    list
}

/// The scene drawn under `upscaler` at `scale`, read back; `None` with no
/// adapter.
fn drawn(upscaler: Upscaler, scale: RenderScale) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    gpu.set_upscaler(upscaler);
    let mut renderer = Renderer::new(Images);
    renderer.add_model(CUBE, Mesh::cube([color(FACE); 6]));
    renderer.set_render_scale(scale);
    let viewport = Viewport::new(LOGICAL, (SIZE, SIZE), 2.0);
    let report = renderer.render(&scene(), &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// The cube's +z face as the core shades it under the standard light.
fn lit_face() -> [u8; 3] {
    let [r, g, b] = FACE.map(|c| f32::from(c) / 255.0);
    let [r, g, b, _] = shade([0.0, 0.0, 1.0], [r, g, b, 1.0], Light::STANDARD);
    [r, g, b].map(|c| (c * 255.0).round() as u8)
}

/// The target pixels around the HUD solid: its rows (40 to 48), from
/// four pixels left of its edge at column 10 to its right edge at 40.
/// Left of the edge is the world's empty black; the rest is the HUD,
/// over the cube where they meet.
fn hud_pixels(pixels: &[u8]) -> Vec<[u8; 4]> {
    (40..48)
        .flat_map(|y| (6..40).map(move |x| (x, y)))
        .map(|at| pixel(pixels, at))
        .collect()
}

/// The world solid's top row's pixels from column 8 to 12, around its
/// edge at column 10.
fn world_edge(pixels: &[u8]) -> Vec<[u8; 4]> {
    let (_, y) = pixel_of(9.0, 3.0);
    (8..12).map(|x| pixel(pixels, (x, y))).collect()
}

/// The upscalers to enlarge with: native (sharp-bilinear under a
/// percentage), FSR 1, and MetalFX where the device has it.
fn enlargers() -> Vec<Upscaler> {
    let mut upscalers = vec![Upscaler::Native, Upscaler::Fsr1];
    if gpu_with(8, 8, &FontFaces::bundled())
        .is_some_and(|gpu| gpu.upscalers().contains(&Upscaler::MetalFxSpatial))
    {
        upscalers.push(Upscaler::MetalFxSpatial);
    }
    upscalers
}

#[test]
fn the_hud_is_drawn_at_full_resolution_whatever_the_render_scale() {
    let Some(auto) = drawn(Upscaler::Native, RenderScale::Auto) else {
        return;
    };
    assert_near(&auto, (12.0, 16.0), HUD, "the HUD");
    for percent in [25, 50] {
        let scaled = drawn(Upscaler::Native, RenderScale::percent(percent)).expect("an adapter");
        assert!(
            hud_pixels(&scaled) == hud_pixels(&auto),
            "the HUD changed at {percent}%"
        );
    }
}

#[test]
fn a_render_scale_draws_the_world_layer_coarser() {
    let (Some(auto), Some(quarter)) = (
        drawn(Upscaler::Native, RenderScale::Auto),
        drawn(Upscaler::Native, RenderScale::percent(25)),
    ) else {
        return;
    };
    let (_, row) = pixel_of(9.0, 3.0);
    // Natively the edge is crisp at column 10, logical x 5.
    assert_eq!(rgb(pixel(&auto, (9, row))), BLACK);
    assert_eq!(rgb(pixel(&auto, (10, row))), WORLD);
    // At 25% x 5 is half a 4-pixel texel in: the edge blends or moves.
    assert_ne!(
        world_edge(&quarter),
        world_edge(&auto),
        "the edge is as crisp"
    );
    // Texel interiors still show the world: the solid, and the cube's lit
    // face, the 3D layer drawn and scaled.
    assert_near(&quarter, (10.0, 4.0), WORLD, "the world solid at 25%");
    assert_near(&quarter, (20.0, 11.0), lit_face(), "the cube at 25%");
    assert_near(&auto, (20.0, 11.0), lit_face(), "the cube natively");
}

#[test]
fn a_render_scale_is_enlarged_by_the_chosen_upscaler() {
    let Some(auto) = drawn(Upscaler::Native, RenderScale::Auto) else {
        return;
    };
    // 50% is a whole 2x enlargement, 75% (48x36) a fractional one.
    for upscaler in enlargers() {
        for percent in [50, 75] {
            let case = format!("{upscaler:?} at {percent}%");
            let scaled = drawn(upscaler, RenderScale::percent(percent)).expect("an adapter");
            let solid = color_at(&scaled, 10.0, 4.0);
            assert!(near(solid, WORLD), "{case}: the world solid is {solid:?}");
            let cube = color_at(&scaled, 20.0, 11.0);
            assert!(
                near(cube, lit_face()),
                "{case}: the cube is {cube:?}, not {:?}",
                lit_face()
            );
            assert!(
                hud_pixels(&scaled) == hud_pixels(&auto),
                "{case}: the HUD changed"
            );
        }
    }
}
