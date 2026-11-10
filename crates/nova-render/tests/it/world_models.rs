//! A cube model drawn in the world under the HUD, offscreen: its visible
//! faces lit as `camera::shade` says, its far −z face hidden by the
//! depth test, mixed in list order with a sprite and a placeholder
//! triangle under it and a sprite over it, and the HUD over all of it.
//! Natively and under sharp-bilinear, which draws the world into a
//! texture of its own. Needs a GPU adapter; skips, passing, without one.

use std::f32::consts::FRAC_PI_6;

use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::{Gpu, Light, Mesh, RenderReport, Renderer, Upscaler, Viewport};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawList, ImageKey, ModelKey, ModelPose, Orientation, Point};

use super::common::fill;
use super::common::pixels::{
    LOGICAL, SIZE, TOP, assert_near, color_at, near, pixel, pixels_inside,
};
use super::wgpu_offscreen::{Images, gpu_with};

const CUBE: ModelKey = ModelKey(1);

/// The cube's faces' colours, in `Mesh::cube`'s order: +x, −x, +y, −y,
/// +z, −z.
const FACES: [[u8; 3]; 6] = [
    [0, 255, 255],
    [255, 0, 255],
    [0, 200, 0],
    [255, 255, 0],
    [230, 40, 40],
    [40, 40, 255],
];
/// Each face's outward normal, in the same order.
const NORMALS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];
const PLUS_Y: usize = 2;
const PLUS_Z: usize = 4;
const MINUS_Z: usize = 5;

/// The placeholder triangle's colour.
const TRIANGLE: [u8; 3] = [90, 160, 230];
/// `rlëD` 2's colour.
const WHITE: [u8; 3] = [255, 255, 255];
/// `rlëD` 9's colour.
const LIT: [u8; 3] = [132, 82, 41];
/// The HUD's opaque block's colour.
const HUD: [u8; 3] = [200, 40, 90];
const BACKGROUND: [u8; 3] = [0, 0, 0];

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn faces() -> [Color; 6] {
    FACES.map(|[r, g, b]| Color::rgba(r, g, b, 255))
}

/// The cube's turn: 30° about x, which tips +y toward the camera, so +z
/// and +y face it and −z and −y face away.
fn turn() -> Orientation {
    Orientation::about([1.0, 0.0, 0.0], FRAC_PI_6)
}

/// The cube at (12, 12), 10 logical units a side. Turned, +z covers
/// logical y 12 − 6.83 to 12 + 1.83 (5.17 to 13.83), +y 13.83 to 18.83,
/// and −z, behind them, 10.17 to 18.83; x 7 to 17 for each.
fn cube_pose() -> ModelPose {
    ModelPose {
        center: at(12.0, 12.0),
        orientation: turn(),
        scale: 10.0,
    }
}

/// The cube's silhouette: x 7 to 17, y 5.17 to 18.83.
const SILHOUETTE: (f32, f32, f32, f32) = (7.0, 5.17, 17.0, 18.83);
/// The −z face's projection: x 7 to 17, y 10.17 to 18.83.
const MINUS_Z_AREA: (f32, f32, f32, f32) = (7.0, 10.17, 17.0, 18.83);

/// The world: a placeholder triangle and a white sprite, each partly
/// under the cube, the cube (if `with_cube`), then a sprite over its top
/// right corner.
fn world(list: &mut DrawList, with_cube: bool) {
    let [r, g, b] = TRIANGLE;
    list.triangle(
        [at(14.0, 16.0), at(22.0, 16.0), at(22.0, 22.0)],
        Color::rgba(r, g, b, 255),
    )
    .sprite(ImageKey::sprite(2, 0), at(6.0, 12.0), Color::WHITE);
    if with_cube {
        list.model(CUBE, cube_pose());
    }
    list.sprite(ImageKey::sprite(9, 0), at(17.0, 5.0), Color::WHITE);
}

/// The HUD: an opaque block over the cube's +y face, and text.
fn hud(list: &mut DrawList) {
    let [r, g, b] = HUD;
    let block = Bounds {
        min: at(8.0, 15.0),
        max: at(11.0, 17.0),
    };
    fill(list, block, Color::rgba(r, g, b, 255));
    list.text("A", at(24.0, 14.0), 6.0, None, Color::WHITE);
}

fn scene(with_cube: bool) -> DrawList {
    let mut list = DrawList::new();
    list.world(|list| world(list, with_cube));
    hud(&mut list);
    list
}

/// `list` drawn with the cube registered, under `upscaler`, read back;
/// `None` with no adapter.
fn drawn(list: &DrawList, upscaler: Upscaler) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    gpu.set_upscaler(upscaler);
    let mut renderer = Renderer::new(Images);
    renderer.add_model(CUBE, Mesh::cube(faces()));
    let viewport = Viewport::new(LOGICAL, (SIZE, SIZE), 2.0);
    let report = renderer.render(list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// Face `face` as the core shades it under the standard light, turned.
fn shaded(face: usize) -> [u8; 3] {
    let normal = turn().rotate(NORMALS[face]).expect("a unit turn");
    let [r, g, b] = FACES[face].map(|c| f32::from(c) / 255.0);
    let [r, g, b, _] = shade(normal, [r, g, b, 1.0], Light::STANDARD);
    [r, g, b].map(|c| (c * 255.0).round() as u8)
}

/// The upscalers drawn, and how far an edge may spread under each, in
/// logical units: a pixel natively, a texel enlarged.
const UPSCALERS: [(Upscaler, f32); 2] = [(Upscaler::Native, 0.5), (Upscaler::SharpBilinear, 1.0)];

#[test]
fn a_cube_model_draws_under_the_hud() {
    for (upscaler, margin) in UPSCALERS {
        let (Some(pixels), Some(without)) = (
            drawn(&scene(true), upscaler),
            drawn(&scene(false), upscaler),
        ) else {
            return;
        };
        let case = format!("{upscaler:?}");

        // The HUD over the cube's +y face.
        assert_eq!(color_at(&pixels, 9.5, 15.5), HUD, "{case}");

        // The visible faces, lit as the core shades them.
        assert_near(&pixels, (10.5, 9.5), shaded(PLUS_Z), &format!("+z {case}"));
        assert_near(&pixels, (12.5, 16.5), shaded(PLUS_Y), &format!("+y {case}"));
        assert_ne!(shaded(PLUS_Z), shaded(PLUS_Y), "the two lit apart");

        // The far −z face hidden by depth, though it is drawn last. Only
        // −z witnesses the depth test: −y, the other far face, projects
        // to y 5.17 to 10.17, outside −z's area, and draws before +z,
        // which covers it in list order alone.
        for colour in [shaded(MINUS_Z), FACES[MINUS_Z]] {
            let showing = pixels_inside(MINUS_Z_AREA)
                .into_iter()
                .filter(|&p| {
                    let [r, g, b, _] = pixel(&pixels, p);
                    near([r, g, b], colour)
                })
                .count();
            assert_eq!(showing, 0, "−z shows at {showing} pixels {case}");
        }

        // The sprite and the triangle before the cube, hidden under it
        // and showing beside it.
        assert_near(
            &pixels,
            (7.5, 12.5),
            shaded(PLUS_Z),
            &format!("over the sprite {case}"),
        );
        assert_eq!(
            color_at(&pixels, 5.5, 12.5),
            WHITE,
            "the sprite beside {case}"
        );
        assert_near(
            &pixels,
            (16.5, 17.5),
            shaded(PLUS_Y),
            &format!("over the triangle {case}"),
        );
        assert_eq!(
            color_at(&pixels, 20.5, 19.5),
            TRIANGLE,
            "the triangle beside {case}"
        );

        // The sprite after it, over its corner.
        assert_eq!(
            color_at(&pixels, 16.5, 6.5),
            LIT,
            "the sprite over it {case}"
        );

        // Outside its silhouette, the frame is the frame without it.
        let (x0, y0, x1, y1) = SILHOUETTE;
        let (x0, y0, x1, y1) = (x0 - margin, y0 - margin, x1 + margin, y1 + margin);
        let mut differ = 0;
        for py in 0..SIZE {
            for px in 0..SIZE {
                let (x, y) = (px as f32 / 2.0, (py as f32 - TOP) / 2.0);
                let inside = x + 0.5 > x0 && x < x1 && y + 0.5 > y0 && y < y1;
                if !inside && pixel(&pixels, (px, py)) != pixel(&without, (px, py)) {
                    differ += 1;
                }
            }
        }
        assert_eq!(differ, 0, "pixels outside the cube changed {case}");
        let cube_pixels = pixels_inside(SILHOUETTE)
            .into_iter()
            .filter(|&p| pixel(&pixels, p) != pixel(&without, p))
            .count();
        assert!(
            cube_pixels > 100,
            "the cube drew {cube_pixels} pixels {case}"
        );
    }
}

/// Where [`crossing`]'s world is clipped: x = 16.
const EDGE: f32 = 16.0;

/// The cube alone, in a world clipped at x = [`EDGE`], through its
/// middle.
fn crossing() -> DrawList {
    let mut list = DrawList::new();
    let clip = Bounds {
        min: at(0.0, 0.0),
        max: at(EDGE, 24.0),
    };
    list.world_in(clip, |list| {
        list.model(CUBE, cube_pose());
    });
    list
}

#[test]
fn a_model_crossing_the_world_clip_is_cut_at_it() {
    for (upscaler, _) in UPSCALERS {
        let Some(pixels) = drawn(&crossing(), upscaler) else {
            return;
        };
        let case = format!("{upscaler:?}");
        let cut = (EDGE * 2.0) as u32;
        for py in 0..SIZE {
            for px in cut..SIZE {
                assert_eq!(
                    pixel(&pixels, (px, py)),
                    [0, 0, 0, 255],
                    "({px}, {py}) {case}"
                );
            }
        }
        // Just left of the edge, the faces are drawn.
        assert_near(&pixels, (15.5, 9.5), shaded(PLUS_Z), &format!("+z {case}"));
        assert_near(&pixels, (15.5, 16.5), shaded(PLUS_Y), &format!("+y {case}"));
        assert_eq!(color_at(&pixels, 15.5, 2.0), BACKGROUND);
    }
}
